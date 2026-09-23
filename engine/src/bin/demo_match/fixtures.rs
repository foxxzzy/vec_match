use anyhow::{bail, Context, Result};
use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use matching_engine_demo::{
    db::users::create_full_user,
    models::{
        user::{CreateUserPreferencesRequest, CreateUserProfileRequest, CreateUserRequest},
        Axis,
    },
};

use crate::scenarios::GateCase;

const DEMO_EMAIL_PATTERN: &str = "portfolio.demo.%@vector.local";

#[derive(Clone)]
pub(crate) struct DemoUser {
    pub(crate) name: String,
    pub(crate) birth_date: NaiveDate,
    pub(crate) height_cm: i16,
    pub(crate) gender_code: String,
    pub(crate) preferred_gender_code: String,
    pub(crate) latitude: f64,
    pub(crate) longitude: f64,
    pub(crate) min_age: i16,
    pub(crate) max_age: i16,
    pub(crate) min_height_cm: Option<i16>,
    pub(crate) max_height_cm: Option<i16>,
    pub(crate) max_distance_km: i32,
    pub(crate) exclude_smokers: bool,
    pub(crate) smoking_status_code: Option<String>,
}

pub(crate) struct Lookups {
    gender_a: String,
    gender_b: String,
    smoker_code: String,
}

pub(crate) fn apply_gate_case(
    gate_case: GateCase,
    lookups: &Lookups,
    a: &mut DemoUser,
    b: &mut DemoUser,
) {
    match gate_case {
        GateCase::Normal => {}
        GateCase::AgeMismatch => {
            a.min_age = 24;
            a.max_age = 30;
            b.birth_date = date(1980, 5, 10);
        }
        GateCase::DistanceMismatch => {
            a.max_distance_km = 25;
            b.max_distance_km = 25;
            b.latitude = 55.9533;
            b.longitude = -3.1883;
        }
        GateCase::HeightMismatch => {
            a.max_height_cm = Some(175);
            b.height_cm = 195;
        }
        GateCase::GenderMismatch => {
            a.preferred_gender_code = lookups.gender_a.clone();
        }
        GateCase::SmokingMismatch => {
            a.exclude_smokers = true;
            b.smoking_status_code = Some(lookups.smoker_code.clone());
        }
    }
}

pub(crate) async fn create_pair(
    pool: &PgPool,
    a_id: Uuid,
    a: &DemoUser,
    b_id: Uuid,
    b: &DemoUser,
    scenario_number: usize,
) -> Result<()> {
    create_demo_auth_user(
        pool,
        a_id,
        &format!("portfolio.demo.s{scenario_number}.a@vector.local"),
    )
    .await?;
    create_demo_auth_user(
        pool,
        b_id,
        &format!("portfolio.demo.s{scenario_number}.b@vector.local"),
    )
    .await?;

    create_demo_public_user(pool, a_id, a).await?;
    create_demo_public_user(pool, b_id, b).await?;

    println!("PAIR CREATED");
    println!("  {}: {}", a.name, a_id);
    println!("  {}: {}", b.name, b_id);
    println!();
    Ok(())
}


pub(crate) async fn check_static_data(pool: &PgPool) -> Result<()> {
    let gender_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM public.genders")
        .fetch_one(pool)
        .await?;
    if gender_count < 2 {
        bail!("the demo needs at least two rows in public.genders");
    }

    let prompt_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.embedded_content_who_are_they")
            .fetch_one(pool)
            .await?;
    if prompt_count == 0 {
        bail!("embedded_content_who_are_they is empty. Load the Supabase seed data first");
    }

    Ok(())
}

pub(crate) async fn load_lookups(pool: &PgPool) -> Result<Lookups> {
    let genders: Vec<String> = sqlx::query_scalar("SELECT code FROM public.genders ORDER BY code")
        .fetch_all(pool)
        .await?;

    let gender_a = genders
        .iter()
        .find(|code| code.eq_ignore_ascii_case("woman") || code.eq_ignore_ascii_case("female"))
        .cloned()
        .unwrap_or_else(|| genders[0].clone());

    let gender_b = genders
        .iter()
        .find(|code| {
            code.as_str() != gender_a.as_str()
                && (code.eq_ignore_ascii_case("man") || code.eq_ignore_ascii_case("male"))
        })
        .cloned()
        .or_else(|| genders.iter().find(|code| code.as_str() != gender_a.as_str()).cloned())
        .context("could not choose two distinct gender codes")?;

    let smoker_code: Option<String> = sqlx::query_scalar(
        "SELECT code FROM public.smoking_statuses WHERE lower(code) = 'smoker' LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    let smoker_code = smoker_code.context(
        "smoking_statuses needs a 'smoker' row because the hard-gate SQL checks that exact code",
    )?;

    Ok(Lookups {
        gender_a,
        gender_b,
        smoker_code,
    })
}

pub(crate) async fn load_available_axes(pool: &PgPool) -> Result<Vec<Axis>> {
    let axis_names: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT axis FROM public.embedded_content_who_are_they ORDER BY axis",
    )
    .fetch_all(pool)
    .await?;

    let mut axes = Vec::new();
    for axis_name in axis_names {
        match Axis::from_text(&axis_name) {
            Ok(axis) => axes.push(axis),
            Err(_) => println!("Warning: ignoring unknown embedded axis '{axis_name}'"),
        }
    }

    if axes.is_empty() {
        bail!("no recognised personality axes were found in embedded_content_who_are_they");
    }
    Ok(axes)
}

pub(crate) async fn reset_demo_users(pool: &PgPool) -> Result<()> {
    sqlx::query("DELETE FROM auth.users WHERE email LIKE $1")
        .bind(DEMO_EMAIL_PATTERN)
        .execute(pool)
        .await?;
    Ok(())
}

async fn create_demo_auth_user(pool: &PgPool, id: Uuid, email: &str) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO auth.users (
            id, aud, role, email, encrypted_password, email_confirmed_at,
            raw_app_meta_data, raw_user_meta_data, created_at, updated_at
        )
        VALUES (
            $1,
            'authenticated',
            'authenticated',
            $2,
            '$2a$10$7EqJtq98hPqEX7fNZaFWoOHI0v0S8m2MvmP0xSg6u40qCMgfHdC6W',
            now(),
            '{"provider":"email","providers":["email"]}'::jsonb,
            '{}'::jsonb,
            now(),
            now()
        )
        "#,
    )
    .bind(id)
    .bind(email)
    .execute(pool)
    .await?;
    Ok(())
}

async fn create_demo_public_user(pool: &PgPool, id: Uuid, user: &DemoUser) -> Result<()> {
    let request = CreateUserRequest {
        id,
        display_name: Some(user.name.clone()),
        birth_date: Some(user.birth_date),
        location_text: Some("Synthetic demo location".to_string()),
        profile: CreateUserProfileRequest {
            date_of_birth: user.birth_date,
            height_cm: Some(user.height_cm),
            gender_code: user.gender_code.clone(),
            religion_code: None,
            relationship_intent_code: None,
            wants_kids_code: None,
            smoking_status_code: user.smoking_status_code.clone(),
            drinking_status_code: None,
            city: Some("Demo City".to_string()),
            country: Some("Demo".to_string()),
            latitude: Some(user.latitude),
            longitude: Some(user.longitude),
            bio: Some("Synthetic portfolio demo user".to_string()),
        },
        preferences: Some(CreateUserPreferencesRequest {
            min_age: Some(user.min_age),
            max_age: Some(user.max_age),
            min_height_cm: user.min_height_cm,
            max_height_cm: user.max_height_cm,
            max_distance_km: Some(user.max_distance_km),
            exclude_smokers: Some(user.exclude_smokers),
            require_same_religion: Some(false),
        }),
        preferred_genders: vec![user.preferred_gender_code.clone()],
        preferred_religions: None,
        preferred_relationship_intents: None,
        photos: None,
    };

    create_full_user(pool, &request).await?;
    Ok(())
}

pub(crate) fn base_pair(lookups: &Lookups, a_name: &str, b_name: &str) -> (DemoUser, DemoUser) {
    let a = DemoUser {
        name: a_name.to_string(),
        birth_date: date(1998, 4, 12),
        height_cm: 170,
        gender_code: lookups.gender_a.clone(),
        preferred_gender_code: lookups.gender_b.clone(),
        latitude: 51.5072,
        longitude: -0.1276,
        min_age: 18,
        max_age: 100,
        min_height_cm: None,
        max_height_cm: None,
        max_distance_km: 100,
        exclude_smokers: false,
        smoking_status_code: None,
    };

    let b = DemoUser {
        name: b_name.to_string(),
        birth_date: date(1997, 9, 3),
        height_cm: 180,
        gender_code: lookups.gender_b.clone(),
        preferred_gender_code: lookups.gender_a.clone(),
        latitude: 51.5074,
        longitude: -0.1278,
        min_age: 18,
        max_age: 100,
        min_height_cm: None,
        max_height_cm: None,
        max_distance_km: 100,
        exclude_smokers: false,
        smoking_status_code: None,
    };

    (a, b)
}

pub(crate) fn scenario_ids(number: usize) -> (Uuid, Uuid) {
    let a_suffix = (number * 2 - 1) as u64;
    let b_suffix = (number * 2) as u64;

    let a = Uuid::parse_str(&format!("00000000-0000-4000-8000-{a_suffix:012x}"))
        .expect("valid demo UUID");
    let b = Uuid::parse_str(&format!("00000000-0000-4000-8000-{b_suffix:012x}"))
        .expect("valid demo UUID");
    (a, b)
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("valid demo date")
}


use std::collections::HashSet;

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::models::user::{
    CreateUserRequest, FullUserResponse, UpdatePreferencesRequest, UpdateProfileRequest,
    UpdateUserRequest, UserComputedRow, UserPhotoRow, UserPreferencesRow, UserProfileRow, UserRow,
};
use crate::models::{Axis, UserID, UserInfoVectors, Vector};
use crate::matching::candidate::Candidate;

#[derive(Debug, sqlx::FromRow)]
struct CandidateVectorRow {
    user_id: Uuid,
    axis: String,
    vector: Vector,
}

// ── Create ──────────────────────────────────────────────────────

/// Insert a new user row + empty profile, preferences, and computed rows
/// inside a single transaction.
pub async fn create_user(
    pool: &PgPool,
    req: &CreateUserRequest,
) -> Result<FullUserResponse, sqlx::Error> {
    create_full_user(pool, req).await
}

/// Insert a full normalized user representation in one transaction.
pub async fn create_full_user(
    pool: &PgPool,
    req: &CreateUserRequest,
) -> Result<FullUserResponse, sqlx::Error> {
    let mut tx: Transaction<Postgres> = pool.begin().await?;

    let user = sqlx::query_as::<_, UserRow>(
        r#"
        INSERT INTO users (id, display_name, birth_date, location_text)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(req.id)
    .bind(&req.display_name)
    .bind(req.birth_date)
    .bind(&req.location_text)
    .fetch_one(&mut *tx)
    .await?;

    let profile = sqlx::query_as::<_, UserProfileRow>(
        r#"
        INSERT INTO user_profile (
            user_id,
            date_of_birth,
            height_cm,
            gender_code,
            religion_code,
            relationship_intent_code,
            wants_kids_code,
            smoking_status_code,
            drinking_status_code,
            city,
            country,
            latitude,
            longitude,
            bio
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        RETURNING *
        "#,
    )
    .bind(req.id)
    .bind(req.profile.date_of_birth)
    .bind(req.profile.height_cm)
    .bind(&req.profile.gender_code)
    .bind(&req.profile.religion_code)
    .bind(&req.profile.relationship_intent_code)
    .bind(&req.profile.wants_kids_code)
    .bind(&req.profile.smoking_status_code)
    .bind(&req.profile.drinking_status_code)
    .bind(&req.profile.city)
    .bind(&req.profile.country)
    .bind(req.profile.latitude)
    .bind(req.profile.longitude)
    .bind(&req.profile.bio)
    .fetch_one(&mut *tx)
    .await?;

    let prefs = &req.preferences;
    let preferences = sqlx::query_as::<_, UserPreferencesRow>(
        r#"
        INSERT INTO user_preferences (
            user_id,
            min_age,
            max_age,
            min_height_cm,
            max_height_cm,
            max_distance_km,
            exclude_smokers,
            require_same_religion
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(req.id)
    .bind(prefs.as_ref().and_then(|p| p.min_age))
    .bind(prefs.as_ref().and_then(|p| p.max_age))
    .bind(prefs.as_ref().and_then(|p| p.min_height_cm))
    .bind(prefs.as_ref().and_then(|p| p.max_height_cm))
    .bind(prefs.as_ref().and_then(|p| p.max_distance_km))
    .bind(
        prefs
            .as_ref()
            .and_then(|p| p.exclude_smokers)
            .unwrap_or(false),
    )
    .bind(
        prefs
            .as_ref()
            .and_then(|p| p.require_same_religion)
            .unwrap_or(false),
    )
    .fetch_one(&mut *tx)
    .await?;

    let preferred_genders = dedupe_codes(&req.preferred_genders);
    for code in &preferred_genders {
        sqlx::query(
            r#"
            INSERT INTO user_preferred_genders (user_id, gender_code)
            VALUES ($1, $2)
            "#,
        )
        .bind(req.id)
        .bind(code)
        .execute(&mut *tx)
        .await?;
    }

    let preferred_religions = dedupe_codes(req.preferred_religions.as_deref().unwrap_or(&[]));
    for code in &preferred_religions {
        sqlx::query(
            r#"
            INSERT INTO user_preferred_religions (user_id, religion_code)
            VALUES ($1, $2)
            "#,
        )
        .bind(req.id)
        .bind(code)
        .execute(&mut *tx)
        .await?;
    }

    let preferred_relationship_intents =
        dedupe_codes(req.preferred_relationship_intents.as_deref().unwrap_or(&[]));
    for code in &preferred_relationship_intents {
        sqlx::query(
            r#"
            INSERT INTO user_preferred_relationship_intents (user_id, intent_code)
            VALUES ($1, $2)
            "#,
        )
        .bind(req.id)
        .bind(code)
        .execute(&mut *tx)
        .await?;
    }

    let mut photos = Vec::new();
    for photo in req.photos.as_deref().unwrap_or(&[]) {
        let row = sqlx::query_as::<_, UserPhotoRow>(
            r#"
            INSERT INTO user_photos (user_id, storage_path, position, is_primary)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(req.id)
        .bind(&photo.storage_path)
        .bind(photo.position)
        .bind(photo.is_primary.unwrap_or(false))
        .fetch_one(&mut *tx)
        .await?;
        photos.push(row);
    }

    let computed = sqlx::query_as::<_, UserComputedRow>(
        r#"
        INSERT INTO user_computed (user_id)
        VALUES ($1)
        RETURNING *
        "#,
    )
    .bind(req.id)
    .fetch_optional(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(FullUserResponse {
        user,
        profile,
        preferences,
        preferred_genders,
        preferred_religions,
        preferred_relationship_intents,
        photos,
        computed,
    })
}

fn dedupe_codes(values: &[String]) -> Vec<String> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut out = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed) {
            out.push(trimmed.to_string());
        }
    }
    out
}

// ── Read ────────────────────────────────────────────────────────

pub async fn get_user(pool: &PgPool, user_id: Uuid) -> Result<FullUserResponse, sqlx::Error> {
    let user = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;

    let profile =
        sqlx::query_as::<_, UserProfileRow>("SELECT * FROM user_profile WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await?;

    let preferences = sqlx::query_as::<_, UserPreferencesRow>(
        "SELECT * FROM user_preferences WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let preferred_genders = sqlx::query_scalar::<_, String>(
        "SELECT gender_code FROM user_preferred_genders WHERE user_id = $1 ORDER BY gender_code",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let preferred_religions = sqlx::query_scalar::<_, String>(
        "SELECT religion_code FROM user_preferred_religions WHERE user_id = $1 ORDER BY religion_code",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let preferred_relationship_intents = sqlx::query_scalar::<_, String>(
        "SELECT intent_code FROM user_preferred_relationship_intents WHERE user_id = $1 ORDER BY intent_code",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let photos = sqlx::query_as::<_, UserPhotoRow>(
        "SELECT * FROM user_photos WHERE user_id = $1 ORDER BY position, created_at",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let computed =
        sqlx::query_as::<_, UserComputedRow>("SELECT * FROM user_computed WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

    Ok(FullUserResponse {
        user,
        profile,
        preferences,
        preferred_genders,
        preferred_religions,
        preferred_relationship_intents,
        photos,
        computed,
    })
}

pub async fn get_user_profile(pool: &PgPool, user_id: Uuid) -> Result<UserProfileRow, sqlx::Error> {
    sqlx::query_as::<_, UserProfileRow>("SELECT * FROM user_profile WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
}

pub async fn get_user_preferences(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<UserPreferencesRow, sqlx::Error> {
    sqlx::query_as::<_, UserPreferencesRow>("SELECT * FROM user_preferences WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
}

pub async fn get_user_computed(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<UserComputedRow>, sqlx::Error> {
    sqlx::query_as::<_, UserComputedRow>("SELECT * FROM user_computed WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

/// Fetch the computed row and all axis vectors needed for matching.
pub async fn get_candidate(pool: &PgPool, user_id: Uuid) -> Result<Candidate, sqlx::Error> {
    let computed =
        sqlx::query_as::<_, UserComputedRow>("SELECT * FROM user_computed WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await?;

    let rows = sqlx::query_as::<_, CandidateVectorRow>(
        r#"
        SELECT user_id, axis, vector
        FROM user_info_vectors
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let vectors = rows
        .into_iter()
        .map(|row| {
            let axis = Axis::from_text(&row.axis).map_err(|_| {
                sqlx::Error::Decode(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid axis stored in DB: {}", row.axis),
                )))
            })?;

            Ok(UserInfoVectors {
                user_id: UserID(row.user_id),
                axis,
                embedding: row.vector,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;

    Ok(Candidate {
        user_id,
        computed,
        vectors,
        score: 0.0,
    })
}

// ── Update ──────────────────────────────────────────────────────

pub async fn update_user(
    pool: &PgPool,
    user_id: Uuid,
    req: &UpdateUserRequest,
) -> Result<UserRow, sqlx::Error> {
    sqlx::query_as::<_, UserRow>(
        r#"
        UPDATE users
        SET display_name  = COALESCE($2, display_name),
            birth_date    = COALESCE($3, birth_date),
            location_text = COALESCE($4, location_text),
            last_seen_at  = now()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(&req.display_name)
    .bind(req.birth_date)
    .bind(&req.location_text)
    .fetch_one(pool)
    .await
}

pub async fn update_profile(
    pool: &PgPool,
    user_id: Uuid,
    req: &UpdateProfileRequest,
) -> Result<UserProfileRow, sqlx::Error> {
    sqlx::query_as::<_, UserProfileRow>(
        r#"
        UPDATE user_profile
        SET date_of_birth             = COALESCE($2, date_of_birth),
            height_cm                 = COALESCE($3, height_cm),
            gender_code               = COALESCE($4, gender_code),
            religion_code             = COALESCE($5, religion_code),
            relationship_intent_code  = COALESCE($6, relationship_intent_code),
            wants_kids_code           = COALESCE($7, wants_kids_code),
            smoking_status_code       = COALESCE($8, smoking_status_code),
            drinking_status_code      = COALESCE($9, drinking_status_code),
            city                      = COALESCE($10, city),
            country                   = COALESCE($11, country),
            latitude                  = COALESCE($12, latitude),
            longitude                 = COALESCE($13, longitude),
            bio                       = COALESCE($14, bio),
            updated_at                = now()
        WHERE user_id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(req.date_of_birth)
    .bind(req.height_cm)
    .bind(&req.gender_code)
    .bind(&req.religion_code)
    .bind(&req.relationship_intent_code)
    .bind(&req.wants_kids_code)
    .bind(&req.smoking_status_code)
    .bind(&req.drinking_status_code)
    .bind(&req.city)
    .bind(&req.country)
    .bind(req.latitude)
    .bind(req.longitude)
    .bind(&req.bio)
    .fetch_one(pool)
    .await
}

pub async fn update_preferences(
    pool: &PgPool,
    user_id: Uuid,
    req: &UpdatePreferencesRequest,
) -> Result<UserPreferencesRow, sqlx::Error> {
    sqlx::query_as::<_, UserPreferencesRow>(
        r#"
        UPDATE user_preferences
        SET min_age               = COALESCE($2, min_age),
            max_age               = COALESCE($3, max_age),
            min_height_cm         = COALESCE($4, min_height_cm),
            max_height_cm         = COALESCE($5, max_height_cm),
            max_distance_km       = COALESCE($6, max_distance_km),
            exclude_smokers       = COALESCE($7, exclude_smokers),
            require_same_religion = COALESCE($8, require_same_religion),
            updated_at            = now()
        WHERE user_id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(req.min_age)
    .bind(req.max_age)
    .bind(req.min_height_cm)
    .bind(req.max_height_cm)
    .bind(req.max_distance_km)
    .bind(req.exclude_smokers)
    .bind(req.require_same_religion)
    .fetch_one(pool)
    .await
}

/// Upsert just the attachment columns on user_computed
pub async fn upsert_user_computed_attachment(
    pool: &PgPool,
    user_id: Uuid,
    closeness_type: &str,
    closeness_margin: f32,
    reassurance_type: &str,
    reassurance_margin: f32,
    summary: &str,
) -> Result<UserComputedRow, sqlx::Error> {
    sqlx::query_as::<_, UserComputedRow>(
        r#"
        UPDATE user_computed
        SET attachment_closeness_type   = $2,
            attachment_closeness_margin  = $3,
            attachment_reassurance_type  = $4,
            attachment_reassurance_margin = $5,
            summary = $6,
            computed_at = now()
        WHERE user_id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(closeness_type)
    .bind(closeness_margin)
    .bind(reassurance_type)
    .bind(reassurance_margin)
    .bind(summary)
    .fetch_one(pool)
    .await
}

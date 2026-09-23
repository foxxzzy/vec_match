use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Request bodies ──────────────────────────────────────────────

/// POST /users/full - create user + normalized profile/preferences/photos in one payload
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub id: Uuid,
    pub display_name: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub location_text: Option<String>,
    pub profile: CreateUserProfileRequest,
    pub preferences: Option<CreateUserPreferencesRequest>,
    pub preferred_genders: Vec<String>,
    pub preferred_religions: Option<Vec<String>>,
    pub preferred_relationship_intents: Option<Vec<String>>,
    pub photos: Option<Vec<CreateUserPhotoRequest>>,
}

pub type CreateFullUserRequest = CreateUserRequest;

#[derive(Debug, Deserialize)]
pub struct CreateUserProfileRequest {
    pub date_of_birth: NaiveDate,
    pub height_cm: Option<i16>,
    pub gender_code: String,
    pub religion_code: Option<String>,
    pub relationship_intent_code: Option<String>,
    pub wants_kids_code: Option<String>,
    pub smoking_status_code: Option<String>,
    pub drinking_status_code: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub bio: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserPreferencesRequest {
    pub min_age: Option<i16>,
    pub max_age: Option<i16>,
    pub min_height_cm: Option<i16>,
    pub max_height_cm: Option<i16>,
    pub max_distance_km: Option<i32>,
    pub exclude_smokers: Option<bool>,
    pub require_same_religion: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserPhotoRequest {
    pub storage_path: String,
    pub position: i16,
    pub is_primary: Option<bool>,
}

/// PATCH /users/{id}
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub location_text: Option<String>,
}

/// PATCH /users/{id}/profile
#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub date_of_birth: Option<NaiveDate>,
    pub height_cm: Option<i16>,
    pub gender_code: Option<String>,
    pub religion_code: Option<String>,
    pub relationship_intent_code: Option<String>,
    pub wants_kids_code: Option<String>,
    pub smoking_status_code: Option<String>,
    pub drinking_status_code: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub bio: Option<String>,
}

/// PATCH /users/{id}/preferences
#[derive(Debug, Deserialize)]
pub struct UpdatePreferencesRequest {
    pub min_age: Option<i16>,
    pub max_age: Option<i16>,
    pub min_height_cm: Option<i16>,
    pub max_height_cm: Option<i16>,
    pub max_distance_km: Option<i32>,
    pub exclude_smokers: Option<bool>,
    pub require_same_religion: Option<bool>,
}

// ── Response / DB row types ─────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub display_name: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub location_text: Option<String>,
    pub is_active: bool,
    pub is_banned: bool,
    pub last_seen_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserProfileRow {
    pub user_id: Uuid,
    pub date_of_birth: NaiveDate,
    pub height_cm: Option<i16>,
    pub gender_code: String,
    pub religion_code: Option<String>,
    pub relationship_intent_code: Option<String>,
    pub wants_kids_code: Option<String>,
    pub smoking_status_code: Option<String>,
    pub drinking_status_code: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub bio: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserPreferencesRow {
    pub user_id: Uuid,
    pub min_age: Option<i16>,
    pub max_age: Option<i16>,
    pub min_height_cm: Option<i16>,
    pub max_height_cm: Option<i16>,
    pub max_distance_km: Option<i32>,
    pub exclude_smokers: bool,
    pub require_same_religion: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserComputedRow {
    pub user_id: Uuid,
    pub computed_at: chrono::DateTime<chrono::Utc>,
    pub attachment_closeness_type: Option<String>,
    pub attachment_closeness_margin: Option<f32>,
    pub attachment_reassurance_type: Option<String>,
    pub attachment_reassurance_margin: Option<f32>,
    pub summary: Option<String>,
}

/// Full user response bundling all sub-tables
#[derive(Debug, Serialize)]
pub struct FullUserResponse {
    pub user: UserRow,
    pub profile: UserProfileRow,
    pub preferences: UserPreferencesRow,
    pub preferred_genders: Vec<String>,
    pub preferred_religions: Vec<String>,
    pub preferred_relationship_intents: Vec<String>,
    pub photos: Vec<UserPhotoRow>,
    pub computed: Option<UserComputedRow>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserPhotoRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub storage_path: String,
    pub position: i16,
    pub is_primary: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

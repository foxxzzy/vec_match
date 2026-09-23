use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserMatchQuestionRow {
    pub question_id: Uuid,
    pub user_id: Uuid,
    pub question: String,
    pub position: i32,
    pub is_required: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct InterestedPairQuestionAnswerRow {
    pub answer_id: Uuid,
    pub user_low_id: Uuid,
    pub user_high_id: Uuid,
    pub question_id: Uuid,
    pub asked_by_user_id: Uuid,
    pub answered_by_user_id: Uuid,
    pub answer: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct MatchQuestionForAnswerRow {
    pub question_id: Uuid,
    pub asked_by_user_id: Uuid,
    pub question: String,
    pub position: i32,
    pub is_required: bool,
    pub existing_answer: Option<String>,
    pub answered_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct InterestedPairAnswerWithQuestionRow {
    pub answer_id: Uuid,
    pub user_low_id: Uuid,
    pub user_high_id: Uuid,
    pub question_id: Uuid,
    pub question: String,
    pub asked_by_user_id: Uuid,
    pub answered_by_user_id: Uuid,
    pub answer: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

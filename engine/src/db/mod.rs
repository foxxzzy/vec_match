pub mod anchor_types;
pub mod content;
pub mod embedded_content;
pub mod hard_gated_candidates;
pub mod interested_pairs;
pub mod ledger;
pub mod match_questions;
pub mod match_queue;
pub mod matches;
pub mod pair_state;
pub mod serious_candidates;
pub mod user_info_vectors;
pub mod users;

use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::utils::env::load_env;

pub const DB_MAX_CONNECTIONS: u32 = 5;

pub async fn init_pool() -> Result<PgPool, sqlx::Error> {
    let database_url = load_env("DATABASE_URL".to_string())
        .expect("DATABASE_URL must be set in .env or environment");

    PgPoolOptions::new()
        .max_connections(DB_MAX_CONNECTIONS)
        .connect(&database_url)
        .await
}

pub mod create;
pub mod read;
pub mod refill_candidates;
pub mod update;

use axum::{
    Router,
    routing::{get, post},
};
use sqlx::PgPool;

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/users", post(create::create_user))
        .route("/users/full", post(create::create_full_user))
        .route(
            "/users/refill_candidates",
            post(refill_candidates::refill_candidates),
        )
        .route(
            "/users/:user_id",
            get(read::get_user).patch(update::update_user),
        )
        .route(
            "/users/:user_id/profile",
            get(read::get_user_profile).patch(update::update_profile),
        )
        .route(
            "/users/:user_id/preferences",
            get(read::get_user_preferences).patch(update::update_preferences),
        )
        .route("/users/:user_id/computed", get(read::get_user_computed))
}

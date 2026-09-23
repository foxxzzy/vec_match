pub mod content;
pub mod export;
pub mod health;
pub mod interaction;
pub mod matching;
pub mod users;
pub mod vectors;

use axum::{
    Router,
    routing::{delete, get, patch, post},
};
use sqlx::PgPool;

pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/db/health", get(health::db_health))
        .route("/content/upload", post(content::upload_content))
        .route("/content/export", get(export::export_content_for_axis))
        .route(
            "/anchor_types/export",
            get(export::export_anchor_types_for_axis),
        )
        .route("/user/interaction", post(interaction::user_interaction))
        .route("/matching/full", post(matching::run_full_matching))
        .route(
            "/matching/queue/release",
            post(matching::release_queue_candidates),
        )
        .route("/matching/queue/:user_id", get(matching::get_match_queue))
        .route(
            "/matching/queue/status",
            patch(matching::update_match_queue_status),
        )
        .route(
            "/matching/queue/decision",
            post(matching::submit_queue_decision),
        )
        .route(
            "/matching/serious/:user_id",
            get(matching::list_serious_candidates),
        )
        .route(
            "/matching/serious/:user_id/count",
            get(matching::count_serious_candidates),
        )
        .route(
            "/matching/serious/:user_id/expire-stale",
            post(matching::expire_stale_serious_candidates),
        )
        .route(
            "/matching/serious/:user_id/:candidate_user_id",
            delete(matching::delete_serious_candidate),
        )
        .route(
            "/matching/pair-state/:user_id/:other_user_id",
            get(matching::get_pair_state),
        )
        .route("/matching/questions", post(matching::create_match_question))
        .route(
            "/matching/questions/:user_id",
            get(matching::list_match_questions),
        )
        .route(
            "/matching/questions/:user_id/:question_id",
            patch(matching::update_match_question).delete(matching::delete_match_question),
        )
        .route(
            "/matching/interested/:user_id",
            get(matching::list_interested_pairs),
        )
        .route(
            "/matching/interested/:user_id/:other_user_id",
            get(matching::get_interested_pair),
        )
        .route(
            "/matching/interested/:user_id/:other_user_id/questions-to-answer",
            get(matching::list_questions_to_answer),
        )
        .route(
            "/matching/interested/:user_id/:other_user_id/answers",
            get(matching::list_interested_answers),
        )
        .route(
            "/matching/interested/answers",
            post(matching::submit_interested_answer),
        )
        .route(
            "/matching/interested/final-decision",
            post(matching::submit_interested_final_decision),
        )
        .route(
            "/matching/interested/expire",
            post(matching::expire_interested_pair),
        )
        .route("/matching/matches/:user_id", get(matching::list_matches))
        .route(
            "/matching/matches/:user_id/:other_user_id",
            get(matching::get_match),
        )
        .route("/matching/matches/unmatch", post(matching::unmatch_pair))
        .route(
            "/vectors/compute-axis-vector",
            post(vectors::compute_final_axis_vector),
        )
        .merge(users::router())
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;

    use super::router;

    #[tokio::test]
    async fn router_builds_without_route_conflicts() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost/postgres")
            .expect("lazy pool should be created");

        let _ = router(pool);
    }
}

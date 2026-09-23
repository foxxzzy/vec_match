use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;
use std::collections::HashMap;

use crate::{
    db::ledger::get_all_user_ledger_entries,
    db::user_info_vectors::insert_user_info_vectors,
    models::{LedgerEntryDb, UserID},
    matching::user_vectors::compute_final_vectors_from_ledger,
};

pub async fn compute_final_axis_vector(
    State(pool): State<PgPool>,
    Json(user_id): Json<UserID>,
) -> Response {
    let all_ledger_entries = match get_all_user_ledger_entries(&pool, user_id).await {
        Ok(entries) => entries,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to fetch ledger entries: {err}"),
            )
                .into_response();
        }
    };

    // the string in this hash map is an axis. Each axis key maps to a vector of ledger entries for that axis
    let entries_by_axis: HashMap<String, Vec<LedgerEntryDb>> =
        filter_by_axis(all_ledger_entries.clone());

    for (axis_key, entries) in entries_by_axis.into_iter() {
        match compute_final_vectors_from_ledger(entries) {
            Ok(vec) => {
                if let Err(err) = insert_user_info_vectors(&pool, &vec).await {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("failed to insert axis vector for {}: {}", axis_key, err),
                    )
                        .into_response();
                }
            }
            Err(err) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to compute axis vector for {}: {}", axis_key, err),
                )
                    .into_response();
            }
        }
    }

    StatusCode::OK.into_response()
}

pub fn filter_by_axis(entries: Vec<LedgerEntryDb>) -> HashMap<String, Vec<LedgerEntryDb>> {
    let mut map: HashMap<String, Vec<LedgerEntryDb>> = HashMap::new();

    for entry in entries.into_iter() {
        let axis_key: String = entry.axis.axis_to_text().to_string();
        map.entry(axis_key).or_insert_with(Vec::new).push(entry);
    }

    map
}

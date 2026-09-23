use std::cmp::Ordering;
use std::collections::HashMap;

use futures::stream::{self, StreamExt, TryStreamExt};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::hard_gated_candidates::mark_candidates_consumed;
use crate::db::{
    DB_MAX_CONNECTIONS, serious_candidates::upsert_serious_candidates, users::get_candidate,
};
use crate::matching::attachment_state::ensure_attachment_for_candidates;
use crate::matching::candidate::Candidate;
use crate::matching::scoring::{
    candidate_passes_threshold, log_score_breakdown, score_candidate,
};
use crate::models::serious_candidate::SeriousCandidate;
use crate::{
    db::hard_gated_candidates::fetch_unconsumed_candidates,
    models::hard_gated_candidate::HardGatedCandidateRow,
};

pub async fn full_matching_algorithm(
    pool: PgPool,
    user_id: Uuid,
) -> Result<Vec<SeriousCandidate>, String> {
    let hard_gated_candidates: Vec<HardGatedCandidateRow> =
        match fetch_unconsumed_candidates(&pool, user_id).await {
            Ok(val) => val,
            Err(e) => {
                return Err(format!("Error fetching hard-gated candidates: {}", e));
            }
        };

    if hard_gated_candidates.is_empty() {
        return Err("no candidates were available for this user_id".to_string());
    }

    let candidate_ids_to_mark_consumed = hard_gated_candidates
        .iter()
        .map(|candidate| candidate.candidate_user_id)
        .collect::<Vec<_>>();

    let expires_at_by_candidate = hard_gated_candidates
        .iter()
        .map(|candidate| (candidate.candidate_user_id, candidate.expires_at))
        .collect::<HashMap<_, _>>();

    let mut person = match get_candidate(&pool, user_id).await {
        Ok(val) => val,
        Err(e) => {
            return Err(format!("Candidate not found: {}", e));
        }
    };

    let mut candidates = match stream::iter(hard_gated_candidates.into_iter().map(|candidate| {
        let pool = &pool;
        async move { get_candidate(pool, candidate.candidate_user_id).await }
    }))
    .buffer_unordered(DB_MAX_CONNECTIONS as usize)
    .try_collect::<Vec<_>>()
    .await
    {
        Ok(candidates) => candidates,
        Err(e) => {
            return Err(format!("Error fetching candidate info: {}", e));
        }
    };

    if let Err(e) = ensure_attachment_for_candidates(&pool, &mut person, &mut candidates).await {
        return Err(format!("Error ensuring attachment candidates: {}", e));
    }

    let mut retained_candidates = Vec::new();

    for mut candidate in candidates {
        let breakdown = score_candidate(&person, &candidate);
        log_score_breakdown(&breakdown);

        if candidate_passes_threshold(&breakdown) {
            candidate.score = breakdown.final_score;
            retained_candidates.push(candidate);
        }
    }

    mark_candidates_consumed(&pool, user_id, &candidate_ids_to_mark_consumed)
        .await
        .map_err(|e| format!("Error marking candidates consumed: {}", e))?;

    if retained_candidates.is_empty() {
        return Ok(vec![]);
    }

    retained_candidates.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
    });

    let serious_candidates =
        build_serious_candidates(user_id, &retained_candidates, &expires_at_by_candidate);

    if let Err(e) = upsert_serious_candidates(&pool, &serious_candidates).await {
        return Err(format!("Error upserting serious candidates: {}", e));
    }

    Ok(serious_candidates)
}

fn build_serious_candidates(
    user_id: Uuid,
    candidates: &[Candidate],
    expires_at_by_candidate: &HashMap<Uuid, Option<chrono::DateTime<chrono::Utc>>>,
) -> Vec<SeriousCandidate> {
    candidates
        .iter()
        .enumerate()
        .map(|(idx, candidate)| SeriousCandidate {
            user_id,
            candidate_user_id: candidate.user_id,
            position: idx as i32,
            score: candidate.score,
            expires_at: expires_at_by_candidate
                .get(&candidate.user_id)
                .copied()
                .flatten(),
        })
        .collect()
}

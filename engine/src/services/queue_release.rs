use sqlx::PgPool;
use uuid::Uuid;

use crate::db;

/// Release up to `slots_needed` serious candidates into match_queue for a user.
///
/// For each available candidate, re-validates that the pair is still valid
/// (not blocked, rejected, matched, or already queued) before inserting
/// into match_queue.
///
/// Returns the number of candidates actually released into the queue.
pub async fn release_serious_candidates_to_queue(
    pool: &PgPool,
    user_id: Uuid,
    slots_needed: i64,
) -> Result<u64, sqlx::Error> {
    if slots_needed <= 0 {
        return Ok(0);
    }

    // Fetch more than needed in case some fail validation
    let fetch_limit = slots_needed * 2;
    let candidates =
        db::serious_candidates::fetch_available_candidates(pool, user_id, fetch_limit).await?;

    let mut released_ids: Vec<Uuid> = Vec::new();
    let mut queue_inserts: Vec<(Uuid, i32, f64)> = Vec::new();
    let mut invalid_ids: Vec<Uuid> = Vec::new();

    for candidate in &candidates {
        if queue_inserts.len() as i64 >= slots_needed {
            break;
        }

        let cid = candidate.candidate_user_id;

        // Check pair_state — if any interaction already exists, skip
        if db::pair_state::pair_state_exists(pool, user_id, cid).await? {
            invalid_ids.push(cid);
            continue;
        }

        // Check if already matched
        if db::matches::match_exists(pool, user_id, cid).await? {
            invalid_ids.push(cid);
            continue;
        }

        // Valid — stage for queue insertion
        queue_inserts.push((cid, candidate.position, candidate.score));
        released_ids.push(cid);
    }

    // Remove invalid candidates from the shortlist
    if !invalid_ids.is_empty() {
        db::serious_candidates::mark_removed(pool, user_id, &invalid_ids).await?;
    }

    if queue_inserts.is_empty() {
        return Ok(0);
    }

    // Insert into match_queue
    let enqueued = db::match_queue::enqueue_candidates(pool, user_id, &queue_inserts).await?;

    // Mark released in serious_candidates
    db::serious_candidates::mark_released(pool, user_id, &released_ids).await?;

    Ok(enqueued)
}

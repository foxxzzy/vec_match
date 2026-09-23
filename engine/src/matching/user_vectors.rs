use crate::models::LedgerEntryDb;
use crate::models::UserInfoVectors;
use crate::models::Vector;
use crate::matching::vector_math::{add_vectors, apply_weight, normalise, time_decay};
use chrono::Utc;

/// Aggregate ledger entries for one axis into a normalised user vector.
pub fn compute_final_vectors_from_ledger(
    entries: Vec<LedgerEntryDb>,
) -> Result<UserInfoVectors, String> {
    if entries.is_empty() {
        return Err("no entries provided".to_string());
    }

    let chosen_axis = entries[0].axis.clone();
    let user_id = entries[0].user_id.clone();
    let half_life_days = chosen_axis.decay_half_life_days();

    // compute weighted vectors for all entries for this axis, then sum them
    let mut final_vec: Option<Vector> = None;

    for v in entries.iter() {
        if v.axis != chosen_axis {
            return Err(format!(
                "Mixed axes in entries: expected {:?}, found {:?}",
                chosen_axis, v.axis
            ));
        }
        let now = Utc::now();
        let time_decay_weight = time_decay(v.created_at, now, half_life_days);
        let weight = v.weight;
        let interaction_weight = f32::from(v.reaction.reaction_to_int());

        let combined_weight = time_decay_weight * weight * interaction_weight;

        let weighted_vec_val = apply_weight(combined_weight, &v.embedding);

        final_vec = Some(match final_vec {
            None => weighted_vec_val,
            Some(current) => add_vectors(current, weighted_vec_val)?,
        });
    }

    let mut final_vec = final_vec.ok_or_else(|| "no entries provided".to_string())?;

    // normalise the vector to remove the magnitude
    normalise(&mut final_vec);

    let user_info_vec = UserInfoVectors {
        user_id,
        axis: chosen_axis,
        embedding: final_vec,
    };

    Ok(user_info_vec)
}

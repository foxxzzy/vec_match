//! Build stored attachment anchors from controlled synthetic interactions.

use std::collections::HashMap;
use std::path::Path;

use sqlx::PgPool;
use uuid::Uuid;

use crate::db::{anchor_types::upsert_anchor_type_vector, embedded_content::get_embedded_content};
use crate::models::{Axis, PromptId, Reaction, Vector};
use crate::matching::vector_math::{add_vectors, apply_weight, normalise};

#[derive(Debug, serde::Deserialize)]
pub struct SyntheticAnchorFile(HashMap<String, SyntheticAnchorType>);

#[derive(Debug, serde::Deserialize)]
pub struct SyntheticAnchorType {
    pub user_id: Uuid,
    pub interactions: Vec<SyntheticInteraction>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SyntheticInteraction {
    pub user_id: Uuid,
    pub id: Uuid,
    pub reaction: Reaction,
}

/// Reads a JSON file shaped like:
/// {
///   "high_avoidant": { "user_id": "...", "interactions": [ {"user_id":"...","id":"...","reaction":"Me"}, ... ] },
///   "neutral_avoidant": { ... }
/// }
pub fn read_synthetic_anchor_file(path: impl AsRef<Path>) -> Result<SyntheticAnchorFile, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice::<SyntheticAnchorFile>(&bytes).map_err(|e| e.to_string())
}

/// Compute and upsert vectors into `anchor_types` for all types in the provided synthetic file.
///
/// Weighting: identical to engine logic *except* no time decay.
/// combined_weight = embedded.weight * interaction_weight
///
/// Assumption: each synthetic type only references a single axis.
pub async fn compute_and_upsert_anchor_types_from_file(
    pool: &PgPool,
    path: impl AsRef<Path>,
) -> Result<(), String> {
    let synthetic = read_synthetic_anchor_file(path)?;
    compute_and_upsert_anchor_types(pool, synthetic).await
}

pub async fn compute_and_upsert_anchor_types(
    pool: &PgPool,
    synthetic: SyntheticAnchorFile,
) -> Result<(), String> {
    for (type_name, anchor_type) in synthetic.0 {
        compute_and_upsert_single_anchor_type(pool, &type_name, anchor_type).await?;
    }

    Ok(())
}

async fn compute_and_upsert_single_anchor_type(
    pool: &PgPool,
    type_name: &str,
    anchor_type: SyntheticAnchorType,
) -> Result<(), String> {
    if anchor_type.interactions.is_empty() {
        return Err(format!("{} has no interactions", type_name));
    }

    let mut final_vec: Option<Vector> = None;
    let mut axis_seen: Option<Axis> = None;

    for interaction in anchor_type.interactions {
        // fetch embedded content for this prompt id
        let embedded = get_embedded_content(pool, &PromptId(interaction.id))
            .await
            .map_err(|e| format!("failed to fetch embedded content {}: {}", interaction.id, e))?;

        // enforce single-axis assumption per type
        if let Some(existing) = &axis_seen {
            if *existing != embedded.axis {
                return Err(format!(
                    "{} has mixed axes: expected {:?}, found {:?} for prompt_id {}",
                    type_name, existing, embedded.axis, embedded.id.0
                ));
            }
        } else {
            axis_seen = Some(embedded.axis.clone());
        }

        let interaction_weight = match interaction.reaction {
            Reaction::Me => 1.0,
            Reaction::NotMe => -1.0,
            Reaction::Skip => 0.0,
        };

        let combined_weight = embedded.weight * interaction_weight;
        if combined_weight == 0.0 {
            continue;
        }

        let weighted_vec_val = apply_weight(combined_weight, &embedded.embedding);

        final_vec = Some(match final_vec {
            None => weighted_vec_val,
            Some(current) => add_vectors(current, weighted_vec_val)?,
        });
    }

    let mut final_vec = final_vec.ok_or_else(|| {
        format!(
            "{} produced no vector (all interactions were Skip or weight==0)",
            type_name
        )
    })?;

    normalise(&mut final_vec);

    let axis = axis_seen.ok_or_else(|| format!("{} had no axis", type_name))?;

    upsert_anchor_type_vector(pool, type_name, &axis, &final_vec)
        .await
        .map_err(|e| format!("failed to upsert anchor_types for {}: {}", type_name, e))?;

    Ok(())
}

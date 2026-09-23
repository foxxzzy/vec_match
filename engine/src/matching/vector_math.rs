use chrono::{DateTime, Utc};

use crate::models::Vector;

pub fn add_vectors(left: Vector, right: Vector) -> Result<Vector, &'static str> {
    if left.len() != right.len() {
        return Err("vectors must have the same length");
    }

    Ok(left
        .iter()
        .zip(right.iter())
        .map(|(left, right)| left + right)
        .collect())
}

pub fn cosine_similarity(left: &Vector, right: &Vector) -> f32 {
    let dot_product: f32 = left.iter().zip(right.iter()).map(|(a, b)| a * b).sum();
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_norm = right.iter().map(|value| value * value).sum::<f32>().sqrt();

    if left_norm == 0.0 || right_norm == 0.0 {
        return 0.0;
    }

    dot_product / (left_norm * right_norm)
}

/// Normalise a vector so interaction volume does not dominate its direction.
pub fn normalise(vector: &mut Vector) {
    let magnitude = vector
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();

    if magnitude != 0.0 {
        for value in vector {
            *value /= magnitude;
        }
    }
}

/// Exponential decay expressed as a half-life in days.
pub fn time_decay(interacted_at: DateTime<Utc>, now: DateTime<Utc>, half_life_days: f32) -> f32 {
    if half_life_days <= 0.0 {
        return 1.0;
    }

    let age_seconds = (now - interacted_at).num_seconds();
    if age_seconds <= 0 {
        return 1.0;
    }

    let age_days = age_seconds as f32 / 86_400.0;
    2.0_f32.powf(-age_days / half_life_days)
}

pub fn apply_weight(weight: f32, embedding: &Vector) -> Vector {
    embedding.iter().map(|value| value * weight).collect()
}

use std::cmp::Ordering;

use uuid::Uuid;

use crate::matching::candidate::{Candidate, attachment_score};
use crate::matching::vector_math::cosine_similarity;
use crate::models::{Axis, Vector};

pub const GLOBAL_MATCH_THRESHOLD: f64 = 0.60;

const ATTACHMENT_WEIGHT: f64 = 1.80;
const FUTURE_FOCUS_WEIGHT: f64 = 1.25;
const MORAL_FRAMING_WEIGHT: f64 = 1.20;
const SOCIAL_VALUES_WEIGHT: f64 = 1.20;
const COMMUNICATION_STYLE_WEIGHT: f64 = 1.10;
const THINKING_MODE_WEIGHT: f64 = 0.95;
const RELATIONSHIP_PACE_WEIGHT: f64 = 0.90;
const CONFLICT_APPROACH_WEIGHT: f64 = 0.80;
const REPAIR_STRATEGY_WEIGHT: f64 = 0.80;
const INTERDEPENDENCE_WEIGHT: f64 = 0.75;
const CHANGE_ORIENTATION_WEIGHT: f64 = 0.65;
const SOCIAL_ENERGY_WEIGHT: f64 = 0.60;
const ROUTINE_PREFERENCE_WEIGHT: f64 = 0.60;
const PLAYFULNESS_WEIGHT: f64 = 0.55;
const RISK_TOLERANCE_WEIGHT: f64 = 0.55;

#[derive(Debug, Clone)]
pub struct AxisScore {
    pub axis_name: String,
    pub score: f64,
    pub weight: f64,
    pub contribution: f64,
}

#[derive(Debug, Clone)]
pub struct CandidateScoreBreakdown {
    pub candidate_user_id: Uuid,
    pub final_score: f64,
    pub used_weight: f64,
    pub axis_scores: Vec<AxisScore>,
}

struct WeightedAxis {
    axis: Axis,
    weight: f64,
}

pub fn score_candidate(person: &Candidate, candidate: &Candidate) -> CandidateScoreBreakdown {
    let mut axis_scores = Vec::new();
    let mut weighted_score_sum = 0.0;
    let mut used_weight_sum = 0.0;

    if let Some(score) = attachment_score(person, candidate) {
        add_axis_score(
            &mut axis_scores,
            &mut weighted_score_sum,
            &mut used_weight_sum,
            "Attachment",
            score,
            ATTACHMENT_WEIGHT,
        );
    }

    for weighted_axis in cosine_axis_weights() {
        let axis_name = weighted_axis.axis.axis_to_text().to_string();
        let maybe_score = cosine_axis_score(person, candidate, weighted_axis.axis);

        if let Some(score) = maybe_score {
            add_axis_score(
                &mut axis_scores,
                &mut weighted_score_sum,
                &mut used_weight_sum,
                &axis_name,
                score,
                weighted_axis.weight,
            );
        }
    }

    let mut final_score = 0.0;
    if used_weight_sum > 0.0 {
        final_score = weighted_score_sum / used_weight_sum;
    }

    CandidateScoreBreakdown {
        candidate_user_id: candidate.user_id,
        final_score,
        used_weight: used_weight_sum,
        axis_scores,
    }
}

pub fn candidate_passes_threshold(breakdown: &CandidateScoreBreakdown) -> bool {
    breakdown.final_score >= GLOBAL_MATCH_THRESHOLD
}

pub fn cosine_axis_score(person: &Candidate, candidate: &Candidate, axis: Axis) -> Option<f64> {
    let person_vector = find_user_vector(person, &axis)?;
    let candidate_vector = find_user_vector(candidate, &axis)?;

    let cosine = cosine_similarity(person_vector, candidate_vector) as f64;

    Some(normalize_cosine_score(cosine))
}

pub fn normalize_cosine_score(cosine: f64) -> f64 {
    ((cosine + 1.0) / 2.0).clamp(0.0, 1.0)
}

pub fn log_score_breakdown(breakdown: &CandidateScoreBreakdown) {
    let mut attachment_text = "missing".to_string();
    for axis_score in &breakdown.axis_scores {
        if axis_score.axis_name == "Attachment" {
            attachment_text = format!("{:.3}", axis_score.score);
            break;
        }
    }

    let mut top_contributions = breakdown.axis_scores.clone();
    top_contributions.sort_by(|left, right| compare_desc(left.contribution, right.contribution));

    let mut weakest_axes = breakdown.axis_scores.clone();
    weakest_axes.sort_by(|left, right| compare_asc(left.score, right.score));

    eprintln!(
        "matching score candidate={} final={:.3} used_weight={:.2} attachment={} top=[{}] weakest=[{}]",
        breakdown.candidate_user_id,
        breakdown.final_score,
        breakdown.used_weight,
        attachment_text,
        format_axis_scores(&top_contributions, 3),
        format_axis_scores(&weakest_axes, 3),
    );
}

fn add_axis_score(
    axis_scores: &mut Vec<AxisScore>,
    weighted_score_sum: &mut f64,
    used_weight_sum: &mut f64,
    axis_name: &str,
    score: f64,
    weight: f64,
) {
    let contribution = score * weight;

    *weighted_score_sum += contribution;
    *used_weight_sum += weight;

    axis_scores.push(AxisScore {
        axis_name: axis_name.to_string(),
        score,
        weight,
        contribution,
    });
}

fn find_user_vector<'a>(candidate: &'a Candidate, axis: &Axis) -> Option<&'a Vector> {
    for vector in &candidate.vectors {
        if vector.axis == *axis {
            return Some(&vector.embedding);
        }
    }

    None
}

fn cosine_axis_weights() -> [WeightedAxis; 14] {
    [
        WeightedAxis {
            axis: Axis::FutureFocus,
            weight: FUTURE_FOCUS_WEIGHT,
        },
        WeightedAxis {
            axis: Axis::MoralFraming,
            weight: MORAL_FRAMING_WEIGHT,
        },
        WeightedAxis {
            axis: Axis::SocialValues,
            weight: SOCIAL_VALUES_WEIGHT,
        },
        WeightedAxis {
            axis: Axis::CommunicationStyle,
            weight: COMMUNICATION_STYLE_WEIGHT,
        },
        WeightedAxis {
            axis: Axis::ThinkingMode,
            weight: THINKING_MODE_WEIGHT,
        },
        WeightedAxis {
            axis: Axis::RelationshipPace,
            weight: RELATIONSHIP_PACE_WEIGHT,
        },
        WeightedAxis {
            axis: Axis::ConflictApproach,
            weight: CONFLICT_APPROACH_WEIGHT,
        },
        WeightedAxis {
            axis: Axis::RepairStrategy,
            weight: REPAIR_STRATEGY_WEIGHT,
        },
        WeightedAxis {
            axis: Axis::Interdependence,
            weight: INTERDEPENDENCE_WEIGHT,
        },
        WeightedAxis {
            axis: Axis::ChangeOrientation,
            weight: CHANGE_ORIENTATION_WEIGHT,
        },
        WeightedAxis {
            axis: Axis::SocialEnergy,
            weight: SOCIAL_ENERGY_WEIGHT,
        },
        WeightedAxis {
            axis: Axis::RoutinePreference,
            weight: ROUTINE_PREFERENCE_WEIGHT,
        },
        WeightedAxis {
            axis: Axis::Playfulness,
            weight: PLAYFULNESS_WEIGHT,
        },
        WeightedAxis {
            axis: Axis::RiskTolerance,
            weight: RISK_TOLERANCE_WEIGHT,
        },
    ]
}

fn compare_desc(left: f64, right: f64) -> Ordering {
    right.partial_cmp(&left).unwrap_or(Ordering::Equal)
}

fn compare_asc(left: f64, right: f64) -> Ordering {
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}

fn format_axis_scores(scores: &[AxisScore], limit: usize) -> String {
    let mut parts = Vec::new();

    for axis_score in scores.iter().take(limit) {
        parts.push(format!(
            "{} score={:.3} weight={:.2} contribution={:.3}",
            axis_score.axis_name, axis_score.score, axis_score.weight, axis_score.contribution
        ));
    }

    if parts.is_empty() {
        return "none".to_string();
    }

    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;
    use crate::models::user::UserComputedRow;
    use crate::models::{UserID, UserInfoVectors};

    #[test]
    fn cosine_normalization_maps_expected_values() {
        assert_eq!(normalize_cosine_score(1.0), 1.0);
        assert_eq!(normalize_cosine_score(0.0), 0.5);
        assert_eq!(normalize_cosine_score(-1.0), 0.0);
    }

    #[test]
    fn missing_axis_vectors_are_skipped() {
        let person = test_candidate(vec![(Axis::FutureFocus, vec![1.0, 0.0])], None, None);
        let candidate_without_vector = test_candidate(vec![], None, None);

        let missing_candidate_score =
            cosine_axis_score(&person, &candidate_without_vector, Axis::FutureFocus);
        assert!(missing_candidate_score.is_none());

        let person_without_vector = test_candidate(vec![], None, None);
        let candidate = test_candidate(vec![(Axis::FutureFocus, vec![1.0, 0.0])], None, None);

        let missing_person_score =
            cosine_axis_score(&person_without_vector, &candidate, Axis::FutureFocus);
        assert!(missing_person_score.is_none());
    }

    #[test]
    fn final_score_computes_when_one_axis_contributes() {
        let person = test_candidate(vec![(Axis::FutureFocus, vec![1.0, 0.0])], None, None);
        let candidate = test_candidate(vec![(Axis::FutureFocus, vec![1.0, 0.0])], None, None);

        let breakdown = score_candidate(&person, &candidate);

        assert_eq!(breakdown.used_weight, FUTURE_FOCUS_WEIGHT);
        assert_eq!(breakdown.final_score, 1.0);
    }

    #[test]
    fn low_attachment_score_contributes_without_hard_removing() {
        let person = test_candidate(
            vec![
                (Axis::FutureFocus, vec![1.0, 0.0]),
                (Axis::MoralFraming, vec![1.0, 0.0]),
            ],
            Some("low_avoidant"),
            Some("high_reassurance_need"),
        );
        let candidate = test_candidate(
            vec![
                (Axis::FutureFocus, vec![1.0, 0.0]),
                (Axis::MoralFraming, vec![1.0, 0.0]),
            ],
            Some("high_avoidant"),
            Some("low_reassurance_need"),
        );

        let breakdown = score_candidate(&person, &candidate);
        let attachment_axis = breakdown
            .axis_scores
            .iter()
            .find(|axis_score| axis_score.axis_name == "Attachment")
            .expect("attachment score should be present");

        assert!(attachment_axis.score < GLOBAL_MATCH_THRESHOLD);
        assert!(candidate_passes_threshold(&breakdown));

        let attachment_only_person =
            test_candidate(vec![], Some("low_avoidant"), Some("high_reassurance_need"));
        let attachment_only_candidate =
            test_candidate(vec![], Some("high_avoidant"), Some("low_reassurance_need"));

        let attachment_only_breakdown =
            score_candidate(&attachment_only_person, &attachment_only_candidate);
        assert!(!candidate_passes_threshold(&attachment_only_breakdown));
    }

    #[test]
    fn threshold_accepts_at_or_above_global_threshold() {
        let rejected = CandidateScoreBreakdown {
            candidate_user_id: Uuid::new_v4(),
            final_score: GLOBAL_MATCH_THRESHOLD - 0.001,
            used_weight: 1.0,
            axis_scores: Vec::new(),
        };
        assert!(!candidate_passes_threshold(&rejected));

        let accepted = CandidateScoreBreakdown {
            candidate_user_id: Uuid::new_v4(),
            final_score: GLOBAL_MATCH_THRESHOLD,
            used_weight: 1.0,
            axis_scores: Vec::new(),
        };
        assert!(candidate_passes_threshold(&accepted));
    }

    fn test_candidate(
        vectors: Vec<(Axis, Vec<f32>)>,
        closeness_type: Option<&str>,
        reassurance_type: Option<&str>,
    ) -> Candidate {
        let user_id = Uuid::new_v4();
        let mut user_vectors = Vec::new();

        for (axis, embedding) in vectors {
            user_vectors.push(UserInfoVectors {
                user_id: UserID(user_id),
                axis,
                embedding,
            });
        }

        Candidate {
            user_id,
            computed: UserComputedRow {
                user_id,
                computed_at: Utc::now(),
                attachment_closeness_type: closeness_type.map(|value| value.to_string()),
                attachment_closeness_margin: None,
                attachment_reassurance_type: reassurance_type.map(|value| value.to_string()),
                attachment_reassurance_margin: None,
                summary: None,
            },
            vectors: user_vectors,
            score: 0.0,
        }
    }
}

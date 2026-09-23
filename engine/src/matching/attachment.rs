use crate::models::Vector;
use crate::matching::vector_math::cosine_similarity;

/*
    This function is going to take in the user Vectors for the ComfortWithCloseness
    and the NeedForReassurance Axis, we will then compare them to the anchor vectors for each of those axis
    we can then decide the attachment type of that user.
*/

pub enum AttachmentStyle {
    DismissiveAvoidant,
    AvoidantLeaning,
    FearfulAvoidant,
    SecureIndependent,
    Secure,
    AnxiousLeaning,
    WarmSecure,
    SecureConnected,
    Anxious,
}

pub struct ComfortAnchors {
    pub low_avoidant: Vector,
    pub high_avoidant: Vector,
    pub neutral_avoidance: Vector,
}

pub struct ReassuranceAnchors {
    pub high_reassurance_need: Vector,
    pub low_reassurance_need: Vector,
    pub neutral_reassurance_need: Vector,
}

pub struct AttachmentComputation {
    pub style: AttachmentStyle,
    pub closeness_margin: f32,
    pub reassurance_margin: f32,
}

// Compare each attachment axis with its stored anchors and retain the top two
// similarities for classification and score margins.
pub fn compute_user_attachment(
    comfort_with_closeness: &Vector,
    need_for_reassurance: &Vector,
    comfort_anchors: &ComfortAnchors,
    reassurance_anchors: &ReassuranceAnchors,
) -> AttachmentComputation {
    let mut comfort_best = ("", f32::NEG_INFINITY);
    let mut comfort_second_best = ("", f32::NEG_INFINITY);
    let mut reassurance_best = ("", f32::NEG_INFINITY);
    let mut reassurance_second_best = ("", f32::NEG_INFINITY);

    for (name, anchor) in [
        ("low_avoidant", &comfort_anchors.low_avoidant),
        ("high_avoidant", &comfort_anchors.high_avoidant),
        ("neutral_avoidance", &comfort_anchors.neutral_avoidance),
    ] {
        // Similarity describes how closely the user's vector resembles this anchor.
        let sim = cosine_similarity(comfort_with_closeness, anchor);

        track_anchor_score((name, sim), &mut comfort_best, &mut comfort_second_best);
    }

    for (name, anchor) in [
        (
            "high_reassurance_need",
            &reassurance_anchors.high_reassurance_need,
        ),
        (
            "low_reassurance_need",
            &reassurance_anchors.low_reassurance_need,
        ),
        (
            "neutral_reassurance_need",
            &reassurance_anchors.neutral_reassurance_need,
        ),
    ] {
        let sim = cosine_similarity(need_for_reassurance, anchor);

        track_anchor_score(
            (name, sim),
            &mut reassurance_best,
            &mut reassurance_second_best,
        );
    }

    // Classify from the nearest anchor on each axis and retain the runner-up
    // margins so the confidence can be inspected separately.
    let style = compute_user_attachment_from_similarities(comfort_best, reassurance_best);

    AttachmentComputation {
        style,
        closeness_margin: score_margin(comfort_best, comfort_second_best),
        reassurance_margin: score_margin(reassurance_best, reassurance_second_best),
    }
}

fn track_anchor_score<'a>(
    candidate: (&'a str, f32),
    best: &mut (&'a str, f32),
    second_best: &mut (&'a str, f32),
) {
    if candidate.1 > best.1 {
        *second_best = *best;
        *best = candidate;
    } else if candidate.1 > second_best.1 {
        *second_best = candidate;
    }
}

fn score_margin(best: (&str, f32), second_best: (&str, f32)) -> f32 {
    if best.1.is_finite() && second_best.1.is_finite() {
        (best.1 - second_best.1).max(0.0)
    } else {
        0.0
    }
}

pub fn compute_user_attachment_from_similarities(
    comfort_best: (&str, f32),
    reassurance_best: (&str, f32),
) -> AttachmentStyle {
    let key = format!("{}:{}", comfort_best.0, reassurance_best.0);

    match key.as_str() {
        "high_avoidant:low_reassurance_need" => AttachmentStyle::DismissiveAvoidant,
        "high_avoidant:neutral_reassurance_need" => AttachmentStyle::AvoidantLeaning,
        "high_avoidant:high_reassurance_need" => AttachmentStyle::FearfulAvoidant,
        "neutral_avoidance:low_reassurance_need" => AttachmentStyle::SecureIndependent,
        "neutral_avoidance:neutral_reassurance_need" => AttachmentStyle::Secure,
        "neutral_avoidance:high_reassurance_need" => AttachmentStyle::AnxiousLeaning,
        "low_avoidant:low_reassurance_need" => AttachmentStyle::WarmSecure,
        "low_avoidant:neutral_reassurance_need" => AttachmentStyle::SecureConnected,
        "low_avoidant:high_reassurance_need" => AttachmentStyle::Anxious,
        _ => AttachmentStyle::Secure, // fallback
    }
}

/// Compatibility score between two attachment styles.
/// Returns a value from 0.0 (terrible pairing) to 1.0 (ideal pairing).
///
/// This models attachment styles as levels of avoidance, anxiety/reassurance
/// need, and earned security instead of using a fixed lookup table. That makes
/// secure partners stabilising without making them magically compatible with
/// every insecure style, and it strongly penalises anxious-avoidant traps.
pub fn attachment_compatibility(a: &AttachmentStyle, b: &AttachmentStyle) -> f32 {
    use AttachmentStyle::*;

    #[derive(Clone, Copy)]
    struct AttachmentProfile {
        avoidance: f32,
        anxiety: f32,
        security: f32,
        disorganization: f32,
    }

    fn profile(s: &AttachmentStyle) -> AttachmentProfile {
        match s {
            DismissiveAvoidant => AttachmentProfile {
                avoidance: 0.95,
                anxiety: 0.15,
                security: 0.25,
                disorganization: 0.0,
            },
            AvoidantLeaning => AttachmentProfile {
                avoidance: 0.65,
                anxiety: 0.25,
                security: 0.55,
                disorganization: 0.0,
            },
            FearfulAvoidant => AttachmentProfile {
                avoidance: 0.85,
                anxiety: 0.85,
                security: 0.20,
                disorganization: 0.90,
            },
            SecureIndependent => AttachmentProfile {
                avoidance: 0.35,
                anxiety: 0.15,
                security: 0.85,
                disorganization: 0.0,
            },
            Secure => AttachmentProfile {
                avoidance: 0.20,
                anxiety: 0.20,
                security: 1.0,
                disorganization: 0.0,
            },
            WarmSecure => AttachmentProfile {
                avoidance: 0.10,
                anxiety: 0.25,
                security: 0.92,
                disorganization: 0.0,
            },
            SecureConnected => AttachmentProfile {
                avoidance: 0.15,
                anxiety: 0.40,
                security: 0.86,
                disorganization: 0.0,
            },
            AnxiousLeaning => AttachmentProfile {
                avoidance: 0.25,
                anxiety: 0.65,
                security: 0.58,
                disorganization: 0.0,
            },
            Anxious => AttachmentProfile {
                avoidance: 0.15,
                anxiety: 0.95,
                security: 0.30,
                disorganization: 0.0,
            },
        }
    }

    let a = profile(a);
    let b = profile(b);

    let avoidance_gap = (a.avoidance - b.avoidance).abs();
    let anxiety_gap = (a.anxiety - b.anxiety).abs();
    let average_security = (a.security + b.security) / 2.0;
    let secure_buffer = a.security.max(b.security);
    let both_insecure = (1.0 - a.security) * (1.0 - b.security);

    // High avoidance in one person plus high anxiety in the other is the classic
    // pursue-withdraw trap. This is symmetric across the pair.
    let anxious_avoidant_trap = (a.avoidance * b.anxiety + b.avoidance * a.anxiety) / 2.0;

    let score = 0.72 + 0.18 * average_security + 0.10 * secure_buffer
        - 0.16 * avoidance_gap
        - 0.14 * anxiety_gap
        - 0.22 * anxious_avoidant_trap
        - 0.10 * both_insecure
        - 0.16 * a.disorganization.max(b.disorganization);

    score.clamp(0.0, 1.0)
}

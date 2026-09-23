use std::collections::HashMap;

use sqlx::PgPool;

use crate::db::{
    anchor_types::get_anchor_types_by_axis, users::upsert_user_computed_attachment,
};
use crate::matching::attachment::{
    AttachmentStyle, ComfortAnchors, ReassuranceAnchors, compute_user_attachment,
};
use crate::matching::candidate::Candidate;
use crate::models::Axis;

/// Ensure a candidate has attachment fields in their computed row.
/// If the fields are missing, compute them, persist them, and update the in-memory candidate.
pub async fn ensure_user_has_attachment(
    pool: &PgPool,
    candidate: &mut Candidate,
    comfort_anchors: &ComfortAnchors,
    reassurance_anchors: &ReassuranceAnchors,
) -> Result<(), Box<dyn std::error::Error>> {
    if candidate.computed.attachment_closeness_type.is_some()
        && candidate.computed.attachment_reassurance_type.is_some()
    {
        return Ok(());
    }

    let comfort_vec = candidate
        .vectors
        .iter()
        .find(|v| v.axis == Axis::ComfortWithCloseness)
        .map(|v| &v.embedding)
        .ok_or("user missing ComfortWithCloseness vector")?;

    let reassurance_vec = candidate
        .vectors
        .iter()
        .find(|v| v.axis == Axis::NeedForReassurance)
        .map(|v| &v.embedding)
        .ok_or("user missing NeedForReassurance vector")?;

    let attachment = compute_user_attachment(
        comfort_vec,
        reassurance_vec,
        comfort_anchors,
        reassurance_anchors,
    );
    let (closeness_type, reassurance_type) = attachment_style_to_axis_types(&attachment.style);
    let summary = attachment_style_summary(&attachment.style);

    candidate.computed = upsert_user_computed_attachment(
        pool,
        candidate.user_id,
        closeness_type,
        attachment.closeness_margin,
        reassurance_type,
        attachment.reassurance_margin,
        summary,
    )
    .await?;

    Ok(())
}

/// Try to ensure attachment for the person and candidates.
/// Missing source data is allowed; later scoring will skip the attachment axis for that pair.
pub async fn ensure_attachment_for_candidates(
    pool: &PgPool,
    person: &mut Candidate,
    potential_matches: &mut Vec<Candidate>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (comfort_anchors, reassurance_anchors) = get_attachment_anchors(pool).await?;

    if let Err(e) =
        ensure_user_has_attachment(pool, person, &comfort_anchors, &reassurance_anchors).await
    {
        eprintln!(
            "attachment ensure skipped for person {}: {}",
            person.user_id, e
        );
    }

    for candidate in potential_matches {
        if let Err(e) =
            ensure_user_has_attachment(pool, candidate, &comfort_anchors, &reassurance_anchors)
                .await
        {
            eprintln!(
                "attachment ensure skipped for candidate {}: {}",
                candidate.user_id, e
            );
        }
    }

    Ok(())
}

async fn get_attachment_anchors(
    pool: &PgPool,
) -> Result<(ComfortAnchors, ReassuranceAnchors), Box<dyn std::error::Error>> {
    let comfort_rows = get_anchor_types_by_axis(pool, &Axis::ComfortWithCloseness).await?;
    let reassurance_rows = get_anchor_types_by_axis(pool, &Axis::NeedForReassurance).await?;

    let mut comfort_by_type = comfort_rows
        .into_iter()
        .map(|row| (row.type_name, row.vector))
        .collect::<HashMap<_, _>>();
    let mut reassurance_by_type = reassurance_rows
        .into_iter()
        .map(|row| (row.type_name, row.vector))
        .collect::<HashMap<_, _>>();

    let comfort_anchors = ComfortAnchors {
        low_avoidant: take_anchor(&mut comfort_by_type, "low_avoidant")?,
        high_avoidant: take_anchor(&mut comfort_by_type, "high_avoidant")?,
        neutral_avoidance: take_anchor(&mut comfort_by_type, "neutral_avoidant")?,
    };
    let reassurance_anchors = ReassuranceAnchors {
        high_reassurance_need: take_anchor(&mut reassurance_by_type, "high_reassurance_need")?,
        low_reassurance_need: take_anchor(&mut reassurance_by_type, "low_reassurance_need")?,
        neutral_reassurance_need: take_anchor(
            &mut reassurance_by_type,
            "neutral_reassurance_need",
        )?,
    };

    Ok((comfort_anchors, reassurance_anchors))
}

fn take_anchor(
    anchors_by_type: &mut HashMap<String, Vec<f32>>,
    type_name: &str,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    anchors_by_type.remove(type_name).ok_or_else(|| {
        Box::<dyn std::error::Error>::from(format!("missing attachment anchor type {}", type_name))
    })
}

/// Map a computed AttachmentStyle back to the (closeness_type, reassurance_type) strings
/// stored in user_computed.
fn attachment_style_to_axis_types(style: &AttachmentStyle) -> (&'static str, &'static str) {
    match style {
        AttachmentStyle::DismissiveAvoidant => ("high_avoidant", "low_reassurance_need"),
        AttachmentStyle::AvoidantLeaning => ("high_avoidant", "neutral_reassurance_need"),
        AttachmentStyle::FearfulAvoidant => ("high_avoidant", "high_reassurance_need"),
        AttachmentStyle::SecureIndependent => ("neutral_avoidance", "low_reassurance_need"),
        AttachmentStyle::Secure => ("neutral_avoidance", "neutral_reassurance_need"),
        AttachmentStyle::AnxiousLeaning => ("neutral_avoidance", "high_reassurance_need"),
        AttachmentStyle::WarmSecure => ("low_avoidant", "low_reassurance_need"),
        AttachmentStyle::SecureConnected => ("low_avoidant", "neutral_reassurance_need"),
        AttachmentStyle::Anxious => ("low_avoidant", "high_reassurance_need"),
    }
}

fn attachment_style_summary(style: &AttachmentStyle) -> &'static str {
    match style {
        AttachmentStyle::DismissiveAvoidant => "DismissiveAvoidant",
        AttachmentStyle::AvoidantLeaning => "AvoidantLeaning",
        AttachmentStyle::FearfulAvoidant => "FearfulAvoidant",
        AttachmentStyle::SecureIndependent => "SecureIndependent",
        AttachmentStyle::Secure => "Secure",
        AttachmentStyle::AnxiousLeaning => "AnxiousLeaning",
        AttachmentStyle::WarmSecure => "WarmSecure",
        AttachmentStyle::SecureConnected => "SecureConnected",
        AttachmentStyle::Anxious => "Anxious",
    }
}

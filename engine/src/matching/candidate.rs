use uuid::Uuid;

use crate::matching::attachment::{
    AttachmentStyle, attachment_compatibility, compute_user_attachment_from_similarities,
};
use crate::models::UserInfoVectors;
use crate::models::user::UserComputedRow;

/// A user bundled with their already-loaded rows, ready for attachment matching.
pub struct Candidate {
    pub user_id: Uuid,
    pub computed: UserComputedRow,
    pub vectors: Vec<UserInfoVectors>,
    pub score: f64,
}

/// Score attachment compatibility without filtering the candidate list.
/// Missing attachment fields mean this axis cannot contribute for the pair.
pub fn attachment_score(person: &Candidate, candidate: &Candidate) -> Option<f64> {
    let person_style = row_to_attachment_style(&person.computed).ok()?;
    let candidate_style = row_to_attachment_style(&candidate.computed).ok()?;

    Some(attachment_compatibility(&person_style, &candidate_style) as f64)
}

fn row_to_attachment_style(
    row: &UserComputedRow,
) -> Result<AttachmentStyle, Box<dyn std::error::Error>> {
    let closeness = row
        .attachment_closeness_type
        .as_deref()
        .ok_or("missing attachment_closeness_type")?;
    let reassurance = row
        .attachment_reassurance_type
        .as_deref()
        .ok_or("missing attachment_reassurance_type")?;

    Ok(compute_user_attachment_from_similarities(
        (closeness, 0.0),
        (reassurance, 0.0),
    ))
}

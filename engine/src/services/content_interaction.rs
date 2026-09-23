use sqlx::PgPool;

use crate::db::{embedded_content::get_embedded_content, ledger::add_to_ledger};
use crate::models::UserInteraction;

/// Resolve the embedded prompt and persist the user's reaction in the ledger.
pub async fn content_interaction(
    interaction: UserInteraction,
    pool: &PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let embedded_data = match get_embedded_content(pool, &interaction.id).await {
        Ok(data) => data,
        Err(e) => return Err(Box::new(e)),
    };

    match add_to_ledger(pool, embedded_data, interaction).await {
        Ok(_) => (),
        Err(e) => return Err(Box::new(e)),
    }

    Ok(())
}

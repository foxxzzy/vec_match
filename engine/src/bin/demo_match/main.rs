mod fixtures;
mod report;
mod runner;
mod scenarios;

use anyhow::{bail, Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    require_local_demo_database()?;
    runner::run_demo().await
}

// The demo deletes and recreates synthetic auth users. Refuse URLs that do not
// point at the default local Supabase database before opening a connection.
fn require_local_demo_database() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    let url = reqwest::Url::parse(&database_url).context("DATABASE_URL must be a URL")?;
    let local_host = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"));

    if !matches!(url.scheme(), "postgres" | "postgresql")
        || !local_host
        || url.port() != Some(54322)
    {
        bail!("demo_match requires the default local Supabase database at localhost:54322");
    }

    Ok(())
}

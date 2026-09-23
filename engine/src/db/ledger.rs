use sqlx::{PgPool, Postgres, Row, Transaction};

use crate::models::{
    Axis, EmbeddingContent, LedgerEntry, LedgerEntryDb, PromptId, Reaction, UserID,
    UserInteraction, Vector,
};

pub async fn add_to_ledger(
    pool: &PgPool,
    embedding: EmbeddingContent,
    interaction: UserInteraction,
) -> Result<(), sqlx::Error> {
    let mut tx: Transaction<Postgres> = pool.begin().await?;

    let entry = LedgerEntry {
        user_id: interaction.user_id,
        prompt_id: embedding.id,
        axis: embedding.axis,
        embedding: embedding.embedding,
        reaction: interaction.reaction,
        weight: embedding.weight,
        anchor: embedding.anchor,
    };

    sqlx::query(
        r#"INSERT INTO ledger_entries (user_id, prompt_id, reaction, embedding, weight, axis, anchor)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           ON CONFLICT (user_id, prompt_id)
           DO UPDATE SET reaction = EXCLUDED.reaction,
                         embedding = EXCLUDED.embedding,
                         weight = EXCLUDED.weight"#,
    )
    .bind(entry.user_id.0)
    .bind(entry.prompt_id.0)
    .bind(reaction_to_int(entry.reaction))
    .bind(entry.embedding.clone())
    .bind(entry.weight)
    .bind(entry.axis.axis_to_text())
    .bind(entry.anchor)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn get_all_user_ledger_entries(
    pool: &PgPool,
    user_id: UserID,
) -> Result<Vec<LedgerEntryDb>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT 
            id,
            created_at,
            user_id,
            prompt_id,
            reaction,
            embedding,
            weight,
            axis,
            anchor
         FROM ledger_entries
         WHERE user_id = $1
         ORDER BY created_at ASC"#,
    )
    .bind(user_id.0)
    .fetch_all(pool)
    .await?;

    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let entry = map_row_to_ledger_entry_db(&row)?;
        entries.push(entry);
    }

    Ok(entries)
}

pub async fn get_ledger_entries_per_axis(
    pool: &PgPool,
    user_id: UserID,
    axis: Axis,
) -> Result<Vec<LedgerEntryDb>, sqlx::Error> {
    let string_axis = axis.axis_to_text();

    let rows = sqlx::query(
        r#"SELECT 
            id,
            created_at,
            user_id,
            prompt_id,
            reaction,
            embedding,
            weight,
            axis,
            anchor
         FROM ledger_entries
         WHERE user_id = $1
           AND axis = $2
         ORDER BY created_at ASC"#,
    )
    .bind(user_id.0)
    .bind(string_axis)
    .fetch_all(pool)
    .await?;

    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let entry = map_row_to_ledger_entry_db(&row)?;
        entries.push(entry);
    }

    Ok(entries)
}

fn map_row_to_ledger_entry_db(row: &sqlx::postgres::PgRow) -> Result<LedgerEntryDb, sqlx::Error> {
    let id: i64 = row.try_get("id")?;
    let created_at = row.try_get("created_at")?;
    let user_uuid = row.try_get("user_id")?;
    let prompt_uuid = row.try_get("prompt_id")?;
    let reaction_val: i16 = row.try_get("reaction")?;
    let embedding: Vector = row.try_get("embedding")?;
    let weight: f32 = row.try_get("weight")?;
    let axis_str: String = row.try_get("axis")?;
    let anchor: bool = row.try_get("anchor")?;

    let reaction = match reaction_val {
        1 => Reaction::Me,
        -1 => Reaction::NotMe,
        0 => Reaction::Skip,
        other => {
            return Err(sqlx::Error::Protocol(
                format!("unknown reaction value: {}", other).into(),
            ));
        }
    };

    let axis = Axis::from_text(axis_str.as_str()).map_err(|err| sqlx::Error::ColumnDecode {
        index: "axis".into(),
        source: Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
    })?;

    Ok(LedgerEntryDb {
        id,
        created_at,
        user_id: UserID(user_uuid),
        prompt_id: PromptId(prompt_uuid),
        reaction,
        embedding,
        weight,
        axis,
        anchor,
    })
}

fn reaction_to_int(reaction: Reaction) -> i16 {
    match reaction {
        Reaction::Me => 1,
        Reaction::NotMe => -1,
        Reaction::Skip => 0,
    }
}

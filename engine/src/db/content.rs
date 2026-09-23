use crate::models::{Axis, DisplayContent, EmbeddingContent};
use sqlx::{PgPool, Postgres, Transaction};

fn axis_to_text(axis: &Axis) -> &'static str {
    use Axis::*;
    match axis {
        ComfortWithCloseness => "ComfortWithCloseness",
        NeedForReassurance => "NeedForReassurance",
        ConflictApproach => "ConflictApproach",
        RepairStrategy => "RepairStrategy",
        FutureFocus => "FutureFocus",
        RelationshipPace => "RelationshipPace",
        MoralFraming => "MoralFraming",
        SocialValues => "SocialValues",
        Interdependence => "Interdependence",
        CommunicationStyle => "CommunicationStyle",
        ThinkingMode => "ThinkingMode",
        SocialEnergy => "SocialEnergy",
        RoutinePreference => "RoutinePreference",
        RiskTolerance => "RiskTolerance",
        ChangeOrientation => "ChangeOrientation",
        Playfulness => "Playfulness",
    }
}

pub async fn save_content_and_prompt(
    pool: &PgPool,
    display: &DisplayContent,
    embed: &EmbeddingContent,
) -> Result<(), sqlx::Error> {
    let mut tx: Transaction<Postgres> = pool.begin().await?;

    let content_insert_res = sqlx::query(
        r#"
        INSERT INTO content_who_are_they (prompt_id, axis, content, anchor, details)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (prompt_id) DO UPDATE
        SET axis = EXCLUDED.axis,
            content = EXCLUDED.content,
            anchor = EXCLUDED.anchor,
            details = EXCLUDED.details
        "#,
    )
    .bind(display.id.0)
    .bind(axis_to_text(&display.axis))
    .bind(display.content.clone())
    .bind(display.anchor)
    .bind(display.details.clone())
    .execute(&mut *tx)
    .await;

    if let Err(e) = content_insert_res {
        tx.rollback().await.ok();
        return Err(e);
    }

    let prompts_insert_res = sqlx::query(
        r#"
        INSERT INTO embedded_content_who_are_they (prompt_id, axis, embedding, weight, anchor)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (prompt_id) DO UPDATE
        SET axis = EXCLUDED.axis,
            embedding = EXCLUDED.embedding,
            weight = EXCLUDED.weight,
            anchor = EXCLUDED.anchor
        "#,
    )
    .bind(display.id.0)
    .bind(axis_to_text(&display.axis))
    .bind(embed.embedding.clone())
    .bind(embed.weight)
    .bind(embed.anchor)
    .execute(&mut *tx)
    .await;

    if let Err(e) = prompts_insert_res {
        tx.rollback().await.ok();
        return Err(e);
    }

    tx.commit().await
}

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

use matching_engine_demo::{
    db,
    db::{
        hard_gated_candidates::{mark_candidates_consumed, refill_hard_gated_candidates},
        ledger::get_all_user_ledger_entries,
        serious_candidates::upsert_serious_candidates,
        user_info_vectors::insert_user_info_vectors,
        users::get_candidate,
    },
    matching::{
        attachment_state::ensure_attachment_for_candidates,
        scoring::{candidate_passes_threshold, score_candidate, GLOBAL_MATCH_THRESHOLD},
        user_vectors::compute_final_vectors_from_ledger,
    },
    models::{
        serious_candidate::SeriousCandidate, Axis, LedgerEntryDb, PromptId, Reaction, UserID,
        UserInteraction,
    },
    services::{
        content_interaction::content_interaction,
        matching_workflow::{record_queue_decision, submit_final_decision},
        queue_release::release_serious_candidates_to_queue,
    },
};

use crate::{
    fixtures::{
        apply_gate_case, base_pair, check_static_data, create_pair, load_available_axes,
        load_lookups, reset_demo_users, scenario_ids, Lookups,
    },
    report::{
        pair_name, print_engine_result, print_header, print_scenario_header, print_summary,
        write_engine_scenario, write_hard_gate_failure, write_report_file, EngineResult,
        ScenarioResult, REPORT_PATH,
    },
    scenarios::{
        reaction_for_case, scenarios, ExpectedStage, PersonalityCase, ScenarioSpec, WorkflowCase,
    },
};

pub(crate) async fn run_demo() -> Result<()> {
    print_header();

    let pool = db::init_pool()
        .await
        .context("failed to connect to DATABASE_URL")?;

    check_static_data(&pool).await?;
    let lookups = load_lookups(&pool).await?;
    let axes = load_available_axes(&pool).await?;

    println!("Local data ready");
    println!("  Embedded personality axes: {}", axes.len());
    println!(
        "  Global compatibility threshold: {:.0}%",
        GLOBAL_MATCH_THRESHOLD * 100.0
    );
    println!();

    let mut results = Vec::new();
    let mut detail_report = String::new();

    for spec in scenarios() {
        results.push(run_scenario(&pool, &lookups, &axes, &spec, &mut detail_report).await?);
    }

    write_report_file(axes.len(), &results, &detail_report)?;

    print_summary(&results);
    println!();
    println!("Full Markdown report written to {REPORT_PATH}");
    println!("Open that file for the clearest explanation of every scenario.");

    Ok(())
}

async fn run_scenario(
    pool: &PgPool,
    lookups: &Lookups,
    axes: &[Axis],
    spec: &ScenarioSpec,
    report: &mut String,
) -> Result<ScenarioResult> {
    print_scenario_header(spec);
    reset_demo_users(pool).await?;

    let (a_id, b_id) = scenario_ids(spec.number);
    let (mut a, mut b) = base_pair(lookups, spec.a_name, spec.b_name);
    apply_gate_case(spec.gate_case, lookups, &mut a, &mut b);
    create_pair(pool, a_id, &a, b_id, &b, spec.number).await?;

    let hard_gate = run_hard_gate(pool, a_id, b_id).await?;
    let hard_gate_passed = hard_gate.0 && hard_gate.1;

    if spec.expected == ExpectedStage::HardGateFail {
        if hard_gate_passed {
            bail!("scenario {} expected a hard-gate failure, but the pair passed", spec.number);
        }

        println!("DATABASE HARD GATE: FAIL");
        println!("Why: {}", spec.explanation);
        println!("Personality scoring: SKIPPED");
        println!(
            "This is intentional. The database rejected the pair before expensive vector work."
        );
        println!();

        write_hard_gate_failure(report, spec)?;

        return Ok(ScenarioResult {
            number: spec.number,
            pair: pair_name(spec),
            hard_gate: "FAIL".to_string(),
            compatibility: "Not run".to_string(),
            outcome: "Rejected before vector scoring".to_string(),
        });
    }

    if !hard_gate_passed {
        bail!("scenario {} expected the hard gate to pass, but it failed", spec.number);
    }

    println!("DATABASE HARD GATE: PASS");
    println!("Both directions satisfy the profile and preference rules.");
    println!();

    let prompt_count = seed_personality_interactions(
        pool,
        a_id,
        b_id,
        axes,
        spec.personality_case,
    )
    .await?;
    let a_vectors = compute_vectors_for_user(pool, a_id).await?;
    let b_vectors = compute_vectors_for_user(pool, b_id).await?;

    println!("PERSONALITY DATA");
    println!("  Prompts seeded per user: {prompt_count}");
    println!("  {} vectors created: {a_vectors}", a.name);
    println!("  {} vectors created: {b_vectors}", b.name);
    println!();

    let engine = score_pair(pool, a_id, b_id).await?;
    print_engine_result(&a.name, &b.name, &engine);

    let engine_passed = candidate_passes_threshold(&engine.a_to_b)
        && candidate_passes_threshold(&engine.b_to_a);

    match spec.expected {
        ExpectedStage::EngineFail if engine_passed => {
            bail!(
                "scenario {} expected the compatibility engine to fail, but it passed",
                spec.number
            )
        }
        ExpectedStage::EnginePass | ExpectedStage::MatchCreated | ExpectedStage::FinalRejected
            if !engine_passed =>
        {
            bail!(
                "scenario {} expected the compatibility engine to pass, but it failed",
                spec.number
            )
        }
        _ => {}
    }

    let workflow_outcome = match spec.workflow_case {
        WorkflowCase::None => {
            if engine_passed {
                "Passed compatibility engine".to_string()
            } else {
                "Rejected by compatibility engine".to_string()
            }
        }
        WorkflowCase::ConfirmMatch => {
            run_confirm_match_workflow(pool, a_id, &a.name, b_id, &b.name).await?
        }
        WorkflowCase::FinalReject => {
            run_final_reject_workflow(pool, a_id, &a.name, b_id, &b.name).await?
        }
    };

    println!("OUTCOME: {workflow_outcome}");
    println!();

    write_engine_scenario(report, spec, &a, &b, &engine, &workflow_outcome)?;

    Ok(ScenarioResult {
        number: spec.number,
        pair: pair_name(spec),
        hard_gate: "PASS".to_string(),
        compatibility: format!(
            "{:.1}% / {:.1}%",
            engine.a_to_b.final_score * 100.0,
            engine.b_to_a.final_score * 100.0
        ),
        outcome: workflow_outcome,
    })
}


async fn run_hard_gate(pool: &PgPool, a: Uuid, b: Uuid) -> Result<(bool, bool)> {
    refill_hard_gated_candidates(pool, a, 10).await?;
    refill_hard_gated_candidates(pool, b, 10).await?;

    let a_to_b = hard_gate_row_exists(pool, a, b).await?;
    let b_to_a = hard_gate_row_exists(pool, b, a).await?;
    Ok((a_to_b, b_to_a))
}

async fn hard_gate_row_exists(pool: &PgPool, user: Uuid, candidate: Uuid) -> Result<bool> {
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM public.hard_gated_candidates
            WHERE user_id = $1
              AND candidate_user_id = $2
              AND consumed = false
        )
        "#,
    )
    .bind(user)
    .bind(candidate)
    .fetch_one(pool)
    .await?;

    Ok(exists)
}

async fn seed_personality_interactions(
    pool: &PgPool,
    a: Uuid,
    b: Uuid,
    axes: &[Axis],
    personality_case: PersonalityCase,
) -> Result<usize> {
    if matches!(personality_case, PersonalityCase::None) {
        return Ok(0);
    }

    let mut total_prompts = 0usize;

    for axis in axes {
        let prompt_ids: Vec<Uuid> = sqlx::query_scalar(
            r#"
            SELECT prompt_id
            FROM public.embedded_content_who_are_they
            WHERE axis = $1
            ORDER BY prompt_id
            LIMIT 3
            "#,
        )
        .bind(axis.axis_to_text())
        .fetch_all(pool)
        .await?;

        let b_reaction = reaction_for_case(personality_case, axis);

        for prompt_id in prompt_ids {
            add_demo_interaction(pool, a, prompt_id, Reaction::Me).await?;
            add_demo_interaction(pool, b, prompt_id, b_reaction.clone()).await?;
            total_prompts += 1;
        }
    }

    Ok(total_prompts)
}

async fn add_demo_interaction(
    pool: &PgPool,
    user_id: Uuid,
    prompt_id: Uuid,
    reaction: Reaction,
) -> Result<()> {
    content_interaction(
        UserInteraction {
            user_id: UserID(user_id),
            id: PromptId(prompt_id),
            reaction,
        },
        pool,
    )
    .await
    .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    Ok(())
}


async fn compute_vectors_for_user(pool: &PgPool, user_id: Uuid) -> Result<usize> {
    let entries = get_all_user_ledger_entries(pool, UserID(user_id)).await?;
    let mut by_axis: HashMap<String, Vec<LedgerEntryDb>> = HashMap::new();

    for entry in entries {
        by_axis
            .entry(entry.axis.axis_to_text().to_string())
            .or_default()
            .push(entry);
    }

    let mut count = 0usize;
    for entries in by_axis.into_values() {
        let vector = compute_final_vectors_from_ledger(entries).map_err(anyhow::Error::msg)?;
        insert_user_info_vectors(pool, &vector).await?;
        count += 1;
    }

    Ok(count)
}

async fn score_pair(pool: &PgPool, a: Uuid, b: Uuid) -> Result<EngineResult> {
    let mut person = get_candidate(pool, a).await?;
    let candidate = get_candidate(pool, b).await?;
    let mut candidates = vec![candidate];

    let attachment_note =
        match ensure_attachment_for_candidates(pool, &mut person, &mut candidates).await {
            Ok(()) => None,
            Err(err) => Some(format!(
                "Attachment scoring was skipped because its source data was unavailable: {err}"
            )),
        };

    let candidate = candidates
        .pop()
        .context("candidate disappeared while preparing scoring")?;

    let a_to_b = score_candidate(&person, &candidate);
    let b_to_a = score_candidate(&candidate, &person);

    if candidate_passes_threshold(&a_to_b) {
        upsert_serious_candidate(pool, a, b, a_to_b.final_score).await?;
    }
    if candidate_passes_threshold(&b_to_a) {
        upsert_serious_candidate(pool, b, a, b_to_a.final_score).await?;
    }

    mark_candidates_consumed(pool, a, &[b]).await?;
    mark_candidates_consumed(pool, b, &[a]).await?;

    Ok(EngineResult {
        a_to_b,
        b_to_a,
        attachment_note,
    })
}

async fn upsert_serious_candidate(
    pool: &PgPool,
    user_id: Uuid,
    candidate_user_id: Uuid,
    score: f64,
) -> Result<()> {
    upsert_serious_candidates(
        pool,
        &[SeriousCandidate {
            user_id,
            candidate_user_id,
            position: 0,
            score,
            expires_at: None,
        }],
    )
    .await?;
    Ok(())
}

async fn run_confirm_match_workflow(
    pool: &PgPool,
    a: Uuid,
    a_name: &str,
    b: Uuid,
    b_name: &str,
) -> Result<String> {
    release_serious_candidates_to_queue(pool, a, 1).await?;
    release_serious_candidates_to_queue(pool, b, 1).await?;

    record_queue_decision(pool, a, b, "like")
        .await
        .map_err(|err| anyhow::anyhow!("{a_name} like failed: {err:?}"))?;
    let mutual = record_queue_decision(pool, b, a, "like")
        .await
        .map_err(|err| anyhow::anyhow!("{b_name} like failed: {err:?}"))?;

    println!("WORKFLOW");
    println!("  {a_name} likes {b_name}");
    println!("  {b_name} likes {a_name}");
    println!("  Pair state after mutual likes: {}", mutual.pair_state.state);

    submit_final_decision(pool, a, b, "yes")
        .await
        .map_err(|err| anyhow::anyhow!("{a_name} final decision failed: {err:?}"))?;
    let final_outcome = submit_final_decision(pool, b, a, "yes")
        .await
        .map_err(|err| anyhow::anyhow!("{b_name} final decision failed: {err:?}"))?;

    let match_row = final_outcome
        .match_row
        .context("both users confirmed, but no match row was created")?;

    println!("  {a_name} final decision: yes");
    println!("  {b_name} final decision: yes");
    println!("  Interested pair state: {}", final_outcome.interested_pair.state);
    println!("  Match row: {} <-> {}", match_row.user_low_id, match_row.user_high_id);
    println!();

    Ok("MATCH CREATED".to_string())
}

async fn run_final_reject_workflow(
    pool: &PgPool,
    a: Uuid,
    a_name: &str,
    b: Uuid,
    b_name: &str,
) -> Result<String> {
    release_serious_candidates_to_queue(pool, a, 1).await?;
    release_serious_candidates_to_queue(pool, b, 1).await?;

    record_queue_decision(pool, a, b, "like")
        .await
        .map_err(|err| anyhow::anyhow!("{a_name} like failed: {err:?}"))?;
    let mutual = record_queue_decision(pool, b, a, "like")
        .await
        .map_err(|err| anyhow::anyhow!("{b_name} like failed: {err:?}"))?;

    println!("WORKFLOW");
    println!("  Both users initially like each other");
    println!("  Pair state after mutual likes: {}", mutual.pair_state.state);

    submit_final_decision(pool, a, b, "yes")
        .await
        .map_err(|err| anyhow::anyhow!("{a_name} final decision failed: {err:?}"))?;
    let final_outcome = submit_final_decision(pool, b, a, "no")
        .await
        .map_err(|err| anyhow::anyhow!("{b_name} final decision failed: {err:?}"))?;

    if final_outcome.match_row.is_some() {
        bail!("final rejection unexpectedly created a match row");
    }

    println!("  {a_name} final decision: yes");
    println!("  {b_name} final decision: no");
    println!("  Interested pair state: {}", final_outcome.interested_pair.state);
    println!("  Match row created: no");
    println!();

    Ok("FINAL DECISION REJECTED".to_string())
}

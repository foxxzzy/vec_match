# Dating compatibility engine

## What VecMatch is

Most dating apps are good at filtering people by age, distance, appearance, and a handful of stated preferences. They are much worse at representing the things that often determine whether two people can actually build a relationship: how they handle conflict, how much reassurance they need, how directly they communicate, what closeness feels like to them, and whether they want their lives to become deeply intertwined.

VecMatch is a prototype compatibility engine built around that problem.

Instead of assigning someone a personality type from one questionnaire, it builds an evolving, multidimensional representation from their reactions to short statements such as:

> “Being comforted by my partner helps me calm down after conflict.”

> “I’d rather let things cool off than talk about an issue immediately.”

> “Doing what’s fair matters more to me than taking someone’s side.”

> “I’m comfortable relying on a partner during difficult times.”

> “I want a relationship where our lives are closely intertwined.”

> “Humour is one of the main ways I connect with people.”

A user can respond **Me**, **Not Me**, or **Skip**. Each reaction contributes to a broader picture across areas such as communication, conflict, attachment, reassurance, values, relationship pace, independence, routine, and playfulness.

The intention is to capture psychology as something messy and multidimensional. Someone might value emotional closeness while still needing space after conflict. They might want a serious long-term relationship but prefer it to develop slowly. They might be highly independent in everyday life while still needing explicit reassurance from a partner.

VecMatch does not force those answers into one simplistic personality label or one overall personality vector. It maintains a separate vector for each area so that agreement in one part of a relationship cannot hide a serious difference in another.

## How prompts become vectors

Every prompt is assigned to a personality axis and converted into an **embedding**.

An embedding is a high-dimensional vector representing the meaning of the prompt. Statements with related meanings should produce vectors pointing in similar directions, even when they use different wording.

For example, these prompts both express a preference for direct communication:

> “I usually say exactly what I mean rather than hinting at it.”

> “I get frustrated when people imply things instead of saying them clearly.”

Their embeddings should be closer to each other than to a prompt about an unrelated subject such as routine or social energy.

The prompt is embedded once and stored in PostgreSQL. The matching process therefore does not repeatedly call an AI model, and no language model is asked to decide whether two people should date.

When a user reacts, the prototype treats the response as a direction:

```text
Me      -> use the prompt embedding
Not Me  -> use the embedding in the opposite direction
Skip    -> record the interaction, but contribute no movement
```

That contribution is also adjusted by the prompt’s importance and by time decay:

```text
contribution =
    prompt embedding
    × reaction direction
    × prompt weight
    × time decay
```

The interaction is stored in a ledger rather than immediately disappearing into a final score. This preserves the information needed to rebuild a user’s vectors if they change a reaction or if the weighting and decay rules are updated.

Interactions are grouped by personality axis. Their weighted vectors are added together and then normalised:

```text
axis vector = normalise(sum of interaction contributions)
```

This produces a collection of vectors describing the user across different areas of compatibility.

The approach allows:

- Semantically similar reactions to reinforce one another.
- Contradictory reactions to pull an axis in competing directions.
- More important prompts to have greater influence.
- Older interactions to gradually matter less.
- Different parts of someone’s personality to remain separate.
- A user’s representation to evolve instead of being permanently fixed after one questionnaire.

This is still a prototype assumption rather than a validated psychological model. In particular, treating **Not Me** as the opposite vector direction is an engineering experiment that would need testing against real user data.

## How two users are compared

Before any vector comparison takes place, PostgreSQL applies the users’ explicit profile and preference constraints in both directions.

These include:

- Age.
- Distance.
- Gender preferences.
- Height preferences.
- Smoking preferences.
- Religion requirements.
- Relationship intentions.

These are treated as hard boundaries. A high personality score cannot compensate for a pairing that one of the users has explicitly ruled out.

For pairs that pass those checks, VecMatch compares the corresponding axis vectors using cosine similarity. Cosine similarity measures whether two vectors point in similar directions rather than whether one user has answered more prompts than the other.

The raw cosine result ranges from `-1` to `1` and is converted into a compatibility score between `0` and `1`:

```text
axis score = (cosine similarity + 1) / 2
```

A value near `1` means the users’ vectors are strongly aligned on that axis. A value near `0` means they point in opposing directions.

Each axis has an importance weight. The final compatibility result is a weighted average of the axes for which both users have enough data:

```text
final score =
    sum(axis score × axis importance)
    / sum(active axis importance)
```

Missing data is not treated as incompatibility. If an axis cannot be scored, it is excluded from both sides of the calculation rather than silently contributing a zero.

The current prototype accepts candidates scoring at or above `60%`. The individual axis scores remain available so the result can be inspected instead of being presented as an unexplained match percentage.

## Attachment is treated differently

Compatibility is not always the same as similarity.

Most axes compare whether two users point in a similar direction. Attachment-related needs are more relational, so VecMatch handles them separately.

The user vectors for **Comfort With Closeness** and **Need For Reassurance** are compared with stored anchor vectors. Those anchors represent patterns such as:

- Low, neutral, or high avoidance of closeness.
- Low, neutral, or high need for reassurance.

The nearest anchor on each axis produces a prototype attachment category. VecMatch then scores the interaction between the two users’ categories rather than assuming that identical attachment patterns are always best.

This is intended to identify dynamics such as one person repeatedly seeking reassurance while the other withdraws when they feel pressured.

For example, consider a pair where:

- One person wants to resolve disagreements immediately.
- The other needs time alone before they can talk productively.
- One interprets distance as a sign that the relationship is in danger.
- The other experiences repeated reassurance requests as pressure.

Neither person is inherently wrong. They may agree strongly on values, humour, lifestyle, and their future, but the interaction between their conflict and reassurance needs could still create predictable friction. VecMatch attempts to represent that interaction rather than flattening everything into “similar” or “different”.

## From compatibility score to match

Passing the compatibility threshold does not create a match.

Candidates move through a staged process:

1. Explicit preferences are checked in both directions.
2. Available personality axes are scored.
3. Passing candidates are ranked by compatibility.
4. Ranked candidates are released into each user’s queue.
5. Both users must independently express interest.
6. Any required questions must be answered.
7. Both users make a final decision.
8. A confirmed match is created only if both final decisions are **yes**.

The engine can decide which pairs appear worth introducing, but attraction, chemistry, circumstances, and the final decision remain human.

The goal is not to predict love, diagnose users, or declare that two people are soulmates. It is to explore whether embeddings and interaction history can represent the messy parts of compatibility more effectively while keeping the resulting score understandable and inspectable.

## Disclaimer: 
This was built as PROTOTYPE to see if I could get it to work, none of it should be hosted in its current state their are security and privacy issues if you were to host this. This is just simply an idea / experiment of how embeddings could be used to match people.

The prompts and weighting will also need to be verified and checked in reality before we can say it is any good or not.

## AI use declaration

AI was used in this project to aid in some of the development for example, the boilerplate for the db endpoints, helping debug, some functions etc.
when AI was used it was used in file / function. I am not a fan of agents swarming the codebase, and all code was checked and verified by me.

AI also created the demo and helped to make my wording for the readme clearer and in formatting of it.

all architectural decisions were made by me.


## Repository map

| Path | Purpose |
| --- | --- |
| `src/api/` | Axum HTTP routes for profiles, interactions, vectors, queues, and match decisions. |
| `src/db/` | SQLx queries and persistence. |
| `src/matching/` | matching domain: vector and anchor construction, attachment classification, scoring, and candidate pipeline. |
| `src/models/` | Request, response, and database types. |
| `src/services/` | Application workflows for interactions, queue release, matching decisions, and optional embedding. |
| `src/services/embedding/` | Optional content embedding client for uploading new prompts. |
| `src/bin/demo_match/` | Local demo split into fixtures, scenarios, execution, and reporting. |
| `supabase/migrations/` | Database schema and hard gate function. |
| `supabase/seed.sql` | Static sample prompts, embeddings, anchors, and lookup values. No users or interactions. |
| `tests/api_endpoints.rs` | HTTP integration tests requiring a local database. |

## Run the local demo

Prerequisites: Rust toolchain supporting edition 2024, Node.js/npm, Docker, and the Supabase CLI installed from this project's `package-lock.json`. The Supabase config specifies PostgreSQL 17 and local port 54322. Use a **disposable local database**; `supabase db reset` clears it and the demo recreates synthetic auth users on each run.

```bash
cd engine
npm ci
npx supabase start
npx supabase db reset
cp .env.example .env
cargo run --bin demo_match
```

Open `DEMO_RESULTS.md` for the regenerated scenario report. The demo checks that `DATABASE_URL` targets `localhost:54322` or `127.0.0.1:54322` before it connects. Do not point that port at a production database.

The standalone local API can be started with `cargo run --bin matching-engine-demo`; it binds to `127.0.0.1:8080`. The optional content upload endpoint needs `OPEN_AI_API_EMBEDDING_API_KEY` in your uncommitted `.env`, whereas the scenario runner and existing seeded content do not.

## Checks

```bash
cargo fmt --check
cargo test --lib
cargo test --test api_endpoints  # requires the local seeded database
```

Some integration tests modify database rows. They run only if `RUN_DB_WRITE_TESTS=1` is set; use that flag exclusively against a disposable local database. A simple route construction test runs without PostgreSQL.

## Scope and security

- The HTTP endpoints currently accept user IDs supplied by callers; they do not authenticate a user or verify ownership of that ID. Bind only to localhost and **do not expose this API to the internet** as written.
- The migration is a snapshot of prototype database permissions. Several tables have broad grants and no row level policies. Do not apply it to a hosted Supabase project without a security review, restrictive policies, and a complete authorization design.
- Personal profile fields, location, relationship preferences, answers, and embeddings deserve careful privacy handling in a real app. The repository contains static content and synthetic sample outcomes, not a production privacy or deletion system.
- There are no deployment manifests, client application, load tests, or verified claims about match quality here. The thresholds and stored prompts are visible to anyone with repository access.

# Dating compatibility engine (Rust demo)

Disclaimer: This was built as PROTOTYPE to see if I could get it to work, none of it should be hosted in its current state their are secuirty issues if you were to host this. This is just simply an idea of how embeddings could be used to match people.

A Rust and PostgreSQL backend prototype for matching people using profile constraints and interaction based personality vectors. The repository includes a repeatable, local scenario runner that uses synthetic people to show when a pair is rejected, scored, queued, or confirmed as a match. [Sample results](DEMO_RESULTS.md) show ten scenarios and per axis score breakdowns.

This is a **portfolio prototype**. Its scoring weights, attachment categories, prompt embeddings, and 60% threshold are illustrative design choices, not validated measures of relationship compatibility.

## AI use declaration

AI was used in this project to aid in some of the development for example, the boilerplate for the db endpoints, helping debug, some functions etc.
when AI was used it was used in file / function. I am not a fan of agents swarming the codebase, and all code was checked and verified by me.

AI also created the demo and helped to make my wording for the readme clearer and in formatting of it.

all architectural decisions were made by me.

## How matching works

1. A PostgreSQL function refills a user's candidate pool after checking profile and preference constraints in both directions (including age, location, height, gender, and smoking preferences). Rejected pairs never reach vector scoring.
2. A user reaction to a prompt becomes an entry in the interaction ledger. Prompt embeddings, reaction direction, prompt weight, and time decay contribute to an axis vector; the result is normalised and stored for reuse.
3. Two attachment related axis are compared to stored anchor vectors. Other scored axis use cosine similarity between the users' vectors, mapped to a 0–1 score.
4. Scores are weighted across available axis and divided by their **active** weight. The prototype accepts scores at or above 0.60; axis with missing data do not contribute. The demo evaluates both directions.
5. Passing candidates enter the serious candidate / release queue flow. Reciprocal interest can advance a pair to questions and final decisions; both final yes decisions create a match.

The demo shows this flow with ten deliberately chosen synthetic pairs. It reuses the real database functions and matching services, but constructs controlled interactions so the expected outcomes are understandable. The precomputed prompt embeddings in `supabase/seed.sql` mean **the demo does not call an embedding API or need an API key**.

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

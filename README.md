# Main Engine Processor — Suara Kita AI Triage

Async Rust service that consumes citizen messages from a Redis queue, runs AI analysis, and persists results to PostgreSQL, pgvector, and Neo4j.

## Architecture

```
Redis queue:voter_inputs → BRPOP → LLM analysis → PostgreSQL (interactions + voters)
                                                  → pgvector (issue embeddings)
                                                  → Neo4j (voter → issue → policy graph)
```

## Features

- **Queue-based** — BRPOP with 30s timeout from `queue:voter_inputs`, concurrent worker pool
- **LLM analysis** — 11-field `LlmAnalysis` via `gpt-oss-120b`: intent, urgency (low/medium/high), scope, sentiment, category, language, location tags, summary
- **Noise rejection** — Pure insults, greetings, spam without a concrete issue → logged as noise, skipped from DB/graph
- **Retry with fallback** — 3-attempt LLM retry with JSON parse validation; missing fields default to safe values
- **Background queue monitoring** — Separate task logs queue depth every 30s, no race with BRPOP
- **Dead letter queue** — Failed pipeline messages pushed to `queue:voter_inputs_dlq` for manual reprocessing
- **Rich graph** — Neo4j: `Voter -[RAISED]-> Issue -[MAPS_TO]-> Policy` with scope, sentiment, location tags
- **Vector search** — pgvector embeddings for similarity matching

## Quick Start

```bash
cp .env.example .env
# Edit .env with your values
cargo run
```

## Environment Variables

| Variable | Description | Default |
|---|---|---|
| `DATABASE_URL` | PostgreSQL connection string | `postgres://voter_app:changeme@localhost:5433/voter_intelligence` |
| `REDIS_HOST` | Redis host | `localhost` |
| `REDIS_PORT` | Redis port | `6380` |
| `REDIS_PASSWORD` | Redis password | `redis` |
| `NEO4J_HOST` | Neo4j host | `localhost` |
| `NEO4J_BOLT_PORT` | Neo4j Bolt port | `7688` |
| `NEO4J_USER` | Neo4j user | `neo4j` |
| `NEO4J_PASSWORD` | Neo4j password | `changeme` |
| `LLM_ENDPOINT` | LLM endpoint URL | `https://openrouter.ai/api/v1` |
| `LLM_API_KEY` | OpenRouter API key | — |
| `LLM_MODEL` | LLM model | `openai/gpt-oss-120b` |
| `WORKER_COUNT` | Concurrent pipeline workers | `4` |
| `RUST_LOG` | Logging level | `info` |

## Contracts

- `contracts/voter_input.json` — Input schema consumed from `queue:voter_inputs`
- `contracts/approved_action.json` — Output schema pushed to `queue:approved_actions`

## Database

- **PostgreSQL** — `voter_profiles`, `interactions` (with CHECK constraints), `issue_embeddings` (pgvector)
- **Neo4j** — Voter → Issue → Policy graph with enriched properties

## Related Repositories

- [telegram-bot](https://github.com/Suara-Kita/telegram-bot) — Ingestion bot that pushes to `queue:voter_inputs`
- [dashboard](https://github.com/Suara-Kita/dashboard) — Admin UI for approving issues into `queue:approved_actions`

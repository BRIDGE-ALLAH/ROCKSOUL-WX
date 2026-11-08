# ROCKSOUL-WX

> **Don't route prompts. Route outcomes.**

ROCKSOUL-WX is a Rust-first, outcome-driven AI gateway foundation. It treats each request as a governed workload with an objective, data classification, permitted execution location, acceptance criteria, and operational constraints.

The product thesis is to evolve from model routing into a workload execution fabric. The current MVP deliberately proves a narrow vertical slice before attempting autonomous multi-provider allocation.

## What works today

- Rust workspace with typed workload contracts, deterministic policy checks, a planner, an OpenAI-compatible provider adapter, and an Axum gateway.
- Dioxus Command Deck connected to the workload execution API.
- `POST /v1/workloads/plan` validates and plans without calling a model.
- `POST /v1/workloads/execute` applies policy, calls the configured provider, evaluates supported checks, and persists a receipt.
- `POST /v1/chat/completions` provides a text-only, non-streaming OpenAI-compatible route.
- `GET /v1/receipts/{id}` retrieves a persisted metadata receipt.
- Bearer API key support, loopback-only unauthenticated development mode, explicit CORS origins, request body limits, provider timeouts, and policy enforcement.
- A local Ollama Compose stack and CI for Rust tests plus Dioxus WebAssembly compilation.

## Quick start with Docker Compose

```bash
cp .env.example .env
python3 -c 'import secrets; print(secrets.token_hex(32))'
```

Put the generated value (64 hex characters) into `WX_API_KEY` in `.env`, then:

```bash
docker compose up --build -d
docker compose exec ollama ollama pull llama3.2
dx serve --package wx-command-deck --platform web --port 8081
```

Open the Dioxus local URL. The Command Deck connects to `http://127.0.0.1:8080`; use the gateway API key from `.env` when prompted. Compose binds gateway and Ollama ports to loopback only.

For host-based development, see [Development Guide](docs/development.md). For endpoints and JSON shapes, see [API Reference](docs/api-reference.md).

## Architecture

```text
Browser / OpenAI SDK
        |
        v
  Rust + Axum Gateway
        |
   Workload Contract
        |
  Deterministic Policy
        |
   Execution Planner
        |
  Configured Provider
 (Ollama / OpenAI-compatible)
        |
   Acceptance Checks
        |
   Execution Receipt
        |
  Dioxus Command Deck
```

Key crates:

| Crate | Responsibility |
|---|---|
| `wx-contract` | Typed workload contracts, execution plans, and policy metadata |
| `wx-policy` | Deterministic, fail-closed privacy and egress checks |
| `wx-planner` | Provider-neutral strategy selection |
| `wx-provider` | Operator-configured OpenAI-compatible HTTP client |
| `wx-gateway` | Authentication, API, execution, CORS, and receipt persistence |

## Product thesis and research agenda

The differentiators remain hypotheses to test, not claims of prior-art uniqueness or achieved performance:

- **Workload Compiler** — compile objectives into constrained execution graphs.
- **Dissent-to-Intervention** — use worker disagreement to select targeted verification or human review.
- **Data Sovereignty Compiler** — preserve data labels through tools, caches, logs, and egress.
- **Proof-Carrying Receipts** — record provenance and named checks without claiming universal proof of truth.
- **Verified Semantic Cache** — measure false-hit risk before optimizing reuse.
- **Temporal Scheduling** — schedule deferrable workloads against measured cost, latency, and energy signals.

See [RFC-001](docs/RFC-001-outcome-driven-intelligence-fabric.md) and the [Research Agenda](docs/research-agenda.md).

## Security and current limits

- Provider URLs, model, and provider credentials are server-side configuration; browser input cannot redirect the gateway to an arbitrary URL.
- Unspecified workload classification defaults to **internal**. Raw-data egress defaults to **denied**.
- Confidential/restricted workloads cannot execute through a public-cloud provider.
- Remote gateway binds require `WX_API_KEY` with at least 32 characters. For public-cloud provider endpoints, HTTPS is mandatory.
- Provider selection is currently a single configured adapter, not an adaptive multi-provider router.
- Streaming is not supported.
- Budgets with `max_cost_microusd` are rejected until token pricing and budget enforcement exist.
- `require_human_approval=true` is blocked until an approval workflow exists.
- A passed acceptance result means only that supported deterministic checks passed; it is not proof of semantic correctness.
- Receipts are local JSON metadata files, not a tamper-evident distributed ledger.
- Multi-tenant auth/isolation, rate limiting, production secret management, and a full Action Escrow runtime are future work.

Do not expose the current gateway directly to the public internet. Put it behind a TLS reverse proxy and add the missing production controls before shared or multi-tenant deployment.

## CI

GitHub Actions checks:

```bash
cargo fmt --all -- --check
cargo test --workspace --exclude wx-command-deck
cargo check -p wx-command-deck --target wasm32-unknown-unknown
```

The core suite includes an in-process mock OpenAI-compatible provider; no real provider credential is needed for CI.

## License

No license has been selected yet. Until one is added, standard copyright restrictions apply.

# Development Guide

## Prerequisites

- Stable Rust and Cargo (edition 2024 support).
- Docker and Docker Compose for the local Ollama provider, or an existing OpenAI-compatible endpoint.
- Dioxus CLI for the web Command Deck.

Install the Dioxus CLI using the official Dioxus installation instructions. The UI uses the web target.

## Option A: Docker Compose (recommended)

Copy the environment template and set a gateway key:

```bash
cp .env.example .env
python3 -c 'import secrets; print(secrets.token_hex(32))'
```

Put the generated 64-character value in `WX_API_KEY` inside `.env`. Then start the local gateway and Ollama:

```bash
docker compose up --build -d
docker compose exec ollama ollama pull llama3.2
```

The gateway is bound to `127.0.0.1:8080`; Ollama is bound to `127.0.0.1:11434`. Receipts are stored in the named `rocksoul_wx_receipts` volume. Do not expose these development ports directly to the public internet.

Check the gateway:

```bash
curl -sS http://127.0.0.1:8080/healthz
```

## Option B: run the gateway on the host

Start Ollama:

```bash
docker compose up -d ollama
docker compose exec ollama ollama pull llama3.2
```

Then load environment variables and run the gateway:

```bash
set -a
source .env
set +a
cargo run -p wx-gateway
```

The default bind is `127.0.0.1:8080`. When binding to a non-loopback address, set `WX_API_KEY` to at least 32 characters. Public-cloud upstream endpoints must use HTTPS.

## Run the Dioxus Command Deck

In another terminal:

```bash
dx serve --package wx-command-deck --platform web --port 8081
```

Open the local Dioxus URL. The UI calls the gateway's `/v1/workloads/execute` endpoint. The gateway's default CORS allow-list includes `http://localhost:8081` and `http://127.0.0.1:8081`. Configure `WX_CORS_ORIGINS` if the UI is hosted elsewhere.

The browser never receives the upstream provider key. If `WX_API_KEY` is configured, enter that gateway key in the Command Deck.

## Use the workload API

Plan without executing:

```bash
curl -sS -X POST http://127.0.0.1:8080/v1/workloads/plan \
  -H 'content-type: application/json' \
  -d '{
    "objective": "Summarize a public changelog",
    "data_classification": "public",
    "raw_data_egress": "deny",
    "allowed_execution": ["local"],
    "acceptance_criteria": ["non_empty_output"]
  }'
```

Execute against the configured local provider:

```bash
curl -sS -X POST http://127.0.0.1:8080/v1/workloads/execute \
  -H "Authorization: Bearer $WX_API_KEY" \
  -H 'content-type: application/json' \
  -d '{
    "contract": {
      "objective": "Summarize a public changelog",
      "data_classification": "public",
      "raw_data_egress": "deny",
      "allowed_execution": ["local"],
      "acceptance_criteria": ["non_empty_output", "min_length:30"]
    },
    "input": "Version 2.1 adds audit exports and fixes CSV parsing."
  }'
```

The response includes the selected plan, generated text, token usage (if provided by the model), supported acceptance checks, and receipt ID. Receipt metadata can be retrieved at `GET /v1/receipts/{id}`.

## Use the OpenAI-compatible API

`POST /v1/chat/completions` accepts the text-only, non-streaming OpenAI Chat Completions request shape. When the configured provider is public cloud, set `X-WX-Data-Classification` and explicitly allow egress with `X-WX-Allow-Egress: true`. The defaults are internal data and no egress.

See [API Reference](api-reference.md) for request schemas, error codes, acceptance criteria, and restrictions.

## Configuration

| Variable | Default | Purpose |
|---|---|---|
| `WX_BIND_ADDR` | `127.0.0.1:8080` | Gateway bind address |
| `WX_API_KEY` | unset | Required for non-loopback binds; min. 32 chars |
| `WX_UPSTREAM_NAME` | `openai-compatible` | Display name in health/receipts |
| `WX_UPSTREAM_BASE_URL` | `http://127.0.0.1:11434/v1` | Server-side provider base URL |
| `WX_UPSTREAM_API_KEY` | unset | Optional upstream Bearer credential |
| `WX_UPSTREAM_MODEL` | `llama3.2` | The only model allowed through this adapter |
| `WX_UPSTREAM_LOCATION` | `local` | `local`, `private_cloud`, or `public_cloud` |
| `WX_UPSTREAM_TIMEOUT_SECONDS` | `60` | Provider timeout, clamped to 1–300 seconds |
| `WX_CORS_ORIGINS` | local Dioxus origins | Comma-separated browser origins |
| `WX_RECEIPT_DIR` | `data/receipts` | Directory for per-execution receipt JSON |

## Tests and checks

```bash
cargo fmt --all -- --check
cargo test --workspace --exclude wx-command-deck
cargo check -p wx-command-deck --target wasm32-unknown-unknown
```

The gateway tests use a local mock OpenAI-compatible server; they do not require an external API key or incur provider costs.

## MVP limitations

- Only one operator-configured OpenAI-compatible provider is active at a time.
- Streaming responses are not supported.
- The Command Deck has no user/tenant management yet.
- A workload with `max_cost_microusd` is rejected until cost metering and budget enforcement exist.
- `require_human_approval=true` is blocked; the approval workflow is not implemented.
- Supported acceptance checks are deterministic and narrow. A passed check is not proof of semantic truth.
- Receipts are local JSON files. They are not a distributed, tamper-evident ledger.
- No rate limiter, multi-tenant isolation, or production secret manager is included yet.

## Implementation rules

- Keep policy validation deterministic and unit tested.
- Add tests for both allowed and denied paths.
- Never log raw workload objectives, provider keys, or model payloads by default.
- Provider URLs are operator-controlled; never accept an upstream URL from a workload request.
- Re-check privacy and authorization before every external side effect.
- Do not claim verification beyond the checks actually executed.

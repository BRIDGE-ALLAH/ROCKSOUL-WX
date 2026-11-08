# Development Guide

## Prerequisites

- Stable Rust and Cargo (edition 2024 support).
- Docker and Docker Compose for local Ollama or one or more OpenAI-compatible endpoints.
- Dioxus CLI for the web Command Deck.

## Local Compose

```bash
cp .env.example .env
python3 -c 'import secrets; print(secrets.token_hex(32))'
```

Put the generated 64-character value in WX_API_KEY inside .env, then:

```bash
docker compose up --build -d
docker compose exec ollama ollama pull llama3.2
```

Gateway and Ollama are bound to host loopback. Receipts are kept in a named Docker volume. Never publish development ports directly to the internet.

## Run gateway on the host

```bash
set -a
source .env
set +a
cargo run -p wx-gateway
```

Default bind is 127.0.0.1:8080. Non-loopback binds require WX_API_KEY with at least 32 characters. Public-cloud provider endpoints must use HTTPS.

## Multi-provider catalog

Create a JSON file with an array of provider profiles. Copy [the catalog example](../config/providers.production.example.json) to config/providers.local.json, replace example values, and set WX_PROVIDER_CATALOG_FILE=./config/providers.local.json.

Each profile contains:
- name, base_url, model, location (local, private_cloud, public_cloud)
- quality_score (0–100) and p50_latency_ms (operator-supplied measurements)
- optional input_cost_microusd_per_1k_tokens and output_cost_microusd_per_1k_tokens
- optional api_key_env (environment variable name, not secret value)
- timeout_seconds and enabled

The key is read from the environment variable named by api_key_env, or its _FILE counterpart. For example, WX_PROVIDER_KEY_FILE=/run/secrets/provider_key supplies the value for WX_PROVIDER_KEY. Do not commit credentials or production provider configuration.

Choose a routing objective and process-local rate:

```bash
WX_ROUTE_OBJECTIVE=balanced # balanced | quality | cost | latency
WX_RATE_LIMIT_PER_MINUTE=120
```

The router filters providers through explicit location allow-list and deterministic privacy policy before ranking. Local is preferred over private cloud, then public cloud, whenever those locations are permitted. Public cloud requires explicit egress permission and cannot receive confidential/restricted content.

Quality/p50 latency must come from repeatable tests. Price rates should be checked against current provider pricing. Receipts contain only post-execution estimates where rates and usage are known; they do not impose hard spend caps.

## Workload API

Plan without executing:

```bash
curl -sS -X POST http://127.0.0.1:8080/v1/workloads/plan \
  -H 'content-type: application/json' \
  -H "Authorization: Bearer $WX_API_KEY" \
  -d '{
    "objective": "Summarize a public changelog",
    "data_classification": "public",
    "raw_data_egress": "deny",
    "allowed_execution": ["local"],
    "acceptance_criteria": ["non_empty_output"]
  }'
```

Execute:

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

## Configuration

| Variable | Default | Purpose |
|---|---|---|
| WX_BIND_ADDR | 127.0.0.1:8080 | Gateway bind address |
| WX_API_KEY / WX_API_KEY_FILE | unset | Bearer key; required for non-loopback; min. 32 chars |
| WX_RECEIPT_HMAC_KEY / WX_RECEIPT_HMAC_KEY_FILE | unset | HMAC key for receipt integrity; required for non-loopback; min. 32 chars |
| WX_PROVIDER_CATALOG_FILE | unset | Optional provider profile JSON; absent means legacy single profile |
| WX_ROUTE_OBJECTIVE | balanced | balanced, quality, cost, latency |
| WX_RATE_LIMIT_PER_MINUTE | 120 | Per-key, process-local fixed-window limit |
| WX_UPSTREAM_NAME | ollama | Legacy single-profile name |
| WX_UPSTREAM_BASE_URL | local Ollama | Legacy provider endpoint |
| WX_UPSTREAM_API_KEY / _FILE | unset | Legacy provider credential |
| WX_UPSTREAM_MODEL | llama3.2 | Legacy configured model |
| WX_UPSTREAM_LOCATION | local | local, private_cloud, public_cloud |
| WX_UPSTREAM_TIMEOUT_SECONDS | 60 | Provider timeout (1–300 seconds) |
| WX_CORS_ORIGINS | local Dioxus origins | Comma-separated allowed browser origins |
| WX_RECEIPT_DIR | data/receipts | Local metadata receipts directory |

The _FILE mechanism is file-mounted secret injection, not a managed secret vault. Secrets are loaded at process start.

## Command Deck

```bash
dx serve --package wx-command-deck --platform web --port 8081
```

Configure the gateway base URL and Bearer key. The browser never receives provider credentials.

## Tests

```bash
cargo fmt --all -- --check
cargo test --workspace --exclude wx-command-deck
cargo check -p wx-command-deck --target wasm32-unknown-unknown
```

Provider tests use local mock servers, not live credentials.

## Current limitations

- Shared single gateway key; no tenant identity, user management, or tenant-level authorization.
- In-memory limiter resets on restart and is not shared across replicas.
- No retry/fallback after execution failure.
- No pre-execution budget reservation or hard spend ceiling; max_cost_microusd remains rejected.
- require_human_approval=true is blocked until approval workflow exists.
- Streaming, multimodal inputs, and embeddings are unsupported.
- Acceptance checks are narrow and do not prove semantic truth.
- When `WX_RECEIPT_HMAC_KEY` is configured, each receipt file includes an HMAC-SHA-256 signature checked on retrieval. This detects content alteration but not deletion/rollback and is not a distributed append-only ledger. Local loopback development can still run unsigned.

Keep policy deterministic and unit-tested. Never log raw prompts, credentials, or model payloads by default. Never accept provider URLs from workload input.

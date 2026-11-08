# Development Guide

## Toolchain

- Stable Rust toolchain with edition 2024 support.
- Cargo.
- Dioxus CLI for the web UI.
- A current browser for manual UI checks.

Install the Dioxus CLI using the official Dioxus installation instructions. Avoid pinning an unreviewed installer script in this repository.

## Run the planning gateway

```bash
cargo run -p wx-gateway
```

The default bind address is `127.0.0.1:8080`. Set `WX_BIND_ADDR` to change it.

## Check health

```bash
curl -sS http://127.0.0.1:8080/healthz
```

## Plan a workload

```bash
curl -sS -X POST http://127.0.0.1:8080/v1/workloads/plan \
  -H 'content-type: application/json' \
  -d '{
    "objective": "Summarize a public changelog",
    "data_classification": "public",
    "raw_data_egress": "allow",
    "allowed_execution": ["local", "public_cloud"],
    "acceptance_criteria": ["output is non-empty"]
  }'
```

Try changing `data_classification` to `confidential` while retaining `public_cloud`; the policy layer should reject the contract.

## Run the Dioxus Command Deck

```bash
dx serve --package wx-command-deck --platform web
```

The UI currently demonstrates the visual language and local draft interaction only. It does not yet call the gateway API.

## Tests

```bash
cargo fmt --all -- --check
cargo test --workspace --exclude wx-command-deck
```

The Dioxus app is excluded from native core tests because this scaffold targets the web renderer. Build it separately with the Dioxus CLI and a WebAssembly target.

## Implementation rules

- Keep policy validation deterministic and unit tested.
- Add tests for both allowed and denied paths.
- Do not log raw workload objectives or payloads by default.
- Keep planning separate from execution.
- Add a provider only behind an explicit adapter interface and a testable capability declaration.
- Every new execution side effect needs an authorization and idempotency story.

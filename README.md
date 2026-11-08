# ROCKSOUL-WX

> **Don't route prompts. Route outcomes.**

ROCKSOUL-WX is a Rust-first, outcome-driven AI gateway. Each request is a governed workload with an objective, data classification, permitted execution locations, acceptance criteria, and operational constraints.

This phase advances the original vertical slice into a **policy-first provider router** and a controlled single-operator deployment path. The router optimizes only inside privacy and trust boundaries; it never overrides them.

## What works today

- Typed workload contracts, deterministic privacy/egress policy, execution planning, OpenAI-compatible provider adapters, and an Axum gateway.
- A provider router that ranks an operator-configured catalog by balanced, quality, cost, or latency objective after filtering candidates by workload policy.
- Local-first and private-cloud-first strategies, with fallback only to locations explicitly allowed by the workload contract.
- POST /v1/workloads/plan validates and plans without calling a model.
- POST /v1/workloads/execute selects an eligible configured provider, executes the workload, runs named deterministic acceptance checks, and persists a metadata receipt.
- POST /v1/chat/completions provides a text-only, non-streaming OpenAI-compatible subset.
- GET /v1/receipts/{id} retrieves persisted execution metadata.
- Bearer-key authentication, constant-time key comparison, configurable in-process per-key rate limiting, CORS allow-list, request-size limits, provider timeouts, and mounted secret-file loading.
- Dioxus Command Deck connected to workload execution.
- Linux Docker Compose, CI, multi-architecture tagged image release workflow with SBOM/provenance, and a Caddy HTTPS deployment template.

## Quick start: local development

~~~bash
cp .env.example .env
python3 -c 'import secrets; print(secrets.token_hex(32))'
~~~

Put the generated value (64 hex characters) in WX_API_KEY in .env, then:

~~~bash
docker compose up --build -d
docker compose exec ollama ollama pull llama3.2
dx serve --package wx-command-deck --platform web --port 8081
~~~

Open the Dioxus local URL. The Command Deck connects to http://127.0.0.1:8080; enter the gateway API key when prompted. Local Compose binds gateway and Ollama ports to loopback.

See the [Development Guide](docs/development.md) and [API Reference](docs/api-reference.md).

## Windows installer

Each tagged release builds an x64 ROCKSOUL-WX-Setup-<version>.exe installer and a SHA-256 checksum. The installer packages the native Rust gateway and local PowerShell start/test helpers. It is a gateway test package, **not a native desktop GUI**.

1. Download the installer and matching checksum from [GitHub Releases](https://github.com/BRIDGE-ALLAH/ROCKSOUL-WX/releases).
2. Verify its SHA-256 checksum before running it.
3. Install and launch **Start Local Gateway** from the Start Menu.
4. Run **Verify Local Gateway** to test the health and authenticated planning endpoints.

The Windows package binds only to 127.0.0.1:8080, generates a per-user API key, and stores receipt metadata under %LOCALAPPDATA%\ROCKSOUL-WX. For actual model execution, install and run Ollama separately and pull llama3.2. See the [Windows testing guide](packaging/windows/WINDOWS-README.md).

The installer is currently unsigned because no Authenticode signing certificate is configured; Windows SmartScreen may show an unknown-publisher warning.

## Provider routing

For multi-provider routing, create a JSON catalog and set WX_PROVIDER_CATALOG_FILE. The gateway loads provider URL, model, trust location, operator-supplied quality/latency scores, token-price metadata, timeout, and an environment-variable **name** for the provider key. Provider credentials stay server-side and may be read through the corresponding _FILE variable.

Set WX_ROUTE_OBJECTIVE to one of:

| Objective | Ranking |
|---|---|
| balanced | Deterministic rank combining quality, latency, and known price |
| quality | Highest configured quality score first |
| cost | Lowest known input + output token price first; unknown rates rank last |
| latency | Lowest configured p50 latency first |

The planner's trust boundary takes priority: local is preferred when allowed, then private cloud, then public cloud. Public-cloud routes require explicit egress permission and cannot receive confidential or restricted workloads. Quality and latency scores are operator-maintained signals; measure your own workloads.

Provider price rates are also operator-maintained. When the provider returns token usage, the receipt records an **estimated post-execution charge**. This is not final billing or a hard budget guarantee. Requests setting max_cost_microusd continue to fail closed until pre-execution reservation and settlement exist.

See [provider catalog template](config/providers.production.example.json).

## Architecture

~~~text
Browser / OpenAI SDK
        |
        v
 Rust + Axum Gateway
        |
  Typed Workload Contract
        |
 Deterministic Privacy Policy
        |
   Execution Planner
        |
 wx-router (policy first)
        |
 Configured provider catalog
        |
 Acceptance checks + telemetry
        |
    Metadata receipt
        |
 Dioxus Command Deck
~~~

| Crate | Responsibility |
|---|---|
| wx-contract | Typed workload contracts, execution plans, privacy labels |
| wx-policy | Deterministic, fail-closed privacy and egress checks |
| wx-planner | Trusted execution-location preference |
| wx-provider | Operator-configured OpenAI-compatible HTTP adapter |
| wx-router | Deterministic provider ranking and estimated token cost |
| wx-gateway | Authentication, API, rate limit, routing, execution, and receipts |

## Production deployment

A single-operator deployment template is provided with:
- Caddy TLS termination and security headers.
- Gateway-only private network exposure, non-root/read-only container hardening, resource limits, and log rotation.
- Mounted secret files and persistent metadata receipts.
- Multi-architecture GHCR release workflow with SBOM/provenance and keyless image signing.

Read the [Production Deployment Runbook](docs/production-deployment.md) before deploying. You must supply a real host, DNS, domain, hosted frontend, provider endpoint/key, and operational monitoring. A template and buildable image do not constitute a live production deployment.

## Security boundaries and current limitations

- Workload classification defaults to **internal** and raw-data egress defaults to **denied**.
- Confidential/restricted workloads are never eligible for public-cloud execution.
- Remote binds require a gateway key of at least 32 characters. Public-cloud provider endpoints require HTTPS.
- Rate limiting is an in-memory fixed 60-second window per shared gateway credential; it is not distributed or per-user/per-tenant.
- This is a single-operator service, not a multi-tenant SaaS platform. Tenant identity, isolated credentials, and per-tenant authorization/quotas are not implemented.
- Provider quality/latency/price data must be maintained by the operator. There is no automatic retry/failover after a provider execution error.
- max_cost_microusd is rejected; post-execution estimates are not hard budget enforcement.
- require_human_approval=true is blocked until an approval workflow exists.
- Streaming, multimodal inputs, and embeddings are not supported.
- Acceptance results mean only that declared deterministic checks passed, not that an answer is semantically true.
- Receipts are local JSON metadata files, not a tamper-evident distributed ledger.
- The _FILE mechanism reads mounted secret files; it is not a managed secret vault or automated rotation service.

Do not expose gateway port 8080 directly. Use the production TLS proxy, restrict network access, and complete the production gates in the runbook before multi-customer use.

## CI

~~~bash
cargo fmt --all -- --check
cargo test --workspace --exclude wx-command-deck
cargo check -p wx-command-deck --target wasm32-unknown-unknown
cargo test -p wx-gateway # Windows runner
cargo build --release -p wx-gateway # Windows runner
~~~

Provider adapter tests use local mock servers. CI does not use live provider credentials or incur provider charges.

## License

No license has been selected yet. Until one is added, standard copyright restrictions apply.

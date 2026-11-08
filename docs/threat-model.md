# ROCKSOUL-WX Threat Model — Routing Release Candidate

This document describes the current repository implementation and its trust boundaries. It is not a security certification or independent penetration-test report.

## Assets

- Workload objectives, inputs, data classifications, and egress permissions.
- Gateway/provider credentials.
- Operator-defined provider profiles, model IDs, trust locations, and routing metadata.
- Provider outputs, token usage, acceptance results, execution receipts.
- DNS, TLS certificates, deployment configuration, and container images.

## Trust boundaries

1. Client to gateway: untrusted JSON, authorization, classification, and egress headers.
2. Gateway to policy/planner/router: contract validation and deterministic policy precede route selection.
3. Gateway to provider: URL/model are operator configured; credentials are injected server-side or from mounted files.
4. Gateway to receipt volume: execution metadata is stored locally; receipts omit raw prompt/output content.
5. Public Internet to Caddy to gateway: TLS and reverse-proxy boundary in the production Compose template.
6. Operator to provider catalog: quality, latency, and prices are administrative inputs and may be stale.

## Implemented controls

| Threat | Current control |
|---|---|
| Unauthorized API use | Bearer gateway key, minimum 32 characters for remote bind, constant-time comparison |
| Excess requests with shared key | In-memory per-key 60-second fixed-window limiter |
| Sensitive prompt to public provider | Deterministic policy blocks confidential/restricted classification; public cloud also requires egress permission |
| Client sends arbitrary upstream URL | Client model must match operator catalog; URL is never accepted from workload input |
| Provider key leaks into browser | Provider keys remain server-side; optional mounted secret files through _FILE |
| Cleartext cloud provider endpoint | Public-cloud profile must use HTTPS |
| Malformed/oversized payload | Strict request structs, JSON error handling, 1 MiB body cap, 256 KiB input cap |
| Slow upstream | Per-provider HTTP timeout and optional workload deadline |
| Unsupported acceptance criterion marked passed | Unknown checks become needs_definition |
| Artifact supply-chain integrity | Tagged image workflow emits SBOM/provenance and keyless Cosign signature |

## Routing invariants

- Policy and execution-location eligibility are evaluated before optimization objectives.
- No provider outside the contract allowed_execution list can be selected.
- Local/private execution is preferred over less trusted locations when the contract explicitly allows fallback.
- Confidential/restricted workloads cannot route to public cloud.
- Public cloud requires raw_data_egress=allow.
- Quality, latency, and price metadata come from operator; they are not measured results until benchmarked.
- Cost estimates are post-execution metadata, not spend reservation.

## Residual risks and release blockers

- **No tenant boundary:** one Bearer key is shared across all clients. This is not multi-tenant authentication/authorization.
- **In-process limiter:** resets on process restart and is not shared across replicas. Distributed edge/API limiting remains needed for scale.
- **No hard budget:** max_cost_microusd is rejected; estimates cannot prevent provider charges.
- **No approval workflow:** approval-required contracts are rejected.
- **No failover semantics:** provider errors are returned; no retries, idempotency, or alternate-provider failover.
- **Local receipt store:** access control, tamper evidence, retention, encryption at rest, backups, and tenant filtering need an operational design.
- **Secret file loading is not a vault:** no managed rotation, scoped credential broker, or audit integration.
- **No tool execution or DLP across tools** in this release.
- **No live cloud-provider certification:** CI uses mocks; provider endpoints, price data, and quality benchmarks need operator testing.
- **No independent penetration test, SLO/alerting, incident exercise, or recovery exercise recorded.**

## Deployment rules

- Do not expose gateway port 8080 publicly; deploy behind TLS termination.
- Restrict firewall/network access to required edge ports.
- Protect mounted secrets, provider config, receipt volumes, and backups.
- Never commit real keys or production provider configuration.
- Review the [Production Deployment Runbook](production-deployment.md) before deployment.

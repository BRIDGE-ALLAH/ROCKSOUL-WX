# ROCKSOUL-WX Roadmap

This roadmap records the remaining work required to move from the v0.2.0 release candidate to an operated production service. A successful build or GitHub Release is not the same as a live production deployment.

## M0 — v0.2.0 release and Windows test package

**Status: release workflow ready; publication pending.**

Completed:
- Rust workspace, policy-first provider routing, Axum gateway, and Dioxus web Command Deck.
- Fail-closed privacy/egress policy and provider allow-listing.
- Linux multi-architecture container build, SBOM/provenance generation, and keyless image signing workflow.
- Caddy-based HTTPS deployment template and single-operator runbook.
- Windows x64 gateway installer source, per-user API key bootstrap, PowerShell smoke test, and Inno Setup compilation in CI.

Remaining acceptance:
- Publish tag `v0.2.0` and GitHub Release.
- Attach `ROCKSOUL-WX-Setup-0.2.0.exe` and its SHA-256 checksum.
- Verify release image tags, signature, SBOM, and provenance.
- Install on a clean Windows 10/11 x64 machine and verify health, authenticated planning, and real provider execution.

## M1 — First controlled production deployment

**Status: deployment template exists; external prerequisites and live verification are pending.**

Required operator inputs:
- Linux host, domain, DNS control, and ports 80/443.
- Hosted Command Deck origin and a real OpenAI-compatible provider endpoint/model.
- Provider credential delivered through the documented secret-file path.
- Named operational owner and an agreed support/incident contact.

Acceptance:
- Deploy an immutable, published image behind Caddy TLS; no direct public gateway port.
- Verify DNS, certificate issuance/renewal, CORS, authentication, provider execution, and receipt retrieval on the real domain.
- Configure external uptime/error/provider-billing monitoring and alert routing.
- Exercise backup and restore for receipt and Caddy volumes; document rollback and credential rotation.
- Record deployment commit, image digest, configuration owner, and smoke-test evidence.

This milestone is intentionally scoped to one operator and one trust domain. It does not claim multi-tenant SaaS readiness.

## M2 — Governed execution controls

**Status: not implemented. Do not accept workloads that depend on these guarantees.**

Required work:
- Pre-execution budget reservation using conservative token/cost estimates, atomic accounting, and explicit handling of unknown provider pricing.
- Post-execution settlement from provider usage, release of unused reservation, and reconciliation for timeouts/ambiguous upstream outcomes.
- Explicit approval records with authenticated approver identity, expiry, reason, immutable decision history, and a one-time execution binding.
- Idempotency keys and duplicate-execution protection across retries and worker restarts.

Acceptance:
- A workload exceeding its budget is rejected before provider execution.
- Concurrent requests cannot overspend the same budget.
- Unknown price/token limits fail closed when a hard cap is required.
- Approval-required workloads cannot execute until an authorized, unexpired approval exists.
- Replay, duplicate approval, expired approval, and settlement-reconciliation tests pass.

## M3 — Identity, authorization, and tenant isolation

**Status: not implemented. Current gateway uses one shared API key.**

Required work:
- Tenant and user identity from a reviewed OIDC/OAuth integration or equivalent trusted identity layer.
- Per-tenant service credentials, key rotation/revocation, scoped RBAC, and audit attribution.
- Per-tenant quotas and authorization on every workload and receipt lookup.
- Tenant-scoped storage paths/keys and isolation tests preventing cross-tenant reads or writes.
- Distributed rate limiting suitable for multiple gateway replicas.

Acceptance:
- No shared global customer credential.
- A user cannot read, cancel, approve, or inspect another tenant's workloads or receipts.
- Quotas and permissions remain consistent across replicas and restarts.
- Authentication, authorization, tenant-boundary, revocation, and rate-limit tests pass.

## M4 — Durable audit and operational reliability

**Status: partially implemented; durable storage and operational reliability remain open.**

Implemented in the receipt-integrity hardening branch:
- Per-record HMAC-SHA-256 signatures when `WX_RECEIPT_HMAC_KEY` is configured.
- Signature verification on retrieval; altered signed records fail closed.
- Non-loopback binds require a 32-character receipt integrity key.

Still required:
- Database-backed receipt persistence, retention, and tested backup/restore.
- Detection of deletion/rollback and append-only audit history.
- Distributed rate limiting, metrics/alerts, retry safeguards, and recovery exercise.

Required work:
- Durable database-backed receipt storage with migrations, access controls, retention, and backup/restore.
- Tamper-evident receipt integrity (for example, a keyed signature or append-only hash chain with protected key management).
- Bounded provider retries only where request semantics and idempotency permit; circuit breaker and clear failure states.
- Distributed rate limiting and service-level metrics for latency, errors, queueing, provider usage, and budget state.
- Vulnerability scanning, dependency update policy, incident runbooks, and a recovery exercise.

Acceptance:
- Receipt writes are atomic and durable through process restarts and interrupted writes.
- Integrity verification detects altered, missing, or reordered records.
- Retry tests prove no duplicate billable execution for non-idempotent requests.
- Alerts are exercised; restore and rollback are demonstrated.

## M5 — Expanded API and client capabilities

**Status: not implemented.**

Required work:
- Streaming responses with cancellation, timeout, backpressure, and receipt lifecycle semantics.
- Multimodal request contracts and provider capability negotiation.
- Embeddings API and storage/retention rules where needed.
- Native desktop Command Deck packaging only if a native GUI is an explicit product requirement; the current Windows EXE installs the local gateway only.

Acceptance:
- Contract, policy, privacy, cancellation, timeout, and provider compatibility tests cover each capability.
- Unsupported content is rejected explicitly rather than silently coerced.
- API documentation and client compatibility tests match actual behavior.

## Release and production stop conditions

Do not label the system “production complete” until M1 has been deployed and verified. Do not market it as multi-tenant or enterprise-ready until M2–M4 are implemented and independently reviewed. M5 is feature expansion and can be prioritized by product demand.

Security testing, provider benchmarking, real-world billing validation, and a production recovery exercise require external environments and cannot be inferred from CI success.

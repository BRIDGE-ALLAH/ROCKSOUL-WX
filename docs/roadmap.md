# ROCKSOUL-WX Roadmap

This roadmap records the remaining work required to move from the published v0.2.0 release toward an operated production service. The v0.2.1 patch adds receipt-integrity verification; release publication is tracked separately from live deployment. A successful build or GitHub Release is not the same as a live production deployment.

## M0 — Release and Windows test package

**Status: v0.2.0 published; v0.2.1 receipt-integrity patch release is in progress.**

Completed:
- Rust workspace, policy-first provider routing, Axum gateway, and Dioxus web Command Deck.
- Fail-closed privacy/egress policy and provider allow-listing.
- Linux multi-architecture container build, SBOM/provenance generation, and keyless image signing workflow.
- Caddy-based HTTPS deployment template and single-operator runbook.
- Windows x64 gateway installer source, per-user gateway and receipt-integrity key bootstrap, PowerShell smoke test, and Inno Setup compilation in CI.
- GitHub Release `v0.2.0` with the Windows installer and SHA-256 checksum.
- Receipt HMAC hardening merged to main; the `v0.2.1` image/installer release workflow has been triggered.

Remaining acceptance:
- Verify `v0.2.1` image signature, SBOM/provenance, installer and checksum assets after its release workflow completes.
- Run the Windows installer on a clean Windows 10/11 machine; the CI runtime install/launch/uninstall test is being added in PR #19.
- Verify real provider execution with an installed local model or configured provider. The installer smoke test does not exercise inference.

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

**Status: partially implemented; durable storage and operations remain open.**

Implemented:
- Per-record HMAC-SHA-256 signatures when `WX_RECEIPT_HMAC_KEY` is configured.
- Signature verification on retrieval; modified records and receipt-ID/storage-key mismatches fail closed.
- Non-loopback binds require a 32-character receipt-integrity key.
- Production Compose mounts the key as a secret; Windows creates a per-user key.

Security boundary:
- HMAC detects content alteration when the key remains protected. It does not detect deletion or rollback of old receipt files and is not an append-only distributed ledger.
- Unsigned legacy receipts from v0.2.0 are not silently promoted to trusted signed receipts after upgrade.
- The operator must protect and back up the HMAC key; losing it makes signed receipts unverifiable.

Still required:
- Database-backed receipt persistence, migrations, retention, and tested backup/restore.
- Append-only history or protected sequence/checkpoints to detect deletion and rollback.
- Distributed rate limiting, metrics/alerts, safe retries/circuit breakers with idempotency, vulnerability scanning, and recovery exercise.

Acceptance:
- Receipt writes survive restarts and interrupted writes.
- Integrity verification detects changes; the selected durable audit design also detects deletion/rollback.
- Retry tests prove non-idempotent requests are not duplicated.
- Alert routing and restore/rollback exercise are demonstrated.

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

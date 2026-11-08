# Initial Threat Model

## Scope

This threat model covers the initial workload contract, planning API, policy crate, and the intended future execution boundary. It is a starting point, not a security certification.

## Assets

- Workload objectives and input data.
- Tenant identity and authorization context (not yet implemented).
- Provider credentials (not yet implemented).
- Execution plans and policy versions.
- Tool/action authorization records (future).
- Execution receipts and telemetry (future).

## Trust boundaries

1. **Client → Gateway:** all incoming JSON is untrusted.
2. **Gateway → Policy Engine:** only typed, parsed contracts are passed; parsing does not imply authorization.
3. **Planner → Runtime:** plans are proposals. A future executor must re-check authorization at execution time.
4. **Runtime → Provider/Tool:** external systems are separate trust domains.
5. **Runtime → Logs/Cache/Receipts:** sensitive content must not be copied without a retention and redaction policy.

## Threats and mitigations

| Threat | Initial/future mitigation |
|---|---|
| Sensitive prompt sent to public provider | v0.1 rejects public-cloud execution for confidential/restricted data and rejects public cloud when raw egress is denied |
| Prompt injection attempts to override policy | Policy enforcement is deterministic and external to model prompts |
| Malformed or ambiguous contract | Strict JSON schema types, unknown-field rejection, validation errors |
| Model proposes an unauthorized tool action | Future Action Escrow must authorize tool calls independently of model output |
| Data leakage through logs | Future structured logging must exclude raw prompt/content by default |
| Cross-tenant data exposure | Tenant isolation is a prerequisite before multi-tenant production deployment |
| Replay or duplicate external action | Future idempotency keys and action-specific replay protections |
| Semantic-cache false hit | Future domain-specific equivalence checks, TTL/freshness constraints, and false-hit evaluation |
| Compromised provider credential | Future secret manager integration, scoped credentials, rotation, and audit trail |
| Policy drift or unsafe learned routing | Versioned policy, offline evaluation, explicit promotion, and rollback |

## Security invariants

- Do not put provider secrets in source code, UI bundles, or logs.
- Do not treat model-generated approval as authorization.
- Do not execute a plan merely because it was returned by the planner.
- Re-check permissions immediately before a side effect.
- Fail closed when classification or destination trust is unclear.
- Do not label an output "verified" unless named checks actually ran and their results are stored.

## Known limitations of v0.1

- No authentication or tenant isolation.
- No provider integration or secret storage.
- No executor or tool invocation.
- No request-size/rate limiting middleware.
- No persistent event log or receipt.
- No data-loss prevention or field-level information-flow tracking.
- The Dioxus UI is a static/local draft scaffold and is not connected to the gateway.

Do not expose the current gateway publicly until authentication, rate limiting, request limits, TLS termination, operational monitoring, and tenant boundaries are implemented and reviewed.

# API Reference — routing release candidate

Base URL for local development: http://127.0.0.1:8080.

All /v1/* routes require Authorization: Bearer <WX_API_KEY> when a gateway key is configured. Without a key, the process must bind to loopback. Protected routes are subject to the in-process per-key rate limit. GET /healthz is public and returns service health, provider count, and route objective without exposing provider URLs, model IDs, or secrets.

## Health

GET /healthz

Example:

```json
{
  "status": "ok",
  "service": "wx-gateway",
  "mode": "workload-execution",
  "provider_count": 2,
  "routing_objective": "balanced"
}
```

## Plan a workload

POST /v1/workloads/plan validates the contract and returns a proposed trust strategy. It does not call a provider.

```json
{
  "objective": "Summarize a public product changelog",
  "data_classification": "public",
  "raw_data_egress": "deny",
  "allowed_execution": ["local"],
  "acceptance_criteria": ["non_empty_output"]
}
```

## Execute a workload

POST /v1/workloads/execute validates policy and planner strategy, filters the configured provider catalog, selects a candidate within the allowed trust boundary, executes one non-streaming request, evaluates deterministic checks, and persists a receipt.

```json
{
  "contract": {
    "objective": "Summarize a public changelog",
    "data_classification": "public",
    "raw_data_egress": "deny",
    "allowed_execution": ["local"],
    "acceptance_criteria": ["non_empty_output", "min_length:40"]
  },
  "input": "Release notes: Version 2.1 adds audit exports and fixes CSV parsing."
}
```

Optional model must match one or more operator-configured catalog entries. It does not permit arbitrary provider or URL selection.

The execution response includes selected provider, output, token usage when supplied, named checks, and receipt. The receipt records provider/location, route objective, observed latency, token usage, and estimated charge when rates and usage are known.

## Provider catalog and routing objective

Set WX_PROVIDER_CATALOG_FILE to a JSON file containing an array of profiles. See [the production catalog example](../config/providers.production.example.json) for the schema. Profiles declare name, server-side base URL, model, trust location, quality score (0–100), p50 latency in milliseconds, optional input/output prices in micro-USD per 1,000 tokens, optional API-key environment-variable name, timeout, and enabled flag.

Credentials are never stored in the catalog. If api_key_env is WX_PRIMARY_PROVIDER_API_KEY, the gateway reads that environment variable or WX_PRIMARY_PROVIDER_API_KEY_FILE.

Set WX_ROUTE_OBJECTIVE to balanced, quality, cost, or latency. Selection is deterministic:
1. Reject invalid contracts and catalogs.
2. Exclude disabled providers and locations not allowed by the workload/planner.
3. Enforce privacy/egress policy; public cloud requires raw_data_egress=allow and cannot receive confidential/restricted data.
4. Prefer local over private cloud and private cloud over public cloud when those locations are permitted as fallback.
5. Rank candidates at the selected trust level using the configured objective.

Quality and latency fields are operator-maintained, not automatically measured by ROCKSOUL-WX. Unknown prices rank last for cost routing. There is no automatic retry or provider failover after an upstream error.

## Acceptance criteria

Supported deterministic checks:
- non_empty_output (also accepts non-empty output or output is non-empty)
- contains:<text> (case-insensitive substring)
- min_length:<integer> (Unicode character count)

Unknown criteria yield needs_definition, not a pass. A passed result only means the listed checks ran and passed; it is not proof of semantic correctness.

## Cost and deadline semantics

deadline_ms is applied as a request timeout.

The router records estimated_cost_microusd after execution only when the provider returns token usage and both operator-supplied price rates are present. The estimate is rounded up to the next micro-USD and is not a provider invoice.

Requests with max_cost_microusd are rejected with HTTP 501 because pre-execution budget reservation, hard caps, and settlement are not implemented. Do not use this release for workloads that require a guaranteed spending ceiling.

require_human_approval=true is blocked with HTTP 409 until an explicit approval workflow exists.

## Read a receipt

GET /v1/receipts/{receipt_id} returns the stored metadata receipt. It is an audit aid, not a cryptographic proof of truth. Receipt files are stored under WX_RECEIPT_DIR.

## OpenAI-compatible chat completion

POST /v1/chat/completions supports a text-only, non-streaming subset of the OpenAI Chat Completions request shape. Public-cloud egress requires:
- X-WX-Data-Classification: public|internal
- X-WX-Allow-Egress: true

Defaults are internal and false. Confidential/restricted data cannot use public-cloud providers.

```bash
curl https://api.example.com/v1/chat/completions \
  -H "Authorization: Bearer $WX_API_KEY" \
  -H "Content-Type: application/json" \
  -H "X-WX-Data-Classification: public" \
  -H "X-WX-Allow-Egress: true" \
  -d '{
    "model": "your-configured-model",
    "messages": [{"role":"user","content":"Explain workload routing."}],
    "stream": false
  }'
```

Response includes X-WX-Receipt-ID. Streaming is not supported (stream=true returns HTTP 501); multimodal content is outside this adapter.

## Common errors

| HTTP | Code | Meaning |
|---|---|---|
| 400 | invalid_json, input_required, model_not_allowed | Invalid request or model not in catalog |
| 401 | unauthorized | Missing or invalid Bearer key |
| 409 | human_approval_required | Approval gate unavailable |
| 422 | workload_policy_rejected, no_eligible_provider | Privacy/location/catalog constraints reject routing |
| 429 | rate_limit_exceeded | In-process per-key request window exhausted |
| 501 | streaming_not_supported, budget_enforcement_unavailable | Deliberately unsupported |
| 502 | provider_unavailable, provider_rejected_request | Upstream provider failed |
| 504 | workload_deadline_exceeded | Deadline elapsed |

## Security boundaries

- Provider URL/credentials are server-side operator configuration.
- Public-cloud provider URLs must use HTTPS.
- Non-loopback binds require a gateway key of at least 32 characters.
- Request bodies are capped at 1 MiB; workload input is capped at 256 KiB.
- Browser origins are allow-listed with WX_CORS_ORIGINS.
- Rate limiting is process-local and shared by clients using one key.
- Receipts are local JSON files; protect and back them up. Multi-tenant access control and tamper-evident storage are not implemented.

# API Reference — v0.1

Base URL for local development: `http://127.0.0.1:8080`.

All `/v1/*` routes require `Authorization: Bearer <WX_API_KEY>` when the gateway has `WX_API_KEY` configured. When no key is configured, the gateway must remain bound to a loopback address. `GET /healthz` is public and returns configuration metadata only (never secrets).

## Health

`GET /healthz`

Example response:

```json
{
  "status": "ok",
  "service": "wx-gateway",
  "mode": "workload-execution",
  "provider": "ollama",
  "provider_location": "local",
  "default_model": "llama3.2"
}
```

## Plan a workload

`POST /v1/workloads/plan`

This endpoint validates the contract and returns a proposed strategy. It does not call a model or execute tools.

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

`POST /v1/workloads/execute`

The gateway validates the contract, checks that the operator-configured provider location is allowed and matches the planner's selected strategy, sends one non-streaming request to the configured provider, runs supported deterministic acceptance checks, and persists a receipt.

Request:

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

Optional `model` must equal `WX_UPSTREAM_MODEL`. The browser/client cannot select a different provider or arbitrary upstream URL.

Supported deterministic acceptance criteria:

- `non_empty_output` (also accepts `non-empty output` or `output is non-empty`)
- `contains:<text>` (case-insensitive substring)
- `min_length:<integer>` (Unicode character count)

Unknown criteria produce `needs_definition`; they are never silently treated as passed. A `passed` result only means that the declared supported checks passed, not that the semantic answer is true.

Budget note: requests with `max_cost_microusd` are rejected until an actual provider price meter and budget enforcement engine is implemented. `deadline_ms` is enforced as a request timeout. `require_human_approval=true` is blocked because the human approval workflow is not implemented yet.

The response contains the proposed plan, model output, token usage (when supplied by the provider), acceptance checks, and a receipt. Receipt files are written as `<receipt-id>.json` under `WX_RECEIPT_DIR`. Raw prompts and model outputs are not written into receipt files.

## Read a receipt

`GET /v1/receipts/{receipt_id}`

Returns the stored metadata receipt. A receipt is an audit record of execution metadata and checks, not a cryptographic proof of truth.

## OpenAI-compatible chat completion

`POST /v1/chat/completions`

Supports the text-only, non-streaming subset of the OpenAI Chat Completions request format.

Required headers for cloud egress:

- `X-WX-Data-Classification: public|internal`
- `X-WX-Allow-Egress: true`

The defaults are `internal` and `false`. Confidential and restricted data cannot use a public-cloud provider.

Example:

```bash
curl http://127.0.0.1:8080/v1/chat/completions \
  -H "Authorization: Bearer $WX_API_KEY" \
  -H "Content-Type: application/json" \
  -H "X-WX-Data-Classification: public" \
  -H "X-WX-Allow-Egress: true" \
  -d '{
    "model": "your-configured-model",
    "messages": [{"role":"user","content":"Explain the idea of workload routing."}],
    "stream": false
  }'
```

The gateway returns the provider's OpenAI-compatible JSON response and includes `X-WX-Receipt-ID` in the response headers. Use the receipt endpoint to inspect metadata.

Streaming is explicitly unsupported in v0.1; `stream=true` returns HTTP 501. Multimodal message content is not accepted by this text-only adapter.

## Common errors

| HTTP | Code | Meaning |
|---|---|---|
| 400 | `invalid_json`, `input_required`, `model_not_allowed` | Request shape or model selection is invalid |
| 401 | `unauthorized` | Missing or invalid gateway Bearer key |
| 409 | `human_approval_required` | Workload requires an approval flow not yet available |
| 422 | `workload_policy_rejected`, `provider_location_not_allowed` | Contract violates policy or provider location |
| 501 | `streaming_not_supported`, `budget_enforcement_unavailable` | Feature is intentionally not implemented |
| 502 | `provider_unavailable`, `provider_rejected_request` | Upstream provider failed |
| 504 | `workload_deadline_exceeded` | Workload deadline elapsed |

## Security boundaries

- Provider URLs and provider credentials are server-side environment configuration.
- Public-cloud provider URLs must use HTTPS.
- Remote binds require a gateway API key of at least 32 characters.
- Request bodies are capped at 1 MiB.
- CORS origins are explicit and configurable; the default is local Dioxus development.
- The receipt currently stores metadata locally on disk; use a protected volume and backups for deployments.

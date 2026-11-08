# Workload Contract

The Workload Contract is the provider-neutral description of a unit of work. The API accepts JSON and validates it before producing an execution plan.

## Example

```json
{
  "objective": "Summarize a public product changelog",
  "data_classification": "public",
  "raw_data_egress": "allow",
  "allowed_execution": ["local", "public_cloud"],
  "max_cost_microusd": 2500,
  "deadline_ms": 8000,
  "acceptance_criteria": [
    "output is non-empty",
    "all release dates are preserved"
  ],
  "require_human_approval": false
}
```

The server assigns a UUID when `id` is omitted.

## Fields

| Field | Type | Default | Meaning |
|---|---|---|---|
| `id` | UUID | generated | Stable workload identifier |
| `objective` | string | required | Outcome to accomplish |
| `data_classification` | enum | `public` | `public`, `internal`, `confidential`, `restricted` |
| `raw_data_egress` | enum | `deny` | Whether raw workload data may leave the trusted boundary |
| `allowed_execution` | enum array | `["local"]` | Permitted locations: `local`, `private_cloud`, `public_cloud` |
| `max_cost_microusd` | integer/null | null | Maximum budget in micro-USD; a value of zero means no spend is permitted |
| `deadline_ms` | integer/null | null | Desired completion deadline in milliseconds |
| `acceptance_criteria` | string array | empty | Human-readable criteria to be implemented by validators |
| `require_human_approval` | boolean | false | Whether execution must pause for an authorized human decision |

## Conservative policy rules (v0.1)

- The objective must not be empty.
- At least one execution location must be specified.
- `raw_data_egress=deny` conflicts with `public_cloud` in the allowed execution list.
- Confidential and restricted workloads must deny raw-data egress.
- Confidential and restricted workloads cannot include public-cloud execution.
- Confidential and restricted workloads must allow local or private-cloud execution.
- Empty acceptance-criteria entries are rejected.

These rules are intentionally conservative and should be expanded through reviewed policy versions. The first version does not implement declassification, redaction proofs, trusted execution environments, or field-level information-flow tracking.

## Important semantics

- `allowed_execution` is an allow-list, not a preference list.
- `raw_data_egress` is not a substitute for a complete data-flow policy.
- `max_cost_microusd` is a planning constraint only until a metering and budget-enforcement runtime exists.
- `deadline_ms` is not an SLA guarantee.
- Acceptance criteria are declarations until a verifier actually evaluates them.
- An execution strategy is a plan label, not evidence that work ran.

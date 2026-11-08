# ROCKSOUL-WX

> **Don't route prompts. Route outcomes.**

ROCKSOUL-WX is an outcome-driven intelligence fabric: a Rust-first runtime for compiling requests into governed workloads, choosing appropriate execution paths, and recording verifiable evidence about how work was completed.

The project deliberately starts smaller than its long-term vision. The first milestone is a safe, inspectable foundation—not an autonomous system that silently changes production policy.

## Product thesis

Most AI gateways optimize the path from a prompt to a model. ROCKSOUL-WX optimizes the path from an objective to an acceptable outcome.

A workload is described by an explicit contract:

- **Objective** — what needs to be accomplished.
- **Constraints** — budget and deadline boundaries.
- **Privacy** — data classification and permitted execution locations.
- **Acceptance criteria** — checks required before an outcome can be called verified.
- **Action policy** — whether external or irreversible actions need approval.

The runtime may eventually choose deterministic software, local models, cloud models, tools, or human review. It must never treat a model's self-reported confidence as proof of correctness.

## Architectural pillars

1. **Intelligence Compiler** — normalize intent into a typed workload contract and execution plan.
2. **Sovereign Execution** — enforce privacy, budget, deadline, and action constraints outside the model.
3. **Verifiable Intelligence** — attach provenance, validation results, and an execution receipt to completed work.

## Initial implementation

The initial repository foundation provides:

- A Rust workspace with separate contract, policy, planner, and gateway crates.
- A Dioxus-based Command Deck application scaffold.
- A workload planning API that does not call an external model.
- Conservative policy checks for sensitive data and public-cloud egress.
- An RFC describing the architecture, threat boundaries, and incremental roadmap.

**Important:** the first release is a planning foundation. It does not yet execute LLM requests, provide cryptographic proof of truth, or guarantee that a workload result is correct.

## Repository layout

```text
apps/
  wx-gateway/       HTTP API and runtime entry point
  wx-command-deck/  Dioxus operator interface
crates/
  wx-contract/      Typed workload contracts and plans
  wx-policy/        Deterministic policy validation
  wx-planner/       Initial execution-strategy planner
docs/
  RFC-001-outcome-driven-intelligence-fabric.md
  workload-contract.md
  threat-model.md
```

## Development

Prerequisites: stable Rust (edition 2024), Cargo, and the Dioxus CLI for the web UI.

Run the gateway:

```bash
cargo run -p wx-gateway
```

The gateway listens on `127.0.0.1:8080` by default.

Check its health:

```bash
curl http://127.0.0.1:8080/healthz
```

Plan a workload (planning only; no model provider is called):

```bash
curl -X POST http://127.0.0.1:8080/v1/workloads/plan \
  -H 'content-type: application/json' \
  -d '{
    "objective": "Summarize the public product changelog",
    "data_classification": "public",
    "raw_data_egress": "allow",
    "allowed_execution": ["local", "public_cloud"],
    "acceptance_criteria": ["output is non-empty"]
  }'
```

Run the Dioxus web app:

```bash
dx serve --package wx-command-deck --platform web
```

## Design principles

- **Policy beats prompt instructions.** Enforcement belongs in deterministic code at system boundaries.
- **No-LLM is a first-class route.** Use deterministic tools where they can satisfy the contract.
- **Fail closed for sensitive data.** Unclear privacy policy must not silently authorize public egress.
- **Verification is scoped.** A receipt records which checks ran; it is not a universal proof of truth.
- **No silent self-modification.** Learned routing policies require offline evaluation and explicit promotion.
- **Start as a modular monolith.** Split services only when operational evidence justifies it.

## Status

Early architecture and implementation scaffold. APIs and schemas are expected to evolve before a stable release. See [RFC-001](docs/RFC-001-outcome-driven-intelligence-fabric.md) for the design.

## License

No license has been selected yet. Until a license is added, standard copyright restrictions apply.

# RFC-001: Outcome-Driven Intelligence Fabric

- **Status:** Draft
- **Version:** 0.1
- **Scope:** Initial architecture and implementation sequence
- **Primary implementation language:** Rust
- **Operator interface:** Dioxus

## Abstract

ROCKSOUL-WX treats an AI request as a governed workload rather than a prompt that must be sent to a language model. A workload carries an objective, constraints, privacy classification, permitted execution locations, acceptance criteria, and action-approval requirements.

The runtime's job is to produce a policy-compliant execution plan. Over time, the platform may execute that plan across deterministic software, local models, private infrastructure, public model providers, tools, or human review. A completed workload should carry an execution receipt describing the evidence collected and checks performed.

This RFC intentionally separates the long-term thesis from the current implementation. The first milestone validates contracts, deterministic policy checks, and plan generation. It does not yet execute model calls or claim that generated outputs are true.

## Motivation

Traditional gateways primarily normalize provider APIs, route requests, apply retries, and record usage. Those capabilities are useful but do not fully answer the operational question:

> What is the safest, least costly, policy-compliant way to complete this piece of work to an acceptable standard?

ROCKSOUL-WX makes the workload—not the provider request—the central unit of planning.

## Goals

1. Represent work in a provider-neutral, typed contract.
2. Keep privacy and authorization enforcement outside model prompts.
3. Support deterministic and no-model execution as first-class future strategies.
4. Make execution plans inspectable before execution.
5. Define acceptance checks independently from model confidence.
6. Record provenance and validation evidence for completed work.
7. Keep the first release small enough to test and operate.

## Non-goals for the first milestone

- Training or hosting a foundation model.
- Calling external LLM providers.
- Executing arbitrary tools or agent actions.
- Dynamic self-modification of production routing policies.
- Claiming cryptographic proof that a semantic answer is true.
- Introducing blockchain or a distributed marketplace before a real trust and settlement requirement exists.
- Splitting the system into microservices prematurely.

## Core concepts

### Workload Contract

The contract defines the objective and operating boundaries. See [workload-contract.md](workload-contract.md).

### Workload Compiler

A future compiler will normalize user intent into a typed contract and an execution graph. In the first milestone, clients provide the contract directly; natural-language compilation is deferred until it can be evaluated safely.

### Policy Engine

A deterministic enforcement layer validates contracts before any runtime is selected. Model output cannot override policy decisions.

### Execution Planner

The planner creates an inspectable strategy and ordered steps. It does not perform inference or tool execution.

### Verification Core

A future subsystem will evaluate acceptance criteria using domain-specific deterministic checks, independent evaluators, or human review. A model's self-reported confidence is not sufficient evidence.

### Execution Receipt

A future receipt records the workload ID, policy version, selected runtime, provider/model versions when applicable, data lineage, tool calls, cost, timing, approval events, and validation results. A receipt is an audit record, not proof that every statement is correct.

## System boundaries

- **Data plane:** accepts workload requests and, in later milestones, executes approved plans.
- **Control plane:** manages policies, providers, budgets, worker capabilities, and operational views.
- **Command Deck:** Dioxus operator interface. It must not be on the critical path for each model token.
- **Storage:** persists contracts, plans, events, and receipts with tenant isolation and retention controls.
- **Provider adapters:** translate internal work steps to provider-specific protocols. Provider adapters never own policy authority.

## Invariants

1. Sensitive data is not sent to a less-trusted execution location without explicit policy authorization.
2. Empty or ambiguous execution allow-lists fail closed.
3. A plan is not an execution, and an execution receipt is not a proof of truth.
4. Model confidence cannot authorize a tool action.
5. Irreversible or externally visible actions require an explicit authorization path.
6. Policy changes are versioned and auditable.
7. Learned routing policies are evaluated offline before production promotion.
8. No provider call occurs in planning-only mode.

## Proposed pipeline

```text
Request
  -> Contract normalization
  -> Policy validation
  -> Execution planning
  -> (future) Approved execution
  -> Acceptance verification
  -> Execution receipt
  -> Operational learning
```

The planner must be deterministic for a given contract and planner version. Any future stochastic planner must record its policy/model version and remain bounded by deterministic enforcement.

## Initial planning strategies

The initial planner supports conservative strategy labels:

- `local_only`
- `local_first`
- `private_cloud_only`
- `public_cloud_eligible`
- `human_approval_required`

These labels describe eligibility and preference, not an actual provider selection. In particular, `public_cloud_eligible` does not mean a provider call has happened.

## Future research tracks

### A. Workload compilation

Evaluate decomposition against direct single-model execution on fixed task suites. Measure acceptance success, cost, latency, failure modes, and human effort.

### B. Disagreement-driven intervention

Treat disagreement as a trigger for targeted retrieval, deterministic checks, clarification, independent evaluation, or human review. Do not use majority vote as a universal correctness test.

### C. Data sovereignty

Track sensitivity labels from sources through transformations, context assembly, tool calls, logs, caches, and outputs. Redaction alone is not a complete information-flow control.

### D. Verified semantic cache

A cache hit must satisfy explicit freshness and equivalence conditions. Similarity scores alone are insufficient. Measure false-hit rates and domain-specific error bounds.

### E. Temporal scheduling

For deferrable work, explore deadline-aware scheduling across local capacity and provider availability. Cost, latency, reliability, and energy/carbon signals must be measured rather than guessed.

### F. Intelligence exchange

Only after capability profiles are grounded in representative workload measurements should the platform experiment with auctions or worker allocation. No token-based economy or external settlement layer is required for the initial product.

## Milestones

### M0 — Foundation

- Typed workload contract.
- Deterministic privacy policy checks.
- Planning API.
- Dioxus Command Deck scaffold.
- Threat model and CI baseline.

### M1 — Runtime boundaries

- Provider-neutral runtime trait.
- Local deterministic executor.
- OpenAI-compatible ingress.
- Explicit provider credentials and secret handling.
- Request IDs, structured telemetry, and bounded timeouts.

### M2 — Verification and receipts

- Acceptance check interface.
- Append-only execution event stream.
- Receipt schema with provenance and policy versions.
- Redaction and retention controls.

### M3 — Selective intelligence

- Capability evaluation harness.
- Local/cloud candidate planner.
- Disagreement-triggered interventions.
- Offline policy evaluation and manual promotion.

### M4 — Distributed and temporal execution

- Durable queue and worker leases.
- Replay and what-if simulation.
- Deadline-aware scheduling.
- Energy/carbon-aware experiments where reliable signals exist.

## Open questions

1. What is the first concrete workload domain: document processing, developer workflows, or enterprise data analysis?
2. Which acceptance criteria can be evaluated deterministically in that domain?
3. What is the minimum receipt that is genuinely useful to an auditor?
4. Which tenant and data-isolation guarantees are required for the first deployment?
5. How should policy authors test rules against recorded workloads before rollout?

## Decision

Build a modular Rust monolith first. Keep the workload contract and policy boundary stable; let provider integrations, UI, and scheduling evolve behind explicit interfaces.

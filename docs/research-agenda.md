# Research Agenda: Beyond Model Routing

This document turns the product thesis into falsifiable engineering hypotheses. The concepts below are exploratory directions, not claims of novelty or proven capabilities. Each track needs a prior-art review and a reproducible evaluation before it becomes a product promise.

## 1. Workload Compiler

**Hypothesis:** representing a request as a typed workload and decomposing it into a constrained execution graph can improve cost, auditability, and acceptance success for selected task families.

Research questions:
- Which tasks benefit from decomposition, and which are better handled by one model call?
- How should the compiler represent dependencies, budgets, deadlines, data labels, and acceptance tests?
- How can plans remain deterministic and explainable when natural-language intent parsing is probabilistic?

Evaluation:
- Compare direct-model baseline, deterministic workflow, and compiled plan on the same fixed dataset.
- Report task success, acceptance-test pass rate, total cost, p50/p95 latency, retries, and human interventions.
- Include negative cases where decomposition makes performance worse.

## 2. Dissent-to-Intervention Engine

**Hypothesis:** disagreement between independent workers is useful when it selects a targeted intervention, not when it is reduced to majority voting.

Potential interventions:
- retrieve missing evidence;
- run deterministic arithmetic or schema checks;
- ask the requester to clarify an ambiguous objective;
- send only the disputed subtask to a specialist;
- request a human review.

Evaluation:
- Measure error detection recall, false-alarm rate, intervention cost, and final acceptance success.
- Compare independent model agreement with calibrated uncertainty and deterministic checks.
- Record correlated model families; three similar models are not three independent witnesses.

## 3. Data Sovereignty Compiler

**Hypothesis:** attaching information-flow labels to data and execution steps can make hybrid local/cloud execution safer than a single coarse provider allow-list.

Research questions:
- How do labels propagate through summaries, embeddings, tool outputs, caches, and logs?
- What transformations are permitted declassification, and who can authorize them?
- How can a plan prove that a sensitive source did not flow into a public egress payload?

Evaluation:
- Build adversarial data-flow tests, including indirect prompt injection and tool-result exfiltration.
- Verify enforcement at the network/tool boundary, not merely in prompt templates.
- Test redaction leakage and cache cross-contamination.

## 4. Proof-Carrying Execution Receipts

**Hypothesis:** a structured receipt with provenance and named validation evidence can reduce audit effort and make operational claims more precise.

A receipt may record:
- workload and plan identifiers;
- contract and policy versions;
- input/source digests (not raw secret content);
- runtime, provider, and model identifiers;
- tool actions and authorization decisions;
- timing, cost, and retry events;
- acceptance checks, results, and evidence references.

Important limitation:
A receipt proves that recorded events and checks were produced by the system under a given trust model. It does not prove that a semantic conclusion is true. Cryptographic signatures, append-only storage, and key management are separate future design decisions.

Evaluation:
- Measure audit reconstruction time and missing-evidence rate.
- Verify receipt integrity and redaction behavior.
- Test whether an auditor can reproduce the execution path without receiving sensitive payloads.

## 5. Verified Semantic Cache

**Hypothesis:** caching a prior result with task-specific equivalence and freshness checks can reduce repeat inference without silently serving a wrong answer.

Possible guard conditions:
- tenant and security-domain match;
- same normalized task and relevant context;
- data freshness window;
- policy/version compatibility;
- acceptance checks still pass;
- explicit invalidation for mutable sources.

Evaluation:
- False-hit rate is the primary safety metric.
- Also measure cache-hit rate, cost saved, freshness misses, and invalidation delay.
- Never use embedding similarity as the only correctness condition.

## 6. Temporal and Carbon-Aware Scheduling

**Hypothesis:** deferrable workloads can be shifted across time or execution locations while preserving deadline and policy constraints.

Planner inputs may include:
- deadline and priority class;
- queue age and worker capacity;
- measured provider latency and availability;
- actual price quotes;
- energy/carbon intensity with source and timestamp.

Evaluation:
- Compare deadline miss rate, total cost, p95 completion time, and estimated emissions.
- Mark missing or stale environmental signals as unknown; do not fabricate carbon savings.
- Keep urgent and interactive tasks out of deferrable queues.

## 7. Intelligence Exchange

An internal capability registry may eventually describe workers (models, deterministic tools, domain services, and humans) using measured capabilities and constraints.

Do not begin with a token economy, public bidding, or financial settlement. First establish:
1. a common workload benchmark;
2. comparable worker capability measurements;
3. a transparent allocation objective;
4. abuse resistance and tenant isolation;
5. predictable failure and cancellation semantics.

Only then test whether auction-like allocation beats a simpler constrained optimizer.

## 8. Simulation and Counterfactual Replay

**Hypothesis:** replaying historical workloads against candidate policies can expose cost, quality, latency, and privacy regressions before a policy is promoted.

Requirements:
- immutable or versioned input snapshots;
- secrets and personal data redacted or replaced with safe fixtures;
- deterministic result comparison where possible;
- provider/model version tracking;
- confidence intervals and sample-size reporting;
- explicit separation between shadow evaluation and production routing.

## Research governance

Every new intelligence feature should have:
- a baseline;
- a defined task distribution;
- a success metric and safety metric;
- a reproducible experiment;
- a failure analysis;
- a rollback path;
- a clear statement of what the result does **not** establish.

The goal is not to make the gateway appear autonomous. The goal is to make workload decisions measurable, bounded, and explainable.

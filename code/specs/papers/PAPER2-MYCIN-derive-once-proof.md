# Paper 2 · MYCIN-2026 — the derive-once / reuse-indefinitely / CPU-offload proof

> **Work item W7** (plan/spec; **paper 2**). The concrete worked proof that anchors paper 2's
> abstract knowledge-compilation claim on a single domain with maximal rhetorical payoff: **MYCIN's
> own**. Slots into [`PAPER2-skeleton.md`](PAPER2-skeleton.md) E1 (compilation benchmark) + E2
> (correction-persistence / model-swap). Plan-of-record split: [`PAPER1-WORKPLAN.md`](PAPER1-WORKPLAN.md).

## 1. The claim, on MYCIN

> A byte-grounded MYCIN-2026 rulebook compiles **once** into a content-addressed (CAS) executable
> library; thereafter, held-out clinical cases are decided on **CPU with zero answer-time model
> calls**, each decision emitting a **machine-checked proof tree** over cited source bytes;
> corrections are made by **editing a rule** (→ new CAS version, regression-gated), not the weights,
> and **survive a model swap**.

Three sub-claims, each a measurable: **derive once** (compile cost paid one time), **reuse
indefinitely** (warm cases reuse the library at ~zero marginal model cost), **offload reasoning to
CPU** (a deterministic engine, not the LLM, computes the decision and its sensitivity).

## 2. Why MYCIN specifically (the rhetorical payoff)

MYCIN worked clinically but **died on build / maintain / trust**, not accuracy (paper 1's history
motif). The three things that killed 1980s expert systems are exactly what this stack dissolves:
- **knowledge acquisition** → the LLM derives the rulebook from byte-grounded literature
  (LLM-as-knowledge-engineer), not a human knowledge engineer;
- **maintenance** → corrections are versioned CAS edits, regression-gated (CI for the knowledge base);
- **trust** → every compiled rule cites verified source bytes, which is the *precondition* for running
  it on CPU without re-invoking the model.

Proving derive-once on **MYCIN's own disease-differential domain** is the cleanest possible
demonstration: we resurrect the canonical dead expert system on the new substrate and show the
failure modes are gone.

## 3. Building blocks that already exist

| piece | what it gives | source |
|---|---|---|
| MYCIN-2026 meningitis rulebook (recursively derived, byte-anchored, citations flagged-for-verification) | the grounded clinical rulebook | [ADJ44](ADJ44-mycin-2026-meningitis.md) |
| ACS rulebook **compiled through adj-lang** → `LoweredProgram { kb, queries }` (29 clauses) | rulebook → executable program path | [ADJ48](ADJ48-mycin-2026-in-adj-lang.md) |
| CAS **program-cache** mechanism: ground once → compiled library → 11th case imports it, **zero answer-time model calls** (proven on 8 U.S.C. 1427) | the derive-once mechanism itself | [ADJ71](ADJ71-cas-program-cache-experiment.md) |
| Uncertainty primitive: engine computes **decision = argmax + sensitivity**, no softmax/temperature | the CPU-side probabilistic reasoning | [ADJ65](ADJ65-uncertainty-primitive.md) |
| edit-override-propagate loop: override one fact → new CAS version → re-derive | correction-persistence | ADJ-CAS (`adj52/cas/`, #5233) |

W7 is the spec that **composes these into one MYCIN proof**; ADJ71 already showed the mechanism works
in a non-clinical domain, so the risk is integration, not invention.

## 4. The proof design

### 4.1 Cold path (derive once — paid one time, with the model)
Byte-provenance the MYCIN-2026 rulebook from literature → adj-lang compile → **CAS executable
library** (versioned, content-addressed). Record cold cost (model calls + tokens + wall-clock).

### 4.2 Warm path (reuse indefinitely — CPU only)
For each held-out clinical case: small/local model ingests the case → byte-accounted IR (facts only);
deterministic compiler emits a small program that **imports the cached library**; the **logic/ProbLog
engine executes on CPU**. Record **answer-time model calls (target: 0)**, warm cost, and the emitted
**proof tree**.

### 4.3 What gets measured
- `answer_time_model_calls` on warm cases — **0 is the headline**.
- `cold_vs_warm_cost` — amortization curve (derive-once pays off after k cases).
- `parity` — compiled-CPU decision vs LLM-answer-time decision on the same held-out cases (the CPU
  path must not lose accuracy to buy determinism).
- `proof_tree_completeness` — every decision step cites a library rule that cites source bytes
  (machine-checked defensible chain; removes the LLM from the verification loop).
- `correction_persistence` — override a clause (e.g. a flagged-for-verification citation that was
  wrong) → new CAS version → regression-gated → re-derive; the fix holds and **propagates** to every
  case citing that rule (ties to paper 1's E2 `propagate_yield`).
- `model_swap_durability` — swap the solver/ingest model; corrections (in the artifact, not weights)
  survive. The contrast model-editing (ROME/MEMIT) can't run cleanly.

### 4.4 Hypotheses
- **H1 (derive-once).** Warm held-out cases decide with **0 answer-time model calls** and CPU-bound
  latency.
- **H2 (parity).** Compiled-CPU accuracy ≈ LLM-answer-time accuracy on the same cases (no accuracy
  tax for determinism).
- **H3 (correctability persists).** A rule edit → versioned CAS → corrects this case and all siblings,
  and **survives a model swap**.
- **H4 (proof).** Every warm decision yields a complete proof tree to cited bytes — a reviewer audits
  the *tree*, never re-runs the model.

## 5. CPU-offload premise (the "reasoning is CPU-bound" claim)

The LLM is a **one-time translator** (literature → grounded rules; case → facts). The **reasoner is a
deterministic engine** (logic-engine / ProbLog), which computes the decision as an **argmax with
sensitivity** — no softmax, no temperature ([ADJ65](ADJ65-uncertainty-primitive.md);
[ADJ16](ADJ16-engine-programmatic-adjudication.md) for the engine path). Reasoning cost moves from
GPU-forward-passes-per-query to CPU-clause-resolution-per-query, and becomes **reproducible**: same
input + same CAS version + same query = same proof tree, byte-for-byte. This is the lever the memory
[[project_cpu_bound_reasoning_problog]] names: distill LLM knowledge into a grounded/correctable
clause library so the LLM leaves the verification loop.

## 6. Scope & dependencies
- **Paper 2**, not paper 1. Depends on the paper-1 preprint being out (the byte-provenance contract
  is the precondition for trust-free reuse).
- Privacy-local deployment (frontier-derives-offline-on-public-sources, local-model-ingests-PHI,
  CPU-executes-locally) is PAPER2 **E3** — this MYCIN anchor makes it concrete (clinical = the domain
  where PHI-local actually matters) but the full privacy experiment is its own work item.
- Honest risk: ADJ44 flagged several citations *for verification* — the derive-once proof must either
  verify them or carry them as explicitly-flagged, regression-gated assumptions (the correctability
  story, not a hidden weakness).

## 7. Build order (when paper 2 starts)

> **Concrete build spec:** [`PAPER2-MYCIN-build-spec.md`](PAPER2-MYCIN-build-spec.md) pins this to the
> in-repo **adj-lang** language (which *is* MYCIN's probabilistic model — `prior`/`contributes`/
> `interacts`/`uncertain`, log-odds, proof DAG, deterministic CPU) and details the **CAS-write gate**
> (N adversarial readers × byte-stability × blind-judge) that admits LLM-derived clauses. Read it for
> the how; this section is the proof-design summary.

1. Byte-provenance + adj-lang-compile the MYCIN-2026 rulebook → versioned CAS library (extends ADJ48 + ADJ71).
2. Held-out clinical case set (synthetic / published de-identified; no PHI); warm-path harness; assert `answer_time_model_calls == 0`.
3. Parity vs LLM-answer-time; proof-tree completeness check.
4. Correction-persistence + model-swap durability (ADJ-CAS override loop).
5. FINDINGS with the amortization curve + a worked proof tree + the honest citation-verification status.
Output: `code/specs/data/mycin-derive-once/` mirroring the ADJ run layout.

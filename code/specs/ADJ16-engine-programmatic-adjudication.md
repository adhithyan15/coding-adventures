# ADJ16 — Engine-Programmatic Adjudication: Replacing LLM Reasoning at Answer Time with a Deterministic Engine

## Overview

The framework's flow today (as of ADJ15) keeps the LLM in the loop
end-to-end: the LLM extracts facts, elicits rulebooks, performs the
audit-trail checks, *and* produces the final verdict. ADJ16 specifies
the path that **replaces the answer-time LLM call with a deterministic
engine** that runs a compiled Prolog (or ProbLog) program built from
the rulebook + facts.

The result: same input + same rulebook + same query produces the same
verdict every time, byte-for-byte reproducibly, with a full proof DAG
that traces which rule + which fact combined to derive the answer.

The LLM stays in the framework — it does the linguistic work
(extraction, elicitation, rendering). What changes is **where the
reasoning happens**: in a deterministic engine, not in a black-box
forward pass.

## Why this matters

ADJ15 demonstrated that **adding an explicit rulebook to the LLM's
answer-time prompt flips verdicts and produces rule citations** at the
3B+ scale. But the citations are imperfect — gemma4's "matches are
flammable materials" is a categorical leap the model made on the fly,
inside its own forward pass, untraceable beyond the final answer text.
A reviewer reading the audit trail sees the cited rule but cannot see
*how the model decided* the source matched that rule.

Engine-programmatic adjudication makes the "how" auditable. The proof
DAG records every rule that fired, every fact that was unified, every
substitution that was made. A reviewer can replay the engine on the
exact same inputs and get the exact same proof. There is no
"untraceable forward pass" between facts and verdict.

This also opens up:

- **Probabilistic adjudication** via ProbLog: rules carry probability
  weights, the engine produces a probability distribution over
  verdicts plus the marginal likelihood of each. *"What's the
  probability the passenger is compliant given the declared items
  match three carry-on rules with weights 0.95, 0.9, and 0.7?"*
- **Counterfactual queries**: rerun the engine with a single fact
  flipped, see how the verdict changes. *"Would the passenger be
  compliant if the matches were removed from the declaration?"*
- **Multi-source disputed answers**: when two rulebooks disagree, the
  engine returns BOTH proof paths attributed to their sources rather
  than silently picking one. (ADJ09 §"Conflicts Between Sources"
  already specs the result shape; ADJ16 wires it.)
- **Provable bounds**: SAT/SMT-style queries over the proof DAG. *"Is
  there any combination of declared items that makes the passenger
  compliant under rulebook A but non-compliant under rulebook B?"*

## What ADJ16 changes from today's flow

```text
                Today (LLM-everything)
   ┌────────────┐    ┌──────────────┐    ┌────────────┐
   │ source     │ →  │ decompose_   │ →  │ pipeline   │
   │ declaration│    │ text         │    │ (ADJ02-05) │
   └────────────┘    └──────────────┘    └──────┬─────┘
                                                ▼
                                         ┌────────────┐
                                         │ LLM final  │  ← VERDICT comes
                                         │ answer call│      from here today
                                         └────────────┘

                ADJ16 (engine-programmatic answer)
   ┌────────────┐    ┌──────────────┐    ┌────────────┐
   │ source     │ →  │ decompose_   │ →  │ pipeline   │
   │ declaration│    │ text         │    │ (ADJ02-05) │
   └────────────┘    └──────────────┘    └──────┬─────┘
                                                ▼
   ┌────────────┐    ┌──────────────┐    ┌────────────┐
   │ rulebook   │ →  │ rule         │ →  │ engine-run │  ← VERDICT comes
   │ (Tentative │    │ compilation  │    │ (Prolog or │      from here in ADJ16
   │  /Reviewed)│    │ (ADJ09)      │    │  ProbLog)  │
   └────────────┘    └──────────────┘    └────────────┘
```

The deltas:

- **Add** a rule-compilation pass (ADJ09's existing pipeline) that
  consumes the typed Rulebook IR (from ADJ14) and emits a knowledge
  base (KB).
- **Add** a fact-lowering pass that consumes the input pipeline's IR
  (post ADJ02-05) and emits engine atoms.
- **Add** a query-construction step that synthesises the Prolog goal
  (`?- compliant(Passenger).` for the TSA demo) from the IR's Query
  nodes.
- **Replace** the LLM answer-time call with `engine.run(KB, facts,
  goal)`, returning a typed `EngineVerdict { result,
  proof_dag, marginals?, latency_ms }`.
- **Keep** an optional `rendering` LLM call that turns
  `EngineVerdict` into natural-language explanation. This is a pure
  paraphrase task (no reasoning); it is faithful by ADJ04's existing
  round-trip discipline.

## Why determinism is the headline property

The LLM-answer-time path has a hidden non-determinism even with
`temperature: 0.0`:

1. **Sampling jitter on quantised models.** ggml/llama.cpp Q4_K_M
   models are deterministic *given the same hardware and exact build*,
   but small differences in BLAS implementation or kernel scheduling
   can flip a single token, which can flip a verdict.
2. **Context-window position sensitivity.** The same prompt, same
   model, same hardware, but with the rulebook inserted at a
   different offset (because the user added a comment or whitespace),
   can change attention weights and verdict.
3. **Per-deployment model drift.** Re-pulling `gemma4:latest` next
   month gets a different snapshot. Old adjudications can't be
   reproduced exactly.

Engine execution has none of these properties. `engine.run(KB, facts,
goal)` is a pure function on its inputs. The proof DAG it returns is
identical across hardware, time, and deployments. *Replay is exact.*

For regulated / air-gapped / appellate-review contexts, this matters:
you cannot defend a decision in court if the system that produced it
gives a different answer next week. Engine-programmatic adjudication
gives the rule-based-system property that the LLM-answer-time path
fundamentally cannot.

## Pipeline stages (formal)

### Stage 1 — Rulebook compilation

**Input**: a `Rulebook` (from `acquire_rulebook` per ADJ14, OR from
some future ingestion of an authoritative regulatory document).

**Output**: a `logic_engine::KnowledgeBase` (KB) — a typed collection
of Prolog/ProbLog clauses.

**Implementation**: `adjudication_connector::lower_to_kb` (already
exists per ADJ11; ADJ16 may need additions for the trust-tier
metadata, e.g. annotating each clause with the source rulebook's
trust level so disputed-answers can attribute correctly).

### Stage 2 — Fact lowering

**Input**: the input pipeline's IR document (post ADJ02-05 audit).

**Output**: a set of Prolog ground facts.

For ADJ10's TSA carry-on declaration:

```text
IR:
  Fact carry_on(toothpaste, "4 oz")
  Fact carry_on(matches)
  Query compliant(passenger)

→ Engine atoms:
  carry_on(passenger_a, toothpaste, quantity(4, oz)).
  carry_on(passenger_a, matches).
  ?- compliant(passenger_a).
```

The `passenger_a` Skolem is the entity the pipeline's Query points at.
Multi-entity adjudications (e.g., a clinical case with
multiple patients) generate one Skolem per entity.

### Stage 3 — Engine execution

**Input**: KB + Prolog/ProbLog goal.

**Output**: `EngineVerdict { result, proof_dag, marginals?,
disputed? }`.

The engine (`logic-engine` for Prolog, future `lp19-engine` or similar
for ProbLog) runs the goal against the KB and returns the proof.
ProbLog returns the marginal probability distribution; Prolog returns
the single deterministic answer.

For the canonical TSA case with a rulebook saying "strike-anywhere
matches are prohibited":

```text
Prolog answer: prohibited_item(passenger_a, matches).
Proof DAG:
  carry_on(passenger_a, matches)              [from facts]
  prohibited(matches)                          [from rule R3]
  ⇒ prohibited_item(passenger_a, matches).    [unification]
  
  compliant(passenger_a) :- 
      ¬∃X: prohibited_item(passenger_a, X)    [from rule R0]
  ⇒ ¬compliant(passenger_a).
  
VERDICT: NON-COMPLIANT (proof above)
```

The proof is byte-for-byte reproducible. Compare to today's
LLM-answer-time path where the proof is an English-language paragraph
the model emits.

### Stage 4 — Verdict rendering (optional, kept LLM-driven)

Take the `EngineVerdict` and produce a natural-language explanation
for human consumption. This is **not** reasoning — it's paraphrase.
It uses the existing `render_node` primitive's prompt discipline.

If a deployment doesn't want LLM at all (truly determinism-only),
this stage is skipped and the JSON `EngineVerdict` is the answer.

## What stays LLM, what moves to engine

| Step | Today (ADJ15) | ADJ16 | Why |
|---|---|---|---|
| Extract facts from source | LLM (decompose_text) | LLM | Linguistic work; the framework has always accepted this. |
| Elicit rulebook from weights | LLM (acquire_rulebook) | LLM | Same — Tentative tier, requires expert review per ADJ09. |
| Validate IR | engine (adjudication_ir::validate) | engine | Already deterministic. |
| Compile rules to KB | (skipped today) | engine (adjudication_connector) | Deterministic by construction. |
| Lower facts to KB | (skipped today) | engine | Deterministic. |
| **Produce verdict** | **LLM** | **engine** | **The substantive change.** |
| Render verdict to NL | (in same LLM call as reasoning) | LLM (separate render call) | Paraphrase only; ADJ04 round-trip discipline applies. |

## Probabilistic extension (ProbLog)

ProbLog rules carry probability weights:

```text
0.95 :: prohibited(matches).         % standard rule, very confident
0.7  :: prohibited(strike_anywhere(matches)).  % less confident specialisation
0.3  :: allowed_in_checked(matches).  % alternative pathway
```

The probabilities can come from:

1. **Multi-model agreement counts**: a rule that 5 of 5 models agree
   on gets weight 1.0; a rule only 2 of 5 produced gets weight 0.4.
   Direct application of ADJ16 to the ADJ15 adversarial-elicitation
   pipeline.
2. **Reviewer confidence**: when promoting `Tentative` → `Reviewed`,
   the expert can annotate each rule with a confidence score.
3. **Source weight**: an `Authoritative` rulebook's rules carry
   higher weight than a `Reviewed` rulebook's.

The engine returns marginal probabilities for each possible verdict
plus the most-likely proof path. For a TSA case, this might be:

```text
P(compliant)     = 0.06
P(non-compliant) = 0.94
Most-likely proof: rule R3 (strike-anywhere matches, p=0.7)
                   + rule R0 (compliance bound, p=1.0)
```

The 0.06 probability of compliance under this case is a real signal: a
reviewer might dig into whether the matches in the declaration are
actually strike-anywhere or just regular safety matches. The
distinction matters under rule R3; ProbLog surfaces it as uncertainty.

## What ADJ16 doesn't fix

1. **Garbage-in-garbage-out at the rulebook layer.** If the elicited
   rulebook says "matches are flammable liquids", the engine will
   still derive non-compliance — *for the wrong reason*. ADJ16 makes
   the wrong reason auditable (the proof shows which rule fired) but
   does not by itself produce correct rules. Multi-model
   adversarial elicitation (ADJ14) + expert review (ADJ09) remain
   the defenses.

2. **The fact-extraction layer stays LLM.** decompose_text could
   misclassify a span or omit a fact. ADJ02-05 catch the worst
   failure modes; ADJ06 retries; but the engine ultimately
   reasons over whatever facts came out of the pipeline.

3. **Linguistic ambiguity in the source.** A declaration that says
   "matches" with no further detail still requires the framework to
   guess whether they're strike-anywhere or safety. The engine
   produces a probability distribution over both readings (in
   ProbLog mode) or an `Uncertainty` node (in Prolog mode) — but
   that's the framework working as designed, not a vulnerability.

4. **Engine completeness.** Some queries can't be answered with the
   rules available. The engine returns `unknown` rather than
   inventing an answer. The framework's behaviour here is correct:
   refuse to fabricate. A real deployment would route `unknown` to
   human review.

## Open questions

1. **Negation-as-failure vs. classical negation.** Prolog's standard
   semantics is closed-world (negation-as-failure). For compliance
   adjudication this is often what you want (*"we have no rule
   saying matches are allowed, so they're not allowed"*). But it
   can produce surprising results when rule coverage is incomplete.
   ProbLog supports both; ADJ16 needs to specify per-domain which
   semantics applies.

2. **How to attribute the verdict when rulebooks disagree.**
   ADJ09 §"Conflicts Between Sources" says both proofs travel
   through. ADJ16 needs to extend `EngineVerdict` with a
   `DisputedAnswer { candidates: Vec<(verdict, proof,
   source_rulebook)>, resolution_required_from }` shape.

3. **Cost of full audit-trail storage.** The proof DAG can be large
   for complex rule sets. Compressing it via DAG canonicalisation
   (proof normal form) is a research question. For demo-scale
   adjudications, no problem.

4. **When does the LLM lose its remaining job?** If the engine
   handles reasoning and the verdict is rendered without an LLM,
   the only remaining LLM role is fact extraction and rulebook
   elicitation. Both could in principle be replaced by:
   - Authoritative rulebooks (already specced as the highest trust
     tier — pre-compiled, no LLM elicitation needed).
   - Structured input formats (JSON declarations, form-based UI,
     barcode scans) that skip the natural-language stage entirely.
   When both are available, the LLM is out of the loop and the
   framework becomes a deterministic rule engine. That's the
   long-term endpoint for high-stakes deployments.

## Implementation sequence

The path from today (ADJ15) to a working engine-programmatic
adjudication looks roughly like:

1. **ADJ16-impl-1**: extend `adjudication-connector` with the
   trust-tier metadata pass-through (every emitted clause carries
   `source_rulebook_id` + `trust_tier` in its metadata so the
   proof DAG can attribute correctly).
2. **ADJ16-impl-2**: add `adjudication_pipeline` mode flag —
   `AnswerMode::Llm` (today's behaviour) vs `AnswerMode::Engine`.
   In `Engine` mode, skip the final LLM call and run the engine
   on the lowered KB + facts + goal.
3. **ADJ16-impl-3**: extend `EngineVerdict` with the
   `DisputedAnswer` shape and wire it into the audit trail.
4. **ADJ16-impl-4**: wire ProbLog probability weights from
   multi-model agreement (ADJ14 adversarial elicitation results).
5. **ADJ16-impl-5**: TSA demo adversarial-engine arm — a fourth
   arm (after raw / pipeline / rulebook-injected) that runs the
   ProbLog engine on the adversarially-elicited rulebook and
   returns a marginal-probability verdict. Measured side-by-side
   against the existing arms.

Each step is a separate PR. Step 1 and 2 are small (extending
existing types and adding a feature flag); step 4 and 5 are the
substantive new capabilities.

## Status

Draft. Captures the design direction; implementation lands as a
sequence of follow-up PRs. The engine and the connector both already
exist in skeleton form (`logic-engine`, `adjudication-connector`);
ADJ16's contribution is wiring them as the answer-time path rather
than the engine-helper-on-the-side role they play today.

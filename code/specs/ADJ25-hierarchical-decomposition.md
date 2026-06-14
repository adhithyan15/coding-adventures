# ADJ25 — Hierarchical Decomposition Pipeline: Per-Level Coverage, Fresh-Agent Retries, Correlation Vector

> **Revision v1 (2026-05-13): foundational reset.** ADJ25 supersedes
> [ADJ01](ADJ01-adjudication-ir-grammar.md) (IR grammar),
> [ADJ02](ADJ02-coverage-checker.md) (coverage), and
> [ADJ06](ADJ06-clarification-dialogue.md) (clarification dialogue)
> wholesale. [ADJ03](ADJ03-polarity-modality-checker.md)
> (polarity/modality) and [ADJ22](ADJ22-typed-quantity-coverage.md)
> (typed-quantity coverage) are demoted from gating checks to audit-
> only. ADJ14/15/17 (rulebook elicitation), ADJ16 (engine
> adjudication), and ADJ18/19 (verdict benches) are paused pending
> convergence on the source-decomposition flow.
>
> The framework's central hypothesis — every byte of the input must be
> accounted for or the model has not done its job — is recast as a
> *hierarchical decomposition invariant*: every byte must be
> represented at every level of a `Document → Sentence → Phrase →
> Claim → TypedComponent` decomposition, with no level skipped, no
> flattening into atom names, and no `Tentative`-trust bypass on
> coverage failures. The LLM produces the decomposition; the framework
> checks it; failures trigger fresh-agent retries scoped to the
> failing parent until the gate is satisfied or the per-level budget
> is exhausted (hard reject, no fallback).

## Why this exists

The empirical record across [ADJ12](ADJ12-small-model-benchmarks.md),
[ADJ15](ADJ15-recursive-rulebook-empirical-results.md),
[ADJ17](ADJ17-adversarial-rulebook-empirical-results.md),
[ADJ18](ADJ18-broadened-tsa-empirical-bench.md),
[ADJ23](ADJ23-decomposition-bench.md), and
[ADJ24](ADJ24-typed-quantity-pipeline-wiring.md) shows that the
framework's headline guarantee — total coverage forces the model to
reason about every byte — is **not currently enforced**:

1. **ADJ15/17 shipped rulebooks the framework's own checker rejected.**
   gemma4's elicited rulebook had `CoverageGap(194, 200)`; llama3.1's
   had `CoverageGap(43, 44)`; all five ADJ15 elicitations were
   `validation_passed = false`. The framework accepted them via
   `Trust::Tentative`. The verdict flips reported in those specs ride
   on top of IR the strict checker said no to.

2. **ADJ23 measured 28% first-pass typed-quantity recall** (10%
   strict pass). ADJ24's retry loop brought it to 36% / 20%. **64% of
   source literals across the bench were still untyped** after
   retries — they were flattened into atom names (`50_wh`,
   `4_inch_blade`), which the byte-tiling check accepts because
   the spans still cover the source bytes. The check is too weak.

3. **ADJ18 measured verdict accuracy on Arm A**, not decomposition
   correctness on Arm B. We never benched the foundational question:
   *"does the LLM produce a hierarchical decomposition in which every
   byte is represented as a typed structured element, at every level?"*
   This spec defines what that question means; a follow-up bench
   measures the answer.

The path forward is to nail the source-decomposition flow before any
further work on rulebooks, engine arms, or cross-domain benches. This
spec is the foundation.

## Scope

**In scope.**

- The new IR shape, hierarchy levels, and per-level coverage invariants.
- The universal fresh-agent parent-scoped retry primitive.
- Correlation IDs flowing source byte → IR node → engine clause → verdict citation.
- Audit-trail extensions recording every retry call.
- Replacement of `decompose_text` (`llm-primitives`) with the new flow.

**Out of scope, paused pending convergence.**

- Rulebook elicitation (ADJ14/15/17) — entirely.
- Engine-programmatic adjudication (ADJ16).
- Verdict / fact-sheet benches (ADJ18/19/20).
- ADJ22 typed-quantity as a *standalone* gating pass (folds into level-4
  coverage here).
- ADJ03 polarity/modality as a gating pass (becomes a level-4
  `TypedComponent` kind; the structural slot is gated, the value the
  model picks is not).
- ADJ04 round-trip, ADJ05 adversarial — remain advisory; no change here.

The discipline: **only work on this flow until the bench shows every
byte of every source declaration represented at every decomposition
level.** No parallel feature work until the foundation gate holds.

## The decomposition hierarchy

Five levels. Each level decomposes the parent level's content into
typed children. Coverage is checked at every parent → child boundary.

```
   Level 0: Document
                │
                │ Contains  (Sentences tile the Document's bytes)
                ▼
   Level 1: Sentence ────── Discarded (sentence-scope: footers, salutations)
                │
                │ Contains  (Phrases tile the Sentence's bytes)
                ▼
   Level 2: Phrase ──────── Discarded (phrase-scope: pleasantries)
                │
                │ Contains  (claim-nodes tile the Phrase's bytes)
                ▼
   Level 3: Fact │ Uncertainty │ Question │ Discarded (claim-scope)
                │
                │ Contains  (TypedComponents tile the Fact's content)
                ▼
   Level 4: Quantity │ Polarity │ Entity │ Predicate │ Comparator │
            TimeRef  │ Modifier   (closed set, expand on bench evidence)
```

The hierarchy is the *skeleton*. The multi-DAG cross-cut edges from
ADJ01 v3 (`Excepts`, `Refers`, `Cites`, `Mentions`, `SameAs`,
`Supersedes`, etc.) carry forward unchanged and layer on top of the
skeleton. Only the skeleton changes; the relation taxonomy does not.

### Level 0 — Document

The root. Exactly one per IR document.

```
kind = Document implies:
    span             = [0, N) where N = len(normalized_text)
    correlation_id   = the document's root correlation id
    children         = Contains-edges to Sentence and (top-level) Discarded nodes
```

### Level 1 — Sentence

A natural-language sentence in the source. Decomposition granularity
is **model-determined**, subject to the per-level coverage check. The
framework does not enforce linguistic correctness ("is this really a
sentence?"); it only enforces that the Sentences tile the Document.

```
kind = Sentence implies:
    span             = a byte range within the Document's span
    children         = Contains-edges to Phrase and (sentence-scope) Discarded nodes
                       collectively tiling the Sentence's span
```

### Level 2 — Phrase

A sub-sentence chunk. Same model-determined granularity policy. A
Phrase is a coherent unit of meaning the model commits to as *"this
stretch of bytes contributes one claim (or one uncertainty, or one
question, or is discardable)."*

```
kind = Phrase implies:
    span             = a byte range within a Sentence's span
    children         = Contains-edges to claim-nodes and (phrase-scope) Discarded nodes
                       collectively tiling the Phrase's span
```

### Level 3 — Claim nodes

Four kinds. Each Phrase decomposes into one or more, exactly tiling
the Phrase's bytes.

| Kind | Meaning |
|---|---|
| `Fact` | A claim-bearing assertion about the world. |
| `Uncertainty` | The model could not commit to a definite reading. |
| `Question` | An interrogative present in the source text. |
| `Discarded` | Pleasantry / metadata / non-domain / explicitly out-of-scope. |

Note on `Question` vs. v3's `Query`: in this spec, `Question` is a
**source-text interrogative** ("Is this allowed?", "How many
batteries can I bring?"). `Query` is reserved for **engine-facing
posed questions** synthesized by the framework or the user when
invoking the engine. They are different kinds; the former is part of
the source decomposition, the latter is downstream of it.

```
kind = Fact implies:
    span             = a byte range within a Phrase's span
    children         = Contains-edges to TypedComponent nodes
                       collectively tiling the Fact's span

kind = Uncertainty implies:
    span             = a byte range within a Phrase's span
    children         = optional (Uncertainty nodes need not decompose to TypedComponents;
                       their interior is left as text the human reviewer or
                       downstream stage handles)

kind = Question implies:
    span             = a byte range within a Phrase's span
    children         = optional (same rationale as Uncertainty)

kind = Discarded implies:
    span             = a byte range at any level (Document/Sentence/Phrase scope)
    discard_reason   = required (closed vocabulary, ADJ01 v3 carries forward)
    children         = none (Discarded does not decompose)
```

### Level 4 — TypedComponents

The decomposition of a `Fact`'s content into structured slots. Closed
starting set, expandable on bench evidence:

| Kind | Form | Purpose |
|---|---|---|
| `Quantity` | `Quantity(value, unit)` | Every numerical literal in the Fact wraps here. `unit` is a free atom; per-domain unit-vocabulary is out of scope for v1. |
| `Polarity` | `Polarity(Affirmed \| Denied)` | Exists if the Fact contains negation cues ("no", "not", "denies", "without"). |
| `Entity` | `Entity(term)` | A named or referential noun phrase. |
| `Predicate` | `Predicate(term)` | The relation / verb of the Fact. |
| `Comparator` | `Comparator(op)` | `op ∈ {Eq, Lt, Le, Gt, Ge, Ne}` for thresholds. |
| `TimeRef` | `TimeRef(term)` | A date, duration, or temporal phrase. |
| `Modifier` | `Modifier(term)` | Adjective / adverb refinement that doesn't fit the above. |

A Fact is fully decomposed when its bytes are exactly tiled by its
TypedComponent children. A Fact whose span includes a numeric literal
but produces no `Quantity` child is a coverage failure at this level.

## Coverage invariants — per level

At every parent → child boundary, the framework checks:

1. **Each child's span ⊆ the parent's span.**
2. **The union of children's spans = the parent's span exactly.**
3. **No two children's spans overlap.**
4. **No child has an empty span** unless it is a synthesized cross-cut
   object (e.g., an `Entity` referenced by multiple Facts — Entity may
   have empty spans when shared; spans live on the `Mentions` edges).

This is the **flat tiling check from ADJ02 v3, applied at every
level**, not just at the document root. It catches:

- `Document → Sentence`: missing whole sentences.
- `Sentence → Phrase`: gaps within a sentence.
- `Phrase → Claim`: phrase-internal omissions.
- `Fact → TypedComponent`: **the flattening failure that today's
  ADJ02 misses.** A Fact whose span covers "50 Wh" but whose only
  child is `Entity(battery_50_wh)` fails this check — the digit run
  `50` is not represented by a `Quantity(50, Wh)` TypedComponent.

### No-flattening rule (level-4 specific)

In addition to span tiling, the framework rejects TypedComponent
configurations that smuggle source content into atom names:

- **No atom may contain a digit run that appears in the source.**
  `50_wh` is rejected; `50` must surface as `Quantity(50, Wh)`. The
  digit run check is purely textual — if `"50"` appears as a substring
  in the source, no atom whose name contains the substring `"50"` is
  accepted.
- **No atom may end in a known unit suffix joined by underscore.**
  Banned suffixes: `_wh`, `_ml`, `_oz`, `_in`, `_inch`, `_inches`,
  `_kg`, `_lb`, `_g`, `_v`, `_mAh`, `_kwh`, `_bpm`, `_mmhg`,
  `_celsius`, `_fahrenheit`, `_count`. The list is expanded on bench
  evidence.
- **No atom may consist of more than two underscore-separated words
  drawn from the source.** Catches `pocket_knife_blade_length` and
  similar collapses. Two-word atoms (`pocket_knife`) are accepted as
  legitimate compound nouns.

Non-numeric **single-word** atoms drawn from the source are accepted:
`matches`, `passenger`, `bag`, `lithium`. The rule's goal is
structural decomposition, not source-word avoidance.

## The retry primitive

Every level transition uses the same retry mechanism. The primitive
is parameterized by:

- `parent_node` — the node whose decomposition failed coverage.
- `prior_attempt` — the IR fragment the previous call produced.
- `gap` — the specific structural violation (uncovered bytes,
  flattening, overlap, etc.).
- `level` — selects the prompt template.

### Fresh-agent, stateless per attempt

The framework launches a **new agent** (new conversation, no chat
history) for every retry attempt. The model sees only what the
framework chooses to put in the prompt. State lives entirely in the
framework, not in the LLM's conversation context.

This matters because:

1. Small models lack the metacognition to reason across turns about
   their own prior omissions. Asking a 0.5B model *"why did you drop
   the 1?"* in a multi-turn dialogue produces noise.
2. State-as-prompt is debuggable and replayable. The audit trail
   captures the literal prompt each retry received; replay is
   deterministic.
3. Scope per call is small. A Fact-level retry sees ~10 bytes of
   source + ~50 bytes of prior IR + a 1-line gap description. That
   fits comfortably in any model's context — including the
   constrained-deployment regime this framework targets.

### Prompt shape

The prompt is **source-shaped, not framework-shaped.** The model is
not asked to know about ADJ02 invariants, the hierarchy taxonomy, or
correlation vectors. The prompt presents:

- The parent's source text, in quotes.
- A natural-language description of what the previous attempt produced
  for this parent.
- A natural-language description of the gap.
- A polite request to check and produce a corrected decomposition.

Worked example for a `Fact → TypedComponent` retry on `"1 carry-on bag"`:

```
In the phrase "1 carry-on bag", a previous attempt produced:

  Fact( carry_on_bag(passenger) )

But the following part of the phrase was not accounted for: "1"
(bytes 0..1 of the phrase).

Can you check this and produce a complete decomposition?

Return JSON of the form:
  { "components": [ { "kind": "...", "span": [s, e], "term": ... } ] }
```

The prompt does **not** say "wrap in `Quantity(1, count)`" or "the
framework requires typed components." Those are framework concerns;
the model just needs to look at the text and produce a complete
decomposition.

### Schema hinting strategy

Default: terse one-line JSON shape hint, as above. If the bench shows
small models can't produce shape-correct JSON unprompted, the prompt
falls back to a fully filled template with placeholder values. This
is per-call configurable; the spec does not commit to a single
strategy. The bench (PR-6) decides.

### One gap per retry

If a parent's decomposition has multiple gaps, the framework issues
**one retry call per gap, sequentially**. Each retry agent sees only
the one gap it is trying to close. This is more focused than batching
all gaps into a single prompt, and produces cleaner audit-trail
records — one (prompt, response, outcome) triple per gap.

### Per-level retry budget and hard reject

Each level has its own configurable budget. Default: **3 attempts per
level per parent node**. On budget exhaustion:

- **Hard reject the parent node.** The framework does not fall back
  to hand-built IR, `Tentative` trust, or any other escape hatch.
  Per [feedback_adjudication_total_coverage_hard_gate](../../memory/feedback_adjudication_total_coverage_hard_gate.md).
- The parent's failure propagates upward: its ancestor's coverage
  check fails (because this parent didn't produce a valid
  decomposition for its span).
- The overall decomposition terminates with a typed `Failed { level,
  parent_node_id, last_gap }` result. The audit trail records every
  attempt made before the budget was exhausted.

### Loop in pseudocode

```
for level in [Document→Sentence, Sentence→Phrase, Phrase→Claim, Fact→TypedComp]:
    for parent_node in nodes_at_level(level):
        attempt = 0
        while attempt < budget_for(level):
            prior        = current_children(parent_node)
            check        = check_coverage(parent_node, prior)
            if check.is_pass():
                break
            for gap in check.gaps():
                new_children = retry_decompose(
                    parent_node = parent_node,
                    prior       = prior,
                    gap         = gap,
                    level       = level,
                )
                replace_children_in_gap_region(parent_node, gap, new_children)
                attempt += 1
                if attempt >= budget_for(level):
                    return Failed{level, parent_node, gap}
return Pass{ir_document}
```

## Correlation vector

Every IR object carries a `correlation_id: CorrelationId`. The IDs
form a tree matching the `Contains`-edge hierarchy:

```
CorrelationId := opaque string, unique within document

every node has exactly one correlation_id.
each node's correlation_id is recorded alongside its span
in the audit trail.
```

**Granularity: per-span**, not per-byte. Every node at every level
gets a single CorrelationId. Byte-level provenance is derivable from
the `(correlation_id, span)` pair by recursion (the parent edge gives
the path).

### Propagation invariant

Every downstream artifact derived from an IR node — engine clauses
emitted by `adjudication-connector`, retry-attempt records in the
audit trail, verdict citations in the engine's proof DAG — **MUST**
carry the originating CorrelationIds.

A check (sibling to coverage) verifies:

1. Every CorrelationId in a downstream artifact refers to an existing
   IR node.
2. Every IR node's CorrelationId appears in at least one downstream
   artifact (or, for the leaves of a non-engine run, in the verdict
   trace).

This makes *"trace this verdict back to source bytes"* a structural
walk, not a manual exercise.

Existing partial machinery — `IRNode.id`, source spans, per-clause
provenance in [adjudication-connector](../packages/rust/adjudication-connector)
(ADJ16 step 1) — is useful precursor but not equivalent. CorrelationId
is one ID space flowing across **all** stages, not separate IDs per
stage.

## Audit-trail extensions

Each retry attempt produces a record:

```
RetryRecord := {
    level:                   DecompLevel,
    parent_node_id:          NodeId,
    parent_correlation_id:   CorrelationId,
    attempt:                 non-negative integer,
    prompt:                  string,             // verbatim prompt sent
    response:                string,             // verbatim response received
    gap:                     GapDescription,     // what triggered this retry
    outcome:                 Passed | RetryAgain | Exhausted,
    model:                   string,             // model identifier
    prompt_hash:             string,             // FNV-1a of prompt body, replay-aligned
    timestamp:               ISO-8601,
}
```

The audit trail contains the full sequence of retry records for every
level transition the framework attempted. A reviewer can replay any
failure: exact prompt, exact response, exact gap. The existing
`adjudication-audit-trail::DialogueResponse` shape is **superseded**
by this richer form; the migration plan retires it.

## What this supersedes and demotes

### Superseded (this spec replaces them)

| Spec | What's replaced |
|---|---|
| [ADJ01 v3](ADJ01-adjudication-ir-grammar.md) | The IR skeleton. Document/Sentence/Phrase become explicit kinds. v3's multi-DAG cross-cut edges (`Excepts`, `Refers`, `Cites`, `Mentions`, `SameAs`, `Supersedes`, etc.) carry forward unchanged. v3's `Section` and `TextRun` are retired in favor of the explicit level kinds. |
| [ADJ02 v3](ADJ02-coverage-checker.md) | The flat tiling check now applies at every level boundary, not only at the document root. The acyclicity invariant carries forward unchanged across the multi-DAG cross-cuts. |
| [ADJ06](ADJ06-clarification-dialogue.md) | The escalation ladder (rung 0/1/2/3) and Socratic question taxonomy are replaced by the single fresh-agent retry primitive. Rung-1 (EHR re-query) and rung-3 (human expert) become future work; this spec covers rung-0 (re-prompt) only, in a stricter fresh-agent-per-attempt form. |

### Demoted to audit-only (recorded, no longer gating)

| Spec | New role |
|---|---|
| [ADJ03 polarity/modality](ADJ03-polarity-modality-checker.md) | Polarity surfaces as a level-4 `TypedComponent` kind. The structural slot is gated (a Fact with negation cues must produce a `Polarity` child); the *value* the model assigns is not gated. The framework does not enforce "the model picked the right polarity." Modality, similarly, becomes an optional level-4 component the model may emit. |
| [ADJ22 typed-quantity coverage](ADJ22-typed-quantity-coverage.md) | Folded into level-4 coverage as the `Quantity` TypedComponent requirement. The standalone `adjudication-coverage::check_typed_quantity_coverage` becomes a no-op once the migration completes. |

### Paused, not deleted

- [ADJ04 round-trip](ADJ04-round-trip-checker.md) — advisory; will be
  re-evaluated once the foundation is stable.
- [ADJ05 adversarial](ADJ05-adversarial-verifier.md) — advisory;
  re-evaluate later.
- [ADJ14 rulebook elicitation](ADJ14-rule-elicitation.md),
  [ADJ15](ADJ15-recursive-rulebook-empirical-results.md),
  [ADJ17](ADJ17-adversarial-rulebook-empirical-results.md) — paused,
  no further development until decomposition is solid.
- [ADJ16 engine-programmatic adjudication](ADJ16-engine-programmatic-adjudication.md) — paused.
- [ADJ18](ADJ18-broadened-tsa-empirical-bench.md),
  [ADJ19](ADJ19-cross-domain-empirical-bench.md),
  [ADJ20](ADJ20-fact-sheets-and-pipeline-reorder.md) — no new bench
  or fact-sheet work until the foundation bench (PR-6 below) shows
  per-level coverage holding across the 8 × 5 matrix.

## Migration plan

This spec lands first, alone, with no code changes. After it merges,
implementation lands in sequenced PRs, each individually testable and
small in scope:

1. ✅ **PR-1 — new IR types** ([#3089](https://github.com/adhithyan15/coding-adventures/pull/3089), merged 2026-05-13).
   Added `Document`, `Sentence`, `Phrase`, `Question` skeleton kinds
   and `Quantity` / `Polarity` / `Predicate` / `Comparator` /
   `TimeRef` / `Modifier` typed-component kinds to
   [adjudication-ir](../packages/rust/adjudication-ir). Additive
   only.
2. ✅ **PR-2 — per-level coverage check** ([#3092](https://github.com/adhithyan15/coding-adventures/pull/3092), merged 2026-05-13).
   `check_hierarchical_coverage` lands in
   [adjudication-coverage](../packages/rust/adjudication-coverage)
   alongside the no-flattening rule. 13 new tests covering every
   gap kind.
3. ✅ **PR-3 — fresh-agent retry primitive** ([#3096](https://github.com/adhithyan15/coding-adventures/pull/3096), merged 2026-05-13).
   `retry_decompose_level` in
   [adjudication-clarification](../packages/rust/adjudication-clarification),
   parameterized over `DecompositionLevel`. Each retry is stateless
   per attempt; the prompt is source-shaped, not framework-shaped.
4. ✅ **PR-4 — orchestrator** ([#3100](https://github.com/adhithyan15/coding-adventures/pull/3100), merged 2026-05-13).
   `decompose_hierarchical` lands in
   [adjudication-pipeline](../packages/rust/adjudication-pipeline)
   (deviating from the spec's `llm-primitives` placement to avoid a
   dependency cycle). Drives the four level-boundary dispatches +
   per-parent coverage-driven retries.
5. ✅ **PR-5 — correlation vector** ([#3103](https://github.com/adhithyan15/coding-adventures/pull/3103), merged 2026-05-13).
   `CorrelationId` type + helpers in `adjudication-ir`, propagation
   through `adjudication-connector` (per-clause `fact_correlation`
   / `rule_correlation` maps), orchestrator assigns deterministic
   IDs.
6. ✅ **PR-6 — foundation bench harness + ADJ26 methodology spec**
   ([#3107](https://github.com/adhithyan15/coding-adventures/pull/3107),
   merged 2026-05-13). Single-cell Rust driver + Python harness +
   methodology spec ready. Empirical results land in a follow-up
   data PR after the bench has run end-to-end against a live Ollama
   instance.
7. ⏳ **PR-7 — cutover** (queued, gated on ADJ26 data). Once the
   bench shows reliable per-level coverage, retire the old
   `decompose_text` flat-IR path, the legacy `Section` kind, and
   the standalone ADJ22 check. Old `adjudication-coverage`
   typed-quantity check becomes a no-op then removed. Specific
   removals depend on what the bench data justifies — premature
   cutover before validating the new flow works against real LLMs
   would risk losing useful machinery.

Each PR is small, focused, and individually testable. None depend on
work outside the decomposition flow.

## Open questions (carried as TBDs)

1. **Closed set of TypedComponent kinds.** The starting set
   (`Quantity`, `Polarity`, `Entity`, `Predicate`, `Comparator`,
   `TimeRef`, `Modifier`) is a guess based on the TSA / clinical /
   contract domains. The bench will reveal what's missing and what's
   over-engineered. Expand with empirical evidence.

2. **Phrase granularity.** "Model-determined" is the loosest workable
   definition. If small models consistently produce degenerate
   Phrases (entire sentence as one Phrase, or single-token Phrases),
   the framework may need to gate on a per-Phrase byte-length range.
   Defer until bench data.

3. **Per-level retry budget.** Default 3 per level. If the bench
   shows `Document → Sentence` almost never retries while
   `Fact → TypedComp` needs 5+, differentiate by level.

4. **JSON shape strategy.** Terse hint by default; fall back to full
   template if the bench shows small models can't produce
   shape-correct JSON unprompted. Per-call configurable.

5. **Cross-cut edges (Excepts, Refers, Cites, ...).** Carry forward
   from ADJ01 v3 unchanged. The hierarchy is the skeleton; the
   cross-cuts layer on top. May simplify the relation taxonomy once
   bench data is available.

6. **Contract-disagreement handling.** If a retry agent's response
   includes commentary like *"I don't think counts are
   measurements"*, the framework restates the gap and re-asks. The
   disagreement is recorded in the audit trail. The gate stays
   non-negotiable. See [feedback_adjudication_no_interpretive_gating](../../memory/feedback_adjudication_no_interpretive_gating.md).

7. **Per-byte vs per-span CorrelationId.** Per-span chosen for v1:
   simpler, every node carries one ID. Per-byte would be stricter but
   requires every node to carry a vector of IDs proportional to its
   span length. Revisit if the audit trail proves insufficient for
   byte-level tracing.

8. **Uncertainty / Question decomposition.** These kinds currently
   don't require level-4 decomposition. If downstream consumers
   (engine, verdict-trace UI) need typed components inside an
   Uncertainty or Question, this opens up; for now, their interior
   is left as text.

## Status

- **v1 draft landed**. This spec replaced the foundation of the
  framework. PRs 1–6 of the migration plan are merged
  (2026-05-13); the framework's IR, coverage check, retry
  primitive, orchestrator, and correlation-vector pass are all in
  place. The ADJ26 foundation-bench harness is ready.
- **PR-7 (cutover) remains queued**. The legacy `decompose_text`
  flat-IR path, the `Section` kind, and the standalone ADJ22
  check stay in place until the ADJ26 empirical-results PR
  demonstrates the new flow's reliability against real LLMs.
- **No paused workstream resumes** (ADJ14 / 15 / 16 / 17 / 18 / 19
  / 20) until ADJ26's empirical results land and meet the gating
  condition the data PR proposes.

## See also

- [project_total_coverage_forces_reasoning](../../memory/project_total_coverage_forces_reasoning.md)
  — the hypothesis this spec structurally enforces.
- [feedback_adjudication_total_coverage_hard_gate](../../memory/feedback_adjudication_total_coverage_hard_gate.md)
  — coverage as hard reject; no `Tentative` bypass.
- [feedback_adjudication_no_interpretive_gating](../../memory/feedback_adjudication_no_interpretive_gating.md)
  — gates on representation, not interpretation.
- [feedback_adjudication_correlation_vector](../../memory/feedback_adjudication_correlation_vector.md)
  — source-byte → verdict traceability.
- [feedback_adjudication_decompose_text_focus](../../memory/feedback_adjudication_decompose_text_focus.md)
  — focus discipline: decompose_text only until foundation is solid.
- **Superseded**: [ADJ01](ADJ01-adjudication-ir-grammar.md),
  [ADJ02](ADJ02-coverage-checker.md),
  [ADJ06](ADJ06-clarification-dialogue.md).
- **Demoted**: [ADJ03](ADJ03-polarity-modality-checker.md),
  [ADJ22](ADJ22-typed-quantity-coverage.md).
- **Paused**: [ADJ14](ADJ14-rule-elicitation.md),
  [ADJ15](ADJ15-recursive-rulebook-empirical-results.md),
  [ADJ16](ADJ16-engine-programmatic-adjudication.md),
  [ADJ17](ADJ17-adversarial-rulebook-empirical-results.md),
  [ADJ18](ADJ18-broadened-tsa-empirical-bench.md),
  [ADJ19](ADJ19-cross-domain-empirical-bench.md),
  [ADJ20](ADJ20-fact-sheets-and-pipeline-reorder.md).

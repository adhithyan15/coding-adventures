# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-05-11 — ADJ03 v2 structural propagation rewrite

### Replaced

The entire v0.1.0 NegEx / ConText-style trigger-detection check is
replaced with a **structural propagation consistency check** over
the v2 IR (per
[ADJ03 v2](../../../specs/ADJ03-polarity-modality-checker.md)).

What's gone:

- `TriggerClass` enum (Negation / Hedge / TemporalPast / etc.)
- `TriggerDirection` (Forward / Backward / Bidirectional)
- `ScopeRule` (UntilSentenceEnd / UntilPunctuation / etc.)
- `Trigger` struct with surface / direction / scope fields
- `TriggerTaxonomy` and the `clinical_default()` English trigger
  list
- Per-trigger scope detection
- All NegEx / ConText machinery

What replaces them:

- `check_propagation(ir_doc) -> PropagationResult` — walks every
  leaf, computes effective polarity / modality via ancestor lookup,
  compares declared vs effective.
- `PropagationResult { violations, warnings }` — violations gate
  the adjudication; warnings are surfaced for review but do not
  gate by default.
- `PropagationViolation` (gating): `InheritChainUnresolved` (every
  ancestor declared Inherit), `RuledOutMustBeAffirmed` (the
  ADJ01 hard rule).
- `PropagationWarning` (non-gating):
  `LeafOverridesAncestorPolarity`, `LeafOverridesAncestorModality`
  — surface legitimate-override cases for audit review (e.g.,
  "denies X, Y; admits Z" structures in real prose).
- Effective polarity / modality resolution with memoization for
  `O(N)` total cost across the IR.

### Algorithm

Pure structural. No LLM call. No language-specific knowledge.
Walks the part_of tree to resolve `Inherit` values; compares each
non-TextRun leaf's declared values against the propagated effective
values.

### Default policy: warn-do-not-block

The propagation check produces warnings for declared overrides
because legitimate overrides exist in real documents. A deployment
can configure warnings-as-errors for strict semantics; the check
itself separates the two so callers choose.

### Tests

10 tests cover:

- Concrete-polarity leaf with no ancestor — Pass
- Child inheriting from parent's Denied polarity — Pass
- Child overriding parent's polarity — Pass with warning
- RuledOut + Affirmed — Pass (the canonical RuledOut shape)
- RuledOut + Denied — Violation (the hard rule)
- All-Inherit chain — InheritChainUnresolved violation
- Multi-level inheritance through TextRuns — Pass
- "Denies X, Y; admits Z" pattern — Pass with one polarity-override
  warning on the affirmed item
- Intermediate TextRun override — no warnings (only leaves emit them)
- Discarded nodes — skipped per ADJ01

`cargo build / test / clippy --no-deps` clean.

## [0.1.0] - 2026-05-11

### Added

- `TriggerClass` enum: `Negation`, `Hedge`, `TemporalPast`,
  `TemporalPresent`, `TemporalFuture`, `Hypothetical`,
  `FamilyHistory`, `RuleOut`, `Subject` — mirrors the controlled
  vocabulary from ADJ03.
- `TriggerDirection` enum (`Forward`, `Backward`, `Bidirectional`)
  controlling which side of the trigger its scope applies to.
- `ScopeRule` enum (`UntilSentenceEnd`, `UntilPunctuation(chars)`,
  `UntilTermination(list)`, `UntilTokenCount(n)`).
- `Trigger { class, surface, direction, scope }` — one entry in
  the taxonomy.
- `TriggerTaxonomy` — a versioned configuration carrying the
  trigger list plus the term-tokenizer settings.
  - `clinical_default()` — NegEx/ConText-style English clinical
    taxonomy.
- `Violation { node_id, trigger_class, required, actual, ... }`
  describes a single check failure with enough detail for
  clarification (ADJ06) to render a question.
- `check_polarity_modality(doc, ir_doc, taxonomy)` — runs the
  per-node check from ADJ03. Pure (no LLM at check time);
  embarrassingly parallel across nodes.
- 13 tests covering: the canonical "denies chest pain" case,
  "father had MI" family-history detection, "ruled out by CT"
  RuledOut vs Denied distinction, hedge detection ("possibly"),
  past-tense temporality ("history of"), permitted cases where
  the trigger's scope does not cover the term, multi-clause
  scope termination via "however"/"but", and end-of-sentence
  scope termination.

### Notes

This is the Rust reference implementation of [`ADJ03`](../../../specs/ADJ03-polarity-modality-checker.md).
The taxonomy is configurable; deployments add domain-specific
triggers via `TriggerTaxonomy::add`. The check is deliberately
per-node (cross-node scope is `ADJ03a`, deferred).

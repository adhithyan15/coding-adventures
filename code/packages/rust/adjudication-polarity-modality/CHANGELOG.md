# Changelog

All notable changes to this project will be documented in this file.

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

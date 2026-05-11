# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-05-11

### Added

- `Probability` enum with two variants: `Certain` (semantic 1.0, recognized
  structurally for the LP19 short-circuit) and `Value(f64)` for genuine
  probabilities in `[0, 1]`.
- `Fact { id, term, probability }` carrying a stable `FactId` and a
  probability that defaults to `Certain`.
- `Rule { id, head, body, probability }` with `BodyLiteral::{Pos, Neg}`
  body literals (positive goals and negation-as-failure).
- `KnowledgeBase` — an indexed collection of Facts and Rules. Looks up
  by head functor/arity for fast clause selection during search.
- `SearchMode` enum: `FindFirst`, `EnumerateAll`, `AutoDetect`.
- `is_all_certain()` on `KnowledgeBase` — the precondition for the
  LP19 short-circuit. The implementation walks every Fact and Rule once
  and is `O(|KB|)`.
- `find_first(query, kb)` — deterministic SLD-style resolution over the
  KB. Returns the first successful `Substitution` or `None`. Uses the
  unification from `logic-core` and the KB's clause index for clause
  selection.
- 14 tests covering: deterministic facts, rules with bodies, multiple
  clauses with backtracking, the all-Certain short-circuit detection,
  rejection of anonymous probabilistic clauses (well-formedness), and a
  small "family relations" worked example used by the LP layer's
  educational specs.

### Scope

This is the first slice of [`LP19`](../../../specs/LP19-probabilistic-logic-core.md).
Subsequent slices will add:

- Proof DAG construction (return all successful derivations, not just
  the first).
- `EnumerateAll` and `AutoDetect` mode implementations.
- Naïve weighted-model-counting backend over the proof DAG's induced
  Boolean formula.
- d-DNNF / SDD compilation (LP19a).

The current slice deliberately limits itself to **deterministic
find-first search** so that the foundation can be reviewed before the
probabilistic backend is added.

### Notes

The Python reference at `code/packages/python/logic-engine` remains the
canonical interpretation. The Rust crate currently is a strict subset
of the Python API surface; subsequent PRs expand to parity.

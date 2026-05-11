# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-05-11

### Added

- `LoweringError` enum naming every reason an IR document fails to
  lower to a logic-engine knowledge base.
- `lower_to_kb(ir_doc)` — transforms an `IRDocument` into a
  `KnowledgeBase`, applying the lowering rules from ADJ11:
  - Fact nodes with `polarity = Affirmed` become `logic_engine::Fact`s
    with `Probability::Certain`.
  - Fact nodes with `polarity = Denied` become `Rule { head: term,
    body: [Neg(term)], probability: Certain }` — the polarity-to-clause
    translation under negation-as-failure.
  - Rule nodes are decoded via the term-encoded subtype convention:
    - `definitional(head, [body...])` → `Rule { probability: Certain }`.
    - `probabilistic(p, head, [body...])` → `Rule { probability: Value(p) }`.
    - `constraint([body...])` → `Rule` with synthetic `_constraint(c_N)` head.
    - `default(head, [body...], [exceptions...])` → `Rule` with `Pos` body
      and `Neg` exception literals.
  - Query nodes are collected separately (not added to the KB).
  - Uncertainty / Exception / Discarded nodes are skipped at the
    engine level (they participate in clarification, audit trail, and
    rule priority, but do not feed clauses to LP19).
- `extract_queries(ir_doc)` — returns the `Term` of every Query node
  in the document. The caller decides which queries to run; typically
  there is exactly one.
- `run_adjudication(ir_doc)` — convenience wrapper that lowers the
  document, runs each query under `SearchMode::AutoDetect`, and
  returns an `AdjudicationResult` per query.
- `AdjudicationResult` carries the query term, the engine's
  `SearchResult` (deterministic substitution or probabilistic
  proof-DAG + probability), and a reference to the IR document for
  audit-trail composition.
- 12 tests covering each lowering rule, the deterministic and
  probabilistic engine paths, and structural error reporting on
  malformed Rule terms.

### Scope

This is the first slice of the **adjudication-connector** crate,
which implements [`ADJ11`](../../../specs/ADJ11-problog-connector.md)
on top of `adjudication-ir` and `logic-engine`. It is the layer that
makes the framework end-to-end runnable.

Not in this slice (planned follow-ups):

- JSON / wire-format loading of IR documents.
- The `as_of` priority semantics for Rule selection (a single
  knowledge base today).
- Engine integration with `LP19c` (conditional probability with
  evidence) — currently `run_adjudication` runs unconditional
  queries.
- ADJ checker passes (ADJ02–05) — these belong in their own crates
  that consume this connector's output.

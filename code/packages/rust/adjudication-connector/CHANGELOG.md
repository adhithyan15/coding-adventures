# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-05-12

### Added

- `TrustTier` enum (`Tentative` / `Reviewed` / `Authoritative`).
  Mirrors `adjudication_rulebook::RulebookTrust` deliberately so the
  connector does not depend upward on adjudication-rulebook —
  conversion happens at the call site.
- `ClauseProvenance { source_rulebook_id, trust_tier }` —
  per-clause attribution record. Attached to every Fact and every
  Rule that the new provenance-aware lowering emits.
- `LoweredKb { kb, fact_provenance, rule_provenance }` — a
  KnowledgeBase plus parallel attribution maps. Replaces the bare
  `KnowledgeBase` return type when callers need to trace clauses
  back to their source.
- `lower_to_kb_with_provenance(ir_doc, provenance)` — same lowering
  as `lower_to_kb`, but records provenance for every emitted Fact
  ID and Rule ID. Edges (other than `Contains`) are also attributed.
  All clauses from one call share the passed-in provenance — the
  *one rulebook in, one provenance out* pattern.
- `LoweredKb::extend(other)` — merge another `LoweredKb` into self,
  reinserting clauses with fresh IDs and re-keying provenance under
  the new IDs. Use this to combine adversarially-elicited rulebooks
  into one KB while preserving per-clause attribution.
- `LoweredKb::provenance_for_fact(id)` /
  `LoweredKb::provenance_for_rule(id)` — lookup helpers used by the
  audit-trail and the (future) disputed-answer resolution layer.
- 13 new unit tests covering: trust-tier string round-trip;
  affirmed fact attribution; denied-fact-as-rule attribution; rule
  attribution; edge attribution (with the `Contains` skip);
  multi-source `extend` preserving origins; lookup by ID; coverage
  of every Rule subtype; error propagation through the
  provenance-aware path.

### Rationale (ADJ16 step 1)

[ADJ16](../../../specs/ADJ16-engine-programmatic-adjudication.md)
proposes replacing the LLM answer-time call with a deterministic
engine that runs a compiled Prolog/ProbLog program over the
rulebook + facts. For the proof DAG returned by the engine to be
auditable, every Fact and every Rule cited in the proof must be
traceable back to (a) which rulebook it came from and (b) what
trust level that rulebook carried at lowering time. This release
adds that pass-through without touching `logic-engine` — the
attribution is held in side-tables keyed by clause ID, so existing
KnowledgeBase consumers keep working unchanged. Step 2 (the
pipeline `AnswerMode::Engine` flag) and step 3 (`DisputedAnswer`
shape) consume these attribution maps.

### Compatibility

`lower_to_kb`, `extract_queries`, `run_adjudication`, and
`AdjudicationResult` are unchanged in shape and semantics. Callers
that don't need attribution should continue to use them.

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

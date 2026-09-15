- `geology/fossil-preservation-subtype.adj` (new) — a sibling to the already-shipped
  `fossil-preservation-type.adj` (`fossil_preservation_type(type, description)`, THREE peer-level
  preservation structures: mold, cast, trace_fossil): a new
  `fossil_preservation_subtype(subtype, parent_type, description)` table names a SPECIFIC KIND of
  one of those three structures, decoded from the `steinkern` definition already quoted in full
  inside `fossil-preservation-type.adj`'s own header — no new WebFetch. That table deliberately
  excludes `steinkern` as a fourth peer row because the SAME source page frames it as a specific
  kind of `cast` (an internal cast), not a fourth preservation structure — a classificatory/scope
  decision, not a correctness one. One row: steinkern → cast. New e2e test file
  `facts_fossilpreservationsubtype_e2e.rs` (3 tests: forward recall with citation, backward recall
  from a bound parent type, honest abstention on a peer type). New manifest objective
  `adj.science.3to5.fossil_preservation_subtype`, matching `fossil-preservation-type.adj`'s own
  precedent of having one (the same shape already used for `comet-part.adj` →
  `comet-tail-type.adj`).

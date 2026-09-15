- `geology/fossil-preservation-type.adj` (new) -- a new `fossil_preservation_type(type,
  description)` table names three preservation STRUCTURES a fossil can be found as
  (mold->three_dimensional_impression_of_all_or_part_of_a_body_fossil_or_trace_fossil,
  cast->replica_of_an_organism_or_a_trace_produced_by_the_infilling_of_a_natural_mold,
  trace_fossil->consists_of_the_evidence_of_living_organisms_but_not_the_actual_organism_itself),
  quoted verbatim from the National Park Service's "Mold Casts and Steinkerns" article --
  `trust authoritative`, the same tier `volcano-type.adj` (a sibling library in this directory,
  USGS-sourced) already uses. Distinct from the sibling `fossil-formation-type.adj`, which
  names three FORMATION MECHANISMS from a different source (Ducksters, consensus) and
  deliberately bundles mold and cast into one coarse `cast_or_mold` row since its own source
  only gives that pairing a single combined sentence -- this table instead refines that
  pairing using a more detailed primary source that gives mold and cast their own separate
  defining sentences; the two tables answer different questions (how did it form? vs. what
  shape is it?) and neither supersedes the other. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -riE "mold_fossil|cast_fossil|trace_fossil|
  fossil_type"` across `adj-facts-stdlib/` found zero hits. WebFetch-verified twice -- the
  second pass pulled the page's glossary-vs-prose structure, confirming `mold`/`cast` come
  from the formal glossary and `trace_fossil` from the introduction, and that none of the
  three quoted sentences is truncated or bundles in an example the way a rejected row would.
  Honest abstention on `steinkern`: a real, well-documented term the same page defines with
  its own clean sentence, but the page itself frames a steinkern as a specific KIND of cast
  (an internal cast), not a fourth preservation type alongside these three. New manifest
  objective `adj.science.3to5.fossil_preservation_type` (band 3-5, `recall` competency,
  `ngss` coverage root). New e2e test `facts_fossilpreservationtype_e2e.rs` (3 tests: direct
  recall, reverse binding, honest abstention on a real-but-subordinate term).

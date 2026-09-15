- `physics/wave-family-mechanism.adj` (new) — a sibling to the already-shipped `wave-types.adj`
  (`wave_family(wave, family)`, 10 rows, which of the two families — mechanical or electromagnetic —
  a NAMED wave belongs to). That table's own header already quotes, verbatim, a THIRD NASA sentence
  (beyond the two per-wave classification sentences already used to build all 10 rows) that states,
  in one continuous contrast, the defining MECHANISM of both families at once: "Light waves, also
  called electromagnetic waves, involve oscillations of electric and magnetic fields rather than
  oscillations of matter." New `wave_family_mechanism(family, mechanism)` table: electromagnetic →
  oscillations_of_electric_and_magnetic_fields, mechanical → oscillations_of_matter. Unlike its
  parent, this sibling is keyed by FAMILY (2 rows), not by individual WAVE (10 rows) — a coarser
  grain, since the underlying fact is itself a per-family property; honest abstention on any
  individual wave name (that belongs to the parent's own key set). New e2e test file
  `facts_wavefamilymechanism_e2e.rs` (3 tests: forward recall with citation, backward recall of the
  oscillations-of-matter family, honest abstention on an individual wave name). No manifest
  objective, matching `wave-types.adj`'s own precedent. First slice of a direct re-examination of the
  4 physics/ MODERATE candidates the sweep had deprioritized — closer inspection showed this one is
  fully and cleanly grounded, just differently shaped than its parent (the "MODERATE" label reflected
  key-granularity, not quote quality).

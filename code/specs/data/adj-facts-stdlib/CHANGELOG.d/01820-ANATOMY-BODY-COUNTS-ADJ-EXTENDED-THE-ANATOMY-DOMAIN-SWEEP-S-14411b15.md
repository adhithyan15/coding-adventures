- `anatomy/body-counts.adj` (extended) — the anatomy/ domain sweep's third candidate,
  `hand_bone_group_count(group, count)` off `hand-bones.adj`, turned out to duplicate
  `body-counts.adj`'s own purpose (`body_count(structure, count)` — "how many of each major
  structure the human body has") once inspected against the REJECT precedent set for other
  count-shaped candidates this sweep (kidney-parts.adj). Rather than ship a second table with an
  identical schema, extended `body_count` with three new rows reusing the SAME already-cited
  NCBI-Bookshelf sentence `hand-bones.adj` already ships in its own header (no new WebFetch):
  carpals → 8, metacarpals → 5, phalanges → 14 ("8 carpal bones ... 5 metacarpal bones ... and 14
  phalanges"). Unlike the table's other six rows (all primary U.S. government sources), this
  source is a patient-education/teaching summary (IQWiG) hosted on NIH/NCBI Bookshelf, so these
  three rows are honestly documented as `consensus`-tier in the header prose, one rung below the
  table's `authoritative` envelope trust — matching `hand-bones.adj`'s own trust tier for the same
  sentence. Extended the existing `facts_anatomy_e2e.rs` test file with a new test covering all
  three new rows. Third slice from the anatomy/ domain sweep. 98th content slice overall.

- `anatomy/lung-size-comparison.adj` (new) — a sibling to the already-shipped `lung-lobes.adj`
  (`lung_lobe_count(lung, count)`, right_lung → 3, left_lung → 2). That table's own header already
  quotes, verbatim, the NCI SEER sentence that fixes the right lung's lobe count AND, in the same
  breath, names three comparative descriptors for the right lung relative to the left — a fact the
  lung/count schema had no room for: "The right lung is shorter, broader, and has a greater volume
  than the left lung." New `lung_size_comparison(lung, comparison)` table decodes those three
  descriptors as the right lung's own rows: shorter, broader, greater_volume. Confirmed via
  discipline #30 that no other table already covers this comparison text as queryable facts. The
  source states these comparisons only in the direction right-relative-to-left, so `left_lung` is
  deliberately left unrowed rather than inverted into a guessed opposite claim the source never
  makes — honest abstention on it. New e2e test file `facts_lungsizecomparison_e2e.rs` (3 tests:
  3-answer forward recall with citation, backward recall, honest abstention). No manifest
  objective, matching `lung-lobes.adj`'s own precedent. Seventh slice from the anatomy/ domain
  sweep — steady at 176 objectives. 102nd content slice overall.

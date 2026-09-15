- `geography/ocean-deepest.adj` (new) — a sibling to the already-shipped `oceans.adj`
  (`ocean_size_rank(ocean, rank)`, ONE size-rank per ocean basin: pacific 1, atlantic 2, indian 3,
  southern 4, arctic 5). That table's own `source` field already quotes a SECOND superlative,
  verbatim, in the very same opening sentence used to fix the Pacific's rank — "The Pacific Ocean
  is the largest and deepest of the world ocean basins" — that the rank-only schema had no room
  for. New `ocean_is_deepest(ocean, superlative)` table: pacific → deepest. Honest abstention on
  atlantic, indian, southern, and arctic, whose own cited span states only a size rank, never a
  depth claim. New e2e test file `facts_oceandeepest_e2e.rs` (3 tests: forward recall with
  citation, backward recall from a bound superlative, honest abstention on atlantic). No manifest
  objective, matching `oceans.adj`'s own precedent of not having one. Second slice from the
  geography/ sweep tranche (after `reference-line-hemisphere-split.adj`).

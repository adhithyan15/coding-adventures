- `astronomy/spectral-class-order.adj` (new) — a sibling to the already-shipped `spectral-classes.adj`
  (`spectral_class_color(spectral_class, color)`, the color NASA assigns each of the seven
  main-sequence spectral classes — o → blue, b → blue_white, a → white, f → yellow_white, g → yellow,
  k → orange, m → red). That table's own header already quotes, verbatim, ONE continuous NASA sentence
  that lists the seven class letters IN ORDER and states the direction of that order (hottest/biggest
  to coolest/smallest) — a fact the color-only schema had no room for. New
  `spectral_class_order(spectral_class, order)` table: o → 1, b → 2, a → 3, f → 4, g → 5, k → 6, m → 7.
  Full 7/7 coverage, no abstention needed among the seven classes — the same "list order → NUMBER
  column" move this directory's own `moon-phases.adj` already established. New e2e test file
  `facts_spectralclassorder_e2e.rs` (3 tests: forward recall with citation, backward recall of the
  hottest class, honest abstention on a non-class letter). No manifest objective, matching
  `spectral-classes.adj`'s own precedent. First slice from a fresh domain sweep of astronomy/ — begins
  after the physics/ domain was fully exhausted this window.

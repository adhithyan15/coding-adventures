- `biology/muscle-nuclei-count.adj` (new) — a sibling to the already-shipped `tissue-types.adj`
  (`tissue_example(tissue, example)`, a representative example/location per basic tissue type —
  muscle → cardiac_or_skeletal). That table's own header already quotes, verbatim, two NCI SEER
  sentences describing skeletal and cardiac muscle fibers in more detail than the tissue-example
  schema captures: "Skeletal muscle fibers are cylindrical, MULTINUCLEATED, striated..." and
  "Cardiac muscle has branching fibers, ONE NUCLEUS PER CELL, striations...". New
  `muscle_nuclei_count(muscle, nuclei)` table: skeletal → multinucleated, cardiac →
  single_nucleus. Honest abstention on smooth muscle (not part of this cited span) and any other
  word. This is a DIFFERENT leftover fact from the one `muscle-striation.adj` already decoded off
  the DIFFERENT parent table `muscle-types.adj` (which happens to cite the same underlying SEER
  muscle page) — not a duplicate, since each parent table is its own distinct library with its own
  citation envelope, and "striated" and "nuclei count" are independent facts sitting in the same
  source sentences. New e2e test file `facts_musclenucleicount_e2e.rs` (3 tests: forward recall
  with citation, backward recall of the single-nucleus type, honest abstention on smooth muscle).
  No manifest objective, matching `tissue-types.adj`'s own precedent of not having one. Third and
  final slice from the fresh biology/ sweep tranche, closing it out — mammal-origin.adj (PR
  #11492), cell-division-genetic-outcome.adj (PR #11500), and this slice all now shipped.

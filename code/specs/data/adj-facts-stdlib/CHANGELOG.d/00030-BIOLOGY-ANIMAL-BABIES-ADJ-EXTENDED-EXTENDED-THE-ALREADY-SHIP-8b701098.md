- `biology/animal-babies.adj` (extended) -- extended the already-shipped
  `animal_baby(animal, baby)` table from 7 to 24 rows. The SAME
  already-cited Wikipedia "List of animal names" table names dozens more
  familiar animals beyond the original seven; seventeen more were added
  (bear, cheetah, deer, eagle, elephant, fox, frog, lion, owl, penguin,
  pig, rabbit, seal, swan, tiger, whale, wolf), each choosing the single
  most common, child-recognizable baby term from its "Young" cell (the
  same judgment already used for the original seven rows, e.g. horse's
  "foal" over "colt"/"filly"). Zero new source page -- WebFetch-verified
  twice (an enumeration pass, then a targeted second pass re-fetching the
  raw "Young"-cell text for all seventeen new rows directly). The
  relation is many-to-one on purpose: several new animals genuinely
  share a baby word with each other and with existing rows in the source
  (cub for bear/cheetah/lion/tiger/wolf; calf for cattle/elephant/whale),
  so a reverse recall on a shared word now returns multiple animals at
  once. Extended the query file and e2e test
  `facts_animalbabies_e2e.rs` to 2 tests (original recall/abstain test +
  a new extension test covering a newly-added row and the reverse
  multi-animal recall on a shared word). No new manifest objective (same
  library, same objective, 167 total unchanged). Also backfilled a
  pre-existing README.md documentation gap: `animal-babies.adj` had
  never been added to the per-directory documentation table.

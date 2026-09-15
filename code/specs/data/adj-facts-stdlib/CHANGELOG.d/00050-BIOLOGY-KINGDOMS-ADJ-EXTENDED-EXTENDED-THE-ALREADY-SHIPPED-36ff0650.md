- `biology/kingdoms.adj` (extended) -- extended the already-shipped
  `kingdom_example(kingdom, example)` table from 5 to 23 rows. The SAME
  already-cited Science Notes "Kingdoms of Life in Biology" page's own
  "Examples:" lines name several organisms per kingdom, not just the one
  originally shipped per kingdom: added birds/crustaceans/sponges
  (animalia), grasses/conifers/multicellular_algae/ferns/mosses (plantae),
  yeast/molds (fungi), diatoms/dinoflagellates/ciliates/slime_molds/
  single_celled_algae (protista), and gram_positive_bacteria/
  gram_negative_bacteria/actinobacteria (bacteria). Zero new source page --
  WebFetch re-verified the live page's "Examples:" lines word-for-word
  before writing. This is the FIFTH successful extend-pattern win in this
  loop's recent run, and the first to extend a table from single-valued to
  many-valued PER KEY (many `example` values per bound `kingdom`) rather
  than adding new keys -- the reverse shape of the many-to-one pattern
  `flame-colors.adj` already established (multiple metals recalling the
  same color). Updated the query/e2e comments accordingly (a bound
  `kingdom` now recalls multiple examples). Extended e2e test
  `facts_kingdoms_e2e.rs` to 2 tests (original recall/abstain test +
  a new extension test covering the newly multi-valued kingdoms and a
  reverse recall on a newly-added example). No new manifest objective
  (same library, same objective, 167 total unchanged). Also backfilled a
  pre-existing README.md documentation gap: `kingdoms.adj` had never been
  added to the per-directory documentation table.

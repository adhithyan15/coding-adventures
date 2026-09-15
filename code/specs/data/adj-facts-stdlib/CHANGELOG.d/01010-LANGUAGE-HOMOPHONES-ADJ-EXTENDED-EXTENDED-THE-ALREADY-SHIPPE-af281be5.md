- `language/homophones.adj` (extended) — extended the already-shipped `homophone(word,
  sound_alike)` table from 3 to 5 rows. The header already quoted BOTH of `there`'s Wiktionary
  homophones ("their, they're") and BOTH of `to`'s ("too, two") when this table originally
  shipped, but only the FIRST homophone per word had ever been turned into a row — the same
  header-only zero-new-WebFetch discovery technique already proven on `synonyms.adj`,
  `opposites.adj`, and `wave-types.adj` (the TENTH extend-pattern win this window), generalized
  to a table whose header quoted MULTIPLE items per word from day one, not just the newest
  extension shape. Added `row(there, theyre)` and `row(to, two)` — `theyre` is the
  apostrophe-free atom label for "they're," since ADJ atom labels cannot carry an apostrophe;
  this mirrors the short-atom-label discipline `contraction.adj` already established (`dont`
  for "don't"). `flower` was already fully captured (its source lists only one homophone).
  WebFetch re-verified both `there`'s and `to`'s Wiktionary "Homophones" lines live before
  writing — both quotes matched exactly. Extended `homophones.query.adj` with two new queries
  documenting the multi-valued recall on `there`/`to`. New e2e test
  `homophone_extension_recalls_the_newly_added_second_sound_alikes` covering both newly added
  rows, alongside the 3 pre-existing tests (direct recall, reverse binding, honest abstention).
  No new manifest objective (same library, same objective, 167 total unchanged).

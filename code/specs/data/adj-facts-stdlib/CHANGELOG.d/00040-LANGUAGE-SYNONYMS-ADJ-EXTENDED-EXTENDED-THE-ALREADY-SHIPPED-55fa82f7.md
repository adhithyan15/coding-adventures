- `language/synonyms.adj` (extended) -- extended the already-shipped
  `synonym(word, synonym)` table from 3 to 17 rows. The header ABOVE
  already quoted the full Wiktionary "Synonyms" line for each of the
  three words when this table originally shipped -- only the FIRST
  synonym in each line had ever been turned into a row, even though the
  same already-quoted span names eight more for happy (content,
  delighted, elated, exultant, glad, joyful, jubilant, merry), three more
  for smart (capable, sophisticated, witty), and three more for quick
  (speedy, rapid, swift). This is the SIXTH successful extend-pattern win
  in this loop's recent run, and needed ZERO new WebFetch to DISCOVER the
  extra values -- they were already sitting in this table's own header
  the whole time; a live WebFetch pass was still run to re-verify each
  Wiktionary "Synonyms" line hadn't drifted before writing. Mirrors
  `kingdoms.adj`'s single-to-many-valued-per-key extension shape (a bound
  `word` now recalls multiple synonyms). Extended the query file and e2e
  test `facts_synonyms_e2e.rs` to 4 tests (original 3 + a new extension
  test). No new manifest objective (same library, same objective, 167
  total unchanged). Updated the README.md row for `synonyms.adj`.

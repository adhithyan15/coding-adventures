- `language/dolch-sight-word-level.adj` (extended, round 3) — completed the Third Grade level
  of the already-shipped `dolch_sight_word_level(word, level)` table to its FULL 41 words (from
  the first 5), re-fetching and re-parsing the SAME cited UFLI "Dolch High Frequency Word List
  Slides" PowerPoint deck the round-2 Pre-Primer extension already used, rather than a new
  source — the exact untried angle this loop's own tracking issue (#12117) backlog flagged
  ("the remaining four levels ... are each still open for the same completion treatment").
  Re-resolved the deck's true slide DISPLAY order from `ppt/presentation.xml`'s `<p:sldId>`
  list and `ppt/_rels/presentation.xml.rels` relationship map (the same method round 2
  introduced) and confirmed the resulting per-level counts (40/52/41/46/41 = 220) exactly match
  both prior sourcing passes' own already-documented counts, an independent cross-check that
  this round's re-parse is consistent. Picked Third Grade over the other three still-partial
  levels (Primer, First Grade, Second Grade) after cross-checking each level's FULL word list
  against `adj-lang`'s reserved-keyword list: First Grade's full list collides on two real
  Dolch words (`from`, `when`, both reserved grammar keywords) and Second Grade's includes a
  word with an internal apostrophe (`don't`) needing atom-quoting care this table's house style
  has not exercised — Third Grade and Primer were the only two collision-free candidates, and
  Third Grade was chosen to keep the added-row count modest (36 new rows vs. Primer's 47).
  Empirically verified all 36 newly-added Third Grade atoms against the real built CLI binary
  in a scratch table BEFORE writing the shipped file — none are reserved-keyword-shaped, all
  parse fine as plain atoms in `row(...)` position. Primer/First Grade/Second Grade are
  unchanged, still shipping only their first five words each — the table now carries 96 of the
  full 220 Dolch words (two COMPLETE levels, Pre-Primer and Third Grade, plus three still-partial
  ones), continuing the same incremental-growth shape `wave2-k8-science-foundations` already
  uses for its own gaps. Extended the query file and e2e test
  `facts_dolchsightwordlevel_e2e.rs` to 8 tests (all 6 prior tests, plus two new tests: forward
  recall on `laugh`, the 41st/last Third Grade word, and a reverse recall now checking all 41
  Third Grade words are bound answers). No new manifest objective (same library, same objective
  `adj.literacy.k2.dolch_sight_word_level`, unchanged `recall` competency).


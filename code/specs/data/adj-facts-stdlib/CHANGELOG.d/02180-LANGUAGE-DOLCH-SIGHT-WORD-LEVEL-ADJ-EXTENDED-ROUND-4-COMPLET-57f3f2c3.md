- `language/dolch-sight-word-level.adj` (extended, round 4) — completed the Primer level of
  the already-shipped `dolch_sight_word_level(word, level)` table to its FULL 52 words (from
  the first 5), re-fetching and re-parsing the SAME cited UFLI "Dolch High Frequency Word List
  Slides" PowerPoint deck rounds 2 and 3 already used, rather than a new source — the exact
  angle round 3's own backlog note flagged as the one remaining collision-free candidate
  (First Grade and Second Grade both still carry unresolved issues: First Grade's full word
  list collides with two `adj-lang` reserved grammar keywords, `from` and `when`; Second
  Grade's includes `don't`, a word with an internal apostrophe this table's house style has
  never exercised as an atom). Re-resolved the deck's true slide DISPLAY order from
  `ppt/presentation.xml`'s `<p:sldId>` list and `ppt/_rels/presentation.xml.rels` relationship
  map (the same method rounds 2 and 3 used) and confirmed the resulting per-level counts
  (40/52/41/46/41 = 220) exactly match all three prior sourcing passes' own already-documented
  counts, an independent cross-check that this round's re-parse is consistent. Independently
  re-verified Primer's FULL 52-word list against `adj-lang`'s reserved-keyword list (`prior`,
  `for`, `contributes`, `from`, `to`, `interacts`, `when`, `and`, `observe`, `uncertain`,
  `source`, `trust`, `locator`, `cites`, `consensus`, `authoritative`, `empirical`, `inferred`,
  `unattributed`) before shipping — zero collisions, and zero words with an internal apostrophe
  or other unusual atom shape. Empirically verified all 52 Primer atoms (not just the 47 new
  ones) against the real built CLI binary in a scratch table before writing the shipped file:
  all parse fine as plain atoms in `row(...)` position, and a reverse `dolch_sight_word_level
  ($W, primer)` query in that same scratch table correctly binds all 52. `they` (a real Dolch
  Primer word) is no longer an abstention case — it is now a genuine row, since Primer is
  complete — so the query/e2e abstention-on-scope case moved to `some` (a real Dolch First
  Grade word, First Grade's sixth, still outside that level's unchanged first-five subset).
  Primer is now the THIRD complete level (after Pre-Primer and Third Grade); First Grade and
  Second Grade are unchanged, still shipping only their first five words each — the table now
  carries 148 of the full 220 Dolch words (three COMPLETE levels plus two still-partial ones),
  continuing the same incremental-growth shape `wave2-k8-science-foundations` already uses for
  its own gaps. Extended the query file and e2e test `facts_dolchsightwordlevel_e2e.rs` to 10
  tests (all 8 prior tests, plus two new tests: forward recall on `please`, the 52nd/last
  Primer word, and a reverse recall now checking all 52 Primer words are bound answers) and
  retargeted the abstention-on-scope test from `they` to `some`. No new manifest objective
  (same library, same objective `adj.literacy.k2.dolch_sight_word_level`, unchanged `recall`
  competency).


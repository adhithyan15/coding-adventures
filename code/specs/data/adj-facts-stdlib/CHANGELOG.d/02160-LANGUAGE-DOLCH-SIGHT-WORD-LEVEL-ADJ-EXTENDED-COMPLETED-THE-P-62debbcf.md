- `language/dolch-sight-word-level.adj` (extended) — completed the Pre-Primer level of the
  already-shipped `dolch_sight_word_level(word, level)` table to its FULL 40 words (from the
  original 5), re-fetching and re-parsing the SAME cited UFLI "Dolch High Frequency Word List
  Slides" PowerPoint deck rather than a new source — exactly the untried angle this loop's own
  tracking issue (#12117) backlog flagged after the file first shipped ("a future round can
  extend any one level with its remaining words as a further instance of this same gap"; the raw
  220-word extraction had already been done once during the original sourcing pass, per that
  backlog note, but this round re-derived it directly from the deck rather than relying on
  unwritten prior-session state). The other four levels (Primer/First Grade/Second Grade/Third
  Grade) are unchanged, still shipping only their first five words each — the table now carries
  60 of the full 220 Dolch words (one COMPLETE level plus four still-partial ones), the same
  incremental-growth shape `wave2-k8-science-foundations` already uses for its own gaps. This
  round re-derived the deck's true slide DISPLAY order from `ppt/presentation.xml`'s `<p:sldId>`
  list and `ppt/_rels/presentation.xml.rels` relationship map (not the zip's raw filename order,
  which is NOT display order for a 226-slide deck — `slide10.xml` sorts before `slide2.xml`
  lexically) and confirmed the resulting per-level counts (40/52/41/46/41 = 220) exactly match
  the original sourcing pass's own already-documented counts, an independent cross-check that
  this round's re-parse is consistent. Empirically verified the newly-added reserved-keyword-shaped
  atom `for` (`adj-lang` reserves `to`/`and`/`for` as grammar keywords) against the real built CLI
  binary in a scratch table BEFORE writing the shipped file, alongside the rest of the 35 new
  Pre-Primer atoms; all parse fine as plain atoms in `row(...)` position. `you` (a real Dolch
  Pre-Primer word) is no longer an abstention case — it is now a genuine row, since Pre-Primer is
  complete — so the query/e2e abstention-on-scope case moved to `they` (a real Dolch Primer word,
  Primer's sixth, still outside that level's unchanged first-five subset). Extended the query file
  and e2e test `facts_dolchsightwordlevel_e2e.rs` to 6 tests (original citation/forward/abstain
  tests, retargeted abstention test, plus two new tests: forward recall on `funny`, the 40th/last
  Pre-Primer word, and a reverse recall now checking all 40 Pre-Primer words are bound answers).
  No new manifest objective (same library, same objective `adj.literacy.k2.dolch_sight_word_level`,
  unchanged `recall` competency).


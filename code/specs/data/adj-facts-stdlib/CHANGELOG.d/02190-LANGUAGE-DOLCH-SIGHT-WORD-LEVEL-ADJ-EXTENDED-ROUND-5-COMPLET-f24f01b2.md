- `language/dolch-sight-word-level.adj` (extended, round 5 — COMPLETE) — completes BOTH
  remaining Dolch levels, First Grade (to its FULL 41 words) and Second Grade (to its FULL 46
  words), in one round rather than the one-level-per-round pace rounds 2-4 used, because this
  round's empirical testing resolved both open questions rounds 3-4 had flagged but never
  actually tested against the real built CLI: First Grade's `from`/`when` (two real Dolch
  words that are also `adj-lang` reserved grammar keywords) empirically parse FINE as plain
  atoms in `row(...)` position and in query position, exactly like `to`/`and`/`for`/`if`
  before them (`adj-lang`'s reserved-word list only blocks a bare word where the grammar
  actually expects a keyword token in that syntactic position; `row(...)`'s argument position
  expects a plain atom, not a keyword, so a reserved word parses there without conflict).
  Second Grade's apostrophe-bearing `don't` genuinely DOES fail to lex as a raw atom
  (`LexerError` confirmed empirically) — resolved by reusing this stdlib's OWN
  already-established house convention rather than inventing a new one:
  `language/contraction.adj` already tables `dont` (no apostrophe) as the atom standing for
  "don't" (its own header states this explicitly), so this file reuses that exact convention.
  Re-downloaded and re-unzipped the SAME cited UFLI "Dolch High Frequency Word List Slides"
  deck, re-resolving the true slide DISPLAY order via `ppt/presentation.xml`'s `<p:sldId>`
  list (the same method rounds 2-4 introduced) and independently re-confirming the per-level
  counts (40/52/41/46/41 = 220) match all four prior sourcing passes. Every one of the 41
  First Grade and 46 Second Grade atoms (not just the newly-added ones) was empirically parsed
  as a plain atom in `row(...)` position and reverse-recalled via `dolch_sight_word_level($W,
  first_grade)` / `($W, second_grade)` against the real built CLI binary in a scratch table
  before being written here — 41/41 and 46/46 answers bound, exit code 0. ALL FIVE Dolch
  levels are now COMPLETE: this table ships the full, faithful 220-word Dolch list (up from
  148/220). The earlier scope-abstention case (`some`/`they`/`you` — a real Dolch word at a
  not-yet-completed level) no longer applies to any real Dolch word; the only remaining
  abstention case is a word outside the cited source entirely (e.g. `elephant`). Extended the
  query file and e2e test `facts_dolchsightwordlevel_e2e.rs` from 10 to 16 tests (forward
  recall on `thank`/`from`/`when` (First Grade) and `many`/`dont` (Second Grade), plus reverse
  recalls binding all 41 First Grade and all 46 Second Grade words) and removed the now-invalid
  scope-abstention test (retargeted to the `elephant`-only case, since no scope boundary
  remains). Also backfilled a pre-existing README.md staleness gap: the `language/` row still
  described Primer as its original 5-word subset even though round 4 had already completed it
  to 52/52 — now describes the full, completed state. No new manifest objective (same library,
  same objective `adj.literacy.k2.dolch_sight_word_level`, unchanged `recall` shape).


# Changelog

This directory is spec/content data, not a compiled package — entries record what content
landed and why, not a semver-tracked API.

## Unreleased

- `language/suffix-meaning.adj` (new) — a `table` naming seven common derivational suffixes and
  what each actually means, quoted verbatim from Reading Rockets' "Common Suffixes" chart
  (reproduced with permission from Corwin Press): `suffix_meaning(suffix, meaning)`, `_ful` →
  `full_of`, `_less` → `without`, `_able`/`_ible` → `is_or_can_be`, `_ness` →
  `state_of_or_condition_of`, `_en` → `made_of`, `_y` → `characterized_by`. The sibling to
  `prefix-meaning.adj`, mirroring its trailing-underscore atom-label convention with a
  LEADING-underscore label instead (`_ful` stands for "-ful", attaching to the END of a word) —
  empirically verified the `adj-lang` lexer's `IDENT` token pattern (`[a-z_][a-z0-9_]*`) accepts
  a leading underscore exactly as it accepts a trailing one, against the real built CLI in a
  scratch table before writing this file. A genuinely DIFFERENT source family from Grammarly,
  which was tried and rejected twice before for this exact angle (its suffix prose bundles
  multiple senses per paragraph rather than one clean per-suffix meaning, unlike its own prefix
  article) — Reading Rockets is an ALREADY-vetted source family in this stdlib (17 prior
  citations, all phonics/phonemic-awareness content), but this is the first citation of its
  "Common Suffixes" chart specifically, confirmed via a full-tree grep that no existing table
  names "suffix" as a row value. `_able`/`_ible` deliberately ship as two rows sharing one
  meaning atom, since the source's own chart bundles "-able, -ible" into one row — the same
  many-keys-to-one-value shape `diphthong-sound.adj`'s `oi`/`oy` → `oi_sound` pair already
  established. Deliberately excludes the chart's own purely-inflectional rows (`-ed`, `-s`/`-es`,
  comparative `-er`, superlative `-est`), each already covered by a sibling table in this stdlib
  (`past-tense-ed-sound.adj`, `plural-s-sound.adj`, `superlative-adjective-rule.adj`). Flags, but
  does not yet need to resolve, a real heteronym-in-spelling risk the source's own chart
  surfaces: the excluded comparative `-er` ("more") and a real, not-yet-shipped AGENTIVE `-er`
  ("one who") are two genuinely different suffixes sharing one spelling — the same kind of
  collision `long-vowel-team-sound.adj` resolved for `ow`/`ow_long_o` — logged in this file's own
  header for whichever future round adds the agentive sense as a new row. curl-fetched the raw
  PDF directly and read it byte-for-byte before writing this file. Honest abstention on `_ic`
  ("having characteristics of", a real suffix the same chart covers, but a near-duplicate of the
  chart's own separate, also-unshipped `-al`/`-ial` definition, avoiding the near-synonym
  pile-up `prefix-meaning.adj`'s own design note cautions against). New
  `suffix-meaning.query.adj` and `facts_suffixmeaning_e2e.rs` (5 tests: forward recall, reverse
  recall, reverse recall binding BOTH spellings that share one meaning, a second forward recall
  on a distinct semantic category, and honest abstention); new manifest objective
  `adj.literacy.3to5.suffix_meaning`. Full verification pipeline green (see PR).

- `language/long-vowel-team-sound.adj` (new) — a `table` naming four lessons of the University
  of Florida Literacy Institute (UFLI) Foundations Toolbox's "Long Vowel Teams Unit Resources
  (Lessons 84-88)" page and the single long vowel sound each spelling represents:
  `long_vowel_team_sound(spelling, sound)`, `ai`/`ay` → `long_a_sound` (lesson 84),
  `ee`/`ea`/`ey` → `long_e_sound` (lesson 85), `oa`/`ow_long_o`/`oe` → `long_o_sound` (lesson
  86), `ie`/`igh` → `long_i_sound` (lesson 87). The FOURTH fresh-WebFetch literacy instance and
  a THIRD UFLI phonics unit, genuinely distinct from `digraph-sound.adj` (a consonant digraph's
  single un-glided consonant sound) and `diphthong-sound.adj` (a glided vowel sound that moves
  between two positions within one syllable) — a long-vowel TEAM instead spells the LONG,
  un-glided sound of the first vowel in the pair. Surfaces a real `ow` HETERONYM-IN-SPELLING
  collision with `diphthong-sound.adj`'s own already-shipped `ow` row: UFLI's cited page tables
  lesson 86's "ow" as the long-O sound ("know"/"grow"), the SAME two letters
  `diphthong-sound.adj` already tables (a different UFLI unit, lesson 96) as the glided /ow/
  diphthong sound ("cow"/"how") — both real, independently source-cited facts about the
  identical spelling. Ships the lesson-86 "ow" row as the disambiguated atom `ow_long_o`
  rather than a bare `ow` — a new, source-driven atom-label convention for this stdlib, the
  same kind `prefix-meaning.adj`'s trailing-underscore `un_`/`re_`/`dis_` labels already
  established when the plain form wasn't representable — so a query on the bare atom `ow`
  against THIS table's predicate honestly ABSTAINS (this table asserts nothing about it),
  while `diphthong_sound(ow, $S)` against the sibling library is completely unaffected and
  still answers `ow_sound`, exactly as before; empirically verified BOTH tables imported
  together resolve without conflict. The other nine rows keep the plain bare-literal-spelling
  atom convention `digraph-sound.adj`/`diphthong-sound.adj` already established, confirmed via
  a full-tree grep that none of them (`ai`, `ay`, `ee`, `ea`, `ey`, `oa`, `oe`, `ie`, `igh`)
  collides with any `row` anywhere else in the stdlib — only "ow" needed special-casing.
  curl-fetched the raw HTML directly and confirmed byte-for-byte before writing this file,
  including the source's own macron-vowel notation (`ā`/`ē`/`ō`/`ī`), quoted verbatim rather
  than paraphrased into ASCII. Lesson 88 ("Vowel Teams Review 1") is deliberately NOT a row —
  the SAME review-lesson exclusion `digraph-sound.adj`/`diphthong-sound.adj` already
  established. Honest abstention on `au`/`aw` (UFLI tables it under the same different "Other
  Vowel Teams" unit, lesson 93, `diphthong-sound.adj`'s own design note already excludes) —
  WebFetch-confirmed this round by fetching that unit's own page directly, which also surfaced
  a NOT-YET-SHIPPED future collision risk worth flagging: its lesson 94 tables "ea /ĕ/, a /ŏ/"
  — a SHORT-vowel reading of "ea" this table's own lesson-85 row does not cover; not a present
  collision (no table in this stdlib ships lesson 94's content), but a future "Other Vowel
  Teams" table would need the same `ow`/`ow_long_o` disambiguation discipline. New
  `long-vowel-team-sound.query.adj` and `facts_longvowelteamsound_e2e.rs` (5 tests: forward
  recall + citation check, reverse recall binding all three long-O spellings, a second forward
  recall, a dedicated test proving the `ow` heteronym resolves honestly across both tables with
  zero conflict, and abstention on `au`); new manifest objective
  `adj.literacy.k2.long_vowel_team_sound`.

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

- `language/dolch-sight-word-level.adj` (new) — a `table` naming which of Edward W. Dolch's five
  grade-banded reading levels (Pre-Primer, Primer, First Grade, Second Grade, Third Grade) a
  common high-frequency "sight word" is first taught at: `dolch_sight_word_level(word, level)`,
  25 rows (the first five words of each level, in the source deck's own listed order). This is
  the THIRD fresh-WebFetch-sourced literacy instance shipped under `wave2-k8-literacy-foundations`,
  and genuinely distinct in KIND from `digraph-sound.adj`/`diphthong-sound.adj` (both phonics:
  a SPELLING → the SOUND it makes) — this is whole-word-recognition vocabulary instead (a WORD →
  the reading-level BAND it is first taught in), the opposite reading strategy from phonics
  decoding. Confirmed via full-tree grep that no shipped table anywhere already names `dolch` or
  `sight word`/`sight_word`. Sourced from the University of Florida Literacy Institute (UFLI)'s
  Virtual Teaching Resource Hub "Irregular and High Frequency Words" page (curl-fetched and
  confirmed byte-for-byte), which links UFLI's own "Dolch High Frequency Word List Slides"
  PowerPoint deck — downloaded directly from the same ufli.education.ufl.edu `trust authoritative`
  source family `digraph-sound.adj`/`diphthong-sound.adj` already established, and unzipped as raw
  OOXML to read each level-divider and word slide's own text run verbatim. Counting the word slides
  between each pair of level dividers gives exactly 40/52/41/46/41 (Pre-Primer/Primer/First
  Grade/Second Grade/Third Grade), summing to exactly 220 — an independent cross-check that this
  deck faithfully reproduces Dolch's own well-documented 220-word count. Ships 25 of the 220 words
  (the first five per level in the deck's own order), the same "representative subset" convention
  `food-groups.adj` already established (4-5 "example" items per category rather than the full
  source list). Two UFLI Foundations Toolbox blends-unit and Long-Vowel-Teams angles were also
  scoped this round before landing here — see below. Empirically verified reserved-keyword-shaped
  atoms (`to`, `and`, `if`) against the real built CLI binary in a scratch table BEFORE writing the
  shipped file, since the `adj-lang` lexer reserves `to`/`and`/`for` as grammar keywords elsewhere
  in the language; confirmed they parse fine as plain atoms in `row(...)` position (this stdlib's
  `homophones.adj` already had one precedent, `row (to, too)`). Honest abstention on `you` (a REAL
  Dolch Pre-Primer word, but outside this table's shipped first-five-per-level subset) and on
  `elephant` (not a Dolch word at all). New e2e test `facts_dolchsightwordlevel_e2e.rs` (5 tests);
  new manifest objective `adj.literacy.k2.dolch_sight_word_level` (`recall` competency).

- `language/diphthong-sound.adj` (new) — a `table` naming the two diphthong lessons of the
  University of Florida Literacy Institute (UFLI) Foundations Toolbox's "Diphthongs and Silent
  Letters Units (Lessons 95-98)" page and the single glided vowel sound each spelling represents:
  `diphthong_sound(diphthong, sound)`, `oi` → `oi_sound`, `oy` → `oi_sound`, `ou` → `ow_sound`,
  `ow` → `ow_sound`. This is the SECOND fresh-WebFetch-sourced literacy instance shipped under
  `wave2-k8-literacy-foundations`, and deliberately a genuinely different phonics angle from
  `digraph-sound.adj` (this loop's first) rather than an extension of it: a DIPHTHONG is one
  glided VOWEL sound, categorically distinct from a consonant digraph's one un-glided consonant
  sound. Confirmed via full-tree grep that no shipped table anywhere already names `diphthong` or
  tables `oi`/`oy`/`ou`/`ow` as a sound-mapping row value. Curl-fetched the raw UFLI page directly
  (not just an AI-summarized WebFetch) and confirmed byte-for-byte before writing this file, the
  same primary .edu source family and `trust authoritative` tier `digraph-sound.adj`/
  `r-controlled-vowel-word.adj` already established for this stdlib. The reverse direction is a
  genuine many-keys-to-one-sound shape — the source's own lesson 95 pairs BOTH `oi` and `oy` with
  the same `/oi/` sound, and lesson 96 pairs BOTH `ou` and `ow` with the same `/ow/` sound — the
  mirror image of `digraph-sound.adj`'s one-key-to-many-sounds `th` case. Lesson 97 ("Vowel Teams
  and Diphthongs Review") and lesson 98 ("kn /n/, wr /r/, mb /m/", the page's own separately
  categorized Silent Letters Unit) are deliberately NOT rows. Honest abstention on `au`/`aw`, a
  spelling many lay phonics materials also call a diphthong, but which UFLI's own broader scope
  and sequence tables under a DIFFERENT unit ("Other Vowel Teams", lesson 93) — this table stays
  scoped to exactly what the cited page itself calls a diphthong. Empirically verified against the
  real built CLI binary before writing the e2e test (forward on all four rows, both many-answer
  reverse cases, and the abstention all behave exactly as designed). New e2e test
  `facts_diphthongsound_e2e.rs` (5 tests); new manifest objective `adj.literacy.k2.diphthong_sound`
  (`recall` competency, matching this library's other plain-lookup literacy facts).

- `language/digraph-sound.adj` (new) — a `table` naming nine common consonant digraph lessons
  and the single speech sound each one represents: `digraph_sound(digraph, sound)`, `ck` →
  `k_sound`, `sh` → `sh_sound`, `th` → `th_voiced_sound` AND `th_unvoiced_sound` (an honest
  one-key/many-values reflection of the source's own voiced/unvoiced lesson split, the same
  shape `opposites.adj`/`synonyms.adj`/`homophones.adj` already established), `ch` → `ch_sound`,
  `wh` → `w_sound`, `ph` → `f_sound`, `ng` → `ng_sound`, `nk` → `nk_sound`. This is the FIRST
  fresh-WebFetch-sourced instance shipped under `wave2-k8-literacy-foundations` — the item's own
  notes record the sibling-table-mining technique as confirmed dry stdlib-wide (mathematics/,
  metrology/, calendar/, geography/, agriculture/, art/, optics/, and others all swept), leaving
  fresh sourcing as the one untried angle. Surveyed several candidate literacy angles (a
  suffix-meaning sibling to the already-shipped `prefix-meaning.adj`, common irregular sight
  words, consonant digraphs) via a full-tree grep for `suffix`/`digraph`/`sight word`/`consonant
  blend`/`diphthong` before scoping — all came back with zero table `row` hits, confirming each
  was genuinely uncovered — then picked consonant digraphs as the strongest candidate once a
  concrete, primary, already-cited-in-this-stdlib source (UFLI) confirmed a clean table-shaped
  digraph → sound span. Quoted verbatim from the University of Florida
  Literacy Institute (UFLI) Foundations Toolbox's "Digraphs Unit Resources (Lessons 42-53)"
  page — curl-fetched directly and confirmed byte-for-byte before writing this file, the same
  primary .edu source family and `trust authoritative` tier `r-controlled-vowel-word.adj` already
  established for this stdlib. Two review lessons (49 "Digraphs Review 1", 53 "Digraphs Review
  2") and two non-digraph-sound lessons in the same numbered unit (42 "FLSZ Spelling Rule", 43
  "-all, -oll, -ull") are deliberately NOT rows. Honest abstention on `qu`, a real digraph the
  same UFLI scope-and-sequence covers elsewhere but not one of these nine lessons. Empirically
  verified against the real built CLI binary before writing the e2e test (forward, reverse, the
  two-row `th` case, and the abstention all behave exactly as designed). New e2e test
  `facts_digraphsound_e2e.rs` (5 tests); new manifest objective `adj.literacy.k2.digraph_sound`
  (`recall` competency, matching this library's other plain-lookup literacy facts).

- `biology/abo-genotype-antigen.adj` (new) — a `rule` DERIVING, for each of the three ABO
  genotypes `abo-genotype-phenotype.adj` tables, which red-cell antigen(s) it ultimately
  produces: `abo_genotype_antigen(genotype, antigen)`, composing the already-shipped
  `abo_genotype_phenotype` table (`biology/abo-genotype-phenotype.adj`, OpenStax) with the
  already-shipped `blood_type_antigen` table (`biology/blood-groups.adj`, NCBI Bookshelf) on the
  literal, exact phenotype atom (`a`/`b`/`ab`) both tables share — `ia_ia` → `a_antigen`,
  `ib_ib` → `b_antigen`, `ia_ib` → `a_and_b_antigens`. This is the FOURTH instance of the
  "heredity" Major Gap (ADJ-STDLIB-COVERAGE.md §5.1/§5.2), and the FIRST to DERIVE rather than
  merely recall a heredity fact — the exact genotype→phenotype→antigen composition
  `abo-genotype-phenotype.adj`'s own header flagged by name as its most promising unexplored
  angle, rather than shipping it prematurely in that earlier, smaller slice. Zero new sourcing:
  both premises were already shipped and already cited before this file existed. Empirically
  verified in a scratch dir against the real built CLI binary before writing this file: all
  three genotypes join cleanly, and the reverse query (`? abo_genotype_antigen($G,
  a_and_b_antigens)`) correctly recovers `ia_ib`. Honest abstention on `ia_i`, `ib_i`, and `i_i`
  propagates through the join from `abo_genotype_phenotype`'s own already-documented abstention
  on those three genotypes (its cited passage never states their phenotype in continuous prose) —
  `blood_type_antigen` itself covers all four ABO phenotypes with no gap on that side, so every
  phenotype the first table can ever produce joins cleanly; the abstention is inherited, not
  independently discovered. New e2e test `facts_abogenotypeantigen_e2e.rs`; new manifest
  objective `adj.science.9to12.abo_genotype_antigen` (`infer` competency, matching this loop's
  other `rule`-derived facts). Also backfills two pre-existing README.md gaps flagged by a prior
  cycle: `biology/heredity-term.adj` and `biology/blood-groups.adj` had never had a row added to
  `adj-facts-stdlib/README.md`'s "Organized by subject, not by level" table, despite both being
  shipped and cited libraries — both rows added in this same commit.

- `biology/abo-genotype-phenotype.adj` (new) — a `table` naming, for each of three named ABO
  blood-type genotype combinations (the two homozygotes, and the one heterozygote a self-cross
  between them produces), the blood-type PHENOTYPE it produces, quoted verbatim from OpenStax
  "Concepts of Biology" §8.3 "Extensions of the Laws of Inheritance" (CC-BY 4.0, Rice University;
  WebFetch-verified against the raw server-rendered page text before writing this file, a new
  source never before cited in this stdlib): `abo_genotype_phenotype(genotype, blood_type)`,
  `ia_ia` → `a`, `ib_ib` → `b`, `ia_ib` → `ab`. This is the THIRD instance of the "heredity" Major
  Gap (ADJ-STDLIB-COVERAGE.md §5.1/§5.2), after `biology/dna-base-pairs.adj` and
  `biology/heredity-term.adj`, and the first to ground it with a real, textbook-solid worked
  example of multiple alleles and codominance rather than either a molecular fact (base-pairing)
  or curated abstract vocabulary. Composition with `heredity-term.adj` itself was checked again
  this cycle (a fresh full-tree grep confirms no shipped table anywhere has `gene`, `allele`,
  `dominant`, `recessive`, `genotype`, or `phenotype` as a row value) and remains dry, matching
  that table's own header, which already documents this exact check running dry twice before it
  shipped; this table instead grounds the SAME underlying concepts with fresh sourcing. The NCBI
  Bookshelf "The ABO blood group" chapter (already cited by `blood-groups.adj`) was tried first
  and rejected for this specific purpose: it states the general facts (three alleles, "inherited
  codominantly over O", "six possible genotypes and four possible blood types") but never
  enumerates the genotype→phenotype mapping in continuous prose (curl-verified: no "AA"/"AO"/
  "BO"/"OO" genotype-label substring anywhere in the chapter text). Deliberately reuses
  `blood-groups.adj`'s own `a`/`b`/`ab` phenotype atoms in its `blood_type` column (rather than
  inventing new ones) so a later rule could chain genotype → phenotype → antigen into a single
  derived fact — that follow-on rule is NOT shipped here, kept as a separate, smaller slice.
  Runs the relation BACKWARD as a genuine recall (binding a blood type recalls its genotype).
  Honest abstention on `ia_i`, `ib_i`, and `i_i` — three more real ABO genotypes NCBI's chapter
  confirms exist, but whose phenotype the cited OpenStax passage never states in continuous
  prose (only a separately-stated general dominance rule implies it, which this table does not
  treat as a substitute for a directly quoted fact). New e2e test
  `facts_abogenotypephenotype_e2e.rs`; new manifest objective
  `adj.science.9to12.abo_genotype_phenotype`.

- `chemistry/meniscus-reading-point.adj` (new) — a `table` naming the two basic shapes a
  liquid's meniscus can take inside a laboratory measuring vessel (concave, convex) and which
  point of its curve (lowest, highest) is actually used to take the reading, quoted verbatim
  from NIST's "Good Measurement Practice for Method of Reading a Meniscus" (GMP 3, NIST
  Interagency Report NIST.IR.7383-2019; WebFetch-verified against the raw document text before
  writing this file, a new source never before cited in this stdlib):
  `meniscus_reading_point(meniscus_shape, reading_point)`, concave → lowest_point, convex →
  highest_point. This is the SECOND instance of the "observation and measurement" Major Gap
  (ADJ-STDLIB-COVERAGE.md §5.1/§5.2), after `chemistry/measuring-tool-si-unit.adj`, and the
  first to ground it with a fresh source rather than composing two already-shipped tables —
  composition was confirmed exhaustively dry for this gap two sessions prior. Distinct from the
  already-shipped tool→quantity and tool→SI-unit tables: this covers the actual reading
  TECHNIQUE (which point of a curved liquid surface to read), a real, named source of
  systematic measurement error the same NIST document later quantifies in its own
  "Uncertainty and Error Analysis in Meniscus Readings" section. Runs the relation BACKWARD as
  a genuine recall (binding a reading point recalls its meniscus shape). Honest abstention on
  `flat` (the cited document discusses exactly two meniscus shapes, concave and convex, and
  names no third shape). The document's own named example of a convex-meniscus liquid,
  mercury, is deliberately kept in the header's prose and in the table's `cites` span rather
  than promoted to an unstated new column, since the source's own two topic sentences key the
  reading rule on the SHAPE, not on a specific liquid. New e2e test
  `facts_meniscusreadingpoint_e2e.rs`; new manifest objective
  `adj.science.6to8.meniscus_reading_point`.

- `earth-science/weathering-cause-type.adj` (new) — a `table` naming five causes of weathering
  (heating and cooling, growth of foreign crystals, collision of rock pieces, exposure to acid,
  exposure to oxygen) and which of the two basic weathering types (physical or chemical) each one
  belongs to, quoted verbatim from the U.S. National Park Service's "Weathering and Erosion"
  article (Scotts Bluff National Monument geology-in-action page; curl- and WebFetch-verified
  against the raw page HTML, a new source never before cited in this stdlib):
  `weathering_cause_type(cause, type)`, heating_and_cooling/foreign_crystal_growth/rock_collision →
  physical, acid_exposure/oxygen_exposure → chemical. This is the SECOND instance of the
  "Earth-processes" Major Gap (ADJ-STDLIB-COVERAGE.md §5.1/§5.2), after
  `geology/earth-layer-matter-behavior.adj`, and the first to ground it with a fresh source rather
  than composing two already-shipped tables — composition was confirmed exhaustively dry for this
  gap in the immediately-prior session. Grounds NGSS 4-ESS2-1 (grade 4, band 3-5) — its own
  performance expectation ("evidence of the effects of weathering or the rate of erosion") presumes
  a learner already knows weathering splits into named mechanisms, an axis nothing already shipped
  in this stdlib names (`geology/rock-types.adj` only gives rock FORMATION a combined phrase;
  `earth-science/metamorphism-cause.adj` covers deep METAMORPHISM causes, a different process).
  Runs the relation BACKWARD as a genuine one-to-many recall in both directions (three physical
  causes, two chemical causes). Honest abstention on `erosion` (the cited article's own structure
  treats it as a distinct, LATER process that moves weathering's products away, not a weathering
  type) and on `frost_wedging`/`crystal_wedging` (the article's own detailed breakdown folds that
  exact mechanism into "the growth of foreign crystals" — a separate row would double-count the
  same cause under a second name). New e2e test `facts_weatheringcausetype_e2e.rs`; new manifest
  objective `adj.science.3to5.weathering_cause_type`.

- `biology/consumer-trophic-level.adj` (new) — a `table` naming the three consumer trophic levels
  an ecosystem's food chain runs on (primary, secondary, tertiary consumer) and what each one eats,
  quoted verbatim from National Geographic Education's "Consumers" article (curl- and
  WebFetch-verified against the raw page HTML, a new source never before cited in this stdlib):
  `consumer_trophic_level(level, eats)`, primary_consumer → primary_producers, secondary_consumer →
  primary_consumers, tertiary_consumer → other_carnivores. This is the SECOND instance of the
  "ecosystems" Major Gap (ADJ-STDLIB-COVERAGE.md §5.1/§5.2), after `animal-habitat-definition.adj`,
  and the first to ground it with a fresh source rather than composing two already-shipped tables —
  a scoping pass across the three remaining single-instance K-8-science gaps (Earth-processes,
  observation/measurement, ecosystems) checked composition first (a full-tree literal-atom census
  restricted to each gap's relevant subject directories) and found every candidate pair either
  already-disqualified as a trivial column-split of siblings sharing ONE citation (e.g.
  `geology/fossil-preservation-subtype.adj` + `geology/fossil-preservation-type.adj`, and
  `geology/rock-type-formation-component.adj` + `earth-science/metamorphism-cause.adj`, both pairs
  decoding the SAME USGS/NPS sentence already used once), or too thin to generalize from (a single
  matching row). Grounds NGSS MS-LS2-3/5-LS2-1 ("matter and energy... among living... parts of an
  ecosystem") — the finer-grained "who eats whom" chain the already-shipped `food-chain-roles.adj`
  (NOAA, producer/consumer/decomposer) has no room for in its own flat `consumer` row. Honest
  abstention on `producer` and `decomposer` (the cited article's own structure keeps both OUTSIDE
  the three consumer trophic levels — its decomposer paragraph opens "In addition to consumers...");
  on `quaternary_consumer` (WebFetch-confirmed absent, two passes); and deliberately does NOT assert
  a numbered trophic-level rank, since the cited article's own body text ("the second trophic
  level") and its own embedded vocabulary glossary ("three" total positions, collapsing secondary
  and tertiary into one shared "third") disagree with each other on the count — encoding only the
  unambiguous "eats" relationship and abstaining on the level-number question entirely rather than
  picking a side. `trust consensus` (National Geographic Education, the same tier
  `animal-habitat-definition.adj`'s own `biome-type.adj` dependency already uses). New manifest
  objective `adj.science.6to8.consumer_trophic_level` (recall, NGSS, band 6-8). New e2e test
  `facts_consumertrophiclevel_e2e.rs` (3 tests: direct recall across all three levels, reverse
  binding, honest abstention on `decomposer`).
- `physics/energy-conversion-example.adj` (new) — a `table` naming four everyday processes and
  which form of energy goes IN and which comes OUT of each, grounding the U.S. EIA's own "law of
  conservation of energy" statement that energy is never created or destroyed, only changed from
  one form into another: `energy_conversion_example(process, energy_in, energy_out)`,
  wood_burning_in_fireplace → chemical/thermal, car_engine_burning_gasoline → chemical/mechanical,
  solar_photovoltaic_cell → radiant/electrical, bicycle_going_downhill → gravitational/motion. This
  is the FIRST genuine instance of the "matter/energy systems" Major Gap (ADJ-STDLIB-COVERAGE.md
  §5.1) — a prior cycle's only prior touch, the already-shipped `physics/energy-forms.adj` and
  `physics/energy-sources.adj`, named energy FORMS and SOURCES but never a CONVERSION between
  forms, so this gap had zero real instances (unlike its sibling K-8-science gaps, each of which
  already had one). Two of the four rows decode a clause already sitting unused inside
  `energy-forms.adj`'s own already-cited EIA "Forms of energy" page ("chemical energy is converted
  to thermal energy when people burn wood in a fireplace"; "the gravitational energy is converting
  to motion energy" on a bicycle going downhill); the other two come from that page's sibling
  "Laws of energy" page — a genuinely NEW fetch this cycle, not previously cited anywhere in this
  stdlib — under its own "Energy is neither created nor destroyed" heading ("A car engine burns
  gasoline, converting the chemical energy in gasoline into mechanical energy. Solar photovoltaic
  cells change radiant energy from the sun into electrical energy."). Honest abstention documented
  on a near-collision deliberately avoided: the "Forms of energy" page's fireplace sentence also
  mentions "burn gasoline in a car's engine" as a second chemical→thermal example, but the "Laws of
  energy" page gives car engines their own dedicated chemical→MECHANICAL sentence instead — both
  physically true of a real engine (motion output plus waste heat), but a single `process` key
  cannot carry two different outputs without contradiction, so this table keeps
  `car_engine_burning_gasoline` mapped to the "Laws of energy" page's dedicated sentence and scopes
  the fireplace clause to `wood_burning_in_fireplace` only. Also honestly abstains on any process
  not one of these four (e.g. a toaster). `trust authoritative` (eia.gov, the same tier
  `energy-forms.adj`/`energy-sources.adj`/`energy-form-family.adj` already use); the "Laws of
  energy" page's own footer credits its content to "National Energy Education Development Project
  (public domain)", reproduced transparently in the header rather than hidden, without changing the
  trust tier (the citable, re-fetchable artifact is the EIA .gov page itself). Added manifest
  objective `adj.science.3to5.energy_conversion_example` (recall, NGSS, band 3-5, matching NGSS
  4-PS3-4's own grade-4 energy-conversion standard). New e2e test `facts_energyconversionexample_e2e.rs`
  (3 tests: direct recall across all four rows, reverse one-to-many recall on the shared
  `chemical` energy_in, and honest abstention on `toaster`).
- `biology/animal-habitat-definition.adj` (new) — a `rule` composing the already-shipped
  `animal_habitat(animal, biome)` table (`biology/animal-habitat.adj`, National Geographic Kids)
  with the already-shipped `biome_type(biome, description)` table (`biology/biome-type.adj`,
  National Geographic Education) to DERIVE `animal_habitat_definition(animal, description)` — WHAT
  an animal's habitat actually IS, not just its name. Neither already-shipped table states outright
  "a bactrian camel's habitat is a dry area where rainfall is less than 50cm/20in per year", but
  animal-habitat's specific animal→biome assignment run through biome-type's general biome→
  definition fact answers it directly: bactrian_camel → dry_areas_where_rainfall_is_less_than_
  50_centimeters_20_inches_per_year, giraffe → open_regions_dominated_by_grass_with_a_warm_dry_
  climate. This is the FIFTH `rule`-based CAUSAL-COMPOSITION library in this loop's science
  curriculum sweep, following the same "compose two independently-citable facts into a derived,
  dual-cited conclusion" discipline `physics/heat-causes-phase-change.adj`,
  `physics/force-causes-acceleration.adj`, `geology/earth-layer-matter-behavior.adj`, and
  `chemistry/measuring-tool-si-unit.adj` already established — a SAME-DIRECTORY instance of that
  discipline (both tables already live in `biology/`), the shape `heat-causes-phase-change.adj`/
  `force-causes-acceleration.adj` first proved out, not the cross-directory shape the two most
  recent slices used — and the FIRST to ground the "ecosystems" Major Gap (ADJ-STDLIB-COVERAGE.md
  §5.1/§5.2) rather than an Earth-processes, physics-mechanics, or observation/measurement one.
  Zero new WebFetch: both composed tables were already shipped and already cited. This is also the
  cycle's answer to the standing question of whether heredity/ecosystems needed a fresh
  WebFetch-sourced table before a second composable table existed — verified against the corpus
  rather than accepted on faith: re-read all 58 then-shipped `biology/` table headers plus
  `environment/ecosystem-factor-type.adj`, found `animal-habitat.adj` + `biome-type.adj` share the
  literal biome atoms `desert`/`grassland` (2 of `animal_habitat`'s 3 rows), confirmed via
  `grep -rn "row (.*\b(desert|forest|grassland|tundra|arctic)\b" adj-facts-stdlib/` that this is the
  ONLY place those atoms collide across the whole stdlib (besides an unrelated `geography/
  oceans.adj` ranking row) — a genuine, previously-unexamined join, not a re-tried and rejected one:
  `biome-type.adj`'s own CHANGELOG entry had already checked `animal-habitat.adj` for overlap, but
  only to rule out a DUPLICATE TABLE ("animal-habitat.adj only maps individual animals to habitat
  names, never tables biome-level defining sentences"), never as a rule-composition join candidate.
  Honest abstention on `polar_bear` (`arctic`) — `animal-habitat.adj`'s own header quotes its
  source's word as the climate region "arctic", never the biome name "tundra" `biome-type.adj`
  tables; `biome-type.adj`'s own header independently confirms tundra is a real, differently-defined
  biome ("has extremely inhospitable conditions, with the lowest measured temperatures"), not merely
  a rename of "arctic" — a genuine cross-source terminology gap, so forcing the join would assert a
  definition neither cited source states, exactly the invented-mapping trap this stdlib's
  honest-abstention discipline forbids. Also checked and ruled out this cycle for heredity/
  ecosystems specifically: `biology/dna_complement` (DNA base pairs) has no shared-atom join
  partner among shipped tables; `biology/cell-division-genetic-outcome` and `biology/
  cell-division-daughter-cells` share their `process` key (mitosis/meiosis) but both decode the SAME
  underlying NHGRI sentences already jointly quoted in each other's own headers, so a rule joining
  them would be a trivial column-merge of siblings from one shared citation, not an independent
  cross-table derivation; `biology/animal-habitat` and `biology/animal-survival-adaptation` still
  share only ONE overlapping animal (`polar_bear`), too thin to generalize a rule from (reconfirming
  the prior cycle's finding); `biology/genetic-code` → `biology/amino-acids` remains a naming/decode
  BRIDGE, not a causal explanation (reconfirming the prior cycle's finding). Empirically verified in
  a scratch dir against the real built CLI binary before writing the shipped files, per the standing
  discipline. New manifest objective `adj.science.3to5.animal_habitat_definition` (band 3-5, `infer`
  competency, matching all four prior rule-derived facts' own precedent, and `biome-type.adj`'s own
  band since the derivation's ceiling is set by the more advanced source table). New e2e test
  `facts_animalhabitatdefinition_e2e.rs` (3 tests: derivation with dual citations, reverse binding,
  honest abstention).
- `engineering/engineering-design-step.adj` (new) — a `table` naming the six-step engineering
  design process (identify the problem, identify criteria and constraints, brainstorm possible
  solutions, select a design, build/test/refine, share the design), quoted verbatim from a NASA
  classroom worksheet PDF: `engineering_design_step(step, action)`. The first table in a new
  `engineering/` subject directory, and the FIRST fresh-WebFetch slice into the "engineering
  design" Major Gap (ADJ-STDLIB-COVERAGE.md §5.1/§5.2) after six prior cycles' exhaustive
  causal-composition sweep found no composable pair anywhere in the stdlib for it (the existing
  `physics/simple-machines.adj` family all decodes the SAME NASA sentence and shares no atom with
  any independently-sourced second table; "engineering design" as an NGSS practice is the
  repeatable PROCESS an engineer follows, not "name the six simple machines"). `trust
  authoritative` (nasa.gov). New manifest objective `adj.science.6to8.engineering_design_step`.
  New e2e test `facts_engineeringdesignstep_e2e.rs`.
- `science/scientific-method-step.adj` (new) — a `table` naming the seven steps of the scientific
  method (ask a question/state a hypothesis, define variables and controls, research, design the
  experiment, run it and record data, analyze the data, draw conclusions and write a report),
  quoted verbatim from NASA Space Place's student science-fair page:
  `scientific_method_step(step, action)`. The first table in a new `science/` subject directory,
  and grounds the "experiments" Major Gap, fully untouched through six prior causal-composition
  cycles (a full-tree atom-overlap census found no table anywhere addressing the scientific
  method, hypotheses, variables, or experimental design). Explicitly a DIFFERENT process from
  `engineering-design-step.adj` (scientist testing an existing idea vs. engineer building
  something new) — the two tables share zero atoms. `trust authoritative` (nasa.gov, spaceplace
  subdomain). New manifest objective `adj.science.3to5.scientific_method_step`. New e2e test
  `facts_scientificmethodstep_e2e.rs`.
- `science/scientific-model-type.adj` (new) — a `table` naming three kinds of scientific model
  (physical, conceptual, mathematical) and each one's one-sentence definition, quoted verbatim from
  a K12 LibreTexts Earth Science chapter on "Scientific Models": `scientific_model_type(type,
  definition)`. Closes the THIRD and last of the three K-8-science Major Gaps flagged as fully
  untouched (alongside "experiments" and "engineering design") — two prior cycles flagged "models"
  as harder to source than its siblings; this cycle tried and rejected three angles (a historical
  atomic-model sequence from MIT OCW — presented as separate narrative paragraphs, not a unified
  table; several NASA/NOAA/NIST "types of models" pages — dead links or no dedicated table; the
  NGSS Framework Appendix F's own definitional sentence — a flat six-noun list with no
  per-item definition) before landing on the K12 LibreTexts chapter, which gives each of three
  model types (a fourth, "computer models," is described only as a special case of mathematical
  models, not a fourth peer type) its own clean defining sentence. `trust consensus` (LibreTexts, a
  curated open-education resource, not a primary standards body). New manifest objective
  `adj.science.6to8.scientific_model_type`. New e2e test `facts_scientificmodeltype_e2e.rs`.
- `biology/heredity-term.adj` (new) — a `table` naming seven core NGSS MS-LS3 heredity vocabulary
  terms (gene, allele, dominant, recessive, genotype, phenotype, trait), each definition quoted
  verbatim from NHGRI's (National Human Genome Research Institute, genome.gov) Talking Glossary of
  Genomic and Genetic Terms: `heredity_term(term, definition)`. The FOURTH fresh-WebFetch slice
  into this item, and the first to ground "heredity" via a genuinely new primary source rather than
  the causal-composition rule-joining technique (two prior cycles had only ever checked, and
  rejected, whether an already-shipped heredity table joins another on a literal key — `biology/
  dna-base-pairs.adj` and `biology/cell-division-genetic-outcome.adj` each have no honest
  composable second table). FIRST CANDIDATE REJECTED: the classic K-8 "dominant/recessive human
  trait" worksheets (widow's peak, earlobe attachment, tongue rolling, dimples) — University of
  Utah's Genetic Science Learning Center and a University of Delaware genetics-myths page each
  independently confirm no published study supports single-gene Mendelian inheritance for any of
  these classroom staples, so shipping it would have encoded a documented genetics-education myth
  as fact. Shipped instead: NHGRI's own glossary, sidestepping the myth trap by grounding the
  VOCABULARY a correct heredity claim is built from rather than a specific (and wrong)
  trait-inheritance claim. `trust authoritative` (genome.gov). New manifest objective
  `adj.science.6to8.heredity_term`. New e2e test `facts_heredityterm_e2e.rs`.

- `chemistry/measuring-tool-si-unit.adj` (new) — a `rule` composing the already-shipped
  `measuring_tool(tool, quantity)` table (`chemistry/measuring-tools.adj`, Chemistry LibreTexts)
  with the already-shipped `si_base_unit(quantity, unit, symbol)` table
  (`metrology/si-base-units.adj`, NIST) to DERIVE `measuring_tool_si_unit(tool, unit, symbol)` —
  WHICH internationally-standardized SI unit a lab measuring tool's own reading is expressed in,
  not just which quantity it measures. Neither already-shipped table states outright "a
  thermometer's reading is standardized in kelvin", but measuring-tools' specific tool→quantity
  assignment run through si-base-units' general metrology principle for that quantity answers it
  directly: ruler → meter ("m"), balance → kilogram ("kg"), thermometer → kelvin ("K"). This is
  the FOURTH `rule`-based CAUSAL-COMPOSITION library in this loop's science curriculum sweep,
  following the same "compose two independently-citable facts into a derived, dual-cited
  conclusion" discipline `physics/heat-causes-phase-change.adj`,
  `physics/force-causes-acceleration.adj`, and `geology/earth-layer-matter-behavior.adj` already
  established — the SECOND cross-directory instance of that discipline (chemistry + metrology,
  not two files in the same subject directory), reusing the SAME cross-directory import/
  query-placement shape `earth-layer-matter-behavior.adj` already proved out, and the FIRST to
  ground the "observation and measurement" Major Gap (ADJ-STDLIB-COVERAGE.md §5.1) rather than an
  Earth-processes or physics-mechanics one. Zero new WebFetch: both composed tables were already
  shipped and already cited (and predate this composition — `si-base-units.adj` shipped in PR
  #8786, well before `measuring-tools.adj` in PR #10479, so the pairing had simply gone
  unnoticed, not been tried and rejected). Honest abstention on `graduated_cylinder` (`volume`) —
  volume is not one of `si_base_unit`'s seven keyed BASE quantities; it is a DERIVED SI unit (m³),
  independently confirmed by the already-shipped sibling `metrology/si-derived-units.adj`
  (`si_derived_unit(volume, "m3")`), so forcing the join would conflate a base quantity with a
  derived one — a real metrology distinction the cited NIST source itself draws, and exactly the
  kind of invented mapping this stdlib's honest-abstention discipline forbids. A scoping pass this
  cycle surveyed ~349 cross-table atom collisions across all 310 shipped `table` declarations in
  adj-facts-stdlib (script-assisted, the same census method the prior cycle used) looking for a
  genuine general-principle + specific-instance pair addressing one of the STILL FULLY UNTOUCHED
  causal-composition gaps (observation/measurement, experiments, models, matter/energy systems,
  heredity, ecosystems, engineering design); several near-misses were found and rejected as too
  thin or coincidental (e.g. `biology/genetic-code` → `biology/amino-acids` is a real literal-key
  join but is a naming/decode BRIDGE, not a causal explanation; `physics/energy-sources`
  and `physics/energy-form-family` both use the atom `nuclear` but in two DIFFERENT senses — an
  energy SOURCE vs. an energy FORM — composing them would conflate the two; `biology/animal-
  habitat` and `biology/animal-survival-adaptation` share only ONE overlapping animal
  (`polar_bear`), too thin a join to generalize from) before `measuring-tools.adj` +
  `si-base-units.adj` surfaced as the clean match. Grounds NGSS science-practice observation/
  measurement (a tool's raw reading is only useful to another scientist once tied to the ONE unit
  everyone has agreed to measure that quantity in — the entire point of the SI system). New
  manifest objective `adj.science.6to8.measuring_tool_si_unit` (band 6-8, `infer` competency,
  matching the three prior rule-derived facts' own precedent). New e2e test file
  `facts_measuringtoolsiunit_e2e.rs` (3 tests: derivation with dual citations, reverse binding,
  honest abstention). Empirically verified the composition in a scratch dir against the real built
  CLI before writing the shipped files. 117th content slice overall.
- `geology/earth-layer-matter-behavior.adj` (new) — a `rule` composing the already-shipped
  `has_state(layer, state)` table (`geology/earth-layers.adj`, USGS) with the already-shipped
  `matter_state(state, property)` table (`chemistry/states-of-matter.adj`, NASA GRC) to DERIVE
  `earth_layer_matter_behavior(layer, behavior)` — WHY each of Earth's internal layers behaves the
  way it does, not just WHAT state it is in. Neither already-shipped table states outright "the
  outer core takes the shape of its container", but Earth-layers' specific state assignment run
  through states-of-matter's general chemistry principle for that state answers it directly:
  outer_core (liquid) → takes_shape_of_container, inner_core (solid) → fixed_shape. This is the
  THIRD `rule`-based CAUSAL-EXPLANATION library in this loop's science curriculum sweep, following
  the same "compose two independently-citable facts into a derived, dual-cited conclusion"
  discipline `physics/heat-causes-phase-change.adj` and `physics/force-causes-acceleration.adj`
  already established — the first CROSS-DIRECTORY instance of that specific discipline (geology +
  chemistry, not two files in the same subject directory), reusing the SAME cross-directory import
  shape `earth-science/season-start-month-number.adj` already proved out. Zero new WebFetch: both
  composed tables were already shipped and already cited. Honest abstention on `crust` (`rigid`)
  and `mantle` (`semi_solid`) — neither is a literal match for any of `matter_state`'s three keyed
  states (solid/liquid/gas), so forcing either to join would assert a behavior neither already-cited
  source states. Grounds NGSS MS-ESS2-1 ("the flow of energy that drives" Earth's material cycling
  — the liquid outer core's ability to flow is exactly why it can convect and generate Earth's
  magnetic field). New manifest objective `adj.science.6to8.earth_layer_matter_behavior` (band 6-8,
  `infer` competency, matching `force-causes-acceleration.adj`'s own precedent for a rule-derived
  fact). New e2e test file `facts_earthlayermatterbehavior_e2e.rs` (3 tests: derivation with dual
  citations, reverse binding, honest abstention). Empirically verified the composition in a scratch
  dir against the real built CLI before writing the shipped files. 116th content slice overall.
- `agriculture/farm-animal-maintenance-level.adj` (new) — a sibling to the already-shipped
  `farm-animals.adj` and `farm-animal-secondary-product.adj`. Sheep's own span IS the sentence
  `farm-animals.adj` already carries as its provenance envelope's `source` field -- but only the
  trailing clause ("...they can produce wool, meat, and milk") was ever decoded into rows; the
  LEADING clause of that same sentence names a husbandry-difficulty fact about the animal itself,
  never decoded anywhere: "Sheep are low maintenance and versatile -- depending on the breed, they
  can produce wool, meat, and milk". New `farm_animal_maintenance_level(animal, level)` table
  decodes that leading clause as its own row: sheep -> low_maintenance. "Maintenance level" (how
  much care the ANIMAL needs) is categorically distinct from "product" (what the animal GIVES).
  The sentence's other unused word, "versatile," is deliberately NOT decoded into a second row: it
  is trivially re-derivable from sheep already carrying three rows across the two existing product
  tables (wool, meat, milk), so it would restate rather than add a fact. No new WebFetch -- reuses
  the same already-cited CFSPH sentence. Honest abstention on every other animal, since none of the
  other four cited spans states a husbandry-difficulty descriptor. New e2e test file
  `facts_farmanimalmaintenancelevel_e2e.rs` (3 tests: forward recall with citation, backward
  recall, honest abstention). No manifest objective, matching the parent tables' own precedent.
  115th content slice overall -- second slice of the agriculture/ domain sweep.
- `agriculture/farm-animal-product-processing.adj` (new) — a sibling to the already-shipped
  `farm-animals.adj` (`farm_animal_product(animal, product)`) and `farm-animal-secondary-product.adj`
  (`farm_animal_secondary_product(animal, product)`). Goat's own per-row provenance in
  `farm-animals.adj` already quotes, VERBATIM, a PROCESSING method applied to its already-recorded
  milk -- a fact neither existing table's schema had room for: "Goat milk may be pasteurized for
  human consumption." New `farm_animal_product_processing(animal, product, processing)` table (a
  three-column shape already precedented in this stdlib, e.g. `biology/amino-acids.adj`) decodes
  that clause as its own row: goat, milk → pasteurized. This is explicitly the SAME clause
  `farm-animal-secondary-product.adj`'s own header already flagged as a processing note rather than
  a second product, left unshipped until now. No new WebFetch -- reuses the same already-cited
  CFSPH sentence. Honest abstention on every other animal/product pair, since none of the other
  four cited spans (chicken, duck, sheep, rabbit) states a processing method. New e2e test file
  `facts_farmanimalproductprocessing_e2e.rs` (3 tests: forward recall with citation, backward
  recall, honest abstention). No manifest objective, matching the parent tables' own precedent.
  114th content slice overall -- first slice of the agriculture/ domain sweep (mathematics/,
  metrology/, and calendar/ were also swept this cycle and found zero shippable candidates; see
  loop-state.json notes for the detailed rejection reasoning).
- `geography/reference-line-hemisphere-location.adj` (new) — a sibling to the already-shipped
  `reference-line-degree.adj` (`reference_line_degree(line, degrees)`, tropic_of_cancer → 23.5,
  tropic_of_capricorn → -23.5). That table's own already-quoted NOAA NESDIS span also names, in
  plain words, which single hemisphere each tropic sits within -- a fact the numeric-degree schema
  had no room for: "One in the Northern Hemisphere called the Tropic of Cancer at +23.5° latitude
  and one in the Southern Hemisphere called the Tropic of Capricorn at − 23.5° latitude." New
  `reference_line_hemisphere_location(line, hemisphere)` table decodes that span as its own rows:
  tropic_of_cancer → northern, tropic_of_capricorn → southern. NOT a duplicate of the
  already-shipped `reference_line_hemisphere_split(line, hemispheres)` table -- that table answers
  a different question ("which pair of hemispheres does this line DIVIDE", only equator/
  prime_meridian) while this table answers "which single hemisphere does this line SIT WITHIN"
  (only the two tropics) -- the two tables are disjoint over lines and disjoint in what they
  assert. No new WebFetch -- reuses the same already-cited NOAA NESDIS sentence. Honest abstention
  on equator and prime_meridian (their own cited spans state no single containing hemisphere) and
  on the polar circles (their own spans never name a hemisphere at all). New e2e test file
  `facts_referencelinehemispherelocation_e2e.rs` (3 tests: forward recall with citation, backward
  recall, honest abstention). No manifest objective, matching `reference-line-degree.adj`'s own
  precedent. Fifth and LAST slice of the geography/ domain sweep -- **GEOGRAPHY/ domain now FULLY
  EXHAUSTED**. 113th content slice overall.
- `geography/landform-extent.adj` (new) — a sibling to the already-shipped `landforms.adj`
  (`landform_description(landform, description)`, plateau → flat_elevated, plain →
  comparatively_level). That table's own already-quoted USGS Feature Type Thesaurus spans also
  state a distinct "extent" (size) descriptor for two landforms -- a fact the descriptor-only
  schema had no room for: plateau → great_extent (from "...areas of great extent and elevation..."),
  plain → considerable_extent (from "...and of considerable extent."). "Extent" is categorically
  distinct from each landform's own descriptor (flat_elevated is about being flat/elevated, not
  size; comparatively_level is about being level, not size), so decoding it is a genuinely separate
  fact. No new WebFetch -- reuses the same already-cited USGS spans. Honest abstention on mountain,
  valley, and canyon, whose own cited spans never use the word "extent" for their size. New e2e
  test file `facts_landformextent_e2e.rs` (3 tests: forward recall with citation, backward recall,
  honest abstention). No manifest objective, matching `landforms.adj`'s own precedent. Fourth slice
  of the geography/ domain sweep. 112th content slice overall.
- `geography/landform-secondary-feature.adj` (extended) — two new rows added to the already-shipped
  sibling table (`landform_secondary_feature(landform, feature)`, off `landforms.adj`). The SAME
  already-cited USGS Feature Type Thesaurus spans this table already quotes name a further,
  previously-unused structural clause for two more landforms: `canyon` ("...narrow, deep
  depressions with steep sides...") → `steep_sides`, and `plain` ("Regions of general uniform
  slope, comparatively level and of considerable extent.") → `uniform_slope`. `canyon` now carries
  TWO rows (it already had `continuous_slope_at_bottom`), making its forward recall a genuine
  multi-answer query, the established pattern this stdlib already uses elsewhere (e.g.
  `geology/igneous-rock-type-eruption-location.adj`). No new WebFetch -- `steep_sides` reuses a
  clause already cited in this table's own header, and `uniform_slope`'s full span reuses the
  SAME quote already cited in `landforms.adj`'s header (no new source). Extended e2e test file
  `facts_landformsecondaryfeature_e2e.rs` (2 new tests: canyon multi-answer recall, plain forward
  recall with citation; 5 tests total). No manifest objective, matching this table's own
  precedent. Third slice of the geography/ domain sweep. 111th content slice overall.
- `geography/map-type-classification.adj` (new) — a sibling to the already-shipped `map-type.adj`
  (`map_type(type, description)`, topographic → shows_the_shape_of_earths_surface). That table's
  own already-quoted Geology.com span for the topographic row also classifies topographic maps as
  a KIND of map -- a fact the type/description schema had no room for: "Topographic maps are
  reference maps that show the shape of Earth's surface." New `map_type_classification(type,
  classification)` table decodes that span as its own row: topographic → reference_map. No new
  WebFetch -- reuses the same already-cited Geology.com sentence. Honest abstention on `political`
  and `physical`, whose own quotes never classify the map as a kind of map the way the topographic
  row's span does. New e2e test file `facts_maptypeclassification_e2e.rs` (3 tests: forward recall
  with citation, backward recall, honest abstention). No manifest objective, matching
  `map-type.adj`'s own precedent. Second slice of the geography/ domain sweep. 110th content slice
  overall.
- `geography/reference-line-degree.adj` (new) — a sibling to the already-shipped `reference-lines.adj`
  (`reference_line(line, marks)`, tropic_of_cancer → northernmost_sun_overhead,
  tropic_of_capricorn → southernmost_sun_overhead). That table's own already-quoted NOAA NESDIS
  span also states the precise signed numeric latitude of each tropic -- a fact the line/marks
  schema had no room for: "...Tropic of Cancer at +23.5° latitude and...Tropic of Capricorn at
  − 23.5° latitude." New `reference_line_degree(line, degrees)` table decodes that span as its own
  rows: tropic_of_cancer → 23.5, tropic_of_capricorn → -23.5 (negative/decimal number literals,
  confirmed supported by the ADJ grammar and already proven in other shipped tables e.g.
  `metrology/metric-prefixes.adj`, `physics/physical-constants.adj`). No new WebFetch -- reuses the
  same already-cited NOAA NESDIS sentence. Honest abstention on `equator`/`prime_meridian`, whose
  own 0-degree fact is already fully captured by their existing `marks` atoms
  (`zero_degrees_latitude`/`zero_degrees_longitude`) rather than left undecoded, and on
  `arctic_circle`/`antarctic_circle`, whose own quotes never state a numeric degree at all. New e2e
  test file `facts_referencelinedegree_e2e.rs` (3 tests: forward recall with citation, backward
  recall on a negative-number bind, honest abstention). No manifest objective, matching
  `reference-lines.adj`'s own precedent. First slice of the geography/ domain sweep (a fresh,
  never-before-swept domain this session — 8 existing tables, background Explore-agent sweep found
  4 STRONG + 3 MODERATE further candidates). 109th content slice overall.
- `anatomy/tooth-part-property.adj` (new) — a sibling to the already-shipped `tooth-parts.adj`
  (`tooth_part_role(part, role)`, dentin → beneath_enamel, cementum → covers_roots). That table's
  own already-quoted MedlinePlus/StatPearls spans also state a descriptive property for two
  parts -- a fact the part/role schema had no room for: "...dentin, a substance harder than
  bone." and "...cementum (calcified material covering the roots of teeth)". New
  `tooth_part_property(part, property)` table decodes those spans as their own rows:
  dentin → harder_than_bone, cementum → calcified_material. No new WebFetch -- reuses the same
  already-cited sentences. Honest abstention on `enamel`, `crown`, `pulp`, `root`, whose own
  quotes never supply a second, descriptive property beyond the role itself. New e2e test file
  `facts_toothpartproperty_e2e.rs` (3 tests: forward recall with citation, backward recall,
  honest abstention). No manifest objective, matching `tooth-parts.adj`'s own precedent. LAST of
  the 3 MODERATE anatomy/ fallback candidates -- closes out the anatomy/ domain sweep entirely
  (both STRONG and MODERATE tiers now fully addressed). 108th content slice overall.
- `anatomy/valve-alternate-name.adj` (new) — a sibling to the already-shipped `heart-valves.adj`
  (`valve_separates`) and `valve-kind.adj` (`valve_kind`). Both of those tables' own already-quoted
  NCI SEER span also names an everyday alternate name for the mitral valve -- a fact neither the
  boundary nor the kind schema had room for: "The left atrioventricular valve is the bicuspid, or
  mitral, valve." New `valve_alternate_name(valve, alt_name)` table decodes that span as its own
  row: mitral → bicuspid. No new WebFetch -- reuses the same already-cited NCI SEER sentence.
  Honest abstention on `tricuspid`, `pulmonary`, and `aortic`, whose own quotes never supply a
  second name. New e2e test file `facts_valvealternatename_e2e.rs` (3 tests: forward recall with
  citation, backward recall, honest abstention). No manifest objective, matching sibling tables'
  own precedent. Second of the 3 MODERATE anatomy/ fallback candidates. 107th content slice
  overall.
- `anatomy/skin-layer-alt-name.adj` (new) — a sibling to the already-shipped `skin-layers.adj`
  (`skin_layer_property(layer, property)`) and `skin-layer-function.adj`
  (`skin_layer_function(layer, function)`). One of those tables' own already-quoted NCI SEER spans
  also names an everyday alternate name for a layer -- a fact neither the property nor the function
  schema had room for: "The subcutis is also known as the hypodermis or subcutaneous layer, and
  functions as both an insulator...". New `skin_layer_alt_name(layer, alt_name)` table decodes that
  span as its own row: subcutaneous → hypodermis. No new WebFetch -- reuses the same already-cited
  NCI SEER sentence. Honest abstention on `epidermis`, a real, already-tabled skin layer whose own
  quote never supplies an alternate name (likewise `dermis`). New e2e test file
  `facts_skinlayeraltname_e2e.rs` (3 tests: forward recall with citation, backward recall, honest
  abstention). No manifest objective, matching the sibling tables' own precedent. First of the 3
  MODERATE anatomy/ fallback candidates, taken up after the STRONG candidate set was exhausted.
  106th content slice overall.
- `anatomy/skin-layer-function.adj` (new) — a sibling to the already-shipped `skin-layers.adj`
  (`skin_layer_property(layer, property)`, epidermis → outermost, dermis → thickest,
  subcutaneous → fat). That table maps each layer to a single positional/compositional
  descriptor, but two of that table's own already-quoted NCI SEER spans also state what the layer
  actually DOES — a fact the layer/property schema had no room for: "...protects the body from the
  environment" (epidermis) and "...functions as both an insulator...and as a shock-absorber..."
  (subcutaneous). New `skin_layer_function(layer, function)` table decodes those spans as their own
  rows: epidermis → protects_body, subcutaneous → insulator, subcutaneous → shock_absorber. No new
  WebFetch -- reuses the same already-cited NCI SEER sentences already quoted in the parent table's
  header. Honest abstention on `dermis`, a real, already-tabled skin layer whose own quote states
  only its location and thickness, never a function. New e2e test file
  `facts_skinlayerfunction_e2e.rs` (3 tests: forward recall with citation, backward recall, honest
  abstention). No manifest objective, matching `skin-layers.adj`'s own precedent. TENTH and LAST of
  the original STRONG anatomy/ domain-sweep candidates — closes out the STRONG set. 105th content
  slice overall.
- `anatomy/respiratory-part-alt-name.adj` (new) — a sibling to the already-shipped
  `respiratory-parts.adj` (`part_function(part, function)`, trachea → main_airway, alveoli →
  gas_exchange, etc.). Two of that table's own per-row quoted NCI SEER spans name not only the
  part's stated function but also an everyday alternate name for the part itself — a fact the
  part/function schema had no room for: "The trachea, commonly called the windpipe, is the main
  airway to the lungs." and "...tiny air sacs called alveoli." New
  `respiratory_part_alt_name(part, alt_name)` table decodes those two spans as their own rows:
  trachea → windpipe, alveoli → air_sacs. Confirmed via discipline #30 that no other table already
  covers a respiratory part's alternate name. Honest abstention on `larynx`, a real, already-tabled
  part whose own quote states only its function, never an everyday alternate name — likewise nose,
  pharynx, lungs, and diaphragm are deliberately left unrowed. New e2e test file
  `facts_respiratorypartaltname_e2e.rs` (3 tests: forward recall with citation, backward recall,
  honest abstention). No manifest objective, matching `respiratory-parts.adj`'s own precedent.
  Ninth slice from the anatomy/ domain sweep — steady at 176 objectives. 104th content slice
  overall.
- `anatomy/muscle-body-aspect.adj` (new) — a sibling to the already-shipped `muscle-groups.adj`
  (`muscle_region(muscle, region)`, biceps_brachii/triceps_brachii → arm, rectus_abdominis →
  abdomen, sartorius/quadriceps → thigh, gastrocnemius → leg, etc.). That table's own header
  already quotes, verbatim, Wikipedia sentences that name each muscle's body region — but three of
  those same quoted spans also name the muscle's spatial aspect within that region (anterior/
  posterior compartment, or ventral aspect), a fact the muscle/region schema had no room for. New
  `muscle_body_aspect(muscle, aspect)` table decodes those three spans as their own rows:
  rectus_abdominis → ventral, sartorius → anterior, gastrocnemius → posterior. SCOPE NOTE: the
  originally sweep-flagged candidate proposed 6 rows (adding biceps_brachii → anterior,
  triceps_brachii → posterior, quadriceps → anterior), but on fresh verification those three
  muscles' own quoted spans use everyday words like "front"/"back" rather than the anatomical
  terms this table's atoms are built from — per this stdlib's own established precedent that an
  atom must echo the source's OWN chosen word, not a same-meaning paraphrase, those three are
  honest abstentions instead, cutting the candidate down to 3 verbatim-grounded rows. New e2e test
  file `facts_musclebodyaspect_e2e.rs` (3 tests: forward recall with citation, backward recall,
  honest abstention on quadriceps). No manifest objective, matching `muscle-groups.adj`'s own
  `trust consensus` precedent. Eighth slice from the anatomy/ domain sweep — steady at 176
  objectives. 103rd content slice overall.
- `anatomy/lung-size-comparison.adj` (new) — a sibling to the already-shipped `lung-lobes.adj`
  (`lung_lobe_count(lung, count)`, right_lung → 3, left_lung → 2). That table's own header already
  quotes, verbatim, the NCI SEER sentence that fixes the right lung's lobe count AND, in the same
  breath, names three comparative descriptors for the right lung relative to the left — a fact the
  lung/count schema had no room for: "The right lung is shorter, broader, and has a greater volume
  than the left lung." New `lung_size_comparison(lung, comparison)` table decodes those three
  descriptors as the right lung's own rows: shorter, broader, greater_volume. Confirmed via
  discipline #30 that no other table already covers this comparison text as queryable facts. The
  source states these comparisons only in the direction right-relative-to-left, so `left_lung` is
  deliberately left unrowed rather than inverted into a guessed opposite claim the source never
  makes — honest abstention on it. New e2e test file `facts_lungsizecomparison_e2e.rs` (3 tests:
  3-answer forward recall with citation, backward recall, honest abstention). No manifest
  objective, matching `lung-lobes.adj`'s own precedent. Seventh slice from the anatomy/ domain
  sweep — steady at 176 objectives. 102nd content slice overall.
- `anatomy/joint-formed-by.adj` (new) — a sibling to the already-shipped `joint-types.adj`
  (`joint_example(joint_type, example)`, hinge/pivot/condyloid/saddle/planar/ball_and_socket →
  a representative joint the source names for each shape, e.g. pivot → atlantoaxial). That table's
  own header already quotes, verbatim, three StatPearls sentences that each name a joint type's
  representative example joint AND, in the same sentence, the specific bones that meet to form
  that joint — a fact the joint_type/example schema had no room for. New `joint_formed_by
  (joint_type, bone)` table decodes those named bones as their own rows (6 total, across the three
  joint types whose quoted span names the forming bones): pivot → atlas, axis; condyloid →
  distal_metacarpals, proximal_phalanges; saddle → trapezium, first_metacarpal. Hinge, planar, and
  ball_and_socket are deliberately not rowed here, since their own quoted spans name only an
  example joint, never the bones that form it. Honest abstention on `hinge`, a real, already-tabled
  joint type whose own quote names no forming bones. New e2e test file `facts_jointformedby_e2e.rs`
  (3 tests: 2-answer forward recall with citation, backward recall, honest abstention). No manifest
  objective, matching `joint-types.adj`'s own precedent. Sixth slice from the anatomy/ domain sweep
  — steady at 176 objectives. 101st content slice overall.
- `anatomy/valve-kind.adj` (new) — a sibling to the already-shipped `heart-valves.adj`
  (`valve_separates(valve, boundary)`, tricuspid/mitral/pulmonary/aortic → their own two-chamber-
  or-vessel boundary). That table's own header already quotes, verbatim, the NCI SEER sentences
  that name each valve AND classify it as one of two physiological kinds — "atrioventricular" (the
  two valves between an atrium and its ventricle) or "semilunar" (the two valves at the base of a
  great vessel leaving a ventricle) — a fact the valve/boundary schema had no room for. New
  `valve_kind(valve, kind)` table decodes that classifying adjective as each valve's own row:
  tricuspid/mitral → atrioventricular, pulmonary/aortic → semilunar. Confirmed via discipline #30
  that no existing table already maps valve→kind under a different name (only `heart-valves.adj`'s
  own header prose mentions these classification terms at all). Honest abstention on `eustachian`,
  a real cardiac valve name but not one of the four valves this table covers. New e2e test file
  `facts_valvekind_e2e.rs` (3 tests: forward recall with citation, 2-answer backward recall,
  honest abstention). No manifest objective, matching `heart-valves.adj`'s own precedent. Fifth
  slice from the anatomy/ domain sweep — steady at 176 objectives. 100th content slice overall.
- `anatomy/heart-chamber-vessel.adj` (new) — a sibling to the already-shipped `heart-chambers.adj`
  (`heart_chamber_function(chamber, function)`, right_atrium/right_ventricle/left_atrium/
  left_ventricle → receives_blood_from_body/pumps_blood_to_lungs/receives_blood_from_lungs/
  pumps_blood_to_body). That table's own header already quotes, verbatim, four StatPearls
  sentences that each name a chamber's function AND, in the same breath, the named vessel(s)/
  valve(s) blood passes through to get there — a fact the chamber/function schema had no room for.
  New `heart_chamber_vessel(chamber, vessel)` table decodes those named structures as their own
  rows (6 total): right_atrium → superior_vena_cava, inferior_vena_cava; right_ventricle →
  pulmonic_valve, pulmonary_artery; left_atrium → pulmonary_veins; left_ventricle → aortic_valve.
  Confirmed via discipline #30 (schema-duplication check) that this is genuinely distinct from the
  already-shipped `valve_separates(valve, boundary)` in `heart-valves.adj` — that table is keyed
  by VALVE and names the two CHAMBERS it separates; this one is keyed by CHAMBER and names the
  VESSEL(S)/VALVE(S) attached to it, and never mentions the vena cavae or pulmonary veins at all.
  Honest abstention on `septum`, a real cardiac structure but not one of the four chambers. New
  e2e test file `facts_heartchambervessel_e2e.rs` (3 tests: forward recall with citation,
  2-answer backward recall, honest abstention). No manifest objective, matching
  `heart-chambers.adj`'s own precedent. Fourth slice from the anatomy/ domain sweep — steady at
  176 objectives. 99th content slice overall.
- `anatomy/body-counts.adj` (extended) — the anatomy/ domain sweep's third candidate,
  `hand_bone_group_count(group, count)` off `hand-bones.adj`, turned out to duplicate
  `body-counts.adj`'s own purpose (`body_count(structure, count)` — "how many of each major
  structure the human body has") once inspected against the REJECT precedent set for other
  count-shaped candidates this sweep (kidney-parts.adj). Rather than ship a second table with an
  identical schema, extended `body_count` with three new rows reusing the SAME already-cited
  NCBI-Bookshelf sentence `hand-bones.adj` already ships in its own header (no new WebFetch):
  carpals → 8, metacarpals → 5, phalanges → 14 ("8 carpal bones ... 5 metacarpal bones ... and 14
  phalanges"). Unlike the table's other six rows (all primary U.S. government sources), this
  source is a patient-education/teaching summary (IQWiG) hosted on NIH/NCBI Bookshelf, so these
  three rows are honestly documented as `consensus`-tier in the header prose, one rung below the
  table's `authoritative` envelope trust — matching `hand-bones.adj`'s own trust tier for the same
  sentence. Extended the existing `facts_anatomy_e2e.rs` test file with a new test covering all
  three new rows. Third slice from the anatomy/ domain sweep. 98th content slice overall.
- `anatomy/eye-part-property.adj` (new) — a sibling to the already-shipped `eye-parts.adj`
  (`eye_part_function(part, function)`, cornea/pupil/iris/lens/retina/optic_nerve →
  bends_light/lets_in_light/controls_light/focuses_light/turns_light_into_signals/
  carries_signals_to_brain). That table's own header already quotes, verbatim, per-row NEI spans
  naming each part's FUNCTION — but three of those same quoted spans also carry a parenthetical or
  descriptive clause naming a PROPERTY of the part (what it IS, not what it does), a fact the
  function schema had no room for: cornea → dome_shaped ("shaped like a dome"), iris →
  colored_part_of_eye ("the colored part of the eye"), retina → light_sensitive_layer ("a
  light-sensitive layer of tissue"). New `eye_part_property(part, property)` table decodes each as
  its own row. Honest abstention on lens, a real already-tabled eye part whose own quote states
  only a function, no descriptive property (same for pupil/optic_nerve, deliberately left unrowed).
  New e2e test file `facts_eyepartproperty_e2e.rs` (3 tests: forward recall with citation, backward
  recall, honest abstention). No manifest objective, matching `eye-parts.adj`'s own precedent.
  Second slice from the anatomy/ domain sweep. 97th content slice overall.
- `anatomy/ear-structure-function.adj` (new) — a sibling to the already-shipped `ear-parts.adj`
  (`ear_structure_region(structure, region)`, ear_canal/malleus/incus/stapes/cochlea →
  outer_ear/middle_ear/middle_ear/middle_ear/inner_ear). That table's own header already quotes,
  verbatim, two chained NIDCD sentences that together name the three middle-ear ossicles AND
  state what they do to the signal — "The bones in the middle ear amplify, or increase, the sound
  vibrations and send them to the cochlea..." — but the structure/region schema had room only for
  WHERE each structure sits, not WHAT the ossicles DO. New `ear_structure_function(structure,
  function)` table decodes that action verb as each ossicle's own row: malleus/incus/stapes → all
  amplifies_sound. Honest abstention on ear_canal, a real already-tabled ear structure whose own
  quote states no action-verb function. New e2e test file `facts_earstructurefunction_e2e.rs` (3
  tests: forward recall with citation, 3-answer backward recall, honest abstention). No manifest
  objective, matching `ear-parts.adj`'s own precedent. First slice from a fresh anatomy/ domain
  sweep — the domain's ~20 shipped tables turned out to be only partially mined (the standing
  "only lung-lobes shipped" note was stale), and a targeted Explore-agent sweep found 9 further
  STRONG sibling-table candidates across the directory.
- `language/subordinating-conjunction-relationship-type.adj` (new) — a sibling to the
  already-shipped `conjunction-type.adj` (`conjunction_type(type, description)`,
  coordinating_conjunction/correlative_conjunction/subordinating_conjunction → their own defining
  sentences). That table's own header already reproduces, verbatim, the Grammarly sentence for the
  subordinating conjunction in full — "Subordinating conjunctions join dependent clauses to the
  independent clauses of sentences, signaling cause and effect, comparison, contrast, time, or some
  other kind of relationship between the clauses." — but the type/description schema had room only
  for the structural fact (joining clauses), not the four named relationship kinds the same
  sentence lists. New `subordinating_conjunction_relationship_type(relationship_type,
  conjunction_type)` table decodes those four kinds as their own rows: cause_and_effect/comparison/
  contrast/time → all subordinating_conjunction. The sentence's own vague trailing "or some other
  kind of relationship" clause is deliberately excluded — it names no additional category. Honest
  abstention on `concession` (a real relationship subordinating conjunctions can express in general
  grammar, but not one this specific quoted sentence names) and on `coordinating_conjunction` (a
  real, already-tabled conjunction category, but not the one this table covers). New e2e test file
  `facts_subordinatingconjunctionrelationshiptype_e2e.rs` (4 tests: forward recall with citation,
  4-answer backward recall, and two honest-abstention cases). No manifest objective, matching
  `conjunction-type.adj`'s own precedent. SEVENTH AND FINAL slice from the language/ domain cleanup
  sweep — this closes the sweep.
- `language/syllable-type-alias.adj` (new) — a sibling to the already-shipped `silent-e-word.adj`
  (`silent_e_word(word, syllable_type)`, wake/whale/while/yoke/yore/rude/hare → all
  vce_long_vowel). That table's own `source` field already quotes, verbatim, the Reading Rockets
  sentence in full — "Also known as \"magic e\" syllable patterns, VCe syllables contain long
  vowels spelled with a single letter, followed by a single consonant, and a silent e." — but the
  word/syllable_type schema had room only for WHICH words follow the pattern, not the sentence's
  own opening alias clause naming the pattern's alternate name. New
  `syllable_type_alias(syllable_type, alias)` table decodes that clause as its own row:
  vce_long_vowel → magic_e. Since the parent tables only ONE syllable type across all seven rows,
  the abstention target instead comes from the parent's own "Six Syllable Types" source article —
  honest abstention on closed_syllable, a real syllable type that article covers but
  silent-e-word.adj (and therefore this sibling) does not table. New e2e test file
  `facts_syllabletypealias_e2e.rs` (3 tests: forward recall with citation, backward recall, honest
  abstention). No manifest objective, matching `silent-e-word.adj`'s own precedent. Sixth slice
  from the language/ domain cleanup sweep.
- `language/determiner-type-alias.adj` (new) — a sibling to the already-shipped
  `determiner-type.adj` (`determiner_type(type, description)`, article/demonstrative_determiner/
  distributive_determiner → their own defining sentences). That table's own header already
  reproduces, verbatim, the Grammarly sentence for the demonstrative determiner in full —
  "Demonstrative determiners, also known as demonstrative adjectives, communicate the placement of a
  noun in space or time." — but the type/description schema had room only for WHAT it does, not the
  parenthetical alias clause naming its alternate name. New `determiner_type_alias(type, alias)`
  table decodes that clause as its own row: demonstrative_determiner → demonstrative_adjective.
  Honest abstention on article, a real already-tabled determiner type whose own sentence states no
  alias. New e2e test file `facts_determinertypealias_e2e.rs` (3 tests: forward recall with citation,
  backward recall, honest abstention). No manifest objective, matching `determiner-type.adj`'s own
  precedent. Fifth slice from the language/ domain cleanup sweep.
- `language/past-tense-ed-sound-effect.adj` (new) — a sibling to the already-shipped
  `past-tense-ed-sound.adj` (`past_tense_ed_sound(word, sound)`, walked/lived/wanted →
  t_sound/d_sound/id_sound). That table's own header already reproduces, verbatim, the 7ESL rule for
  the /id/ sound in full — "Final -ed is pronounced /id/ after 'T', and 'D' sounds. The sound /id/ adds
  a whole syllable to a word." — but the word/sound schema had room only for the sound itself, not the
  second sentence's pronunciation-effect claim. New `past_tense_ed_sound_effect(sound, effect)` table
  decodes that second sentence as its own row: id_sound → adds_a_whole_syllable. Honest abstention on
  t_sound, a real already-tabled -ed sound whose own rule states no comparable effect. New e2e test file
  `facts_pasttenseedsoundeffect_e2e.rs` (3 tests: forward recall with citation, backward recall, honest
  abstention). No manifest objective, matching `past-tense-ed-sound.adj`'s own precedent. Fourth slice
  from the language/ domain cleanup sweep.
- `language/greek-alphabet-standardization.adj` (new) — a sibling to the already-shipped
  `greek-alphabet.adj` (`greek_letter_position(letter, position)`, the 24 letter→position mappings).
  That table's own `source` field already quotes, verbatim, a Wikipedia sentence naming WHEN the
  Euclidean alphabet the letter-position table is built from became standard — "by the end of the 4th
  century BC, the Ionic-based Euclidean alphabet, with 24 letters, ordered from alpha to omega, had
  become standard" — a fact the letter/position schema had no room for. New
  `greek_alphabet_standardization(alphabet_name, standardized_by_period)` table decodes that clause as
  its own row: euclidean_alphabet → fourth_century_bc. (Note: the ADJ atom is spelled `fourth_century_bc`,
  not `4th_century_bc` — ADJ atoms must lex as identifiers and cannot start with a digit.) Honest
  abstention on attic_alphabet, a real but distinct Greek alphabet variant the cited span does not name.
  New e2e test file `facts_greekalphabetstandardization_e2e.rs` (3 tests: forward recall with citation,
  backward recall, honest abstention). No manifest objective, matching `greek-alphabet.adj`'s own
  precedent. Third slice from the language/ domain cleanup sweep.
- `language/morse-code-origin.adj` (new) — a sibling to the already-shipped `morse-code.adj` and
  `morse-code-standard.adj`. The SAME already-quoted Wikipedia sentence also names who proposed the code
  International Morse code was derived from, and when — "a much-improved proposal by Friedrich Gerke in
  1848" — a fact neither sibling's schema had room for. New `morse_code_origin(code_system, originator,
  year)` table decodes that clause as its own row: international_morse_code → friedrich_gerke, 1848. A
  THREE-column table, since originator and year belong together as one origin event rather than two
  separate sibling tables. Honest abstention on american_morse_code, the same target
  `morse-code-standard.adj` already uses. New e2e test file `facts_morsecodeorigin_e2e.rs` (3 tests:
  forward recall with citation, backward recall, honest abstention). No manifest objective, matching
  `morse-code.adj`'s own precedent. Second slice from the language/ domain cleanup sweep.
- `language/morse-code-standard.adj` (new) — a sibling to the already-shipped `morse-code.adj`
  (`morse_code(letter, pattern)`, the 26 letter→dot/dash mappings). That table's own `source` field already
  quotes, verbatim, a Wikipedia sentence naming the international standard that specifies the code — "the
  current international standard, International Morse Code Recommendation, ITU-R M.1677-1" — a fact the
  letter/pattern schema had no room for. New `morse_code_standard(code_system, standard_id)` table decodes
  that clause as its own row: international_morse_code → itu_r_m_1677_1. Honest abstention on
  american_morse_code, a real but distinct historical Morse variant the cited span does not name. New e2e
  test file `facts_morsecodestandard_e2e.rs` (3 tests: forward recall with citation, backward recall,
  honest abstention). No manifest objective, matching `morse-code.adj`'s own precedent. First slice from a
  targeted sibling-table sweep of the language/ (literacy) domain — this domain had already been swept
  repeatedly for this pattern, but 7 small untabled leftover facts remained; this is the first of them.
- `oceanography/ocean-current-strongest-location.adj` (new) — a sibling to the already-shipped
  `ocean-current-drivers.adj` and `ocean-current-surface-position.adj`. That table's own header already
  quotes, verbatim, a NOAA sentence stating tidal currents "are strongest near the shore, and in bays and
  estuaries along the coast" — a location fact the single `driver` atom had no room for. Both prepositional
  phrases are simultaneously true (the source's "and" joins two coexisting facts, not two competing
  readings), so they fold into ONE compound atom, matching this stdlib's own established convention. New
  `ocean_current_strongest_location(current_type, location)` table decodes that clause as its own row:
  tidal_currents → near_the_shore_and_in_bays_and_estuaries_along_the_coast. Honest abstention on
  thermohaline_circulation, whose cited span names no location. New e2e test file
  `facts_oceancurrentstrongestlocation_e2e.rs` (3 tests: forward recall with citation, backward recall,
  honest abstention). No manifest objective, matching `ocean-current-drivers.adj`'s own precedent. Fourth
  slice from the oceanography/ domain sweep. (The remaining candidate, `named_current_climate_effect` off
  the same file's gulf_stream aside — "brings milder winter weather to Bergen, Norway, than to New York" —
  was re-inspected and confirmed a dead end: the claim is inherently a comparison between two named places,
  which cannot honestly fit a binary (current, effect) shape without arbitrary compression of a relation
  the schema was never built to hold. This closes out the oceanography/ domain sweep.)
- `oceanography/ocean-current-surface-position.adj` (new) — a sibling to the already-shipped
  `ocean-current-drivers.adj` (`ocean_current_driver(current_type, driver)`, ONE physical driver per
  current type — wind_driven_currents → wind). That table's own header already quotes, verbatim, a NOAA
  sentence stating "Winds drive currents that are at or near the ocean's surface." — a positional fact the
  single `driver` atom had no room for. New `ocean_current_surface_position(current_type, position)` table
  decodes that clause as its own row: wind_driven_currents → at_or_near_the_oceans_surface. Honest
  abstention on thermohaline_circulation, whose cited span names no position. New e2e test file
  `facts_oceancurrentsurfaceposition_e2e.rs` (3 tests: forward recall with citation, backward recall,
  honest abstention on thermohaline_circulation). No manifest objective, matching
  `ocean-current-drivers.adj`'s own precedent. Third slice from the oceanography/ domain sweep.
- `oceanography/ocean-instrument-secondary-quantity.adj` (new) — a sibling to the already-shipped
  `ocean-observing-instruments.adj` (`ocean_instrument(instrument, quantity)`, ONE quantity per
  instrument — sonar → distance_to_object). That table's own header already quotes, verbatim, a NOAA
  sentence stating sonar determines "the range and orientation of the object" — a second, distinct
  quantity ("orientation") the single `quantity` atom had no room for. New
  `ocean_instrument_secondary_quantity(instrument, secondary_quantity)` table decodes that clause as its
  own row: sonar → orientation_of_object. Honest abstention on tide_gauge, whose cited span names no
  second quantity. New e2e test file `facts_oceaninstrumentsecondaryquantity_e2e.rs` (3 tests: forward
  recall with citation, backward recall, honest abstention on tide_gauge). No manifest objective, matching
  `ocean-observing-instruments.adj`'s own precedent. Second slice from the oceanography/ domain sweep.
- `oceanography/ocean-zone-scientific-name.adj` (new) — a sibling to the already-shipped
  `ocean-zones.adj` (`ocean_zone(zone, order)`) and `ocean-zone-depth.adj` (`ocean_zone_depth(zone,
  max_depth_meters)`). The SAME already-quoted WHOI "Ocean Zones" midnight-zone sentence — "The midnight
  zone, or bathypelagic, extends to about 4,000 meters..." — also names an alternate scientific name for
  that zone, a fact neither sibling's schema had room for (one tables sequence position, the other tables
  depth). New `ocean_zone_scientific_name(zone, scientific_name)` table decodes that clause as its own row:
  midnight_zone → bathypelagic. Honest abstention on sunlight_zone and twilight_zone, whose cited spans
  name no alternate scientific name. New e2e test file `facts_oceanzonescientificname_e2e.rs` (3 tests:
  forward recall with citation, backward recall, honest abstention on sunlight_zone). No manifest
  objective, matching both siblings' own precedent. First slice from the oceanography/ domain sweep.
- `meteorology/precipitation-freeze-threshold.adj` (new) — a sibling to the already-shipped
  `precipitation-types.adj` (`precip_form(precip, form)`, ONE defining physical form per precipitation
  type — freezing_rain → glaze_of_ice). That table's own header already quotes, verbatim, an NWS sentence
  stating the temperature at or below which freezing rain refreezes on contact (32 degrees F) — a numeric
  figure the single `form` atom had no room for. New `precipitation_freeze_threshold_f(precip,
  temperature_f)` table decodes that clause as its own row: freezing_rain → 32. Honest abstention on rain,
  snow, sleet, and hail, whose cited spans state no numeric freeze threshold. New e2e test file
  `facts_precipitationfreezethreshold_e2e.rs` (3 tests: forward recall with citation, backward recall,
  honest abstention on rain). No manifest objective, matching `precipitation-types.adj`'s own precedent.
  Fourth slice from the meteorology/ domain sweep. (The remaining MODERATE candidate,
  `hurricane_damaged_component` off `hurricane-categories.adj`/`hurricane-category-home-damage.adj`, was
  re-inspected and confirmed a dead end: its five structural-effect spans require inconsistent
  interpretive judgment calls to decompose — some are clean comma/and-joined noun lists, others bundle a
  severity choice ("damage or removal") or use distinct verb phrases per row rather than a shared list —
  so no honest single-schema decomposition was possible.)
- `meteorology/precipitation-source-cloud.adj` (new) — a sibling to the already-shipped
  `precipitation-types.adj`, `precipitation-minimum-diameter.adj`, and
  `precipitation-alternate-form.adj`. That SAME already-quoted NWS Glossary hail sentence also names the
  originating cloud type — a fact none of those three siblings had a column for. New
  `precipitation_source_cloud(precip, cloud)` table decodes that clause as its own row: hail →
  cumulonimbus. Honest abstention on rain, snow, sleet, and freezing_rain, whose cited spans name no
  originating cloud. New e2e test file `facts_precipitationsourcecloud_e2e.rs` (3 tests: forward recall
  with citation, backward recall, honest abstention on rain). No manifest objective, matching
  `precipitation-types.adj`'s own precedent. Third slice from the meteorology/ domain sweep.
- `meteorology/precipitation-alternate-form.adj` (new) — a sibling to the already-shipped
  `precipitation-types.adj` (`precip_form(precip, form)`, ONE defining physical form per precipitation
  type — hail → balls_of_ice) and `precipitation-minimum-diameter.adj`. That SAME already-quoted NWS
  Glossary hail sentence lists TWO descriptive terms joined by "or" — a fact the single `form` atom folded
  into one label, keeping only the second term. New `precipitation_alternate_form(precip, form)` table
  decodes the first listed term as its own row: hail → irregular_pellets. Honest abstention on rain, snow,
  sleet, and freezing_rain, whose cited spans each name only one descriptive term with no listed
  alternative. New e2e test file `facts_precipitationalternateform_e2e.rs` (3 tests: forward recall with
  citation, backward recall, honest abstention on rain). No manifest objective, matching
  `precipitation-types.adj`'s own precedent. Second slice from the meteorology/ domain sweep.
- `meteorology/cloud-signal.adj` (new) — a sibling to the already-shipped `cloud-type.adj`
  (`cloud_type(cloud, weather_indication)`, each of three common cloud types and the single combined
  weather indication its presence signals). Two of that table's own three quoted NWS sentences each list
  MORE than one signal joined by an exhaustive "or" list — a fact the single `weather_indication` atom
  folded into one compound label. New `cloud_signal(cloud, signal)` table decodes each listed signal as
  its own row: cirrus → approaching_warm_front, cirrus → upper_level_jet_streak, stratus →
  precipitation_free, stratus → light_precipitation, stratus → drizzle. Honest abstention on
  cumulonimbus, whose cited span names only one signal with no listed alternative. New e2e test file
  `facts_cloudsignal_e2e.rs` (3 tests: forward multi-answer recall with citation, backward recall, honest
  abstention on cumulonimbus). No manifest objective, matching `cloud-type.adj`'s own precedent. First
  slice from a fresh domain sweep of meteorology/ — begins after the geology/ domain was fully exhausted
  this window.
- `geology/igneous-rock-type-eruption-location.adj` (new) — a sibling to the already-shipped
  `igneous-rock-type.adj` (`igneous_rock_type(type, description)`, the two-way intrusive/extrusive split
  by cooling location). That table's own header already quotes, verbatim, the extrusive row's defining
  NPS sentence in full, listing TWO locations joined by "or" — a fact the single `description` atom
  folded into one compound label. New `igneous_rock_type_eruption_location(type, location)` table decodes
  each listed location as its own row: extrusive → surface, extrusive → atmosphere. Honest abstention on
  intrusive, whose cited span names only one location with no listed alternative. New e2e test file
  `facts_igneousrocktyperuptionlocation_e2e.rs` (3 tests: forward multi-answer recall with citation,
  backward recall, honest abstention on intrusive). No manifest objective, matching `igneous-rock-type.adj`'s
  own precedent. Second and final slice from the geology/ domain sweep — closes out that sweep entirely.
- `geology/rock-type-formation-component.adj` (new) — a sibling to the already-shipped `rock-type.adj`
  (`rock_type(rock, formation_process)`, the process by which each of the three basic rock classes
  forms — igneous → crystallized_molten_rock, sedimentary → deposited_weathered_material, metamorphic
  → heat_and_pressure_transformation). Two of that table's own three per-row USGS quotes each list MORE
  than one material/agent joined by an exhaustive "or" or comma-list — a fact the single
  `formation_process` atom folded into one compound label. New
  `rock_type_formation_component(rock, component)` table decodes each listed item as its own row:
  sedimentary → pre_existing_rocks, sedimentary → pieces_of_once_living_organisms, metamorphic →
  high_heat, metamorphic → high_pressure, metamorphic → hot_mineral_rich_fluids. Honest abstention on
  igneous, whose cited span names only one material with no listed alternative. New e2e test file
  `facts_rocktypeformationcomponent_e2e.rs` (3 tests: forward multi-answer recall with citation,
  backward recall, honest abstention on igneous). No manifest objective, matching `rock-type.adj`'s own
  precedent. First slice from a fresh domain sweep of geology/ — begins after the astronomy/ domain was
  fully exhausted this window.
- `astronomy/space-rock-alt-name.adj` (new) — a sibling to the already-shipped `space-rock-stage.adj`
  (`space_rock_stage(stage, description)`, the same rocky object's name at three stages of its
  journey — meteoroid → still_a_rock_in_space, meteor →
  called_a_fireball_or_shooting_star_when_it_burns_up_in_the_atmosphere, meteorite →
  survives_the_atmosphere_and_hits_the_ground). That table's own header already quotes, verbatim, the
  meteor sentence naming TWO everyday alternate terms in one continuous clause — a fact the
  single-description-atom schema had no room to decode as two separate, independently queryable names.
  New `space_rock_alt_name(stage, alt_name)` table: meteor → fireball, meteor → shooting_star. Honestly
  narrower than its parent: honest abstention on meteoroid and meteorite, whose cited spans name only
  the one primary term each. New e2e test file `facts_spacerockaltname_e2e.rs` (3 tests: forward
  recall returning both alternate names with citation, backward recall of the fireball-named stage,
  honest abstention on meteoroid). No manifest objective, matching `space-rock-stage.adj`'s own
  precedent. Third and final slice from the astronomy/ domain sweep — closes out that sweep entirely.
- `astronomy/dwarf-planet-criterion-status.adj` (new) — a sibling to the already-shipped
  `planet-criterion.adj` (`planet_criterion(criterion, requirement)`, the three IAU requirements a
  body must meet to count as a full planet). That table's own header already quotes, verbatim, a NASA
  sentence used only to justify EXCLUDING `dwarf_planet` as a row: "Dwarf planets like Pluto were
  defined as objects that orbit the Sun, and are nearly round, but have not been able to clear their
  orbit of debris." New `dwarf_planet_criterion_status(criterion, status)` table decodes that same
  sentence's per-criterion status instead: orbit → met, roundness → met, cleared_orbit → not_met. Full
  3/3 coverage, no abstention needed among the three criteria — the sentence's own leading/trailing
  clause structure fixes each status. New e2e test file `facts_dwarfplanetcriterionstatus_e2e.rs`
  (3 tests: forward recall with citation, backward recall of the not-met criterion, honest abstention
  on a non-criterion atom). No manifest objective, matching `planet-criterion.adj`'s own precedent.
  Second slice from the fresh astronomy/ domain sweep begun after physics/ was fully exhausted.
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
- `physics/optic-focal-point-location.adj` (new) — a sibling to the already-shipped `lens-types.adj`
  (`optic_action(optic, action)`, whether each of four basic optical elements converges or diverges
  parallel light — convex_lens → converges_light, concave_lens → diverges_light, concave_mirror →
  converges_light, convex_mirror → diverges_light). That table's own header already quotes, verbatim,
  an OpenStax sentence for THREE of the four rows that states WHERE the focal point sits — a fact the
  action-only schema had no room for. New `optic_focal_point_location(optic, location)` table:
  convex_lens → opposite_side_of_the_lens, concave_mirror → same_side, convex_mirror →
  behind_the_mirror. Genuinely new information beyond the parent's converge/diverge action: it
  distinguishes a REAL focal point (convex_lens, concave_mirror) from a VIRTUAL one (convex_mirror),
  the core distinction for reasoning about real vs. virtual image formation. Honestly narrower than
  its parent: honest abstention on concave_lens, whose already-cited span describes ray divergence but
  never states a focal-point location. New e2e test file `facts_opticfocalpointlocation_e2e.rs`
  (3 tests: forward recall with citation, backward recall of the behind-the-mirror element, honest
  abstention on concave_lens). No manifest objective, matching `lens-types.adj`'s own precedent. Third
  and final slice from the direct re-examination of the physics/ MODERATE candidates the sweep had
  deprioritized — closes out that batch (the fourth, `circuit_part_input_energy`, was confirmed a
  genuine dead end rather than shipped).
- `physics/band-emitter.adj` (new) — a sibling to the already-shipped `em-spectrum.adj`
  (`band_use(band, application)`, a representative everyday use / effect / detector NASA associates
  with each of the seven EM bands — radio → radio_stations, x_ray → teeth, etc.). That table's own
  header already quotes, verbatim, three NASA sentences that separately state WHO or WHAT emits the
  band: "Ultraviolet radiation is emitted by the Sun...", "Night vision goggles pick up the infrared
  light emitted by our skin and objects with heat.", "Our eyes detect visible light. Fireflies,
  light bulbs, and stars all emit visible light." New `band_emitter(band, emitter)` table:
  ultraviolet → sun, infrared → skin_and_heat_objects, visible → fireflies (the first of three
  listed emitters, per this stdlib's own established convention of carrying the first item a source
  lists when more than one is given). Honestly narrower than its parent: honest abstention on radio,
  microwave, x_ray, and gamma_ray, whose already-cited spans state a USE (already captured by
  `em-spectrum.adj`'s own column) but never an emitter. New e2e test file `facts_bandemitter_e2e.rs`
  (3 tests: forward recall with citation, backward recall of the sun-emitted band, honest abstention
  on radio). No manifest objective, matching `em-spectrum.adj`'s own precedent. Second slice from a
  direct re-examination of the physics/ MODERATE candidates the sweep had deprioritized — closer
  inspection confirmed this one was genuinely buildable, just narrower than the parent's full
  seven-band domain.
- `physics/wave-family-mechanism.adj` (new) — a sibling to the already-shipped `wave-types.adj`
  (`wave_family(wave, family)`, 10 rows, which of the two families — mechanical or electromagnetic —
  a NAMED wave belongs to). That table's own header already quotes, verbatim, a THIRD NASA sentence
  (beyond the two per-wave classification sentences already used to build all 10 rows) that states,
  in one continuous contrast, the defining MECHANISM of both families at once: "Light waves, also
  called electromagnetic waves, involve oscillations of electric and magnetic fields rather than
  oscillations of matter." New `wave_family_mechanism(family, mechanism)` table: electromagnetic →
  oscillations_of_electric_and_magnetic_fields, mechanical → oscillations_of_matter. Unlike its
  parent, this sibling is keyed by FAMILY (2 rows), not by individual WAVE (10 rows) — a coarser
  grain, since the underlying fact is itself a per-family property; honest abstention on any
  individual wave name (that belongs to the parent's own key set). New e2e test file
  `facts_wavefamilymechanism_e2e.rs` (3 tests: forward recall with citation, backward recall of the
  oscillations-of-matter family, honest abstention on an individual wave name). No manifest
  objective, matching `wave-types.adj`'s own precedent. First slice of a direct re-examination of the
  4 physics/ MODERATE candidates the sweep had deprioritized — closer inspection showed this one is
  fully and cleanly grounded, just differently shaped than its parent (the "MODERATE" label reflected
  key-granularity, not quote quality).
- `physics/mirror-focal-length-sign.adj` (new) — a sibling to the already-shipped `lens-types.adj`
  (`optic_action(optic, action)`, whether each of four basic optical elements converges or diverges
  parallel light — convex_lens → converges_light, concave_lens → diverges_light, concave_mirror →
  converges_light, convex_mirror → diverges_light). That table's own header already quotes,
  verbatim, an OpenStax sentence for EACH of the two mirror rows that states the SIGN of the
  mirror's focal length: "The focal length f of a concave mirror is positive, since it is a
  converging mirror" and "The focal length and power of a convex mirror are negative, since it is a
  diverging mirror." New `mirror_focal_length_sign(mirror, sign)` table: concave_mirror → positive,
  convex_mirror → negative. Honestly narrower than its parent: honest abstention on `convex_lens`
  and `concave_lens`, whose already-cited spans describe ray behavior but never state a sign
  convention. New e2e test file `facts_mirrorfocallengthsign_e2e.rs` (3 tests: forward recall with
  citation, backward recall of the negative-sign mirror, honest abstention on convex_lens). No
  manifest objective, matching `lens-types.adj`'s own precedent. Third and final STRONG slice from
  the physics/ sweep, closing out simple-machine-function.adj (PR #11523),
  heat-transfer-example.adj (PR #11531), and this one.
- `physics/heat-transfer-example.adj` (new) — a sibling to the already-shipped `heat-transfer.adj`
  (`heat_transfer_mode(mode, mechanism)`, HOW / through-what each mode moves heat — conduction →
  direct_contact, convection → motion_of_gasses_and_liquids, radiation → light_waves). That table's
  own header already quotes, verbatim, the NASA sentence that states each mechanism, and the SAME
  sentence closes with a parenthetical `(e.g., ...)` everyday example: "Conduction - ... (e.g.,
  holding a cup of hot chocolate, walking bare foot across a cold floor)", "Convection - ... (e.g.,
  when warm air rises in your home)", "Radiation - ... (e.g., sunlight on your skin, heating food in
  a microwave)". New `heat_transfer_example(mode, example)` table: conduction → hot_chocolate_cup,
  convection → warm_air_rising, radiation → sunlight_on_skin. Covers the full three-mode domain with
  NO abstention among the three (conduction and radiation each have a second example — cold floor,
  microwave food — sitting unused for a possible future alt-example sibling; convection lists only
  the one). New e2e test file `facts_heattransferexample_e2e.rs` (3 tests: forward recall with
  citation, backward recall of the convection mode, honest abstention on evaporation). No manifest
  objective, matching `heat-transfer.adj`'s own precedent. Second slice from the fresh physics/
  sweep, queued after `simple-machine-function.adj`.
- `physics/simple-machine-function.adj` (new) — a sibling to the already-shipped
  `simple-machines.adj` (`simple_machine_example(machine, example)`, one everyday example per
  simple machine) and its own already-shipped sibling `simple-machine-alt-example.adj` (the second
  everyday example for five of the six). Both decode only the trailing `(examples: ...)`
  parenthetical of each NASA definition sentence; the SAME six sentences open with a leading
  clause stating what the machine actually DOES: "Lever: uses a surface situated on a fulcrum
  (pivoting point) to move an object...", "Screw: helps to fasten two objects together...". New
  `simple_machine_function(machine, function)` table: lever → moves_object_over_fulcrum, inclined
  plane → moves_objects_up_angled_surface, wedge → splits_or_separates_objects, screw →
  fastens_objects_together, wheel_and_axle → turns_or_moves_load, pulley →
  changes_direction_of_object. Covers the full six-machine domain with NO abstention among the six
  (even screw, which `simple-machine-alt-example.adj` must skip since it has no distinct second
  example) — abstains only outside that set. New e2e test file
  `facts_simplemachinefunction_e2e.rs` (3 tests: forward recall with citation, backward recall of
  the fastening machine, honest abstention on a non-simple-machine word). No manifest objective,
  matching `simple-machines.adj`'s own precedent. First slice from a fresh physics/ sweep, launched
  after biology/'s sweep-discoverable candidates were exhausted across 3 rounds.
- `biology/vitamin-deficiency-symptom.adj` (new) — a sibling to the already-shipped `vitamins.adj`
  (`deficiency_disease(vitamin, disease)`, the classic deficiency DISEASE each vitamin's lack
  causes — vitamin_c → scurvy, vitamin_d → rickets, etc.). That table's own header already quotes,
  verbatim, the NIH ODS sentence that states each disease, and for five of the seven vitamins the
  SAME sentence also states the disease's defining SYMPTOM: "the bones become soft, weak,
  deformed, and painful" (vitamin_d), "tingling and numbness in the feet and hands..." (vitamin_b1),
  "makes people tired and weak" (vitamin_b12), and so on. New `vitamin_deficiency_symptom(vitamin,
  symptom)` table: vitamin_a → inability_to_see_in_low_light, vitamin_d →
  soft_weak_deformed_painful_bones, vitamin_b1 → tingling_and_numbness_in_feet_and_hands, vitamin_b9
  → weakness_and_fatigue, vitamin_b12 → tired_and_weak. Honestly narrower than its parent: honest
  abstention on `vitamin_c` and `vitamin_b3`, whose already-cited spans name the disease but state
  no symptom. New e2e test file `facts_vitamindeficiencysymptom_e2e.rs` (3 tests: forward recall
  with citation, backward recall of the tired-and-weak vitamin, honest abstention on vitamin_c). No
  manifest objective, matching `vitamins.adj`'s own precedent of not having one. First slice of a
  fresh, narrower biology/ pass (revisiting a candidate the prior sweep round rated MODERATE and
  deprioritized; closer inspection showed 5 of 7 rows have a fully groundable symptom in the
  already-quoted spans).
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
- `biology/cell-division-genetic-outcome.adj` (new) — a sibling to the already-shipped
  `cell-division-daughter-cells.adj` (`cell_division_daughter_cells(process, count)`, HOW MANY
  daughter cells mitosis/meiosis each yield). That table's own header deliberately quotes,
  verbatim, the OTHER clause of each defining NHGRI sentence "for honesty" while explicitly stating
  it is NOT what the count-only schema extracts: mitosis's daughter cells "have identical genomes,"
  meiosis's are "haploid... the gametes." New `cell_division_genetic_outcome(process, outcome)`
  table: mitosis → genetically_identical, meiosis → haploid. Honest abstention on
  `binary_fission` (the prokaryotic process), matching the parent table's own abstention boundary.
  New e2e test file `facts_celldivisiongeneticoutcome_e2e.rs` (3 tests: forward recall with
  citation, backward recall of the haploid process, honest abstention). The parent table DOES carry
  a manifest objective (`adj.science.6to8.cell_division_daughter_cells`) scoped specifically to the
  daughter-cell COUNT competency; this sibling decodes a different leftover fact from the same
  cited spans rather than a new standalone curriculum competency, so — matching this session's
  established sibling-table precedent — no new manifest objective was added for it. Second slice
  from the fresh biology/ sweep tranche, queued after `mammal-origin.adj`.
- `biology/mammal-origin.adj` (new) — a sibling to the already-shipped `animal-classes.adj`
  (`animal_class(animal, class)`, WHICH vertebrate class an animal belongs to — cat, kangaroo, fox,
  rabbit, bandicoot, quoll, koala are all just `mammal`). That table's own header already quotes,
  verbatim, two Australian Museum spans that split those same seven mammals into two disjoint
  origin groups — a distinction the one-class-per-animal schema had no room for: "introduced
  mammals such as cats, foxes and rabbits" and "marsupials like kangaroos, bandicoots, quolls and
  the Koala". New `mammal_origin(animal, origin)` table covers the full seven-mammal domain with no
  abstention: cat/fox/rabbit → introduced, kangaroo/bandicoot/quoll/koala → marsupial. New e2e test
  file `facts_mammalorigin_e2e.rs` (3 tests: forward recall with citation, backward recall of all
  introduced mammals, full-domain coverage with no abstention). No manifest objective, matching
  `animal-classes.adj`'s own precedent of not having one. First slice from a fresh biology/ sweep
  tranche (of the domain's remaining ~44 tables), discovered after the prior tranche
  (vertebrate-thermoregulation.adj / muscle-striation.adj / insulin-glucagon-trigger.adj)
  completed; two more strong candidates (`cell-division-genetic-outcome.adj`,
  `muscle-nuclei-count.adj`) are queued next.
- `biology/insulin-glucagon-trigger.adj` (new) — a sibling to the already-shipped
  `hormone-glands.adj` (`hormone_gland(hormone, gland)`, WHICH gland secretes a hormone —
  insulin/glucagon → pancreas, etc.). That table's own header already quotes, verbatim, the
  trigger condition for the pancreas's two glucose-regulating hormones, and buried in those two
  spans, alongside the gland the parent table already captures, is a second fact the
  hormone→gland schema had no room for: WHAT blood-glucose condition triggers each hormone's
  release. New `secretion_trigger(hormone, blood_glucose_level)` table: insulin → high, glucagon
  → low. Honestly narrow — abstains on every other hormone in `hormone-glands.adj`, since the
  source never ties them to a blood-glucose trigger. New e2e test file
  `facts_insulinglucagontrigger_e2e.rs` (3 tests: forward recall with citation, backward recall of
  the low-glucose hormone, honest abstention on a hormone with no glucose trigger). No manifest
  objective, matching `hormone-glands.adj`'s own precedent of not having one. Third slice from the
  fresh biology/ sweep tranche, queued after `muscle-striation.adj`.
- `biology/muscle-striation.adj` (new) — a sibling to the already-shipped `muscle-types.adj`
  (`muscle_trait(muscle, trait)`, ONE distinctive characteristic per muscle type — skeletal →
  voluntary, smooth → involuntary, cardiac → intercalated_disks). That table's own header already
  quotes, verbatim, each muscle type's defining NCI SEER sentence, and buried in every one of those
  three spans, alongside the trait the parent table already captures, is a second, cleanly binary
  fact the voluntary/trait schema had no room for: whether the tissue is striated. New
  `muscle_striated(muscle, striated)` table covers the full three-type domain with no abstention:
  skeletal → yes, smooth → no, cardiac → yes. New e2e test file `facts_musclestriation_e2e.rs` (3
  tests: forward recall with citation, backward recall of the one non-striated type, full-domain
  coverage with no abstention). No manifest objective, matching `muscle-types.adj`'s own precedent
  of not having one. Second slice from the fresh biology/ sweep tranche, queued after
  `vertebrate-thermoregulation.adj`.
- `biology/vertebrate-thermoregulation.adj` (new) — a sibling to the already-shipped
  `vertebrate-groups.adj` (`vertebrate_trait(class, trait)`, ONE distinctive body-covering feature
  per class — fish → gills, bird → feathers, etc.). That table's own header already quotes,
  verbatim, the full NPS "Vertebrate Grab Bag" trait list for every class, and buried in every one
  of those five spans, alongside the body-covering feature the parent table already captures, is a
  second, cleanly binary fact the one-trait-per-class schema had no room for: whether the class is
  ectothermic ("cold-blooded") or endothermic ("warm-blooded"). New
  `vertebrate_thermoregulation(class, type)` table covers the full five-class domain with no
  abstention: fish → ectothermic, amphibian → ectothermic, reptile → ectothermic, bird →
  endothermic, mammal → endothermic. New e2e test file `facts_vertebratethermoregulation_e2e.rs` (3
  tests: forward recall with citation, backward recall of all ectothermic classes, full-domain
  coverage with no abstention). No manifest objective, matching `vertebrate-groups.adj`'s own
  precedent of not having one. First slice from a fresh biology/ sweep tranche (49 tables) —
  discovered after the 1-2-table-domain sweep initiative completed; two more strong candidates
  (`muscle-striation.adj`, `insulin-glucagon-trigger.adj`) are queued next.
- `transportation/green-signal-permitted-movement.adj` (new) — a sibling to the already-shipped
  `traffic-lights.adj` (`traffic_light_meaning(color, meaning)`, green → the single atomic value
  `proceed`). That table's own header already quotes, verbatim, the MUTCD Section 4D.04 sentence
  for green, and that same sentence enumerates FOUR distinct permitted movements — that the
  color-to-meaning schema collapsed into one word: "permitted to proceed straight through or turn
  right or left or make a U-turn movement." New `green_signal_permitted_movement(movement)` table
  (keyless — all four movements are simultaneously true for a green signal, unlike a keyed
  lookup): straight_through, turn_right, turn_left, u_turn. No manifest objective, matching
  `traffic-lights.adj`'s own precedent of not having one. New e2e test file
  `facts_greensignalpermittedmovement_e2e.rs` (3 tests: forward recall with citation, all four
  movements present, full coverage with no abstention). First and only strong candidate from the
  transportation/ sweep tranche — two leads were rejected: red's "unless entering the intersection
  to make another movement permitted by another signal indication" exception clause (too
  thin/note-like, qualifies the already-recorded `stop` value rather than adding an independent
  fact) and yellow's termination-sequence clause (states only one transition, too thin for a
  standalone table). transportation/ (1 table) is now effectively closed. **This completes the
  full 1-2-table-domain sweep initiative** covering geometry/, mathematics/, geography/,
  metrology/, agriculture/, art/, calendar/, environment/, money/, music/, nutrition/, optics/, and
  transportation/.
- `nutrition/food-group-alternative-form.adj` (new) — a sibling to the already-shipped
  `food-groups.adj` (`food_group(food, food_group)`, which sorts whole, solid foods into one of
  the five MyPlate groups). Three of that table's own per-group definitional sentences, already
  quoted verbatim in its header, also name a non-solid-food alternative that counts toward the SAME
  group — that the solid-food-only schema had no room for: fruits →
  hundred_percent_fruit_juice ("The Fruit Group includes all fruits and 100% fruit juice."),
  vegetables → hundred_percent_vegetable_juice ("Any vegetable or 100% vegetable juice counts as
  part of the Vegetable Group."), dairy → lactose_free_milk, fortified_soy_milk,
  fortified_soy_yogurt ("The Dairy Group includes milk, yogurt, cheese, lactose-free milk and
  fortified soy milk and yogurt."). New `food_group_alternative_form(food_group, alternative_form)`
  table. Honest abstention on grains and protein, whose own cited definitional sentences name no
  comparable alternative form. New e2e test file `facts_foodgroupalternativeform_e2e.rs` (3 tests:
  forward recall with citation, backward recall from a bound form, honest abstention on grains). No
  manifest objective, matching `food-groups.adj`'s own precedent of not having one. Second and
  final strong slice from the nutrition/ sweep tranche — nutrition/ (1 table) is now EFFECTIVELY
  CLOSED (2/2 strong candidates shipped).
- `nutrition/vegetable-subgroup.adj` (new) — a sibling to the already-shipped `food-groups.adj`
  (`food_group(food, food_group)`, which sorts every vegetable into the SAME coarse `vegetables`
  bucket). That table's own header already quotes, verbatim, USDA MyPlate's finer vegetable
  subgroup classification for the same five vegetables — that the one-bucket-per-food schema had
  no room for: broccoli → dark_green, spinach → dark_green, carrots → red_and_orange, tomatoes →
  red_and_orange, corn → starchy. New `vegetable_subgroup(vegetable, subgroup)` table covers the
  full five-vegetable domain with no abstention, since every shipped vegetable has a subgroup in
  the cited span. New e2e test file `facts_vegetablesubgroup_e2e.rs` (3 tests: forward recall with
  citation, backward recall from a bound subgroup, full-domain coverage with no abstention). No
  manifest objective, matching `food-groups.adj`'s own precedent of not having one. First slice
  from the nutrition/ sweep tranche (a second strong candidate, `food-group-alternative-form.adj`,
  is queued next).
- `music/solfege-alt-name.adj` (new) — a sibling to the already-shipped `solfege.adj`
  (`solfege_degree(syllable, degree)`, ONE scale-degree number per syllable). That table's own
  `source` field already quotes, verbatim, an alternate spelling or name for two of the seven
  syllables — that the degree-only schema had no room for: do's cited span also states "(spelt doh
  in tonic sol-fa)" and ti's cited span also states "(or si)". New `solfege_alt_name(syllable,
  alt_name)` table: do → doh, ti → si. Honest abstention on the other five syllables (re, mi, fa,
  sol, la), whose own cited span states no alternate spelling or name. New e2e test file
  `facts_solfegealtname_e2e.rs` (3 tests: forward recall with citation, backward recall from a
  bound alternate, honest abstention on mi). No manifest objective, matching `solfege.adj`'s own
  precedent of not having one. First and only strong candidate from the music/ sweep tranche (the
  header's own scale-degree-name truth table — tonic, supertonic, mediant, etc. — was checked and
  confirmed to be non-verbatim author-compiled prose, not a quoted source span, so it's out of
  scope); music/ (1 table) is now effectively closed after this ships.
- `money/coin-penny-discontinued.adj` (new) — a sibling to the already-shipped `us-coins.adj`
  (`coin_cents(coin, cents)`, ONE cent-value per coin). That table's own header already quotes,
  verbatim, the penny's U.S. Mint Coin Classroom span ("The one-cent coin ceased circulating in
  2025 after 232 years of production."), and packed inside that SAME quote, alongside the cent
  value the parent table already captures, are two more atomic facts the value-only schema had no
  room for: a discontinuation status/year (2025) and a production span (232 years). New
  `coin_status(coin, status, year, production_years)` table: penny → (discontinued, 2025, 232).
  Deliberately a SINGLE row — every other coin carries no row at all, since none of their own
  cited U.S. Mint spans states any circulation-status change; abstention by simple absence, not an
  invented `circulating` status. New e2e test file `facts_coinpennydiscontinued_e2e.rs` (3 tests:
  forward recall with citation, recall with status pre-bound, honest abstention on other coins). No
  manifest objective, matching `us-coins.adj`'s own precedent of not having one. Second and final
  strong slice from the money/ sweep tranche — money/ (2 tables) is now EFFECTIVELY CLOSED (2/2
  strong candidates shipped); a moderate candidate (`coin-collectible-denominations.adj`, us-coins
  .adj's own everyday-vs-collectible partition) was deliberately not pursued, judged too thin
  (requires inferring a two-way classification from one descriptive sentence).
- `money/bill-back-vignette.adj` (new) — a sibling to the already-shipped `us-bills.adj`
  (`bill_portrait(dollars, portrait)`, ONE front-of-note portrait per bill). That table's own
  header already quotes, verbatim, the SAME U.S. Currency Education Program feature-sheet sentence
  used for the front portrait, and for six of the seven bills that same sentence also names the
  back-of-note vignette — that the portrait-only schema had no room for: 1 → great_seal,
  2 → declaration_signing, 10 → treasury_building, 20 → white_house, 50 → us_capitol,
  100 → independence_hall. New `bill_back_vignette(dollars, vignette)` table. Honest abstention on
  the $5 note, whose own cited feature-sheet sentence stops after the front portrait and names no
  back vignette. New e2e test file `facts_billbackvignette_e2e.rs` (3 tests: forward recall with
  citation, backward recall from a bound vignette, honest abstention on the $5 note). No manifest
  objective, matching `us-bills.adj`'s own precedent of not having one. First slice from the
  money/ sweep tranche.
- `environment/aqi-category-color.adj` (new) — a sibling to the already-shipped
  `air-quality-index.adj` (`air_quality_index(min_aqi, category)`, a RANGE/BRACKET lookup keyed by
  the numeric breakpoint of each of the six EPA AQI bands). That table's own per-row provenance
  already quotes, verbatim, a leading color word for every one of its six rows — the parent's own
  header even lays this out explicitly as a "source's colour" column in its truth table, shown
  "only to make the step visible" but never materialized as a row — that the `min_aqi, category`
  schema had no room for: good → green, moderate → yellow, unhealthy_for_sensitive_groups →
  orange, unhealthy → red, very_unhealthy → purple, hazardous → maroon. New
  `aqi_category_color(category, color)` table covers the full six-category domain with no
  abstention, since every category's cited span names a color. New e2e test file
  `facts_aqicategorycolor_e2e.rs` (3 tests: forward recall with citation, backward recall from a
  bound color, full-domain coverage with no abstention). No manifest objective, matching
  `air-quality-index.adj`'s own precedent of not having one. First and only strong candidate from
  the environment/ sweep tranche (environment/'s other table, `ecosystem-factor-type.adj`, was
  checked and confirmed to have nothing further viable); environment/ (2 tables) is now
  effectively closed after this ships.
- `agriculture/farm-animal-secondary-product.adj` (new) — a sibling to the already-shipped
  `farm-animals.adj` (`farm_animal_product(animal, product)`, ONE clear, source-stated product per
  animal: chicken eggs, duck eggs, sheep wool, rabbit wool, goat milk). That table's own per-row
  provenance already quotes, verbatim, a SECOND product for three of the five animals — that the
  one-product-per-animal schema had no room for: chicken's cited span also states "in eggs or
  meat," duck's cited span also states "produce meat and eggs," and sheep's cited span also states
  "produce wool, meat, and milk." New `farm_animal_secondary_product(animal, product)` table:
  chicken → meat, duck → meat, sheep → meat, sheep → milk. Honest abstention on rabbit and goat,
  whose own cited spans state only a USE ("used for fiber arts") or a PROCESSING note ("may be
  pasteurized") for their already-recorded product, not a genuinely different second product. New
  e2e test file `facts_farmanimalsecondaryproduct_e2e.rs` (3 tests: forward recall with citation,
  backward recall from a bound product, honest abstention on goat). No manifest objective, matching
  `farm-animals.adj`'s own precedent of not having one. First slice from the agriculture/ sweep
  tranche (1 strong candidate found; agriculture/ is now effectively closed after this ships).
- `metrology/time-unit-composition.adj` (new) — a sibling to the already-shipped `time-units.adj`
  (`time_unit_seconds(unit, seconds)`, ONE seconds-length per time unit: minute 60, hour 3600, day
  86400). That table's own `source` field already quotes, verbatim, a unit-to-unit relation for
  two of the three units — how many of the next smaller unit each one is composed of — that the
  seconds-only schema had no room for: "1 h = 60 min = 3600 s" and "1 d = 24 h = 86 400 s". New
  `time_unit_composition(unit, sub_unit, count)` table: hour → (minute, 60), day → (hour, 24).
  Honest abstention on minute, whose own cited span ("1 min = 60 s") states only its seconds-length
  with no unit-to-unit relation to a smaller unit on the cited page. New e2e test file
  `facts_timeunitcomposition_e2e.rs` (3 tests: forward recall with citation, backward recall from a
  bound sub-unit and count, honest abstention on minute). No manifest objective, matching
  `time-units.adj`'s own precedent of not having one. First slice from the metrology/ sweep tranche
  (1 strong + 1 moderate candidate found; the moderate one — a bare aggregate count of SI derived
  units — was deliberately not shipped, see loop-state notes).
- `geography/landform-secondary-feature.adj` (new) — a sibling to the already-shipped
  `landforms.adj` (`landform_description(landform, description)`, ONE defining descriptor per
  landform: mountain, valley, plateau, plain, canyon). That table's own header already quotes the
  FULL USGS Feature Type Thesaurus span for every row, but the descriptor-only schema reduced each
  span to a single atom, leaving a second, structural clause unused for three of the five
  landforms: valley's span also states "containing a stream with an outlet," plateau's span also
  states "limited on at least one side by an abrupt descent," and canyon's span also states "the
  bottom of which generally has a continuous slope." New `landform_secondary_feature(landform,
  feature)` table: valley → contains_stream_with_outlet, plateau → bounded_by_abrupt_descent,
  canyon → continuous_slope_at_bottom. Honest abstention on mountain and plain, whose own cited
  spans state only the single descriptor already captured by the parent table. New e2e test file
  `facts_landformsecondaryfeature_e2e.rs` (3 tests: forward recall with citation, backward recall
  from a bound feature, honest abstention on mountain). No manifest objective, matching
  `landforms.adj`'s own precedent of not having one. Third and last slice from the geography/ sweep
  tranche — geography/ (5 tables) is now fully exhausted (3/3 candidates shipped:
  reference-line-hemisphere-split, ocean-deepest, landform-secondary-feature).
- `geography/ocean-deepest.adj` (new) — a sibling to the already-shipped `oceans.adj`
  (`ocean_size_rank(ocean, rank)`, ONE size-rank per ocean basin: pacific 1, atlantic 2, indian 3,
  southern 4, arctic 5). That table's own `source` field already quotes a SECOND superlative,
  verbatim, in the very same opening sentence used to fix the Pacific's rank — "The Pacific Ocean
  is the largest and deepest of the world ocean basins" — that the rank-only schema had no room
  for. New `ocean_is_deepest(ocean, superlative)` table: pacific → deepest. Honest abstention on
  atlantic, indian, southern, and arctic, whose own cited span states only a size rank, never a
  depth claim. New e2e test file `facts_oceandeepest_e2e.rs` (3 tests: forward recall with
  citation, backward recall from a bound superlative, honest abstention on atlantic). No manifest
  objective, matching `oceans.adj`'s own precedent of not having one. Second slice from the
  geography/ sweep tranche (after `reference-line-hemisphere-split.adj`).
- `geography/reference-line-hemisphere-split.adj` (new) — a sibling to the already-shipped
  `reference-lines.adj` (`reference_line(line, marks)`, ONE degree-marking property per line:
  equator, prime_meridian, tropic_of_cancer, tropic_of_capricorn, arctic_circle,
  antarctic_circle). That table's own `source` and `cites` fields already quote a SECOND fact,
  verbatim, for two of the six lines — which pair of hemispheres each line divides the Earth
  into — that the marks-only schema had no room for: the equator's cited NOAA "What is
  latitude?" source states "it equally divides the Earth into the Northern and Southern
  hemispheres," and the prime meridian's cited NOAA "What is longitude?" corroboration states
  "It divides the Earth into the eastern and western hemispheres." New
  `reference_line_hemisphere_split(line, hemispheres)` table: equator → northern_southern,
  prime_meridian → eastern_western. Honest abstention on the two tropics and two polar circles,
  whose own cited spans state a latitude limit or polar ring, not a hemisphere split. New e2e
  test file `facts_referencelinehemispheresplit_e2e.rs` (3 tests: forward recall with citation,
  backward recall from a bound hemisphere pair, honest abstention on tropic_of_cancer). No
  manifest objective, matching `reference-lines.adj`'s own precedent of not having one. First
  slice from a fresh geography/ sweep (mathematics/ swept and closed empty first — see below).
- `geometry/polygon-alt-name.adj` (new) — a sibling to the already-shipped `shapes.adj`
  (`polygon_sides(shape, sides)`, ONE side-count per polygon: triangle, quadrilateral, pentagon,
  hexagon, heptagon, octagon, nonagon, decagon). That table's own `source` field already quotes
  the MathWorld "Polygon" names table's triangle row verbatim — "3 | triangle (trigon)" — but the
  sides-only schema had no room for the parenthetical alternate name that row also states. New
  `polygon_alt_name(shape, alt_name)` table: triangle → trigon. Honest abstention on the other
  seven polygons, whose own MathWorld "Polygon" names-table rows carry no parenthetical alternate
  name. New e2e test file `facts_polygonaltname_e2e.rs` (3 tests: forward recall with citation,
  backward recall from a bound alternate name, honest abstention on quadrilateral). No manifest
  objective, matching `shapes.adj`'s own precedent of not having one. Fifth and LAST slice from
  the geometry/ sweep tranche — geometry/ (9 files) is now fully exhausted (5/5 candidates from
  the original sweep shipped: radius-definition, quadrilateral-secondary-property,
  quadrilateral-alt-name, triangle-alt-name, polygon-alt-name).
- `geometry/triangle-alt-name.adj` (new) — a sibling to the already-shipped `triangle-types.adj`
  (`triangle_sides(triangle, condition)`, ONE defining side-condition per triangle side-class:
  equilateral, isosceles, scalene). That table's equilateral row's own already-quoted MathWorld
  sentence — "An equilateral triangle is a triangle with all three sides of equal length a,
  corresponding to what could also be known as a 'regular' triangle." — also names an ALTERNATE
  word for equilateral that the condition-only schema had no room for. New
  `triangle_alt_name(triangle, alt_name)` table: equilateral → regular. Honest abstention on
  isosceles and scalene, whose own cited spans name no alternate word. New e2e test file
  `facts_trianglealtname_e2e.rs` (3 tests: forward recall with citation, backward recall from a
  bound alternate name, honest abstention on isosceles). No manifest objective, matching
  `triangle-types.adj`'s own precedent of not having one. Fourth slice from the geometry/ sweep
  tranche.
- `geometry/quadrilateral-alt-name.adj` (new) — a sibling to the already-shipped
  `quadrilateral-types.adj` (`quadrilateral_property(shape, property)`, ONE defining property
  per quadrilateral: square, rectangle, rhombus, parallelogram, trapezoid). That table's rhombus
  row's own already-quoted MathWorld sentence — "A rhombus is a quadrilateral with both pairs of
  opposite sides parallel and all sides the same length, i.e., an equilateral parallelogram." —
  also names an ALTERNATE word for rhombus that the property-only schema had no room for. New
  `quadrilateral_alt_name(shape, alt_name)` table: rhombus → equilateral_parallelogram. Honest
  abstention on square, rectangle, parallelogram, and trapezoid, whose own cited spans name no
  alternate word. New e2e test file `facts_quadrilateralaltname_e2e.rs` (3 tests: forward recall
  with citation, backward recall from a bound alternate name, honest abstention on square). No
  manifest objective, matching `quadrilateral-types.adj`'s own precedent of not having one. Third
  slice from the geometry/ sweep tranche.
- `geometry/quadrilateral-secondary-property.adj` (new) — a sibling to the already-shipped
  `quadrilateral-types.adj` (`quadrilateral_property(shape, property)`, ONE defining property
  per quadrilateral: square, rectangle, rhombus, parallelogram, trapezoid). That table's
  `property` column has room for only the PRIMARY defining property; the SAME already-quoted
  MathWorld sentences also state a SECOND, distinct property for two of the five quadrilaterals:
  rectangle → opposite_sides_equal_length ("opposite sides of equal lengths a and b"),
  parallelogram → opposite_angles_equal ("opposite sides parallel (and therefore opposite angles
  equal)"). Honest abstention on square, rhombus, and trapezoid, whose own cited spans state only
  the single primary property. New e2e test file `facts_quadrilateralsecondaryproperty_e2e.rs` (3
  tests: forward recall of both with citation, backward recall from a bound property, honest
  abstention on square). No manifest objective, matching `quadrilateral-types.adj`'s own
  precedent of not having one. Second slice from the geometry/ sweep tranche.
- `geometry/radius-definition.adj` (new) — a sibling to the already-shipped `circle-parts.adj`
  (`circle_part(part, description)`, keyed by circle PART: radius, diameter, circumference,
  chord). This table recalls a DIFFERENT axis the SAME already-cited MathWorld "Radius" sentence
  states — what "radius" measures for TWO shapes, not just the circle `circle-parts.adj` already
  tables: `circle-parts.adj`'s own `source` field already quotes the full sentence "The distance
  from the center of a circle to its perimeter, or from the center of a sphere to its surface,"
  but that table's part-keyed schema had no row for the sphere half. New
  `radius_definition(shape, description)` table: circle → center_to_perimeter, sphere →
  center_to_surface. Honest abstention on any solid the cited sentence does not name (cube, cone,
  cylinder). New e2e test file `facts_radiusdefinition_e2e.rs` (3 tests: forward recall of both
  with citation, backward recall from a bound description, honest abstention on cube). No
  manifest objective, matching `circle-parts.adj`'s own precedent of not having one. First slice
  from a background Explore-agent sweep of `geometry/` (9 files) — found via a
  header/body-revisit of `circle-parts.adj`'s own `source` field (not even the header prose).
  Four more moderate-confidence candidates from the same sweep are queued
  (`quadrilateral-types.adj` second-clause properties, a rhombus/triangle alt-name pair, and a
  `shapes.adj` polygon alt-name — all single-to-few-row, deferred pending prioritization).
- `physics/energy-form-family.adj` (new) — a sibling to the already-shipped `energy-forms.adj`
  (`energy_form_token(form, token)`, ONE defining token per named energy form): a new
  `energy_form_family(form, family)` table names which of the EIA page's two families —
  potential or kinetic — each of the same eight forms belongs to. `energy-forms.adj`'s own
  header already summarized this split, but it was re-verified LIVE via WebFetch this cycle
  against the same already-cited EIA "Forms of energy" page: "Many forms of energy exist, but
  energy is either potential energy or kinetic energy," with chemical/mechanical/nuclear/
  gravitational listed under "Potential energy" and radiant/thermal/motion/electrical under
  "Kinetic energy" — byte-identical to the existing header's claim. Honest abstention on `sound`
  (a real form the same EIA page also lists under Kinetic, but not one of `energy-forms.adj`'s
  eight tabled forms). New e2e test file `facts_energyformfamily_e2e.rs` (3 tests: forward recall
  of all eight with citation, backward recall of all four kinetic forms, honest abstention on
  sound). No manifest objective, matching `energy-forms.adj`'s own precedent of not having one.
  Fourth and LAST slice from the physics/ sweep tranche — physics/ (21 files) is now fully
  exhausted except a weak, not-recommended `circuit-parts.adj` grab-bag.
- `physics/band-secondary-use.adj` (new) — a sibling to the already-shipped `em-spectrum.adj`
  (`band_use(band, application)`, ONE representative everyday use per EM band): a new
  `band_secondary_use(band, application)` table names a SECOND everyday use the SAME already-cited
  NASA "Imagine the Universe!" page states for a band, decoded from text already sitting unused
  inside `em-spectrum.adj`'s own provenance block — no new WebFetch. Two rows: microwave →
  astronomy, x_ray → airport_security. Honest abstention on the other five bands (radio, infrared,
  visible, ultraviolet, gamma_ray), whose own cited spans name only one use each. New e2e test file
  `facts_bandsecondaryuse_e2e.rs` (3 tests: forward recall of both with citation, backward recall
  from a bound application, honest abstention on radio). No manifest objective, matching
  `em-spectrum.adj`'s own precedent of not having one. Third slice from the physics/ sweep tranche.
- `physics/simple-machine-alt-example.adj` (new) — a sibling to the already-shipped
  `simple-machines.adj` (`simple_machine_example(machine, example)`, ONE everyday example per
  simple machine): a new `simple_machine_alt_example(machine, alt_example)` table names the SECOND
  everyday example the SAME already-cited NASA educator-notes page lists for a machine, decoded
  from spans already sitting unused inside `simple-machines.adj`'s own header and provenance
  block — no new WebFetch. Five rows: lever → scissors, inclined_plane → stairs, wedge → knife,
  wheel_and_axle → clock, pulley → water_well. Honest abstention on `screw` (its first-listed
  example is literally the word "screw", not a distinct object, and its second, "bottle caps", is
  already `simple-machines.adj`'s primary row — no unused second example remains). New e2e test
  file `facts_simplemachinealtexample_e2e.rs` (3 tests: forward recall of all five with citation,
  backward recall from a bound alternate example, honest abstention on screw). No manifest
  objective, matching `simple-machines.adj`'s own precedent of not having one. Second slice from
  the physics/ sweep tranche.
- `physics/phase-change-alt-name.adj` (new) — a sibling to the already-shipped
  `states-of-matter.adj` (`phase_change_name(change, name)`, ONE primary name per phase-change
  direction): a new `phase_change_alt_name(change, alt_name)` table names the older/alternate word
  the SAME already-cited LibreTexts pages state for a direction, decoded from spans already
  sitting unused inside `states-of-matter.adj`'s own header and provenance block — no new
  WebFetch. Three rows: solid_to_liquid → fusion, liquid_to_solid → solidification, liquid_to_gas
  → boiling, all read off the SAME already-quoted spans that table's single-name schema had no
  room for. Deliberately narrow: only these three of the table's six directions have an alternate
  name in the already-cited spans. New e2e test file `facts_phasechangealtname_e2e.rs` (3 tests:
  forward recall of all three with citation, backward recall from a bound alternate name, honest
  abstention on condensation). No manifest objective, matching `states-of-matter.adj`'s own
  precedent of not having one. Discovered via a background Explore-agent sweep of `physics/` (21
  files, a previously untouched large domain) — first slice from a newly-discovered tranche of
  unswept domains (agriculture/art/calendar/environment/geography/geometry/mathematics/
  metrology/money/music/nutrition/optics/physics/transportation).
- `oceanography/ocean-zone-depth.adj` (new) — a sibling to the already-shipped `ocean-zones.adj`
  (`ocean_zone(zone, order)`, ONE ordinal position per depth zone): a new
  `ocean_zone_depth(zone, max_depth_meters)` table names the approximate depth in meters the SAME
  already-cited WHOI "Ocean Zones" page states for each zone, decoded from spans already sitting
  unused inside `ocean-zones.adj`'s own header truth table — no new WebFetch. Three rows:
  sunlight_zone → 200, twilight_zone → 1000, midnight_zone → 4000, all read off the SAME
  already-quoted WHOI spans that table's single-order schema had no room for. New e2e test file
  `facts_oceanzonedepth_e2e.rs` (3 tests: forward recall with citation, backward recall from a
  bound depth, honest abstention on the abyssal zone). New manifest objective
  `adj.science.3to5.ocean_zone_depth`, matching `ocean-zones.adj`'s own precedent of having one.
- `meteorology/precipitation-minimum-diameter.adj` (new) — a sibling to the already-shipped
  `precipitation-types.adj` (`precip_form(precip, form)`, ONE defining physical form per
  precipitation type): a new `precipitation_min_diameter(precip, min_diameter_mm)` table names
  the numeric diameter threshold the SAME NOAA NWS Glossary states for a type, decoded from spans
  already sitting unused inside `precipitation-types.adj`'s own provenance block — no new
  WebFetch. Two rows: rain → 0.5, hail → 5, both read off the SAME already-quoted NWS Glossary
  spans that table's single-form schema had no room for. Deliberately narrow: only these two of
  the table's five precipitation types have a diameter figure in the already-cited spans. New e2e
  test file `facts_precipitationmindiameter_e2e.rs` (3 tests: both-term recall with citation,
  backward recall from a bound diameter, honest abstention on snow). No manifest objective,
  matching `precipitation-types.adj`'s own precedent of not having one.
- `meteorology/hurricane-category-home-damage.adj` (new) — a sibling to the already-shipped
  `hurricane-categories.adj` (`damage_level(category, descriptor)`, ONE generic damage word per
  Saffir-Simpson category): a new `hurricane_home_damage(category, home_damage_effect)` table
  names the SPECIFIC well-built-home structural effect the SAME NHC page describes at each
  category, decoded from spans already sitting unused inside `hurricane-categories.adj`'s own
  provenance block — no new WebFetch. Five rows, all distinct (unlike the parent table's
  descriptor column, which honestly duplicates `catastrophic_damage` for categories 4 and 5).
  New e2e test file `facts_hurricanehomedamage_e2e.rs` (3 tests: forward recall with citation,
  backward recall from a bound effect, honest abstention on category 6). No manifest objective,
  matching `hurricane-categories.adj`'s own precedent of not having one.
- `geology/fossil-preservation-subtype.adj` (new) — a sibling to the already-shipped
  `fossil-preservation-type.adj` (`fossil_preservation_type(type, description)`, THREE peer-level
  preservation structures: mold, cast, trace_fossil): a new
  `fossil_preservation_subtype(subtype, parent_type, description)` table names a SPECIFIC KIND of
  one of those three structures, decoded from the `steinkern` definition already quoted in full
  inside `fossil-preservation-type.adj`'s own header — no new WebFetch. That table deliberately
  excludes `steinkern` as a fourth peer row because the SAME source page frames it as a specific
  kind of `cast` (an internal cast), not a fourth preservation structure — a classificatory/scope
  decision, not a correctness one. One row: steinkern → cast. New e2e test file
  `facts_fossilpreservationsubtype_e2e.rs` (3 tests: forward recall with citation, backward recall
  from a bound parent type, honest abstention on a peer type). New manifest objective
  `adj.science.3to5.fossil_preservation_subtype`, matching `fossil-preservation-type.adj`'s own
  precedent of having one (the same shape already used for `comet-part.adj` →
  `comet-tail-type.adj`).
- `geology/earth-layer-thickness.adj` (new) — a sibling to the already-shipped `earth-layers.adj`
  (`has_state(layer, state)`, ONE physical-state fact per layer): a new
  `earth_layer_thickness(layer, thickness_km)` table names the THICKNESS in kilometers the SAME
  already-cited USGS page states for a layer, decoded from spans already sitting unused inside
  `earth-layers.adj`'s own header and provenance block — no new WebFetch. Three rows: mantle →
  2900, outer_core → 2200, inner_core → 1250, all read off the SAME already-quoted USGS spans that
  table's single-state schema had no room for. Deliberately narrow: the crust's own span states
  only that it is "very thin," with no figure to decode, so it abstains honestly. New e2e test
  file `facts_earthlayerthickness_e2e.rs` (3 tests: forward recall of all three figures with
  citation, backward recall from a bound thickness, honest abstention on the crust). No manifest
  objective, matching `earth-layers.adj`'s own precedent of not having one.
- `astronomy/celestial-object-alt-name.adj` (new) — a sibling to the already-shipped
  `celestial-objects.adj` (`celestial_property(object, property)`, ONE defining property per
  object): a new `celestial_object_alt_name(object, alt_name)` table names an ALTERNATE NAME the
  source uses for an object, decoded from spans already sitting unused inside
  `celestial-objects.adj`'s own provenance block — no new WebFetch. Two rows: moon →
  planetary_satellites, asteroid → minor_planets, both read off the SAME already-quoted NASA
  spans that table's single-property schema had no room for. Deliberately narrow: only these two
  of the table's five basic types have an alternate name in the already-cited spans. New e2e test
  file `facts_celestialobjectaltname_e2e.rs` (2 tests: both-term recall with citation, honest
  abstention on an object with no cited alternate name). No manifest objective, matching
  `celestial-objects.adj`'s own precedent of not having one.
- `biology/animal-baby-sex.adj` (new) — a sibling to the already-shipped `animal-babies.adj`
  (`animal_baby(animal, baby)`, ONE generic baby name per animal): a new `animal_baby_sex(animal,
  sex, baby)` table names the SEX-SPECIFIC baby term the source distinguishes, decoded from a
  span already sitting unused inside `animal-babies.adj`'s own `source` field
  (`"colt (male), filly (female), foal, weanling, yearling"`) — no new WebFetch. Of the 24
  animals `animal-babies.adj` ships, `horse` is the ONLY one whose Wikipedia "Young" cell draws a
  clean male/female split, so this table is honestly narrow: one animal, two rows (male → colt,
  female → filly). New e2e test file `facts_animalbabysex_e2e.rs` (2 tests: both-term recall
  with citation, honest abstention on an animal the source does not sex-distinguish). No manifest
  objective, matching `animal-babies.adj`'s own precedent of not having one.
- `biology/amino-acid-three-letter-code.adj` (new) — a sibling to the already-shipped
  `amino-acids.adj` (`amino_acid_code(amino_acid, code)`, the ONE-letter code): a new
  `amino_acid_three_letter_code(amino_acid, code)` table names the OTHER column the SAME
  already-cited DDBJ page tables, which the existing table's schema had no room for.
  `amino-acids.adj`'s own header already explains the source page has three columns
  (3-letter/1-letter/name) and quotes the verbatim triple for glycine, but only ever paired the
  name with the one-letter code. WebFetch-verified TWICE this cycle for consistency, both
  byte-identical: all twenty standard amino acids' three-letter codes read directly off the same
  page. New e2e test file `facts_aminoacidthreeletter_e2e.rs` (3 tests: both-direction recall,
  the two acidic residues, honest abstention on a non-standard amino acid). No manifest
  objective, matching `amino-acids.adj`'s own precedent of not having one.
- `biology/start-codon.adj` (new) — a sibling to the already-shipped `genetic-code.adj`
  (`codon_amino_acid(codon, amino_acid)` over all 64 codons): a new `start_codon(codon, role)`
  table names WHICH codons NCBI's own `Starts` annotation line flags, decoded from the SAME
  verbatim five-line block already quoted inside `genetic-code.adj`'s `source` field — no new
  WebFetch. The `Starts` line carries exactly three `M`s (columns 4, 20, 36), decoding to `ttg`,
  `ctg`, `atg`. Deliberately does NOT pair these with an amino acid (translation always initiates
  with the same methionine regardless of which start codon recruited it, so a `(codon,
  amino_acid)` pairing for `ttg`/`ctg` would misstate biology); `role` names only what the source
  itself asserts. New e2e test file `facts_startcodon_e2e.rs` (3 tests: recall the primary start
  codon, recall both alternative start codons, honest abstention on an unflagged codon). No
  manifest objective, matching `genetic-code.adj`'s own precedent of not having one.
- `language/syllable-blending.adj` (new) — a new THIRTEENTH literacy sub-skill library, the exact
  OPPOSITE direction from `syllable-segmentation.adj`: `syllable_blending(syllable_one,
  syllable_two, word)` names the word formed by BLENDING two separate syllables together, the
  same "opposite direction, separate table" shape already proven by `phoneme-blending.adj` versus
  `phoneme-segmentation.adj`. One row: (lap, top, laptop). Discovered via a full inventory of
  Reading Rockets' "Phonological and Phonemic Awareness: In Practice" page's named sections, in
  its distinct "Blending syllables" section (not the "Segmenting syllables" section
  `syllable-segmentation.adj` cites). WebFetch-verified THREE separate times for consistency
  before writing, all byte-identical. New e2e test file `facts_syllableblending_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention on an untabled pair). New manifest objective
  `adj.literacy.k2.syllable_blending` (173 -> 174 total).
- `language/syllable-segmentation.adj` (new) — a new TWELFTH literacy sub-skill library, a sibling
  to the already-shipped `syllable-count.adj` (which recalls only HOW MANY syllables a word has):
  `syllable_segmentation(word, syllable_one, syllable_two)` names the actual two parts each word
  breaks into. Discovered via a header-revisit: `syllable-count.adj`'s own header already quotes,
  verbatim, the exact syllable split for all four of its words ("pea"/"nut", "pen"/"cil",
  "sun"/"set", "lap"/"top"), but that table's `syllable_count(word, count)` schema has no room for
  the actual syllable text -- the same "sibling table, different schema" shape already used for
  `comet-part.adj`/`comet-tail-type.adj`, `lung-lobes.adj`/`lung-lobe-names.adj`, and
  `chemical-bonds.adj`/`chemical-bond-family.adj`. Four rows: peanut/pea/nut, pencil/pen/cil,
  sunset/sun/set, laptop/lap/top. WebFetch-verified TWICE more this cycle for consistency, both
  byte-identical to `syllable-count.adj`'s existing quotes. New e2e test file
  `facts_syllablesegmentation_e2e.rs` (4 tests: direct recall, reverse binding, coverage of all
  four words, honest abstention on an untabled word). New manifest objective
  `adj.literacy.k2.syllable_segmentation` (171 -> 172 total, matching `syllable-count.adj`'s own
  precedent of having a manifest objective).
- `language/phoneme-segmentation.adj` (new) — a new ELEVENTH literacy sub-skill library, the
  exact OPPOSITE direction from `phoneme-blending.adj`: `phoneme_segmentation(word, sound_one,
  sound_two, sound_three)`. Where `phoneme-blending.adj` composes three sounds into a word, this
  decomposes a word into its three sounds. One row: (feet, f, ee, t). Discovered via a fresh
  WebFetch of the same already-vetted Reading Rockets "In Practice" page's "Segmenting sounds in
  a syllable" section, not yet explored by any prior slice. WebFetch-verified THREE separate
  times for consistency before writing, all byte-identical. New e2e test file
  `facts_phonemesegmentation_e2e.rs` (3 tests: direct recall of the three sounds, reverse
  binding into the word, honest abstention on an untabled word). New manifest objective
  `adj.literacy.k2.phoneme_segmentation` (170 -> 171 total).
- `language/phoneme-addition.adj` (new) — a new TENTH literacy sub-skill library, the narrowest
  sibling of `phoneme-blending.adj`: `phoneme_addition(sound_one, sound_two, word)`. Same
  direction as `phoneme-blending.adj` (sounds combining INTO a word), but for exactly TWO sounds
  rather than three, a distinct arity the cited page treats as its own named skill ("Adding
  sounds," distinct from "Blending sounds"). One row: (i, s, ice). Discovered via the SAME
  already-vetted Reading Rockets "In Practice" page's "Adding sounds" section, explicitly flagged
  as a future candidate in `phoneme-blending.adj`'s own header when that library shipped.
  WebFetch-verified THREE separate times across this session for consistency, all byte-identical.
  New e2e test file `facts_phonemeaddition_e2e.rs` (3 tests: direct recall, reverse binding into
  both sounds, honest abstention on an untabled addition). New manifest objective
  `adj.literacy.k2.phoneme_addition` (170 -> 171 total).
- `language/phoneme-blending.adj` (new) — a new NINTH literacy sub-skill library:
  `phoneme_blending(sound_one, sound_two, sound_three, word)`, the OPPOSITE direction from
  `phoneme-deletion.adj`/`phoneme-substitution.adj` (which decompose or swap sounds in a word):
  this recalls the word formed by BLENDING three separate sounds together. One row: (s, o, p,
  soap). Discovered by re-visiting the SAME already-vetted Reading Rockets "In Practice" page
  (already cited by `syllable-count.adj`/`phoneme-substitution.adj`/`phoneme-deletion.adj`/
  `syllable-substitution.adj`/`syllable-deletion.adj`/`onset-rime.adj`) for its "Blending sounds"
  section, distinct from its "Adding sounds" section (which blends only TWO sounds, a different
  arity, left as a future `phoneme-addition.adj` candidate). WebFetch-verified THREE separate
  times for consistency before writing, all byte-identical. New e2e test file
  `facts_phonemeblending_e2e.rs` (3 tests: direct recall, reverse binding into all three sounds,
  honest abstention on an untabled blend). New manifest objective
  `adj.literacy.k2.phoneme_blending` (169 -> 170 total).
- `chemistry/acids-bases.adj` (extended) — extended the already-shipped `acid_or_base(substance,
  classification)` table from 12 to 16 rows. This table's own header had ALWAYS explicitly stated
  that the source's acid column also lists HClO3 (chloric_acid) and its base column also lists
  RbOH (rubidium_hydroxide), CsOH (caesium_hydroxide), and Sr(OH)2 (strontium_hydroxide),
  explicitly noting they were "omitted only to keep the set balanced at six acids and six bases --
  nothing about them was uncertain." Since the header itself already confirmed these four are
  grounded in the same already-cited LibreTexts table, adding them is a pure addition sharing the
  existing acid/base classification -- the same header-revisit extend-pattern shape already proven
  on `element-groups.adj`. WebFetch re-verified the full source table live, TWO separate passes,
  both confirming all sixteen rows byte-identical. New e2e test
  `chemistry_acid_or_base_recalls_the_four_substances_added_this_cycle`. No new manifest objective
  (this library has never had one, matching the no-manifest precedent already established for
  sibling chemistry tables).
- `chemistry/chemical-bond-family.adj` (new) — a new sibling to the already-shipped
  `chemical-bonds.adj` (which recalls only each bond's single defining TOKEN): a new
  `bond_family(bond, family)` table names which of the source's two families -- PRIMARY (strong)
  or SECONDARY (weak) -- each of the four bond types falls into. Discovered via a header-revisit:
  `chemical-bonds.adj`'s own header already quotes the LibreTexts sentence classifying all four
  bonds ("Primary bonding (ionic, covalent and metallic) is strong ... In contrast, secondary
  bonding is weak ..."), but that table's header explicitly says "the family split is not part of
  this table" -- a deliberate single-axis scope decision, not a missing citation. Since the fact
  is cleanly grounded in the SAME already-cited sentence, it earns its own sibling table with a
  DIFFERENT schema, the same "sibling table, different schema" shape already used for
  `comet-part.adj`/`comet-tail-type.adj` and `lung-lobes.adj`/`lung-lobe-names.adj`.
  WebFetch re-verified the sentence live, TWO separate passes, both byte-identical. The
  reverse-binding query on `primary` is a genuine one-to-many recall (ionic ; covalent ;
  metallic). New e2e test `facts_bondfamily_e2e.rs`. No manifest objective added, matching
  `chemical-bonds.adj`'s own precedent (not wired into the loop's manifest coverage tracking).
- `anatomy/lung-lobe-names.adj` (new) — a new sibling to the already-shipped `lung-lobes.adj`
  (which recalls only lobe COUNT per lung): a new `lung_lobe_name(lobe, lung)` table names each
  of the five individual lobes and which lung it belongs to (right_upper_lobe/right_middle_lobe/
  right_lower_lobe -> right_lung; left_upper_lobe/left_lower_lobe -> left_lung). Discovered via a
  header-revisit: `lung-lobes.adj`'s own header already quotes the corroborating StatPearls
  sentence naming all five lobes, but that table's `lung_lobe_count(lung, count)` schema has no
  room for individual lobe names -- a genuinely new predicate/table, the same "sibling table,
  different schema" shape already used for `comet-part.adj`/`comet-tail-type.adj`, rather than an
  extend of the existing table. WebFetch re-verified the sentence live, THREE separate passes, all
  byte-identical to what `lung-lobes.adj`'s header already quoted. The reverse-binding query on
  `right_lung` is a genuine one-to-many recall (all three right lobes). New e2e test
  `facts_lunglobenames_e2e.rs`. No manifest objective added, matching `lung-lobes.adj`'s own
  precedent -- this anatomy sub-family (also including `kidney-parts.adj`/`long-bone-parts.adj`)
  is not wired into the loop's manifest coverage tracking.
- `language/onset-rime.adj` (extended) — extended the already-shipped `onset_rime(word, onset,
  rime)` table from 2 to 4 rows. Discovered via the SAME fresh WebFetch pass that surfaced
  `syllable-deletion.adj`: Reading Rockets' "Phonological and Phonemic Awareness: In Practice"
  module -- a page already cited by `syllable-count.adj`/`phoneme-substitution.adj`/
  `phoneme-deletion.adj`/`syllable-substitution.adj`/`syllable-deletion.adj` -- has a "Blending
  Onset and Rime" section (map -> m/ap) and an "Onset-rime Completion" section (tape -> t/ape),
  each naming a word/onset/rime triple in the EXACT shape `onset-rime.adj` already carries (a
  header-revisit-style discovery, but from a DIFFERENT already-cited page than the one the
  original two rows came from, rather than the same table's own header). Both new rows are a
  pure addition; the original sleep/blast rows are untouched. WebFetch-verified THREE separate
  times for consistency before writing, all byte-identical (the third pass surfaced an even
  cleaner clean-prose sentence for `map`: "So in the word 'map,' /m/ is the onset and /ap/ is
  the rime."). New e2e test `onset_rime_extension_recalls_the_newly_added_map_and_tape_splits`.
  No new manifest objective (same library, same objective, 169 total unchanged).
- `language/syllable-deletion.adj` (new) — a new EIGHTH literacy sub-skill library:
  `syllable_deletion(original_word, removed_syllable, new_word)`, the syllable-level analogue of
  `phoneme-deletion.adj` the same way `syllable-substitution.adj` is the syllable-level analogue
  of `phoneme-substitution.adj`. Discovered while scoping the next literacy slice: all five
  previously-scoped literacy candidates (syllable-count.adj, onset-rime.adj, initial-sound.adj,
  phoneme-substitution.adj, possessive-noun.adj) turned out to be honestly narrow-by-design with
  no unused header material -- a fresh WebFetch of the SAME already-vetted Reading Rockets "In
  Practice" page (`syllable-count.adj`/`phoneme-substitution.adj`/`phoneme-deletion.adj`/
  `syllable-substitution.adj` all already cite it) surfaced its "Deleting syllables" section,
  distinct from the "Deleting Sounds" section `phoneme-deletion.adj` draws from: `row(pencil,
  cil, pen)`. WebFetch-verified THREE separate times for consistency before writing, all
  byte-identical. New library, new e2e test file `facts_syllabledeletion_e2e.rs` (3 tests: direct
  recall, reverse binding, honest abstention on an untabled deletion). New manifest objective
  `adj.literacy.k2.syllable_deletion` (168 -> 169 total).
- `chemistry/element-groups.adj` (extended) — extended the already-shipped
  `element_group_family(element, family)` table from 14 to 28 rows. Each of the five families'
  own already-cited Wikipedia sentence, quoted in full in the header since this table first
  shipped, named MORE member elements than had ever been turned into rows — only the first
  three of each family had been. Added rubidium, caesium, francium (alkali_metal); strontium,
  barium, radium (alkaline_earth_metal); iodine, astatine, tennessine (halogen); krypton,
  xenon, radon (noble_gas); cobalt (transition_metal) — the fourth extend-pattern shape
  (revisit an already-shipped table's own header for material named but never turned into
  rows/values), already proven on `pronoun-type.adj`/`contraction.adj`/`brain-parts.adj`, this
  time across FIVE keys in one table rather than one. Deliberately did NOT add `oganesson`
  even though the noble_gas sentence names it: the source hedges it as a noble gas only "in
  some cases" (its predicted chemistry is debated), so unlike every other name in the same
  sentences that clause does not support a firm row — a considered exclusion, not an
  oversight. The reverse-binding query on `noble_gas` now recalls all six shipped noble gases
  instead of three; the query file and new e2e test
  `chemistry_element_group_family_extension_recalls_newly_added_elements` were updated/added
  accordingly. No new manifest objective (same library, same objective, 168 total unchanged).
- `anatomy/brain-parts.adj` (extended) — extended the already-shipped `brain_part_function(
  brain_part, function)` table from 6 to 15 rows. The `brainstem` row's own cited StatPearls
  source sentence always listed TEN autonomic functions ("breathing, temperature regulation,
  respiration, heart rate, wake-sleep cycles, coughing, sneezing, digestion, vomiting, and
  swallowing"), but only the first, `breathing`, had ever been turned into a row. Added the
  other nine as new rows sharing the same `brainstem` key -- the same many-valued-per-key
  shape `kingdoms.adj`/`opposites.adj`/`synonyms.adj` already established -- so the original
  `row (brainstem, breathing)` is untouched and every new row is a pure addition, not a
  restructure. This is distinct from the `insect-parts.adj` case checked earlier this window,
  where the unused header material lives inside an existing COMPOUND atom on a single row
  (`digestion_and_reproduction`) rather than as a candidate for new standalone rows -- that
  table was correctly left alone since splitting it would alter an already-shipped fact rather
  than purely add to the table. WebFetch re-verified the brainstem sentence live before
  writing -- byte-identical to what the header already quoted. New e2e test
  `anatomy_brain_parts_extension_recalls_newly_added_brainstem_functions`. No new manifest
  objective (same library, same objective, 168 total unchanged).
- `language/contraction.adj` (extended) — extended the already-shipped `contraction(word,
  expansion)` table from 3 to 16 rows. The original dont/cant/wont rows turn out to be exactly
  THREE members of a larger 16-row "Negative Contractions" table on the same already-cited
  Grammarly page -- every negative contraction the source lists, each with exactly ONE
  unambiguous expansion. Added arent, couldnt, didnt, doesnt, hadnt, hasnt, havent, isnt,
  mustnt, shouldnt, wasnt, werent, wouldnt -> are_not/could_not/did_not/does_not/had_not/
  has_not/have_not/is_not/must_not/should_not/was_not/were_not/would_not. Deliberately did
  NOT pull from the page's separate "Common Contractions" table, whose entries (e.g. `he's`
  -> "he has, he is") are genuinely ambiguous -- WebFetch re-verified the full Negative
  Contractions table across two separate passes, both confirming byte-identical rows and that
  every row has a single expansion with no comma-separated alternatives. `shouldnt` was
  previously this table's own honest-abstention example; the query file and e2e tests were
  updated to use `hes` (the genuinely ambiguous case) as the new abstention example instead.
  New e2e test `contraction_extension_recalls_newly_added_negative_contractions`. No new
  manifest objective (same library, same objective, 168 total unchanged).
- `language/pronoun-type.adj` (extended) — extended the already-shipped `pronoun_type(type,
  description)` table from 3 to 4 rows. Added `distributive_pronoun` -> `refers_to_nouns_as_
  individual_elements_of_larger_groups`, quoted verbatim from the same already-cited Grammarly
  "Pronouns: Definition and Examples" page. Discovered via a careful WebFetch re-check of the
  page's reflexive/intensive/possessive/reciprocal/distributive material the header had always
  named but never fully investigated — two other candidates (intensive_pronoun, reciprocal_pronoun)
  were investigated and REJECTED after multiple WebFetch passes surfaced inconsistent/non-defining
  sentences for them (intensive's own sentence is a comparison that never states its actual
  function; reciprocal's own sentence names a closed word-list rather than a function), while
  distributive_pronoun's sentence was confirmed byte-identical across four separate fetches and
  matches the table's existing "type refers to/is X" pattern. New e2e test
  `pronoun_type_extension_recalls_the_newly_added_distributive_pronoun`, alongside the 3
  pre-existing tests. No new manifest objective (same library, same objective, 168 total unchanged).
- `astronomy/comet-tail-type.adj` (new) — a sibling library to the already-shipped `comet-part.adj`:
  a new `comet_tail_type(tail, description)` table names the two separate tails a comet actually
  has (dust_tail, ion_tail) and the defining path each one traces, quoted verbatim from the SAME
  NASA Space Place "What Is a Comet?" page `comet-part.adj` already cites — discovered while
  investigating that page's own header note that it "goes on to further sub-divide the tail into
  a dust tail and an ion tail," a genuinely new sub-question (not a `comet-part.adj` extend, since
  dust/ion are sub-types of the single `tail` part, not new peer-level physical parts) using an
  already-fetched source, re-verified live before writing. Honest abstention on `coma`/`nucleus`
  (real comet parts, already tabled in the coarser sibling `comet-part.adj`, not tail sub-types).
  This is genuinely a CLOSED two-way split — the source's own sentence states a comet has exactly
  two separate tails. New manifest objective `adj.science.3to5.comet_tail_type` (168 total,
  prerequisite on `adj.science.3to5.comet_part`) — the first genuinely NEW-topic content slice in
  a while, after a run of extend-pattern wins; a full sweep of ~20 other science-lane tables this
  cycle found nothing else extendable (all closed/exhaustive sets or documented considered
  exclusions). New e2e test `facts_comettailtype_e2e.rs` (3 tests: direct recall, reverse binding,
  honest abstention on a different physical part).
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
- Added this `CHANGELOG.md` (a standing gap flagged during the adj-curriculum loop's Wave 2
  literacy scoping pass, mirroring the same fix already made for `adj-formula-stdlib/`).
- `language/word-families.adj` (new) — the FIRST literacy-domain library in the ADJ stdlib.
  `word_family(word, family)` tables the "-an" word family's six core CVC (consonant-vowel-
  consonant) members (pan, fan, ran, man, tan, van), quoted verbatim from Reading Rockets'
  "Meet the Word Families." A `rule` DERIVES `rhymes_with(word1, word2)` from shared family
  membership — composing the specific family-membership fact with the general word-family
  principle ("words look alike at the end if they sound alike at the end"), the same
  rule-based-inference discipline `geometry/shape-composition.adj` already established, so
  this satisfies Wave 2's own requirement that "each tranche must include composition... not
  only direct recall" (`ADJ-STDLIB-COVERAGE.md` §7). Grounds CCSS RF.K.2.a ("Recognize and
  produce rhyming words"). Deliberately scoped to ONE family with its plain three-letter core
  members only — not the full 37-family phonics inventory, nor the longer blended/multi-
  syllable "-an" words the same source also lists (plan, scan, bran, began) — keeping this
  first slice small and citable, the same discipline every prior curriculum item in this loop
  has used. New manifest objective `adj.literacy.k2.rhyming_word_families` (the first
  literacy-domain manifest entry; also introduces the `ccss.ela` coverage root and `literacy`
  domain values, mirroring how `adj.science.clinical.bmi` introduced the first clinical-domain
  entry one PR earlier). New e2e test `facts_wordfamilies_e2e.rs` (3 tests: direct family
  recall, derived rhyme composition, honest abstention on an unshipped word).
- `physics/heat-causes-phase-change.adj` (new) — the FIRST causal-explanation library in the
  ADJ stdlib's science domain (ADJ-STDLIB-COVERAGE.md §5.1's Science row names "causal
  explanations" as a Major Gap). A new `heat_direction(change, direction)` table names which
  of the four everyday phase changes heat flows IN for (`heating`) and which it flows OUT for
  (`cooling`), quoted verbatim from the SAME LibreTexts page the sibling `states-of-matter.adj`
  already cites. A `rule` DERIVES `causes_phase_change(direction, name)` by composing this new
  table with the ALREADY-SHIPPED `phase_change_name` table from that sibling library — a
  genuine CROSS-FILE composition (not just within one file, like `word-families.adj`), proving
  the stdlib's own stated goal that "an AI agent working in a domain can reason through this
  library the way a student reasons up from foundations." Grounds NGSS 2-PS1-4. Deliberately
  scoped to the four transitions the cited sentence names directly (melting, freezing,
  vaporization, condensation) — NOT sublimation or deposition, which the same source describes
  only by temperature/pressure condition, not a parallel heat-direction sentence, so asserting
  a row for either would outrun what is actually cited. New manifest objective
  `adj.science.k2.heat_causes_phase_change` (band K-2, matching NGSS 2-PS1-4's own grade level;
  uses the `infer` competency for a `rule`-derived fact, mirroring
  `adj.math.k2.spatial_composition`'s precedent). New e2e test `facts_heatphasechange_e2e.rs`
  (2 tests).
- `language/word-families.adj` — extended with a SECOND word family, "-at" (cat, bat, fat, sat,
  rat, pat, mat, hat), added as eight new rows in the existing `word_family` table alongside the
  "-an" family shipped in the prior slice. The existing `rhymes_with` rule is reused UNCHANGED —
  it generalizes over any `$Family` value already in the table, so this slice required zero rule
  or engine changes, demonstrating the composition pattern scales to new vocabulary for free.
  Quoted verbatim from a DIFFERENT Reading Rockets page than the "-an" family's (a kindergarten
  phonological-awareness parent guide), documented in the file's header prose per the "one table,
  one declared provenance envelope, every other row-group's real citation in prose" discipline
  `physics/states-of-matter.adj` established. Deliberately excludes "flat" (a four-letter
  consonant-blend word) to preserve the strict three-letter CVC scope. No new manifest objective
  needed — extends the same already-covered library `adj.literacy.k2.rhyming_word_families`
  already references. New e2e test `rhymes_with_isolates_a_second_family_with_the_same_unmodified_rule`
  (4th test in `facts_wordfamilies_e2e.rs`), which also asserts NO cross-contamination between
  the two families.
- `physics/force-causes-acceleration.adj` (new) — the SECOND causal-explanation library in the
  ADJ stdlib's science domain, following `heat-causes-phase-change.adj`'s precedent. A `rule`
  composes Newton's second law's own general statement (from the already-shipped
  `newton-laws.adj`) with a specific force→example fact (from the already-shipped `forces.adj`)
  to DERIVE `force_causes_acceleration(force, example)` — a genuine CROSS-FILE composition
  reusing TWO already-verified NASA citations with zero new sourcing work. Grounds NGSS MS-PS2-2.
  Deliberately scoped to the second law only (F = m·a governs any force's relationship to
  acceleration) — the first law (inertia) and third law (action-reaction) describe different
  causal relationships, left for a later pass. New manifest objective
  `adj.science.6to8.force_causes_acceleration` (band 6-8, uses the `infer` competency for a
  rule-derived fact). New e2e test `facts_forcecausesacceleration_e2e.rs` (3 tests: direct
  derivation with dual citations, reverse binding, honest abstention on an untabled force).
- `language/word-families.adj` — extended with a THIRD word family, "-ig" (big, pig, fig, dig,
  wig), added as five new rows in the existing `word_family` table alongside "-an" and "-at". The
  existing `rhymes_with` rule is again reused UNCHANGED. Quoted verbatim from a THIRD source, Super
  Teacher Worksheets (a widely-used K-5 phonics teaching-resource site, `trust consensus`, same
  tier as the other two): "Words include: big, pig, fig, dig, wig and twig." — WebFetch-verified
  twice for consistency before writing. Deliberately excludes "twig" (a four-letter
  consonant-blend word) to preserve the strict three-letter CVC scope, the same discipline "-an"
  and "-at" already established. No new manifest objective needed — extends the same
  already-covered library `adj.literacy.k2.rhyming_word_families`. New e2e test
  `rhymes_with_isolates_a_third_family_and_abstains_on_the_excluded_blend_word` (5th test in
  `facts_wordfamilies_e2e.rs`), which asserts NO cross-contamination with either prior family AND
  honest abstention on "twig".
- `language/word-families.adj` — extended with a FOURTH word family, "-ug" (tug, rug, hug, mug,
  jug, dug, bug), added as seven new rows in the existing `word_family` table alongside "-an",
  "-at", and "-ig". The existing `rhymes_with` rule is reused UNCHANGED for the fourth time
  running. Quoted verbatim from a SIBLING Super Teacher Worksheets page to "-ig"'s (same site,
  same `consensus` trust tier, its own real citation): "This printable word family unit covers
  words that end with the letters -ug. List includes: snug, plug, slug, shrug, tug, rug, hug, mug,
  jug, dug, and bug." — WebFetch-verified twice for consistency, mirroring "-ig"'s bar.
  Deliberately excludes "snug", "plug", "slug", and "shrug" (four- and five-letter
  consonant-blend words) to preserve the strict three-letter CVC scope, the same discipline every
  prior family has used. No new manifest objective needed — extends the same already-covered
  library `adj.literacy.k2.rhyming_word_families`. New e2e test
  `rhymes_with_isolates_a_fourth_family_and_abstains_on_excluded_blend_words` (6th test in
  `facts_wordfamilies_e2e.rs`), which asserts NO cross-contamination with any of the three prior
  families AND honest abstention on "snug".
- `earth-science/season-start-month-number.adj` (new) — the FIRST CROSS-DIRECTORY `rule`
  composition in the ADJ stdlib's science domain (prior `rule` compositions -- word-families,
  heat-causes-phase-change, force-causes-acceleration -- all stayed within one subject directory).
  Bridges the already-shipped `season_start_month` table (`earth-science/seasons.adj`) with the
  already-shipped `month_number` table (`calendar/months.adj`) to DERIVE
  `season_start_month_number(season, number)` -- the exact bridge `seasons.adj`'s own header
  comment already invited ("the concrete bridge from RECALL to COMPUTE"). Reuses TWO
  already-verified citations (NOAA meteorological-seasons, ISO 8601 month numbering) with zero new
  sourcing work. Grounds NGSS 1-ESS1-2. This file lives in `earth-science/` (its natural home) and
  reaches its calendar sibling via a relative `../calendar/months.adj` import -- empirically
  confirmed (by reading `adj-lang-cli`'s `FsProvider` sandbox-root source directly, not just
  guessing from the error message) that this resolves cleanly because the CLI's import sandbox is
  rooted at the TOP-LEVEL PROGRAM's directory, not each importer's own directory -- so the
  companion `season-start-month-number.query.adj` is placed at the package ROOT, mirroring
  `mathematics/word-problems.query.adj`'s already-established cross-directory-import pattern. New
  manifest objective `adj.science.k2.season_start_month_number` (band K-2, `infer` competency).
  New e2e test `facts_seasonmonthnumber_e2e.rs` (3 tests: direct derivation with dual citations,
  reverse binding, honest abstention on an untabled season).
- `astronomy/planet-ordinal-position.adj` (new) — the SECOND cross-directory `rule` composition
  in the ADJ stdlib's science domain, following `earth-science/season-start-month-number.adj`'s
  precedent. Bridges the already-shipped `planet_order` table (`astronomy/planets.adj`) with the
  already-shipped `ordinal_number` table (`mathematics/ordinal-numbers.adj`) to DERIVE
  `planet_ordinal_position(planet, ordinal)` -- grounding the common early-elementary framing
  "Earth is the THIRD planet from the Sun." Reuses TWO already-verified citations (NASA planet
  order, standard English ordinal-number convention) with zero new sourcing work. Honest
  abstention on Pluto (reclassified a dwarf planet in 2006, deliberately not a row in
  `planets.adj`) -- the rule abstains rather than inventing a position. Same cross-directory
  pattern as `season-start-month-number.adj`: the library lives in `astronomy/` (its natural
  home) and imports its mathematics sibling via a relative `../mathematics/ordinal-numbers.adj`
  path; its `.query.adj` companion is placed at the package root so the CLI's import sandbox
  (rooted at the top-level program's own directory) resolves the `../` hop. New manifest
  objective `adj.science.k2.planet_ordinal_position` (band K-2, `infer` competency). New e2e test
  `facts_planetordinalposition_e2e.rs` (3 tests: direct derivation with dual citations, reverse
  binding, honest abstention on Pluto).
- `astronomy/moon-phase-ordinal-position.adj` (new) — the THIRD cross-directory `rule`
  composition in the ADJ stdlib's science domain, and the SECOND time the exact number-to-
  ordinal-word bridge pattern (first used in `astronomy/planet-ordinal-position.adj`) has been
  applied — this time to a DIFFERENT already-shipped table in the same `astronomy/` directory.
  Bridges the already-shipped `moon_phase_order` table (`astronomy/moon-phases.adj`) with the
  already-shipped `ordinal_number` table (`mathematics/ordinal-numbers.adj`) to DERIVE
  `moon_phase_ordinal_position(phase, ordinal)` -- grounding "the full Moon is the FIFTH phase in
  the cycle." Reuses TWO already-verified citations (NASA Moon phases, standard English ordinal-
  number convention) with zero new sourcing work. Honest abstention on "eclipse" (a different
  astronomical event, deliberately not a row in `moon-phases.adj`). Same cross-directory pattern
  as `planet-ordinal-position.adj`/`season-start-month-number.adj`: the library lives in
  `astronomy/` (its natural home) and imports its mathematics sibling via a relative
  `../mathematics/ordinal-numbers.adj` path; its `.query.adj` companion is placed at the package
  root. New manifest objective `adj.science.k2.moon_phase_ordinal_position` (band K-2, `infer`
  competency). New e2e test `facts_moonphaseordinalposition_e2e.rs` (3 tests: direct derivation
  with dual citations, reverse binding, honest abstention on "eclipse").
- `language/word-families.adj` — extended with a FIFTH word family, "-og" (dog, hog, fog, log,
  jog), added as five new rows in the existing `word_family` table alongside "-an", "-at", "-ig",
  and "-ug". The existing `rhymes_with` rule is reused UNCHANGED for the fifth time running.
  Quoted verbatim from a THIRD Super Teacher Worksheets page (same site, same `consensus` trust
  tier, its own real citation): "Here is a collection of printable activities for young readers
  to learn about the 'og' family of words... Words included: clog, jog, dog, hog, frog, fog, and
  log." — WebFetch-verified twice for consistency, mirroring "-ig"/"-ug"'s bar. Deliberately
  excludes "clog" and "frog" (four-letter consonant-blend words) to preserve the strict
  three-letter CVC scope, the same discipline every prior family has used. IMPORTANT: this slice
  also fixed the library's own abstention worked-example and test, which had used "dog" as the
  "unshipped word" case since slice 1 -- now that "dog" is itself a real `-og` member, both
  `word-families.query.adj` and `facts_wordfamilies_e2e.rs`'s abstention case were switched to
  "cup" (still genuinely untabled). No new manifest objective needed -- extends the same
  already-covered library `adj.literacy.k2.rhyming_word_families`. New e2e test
  `rhymes_with_isolates_a_fifth_family_and_abstains_on_excluded_blend_words` (7th test in
  `facts_wordfamilies_e2e.rs`), which asserts NO cross-contamination with any of the four prior
  families AND honest abstention on "frog".
- `language/syllable-count.adj` (new) — the SECOND literacy sub-skill library in the ADJ stdlib,
  deliberately DIFFERENT in shape from `word-families.adj`'s rhyme-family derivation: a genuinely
  new phonological-awareness skill (syllable segmentation, CCSS RF.K.2.b) rather than another word
  family (RF.K.2.a). A new `syllable_count(word, count)` table names how many syllables each of
  four words has, quoted verbatim from Reading Rockets' "Phonological and Phonemic Awareness: In
  Practice" module, which demonstrates syllable segmentation as a classroom technique (one index
  card placed per syllable while the teacher says each part aloud): peanut, pencil, sunset, and
  laptop, all explicitly segmented on the page. WebFetch-verified TWICE for consistency before
  writing -- the first pass over-eagerly attributed a syllable count to "classroom" that the
  second, more careful pass found was NOT actually syllable-segmented on the page (just used in an
  unrelated sentence), so it was correctly dropped. All four confirmed words happen to be
  two-syllable in this cited source (the page's demonstration does not segment a one- or
  three-syllable word), so the table is honestly narrow rather than fabricating contrast; a future
  slice can add variety once a comparably clean citation for a different count is found. Grounds
  CCSS RF.K.2.b. New manifest objective `adj.literacy.k2.syllable_count` (band K-2, `recall`
  competency -- a pure lookup, not a `rule`-derived fact, since no composition was needed or
  available here). New e2e test `facts_syllablecount_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an unshipped word).
- `biology/mitosis-phase-order.adj` (new) + `biology/mitosis-phase-ordinal-position.adj` (new) --
  the FIRST biology-domain entry in the ordinal-bridge composition pattern
  `earth-science/season-start-month-number.adj`, `astronomy/planet-ordinal-position.adj`, and
  `astronomy/moon-phase-ordinal-position.adj` already established, and the FOURTH cross-directory
  `rule` composition overall. `mitosis-phases.adj` (already shipped) tables each phase's defining
  event but only encodes cycle ORDER as row order, not a queryable number, so
  `mitosis-phase-order.adj` makes that same source's ordering ("The four phases of mitosis are
  Prophase ... Metaphase ... Anaphase ... Telophase", the SAME NCI SEER sentence
  `mitosis-phases.adj` already cites -- zero new sourcing risk) a first-class
  `mitosis_phase_order(phase, order)` fact, mirroring `astronomy/moon-phases.adj`'s own
  `moon_phase_order` column. `mitosis-phase-ordinal-position.adj` then bridges that new fact to
  the already-shipped `mathematics/ordinal-numbers.adj` exactly as the astronomy ordinal bridges
  do, deriving `mitosis_phase_ordinal_position($Phase, $Ordinal)` (e.g. "anaphase" -> "third").
  Honest abstention on `interphase` (the resting phase BETWEEN divisions, deliberately excluded
  from both tables, mirroring `mitosis-phases.adj`'s own exclusion). New manifest objective
  `adj.science.6to8.mitosis_phase_ordinal_position` (band 6-8, matching where NGSS places cell
  division, vs. the K-2 band of the three prior astronomy/earth-science ordinal bridges). New
  e2e test `facts_mitosisphaseordinalposition_e2e.rs` (3 tests: direct derivation with dual
  citations, reverse binding, honest abstention on `interphase`).
- `language/initial-sound.adj` (new) -- the THIRD literacy sub-skill library in the ADJ stdlib,
  deliberately different in shape from both prior ones: `word-families.adj` derives RHYMING
  (RF.K.2.a, shared END sound) via a `rule`, `syllable-count.adj` recalls a SYLLABLE COUNT
  (RF.K.2.b) -- this one recalls a word's BEGINNING sound (phoneme identity/isolation, RF.K.2.d)
  as a pure lookup, `initial_sound(word, sound)`. Quoted verbatim from Reading Rockets' "Reading
  101 for Parents: Phonological and Phonemic Awareness" guide, WebFetch-verified TWICE for
  consistency before writing: "Bell, bike, and boy all have /b/ at the beginning." -- the site's
  own canonical phoneme-identity example (confirmed appearing word-for-word on more than one
  Reading Rockets page). Deliberately scoped to ONLY the three words and one sound (/b/) this
  single cited sentence names -- all three happen to share one phoneme, so the table is honestly
  narrow (mirroring `syllable-count.adj`'s all-2-syllable table) rather than fabricating a second
  sound group from an uncited word list. Grounds CCSS RF.K.2.d. New manifest objective
  `adj.literacy.k2.initial_sound` (band K-2, `recall` competency). New e2e test
  `facts_initialsound_e2e.rs` (3 tests: direct recall, reverse binding across all three words
  sharing /b/, honest abstention on an unshipped word).
- `chemistry/measuring-tools.adj` (new) -- a genuinely new "observation and measurement" axis
  (ADJ-STDLIB-COVERAGE.md 5.1's named Major Gap for K-8 science), distinct from the sibling
  `lab-equipment.adj`'s tool->purpose-verb table. A new `measuring_tool(tool, quantity)` table
  names which ONE quantity each of four common lab tools measures (ruler->length,
  graduated_cylinder->volume, balance->mass, thermometer->temperature), quoted verbatim from a
  Chemistry LibreTexts introductory lab manual, "Introducing Measurements in the Laboratory",
  whose four-part lab exercise each opens with a sentence naming the tool and the quantity/unit
  it measures. WebFetch-verified TWICE for consistency before writing. Deliberately NOT a 5th
  ordinal-bridge instance -- the science lane's four prior slices (season/planet/moon-phase/
  mitosis) already saturate that pattern; this slice diversifies into a different axis
  (observation/measurement) entirely, after a survey of chemistry reaction-types.adj/gas-laws.adj
  and earth-science rock-types.adj found no clean, uninvented causal pairing available without
  fabricating an unstated link. Grounds the NGSS science-practice observation/measurement gap.
  New manifest objective `adj.science.3to5.measuring_tools` (band 3-5, `recall` competency -- a
  pure lookup, not a `rule`-derived fact). New e2e test `facts_measuringtools_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention on an unshipped tool).
- `language/onset-rime.adj` (new) -- the FOURTH literacy sub-skill library in the ADJ stdlib,
  deliberately different in shape from the three prior ones: `word-families.adj` derives RHYMING
  (RF.K.2.a), `syllable-count.adj` recalls a SYLLABLE COUNT (RF.K.2.b), and `initial-sound.adj`
  recalls a BEGINNING sound (RF.K.2.d) -- this one recalls how a single-syllable word splits into
  its ONSET (sound(s) before the vowel) and RIME (the vowel and everything after) as a pure
  lookup, `onset_rime(word, onset, rime)`, a THREE-column table (the shape
  `metrology/si-base-units.adj` already established). Quoted verbatim from Reading Rockets'
  "Tuning In to the Sounds in Words" article, WebFetch-verified TWICE for consistency before
  writing: "sleep could be broken into /sl/ and /eep/" and "Here are two ways to break up the
  word blast: Onset (bl) – Rime (ast)". Deliberately scoped to ONLY these two words the cited
  page splits explicitly -- honestly narrow (mirroring `syllable-count.adj`'s and
  `initial-sound.adj`'s precedent) rather than inventing a split for an uncited word. Grounds
  CCSS RF.K.2.c. New manifest objective `adj.literacy.k2.onset_rime` (band K-2, `recall`
  competency). New e2e test `facts_onsetrime_e2e.rs` (3 tests: direct segmenting recall, reverse
  blending recall, honest abstention on an unshipped word).
- `language/phoneme-substitution.adj` (new) -- the FIFTH literacy sub-skill library in the ADJ
  stdlib, completing coverage of all five named parts of CCSS RF.K.2. Deliberately different in
  shape from all four prior ones (rhyme derivation RF.K.2.a, syllable count RF.K.2.b, onset/rime
  RF.K.2.c, initial sound RF.K.2.d) -- this recalls what happens when you SUBSTITUTE one sound in
  a word for another, grounding RF.K.2.e ("Add or substitute individual sounds in simple,
  one-syllable words to make new words") as a pure lookup,
  `phoneme_substitution(original_word, original_sound, new_sound, new_word)`, a FOUR-column
  table. Quoted verbatim from Reading Rockets' "Phonological and Phonemic Awareness: In Practice"
  module (the SAME page `syllable-count.adj` already cites, a different section), WebFetch-
  verified TWICE for consistency before writing: "I can change one sound in a word to form a new
  word. Watch me. I will change 'make' to 'bake'." and "The first sound in make is /m/. The first
  sound in bake is /b/." Deliberately scoped to ONLY this ONE substitution the cited page walks
  through step by step -- honestly narrow (mirroring `onset-rime.adj`'s and `initial-sound.adj`'s
  precedent) rather than inventing a substitution the source does not demonstrate. New manifest
  objective `adj.literacy.k2.phoneme_substitution` (band K-2, `recall` competency). New e2e test
  `facts_phonemesubstitution_e2e.rs` (3 tests: direct recall of the new word, reverse binding of
  the original word/sound, honest abstention on an untabled substitution).
- `meteorology/weather-instruments.adj` (new) -- a DIFFERENT "observation and measurement" axis
  from the already-shipped `chemistry/measuring-tools.adj` (lab tools) -- this one covers
  weather-OBSERVING instruments. A new `weather_instrument(instrument, quantity)` table names
  which ONE quantity each of six instruments measures (anemometer->wind_speed,
  weather_vane->wind_direction, barometer->atmospheric_pressure,
  thermometer->air_temperature, hygrometer->humidity, rain_gauge->rainfall), quoted verbatim
  from NOAA's "Build Your Own Weather Station" education page, whose six section headings each
  name one instrument and the quantity it measures. WebFetch-verified TWICE for consistency
  before writing. `trust authoritative` -- a primary NOAA (.gov) source, matching the sibling
  `precipitation-types.adj`/`wind-scale.adj` NOAA sources' tier. Continues diversifying the
  science lane after a fresh survey of biology food-chain-roles.adj/animal-diets.adj and
  meteorology precipitation-types.adj/wind-scale.adj again found no clean, uninvented causal
  pairing (the food-chain-role/diet-category vocabularies don't share a key without asserting an
  unstated "herbivore IS a consumer" link). Grounds the NGSS science-practice observation/
  measurement gap. New manifest objective `adj.science.3to5.weather_instruments` (band 3-5,
  `recall` competency). New e2e test `facts_weatherinstruments_e2e.rs` (3 tests: direct recall,
  reverse binding, honest abstention on a non-weather instrument).
- `biology/monarch-life-cycle.adj` (new) -- a genuinely NEW content shape for this loop's science
  sweep, neither an instrument-measures-quantity table (like `chemistry/measuring-tools.adj`) nor
  an ordinal-WORD bridge (like the four already-shipped season/planet/moon-phase/mitosis ordinal-
  position libraries) -- a plain numbered life-cycle-stage recall table, applying the SAME shape
  `earth-science/water-cycle.adj` already established for a physical-process cycle to a
  BIOLOGICAL one. A new `monarch_life_stage(stage, order)` table names the position of each of
  the monarch butterfly's four life stages (egg->1, larva->2, pupa->3, adult->4), quoted verbatim
  from the USDA Forest Service's "Monarch Butterfly Biology" page, WebFetch-verified TWICE for
  consistency before writing: "The monarch has four distinct life stages: egg, larva
  (caterpillar), pupa (chrysalis), and adult." `trust authoritative` -- a primary U.S. government
  (USDA, .gov) source. Honest abstention on "nymph" (the incomplete-metamorphosis term, e.g. a
  grasshopper -- not one of the monarch's complete-metamorphosis stages). Grounds NGSS 3-LS1-1
  ("Develop models to describe that organisms have unique and diverse life cycles"). New manifest
  objective `adj.science.3to5.monarch_life_cycle` (band 3-5, `recall` competency). New e2e test
  `facts_monarchlifecycle_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on
  "nymph").
- `language/compound-word-spelling-example.adj` (new) -- the SIXTH literacy sub-skill library,
  and the FIRST to move beyond CCSS RF.K.2 (all five named parts of which -- rhyming, syllable
  count, onset/rime, initial sound, phoneme substitution -- are now shipped). Grounds a SPELLING
  pattern instead of a phonological-awareness one: teaching a beginner to spell multisyllable
  words is easier when the word is a compound built from two words the learner can already
  spell. A new `compound_word_spelling_example(word, teaching_use)` table names the four
  compound words a primary source uses to teach this (catfish, hotdog, playground, yellowtail),
  quoted verbatim from Reading Rockets' "How Spelling Supports Reading" article, WebFetch-
  verified TWICE for consistency before writing. `trust consensus`, the same tier as the other
  Reading Rockets citations already shipped in this directory. Deliberately gives the table a
  genuine second column (`teaching_use`, a constant label for every row) rather than a bare
  `columns word` -- an earlier draft with a single-column table was empirically verified in a
  scratch dir to NOT produce ordinary `recall`/`abstained` query semantics on a fully-ground
  query (the engine instead falls back to its hypothesis-ranking/adjudication mode), so every
  table in this stdlib should keep at least two genuine columns even when the second is a
  constant. Deliberately does NOT cite a specific CCSS standard code: the closest candidates
  (RF.1.3.e general phonics, L.2.4.d compound-word MEANING prediction) both describe a different
  skill than what this source supports (spelling ease via compound decomposition, not decoding
  or meaning), so `standards` stays empty rather than force-citing a mismatched code. Honest
  abstention on "cupcake" (a real compound word, but not one this source names). New manifest
  objective `adj.literacy.k2.compound_word_spelling_example` (band K-2, `recall` competency).
  New e2e test `facts_compoundwordspellingexample_e2e.rs` (3 tests: direct recall, reverse
  binding of all four example words, honest abstention on an uncited compound).
- `oceanography/ocean-observing-instruments.adj` (new) -- a THIRD "observation and measurement"
  axis for the science domain, after `chemistry/measuring-tools.adj` (lab tools) and
  `meteorology/weather-instruments.adj` (weather-observing instruments) -- this one covers
  OCEAN-observing instruments. A new `ocean_instrument(instrument, quantity)` table names which
  ONE quantity each of three instruments measures or detects (tide_gauge -> sea_level,
  hydrophone -> underwater_sound, sonar -> distance_to_object), quoted verbatim from three
  DIFFERENT NOAA oceanservice.noaa.gov "facts" pages, WebFetch-verified before writing.
  `trust authoritative` -- a primary NOAA (.gov) source, the same tier `weather-instruments.adj`'s
  source earned. UNLIKE `weather-instruments.adj` (six rows sharing ONE source page), this
  table's three rows each cite a DIFFERENT page -- since the ADJ table grammar carries only one
  table-level `source`/`locator`/`trust` block (confirmed by reading `weather-instruments.adj`
  and `word-families.adj` before writing), the table's own citation is the primary/first-listed
  source (tide-gauge.html) and each other row's own distinct citation is documented in the
  file's header prose, the same discipline `word-families.adj`'s multi-family extensions
  established. Deliberately excludes a CTD (which measures MULTIPLE quantities at once --
  conductivity, temperature, and depth -- not one, so it does not fit this table's
  one-instrument-one-quantity shape) and a buoy/ocean glider (both 404'd on
  oceanservice.noaa.gov this session, no citable page found). New manifest objective
  `adj.science.3to5.ocean_instruments` (band 3-5, `recall` competency, matching
  measuring-tools.adj's and weather-instruments.adj's band). New e2e test
  `facts_oceaninstruments_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on
  a CTD).
- `language/silent-e-word.adj` (new) -- the SEVENTH literacy sub-skill library, and the SECOND
  to move beyond CCSS RF.K.2, following `compound-word-spelling-example.adj`'s precedent into
  ANOTHER spelling pattern: the "silent e" / "magic e" (VCe) syllable type -- a long vowel
  spelled with a single letter, followed by a single consonant, followed by a silent "e". A new
  `silent_e_word(word, syllable_type)` table names the seven example words a primary source
  uses to teach this (wake, whale, while, yoke, yore, rude, hare), quoted verbatim from Reading
  Rockets' "Six Syllable Types" article, WebFetch-verified TWICE for consistency before writing.
  `trust consensus`, the same tier as the other Reading Rockets citations already shipped in
  this directory. Deliberately does NOT populate the manifest's `standards` field: CCSS RF.1.3.c
  ("Know final -e and common vowel team conventions for representing long vowel sounds") is a
  genuinely clean fit for this pattern (confirmed via two independent sources), but every
  objective in this stdlib so far describes its grounding standard in CHANGELOG/README prose
  rather than the manifest's `standards` array, so this library follows that same established
  convention rather than unilaterally breaking it. Honest abstention on "snake" (a real VCe
  word, but not one this source names). New manifest objective
  `adj.literacy.k2.silent_e_word` (band K-2, `recall` competency). New e2e test
  `facts_silentEword_e2e.rs` (3 tests: direct recall, reverse binding of all seven example
  words, honest abstention on an uncited VCe word).
- `language/r-controlled-vowel-word.adj` (new) -- the EIGHTH literacy sub-skill library, and the
  THIRD to move beyond CCSS RF.K.2, following `compound-word-spelling-example.adj`'s and
  `silent-e-word.adj`'s precedent into a phonics pattern: "r-controlled vowels" (aka "bossy r"),
  where a vowel followed by "r" no longer makes its expected sound. A new
  `r_controlled_vowel_word(word, pattern)` table names five example words and the r-controlled
  digraph in each (barn -> ar, corn -> or, fern -> er, bird -> ir, curl -> ur), quoted verbatim
  from the University of Florida Literacy Institute (UFLI)'s phonics foundations toolbox: "There
  are three main r-controlled vowel sounds: the /ar/ sound, as in barn; the /or/ sound, as in
  corn; and the /er/ sound, as in fern, bird, and curl." WebFetch-verified TWICE for consistency
  before writing (two independent fetches of the same page). `trust authoritative` -- UFLI is a
  university literacy research center (University of Florida, .edu), a primary academic source.
  DESIGN NOTE: the source groups fern/bird/curl under ONE phonetic label ("/er/ sound") despite
  three different spellings (er/ir/ur) -- `pattern` here is the LITERAL r-controlled digraph
  objectively present in each word's own spelling, NOT an assertion that the source itself
  distinguished er/ir/ur as separate categories (it did not), the same discipline
  `word-families.adj`'s `family` column already established for naming letters-in-the-word
  rather than a source-stated grouping. Honest abstention on "star" (a real ar-pattern word, but
  not one this source names). New manifest objective `adj.literacy.k2.r_controlled_vowel_word`
  (band K-2, `recall` competency). New e2e test `facts_rcontrolledvowelword_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention on an uncited word).
- `language/fable-moral.adj` (new) -- the NINTH literacy sub-skill library, and the FIRST to
  ground a whole-TEXT comprehension artifact rather than a word-level phonics/spelling fact: a
  classic fable's own narrator-stated moral. A new `fable_moral(fable, moral)` table names three
  fables and their own stated lessons (tortoise_and_the_hare -> "Slow but steady wins the
  race.", shepherds_boy_and_the_wolf -> "There is no believing a liar, even when he speaks the
  truth.", boy_and_the_filberts -> "Do not attempt too much at once."), quoted verbatim from
  George Fyler Townsend's classic English translation of Aesop's Fables, hosted by Project
  Gutenberg -- a legitimate public-domain literary primary source. `trust authoritative`.
  RESEARCH DISCIPLINE: of SIX candidate fables originally surveyed on the same page, only these
  THREE give a clean, unambiguous, narrator-voice closing moral, verified by reading the raw
  page text directly. The other three -- "The Ants and the Grasshopper", "The Fox and the Crow",
  and "The Lion and the Mouse" -- were deliberately EXCLUDED after verification found their
  closing line is a character's spoken dialogue (the ants' taunt, the fox's gloat, the mouse's
  own words), not the narrator's own stated moral; asserting those as "the fable's moral" the
  same way the three shipped rows are stated would overclaim what the source actually does.
  GRAMMAR DISCOVERY: the ADJ query grammar accepts a quoted-string literal as a `table` row
  VALUE, but NOT as a query argument -- a query can only ground an atom/number or bind a $Var,
  so "which fable has moral X" is answered by enumerating with `? fable_moral($F, $Moral)` and
  reading off the match, not by querying with the moral string itself as a ground argument (a
  new finding for this stdlib, documented in the file's own header for future sentence-valued
  tables). Honest abstention on "the_fox_and_the_crow". New manifest objective
  `adj.literacy.k2.fable_moral` (band K-2, `recall` competency). New e2e test
  `facts_fablemoral_e2e.rs` (3 tests: direct recall, reverse binding of all three fables, honest
  abstention on a fable whose closing line is dialogue).
- `language/vocabulary-in-context.adj` (new) -- the TENTH literacy sub-skill library. A new
  `vocabulary_in_context(word, meaning)` table names three vocabulary words whose meaning a
  primary source teaches via a worked context-clue example sentence (ornithology ->
  scientific_study_of_birds, sentence: "People who study birds are experts in ornithology.";
  frugivorous -> eats_fruit_as_primary_food, sentence: "Frugivorous birds prefer eating fruit to
  any other kind of food."; inconspicuous -> hidden_or_not_easily_seen, sentence: "Some birds
  like to build their nests in inconspicuous spots -- high up in the tops of trees, well hidden
  by leaves."), quoted verbatim from Reading Rockets' "Using Context Clues to Understand Word
  Meanings" article, `trust consensus` -- the same tier as the other Reading Rockets citations
  already shipped in this directory. DESIGN NOTE: `meaning` is a short constant-style label
  rather than a full-sentence definition -- `fable-moral.adj`'s grammar discovery found that a
  quoted-string literal works as a `table` row VALUE but not as a query ARGUMENT, so using a
  short atom here (unlike `fable-moral.adj`'s sentence-valued `moral` column) keeps BOTH the
  direct and reverse queries usable as ordinary ground-argument binding queries. Honest
  abstention on "arboreal" (a real vocabulary word, but not one this source defines). New
  manifest objective `adj.literacy.k2.vocabulary_in_context` (band K-2, `recall` competency).
  New e2e test `facts_vocabularyincontext_e2e.rs` (3 tests: direct recall, reverse binding,
  honest abstention on an undefined word).
- `meteorology/cloud-type.adj` (new) -- the ELEVENTH science slice, and a genuinely new
  "observation and measurement" axis from the already-shipped `weather-instruments.adj` and
  `ocean-observing-instruments.adj` (instrument -> quantity measured): this table names a cloud
  TYPE and the weather it indicates, not an instrument at all. A new `cloud_type(cloud,
  weather_indication)` table names three cloud types (cirrus -> approaching_warm_front,
  cumulonimbus -> heavy_rain_thunderstorm, stratus -> light_rain_drizzle_or_none), quoted
  verbatim from the National Weather Service's (Louisville forecast office) "Cloud
  Classification" education page, `trust authoritative`. WebFetch-verified before writing (note:
  the related jetstream.noaa.gov domain 403s WebFetch entirely -- weather.gov was used instead,
  per this stdlib's established workaround). Honest abstention on "altocumulus" (a real cloud
  type, but not one this source classifies by weather indication). New manifest objective
  `adj.science.3to5.cloud_type` (band 3-5, `recall` competency, `ngss` coverage root). New e2e
  test `facts_cloudtype_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on
  an untabled cloud).
- `geology/rock-type.adj` (new) -- the TWELFTH science slice, and a new file in the ALREADY-
  SHIPPED `geology/` directory (alongside `earth-layers.adj` and `mineral-hardness.adj`). A new
  `rock_type(rock, formation_process)` table names the three basic classes geologists sort ALL
  rocks into and HOW each class forms (igneous -> crystallized_molten_rock, sedimentary ->
  deposited_weathered_material, metamorphic -> heat_and_pressure_transformation). UNLIKE
  `earth-layers.adj` (four rows sharing one USGS publication), this table's three rows each cite
  a DIFFERENT USGS FAQ page ("What are igneous/sedimentary/metamorphic rocks?"), so it uses the
  multi-source pattern `ocean-observing-instruments.adj`/`fable-moral.adj` established: the
  table-level citation carries the primary (igneous) source, and the other two rows' own
  distinct citations are documented in the file's header prose. All three quotes WebFetch-
  verified before writing. `trust authoritative` -- every row's own source page is a primary
  U.S. government (USGS, .gov) source. Honest abstention on "coal" (a real rock, but not one of
  the three rock-type classes tabled here). New manifest objective `adj.science.3to5.rock_type`
  (band 3-5, `recall` competency, `ngss` coverage root). New e2e test `facts_rocktype_e2e.rs`
  (3 tests: direct recall, reverse binding, honest abstention on an untabled rock).
- `language/past-tense-ed-sound.adj` (new) -- the ELEVENTH literacy sub-skill library, and a
  genuinely new phonics pattern beyond CCSS RF.K.2's five parts and the spelling/whole-text/
  vocabulary slices already shipped: the regular -ed past-tense suffix is spelled the same way
  every time, but PRONOUNCED one of three different ways depending on the final sound of the
  base verb. A new `past_tense_ed_sound(word, sound)` table names three worked examples (walked
  -> t_sound, lived -> d_sound, wanted -> id_sound), quoted verbatim from 7ESL's "Pronunciation
  of ED: Past Tense Pronunciation for Regular Verbs" article, `trust consensus` -- a general
  ESL-learning site, the same tier this stdlib already reserves for its other non-.gov language
  sources (Wikipedia's Greek-alphabet/Morse-code entries). WebFetch-verified before writing.
  Honest abstention on "played" (also /d/-sounded, but not one of the three tabled example
  words). New manifest objective `adj.literacy.k2.past_tense_ed_sound` (band K-2, `recall`
  competency, `ccss.ela` coverage root). New e2e test `facts_pasttenseedsound_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention on an untabled word).
- `language/plural-s-sound.adj` (new) -- the TWELFTH literacy sub-skill library, a sibling
  phonics pattern to `past-tense-ed-sound.adj`: the regular plural -s/-es suffix is pronounced
  one of three different ways depending on the final sound of the singular noun. A new
  `plural_s_sound(word, sound)` table names three worked examples (hats -> s_sound, dogs ->
  z_sound, boxes -> iz_sound), quoted verbatim from Speakspeak's "Pronunciation of 's' and 'es'
  plural endings" article, `trust consensus` -- the same tier as `past-tense-ed-sound.adj`'s
  7ESL citation. WebFetch-verified before writing. Honest abstention on "cats" (also
  /s/-sounded, but not one of the three tabled example words). New manifest objective
  `adj.literacy.k2.plural_s_sound` (band K-2, `recall` competency, `ccss.ela` coverage root).
  New e2e test `facts_pluralssound_e2e.rs` (3 tests: direct recall, reverse binding, honest
  abstention on an untabled word). NOTE: this slice replaced a dropped "science 13th slice:
  water cycle stages" candidate after discovering `earth-science/water-cycle.adj` already
  tables `water_cycle_stage(stage, step_number)` covering the same ground -- see this
  directory's `README.md` for a fuller account of the duplication discovered this cycle
  (also `physics/simple-machines.adj` vs. a dropped teachengineering.org candidate, and
  `earth-science/rock-types.adj` vs. the already-merged `geology/rock-type.adj`).
- `biology/rainforest-layer.adj` (new) -- a science slice picked using the new mandatory
  full-tree-grep-before-scoping discipline (see the entry above): `grep -ril "rainforest"
  code/specs/data/adj-facts-stdlib/` confirmed ZERO existing coverage before this file was
  written, unlike moon phases and food chain roles, both confirmed already covered elsewhere in
  the stdlib during the same research pass. A new `rainforest_layer(layer, description)` table
  names the four rainforest layers top to bottom and a one-fact description of each (emergent ->
  tallest_trees_dominate_skyline, canopy -> deep_treetop_vegetation_layer, understory ->
  dark_humid_layer_below_canopy, forest_floor -> darkest_layer_hard_for_plants_to_grow), quoted
  verbatim from National Geographic Education's "Rain Forest" entry, `trust consensus` -- a
  reputable education organization, not primary government, the same tier this stdlib already
  reserves for its other non-.gov sources. WebFetch-verified before writing (fetched twice, once
  for the overall page and once specifically to confirm the emergent layer's tree-height
  sentence verbatim). Honest abstention on "soil_layer" (not one of the four named layers). New
  manifest objective `adj.science.3to5.rainforest_layer` (band 3-5, `recall` competency, `ngss`
  coverage root). New e2e test `facts_rainforestlayer_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an untabled layer).
- `language/idiom-meaning.adj` (new) -- the THIRTEENTH literacy sub-skill library, and a
  genuinely new figurative-language skill beyond CCSS RF.K.2's five parts and the
  phonics/spelling/whole-text/vocabulary slices already shipped: an idiom's literal words do NOT
  give its meaning. Picked using the mandatory full-tree-grep-before-scoping discipline --
  `grep -ril "idiom\|proverb" code/specs/data/adj-facts-stdlib/` confirmed ZERO existing coverage
  before this file was written. A new `idiom_meaning(idiom, meaning)` table names three common
  idioms and their meanings (piece_of_cake -> very_easy_to_do, break_the_ice ->
  start_a_conversation, under_the_weather -> feeling_slightly_ill), quoted verbatim from Oxford
  International English's "30 Useful English Idiomatic Expressions & Their Meanings" article,
  `trust consensus` -- the same tier as this stdlib's other non-.gov language sources (7ESL,
  Speakspeak). WebFetch-verified before writing. Honest abstention on
  "raining_cats_and_dogs" (a real, well-known idiom, but not one of these three tabled example
  idioms). New manifest objective `adj.literacy.3to5.idiom_meaning` (band 3-5 -- idioms are
  typically a CCSS L.3.5.b, grade 3+ skill, unlike most of this stdlib's other K-2 literacy
  slices -- `recall` competency, `ccss.ela` coverage root). New e2e test
  `facts_idiommeaning_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on an
  untabled idiom).
- `language/synonyms.adj` (new) -- a sibling library to the already-shipped `opposites.adj`
  (antonyms): a new `synonym(word, synonym)` table names three common words and a synonym of
  each (happy -> cheerful, smart -> bright, quick -> fast), quoted verbatim from the English
  Wiktionary entry for each word's own "Synonyms" line -- the SAME source family and `trust
  consensus` tier `opposites.adj` already established for antonyms. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -ril "synonym"
  code/specs/data/adj-facts-stdlib/` confirmed ZERO existing coverage before this file was
  written (an "antonyms" candidate was considered first and DROPPED once `opposites.adj` was
  discovered to already cover that ground). WebFetch-verified before writing. Only one direction
  is shipped per pair, mirroring `opposites.adj`'s own established convention. Honest abstention
  on "purple" (a real word, but with no shipped synonym in this table -- the same abstention
  example `opposites.adj` already uses). New manifest objective `adj.literacy.k2.synonym_pair`
  (band K-2, `recall` competency, `ccss.ela` coverage root). New e2e test `facts_synonyms_e2e.rs`
  (3 tests: direct recall, reverse binding, honest abstention on an untabled word).
- `biology/animal-habitat.adj` (new) -- a sibling library to the already-shipped
  `animal-homes.adj`, but a DIFFERENT axis: not the built STRUCTURE an animal lives in (a hive,
  a nest, a burrow), but the broad BIOME/environment type an animal calls home. A new
  `animal_habitat(animal, biome)` table names three animals and the biome each lives in
  (polar_bear -> arctic, bactrian_camel -> desert, giraffe -> grassland), quoted verbatim from
  National Geographic (`kids.nationalgeographic.com` for the polar bear and Bactrian camel fact
  pages, `education.nationalgeographic.org` for the giraffe/grassland sentence) -- the same
  source family and `trust consensus` tier this stdlib already reserves for
  `rainforest-layer.adj`'s National Geographic Education citation. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -ril "habitat\|biome"
  code/specs/data/adj-facts-stdlib/` found only two incidental prose mentions of the word
  "habitat" (in `fungus-parts.adj` and `rainforest-layer.adj`), neither tabling an animal-biome
  relation, and `animal-homes.adj` was re-read in full to confirm its five rows are all built
  structures (bee/bird/spider/rabbit/beaver), a genuinely disjoint column semantic and animal
  set from this table -- CONFIRMED distinct, not a duplicate. WebFetch-verified before writing.
  Honest abstention on "dog" (a real animal, but with no shipped habitat in this table). New
  manifest objective `adj.science.k2.animal_habitat` (band K-2, `recall` competency, `ngss`
  coverage root, mirroring `adj.science.k2.heat_causes_phase_change`'s band/coverage-root
  convention for K-2 science). New e2e test `facts_animalhabitat_e2e.rs` (3 tests: direct
  recall, reverse binding, honest abstention on an untabled animal).
- `language/homophones.adj` (new) -- a sibling library to the already-shipped `opposites.adj`
  (antonyms) and `synonyms.adj`: a new `homophone(word, sound_alike)` table names three common
  words and a word that sounds the same but is spelled/means differently (there -> their,
  flower -> flour, to -> too), quoted verbatim from the English Wiktionary entry for each word's
  own "Homophones" line -- the SAME source family and `trust consensus` tier `opposites.adj`/
  `synonyms.adj` already established. Picked using the mandatory full-tree-grep-before-scoping
  discipline -- `grep -ril "homophone\|homonym" code/specs/data/adj-facts-stdlib/` confirmed
  ZERO existing coverage before this file was written. WebFetch-verified before writing. Only
  one direction is shipped per pair, mirroring `opposites.adj`'s and `synonyms.adj`'s own
  established convention. Honest abstention on "here" (a real word with a real homophone "hear",
  but not one this table carries). New manifest objective `adj.literacy.k2.homophone_pair`
  (band K-2, `recall` competency, `ccss.ela` coverage root). New e2e test
  `facts_homophones_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on an
  untabled word).
- `biology/plant-need.adj` (new) -- what a plant needs to grow and the specific role each input
  plays in photosynthesis. A new `plant_need(need, role)` table names three inputs and the role
  each plays (sunlight -> excites_chlorophyll_electrons, water -> split_for_oxygen_and_electrons,
  carbon_dioxide -> combined_to_make_glucose), quoted verbatim from Washington State University's
  "Ask Dr. Universe" science-outreach column -- "How do flowers use sunlight and water to grow?"
  -- `trust consensus` (a university outreach column, not a primary research paper). Picked using
  the mandatory full-tree-grep-before-scoping discipline -- `grep -rilE "\bplant.need|\bgerminat"
  code/specs/data/adj-facts-stdlib/` found only one incidental prose mention of "germinate" (in
  `seed-parts.adj`), confirming zero prior coverage before this file was written. Also confirmed
  this cycle: "simple circuits" and "states of energy" are BOTH dead ends, already covered by
  `physics/circuit-parts.adj` and `physics/energy-forms.adj`+`physics/energy-sources.adj`.
  WebFetch-verified before writing. Deliberately scoped to ONLY the three inputs the source gives
  a distinct role sentence for -- soil/nutrients is mentioned only in passing, with no role
  sentence of its own, so it is NOT a row. Honest abstention on "soil" (a real plant-growth
  input, but with no shipped role in this table) and "moonlight" (not a real input). New manifest
  objective `adj.science.3to5.plant_need` (band 3-5 -- the photosynthesis/electron-excitation
  language is more technical than typical K-2 content, matching `rainforest-layer.adj`'s band
  3-5 precedent -- `recall` competency, `ngss` coverage root). New e2e test
  `facts_plantneed_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on an
  untabled input).
- `language/sentence-type.adj` (new) -- a new `sentence_type(example, type)` table names four
  example sentences and which of the four grammatical sentence types each one is (declarative,
  interrogative, imperative, exclamatory), quoted verbatim from Grammarly's "4 Types of
  Sentences to Know, With Examples" article -- `trust consensus`, the same tier this stdlib
  already reserves for other non-.gov language sources (7ESL, Speakspeak). Grounds CCSS
  L.1.1.j. Picked using the mandatory full-tree-grep-before-scoping discipline -- `grep -rilE
  "\bsentence.type|\bdeclarative\b|\binterrogative\b|\bimperative\b" code/specs/data/adj-facts-stdlib/`
  confirmed ZERO existing coverage before this file was written. WebFetch-verified before
  writing. Uses SHORT ATOM-STYLE labels for the `example` column (not full-sentence string
  literals), mirroring `vocabulary-in-context.adj`'s established discipline of avoiding the ADJ
  query-grammar limitation where a quoted-string literal works as a table row VALUE but not as
  a query ARGUMENT. Honest abstention on "the cat sat on the mat" (a real, well-formed
  declarative sentence, but not one this specific cited page names). New manifest objective
  `adj.literacy.k2.sentence_type` (band K-2, `recall` competency, `ccss.ela` coverage root).
  New e2e test `facts_sentencetype_e2e.rs` (3 tests: direct recall, reverse binding, honest
  abstention on an untabled sentence).
- `earth-science/metamorphism-cause.adj` (new) -- what causes a rock to become metamorphic, and
  what that process does to it. A new `metamorphism_cause(cause, effect)` table names three
  causes (heat, pressure, hot_mineral_rich_fluids) and their shared effect
  (denser_more_compact_rock), quoted verbatim from the U.S. Geological Survey's "What are
  metamorphic rocks?" FAQ page -- `trust authoritative`, a primary U.S. government geology
  source, the same tier `rock-types.adj` already established for its own NPS citation. A
  sibling library to the already-shipped `rock-types.adj`, but a genuinely different, FINER-
  grained axis: `rock-types.adj` gives ONE combined phrase for how metamorphic rock forms
  ("heat_and_pressure"), while this table decomposes the THREE distinct causes the USGS source
  names, each with its own row and the shared effect the source states. Picked using the
  mandatory full-tree-grep-before-scoping discipline -- `grep -rilE "\brock.cycle\b|\btransform"
  code/specs/data/adj-facts-stdlib/` found only incidental prose hits (mitosis-phases.adj's
  "chromatin is transformed," monarch-life-cycle.adj's "transforms inside," plate-boundaries.adj's
  unrelated "transform" plate-boundary type), confirming zero prior coverage of a
  metamorphism-cause relation before this file was written. WebFetch-verified before writing.
  Honest abstention on "sunlight" and "cold" (not cited causes of metamorphism). New manifest
  objective `adj.science.3to5.metamorphism_cause` (band 3-5, `recall` competency, `ngss`
  coverage root). New e2e test `facts_metamorphismcause_e2e.rs` (3 tests: direct recall,
  reverse binding enumerating all three causes, honest abstention on an untabled cause).
- `language/part-of-speech.adj` (new) -- a new `part_of_speech(word, category)` table names
  three example words and which grammatical part of speech each one is (noun, verb,
  adjective), in a sentence that shows it doing that job, quoted verbatim from Grammarly's
  "The 8 Parts of Speech" article -- `trust consensus`, the same source family already used by
  `sentence-type.adj`. Grounds CCSS L.K.1.b. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -rilE "\bnoun\b|\bverb\b|\badjective\b|part_of_speech"
  code/specs/data/adj-facts-stdlib/` found only incidental prose hits (grammar descriptions in
  `past-tense-ed-sound.adj`/`plural-s-sound.adj` and unrelated adjective-as-word-choice usages
  elsewhere), confirming zero prior coverage of a word-to-part-of-speech classification before
  this file was written. WebFetch-verified TWICE before writing. Uses SHORT ATOM-STYLE labels
  for the `word` column, mirroring `sentence-type.adj`'s established discipline. Honest
  abstention on "slowly" (a real word, an adverb, but not one of the three parts of speech this
  table covers). New manifest objective `adj.literacy.k2.part_of_speech` (band K-2, `recall`
  competency, `ccss.ela` coverage root). New e2e test `facts_partofspeech_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention on an untabled word).
- `biology/frog-life-cycle.adj` (new) -- a sibling library to the already-shipped
  `monarch-life-cycle.adj`, applying the SAME plain numbered life-cycle-stage recall shape
  (`frog_life_stage(stage, order)`) to a DIFFERENT organism. Three rows (egg->1, tadpole->2,
  frog->3), quoted verbatim from National Geographic Kids UK's "The Frog Life Cycle for Kids"
  page's three numbered stage headings ("Stage 1: Extraordinary eggs", "Stage 2: Teeny
  tadpoles!", "Stage 3: Fully-grown frog!") -- `trust consensus`, the same tier this stdlib
  already reserves for other National Geographic sources (`animal-habitat.adj`,
  `rainforest-layer.adj`). Grounds NGSS 3-LS1-1, the same standard `monarch-life-cycle.adj`
  grounds. Picked using the mandatory full-tree-grep-before-scoping discipline -- `grep -rilE
  "\bfrog\b|\btadpole\b|\bfroglet\b" code/specs/data/adj-facts-stdlib/` found only incidental
  prose hits (`animal-classes.adj` lists "frog" as an amphibian example, `word-families.adj`
  lists "frog" in a rhyme list; neither is a life-cycle table), confirming zero prior coverage
  before this file was written. WebFetch-verified TWICE for consistency, both fetches returning
  the SAME three numbered headings. The source narrates leg growth occurring during the tadpole
  stage but gives that transition no separate numbered heading, so this table deliberately does
  NOT invent a fourth "froglet" row the source never numbers. Honest abstention on "adult" (the
  source's own heading says "frog," not "adult"). New manifest objective
  `adj.science.3to5.frog_life_cycle` (band 3-5, `recall` competency, `ngss` coverage root,
  mirroring `monarch-life-cycle.adj`'s exact band/competency). New e2e test
  `facts_froglifecycle_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on
  an untabled stage name).
- `language/contraction.adj` (new) -- a new `contraction(word, expansion)` table names three
  contractions and the two-word phrase each stands for (dont->do_not, cant->can_not,
  wont->will_not), quoted verbatim from Grammarly's "What Are Contractions in Writing?
  Definition and Examples" article -- `trust consensus`, the same source family already used
  by `sentence-type.adj`/`part-of-speech.adj`. Grounds CCSS L.2.2.c. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -rilE "\bcontraction\b|\bapostrophe\b"
  code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely fresh topic
  before this file was written. WebFetch-verified before writing. Uses apostrophe-free
  underscore-joined atom labels for both columns (ADJ atom labels cannot contain punctuation),
  mirroring `sentence-type.adj`'s established discipline for punctuation-bearing content; the
  `source` citation string still quotes the original punctuated text ("don't = do not") so the
  mapping stays independently checkable. Deliberately keeps the source's own two-word expansion
  "can not" for "can't" rather than silently "correcting" it to the more common single-word
  spelling "cannot". Honest abstention on "shouldnt" (a real contraction, "should not," but not
  one of these three tabled rows). New manifest objective `adj.literacy.k2.contraction` (band
  K-2, `recall` competency, `ccss.ela` coverage root). New e2e test `facts_contraction_e2e.rs`
  (3 tests: direct recall, reverse binding, honest abstention on an untabled contraction).
- `biology/animal-adaptation.adj` (new) -- a new `animal_adaptation(animal, adaptation)` table
  names three animals and the one survival adaptation each is known for (arctic_fox->camouflage,
  groundhog->hibernation, canada_goose->migration), each row quoted verbatim from a DIFFERENT
  nationalgeographic.com animal-facts page -- `trust consensus`, the same tier this stdlib
  already reserves for other National Geographic sources (`animal-habitat.adj`,
  `rainforest-layer.adj`). Grounds NGSS 3-LS4-3. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -rilE "\bgroundhog\b|\bcanada.goose\b|
  \barctic.fox\b" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely
  fresh topic before this file was written. Each of the three citations was independently
  WebFetch-verified with an explicit keyword search confirming the adaptation term ("camouflage",
  "hibernation", "migrate") appears in a clean quotable sentence on its own page -- a genuinely
  different source family from an earlier, unsuccessful attempt to cite NPS teacher-lesson-plan
  pages for this same topic, which turned out to be activity prompts rather than pages that state
  concrete animal-to-adaptation facts. Since each row's animal comes from a DIFFERENT source page
  and an ADJ `table` carries only ONE table-level `source`/`locator`/`trust` block, the table's
  own citation is the arctic_fox row's (the primary/first-listed source), and the other two rows'
  own distinct citations are documented in header prose -- mirroring
  `ocean-observing-instruments.adj`'s established multi-source discipline. Honest abstention on
  "penguin" (a real, well-known animal, but not one of these three tabled here). New manifest
  objective `adj.science.3to5.animal_adaptation` (band 3-5, `recall` competency, `ngss` coverage
  root). New e2e test `facts_animaladaptation_e2e.rs` (3 tests: direct recall, reverse binding,
  honest abstention on an untabled animal).
- `language/possessive-noun.adj` (new) -- a new `possessive_noun(word, category)` table names
  three example nouns and which of the three possessive-noun categories each one's possessive
  form falls into (dog->singular_possessive, bottles->plural_possessive,
  geese->irregular_possessive), in a sentence that shows the possessive form in use, quoted
  verbatim from Grammarly's "Possessive Nouns: How to Use Them, With Examples" article --
  `trust consensus`, the same source family already used by
  `sentence-type.adj`/`part-of-speech.adj`/`contraction.adj`. Grounds CCSS L.2.2.c. Picked
  using the mandatory full-tree-grep-before-scoping discipline -- `grep -rilE "\bpossessive\b"
  code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely fresh topic
  before this file was written. WebFetch-verified TWICE before writing. Uses apostrophe-free
  atom labels for the `word` column (ADJ atom labels cannot contain punctuation), mirroring
  `contraction.adj`'s established discipline for punctuation-bearing content; the header prose
  quotes the ORIGINAL punctuated example sentences so each mapping stays independently
  checkable. Honest abstention on "cat" (a real noun whose possessive is "cat's," but not one
  of these three tabled here). New manifest objective `adj.literacy.k2.possessive_noun` (band
  K-2, `recall` competency, `ccss.ela` coverage root). New e2e test
  `facts_possessivenoun_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on
  an untabled noun).
- `biology/animal-survival-adaptation.adj` (new) -- a new
  `animal_survival_adaptation(animal, adaptation)` table names three animals and the one
  survival adaptation each is known for (bats->echolocation, polar_bear->insulation,
  poison_dart_frog->warning_coloration), each row quoted verbatim from a DIFFERENT
  nationalgeographic.com animal-facts page -- `trust consensus`, the same tier this stdlib
  already reserves for other National Geographic sources (`animal-adaptation.adj`,
  `animal-habitat.adj`). Grounds NGSS 3-LS4-3. DELIBERATELY uses a different predicate name
  from the already-shipped `animal_adaptation` table (`animal-adaptation.adj`) -- that table
  already closed out arctic_fox/groundhog/canada_goose as its three rows, so this genuinely
  different set of animals/adaptations gets its own predicate rather than extending a closed
  table, mirroring how `monarch_life_stage`/`frog_life_stage` used distinct predicate names for
  the same shape applied to different organisms. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -rilE "\bmicrobat|animal_survival_adaptation|
  \becholocation\b|\baposematic\b" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming
  a completely fresh topic before this file was written. All three citations WebFetch-verified
  before writing (the bats quote needed a second, more targeted search for "microbats" in the
  page's "Classification" section, but is present verbatim). Since each row's animal comes from a
  DIFFERENT source page and an ADJ `table` carries only ONE table-level `source`/`locator`/
  `trust` block, the table's own citation is the bats row's (the primary/first-listed source),
  and the other two rows' own distinct citations are documented in header prose -- mirroring
  `animal-adaptation.adj`'s established multi-source discipline. Note the `polar_bear` atom also
  appears in `animal-habitat.adj` for a DIFFERENT fact (its habitat, not this adaptation) -- not
  a conflict, since that is a different predicate. Honest abstention on "chameleon" (a real,
  well-known animal, but not one of these three tabled here). New manifest objective
  `adj.science.3to5.animal_survival_adaptation` (band 3-5, `recall` competency, `ngss` coverage
  root). New e2e test `facts_animalsurvivaladaptation_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an untabled animal).
- `language/simile-meaning.adj` (new) -- a new `simile_meaning(simile, meaning)` table names
  three common similes and what each actually means (as_brave_as_a_lion->extremely_courageous,
  like_a_needle_in_a_haystack->very_difficult_to_find, as_free_as_a_bird->free_or_unrestricted),
  quoted verbatim from Grammarly's "Simile: Definition and Examples" article's "Common simile
  examples" table -- `trust consensus`, the same source family already used by
  `sentence-type.adj`/`part-of-speech.adj`/`contraction.adj`/`possessive-noun.adj`. A sibling
  figurative-language library to `idiom-meaning.adj`, using the same band (3-5) and the same
  apostrophe/punctuation-free underscore-joined atom-label discipline for multi-word phrases.
  Grounds CCSS L.5.5.a. Picked using the mandatory full-tree-grep-before-scoping discipline --
  `grep -rilE "\bsimile\b" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a
  completely fresh topic before this file was written (only an incidental "figurative-language"
  prose mention in `idiom-meaning.adj`'s own header, not an actual simile table). WebFetch-verified
  before writing. Honest abstention on "as_busy_as_a_bee" (a real, well-known simile, but not one
  of these three tabled here). New manifest objective `adj.literacy.3to5.simile_meaning` (band
  3-5, `recall` competency, `ccss.ela` coverage root). New e2e test
  `facts_simile_meaning_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on
  an untabled simile).
- `biology/plant-life-cycle.adj` (new) -- a sibling library to the already-shipped
  `monarch-life-cycle.adj`/`frog-life-cycle.adj`, applying the SAME plain numbered
  life-cycle-stage recall shape (`plant_life_stage(stage, order)`) to a flowering plant's early
  life. Three rows (seed->1, germination->2, seedling->3), quoted verbatim from Ducksters'
  "Flowering Plants" (Biology for Kids) article's "Life-cycle of a Flowering Plant" section --
  `trust consensus`. This is the FIRST citation from Ducksters in this stdlib -- a reputable,
  long-running kids-science-education site, the same tier this stdlib already reserves for
  other non-.gov kids-education sources (National Geographic Kids, Grammarly), not a primary
  .gov source. Grounds NGSS 3-LS1-1. Picked using the mandatory full-tree-grep-before-scoping
  discipline -- `grep -rilE "\bgermination\b|plant_life_stage" code/specs/data/adj-facts-stdlib/`
  found ZERO hits, confirming a completely fresh topic before this file was written. Three
  candidate sources were ruled out before Ducksters: natgeokids.com's UK plant-life-cycle page
  (no clean numbered stages), coolkidfacts.com (no numbered list), and smartclass4kids.com
  (numbered but an unbranded low-trust site). WebFetch-verified before writing. Honest
  abstention on "flowering" (a real later stage the source's own narrative goes on to describe,
  but not one of these three tabled here, keeping this slice the same size as every sibling
  life-cycle library). New manifest objective `adj.science.3to5.plant_life_stage` (band 3-5,
  `recall` competency, `ngss` coverage root). New e2e test `facts_plantlifecycle_e2e.rs` (3
  tests: direct recall, reverse binding, honest abstention on an untabled stage name).
- `language/prefix-meaning.adj` (new) -- a new `prefix_meaning(prefix, meaning)` table names
  three common English prefixes and what each actually means (un_->negation_or_absence,
  re_->doing_again, dis_->negation_or_reversal), quoted verbatim from Grammarly's "Prefixes:
  Definition and Examples" article -- `trust consensus`, the same source family already used by
  `sentence-type.adj`/`part-of-speech.adj`/`contraction.adj`/`possessive-noun.adj`/
  `simile-meaning.adj`. Introduces a NEW atom-label convention for this stdlib -- a TRAILING
  UNDERSCORE marks that an atom is a prefix attaching to the front of a word (ADJ atom labels
  cannot contain hyphens), distinct from the underscore-joined multi-word-phrase convention
  `idiom-meaning.adj`/`simile-meaning.adj` already established. Grounds CCSS L.4.4.b. Picked
  using the mandatory full-tree-grep-before-scoping discipline -- `grep -rilE "\bprefix\b|
  \bsuffix\b" code/specs/data/adj-facts-stdlib/` found only `metric-prefixes.adj` (a genuinely
  DIFFERENT topic -- metric UNIT prefixes like kilo-/centi-, not word-morphology) plus incidental
  "suffix" prose mentions in `past-tense-ed-sound.adj`/`plural-s-sound.adj`, confirming a
  completely fresh word-morphology topic before this file was written. WebFetch-verified before
  writing. Honest abstention on "over_" (a real, well-known prefix, but not one of these three
  tabled here). New manifest objective `adj.literacy.3to5.prefix_meaning` (band 3-5, `recall`
  competency, `ccss.ela` coverage root). New e2e test `facts_prefixmeaning_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention on an untabled prefix).
- `oceanography/ocean-zones.adj` (new) -- a sibling library to the already-shipped
  `plant-life-cycle.adj`/`frog-life-cycle.adj`, applying the SAME plain numbered
  ordered-sequence recall shape (`ocean_zone(zone, order)`) to the ocean's first three
  depth-based light zones. Three rows (sunlight_zone->1, twilight_zone->2, midnight_zone->3),
  quoted verbatim from the Woods Hole Oceanographic Institution (WHOI) "Ocean Zones" page's
  "What are the five ocean zones?" section, which lists all five zones in depth order in one
  summary sentence before giving each its own subsection in that same order -- `trust
  consensus` (WHOI is a reputable, long-running oceanographic research institution, but is NOT
  a .gov domain, distinct from the `authoritative` tier this stdlib reserves for primary .gov
  sources like NOAA, which the sibling `ocean-observing-instruments.adj` -- the only other
  library in this same directory -- uses). Grounds NGSS 3-5 ocean-systems standards. Picked
  using the mandatory full-tree-grep-before-scoping discipline --
  `grep -rilE "\bepipelagic\b|\bmesopelagic\b|\bbathypelagic\b|ocean_zone|\bsunlight.zone\b|
  \btwilight.zone\b|\bocean.layer"  code/specs/data/adj-facts-stdlib/` found ZERO hits,
  confirming a completely fresh topic before this file was written. WebFetch-verified before
  writing (twice, across two cycles of this loop). Honest abstention on "abyssal_zone" (a real
  deeper zone the source names, but not one of these three tabled here, keeping this slice the
  same size as every sibling ordered-sequence library). New manifest objective
  `adj.science.3to5.ocean_zone` (band 3-5, `recall` competency, `ngss` coverage root). New e2e
  test `facts_oceanzones_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on
  an untabled zone name).
- `language/capitalization-rule.adj` (new) -- a new `capitalization_rule(rule, description)`
  table names three common English capitalization rules and what each actually requires
  (first_word_of_sentence->capitalize_first_letter, pronoun_i->capitalized_anywhere_in_sentence,
  proper_noun->capitalized_regardless_of_position), quoted verbatim from Grammarly's
  "Capitalization Rules and Examples" article -- `trust consensus`, the same source family
  already used by `sentence-type.adj`/`part-of-speech.adj`/`contraction.adj`/
  `possessive-noun.adj`/`simile-meaning.adj`/`prefix-meaning.adj`. Grounds CCSS L.K.2.a. Picked
  using the mandatory full-tree-grep-before-scoping discipline --
  `grep -rilE "\bcapitali[sz]" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a
  completely fresh topic before this file was written. WebFetch-verified before writing (twice,
  across two cycles of this loop). Honest abstention on "quotation" (a real capitalization rule
  the same article covers in its "Capitalization and quotes" section, but not one of these three
  tabled here). New manifest objective `adj.literacy.k2.capitalization_rule` (band K-2, matching
  `sentence-type.adj`'s band, `recall` competency, `ccss.ela` coverage root). New e2e test
  `facts_capitalizationrule_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention
  on an untabled rule).
- `oceanography/ocean-current-drivers.adj` (new) -- a sibling library to `ocean-zones.adj`, a
  DIFFERENT oceanography axis (what moves the water, not how deep light reaches). A new
  `ocean_current_driver(current_type, driver)` table names three ocean-current categories and the
  physical driver that creates each (tidal_currents->tides, wind_driven_currents->wind,
  thermohaline_circulation->density_differences_from_temperature_and_salinity), quoted verbatim
  from NOAA National Ocean Service's "What is a current?" page, which numbers exactly these three
  driving mechanisms as its own answer to that question -- `trust authoritative` (NOAA is a
  primary .gov source, the same tier the sibling `ocean-observing-instruments.adj` -- the only
  other library in this same directory -- already uses, distinct from the `consensus` tier
  `ocean-zones.adj` uses for its non-.gov WHOI citation). A MULTI-SOURCE-STYLE table (see
  `animal-survival-adaptation.adj`): each row's quote comes from a different paragraph of the same
  page, with the table's own `source` field carrying the first row's (tidal_currents) quote and
  the other two rows' quotes documented in the file's header prose. Grounds NGSS 3-5
  ocean-systems standards. Picked using the mandatory full-tree-grep-before-scoping discipline --
  `grep -rilE "\btidal.current\b|\bthermohaline\b|\bocean.current.driver\b|wind.driven.current|
  \bgulf.stream\b" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely
  fresh topic before this file was written. WebFetch-verified before writing (twice, across two
  cycles of this loop). Honest abstention on "gulf_stream" (a real, specific named current the
  same page mentions, but not one of these three driver CATEGORIES tabled here). New manifest
  objective `adj.science.3to5.ocean_current_driver` (band 3-5, `recall` competency, `ngss`
  coverage root). New e2e test `facts_oceancurrentdrivers_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an untabled current name).
- `language/superlative-adjective-rule.adj` (new) -- a new `superlative_adjective_rule(rule,
  description)` table names three common English superlative-adjective formation rules and what
  each actually requires (one_syllable_adjective->add_est_suffix,
  one_syllable_consonant_vowel_consonant->double_final_consonant_before_est,
  adjective_ending_in_y->change_y_to_i_before_est), quoted verbatim from Grammarly's "What Are
  Superlative Adjectives? Definition and Examples" article -- `trust consensus`, the same source
  family already used by `sentence-type.adj`/`part-of-speech.adj`/`contraction.adj`/
  `possessive-noun.adj`/`simile-meaning.adj`/`prefix-meaning.adj`/`capitalization-rule.adj`.
  Grounds CCSS L.4.1.a. Picked using the mandatory full-tree-grep-before-scoping discipline --
  `grep -rilE "\bsuperlative\b|superlative_adjective_rule|\bcomparative_adjective\b"
  code/specs/data/adj-facts-stdlib/` found only an incidental unrelated match in
  `meteorology/hurricane-categories.adj`'s header prose, confirming a completely fresh topic
  before this file was written. WebFetch-verified before writing (twice, across two cycles of this
  loop -- the second pass specifically re-confirmed exact wording after an initial fetch surfaced
  a bullet-list fragment for a DIFFERENT, ultimately rejected rule about long adjectives). Honest
  abstention on "three_or_more_syllable_adjective" (a real rule the same article covers -- longer
  adjectives use "most" instead of "-est" -- but whose own supporting text on the page is a
  bullet-list fragment rather than a clean quotable sentence, and which is not one of these three
  tabled here). New manifest objective `adj.literacy.3to5.superlative_adjective_rule` (band 3-5,
  `recall` competency, `ccss.ela` coverage root). New e2e test `facts_superlativeadjectiverule_e2e.rs`
  (3 tests: direct recall, reverse binding, honest abstention on an untabled rule).
- `geology/fossil-formation-type.adj` (new) -- a new `fossil_formation_type(type, description)`
  table names three ways a fossil can form and what each actually is
  (amber->preserved_in_hardened_tree_sap, cast_or_mold->impression_of_a_living_organism,
  permineralization->mineral_deposits_form_a_cast_of_the_organism), quoted verbatim from
  Ducksters' "Earth Science for Kids: Fossils" page -- `trust consensus`, the same tier
  `ocean-zones.adj` uses for its non-.gov WHOI citation. Grounds NGSS 3-5 earth-science
  standards. Picked using the mandatory full-tree-grep-before-scoping discipline --
  `grep -rilE "\bfossil\b|fossil_formation_type|\bpermineralization\b|\bamber\b|cast_or_mold"
  code/specs/data/adj-facts-stdlib/` found only two incidental unrelated matches
  (`astronomy/spectral-classes.adj` and `biology/genetic-code.adj`'s "amber" STOP-codon
  nickname), confirming a completely fresh topic before this file was written.
  WebFetch-verified before writing (twice). Honest abstention on "freezing" (a real
  preservation method the same page mentions, but only in a single bare sentence rather
  than the fuller description style used for the three tabled here). NPS's "How Fossils
  Form" page was investigated first and deprioritized -- it mentions fossil types but lacks
  clean one-sentence definitions per type, unlike Ducksters. New manifest objective
  `adj.science.3to5.fossil_formation_type` (band 3-5, `recall` competency, `ngss` coverage
  root). New e2e test `facts_fossilformationtype_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an untabled formation type).
- `language/noun-type.adj` (new) -- a new `noun_type(type, definition)` table names three
  noun types and what each actually is (common_noun->generic_name_of_an_item_in_a_class_or_group,
  collective_noun->denotes_a_group_or_collection_of_people_or_things,
  abstract_noun->cannot_be_perceived_by_the_senses), quoted verbatim from Grammarly's "Nouns:
  Definition and Examples" article -- `trust consensus`, the same source family already used
  by `sentence-type.adj`/`part-of-speech.adj`/`possessive-noun.adj`/
  `superlative-adjective-rule.adj`. Grounds CCSS L.1.1.b. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -rilE "\bnoun_type\b|\bcommon_noun\b|
  \bcollective_noun\b|\babstract_noun\b|\bconcrete_noun\b" code/specs/data/adj-facts-stdlib/`
  found ZERO hits, confirming a completely fresh topic before this file was written.
  WebFetch-verified before writing (twice). Honest abstention on "possessive_noun" (a real
  noun type the same page mentions and describes functionally, but for which it gives no
  single formal one-sentence definition the way it does for the three tabled here -- a
  distinct concept from the already-shipped `possessive-noun.adj`, whose
  `possessive_noun(word, category)` table classifies possessive-FORM examples, not the
  general definition of "what is a possessive noun"). New manifest objective
  `adj.literacy.k2.noun_type` (band K-2, `recall` competency, `ccss.ela` coverage root). New
  e2e test `facts_nountype_e2e.rs` (3 tests: direct recall, reverse binding, honest
  abstention on an untabled noun type).
- `astronomy/solar-eclipse-type.adj` (new) -- a new `solar_eclipse_type(type, description)`
  table names three solar eclipse types and what each actually is
  (total_solar_eclipse->completely_blocking_the_face_of_the_sun,
  annular_solar_eclipse->moon_at_or_near_its_farthest_point_from_earth,
  partial_solar_eclipse->sun_moon_and_earth_not_perfectly_lined_up), quoted verbatim from
  NASA's "Types of Solar Eclipses" page -- `trust authoritative`, the same tier the sibling
  `moon-phases.adj` (the other library in this directory) already uses for its NASA citation.
  Picked using the mandatory full-tree-grep-before-scoping discipline --
  `grep -rilE "\bsolar_eclipse\b|\btotal_solar_eclipse\b|\bannular\b|\bpartial_solar_eclipse\b|
  \bhybrid_solar_eclipse\b|eclipse_type" code/specs/data/adj-facts-stdlib/` found ZERO hits
  (the sibling `moon-phases.adj` only mentions "eclipse" once, as a deliberately-excluded
  non-phase example), confirming a completely fresh topic before this file was written.
  WebFetch-verified before writing (twice). Honest abstention on "hybrid_solar_eclipse" (a
  real eclipse type the same page also names, but whose own explanation takes TWO sentences
  -- how Earth's curved surface lets an eclipse shift between annular and total -- rather
  than one clean quotable sentence like the three tabled here). New manifest objective
  `adj.science.3to5.solar_eclipse_type` (band 3-5, `recall` competency, `ngss` coverage
  root). New e2e test `facts_solareclipsetype_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an untabled eclipse type).
- `language/verb-type.adj` (new) -- a new `verb_type(type, description)` table names three
  verb types and what each actually is
  (action_verb->physical_action_or_activity_that_can_be_seen_or_heard,
  linking_verb->connects_the_subject_to_other_words_in_the_sentence,
  auxiliary_verb->changes_another_verbs_tense_voice_or_mood), quoted verbatim from
  Grammarly's "Verbs: Definition and Examples" article -- `trust consensus`, the same
  source family already used by `sentence-type.adj`/`part-of-speech.adj`/`noun-type.adj`.
  Grounds CCSS L.1.1.e. Picked using the mandatory full-tree-grep-before-scoping discipline
  -- `grep -rilE "\bverb_type\b|\baction_verb\b|\blinking_verb\b|\bauxiliary_verb\b|
  \bhelping_verb\b|\btransitive_verb\b" code/specs/data/adj-facts-stdlib/` found ZERO hits,
  confirming a completely fresh topic before this file was written. WebFetch-verified
  before writing (twice). Honest abstention on "transitive_verb" (a real verb category the
  same page also covers, but only through worked examples and category description rather
  than a single formal definition sentence the way it does for the three tabled here). New
  manifest objective `adj.literacy.k2.verb_type` (band K-2, `recall` competency, `ccss.ela`
  coverage root). New e2e test `facts_verbtype_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an untabled verb type).
- `astronomy/comet-part.adj` (new) -- a new `comet_part(part, description)` table names
  three physical parts of a comet and what each actually is
  (nucleus->solid_frozen_core_at_the_heart_of_the_comet,
  coma->fuzzy_cloud_of_gas_and_dust_around_the_nucleus,
  tail->streams_away_from_the_nucleus_pushed_by_sunlight_and_solar_particles), quoted
  verbatim from NASA Space Place's "What Is a Comet?" page -- `trust authoritative`, the
  same tier the sibling `moon-phases.adj`/`solar-eclipse-type.adj` (the other libraries in
  this directory) already use for their NASA citations. Grounds NGSS 3-5 space-systems
  standards. Picked using the mandatory full-tree-grep-before-scoping discipline --
  `grep -rilE "\bcomet_part\b|\bnucleus\b|\bcoma\b|comet.*tail|\bshort_period_comet\b|
  \blong_period_comet\b" code/specs/data/adj-facts-stdlib/` found only incidental
  unrelated matches (cell/atomic "nucleus" in biology/chemistry libraries), confirming a
  completely fresh comet-specific topic before this file was written. WebFetch-verified
  before writing (twice). Honest abstention on "short_period_comet" (a real comet-related
  term the same page also names, but one that classifies comets by ORBITAL PERIOD rather
  than by physical anatomy -- a different axis from nucleus/coma/tail, and not one of these
  three tabled here). New manifest objective `adj.science.3to5.comet_part` (band 3-5,
  `recall` competency, `ngss` coverage root). New e2e test `facts_cometpart_e2e.rs`
  (3 tests: direct recall, reverse binding, honest abstention on an untabled comet term).
- `language/pronoun-type.adj` (new) -- a new `pronoun_type(type, description)` table names
  three pronoun types and what each actually is
  (personal_pronoun->changes_form_based_on_grammatical_person,
  indefinite_pronoun->refers_generally_without_specific_identification,
  interrogative_pronoun->used_in_questions), quoted verbatim from Grammarly's "Pronouns:
  Definition and Examples" article -- `trust consensus`, the same source family already
  used by `noun-type.adj`/`verb-type.adj`. Grounds CCSS L.1.1.d. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -rilE "\bpronoun_type\b|
  \bpersonal_pronoun\b|\bindefinite_pronoun\b|\binterrogative_pronoun\b|
  \brelative_pronoun\b" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a
  completely fresh topic before this file was written. WebFetch-verified before writing
  (twice -- the second pass confirmed the FULL sentences, since a first, shorter-truncated
  fetch had clipped the personal-pronoun sentence and undercounted the relative-pronoun
  passage's sentence count). Honest abstention on "relative_pronoun" (a real pronoun type
  the same page also covers, but whose own explanation takes THREE sentences rather than
  one clean self-contained sentence like the three tabled here). New manifest objective
  `adj.literacy.k2.pronoun_type` (band K-2, `recall` competency, `ccss.ela` coverage root).
  New e2e test `facts_pronountype_e2e.rs` (3 tests: direct recall, reverse binding, honest
  abstention on an untabled pronoun type).
- `astronomy/space-rock-stage.adj` (new) -- a new `space_rock_stage(stage, description)` table
  names three stages a single rocky object passes through, not three different kinds of object
  (meteoroid->still_a_rock_in_space,
  meteor->called_a_fireball_or_shooting_star_when_it_burns_up_in_the_atmosphere,
  meteorite->survives_the_atmosphere_and_hits_the_ground), quoted verbatim from NASA Science's
  "Meteors & Meteorites" page -- `trust authoritative`, the same tier the sibling
  `comet-part.adj`/`solar-eclipse-type.adj` (the other libraries in this directory) already use
  for their NASA citations. Grounds NGSS 3-5 space-systems standards. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -rilE "meteoroid|meteorite|meteor_type|
  space_rock_type" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely
  fresh topic before this file was written. WebFetch-verified before writing (twice -- the
  second pass specifically re-checked the meteoroid sentence's exact wording against its
  surrounding paragraph, since a first-pass fetch can silently paraphrase a short or
  awkwardly-worded source sentence). Honest abstention on "asteroid" (a real object the same
  page mentions in passing -- "Meteoroids range in size from dust grains to small asteroids" --
  but never defines in a sentence of its own on this page, unlike the three stages tabled here).
  New manifest objective `adj.science.3to5.space_rock_stage` (band 3-5, `recall` competency,
  `ngss` coverage root). New e2e test `facts_spacerockstage_e2e.rs` (3 tests: direct recall,
  reverse binding, honest abstention on an untabled term).
- `language/preposition-type.adj` (new) -- a new `preposition_type(type, description)` table
  names three categories of preposition and what each actually shows
  (preposition_of_place->shows_where_something_is_or_where_something_happened,
  preposition_of_time->shows_when_something_happened_or_will_happen,
  preposition_of_direction->shows_how_something_is_moving_or_which_way_its_going), quoted
  verbatim from Grammarly's "Prepositions: Definition, Types, and Examples" article --
  `trust consensus`, the same source family already used by `noun-type.adj`/`verb-type.adj`/
  `pronoun-type.adj`. Grounds CCSS L.1.1.i. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -rilE "preposition_type|
  preposition_of_place|preposition_of_time|preposition_of_direction|
  preposition_of_manner" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a
  completely fresh topic before this file was written. WebFetch-verified before writing
  (twice -- the second pass specifically re-checked whether the direction/movement sentence
  was complete on its own or ran on into a following worked example, since a first-pass fetch
  can misjudge where a source sentence actually ends). Honest abstention on
  "preposition_of_manner_cause_or_purpose" (a real category the same page also names, but one
  that bundles THREE distinct functions -- manner, cause, or purpose -- under a single label,
  rather than one clean single-concept category like the three tabled here). New manifest
  objective `adj.literacy.k2.preposition_type` (band K-2, `recall` competency, `ccss.ela`
  coverage root). New e2e test `facts_prepositiontype_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an untabled preposition category).
- `language/conjunction-type.adj` (new) -- a new `conjunction_type(type, description)` table
  names three categories of conjunction and what each actually does
  (coordinating_conjunction->joins_words_phrases_and_clauses_of_equal_grammatical_rank,
  correlative_conjunction->are_pairs_of_conjunctions_that_work_together,
  subordinating_conjunction->joins_dependent_clauses_to_independent_clauses), quoted verbatim
  from Grammarly's "Conjunctions" article -- `trust consensus`, the same source family already
  used by `noun-type.adj`/`verb-type.adj`/`pronoun-type.adj`/`preposition-type.adj`. Grounds
  CCSS L.1.1.i. Picked using the mandatory full-tree-grep-before-scoping discipline --
  `grep -rilE "conjunction_type|coordinating_conjunction|correlative_conjunction|
  subordinating_conjunction|conjunctive_adverb" code/specs/data/adj-facts-stdlib/` found ZERO
  hits, confirming a completely fresh topic before this file was written. WebFetch-verified
  before writing (twice -- the second pass specifically re-confirmed the coordinating and
  correlative sentences were complete, with no truncation or missing clauses). Honest
  abstention on "conjunctive_adverb" (a real category the same page also names, but one that
  belongs to a DIFFERENT word class -- an adverb, not a conjunction -- rather than being a
  fourth conjunction type). New manifest objective `adj.literacy.k2.conjunction_type` (band
  K-2, `recall` competency, `ccss.ela` coverage root). New e2e test
  `facts_conjunctiontype_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention
  on an untabled conjunction type).
- `astronomy/planet-criterion.adj` (new) -- a new `planet_criterion(criterion, requirement)`
  table names the three IAU requirements a body must meet to count as a full planet, not a
  dwarf planet (orbit->orbits_its_host_star, roundness->is_mostly_round,
  cleared_orbit->gravity_cleared_away_other_objects_of_similar_size_near_its_orbit), quoted
  verbatim from NASA Science's "Dwarf Planets" page -- `trust authoritative`, the same tier
  the sibling `comet-part.adj`/`space-rock-stage.adj` (the other libraries in this directory)
  already use for their NASA citations. Grounds NGSS 3-5 space-systems standards. Picked using
  the mandatory full-tree-grep-before-scoping discipline -- `grep -rilE "planet_criterion|
  orbits_its_host_star|is_mostly_round|cleared_away|dwarf_planet"
  code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely fresh topic
  before this file was written. WebFetch-verified before writing (twice -- the second pass
  specifically confirmed the introductory sentence's exact wording and that it directly
  precedes the three-item list). Honest abstention on "dwarf_planet" (a real classification
  the same page also names, but one that is defined as satisfying the FIRST TWO criteria
  while FAILING the third -- a compound classification built FROM these criteria, not a
  fourth criterion itself, and not one of these three tabled here). New manifest objective
  `adj.science.3to5.planet_criterion` (band 3-5, `recall` competency, `ngss` coverage root).
  New e2e test `facts_planetcriterion_e2e.rs` (3 tests: direct recall, reverse binding,
  honest abstention on an untabled term).
- `language/determiner-type.adj` (new) -- a new `determiner_type(type, description)` table
  names three determiner categories and what each actually does (article->precedes_a_noun_
  and_identifies_it_as_specific_or_nonspecific, demonstrative_determiner->communicates_the_
  placement_of_a_noun_in_space_or_time, distributive_determiner->refers_to_a_group_or_
  individual_parts_within_a_group), quoted verbatim from Grammarly's "What Are Determiners?"
  article -- `trust consensus`, the same tier the sibling `noun-type.adj`/`verb-type.adj`/
  `pronoun-type.adj`/`preposition-type.adj`/`conjunction-type.adj` (the other libraries in
  this directory) already use for their Grammarly citations. Grounds CCSS L.K.1.b/L.1.1.
  Picked using the mandatory full-tree-grep-before-scoping discipline -- `grep -rilE
  "determiner_type|article|demonstrative_determiner|distributive_determiner|
  possessive_determiner" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a
  completely fresh topic before this file was written. WebFetch-verified before writing
  (twice -- the second pass specifically re-confirmed all three sentences were complete,
  with no truncation or additional clauses following them). Honest abstention on
  "possessive_determiner" (a real category the same page also names, but one whose own
  defining sentence bundles TWO separate facts -- that it is the possessive form of a
  personal pronoun, AND that it can appear before a noun -- plus a full inline list of
  examples, rather than one clean single-fact sentence like the three tabled here). New
  manifest objective `adj.literacy.k2.determiner_type` (band K-2, `recall` competency,
  `ccss.ela` coverage root). New e2e test `facts_determinertype_e2e.rs` (3 tests: direct
  recall, reverse binding, honest abstention on an untabled determiner type).
- `geology/volcano-type.adj` (new) -- a new `volcano_type(type, description)` table names
  three types of volcano and what each actually is (cinder_cone->is_the_simplest_type_of_
  volcano, shield_volcano->built_almost_entirely_of_fluid_lava_flows, composite_volcano->
  also_called_a_stratovolcano), quoted verbatim from USGS's "About Volcanoes" page --
  `trust authoritative`, the same tier the sibling `rock-type.adj`/`mineral-hardness.adj`
  (the other libraries in this directory) already use for their USGS citations. Grounds
  NGSS 4-ESS1-1/MS-ESS2-1. Picked using the mandatory full-tree-grep-before-scoping
  discipline -- `grep -rilE "cinder_cone|shield_volcano|composite_volcano|lava_dome"
  code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely fresh topic
  before this file was written. WebFetch-verified before writing (twice -- the second pass
  specifically confirmed each sentence's exact wording and where it actually ends). Honest
  abstention on "lava_dome" (a real term the same page also names, but one the source
  ITSELF explicitly disclaims as not a type: "these are technically not a 'volcano type'
  but rather an eruption phenomenon"). New manifest objective `adj.science.3to5.volcano_type`
  (band 3-5, `recall` competency, `ngss` coverage root). New e2e test
  `facts_volcanotype_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on
  an untabled term).
- `language/end-punctuation-mark.adj` (new) -- a new `end_punctuation_mark(mark,
  description)` table names three marks that end a sentence and what each actually does
  (period->ends_a_declarative_sentence, question_mark->communicates_that_a_sentence_is_a_
  question, exclamation_point->makes_sentences_exciting), quoted verbatim from Grammarly's
  "Punctuation: The Best Guide to Using Punctuation Marks" article -- `trust consensus`,
  the same tier the sibling `noun-type.adj`/`verb-type.adj`/`pronoun-type.adj`/
  `preposition-type.adj`/`conjunction-type.adj`/`determiner-type.adj` (the other libraries
  in this directory) already use for their Grammarly citations. Grounds CCSS L.K.2.b.
  Picked using the mandatory full-tree-grep-before-scoping discipline -- `grep -rilE
  "end_punctuation|punctuation_mark|question_mark|exclamation_point|declarative_sentence"
  code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely fresh topic
  before this file was written. WebFetch-verified before writing (twice -- the second pass
  specifically re-confirmed each sentence's exact wording via the surrounding paragraph).
  Honest abstention on "comma" (a real mark the same page also covers, but one that belongs
  to a DIFFERENT category -- a mid-sentence pause mark, not an end-of-sentence mark like
  the three tabled here). New manifest objective `adj.literacy.k2.end_punctuation_mark`
  (band K-2, `recall` competency, `ccss.ela` coverage root). New e2e test
  `facts_endpunctuationmark_e2e.rs` (3 tests: direct recall, reverse binding, honest
  abstention on an untabled punctuation mark).
- `geography/map-type.adj` (new) -- a new `map_type(type, description)` table names three
  types of map and what each actually shows (political->shows_boundaries_between_countries_
  states_counties_and_other_political_units, physical->shows_the_natural_landscape_features_
  of_earth, topographic->shows_the_shape_of_earths_surface), quoted verbatim from
  Geology.com's "Types of Maps" article -- `trust consensus`, the same tier the sibling
  `rock-type.adj` uses for some of its non-USGS citations. Grounds NGSS/social-studies map-
  skills standards for grades 3-5. Picked using the mandatory full-tree-grep-before-scoping
  discipline -- `grep -rilE "map_type|physical_map|political_map|topographic_map|
  climate_map" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely
  fresh topic before this file was written. WebFetch-verified before writing (twice -- the
  second pass specifically confirmed each sentence's exact wording, the section heading it
  appears under, and that no additional clause is attached as part of the same defining
  thought). Honest abstention on "weather" (a real map category the same page also covers,
  but one whose own section never states a single complete defining sentence the way the
  three tabled here do). New manifest objective `adj.science.3to5.map_type` (band 3-5,
  `recall` competency, `ngss` coverage root -- no dedicated social-studies coverage root is
  declared in this manifest yet, so this follows the same convention already used for other
  geography-adjacent 3-5 recall content). New e2e test `facts_maptype_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention on an untabled map type).
- `language/figurative-language-type.adj` (new) -- a new `figurative_language_type(type,
  description)` table names three figures of speech and what each actually does
  (metaphor->describes_something_in_a_way_thats_not_literally_true_to_make_a_comparison,
  personification->gives_human_characteristics_to_nonhuman_or_abstract_things,
  hyperbole->a_great_exaggeration_used_to_add_emphasis), quoted verbatim from Grammarly's
  "Figurative Language Examples: 6 Common Types and Definitions" article -- `trust
  consensus`, the same tier the sibling `noun-type.adj`/`verb-type.adj`/`pronoun-type.adj`/
  `preposition-type.adj`/`conjunction-type.adj`/`determiner-type.adj`/
  `end-punctuation-mark.adj` (the other libraries in this directory) already use for their
  Grammarly citations. Grounds CCSS L.5.5a. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -rilE
  "figurative_language|metaphor|personification|hyperbole|allusion"
  code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely fresh topic
  before this file was written. WebFetch-verified before writing (twice -- the second pass
  specifically confirmed each sentence's exact wording and that it stands alone as a
  complete definition before any follow-up example sentence). Honest abstention on
  "allusion" (a real device the same page also names with its own clean defining sentence,
  but one that works by referencing an external work, person, or event rather than by
  comparison, exaggeration, or personification -- a different rhetorical mechanism than the
  three tabled here). Simile and idiom, also named on the same page, are deliberately
  excluded too: both are already grounded as their own separately-shipped libraries in this
  directory (`simile-meaning.adj`, `idiom-meaning.adj`), so tabling them again here under a
  different predicate would duplicate coverage rather than add to it. New manifest objective
  `adj.literacy.k2.figurative_language_type` (band K-2, `recall` competency, `ccss.ela`
  coverage root). New e2e test `facts_figurativelanguagetype_e2e.rs` (3 tests: direct
  recall, reverse binding, honest abstention on an untabled figure of speech).
- `biology/biome-type.adj` (new) -- a new `biome_type(biome, description)` table names four
  major biomes and what defines each
  (desert->dry_areas_where_rainfall_is_less_than_50_centimeters_20_inches_per_year,
  forest->dominated_by_trees_and_cover_about_one_third_of_the_earth,
  grassland->open_regions_dominated_by_grass_with_a_warm_dry_climate,
  tundra->has_extremely_inhospitable_conditions_with_the_lowest_measured_temperatures),
  quoted verbatim from National Geographic Education's "The Five Major Types of Biomes"
  article -- `trust consensus`, the same tier already used for `map-type.adj`'s Geology.com
  citation and `figurative-language-type.adj`'s Grammarly citation. Picked using the
  mandatory full-tree-grep-before-scoping discipline -- `grep -rilE "biome_type|tundra"
  code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely fresh topic
  before this file was written (also confirmed `biology/animal-habitat.adj` maps individual
  animals to habitat NAMES, never tables biome-level defining sentences, so no overlap).
  WebFetch-verified before writing (twice -- the second pass pulled the full surrounding
  paragraph for each candidate sentence to confirm it stands alone grammatically and isn't
  qualified or contradicted by an immediately adjacent sentence). Honest abstention on
  "aquatic" (the source's fifth major biome, but one whose own section opens by deferring to
  its freshwater and marine sub-categories rather than stating a single, complete defining
  sentence the way the four tabled here do). New manifest objective
  `adj.science.3to5.biome_type` (band 3-5, `recall` competency, `ngss` coverage root). New
  e2e test `facts_biometype_e2e.rs` (3 tests: direct recall, reverse binding, honest
  abstention on an untabled biome).
- `language/sound-device-type.adj` (new) -- a new `sound_device_type(device, description)`
  table names two sound devices and what each actually does
  (onomatopoeia->is_when_a_word_imitates_the_natural_sound_of_a_thing,
  alliteration->repeating_consonant_sounds_right_next_to_each_other), quoted verbatim from
  Grammarly's "20 Types of Figures of Speech: Definitions and Examples" article -- `trust
  consensus`, the same tier `figurative-language-type.adj` (a sibling library in this
  directory, sourced from a DIFFERENT Grammarly article) already uses. Picked using the
  mandatory full-tree-grep-before-scoping discipline -- `grep -rilE
  "sound_device_type|onomatopoeia|alliteration" code/specs/data/adj-facts-stdlib/` found ZERO
  hits, confirming a completely fresh topic before this file was written. WebFetch-verified
  before writing (twice -- the second pass pulled the full surrounding paragraph for each
  candidate sentence, plus the complete list of all twenty device headings on the page, to
  confirm each stands alone grammatically and to check whether any other device on the page
  also had an equally clean single defining sentence -- only these two did; the article's
  other eighteen devices each lean on a following example, a comparison to a neighboring
  device, or multiple clauses rather than one clean single-fact sentence). Only two rows are
  shipped, an intentionally smaller table than most siblings in this directory, since only
  two of the page's twenty devices carry a genuinely standalone defining sentence -- the
  honest-abstention discipline applies to table SIZE as much as to individual queries; no
  padding with weaker rows. Honest abstention on "simile" (a real figure of speech the same
  page also covers, but already grounded as its own separately-shipped library in this
  directory, `simile-meaning.adj`, so tabling it again here under a different predicate would
  duplicate coverage rather than add to it). New manifest objective
  `adj.literacy.k2.sound_device_type` (band K-2, `recall` competency, `ccss.ela` coverage
  root). New e2e test `facts_sounddevicetype_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an untabled sound device).
- `environment/ecosystem-factor-type.adj` (new) -- a new `ecosystem_factor_type(factor,
  description)` table names the two kinds of ecosystem factor and what defines each
  (biotic->a_living_organism_that_shapes_its_environment,
  abiotic->a_non_living_part_of_an_ecosystem_that_shapes_its_environment), quoted verbatim
  from two sibling National Geographic Education resource pages, "Biotic Factors" and
  "Abiotic Factors" -- `trust consensus`, the same tier `biome-type.adj` (a sibling library,
  sourced from the same publisher's "The Five Major Types of Biomes" article) already uses.
  Picked using the mandatory full-tree-grep-before-scoping discipline -- `grep -rilE
  "ecosystem_factor_type|biotic|abiotic" code/specs/data/adj-facts-stdlib/` found ZERO hits,
  confirming a completely fresh topic before this file was written. WebFetch-verified before
  writing (twice per page -- the second pass pulled the full surrounding paragraph for each
  candidate sentence, confirming it stands alone grammatically and that the sentence
  following it supplies examples rather than a qualification the definition depends on).
  Unlike most sibling tables in this directory, biotic and abiotic are not two items picked
  out of a longer enumerable list -- together they exhaust the two-way classification these
  sources describe, so there is no third "real but excluded" factor type to name. Honest
  abstention on "producer" instead: a real and commonly taught ecology term, but one that
  names a food-chain ROLE (already grounded under its own predicate in
  `biology/food-chain-roles.adj`), not a biotic/abiotic FACTOR TYPE. New manifest objective
  `adj.science.3to5.ecosystem_factor_type` (band 3-5, `recall` competency, `ngss` coverage
  root). New e2e test `facts_ecosystemfactortype_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an untabled ecology term).
- `language/clause-type.adj` (new) -- a new `clause_type(type, description)` table names the
  two structural kinds of clause and what makes a clause one or the other
  (independent_clause->is_a_clause_that_alone_is_a_complete_sentence,
  dependent_clause->is_a_clause_that_alone_is_not_a_complete_sentence), quoted verbatim from
  Grammarly's "Independent and Dependent Clauses: Rules and Examples" article -- `trust
  consensus`, the same tier `determiner-type.adj`/`noun-type.adj`/`verb-type.adj`/
  `pronoun-type.adj`/`conjunction-type.adj` (the other libraries in this directory) already
  use for their Grammarly citations. Picked using the mandatory full-tree-grep-before-scoping
  discipline -- `grep -riE "clause_type|clause-type"` across `adj-facts-stdlib/` found no
  existing table for this predicate, only prose mentions of "clause" in sibling files.
  WebFetch-verified twice -- the second pass pulled the first three occurrences of
  "independent clause" and "dependent clause" from the top of the article, in order, to
  confirm the two clean, parallel, single-fact sentences tabled here (not the surrounding
  elaboration, which bundles in extra facts like "an independent clause ... is a simple
  sentence") are the article's own defining pair. Unlike most sibling tables in this
  directory, independent/dependent is a genuinely EXHAUSTIVE split, not an arbitrary subset
  of a longer list -- the article's own opening line states "every clause is either one or
  the other." Honest abstention on "noun_clause" instead: a real, well-documented clause
  category (Grammarly has its own dedicated guide to noun clauses), but one that names a
  FUNCTIONAL role a dependent clause can play, not a third structural type alongside these
  two. New manifest objective `adj.literacy.k2.clause_type` (band K-2, `recall` competency,
  `ccss.ela` coverage root). New e2e test `facts_clausetype_e2e.rs` (3 tests: direct recall,
  reverse binding, honest abstention on an untabled clause category).
- `geology/fossil-preservation-type.adj` (new) -- a new `fossil_preservation_type(type,
  description)` table names three preservation STRUCTURES a fossil can be found as
  (mold->three_dimensional_impression_of_all_or_part_of_a_body_fossil_or_trace_fossil,
  cast->replica_of_an_organism_or_a_trace_produced_by_the_infilling_of_a_natural_mold,
  trace_fossil->consists_of_the_evidence_of_living_organisms_but_not_the_actual_organism_itself),
  quoted verbatim from the National Park Service's "Mold Casts and Steinkerns" article --
  `trust authoritative`, the same tier `volcano-type.adj` (a sibling library in this directory,
  USGS-sourced) already uses. Distinct from the sibling `fossil-formation-type.adj`, which
  names three FORMATION MECHANISMS from a different source (Ducksters, consensus) and
  deliberately bundles mold and cast into one coarse `cast_or_mold` row since its own source
  only gives that pairing a single combined sentence -- this table instead refines that
  pairing using a more detailed primary source that gives mold and cast their own separate
  defining sentences; the two tables answer different questions (how did it form? vs. what
  shape is it?) and neither supersedes the other. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -riE "mold_fossil|cast_fossil|trace_fossil|
  fossil_type"` across `adj-facts-stdlib/` found zero hits. WebFetch-verified twice -- the
  second pass pulled the page's glossary-vs-prose structure, confirming `mold`/`cast` come
  from the formal glossary and `trace_fossil` from the introduction, and that none of the
  three quoted sentences is truncated or bundles in an example the way a rejected row would.
  Honest abstention on `steinkern`: a real, well-documented term the same page defines with
  its own clean sentence, but the page itself frames a steinkern as a specific KIND of cast
  (an internal cast), not a fourth preservation type alongside these three. New manifest
  objective `adj.science.3to5.fossil_preservation_type` (band 3-5, `recall` competency,
  `ngss` coverage root). New e2e test `facts_fossilpreservationtype_e2e.rs` (3 tests: direct
  recall, reverse binding, honest abstention on a real-but-subordinate term).
- `language/point-of-view.adj` (new) -- a new `point_of_view(type, description)` table names
  three narrative PERSPECTIVES a story can be told from
  (first_person->the_reader_accesses_the_story_through_one_person,
  second_person->uses_the_pronoun_you,
  third_person->the_narrator_has_the_ability_to_know_everything), quoted verbatim from
  Grammarly's "What Is Point of View in Writing, and How Does It Work?" article -- `trust
  consensus`, the same tier `pronoun-type.adj`/`sentence-type.adj`/`part-of-speech.adj` (the
  other libraries in this directory) already use for their Grammarly citations. Point of view
  is a LITERARY DEVICE, distinct from the sibling `pronoun-type.adj`, which tables
  grammatical pronoun CATEGORIES (personal/indefinite/interrogative) rather than narrative
  perspective -- the two libraries answer different questions and neither overlaps the
  other. Picked using the mandatory full-tree-grep-before-scoping discipline --
  `grep -riE "point_of_view|first_person|third_person|narrator"` across `adj-facts-stdlib/`
  found zero table-level hits, only an unrelated prose mention of "narrator" in
  `fable-moral.adj`. WebFetch-verified twice -- the second pass pulled the full opening
  paragraph for `first_person` and `third_person`, confirming each quoted sentence is the
  article's own first, complete, standalone defining sentence (the elaboration that follows
  each one is a separate sentence, not folded into the quoted definition). Honest abstention
  on `third_person_omniscient`: a real, well-documented term the same article defines with
  its own clean sentence, but the article itself frames it as a SUBTYPE of third person
  (alongside third_person_limited and third_person_objective), not a fourth peer point of
  view. New manifest objective `adj.literacy.k2.point_of_view` (band K-2, `recall`
  competency, `ccss.ela` coverage root). New e2e test `facts_pointofview_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention on an untabled subtype).
- `astronomy/lunar-eclipse-type.adj` (new) -- a new `lunar_eclipse_type(type, description)`
  table names three named lunar eclipse types and what each actually is
  (total_lunar_eclipse->the_moon_moves_into_the_inner_part_of_earths_shadow_the_umbra,
  partial_lunar_eclipse->an_imperfect_alignment_of_sun_earth_and_moon_results_in_partial_umbra_passage,
  penumbral_eclipse->the_moon_travels_through_earths_penumbra_the_faint_outer_part_of_its_shadow),
  quoted verbatim from NASA's "Eclipses and the Moon" page -- `trust authoritative`, the same
  tier the sibling `solar-eclipse-type.adj` already uses for its NASA citation. Picked using
  the mandatory full-tree-grep-before-scoping discipline -- zero hits for
  `lunar_eclipse|blood_moon` before writing. WebFetch-verified twice -- the second pass
  pulled the full paragraph under each heading, confirming each quoted sentence is the
  page's own first, complete, standalone defining sentence, with a separate elaboration
  sentence following each one (not folded into the quote). This cycle also ruled out two
  other candidates for bundling 3+ facts per sentence rather than one (spring/neap tides,
  nebula types) -- a discipline reinforcement: a source's sentence must state exactly ONE
  fact to earn a row here, no matter how authoritative the source. Honest abstention on
  `blood_moon`: a real term the SAME page discusses under its own heading, but as a
  nickname for the reddish color a total lunar eclipse produces, not a fourth peer eclipse
  type. New manifest objective `adj.science.3to5.lunar_eclipse_type` (band 3-5, `recall`
  competency, `ngss` coverage root). New e2e test `facts_lunareclipsetype_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention on an untabled nickname).
- `language/comma-rule.adj` (new) -- a new `comma_rule(rule, description)` table names three
  comma rules and what each actually says to do
  (comma_in_a_series->use_commas_to_separate_elements_in_a_list_of_more_than_two_elements,
  comma_before_but->use_a_comma_before_but_when_it_is_joining_two_independent_clauses,
  comma_with_direct_address->set_off_the_name_with_commas_when_addressing_another_person_by_name),
  quoted verbatim from Grammarly's "Rules for Using Commas, With Examples" article -- `trust
  consensus`, the same tier the sibling `end-punctuation-mark.adj`/`capitalization-rule.adj`
  already use for their Grammarly citations. This table is the natural complement to
  `end-punctuation-mark.adj`, which explicitly named comma as a real mark it deliberately
  excludes because it belongs to a different category (a mid-sentence pause mark, not an
  end-of-sentence mark) -- this table now grounds that mid-sentence category on its own
  terms. Picked using the mandatory full-tree-grep-before-scoping discipline -- zero hits
  for `comma_rule|oxford_comma|direct_address` before writing. WebFetch-verified twice --
  the second pass pulled the full paragraph under each of the three chosen headings,
  confirming each quoted sentence is the article's own first, complete, standalone rule
  sentence, with worked examples following it rather than folded into the quote. Honest
  abstention on `oxford_comma`: a real, well-known term the same page discusses under its
  own heading, but its own rule sentence bundles the placement rule together with a caveat
  about its optionality rather than stating one clean single fact. New manifest objective
  `adj.literacy.k2.comma_rule` (band K-2, `recall` competency, `ccss.ela` coverage root).
  New e2e test `facts_commarule_e2e.rs` (3 tests: direct recall, reverse binding, honest
  abstention on an untabled term).
- `astronomy/sun-layer.adj` (new) -- a new `sun_layer(layer, description)` table names two
  layers of the Sun and what each actually is
  (photosphere->the_visible_surface_of_the_sun, corona->the_suns_outer_atmosphere), quoted
  verbatim from NASA's "Layers of the Sun" blog post (The Sun Spot) -- `trust authoritative`,
  the same tier the sibling `celestial-objects.adj`/`comet-part.adj`/`space-rock-stage.adj`/
  `lunar-eclipse-type.adj` already use for their NASA citations. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- zero hits for `sun_layer` before writing (a
  stray `corona` hit in `anatomy/heart-valves.adj` was confirmed to be "coronary sinus", an
  unrelated anatomical structure, not a false conflict). WebFetch-verified twice -- the
  second pass pulled the full surrounding paragraph for every layer named on the page,
  confirming `photosphere` and `corona` are the only two whose own sentence states a single
  clean fact. Only two rows are shipped, an intentionally smaller table than most siblings
  in this directory, since core/radiative-zone/convection-zone/chromosphere each bundle
  location together with process or temperature details in one grammatically unified
  sentence rather than stating one clean fact -- the honest-abstention discipline applies to
  table SIZE as much as to individual queries, reinforcing the "reject bundled-fact
  sentences" lesson from earlier this session. Honest abstention on `chromosphere`: a real
  solar layer the same page names, but its sentence bundles position together with a
  temperature range as one relative clause rather than one clean fact. New manifest
  objective `adj.science.3to5.sun_layer` (band 3-5, `recall` competency, `ngss` coverage
  root). New e2e test `facts_sunlayer_e2e.rs` (3 tests: direct recall, reverse binding,
  honest abstention on a real-but-bundled layer).
- `language/adverb-type.adj` (new) -- a new `adverb_type(type, description)` table names
  four adverb types and what each actually describes (manner->describes_how_an_action_is_
  performed, place->describes_where_an_action_happens, frequency->describes_how_often_an_
  action_occurs, duration->describes_how_long_an_action_lasts), quoted verbatim from
  Grammarly's "What Is an Adverb? Definition and Examples" article's "Types of adverbs"
  table -- `trust consensus`, the same tier already used by the sibling `verb-type.adj`/
  `sentence-type.adj`/`part-of-speech.adj`/`noun-type.adj`/`preposition-type.adj`. Closes
  out adverbs as the last major part-of-speech family this stdlib had not yet named on its
  own. Picked using the mandatory full-tree-grep-before-scoping discipline -- zero hits for
  `adverb_type` before writing. WebFetch-verified twice -- the second pass pulled every row
  of the source's "Types of adverbs" table, confirming manner/place/frequency/duration are
  each stated as their own clean, single-fact sentence. Honest abstention on `time`: the
  SAME table names a fifth adverb type, but its own defining sentence -- "Adverbs of time
  describe when, how long, or how often something happens" -- bundles three distinct facts
  into one sentence rather than stating a single clean fact, the same "reject bundled
  facts" discipline reinforced across recent slices (fossil-preservation-type, lunar-
  eclipse-type, comma-rule, sun-layer). New manifest objective `adj.literacy.k2.
  adverb_type` (band K-2, `recall` competency, `ccss.ela` coverage root, matching the
  sibling `*_type` part-of-speech objectives' band convention). New e2e test
  `facts_adverbtype_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on
  a real-but-bundled type).
- `earth-science/soil-texture-class.adj` (new) -- a new `soil_texture_class(class,
  description)` table names the three soil particle-size separates and the diameter range
  that actually defines each (clay->less_than_two_thousandths_of_a_millimeter_in_diameter,
  silt->between_two_thousandths_and_five_hundredths_of_a_millimeter,
  sand->larger_than_five_hundredths_of_a_millimeter_in_diameter), quoted verbatim from
  Wikipedia's "Soil texture" article -- `trust consensus`, the same tier this stdlib
  already reserves for other Wikipedia citations. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- zero hits for `soil_texture_class`/
  `sand`/`silt`/`clay` before writing; distinct from the sibling `soil-horizons.adj`
  (a completely different axis -- vertical layers a soil pit exposes, not the
  particle-size classes that make up any one of those layers). Description atoms spell
  the decimal millimeter figures out in words rather than embedding a literal decimal
  point -- confirmed empirically that ADJ atoms cannot contain a `.` (a
  `less_than_0.002_millimeters` atom fails to parse) -- while the exact verbatim figures
  stay independently checkable in each row's quoted `source` span. Honest abstention on
  `loam`: a real, extremely common soil-texture term, but a composite class made of
  sand/silt/clay mixed together rather than one of the three particle-size separates
  itself. New manifest objective `adj.science.3to5.soil_texture_class` (band 3-5,
  `recall` competency, `ngss` coverage root). New e2e test
  `facts_soiltextureclass_e2e.rs` (3 tests: direct recall, reverse binding, honest
  abstention on a real-but-different-axis term).
- `language/prefix-meaning.adj` (extended) -- adds three more prefixes
  (non_->not_or_negation, pre_->happening_before, inter_->among_between) to the
  already-shipped three-row table (un_/re_/dis_), all from the SAME already-cited
  Grammarly "Prefixes: Definition and Examples" page, which turns out to define
  roughly seventy prefixes as clean, single-fact short phrases in its own table --
  only three had been used so far. Picked for genuinely distinct semantic categories
  rather than near-synonyms of the negation family already covered: `non_` (negation,
  distinct word from un_/dis_), `pre_` (temporal -- before), `inter_` (relational --
  among/between). WebFetch-verified twice. This cycle also researched and abandoned a
  `homograph` candidate -- every source tried (7ESL, general web summaries) bundles
  both meanings of a homograph word into one comparative sentence rather than stating
  one clean meaning per sentence -- and an `interjection-type` candidate, dropped
  after a secondary source's own "cognitive interjection" definition muddled with its
  "emotive" category, an editorial-quality red flag. `over_` remains the abstention
  target (a real prefix the same source page also covers, but still deliberately not
  a row). Extended e2e test `facts_prefixmeaning_e2e.rs` (now 5 tests: direct recall
  and reverse binding for both an original row and a newly added row, honest
  abstention).
- `earth-science/cloud-types.adj` (extended) -- adds three more clouds
  (cirrostratus->high, cirrocumulus->high, altocumulus->middle) to the
  already-shipped four-row table (cirrus/altostratus/stratus/cumulus), all drawn
  directly from this table's OWN already-quoted `source` sentence -- "The three
  main types of high clouds are cirrus, cirrostratus, and cirrocumulus. The two
  main type of mid-level clouds are altostratus and altocumulus..." -- which had
  already named all seven clouds even though only four were tabled, so extending
  from four rows to seven required no new WebFetch, just reading the span already
  captured in the file. Distinct from the sibling `meteorology/cloud-type.adj`,
  which answers a completely different question (what WEATHER does a cloud's
  presence indicate?) from the SAME NWS page's separate weather-indication
  sentences, not the altitude-deck sentence this table uses -- the two tables do
  not overlap even though `altocumulus` happens to be named (as an abstention
  target, for an unrelated reason) in both files. Extended e2e test
  `facts_clouds_e2e.rs` (now 2 tests: the original altitude-recall test plus a
  new test binding both newly added rows).
- `language/noun-type.adj` (extended) -- adds three more noun types
  (concrete_noun->perceived_by_the_senses_physical_or_tangible,
  countable_noun->can_be_counted, uncountable_noun->impossible_to_count) to the
  already-shipped three-row table (common_noun/collective_noun/abstract_noun),
  all from the SAME already-cited Grammarly "Nouns: Definition and Examples"
  page -- this file's own header already noted the source gives clean
  single-sentence definitions for common, proper, concrete, abstract,
  collective, singular, plural, countable, uncountable, and gerund nouns, but
  only three had been turned into rows. `concrete_noun` deliberately pairs
  with the already-shipped `abstract_noun` (perceived by the senses vs. not),
  and `countable_noun`/`uncountable_noun` form their own natural pair.
  WebFetch-verified before adding. Three OTHER candidates from the same page
  were deliberately excluded for bundling two distinct facts into one
  sentence rather than stating a single clean fact: `proper_noun` ("...is a
  specific name of a person, place, or thing AND is always capitalized" --
  naming function + a separate capitalization rule), `singular_noun`/
  `plural_noun` ("...refers to one/more than one person, place, thing, or
  idea AND requires a singular/plural verb" -- referent + grammatical
  agreement rule), and `gerund` ("...a verb form that ends in -ing AND
  functions as a noun in a sentence" -- morphological fact + syntactic-
  function fact). Extended e2e test `facts_nountype_e2e.rs` (now 6 tests:
  direct recall and reverse binding for both an original row and a newly
  added row, honest abstention on the pre-existing untabled term, and honest
  abstention on a newly-identified bundled-fact candidate).
- `biology/pond-zone.adj` (new) -- a new `pond_zone(zone, description)` table
  names three zones of a freshwater lake or pond and what each actually is
  (littoral_zone->close_to_the_shore,
  limnetic_zone->open_and_well_lit_area_of_a_freestanding_body_of_fresh_water,
  profundal_zone->deep_zone_located_below_the_range_of_effective_light_penetration),
  quoted verbatim from three separate Wikipedia articles ("Littoral zone",
  "Limnetic zone", "Profundal zone"), each article's own opening sentence --
  `trust consensus`, the same tier this stdlib already reserves for other
  Wikipedia citations (e.g. `soil-texture-class.adj`). Distinct from the
  already-shipped `oceanography/ocean-zones.adj`, which names three OCEAN
  depth zones (sunlight/twilight/midnight, ordered by how far sunlight
  reaches through open ocean water) -- a completely different body of water
  and organizing question. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- zero hits for
  `littoral`/`limnetic`/`profundal`/`pond_zone` before writing. Honest
  abstention on `benthic_zone`: a real freshwater-zone term, but not one of
  the three tabled here. New manifest objective `adj.science.6to8.pond_zone`
  (band 6-8, `recall` competency, `ngss` coverage root). New e2e test
  `facts_pondzone_e2e.rs` (3 tests: direct recall, reverse binding, honest
  abstention).
- `language/part-of-speech.adj` (extended) -- extended the existing
  `part_of_speech(word, category)` table from 3 to 5 rows, adding
  quietly->adverb and against->preposition, using the SAME already-cited
  Grammarly "The 8 Parts of Speech" article (which covers all 8 parts of
  speech; only 3 were originally tabled). Both new rows are the article's
  own clean, standalone, single-fact example sentences ("I entered the
  room quietly." / "I left my bike leaning against the garage."),
  WebFetch-verified against the live page including the surrounding
  sentences to confirm neither is folded into a longer bundled passage.
  Checked and rejected the remaining three parts of speech on the same
  page as extension candidates because none has one clean standalone
  sentence: pronoun (bundles two separate quoted sentences with framing),
  conjunction (one sentence covers two conjunctions -- "and" and "but" --
  bundled together), interjection (each example is bundled with its own
  punctuation demonstration). Extended `facts_partofspeech_e2e.rs` to 5
  tests (added direct recall and reverse binding for the two newly added
  rows). No manifest change (same library, no new objective).
- `biology/seed-dispersal-mechanism.adj` (new) -- a new
  `seed_dispersal_mechanism(mechanism, description)` table names four ways
  a plant disperses its seeds and how each actually works
  (barochory->uses_gravity_as_a_simple_means_of_seed_dispersal,
  ballochory->seed_is_forcefully_ejected_by_explosive_dehiscence_of_the_fruit,
  anemochory->seeds_float_on_the_breeze_or_flutter_to_the_ground,
  epizoochory->transported_on_the_outside_of_vertebrate_animals), each
  quoted verbatim from its own subsection of Wikipedia's "Seed dispersal"
  article -- `trust consensus`, a MULTI-SOURCE-STYLE table (see
  `ocean-current-drivers.adj`). Picked after exhausting the
  "extend an existing table" pattern across 12 not-yet-checked science
  tables this window (animal-habitat.adj, plant-need.adj,
  ecosystem-factor-type.adj, fossil-formation-type.adj,
  fossil-preservation-type.adj, biome-type.adj, animal-adaptation.adj,
  animal-survival-adaptation.adj, plant-life-cycle.adj, frog-life-cycle.adj,
  ocean-current-drivers.adj, metamorphism-cause.adj -- none extendable),
  then researching seed-dispersal as a fresh topic and finding that three
  additional non-Wikipedia sources (NPS, which 404'd; USDA Forest Service
  research papers; a kids'-science page) all failed the clean-single-fact-
  sentence bar before Wikipedia's own per-mechanism subsections succeeded.
  Honest abstention on `hydrochory` (water dispersal -- every candidate
  sentence checked either conflates the mechanism with dispersal distance
  or is qualified by a following sentence) and `endozoochory` (ingestion
  dispersal -- its defining sentence bundles the definition together with
  a separate empirical claim about tree-species prevalence). New manifest
  objective `adj.science.6to8.seed_dispersal_mechanism` (band 6-8, `recall`
  competency, `ngss` coverage root; 159 objectives total, up from 158). New
  e2e test `facts_seeddispersalmechanism_e2e.rs` (3 tests: direct recall,
  reverse binding, honest abstention).
- `language/verb-type.adj` (extended) -- extended the existing
  `verb_type(type, description)` table from 3 to 4 rows, adding
  stative_verb->describes_a_subjects_state_or_feeling, using the SAME
  already-cited Grammarly "Verbs: Definition and Examples" article. The
  new row is the article's own clean, standalone, single-fact defining
  sentence ("Stative verbs describe a subject's state or feeling..."),
  WebFetch-verified against the live page. Checked and rejected two other
  verb categories on the same page as extension candidates because
  neither has one clean standalone sentence: modal auxiliary verb
  (bundles the definition with a second sentence about not being the
  main verb), phrasal verb (definition bundled with its mechanism).
  Extended `facts_verbtype_e2e.rs` to 5 tests (added direct recall and
  reverse binding for the newly added row). No manifest change (same
  library, no new objective).
- `biology/symbiosis-type.adj` (new) -- a new `symbiosis_type(type,
  description)` table names three types of symbiotic relationship and
  what actually defines each (mutualism->both_parties_benefit,
  commensalism->one_organism_benefits_and_the_other_is_not_significantly_harmed_or_helped,
  parasitism->the_parasite_benefits_while_the_host_is_harmed), each quoted
  verbatim from its own standalone sentence in Wikipedia's "Symbiosis"
  article -- `trust consensus`, a MULTI-SOURCE-STYLE table (see
  `ocean-current-drivers.adj`, `seed-dispersal-mechanism.adj`). Picked
  after checking 9 not-yet-reviewed science tables across
  chemistry/physics/geology/geography/anatomy/meteorology this window
  (mixture-types.adj, reaction-types.adj, element-categories.adj,
  friction-types.adj, precipitation-types.adj, joint-types.adj,
  acids-bases.adj, gas-laws.adj, forces.adj, separation-methods.adj --
  none extendable, all closed exhaustive classifications), then
  researching symbiosis as a fresh topic. Honest abstention on
  `amensalism`: a real interaction category the same article's opening
  paragraph also names, but its own defining sentence bundles it together
  with `competition` in one semicolon-joined compound sentence rather
  than stating one clean fact each the way mutualism/commensalism/
  parasitism do. Picked using the mandatory full-tree-grep-before-scoping
  discipline -- zero hits for
  "symbiosis_type"/"mutualism"/"commensalism"/"parasitism" before this
  file was written. New manifest objective
  `adj.science.6to8.symbiosis_type` (band 6-8, `recall` competency,
  `ngss` coverage root; 160 objectives total, up from 159). New e2e test
  `facts_symbiosistype_e2e.rs` (3 tests: direct recall, reverse binding,
  honest abstention).
- `language/author-purpose.adj` (new) -- a new `author_purpose(purpose,
  description)` table names the three classic reasons an author writes
  something (persuade->convince_the_reader_of_the_merits_of_a_particular_point_of_view,
  inform->enlighten_the_readership_about_a_real_world_topic,
  entertain->keep_things_as_interesting_as_possible), each quoted verbatim
  from its own standalone sentence in LiteracyIdeas' "The Author's
  Purpose: Ultimate Guide for Teachers and Students" article -- `trust
  consensus`, a MULTI-SOURCE-STYLE table (see `point-of-view.adj`,
  `comma-rule.adj`, `figurative-language-type.adj`). Picked after checking
  12 not-yet-reviewed literacy tables this window (clause-type.adj,
  comma-rule.adj, point-of-view.adj, sound-device-type.adj,
  figurative-language-type.adj, sentence-type.adj, idiom-meaning.adj,
  simile-meaning.adj, past-tense-ed-sound.adj, plural-s-sound.adj,
  silent-e-word.adj, r-controlled-vowel-word.adj -- none extendable, each
  already documenting in its own header exactly why its rejected
  candidates don't qualify), then researching author's purpose as a fresh
  topic (Grammarly has no dedicated page; pivoted through
  education.com/study.com/twinkl.com, which bundle purpose with genre
  examples in one sentence, before LiteracyIdeas' page succeeded with
  clean parallel single-fact sentences). Honest abstention on `describe`:
  a real fourth purpose the same article also names, but its defining
  sentence is framed as a photograph comparison rather than the same
  parallel "when an author's purpose is to X, they Y" pattern the three
  tabled here share, and the classic PIE mnemonic this table grounds names
  only these three as the canonical peer set. New manifest objective
  `adj.literacy.k2.author_purpose` (band K-2, `recall` competency,
  `ccss.ela` coverage root; 161 objectives total, up from 160). New e2e
  test `facts_authorpurpose_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention).
- `earth-science/seismic-wave-arrival-order.adj` (new) -- a new
  `seismic_wave_arrival_order(wave, description)` table names the two
  named seismic body waves and which one an earthquake sends out first
  (p_wave->are_the_first_waves_to_arrive_after_an_earthquake,
  s_wave->are_the_next_waves_to_arrive_after_p_waves), each quoted
  verbatim from its own standalone, parallel-worded sentence in Cal OES
  (California Governor's Office of Emergency Services) News' "What Are
  P-Waves and S-Waves?" article -- `trust authoritative` (a California
  state government .gov source). Distinct from the sibling
  `physics/wave-types.adj`, which classifies waves into the
  mechanical/electromagnetic FAMILY axis (and explicitly abstains on
  seismic waves itself), not this arrival-order axis. Picked after
  checking several not-yet-reviewed science tables this window
  (galaxy-types.adj, wave-types.adj -- both exhaustive fixed
  classifications from a single source sentence, non-extendable), then
  researching seismic waves as a fresh topic -- USGS bundles P/S wave
  facts together in comparative sentences ("P waves travel through solid
  and liquid, but S waves do not"), MTU and Wikipedia bundle multiple
  facts or use a different framing dimension, before Cal OES News
  succeeded with clean parallel single-fact sentences. Honest abstention
  on surface_wave: a real third seismic wave commonly grouped with these
  two, but every source checked either bundles its definition with a
  second distinct fact or uses a different framing than the
  arrival-order pattern the two tabled here share. New manifest objective
  `adj.science.6to8.seismic_wave_arrival_order` (162 objectives total, up
  from 161). New e2e test `facts_seismicwavearrivalorder_e2e.rs` (3
  tests: direct recall, reverse binding, honest abstention).
- `language/text-structure-type.adj` (new) -- a new `text_structure_type(type,
  description)` table names three ways a nonfiction text organizes its
  information (cause_and_effect->tells_why_something_happened_and_what_happened,
  compare_and_contrast->examines_the_similarities_and_differences_between_two_or_more_things,
  description->describes_a_topic_to_give_the_reader_a_mental_picture), each
  quoted verbatim from its own standalone sentence in Reading Rockets'
  "Teaching Text Structure" article -- `trust consensus`, the same tier
  this stdlib already reserves for other Reading Rockets citations (e.g.
  `word-families.adj`, `vocabulary-in-context.adj`). Picked after checking
  7 not-yet-reviewed literacy tables this window (opposites.adj,
  vowels.adj, word-families.adj, alphabet.adj, greek-alphabet.adj -- none
  extendable, each already exhaustive or deliberately CVC-scoped), then
  researching text structure as a fresh topic. Honest abstention on
  `sequence`: a real text structure the same article also names, but its
  defining sentence joins two distinct functions with "or" ("describes
  items or events in order, OR explains the steps to follow") rather than
  stating one clean fact; also honest abstention on `problem_and_solution`
  for the same reason (its sentence bundles three structural components in
  sequence). New manifest objective `adj.literacy.k2.text_structure_type`
  (163 objectives total, up from 162). New e2e test
  `facts_textstructuretype_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention).
- `geology/igneous-rock-type.adj` (new) -- a new `igneous_rock_type(type,
  description)` table names the two broad types of igneous rock and what
  actually defines each (intrusive->solidifies_within_earth,
  extrusive->erupted_onto_the_surface_or_into_the_atmosphere), each quoted
  verbatim from its own standalone sentence in the U.S. National Park
  Service's "Igneous Rocks" geology page -- `trust authoritative` (a
  U.S. government .gov source). Distinct from the sibling
  `earth-science/rock-types.adj`, which names the three ROCK-CYCLE families
  (igneous/sedimentary/metamorphic) by formation mechanism, not this
  within-igneous split by cooling location. Picked after checking two
  not-yet-reviewed science tables this window (heat-transfer.adj,
  chemical-bonds.adj -- both closed exhaustive classifications,
  non-extendable), then researching igneous rock types as a fresh topic.
  Unlike most sibling tables, intrusive/extrusive is a genuinely
  EXHAUSTIVE two-way split (the source's own opening line states there are
  exactly two broad types), so honest abstention is instead demonstrated
  with `hypabyssal`: a real geological term for shallow-depth cooling, but
  one this source's own two-category framework does not name or pin to a
  defining sentence. New manifest objective
  `adj.science.3to5.igneous_rock_type` (164 objectives total, up from
  163). New e2e test `facts_igneousrocktype_e2e.rs` (3 tests: direct
  recall, reverse binding, honest abstention).
- `language/phoneme-deletion.adj` (new) -- a new `phoneme_deletion(original_word,
  removed_sound, new_word)` table names the one phoneme-deletion demonstration
  (bike->by, removing the last sound /k/) walked through on Reading Rockets'
  "Phonological and Phonemic Awareness: In Practice" module -- the SAME
  already-vetted page `syllable-count.adj` and `phoneme-substitution.adj`
  already cite (a different, "Deleting Sounds" section of it), so this slice
  carries zero new sourcing risk. `trust consensus`. The row composes two
  distinct sentences from that section ("I will change 'bike' to 'by'." +
  "The last sound in 'bike' is /k/.") the same way `phoneme-substitution.adj`'s
  own row already does. Also checked the same page's "suntan"->"sunset"
  demonstration before writing this table: that example substitutes a whole
  SYLLABLE, not a single phoneme, so it is a genuinely different skill and was
  deliberately left out (a future candidate for its own syllable-substitution
  table). New manifest objective `adj.literacy.k2.phoneme_deletion` (165
  objectives total, up from 164). New e2e test `facts_phonemedeletion_e2e.rs`
  (3 tests: direct recall, reverse binding, honest abstention).
- `biology/cell-division-daughter-cells.adj` (new) -- a new numeric-cell
  `cell_division_daughter_cells(process, count)` table names the two
  eukaryotic cell-division processes and how many daughter cells each one
  produces (mitosis->2, meiosis->4), each quoted from a fetched NIH National
  Human Genome Research Institute "Genetics Glossary" page -- `trust
  authoritative`, the same tier `dna-base-pairs.adj`/`anatomy/body-counts.adj`
  already establish for genome.gov. A genuinely NEW library, not an
  extension of the already-shipped `mitosis-phases.adj` family -- meiosis is
  a wholly different biological process, not another phase of mitosis.
  Picked after checking earth-science/geology/meteorology/astronomy tables
  this window (soil-horizons.adj, plate-boundaries.adj, mineral-hardness.adj,
  hurricane-categories.adj, wind-scale.adj, wave-properties.adj,
  spectral-classes.adj, digestive-organs.adj -- all exhaustive fixed
  classifications, non-extendable), then researching mitosis/meiosis as a
  fresh topic. Honest abstention on `binary_fission`: a real cell-division
  process, but the one prokaryotes/bacteria use, not one of the two
  eukaryotic processes this table names. New manifest objective
  `adj.science.6to8.cell_division_daughter_cells` (166 objectives total, up
  from 165). New e2e test `facts_celldivisiondaughtercells_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention).
- `language/syllable-substitution.adj` (new) -- a new
  `syllable_substitution(original_word, new_word, changed_position)` table
  names the one syllable-substitution demonstration (suntan->sunset, second
  syllable) walked through on Reading Rockets' "Phonological and Phonemic
  Awareness: In Practice" module -- the SAME already-vetted page
  `syllable-count.adj`, `phoneme-substitution.adj`, and `phoneme-deletion.adj`
  already cite (its "Substituting Syllables" section), so this slice carries
  zero new sourcing risk. `trust consensus`. The row composes two distinct
  CLEAN PROSE sentences from that section ("I will change 'suntan' to
  'sunset'." + "The second syllable is different.") the same way
  `phoneme-substitution.adj`'s and `phoneme-deletion.adj`'s own rows already
  do -- deliberately NOT drawing from the section's bracketed
  stage-direction text, which names the literal old/new syllable text
  ("tan"/"set") but is instructional stage direction rather than a stated
  fact, so this table's third column is the POSITION the source's own prose
  states, not the syllable text itself. New manifest objective
  `adj.literacy.k2.syllable_substitution` (167 objectives total, up from
  166). New e2e test `facts_syllablesubstitution_e2e.rs` (3 tests: direct
  recall, reverse binding, honest abstention).
- `biology/animal-classes.adj` (extended) -- extended the already-shipped
  `animal_class(animal, class)` table from 8 to 18 rows, adding fox, rabbit,
  bandicoot, quoll, koala (mammal), cassowary, hummingbird (bird), lizard,
  crocodile (reptile), and ray (fish). Every added animal is drawn from
  material ALREADY quoted in the table's own header ("introduced mammals such
  as cats, foxes and rabbits", "marsupials like kangaroos, bandicoots, quolls
  and the Koala", "the Emu and Southern Cassowary", "tiny hummingbirds up to
  huge ostriches", "turtles, lizards, snakes and crocodiles", "sharks and
  rays") -- zero new WebFetch needed, mirroring the "extend an existing
  table" pattern already used for `cloud-types.adj` and `noun-type.adj`.
  Picked after checking plant-tropisms.adj, vertebrate-groups.adj,
  blood-groups.adj, kingdoms.adj, energy-sources.adj, sound-properties.adj,
  em-spectrum.adj, light-colors.adj, flame-colors.adj, and ph-scale.adj this
  window -- none as directly extendable as animal-classes.adj's own
  already-quoted list sentences. `bat` remains the honest-abstention target
  (a real mammal, deliberately excluded as a surprising borderline case for
  beginners). Extended e2e test `facts_animalclasses_e2e.rs` to 2 tests (the
  original + a new extension test covering fox/ray/cassowary/bat). No new
  manifest objective (same library, same objective).
- `language/idiom-meaning.adj` (extended) -- extended the already-shipped
  `idiom_meaning(idiom, meaning)` table from 3 to 23 rows. The source page's
  OWN title states it covers "30 Useful English Idiomatic Expressions" (the
  live page in fact lists 50), so the original 3-row slice was a narrow
  first cut, not the page's own limit. Added cut_corners,
  hit_the_nail_on_the_head, cost_an_arm_and_a_leg,
  bite_off_more_than_you_can_chew, beat_around_the_bush,
  cry_over_spilled_milk, get_your_act_together,
  kill_two_birds_with_one_stone, let_the_cat_out_of_the_bag,
  pull_someones_leg, burn_the_midnight_oil, bite_the_bullet, break_a_leg,
  call_it_a_day, steal_someones_thunder, the_ball_is_in_your_court,
  throw_in_the_towel, speak_of_the_devil, once_in_a_blue_moon, and
  catch_someone_red_handed -- all 20 from the SAME already-cited Oxford
  International English page, zero new source needed, each with its own
  clean one-sentence "Meaning: ..." definition, WebFetch-verified twice
  (a targeted second pass re-fetched five of the new rows' raw text
  directly, confirming the extraction is accurate). Idioms whose page entry
  uses a slash or bracketed variant (e.g. "Hit the sack/hay", "Cut
  [somebody] some slack") were deliberately left out of this batch, since a
  single unquoted atom cannot honestly represent an "either/or" variant
  phrase. Extended e2e test `facts_idiommeaning_e2e.rs` to 4 tests (original
  3 + a new extension test). No new manifest objective (same library, same
  objective, 167 total unchanged).
- `biology/plant-tropisms.adj` (extended) -- extended the already-shipped
  `tropism_stimulus(tropism, stimulus)` table from 5 to 12 rows. The SAME
  already-cited Wikipedia "Tropism" article's own "Types of tropism" list
  names seven MORE tropisms beyond the original five, each with its own
  clean single-fact definition sentence: aerotropism->wind,
  electrotropism->electric_field, heliotropism->sun_direction,
  magnetotropism->magnetic_fields, selenotropism->moon_direction,
  thermotropism->temperature, and traumatotropism->wounding.
  WebFetch-verified twice (a targeted second pass re-fetched all seven new
  terms' raw definition text directly) -- zero new source page needed.
  Honest abstention updated to `inotropism`: a real term on the SAME
  Wikipedia page, but naming a MUSCLE's contraction response to drugs, not
  a plant's growth response to an environmental stimulus -- the wrong
  domain entirely. (The page's lead paragraph also uses "anemotropism" as
  its own naming-convention example for a wind-response, a different name
  for the same wind-response the page's own types list separately calls
  "Aerotropism" -- this table uses the types-list's own canonical name,
  mirroring how it already uses `gravitropism` rather than the page's
  noted synonym `geotropism`.) Extended e2e test `facts_planttropisms_e2e.rs`
  to 2 tests. No new manifest objective (same library, same objective, 167
  total unchanged). This is the THIRD successful extend-pattern win this
  window (after animal-classes.adj +10 rows and idiom-meaning.adj +20
  rows) -- checking whether a table's own already-cited source names more
  items than tabled continues to be the strongest, safest move for both
  lanes.
- `language/simile-meaning.adj` (extended) -- extended the already-shipped
  `simile_meaning(simile, meaning)` table from 3 to 15 rows. The SAME
  already-cited Grammarly "Common simile examples" table names twelve MORE
  similes beyond the original three, each paired with its own clean
  meaning in the SAME table row shape as the original three: added
  like_a_fish_out_of_water, as_fast_as_a_cheetah, like_a_deer_in_headlights,
  as_cool_as_a_cucumber, like_a_kid_in_a_candy_store, as_strong_as_an_ox,
  like_watching_paint_dry, as_cute_as_a_button, like_two_peas_in_a_pod,
  as_flat_as_a_pancake, like_birds_of_a_feather, and as_hungry_as_a_horse.
  Zero new source page -- WebFetch-verified twice (a targeted second pass
  re-fetched five of the new rows' raw table-row text directly). Checked
  vocabulary-in-context.adj first this cycle (the three remaining words on
  its source page -- incubate, predators, migrate -- lack an explicitly
  stated meaning, only illustrative example sentences, confirmed
  non-extendable). Extended e2e test `facts_simile_meaning_e2e.rs` to 4
  tests (original 3 + a new extension test). No new manifest objective
  (same library, same objective, 167 total unchanged). This is the FOURTH
  successful extend-pattern win this window.
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
- `language/opposites.adj` (extended) -- extended the already-shipped
  `opposite(word, opposite)` table from 7 to 21 rows. The header ABOVE
  already quoted the full Wiktionary "Antonyms" line for `hot`, `big`,
  `happy`, and `open` when this table originally shipped -- only the
  FIRST antonym in each of those four lines had ever been turned into a
  row, even though the same already-quoted span names one more (hot:
  chilled), five more (big: little, tiny, minuscule, miniature,
  minute), seven more (happy: blue, depressed, down, miserable, moody,
  morose, unhappy), and one more (open: shut). `fast`, `wet`, and `hard`
  were already fully captured (their sources name only one antonym
  each). This is the EIGHTH successful extend-pattern win in this
  loop's recent run. WebFetch re-verified all seven Wiktionary
  "Antonyms" lines live before writing (confirming each already-quoted
  span is accurate and unchanged, and specifically that `big`'s header
  ellipsis stopped at exactly six total antonyms). Mirrors
  `synonyms.adj`'s single-to-many-valued-per-key extension shape (a
  bound word now recalls multiple opposites) -- the sibling table this
  extension technique was first proven on. Extended the query file and
  e2e test `facts_opposites_e2e.rs` to 2 tests (original recall/abstain
  test + a new extension test covering three of the newly multi-valued
  words). No new manifest objective (same library, same objective, 167
  total unchanged). Also backfilled a pre-existing README.md
  documentation gap: `opposites.adj` had never been added to the
  per-directory documentation table (the THIRD such gap found this
  window, after `kingdoms.adj` and `animal-babies.adj`).
- `physics/wave-types.adj` (extended) -- extended the already-shipped
  `wave_family(wave, family)` table from 7 to 10 rows. The header ABOVE
  already quoted the SAME electromagnetic sentence naming seven
  electromagnetic waves ("gamma rays, X-rays, ultraviolet light,
  visible light, infrared light, microwaves, and radio waves") when
  this table originally shipped -- only four of the seven had ever been
  turned into a row. Added ultraviolet, infrared, and microwave, all
  citing the same already-vetted NASA URL and authoritative trust tier.
  This is the NINTH successful extend-pattern win in this loop's recent
  run, and the first this cycle to apply the header-only zero-WebFetch
  discovery technique (first proven on `synonyms.adj`/`opposites.adj`)
  to adding new KEYS (more waves) rather than more values per an
  existing key. Checked several other science candidates first this
  cycle and ruled them out: `chemistry/lab-equipment.adj`'s header
  explicitly documents its four dropped items (Erlenmeyer flask,
  volumetric flask, pipet, wash bottle) as a deliberate prior exclusion
  rather than an oversight; `biology/seed-parts.adj` is a fully closed
  7-part anatomical set with no unused header material. Extended the
  query file and e2e test `facts_wavetypes_e2e.rs` to 2 tests (original
  recall/reverse/abstain test + a new extension test covering all three
  newly-added electromagnetic waves). No new manifest objective (same
  library, same objective, 167 total unchanged). Also backfilled a
  pre-existing README.md documentation gap: `wave-types.adj` itself had
  no dedicated per-directory documentation row (only mentioned in
  passing from `seismic-wave-arrival-order.adj`'s row) -- the FOURTH
  such gap found this window.

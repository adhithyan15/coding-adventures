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


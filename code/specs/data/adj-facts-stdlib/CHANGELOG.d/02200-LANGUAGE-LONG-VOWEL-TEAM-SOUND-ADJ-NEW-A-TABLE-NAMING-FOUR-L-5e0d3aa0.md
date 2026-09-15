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


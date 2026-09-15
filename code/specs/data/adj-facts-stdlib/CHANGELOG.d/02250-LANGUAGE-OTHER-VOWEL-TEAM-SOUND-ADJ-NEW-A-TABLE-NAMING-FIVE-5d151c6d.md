- `language/other-vowel-team-sound.adj` (new) — a `table` naming five lessons of the
  University of Florida Literacy Institute (UFLI) Foundations Toolbox's "Other Vowel Teams
  Unit Resources (Lessons 89-94)" page and the single sound each spelling represents:
  `other_vowel_team_sound(spelling, sound)`. `u`/`oo` → `short_oo_sound` (lesson 89, the
  repeated short /oo/ heard in "book"); `oo` ALSO carries `long_u_sound` (lesson 90, a
  genuinely different, longer sound heard in "moon") — the same one-key/many-values shape
  `digraph-sound.adj`'s own `th` row established; `ew`/`ui`/`ue` join `oo` on `long_u_sound`
  (lesson 91, the source's own shared `/ū/` notation); `au`/`aw`/`augh` → `aw_sound` (lesson
  93); and lesson 94 gives two short-vowel exceptions, `ea /ĕ/` and `a /ŏ/`. This is the FIFTH
  UFLI phonics unit shipped in this stdlib, the direct sequel to `long-vowel-team-sound.adj`'s
  own Long Vowel Teams unit (84-88) on the same toolbox page family, and it resolves a real,
  already-flagged collision: the lesson-94 "ea" row ships as the disambiguated atom
  `ea_short_e`, NOT a bare `ea`, because the bare spelling "ea" already carries a DIFFERENT,
  genuinely distinct source-cited sound in the sibling `long-vowel-team-sound.adj` library (the
  steady long-E reading in "team"/"rain" there, vs. this table's short-E reading in
  "bread"/"head" here) — the identical `ow`/`ow_long_o`-style disambiguation discipline that
  table's own header already flagged this exact "ea"/short-vowel collision risk as a future-round
  concern for. A bare-`ea` query against THIS table's predicate honestly abstains, while
  `long_vowel_team_sound(ea, $S)` against the sibling table is completely unaffected; both
  empirically verified to coexist without conflict when imported together. This round also
  resolves the converse of two abstentions `long-vowel-team-sound.adj`'s own header already
  documented: "au"/"aw" were deliberately not rows there because UFLI tables them under this
  very unit (lesson 93) — they are genuine rows here instead. Deliberately does NOT
  disambiguate the bare atom `a` despite its reuse as a row value in several other,
  categorically unrelated tables (`alphabet.adj`, `vowels.adj`, `morse-code.adj`,
  `dolch-sight-word-level.adj`, `soil-horizons.adj`, `blood-groups.adj`) — none of them is a
  sibling phonics spelling-to-sound table, so there is no genuine same-kind collision risk to
  guard against. Excludes lesson 92 ("Vowel Teams Review 2"), a cumulative review lesson, the
  same review-lesson exclusion already established for this stdlib. Honest abstention on `ey`
  (UFLI tables it under the separate Long Vowel Teams unit, lesson 85, not this cited page).
  curl-fetched directly and confirmed byte-for-byte before writing this file. `trust
  authoritative` — same tier and source family as `digraph-sound.adj`/`diphthong-sound.adj`/
  `long-vowel-team-sound.adj`/`silent-letter-sound.adj`. New `other-vowel-team-sound.query.adj`
  and `facts_othervowelteamsound_e2e.rs` (6 tests: forward recall + citation check, reverse
  recall binding all four `long_u_sound` spellings, the `oo` one-key-two-sounds shape, the
  `ea`/`ea_short_e` cross-table heteronym proof, a direct forward recall on the disambiguated
  atom, and honest abstention on `ey`); new manifest objective
  `adj.literacy.k2.other_vowel_team_sound`.


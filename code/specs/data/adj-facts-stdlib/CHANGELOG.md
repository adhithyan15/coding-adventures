# Changelog

This directory is spec/content data, not a compiled package — entries record what content
landed and why, not a semver-tracked API.

## Unreleased

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

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

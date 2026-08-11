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

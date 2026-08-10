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

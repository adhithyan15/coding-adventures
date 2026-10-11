### Fixed — forty-five later-chapter lessons now name the earlier lessons they build on

The gap report's `laterChapterLessonsWithoutPrerequisites` falls from **50 to
5**. A lesson past chapter 1 with `prerequisites: []` is a root of the
prerequisite graph. language-ladder unlocks a root, and every lesson chained
after it, on a fresh profile, and the closed-world validator credits a root
with knowing nothing. All 50 were read, and 45 now name what they build on.
Only the lesson frontmatter changed. No source file changed, and the book,
narration and modality outputs move only their source hashes.

Each prerequisite is the lesson that introduces what the body uses, and is
earlier in both `sequence` and curriculum path order:

- **38 reuse or assume earlier material.** Some reuse a word or phrase, such as
  ES-C412-gimnasio's *está abierto* or IT-C03-io's *sto bene — e tu?*. Some use
  an ending or a letter, such as HI-C42-come's "the endings you already know"
  and AR-C02-ism's "letters from Chapter 1". Some open with a warm-up such as
  "You can introduce yourself". Some say "a word you already know", which
  points at the previous chapter's noun review. Others are anchor-word lessons
  whose warm-up retrieves "the last thing you learned"; 155 of their 159
  siblings already name the lesson before them. Spanish chapter 267's
  negation lessons now chain to ES-C06-hablo-espanol, which first taught *no*
  before the verb. TA-C42-come, TE-C43-come and TE-C44-look teach a headword
  that chapter 32 already taught, and now name that lesson.
- **7 are linked by their own `reviews_of`.** These are FR-C02-je,
  HI-C02-naam, IT-C06-numeri-1-5, KA-C02-hesaru, ML-C02-peru, TA-C02-peyar
  and TE-C02-peru. A lesson cannot resurface one the learner has not reached.
  29,069 of the 29,355 lessons that declare both fields already keep
  `reviews_of` inside their prerequisite closure.

The 5 left are honest roots. GU-C02-naam, SA-C02-nama and LA-C33-vesper are
self-contained word lessons: each teaches one word and its etymology, uses no
earlier lesson, and reviews none. KA-S111-vowel-sign-e and TA-S101-letter-pa
are script-recognition run heads. The words they say "you already say" come
later in curriculum.json, so naming them would fail
`curriculum-prerequisite-order`. Their only earlier words are distractors in a
find-the-letter drill, and naming those would be decoration. No frontmatter
field marks a root as intended, so they stay listed.

`tests/integration.test.ts` gains a ceiling of 5 that may only fall, with the
listing as its failure message. Every other gap-report number is unchanged.
That includes durations, chapter gates and payoffs, forward references
(1,173), forward prerequisites and reviews (0), script closure (0), the glyph
budget, and the drivable counts (185 = 178 by type + 7 by content).

### Changed - one verb per chapter, slice 3: chapter 62 (HL-C99)

- Split chapter 62 -- `traer`, `conseguir`, `jugar`, `conocer` -- into six:
  one verb each, a review chapter, and a synthesis chapter. **No Spanish
  chapter now teaches four verbs.**
- The review chapter closes the stem-change system. e->ie and o->ue were
  already held; `conseguir` adds e->i and `jugar` adds u->ue. There are
  **four patterns in total, and u->ue has exactly one member in the entire
  language** -- which is a single fact wearing a pattern's shape, and easier
  once said out loud.
- The synthesis chapter collects the three pairs where English offers one word
  and Spanish forces a choice: *preguntar*/*pedir*, *traer*/*llevar*,
  *conocer*/*saber*. They were taught chapters apart and had never been placed
  side by side. The decision, not the word, is the work.
- Fix a `concept_tag` that would have corrupted the verb-coverage report:
  `ES-SYNTHESIS-VERB-SPLITS` matches `verbs.ts`'s `/(^|-)VERB-/` namespace
  test, so a synthesis lesson was being counted as a Spanish verb named
  *splits*. Renamed `ES-SYNTHESIS-PAIR-CHOICES`; audited every review and
  synthesis tag added in this arc, and this was the only one.
- Remove three self-references ("this course", "the course") caught by
  `standalone-book`. One curriculum derives N books; no derived book may claim
  to be the course.
- Spanish 64 -> **69 chapters**; old 63-64 -> 68-69. Fully drivable chapters
  **346 -> 351**.


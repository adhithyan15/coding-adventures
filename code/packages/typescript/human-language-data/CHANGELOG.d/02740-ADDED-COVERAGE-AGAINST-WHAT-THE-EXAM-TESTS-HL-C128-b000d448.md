### Added - coverage against what the exam tests (HL-C128)
- Add `core/exam-inventory-es-a1.json`, `src/exam-inventory.ts` and
  `tests/exam-inventory.test.ts`: the first measurement in this package that
  can FALL, and the first that does not rise merely because a lesson was added.
- Every other number here walks our own lessons, so all of them improve when
  the corpus grows -- including growth on something no examiner asks about. This
  one resolves the corpus against an external, finite list: the A1 grammar an
  examiner may expect, restated in our own words from the structure of the Plan
  Curricular del Instituto Cervantes.
- The mapping is an **executable probe**, not an annotation. A `coveredBy:`
  field filled in once is a claim about the corpus frozen at a moment in time,
  and it goes stale silently and flatteringly. `probe: ["ES-GRAMMAR-NOUN-GENDER"]`
  is recomputed every run: retire the atom and coverage falls.
- `probe: null` means UNCOVERED, never "skip". Excluding unmapped points from
  the denominator would let the percentage be improved by deleting a mapping --
  the one edit that changes nothing about what a reader knows.
- **Spanish A1: 53 of 85 points, 62%**, after 220 chapters that had climbed to a
  B2 node. Missing entirely: the demonstratives (3 of 3 points), `muy`, the
  `al`/`del` contractions, the gerund, `quien`, the personal `a`.
- The gate was verified adversarially rather than assumed: an empty probe --
  the one malformed shape that scores as covered -- throws at load, and deleting
  a point fails the pin.


### Added — family and neighbour equivalents panels (HL41)

Book lessons can now carry a small **In the family, and next door** panel. It gives the headword's equivalents in the track's sister languages and in one neighbour language, each in its own script with romanization, and ends with the English word.

- **Comparison sets.** `core/comparison-sets.json` names each track's family languages and neighbour. For example, Tamil compares with Kannada, Telugu and Malayalam, plus Hindi as its neighbour.
- **Data.** Each panel lives in its own owner file, `<track>/equivalents.d/<LESSON-ID>.json`, read through the guarded shard reader.
- **Validation.** `validateEquivalents` is part of `runValidate`. It refuses:
  - a missing or non-vocabulary lesson, or two files for one lesson;
  - a language outside the track's set, or languages out of set order;
  - a form outside its language's Unicode script, a paradigm given where one form belongs, or a form longer than four words;
  - a non-Latin form without romanization;
  - an empty `english` or `source`;
  - a `|` in any panel text, which would split the table row;
  - a malformed owner file (not an object, `equivalents` not a list), which is reported rather than thrown.
- **Same root.** `sameRoot` is optional. The panel marks only `true`, so a pair nobody has judged is never shown as related.
- **Robust book build.** Book generation does not run the validator, so it skips a malformed entry rather than crashing on it.
- **Book only.** `withEquivalentsPanels` appends the panel to the book's copy of the lesson, after its first teaching block, as the filmstrips are added. Narration, the app, duration estimates and the lesson's `sourceHash` all keep the authored lesson.
- **Tests.** `tests/equivalents.test.ts` (17 tests) pins the validator, the panel's shape, and that only the book's copy of a lesson changes.

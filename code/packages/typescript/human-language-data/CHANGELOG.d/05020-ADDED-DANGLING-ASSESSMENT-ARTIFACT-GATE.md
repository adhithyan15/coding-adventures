### Added - dangling assessment-contract references now fail CI

- Add `assessment-artifacts.ts` and `assessment-artifact-cli.ts`: every artifact
  a `<track>/assessment.json` promises — task-shape inventory, timed-mock rubric,
  answer key, for CEFR levels and external capstones alike — must be a file that
  exists. Measured at introduction: **13 of 23 tracks carry a contract, all 13
  dangle, and between them they name 351 distinct artifacts that are not on
  disk** (276 mocks/rubrics/answer keys, 75 task-shape inventories). No `mocks/`
  directory existed anywhere in the repository.
- Pin that known set per track under
  `core/assessment-artifact-ceiling/<track>.json` as a **ceiling**: an unpinned
  dangler fails, and a pinned path that now exists also fails, telling the author
  to lower the pin. The pin is a set of paths rather than a count, so paying one
  debt while taking on another is caught.
- Add `artifact-presence.ts`. `artifactExists` treats only `ENOENT`/`ENOTDIR` as
  absent and throws on every other errno, naming it — `existsSync` reports "not
  there" for "I was not allowed to look", and the difference decides whether a
  gate goes quiet under an I/O fault. `loadAssessmentContracts` and
  `listExternalExamCapstones` now use it.
- Validate CEFR-level `taskInventory`, `additionalComponents.taskInventory`,
  mock `rubric` and `answerKey` with `artifactReference`, as the external-capstone
  half of the same parser already did. These are paths a checker joins to a track
  directory; only one of the two halves was checking their shape.
- Refuse symlinks on both sides, per `shard.ts`'s rule that a guard living only
  inside the reader is a guard the writer forgets. `artifactExists` probes with
  `lstat` and throws on a link, so `mocks/a1/rubric.md -> ../../README.md` cannot
  satisfy the presence gate; `--write` refuses to write through a linked ceiling
  file or a linked ceiling directory, which would otherwise make an ordinary
  `generate:assessment-artifacts` an arbitrary write.
- Validate `track.id` against `TRACK_ID` in `loadAssessmentContracts` **before**
  joining it to a path. `core/languages.json` is an unchecked cast, so an id of
  `../../../../etc` would have been stat'ed and read outside the tree.

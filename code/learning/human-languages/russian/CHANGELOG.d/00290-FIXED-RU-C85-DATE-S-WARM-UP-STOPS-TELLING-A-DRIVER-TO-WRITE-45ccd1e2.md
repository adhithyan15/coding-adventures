## Fixed — RU-C85-date's warm-up stops telling a driver to write

The drivable-writing detector in human-language-data now also reads a writing
verb chained onto an earlier step. RU-C85-date's warm-up said "A date needs an
ordinal: say *первый, второй, третий*, and write the first as a sign writes
it, *1-й*." in bare prose, so the audio edition told a driver to write (issue
#12070). The saying stays prose; the writing is now a `[YOU WRITE: the first
as a sign writes it, *1-й*]` cue, and the lesson stays `drivable: true`.

- Regenerated: book chapter 85, its narration (`.json` and `.txt`), generated
  book and narration hashes, and the `core/lesson-modality` owner (source hash
  only).


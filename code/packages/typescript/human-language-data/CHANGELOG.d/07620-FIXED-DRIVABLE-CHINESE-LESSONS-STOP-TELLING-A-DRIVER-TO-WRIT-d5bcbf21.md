### Fixed — drivable Chinese lessons stop telling a driver to write

- Issue #12070, third pass. `tests/drivable-writing-cues.test.ts` recorded 57
  drivable Chinese lessons (chapters 12-28 and their review lessons) that
  still asked for writing in bare prose: 69 sentences such as "Copy **学**
  once.", "Say **son**, then write 儿 and 子 separately." and the
  "**Write.** 的 — …" line of the four-skill practice blocks. Narration reads
  bare prose unhedged, so the audio edition told a driver to write. Each is
  now a `[YOU WRITE: …]` cue, the form #16893 used for chapters 3-6.
  `tests/drivable-writing-debt/chinese.json` is deleted: the test reads an
  absent file as "this track has no debt".
- Seventeen more writing tasks in the same lessons used an "and write" clause
  ("Say, read, and write each.", "Cover it, wait five seconds, and write
  it."), which the detector deliberately does not match. They are cues too.
- No flagged sentence was a non-writing use, so nothing was reworded instead
  of cued. Judgement calls (the `**Write.**` labels, "Copy" as "one copy of",
  a decision moved ahead of its cue) are listed in the Chinese track
  changelog.
- Regenerated: Chinese book chapters, narration (`.json` and `.txt`), their
  generated book and narration hashes, and the 57 `core/lesson-modality`
  owners. Only the source hash changed in each owner; every lesson is still
  `drivable: true`.

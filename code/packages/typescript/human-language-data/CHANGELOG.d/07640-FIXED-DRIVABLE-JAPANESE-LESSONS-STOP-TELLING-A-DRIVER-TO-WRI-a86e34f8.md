### Fixed — drivable Japanese lessons stop telling a driver to write

- Issue #12070, fifth pass. `tests/drivable-writing-cues.test.ts` recorded
  102 drivable Japanese lessons (book chapters 7-18 and 131-137, with their
  review lessons) that still asked for writing in bare prose: 153 flagged
  spans such as "Write **い**, **ち**, and **と**.",
  "4. **Write:** hear the word, wait ten seconds, and write all five signs.",
  "Copy **あね**, hide it, and write it from the meaning.", "Hear, point,
  say, read, then write **かた**." and "Circle the dakuten." Narration reads
  bare prose unhedged, so the audio edition told a driver to write. Each
  writing task is now a `[YOU WRITE: …]` cue, the form #16893, #16994 and
  #17014 used. `tests/drivable-writing-debt/japanese.json` is deleted: the
  test reads an absent file as "this track has no debt".
- Two of the flagged sentences were circling tasks whose point was a
  question the learner can answer aloud, so they became spoken alternatives
  instead of cues: "Circle the dakuten." (JA-C09-ichido) now asks which mora
  carries it, and "Circle **う**, the second beat of *mō*."
  (JA-C10-mou-sukoshi) now asks the learner to name that sign. One more
  writing step the detector cannot see ("3. **Read/write:** independently
  retrieve only **もうすこし**.", JA-C10-slower-please) is split into a
  reading step and a cue. Judgement calls are listed in the Japanese track
  changelog.
- JA-C01-practice's review pulse had lost its kana to an encoding accident
  when it was authored ("From memory, write ??, ???, ?????, …"). The cue
  now names the six words the block's `assesses` list and the surrounding
  sentence pin down: **はい**, **いいえ**, **こんにちは**, **ありがとう**,
  **日本語** and **コーヒー**. Nine other lessons carry the same `?` damage
  (JA-C01-iie, JA-W01-ko, -n, -wa, JA-W03-sa, -ka, -dakuten,
  JA-W05-mouth-component, -nichi-kanji); none is in this ledger, so they
  are left for their own change.
- Regenerated: Japanese book chapters 7-18 and 131-137, narration (`.json`
  and `.txt`), their generated book and narration hashes, and the 102
  `core/lesson-modality` owners. Only the source hash changed in each owner;
  every lesson is still `drivable: true`.

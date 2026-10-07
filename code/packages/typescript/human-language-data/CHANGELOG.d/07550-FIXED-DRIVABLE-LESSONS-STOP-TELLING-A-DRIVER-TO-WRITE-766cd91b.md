### Fixed — drivable lessons stop telling a driver to write

- Issue #12070. Eleven Chinese lessons in chapters 3–6 that the modality
  manifest marks `drivable: true` opened their warm-up with a bare "Draw the
  shape…" or "Write 是…". The narration export reads bare prose unhedged, so
  the audio edition told a driver to write. Each warm-up now authors the task
  as a `[YOU WRITE: …]` cue, which narration defers ("[once you have stopped
  driving — write: …]") and the book prints as "*Write it:* …". The cue does
  not create a `writing` block, so every lesson stays drivable.
  - The nine the issue named: ZH-C03-yi, ZH-C03-san, ZH-C03-wu, ZH-C04-ri,
    ZH-C04-bu, ZH-C04-practice, ZH-C05-shi, ZH-C05-practice,
    ZH-C06-zaijian.
  - Two more with the same warm-up the sweep found: ZH-C03-er, ZH-C03-si.
  - Where prose followed the old instruction ("Count the strokes…", "Then
    say **hǎo**…"), it is now its own paragraph, so the printed cue is not
    run into the next sentence.
  - ZH-C05-practice: "It **rises**. Write it and it falls." becomes
    "Written down, it falls." It was a statement about the tone mark, but
    read aloud it was an imperative.
- Regenerated: Chinese book chapters 3–6, narration ch03–ch06 (`.json` and
  `.txt`), their generated book and narration hashes, and the eleven
  lessons' `core/lesson-modality` owners (source hash only; every lesson is
  still `voice`, `drivable: true`).
- New test `tests/drivable-writing-cues.test.ts`, with its detector in
  `tests/drivable-writing-imperatives.ts`. It scans every drivable lesson in
  all 23 tracks for an imperative writing verb (write, rewrite, draw, copy,
  circle, underline, sketch, jot) opening a sentence or a "…, then write"
  clause outside a cue. "Outside a cue" is decided by the narration's own
  `splitNarrationCues`. The pattern leaves alone the mentions found in the
  corpus: the verb as a subject ("**Write** is Old English"), italic and
  dash glosses, "Write: **लिहिणे**" vocabulary lines, "copy the sound",
  comma lists of glosses, and recall questions answered in brackets. Each
  exclusion has a fixture control.
- The sweep found the same shape in 402 more drivable lessons across 15
  tracks (marwadi 187, japanese 95, chinese 57, hindi 15, urdu 11, marathi
  8, persian 5, russian 5, spanish 5, bengali 3, french 3, punjabi 3,
  portuguese 2, telugu 2, malayalam 1). Each needs an author's choice
  between a cue and a spoken task, so they are recorded rather than
  rewritten here: `tests/drivable-writing-debt/<track>.json`, one file per
  track. The test requires the findings and the ledger to agree exactly. A
  new offender fails as new debt, and a fixed lesson fails until its entry
  is deleted, so the ledger can only shrink.

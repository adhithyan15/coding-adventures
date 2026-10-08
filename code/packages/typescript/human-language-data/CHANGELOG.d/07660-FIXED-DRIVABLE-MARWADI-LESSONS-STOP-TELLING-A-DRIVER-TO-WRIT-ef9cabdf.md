### Fixed — drivable Marwadi lessons stop telling a driver to write

- Issue #12070, fourth pass. `tests/drivable-writing-cues.test.ts` recorded
  220 drivable Marwadi lessons that still asked for writing in bare prose:
  377 flagged passages such as "With the page covered, write **राम** and
  **सा** separately.", "Look, cover, wait five seconds, and write **पति**.",
  "Say and write market, then recall …" and the "4. **Write:** hear the
  question, wait ten seconds, and write it" line of the four-skill
  checkpoints. Narration reads bare prose unhedged, so the audio edition told
  a driver to write. Each is now a `[YOU WRITE: …]` cue, the form #16893,
  #16994 and #17014 used. `tests/drivable-writing-debt/marwadi.json` is
  deleted: the test reads an absent file as "this track has no debt".
- Writing tasks the detector does not match are cues too: eleven warm-ups that
  asked the learner to "form" signs, and two script-close lessons
  (MW-R36-letters-close, MW-R36-script-close) that set a dictation with no
  verb at all. Eight `[YOU RECALL: write …]` cues in four of these lessons
  became `[YOU WRITE: … from memory]`: RECALL is a spoken action, so the
  narration read them out with no deferral, and the detector, which skips
  every cue, cannot see them. The same RECALL-write shape remains in eleven
  drivable Marwadi lessons outside the ledger and in other tracks.
- No flagged sentence was a non-writing use and no lesson was left in debt.
  Judgement calls (dropping the `**Write:**` label so the item is the cue,
  "the word for …" for meaning cues, "one copy of" for "Copy once", which
  follow-ups go inside a cue and which become their own paragraph) are listed
  in the Marwadi track changelog.
- Regenerated: Marwadi book chapters, narration (`.json` and `.txt`), their
  generated book and narration hashes, and the 220 `core/lesson-modality`
  owners. Only the source hash changed in each owner; every lesson is still
  `drivable: true`.

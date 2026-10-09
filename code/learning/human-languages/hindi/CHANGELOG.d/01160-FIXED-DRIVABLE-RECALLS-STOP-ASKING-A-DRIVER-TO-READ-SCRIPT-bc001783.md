## Fixed — drivable recalls stop asking a driver to read script

`[YOU RECALL: …]` is a spoken cue action, so the narration reads a recall
to a driver as an ordinary turn ("your turn — recall: …") with no deferral.
The spaced script-recognition recalls in this track's drivable lessons held a
reading step inside that spoken cue (`[YOU RECALL: say *X*, then read
**Y**]`), so a driver heard "recall: say X, then read Y" and was asked to read
printed script at speed (issue #12070). Each such cue is now split: the spoken
steps stay in a `[YOU RECALL: …]` cue and each reading step becomes a
`[YOU READ: …]` cue, one bullet each, in the order the author gave. READ is a
manual cue action, so the narration says "once you have stopped driving —
read: Y", and the book prints "*Recall:* say *X*" and "*Read it:* **Y**". The
new reading check in human-language-data (`tests/drivable-writing-cues.test.ts`)
now fails on any drivable recall that asks for printed script to be read
("read **…**", "read the sign **…**", "read a printed …"). Every edited lesson
stays `drivable: true` (only its `core/lesson-modality` source hash changes).

- **Count:** 128 cues in 118 drivable lessons.
- `say *X*, then read **Y**` (64) → `[YOU RECALL: say *X*]` then
  `[YOU READ: **Y**]`; `read **Y**, then say *X*` (41) → the same two cues
  the other way round; `read **Y**` alone (7) → `[YOU READ: **Y**]`.
- `say *X*, then read **Y** and say what it means` (7) → `[YOU RECALL: say
  *X*]` then `[YOU READ: **Y**, then say what it means]`. Judgement call:
  saying what **Y** means needs **Y** read first, so it travels with the
  reading, in the form existing READ cues already use ("**X**, then say what
  it means").
- `read **Y**, then say it without looking` (7, HI-C68-india to
  HI-C74-closed, book chapters 75-81) → one
  `[YOU READ: **Y**, then say it without looking]`. Judgement call: "it" is
  the word just read, so the spoken half cannot be done on its own and stays
  in the READ cue rather than becoming a recall of nothing.
- Three-step recalls keep their order: HI-C37-dudh's "say *lenā*, then say
  *pūchnā*, then read **मदद करना**" → one recall with both says, then the
  READ; HI-C41-big's "say *dost*, then read **यह**, then say *vah*" → recall,
  READ, recall.
- None of these cues carried a spacing tag, so there was none to move.
- Regenerated: 34 book chapters and their hashes, the narration (`.json` and
  `.txt`) and narration hashes for the same chapters, and the 118
  `core/lesson-modality` owners (source hash only).

HI-C01-practice's opening Guided Practice turn was
`[YOU SAY: read all five aloud, left to right]`: a spoken cue asking a driver
to read the chapter's printed words off the page. It names nothing in bold,
so the reading detector cannot see it; a security review found it by hand. It
now asks for the same five words from memory, by meaning ("the two hellos,
the two thanks, and the farewell"), which a driver can do.

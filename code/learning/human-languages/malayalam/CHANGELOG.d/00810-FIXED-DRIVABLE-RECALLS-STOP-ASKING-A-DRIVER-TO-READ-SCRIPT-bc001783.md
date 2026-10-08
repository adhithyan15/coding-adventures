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

- **Count:** 66 cues in 66 drivable lessons.
- `read **Y**` alone (55) → `[YOU READ: **Y**]`; `say *X*, then read **Y**`
  (7) and `read **Y**, then say *X*` (1) → a recall and a READ, in the
  authored order; "say, read, say" (2) → recall, READ, recall; ML-C66-plough's
  "read **മാത്രം**, then say *sammataṁ*, then read **മര്യാദ**" → READ, recall,
  READ.
- None of these cues carried a spacing tag, so there was none to move.
- Regenerated: 36 book chapters and their hashes, the narration and narration
  hashes for the same chapters, and the 66 `core/lesson-modality` owners
  (source hash only).

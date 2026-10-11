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

- **Lessons:** UR-C15-dhoop (`read **Y**` → `[YOU READ: **Y**]`), UR-C17-kaam
  ("read, say, say" → READ, then one recall with both says) and UR-C17-yih
  ("say, read, say" → recall, READ, recall).
- None of these cues carried a spacing tag, so there was none to move.
- Regenerated: book chapters 15 and 17 and their hashes, their narration and
  narration hashes, and the three `core/lesson-modality` owners (source hash
  only).

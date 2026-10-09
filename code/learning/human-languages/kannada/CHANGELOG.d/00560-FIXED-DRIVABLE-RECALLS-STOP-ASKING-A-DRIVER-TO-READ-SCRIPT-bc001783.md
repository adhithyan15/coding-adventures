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

- **Count:** 83 cues in 82 drivable lessons.
- `read **Y**` alone (61) → `[YOU READ: **Y**]`; `say *X*, then read **Y**`
  (12) and `read **Y**, then say *X*` (3) → a recall and a READ, in the
  authored order; KA-C35-snehita and KA-C73-in-order-to's "read **Y**, then
  say *X*, then read **Z**" → READ, recall, READ; one "say, read, say" →
  recall, READ, recall.
- Spaced recalls that were only a reading keep their spacing tag on the READ
  cue, as the writing recalls did on their WRITE cue:
  `[YOU RECALL: read **ಟ**, and say its sound — **R2**]` →
  `[YOU READ: **ಟ**, then say its sound — **R2**]` (KA-C74-first, KA-C75-ninth,
  KA-C75-on-a-sign), and KA-C74-first's `read the sign **ೇ**, and say what it
  does to a letter — **R3**` → `[YOU READ: the sign **ೇ**, then say what it
  does to a letter — **R3**]`. Judgement call: the spoken half names the sound
  or the job of the sign just read, so it stays with the reading; ", and say"
  becomes ", then say", the form existing READ cues use.
- Regenerated: 45 book chapters and their hashes, the narration and narration
  hashes for the same chapters, and the 82 `core/lesson-modality` owners
  (source hash only).

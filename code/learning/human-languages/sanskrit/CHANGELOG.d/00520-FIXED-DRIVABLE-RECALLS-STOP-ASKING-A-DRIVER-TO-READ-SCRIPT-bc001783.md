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

- **Count:** 155 cues in 155 drivable lessons.
- `say *X*, then read **Y**` (88) and `read **Y**, then say *X*` (59) → a
  recall and a READ, in the authored order; `read **Y**` alone (4) →
  `[YOU READ: **Y**]`.
- SA-C62-fifth's spaced `[YOU RECALL: read **ऋ** — **R1**, one lesson back]`
  → `[YOU READ: **ऋ** — **R1**, one lesson back]`: the spacing tag stays on
  the cue that does the recalling, which is now the READ.
- "read **Y** and say what it means" (SA-C59-bharatam, and once after a say)
  → `[YOU READ: **Y**, then say what it means]`; SA-C53-yatra-tatra's "say
  **तत्र**, then read **यत्र** and say what the swap did" → `[YOU RECALL:
  say **तत्र**]` then `[YOU READ: **यत्र**, then say what the swap did]`.
  Judgement call: the spoken half is about the word just read, so it stays
  with the reading.
- Regenerated: 36 book chapters and their hashes, the narration and narration
  hashes for the same chapters, and the 155 `core/lesson-modality` owners
  (source hash only).

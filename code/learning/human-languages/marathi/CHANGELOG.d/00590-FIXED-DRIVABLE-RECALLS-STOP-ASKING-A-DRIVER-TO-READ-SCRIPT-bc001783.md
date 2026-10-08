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

- **Lesson:** MR-R37-who-how-many. `[YOU RECALL: read the form label **आवडती
  कृती** and say why it ends in **-ती**]` → `[YOU READ: the form label
  **आवडती कृती**, then say why it ends in **-ती**]`. Judgement call: the
  spoken half explains the label just read, so the whole cue becomes READ;
  there is no spoken step left to keep as a recall.
- Left as it is: MR-R40-asking-column's `[YOU RECALL: say the line of your
  message that means *I read Marathi*]`. "read" there is part of the
  sentence being recalled, not an instruction.
- Regenerated: book chapter 37 and its hash, its narration and narration
  hash, and the lesson's `core/lesson-modality` owner (source hash only).

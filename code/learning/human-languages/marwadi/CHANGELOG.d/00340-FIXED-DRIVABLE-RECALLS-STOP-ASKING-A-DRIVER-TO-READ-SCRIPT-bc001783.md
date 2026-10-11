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

- **Lesson:** MW-R37-count-fifteen. `[YOU RECALL: read a printed ticket and
  say the figure on it aloud — **R3**]` → `[YOU READ: a printed ticket, then
  say the figure on it aloud — **R3**]`. The cue was only a reading (of a
  printed figure), so it becomes one READ cue and keeps its spacing tag, as
  the writing recalls in this lesson keep theirs on WRITE.
- Left as they are: MW-C105-bharno and MW-C105-janno's "say the Marwadi for
  to read" recalls. "to read" is a gloss of the word being recalled.
- Regenerated: book chapter 37 and its hash, its narration and narration
  hash, and the lesson's `core/lesson-modality` owner (source hash only).

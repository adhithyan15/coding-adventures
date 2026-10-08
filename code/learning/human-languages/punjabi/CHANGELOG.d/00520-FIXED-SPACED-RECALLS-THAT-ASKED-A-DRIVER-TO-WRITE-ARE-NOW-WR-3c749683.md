## Fixed — spaced recalls that asked a driver to write are now writing cues

The drivable-writing detector in human-language-data now reads inside
`[YOU RECALL: …]` cues. RECALL is a spoken cue action, so the narration read a
spaced recall such as `[YOU RECALL: write **X** — **R2**]` to a driver as
"your turn — recall: write X — R2", with no deferral (issue #12070). Each such
cue in a drivable lesson of this track is now a `[YOU WRITE: … from memory]`
cue, the form the Marwadi fix used: WRITE is a manual action, so the narration
says "once you have stopped driving — write: …" and the book prints "*Write
it:* …". The spacing tag (**R1**–**R4**) and the distance note stay in the
cue, and "from memory" keeps what RECALL said in its name. Every lesson stays
`drivable: true`.

- **Lessons:** PA-C44-chautha (two cues), PA-C44-pahila (one).
- PA-C44-pahila's "ask *kitthe?* and write it — **R2**" is one of "Four
  recalls, at four distances", so it stays one list item: `[YOU WRITE:
  *kitthe?* from memory, saying the question aloud as you write it — **R2**,
  five lessons back]`. Splitting the asking into a cue of its own would have
  made five items under a line that says four, and the block assesses the
  question and its spelling together.
- Regenerated: book chapter 44, its narration (`.json` and `.txt`), the
  generated book and narration hashes, and the two `core/lesson-modality`
  owners (source hash only).

## Fixed — drivable lesson prose stops asking a driver to read or handle cards

Narration reads bare prose aloud as written, so a prose instruction to read
printed script, handle cards or cover the page reached a driver unhedged (issue
#12070, tenth pass): "Hear, picture the part, say, and read **おなか**." was
narrated word for word. Each such step now moves into a cue the narration
defers (`[YOU READ: …]`, `[YOU COVER: …]`, `[YOU CHECK: …]`, `[YOU FIND: …]`,
`[YOU WRITE: …]`: "once you have stopped driving — …"), in the authored order,
or is said for the ear and voice where the step was not about the page. Prose
that followed a new cue in the same paragraph now has a paragraph of its own.
The new prose check in human-language-data demands zero such spans in drivable
lessons. Every edited lesson stays `drivable: true` (only its
`core/lesson-modality` source hash changes).

- **Count:** 25 drivable lessons; 28 `[YOU READ: …]` cues and 1 ear-and-voice
  rewrite.
- Letter-by-letter readings ("Read right to left: **ک** *k*, …", "Read from the
  right edge: **ہ** *h*, …", "Read **خدا** first, then **حافظ**, and preserve
  that space …"), UR-C17's "**Read it off the page.**", "Read that sentence off
  the page", "Read these off the page, naming the letters …", "read down the
  right edge", the digit payoffs UR-C28..C32, the UR-C113 / C114 warm-ups, the
  UR-C146 notices and the UR-C33 reading lessons become READ cues.
- UR-C09-bahan "Read on for why." → "The reason comes next."
- Left alone: "Look at what happened to the noun", "Look hard at the plural"
  (the idiom), "a word you hold by ear, not yet by eye" (description).

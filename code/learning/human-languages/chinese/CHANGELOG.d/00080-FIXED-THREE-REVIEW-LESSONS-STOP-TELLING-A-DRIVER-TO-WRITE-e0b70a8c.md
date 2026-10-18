## Fixed — three review lessons stop telling a driver to write

The drivable-writing detector in human-language-data now also reads writing
verbs chained onto an earlier step ("Say and write …", "Say, read, and write
each answer.") and writing verbs after a fronted phrase ("Without looking
back, write …"). Three drivable review lessons still used those shapes, so
the audio edition told a driver to write (issue #12070). Each is now a
`[YOU WRITE: …]` cue; every lesson stays `drivable: true`.

- **Lessons:** ZH-R16-identity-2, ZH-R17-old-three-r2,
  ZH-R18-looking-three-r2.
- ZH-R16-identity-2 warm-up: "Say and write **the Chinese language** without
  a model." becomes "Say **the Chinese language**." and a cue for the same
  word.
- ZH-R16-identity-2 guided practice: "Say, read, and write each answer." keeps
  "Say and read each answer." as prose; the advice about the two 汉
  compounds is about choosing the character to write, so it moves inside the
  cue after a dash.
- ZH-R17-old-three-r2 and ZH-R18-looking-three-r2: "Without looking back,
  write …" moves the fronted phrase inside the cue, and "Read each answer
  aloud only after all three are complete." becomes its own paragraph.
- Regenerated: book chapters 16-18, their narration (`.json` and `.txt`),
  generated book and narration hashes, and the three `core/lesson-modality`
  owners (source hash only).


## Fixed — drivable lessons stop asking a driver to gesture

A spoken cue is read to a driver as an ordinary turn, and so is bare prose.
Drivable lessons in this track still asked for a hand or a gesture inside one:
pointing at a person, and labels that name pointing (issue #12070, ninth
pass). Each ask is now said for the ear and voice where that keeps the
learning goal, or moved into a cue the narration defers (`[YOU POINT: …]`,
`[YOU READ: …]`: "once you have stopped driving — …"). The new gesture check
in human-language-data demands zero such spoken cues in drivable lessons.
Every edited lesson stays `drivable: true` (only its `core/lesson-modality`
source hash changes).

- **Count:** 4 spoken cues and 1 prose instruction in 5 drivable lessons.
- ES-C03-practice and ES-C03-repaso-tu-usted: "point to either speaker / one
  person and ask **¿Quién?**" → "think of …".
- ES-C53-lo-que: "— pointing at a thing, then at a gap" → "— a known thing,
  then a gap"; ES-C59-repaso-demostrativos: "then answer it and point again
  with a noun" → "then answer it, putting a noun after the pointing word".
  Judgement call: both were figures of speech, but heard as part of a spoken
  turn they ask for a point.
- ES-C303-que-significa: "Point at the written word, or say it back, and ask"
  → "Say the word back, and ask" — the spoken option was already there.
- Left alone: the tongue's tap ("one soft tap on the *r*", "Tap the final
  **r** of **poner** lightly"), and headings such as "*este* — pointing at
  what is in your hand", which describe the word.


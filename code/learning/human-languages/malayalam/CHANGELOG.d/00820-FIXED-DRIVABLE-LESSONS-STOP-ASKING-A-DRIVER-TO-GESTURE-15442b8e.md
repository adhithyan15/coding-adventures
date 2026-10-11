## Fixed — drivable lessons stop asking a driver to gesture

A spoken cue is read to a driver as an ordinary turn, and so is bare prose.
Drivable lessons in this track still asked for a hand or a gesture inside one:
counting on fingers, placing a hand on the body, and pointing while saying a
demonstrative (issue #12070, ninth pass). Each ask is now said for the ear and
voice where that keeps the learning goal, or moved into a cue the narration
defers (`[YOU POINT: …]`, `[YOU READ: …]`: "once you have stopped driving —
…"). The new gesture check in human-language-data demands zero such spoken
cues in drivable lessons. Every edited lesson stays `drivable: true` (only its
`core/lesson-modality` source hash changes).

- **Count:** 17 spoken cues and 7 prose instructions in 14 drivable lessons.
- Demonstrative drills: `[YOU SAY: "X" three times, pointing at something
  different each time]` → `…, picturing something different each time]`, and
  the prose "Say it, and point while you say it … it means whatever your
  finger is on" → "Say it, and picture the thing it lands on as you say it …
  it means whatever the speaker is pointing at". Judgement call: picturing
  keeps the point of the drill — a demonstrative means nothing until it lands
  on something — without the finger; the explanation still names pointing, as
  a description of what the word does, not as a request. Lessons: ML-C41-here,
  ML-C41-that, ML-C41-there, ML-C41-this, ML-C41-where, ML-C41-who (cue and
  prose), ML-C42-big, ML-C42-good, ML-C42-new, ML-C42-old, ML-C42-small (cue).
- ML-C07-numbers-1-5 and ML-C07-numbers-6-10: `[YOU HEAR: *añcŭ*; YOU SHOW:
  5]` → `[YOU HEAR: *añcŭ*, then say the number — five]`; "while raising one
  more finger" → ", one number to each beat"; the prose "Point to one finger
  and say *onnŭ*. Add one finger at a time" → "Say *onnŭ*. Count up aloud, one
  word to each beat".
- ML-C57-back: "and place each one with a hand" → "and say where on the body
  each one is".
- Left alone: ML-C85-gemination's "touching the sound once" (a single
  consonant), ML-C62-broom's "say which of the two you hold in a fist"
  (something to say), and ML-C01-namaskaram's "Imagine meeting someone with
  your palms together" (imagined).

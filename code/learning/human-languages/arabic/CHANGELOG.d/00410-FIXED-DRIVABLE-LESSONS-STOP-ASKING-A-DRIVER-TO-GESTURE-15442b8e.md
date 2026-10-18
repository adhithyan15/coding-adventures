## Fixed — drivable lessons stop asking a driver to gesture

A spoken cue is read to a driver as an ordinary turn, and so is bare prose.
Drivable lessons in this track still asked for a hand or a gesture inside one:
touching the head and the hand (issue #12070, ninth pass). Each ask is now
said for the ear and voice where that keeps the learning goal, or moved into a
cue the narration defers (`[YOU POINT: …]`, `[YOU READ: …]`: "once you have
stopped driving — …"). The new gesture check in human-language-data demands
zero such spoken cues in drivable lessons. Every edited lesson stays
`drivable: true` (only its `core/lesson-modality` source hash changes).

- **Count:** 1 prose instruction in 1 drivable lesson.
- AR-R22-the-head-and-the-hand: "Touch your head, then your hand, and give the
  Arabic for each" → "Picture your head, then your hand, …".


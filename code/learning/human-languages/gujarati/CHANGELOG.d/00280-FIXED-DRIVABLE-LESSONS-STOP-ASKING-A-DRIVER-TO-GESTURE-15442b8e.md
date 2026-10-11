## Fixed — drivable lessons stop asking a driver to gesture

A spoken cue is read to a driver as an ordinary turn, and so is bare prose.
Drivable lessons in this track still asked for a hand or a gesture inside one:
pointing at pictures, touching a hand, a hand in front of the mouth, and
pointing while saying a demonstrative (issue #12070, ninth pass). Each ask is
now said for the ear and voice where that keeps the learning goal, or moved
into a cue the narration defers (`[YOU POINT: …]`, `[YOU READ: …]`: "once you
have stopped driving — …"). The new gesture check in human-language-data
demands zero such spoken cues in drivable lessons. Every edited lesson stays
`drivable: true` (only its `core/lesson-modality` source hash changes).

- **Count:** 1 spoken cue and 6 prose instructions in 7 drivable lessons.
- GU-C36-te: "*te* as *that*, pointing at something across the room" →
  "picturing something across the room".
- GU-C20-hear-bajar, GU-C22-hear-rasto, GU-C22-hear-shahar, GU-C23-hear-
  dukaan: "Point to the … picture when you hear …" → "Picture a … when you
  hear …"; GU-C21-hear-haath "Touch your hand" → "Picture your hand".
- GU-C07-khaavun: "hold a hand in front of your mouth and say *kite*" → "say
  *kite* and listen for the breath after the *k*".

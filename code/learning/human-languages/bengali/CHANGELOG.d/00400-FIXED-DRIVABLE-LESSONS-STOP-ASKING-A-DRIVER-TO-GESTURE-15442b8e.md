## Fixed — drivable lessons stop asking a driver to gesture

A spoken cue is read to a driver as an ordinary turn, and so is bare prose.
Drivable lessons in this track still asked for a hand or a gesture inside one:
a palms-together label and labels or prose that ask for pointing (issue
#12070, ninth pass). Each ask is now said for the ear and voice where that
keeps the learning goal, or moved into a cue the narration defers (`[YOU
POINT: …]`, `[YOU READ: …]`: "once you have stopped driving — …"). The new
gesture check in human-language-data demands zero such spoken cues in drivable
lessons. Every edited lesson stays `drivable: true` (only its `core/lesson-
modality` source hash changes).

- **Count:** 4 spoken cues and 1 prose instruction in 5 drivable lessons.
- BN-C01-practice: "hello / goodbye, palms together — *nômoshkar*" → "the
  palms-together greeting" (a description, not an instruction).
- BN-C21-oi "the pair, pointing twice" → "the pair, near then far"; BN-C22-ra
  "pointing at them — *ei bhāirā*" → "these brothers here"; BN-R25-the-one-it-
  happens-to "pointing, counting, colouring" → "near, counted and coloured".
  Judgement call: labels, not requests, but heard as "say: pointing at them"
  they read as one.
- BN-R21-pointing: "Take the whole book's nouns and point at them, near and
  far" → "… and say each one near and far".
- Left alone: the non-drivable BN-C22-gulo, BN-C23-she, BN-R24-all-of-us and
  BN-W04 cues, which keep their pointing.

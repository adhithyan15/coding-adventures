## Fixed — drivable lessons stop asking a driver to gesture

A spoken cue is read to a driver as an ordinary turn, and so is bare prose.
Drivable lessons in this track still asked for a hand or a gesture inside one:
pointing while saying a demonstrative, and pointing at a letter (issue #12070,
ninth pass). Each ask is now said for the ear and voice where that keeps the
learning goal, or moved into a cue the narration defers (`[YOU POINT: …]`,
`[YOU READ: …]`: "once you have stopped driving — …"). The new gesture check
in human-language-data demands zero such spoken cues in drivable lessons.
Every edited lesson stays `drivable: true` (only its `core/lesson-modality`
source hash changes).

- **Count:** 1 spoken cue and 1 prose instruction in 2 drivable lessons.
- PA-C42-oh: "*oh* as *that*, pointing across the room" → "picturing something
  across the room".
- PA-R28-head-na-r4: "then point to **ਨ**" is script work, so it becomes `[YOU
  POINT: **ਨ**]`.

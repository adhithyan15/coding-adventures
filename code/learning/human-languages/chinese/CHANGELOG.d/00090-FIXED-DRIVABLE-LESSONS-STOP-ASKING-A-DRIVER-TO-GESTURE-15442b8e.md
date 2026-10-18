## Fixed — drivable lessons stop asking a driver to gesture

A spoken cue is read to a driver as an ordinary turn, and so is bare prose.
Drivable lessons in this track still asked for a hand or a gesture inside one:
pointing at strokes and components of a character, and pointing at meanings
(issue #12070, ninth pass). Each ask is now said for the ear and voice where
that keeps the learning goal, or moved into a cue the narration defers (`[YOU
POINT: …]`, `[YOU READ: …]`: "once you have stopped driving — …"). The new
gesture check in human-language-data demands zero such spoken cues in drivable
lessons. Every edited lesson stays `drivable: true` (only its `core/lesson-
modality` source hash changes).

- **Count:** 9 prose instructions in 9 drivable lessons.
- Pointing at a stroke or a component is script work the ear cannot do, so it
  becomes a `[YOU POINT: …]` cue the narration defers: ZH-C13-da, ZH-C13-xue,
  ZH-C13-xuesheng, ZH-C14-xiao, ZH-C15-zhong, ZH-C18-kanjian.
  ZH-C15-zhongxuesheng "Read **中学**, then point to 生" becomes one `[YOU READ:
  **中学**, then point to 生]` cue.
- ZH-R12-family-six and ZH-R13-school-six: "point to meanings" → "say each
  meaning". Left alone in those two lessons: the meaning and character cards,
  which are reading and handling rather than gesture.


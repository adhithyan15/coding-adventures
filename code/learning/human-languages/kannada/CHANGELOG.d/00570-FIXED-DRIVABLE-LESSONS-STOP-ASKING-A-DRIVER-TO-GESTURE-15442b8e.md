## Fixed — drivable lessons stop asking a driver to gesture

A spoken cue is read to a driver as an ordinary turn, and so is bare prose.
Drivable lessons in this track still asked for a hand or a gesture inside one:
pointing while saying a demonstrative (issue #12070, ninth pass). Each ask is
now said for the ear and voice where that keeps the learning goal, or moved
into a cue the narration defers (`[YOU POINT: …]`, `[YOU READ: …]`: "once you
have stopped driving — …"). The new gesture check in human-language-data
demands zero such spoken cues in drivable lessons. Every edited lesson stays
`drivable: true` (only its `core/lesson-modality` source hash changes).

- **Count:** 12 spoken cues and 6 prose instructions in 12 drivable lessons.
- Demonstrative drills: `[YOU SAY: "X" three times, pointing at something
  different each time]` → `…, picturing something different each time]`, and
  the prose "Say it, and point while you say it … it means whatever your
  finger is on" → "Say it, and picture the thing it lands on as you say it …
  it means whatever the speaker is pointing at". Judgement call: picturing
  keeps the point of the drill — a demonstrative means nothing until it lands
  on something — without the finger; the explanation still names pointing, as
  a description of what the word does, not as a request. Lessons: KA-C41-here,
  KA-C41-that, KA-C41-there, KA-C41-this, KA-C41-where, KA-C41-who (cue and
  prose), KA-C42-big, KA-C42-good, KA-C42-new, KA-C42-old, KA-C42-small (cue).
- KA-C50-now: "and point at the near one and the far one" → "and say which is
  near and which is far".
- Left alone: KA-R04-second-pass-going-and-coming-back's "wave away an apology
  — *paravāgilla*" (a speech act, not a wave).

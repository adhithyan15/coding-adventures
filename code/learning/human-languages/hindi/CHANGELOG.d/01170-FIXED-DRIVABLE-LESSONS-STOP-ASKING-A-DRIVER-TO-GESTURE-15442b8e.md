## Fixed — drivable lessons stop asking a driver to gesture

A spoken cue is read to a driver as an ordinary turn, and so is bare prose.
Drivable lessons in this track still asked for a hand or a gesture inside one:
pointing while saying a demonstrative, touching a body part as it is named,
and offering with both hands (issue #12070, ninth pass). Each ask is now said
for the ear and voice where that keeps the learning goal, or moved into a cue
the narration defers (`[YOU POINT: …]`, `[YOU READ: …]`: "once you have
stopped driving — …"). The new gesture check in human-language-data demands
zero such spoken cues in drivable lessons. Every edited lesson stays
`drivable: true` (only its `core/lesson-modality` source hash changes).

- **Count:** 15 spoken cues and 9 prose instructions in 18 drivable lessons.
- Demonstrative drills: `[YOU SAY: "X" three times, pointing at something
  different each time]` → `…, picturing something different each time]`, and
  the prose "Say it, and point while you say it … it means whatever your
  finger is on" → "Say it, and picture the thing it lands on as you say it …
  it means whatever the speaker is pointing at". Judgement call: picturing
  keeps the point of the drill — a demonstrative means nothing until it lands
  on something — without the finger; the explanation still names pointing, as
  a description of what the word does, not as a request. Lessons: HI-C40-here,
  HI-C40-that, HI-C40-there, HI-C40-this, HI-C40-where, HI-C40-who (cue and
  prose), HI-C41-big, HI-C41-good, HI-C41-new, HI-C41-old, HI-C41-small (cue).
- HI-C46-finger "and touch each as you name it" → "picturing each as you name
  it"; HI-C53-shoulder "and touch it" → "picturing your shoulder";
  HI-C57-present "offered with both hands" → "as if offering it"; HI-C72-price
  "pointing at something in the room" → "picturing something you might buy".
- Prose: HI-C88-aaega and HI-C89-kar-raha-hai "point at somebody else" →
  "think of somebody else"; HI-R69-reported "Say the *lekin* sentence and
  point at both" → "… and name both" (both verbs, which the ear can name).

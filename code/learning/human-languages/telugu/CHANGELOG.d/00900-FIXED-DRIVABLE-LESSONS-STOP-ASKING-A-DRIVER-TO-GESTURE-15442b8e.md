## Fixed — drivable lessons stop asking a driver to gesture

A spoken cue is read to a driver as an ordinary turn, and so is bare prose.
Drivable lessons in this track still asked for a hand or a gesture inside one:
pointing while saying a demonstrative, and pointing at a word on the page
(issue #12070, ninth pass). Each ask is now said for the ear and voice where
that keeps the learning goal, or moved into a cue the narration defers (`[YOU
POINT: …]`, `[YOU READ: …]`: "once you have stopped driving — …"). The new
gesture check in human-language-data demands zero such spoken cues in drivable
lessons. Every edited lesson stays `drivable: true` (only its `core/lesson-
modality` source hash changes).

- **Count:** 12 spoken cues and 9 prose instructions in 15 drivable lessons.
- Demonstrative drills: `[YOU SAY: "X" three times, pointing at something
  different each time]` → `…, picturing something different each time]`, and
  the prose "Say it, and point while you say it … it means whatever your
  finger is on" → "Say it, and picture the thing it lands on as you say it …
  it means whatever the speaker is pointing at". Judgement call: picturing
  keeps the point of the drill — a demonstrative means nothing until it lands
  on something — without the finger; the explanation still names pointing, as
  a description of what the word does, not as a request. Lessons: TE-C41-here,
  TE-C41-that, TE-C41-there, TE-C41-this, TE-C41-where, TE-C41-who (cue and
  prose), TE-C42-big, TE-C42-good, TE-C42-new, TE-C42-old, TE-C42-small (cue).
- TE-C57-bone: "then point at your మెడ and say *nāku noppi*" → "then name your
  మెడ and say *nāku noppi*".
- TE-C151-chhaya: the warm-up "Point once to the rare **ఙ** …" points at
  script, which the ear cannot do, so it becomes `[YOU POINT: the rare **ఙ**
  you met in **వాఙ్మయం**, once]`, which the narration defers.
- TE-R95-second-pass-asking: "Point at something and ask how to say it" →
  "Think of something near you and ask …". TE-R99-second-pass-welcome: "Greet
  them with palms together, and say …" → "Greet them, and say …".
- Left alone: TE-C95-show's "When a word will not come, point and say
  **చూపించండి**" (advice for a real conversation), the TE-R99 review prompts
  "hand them a gift" / "hand over a gift" (situations to speak in), and the
  descriptive prose of TE-C51 and TE-C56.

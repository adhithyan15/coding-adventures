## Fixed — drivable lessons stop asking a driver to gesture

A spoken cue is read to a driver as an ordinary turn, and so is bare prose.
Drivable lessons in this track still asked for a hand or a gesture inside one:
pointing while saying a demonstrative, and making the añjali (issue #12070,
ninth pass). Each ask is now said for the ear and voice where that keeps the
learning goal, or moved into a cue the narration defers (`[YOU POINT: …]`,
`[YOU READ: …]`: "once you have stopped driving — …"). The new gesture check
in human-language-data demands zero such spoken cues in drivable lessons.
Every edited lesson stays `drivable: true` (only its `core/lesson-modality`
source hash changes).

- **Count:** 12 spoken cues and 6 prose instructions in 12 drivable lessons.
- Demonstrative drills: `[YOU SAY: "X" three times, pointing at something
  different each time]` → `…, picturing something different each time]`, and
  the prose "Say it, and point while you say it … it means whatever your
  finger is on" → "Say it, and picture the thing it lands on as you say it …
  it means whatever the speaker is pointing at". Judgement call: picturing
  keeps the point of the drill — a demonstrative means nothing until it lands
  on something — without the finger; the explanation still names pointing, as
  a description of what the word does, not as a request. Lessons: SA-C14-here,
  SA-C14-that, SA-C14-there, SA-C14-this, SA-C14-where, SA-C14-who (cue and
  prose), SA-C15-big, SA-C15-good, SA-C15-new, SA-C15-old, SA-C15-small (cue).
- SA-C30-anjali: "then say *namaste* and name what your hands are doing" → "…
  and name the gesture that goes with it" (the answer is the lesson's word,
  *añjaliḥ*). The new check does not see this one — it presupposes the gesture
  without naming a movement — so it was found by reading.
- Left alone: SA-R12-second-pass-what-sanskrit-leaves-out's "wave away a
  thank-you — *na cintā*" (a speech act) and SA-C01-namaste's description of
  the añjali.

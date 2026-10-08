## Fixed — drivable lessons stop asking a driver to gesture

A spoken cue is read to a driver as an ordinary turn, and so is bare prose.
Drivable lessons in this track still asked for a hand or a gesture inside one:
pointing while saying a demonstrative, pointing at shapes on the page, and
labels that name a gesture (issue #12070, ninth pass). Each ask is now said
for the ear and voice where that keeps the learning goal, or moved into a cue
the narration defers (`[YOU POINT: …]`, `[YOU READ: …]`: "once you have
stopped driving — …"). The new gesture check in human-language-data demands
zero such spoken cues in drivable lessons. Every edited lesson stays
`drivable: true` (only its `core/lesson-modality` source hash changes).

- **Count:** 6 spoken cues and 5 prose instructions in 9 drivable lessons.
- UR-C17-yih and UR-C18-voh: "pointing each time", "and point at something
  across the room", "pointing differently each time" → picturing ("picturing
  one mango near and one far"); UR-C17-yih's "Point at something within reach
  and name it" → "Picture something within reach …"; UR-C17-kaam "then point
  at your own ear and use it" → "then use it to name your own ear".
- Review labels that name a gesture: UR-R23-into-and-onto "pointing and
  placing at once" → "near and inside at once", UR-R24-here-there-where
  "gesturing, then naming" → "the bare *there*, then the named place",
  UR-R27-where-you-are-from "pointing, near and far" → "near and far".
  Judgement call: each label described what the phrase does, but a listener at
  speed hears "say: pointing …" as a request.
- UR-C01-ji-han and UR-C03-khushi-hui: pointing at the shapes of **شکریہ** is
  script work, so it becomes a `[YOU POINT: …]` cue on its own line.
- UR-C29-chhe: "Hand at the mouth" → "Listen for the breath".
- Left alone: UR-C17-kahan's gloss "pointing and asking at once", and the non-
  drivable reviews UR-C17-review-shapes, UR-C18-review-aap and UR-C24-vahan,
  which keep their gestures (their narration opens with the hands-and-eyes
  notice).

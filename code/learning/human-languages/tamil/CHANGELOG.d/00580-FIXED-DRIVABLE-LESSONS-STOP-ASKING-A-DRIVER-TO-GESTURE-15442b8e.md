## Fixed — drivable lessons stop asking a driver to gesture

A spoken cue is read to a driver as an ordinary turn, and so is bare prose.
Drivable lessons in this track still asked for a hand or a gesture inside one:
counting on fingers, raising each hand, a bow, and pointing while saying a
demonstrative (issue #12070, ninth pass). Each ask is now said for the ear and
voice where that keeps the learning goal, or moved into a cue the narration
defers (`[YOU POINT: …]`, `[YOU READ: …]`: "once you have stopped driving —
…"). The new gesture check in human-language-data demands zero such spoken
cues in drivable lessons. Every edited lesson stays `drivable: true` (only its
`core/lesson-modality` source hash changes).

- **Count:** 19 spoken cues and 10 prose instructions in 17 drivable lessons.
- Demonstrative drills: `[YOU SAY: "X" three times, pointing at something
  different each time]` → `…, picturing something different each time]`, and
  the prose "Say it, and point while you say it … it means whatever your
  finger is on" → "Say it, and picture the thing it lands on as you say it …
  it means whatever the speaker is pointing at". Judgement call: picturing
  keeps the point of the drill — a demonstrative means nothing until it lands
  on something — without the finger; the explanation still names pointing, as
  a description of what the word does, not as a request. Lessons: TA-C40-here,
  TA-C40-that, TA-C40-there, TA-C40-this, TA-C40-where, TA-C40-who (cue and
  prose), TA-C41-big, TA-C41-good, TA-C41-new, TA-C41-old, TA-C41-small (cue).
- TA-C07-numbers-1-5 and TA-C07-numbers-6-10: `[YOU HEAR: *aintu*; YOU SHOW:
  5]` (a nested SHOW cue, read out as "YOU SHOW: 5") → `[YOU HEAR: *aintu*,
  then say the number — five]`; "while raising one more finger" → ", one
  number to each beat"; the prose "Raise one more finger for each word" →
  "Count up aloud, one word to each beat".
- TA-C66-left and TA-C66-whichway-recall: "raise each hand as you say it" →
  "say *right* or *left* after each"; the warm-up "Which hand is the **வலது
  கை**? Hold it up." → "Say which: right or left."
- TA-C01-vanakkam-family-register: `[YOU SAY: "vaṇakkam" with a small bow]` →
  `[YOU SAY: "vaṇakkam", the word that names the bow]`; the prose "Say
  *vaṇakkam* with pressed palms or a small head-bow" now describes the gesture
  ("*Vaṇakkam* is said with …") instead of asking for it.
- TA-C70-roof: "Point up at the கூரை, down at the தரை" → "Picture the கூரை up
  above, the தரை down below".
- Left alone: TA-C93-show's "When a word will not come, point and say
  **காட்டுங்கள்**" (advice for a real conversation, not a task in the lesson),
  TA-C68-buy's "say which way each hand is moving" (something to say), and
  every gloss or vocabulary list that names a hand, a finger, a tap or a
  touch.

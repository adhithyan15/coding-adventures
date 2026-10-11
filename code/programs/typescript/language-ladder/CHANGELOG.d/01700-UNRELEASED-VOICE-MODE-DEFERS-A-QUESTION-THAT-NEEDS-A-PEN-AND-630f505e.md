## Unreleased — voice mode defers a question that needs a pen, and warns before a set-aside title

- **A scored question can be deferred.** The narration generator now marks a
  scored `activity` segment `spoken: false` when its prompt needs a hand or an
  eye ("Write son in Chinese characters.", "Type 'a car' in hiragana."), exactly
  as it marks a `[YOU WRITE: …]` cue. `buildVoiceScript` honours the flag: it
  says "Once you have stopped driving — question: …", leaves no answer gap, and
  adds the question to the end-of-lesson recap. An activity with no flag (older
  narration) is no longer simply asked: `isHandsOnActivity` fails safe, judging
  it with the generator's own `activityPromptNeedsEyesOrHands` (the same
  fallback `isHandsOnCue` makes to `MANUAL_CUE_ACTIONS`) and deferring it too
  when it still says "type", because narration without the flag was never
  reworded for the ear. Every question it does ask already arrives
  worded for the ear — the generator says "Type the Japanese for 'a stick'" as
  "Say the Japanese for 'a stick'" — so nothing here rewords a prompt.
- **The stop guard comes before a set-aside section's title**, not straight
  after it. The title is the section's own words and can ask for the very thing
  the guard warns about ("Script you'll notice: Read ன்னி"); the narration's
  plain-text script moved its guard the same way.
- Tests: deferred and asked activities on synthetic lessons, activities with no
  flag judged by the generator's test (deferred: "From bachchā, write the taught
  word.", "Type the Japanese for 'a stick'."; asked: "Does it point to my or
  your?"), the flag winning in both directions, and a real-corpus
  check that no deferred question in Chinese chapter 12 is ever asked; the
  guard-order test now pins guard, then title, then content.
- **An activity with no verdict is judged on what it accepts too.**
  `isHandsOnActivity` now falls back to the generator's
  `activityNeedsEyesOrHands`, which reads an activity's `assesses` and
  `acceptedResponses` (new optional `NarrationSegment` fields, as the generator
  emits them; `answer` and `accepted` stand in when they are absent) as well as
  its prompt: a question whose every accepted answer is script ("女") or that
  grades a `-SCRIPT-` atom with a script answer is deferred, as are the second
  security review's prompt shapes ("Refuse in one word, in Devanagari.", "… in
  two written lines.", "Record the retrieval string …", "… is na: ਨ or ਸ?").
  Tests cover each, with romanized and word-atom controls.

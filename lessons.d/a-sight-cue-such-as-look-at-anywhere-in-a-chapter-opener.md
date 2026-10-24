---
category: Testing & coverage
---

# A sight cue such as 'look at' anywhere in a chapter opener, even in an example's English, makes the chapter undrivable

The Gujarati B1 vocabulary opened chapter 544 with ફેર "a difference",
whose example read "If you look at both cloths carefully, you'll be able
to tell the difference." The modality deriver reads the whole lesson,
English included, and marked it `sight` (reason `sight-cue`). A chapter
whose first lesson is not drivable counts against the by-content ceiling
in `tests/drivable-prefix.test.ts`, which may only fall, so the full
suite went from 8 to 9 and failed. Every generation-time screen had
passed; only the full package vitest caught it.

The fix was in the lesson: the example now says the learner should listen
to both songs ("જો તમે બંને ગીતો સાંભળશો તો …"), so the opener is
drivable again. The ceiling was not touched.

What to do differently: before generating, grep every chapter's FIRST
lesson (ex and ex_en included) for sight and hand cues ("look at",
"see the", "write", "point to", "read the") and reword them for the ear.
Then run the full human-language-data vitest, not just a selection,
before pushing generated vocabulary.

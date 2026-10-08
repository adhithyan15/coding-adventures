---
category: Repo policy / workflow reminders
---

# A cue verb missing from MANUAL_CUE_ACTIONS is read to a driver, so every corpus cue verb must be classified by a test

`narration.ts` decides whether a driver may do a `[YOU <VERB>: …]` cue by
looking the verb up in `MANUAL_CUE_ACTIONS`; anything not listed is read out
as an ordinary turn with an eight-second pause. The set was six verbs, and its
comment promised that any other manual verb "will be caught by the lesson's
`type: writing` or its script block long before it gets here". It was not.
An inventory found `[YOU COPY: …]` and `[YOU CIRCLE: …]` in drivable lessons,
40 drivable `[YOU LOOK: at … and put your finger on …]` cues and about 130
drivable `[YOU READ: …]` cues, all narrated to a driver unhedged. Twenty-four
drivable Japanese `[YOU RECALL: point to the sign in …]` cues did the same
from inside a spoken verb.

A list whose default is "safe" goes stale silently, because nothing fails when
a new member should have been added. The fix pairs the list with its
complement (`SPOKEN_CUE_ACTIONS`) and a corpus test
(`tests/cue-action-classification.test.ts`) that walks every cue and fails on
a head verb in neither set. A new verb now arrives as a failing test that asks
for a decision.

When you add a cue verb to a lesson, the test will name it: put it in
`SPOKEN_CUE_ACTIONS` only if a driver can do every use of it by voice and ear,
otherwise in `MANUAL_CUE_ACTIONS`. The same applies to any "everything else is
fine" allowlist: give it a complement and a test that the two cover the data.

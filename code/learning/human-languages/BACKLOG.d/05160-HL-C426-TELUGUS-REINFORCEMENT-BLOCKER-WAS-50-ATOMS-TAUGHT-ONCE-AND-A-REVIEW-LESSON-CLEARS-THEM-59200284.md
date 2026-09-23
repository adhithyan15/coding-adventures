## HL-C426-59200284 — Telugu's reinforcement blocker was 50 atoms taught once, and a review lesson clears them without touching the vocabulary count

**Status: CLOSED (2026-09-23) — implemented and measured.** Found by reading the
gate's own defect list rather than its shortfall number, after HL-C424 costed
Telugu's pre-A1 and found three blockers where the ladder printed one.

### The ladder prints one blocker per track, and Telugu had two

`report-cli`'s per-track ladder placed telugu in the **vocabulary** group with
*"vocabulary short 79"*, and said nothing else. The gate's own record disagrees:

```json
"blockers": [
  {"criterion": "vocabulary",    "detail": "221 headwords at or below pre-A1, against 300", "shortfall": 79},
  {"criterion": "reinforcement", "detail": "50 atom(s) at or below pre-A1 are revisited fewer than twice", "shortfall": 50}
]
```

§3.1 is a **conjunction**. A track with two blockers that shows one is a report
that understates the work, and every track on that ladder may be hiding the
same thing. Filed separately as HL-C427; it is a reporting defect, not a content
one, and it should not ride in a content PR.

### What the 50 atoms actually were

Not a scatter. `measureContinuity` names each one, and dumping them with the
sequence of the lesson that introduced them gives a shape a summary cannot:

| where | atoms | revisits |
|---|---|---|
| chapters 1-5, the opening lexicon | 42 | 40 at r=1, 2 at r=0 |
| six script-recognition atoms | 6 | 5 at r=1, 1 at r=0 |
| one chapter-40 lexeme | 1 | r=1 |
| one late script atom | 1 | r=1 |

**Forty-seven of the fifty needed exactly one more revisit.** The cause is
visible in the track's own layout: chapters 1 to 5 each end with a `practice`
lesson and **chapters 6 to 31 have none at all**. Every opening atom got its
first revisit from its own chapter recap and nothing afterwards. The gate wants
two. The corpus was one short, everywhere, for the same structural reason.

### Why this is the cheap blocker and the vocabulary one is not

HL-C424 measured what a plain vocabulary tranche costs: 79 new headwords are
roughly 237 new atoms, each wanting two revisits, which would have taken
reinforcement from 50 to about 130 while the headline number read *finished*.

The reverse trade is what makes this entry worth doing:

- `CONTENT_TYPES` is `{word, phrase}`. A `review` lesson is **outside** it, so
  it adds **no headword** and cannot move the vocabulary shortfall.
- It introduces no atoms, so it costs nothing against
  `maxNewAtomsPerChapter` (12) or `maxNewAtomsPerLesson` (3).
- `practisedAtoms` counts one revisit per LESSON, not per mention, so a single
  review lesson listing eleven atoms discharges eleven debts at once.

So reinforcement is the one pre-A1 criterion that can be cleared **without
buying new debt**, and it should be cleared first on every track.

### The shape, and it is the template

Nine `review` lessons, each placed where the material it revisits has a reason
to come back rather than at the end of the track:

| lesson | seq | revisits | why there |
|---|---|---|---|
| TE-C06-second-pass-greetings | 325 | the opening five words | first chapter after the last recap |
| TE-C07-second-pass-names | 345 | the name exchange, and three vowel letters | the letters open ఒకటి · ఐదు · ఆరు, and the chapter counts |
| TE-C09-second-pass-wellbeing | 367 | how-are-you, and పరవాలేదు | పరవాలేదు is the reply to that chapter's క్షమించండి |
| TE-C10-second-pass-farewells | 377 | the three goodbyes | రేపు కలుద్దాం lands in the days-of-the-week chapter |
| TE-C12-second-pass-verbs | 397 | the first three verbs, and పరవాలేదు again | లేదు is ఉండు's negative twin |
| TE-S168-script-recall-split-sounds | 455 | శ · ఫ | both exist to split a sound its neighbour cannot carry |
| TE-C40-second-pass-meal | 875 | టీ · పాలు · భోజనం | asked for with the ‑ండి ending |
| TE-S169-script-recall-courtesy-letters | 1412 | ఞ · ◌ౌ | both hide inside this chapter's courtesy words |
| TE-S170-script-dictation-courtesy-letters | 1422 | ఞ · ◌ౌ again | the two atoms at r=0 need two passes, not one |

Measured before and after, same command:

```
before   telugu: vocabulary 79, reinforcement 50
after    telugu: vocabulary 79
```

Telugu is now the only track of the 23 whose pre-A1 rung is blocked by **one**
criterion. Forward references held at the baseline 10 with none from the nine.

### Three things the gates caught that reading had not

1. **`delivery: script` is a marker for `type: writing` only.** Two of the three
   new script lessons are `type: review` and carried it; `modality-manifest`
   asserts the two sets are equal per track. The existing
   `TE-S167-script-recall-vowels` had the answer and was not consulted.
2. **Naming a chapter number in prose is capped, per track.** The first draft
   added sixteen cross-chapter references against a Telugu ceiling of 46 and
   took it to 62. HL-C102's rule is right and the fix improved the lessons:
   *name the thing, not the number* — "the opening five", "when you learned to
   answer *how are you*", "the two-word form you learned with it".
3. **Two lessons ran 327 effective seconds** against the 300s ceiling, from
   prose length and prompt count rather than from the tables.

### What this does NOT do, and should not be read as doing

It moves **no exam item**, because no exam item was ever scored against Telugu.
It clears one of five corpus criteria at one level for one track. A reader who
finishes this material still cannot sit a pre-A1 paper — the vocabulary rung is
79 words away and untouched by design.

### The generalisation, for the other 22 tracks

Every track is blocked at pre-A1 and every track has the same structural gap:
recaps at the front and none after. Before authoring vocabulary anywhere, dump
that track's thin-atom list with `measureContinuity` and place review lessons
against it. Vocabulary and its revisits have to be planned together, and the
revisits are cheaper when they ride on material the reader has a reason to meet
again.

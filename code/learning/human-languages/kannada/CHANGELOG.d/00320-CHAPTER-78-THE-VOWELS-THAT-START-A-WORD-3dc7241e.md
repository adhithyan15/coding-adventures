## Chapter 78 — the vowels that start a word

Six independent vowels — **ಆ ಎ ಏ ಒ ಐ ಋ** — taught as writing, with a cited
stroke order for each. Characters the corpus prints without teaching fall
**19 → 13**.

**No exam point moves with this chapter.** `KA-A1-L-09` demands the whole used
set, and thirteen characters are still short of it, so Kannada stays at
**197/258**. That is stated up front rather than buried: this chapter pays down
debt, and the point it is aimed at closes in the tranche after it.

| metric | before → after |
|---|---|
| exam-point coverage | 197/258 → **197/258 (76%)**, unchanged |
| characters used but untaught | **19 → 13** |
| atoms taught | 434 → 440 |
| measurable lessons | 326 → 333 |
| writing-practice lessons | 82 → 89 |
| `forwardReferences` | unchanged |
| `scriptClosureViolations` | unchanged |
| `durationViolations` | unchanged (0) |
| `atomsNeverRevisited` | unchanged |
| `payoffSurprises` | unchanged (0) |
| reinforcement-window misses | 687 → 707 |

### Why these six and not the other thirteen

The nineteen characters left after chapters 67–73 split cleanly in two, and the
split is about **sources, not difficulty**.

These six have an attested stroke order. `data/scripts/kannada.json` carries a
cited Wikimedia Commons animation for each, with frame counts, and chapter 68
had already shown what a lesson built on one looks like by teaching ಅ with its
four-movement order and the citation printed underneath. So these six are taught
the same way — as **writing**, not recognition.

The other thirteen — the vowel signs ii, ai and au, and ಖ ಘ ಠ ಢ ಣ ಧ ಫ ಭ ಶ ಷ —
have **no sourced ductus anywhere in this project**. The honest fallback for an
unsourced letter is a recognition lesson, and thirteen of those is its own
tranche. Nothing here invents a pen path to make a count move.

### What was added

| lesson | character | pen lifts | anchored in |
|---|---|---|---|
| `KA-S155-letter-aa` | ಆ | 1 | ಆರು — six |
| `KA-S156-letter-e` | ಎ | 0 | ಎರಡು — two |
| `KA-S157-letter-ee` | ಏ | 1 | ಏನು — what? |
| `KA-S158-letter-o` | ಒ | 0 | ಒಂದು — one |
| `KA-S159-letter-ai` | ಐ | 0 | ಐದು — five |
| `KA-S160-letter-vocalic-r` | ಋ | 2 | ಋತು — season |
| `KA-R78-vowels-recall` | — | — | cold retrieval of all six |

**Five of the six open a number the reader already counts with.** These are not
letters kept back for being exotic; they were on the page from chapter seven,
and only now does the reader have the shapes.

**ಏ closes the first question this book ever taught** — ನಿಮ್ಮ ಹೆಸರು ಏನು?, from
chapter two. A learner could ask it out loud and not write its last word.

The recall sorts the six by **pen lifts** rather than by sound: none for ಎ, ಒ
and ಐ; one for ಆ and ಏ; two for ಋ. Sorting by sound gives a list to memorise;
sorting by what the hand does gives something a learner can check themselves
against.

### A draft claim that was wrong, and how it was caught

The first draft of `KA-S155-letter-aa` told the reader that **ಆ is ಅ plus an
upright on the right**, and that this is how the script marks length across the
vowel letters.

`data/scripts/kannada.json` says no such thing. It gives ಅ and ಆ the **same four
components** — a compact left loop, a broad lower bowl, a rounded right loop, an
inward horizontal bar — and records the difference as a **pen lift**: ಅ runs all
four without lifting, ಆ stops once at the upper right. The generalisation was
worse still: ಇ and ಈ share almost no components at all, so "across the vowel
letters" was invented.

Both claims came out. The lesson now says what the data says — same four parts,
one lift apart — and points the reader at the lift, which is the thing the hand
remembers when the eye is still deciding.

A second draft claim, that ಏನು "was the second word this book taught you", was
also wrong: it is the eleventh lesson. It now says what it is — the last word of
the first question the book teaches.

### A third one the gates did not catch

`data/scripts/kannada.json` gives ಋ the ISO-15919 sound **`r̥`** — a plain `r`
plus U+0325 COMBINING RING BELOW — and the first draft copied that straight into
the lesson. `validate` passed. All twelve `check:*` gates passed. The full suite
did not:

```
kannada/book/chapters/ch78-the-vowels-that-start-a-word.tex U+0325 (main)
```

Latin Modern Roman has no glyph for that combining mark, so the generated
chapter would have printed a hole. **The track already had a renderable
convention**: every other Kannada and Telugu lesson writes this sound `ṛ`
(U+1E5B, precomposed), including `KA-S131-vowel-sign-vocalic-r` in this very
track. A data file's `sound` field is reference data, not learner-facing prose.
Recorded in `lessons.d/`.


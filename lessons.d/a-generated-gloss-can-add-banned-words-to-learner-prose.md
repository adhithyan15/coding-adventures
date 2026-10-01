---
category: Repo policy / workflow reminders
---

# A generated gloss can add banned words to learner prose

**What happened:** a Portuguese A2 vocabulary tranche glossed *apenas* as
"only, just". The tranche generator prints the gloss twice in each word
lesson: in the "You'll want to know" paragraph and in the wrap-up recall. That
put two new "just" into learner-facing prose. CI's `build (ubuntu-latest)`
failed in `tests/banned-words.test.ts` (HL10 §7.4), which caps the corpus-wide
count of *simply*, *just*, *obviously* and *as you know* at its current debt:

> expected 1058 to be less than or equal to 1056

My local validation set (track corpus test, level-gate, chapters,
chapter-references, payoff-summary-case, membership shards) did not include
`banned-words`.

**Fix:** gloss *apenas* as "only, merely", regenerate the tranche, and
re-run. The same grep then found three more in tranches that were not yet
pushed: Latin *sīcut* "just as", Latin *iūstus* "just, fair", and Marathi notes
glossing the particle *च* as "just". All were reworded before pushing.

**What to do differently:**

1. Add `tests/banned-words.test.ts` to the targeted set for any change that
   adds lesson prose. It runs in seconds.
2. Before generating a tranche, grep the word list, gloss overrides and spine
   notes for `\b(just|simply|obviously|as you know)\b`. A gloss is prose: it
   reaches the learner twice per lesson.
3. Remember that the English word *just* is often the natural gloss: "just as",
   "just now", "fair, just". Pick a synonym ("merely", "even as", "fair,
   righteous", "right now").

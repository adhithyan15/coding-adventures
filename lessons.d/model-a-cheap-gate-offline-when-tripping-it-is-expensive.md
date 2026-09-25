---
category: Testing & coverage
---

# Model a cheap gate offline when tripping it is expensive

HL-C439 wrote five Hindi retrieval lessons and spent **six trimming passes per
lesson** discovering the duration formula by collision: write, run `validate`
(30s), read the failure, cut fifty words, repeat. Three lessons first measured
495s, 502s and 358s against a 300s ceiling.

HL-C440 wrote **nine** Tamil lessons and trimmed **zero** times. The difference
was a fifty-line script that mirrors the gates against a markdown file that is
not in the repo yet:

```
check-draft.mjs <file.md>
  computed duration + its four inputs (words, prompts, repeat cues, pauses)
  headings classifyBlock would reject; first-block / last-block rules
  banned words; nested emphasis; forward-reference blocklist hits
```

Three things made it worth building rather than guessing at:

**Verify the model against known-good output before trusting it.** Run against
the five committed HL-C439 lessons it reproduced `estimateLessonDuration`
exactly — every field, not just the total (186/178/291/286/283). A mirror that
is close is worse than none, because it teaches you to trust a number that
drifts.

**It catches what the formula cannot.** Two banned words (`simply`) in a draft,
which would have failed the corpus-wide ceiling on the first CI run.

**Model the coverage claim too, not only the gates.** A second script read each
draft's `practises` list and compared it against the live continuity
measurement: 88 atom-passes against 88 slots required, zero under-covered, zero
strays — proved before a single file entered the repo.

The rule: **when a gate is cheap to model and expensive to trip, model it.**
Cheap means the inputs are pure functions of the file. Expensive means a 30s
validator round trip, a 6-minute test suite, or a 25-minute CI cycle. Copy the
predicate out of the source rather than re-deriving it from the error messages,
and check the copy against output you already trust.

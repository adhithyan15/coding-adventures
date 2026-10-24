---
category: Repo policy / workflow reminders
---

# A chapter number does not determine a human-language lesson's CEFR stage

I inferred that Marwadi chapter 84 was A1 from its ordinal and filed a
late-only mock-token issue for `गांव`. The actual `MW-C84-gav` membership points
to `MW-EXT-84-TOWN84`, whose declared stage is `pre-A1`, so the issue was a false
positive and was closed with a correction. Before asserting that an assessment
uses a word from a later rung, resolve the lesson's curriculum-membership path
and extension to its declared stage. Chapter counts and changelog summaries are
not a substitute for that source-backed lookup.

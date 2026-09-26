---
category: Repo policy / workflow reminders
---

# An interspersed lesson may carry only one Writing segment, so a second writing-shaped heading has to become a Script heading

A retrieval lesson placed in arabic's writing chapter was written with two blocks
whose headings began `## Writing:` — one for the letter family, one for the mark
that is not a vowel. Every offline check passed. `tests/modality.test.ts` did not:

    modality-writing-segment-not-separable
    AR-R45-the-last-page: carries 2 writing segments (...); an interspersed
    lesson may carry one, otherwise make it a type: writing lesson

The rule is about **separability**. A lesson whose type is `writing` is a writing
lesson and may carry as many writing segments as it needs. A lesson of any other
type that is *interspersed* among non-writing lessons may carry **one**, so the
modality manifest can hand it to a single owner.

**The fix is a heading, not a type.** `## Script:` and `## Writing:` are both legal
block prefixes in `classifyBlock`, and only the second is counted as a writing
segment. Renaming the second block to `## Script: …` cleared the finding and
changed nothing a learner sees — both blocks were about letter shapes anyway.

Changing `type: review` to `type: writing` would also have cleared it, and would
have been the wrong fix: `review` sits outside `CONTENT_TYPES`, which is exactly
what keeps a retrieval page from moving the track's vocabulary, idiom, sense and
culture-claim budgets. Retyping the lesson to satisfy a heading rule would have
dragged all four counters with it.

**The general shape.** When a corpus gate names two legal spellings and rejects the
combination, check which spelling the gate actually counts before reaching for the
frontmatter. The cheap fix is usually in the prose.

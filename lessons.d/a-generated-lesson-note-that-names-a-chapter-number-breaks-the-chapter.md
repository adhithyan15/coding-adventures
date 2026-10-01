---
category: Repo policy / workflow reminders
---

# A generated lesson note that names a chapter number breaks the chapter-references baseline

**What happened:** the Portuguese A2 spine chapters were written from a compact
spec, and one hand-written note said "**escrevi** is the one-word past of
chapter 15". CI's `build (ubuntu-latest)` then failed in
`tests/chapter-references.test.ts`:

> portuguese: cross-chapter prose references must not grow past 63: expected 64

That test (HL-C102) pins every track's count of lesson prose that names
another chapter by number. Such pointers rot silently whenever chapters
renumber. My local validation ran only a targeted set (the track's corpus
test, level-gate, chapters, payoff-summary-case and membership-shards). That
set did not include `chapter-references`, so the regression reached CI.

**Fix:** name the thing instead of the number: "**escrevi** is the one-word
past, like **falei**".

**What to do differently:**

1. In a note, never point at another chapter by number. Name the lesson's
   content ("the one-word past", "when you first met it") or drop the pointer.
2. Add `tests/chapter-references.test.ts` to the targeted test set for any
   change that adds or edits lesson prose. It runs in seconds.
3. Before writing notes for a generator spec, grep the spec for `chapter \d`.

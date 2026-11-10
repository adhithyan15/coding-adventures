---
category: Testing & coverage
---

# A review prompt added to an early-script lesson must stay romanized when its target glyph is not yet taught

While repairing Kannada chapter 12's payoff metadata, the final lesson gained a
second family review cue written as **ಅವರ ಅಕ್ಕ**. The words were already owned,
but the lesson appears before Kannada's script runway teaches initial ಅ. The
focused inventory test passed while `script-closure.test.ts` correctly reported
one new decoding violation.

Early spoken review prompts must follow both the knowledge boundary and the
script boundary. Keep an already-owned phrase romanized when any of its glyphs
is still untaught at that sequence, then run the full script-closure gate after
every prose edit to a pre-runway lesson. Here the honest repair was *avara akka*:
the family atoms remain assessed without asking the learner to decode a glyph
the book has not taught yet.

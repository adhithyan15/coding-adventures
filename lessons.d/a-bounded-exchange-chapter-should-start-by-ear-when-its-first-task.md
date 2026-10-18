---
category: Testing & coverage
---

# A bounded exchange chapter should start by ear when its first task does not require sight

Sanskrit chapter 274 taught an attested rental exchange as a whole, then asked
the learner to “Read the question and answer once slowly.” That single verb made
the chapter's opening lesson sight-only. The focused Sanskrit suite and every
generated-artifact check passed, but the full CI suite correctly rejected a
ninth chapter that could not start while driving: the corpus ceiling is eight
and may only fall.

The task did not actually need print. Its goal was hearing and saying a complete
exchange, while the two later retrievals were already voice-drivable. Replacing
the instruction with “Listen to the question and answer once slowly” made the
opening voice-drivable without weakening the source, answer-delay, or mastery
boundaries.

Before adding a bounded conversation chapter, run `tests/drivable-prefix.test.ts`
in addition to the language wrapper. If the first task only asks learners to
hear and repeat, say that explicitly. Do not introduce a visual dependency with
“read,” “look,” or “see” merely because the authored lesson also has a printed
form.

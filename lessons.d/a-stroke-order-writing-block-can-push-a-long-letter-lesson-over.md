---
category: Repo policy / workflow reminders
---

# A stroke-order writing block can push a long letter lesson over the 300-second duration budget

Replacing a "copy what you see" writing block with a cited stroke order adds
words: one sentence per movement (each numbered line also counts its number
as a word), a "Pen lifts" note and the source quote. For short letter lessons
that is harmless, but in the third Kannada consonant batch KA-S128 (ಝ, nine
movements) and KA-S129 (ಥ, seven movements plus a slug note) were already
long, and the new blocks took their computed duration to 320s and 317s. The
full human-language-data suite then failed three tests: the integration
spine check, the gap report's `durationViolations`, and `runValidate` on the
real curriculum. The script-ductus suite and the Kannada-only tests passed,
so the failure only showed in the full suite.

The fix was to cut the added prose, never the ceiling: shorter pen-lift and
slug notes, and three shorter stroke-order sentences (kept identical in the
record, the lesson and the evidence pins), which brought them to 297s and
298s.

What to do differently: before regenerating books, measure each edited
lesson with `estimateLessonDuration` from `dist/report.js` (computed =
ceil((words/2 + 8*prompts + 4*repeat cues + pauses) * 1.15), and the error is
at 300s or more). Long letters with many movements, in lessons that already
carry a long introduction, need the shortest notes.

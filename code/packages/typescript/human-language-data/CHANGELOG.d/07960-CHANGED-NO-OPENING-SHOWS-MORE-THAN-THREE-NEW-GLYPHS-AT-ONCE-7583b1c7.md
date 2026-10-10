### Changed — no opening shows more than three new glyphs at once

The script-ramp report's over-budget list (`ramp.script.lessons`, policy
`maxNewGlyphsPerLesson: 3`) falls from 28 lessons to 0:
`summary.scriptRampOverBudgetLessons` 28 -> 0, and the gentle-ramp
`glyph-step` queue empties. The
27 lessons fixed span twelve tracks: the chapter-1 greetings of Bengali,
Gujarati, Hindi, Kannada, Marathi, Persian, Punjabi, Russian, Sanskrit, Tamil,
Telugu and Urdu, plus four letter-list lessons (TE-C16-nelalu,
KA-S122-letter-ca, BN-C02-alaap, KA-C02-santosha). The 28th, Chinese
ZH-C01-ni, fell under budget with the Chinese chapter-1 rewrite this branch
was rebased onto.

The fixes follow the Malayalam and Gujarati openings: a greeting stays
meaning-first under a romanized heading, shows only the shapes the lesson
teaches or traces (never more than three new), and says where the whole
spelling arrives; the shapes it no longer shows now first appear in the
lessons that teach them. Long preview lists were split across adjacent
lessons (Kannada's month names over the ಚ and ಪ lessons; Telugu's over the
rare-letter lessons that already follow the months lesson).

`src/ramp.ts` is unchanged, and no other gate moved the wrong way: script
closure stays 0, exposure-exempted glyphs stay 1,686,
duration violations stay 0 (longest edited lesson 293 s computed), chapter
gates, writing-ramp, modality counts and payoffs are unchanged except that one
more lesson carries a writing segment (MR-C01-namaskar's trace).
`continuity.forwardReferences` falls 1175 -> 1173 and metalanguage technical
uses before introduction 6587 -> 6586.

Tests: the steepest-lesson pin in `tests/ramp.test.ts` (TE-C01-namaskaram at
8) becomes a gate at zero — no lesson over budget, no steepest lesson. In
`tests/script-closure.test.ts`, "the pace budget is not vacuous" was read off
the real corpus (`lessonViolations > 0`) and pointed the wrong way once this
burn-down paid that debt; it is now shown on a one-lesson synthetic track that
puts four new Tamil shapes in front of the learner and trips the budget
exactly once. The Sanskrit writing-ladder comment describes the new lesson-one
trace.

Driver safety: RU-C01-privet's wrap-up recall asks which letter looks like
Latin B and which like Latin p, spoken, instead of asking the learner to point
at them — its core is marked drivable.

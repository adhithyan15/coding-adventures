### Changed — the headword-romanization ceiling falls from 25 to 3

`tests/script-closure.test.ts` holds `headwordsWithoutRomanization` under a per-track
ceiling. Sixteen Arabic lessons (chapters 1–26) and six Marathi chapter 25 form
lessons now declare a romanization, so the ceiling drops from arabic 16, marathi 8,
hindi 1 to marathi 2, hindi 1, and Arabic leaves it (its debt is zero, so the
"names only tracks that actually carry the debt" test would otherwise fail).

The three that remain are named in the test as deliberate rather than debt: each is a
model-free writing checkpoint whose own text promises the learner no romanization —
HI-A1F01-name-no-model (already pinned romanization-free by
`tests/corpus/hindi-a1-form.test.ts`) and MR-A1F10/MR-A1F11, whose headwords are the
completed answer rows. Paying them means a headword that is not the answer, as
MR-A1F13-six-field-independent has, not a romanization.

Report deltas: `headwordsWithoutRomanization` 25 -> 3. Unchanged: script-closure
violations 0, `exposureExemptedGlyphs` 1686 and `exposureOnly` 714 (every headword
glyph of the sixteen Arabic word lessons was already taught by an earlier script
lesson, and the six Marathi lessons are script lessons themselves, so the exemption
removes nothing new), `scriptRampOverBudgetLessons` 0, duration violations 0, chapter
gates unchanged.

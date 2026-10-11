### Fixed — the closure test stops requiring debt that has been paid

- **Why.** The Russian, Persian and Arabic openings, Urdu, Punjabi and two
  Chinese lessons stopped asking for untaught letters, and the corpus's
  script-closure violations fell 195 -> 2. `tests/script-closure.test.ts`
  asserted that closure finds more than five times the script-ramp
  violations, a comparison the debt was always meant to fall through. It
  failed because debt was paid, the failure its own comment warns about.
- **The claim, demonstrated instead of read off the corpus.** A two-lesson
  synthetic track, each lesson inside the glyph budget and neither a script
  lesson, passes the pace budget (0) and fails closure (2). The corpus is
  still required to have pace-budget work left, so the two measurements are
  not agreeing only because both are empty.
- **Ceilings lowered, re-measured:** corpus violations 271 -> 2, and Arabic
  headwords without a romanization 37 -> 16 (twenty-one lessons now declare
  the romanization their titles already gave).
- `src/script-closure.ts` is unchanged.

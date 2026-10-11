### Fixed — lesson prose cannot carry "?" where a word belongs

- New `src/lost-script.ts` (`measureLostScript`, `renderLostScript`,
  `SPACED_QUESTION_MARK_LANGUAGES`, exported from the package index) and its
  gate, `tests/lost-script.test.ts`. Twenty-six Japanese lessons had shipped
  with their kana and kanji replaced by ASCII question marks, one per lost
  character ("Say ??? and tap all three morae", "Write ? from memory before
  tracing ?.", "?hard to exist?"), through every existing gate and into the
  published book and narration: the damage is valid UTF-8, Markdown and
  LaTeX, and `check:books` faithfully reproduced it.
- The module flags three shapes of `?` standing where a word belongs, in
  lesson bodies (including `hl-activity` prompts) and in the generated book's
  `.tex`:
  - **run** — two or more marks not following a word (`Say ??? and`,
    `**??**`). Doubled punctuation after a word (`Why?? (**quārē**.)`) is
    not lost script and passes.
  - **glued** — a mark with a letter straight after it (`?hard to exist?`,
    `espa?ol`).
  - **lone** — a mark with whitespace before it and the sentence carrying on
    after it (`write ? once`, `Write ?, then`, `tracing ?.`), across a
    wrapped line too.
- Allowances, each forced by a real line in the corpus: the lone rule is off
  for French, whose typography spaces the mark ("Combien ? has had no
  possible answer"); a `?` paired with `¿` ("the ¿ ? marks") or named beside
  its sibling marks ("# ? and ! — …") passes; code spans, fenced code and URLs
  are blanked first. Run and glued stay on for every track.
- Every pattern is linear: no nested quantifier, and fences are blanked by an
  `indexOf` scan rather than a lazy `[\s\S]*?` regex. The test drives each
  pattern and blanking pass with nine 50,000-character adversarial inputs.
- The corpus gate reads the committed `.tex` (which `check:books` pins to the
  generator byte-for-byte) rather than rendering every book again. It names
  each finding and pins that one planted line fires once.
- Audit of every track before the gate landed: real damage only in the 25
  Japanese review pulses (restored in this change; reasoning in the Japanese
  track changelog) and JA-C01-practice (restored in the drive-debt change).
  Not damage: the doubled `Why??`, `Which??`, `Where??`, `How much??`,
  `Of what kind??` and `Surely ... not??` in LA-R119, SA-R190, FA-R88, FA-R98
  and FA-R139 (a gloss ending in `?` meets the prompt's `?`, a cosmetic slip
  left for its own change); French's spaced `?`; the quiz and table blanks in
  ES-C03-como and GU-W22-ttha; the punctuation lessons of Malayalam, Kannada,
  Marathi and Russian; TypeScript `??` in backlog entries; URL query strings.
  The ASCII-only `spanish/changelog.d/a1-vocab-04.md` writes `¿Cuántos años`
  as `?Cuantos anos` on purpose throughout and is not lesson prose.
- Regenerated: Japanese book chapters 1-6, narration ch01-ch06, their
  generated book and narration hashes, and 25 `core/lesson-modality` owners.

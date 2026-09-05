## Canonical book Chapters 6–33

- Fifty-one lessons now use schema v2 with explicit shared-spine placement,
  prerequisite-safe sequence numbers, honest sub-five-minute duration budgets,
  typed teaching blocks, and machine-checkable knowledge boundaries.
- Forty lessons across Chapters 6–33 generate twenty-eight LaTeX chapters from
  the same canonical AST loaded by Language Ladder. Per-chapter source hashes
  make app/book drift a test failure.
- Eleven writing companions in Chapters 1–2 also migrated to schema v2, but
  remain embedded in the hand-authored opening chapters so script appears only
  when the learner needs it rather than as a detached alphabet course.
- Devanagari, Arabic, and Cyrillic examples use vendored fonts. The shared
  renderer also handles stacked accents and historical-linguistics notation;
  the 114-page XeLaTeX build has zero missing glyphs.
- Canonical coverage now runs continuously through Chapter 33. The curriculum
  report remains at zero duration violations and zero unknown prerequisites,
  while lesson chapters missing from books fall from 104 to 76.


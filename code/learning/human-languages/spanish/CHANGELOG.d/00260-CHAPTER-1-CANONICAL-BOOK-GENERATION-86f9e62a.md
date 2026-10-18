## Chapter 1 — canonical book generation

- Replaced the handwritten first chapter with deterministic LaTeX rendered
  from its seven schema-v2 lesson ASTs in prerequisite-safe sequence order.
- Added a committed FNV-1a source fingerprint manifest shared with Language
  Ladder, plus a CI drift check that rejects stale generated TeX.
- Built the 122-page XeLaTeX book and visually checked the generated opener,
  grammar, culture, guided-practice, and recall layouts.


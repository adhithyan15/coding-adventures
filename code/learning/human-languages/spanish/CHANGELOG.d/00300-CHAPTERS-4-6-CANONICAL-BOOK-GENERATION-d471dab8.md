## Chapters 4–6 — canonical book generation

- Replaced three handwritten LaTeX chapters with deterministic output from all
  27 schema-v2 well-being, writing, farewell, and first-verb micro-lessons.
  Language Ladder now verifies the same source hashes across all 51 lessons in
  generated Chapters 1–6.
- Added width-aware Markdown-table rendering so question families, register
  choices, farewell timing, and verb conjugations keep their row-and-column
  meaning in print. The renderer also emits a safe LaTeX approximation symbol.
- Corrected two headerless canonical verb tables and removed three unsupported
  glyphs exposed by the first forced build without weakening the etymology.
- Forced the complete XeLaTeX build to 158 pages and visually inspected every
  page in Chapters 4–6. The generated span has no missing glyphs, layout-box
  warnings, bookmark warnings, clipping, collisions, or malformed tables.


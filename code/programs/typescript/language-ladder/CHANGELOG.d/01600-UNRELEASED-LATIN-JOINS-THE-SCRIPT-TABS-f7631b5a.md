## Unreleased — Latin joins the script tabs

- The canonical Latin inventory (`data/scripts/latin.json`) is the new last
  entry of `SCRIPTS`, after Gurmukhi, so the app shows a Latin tab: the
  Latin-script tracks' 57 rows (letters, ª º, ¿ ¡), with the prose stroke
  order of the 18 cited glyphs. No existing script moves index.
- New `tests/glyph-evidence/097-latin.evidence.ts` pins that position, the
  row count and the cited rows' sources. `096-gurmukhi` now pins Gurmukhi as
  the second-to-last script, and `095-bengali` pins Bengali third from last.

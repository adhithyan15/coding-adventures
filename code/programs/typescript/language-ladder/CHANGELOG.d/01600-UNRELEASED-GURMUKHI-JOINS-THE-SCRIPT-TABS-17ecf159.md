## Unreleased — Gurmukhi joins the script tabs

- The canonical Gurmukhi inventory (`data/scripts/gurmukhi.json`) is the new
  last entry of `SCRIPTS`, after Bengali, so the app shows a Gurmukhi tab.
  The tab lists the Punjabi track's 33 letters, with the prose stroke order
  of the 27 cited letters. It shows no figure, since the app does not load
  the Gurmukhi font. No existing script moves index.
- New `tests/glyph-evidence/096-gurmukhi.evidence.ts` pins that position, the
  letter count and the cited rows' GNPS source. `095-bengali` now pins Bengali
  as the second-to-last script.

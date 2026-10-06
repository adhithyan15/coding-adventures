## Unreleased — Bengali joins the script tabs

- The canonical Bengali inventory (`data/scripts/bengali.json`) is the last
  entry of `SCRIPTS`, so the app shows a Bengali tab: the track's 30
  letters, with the prose stroke order of the seven cited letters and no
  figure, since the app does not load the Bengali font. No existing script
  moves index.
- New `tests/glyph-evidence/095-bengali.evidence.ts` pins that position, the
  letter count and the cited rows' LipiTk source.

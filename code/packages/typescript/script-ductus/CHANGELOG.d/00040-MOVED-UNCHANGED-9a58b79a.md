### Moved, unchanged
- `strokes.ts` — hand-authored pen paths, their labelled segments, pen lifts, and
  the citation each stroke order carries.
- `truetype.ts` — zero-dependency TrueType reader for the real glyph outline.
- `ductusview.ts` — the filmstrip builder and SVG serialiser.
- All three test files, intact. The font-verification tests are the point of the
  package: `fractionOnInk`, consecutive-segment-meeting, and whole-ink coverage
  are what keep a hand-authored pen path honest, and they moved across without
  being loosened.


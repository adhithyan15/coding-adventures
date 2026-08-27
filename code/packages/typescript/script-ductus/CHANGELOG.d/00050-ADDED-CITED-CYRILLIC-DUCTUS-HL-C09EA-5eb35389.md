### Added — cited Cyrillic б ductus (HL-C09EA)

- Render **б** as a counterclockwise lower body followed by a rising shoulder and rightward top flag in one zero-lift stroke.
- Preserve RussianIrina's 01:13–01:18 body-to-flag order while routing the handwritten diagonal transition through Noto Sans Cyrillic's printed upper-left shoulder.
- Add source, font-routing, on-ink, whole-glyph, and two-frame filmstrip coverage; the focused suite now passes 963 tests.

## 0.1.0 — extracted from language-ladder

The package exists so that something other than the app can ask *how is this
letter written?*

`strokes.ts`, `truetype.ts` and `ductusview.ts` lived in
`code/programs/typescript/language-ladder/src`. Nothing under `code/packages/`
may depend on something under `code/programs/`, so the book generator — the other
consumer that wants filmstrips, as printed figures rather than a live SVG — could
not import them at all. `ductusview.ts`'s own header already anticipated this:
*"the book pipeline can take the serialised string instead."*


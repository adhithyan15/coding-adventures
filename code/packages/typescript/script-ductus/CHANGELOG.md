# Changelog

## Unreleased

### Added — cited Cyrillic в ductus (HL-C09EB)

- Render **в** as a baseline-to-upper-loop return followed by a counterclockwise lower bowl in one zero-lift stroke.
- Preserve RussianIrina's 01:33–01:38 school-hand order while fitting its tall cursive ascender loop through Noto Sans Cyrillic's compact printed upper bowl and left stem.
- Add source, font-routing, on-ink, whole-glyph, and two-frame filmstrip coverage; the focused suite now passes 970 tests.

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

### Moved, unchanged
- `strokes.ts` — hand-authored pen paths, their labelled segments, pen lifts, and
  the citation each stroke order carries.
- `truetype.ts` — zero-dependency TrueType reader for the real glyph outline.
- `ductusview.ts` — the filmstrip builder and SVG serialiser.
- All three test files, intact. The font-verification tests are the point of the
  package: `fractionOnInk`, consecutive-segment-meeting, and whole-ink coverage
  are what keep a hand-authored pen path honest, and they moved across without
  being loosened.

### Moved with them
- `data.ts` → `scriptdata.ts`, plus the `Letter` and `ScriptData` types that
  describe the curriculum's script JSON files. They belong beside the pen paths:
  a letter's stroke order is verified against the very font its `ScriptData`
  names, so a test can assert the two agree only if both live here.
  `language-ladder/src/types.ts` re-exports them, so every existing
  `from "./types.ts"` import in the app is unchanged.

### Changed in the app
- `main.ts` and nine test files import from `@coding-adventures/script-ductus`.
- `vite.config.ts`'s `handwriting-tools` manual chunk matched
  `language-ladder/src/(strokes|ductusview|truetype).ts` by path. Repointed at
  the package — `check:bundle` caught it, and without the fix 7,600 lines of
  handwriting code would have moved into the interactive shell rather than
  loading when a learner opens a letter. `scriptdata` is deliberately NOT in that
  chunk: the shell needs `SCRIPTS` on first paint.
- `BUILD` chain-installs the new package before the app's own `npm install`.

### Note on jsdom
`jsdom` is a devDependency for exactly two tests: the SVG serialiser's escaping is
checked by handing its output to a real parser and asserting a hostile caption
cannot break out of an attribute or smuggle in a `<script>`. A string comparison
would pass on markup no browser accepts, which is the bug those tests exist to
catch — so the environment moved with them rather than the tests being weakened
to fit a Node-only config.

845 tests pass in the package; the app's 725 tests, typecheck, build and
`check:bundle` all pass unchanged.

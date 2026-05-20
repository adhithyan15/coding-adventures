# Changelog — mosaic-emit-html

## 0.2.0 — 2026-05-19

Add the three-language pipeline path alongside the legacy single-file `.mosaic`
renderer. Prerequisite for UI29 kernel primitive support.

- New `pipeline` module exposing `from_pipeline(interface, layout, style)`,
  taking the `MosmodelComponent` + `LayoutDef` + `StyleDef` triple.
- New public types `PipelineEmitResult` and `PipelineEmitError`, re-exported
  from `lib.rs`. Variants: `ComponentNameMismatch`, `UnknownPrimitive`.
- Primitive coverage in this first cut: `Box`, `Row`, `Column`, `Text`,
  `Image`, `Spacer`, `Divider`, `Icon`. UI29 kernel additions are a follow-up
  PR. Unknown primitives error out — silent fallback is not in scope.
- **Slot interpolation:** slot refs become Handlebars-style `{{slotName}}`
  template tokens. The host either pre-substitutes them server-side or pipes
  the output through a downstream JS hydrator (out of scope).
- **Style strategy:** mosstyle parts flatten to inline `style="..."`
  attributes on the matching element. Built-in primitive styles (e.g. the
  flexbox defaults for `Row`/`Column`) merge with the author's part style;
  author wins on collisions (last-property-wins, matching CSS specificity).
- **Emit refs are silently dropped** with a `<!-- emit "<name>" dropped: HTML is static -->`
  comment. Static HTML has no analog for the Flux dispatch callback.
- The output is an HTML *fragment* — no `<!DOCTYPE>` / `<html>` / `<body>`
  wrapping. A `<div data-mosaic-component="<Name>">` wrapper sits at the
  outermost level for hydration targeting.
- New dependencies: `mosmodel-compiler`, `moslayout-compiler`, `mosstyle-compiler`.
- 14 new unit tests covering: empty box, slot-ref placeholder, flex styles,
  literal image src, slot-ref image src, part-style flattening,
  camelCase→kebab CSS normalisation, nested tree order, name mismatch,
  unknown primitive, emit drop comment, void elements, HTML escaping,
  banner line. All passing.

## 0.1.0 — 2026-05-11

Initial release.

- Implement `HtmlRenderer` — pure HTML static snapshot backend for the Mosaic compiler.
- Implements `MosaicRenderer` trait; driven by `MosaicVM`.
- Fixture JSON support: slot values resolved from `serde_json::Map` at compile time.
- All Mosaic primitives mapped to HTML elements with inline `style=""` attributes:
  Box, Column, Row, Text, Image, Spacer, Scroll, Divider, Stack, Icon, Grid.
- `when` blocks: compile-time suppression based on fixture boolean value.
  Missing fixture defaults to `true` (show content for design review).
- `each` blocks: first fixture array element substituted for loop variable (v1).
- Grid → static `<table>` with `<thead>`/`<tbody>` populated from fixture arrays.
- HTML escaping via `html_escape()` for all user-provided slot values.
- CSS inlining: optional CSS string placed in `<style>` block; falls back to
  a minimal reset (`box-sizing: border-box; body margin: 0; font-family: sans-serif`).
- Full `<!DOCTYPE html>` document output with `<html lang="en">`, `<head>`, `<title>`, `<body>`.
- 17 unit tests, all passing.

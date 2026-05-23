# Changelog — mosaic-emit-html

## [Unreleased]

### Added — U29-2-K-html — `HostCheckbox` + `HostRadio` kernel primitive lowerings

Both new UI29-2 primitives lower to native HTML form controls:

- `HostCheckbox` → `<input type="checkbox" …>`
- `HostRadio`    → `<input type="radio" …>`

Static HTML has no JS runtime, so the standard pattern from
`HostInput` and `HostButton` is reused — slot-typed props become
`data-*` markers the host's template engine or hydration pass can
post-process:

- `checked: true|false` keyword → bare `checked` attribute or omitted.
- `checked: slot: c` → `data-checked="{{c}}"` template marker.
- `disabled: true|false` / `slot: d` → bare `disabled` / `data-disabled` marker.
- `label: …` (string or slot) → wraps the input in `<label><input …> body</label>`
  (idiomatic; no id+for pair to invent).
- `onToggle`/`onSelect: emit: onX` → `data-on-toggle="onX"` /
  `data-on-select="onX"` marker for hydration to bind real listeners.

HostRadio-specific:

- `group: "name"` / `slot: g` → real HTML `name=` attribute. The
  browser enforces the radio-mutex for free when multiple radios
  share a `name` — no script needed.
- `value: "v"` / `slot: v` → real HTML `value=` attribute (the
  form-submit value).

HostCheckbox-specific:

- `indeterminate: true` / `slot: i` → `data-indeterminate="true"` /
  `data-indeterminate="{{i}}"`. There is no HTML attribute for
  `indeterminate` — it's a JS DOM property — so the marker lets the
  host's hydration script set `el.indeterminate = …` imperatively.

10 new tests cover: bare inputs, `checked: true` keyword, `checked:
slot` data marker, label wrapping, `onToggle` data marker,
`indeterminate` data marker, bare radio, `group:` → `name=`,
`value:` → `value=`, `onSelect:` data marker.

## 0.3.0 — 2026-05-19

U29-K-html — UI29 kernel primitives in the pipeline emitter. Extends the
three-language pipeline path (added in 0.2.0) with the seven UI29 kernel
primitives beyond the original Box/Row/Column set.

- New host primitives in the pipeline path:
  - `Stack` → `<div style="position: relative">` (children layered)
  - `HostInput` → `<input type="text" value="{{value}}" ...>` with
    optional `value` / `placeholder` / `read-only` props (slot ref,
    string literal, or keyword `true`/`false` per prop)
  - `HostButton` → `<button>{{label}}</button>` with optional
    `disabled` keyword/slot-ref handling
  - `HostScroll` → `<div style="overflow: auto">`
  - `HostTable` → semantic `<table>` with sub-tags
    `HostTableColGroup` (`<colgroup>` + `<col>` children),
    `HostTableHead` (`<thead>` with `<tr>`/`<th>`),
    `HostTableBody` (`<tbody>` with `<tr>`/`<td>`),
    `HostTableFoot` (`<tfoot>` with `<tr>`/`<td>`)
- New meta-primitives:
  - `For (each: <expr|slot>, as: <name>, index: <name>?)` lowers to a
    `<!-- mosaic-for each="..." as="..." index="..." -->` … `<!-- /mosaic-for -->`
    comment wrapper. The host's template engine resolves the loop at
    render time.
  - `If (when: <expr|slot>)` lowers to `<!-- mosaic-if when="..." -->` …
    `<!-- /mosaic-if -->`. Literal `when: true` / `when: false` is folded
    at compile time (no markers, just the chosen branch).
  - `Else` is recognised as a sibling of `If` and consumed by the
    sibling walker; an orphan `Else` emits a diagnostic comment marker.
- The `If` / `For` comment-marker shape is a deliberate static-HTML
  compromise — HTML has no runtime conditional or loop. A future
  Mosaic HTML runtime PR could replace the markers with client-side
  JS expansion; the marker format is neutral about what resolves it.
- 14 new tests added (45 total in the pipeline test module), covering
  every primitive listed above plus the literal-fold and expression
  round-trip paths. All existing pipeline + legacy tests pass unchanged.

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

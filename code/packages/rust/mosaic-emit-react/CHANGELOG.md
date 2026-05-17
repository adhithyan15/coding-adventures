# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased] - UI24: Flux dispatch union

### Changed (breaking)

- `ReactRenderer::begin_component` now accepts `emits: &[MosaicEmit]`
  matching the updated `MosaicRenderer` trait signature.
- Generated `interface ComponentProps` now includes a required `dispatch:
  (event: ComponentEvent) => void` prop as the last entry, replacing the
  per-emit optional callback props from the earlier design.
- Generated output now includes a `type ComponentEvent = | ...` discriminated
  union before the interface.  Zero-emit components get `type ComponentEvent =
  never`.

### Added

- `ReactRenderer::emit_name_to_type_field` — converts `onNavigate` →
  `"navigate"` (strip `on` prefix, lower-case first char).
- `ReactRenderer::generate_event_union` — produces the `type XEvent = ...`
  block.
- `ReactRenderer::generate_props_interface` updated to append `dispatch`
  as the final required prop.
- 11 new unit tests covering the dispatch union pattern (tests 17–27):
  zero-emit `never` type, dispatch always present, event union type, type
  field conversion, void emits, multi-param emits, kebab→camelCase params,
  union-before-interface ordering, required dispatch, dispatch in function
  params.

### Fixed

- Clippy: replaced `format!("<hr />")` with `"<hr />".to_string()`.

## [0.1.0] - 2026-05-11

### Added

- `ReactRenderer` struct implementing `MosaicRenderer` to produce React
  functional components as `.jsx` strings.
- Mosaic node → JSX mapping: Box/Column/Row → `<div>` with flex styles,
  Text → `<span>`, Image → `<img />`, Spacer → `<div style={{flex:1}}>`,
  Scroll → `<div style={{overflow:'auto'}}>`, Divider → `<hr />`,
  Stack → `<div style={{position:'relative'}}>`, Icon → `<span className="icon">`,
  Grid → `<table>` with `.map()` header/row rendering.
- Slot → TypeScript prop mapping: text→string, number→number, bool→boolean,
  image→string (URL), list<T>→Array<T>, node→React.ReactNode.
- TypeScript `interface ComponentProps {}` generated from slot declarations.
- `when @slot { ... }` → `{slot && (<>...</>)}` conditional JSX.
- `each @items as item { ... }` → `{items.map((item, _idx) => (<>...</>))}`.
- camelCase conversion for all slot names and CSS properties.
- Wired into `mosaic-compile` CLI as `--backend react` (outputs `ComponentName.jsx`).
- 16 unit tests covering all node types, slots, when/each blocks, and versioning.

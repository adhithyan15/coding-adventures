# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- `pipeline::from_pipeline(&MosmodelComponent, &LayoutDef, &StyleDef)` —
  new entry point implementing UI24 (Flux event dispatch). Generates a
  TSX file with a per-component `<Name>Event` discriminated union and a
  single required `dispatch` prop, replacing the N-callback pattern of
  the legacy `ReactRenderer`. Lives alongside (does not replace) the
  legacy `MosaicVM`-driven path.
- 21 unit tests covering: event union shape, dispatch-prop-is-required,
  destructuring order, `on`-prefix stripping, camelCase param conversion,
  void emit variants, zero-emit components (`type X = never`), reserved
  emit name rejection (`dispatch`/`children`/`key`), node-slot optionality,
  primitive lowering for Box/Row/Column/Text/Image/Spacer/Scroll/Divider/
  Stack/Icon, `Text { content: @slot }` → `<span>{slot}</span>`, nested
  containers, unknown-primitive errors, and component-name-mismatch
  validation across the three IRs.
- Dependencies on `mosmodel-compiler`, `moslayout-compiler`, and
  `mosstyle-compiler` to consume their public IR types.

### Known limitations of the first-cut pipeline path

- `Grid` and `Input` primitives lower to placeholder `<div data-mosaic-todo>`
  elements; real renderers land in follow-up PRs.
- `connects` clauses are not yet wired to JSX event handlers; the
  function body renders structure but does not yet call `dispatch(...)`
  in response to clicks/keystrokes.
- mosstyle properties are not yet inlined into JSX `style={{...}}`
  objects (the `StyleDef` argument is accepted to lock the signature but
  not yet consumed).

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

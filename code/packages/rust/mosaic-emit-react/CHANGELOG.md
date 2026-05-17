# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added — Grid primitive (pipeline path, UI26 §6.2)

- The moslayout `Grid` primitive now lowers to a full `<table>` with
  `<thead>`/`<tbody>` and nested `.map(...)` callbacks, replacing the
  previous placeholder `<div data-mosaic-todo="grid">` element.
- Required slot refs: `headers` and `rows`. Optional slot refs: `selected-row`,
  `selected-col`, `edit-row`, `edit-col` — when bound, each cell gets a
  conditional inline style that paints the selection / editing highlight.
- Optional emit ref: `onNavigate` — when bound, each `<td>` gets an
  `onClick={() => dispatch({ type: "...", row: r, col: c })}` handler.
- Part style attaches to the `<table>` element itself.
- 9 new tests cover: bare Grid → `<table>`, headers `.map()` for `<th>`,
  nested rows `.map()` for `<tr>` + `<td>`, onNavigate dispatch wiring,
  selected-row/col highlight expression, edit-row/col highlight expression,
  part style on table, missing-headers error, missing-rows error.
- End-to-end smoke test: a 3-file Grid component with selection slots
  and onNavigate emit compiles to a `<table>` with header `.map()`,
  body nested `.map()`, conditional cell selection style, and per-cell
  dispatch handler. Verified.

### Known limitations (deferred)

- No `column-widths` binding — column widths are flex-default. Tracked
  for a follow-up once moslayout supports per-column width arrays.
- No inline `<input>` editor inside cells when `edit-row`/`edit-col`
  matches — the host is expected to render an external editor (e.g. via
  FormulaBar) and push state back via the `edit-*` slots, matching
  UI26 §7.5.
- No `viewport-offset` virtualisation — the emitter renders all rows
  passed in the `rows` slot. Row-windowing is the host's responsibility.
- Highlight colours are hard-coded (`#264f78` selection, `#1f4f3f`
  editing). A future iteration will inline the design-token values from
  mosstyle instead.


### Added — Input primitive (pipeline path, UI25)

- The moslayout `Input` primitive now lowers to a real `<input type="text" />`
  (or `<textarea />` when `multiline: true`), replacing the previous
  placeholder `<div data-mosaic-todo="input">`.
- Bound attributes: `value={...}` from slot refs, `readOnly={...}` from
  either slot refs or `true`/`false` keyword literals, `maxLength={N}`
  from numeric props.
- `onChange` emit refs are special-cased to carry `e.target.value` as the
  dispatch payload — i.e.
  `onChange={e => dispatch({ type: "change", value: e.target.value })}`.
- `onCommit` and `onCancel` emit refs are merged into a single
  `onKeyDown` handler keyed on Enter / Escape, matching the
  UI25 §10 React generated-output example.
- 10 new tests: bare Input, multiline → textarea, value slot binding,
  readOnly slot binding, readOnly literal `true` binding, maxLength
  numeric binding, onChange payload, merged onKeyDown handler, Input
  nested inside Row, part style inlined on Input element.
- End-to-end smoke test: a 3-file `Bar` component (value + readOnly +
  onChange + onCommit + onCancel) compiles to a complete TSX file with
  the correct attributes, payload-carrying onChange handler, and merged
  onKeyDown handler. Verified.

### Known limitations (deferred to follow-up PRs)

- `placeholder: "..."` is not yet supported because moslayout's grammar
  does not yet accept string literals as prop values. Once grammar adds
  string props, the Input emitter will pick up `placeholder` for free.
- Payload-carrying emits other than `onChange` still produce void
  dispatches via the general connects-wiring path; a generic
  `connects: onX(p: text) -> emit onY(p: p)` form in moslayout will
  generalise the special-case.


### Added — connects wiring (pipeline path)

- Every moslayout prop whose value is an `EmitRef(emit_name)` now produces a
  JSX event handler attribute on the enclosing element that fires the
  matching dispatch variant. Example: a `Box (onClick: emit: onClick)`
  layout node now lowers to
  `<div onClick={() => dispatch({ type: "click" })}>...</div>`.
- 6 new tests covering: emit-ref → handler, `on`-prefix stripping on the
  type literal, multiple emit refs on one node, slot refs do NOT produce
  handlers, reserved emit name rejection, handler placement inside the
  opening tag.
- End-to-end smoke test: a minimal `Btn.mil` / `Btn.mll` / `Btn.msl`
  produces TSX that dispatches `{ type: "click" }` on click. Verified.

### Added — style inlining (pipeline path)

- The mosstyle `StyleDef` is now consumed: every layout node whose
  `part_name` matches a `part` block in the `.msl` source receives an
  inline `style={{ ... }}` JSX attribute with the resolved properties.
- kebab-case CSS property names are camelCased for React inline styles
  (`background-color` → `backgroundColor`, `font-size` → `fontSize`).
- Built-in primitive styles (e.g. `Row`'s `display: "flex"`) and
  author-declared part styles are merged into one `style={{ ... }}`
  attribute; author properties appear after built-ins so they win
  collisions (last-property-wins, mirroring CSS specificity).
- Embedded double quotes in style values are escaped to keep the JSX
  string literal well-formed.
- 8 new tests: single prop, multi prop, kebab→camel conversion, missing
  part name (no style), part name absent from style def (no style),
  built-in + author merge, `state` blocks silently ignored (TODO marked
  in the implementation for a future hover/focus PR), quote escaping.
- End-to-end smoke: a 3-file `Panel` component compiles to TSX with
  `<div style={{ display: "flex", flexDirection: "row", backgroundColor:
  "#1e1e1e", padding: "8px" }}>...</div>`. Verified.

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

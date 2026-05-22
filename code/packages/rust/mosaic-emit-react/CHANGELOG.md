# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added — `HostDialog` kernel primitive (UI29-1, U29-1-K-react)

- New `HostDialog` lowering emits React's native `<dialog>` element with
  a paired `useRef` + `useEffect` block that drives `showModal()`/`show()`/
  `close()` from the `open` slot.
  - `modal: true` (default) → `d.showModal()` (top layer + `::backdrop`).
  - `modal: false` → `d.show()` (non-modal popover, in-flow).
  - `open: slot: x` appears in the `useEffect` dep array as `[x]`.
  - `onOpen: emit: onX` dispatches from inside the open branch of the
    effect, in the same tick that calls `showModal()`/`show()`.
  - `onClose: emit: onX` wires the dialog's `onClose` React handler to
    `dispatch({ type: "x" })`.
  - `dismiss-on-backdrop: false` adds `onCancel={e => e.preventDefault()}`
    so Escape / backdrop click does not close the dialog.
  - `title: slot: t` adds an `<h2>{t}</h2>` first child inside the dialog.
  - Children render normally inside the `<dialog>` element via the
    standard children walk (with the title heading inserted first when
    present).
- When *any* `HostDialog` is present in the layout, the emitter adds an
  `import { useRef, useEffect } from "react";` line at the top of the
  generated TSX file. Components without HostDialog continue to omit
  the hook import so strict-mode `noUnusedLocals` hosts stay clean.
- Multiple HostDialogs in the same component get distinct ref names
  (`dialogRef_0`, `dialogRef_1`, ...) assigned in DFS source order.
- 12 new tests covering: empty dialog ref+effect, `open` slot in dep
  array, `modal: true`/`modal: false`, `onClose`, children rendering,
  `title` heading, `dismiss-on-backdrop: false`, hook imports added,
  hook imports omitted when no dialog, multiple dialogs get unique
  ref names, and `onOpen` dispatch placement.

### Added — `For` / `If` / `Else` meta-primitives (UI29 §3.1, §3.2)

- New pipeline emitters for the three control-flow meta-primitives that
  complete the React kernel surface (U29-K-react).
- `For (each: <slot-or-expr>, as: <name>, index: <name>?) { ... }` lowers
  to a JSX `{coll.map((<as>, <index>?) => <body-jsx>)}` expression.
  - SlotRef `each:` camelCases to an identifier; `Expr` `each:` passes
    through verbatim.
  - When `index:` is omitted the callback declares a single parameter.
  - The body renders through the standard tree emitter, so a
    `Text (content: slot: <as>)` reference inside the body resolves to
    `{<as>}` and naturally closes over the callback parameter.
- `If (when: <slot-or-expr>) { <then> }` lowers to `{cond && (<then>)}`
  when no `Else` sibling follows.
- `If` immediately followed by `Else { <else> }` is paired by a small
  sibling-lookahead helper in the container children loop and lowers to
  `{cond ? (<then>) : (<else>)}`.
- An orphan `Else` (no preceding `If`) emits a `{/* Else with no
  preceding If — ignored */}` JSX comment instead of failing the emit.
- 11 new tests covering: slot-ref `each:`, expression `each:`, `index:`
  bindings, body resolving the `as:` name, `If` short-circuit form,
  `If/Else` ternary form, expression `when:`, orphan `Else` comment,
  nested `For` loops, `For` body containing `If/Else`, and a regression
  pin for the plain Box/Text container path.

### Added — Grid sticky header + scroll container (WA5, UI27 §6/§7.3)

- New compile-time keyword prop `sticky-header: true` on the Grid
  primitive. When set, the emitter:
  1. wraps the entire `<table>` in a `<div style={{ overflow: "auto",
     maxHeight: ... }}>` scroll container;
  2. adds `position: "sticky", top: 0, zIndex: 1` to `<thead>`.
- The scroll container takes its bound from a new optional `total-height`
  prop, which accepts either a literal number (`total-height: 600` →
  `maxHeight: "600px"`) or a slot ref (`total-height: slot: viewport-px`
  → `maxHeight: \`${viewportPx}px\``).
- When `sticky-header: true` is set without `total-height`, the wrapper
  still emits with `overflow: "auto"` only; the caller controls the
  scroll bound via its own parent CSS.
- `total-height` without `sticky-header` is a deliberate no-op — sticky
  is the user-facing feature, total-height is its configuration.
- Backwards-compatible: when `sticky-header` is absent or `false`, the
  emitter produces byte-identical output to pre-WA5 (no wrapper, bare
  `<thead>`).
- 5 new tests: literal total-height wraps in scroll div, slot ref
  total-height uses template literal, sticky without total-height omits
  maxHeight but still stickies, no sticky keyword preserves pre-WA5
  output, total-height without sticky is a no-op.
- End-to-end smoke verified via mosaic-compile.

### Added — Grid row stripes via `data-row:even` / `:odd` (WA4, UI27 §5)

- Authors can now declare alternating row colours by attaching `state
  even { ... }` and / or `state odd { ... }` blocks to the
  `sheet/data-row` sub-part. The emitter resolves them via the existing
  composite-key map (`sheet/data-row:even`, `sheet/data-row:odd`) and
  emits a conditional spread per `<tr>`:
  `<tr key={r} style={{ ...base, ...(r % 2 === 0 ? { evenProps } : {}), ...(r % 2 === 1 ? { oddProps } : {}) }}>`.
- Either state can be declared independently — only the declared spread
  appears in the output. When neither is declared, the static
  `style={{ <data-row defaults> }}` path is preserved exactly
  (backwards-compatible).
- `lookup_state` (formerly hardcoded to look up `cell:{state}`) was
  generalised to `lookup_state_on(subpart, state)` so other sub-parts
  can grow state blocks in the same way (e.g. `header-cell:hover` for
  future column-header hover styling).
- 5 new tests covering: even-only emits one conditional spread,
  even+odd emits two in source order, base data-row props precede
  conditional stripes (UI27 §2 cascade), no states keeps the static
  pre-WA4 output, stripes without base props emit solo conditionals.
- End-to-end smoke: a 3-file grid with `state even / odd` blocks
  compiles to TSX with the expected conditional `<tr>` style. Verified.


### Added — Input `placeholder` from string-literal prop values

- The Input emitter consumes the new `LayoutPropValue::String` variant
  (moslayout STRING token) and writes it through as a JSX
  `placeholder="..."` attribute. The placeholder text is escaped for
  JSX double-quoted attribute syntax (`\` and `"` get backslash-escaped).
- The previous known-limitation note in the emitter doc-comment ("string
  literals as prop values aren't yet supported by the grammar") is
  removed; the limitation is resolved upstream in `moslayout-compiler`.
- Two new tests cover the literal binding and the escape rules.

### Changed — Grid selection / editing colours come from `.msl`

- The Grid primitive's per-cell selection (`r === selRow && c === selCol`)
  and editing (`r === edRow && c === edCol`) highlight spreads now read
  their colours from the author's mosstyle source: `state selected
  { ... }` and `state editing { ... }` under `part sheet/cell` are
  surfaced into the part-style map under composite keys
  (`sheet/cell:selected`, `sheet/cell:editing`) and inlined into the
  spread. When either state block is omitted the emitter falls back to
  the same hardcoded defaults it shipped before (`#264f78` selected,
  `#1f4f3f` editing), so existing demos render unchanged.
- The part-style map now generally exposes one entry per declared state
  block via the composite key `{part}:{state}`. Other emitters that
  want per-state lookups (Input hover, Button pressed, etc.) can read
  the same shape without re-walking the `StyleDef`.
- Three new tests: author selected overrides default, author editing
  overrides default, mixed case where only one state is declared still
  uses the default for the other.
- The VisiCalc demo's `Grid.dark.msl` now declares `state selected` and
  `state editing` blocks under `part sheet/cell`, keeping the rendered
  output identical while moving the colours to the theme.

### Added — Grid `column-widths` slot ref

- The Grid primitive now reads an optional `column-widths` slot-ref prop
  (UI26 §2.1). When bound, the emitter writes a `<colgroup>` between the
  opening `<table>` and the `<thead>`, mapping each width to a
  `<col style={{ width: "${w}px" }} />`. Both `<th>` and `<td>` in the
  same column inherit the width through the standard HTML table model.
- When the prop is absent (existing demos), no `<colgroup>` is emitted,
  preserving the previous flex-default behaviour.
- Two new tests cover both branches.
- The VisiCalc demo (`demo/visicalc/mosaic/Grid.mil` and
  `Grid.desktop.mll`) was updated to declare and bind the slot, and
  `App.tsx` now passes `state.columnWidths` through. Resolves known
  limitation #5 in `demo/visicalc/README.md`.

### Changed — event-union types are now exported

- `emit_event_union` now writes `export type {Component}Event = ...`
  (and `export type {Component}Event = never` in the empty-emit case)
  so host applications can `import type { GridEvent } from "./Grid"`
  directly instead of redeclaring the event-union shape inline. The
  VisiCalc demo (`demo/visicalc/src/app/state.ts`) previously carried a
  hand-maintained copy of `GridEvent` and `FormulaBarEvent` for exactly
  this reason; it now imports them from the generated component files.
- Two new tests assert the `export` keyword is emitted in both the
  empty-emit and one-or-more-emit cases.

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

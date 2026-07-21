# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added - UI35 drag-and-drop lowering (`HostDraggable` / `HostDropTarget`)

The React backend now lowers the kernel's two new drag primitives (see
`code/specs/UI35-host-drag-drop.md`). Before this, a kanban board — "drag a card
to another column" — was simply not expressible in Mosaic; every app had to drop
to hand-written host code.

The lowering deliberately does **not** use the HTML5 drag-and-drop API. Three
contracts from the spec drive the design:

- **Touch works.** HTML5 `dragstart`/`dragover` never fire on touch devices, so
  the emitted code uses **pointer events** (`onPointerDown` / `onPointerEnter` /
  `onPointerUp`), which are unified across mouse, pen, and touch.
- **Keyboard is equivalent, not an afterthought.** Every draggable is
  `tabIndex={0}` with `role="button"` and `aria-roledescription="draggable"`.
  Space/Enter grabs, arrows move the cursor across `[data-mosaic-drop-key]`
  targets, Space/Enter drops, Escape cancels — and a keyboard drop dispatches
  the *same* event as a pointer drop, so the app cannot accidentally support one
  and not the other.
- **Screen readers are told what happened.** A visually-hidden
  `aria-live="polite"` region is emitted alongside the tree and updated on grab,
  move, drop, and cancel.

Implementation notes, following the `HostDialog` precedent:

- `layout_contains_drag` is a presence check, not a node table — the drag
  controller is *shared* by every draggable in a component, so one flag is all
  the emitter needs. It gates the `useState`/`useRef` imports and the controller
  block, so a layout with no drag emits byte-identical output to before (pinned
  by a test).
- `emit_drag_controller_hooks` emits the one controller: the grabbed payload
  (`useRef`), the hovered target (`useState`), the live region (`useRef`), and
  the `mosaicPosition` helper that turns a pointer's Y within the target's bounds
  into `before` / `after` / `into` by thirds.
- **Drops are proposals, not mutations.** The emitted handler dispatches
  `{ key, kind, targetKey, position }` and does nothing else; the engine decides
  whether the move is legal. The emitter never reorders anything itself.
- **Keyboard equivalence is structural, not duplicated.** A keyboard drop does
  not re-implement the pointer drop — it dispatches a real `pointerup` at the
  hovered target so the target's own handler runs. There is exactly one drop
  path, so the two can never drift into dispatching different payloads. A
  release with no target under the cursor degrades to a cancel rather than
  leaving the drag stuck in flight.
- **Target lookup is scoped to the component instance** with `useId`: drop targets
  are stamped with the instance id and lookup filters on it. A document-wide query
  would let arrow keys in one mounted board walk onto — and announce — a drop
  target belonging to a different one. Scoping deliberately does *not* wrap the
  tree in an element: a wrapper is invalid inside `<tbody>`/`<ul>`/`<select>`
  (the parser hoists non-table content out of a table), and this emitter lowers
  components into exactly those contexts. It also would not have worked — a
  *child* component'''s targets sit inside the wrapper'''s subtree too.
- **`onDragEnd { dropped }` reports the real outcome.** A keyboard release returns
  whether the target actually accepted, not merely that an event was dispatched:
  a disabled target bails out of its own handler, and reporting success there
  would leave the drag stuck in flight (so the next Space releases instead of
  grabbing) while claiming the card had landed. Disabled targets also drop their
  key attribute entirely, so the keyboard cursor cannot land on a target the
  pointer could never hover.
- **The hovered target lives in a ref**, mirrored into state only to drive
  re-render. Keyboard auto-repeat fires far faster than React commits, so reading
  the state value would recompute the same next position repeatedly and the
  cursor would stall while an arrow key is held.
- **The keyboard drop survives jsdom.** `PointerEvent` has no constructor there,
  so the emitted code feature-tests and falls back to `MouseEvent` — otherwise the
  standard React test environment would throw on the very path that proves the
  keyboard contract.
- An unhandled shape on `drag-key`/`drop-key` is now an **error**, not a silent
  degradation to an empty key (which would still register and match other keyless
  targets — cards landing in the wrong column rather than a build failure).
- **Generated names live in a `mosaic$…` namespace.** Slot identifiers are
  camel-cased from kebab-case and validated against `^[_A-Za-z][_A-Za-z0-9]*$`,
  so they can never contain `$`. A slot innocently named `mosaic-drag` therefore
  cannot collide with the controller — without the namespace it would emit a
  parameter and a `const` of the same name and fail to compile.
- Controller helpers are emitted **per half used**: a drop-target-only component
  does not get the grab/cancel/step helpers, and vice versa, because an unused
  local trips `noUnusedLocals` (TS6133) under a strict host tsconfig.

12 new tests cover each contract, the scoping, the namespace collision, and the
absence case (a layout with no drag emits byte-identical output to before).

### Fixed - destructure only the props the component body references

The generated component previously destructured *every* slot in its parameter
list, even slots the layout never reads (e.g. Grid's forward-compat
`totalHeight`). Under a strict host `tsconfig` (`noUnusedLocals` /
`noUnusedParameters`) that tripped `TS6133: 'x' is declared but its value is
never read`, breaking `tsc -b` — as it did for the visicalc React demo's
`npm run build`. `emit_function` now builds the body first and destructures only
the slots whose camelCased identifier actually appears in it (via
`body_references_identifier`, a whole-identifier match, not a substring). Unread
slots stay in the `{Component}Props` interface — so callers may still pass them —
they're just no longer bound as unused locals. `dispatch` remains destructured
last unconditionally (UI24 §3.3, the required event-sink contract). Verified: the
visicalc React demo's `npm run build` (tsc + vite) now passes.

### Changed - `--emit-project` Vite shell uses a host adapter

`src/main.tsx` now mounts the generated component through
`window.mosaicHost.getProps` and `window.mosaicHost.handleEvent`, with
deterministic sample values as fallback props. Previously the Vite shell only
passed sample props and logged events locally, which made generated app shells
hard to wire to shared business logic.

The generated shell also listens for a `mosaic-host-ready` browser event and
refreshes props when it fires, allowing async WASM or Electron host installers
to attach `window.mosaicHost` after the React bundle has loaded.

The React project shell now also emits `tsconfig.json`, matching its
`npm run build` script (`tsc && vite build`) so generated shells are directly
type-checkable.

### Added — UI28-1 §6.3 — Automatic React keys for `For` iterations

Every `For` body is now wrapped in a `<React.Fragment key={...}>` so
React's reconciler always has a stable per-iteration identity. This
is the UI28-1 §5 performance property the spec promises — eliminates
React's "Each child in a list should have a unique 'key' prop"
runtime warning by default.

Two emission shapes:

- **Author bound `index: <name>`** — that name doubles as the
  React.Fragment key source. Callback signature is `(<as>, <index>)`,
  wrapper is `<React.Fragment key={<index>}>`.
- **Author omitted `index:`** — emitter injects an implicit `_idx`
  parameter into the .map callback and uses it as the key. Callback
  signature is `(<as>, _idx)`, wrapper is `<React.Fragment key={_idx}>`.
  The underscored name signals "framework-internal"; author body code
  is unaffected.

The wrapper is always the long-form `<React.Fragment>`, never the
shorthand `<>`, because JSX shorthand fragments cannot carry
attributes. Multi-child bodies that previously wrapped in `<>...</>`
now wrap in `<React.Fragment key={...}>...</React.Fragment>` —
single-child bodies, which previously emitted bare, also gain the
keyed wrapper for uniformity.

A new layout-scan helper `layout_contains_for` extends the file-
header import detection: any component whose layout has a For now
triggers `import React from "react";` (otherwise the file would
reference the `React` namespace without importing it). Components
without For or React.*-typed slots still skip the import — the
existing `noUnusedLocals` discipline holds.

5 new tests + 4 snapshot tests updated to assert the new shapes.
Total tests: 198 (was 193, +5). Notable:
`ui28_1_react_for_with_explicit_index_uses_that_name_as_key`,
`ui28_1_react_for_without_index_injects_implicit_idx_for_key`,
`ui28_1_react_for_kebab_case_index_camel_cases_in_both_callback_and_key`,
`ui28_1_react_for_triggers_react_namespace_import_even_for_primitive_only_interface`,
`ui28_1_react_for_multi_child_body_still_uses_react_fragment_wrapper`.

### Added — UI32-K-react — `--emit-project` Vite shell

Mirrors the XAML pattern (UI32 spec §2.1, PR #3917, spec PR #4286)
for the React backend: when `EmitOptions::emit_project` is on, the
emitter returns a `ProjectFiles` value alongside the component TSX
so `mosaic-compile --backend react --emit-project` produces a
runnable Vite project. Author types `npm install && npm run dev`
and sees the component at `http://localhost:5173` — no host code
required.

New public API:

- `pub struct EmitOptions` — `emit_project: bool`, pinned versions
  (`pinned_react`, `pinned_vite`, `pinned_vite_react_plugin`,
  `pinned_typescript`, `pinned_types_react[_dom]`,
  `pinned_node_engines`), optional `package_name` override.
- `pub struct ProjectFiles` — `package_json`, `vite_config`,
  `index_html`, `main_tsx`, `readme` (UI32 spec §2.2 React row).
- `pub enum ProjectShellError` — `InvalidNpmPackageName(String)`
  surfaced through `PipelineEmitError::UnsafeSlotName`.
- `pub struct PipelineEmitResultWithProject` — `output`,
  `component_name`, `project: Option<ProjectFiles>`.
- `pub fn from_pipeline_with_options(...)` — new entry point.
  Existing `from_pipeline(...)` is unchanged (3-arg signature).

Emitted shell:

- `package.json` — pinned react@18.3.1, react-dom@18.3.1,
  vite@5.4.10, @vitejs/plugin-react-swc@3.7.1, typescript@5.7.2,
  @types/react@18.3.18, @types/react-dom@18.3.5. `engines.node`
  floors at `>=18.0.0`. All deps exact-pinned (no `^`/`~`/`*`/
  `latest`/`>=`) per UI32 spec §3.6.3.
- `vite.config.ts` — Vite 5 + React-SWC plugin.
- `index.html` — Vite root with `<div id="root">` +
  `<script type="module" src="/src/main.tsx">`.
- `src/main.tsx` — `createRoot(...).render(<StrictMode>
  <Component dispatch={(ev)=>console.log("event:",ev)} />
  </StrictMode>)`. Imports sibling-relative (`../{Component}`)
  so the `.tsx` stays at project root.
- `README.md` — prereqs (Node ≥18), run commands, file map.

Banner on every file (spec §3.5): `// AUTO-GENERATED by
mosaic-compile --emit-project. Edits will be overwritten on next
emit. // Fork the file (remove this banner) to customise.`
package.json uses a `"//"` key for the banner (JSON has no
comments).

Validation per spec §3.6.2: derived npm name flows through
`is_valid_npm_name` which enforces lowercase + ≤214 chars +
no leading dot/underscore + URL-safe chars. Auto-derivation
(`mosaic-{kebab(Component)}`) produces a valid name for any
PascalCase component. An explicit `package_name` override that
fails the check returns `ProjectShellError::InvalidNpmPackageName`
fail-loud — no silent substitution.

10 new tests pin: back-compat (default = no shell, identical
TSX), emit-true returns shell, banner on every file, byte-
determinism, invalid name rejected, name derivation, exact pinning
+ no forbidden version forms (engines.node correctly exempted),
file enumeration tripwire, no env reads, is_valid_npm_name truth
table. Total React emitter tests: 193 (was 183, +10).

**Scope note: lockfile vendoring deferred to L2.1 follow-up.**
UI32 spec §3.6.3 mandates a vendored lockfile (`package-lock.json`)
alongside the pinned `package.json`. Generating one without
shelling to `npm install` at emit time (which would violate spec
§3.8) requires a separate offline-lockfile script that's out of
scope for the L2 cycle. Workaround for now: pin exact versions
in `package.json` (no `^`/`~`); `npm install` will fetch the
exact top-level versions but transitive deps remain unpinned.
Tracked as L2.1.

### Added — UI31-L10 — For-in-HostTable section + Keyword content seam

Three coordinated extensions that let `HostTable` compositions drive
dynamic row data through For loops while keeping semantic
`<table>`/`<thead>`/`<tbody>`/`<tr>`/`<th>`/`<td>` markup:

- **For-of-Row in a section** — `HostTableBody { For (each:…, as: row)
  { Row { … } } }` now lowers to
  `<tbody>{rows.map((row) => <tr>…</tr>)}</tbody>`. Without this
  seam the generic walker would have produced
  `<tbody>{rows.map((row) => <div>…</div>)}</tbody>` — `<div>`
  inside `<tbody>` is HTML-parser-invalid and breaks every
  backend's table semantics.

- **For-of-cell in a Row** — `Row { For (each:…, as: header) { Text
  (content: header) } }` now lowers to
  `<tr>{cols.map((header) => <th><span>{header}</span></th>)}</tr>`.
  The seam produces one `<th>`/`<td>` per item rather than wrapping
  the entire `.map(...)` in a single cell.

- **Keyword content in Text** — `Text (content: <For-binding>)` now
  interpolates as `<span>{binding}</span>`. Previously the Text
  emitter only handled SlotRef content; Keyword content fell
  through to the generic emit (empty `<span></span>`), so cells
  iterated by a For rendered blank. Slot names pass through
  `is_safe_js_identifier` so an unsafe keyword like `"x; alert(1)"`
  drops silently rather than landing in the JSX interpolation.

Together these enable the L10 VisiCalc Grid migration: the demo's
`Grid.desktop.mll` + `Grid.touch.mll` now compose from HostTable*
kernel primitives instead of the legacy built-in `Grid` primitive,
producing semantic table markup on every backend that has the UI31
HostTable lowering (React + HTML get this PR's For seams; the
WebComponent + Flutter + Qt + SwiftUI + XAML backends still produce
broken For-in-section output and are tracked as follow-ups).

4 new tests, total 183 (was 179):
- `host_table_for_of_row_in_body_emits_map_of_tr`
- `host_table_for_of_cell_in_head_row_emits_map_of_th`
- `text_with_keyword_content_lowers_to_span_with_binding_expr`
- `text_with_unsafe_keyword_content_drops_silently`

### Added — UI29-4 `HostLink` + `HostTooltip` + `HostNumberInput` (U29-4-K-react)

Three new kernel primitives lower to React widgets:

- **`HostLink` → `<a href onClick>`**
  - `href: str|slot` → `href="..."` / `href={slot}`
  - `label: str|slot` → JSX text body
  - `target: new-tab` → `target="_blank" rel="noopener noreferrer"`
    (security default — prevents reverse-tabnabbing per the
    HTML5 living-standard recommendation and the eslint-plugin-
    react `react/jsx-no-target-blank` rule)
  - `target: parent | top | same` → standard HTML `target=` values
  - `external: false` keyword → onClick handler with
    `e.preventDefault()` so the host's router takes over
  - `onActivate: emit: onX` → dispatched as `{type:"x", href: ...}`
    (combined with preventDefault when both are present)
- **`HostTooltip` → `<span title={text}>{children}</span>`**
  - `text: str|slot` → `title="..."` / `title={slot}`
  - Single child (or all children) wraps inside the span
  - Plain-text only in v1 per UI29-4 §3.2; rich-content tooltips
    are reserved for UI29-5
- **`HostNumberInput` → `<input type="number" inputMode="numeric">`**
  - `value: slot` → controlled-input `value={slot}`
  - `min` / `max` / `step` numeric literals → matching JSX
    expression attributes
  - `placeholder: str` → `placeholder="..."`
  - `disabled: slot|bool`
  - `onChange: emit: onX` → `dispatch({type:"x", value:
    e.target.valueAsNumber})` — the DOM's standard numeric parser,
    matching the kernel-canonical `value: number` payload
  - `inputMode="numeric"` is always set — triggers the mobile
    numeric keyboard, one of the "what composition loses" items
    flagged in the UI29-4 survey

8 new tests pin: HostLink href+label rendering, the
target="_blank" security pin (rel="noopener noreferrer" pair),
external: false + onActivate combined preventDefault+dispatch,
HostTooltip string + slot text variants wrapping children,
HostNumberInput bare shape (with inputMode), min/max/step
literals, and the onChange valueAsNumber dispatch.

Security review caught no findings. The `escape_for_jsx_double_
quoted` + `validate_slot_or_field_name` / `validate_emit_name`
helpers (added in earlier PRs) already cover the new
interpolation sites; the `target="_blank"` + `rel="noopener
noreferrer"` pairing is emitted as a single literal block so the
two attrs can't be decoupled by future refactors.

### Added — `HostCheckbox.indeterminate` slot (UI29-2 follow-up)

Closes the deferred work flagged in the previous `Added` block.
React's `<input type="checkbox">` cannot reach the tri-state visual
through HTML attributes — `indeterminate` is a JS DOM property on
`HTMLInputElement`, not an HTML attribute. The fix mirrors
`HostDialog`'s `dialog_nodes` infrastructure: a separate
`indeterminate_checkbox_nodes` collection threads through the JSX
walkers, and `emit_function` now emits one `useRef
<HTMLInputElement>` + `useEffect` pair per indeterminate-tracking
checkbox at the top of the function body. The emitted effect:

```tsx
const checkboxRef_0 = useRef<HTMLInputElement>(null);
useEffect(() => {
  if (checkboxRef_0.current) {
    checkboxRef_0.current.indeterminate = !!isMixed;
  }
}, [isMixed]);
```

and the `<input>` lowering gains `ref={checkboxRef_0}` so React
hands the DOM node to the effect. Multiple indeterminate
checkboxes in the same component get distinct ref names
(`checkboxRef_0`, `checkboxRef_1`, …) assigned in DFS source
order — same pattern as `dialogRef_<n>`.

`HostCheckbox` instances WITHOUT an `indeterminate:` slot stay at
the pre-FU minimal shape (no ref, no effect, no hook import).
Keyword-form `indeterminate: true/false` literals don't emit hooks
either — only the runtime-driven SlotRef case does.

3 new tests pin: the useRef/useEffect emission with correct slot
binding, the negative-case "no hooks when slot absent" guard, and
distinct-ref-names-when-multiple regression.

The React backend now matches the indeterminate coverage already
provided by the Qt / XAML / HTML / WebComponent backends. The
spec-promised follow-up is closed.

### Added — `HostCheckbox` + `HostRadio` kernel primitives (UI29-2, U29-2-K-react)

- New `HostCheckbox` lowering emits a native `<input type="checkbox" />`.
  - `checked: slot: c` → `checked={c}` (controlled-input pattern).
  - `disabled: slot: d` / `disabled: true|false` → `disabled={d|true|false}`.
  - `label: "..."` / `label: slot: l` → wraps the input in a
    `<label><input … /> {label}</label>` element (idiomatic React
    single-row pattern, no id-juggling needed).
  - `onToggle: emit: onX` → `onChange={e => dispatch({ type: "x",
    checked: e.target.checked })}`, matching the kernel-canonical
    `checked: bool` payload (UI29-2 §2.2).
  - The `indeterminate` slot is **deferred to a follow-up PR**: it
    requires a `useRef` + `useEffect` pair (DOM's `indeterminate` is
    JS-API-only, not an HTML attribute), and that plumbing is left out
    of this first cut to keep the diff reviewable. Authors who declare
    `indeterminate:` today get a working two-state checkbox; the third
    state is silently dropped until the follow-up.
- New `HostRadio` lowering emits a native `<input type="radio" />`.
  - `checked: slot: c` → `checked={c}` (controlled-input).
  - `group: "name"` / `group: slot: g` → `name="name"` / `name={g}`,
    which couples radios into a browser-enforced mutex set (DOM radios
    with the same `name` deselect each other automatically).
  - `value: "v"` / `value: slot: v` → `value="v"` / `value={v}`, the
    form-submit value the host receives in `onSelect`'s payload.
  - `disabled` and `label` mirror HostCheckbox exactly.
  - `onSelect: emit: onX` → `onChange={e => dispatch({ type: "x",
    value: e.target.value })}` per UI29-2's `value: text` payload.
- v1 keeps each `HostRadio` standalone with its own `checked` slot —
  the host is responsible for the React-state mutex. The proper
  `RadioGroup` userland component is reserved for UI29-2.1.
- 12 new unit tests cover the bare-input shape, controlled-input
  wiring, `disabled`, `label` wrapping, group/value/checked, and the
  onToggle/onSelect dispatch payloads.

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
- The VisiCalc demo (`code/programs/mosaic/visicalc/Grid.mil` and
  `Grid.desktop.mll`) was updated to declare and bind the slot, and
  `App.tsx` now passes `state.columnWidths` through. Resolves known
  limitation #5 in `code/programs/typescript/visicalc/README.md`.

### Changed — event-union types are now exported

- `emit_event_union` now writes `export type {Component}Event = ...`
  (and `export type {Component}Event = never` in the empty-emit case)
  so host applications can `import type { GridEvent } from "./Grid"`
  directly instead of redeclaring the event-union shape inline. The
  VisiCalc demo (`code/programs/typescript/visicalc/src/app/state.ts`) previously carried a
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

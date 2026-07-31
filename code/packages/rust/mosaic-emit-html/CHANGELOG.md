# Changelog — mosaic-emit-html

## [Unreleased]

### Added - host-owned surface composition

`HostSurface ( content: slot: ... )` now emits a stable DOM mount point with a
trusted node-slot template marker for host-provided browser content.

### Added - UI35 drag-and-drop (`HostDraggable` / `HostDropTarget`)

The HTML backend now lowers the kernel's two drag primitives (see
`code/specs/UI35-host-drag-drop.md`), following React as the reference
implementation but keeping this backend's own discipline.

**The markup half is pure markup.** Author-supplied values never reach
JavaScript source — they go into `data-*` attributes through
`escape_html_attr`, and the runtime reads them back via `element.dataset`.
So unlike `HostDialog`, the drag primitives emit **no inline script at all**,
and a drag card costs zero bytes of script per node. `tabindex`, `role`, and
`aria-roledescription` are emitted as markup so the card is announced correctly
even before the runtime loads.

**The behaviour half lives in the emitted `main.js` runtime**, as delegated
listeners on the component root — matching how every other event in this backend
is wired (`data-on-*` marker + delegation), rather than inventing a second
mechanism.

Contracts, all three proven by tests:

- **Touch works.** Pointer events throughout. HTML5 `dragstart`/`dragover`
  never fire from a touch, so an HTML5 lowering would be silently desktop-only.
- **Keyboard equivalence through one drop path.** Space/Enter grabs, arrows move,
  Space/Enter drops, Escape cancels. Both input methods call the same
  `mosaicCommitDrop`, so the proposal payload is constructed in exactly one
  place and the two cannot drift apart (a test pins the payload to a single
  construction site).
- **Announcements.** A visually-hidden `aria-live` region, written with
  `textContent` — never `innerHTML`, since the label is application data.

Two failure modes specific to this backend, both caught by tests:

- **`render()` replaces `root.innerHTML` wholesale** on every host response, so
  drag state is module-scoped rather than stashed on elements, and the hovered
  target is tracked by *key* rather than by element reference — the element that
  key names may be a different object after a re-render.
- **The live region is parented to `<body>`, not to `root`**, for the same
  reason: inside `root` the next render would destroy it and turn every later
  announcement into a silent no-op.

**Choosing pointer events is not by itself enough to make touch work.** A
direct-manipulation pointer receives *implicit pointer capture* on `pointerdown`,
so every later event in the gesture retargets to the element it began on:
`pointerenter` never fires for any other drop target, and `pointerup`'s target is
still the source card. The obvious implementation therefore resolves every touch
drop back to its source column and the card snaps back — the precise failure
pointer events were chosen to avoid. Hit testing goes through `elementFromPoint`
instead, which reports what is actually under the finger, and the draggable
carries `touch-action: none` so the browser does not claim the gesture as a
scroll. Capture is embraced rather than fought: it is what makes a release
*outside* the component or the window still deliver `pointerup` to us.

Other behaviours worth naming, each with a test:

- **A press is not a drag.** A 5px movement threshold and a primary-button check
  gate the grab. Without them, clicking a card grabbed it and immediately dropped
  it on its own enclosing container — a spurious reorder whose position depended
  on where in the card you clicked, and any button nested in a card was unusable.
- **A drag cannot be stranded.** `pointercancel` is handled (the browser reclaims
  touch gestures it decides are scrolls), and capture covers release outside the
  root. A stuck drag is not benign: the next Space would release instead of
  grabbing, and the next press would emit a second `onDragStart` with no
  intervening `onDragEnd`.
- **Escape survives a re-render.** Once a drag is in flight the keyboard handler
  no longer requires the event to come from the grabbed card. The first host
  response replaces the subtree, detaching the focused element and dropping focus
  to `<body>` — a focus-gated Escape would become unreachable exactly when it is
  needed, leaving a keyboard-only user with an unresolvable drag.
- **`onDrop` settles before `onDragEnd`.** Firing both unawaited put two host
  round-trips in flight, each merging props and re-rendering, so an `onDragEnd`
  response computed from pre-drop state could land last and visually revert the
  move.
- **`dropped` reports acceptance, not attempt** — a refused drop cancels, on the
  pointer path as well as the keyboard one.
- **Disabled and keyless targets are excluded from the keyboard walk**, not
  merely refused: the cursor cannot land on and announce a target the pointer
  could never hover, and several keyless targets would all match the same probe
  so the walk could never reach past the first.
- **A disabled card announces as disabled** (`aria-disabled`), rather than
  presenting as an actionable button that does nothing when activated.

`buildEvent` gained an `overrides` channel for payload values the DOM cannot
supply (which card was grabbed, where in the target it landed). It matches by
*own* property: `in` walks the prototype chain, so an emit param named
`constructor` would assign a function off `Object.prototype` — which
`JSON.stringify` then drops, silently delivering an event missing a declared
param — and one named `__proto__` would invoke the setter and add no key at all.

19 new tests.

### Added - HTML event hydration markers

`HostButton` now preserves `onClick`/`onTap` emits as `data-on-click`, and
`HostInput` preserves `onChange`/`onCommit`/`onCancel` emits as
`data-on-change`, `data-on-commit`, and `data-on-cancel`. The HTML backend
stays static and script-free while giving a downstream hydrator the same Mosaic
event names used by the interactive shells.

### Added — UI32-K-html — `--emit-project` standalone-HTML shell

L3 of UI32 (spec PR #4286). Lifts the `index-shell.html` pattern
from UI31-M (PR #4219) into mosaic-compile's single-component
path: `mosaic-compile --backend html --emit-project` now produces
a complete `<!DOCTYPE html>` document alongside the component
fragment, viewable in any browser with zero install.

New public API (mirrors L2 React, PR #4297):

- `pub struct EmitOptions { emit_project: bool }` — minimal,
  no pinned-versions / tooling fields (HTML has no build step).
- `pub struct ProjectFiles { index_html, readme }` — UI32 §2.2
  HTML row enumeration.
- `pub struct PipelineEmitResultWithProject { output,
  component_name, project: Option<ProjectFiles> }`.
- `pub fn from_pipeline_with_options(...)` — new entry point.
  Existing `from_pipeline(...)` unchanged.

Emitted shell:

- `index.html` — complete `<!DOCTYPE html>` document with
  `<head>` (charset, viewport, `<title>`) + `<body>` containing
  `<section data-component="X">` that **inlines the component
  fragment** (4-space indent for readability). Open in any
  browser; no install step, no `<script>` tags. The inlined
  fragment is the emitter's own previous output, so no new
  escape surface.
- `README.md` — open-in-browser prose (macOS / Linux / Windows
  commands) + file map.

Banner on every file: `<!-- AUTO-GENERATED by mosaic-compile
--emit-project. Edits will be overwritten on next emit. -->`.

UI32 §3.6.2 HTML row: no validation surface beyond ASCII —
component name lands only in HTML comments, `<title>` content,
and `data-component="…"` attribute, all safe for the upstream-
validated ASCII identifier shape.

8 new tests cover: back-compat (default = no shell, identical
fragment), emit-true returns shell, banner on every file, byte-
determinism, fragment-actually-inlined (not a placeholder
reference), output-path enumeration tripwire, no env reads,
README mentions component name. Total HTML emitter tests: 90
(was 82, +8).

### Added — UI31-L10 — For-in-HostTable section + Keyword content seam

Mirrors the React backend's L10 wiring with HTML's Mustache-bracket
idiom (`<!-- mosaic-for each="…" as="…" -->` … `<!-- /mosaic-for -->`):

- **For-of-Row in a section** — `HostTableBody { For (each:…, as: row)
  { Row { … } } }` now lowers to
  `<tbody><!-- mosaic-for each="rows" as="row" --><tr>…</tr><!-- /mosaic-for --></tbody>`,
  where the inner `<tr>` flows through `emit_table_row` so cells
  emit as native `<th>`/`<td>` (not the flex-`<div>` the generic
  walker would have produced).

- **For-of-cell in a Row** — `Row { For (each:…, as: header) { Text
  (content: header) } }` now lowers to
  `<tr><!-- mosaic-for each="cols" as="header" --><th>{{header}}</th><!-- /mosaic-for --></tr>`.
  The downstream template engine expands the loop to one cell per
  item rather than wrapping the entire iteration in a single cell.

- **Keyword content in Text** — `Text (content: <For-binding>)` now
  lowers to the same `{{binding}}` Mustache form that SlotRef
  content uses, so cells iterated by a For actually render the
  bound value rather than blank.

Together these unblock the L10 VisiCalc Grid migration on the HTML
backend.

3 new tests, total 82 (was 79):
- `host_table_for_of_row_in_body_emits_for_bracket_around_tr`
- `host_table_for_of_cell_in_head_row_emits_for_bracket_around_th`
- `text_with_keyword_content_in_cell_lowers_to_mustache_placeholder`

### Added — U29-4-K-html — `HostLink` + `HostTooltip` + `HostNumberInput` kernel primitive lowerings

Three new kernel primitives lower to native HTML elements:

- **`HostLink` → `<a href ...>label</a>`**:
  - `href: str|slot` → real `href=` attribute (string literal or `{{slot}}` template marker)
  - `label: str|slot` → text body
  - `target: new-tab` → `target="_blank"` paired with `rel="noopener noreferrer"` (security default — prevents reverse-tabnabbing; eslint react/jsx-no-target-blank parity)
  - `target: parent|top|same` → standard HTML `target=` values
  - `external: false` keyword → `data-external="false"` marker for hydration to intercept
  - `onActivate: emit: onX` → `data-on-activate="onX"` marker

- **`HostTooltip` → `<span title="text">{children}</span>`**:
  - `text: str|slot` → real `title=` attribute (string literal or `{{slot}}` marker)
  - Single child wraps inside; multi-child case walks recursively
  - Plain-text only in v1 per UI29-4 §3.2

- **`HostNumberInput` → `<input type="number" inputmode="numeric" ...>`**:
  - `value: slot|number` → real `value=` attribute (slot template or literal)
  - `min` / `max` / `step` numeric literals → matching HTML attribute values
  - `placeholder: str|slot` → real `placeholder=` attribute
  - `disabled: slot|bool` → bare `disabled` keyword OR `data-disabled` marker
  - `onChange: emit: onX` → `data-on-change="onX"` marker
  - `inputmode="numeric"` always set — triggers mobile numeric keyboard

7 new tests pin: HostLink href+label rendering, the target=_blank security pin (rel="noopener noreferrer" paired emission), the external+onActivate data-* markers, HostTooltip span+title wrapping, HostNumberInput inputmode=numeric default, min/max/step numeric literal pass-through, and the onChange data marker.

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

# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Fixed - project-shell named node-slot mounting

Generated Web Component shells now recognize `node` and component props from
`window.mosaicHost`, attach host-provided `Element` values to the component's
light DOM with the declared `slot` name, and replace only the prior host-owned
element on refresh. `HostSurface` named slots can now receive real content in a
runnable shell instead of a stringified attribute.

### Added - host-owned surface composition

`HostSurface ( content: slot: ... )` now lowers through the platform-native
named-slot mechanism, preserving the Mosaic-styled shadow-DOM container.

### Added — UI32-K-webcomp — `--emit-project` standalone-HTML shell

L4 of UI32 ([spec PR #4286](https://github.com/adhithyan15/coding-adventures/pull/4286); L2 React #4297, L3 HTML #4309). `mosaic-compile --backend webcomponent --emit-project` now produces an `index.html` shell alongside the component `.js`:

- `index.html` — complete `<!DOCTYPE html>` document with
  `<script type="module" src="./{Component}.js"></script>` and
  `<mos-{kebab(Component)}></mos-{kebab(Component)}>` in `<body>`.
  The shell tag matches the `customElements.define` registration
  the emitter produces (same `to_kebab_case` helper + `mos-` prefix).
- `README.md` — open-in-browser prose with the
  `python3 -m http.server 8000` / `npx serve` recipes (ES modules
  must be served over HTTP, not `file://`, due to CORS).

New public API (matches L2 React / L3 HTML pattern):

- `pub struct EmitOptions { emit_project: bool }`
- `pub struct ProjectFiles { index_html, readme }`
- `pub struct PipelineEmitResultWithProject { output,
  component_name, project: Option<ProjectFiles> }`
- `pub fn from_pipeline_with_options(...)` — new entry point.
  Existing `from_pipeline(...)` is unchanged.

UI32 §3.6.2 WebComponent row contract: the Custom Element tag
name MUST contain a hyphen per HTML spec. The `mos-` prefix +
`to_kebab_case` helper guarantee this for any PascalCase or
single-word component (`Hello` → `mos-hello`, `ProfileCard` →
`mos-profile-card`). The shell tag and the
`customElements.define("mos-...", ...)` registration agree by
construction.

9 new tests cover the spec §3 gates plus a Custom Element
hyphen-contract test that exercises single-word + multi-word
component names. Total tests: 92 (was 83, +9).

### Added

- **UI31-K-webcomp** — RTL contract for `HostTable`. The shadow-DOM
  `<table>` now carries a native HTML `dir` attribute when the
  layout author writes `dir: ltr|rtl|auto` or `dir: slot: foo`:
  - Allow-listed keyword form (`dir: rtl` / `dir: ltr` / `dir: auto`)
    emits a literal `<table dir="rtl">`. Unknown keywords drop
    silently — the allow-list is the security gate that prevents an
    attacker-controlled keyword from breaking out of the attribute
    quotes (`rtl" onerror="..."` becomes a no-op rather than an
    injection).
  - Slot-ref form (`dir: slot: layout-direction`) emits
    `<table dir="${layoutDirection}">`, interpolated through the
    shadow-DOM render template literal. Slot names round-trip
    through `to_camel_case_first_lower` so `layout-direction`
    lands as `layoutDirection`.
  - The `dir` attribute is scoped to the root `<table>` only.
    Sub-tags (`<thead>` / `<tbody>` / `<tfoot>` / `<colgroup>`)
    inherit directionality via HTML's normal cascade, so giving
    them their own `dir` attribute would just be redundant churn
    in the markup.
  - 7 new tests cover the a11y gate (the lowering must be a real
    `<table>`, never `<div role="grid">` div-soup), the three
    allow-listed keywords, the slot-ref interpolation, the
    silent-drop on unknown keywords (with an injection-style payload
    in the bad input to nail down the security claim), the root-
    only scoping, and a bare-table regression guard. Total tests:
    83 (was 76).

- **U29-4-K-webcomp** — `HostLink` + `HostTooltip` + `HostNumberInput`
  kernel primitives (UI29-4) lower to shadow-DOM-rendered HTML
  elements with inline handlers reaching the Custom Element via
  `this.getRootNode().host.dispatch(...)`:
  - **`HostLink` → `<a href ...>label</a>`** with the same
    `target="_blank"` + `rel="noopener noreferrer"` security default
    the React and HTML backends ship (paired as one literal so
    they can't be decoupled). `external: false` + `onActivate`
    produces a combined onclick that calls `event.preventDefault()`
    + shadow-DOM-aware dispatch with the href in the payload.
  - **`HostTooltip` → `<span title="${text}">child(ren)</span>`**
    (plain-text only in v1).
  - **`HostNumberInput` → `<input type="number" inputmode="numeric"
    ...>`** with `value`/`min`/`max`/`step` slot or literal
    pass-through, `placeholder`, `disabled`, and onchange wiring
    that dispatches with `event.target.valueAsNumber` (DOM standard
    numeric parser, matching the kernel-canonical `value: number`
    payload type).
  - New `find_number` helper added alongside the existing
    `find_string` / `find_slot_ref` / `find_keyword` /
    `find_emit_ref` to look up numeric literal props.
  - 7 new tests cover: HostLink href+label rendering, the
    target=_blank security pin, the external+onActivate combined
    preventDefault+dispatch, HostTooltip span+title wrapping,
    HostNumberInput minimum shape, min/max/step numeric pass-
    through, and the onchange valueAsNumber dispatch.

- **U29-2-K-webcomp** — `HostCheckbox` + `HostRadio` kernel primitives
  (UI29-2) lower to native `<input type="checkbox|radio">` elements
  inside the shadow DOM:
  - Inline handlers wire `onchange` to `this.getRootNode().host.dispatch(...)`
    — the shadow-DOM-aware form used by HostInput/HostButton.
  - `checked` / `disabled` slot refs use template-literal conditional
    attributes: `${slot ? " checked" : ""}` / `${slot ? " disabled" : ""}`.
  - `label` (string or slot) wraps the input in a `<label>` element.
  - `HostCheckbox.onToggle: emit: onX` becomes
    `onchange="…dispatch({type:'x',checked:event.target.checked})"`.
  - `HostCheckbox.indeterminate: slot|true` becomes a
    `data-indeterminate="${slot}"` / `data-indeterminate="true"`
    marker; the host's post-render pass sets the DOM property
    imperatively (no HTML attr exists for `indeterminate`).
  - `HostRadio.group` lowers to the real HTML `name=` attribute
    (browser-enforced mutex when multiple radios share `name`).
  - `HostRadio.value` lowers to the real HTML `value=` attribute.
  - `HostRadio.onSelect: emit: onX` becomes a positive-transition-
    gated `onchange="if(event.target.checked)…dispatch({type:'x',
    value:event.target.value})"` per UI29-2 §2.2 ("this radio was
    chosen", not "was deselected by a sibling").
  - 9 new unit tests cover: bare inputs, conditional checked, the
    shadow-DOM-aware onchange dispatch with `checked:` payload,
    label wrapping, indeterminate data marker, bare radio,
    `group:` → `name=`, `value:` → `value=`, and the positive-
    gated radio dispatch.

- **U29-1-K-webcomp** — `HostDialog` kernel primitive (UI29-1) lowers
  to a `<dialog id="mos-dlg-N">…</dialog>` element inside the shadow
  root plus a post-`innerHTML` lifecycle block in `_render()` that
  calls `showModal()` / `show()` / `close()` and wires the dialog's
  `close` event back to `this.dispatch(...)`.
  - `open: slot: x` reads `this.getAttribute("x") === "true"` to
    drive `showModal()` vs `close()`.
  - `modal: false` (compile-time keyword) selects `show()` instead of
    `showModal()`.
  - `title: slot: x` / `title: "literal"` injects an `<h2>` as the
    first child of the `<dialog>` for accessible heading semantics.
  - `dismiss-on-backdrop: false` adds a `cancel`-event interceptor
    that `preventDefault()`s the native Esc/backdrop close.
  - `onClose` / `onOpen` emits wire into `this.dispatch(...)` via the
    CustomEvent path (bubbles + composed, crosses the shadow boundary).
  - Multiple `HostDialog`s in one component are disambiguated with
    monotonic ids (`mos-dlg-0`, `mos-dlg-1`, …).

## [0.2.0] - 2026-05-19

### Added

- New `pipeline` module exposing `from_pipeline(interface, layout, style)`
  that consumes the three-language pipeline triple (mosmodel +
  moslayout + mosstyle) and emits a Custom Element class with shadow
  DOM, observed attributes per slot, and CustomEvent-based dispatch
  following the UI24 Flux pattern.
- `PipelineEmitResult` and `PipelineEmitError` public types matching
  the React / Qt / SwiftUI backends so `mosaic-compile` can treat all
  pipeline backends uniformly.
- Primitive lowering for `Box`, `Row`, `Column`, `Text`, `Image`,
  `Spacer`, `Divider`, `Icon` on the new path. UI29 kernel primitives
  (`Stack`, `HostInput`, `HostButton`, `HostScroll`, `If`, `For`,
  `HostTable`) are tracked as a follow-up PR.

### Dependencies

- Added `mosmodel-compiler`, `moslayout-compiler`, `mosstyle-compiler`
  as path dependencies.

### Unchanged

- The legacy `WebComponentRenderer` driven by `MosaicVM` is left
  intact for backwards compatibility with any consumer still on the
  single-file `.mosaic` path.

## [0.1.0] - 2026-04-04

### Added

- Initial package scaffolding generated by scaffold-generator

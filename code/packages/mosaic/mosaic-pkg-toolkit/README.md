# mosaic-pkg-toolkit

Bootstrap-shaped Mosaic UI component library — ready-made components
composed purely from UI29 kernel primitives, lowering to every
backend (React, SwiftUI, Qt, WebComponent, HTML, XAML) with no
per-backend code.

See [`code/specs/mosaic-pkg-toolkit.md`](../../specs/mosaic-pkg-toolkit.md)
for the architecture, component catalog, and phasing plan.

## v0.1 — exports so far

**11 of 13 Tier-1 components shipped:**

- **`Alert`** — colored info banner with `variant`, optional inline
  dismiss button. Composed from `Box` + `Row` + `Text` + `If` +
  `HostButton`.
- **`Badge`** — small pill label. `Box[badge] { Text }`. Slots:
  `label`, `variant`.
- **`Button`** — styled push button with `variant`, `size`,
  `disabled` slots. Wraps the kernel `HostButton`.
- **`Checkbox`** — labeled checkbox. `Row { If checked HostButton[✓]
  Else HostButton[], Text }`. Slots: `label`, `checked`,
  `disabled`. Emit: `onChange`. Host owns the state.
- **`Field`** — labelled text input + help/error pattern.
  Bootstrap's "form group". Slots: `label`, `value`,
  `placeholder`, `help`, `error`, `disabled`. Emits:
  `onChange(value: text)`, `onCommit`.
- **`Input`** — styled single-line text input. Wraps the kernel
  `HostInput`. Slots: `value`, `placeholder`, `disabled`, `size`.
  Emits: `onChange(value: text)`, `onCommit`.
- **`ListGroup`** — vertical list of selectable text rows.
  Iterates via `For`. Slots: `items` (`list<text>`),
  `selected-index`. Emit: `onSelect(index: number)`.
- **`Modal`** — titled modal dialog wrapping the kernel
  `HostDialog`. Slots: `title`, `message`, `open`, `close-label`.
  Emit: `onClose`. The XAML backend produces a ContentDialog
  root; other backends use their native dialog primitives.
- **`Radio`** — labeled radio (single-select). Same shape as
  Checkbox with circular styling. Slots: `label`, `selected`,
  `disabled`. Emit: `onSelect`.
- **`Spinner`** — indeterminate loading indicator.
  `Stack[spinner] { Icon[spinner-glyph] }`. Slots: `size`,
  `variant`, `aria-label`.
- **`Toast`** — bottom-anchored notification.
  `If open { Box[toast] { Column { Row[toast-header],
  Box[toast-body] } } }`. Slots: `title`, `message`, `variant`,
  `open`. Emit: `onClose`.
- **`Pagination`** — Bootstrap's page-navigation row («prev | 1 2
  3 | next »). Each chip wraps the UI29-4 `HostLink` primitive
  (same convention as Nav/Breadcrumb). Slots: `pages: list<text>`,
  `prev-label`, `next-label`, `active-index`. Emits: `onPrev`,
  `onNext`, `onPageSelect(index: number)`.
- **`InputGroup`** — Bootstrap's input-with-addons pattern. Text
  input flanked by optional prefix and/or suffix text (e.g. `$`,
  `.00`, `@username`). Slots: `prefix`, `suffix`, `value`,
  `placeholder`, `disabled`. Emits: `onChange(value: text)`,
  `onCommit`.
- **`Accordion`** — vertical expand/collapse panel stack. Slots:
  `headers: list<text>`, `bodies: list<text>`, `open-index`. Emit:
  `onToggle(index: number)`. See CHANGELOG for the v0.7 always-
  visible-body limitation (kernel `If` doesn't yet support
  comparison).
- **`Tabs`** — horizontal tab bar + single body panel. Host owns
  the active-index → active-body mapping. Slots: `headers:
  list<text>`, `active-body`, `active-index`. The active header
  renders through a distinct `tabs-tab-active` part. Emit:
  `onSelect(index: number)`.
- **`DropdownMenu`** — toggle button + revealed item list. Host
  owns the open flag. Slots: `label`, `items: list<text>`, `open`.
  Emits: `onToggle`, `onSelect(index: number)`.
- **`Navbar`** — top-of-page brand + HostLink row. Slots: `brand`,
  `items: list<text>`, `active-index`. Emit: `onSelect(index)`.
- **`Select`** — dropdown selector. Same toggle + If(open) pattern
  as DropdownMenu, but onChange carries the selected option *text*.
  Slots: `value`, `options: list<text>`, `placeholder`, `open`,
  `disabled`. Emits: `onToggle`, `onChange(value: text)`.

Every component ships `.light.msl` and `.dark.msl` themes.

## Roadmap

Per spec §7:

| Phase | Components |
|---|---|
| v0.1 PR-1 | Button, Alert |
| v0.1 PR-2 | Badge, Spinner, Toast |
| v0.1 PR-3 | Input, Checkbox, Radio |
| v0.1 PR-4 | ListGroup, Modal |
| v0.1 PR-5 (this) | Field |
| v0.1 PR-6 | Card, Container — depend on UI29-2 children pass-through |
| v0.2 | Tier 2 — Nav, Navbar, Pagination, Breadcrumb, InputGroup, ButtonGroup, Tabs, DropdownMenu, Accordion, Select |
| v0.3 | Bootstrap-aesthetic theme overlay |
| v0.4 | Tier 3 — Tooltip, Popover, Carousel, Offcanvas (depends on kernel follow-ups) |
| v0.5 | Responsive grid (depends on mosstyle breakpoints) |

## Using the toolkit (when fully released)

```toml
# mosaic-package.toml (the host project)
[dependencies]
mosaic-pkg-toolkit = "0.1"
```

```moslayout
// MyComponent.mll
layout MyComponent {
  Column {
    Alert (variant: "info", message: slot: status-message)
    Button (variant: "primary", label: "Submit", onClick: emit: onSubmit)
  }
}
```

## Tests

```bash
cd code/packages/mosaic/mosaic-pkg-toolkit
cargo test
```

Smoke tests assert every exported component's `.mil` / `.mll` /
`.msl` triple round-trips through the three IR compilers, and that
the manifest is internally consistent.

`native_complete_gate.rs` (issue #12024) goes one step further: for
each of the five native backends (SwiftUI, Qt, XAML, Flutter,
Compose) it runs the real `mosaic-package-artifact-builder`
degradation analyzer against the whole package and asserts nothing
*unexpected* was dropped — the atom-level counterpart to the
whole-app TaskApp CI gate, which mixes 21 components' worth of
failure surface into one signal. The toolkit isn't fully
degradation-clean yet: 9 pre-existing native-UI gaps (native
indeterminate checkbox state, native radio-group mutual exclusion,
Modal's dialog lifecycle on XAML and Flutter — see #13006, #13007,
#13008, #13010) are explicitly allowlisted in the test file, each
pointing at its own tracking issue. Anything not on that list — on
any component, existing or new — fails the test immediately.

## Why kernel-only?

The toolkit composes purely from UI29 kernel primitives — no per-
backend code anywhere. Adding a new Mosaic backend automatically
adds the toolkit to it. See spec §1 + §12 for the strategic story.

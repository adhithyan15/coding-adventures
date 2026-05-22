# mosaic-pkg-toolkit

Bootstrap-shaped Mosaic UI component library — ready-made components
composed purely from UI29 kernel primitives, lowering to every
backend (React, SwiftUI, Qt, WebComponent, HTML, XAML) with no
per-backend code.

See [`code/specs/mosaic-pkg-toolkit.md`](../../specs/mosaic-pkg-toolkit.md)
for the architecture, component catalog, and phasing plan.

## v0.1 — PR-1 (scaffold)

**Exports:**

- **`Button`** — styled push button with `variant` (primary /
  secondary / success / danger / warning / info / light / dark),
  `size` (sm / md / lg), and `disabled` slots. Wraps the kernel
  `HostButton`.
- **`Alert`** — colored info banner with `variant`, optional inline
  dismiss button. Composed from `Box` + `Row` + `Text` + `If` +
  `HostButton`.

Both ship a `.light.msl` and `.dark.msl` theme.

## Roadmap

Per spec §7:

| Phase | Components |
|---|---|
| v0.1 PR-1 (this) | Button, Alert |
| v0.1 PR-2..N | Badge, Card, Checkbox, Container, Field, Input, ListGroup, Modal, Radio, Spinner, Toast — 11 more Tier-1 components |
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
cd code/packages/mosaic-pkg-toolkit
cargo test
```

Smoke tests assert every exported component's `.mil` / `.mll` /
`.msl` triple round-trips through the three IR compilers, and that
the manifest is internally consistent.

## Why kernel-only?

The toolkit composes purely from UI29 kernel primitives — no per-
backend code anywhere. Adding a new Mosaic backend automatically
adds the toolkit to it. See spec §1 + §12 for the strategic story.

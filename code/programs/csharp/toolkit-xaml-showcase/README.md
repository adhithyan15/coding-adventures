# toolkit-xaml-showcase — static visual preview of the Bootstrap-shape toolkit

A browser-openable visual tour of every component shipped in
`code/packages/mosaic/mosaic-pkg-toolkit`. The CSS here is a hand-tuned
approximation of each component's `.light.msl` so the static page
is faithful to what the real Mosaic-rendered output looks like.

**For the runnable WinUI 3 demo**, see
[`../toolkit-multi-demo/`](../toolkit-multi-demo/) — it hosts four
of these same components (Button, Alert, Badge, Spinner) in a real
WinUI 3 window built from auto-emitted `mosaic-emit-xaml` XAML.

## What's here

```
mosaic/        — copies of all 20 toolkit components' .mil/.mll/.light.msl
                  triples (from code/packages/mosaic/mosaic-pkg-toolkit/src/)
preview.html   — open in any browser to SEE the full catalog rendered
README.md      — this file
```

## Quick look

```sh
# Any platform, no build step:
open preview.html       # macOS
xdg-open preview.html   # Linux
start preview.html      # Windows
```

The page shows Button (variants), Alert (variants + dismissible),
Badge, Spinner, Toast, Input, InputGroup, Field, Checkbox, Radio,
ListGroup, Pagination, Breadcrumb, Nav, ButtonGroup, Tabs,
Accordion, DropdownMenu, Navbar, and Select — every one of them
styled to match its `.light.msl`.

## Compiling these components yourself

Each component is a `.mil` (interface) + `.mll` (layout) +
`.{light,dark}.msl` (style) triple. Compile any of them with:

```sh
# WinUI 3 / XAML — see ../toolkit-multi-demo/ for end-to-end build
mosaic-compile --backend xaml \
  --interface mosaic/Pagination.mil \
  --layout    mosaic/Pagination.mll \
  --style     mosaic/Pagination.light.msl \
  --output    Pagination.xaml

# Or the whole package at once for React / SwiftUI / Qt
mosaic-compile pkg --backend react --output out/ \
  code/packages/mosaic/mosaic-pkg-toolkit/
```

## Component inventory

| Component   | Pattern                                              |
|---|---|
| Button      | wraps `HostButton`, variant via .msl                 |
| Alert       | colored info banner with optional dismiss            |
| Badge       | pill label, no emits                                 |
| Spinner     | loading glyph (`<ProgressRing>` on XAML backend)     |
| Toast       | bottom-anchored notification (open-flag guarded)     |
| Input       | wraps `HostInput`                                    |
| InputGroup  | text input + optional prefix/suffix addons           |
| Field       | label + HostInput + help/error                       |
| Checkbox    | wraps native `HostCheckbox` (UI29-2)                 |
| Radio       | wraps native `HostRadio` (UI29-2)                    |
| ListGroup   | vertical list of selectable rows                     |
| Modal       | wraps `HostDialog`                                   |
| Tooltip     | wraps native `HostTooltip` (UI29-4)                  |
| NumberInput | wraps native `HostNumberInput` (UI29-4)              |
| Nav         | horizontal row of `HostLink` nav items               |
| ButtonGroup | row of related buttons with shared borders          |
| Breadcrumb  | hierarchical trail of `HostLink` crumbs              |
| Pagination  | « prev | 1 2 3 | next » row of HostLink chips       |
| Tabs        | horizontal tab bar + active body panel               |
| Accordion   | vertical expand/collapse stack                       |
| DropdownMenu| toggle button + revealed item list                   |
| Navbar      | brand + HostLink row                                 |
| Select      | dropdown selector with toggle + options list         |

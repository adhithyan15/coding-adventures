# UI31 — `HostTable` kernel primitive family

> **Status.** Draft, gates the UI31-K-* per-backend implementation
> cycle and the VisiCalc Grid-component migration.
>
> **Parent.** UI29 — Primitive Kernel + Userland Component Packages
> (`code/specs/UI29-primitive-kernel.md`).
>
> **Scope.** Promotes table semantics to the kernel as a family of
> six primitives (`HostTable`, `HostTableHeader`, `HostTableBody`,
> `HostTableRow`, `HostTableHeaderCell`, `HostTableCell`) so every
> Mosaic backend lowers them to its native table widget — with
> accessibility and right-to-left layout as **non-negotiable**
> contracts.

---

## 1. Why this family belongs in the kernel

Tabular data is everywhere — spreadsheets, calendars, file managers,
chat history, transaction lists. Every native UI toolkit ships a
purpose-built widget for it: HTML/DOM has `<table>`, SwiftUI has
`Table`, WinUI has `DataGrid`, Qt has `TableView`, Flutter has
`DataTable`. Each provides:

1. **Built-in accessibility semantics.** Screen readers announce
   "row 3, column 2, cell 'Sales'" without any author code.
   Keyboard navigation (arrow keys move selection, Tab moves focus,
   Enter starts editing, Escape cancels) is wired by the platform.
   ARIA `aria-rowindex` / `aria-colindex` (or the platform
   equivalent) flow automatically from the native element.
2. **Built-in layout direction.** Set `<html dir="rtl">` once and
   the browser flips column order, selection-band positioning,
   scrollbar side, focus-ring side — all of it. SwiftUI's
   `Environment(\.layoutDirection)`, Qt's `LayoutMirroring.enabled`,
   Flutter's `Directionality`, WinUI's `FlowDirection` work the
   same way.
3. **Built-in keyboard interaction model.** Browser tables ship Tab
   between focusable cells. SwiftUI `Table` ships single + multi
   selection. WinUI `DataGrid` ships column reordering + sorting.
   Composed userland tables get none of this for free.
4. **Built-in screen-reader optimizations.** Real `<table>` elements
   are special-cased by AT software (NVDA, JAWS, VoiceOver, TalkBack)
   in ways `<div role="grid">` div-soup never matches — table-
   navigation mode, automatic header-cell association, column-name
   announcements as you cross cells.

A userland-composed table built from `Row` of `Cell` of `Text`
(div-soup with `role="grid"`) cannot match any of these. The current
built-in `Grid` primitive (UI26 §3.1) is hard-coded into the React
emitter and produces a real `<table>`, but only on React — every
other backend's emitter either errors out or emits a placeholder.

UI31 fixes this gap by making table semantics first-class kernel
primitives, so every backend lowers to its native table widget
through the same per-backend emitter wiring pattern that
`HostDialog` / `HostInput` / etc. already use.

Adding `HostTable` + 5 sibling primitives brings the kernel from 21
to 27 primitives. UI29 §2.4 allows the kernel to grow "slowly —
perhaps one or two primitives per year — and never shrink"; this is
a six-primitive jump, but they're all tightly coupled to one
concept (a table). Treating them as a single family keeps the
spirit of the §2.4 constraint.

---

## 2. The primitive family

> **Amendment from L1's first draft (recorded here for the record):**
> the React emitter shipped under UI29 §2.1 with HostTable structural
> sub-tags already in place. The original L1 draft proposed
> `HostTableHeader` / `HostTableHeaderCell` / `HostTableCell` /
> `HostTableRow`; the SHIPPED names are `HostTableHead` /
> `HostTableBody` / `HostTableFoot` / `HostTableColGroup` + `Row`
> with cells as direct `Row` children. The shipped names match HTML
> shorthand more cleanly (`<thead>` not `<table-header>`). This
> section is rewritten to match what's already on disk.

```
HostTable                ← root container; lowers to native table widget
├── HostTableColGroup    ← <colgroup> equivalent; carries width/style per col
│   └── Col              ← one <col /> per column
├── HostTableHead        ← <thead> equivalent
│   └── Row              ← <tr> equivalent; cells get wrapped in <th> here
│       └── (any node)   ← becomes a <th> cell wrapping the node's JSX
├── HostTableBody        ← <tbody> equivalent
│   └── Row              ← <tr>; cells get wrapped in <td> here
│       └── (any node)   ← becomes a <td> cell
└── HostTableFoot        ← <tfoot> equivalent; same Row+cell shape as Body
```

The tree shape mirrors HTML's `<table>` exactly — author-side
mental model maps one-to-one to the most familiar table syntax.
On non-HTML backends the emitter walks the same tree but lowers
each node to the native table-widget API (`SwiftUI.Table`'s
`TableColumn`, WinUI `DataGrid`'s `DataGridColumn`, etc.).

**Cell-tag is implicit, not author-controlled.** Whether a cell
becomes `<th>` or `<td>` is decided by the parent section (`Head`
→ `<th>`, `Body`/`Foot` → `<td>`). This avoids the
`HostTableHeaderCell` vs `HostTableCell` distinction the first
draft proposed — authors don't have to remember which to use,
and the kernel can't accidentally produce a row of `<th>`s inside
a `<tbody>`.

### 2.1 `HostTable`

The root container. Carries slots shared by the whole table. The
existing UI29 §2.1 emitter recognises the part-style slot;
the UI31 amendment adds `dir` (RTL) and reaffirms the
selection/edit/sticky slots from the v1 spec which now lower via
the same code path.

| Prop              | Kind        | Required | Meaning                                                                 |
|---|---|---|---|
| `dir`             | slot/keyword| no       | `ltr` (default) / `rtl` / `auto`. Lowers to the `dir` attribute on the root `<table>` so the browser flips column order. Inherits from `<html dir=…>` when unset. *(UI31-added.)* |
| `selected-row`    | slot/number | no       | Rendered body-row index (or -1). In React, binding both selection slots reveals the cell within the nearest scroll frame after updates, allowing for sticky header height; it does not move keyboard focus. Out-of-range indices do nothing. Native reveal and accessibility cursor semantics require separate backend acceptance. |
| `selected-col`    | slot/number | no       | Rendered column index, paired with selected-row for React reveal.                                                       |
| `edit-row`        | slot/number | no       | Row index being edited (or -1). Same plumbing pattern as `selected-row`. |
| `edit-col`        | slot/number | no       | Column index being edited (or -1).                                                       |
| `sticky-header`   | keyword     | no       | `true` / `false`. When true, the rendered `<thead>` gets `position: sticky; top: 0` so the header pins on scroll. Default `false`. *(Existing built-in `Grid` uses this same prop name; HostTable adopts it.)* |
| `total-height`    | slot/number | no       | Pixel height of the scroll viewport. Used together with `sticky-header: true` to bound the scrolling region. |
| `onNavigate`      | emit        | no       | Fires when selection moves. Payload: `{row: number, col: number}`. Reserved for the [L10] grid-style binding; not yet wired in the v1 HostTable emitter. |
| `onCellEdit`      | emit        | no       | Fires when an edit commits. Same reservation as above.                  |

### 2.2 Structural sub-tags

`HostTableColGroup`, `HostTableHead`, `HostTableBody`,
`HostTableFoot` are pure structural wrappers — they have no own
slots. Each lowers to its matching HTML element (`<colgroup>` /
`<thead>` / `<tbody>` / `<tfoot>`). The emitter rejects them with
a graceful `{/* X is only valid inside HostTable */}` comment if
they appear outside a `HostTable` parent.

### 2.3 `Row` and cell children

`Row` lowers to `<tr>`. Cells are NOT a dedicated primitive — each
immediate child of a `Row` becomes a cell automatically:

- Inside `HostTableHead`, the row's children get wrapped in `<th>`.
- Inside `HostTableBody` / `HostTableFoot`, the row's children get
  wrapped in `<td>`.

The inner content of the cell is whatever the child primitive's
own JSX would be — `Text` becomes `<span>{slot}</span>`, `HostButton`
becomes a clickable button, a slot-ref becomes a plain text node,
etc. This keeps composition open: an interactive cell (HostButton
with onClick) costs no special emitter wiring.

### 2.4 `Col` (only inside `HostTableColGroup`)

Lowers to `<col />`. Carries an optional `width` keyword/number
that flows through to the `<col style="width: Xpx" />` attribute,
giving authors stable column widths regardless of cell content.

---

## 3. Non-negotiable contracts

These are the constraints that drove UI31 in the first place. Every
per-backend lowering MUST satisfy both.

### 3.1 Accessibility — native semantics, not div-soup

Each backend's `HostTable` MUST lower to a widget with native
accessibility semantics. Specifically:

| Backend       | Required output                                                                                                                                                              |
|---|---|
| React         | `<table><thead><tr><th>…</th></tr></thead><tbody><tr><td>…</td></tr></tbody></table>` — real HTML table elements. NOT `<div role="grid">`.                                  |
| HTML          | Same as React (static markup).                                                                                                                                              |
| WebComponent  | Shadow-DOM real `<table>` elements. NOT `<div role="grid">`.                                                                                                                |
| SwiftUI       | `SwiftUI.Table` for statically defined columns (macOS 12+ / iOS 16+). Mosaic's canonical runtime-sized Grid columns use `TableColumnForEach` on macOS 14.4+ / iOS 17.4+ and fall back to `List` with `Section` rows on older systems — never `LazyVGrid` of `HStack`, which loses table semantics. |
| Qt            | `QtQuick.Controls.TableView` (Qt 6.2+). NOT a `Repeater` of `RowLayout` of `Text`.                                                                                          |
| Flutter       | `DataTable` widget. NOT a `Column` of `Row` of `Text`. Generated shells require Flutter 3.32+, the first stable release whose underlying `Table` exposes explicit table/row/cell semantics roles. |
| WinUI / XAML  | Component-scoped `Grid`/cell controls whose automation peers implement UIA Table/Grid and TableItem/GridItem patterns for the canonical indexed dynamic shape. The visual tree remains `Grid`/`ItemsRepeater`, preserving arbitrary interactive cell content without the archived Community Toolkit DataGrid dependency. Unsupported shapes keep a structural `Grid` fallback and an explicit native-completeness degradation. NEVER claim table semantics from a plain layout Grid. |

**Verification.** Every UI31-K-* PR's test suite MUST include a
test that grep-asserts the generated output contains the native
element name. React/HTML/WebComp tests grep for `<table` and
`<thead`. Flutter tests grep for `DataTable(`. Qt tests grep for
`TableView`. SwiftUI tests grep for `SwiftUI.Table` or `Table {`.
XAML tests grep for the generated MosaicTable controls and UIA provider
interfaces, and separately pin the conservative `Grid` fallback. Tests that fail
this gate block the PR — no exceptions,
no "we'll do it in a follow-up."

**Keyboard navigation follows the native semantic control.** Backends use the
platform widget when one exists. WinUI's generated semantic cell control supplies
arrow-key focus movement because WinUI has no core DataGrid. Selected-cell
tracking flows through the `selected-row`
/ `selected-col` slots so the host's state model stays in sync with
the platform's selection model.

**ARIA flows automatically.** On the web backends, real `<table>`
elements give us `aria-rowcount` / `aria-colcount` from the browser
automatically when `aria-rowindex` / `aria-colindex` are set. The
emitter sets the index attributes from the IR's row-index /
col-index slots. Screen-reader testing across NVDA / VoiceOver /
TalkBack is part of the per-backend PR's manual test plan (CI can't
run AT software, so this is a human check).

### 3.2 Right-to-left layout — respect the host's direction

Each backend's `HostTable` MUST honour the host's layout direction.
Specifically:

| Backend       | RTL mechanism                                                                                                                                          |
|---|---|
| React         | Emits `dir={dir}` on the root `<table>` (or inherits from `<html dir="rtl">` when the `dir` slot is omitted). Column order flips automatically. |
| HTML          | Static `<table dir="rtl">` when the IR sets `dir: rtl`. Otherwise inherits from the containing document.                                          |
| WebComponent  | Sets `dir` on the shadow-root host element. Inherits from light-DOM ancestor when unset.                                                          |
| SwiftUI       | Inherits from `Environment(\.layoutDirection)`. The `HostTable`'s `dir` slot, when set, wraps the rendered `Table` in `.environment(\.layoutDirection, .rightToLeft)`. |
| Qt            | `LayoutMirroring.enabled: true` on the root container when `dir == rtl`. Inherits from parent otherwise.                                          |
| Flutter       | Wraps the `DataTable` in `Directionality(textDirection: TextDirection.rtl, …)` when `dir == rtl`. Inherits from `Directionality.of(context)` otherwise. |
| WinUI / XAML  | `FlowDirection="RightToLeft"` on the generated MosaicTable root (or structural `<Grid>` fallback). Inherits from `<Window FlowDirection="…">` when unset.            |

**Verification.** Every UI31-K-* PR's test suite MUST include a
test that flips the `dir` input (or its emitter-side equivalent)
and asserts the right knob toggles in the generated output:

- React test: assert generated `.tsx` contains `dir="rtl"` when
  the slot is set.
- HTML / WebComp tests: same.
- Flutter test: assert generated `.dart` contains `Directionality(`
  + `TextDirection.rtl` when the slot is set.
- SwiftUI test: assert `.environment(\.layoutDirection, .rightToLeft)`.
- Qt test: assert `LayoutMirroring.enabled: true`.
- XAML test: assert `FlowDirection="RightToLeft"`.

Tests that fail this gate block the PR.

**Bidi text rendering** (numerals in Arabic locales, mixed
LTR/RTL strings) is the platform's responsibility — the kernel
just sets the direction, the platform handles the rest. No
per-emitter bidi handling needed.

### 3.3 Why these are non-negotiable

Accessibility and RTL are not v2 features. A grid widget without
arrow-key navigation, without screen-reader announcements, without
column-flip on RTL locales is unusable for half the users of any
non-trivial app. We accept the cost of native widgets per backend
(more emitter code per primitive, larger PRs, harder cross-backend
testing) in exchange for getting all four (a11y + RTL + keyboard +
focus) for free from each platform's table widget.

---

## 4. Migration — the existing `Grid` built-in primitive

The pre-UI31 `Grid` primitive (UI26 §3.1) is:

- Only lowered by the React emitter (produces a real `<table>` with
  sticky header, cell selection, etc.).
- Hard-coded into the emitter — no userland override possible.
- Errors out or emits placeholders on every other backend.

After UI31 lands, the `Grid` primitive is **deprecated**:

1. **Now (UI31-K-react ships):** `Grid` continues to work on React
   for back-compat. A deprecation warning fires at compile time
   advising authors to migrate to `HostTable`-composed layouts. The
   built-in stays in `moslayout-compiler::PRIMITIVES` but marked
   `#[deprecated]` in the compiler source.

2. **UI31-K-{html,webcomp,flutter,qt,swiftui,xaml} ship:** every
   non-React backend now has a real working table primitive. The
   `Grid` built-in's "not yet supported" error message is amended
   to "deprecated; use HostTable* primitives. See
   `code/specs/UI31-host-table.md` for migration."

3. **VisiCalc Grid.mll migrated (UI31-L10 in the plan):** the demo's
   `Grid.desktop.mll` is rewritten to compose from `HostTable*`
   primitives. The hand-written grids in every VC2-* demo
   (VC2-html/webcomp/flutter/qt/swiftui/xaml) get stripped because
   the auto-generated Grid component now lowers correctly on every
   backend.

4. **One release cycle after #3:** the `Grid` built-in primitive is
   removed from `PRIMITIVES`. Any package that still references it
   errors at compile time with a pointer to this spec.

The deprecation gives downstream packages (mosaic-pkg-grid, any
user-authored grids) a clear migration path — write the layout
once using `HostTable*`, get all six non-React backends for free,
and don't worry about the legacy built-in disappearing under your
feet.

---

## 5. Implementation plan

The UI31 cycle splits into a kernel spec (this doc), kernel-roster
update (PR), seven per-backend lowerings, then two VisiCalc-payoff
PRs. Each gets its own branch and PR:

| Item   | Branch                                        | Scope |
|---|---|---|
| L1 (this PR) | `feat/ui31-host-table-spec`           | Spec only. |
| L2     | `feat/ui31-k-react`                         | mosaic-emit-react: `HostTable*` → real `<table>`. Includes a11y + RTL gates. |
| L3     | `feat/ui31-g-kernel-primitives`             | Add 6 names to `moslayout-compiler::PRIMITIVES` and `mosaic-package-resolver::KERNEL_PRIMITIVES`. Kernel grows 21 → 27. |
| L4-L9  | `feat/ui31-k-{html,webcomp,flutter,qt,swiftui,xaml}` | One PR per remaining backend, same shape as L2. |
| L10    | `feat/visicalc-grid-to-host-table`          | Rewrite `code/programs/mosaic/visicalc/Grid.{desktop,touch}.mll` to compose from `HostTable*` primitives instead of the built-in `Grid`. |
| L11    | `feat/visicalc-demos-strip-handwritten-grids` | Replace the hand-written grid blocks in every VC2-* demo with mounts of the now-correctly-generated Grid component. |

The cycle's order intentionally puts L2 (React) before L3 (kernel
roster) so the React emitter can be developed against the existing
`Grid` built-in as a reference; once L2 ships the roster update in
L3 doesn't risk breaking the active emitter.

---

## 6. Open questions

1. **HostTableColumn / column definitions.** SwiftUI `Table` and
   WinUI `DataGrid` model columns as first-class objects (each
   carrying its own header text, width, sort handler). The
   HostTableHeaderCell-as-child approach in §2 is HTML-shaped;
   should we add a `HostTableColumn` primitive that parallels
   header cells but lives at the table root? Deferred — v1 of UI31
   keeps the HTML-shaped tree; an amendment can add column
   definitions when authors need them.

2. **Virtualization.** Large tables (10k+ rows) need viewport
   windowing. HTML doesn't ship this; SwiftUI `Table` virtualizes
   automatically; WinUI `DataGrid` virtualizes automatically; Qt
   `TableView` virtualizes automatically; Flutter `DataTable` does
   NOT virtualize (use `DataTable2` package or row-windowing in
   author code). v1 of UI31 documents the per-backend default but
   doesn't add a `virtualize: bool` slot. Follow-up if a real
   author hits the wall.

3. **Multi-column sort.** Browser `<table>` doesn't ship sorting;
   SwiftUI/WinUI/Qt do. Out of scope for v1.

4. **Inline edit vs editing-cell overlay.** Two patterns exist:
   the cell renders an `<input>` in place when `edit-row==r &&
   edit-col==c` (browser pattern, what UI26 Grid does today), OR
   the table dispatches `onCellEdit` and the host overlays an
   editing UI. v1 picks the in-place pattern because every
   platform supports it (HTML, Flutter, Qt, SwiftUI, WinUI) and
   it round-trips cleanly through the slot model.

5. **HostTable in a userland package.** Should we ship a
   `mosaic-pkg-table` userland component (matching
   `mosaic-pkg-dialog`'s thin-wrapper pattern) for authors who
   want a styled default? Probably yes once UI31-K-* settles —
   deferred to UI31-P (post-cycle, not blocking).

---

## 7. Out of scope

- Sorting (column-click sort).
- Filtering / search.
- Multi-row selection.
- Drag-to-reorder columns or rows.
- Pagination controls.
- CSV / Excel export.
- Frozen columns (only frozen header row in v1).
- Variable row heights.

All of these are author-level concerns that can be added in v2 of
UI31 or in a userland `mosaic-pkg-table-pro` package — they're not
required for the kernel-primitive contract.

---

**Reviewer checklist:**

- [ ] Are the six primitives (HostTable + 5 children) the right
      decomposition? Should HostTableColumn be added in v1?
- [ ] Is the a11y contract (§3.1) airtight? Are the grep-assert
      tests sufficient verification, or do we need WCAG automated
      tests (axe-core for web, Accessibility Inspector for
      SwiftUI, etc.) as well?
- [ ] Is the RTL contract (§3.2) airtight? Does the spec correctly
      describe each backend's native direction-flip mechanism?
- [ ] Does the migration plan (§4) give downstream packages enough
      lead time? Is one release cycle from "all backends ship" to
      "Grid built-in removed" too aggressive?
- [ ] Are the open questions (§6) all genuinely deferrable, or do
      any of them block v1?

## 8. Measured viewport capacity (proposed; #14372)

VisiCalc's scrolling acceptance (#14277) exposes a distinction from UI48:
`size-class` selects an application layout, while a table's **row capacity**
is a measurement of one control. Do not add pixel dimensions to UI48 or teach
individual hosts VisiCalc's row height. This amendment is a contract for the
next implementation; it is not a claim that the observer ships today.

### Authoring and ownership

A table may opt in with `onViewportRows: emit: onViewportRows`, where the emit
has the single parameter `rows: number`. Without that binding, generation and
behavior remain unchanged. A userland Grid must forward the event explicitly;
unwired consumers must not acquire an undeclared dispatch event.

The generated host measures geometry. The application owns the logical row
window, workbook bounds and selection. `rows` is a positive integral capacity,
not an absolute workbook row, a scroll offset, or a request to move selection.
An adapter clamps measured capacity to its own data bounds before applying its
existing resize operation. VisiCalc can retain the strict validation on its
public `resizeViewport` event by using a separate capacity-event translation.

### Measurement contract

- Measure the nearest bounded scroll frame's **client height**, which excludes
  borders and the horizontal scrollbar. The frame must have a definite height;
  a maximum height alone allows rendered content to change its own constraint.
- Subtract any pinned table header/footer occlusion. Derive body-row pitch from
  actual rendered geometry, including inter-row spacing. Do not assume the
  authored content-box height is the full pitch.
- For uniform rows, report `max(1, floor(availableHeight / rowPitch))`. This is
  the number of fully visible body rows; overscan is a separate app policy.
- An empty body, hidden frame, or zero-size row supplies no valid measurement:
  emit nothing and retry when measurable geometry appears. A consumer must
  initially supply at least one measurable row rather than wait for an event
  before supplying any rows. Do not turn an unmeasurable state into capacity 1.
- Variable-height rows require a separate contract; this first observer must
  detect unsupported geometry and avoid reporting a misleading uniform-row
  capacity. It must expose the limitation through the host's diagnostic path.

### Lifecycle and event stability

Observe changes to the frame and representative row/header/footer geometry,
not only window resize. Font/text-scale changes and changes to surrounding
chrome can alter capacity without a window resize.

Deliver outside the React commit callback, coalesce measurement, and emit only
when a valid capacity changes. Keep the last reported capacity across renders;
rebinding a callback must not re-emit the same initial value. Disconnect all
observers and cancel queued deliveries when the table unmounts or its frame
changes. A stale callback must not dispatch into a disposed app. Changing the
row window in response must settle, not cause a resize/render feedback loop.

### Acceptance required before enabling VisiCalc

1. Resize one running application through small, tall and small frame heights;
   verify capacity changes and settles without repeated unchanged events.
2. Repeat with changed row pitch/text scale, a horizontal scrollbar, and a
   pinned header. Measure the active cell's bounds after shrinking while the
   selection is at the start, middle and end of the workbook.
3. Check rendered-row bounds and logical selection, including an empty workbook
   that still renders editable rows. Test hidden-to-visible and unmount/remount.
4. Prove cleanup prevents post-disposal dispatch and observer accumulation.
5. Compile and exercise generated React 18/19 consumers, including the pinned
   TypeScript 5.7 TaskApp host. Keep existing unrelated tables unchanged.
6. Report native observer support honestly. React acceptance does not establish
   WinUI, Qt, Flutter, Compose or SwiftUI behavior; each needs its own resize gate.

## 9. Row headers and data-cell coordinates

Tracking: #14388 under #14277 and the VisiCalc reference-app epic #14267.
The React baseline is implemented; native acceptance remains under #14388.
Section 2 describes the currently shipped section-only cell inference; that
behavior remains the default for tables without authored row headers.

### React authoring baseline

Use `table-cell-role: row-header`, `column-header`, `corner` or `data` on
a Text or structural Box directly inside a table Row, including a per-cell For
body. Roles are literal keywords. Row and column headers become `th` with the
corresponding scope; corner becomes an aria-hidden `td`; data becomes `td`.
The part's authored styles apply to the cell wrapper with default padding zero.
A structural Box becomes that wrapper and contributes its children directly;
it may not carry other props whose behavior would be lost by removing the Box.
Text remains an inner span, with its authored geometry on the wrapper.
Unannotated nodes retain the original section-inferred wrapper and inner styles.

`mosaic-pkg-grid::RowHeaderGrid` is an opt-in sibling of Grid using the same Cell
component and event contract. It accepts a `row-headers: list<text>` parallel to
viewport-rows. The Rust adapter derives these labels from absolute row identity;
the layout does not calculate or insert domain row numbers into data values.
Its separate 48px header column does not enter column-widths or column-headers.
Consumers of the existing Grid need no new slots, events or changed markup.

React reveal counts body `td` data cells and reserves sticky row-header bounds
on the logical leading side. Other backends report
`accessibility.authored-table-cell-unimplemented` for annotated cells; strict
native-complete builds reject that degradation. Native support is not implied.

### Semantic intent and compatibility

A body row may have a leading, non-editable header identifying the row. This is
header metadata, not an extra value in the row's data array. The header must
lower to the platform's row-header semantics: for React/HTML, a `th` with
`scope="row"` inside the body row. A visual Box or `td` containing a number does
not satisfy this requirement. Column labels remain column headers. A blank
corner above the row headers must not be announced as a data column.

The package must opt in explicitly. Existing Grid consumers that supply only
viewport-rows, column-headers and column-widths retain their current data shape
and behavior. Adding a row-header feature must not require every composed
consumer to supply new bindings or forward new events. Default binding and
unforwarded-event behavior must be tested through the package resolver.

Do not add a VisiCalc-specific primitive or emitter branch. The implementation
must establish a reusable authoring mechanism for row-header intent and actual
cell-wrapper styling, then compose it in Grid. Record the chosen syntax and
backend support in this section when it ships; native backends must diagnose
unsupported semantics rather than presenting an ordinary data cell as supported.

### Three coordinate spaces

Keep these separate throughout the adapter, generated event wiring and reveal:

| Coordinate | Meaning | Example at the bottom of a 100-row workbook |
|---|---|---|
| Workbook row | Absolute, zero-based domain row | 99 |
| Rendered row | Index within the current viewport-rows | 2 for a three-row window starting at workbook row 97 |
| Data column | Zero-based index within the row's data values | 25 for Z, even with a row header |

The row header is excluded from data-column indices. Adding or hiding it must
not change selected-col, edit-col, click payloads or formula targets. A physical
DOM-cell index is not a data-column index once header cells are present. Reveal
must identify the corresponding data cell by semantics or explicit generated
metadata, not by assuming `row.cells[col]` is always the correct element.

The application adapter supplies absolute row identity independently of rendered
row position. In VisiCalc, a window at offset 97 displays labels 98, 99 and 100;
it must not restart at 1 after a resize or scroll. Row identity is presentation
metadata: it must not be inserted into spreadsheet-core values or serialized as
an extra workbook column. Column width and heading arrays continue to describe
data columns; the leading header owns its own width.

### Geometry and selection visibility

Use the actual cell wrapper for header semantics, borders, padding and sticky
positioning. A styled inner element does not control the browser's default
`td`/`th` padding; stacking both can create extra gutters and change measured row
pitch. The implementation must make wrapper geometry authorable and verify its
interaction with the inner editable Cell instead of relying on browser defaults.

A pinned leading header must stay aligned with body rows during vertical scroll
and remain visible during horizontal scroll. Column headers remain aligned with
data columns. The corner must have an intentional stacking order relative to
both pinned regions. Account for layout direction using the platform's logical
leading edge; do not hard-code the scrollbar side or assume positive scrollLeft.

Selection reveal must reserve the visible space occupied by sticky row headers,
just as it reserves sticky column-header height. Revealing column A must not
place A behind the row header. Revealing Z must not scroll the page or move
formula-editor focus. Capacity observation continues to measure body-row pitch;
adding a header must not create resize/dispatch feedback or duplicate row counts.

### Required acceptance

- Compare a Grid with and without headers: identical data coordinates, emitted
  click/edit payloads and workbook mutations; legacy consumers compile unchanged.
- Navigate and edit A1, the first row after a shifted window, and Z100 through
  generated controls backed by the real Rust app. Verify absolute labels after
  shrinking and expanding the same running application.
- Inspect semantic row and column headers and their associations. Row labels
  must not become editable cells or extra stops in the data-cell keyboard path.
- Measure header/data column alignment, row pitch, sticky-header clearance and
  rendered-row bounds while scrolling horizontally and vertically. Cover light
  and dark themes, narrow layout, changed text scale and layout direction.
- Test reveal against header-bearing DOM rows, including the first and final
  data columns. Keep the existing no-header behavior covered.
- Run native interaction and accessibility acceptance for each claimed backend;
  source generation and React screenshots alone do not establish native support.

Whole-workbook physical scroll transport remains a separate requirement of
#14277. Numbered labels and bounded rendering must not be described as completed
virtual scrolling while the scrollbar can only traverse the current slice.

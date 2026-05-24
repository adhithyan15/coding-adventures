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

## 2. The six primitives

```
HostTable                ← root container
├── HostTableHeader      ← <thead> equivalent
│   └── HostTableRow
│       └── HostTableHeaderCell
└── HostTableBody        ← <tbody> equivalent
    └── HostTableRow
        ├── HostTableHeaderCell  (row-label cells)
        └── HostTableCell        (data cells)
```

The tree shape mirrors HTML's `<table>` exactly — author-side
mental model maps one-to-one to the most familiar table syntax.
On non-HTML backends the emitter walks the same tree but lowers
each node to the native table-widget API (`SwiftUI.Table`'s
`TableColumn`, WinUI `DataGrid`'s `DataGridColumn`, etc.).

### 2.1 `HostTable`

The root container. Carries slots shared by the whole table.

| Prop              | Kind        | Required | Meaning                                                                 |
|---|---|---|---|
| `selected-row`    | slot/number | no       | Index of the currently-selected row (or -1 for none).                   |
| `selected-col`    | slot/number | no       | Index of the currently-selected column (or -1 for none).                |
| `edit-row`        | slot/number | no       | Index of the row currently being edited (or -1 for none).               |
| `edit-col`        | slot/number | no       | Index of the column currently being edited (or -1 for none).            |
| `sticky-header`   | keyword     | no       | `true` (default) / `false`. When true, header row pins on scroll.       |
| `total-height`    | slot/number | no       | Pixel height of the scroll viewport. Required when sticky-header=true. |
| `dir`             | slot/keyword| no       | `ltr` (default) / `rtl` / `auto`. Drives layout direction.              |
| `onNavigate`      | emit        | no       | Fires when selection moves. Payload: `{row: number, col: number}`.      |
| `onCellEdit`      | emit        | no       | Fires when an edit commits. Payload: `{row, col, value: text}`.         |

### 2.2 `HostTableHeader` / `HostTableBody`

Wrappers that map to `<thead>` / `<tbody>`. Both accept children
that are `HostTableRow` nodes; the emitter rejects any other child
type with a clear error.

No own slots — they're pure structural markers so the emitter knows
which rows belong above the sticky-header fold and which scroll
below.

### 2.3 `HostTableRow`

A single row. Children are `HostTableHeaderCell` and/or
`HostTableCell` nodes.

| Prop              | Kind        | Required | Meaning                                                |
|---|---|---|---|
| `row-index`       | slot/number | no       | Author-supplied 0-based row index. The emitter passes it through to backend-specific row-index attributes (HTML's `aria-rowindex`, SwiftUI's `id`, etc.). When omitted, the emitter infers from sibling position. |

### 2.4 `HostTableHeaderCell` / `HostTableCell`

Individual cells.

| Prop          | Kind        | Required | Meaning                                                         |
|---|---|---|---|
| `content`     | slot/string | no       | The cell's visible text. When omitted, children render in place. |
| `col-index`   | slot/number | no       | Author-supplied 0-based column index. Same role as `row-index`.  |
| `colspan`     | slot/number | no       | HTML `colspan` equivalent. Defaults to 1.                        |
| `rowspan`     | slot/number | no       | HTML `rowspan` equivalent. Defaults to 1.                        |
| `onClick`     | emit        | no       | Fires when the cell is clicked. Bubbles up to the table's `onNavigate`. |

`HostTableHeaderCell` and `HostTableCell` are structurally
identical at the prop level; the type distinction tells the emitter
which native element to use (`<th>` vs `<td>`, `TableColumn` header
vs body, etc.).

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
| SwiftUI       | `SwiftUI.Table` (macOS 12+ / iOS 16+). Older deployment targets must fall back to `List` with rows — never `LazyVGrid` of `HStack`, which loses table semantics.            |
| Qt            | `QtQuick.Controls.TableView` (Qt 6.2+). NOT a `Repeater` of `RowLayout` of `Text`.                                                                                          |
| Flutter       | `DataTable` widget. NOT a `Column` of `Row` of `Text`.                                                                                                                       |
| WinUI / XAML  | `Microsoft.UI.Xaml.Controls.DataGrid` from the Windows Community Toolkit. If unavailable, `Grid` with `Grid.RowDefinitions`/`ColumnDefinitions` + `AutomationProperties.Name` per cell to preserve table semantics for Narrator. NEVER `StackPanel` of `StackPanel`. |

**Verification.** Every UI31-K-* PR's test suite MUST include a
test that grep-asserts the generated output contains the native
element name. React/HTML/WebComp tests grep for `<table` and
`<thead`. Flutter tests grep for `DataTable(`. Qt tests grep for
`TableView`. SwiftUI tests grep for `SwiftUI.Table` or `Table {`.
XAML tests grep for `<DataGrid` (or document the `Grid` fallback
explicitly). Tests that fail this gate block the PR — no exceptions,
no "we'll do it in a follow-up."

**Keyboard navigation comes from the native widget.** None of the
emitters need to hand-roll arrow-key handlers — the platform widget
ships them. Selected-cell tracking flows through the `selected-row`
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
| WinUI / XAML  | `FlowDirection="RightToLeft"` on the root `<DataGrid>` (or `<Grid>` fallback). Inherits from `<Window FlowDirection="…">` when unset.            |

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
| L10    | `feat/visicalc-grid-to-host-table`          | Rewrite `demo/visicalc/mosaic/Grid.{desktop,touch}.mll` to compose from `HostTable*` primitives instead of the built-in `Grid`. |
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

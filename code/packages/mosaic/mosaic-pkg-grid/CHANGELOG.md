# Changelog

All notable changes to `mosaic-pkg-grid` are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/) and
the package follows semantic versioning.

## 0.2.1 — 2026-06-04 — UI34 package-resolver compatibility

End-to-end resolver compatibility: with UI34 PRs #4969 (resolver) +
#4974 + #4972 merged, a consumer's `.mll` can now write
`pkg::mosaic-pkg-grid::Grid (…)` and have it byte-identically
expand to the same kernel-primitive composition the visicalc
demo used to inline by hand.  All eight VisiCalc demos (React,
HTML, WebComponent, SwiftUI, Qt, Flutter, Compose-Desktop,
Android) now consume the package directly via `mosaic-compile
--package-search-path`.

### Added

- **`Cell.mil`** gains a `slot edit-content : text` and an
  `emit onChange ( value : text )`.  `edit-content` carries the
  host's live edit buffer, and `onChange` fires per keystroke
  so a host reducer can keep it in sync — this is the missing
  half of the controlled-`<input>` round-trip that a bare
  `slot: value` could not complete (React rejects keystrokes on
  a controlled input without an `onChange`).  Landed in
  PR #4974.
- **`Grid.mil`** gains a matching `emit onFormulaChange ( value :
  text )` so consumers can route per-keystroke cell-edit events
  out to a FormulaBar-style sibling.  PR #4974.
- **`Cell.mll`** wires `state-when-selected: slot: is-selected`
  and `state-when-editing: slot: is-editing` on the Box so the
  `cell:selected` / `cell:editing` mosstyle blocks fire when the
  host (or Grid) passes the matching boolean.  Earlier draft
  used the expression form `( is-selected )`; the slot-ref form
  routes through the UI34 resolver's `rewrite_bindings` step so
  the call-site predicate (e.g. `r == selectedRow && c ==
  selectedCol`) ends up inlined verbatim instead of leaving the
  literal identifier `is-selected` (an invalid JS identifier
  because of the hyphen) in the emitted code.  PRs #4972,
  #4974.
- **`Cell.dark.msl`** gains a `state selected { … }` block
  matching the visicalc palette (#264f78 background, #007acc
  accent outline, #ffffff text) so consumers that style the
  grid via `pkg::mosaic-pkg-grid::Cell` get a sensible
  selection highlight default without re-declaring it at every
  call site.  PR #4972.

### Changed

- **`Grid.mll`** Cell call site now passes `edit-content` and
  `onChange` through to Cell, and drops the previously-unused
  `[ body ]` part-name label so Cell's own `cell` part flows
  through to the consumer's `.msl` (per UI34 §5.1, a call-site
  part-name shadows the resolved root's).  PR #4974.

## 0.2.0 — 2026-05-25 — UI28-1 cell-and-column composition complete

The v0.1.0 scaffold finishes. The two declared v0.1.0 follow-ups
(empty header row; body shows only `row`, not per-column cells) are
both closed. Grid now renders ALL cells across both axes, the header
row carries one `<th>` per column, the `<colgroup>` carries per-
column widths, and per-cell selection / editing predicates are
computed at the Cell call site without spilling the comparison logic
to either the host (encapsulation upward) or to Cell (encapsulation
downward).

This is the version UI28-1 §3 specifies — the userland-Grid
counterpart to UI28's never-shipped Cell-as-kernel-primitive draft.

### Added

- **`Cell.mil`** gains an `is-selected: bool` slot. The Cell.mll
  layout doesn't branch on it directly — sub-part styling
  (`cell:selected`) does — but the slot has to be declared so Grid
  can thread the value through the component-ref boundary.
- **`Grid.mil`** gains two parallel-array slots:
  - `column-headers : list<text>` — one header label per column.
  - `column-widths  : list<number>` — pixel width per column.
  Per UI28-1 §9 open-question note: parallel arrays are a v0.2.0
  pragmatism. mosmodel doesn't yet support record types, so we
  cannot declare `list<column-meta>`. v0.3.0 migrates.
- **`Grid.mll`** is the headline change. It now contains:
  - **`HostTableColGroup`** section with `For (each: slot:
    column-widths, ...)` driving one `<col width>` per column.
  - **`HostTableHead`** section with `For (each: slot:
    column-headers, ...)` rendering the header row — previously
    empty.
  - **`HostTableBody`** with nested For: outer over rows, inner
    over each row's cells. The inner uses UI29 §3.4 (For-binding-
    as-iterable: `For (each: row, as: v, index: c)` — landed in
    [PR #4398](https://github.com/adhithyan15/coding-adventures/pull/4398)).
    Each Cell receives its `(r, c)`-determined `value` plus the
    predicate-computed `is-editing` / `is-selected` booleans.
- **Per-cell predicates via expression-in-slot-binding** at the
  Cell call site:
  - `is-editing: ( r == editRow && c == editCol )`
  - `is-selected: ( r == selectedRow && c == selectedCol )`
  The `(...)` grouping triggers the moslayout-compiler's Expr
  branch (UI29 §3.3); the expression text passes verbatim into the
  target language at each emitter.
- **Stable iteration keys** flow through every For via explicit
  `index:` bindings (`cw`, `ch`, `r`, `c`). Each emitter threads
  these into its framework-native list key: React `key={r}` (UI28-1
  §6.3, [PR #4396](https://github.com/adhithyan15/coding-adventures/pull/4396)),
  Flutter `KeyedSubtree(ValueKey(r))` (§6.2, [PR #4393](https://github.com/adhithyan15/coding-adventures/pull/4393)),
  SwiftUI `ForEach(id: \.offset)`, Qt `Repeater` index property,
  XAML `ItemsRepeater` x:Bind. HTML/WebComponent are static so no
  key is needed.
- **7 new tests** added to `tests/package_compiles.rs`:
  - `ui28_1_cell_declares_is_selected_slot`
  - `ui28_1_grid_declares_column_headers_and_widths_slots`
  - `ui28_1_grid_mll_uses_nested_for_via_u29_3_4_scope` (the
    end-to-end regression guard for PR #4398's scope walker)
  - `ui28_1_grid_mll_predicate_uses_expression_in_slot_binding`
  - `ui28_1_grid_mll_has_colgroup_section_for_column_widths`
  - `ui28_1_grid_mll_header_for_renders_column_headers`
  - `ui28_1_no_new_kernel_primitive_was_added` (sanity guard:
    every tag in our .mll files is either UI29 kernel or this
    package's userland Cell/Column — Cell-as-component per UI28-1
    §2 constraint 1 must hold)

  Total tests: 12 (was 5, +7). All green.

### Changed

- **`Cell.mil`** preamble rewritten to spell out the encapsulation
  contract (UI28-1 §2 constraint 4): the host pushes plain
  coordinate numbers, Grid composes per-cell predicates, Cell
  receives bools.
- **`Grid.mil`** preamble rewritten to enumerate the v0.2.0
  changes, document the slot vocabulary's parallel-array shape,
  and note the upcoming v0.3.0 record-type migration path.
- **`Grid.mll`** preamble explains the composition end-to-end:
  primitive map, why each section exists, how per-cell predicates
  resolve, where stable keys live, what's out of scope for v0.2.0
  (sticky-header, custom renderers, list virtualization inside
  Grid, etc.).

### Enabled by — three landed dependencies

This release was unblocked by three companion PRs that ship the
mechanical capabilities Grid v0.2.0 builds on. Every one of them
ships kernel-mechanism work — none promote new kernel primitives:

- **[PR #4388](https://github.com/adhithyan15/coding-adventures/pull/4388)** — UI28-1 spec (the authoritative design)
- **[PR #4393](https://github.com/adhithyan15/coding-adventures/pull/4393)** — Flutter For/If/Else lowering (§6.2)
- **[PR #4396](https://github.com/adhithyan15/coding-adventures/pull/4396)** — React For auto-key from `index:` (§6.3)
- **[PR #4398](https://github.com/adhithyan15/coding-adventures/pull/4398)** — UI29 §3.4 For-loop binding scope + 5-emitter Keyword arm (§6.1)

### Known limitations / out of scope

The list from UI28-1 §7, restated here for the changelog reader:

- **Sticky header** — author composes `HostScroll { Grid {...} }`
  themselves. Deferred per UI28-1 §2 constraint 5.
- **Custom cell renderers** (image, button, checkbox, sparkline) —
  v0.3.0 will extend Cell's `cell-type` to switch.
- **Column groups, sortable headers, pinned columns** — each its
  own design problem (UI28-2).
- **List virtualization INSIDE Grid** — needs a `HostVirtualList`
  kernel primitive. Today the HOST slices to viewport before
  pushing, which works fine for the VisiCalc-scale (100×26).
- **Mosmodel record type** — when it lands, `column-headers` +
  `column-widths` collapse to `columns: list<column-meta>`.

## 0.1.0 — 2026-05-19

Initial release. Exports Grid, Cell, Column. Built on UI29 kernel
primitives (Box, Row, Text, HostInput, HostTable + sub-tags, If, For).

### Added

- `mosaic-package.toml` manifest declaring `[components].exports =
  ["Grid", "Cell", "Column"]` and targeting UI29 kernel version `"1"`.
- `Cell.mil` / `Cell.mll` / `Cell.dark.msl`: an editable spreadsheet
  cell composed of `Box` + `If`/`Else` + `HostInput` + `Text`.
- `Column.mil` / `Column.mll`: a metadata-only column declaration with
  a single hollow `Box` marker layout (moslayout currently requires a
  single root node per component).
- `Grid.mil` / `Grid.mll` / `Grid.dark.msl`: the data grid itself,
  composed of `HostTable` + `HostTableHead` + `HostTableBody` + `Row` +
  `For` + the package's own `Cell` component.
- `tests/package_compiles.rs`: integration smoke test that round-trips
  every source file through `mosmodel-compiler`, `moslayout-compiler`,
  and `mosstyle-compiler`.

### Known limitations

- **Header row is empty in v0.1.0.**  Rendering one header cell per
  declared Column requires the Grid interface to accept a `columns`
  list slot (v0.2.0).
- **Body rows show `row` only.**  Full per-column iteration needs the
  same `columns` slot plus the expression-grammar `row[c]` field-access
  syntax defined in UI29 §3.3 but not yet landed (UI29-G3).
- **Backend lowerings still landing.**  `For`, `If`, `HostInput`, and
  `HostTable` (with `HostTableHead` / `HostTableBody` / `Row` slots) are
  kernel primitives whose per-backend lowerings are in flight under
  `U29-K-react`, `U29-K-swiftui`, `U29-K-qt`, `U29-K-webcomp`, and
  `U29-K-html`.  The smoke test asserts language-frontend correctness
  (parse + analyze + validate at the .mil/.mll/.msl layer); end-to-end
  backend artifact generation lights up once those PRs land.
- **No theming cascade.**  The package ships `dark.msl` only; light-mode
  styles and host overrides arrive once the multi-theme cascade lands
  in mosstyle.

The architecture (manifest-driven, kernel-primitive-only composition,
backend-agnostic) is what v0.1.0 proves.  v0.2.0 fills in the Grid
behaviour once the supporting grammar / resolver / kernel-coverage PRs
land.

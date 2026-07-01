# mosaic-pkg-grid

> Spreadsheet-style data grid built on UI29 kernel primitives.

This is the first userland Mosaic component package — the headline
deliverable of **UI29 (Primitive Kernel + Userland Component Packages)**.
It proves the architecture: the framework core ships only ~15 kernel
primitives, and useful components like Grid live *outside* the core, in
plain moslayout source files, the same shape as any user component.

## What this package exports

Three components, listed in `mosaic-package.toml`'s `[components].exports`:

| Component | Role | File trio |
|---|---|---|
| `Grid`   | the spreadsheet itself — header row + data rows | `Grid.mil` / `Grid.mll` / `Grid.dark.msl` |
| `Cell`   | one editable cell (read display ↔ edit input) | `Cell.mil` / `Cell.mll` / `Cell.dark.msl` |
| `Column` | declarative column metadata (no rendered output) | `Column.mil` / `Column.mll` |

Column ships no `.msl`: it produces no visible tree, so there is nothing
to style.

## How it fits in the stack

```
              ┌────────────────────────────────────────────┐
              │  Host application (.mll uses `Grid`)       │
              └─────────────────────┬──────────────────────┘
                                    │ component reference
                                    ▼
              ┌────────────────────────────────────────────┐
              │  mosaic-pkg-grid (this package)            │
              │  Grid → HostTable + For + Cell             │
              │  Cell → Box + If + HostInput + Text        │
              │  Column → metadata-only Box marker         │
              └─────────────────────┬──────────────────────┘
                                    │ kernel primitives only
                                    ▼
              ┌────────────────────────────────────────────┐
              │  UI29 kernel (15 primitives)               │
              │  Box / Row / Column / Stack / Text /       │
              │  Image / Spacer / Divider / Icon /         │
              │  If / For / HostInput / HostButton /       │
              │  HostTable / HostScroll                    │
              └─────────────────────┬──────────────────────┘
                                    │ per-backend lowering
                                    ▼
                  React / SwiftUI / Qt / WebComponent / HTML
```

A host that wants a Grid adds `mosaic-pkg-grid` to its package
dependencies; a host that doesn't, pays nothing.  A host that wants a
*richer* Grid can fork this package and publish their own — the same way
React's ecosystem treats data-grid libraries as userland.

## v0.1.0 architecture (what is proven)

* Three components, all manifest-declared.
* Three components, all composed entirely from kernel primitives — no
  bespoke per-backend code.
* Three components, all expressed in plain moslayout source files (the
  same `.mil` / `.mll` / `.msl` trio any user component uses).
* The smoke test at `tests/package_compiles.rs` round-trips every source
  file through its respective compiler crate.

## v0.2.0 — what's done (the v0.1.0 caveats are CLOSED)

v0.1.0 shipped a working scaffold with two declared follow-ups. As of
v0.2.0 (per [UI28-1](../../specs/UI28-1-grid-v3-userland-revised.md)),
both are CLOSED:

* **Header row renders.** `Grid.mll`'s `HostTableHead` now contains
  `For (each: slot: column-headers, as: h)` producing one `<th>` per
  column. Grid.mil gained a `column-headers: list<text>` slot.
* **Body iterates per-column.** The body is nested-For now:
  `For (each: slot: viewport-rows, as: row) { Row { For (each: row,
  as: v) { Cell ( value: ( v ), ... ) } } }`. The inner
  `For ( each: row, ... )` is the UI29 §3.4 "For-binding-as-iterable"
  shape ([PR #4398](https://github.com/adhithyan15/coding-adventures/pull/4398)).
* **`<colgroup>` carries per-column widths.** `HostTableColGroup`
  with `For (each: slot: column-widths, as: w)` emits one `<col>`
  per column with its declared width.

Per-cell `is-editing` / `is-selected` predicates are computed at the
Cell call site using expression-in-slot-binding:

```
is-editing:  ( r == editRow && c == editCol )
is-selected: ( r == selectedRow && c == selectedCol )
```

The host pushes only the plain coordinate slots (`edit-row`,
`selected-col`, …). Grid is the encapsulation boundary that converts
coordinates → per-cell booleans. Cell receives booleans, never
coordinates.

## What's still out of scope (deferred to UI28-2 / v0.3.0)

* **Sticky header.** Authors compose `HostScroll { Grid { ... } }`
  themselves (UI28-1 §2 constraint 5).
* **Custom cell renderers** (image, button, checkbox, sparkline).
  v0.3.0 extends Cell's `cell-type` to switch.
* **Column groups, sortable headers, pinned columns.** UI28-2.
* **List virtualization INSIDE Grid.** Today the host slices to
  viewport before pushing — `viewport-rows` IS the visible window.
* **Mosmodel record type.** When it lands, the parallel
  `column-headers` + `column-widths` slots collapse to
  `columns: list<column-meta>`.
* **Multi-theme cascade.** Ships `dark.msl` only; light-mode and
  host overrides arrive once the mosstyle cascade lands.

## Usage (v0.2.0, working end-to-end)

The host declares the data + viewport coordinates as slots and wires
emits to its reducer. The Grid does the rest:

```moslayout
// In a host component's .mll:
layout SpreadsheetApp {
  Column [ root ] {
    Grid (
      viewport-rows:  slot: viewport-rows ,
      column-headers: slot: column-headers ,
      column-widths:  slot: column-widths ,
      selected-row:   slot: app-selected-row ,
      selected-col:   slot: app-selected-col ,
      edit-row:       slot: app-edit-row ,
      edit-col:       slot: app-edit-col ,
      edit-content:   slot: app-edit-content ,
      onNavigate:     emit: handleNavigate ,
      onEditCommit:   emit: handleCommit ,
      onEditCancel:   emit: handleCancel
    )
  }
}
```

Grid produces semantic table markup on every backend that has the
UI31 HostTable lowering — React `<table>`, HTML `<table>`,
WebComponent shadow-DOM `<table>`, Flutter `DataTable`, Qt `TableView`
/ `Repeater`, SwiftUI `Grid` / `Table`, XAML `ItemsRepeater` with row
view-models.

### What the host pushes

* `viewport-rows: list<list<text>>` — sliced to the visible window;
  one inner list per displayed row, one cell per column. Grid
  iterates BOTH axes per-render.
* `column-headers: list<text>` — header labels, one per column,
  parallel-shaped to each `viewport-rows[r]`.
* `column-widths: list<number>` — pixel widths, parallel-shaped to
  column-headers. Drives the `<colgroup>`.
* `selected-row` / `selected-col` / `edit-row` / `edit-col` —
  plain numbers, `-1` means "none". Grid composes per-cell
  `is-editing` / `is-selected` from these internally.
* `edit-content: text` — the in-progress edit buffer the host owns.

## Smoke test

```bash
cd code/packages/mosaic-pkg-grid
cargo test
```

The smoke test asserts:

1. `mosaic-package.toml` parses and declares the three expected exports.
2. Each `.mil` compiles via `mosmodel-compiler`.
3. Each `.mll` compiles via `moslayout-compiler` against its `.mil`
   interface descriptor.
4. Each `.dark.msl` compiles via `mosstyle-compiler` against its `.mll`
   part map.

If `Grid.mll` ever stops compiling (because a future resolver PR
tightens its tag-validation to reject unknown primitives), the test
documents the upgrade path: flip Grid's `.mll` assertion to `expect_err`
until U29-R2 + all U29-K-* resolvers cover `HostTable` / `For` / `If` /
userland component references.

## Position in UI29's roadmap

* **U29-P1**: ship the source tree, declare the manifest, prove the
  smoke test. **Done in v0.1.0.**
* **UI28-1 v0.2.0 (this release)**: complete the Cell-and-Column
  composition — full header row, nested-For body, per-cell
  predicates, stable iteration keys. **Done.**
* **U29-D1**: migrate `code/programs/mosaic/visicalc/Grid.{desktop,touch}.mll`
  to reference this package's `Grid` (replacing the local degraded
  HostTable composition the L10 migration shipped). Next.
* **U29-X1**: remove the legacy `Grid` built-in primitive from
  `moslayout-compiler::PRIMITIVES` + its special-case lowering in
  `mosaic-emit-react`, now that Grid is fully userland and only
  kernel-primitive lowering is on the critical path.

## License

MIT OR Apache-2.0.

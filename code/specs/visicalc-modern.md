# VisiCalc Modern Reconstruction

## Overview

VisiCalc Modern is a from-scratch Rust spreadsheet that reproduces
the *idea* of VisiCalc — a grid of cells, formulas with cross-cell
references, instant recalculation — using only modern infrastructure:
the Mosaic UI language, the layered statistics core, and the
spreadsheet engine. The spreadsheet itself is the canary application
that exercises the whole stack end-to-end.

Where the *faithful* reconstruction (`visicalc-faithful.md`) runs the
1979 binary, this track builds the spreadsheet again from current
materials. The two are not layers and do not share code; they share
specs and a vision. The faithful track is preservation; the modern
track is *exercise* — it forces the new substrate (numeric-tower,
r-vector, statistics-core, spreadsheet-core) to be complete enough
to host a real, end-user-facing application, and it forces the
Mosaic UI compiler to be complete enough to render a non-trivial
interactive grid.

The application lives at `code/programs/rust/visicalc-modern/` and
depends on every Layer-0, -1, and -3 crate in the statistics stack
plus the Mosaic compiler and the chosen Mosaic backend (paint-vm
for native, web-component for browser).

---

## Why Modern as Well as Faithful

The faithful track answers "can we make the 1979 program run?" — a
preservation question.

The modern track answers four different questions:

1. **Does the Layer 1 statistics-core have the right shape?** Building
   `=AVERAGE(A1:A10)` end-to-end means writing the spreadsheet-core
   dispatcher, which resolves a name to a function pointer in
   statistics-core. If the function pointer's signature is awkward,
   the dispatcher is awkward, and the design needs revision *before*
   we hand it to the future R runtime where the awkwardness would
   be permanent.
2. **Is the Mosaic UI complete enough for a real application?** A
   spreadsheet with 1000 cells, scrolling, in-place cell editing,
   formula bar, and clipboard is the most demanding UI Mosaic has
   tried to render. If something is missing, the modern VisiCalc
   work surfaces it.
3. **Does the recalc engine perform?** A spreadsheet user expects
   sub-100ms response on a 10 000-cell sheet. If `spreadsheet-core`'s
   recalc algorithm is too slow, this is where we find out.
4. **Can statistics-core's first phase serve as a real formula
   library?** Phase 1 (descriptive + counting + rank) is enough to
   support a VisiCalc-replica function set (`@SUM`, `@AVERAGE`,
   `@MIN`, `@MAX`, `@COUNT`, `@STDEV`, `@VAR`, `@MEDIAN`, `@RANK`).
   The modern track stresses these in a real workload.

---

## Where It Fits

```
   visicalc-modern (binary)              ← THIS SPEC's program
        │
        │ Mosaic .mosaic source
        │ compiles to paint-vm or web-component backend
        ▼
   ┌───────────────────────────────────────────────────────┐
   │   Mosaic-compiled UI                                  │
   │   - cell grid component (virtual scrolling)           │
   │   - formula bar                                       │
   │   - menu / status line                                │
   └─────────────────────────┬─────────────────────────────┘
                             │ user events: edit cell, navigate
                             ▼
   ┌───────────────────────────────────────────────────────┐
   │   spreadsheet-core (Rust)                             │
   │   formula AST + DAG + recalc + dispatch               │
   └─────────────────────────┬─────────────────────────────┘
                             │ delegates math
                             ▼
   ┌───────────────────────────────────────────────────────┐
   │   statistics-core · math-core · financial-core · …    │
   └─────────────────────────┬─────────────────────────────┘
                             ▼
                  numeric-tower · r-vector
```

A follow-up spec, `mosaic-spreadsheet-component.md`, defines the
specific Mosaic component for the cell grid: virtual scrolling, cell
edit mode, selection, formula bar, clipboard. Not in this PR.

---

## §1 What "Modern VisiCalc" Means

The modern track replicates VisiCalc's *user experience* and *function
set* but uses modern materials. Concretely:

| Aspect                | VisiCalc Faithful   | VisiCalc Modern                          |
|-----------------------|---------------------|------------------------------------------|
| Display               | 40×24 text grid     | resizable window, virtual-scrolling grid |
| Input                 | Apple II keyboard   | full keyboard + mouse + clipboard         |
| Function set          | 25 functions (1979) | VisiCalc 25 + Lotus 80 + Excel ~100       |
| Formula language      | A1 refs, `@FN(…)`   | A1 refs, `=FN(…)`, both `@` and `=` accepted |
| File format           | VisiCalc binary    | XLSX (read/write), CSV, JSON-flat         |
| Recalc model          | manual `!`         | automatic / manual / iterative            |
| Performance           | 32 KB / 1 MHz      | 10 000 cells in < 100 ms                  |
| Underlying CPU        | 6502                | native Rust                               |
| Color                 | none                | optional cell formatting                  |
| Charting              | none                | none in v1 (chart-core is a future crate) |
| History (undo)        | none                | full undo/redo                            |
| Multi-sheet           | no                  | yes                                       |

The function set extension is the major modernization. Where 1979
VisiCalc was a small sealed environment, the modern reconstruction
is open-ended — every Excel statistical function, every Lotus
financial function, every R distribution sampler is callable from a
cell. The same function name conventions Excel users expect
(`AVERAGE`, `STDEV`, `NPV`) work, plus the legacy VisiCalc names
(`@SUM`, `@AVG`) for users referencing 1979-era documentation.

---

## §2 Phases

Each phase is its own follow-up implementation PR. The phases assume
the spec PRs A, B, C have landed and the corresponding implementation
PRs are progressing in parallel.

### Phase M1 — Headless Shell

Just enough to prove the dispatch path:

- Construct a `Workbook` programmatically
- Type a few formulas via API (no UI)
- Recalc and assert results
- Tests against a corpus of VisiCalc-original example sheets
  (transcribed from the reference card)

Depends on: spreadsheet-core impl phase 1 (parser + DAG + recalc)
and statistics-core phase 1 (descriptive + counting + rank).

### Phase M2 — Mosaic Cell Grid

Drives out the missing virtual-scrolling table from `code/specs/table.md`
plus the cell-edit interaction:

- Cell rendering with text alignment (numbers right, text left,
  matching VisiCalc)
- Selection model (single cell, range, named range)
- In-place editing: click a cell, type, press Enter
- Formula bar: shows the current cell's formula; edits route to the
  cell

Depends on: Mosaic compiler's web-component or paint-vm backend.

### Phase M3 — VisiCalc Function Parity

Implement and dispatch the original VisiCalc 25-function set,
verifying that each works in a real workbook context:

| Function | Source                                |
|----------|----------------------------------------|
| `@SUM`, `@AVERAGE`, `@MIN`, `@MAX`, `@COUNT` | statistics-core::descriptive |
| `@STDEV` (Lotus addition; not in 1979)        | statistics-core::descriptive::sd_pop |
| `@VAR` (Lotus addition)                       | statistics-core::descriptive::var_pop |
| `@ABS`, `@INT`, `@SQRT`, `@EXP`, `@LN`, `@LOG`, `@SIN`, `@COS`, `@TAN`, `@ASIN`, `@ACOS`, `@ATAN` | math-core (when implemented; until then, std `f64` methods) |
| `@PI`                                          | math-core constant |
| `@NPV`                                         | financial-core (when implemented) |
| `@IF`                                          | spreadsheet-core inlined |
| `@LOOKUP`                                      | lookup-core (when implemented) |
| `@ERROR`, `@NA`, `@TRUE`, `@FALSE`             | spreadsheet-core sentinels |
| `@AND`, `@OR`, `@NOT`                          | spreadsheet-core inlined |

If a Layer 1 crate that hosts a function is not yet implemented when
M3 ships, the function is implemented inline in `visicalc-modern`'s
own dispatch table as a temporary measure, with a TODO and a
follow-up PR queued to extract.

### Phase M4 — File Formats

XLSX read/write via a future `xlsx-io` crate, plus CSV and a
JSON-flat format for diffing. CSV and JSON ship in M4; XLSX may slip
to M5 depending on `xlsx-io` readiness.

### Phase M5 — Polish

- Undo/redo (event-sourced log of edits)
- Multi-sheet workbooks
- Named ranges
- Cell formatting (number format, text alignment, font weight)
- Print preview
- Keyboard shortcut parity with Excel (`Ctrl-C`, `Ctrl-V`,
  `F2` to edit, `Ctrl-Z` to undo, etc.)

### Phase M6 — Excel Function Parity

Beyond VisiCalc's 25, ship the full Excel statistical-function
catalog as those phases of statistics-core ship. Phase M6 is the
ongoing track that follows statistics-core's phase progression.

---

## §3 Mosaic Component Surface

`mosaic-spreadsheet-component.md` (follow-up) details the components.
Outline of what M2 needs:

```
SpreadsheetWorkbook(workbook: WorkbookHandle) {
    Toolbar()
    FormulaBar(active_cell: CellAddress, formula: String)
    SheetTabs(sheets: list<Sheet>)
    SheetView(sheet: Sheet, viewport: Viewport)
}

SheetView(sheet, viewport) {
    ColumnHeaders(start_col, end_col)
    RowHeaders(start_row, end_row)
    CellGrid(visible_cells: Range, selection: Selection)
}

CellGrid {
    // virtual-scrolling, only renders visible cells
    // delegates to Cell for each visible position
}

Cell(value: CellValue, format: CellFormat, selected: bool, edit_mode: bool) {
    when edit_mode { CellEditor(formula: String) }
    else { CellDisplay(formatted_text: String, alignment: Alignment) }
}
```

These are Mosaic component declarations; the compiler emits paint-vm
or web-component code per the chosen backend.

The cell editor needs *autocomplete* for function names, range
references, and named ranges, but autocomplete is a Phase M5
deliverable; M2 ships a plain text input.

---

## §4 The Renderer Bridge

Modern VisiCalc renders through Mosaic. Two backends matter:

- **paint-vm** (native): the Rust paint-vm executes layout-IR
  paint instructions on a window via Direct2D / Quartz / Skia
  (per platform). The visicalc-modern binary is a native
  executable.
- **web-component** (browser): the same Mosaic source compiles to
  Web Components, rendering in a browser tab. Useful for
  notebook embedding and remote demos.

Both share the same `.mosaic` source and the same
`spreadsheet-core` library underneath. The binary chooses backend
by build flag.

A *third* backend, terminal (à la `tput` / `crossterm`), is an
attractive idea for parity with the faithful track's terminal mode.
Out of scope for v1; gets added once paint-vm has a terminal output
target (currently it does not).

---

## §5 Test Plan

End-to-end tests live in `code/programs/rust/visicalc-modern/tests/`:

1. **Headless integration** (no UI): construct workbooks, recalc,
   assert results.
2. **Mosaic UI driver tests** using a headless paint-vm output: drive
   keystrokes, capture rendered frames, assert frame contents.
3. **Excel parity** (read XLSX, recalc, write XLSX, diff against
   Excel-recalculated reference): once `xlsx-io` lands.
4. **VisiCalc reference-card examples**: every formula from the 1979
   reference card runs in modern VisiCalc and produces the same
   numeric result.

Performance tests: 10 000-cell workbook builds, recalcs, scrolls.
Targets per `spreadsheet-core.md` §18.

---

## §6 Out of Scope

- Charting (separate `chart-core`)
- Pivot tables (separate `pivot-core`)
- Conditional formatting (display only; future)
- Data validation (input gating; future)
- Real-time data refresh
- Macros / VBA / LAMBDA / LET (formula-language extension; future)
- Multi-user collaborative editing (CRDT-based; way out of scope)
- Mobile / touch UI

---

## §7 Comparison to Faithful

The two tracks are independent but cross-link:

| Dimension              | Faithful (Python)                  | Modern (Rust)                        |
|------------------------|-------------------------------------|--------------------------------------|
| Boot state             | 1979 binary loaded from .dsk        | Native Rust startup                  |
| CPU                    | mos6502-simulator                   | host CPU                             |
| Display                | text page or CRT shader             | Mosaic-rendered native window or web |
| Function set           | sealed at 25                        | open-ended; grows with statistics-core |
| File format            | VisiCalc binary on disk image       | XLSX, CSV, JSON                      |
| Tests                  | screen-state diffs vs scripted keystrokes | function dispatch + UI driver |
| Performance bar        | "fast enough to use"                | sub-100ms recalc on 10k cells        |
| Educational value      | hardware emulation, 6502 assembly  | UI compilation, formula language, dispatch design |

Cross-linking happens at the spec level (both tracks reference the
same `statistics-core.md` for function semantics, even though the
faithful track does not call into the Rust crate — its functions live
in the 6502 binary). When a function's behavior in the faithful
track differs from the modern track (e.g. `@AVERAGE` over a range
containing a "blank" cell), the spec is the single source of truth
and both implementations are checked against it.

---

## References

- Bricklin & Frankston, *VisiCalc User's Guide* (1979) — function
  reference
- Microsoft Excel function reference — for the modernization's
  expanded function set
- `statistics-core.md` — function semantics
- `spreadsheet-core.md` — engine semantics
- `code/specs/UI00-mosaic.md` — UI compiler
- `code/specs/UI01-mosaic-vm.md`, `UI02-layout-ir.md` — rendering
  pipeline
- `code/specs/table.md` — table component (the virtual-scrolling
  cell grid will land here once specced)

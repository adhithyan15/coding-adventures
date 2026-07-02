# VisiCalc — Compose for Desktop demo (live, on the Rust engine)

The Compose Multiplatform (Desktop) VisiCalc demo, now **computing on the shared
Rust `spreadsheet-core` engine** through its C ABI (`spreadsheet-capi`), reached
via the **Java Foreign Function & Memory API** — the zero-JNI path Compose and
Android use. The same engine the SwiftUI / Qt demos link natively, the Flutter
demo loads via dart:ffi, and the web demos run as WebAssembly. Fourth native
backend on the one engine.

## What it shows

A `Window` (from `androidx.compose.ui.window`) mounting the auto-generated
`FormulaBar` and `Grid` composables (`src/main/kotlin/generated/`, produced by
`mosaic-compile --backend compose` from the shared `code/programs/mosaic/visicalc/*`
sources).

- The grid renders **engine-computed** values: the classic cross-footing budget
  where column E totals each row, row 5 totals each column, and E5 is the grand
  total (169) — all formulas evaluated by the Rust engine, not hard-coded.
- Editing the formula bar writes through to the engine via
  `SpreadsheetModel.setCell`; the host rebuilds `viewportRows` from the engine,
  so Compose recomposes the grid with the recomputed values.

## How it's wired to the engine

```
Compose composables (generated)  ──  SpreadsheetModel / SpreadsheetSession (Engine.kt)
   Grid(viewportRows = …)            │  sc_set_cell / sc_get_value … (Java FFM, String↔char*)
                                     ▼
   native/libspreadsheet_capi.dylib  ←  spreadsheet-capi (Rust C ABI)  ←  spreadsheet-core
```

`Engine.kt` binds the C ABI with `java.lang.foreign` (`Linker`/`SymbolLookup`/
`Arena`) — **no third-party FFI dependency** — and maps the engine's JSON value
shape into display text.

## Build, test, run

Requires **JDK 21+** (the Java FFM API is preview on 21, final on 22) and a Rust
toolchain to build the engine.

```bash
bash scripts/build.sh    # regenerate composables + build & vendor the engine (cdylib)
bash scripts/verify.sh   # HEADLESS: compile Engine.kt + smoke, run via FFM — proves
                         #   the grid is engine-computed and recomputes on edit
gradle --no-daemon run   # launch the desktop window (FFM run args set in build.gradle.kts)
```

`scripts/verify.sh` compiles `Engine.kt` + `test/EngineSmoke.kt` with `kotlinc`
(no Compose, no Gradle) and runs them with `--enable-preview
--enable-native-access=ALL-UNNAMED`, loading the vendored engine. It asserts the
grid is engine-computed (E1 = 38, A5 = 39, E5 = 169), that editing A1 15 → 115
recomputes the totals (E5 → 269), and that a formula computes with binary-op
error propagation (`=1/0` → `#DIV/0!`, and `=A1+1` over it → `#DIV/0!`).

## Infinite virtualized sheet

`SpreadsheetSession` (`Engine.kt`) also binds the engine's **viewport
primitive** over Java FFM — `window(r0,c0,r1,c1)` (a dense `List<List<String>>`
rectangle), `usedRange()`, `columnLetters()`, `currentRevision()`, and
`changedSince()` — so a windowed Compose grid can render only the visible
rectangle of an unbounded sheet (the Compose sibling of the web/SwiftUI/Qt/
Flutter infinite views). The window JSON is nested, so it's parsed by a tiny
in-file JSON reader rather than `display()`'s per-value regex.

### The touch FormulaBar layout (`FormulaBarTouch`)

The **Touch bar** button (shown in classic-grid mode) swaps the formula bar
between two composables generated from the *same* `FormulaBar.mil` interface:

- **Desktop** (`FormulaBar.desktop.mll` → `FormulaBar`): a `Row` — the
  cell-address label sits to the **left** of the input.
- **Touch** (`FormulaBar.touch.mll` → `FormulaBarTouch`): a `Column` — the
  address label stacks **above** a full-width input, the phone arrangement.

`scripts/build.sh` emits both. Kotlin names a composable after the `.mil`
component (`FormulaBar`) and the emitter also emits the shared
`sealed class FormulaBarEvent`, so to let both variants live in package
`generated` the touch output has its **duplicate** `FormulaBarEvent` stripped
(it reuses the one from `FormulaBar.kt`) and its composable renamed to
`FormulaBarTouch`. `Main.kt` holds a `touch` flag and calls one or the other,
passing the identical `fbDispatch` — so editing behaves the same in both and
only the shape changes. This is the UI30 "one component, many layouts, identical
host contract" invariant made a runtime toggle — the Compose sibling of the Qt
demo's toggle and the web demo's switcher.

### The scrollable infinite GUI (`InfiniteSheet.kt`)

The **Infinite sheet** button in the running app toggles from the classic 5×5
grid to `InfiniteSheet` — a virtualized, effectively-infinite (u32 × u32,
sparse) sheet rendered on the same engine. The body is a `LazyColumn`, which
natively virtualizes: it composes a row item only while it's near the viewport
and recycles it on scroll, so a 1000-row sheet costs the handful of rows you can
see. Each composed row makes **one** engine `get_display_window` over its
`1×totalCols` strip (`InfiniteSheetModel.rowCells`) — display strings, each
already rendered through its Excel-style format code (the seed formats the
cross-foot totals as `#,##0.00` and the far-flung `Z1000` total as a percent),
so the Compose host paints them directly. Per-frame engine work is proportional
to *visible* rows, never the sheet's height.

Frozen chrome without a second scroller: the row-number gutter rides as each
`LazyColumn` row's first child, *outside* the horizontal scroll, so it stays
pinned left and scrolls vertically with the body for free; every row and the
column-letter header share one horizontal `ScrollState`, so dragging any row
pans them all in lockstep (the header is gesture-disabled — it only follows).
Tap a cell → `selectInf(row,col)` (clamps, loads the source into the formula
bar); press Enter → `commitInf(text)` (writes through, recomputes dependents,
regrows the extent). The **"Fill ↓ 10"** button next to the formula bar calls
`InfiniteSheetModel.fillDown(10)` (over the C ABI's `sc_fill`) to replicate the
selected cell into the 10 rows below it — the engine shifts each copy's relative
references (`=A1`→`=A2`, …), pins absolute (`$`) refs, and carries the format.
The **Copy / Cut / Paste** buttons drive the engine's clipboard
(`InfiniteSheetModel.copyCell`/`cutCell`/`pasteCell` over the C ABI's
`sc_copy`/`sc_cut`/`sc_paste`): copy the selected cell, then paste it elsewhere
with its relative references shifted by the destination's offset (absolute `$`
refs pinned, format carried); a cut clears the source on paste, and `pasteCell`
returns `false` (a no-op) for an empty clipboard.
The **Save / Load** buttons serialize the whole workbook
(`InfiniteSheetModel.saveBook` over the C ABI's `sc_serialize`) to a JSON
document held in memory and restore it (`loadBook` / `sc_deserialize`): the
document captures only the source (formula text + typed literals) and per-cell
formats — not the computed values, which the engine recomputes on load, so a
loaded formula stays live.
The **Undo / Redo** buttons walk the engine's snapshot history
(`InfiniteSheetModel.undoEdit`/`redoEdit` over the C ABI's `sc_undo`/`sc_redo`);
they disable at the history ends via `canUndo`/`canRedo`. Every edit is
reversible and a restored formula recomputes live.
The **+ Row / − Row / + Col / − Col** buttons are **structural edits**
(`InfiniteSheetModel.insertRow`/`deleteRow`/`insertCol`/`deleteCol` over the C
ABI's `sc_insert_rows`/`sc_delete_rows`/`sc_insert_cols`/`sc_delete_cols`): insert
or delete the selected cell's row/column, and the engine shifts every formula
reference at or after the band so dependents keep pointing at their precedents
(`=A1+A2` with a row inserted above becomes `=A1+A3`); a reference whose whole band
is deleted becomes `#REF!`.
The **.00 / % / $ / Gen** buttons apply a number **format** to the selected cell
(`InfiniteSheetModel.applyFormat` over the C ABI's `sc_set_format`, with the code
`#,##0.00`, `0.0%`, `$#,##0.00`, or `""` to clear). The format is display-only —
the engine renders the stored value through the code, so the underlying number is
unchanged.
The **find / replace** group (a `find` box + a `replace` box + **Find** /
**Replace** buttons) searches and rewrites cell SOURCES:
`InfiniteSheetModel.findAll` (over the C ABI's `sc_find_all`) returns the A1
addresses whose formula text contains the query (case-insensitive) and the
**Find** button jumps the selection to the first hit (`selectA1` parses column
letters past Z); `InfiniteSheetModel.replaceAll` (over `sc_replace_all`) rewrites
the query → replacement in every cell's source and recomputes, with the footer
echoing the match / replace count. Because the engine re-parses each rewrite
through its centralised coerce (`set_raw`), a rewritten formula stays live
(`H1`→`H2` turns `=H1+5` into a recomputed `=H2+5`) and a rewritten literal stays
typed (`15`→`99` re-totals every dependent).
`InfiniteSheetModel` (in `Engine.kt`) seeds far-flung
sparse cells (`Z1000`, `BA50`, `BB50`) and derives the extent from `usedRange()`
+ a margin.
The bottom **sheet tab bar** drives a multi-sheet **workbook**: the workbook
holds several sheets and bare-`A1` ops address the *active* one, while a formula
reaches **across** with a qualifier (`=Summary!B3`). Each tab is a chip (the
active one tints to the accent); **click** to switch, the active tab carries
inline **✎ rename** (a dialog) and **✕ delete** affordances, and **+ Sheet** adds
one (`InfiniteSheetModel.selectSheet`/`addSheet`/`renameSheet`/`deleteSheet` over
the C ABI's `sc_set_active_sheet`/`sc_add_sheet`/`sc_rename_sheet`/
`sc_delete_sheet`, with `sc_sheet_names`/`sc_active_sheet` reading the tab list).
The seed adds a second sheet, **Summary** (`B3 = A1+A2 = 300`), and `Sheet1!G1 =
=Summary!B3` pulls that value across; editing a Summary input recomputes the
cross-sheet dependent live. Renaming `Summary` rewrites every referencing
qualifier; deleting a referenced sheet turns the dangling reference into `#REF!`,
and the engine keeps at least one sheet (deleting the last is a no-op). The
single-sheet path is byte-identical — an unqualified `A1` still means the active
sheet.

#### Visual design

`InfiniteSheet.kt` mirrors the **reference visual language** defined by the web
demo (`code/programs/typescript/visicalc-html/infinite.html`) so every VisiCalc backend reads as one
considered dark, modern-spreadsheet surface — the same token set the Qt and
Flutter ports use. The palette lives in a small set of `Color` design tokens at
the top of the file (`BG`/`PANEL`/`SURFACE`/`LINE`/`INK`/`MUTED`/`ACCENT`…),
echoing the web demo's CSS custom properties. From those it builds: a
panel-wrapped **toolbar** with an address **pill**, an italic `fx` marker, then a
grown formula field with an accent **focus ring** (driven by the field's
`MutableInteractionSource` focus state); the actions are **segmented button
groups** (drag-fill · clipboard · file · history · find/replace) — a reusable
`toolButton` composable with hover/pressed/disabled states, plus a compact
`searchField` for the find/replace inputs — separated by thin rules. The grid
gets subtle **zebra** row banding, a 2-px **accent selection ring**, and the
selected cell's **row + column headers tint to the accent**; a hairline-separated
**status footer** echoes the live virtual-grid size and revision.

### Verification

Headless proof: `scripts/verify.sh` (kotlinc + FFM) seeds far-flung sparse cells
and asserts the window is engine-computed + dense (A1=15, E1=38, E5=169), a
formula 1000 rows down (`Z1000` = 39) is reachable, the gaps are empty (sparse),
column letters run AA/BA, and editing `A1` dirties the far dependent `Z1000` via
`changedSince`. It also drives `InfiniteSheetModel` directly: `rowCells`
one-read rows, `selectInf` clamping + source load, `commitInf` recompute
(A2 `8`→`108` ⇒ E2 151, A5 139, E5 269), `fillDown` (`I1 = =H1*10` filled
down 10 rows ⇒ I2 = H2*10 = 30, I3 = H3*10 = 40, source I1 = 20 untouched), and
the clipboard (`copyCell` I1 → `pasteCell` at I4 ⇒ I4 = H4*10 = 60; `cutCell` A1
→ paste at C1 moves it, A1 clears, a second paste returns false), and a
save/load round trip (`saveBook` → mutate A1 ⇒ E1 523.00 → `loadBook` restores
A1 15 / E1 38.00, the loaded formula stays live with A1=5 ⇒ E1 28.00, and
malformed input is rejected), and an undo/redo walk on a fresh session (two
edits → undo both → redo both with the formula recomputing live → a fresh edit
forks history). And it drives **find / replace**: `findAll("15")` locates the one
literal (`A1`), a case-insensitive `findAll("sum")` finds the total formulas,
empty / no-match queries return nothing, `selectA1("Z1000")` parses a far address,
and `replaceAll` rewrites both a literal (`15`→`99` ⇒ E1 122.00) and a formula
reference (`H1`→`H2` ⇒ `=H1+5` recomputes to 25) keeping each live. Finally a
**multi-sheet** section proves the workbook over the C ABI: a cross-sheet
`=Summary!B3` computes and stays live as its precedent changes (300 → 350), a
rename rewrites the referencing qualifier (`Summary` → `Totals`), deleting a
referenced sheet yields `#REF!` (and the last sheet can't be deleted), and the
`InfiniteSheetModel` seed exposes `Sheet1`/`Summary` with a live cross-ref.

The Compose UI itself (`InfiniteSheet.kt` + the `Main.kt` toggle) is verified to
compile against the real Compose Desktop APIs via `gradle compileKotlin`; the
engine-backed logic it drives is the headlessly-proven model above. (A live GUI
needs a Compose Desktop window — `gradle run`.)

## Why "Compose for Desktop" rather than Jetpack Compose for Android?

Same `androidx.compose.*` packages, same composable functions, same
`MaterialTheme`.  The runtime API is identical.  We target Desktop here so the
demo runs locally with no emulator.  An Android variant is a straight UI port
(swap `Window` for an `Activity` + `setContent`) — the one engine difference is
that Android's runtime has no Java FFM, so the engine would be reached through a
thin **JNI** bridge over the same C ABI, with the Rust library cross-compiled to
a per-ABI `.so` and bundled under `jniLibs/`. The C ABI is unchanged; only the
binding mechanism differs (FFM on the JVM, JNI on Android).

## File tree

```
code/programs/kotlin/visicalc-compose/
├── README.md                     ← this file
├── BUILD                          ← `gradle run` from the build-tool
├── .gitignore                     ← .gradle/, .gradle-out/, build/, native/, …
├── settings.gradle.kts            ← includes ../../code/packages/kotlin/mosaic-flux-compose
├── build.gradle.kts              ← kotlin("jvm") + compose plugin; JDK 21 + FFM run args
├── native/                        ← vendored libspreadsheet_capi.* (git-ignored)
├── scripts/
│   ├── build.sh                  ← mosaic-compile --backend compose + build/vendor engine
│   └── verify.sh                 ← headless FFM smoke (kotlinc + java --enable-preview)
├── test/
│   └── EngineSmoke.kt            ← asserts the grid is engine-computed + recomputes
└── src/main/kotlin/
    ├── Main.kt                   ← `application { Window { ... } }`; classic grid + infinite toggle
    ├── InfiniteSheet.kt          ← virtualized infinite-sheet composable (LazyColumn over the viewport)
    ├── Engine.kt                 ← Java FFM bindings + SpreadsheetModel + InfiniteSheetModel
    └── generated/                ← FormulaBar.kt + Grid.kt (mosaic-compile output)
```

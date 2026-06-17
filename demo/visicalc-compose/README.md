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
`mosaic-compile --backend compose` from the shared `demo/visicalc/mosaic/*`
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
`InfiniteSheetModel` (in `Engine.kt`) seeds far-flung
sparse cells (`Z1000`, `BA50`, `BB50`) and derives the extent from `usedRange()`
+ a margin.

### Verification

Headless proof: `scripts/verify.sh` (kotlinc + FFM) seeds far-flung sparse cells
and asserts the window is engine-computed + dense (A1=15, E1=38, E5=169), a
formula 1000 rows down (`Z1000` = 39) is reachable, the gaps are empty (sparse),
column letters run AA/BA, and editing `A1` dirties the far dependent `Z1000` via
`changedSince`. It also drives `InfiniteSheetModel` directly: `rowCells`
one-read rows, `selectInf` clamping + source load, `commitInf` recompute
(A2 `8`→`108` ⇒ E2 151, A5 139, E5 269), and `fillDown` (`I1 = =H1*10` filled
down 10 rows ⇒ I2 = H2*10 = 30, I3 = H3*10 = 40, source I1 = 20 untouched).

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
demo/visicalc-compose/
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

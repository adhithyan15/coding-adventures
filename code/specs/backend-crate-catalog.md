# Backend Crate Catalog

## Overview

This spec enumerates every Rust crate that, taken together, powers a
spreadsheet program — any spreadsheet program, from the 1979 VisiCalc
function set through Excel-2024-with-LAMBDA through whatever comes
after. It pins three things that the existing per-domain specs imply
but do not state directly:

1. **The portability bar.** Every backend crate compiles cleanly to
   native, to WASM, and to a C dynamic library on Windows, macOS,
   Linux. No platform-specific code, no global state, no file I/O,
   no hidden clocks, no `unsafe` outside narrowly-scoped FFI shims.
2. **The FFI surface.** Rust is the canonical home of unsafe and
   OS-touching code in this repo. Every backend crate exposes a
   `cbindgen`-generated C ABI so any host language ecosystem — .NET
   for XAML, C++ for Qt, Swift for Metal, Kotlin for Compose, JS for
   Web Components, Python for notebooks — can call it through its
   native FFI bridge.
3. **The catalog.** Roughly 35 crates, split across substrate,
   function domains, cell features, engine, advanced features,
   workbook state, and I/O. With status, dependencies, priority for
   each VisiCalc / Lotus / Excel parity tier, and the implementation
   sequence.

The companion to the Mosaic UI vision: Mosaic compiles **one** UI
source to many backends (paint-vm, Web Components, React, SwiftUI,
Jetpack Compose; future XAML, Qt, Win32, Metal). The crates in this
catalog compile **one** Rust workspace to a C ABI that all those
backends can call. Together: write the spreadsheet once; run
anywhere.

---

## Where It Fits

```
   ┌─────────────────────────────────────────────────────────────┐
   │   Mosaic UI source (.mosaic files — written once)           │
   └────────────────┬────────────────────────────────────────────┘
                    │ compiles to:
   ┌────────────────┴────────────────────────────────────────────┐
   │  paint-vm    Web Components   SwiftUI    Compose            │
   │  (native)    React            (Metal)    (JVM)              │
   │  (Rust)      (JS/TS, browser) (Swift)    (Kotlin)           │
   │                                                             │
   │  Future: XAML, Qt, Win32, Flutter, terminal                 │
   └────────────────┬────────────────────────────────────────────┘
                    │ all host runtimes call into:
   ┌────────────────▼────────────────────────────────────────────┐
   │   Rust backend crates (THIS CATALOG)                        │
   │   Native Rust API + cbindgen-generated C ABI                │
   │   + optional WASM, JNI, swift-bridge, cxx, P/Invoke shims   │
   └─────────────────────────────────────────────────────────────┘
```

**Defined by:** this spec.

**Implemented across:** ~35 Rust crates under `code/packages/rust/`
plus a handful of `*-ffi` shim crates that produce C headers and
language-specific bindings.

**Used by:** every spreadsheet UI, every notebook environment, every
batch tool, every language-bridge wrapper in the repo.

---

## §1 Portability Bar

Every crate in this catalog complies with the following. Compliance is
checked in CI on every PR that touches a backend crate.

### Rules

1. **No `#[cfg(target_os = "...")]`** in `src/`. Platform divergence,
   when required (e.g. system clock for `NOW()`), is injected through
   an interface, not branched at compile time. The `*-os-shim` crates
   are the only place platform-specific code lives.
2. **WASM-compatible.** `cargo build --target wasm32-unknown-unknown`
   succeeds with no features beyond the default. No
   `std::time::SystemTime`, no thread-local statics, no
   `std::process`, no FFI into platform libraries from inside the
   crate. The WASM build job runs in CI.
3. **No file I/O.** Every format gets its own `*-io` crate that
   depends on the core. The core does not read or write files. Stdin,
   stdout, env vars all forbidden in the core.
4. **No global mutable state.** No `static mut`, no `lazy_static!`
   holding mutable handles, no thread-local mutable state. All state
   (workbook, RNG, clock) passes through explicit parameters. This
   makes embedding multiple spreadsheets in one process safe.
5. **No hidden clocks.** Functions that read time (`NOW()`,
   `TODAY()`) take a `&dyn Clock` parameter. Tests inject a fixed
   clock. Production wires a real clock at the binary boundary.
6. **No `unsafe`** outside `*-ffi` and `*-os-shim` crates. The
   substrate crates, function-domain crates, engine, and feature
   crates use only safe Rust. Catalog includes a compliance check.
7. **No `panic!`** in hot paths. Bounds errors return `Result`;
   logic-bug invariants use `debug_assert!`. Catalog includes a
   `forbid(panic_in_result_fn)` lint via Clippy.
8. **FFI-shaped public API.** Public functions on every backend
   crate use only types that survive the C ABI when wrapped:
   integers, floats, raw pointers, `repr(C)` structs, length-prefixed
   byte slices for strings, integer error codes. Generics are
   permitted in *internal* code but must monomorphize at the public
   boundary. See §2.

### Enforcement

A new package, `code/programs/rust/portability-checker/`, runs as a
CI job on every PR. It:

- `cargo build --target wasm32-unknown-unknown` for every catalog
  crate; failure is a blocker
- Greps every `src/` for `cfg(target_os`, `cfg(unix)`, `cfg(windows)`,
  `std::time::SystemTime`, `std::fs`, `std::process`,
  `lazy_static!`, `static mut`; matches in catalog crates fail
- Counts `unsafe` blocks per crate; allowed only in crates ending in
  `-ffi` or `-os-shim`
- Runs the cbindgen header generator and asserts the output is
  well-formed C and that all public symbols are exported

---

## §2 FFI Architecture

Rust is the canonical home of OS-touching and unsafe code. Every
backend crate exposes its functionality to non-Rust consumers
through a stable C ABI generated by `cbindgen`. Each language
ecosystem provides its own bridge to that C ABI.

### Two crates per domain

For each domain (statistics, financial, math, lookup, text, datetime,
spreadsheet, …):

```
<domain>-core        — Rust-idiomatic library; native Rust API
<domain>-ffi         — Thin wrapper exposing a stable C ABI:
                       extern "C" fn xx_create() -> *mut XxHandle;
                       extern "C" fn xx_call(h: *mut XxHandle, …) -> i32;
                       extern "C" fn xx_destroy(h: *mut XxHandle);
                       + a generated <domain>.h header
```

The `*-ffi` crate is built as `cdylib` and `staticlib` alongside
`rlib`. `cbindgen` runs during `cargo build` (via `build.rs`) and
emits `<domain>.h` next to the compiled library. Downstream language
bindings consume the header.

### Type translation at the boundary

| Rust type            | FFI representation                                  |
|----------------------|------------------------------------------------------|
| `i64`, `f64`, `u32`, … | matching C primitives                              |
| `String`              | `*const u8` (UTF-8) + `usize` length (caller frees) |
| `&[T]` (numeric)      | `*const T` + `usize` length                          |
| `Vec<T>` (return)     | output buffer: `*mut T` + `usize` capacity + `*mut usize` returned-length |
| `Result<T, E>`        | i32 error code; `T` written through out-pointer    |
| `Option<T>` (numeric) | sentinel value (NA bit pattern) carries Optionality |
| `&str` (input)        | `*const u8` + `usize` length                         |
| Opaque resource       | `*mut OpaqueHandle` (struct forward-declared in C) |
| Closure / `dyn Fn`    | function pointer + `void *user_data`                 |

Rust-only types (`Vec`, `Box`, `HashMap`, `Rc`, etc.) **never appear
in `*-ffi` public signatures**. They are constructed inside the FFI
function body, used for the work, and dropped before return.

### Error model across FFI

A single error-code convention applies to every `*-ffi` crate:

```c
typedef enum {
    XX_OK              = 0,
    XX_EMPTY_INPUT     = 1,
    XX_DOMAIN_ERROR    = 2,
    XX_SHAPE_MISMATCH  = 3,
    XX_SINGULAR        = 4,
    XX_NO_CONVERGENCE  = 5,
    XX_BAD_PARAMETER   = 6,
    XX_OVERFLOW        = 7,
    XX_OUT_OF_MEMORY   = 8,
    XX_INVALID_HANDLE  = 9,
    XX_NULL_POINTER    = 10,
    XX_BUFFER_TOO_SMALL = 11,
} xx_status_t;
```

Detailed messages are opt-in:

```c
extern void xx_last_error_message(char *buf, size_t buf_len, size_t *out_len);
```

A thread-local last-error buffer in the FFI crate holds the most
recent message; consumers fetch it after a non-zero status. (This is
the *only* thread-local mutable state allowed in the catalog — and
it lives in `*-ffi`, not core.)

### Memory ownership

Every allocation has a paired free. The convention is to expose
`xx_alloc(...)`, `xx_free(ptr)`, and document who owns what at every
function. Callers from C# or C++ wrap these in RAII.

For zero-copy paths (passing a `*const f64` slice into a stats
function), the caller retains ownership and Rust merely borrows for
the duration of the call.

### Per-language bridges

The catalog commits to producing the C header. Language bridges are
optional and additive:

| Bridge crate                       | What it provides                              |
|------------------------------------|------------------------------------------------|
| `<domain>-ffi`                     | C header + `cdylib`/`staticlib` (canonical)   |
| `<domain>-wasm`                    | `wasm-bindgen` exports for browser/Node       |
| `<domain>-jni`                     | JNI bindings for Compose / Java               |
| `<domain>-swift`                   | `swift-bridge` (or hand-rolled) for SwiftUI / Metal |
| `<domain>-cxx`                     | `cxx` crate bindings for Qt                   |
| `<domain>-pinvoke`                 | C# P/Invoke notes (no Rust crate — host wraps C header) |
| `<domain>-py`                      | PyO3 bindings for Python notebooks            |
| `<domain>-rb`                      | Magnus or rb-sys for Ruby                     |
| `<domain>-node`                    | N-API for Node.js                             |

The C header is the **floor**. Every other bridge is a convenience,
shipped only when a consumer needs it. We do not pre-emptively build
all of these.

### cbindgen vs. UniFFI

UniFFI is convenient for Kotlin/Swift/Python but does not target
.NET/C++/C and has Mozilla's release cadence. We commit to
**cbindgen as the floor** because it produces plain C headers that
every ecosystem can consume. UniFFI may be added as an optional
convenience on top.

---

## §3 The `SpreadsheetFunction` Trait

Every function-domain crate (statistics, financial, math, lookup,
text, datetime, engineering, database, array) implements the same
trait for every function it exposes. The engine dispatcher resolves
formula names to trait objects.

```rust
pub trait SpreadsheetFunction: Send + Sync {
    /// Canonical name (e.g. "mean", "npv", "vlookup").
    fn canonical_name(&self) -> &'static str;

    /// Frontend aliases (e.g. ["AVERAGE", "AVG"] for mean).
    fn aliases(&self) -> &'static [&'static str];

    /// Argument shape and coercion specification.
    fn arg_spec(&self) -> &'static ArgSpec;

    /// Whether the function is volatile (NOW, RAND, …).
    fn is_volatile(&self) -> bool { false }

    /// Whether the function can return a dynamic array.
    fn is_array_returning(&self) -> bool { false }

    /// Invoke the function with already-coerced arguments.
    fn call(&self, args: &[CellValue], ctx: &EvalContext)
        -> Result<CellValue, SpreadsheetError>;
}
```

The argument spec carries per-argument kind (Scalar, Vector, Range,
Predicate, Array), coercion mode (ToNumber, ToText, ToLogical), NA
action, and variadic min/max. The dispatcher coerces actual arguments
against the spec before calling `call`, producing `#VALUE!` on
mismatch — the function body sees only well-typed input.

The `EvalContext` carries:

```rust
pub struct EvalContext<'a> {
    pub workbook: &'a Workbook,
    pub current_cell: CellAddress,
    pub epoch: u64,
    pub clock: &'a dyn Clock,
    pub rng: &'a mut RngState,
    pub iteration: Option<IterationState>,
}
```

The context is read-only on the workbook (functions cannot mutate
cells; only the recalc engine does) and holds the *only* clock and
RNG used during evaluation. Determinism follows from this: same
context, same args, same result.

### Registration

Each function-domain crate exposes:

```rust
pub fn register_functions(registry: &mut FunctionRegistry);
```

The registry is a `HashMap<UniCase<&'static str>, &'static dyn SpreadsheetFunction>`.
At spreadsheet startup, the binary calls each domain's registrar; the
registry is then read-only during recalc. This pattern lets a
slimmed-down spreadsheet (e.g. one without `lookup-core`) ship by
simply not calling `lookup_core::register_functions`.

---

## §4 The Crate Catalog

Status legend: ✅ shipped on main, 🚧 in progress, ⬜ not started.

Priority legend: **V** = required for VisiCalc 1979 parity, **L** =
required for Lotus 1-2-3 parity, **E** = required for Excel parity
(today's function set), **+** = nice-to-have / future.

### Layer 0 — Substrate

| Crate | Status | Priority | Depends on | Scope |
|-------|--------|----------|------------|-------|
| `numeric-tower` | ✅ | V | std + num-bigint + num-rational | Int/BigInt/Rational/Float/Complex/Decimal; coercion |
| `r-vector` | ✅ | V | numeric-tower | Atomic vectors; NA; indexing; recycling; names |
| `data-frame` | ⬜ | L | r-vector | Tabular type: list of equal-length named vectors |
| `clock` | ⬜ | E | none | `Clock` trait + fixed + system impls |
| `rng-state` | (in statistics-core) | V | none | MT19937 — currently lives in statistics-core; may extract if other crates need it |

### Layer 1 — Function Domains

| Crate | Status | Priority | Depends on | Scope |
|-------|--------|----------|------------|-------|
| `statistics-core` Phase 1 | ✅ | V | numeric-tower, r-vector | descriptive + rank + counting |
| `statistics-core` Phase 2 | ⬜ | E | Phase 1 | distributions (18 × d/p/q/r) + special functions |
| `statistics-core` Phase 3 | ⬜ | E | Phase 2 | hypothesis tests (HTest envelope) |
| `statistics-core` Phase 4 | ⬜ | L | Phase 1 + linalg | regression: lm, glm, nls, predict |
| `statistics-core` Phase 5 | ⬜ | + | Phase 4 | ANOVA, multivariate (PCA, factanal, lda, qda) |
| `statistics-core` Phase 6 | ⬜ | + | Phase 2 | time series (acf, pacf, arima, decompose, stl) |
| `statistics-core` Phase 7 | ⬜ | + | Phase 2 | smoothing (density, lowess, loess, splines) |
| `statistics-core` Phase 8 | ⬜ | + | Phase 2 | resampling (sample, bootstrap, jackknife, permutation) |
| `math-core` | ⬜ | V | numeric-tower, cas-complex | Trig, exp/log, ROUND/CEILING/FLOOR/MROUND, MOD, GCD/LCM, ABS/SIGN/INT, POWER, EXP, LN/LOG/LOG10, SQRT, SUMPRODUCT, MMULT/MINVERSE/MDETERM, FACT/FACTDOUBLE, COMBIN/PERMUT |
| `financial-core` | ⬜ | V (V's @NPV) → L → E | numeric-tower, datetime-core | TVM (NPV/IRR/MIRR/XIRR/PMT/IPMT/PPMT/FV/PV/RATE/NPER), depreciation (SLN/DDB/SYD/VDB/DB/AMORLINC/AMORDEGRC), bonds (PRICE/YIELD/DURATION/MDURATION/ACCRINT/ACCRINTM/COUPDAYBS/COUPDAYS/COUPDAYSNC/COUPNCD/COUPNUM/COUPPCD), treasury (TBILLEQ/TBILLPRICE/TBILLYIELD), dollar conversions (DOLLARDE/DOLLARFR), interest (CUMIPMT/CUMPRINC/EFFECT/NOMINAL/INTRATE/RECEIVED/DISC/PRICEDISC/PRICEMAT/YIELDDISC/YIELDMAT) |
| `lookup-core` | ⬜ | V (V's @LOOKUP) → E | r-vector | VLOOKUP/HLOOKUP, INDEX/MATCH, XLOOKUP/XMATCH, OFFSET, INDIRECT, CHOOSE, ADDRESS, AREAS, ROW/COLUMN/ROWS/COLUMNS, FORMULATEXT, GETPIVOTDATA |
| `text-core` | ⬜ | L → E | numeric-tower | LEFT/RIGHT/MID/LEN, UPPER/LOWER/PROPER, TRIM/CLEAN, SUBSTITUTE/REPLACE, FIND/SEARCH, TEXT/VALUE/NUMBERVALUE, REPT, CODE/CHAR, UNICODE/UNICHAR, CONCAT/TEXTJOIN/CONCATENATE, REGEX helpers (REGEXEXTRACT/REGEXMATCH/REGEXREPLACE), TEXTSPLIT, TEXTBEFORE/TEXTAFTER, EXACT, ASC, DOLLAR, FIXED, T, BAHTTEXT |
| `datetime-core` | ⬜ | L → E | numeric-tower, clock | DATE/TIME/DATEVALUE/TIMEVALUE, NOW/TODAY (clock-injected), YEAR/MONTH/DAY/HOUR/MINUTE/SECOND, WEEKDAY/WEEKNUM/ISOWEEKNUM, EOMONTH/EDATE/DATEDIF, NETWORKDAYS/NETWORKDAYS.INTL/WORKDAY/WORKDAY.INTL, YEARFRAC (5 day-count conventions), DAYS/DAYS360, both 1900 and 1904 Excel epochs, POSIXct interop, holiday calendars (pluggable) |
| `engineering-core` | ⬜ | E | numeric-tower, cas-complex, math-core | Base conversion (BIN2DEC/DEC2BIN/HEX2DEC/OCT2DEC and inverses), bitwise (BITAND/BITOR/BITXOR/BITLSHIFT/BITRSHIFT), complex (COMPLEX/IMABS/IMARGUMENT/IMCONJUGATE/IMCOS/IMCOSH/IMDIV/IMEXP/IMLN/IMLOG10/IMLOG2/IMPOWER/IMPRODUCT/IMREAL/IMSIN/IMSINH/IMSQRT/IMSUB/IMSUM/IMTAN), unit conversion (CONVERT — ~180 unit pairs), DELTA/GESTEP, ERF/ERFC/ERF.PRECISE/ERFC.PRECISE, BESSEL.J/Y/I/K |
| `database-core` | ⬜ | L → E | r-vector, data-frame | DSUM, DAVERAGE, DCOUNT, DCOUNTA, DGET, DMAX, DMIN, DPRODUCT, DSTDEV, DSTDEVP, DVAR, DVARP — predicate-based aggregation over a tabular range with header row |
| `array-core` | ⬜ | E (dynamic-array Excel) | r-vector | SEQUENCE, RANDARRAY, TOROW/TOCOL, WRAPROWS/WRAPCOLS, TAKE/DROP/EXPAND, HSTACK/VSTACK, CHOOSEROWS/CHOOSECOLS, FILTER/SORT/SORTBY/UNIQUE; the shape-aligning helpers that spreadsheet-core's broadcast layer uses |

### Layer 2 — Cell Features

| Crate | Status | Priority | Depends on | Scope |
|-------|--------|----------|------------|-------|
| `cell-format` | ⬜ | V (V's basic number display) → E | numeric-tower, datetime-core | Excel format-string parser/applier: positive/negative/zero/text sub-formats, `#`/`0`/`?` digits, thousands separator, scientific, percent, date/time codes, color codes, conditions (`[>100]`), locale handling |
| `conditional-format` | ⬜ | E | spreadsheet-core, cell-format | Rule evaluation (formula, cell-value, top/bottom N, above/below avg, duplicates), color scales, data bars, icon sets |
| `data-validation` | ⬜ | E | spreadsheet-core | Input gating: list, integer/decimal range, length, date, time, custom formula |
| `comment-store` | ⬜ | E | spreadsheet-core | Per-cell comments + threaded discussions |
| `hyperlink-store` | ⬜ | E | spreadsheet-core | Per-cell hyperlinks |

### Layer 3 — Engine

| Crate | Status | Priority | Depends on | Scope |
|-------|--------|----------|------------|-------|
| `spreadsheet-core` Phase 1 | ⬜ | V | every function-domain crate + cell-format | Cell model, formula AST, dependency DAG (Tarjan SCC), recalc, dispatch via `SpreadsheetFunction`, empty-cell sentinel, error propagation. Inlines logical (IF/AND/OR/NOT/IFERROR/IFNA/IFS/SWITCH/XOR) and info (ISBLANK/ISERROR/ISNA/...) families |
| `spreadsheet-core` Phase 2 | ⬜ | E | Phase 1 | Dynamic-array spilling (Excel 365), implicit intersection compatibility, multi-sheet workbooks |
| `formula-lambda` | ⬜ | E (modern Excel) → + | spreadsheet-core | LAMBDA, LET, BYROW/BYCOL, MAP/REDUCE/SCAN, MAKEARRAY, ISOMITTED |
| `range-expression` | (in spreadsheet-core) | V | r-vector | A1, R1C1, structured references; 3-D references — currently in spreadsheet-core; may extract |

### Layer 4 — Advanced Features

| Crate | Status | Priority | Depends on | Scope |
|-------|--------|----------|------------|-------|
| `table-core` | ⬜ | L → E | spreadsheet-core, data-frame | Excel ListObject — typed columns, header row, totals row, banded rows, structured-reference targets |
| `pivot-core` | ⬜ | E | data-frame, statistics-core | Pivot tables: row/column/value/filter areas, aggregation functions, calculated fields, slicers, calculated items |
| `chart-core` | ⬜ | E | data-frame, r-vector | Renderer-agnostic chart model: data binding, scales (linear/log/category/time), axes, geometries (line, bar, scatter, area, pie, histogram), legends, annotations — Mosaic renders |
| `sparkline-core` | ⬜ | E | chart-core | Mini-charts inside cells: line, column, win-loss |
| `solver-core` | ⬜ | E (SOLVER add-in) | r-vector, math-core | LP (simplex), NLP (GRG / SQP), evolutionary; constraint specification; sensitivity analysis |
| `goal-seek` | ⬜ | L → E | spreadsheet-core | Single-variable root-finding for a target cell value |
| `scenario-core` | ⬜ | E | spreadsheet-core | Named scenarios; scenario summary report |

### Layer 5 — Workbook State (Non-Recalc)

| Crate | Status | Priority | Depends on | Scope |
|-------|--------|----------|------------|-------|
| `undo-redo` | ⬜ | E | spreadsheet-core | Event-sourced edit log; undo/redo of all workbook mutations |
| `clipboard-payload` | ⬜ | E | spreadsheet-core | Cut/copy/paste serialization formats (Excel XML, HTML table, plain text, native binary) |
| `selection-model` | ⬜ | V | r-vector | Active cell, ranges, viewport — the state Mosaic reads to render highlights |
| `print-layout` | ⬜ | E | spreadsheet-core, cell-format | Page breaks, headers/footers, print titles, scale, fit-to-page |
| `outline-grouping` | ⬜ | E | spreadsheet-core | Row/column collapsible groups |
| `workbook-protection` | ⬜ | E | spreadsheet-core | Locked cells, sheet protection (display-only metadata) |

### Layer 6 — I/O (each optional, all separate crates)

| Crate | Status | Priority | Depends on | Scope |
|-------|--------|----------|------------|-------|
| `csv-io` | ⬜ | V (V's save format proxy) → E | r-vector, datetime-core | CSV / TSV read + write; quoting; encoding |
| `xlsx-io` | ⬜ | E | spreadsheet-core + every cell-feature + chart-core + table-core | XLSX / XLSM read + write; the big one — full Open XML support |
| `ods-io` | ⬜ | + | spreadsheet-core | OpenDocument Spreadsheet format |
| `visicalc-io` | ⬜ | (faithful track) | spreadsheet-core | 1979 VisiCalc binary file format |
| `lotus-wk1-io` | ⬜ | L | spreadsheet-core | Lotus 1-2-3 release 1/1A WK1 format |
| `lotus-wk3-io` | ⬜ | L | spreadsheet-core | Lotus 1-2-3 release 3 WK3 |
| `lotus-wk4-io` | ⬜ | L | spreadsheet-core | Lotus 1-2-3 release 4/5 WK4 |
| `slk-io` | ⬜ | + | spreadsheet-core | SYLK (Multiplan) |
| `html-export` | ⬜ | E | spreadsheet-core, cell-format | Render a workbook to HTML |
| `pdf-export` | ⬜ | E | spreadsheet-core, cell-format, print-layout | Render a workbook to PDF |
| `json-flat-io` | ⬜ | + | spreadsheet-core | Round-trippable JSON for diffing |
| `parquet-io` | ⬜ | + | data-frame | Apache Parquet for big-data interchange |
| `arrow-io` | ⬜ | + | data-frame | Apache Arrow IPC |

### Layer 7 — FFI Shims

| Crate | Status | Priority | Depends on | Scope |
|-------|--------|----------|------------|-------|
| `<domain>-ffi` (one per backend crate) | ⬜ | as crate it wraps | the core crate it wraps + cbindgen | C ABI exposure; produces `.h` |
| `spreadsheet-ffi` (umbrella) | ⬜ | V | spreadsheet-core + all function domains + all I/O | Single C ABI for "I want a full spreadsheet engine"; bundles registrations |
| `<domain>-wasm` | ⬜ | E (web frontend) | the core + wasm-bindgen | Browser/Node bindings |
| `<domain>-jni` | ⬜ | + | core + jni-rs | Compose / Java |
| `<domain>-swift` | ⬜ | + | core + swift-bridge | SwiftUI / Metal |
| `<domain>-cxx` | ⬜ | + | core + cxx | Qt / generic C++ |
| `<domain>-py` | ⬜ | + | core + pyo3 | Python notebooks |
| `<domain>-rb` | ⬜ | + | core + magnus | Ruby tooling |
| `<domain>-node` | ⬜ | + | core + napi-rs | Node.js |

### Layer 8 — Frontends

| Program | Status | Priority | Scope |
|---------|--------|----------|-------|
| `visicalc-modern` (Rust binary) | ⬜ | V | Mosaic UI + spreadsheet-core; the canary application |
| `visicalc-faithful` (Python) | ⬜ | (faithful track) | 1979 binary on the existing mos6502-simulator |
| `r-runtime` | ⬜ | + | Future R language frontend on the same Layer 1 cores |
| `s-runtime` | ⬜ | + | Future S frontend |
| Future: notebook kernel, CLI batch tool, … | ⬜ | + | Headless and embedded consumers |

### Crate count

| Layer | Crates |
|-------|--------|
| 0 — Substrate | 5 (3 done) |
| 1 — Function domains | 10 + statistics-core's 7 remaining phases |
| 2 — Cell features | 5 |
| 3 — Engine | 4 |
| 4 — Advanced features | 7 |
| 5 — Workbook state | 6 |
| 6 — I/O | 14 |
| 7 — FFI shims | ~9 categories × per-domain |
| 8 — Frontends | 2 immediate + more |

Total: roughly **55-60 crates** when complete, **3 shipped**, **15-20**
on the critical path to full VisiCalc-Modern parity with Excel,
**35+** in the comprehensive tail.

---

## §5 Implementation Sequence

The order optimizes for *unblock-the-next-thing*. Earlier crates open
parallel work tracks.

### Wave 1 — Unblock the engine

```
Impl PR A: clock + selection-model    (tiny crates; Wave-2 work depends on them)
Impl PR B: math-core                  (smallest function domain; proves the
                                        SpreadsheetFunction trait at scale)
Impl PR C: cell-format                (used by every I/O; needed before any value is displayed)
Impl PR D: datetime-core              (depended on by xlsx-io, financial-core, csv-io)
Impl PR E: spreadsheet-core Phase 1   (the engine — depends on B, C, D)
```

After Wave 1, every subsequent function-domain PR is independent of
the others. Wave 2 fans out.

### Wave 2 — Function domains (parallel)

```
Impl PR F: lookup-core
Impl PR G: text-core
Impl PR H: financial-core
Impl PR I: array-core
Impl PR J: database-core
Impl PR K: engineering-core
Impl PR L: statistics-core Phase 2 (distributions + special functions)
```

All seven can land in any order. None blocks any other.

### Wave 3 — Frontends and I/O

```
Impl PR M: visicalc-modern shell (Mosaic UI; headless tests first)
Impl PR N: csv-io                (proves the I/O pattern; smallest format)
Impl PR O: spreadsheet-ffi       (the C ABI for everything that's shipped; smoke-tests cbindgen)
Impl PR P: xlsx-io               (the big one; reuses every prior crate)
```

### Wave 4 — Advanced features and Phase 2+ of engine

```
table-core, pivot-core, chart-core,
spreadsheet-core Phase 2 (dynamic arrays, multi-sheet),
statistics-core Phase 3-8, formula-lambda, conditional-format, …
```

### Faithful track (parallel, Python-side)

Independent of the Rust waves. Lives at:

```
Impl PR α: apple-ii-machine     (Python, on existing mos6502-simulator)
Impl PR β: apple-ii-disk + DOS 3.3
Impl PR γ: visicalc-faithful boot
```

---

## §6 Conventions

Every crate in this catalog follows the same conventions. The
existing `numeric-tower`, `r-vector`, `statistics-core` already
comply; new crates must.

### Error types

Each domain crate has its own error enum (`MathError`,
`FinancialError`, ...). The variants follow `StatsError`'s shape:
named for the failure category, carrying just enough structured data
to translate to spreadsheet errors at the dispatch boundary.

The mapping to spreadsheet errors is centralized in
`spreadsheet-core`'s dispatcher: `MathError::DomainError → #NUM!`,
`LookupError::NoMatch → #N/A`, etc. Each function-domain crate does
not need to know about `#REF!`-style names.

### NA propagation

The single rule from `na-semantics.md` applies across every domain:
NA in input → NA in output, unless a reduction takes `na_rm = true`.
Reductions in every domain accept `na_rm: bool` as a final positional
argument in their Rust API.

### Naming

- Function-domain crate: `<domain>-core` (e.g. `statistics-core`)
- FFI shim: `<domain>-ffi`
- I/O: `<format>-io` (e.g. `xlsx-io`, not `io-xlsx`)
- The "umbrella" FFI: `spreadsheet-ffi`
- Frontend program (not crate): lives in `code/programs/rust/`, not `code/packages/rust/`

### Function naming inside crates

Match R when R has a name; match Excel's lowercased name when R
doesn't (or differs in shape). Per-frontend aliases live in
`spreadsheet-core`'s dispatch manifest, not in the core crates.

Examples:
- `statistics_core::descriptive::mean` (R name)
- `financial_core::tvm::npv` (Excel name; R doesn't have it)
- `lookup_core::vlookup` (Excel name)
- `text_core::concat` (Excel name; R's `paste` is different shape)

### Public API hygiene

- No `pub` types containing `Rc<RefCell<…>>` or other interior
  mutability — pass `&mut T` explicitly
- No `pub` generic functions unless they monomorphize at the
  call site within the same crate (FFI requires monomorphic public)
- `#[non_exhaustive]` on all `pub` enums representing error
  variants — additions are non-breaking
- Every `pub fn` has a doc comment with an example that runs in
  doctest

### Tests and parity

- Unit tests in `tests/` per function family
- Parity tests against R / Excel / both, in
  `code/programs/rust/<domain>-parity/`
- Property-based tests via `proptest` for arithmetic and shape laws
- ULP-tolerance assertions per `statistics-core.md` §5 for numerical
  functions

### CHANGELOG and README

Every crate ships with:
- `README.md` — what it does, how it fits, usage example
- `CHANGELOG.md` — appended on every release
- `BUILD` / `BUILD_windows` per the repo's build-tool conventions
  (consult lessons.md for leaf-to-root install order and Windows
  quirks)

---

## §7 Open Questions Deferred to Implementation

- **Should `cas-complex` be a transitive dependency of
  `engineering-core` (clean) or should `engineering-core` re-implement
  its complex arithmetic (independent)?** Default: depend on
  `cas-complex` for the IM* family; extract a `numeric-complex` crate
  if and only if a non-symbolic consumer outside `cas-*` and
  `engineering-core` needs it.
- **`solver-core` algorithm choice.** Excel's SOLVER bundles three
  engines (Simplex LP, GRG Nonlinear, Evolutionary). We will ship
  Simplex LP and GRG Nonlinear; evolutionary deferred.
- **`pivot-core` aggregation extensibility.** Built-in functions are
  fixed; user-defined aggregation functions (Excel's calculated
  fields) come in v2.
- **`xlsx-io` round-trip fidelity.** Goal: any XLSX written by recent
  Excel reads, recalcs to identical values, and re-writes without
  corrupting Excel-specific metadata we don't understand
  (charts, themes, named formulas, ribbon customizations). v1 may
  preserve unknown sections as opaque blobs; v2 attempts full
  semantic round-trip.

---

## §8 Out of Scope

- **Macros / VBA.** The spreadsheet has a formula language but not
  an imperative macro language. Modern alternative: LAMBDA in
  `formula-lambda`.
- **Multi-user real-time collaboration.** CRDT-based co-editing is
  its own architecture and lives elsewhere.
- **Ribbon UI / chrome.** That is Mosaic's concern; the catalog
  provides the *state* (selection, undo log, command registry) but
  not the visuals.
- **Mobile / touch.** Same — Mosaic compiles to the target;
  touch-specific interaction patterns are a UI concern.
- **Power Query / external data connections.** Out of scope; if added,
  becomes `connection-core` + per-source connectors.
- **Reporting Services / Power BI integration.** Out of scope.

---

## References

- `code/specs/numeric-tower.md`, `na-semantics.md`, `r-vector.md`,
  `vectorization-rules.md` — the substrate specs
- `code/specs/statistics-core.md` — function catalog the Layer 1
  domain crates align with
- `code/specs/spreadsheet-core.md` — engine spec
- `code/specs/visicalc-modern.md`, `visicalc-faithful.md` — the two
  frontend tracks
- `code/specs/excel-formula-grammar.md` — the formula language
- `code/specs/UI00-mosaic.md` — the UI compiler whose backends this
  catalog interoperates with
- `cbindgen` (https://github.com/mozilla/cbindgen) — C header
  generator
- `wasm-bindgen`, `jni-rs`, `swift-bridge`, `cxx`, `pyo3`,
  `napi-rs`, `magnus` — language bridges referenced
- The repo's existing `cas-*` Rust crates as exemplars of the
  small-Cargo-toml, leaf-to-root pattern this catalog continues

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
   OS-touching code in this repo. Every host language ecosystem
   gets a zero-dependency in-repo bridge crate (per `DS01-ffi-bridges.md`)
   that wraps that runtime's C API in safe Rust. Native-extension
   crates pair a domain core (e.g. `statistics-core`) with a bridge
   (e.g. `python-bridge`) to ship a callable module to the host.
   We use *no* external FFI frameworks — no PyO3, Magnus, napi-rs,
   cbindgen, UniFFI, wasm-bindgen, jni-rs, swift-bridge, or cxx.
   The bridges are 300-1200 lines of explicit, greppable code each.
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
   │   Pure Rust cores  +  zero-dep in-repo bridge crates         │
   │   (python-bridge, ruby-bridge, node-bridge, lua-bridge,      │
   │    perl-bridge, objc-bridge, erl-nif-bridge — existing       │
   │    plus wasm-bridge, jvm-bridge, dotnet-bridge, cpp-bridge,  │
   │    c-bridge — to be built)                                   │
   │   +  <domain>-<language>-native extension crates per         │
   │      DS01-ffi-bridges.md pattern                             │
   └─────────────────────────────────────────────────────────────┘
```

**Defined by:** this spec.

**Implemented across:** ~35 backend Rust crates under
`code/packages/rust/` plus the repo's in-repo bridge crates
(`python-bridge`, `ruby-bridge`, `node-bridge`, `lua-bridge`,
`perl-bridge`, `objc-bridge`, `erl-nif-bridge`, plus `wasm-bridge`,
`jvm-bridge`, `dotnet-bridge`, `cpp-bridge`, `c-bridge` to be built)
plus per-(domain, language) native-extension crates that glue
domains to bridges. See `DS01-ffi-bridges.md` for the bridge
architecture.

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
6. **No `unsafe`** outside bridge crates and `*-os-shim` crates. The
   substrate crates, function-domain crates, engine, and feature
   crates use only safe Rust. Native-extension crates may carry small
   amounts of `unsafe` where they hand a Rust pointer to a foreign
   runtime, but the unsafety is concentrated at the entry-point
   surface, not inside the algorithm. Catalog includes a compliance
   check.
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
- Counts `unsafe` blocks per crate; allowed only in bridge crates,
  `*-os-shim` crates, and the entry-point modules of
  `*-<language>-native` extension crates
- Builds every native-extension crate as `cdylib` and asserts the
  output loads in the target runtime (smoke test per language: import
  the module, instantiate the type, call one function)

---

## §2 FFI Architecture

Rust is the canonical home of OS-touching and unsafe code. Every
backend crate exposes its functionality to non-Rust consumers
through **the repo's own zero-dependency bridge crates**, per
`DS01-ffi-bridges.md`. We do not use PyO3, Magnus, napi-rs, cbindgen,
UniFFI, wasm-bindgen, jni-rs, swift-bridge, cxx, or any other
external FFI framework as a load-bearing dependency. Each bridge is a
hand-written ~300-1200 line crate with zero third-party dependencies
that wraps a host runtime's C API in safe Rust functions.

### Three-crate pattern per (domain, host language)

For each pairing of a Layer 1 backend crate with a host language
ecosystem:

```
<domain>-core              — pure Rust library; no FFI knowledge
                             (e.g. statistics-core, spreadsheet-core)

<language>-bridge          — zero-dep wrapper around the host runtime's C API
                             (e.g. python-bridge, ruby-bridge)
                             Shared across every <domain>-<language>-native
                             extension; written once per host language.

<domain>-<language>-native — the bridge between the two:
                             - imports <domain>-core for the algorithms
                             - imports <language>-bridge for the FFI calls
                             - exports the language-runtime entry points
                               (#[no_mangle] pub extern "C" fn PyInit_*, etc.)
                             - marshals between Rust types and runtime types
                             (e.g. statistics-core-python-native)
```

This matches DS01's example with `directed-graph-python-native`. The
native-extension crate is the *only* place the bridge and the core
meet; the core never imports any bridge crate, never knows about
foreign object types, and tests as pure-Rust.

### Why not external FFI tools

Per DS01: external frameworks (PyO3, Magnus, napi-rs, etc.) add
15,000–50,000 LOC of macro-generated trait dispatch, hide the actual
C API behind generated code, require host-language development
headers at build time, pull in heavy build-time dependency trees
(`proc-macro2`, `syn`, `bindgen`, `clang-sys`), and fail in
cross-toolchain scenarios (Ruby-on-MinGW with Rust-on-MSVC). The
repo's bridges have **zero Rust dependencies** beyond `core` and
`std`, declare the host C API's `extern "C"` signatures themselves,
compile on any platform with just a Rust toolchain, and produce
shallow stack traces that show your code calling `Py_INCREF`, not 14
layers of macro-generated dispatch.

The bridges also commit to the host runtimes' **stable C ABIs**:

- Python's Limited API (PEP 384, stable since Python 3.2 / 2011)
- Ruby's C extension API (unchanged since Ruby 1.8 / 2003)
- Node.js's N-API (designed for ABI stability across Node versions)
- Erlang's NIF API (stable across OTP versions)
- Lua's C API (stable since Lua 5.1)
- Perl's XS API
- Objective-C runtime + Metal + CoreGraphics + CoreText

### Type translation at the boundary

### Type translation at the boundary

Each bridge crate exposes explicit, named marshaling functions
(`str_from_py`, `int_from_py`, `list_from_py`, etc. — see DS01 §Python
Bridge for the canonical example). The native-extension crate calls
these explicitly; there is no `FromPyObject`-style trait magic. The
common types each bridge handles:

| Conceptual type       | Bridge representation                              |
|-----------------------|----------------------------------------------------|
| `i64`, `f64`, `u32`, …| Direct primitive marshal: `int_from_py(obj) -> i64`, etc. |
| `String` ↔ str        | `str_from_py(obj) -> &str` (borrows), `str_to_py(s) -> *mut PyObject` (new ref) |
| `&[T]` numeric        | `list_from_py(obj, convert_fn) -> Vec<T>`           |
| `Vec<T>` return       | `list_to_py(slice, convert_fn) -> *mut PyObject`   |
| `Result<T, E>`        | Convert error variant to bridge's `raise_*` then return language-runtime null/nil/undefined |
| `Option<T>` (numeric) | NA bit pattern carries Optionality (Layer 0 convention) |
| Opaque resource       | `wrap<T>(type_obj, value)` / `unwrap<T>(obj)` — Rust struct boxed inside a host-runtime object |
| Closure / callback    | Function pointer + opaque-handle user data         |

Rust-only types (`Vec`, `Box`, `HashMap`, `Rc`, etc.) **never appear
in `extern "C"` entry-point signatures**. They are constructed inside
the native-extension function body, used for the work, and dropped
before return.

### Error model across native extensions

Each Layer 1/3 core's error enum (`StatsError`, `MathError`, …) maps
to host-runtime exceptions/errors via the bridge:

| Source                 | Translation in native-extension                  |
|------------------------|--------------------------------------------------|
| `StatsError::DomainError`  | `python_bridge::error::raise_value_error(...)` |
| `StatsError::EmptyInput`   | `python_bridge::error::raise_value_error(...)` |
| `StatsError::Singular`     | `python_bridge::error::raise_arithmetic_error(...)` |
| Spreadsheet `#NUM!`        | Translates to host language's numeric-error type |
| Spreadsheet `#REF!`        | `LookupError`-equivalent in host                |

Translation is mechanical (a `match` arm per error variant). The
native-extension crate owns the table; the bridge owns the
`raise_*` machinery.

### Memory ownership

Every bridge documents new-reference vs. borrowed-reference at every
function. The native-extension crate must:

- `incref` when returning a borrowed reference to the host
- `decref` exactly once when releasing a new reference it owns
- Box-and-leak Rust data when storing in a host object; reconstruct
  the Box and drop in the host object's `dealloc`/`finalize`/`__del__`
  callback

This is manual but visible. Refcount bugs surface as leaks (forgot
`decref`) or use-after-free (`decref` too early) — both diagnosable
from a shallow stack trace, unlike framework-mediated refcount bugs.

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

### Layer 7 — Bridges and Native Extensions

Two sub-layers: **bridge crates** (one per host-language ecosystem,
shared across every domain) and **native-extension crates** (one per
domain × language, the glue that imports both).

**Bridge crates** — already in the repo follow DS01's zero-dependency
pattern. The ones we have plus the ones we need:

| Bridge crate | Status | LOC | Wraps | Used for Mosaic backend? |
|--------------|--------|-----|-------|--------------------------|
| `python-bridge` | ✅ | 569 | CPython C API (Limited API, PEP 384) | No — Python notebooks, scripting |
| `ruby-bridge` | ✅ | 577 | Ruby C extension API | No — Ruby tooling |
| `node-bridge` | ✅ | 1175 | Node.js N-API | No — Node tooling (but adjacent to browser) |
| `lua-bridge` | ✅ | 548 | Lua 5.1+ C API | No — embedded Lua scripting |
| `perl-bridge` | ✅ | 633 | Perl XS API | No — Perl tooling |
| `objc-bridge` | ✅ | 1085 | Objective-C runtime, **Metal**, CoreGraphics, CoreText | **Yes — Mosaic Metal/SwiftUI backend** |
| `erl-nif-bridge` | ✅ | 1030 | Erlang NIF API | No — BEAM tooling |
| `wasm-bridge` | ⬜ | est. ~600 | wasm32-unknown-unknown extern "C" + JS-side glue conventions | **Yes — Mosaic Web Components / React backends** |
| `jvm-bridge` | ⬜ | est. ~800 | JNI (libjvm `JNI_*` functions); Kotlin's `@JvmStatic` ABI | **Yes — Mosaic Compose backend** |
| `dotnet-bridge` | ⬜ | est. ~700 | .NET P/Invoke conventions (C ABI surface that `[DllImport]` consumes) | **Yes — Mosaic XAML / WPF / WinUI / MAUI** |
| `cpp-bridge` | ⬜ | est. ~600 | C++ ABI surface (extern "C" + name-mangling-aware headers for class methods) | **Yes — Mosaic Qt backend** |
| `c-bridge` | ⬜ | est. ~400 | Plain C ABI floor — extern "C" + manually-emitted header (no cbindgen) | **Yes — Mosaic Win32 backend; floor for all others** |

The `c-bridge` is the *floor* — it's a hand-emitted `.h` file with
`extern "C"` declarations matching each native-extension crate's
entry points. Every higher bridge (`dotnet-`, `cpp-`, `jvm-`,
`wasm-`) imports it. No cbindgen; we own the header, line by line,
in the spirit of DS01's "explicit and greppable" principle.

**Native-extension crates** — one per (domain, host language).
Imports its domain core + its language bridge, exports the runtime
entry points. Lives at `code/packages/rust/<domain>-<language>-native/`.

| Naming | Imports | Exports |
|--------|---------|---------|
| `statistics-core-python-native` | `statistics-core` + `python-bridge` | `PyInit_statistics_core` |
| `statistics-core-ruby-native` | `statistics-core` + `ruby-bridge` | `Init_statistics_core` |
| `statistics-core-node-native` | `statistics-core` + `node-bridge` | `napi_register_module_v1` |
| `statistics-core-wasm-native` | `statistics-core` + `wasm-bridge` | `extern "C"` exports per function |
| `statistics-core-objc-native` | `statistics-core` + `objc-bridge` | `extern "C"` Swift-callable surface |
| `statistics-core-jvm-native` | `statistics-core` + `jvm-bridge` | `Java_*_*` JNI entry points |
| `statistics-core-dotnet-native` | `statistics-core` + `dotnet-bridge` | `extern "C"` P/Invoke surface |
| `statistics-core-cpp-native` | `statistics-core` + `cpp-bridge` | `extern "C"` + C++ header alongside |
| `spreadsheet-core-<lang>-native` | `spreadsheet-core` + Layer 1 cores + bridge | as above for each language |

Pattern repeats per (domain, language). The catalog does **not**
preemptively build all `n_domains × n_languages` crates. Each one
ships when a real consumer asks for it. Day one: the **wasm-native**
crates of `spreadsheet-core` and `statistics-core` so the Mosaic
Web-Components backend can call the Rust engine from a browser.

### Crate-count update

Bridges: 7 ✅ + 5 ⬜ = 12 total when complete.

Native extensions: domain × language matrix; ~12 domains × ~12
languages = up to 144 possible, but only the cells with real
consumers get built. Realistic near-term: ~20 native-extension
crates across the first wave of frontends.

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

### Wave 3 — Frontends, I/O, and the first bridges

```
Impl PR M: visicalc-modern shell             (Mosaic UI; headless tests first)
Impl PR N: csv-io                            (proves the I/O pattern)
Impl PR O: wasm-bridge                       (zero-dep wrapper around wasm32 extern "C" surface
                                              + JS glue conventions; needed before any browser
                                              demo of visicalc-modern can ship)
Impl PR P: spreadsheet-core-wasm-native      (first native-extension; proves the
                                              core + bridge + native-extension pattern)
Impl PR Q: statistics-core-wasm-native       (same pattern, second domain)
Impl PR R: xlsx-io                           (the big one; reuses every prior crate)
```

The wasm-bridge has priority over other missing bridges because
Mosaic's Web Components backend is the lowest-friction way to demo
visicalc-modern (no installer, no native compile per platform). The
Metal track depends on objc-bridge which already exists; the Compose
/ XAML / Qt / Win32 tracks depend on their bridges (jvm-bridge,
dotnet-bridge, cpp-bridge, c-bridge) and are deferred to Wave 5 or
beyond as those frontend targets prioritize.

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
- Bridge crate (one per host language ecosystem): `<language>-bridge`
  (e.g. `python-bridge`, `wasm-bridge`)
- Native-extension crate (one per domain × language): `<domain>-<language>-native`
  (e.g. `statistics-core-python-native`, `spreadsheet-core-wasm-native`)
- I/O: `<format>-io` (e.g. `xlsx-io`, not `io-xlsx`)
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
- `code/specs/DS01-ffi-bridges.md` — the bridge architecture this
  catalog builds on; defines the zero-dep, no-macros, no-codegen,
  ~300-1200 LOC pattern that every bridge crate follows
- The repo's existing bridge crates as exemplars:
  `python-bridge` (569 LOC), `ruby-bridge` (577), `node-bridge`
  (1175), `lua-bridge` (548), `perl-bridge` (633), `objc-bridge`
  (1085, wraps Metal + CoreGraphics + CoreText), `erl-nif-bridge`
  (1030). All zero third-party Rust dependencies.
- The repo's existing `cas-*` Rust crates as exemplars of the
  small-Cargo-toml, leaf-to-root pattern this catalog continues
- *Not* used: PyO3, Magnus, napi-rs, cbindgen, UniFFI, wasm-bindgen,
  jni-rs, swift-bridge, cxx — these external frameworks are
  deliberately avoided per DS01 rationale (debuggability, comprehension,
  dependency weight, build portability, ABI stability)

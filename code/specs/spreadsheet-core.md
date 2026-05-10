# Spreadsheet Core

## Overview

`spreadsheet-core` is the Rust crate that implements a spreadsheet's
*essential machinery*: the grid of named cells, the formulas they
contain, the dependency graph between cells, and the recalculation
engine that keeps everything consistent. It is the engine that Modern
VisiCalc will sit on top of, but it has no UI of its own — it is a
headless library that any frontend can drive.

The boundary the crate draws is sharp:

> **`spreadsheet-core` owns the cell, the formula, the graph, and the
> dispatch. It does not own statistics, finance, or any specific
> mathematical operation. Those live in their respective Layer 1 cores.
> When `=AVERAGE(A1:A10)` evaluates, `spreadsheet-core` parses the
> formula, resolves `A1:A10` to a vector, looks up `AVERAGE` in the
> dispatch table, and calls `statistics_core::descriptive::mean`.**

The crate inlines *only* the function families that have no life
outside spreadsheets: logical (`IF`, `AND`, `OR`, `NOT`, `IFERROR`)
and information (`ISBLANK`, `ISERROR`, `ISNA`). Pulling those into
their own crates would create dependencies with no second consumer.

The crate lives at `code/packages/rust/spreadsheet-core/` and depends
on `numeric-tower`, `r-vector`, `statistics-core`, `math-core` (when
implemented), `financial-core` (when implemented), and the
grammar-tools-generated `excel-formula-lexer` and
`excel-formula-parser` (compiled from `code/grammars/excel-formula.tokens`
and `excel-formula.grammar` per the existing
`excel-formula-grammar.md` spec).

---

## Where It Fits

```
   visicalc-modern UI / any spreadsheet frontend       (Layer 4)
                       │
                       │  cell edits, range queries, recalc requests
                       ▼
   ┌──────────────────────────────────────────────────────────┐
   │   spreadsheet-core              ← THIS SPEC              │
   │                                                          │
   │   ┌─────────────────────────────────────────────────┐    │
   │   │  Workbook → Sheets → Cells                      │    │
   │   │  Cell = Value | Formula                          │    │
   │   │  Value: empty | bool | number | text | error    │    │
   │   │  Formula: parsed AST + last evaluated value     │    │
   │   └─────────────────────────────────────────────────┘    │
   │                       │                                  │
   │   ┌───────────────────▼─────────────────────────────┐    │
   │   │  Dependency DAG  +  topological recalc         │    │
   │   │  Cycle detection (Tarjan SCC)                  │    │
   │   └───────────────────┬─────────────────────────────┘    │
   │                       │                                  │
   │   ┌───────────────────▼─────────────────────────────┐    │
   │   │  Dispatch table: name → core fn                 │    │
   │   │  AVERAGE → statistics_core::descriptive::mean   │    │
   │   │  Inlined: IF / AND / OR / NOT / IFERROR / IS*  │    │
   │   └───────────────────┬─────────────────────────────┘    │
   │                       │                                  │
   │   ┌───────────────────▼─────────────────────────────┐    │
   │   │  Excel-formula-grammar parser (existing)        │    │
   │   └─────────────────────────────────────────────────┘    │
   └──────────────────────────────────────────────────────────┘
                       │
                       │  delegates math
                       ▼
   statistics-core · math-core · financial-core · lookup-core · text-core
                       │
                       ▼
                numeric-tower · r-vector · na-semantics
```

**Used by:** `visicalc-modern` (the Rust-side VisiCalc reconstruction);
any future spreadsheet UI; potential CLI spreadsheet tool;
diff-and-test tooling for spreadsheet files.

**Not used by:** `r-runtime` or `s-runtime`. They have their own
syntax tree, their own dispatch, their own dependency model. The
shared piece is the Layer 1 cores beneath; the spreadsheet's grammar
and dependency graph are spreadsheet-specific.

---

## §1 The Cell

A cell is the atomic unit. It has a position and one of two
contents: a literal value, or a formula whose evaluation produces a
value.

```rust
pub struct Cell {
    pub address: CellAddress,
    pub content: CellContent,
    pub format: CellFormat,         // display only; not consulted by recalc
}

pub enum CellContent {
    Empty,                         // distinct from "string of length zero"
    Value(CellValue),              // literal
    Formula {
        ast: FormulaAst,
        cached_value: Option<CellValue>,   // None until first eval
        last_eval_epoch: u64,              // for incremental recalc
    },
}

pub enum CellValue {
    Empty,                          // see §2 — empty as a *value*
    Boolean(bool),
    Number(Number),                 // from numeric-tower
    Text(String),
    Error(SpreadsheetError),
    Array(CellArray),               // 2-D array result (dynamic-array spilling)
    Reference(RangeRef),            // intermediate; usually unwrapped before storage
}

pub enum SpreadsheetError {
    Ref,        // #REF!  — a reference no longer points to a valid cell
    Name,       // #NAME? — unrecognized function or identifier
    DivZero,    // #DIV/0!
    Value,      // #VALUE! — type mismatch
    NA,         // #N/A — explicit NA, or unmatched lookup
    Num,        // #NUM!  — numerical error (out of range, no convergence)
    Null,       // #NULL! — empty intersection
    Calc,       // #CALC! — dynamic-array calculation issue (Excel 365)
    Spill,      // #SPILL! — dynamic-array would overwrite non-empty cell
    Getting,    // #GETTING_DATA — async/external data pending (rare)
}
```

The error variants are specifically those Microsoft Excel propagates
on the wire. Lotus 1-2-3 had only `ERR` and `NA`; we map both into
`Error(NA)` and `Error(Calc)` respectively when reading legacy files.

---

## §2 The Empty Cell — Distinct From NA

The empty cell is a sentinel that **does not exist** in `r-vector`.
It is a spreadsheet-specific value with its own coercion rules:

| Context                        | Empty cell coerces to |
|--------------------------------|-----------------------|
| Arithmetic (`A1 + 5` where A1 is empty) | `0` |
| Text concat (`A1 & "x"`)       | `""` |
| Equality (`A1 = 0`)            | `TRUE` |
| Equality (`A1 = ""`)            | `TRUE` |
| `COUNT(A1:A10)` over partly-empty | empty cells skipped |
| `COUNTA(A1:A10)` over partly-empty | empty cells skipped (only counts non-empty) |
| `COUNTBLANK(A1:A10)` | empty cells counted |
| Passed to a Layer 1 core function | converted to NA at the boundary |

The last row is the load-bearing one: when a range gets passed to
`statistics_core::mean`, blank cells materialize as NA in the
resulting `Vector<Double>`. This is how `=AVERAGE(A1:A10)` does the
right thing even when A3 is blank.

The reverse of this conversion — explicit `=NA()` in a cell —
materializes as `Error(NA)` on the spreadsheet side and as
`Number::Float(NA_REAL)` when extracted to a vector.

The two sentinels both exist in `CellContent::Empty` (the cell is
literally empty) and `CellValue::Error(NA)` (the cell explicitly
contains an NA), and they are distinct everywhere except at the
boundary to `r-vector` where they happen to merge. Excel makes the
same distinction; we reproduce it.

---

## §3 Cell Addresses and Ranges

```rust
pub struct CellAddress {
    pub sheet: SheetId,
    pub row: u32,        // 1-based; A1 is row 1, col 1
    pub col: u32,
    pub absolute_row: bool,    // $ on row
    pub absolute_col: bool,    // $ on col
}

pub struct CellRange {
    pub start: CellAddress,
    pub end: CellAddress,
}

pub enum RangeRef {
    Cell(CellAddress),
    Range(CellRange),
    Union(Vec<RangeRef>),                 // A1:A10, B1:B10
    Intersection(Box<RangeRef>, Box<RangeRef>),  // (A1:A10) (1:1) → A1
    StructuredTable(TableRef),            // [Table1[Column1]]
    NamedRange(String),                   // resolves to a CellRange
}
```

A1 notation parsing and printing is bidirectional and round-trips
exactly. R1C1 notation is a display-only mode toggled per workbook;
the internal representation is always A1.

The `absolute_row` / `absolute_col` flags drive *fill-down*
behavior — copying a formula from A1 to A2 increments relative rows
but not absolute. The recalc engine itself does not care about
absoluteness; it sees the resolved address.

Sheet IDs are dense `u32`s, separately tracked from sheet names so
that renaming a sheet does not require rewriting every formula. The
formula AST holds `SheetId`; the parser converts names at parse time.

---

## §4 The Workbook

```rust
pub struct Workbook {
    pub sheets: Vec<Sheet>,
    pub sheet_id_by_name: HashMap<String, SheetId>,
    pub named_ranges: HashMap<String, RangeRef>,
    pub tables: HashMap<String, Table>,    // structured-reference targets
    pub graph: DependencyGraph,
    pub epoch: u64,                        // bumped on every successful recalc
    pub recalc_mode: RecalcMode,
    pub iteration: IterationConfig,        // for circular references with iterative resolution
}

pub enum RecalcMode {
    Automatic,                  // recalc immediately after each edit
    AutomaticExceptDataTables,  // Excel default
    Manual,                     // user triggers via API
}
```

The workbook is the unit of recalc. Multi-sheet dependencies live in
the same graph.

---

## §5 The Dependency Graph

```rust
pub struct DependencyGraph {
    edges_out: HashMap<CellAddress, Vec<CellAddress>>,  // who I depend on
    edges_in:  HashMap<CellAddress, Vec<CellAddress>>,  // who depends on me
    volatile:  HashSet<CellAddress>,                    // RAND, NOW, TODAY, …
}
```

Two index directions because both queries are hot:

- After editing C1, "what cells must recalc?" is `transitive_closure(edges_in[C1])`.
- After parsing a formula in D5, "register its dependencies" is
  `for ref in formula.refs { edges_out[D5].push(ref); edges_in[ref].push(D5); }`.

### Building the graph

The graph is rebuilt incrementally on every formula edit, never
en-masse. When a cell's formula changes:

1. Drop all existing `edges_out[cell]` entries (and the corresponding
   `edges_in[*]` reverse entries).
2. Walk the new formula AST collecting `CellRef` nodes; expand ranges
   to per-cell entries.
3. Add the new edges.

Range references (`A1:A10`) are stored as 10 separate edges. This
makes recalc fan-out simple at the cost of memory for large ranges.
For ranges over 1 000 cells, an alternative range-keyed graph is
considered (deferred to v2 — the educational v1 stays simple).

### Volatile functions

`RAND`, `RANDBETWEEN`, `RANDARRAY`, `NOW`, `TODAY`, `OFFSET`,
`INDIRECT`, `INFO`, `CELL` are *volatile*: they recalc on every recalc
even if their inputs did not change. They live in a separate
`volatile` set so the recalc engine can union the dirty set with all
volatile cells before topological-sorting.

### Cycle detection

Cycles are detected via Tarjan's strongly-connected-components
algorithm run on the graph. If any SCC has size > 1, that's a cycle
and (by default) every cell in the SCC evaluates to `Error(Ref)`.
With `IterationConfig::enabled = true`, the engine instead does
fixed-point iteration up to `max_iter` rounds with a tolerance of
`epsilon`, matching Excel's "iterative calculation" mode.

---

## §6 Recalc

```rust
pub struct RecalcEngine {
    workbook: &mut Workbook,
}

impl RecalcEngine {
    /// Recalculate everything that depends on the given dirty set.
    pub fn recalc(&mut self, dirty: &[CellAddress]) -> RecalcResult;

    /// Recalculate everything (bumps epoch).
    pub fn recalc_all(&mut self) -> RecalcResult;
}

pub struct RecalcResult {
    pub cells_updated: usize,
    pub errors: Vec<(CellAddress, SpreadsheetError)>,
    pub epoch: u64,
}
```

### Algorithm

1. Compute the full dirty set: union of input dirty + transitive
   downstream + every volatile cell.
2. Run Tarjan SCC on the subgraph induced by the dirty set.
3. Each SCC of size 1 is a non-cyclic cell — evaluate it.
4. Each SCC of size > 1 is a cycle — either error all cells (default)
   or iterate per `IterationConfig` (Excel "enable iterative
   calculation").
5. Within the topological order of SCCs, evaluate each cell's formula
   AST against the workbook's current state.
6. Cache the result in `cached_value` and `last_eval_epoch`.

The iteration ordering is deterministic given the same dirty set and
graph, so two recalcs from the same starting state produce identical
results — important for differential testing of spreadsheet files.

### Incremental recalc

When only `cells_updated < total_cells`, the engine skips the rest of
the workbook entirely. This is what makes interactive editing
performant. The dirty set is small in normal operation (one user
edit, plus the volatile set, plus their transitive downstream).

### Parallelism

Within an SCC of size 1 there is nothing to parallelize. Across
independent SCCs at the same topological level, evaluation runs
through `rayon::par_iter` if the level has more than a threshold of
nodes (default 16). Numeric reproducibility is preserved because each
formula evaluates in isolation; only the *order of completion*
varies, not any individual result.

---

## §7 Formula Parsing

The formula language uses the existing grammar at
`code/grammars/excel-formula.tokens` and
`code/grammars/excel-formula.grammar`, documented in
`excel-formula-grammar.md`. This crate does not redefine the grammar;
it consumes the artifacts produced by the build system from those
two files (the generated `excel-formula-lexer` and
`excel-formula-parser` Rust crates).

The AST after parsing:

```rust
pub enum FormulaAst {
    Literal(CellValue),
    CellRef(CellAddress),
    RangeRef(CellRange),
    Name(String),                          // resolves to NamedRange or function
    UnaryOp { op: UnaryOp, operand: Box<FormulaAst> },
    BinaryOp { op: BinaryOp, lhs: Box<FormulaAst>, rhs: Box<FormulaAst> },
    Postfix { op: PostfixOp, operand: Box<FormulaAst> },     // %
    FunctionCall { name: String, args: Vec<FormulaAst> },
    Array(Vec<Vec<FormulaAst>>),           // {1,2;3,4}
    StructuredRef(TableRef),
    Error(SpreadsheetError),               // explicit #REF! literal
}
```

Parsing produces this AST; evaluation walks it. The crate provides
helpers:

```rust
pub fn parse(input: &str) -> Result<FormulaAst, ParseError>;
pub fn print(ast: &FormulaAst) -> String;     // round-trips parse
pub fn refs(ast: &FormulaAst) -> Vec<RangeRef>;     // for graph building
pub fn is_volatile(ast: &FormulaAst) -> bool;
```

The `print` function is needed because saving an XLSX file requires
re-emitting formula text from the AST, and Excel is picky about
whitespace and parenthesization in some structured-reference forms.

---

## §8 Function Dispatch

The dispatch table maps Excel/Lotus/VisiCalc identifier strings to
core function pointers:

```rust
pub struct DispatchTable {
    entries: HashMap<UniCase<String>, DispatchEntry>,
    // UniCase = case-insensitive ASCII, matches Excel's name resolution
}

pub struct DispatchEntry {
    pub canonical_name: &'static str,         // e.g. "AVERAGE"
    pub aliases: &'static [&'static str],     // e.g. ["AVG"] (Lotus)
    pub frontend_origin: FrontendOrigin,      // VisiCalc, Lotus, Multiplan, Excel
    pub fn_kind: FnKind,
}

pub enum FnKind {
    /// Pure function — same input always gives same output
    Pure(fn(&[CellValue]) -> Result<CellValue, SpreadsheetError>),
    /// Volatile — depends on time / RNG / cell location
    Volatile(fn(&[CellValue], &VolatileContext) -> Result<CellValue, SpreadsheetError>),
    /// Dynamic-array — may return an array that spills into adjacent cells
    DynamicArray(fn(&[CellValue]) -> Result<CellArray, SpreadsheetError>),
    /// Inlined logical/info — see §9
    Inlined(InlineFn),
}
```

The dispatch table is built at crate-init time from a static
manifest. Each Layer 1 core (statistics-core, financial-core, etc.)
exports a `register_dispatch(table: &mut DispatchTable)` function that
adds its entries. The manifest is the source of truth for the
per-frontend alias tables in `statistics-core.md` Part III; mismatch
between manifest and that table is a CI failure.

Lookup is case-insensitive ASCII (R is sensitive, Excel is not — and
we are emulating Excel here). UniCase normalizes at insert and lookup.

### Argument coercion

Each dispatch entry declares its expected argument shape:

```rust
pub struct ArgSpec {
    pub kind: ArgKind,           // Scalar, Vector, Range, Predicate, Array
    pub coerce: CoerceMode,      // ToNumber, ToText, ToLogical, ToVector, AsIs
    pub na_action: NaAction,
    pub variadic_min: Option<usize>,
}
```

The dispatcher coerces actual arguments per this spec before invoking
the underlying core function. Type errors at this stage become
`#VALUE!`. NA propagation respects each Layer 1 core's contract.

---

## §9 Inlined Logical and Information Functions

These functions have no life outside spreadsheets. Their semantics are
spreadsheet-specific (especially `IFERROR`, which catches the
spreadsheet error sentinel — a concept that does not exist in R or S).
Inlined here.

### Logical

| Function | Signature | Semantics |
|----------|-----------|-----------|
| `IF(test, then, else)` | `(CellValue, CellValue, CellValue) -> CellValue` | Lazy: only evaluates branch corresponding to `test` |
| `IFS(test1, val1, test2, val2, ...)` | variadic | First true wins; `#N/A` if none |
| `SWITCH(expr, val1, res1, ..., default)` | variadic | Equality match; default optional |
| `AND(args...)` | variadic | Short-circuits on first FALSE |
| `OR(args...)` | variadic | Short-circuits on first TRUE |
| `XOR(args...)` | variadic | True iff odd number of TRUEs |
| `NOT(x)` | unary | Logical negation |
| `IFERROR(value, value_if_error)` | binary | If `value` is any error, return `value_if_error` |
| `IFNA(value, value_if_na)` | binary | Like IFERROR but only catches `#N/A` |
| `TRUE()`, `FALSE()` | nullary | Constants |

`IF`, `IFS`, `SWITCH`, `AND`, `OR` are **lazy**: they do not evaluate
their non-selected arguments. This is the only place in the
spreadsheet language where eager evaluation is wrong — Excel's
behavior is well-known here.

Implementation: the dispatch table marks these `FnKind::Inlined`, and
the AST evaluator handles them specially before generic argument
evaluation.

### Information

| Function | Signature | Semantics |
|----------|-----------|-----------|
| `ISBLANK(ref)` | `RangeRef -> bool` | Cell content is `Empty` (not `Error`, not `""`) |
| `ISERROR(value)` | `CellValue -> bool` | Any of the 9 error variants |
| `ISERR(value)` | `CellValue -> bool` | Error other than `#N/A` |
| `ISNA(value)` | `CellValue -> bool` | Specifically `#N/A` |
| `ISNUMBER(value)` | `CellValue -> bool` |  |
| `ISTEXT(value)` | `CellValue -> bool` |  |
| `ISLOGICAL(value)` | `CellValue -> bool` |  |
| `ISFORMULA(ref)` | `RangeRef -> bool` | The cell's content is a formula |
| `ISREF(value)` | `CellValue -> bool` | The argument resolves to a valid reference |
| `ISEVEN(num)`, `ISODD(num)` | numeric tests |
| `N(value)` | `CellValue -> Number` | Coerce-to-number with non-numeric → 0 |
| `T(value)` | `CellValue -> String` | Coerce-to-text with non-text → "" |
| `TYPE(value)` | `CellValue -> i32` | 1=number, 2=text, 4=logical, 16=error, 64=array |
| `NA()` | nullary | Returns `Error(NA)` |
| `ERROR.TYPE(value)` | `CellValue -> i32` | 1=#NULL!, 2=#DIV/0!, 3=#VALUE!, 4=#REF!, 5=#NAME?, 6=#NUM!, 7=#N/A, 8=#GETTING_DATA |

These are `FnKind::Pure` because they are not lazy — they evaluate
their argument like normal functions and inspect its type or
identity.

---

## §10 Array Formulas and Dynamic Arrays

Two regimes exist in real Excel:

- **Legacy `{}`-bracketed array formulas**: explicitly entered as
  arrays; results sized to the entered range.
- **Dynamic arrays (Excel 365)**: any formula can return an array,
  which spills into adjacent cells until it hits a non-empty cell or
  the sheet edge.

The crate supports both, controlled by a per-workbook flag
`dynamic_arrays_enabled`. When enabled (the modern default), every
formula is treated as potentially-array-returning. Spilling rules:

1. The formula is evaluated; result is a `CellArray` of dimension
   r × c.
2. Starting at the formula's home cell, the spill range is
   `[home, home+r-1]` × `[home, home+c-1]`.
3. If any cell in the spill range (other than home) is non-empty,
   the home cell evaluates to `Error(Spill)` and no spill happens.
4. Otherwise, every cell in the spill range becomes a *spilled*
   reference back to the home formula. Editing a spilled cell
   replaces the home formula with `Error(Calc)` and clears the spill.

Spilled cells are not separate formulas in the dependency graph;
they are bookkeeping in `Sheet::spill_map: HashMap<CellAddress, CellAddress>`
that points back to the producing formula. The graph treats the
home cell as the only formula, with `edges_out` reflecting the home
formula's references.

### Implicit intersection

Legacy Excel collapsed `=A1:A10` (a range used in scalar context) to
the row-aligned scalar — implicit intersection. The modern dynamic-array
regime returns the full array instead. The flag
`Workbook::implicit_intersection` controls this; default `false`
(modern behavior). Loaded XLSX files set this from the file's
compatibility metadata.

---

## §11 Names and Named Ranges

```rust
pub struct NamedRange {
    pub name: String,
    pub scope: NameScope,                  // Workbook | Sheet(SheetId)
    pub target: RangeRef,
    pub comment: Option<String>,
}

pub enum NameScope {
    Workbook,        // visible to every formula
    Sheet(SheetId),  // shadows workbook-scope name when in that sheet
}
```

Names appear in formulas as `Name(s)` AST nodes; the evaluator
resolves them via `Workbook::named_ranges`, with sheet-scoped names
taking priority over workbook-scoped.

A name shadows a function: if the user names a range `SUM`, it wins
over the built-in `SUM` function. (Excel does this; we follow.)
Resolving `Name → Function` is the fallback path; resolving
`Name → NamedRange` is primary.

---

## §12 Tables and Structured References

Excel tables (`ListObject` in the file format) carry headers and
typed columns. Structured references use bracket syntax:

```
Table1[Column1]                    — entire column
Table1[#Headers]                    — header row
Table1[#Totals]                     — totals row
Table1[@[Column1]:[Column2]]        — current-row span
Table1[[#All],[Column1]]            — all rows including header/totals
```

Parsing is in `excel-formula-grammar.md`; semantics here:

```rust
pub struct Table {
    pub name: String,
    pub range: CellRange,
    pub headers: Vec<String>,
    pub has_header_row: bool,
    pub has_totals_row: bool,
    pub style: TableStyle,
}

pub enum TableRef {
    EntireTable(String),
    Column(String, String),                  // table, column
    HeaderRow(String),
    TotalsRow(String),
    CurrentRow(String, Vec<String>),          // table, columns
    All(String, Vec<String>),                 // including header + totals
}
```

`TableRef` resolves to a `RangeRef` at evaluation time. The
dependency graph stores the resolved range, so adding rows to a
table invalidates all formulas referencing it.

---

## §13 The Cache and Epochs

Every cell has `cached_value` and `last_eval_epoch`. The workbook
has an `epoch` counter, bumped each successful `recalc_all` call.

A cell is *up to date* iff its `last_eval_epoch == workbook.epoch`
and its dependencies are all up to date. This invariant lets
incremental recalc cheaply skip cells whose `last_eval_epoch` matches
the current epoch.

Loading a file from disk sets all `cached_value` to `None` and
triggers a `recalc_all`. Saving stores both the formula text and the
last cached value (matching XLSX behavior for tools that don't
recalc on open).

---

## §14 Errors That Propagate

Spreadsheet errors propagate through arithmetic and function calls
specifically:

| Operation                  | Error result                                     |
|----------------------------|---------------------------------------------------|
| `error + anything`         | the error (left has priority over right)         |
| `IF(error, t, f)`          | the error (the test errored before branching)   |
| `IFERROR(error, fallback)` | `fallback`                                       |
| `IFNA(#N/A, fallback)`     | `fallback` (only for #N/A specifically)          |
| `ISERROR(error)`           | `TRUE` (does not propagate; it is a query)       |

Multiple errors in one expression: the *first* error encountered in
left-to-right evaluation wins. This matches Excel.

Layer 1 core errors (`StatsError` etc.) are translated to spreadsheet
errors at the dispatch boundary per the table in
`statistics-core.md` §1. The translation lives in the dispatcher.

---

## §15 Iterative Calculation

For users who deliberately introduce circular references (financial
models with goal-seek, etc.):

```rust
pub struct IterationConfig {
    pub enabled: bool,
    pub max_iter: u32,           // default 100
    pub epsilon: f64,             // default 0.001 (Excel default)
}
```

When enabled and the SCC has size > 1, the engine evaluates the cycle
in a loop, applying each formula to the previous iteration's values.
Termination: change between iterations < epsilon for every cell, OR
`max_iter` reached. On non-convergence, cells take their last value
(no error).

Without `enabled = true`, every cell in a cycle gets `Error(Ref)`.

---

## §16 Persistence

Out of scope for this crate. Loading and saving XLSX files lives in a
future `xlsx-io` crate. This crate exposes:

```rust
pub fn workbook_from_grid(rows: &[&[Cell]]) -> Workbook;
pub fn workbook_to_grid(wb: &Workbook) -> Vec<Vec<Cell>>;
```

— enough for in-memory construction and inspection. Round-tripping
through XLSX (with all the file format's quirks) is the next layer
up.

---

## §17 Test Vectors

Required parity tests against Microsoft Excel and LibreOffice Calc:

1. Recalc determinism: same edits in same order produce same final
   state, byte-for-byte
2. Cycle detection: every cycle correctly identifies all cells in
   the SCC
3. Volatile recalc: every recalc updates `RAND` even with no edits
4. Dynamic-array spilling: formulas returning arrays spill correctly,
   produce `#SPILL!` when blocked
5. Empty-cell coercion: `=A1+5` where A1 is blank returns 5
6. NA round-trip: `=NA()` in A1, `=A1+5` in B1 → `#N/A` in B1
7. Lazy evaluation: `IF(A1=0, 0, 1/A1)` does not raise `#DIV/0!` when A1 = 0
8. Function dispatch: every Excel function in the manifest invokes
   its declared core function (manifest cross-check)
9. Round-trip: every formula from a corpus parses, prints back to
   identical text (modulo whitespace), evaluates identically

The corpus lives at `code/packages/rust/spreadsheet-core/tests/corpora/`
and includes:

- The full Excel function reference (one formula per function)
- The Lotus 1-2-3 sample workbook set (period-correct)
- A handful of pathological cases (deep recursion, large arrays,
  many cycles)

---

## §18 Performance Targets

For the educational reconstruction (not production-Excel-grade):

- Recalc 10 000 cells in < 100 ms on a modern laptop
- Cycle detection on 100 000-cell graph in < 50 ms
- Memory: < 100 bytes per cell in steady state, exclusive of formula
  AST size

These are achievable with the simple representations above; we do not
need the optimizations real Excel applies (formula deduplication,
lazy graph construction, JIT compilation of hot formulas).

---

## §19 Out of Scope

- File I/O (XLSX, ODS, CSV) — `xlsx-io`, `ods-io`, `csv-io`
- UI rendering — `visicalc-modern` and other frontends
- Charts — `chart-core`
- Pivot tables — `pivot-core`
- Macros / VBA / LAMBDA / LET — language-runtime concern; LAMBDA and
  LET land in v2 once the formula AST has user-defined function
  support
- Real-time data refresh (RTD) — out of scope
- Comments and threaded discussions — `xlsx-io` carries them; this
  crate does not consult them
- Conditional formatting — display only; not a recalc input
- Data validation — input gating; not a recalc input

---

## References

- Microsoft `MS-XLSX` Open Specifications, §2.2.2 (formula grammar) —
  consumed via `excel-formula-grammar.md`
- Excel function reference (Microsoft Learn) — primary alias source
- Lotus Development Corp., *1-2-3 Reference Manual* (1983)
- Bricklin & Frankston, *VisiCalc User's Guide* (1979)
- Sestoft, *Spreadsheet Implementation Technology* (2014) — closest
  textbook on this exact topic
- Tarjan, "Depth-first search and linear graph algorithms" (1972) — SCC
  algorithm

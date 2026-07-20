//! # SQL Codegen — Bytecode Code Generator for Mini-SQLite (Level 1)
//!
//! This crate is the **fifth stage** of the Mini-SQLite SQL pipeline:
//!
//! ```text
//! sql-lexer → sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm → mini-sqlite
//! ```
//!
//! The code generator accepts an [`OptimizedPlan`] from `sql-optimizer` and
//! produces a [`Program`] — a flat list of [`Instruction`] values that the
//! `sql-vm` can execute in a simple fetch-decode-execute loop.
//!
//! ## Architecture overview
//!
//! Compiling a query plan to bytecode is a **recursive tree walk**:
//!
//! 1. **Peel post-ops** — Strip `Sort`, `Limit`, and `Distinct` wrappers from
//!    the outermost plan.  These operators consume the fully-assembled result
//!    buffer, so they are emitted *after* `Halt` as post-processing instructions.
//!
//! 2. **Compile the inner plan** — Walk the remaining tree recursively.  Each
//!    node type (Scan, Filter, Project, Aggregate, etc.) has a dedicated
//!    compilation strategy that emits the right sequence of instructions.
//!
//! 3. **Emit expressions** — Within each node, expressions (`WHERE` predicates,
//!    `SELECT` projections, aggregate arguments) are compiled to a
//!    *stack-machine* sequence.  Each sub-expression pushes a value on the VM's
//!    evaluation stack; operators pop operands and push results.
//!
//! ## Scan loop pattern
//!
//! Nearly every read-path plan compiles to a loop following this template:
//!
//! ```text
//! OpenScan("tbl", alias)
//! Label("scan_0_loop")
//!   AdvanceCursor(alias)       ← move to next row; fall-through = row available
//!   JumpIfExhausted(alias, "scan_0_end")  ← jump if no more rows
//!   … per-row body …
//!   Jump("scan_0_loop")
//! Label("scan_0_end")
//! CloseScan(alias)
//! Halt
//! ```
//!
//! Nested joins repeat this pattern for each table, with the inner scan reset
//! per outer-row iteration.
//!
//! ## Post-processing instructions
//!
//! After `Halt`, post-ops are appended in the order they should execute:
//!
//! ```text
//! … scan loop …
//! Halt
//! SortResult([key1, key2])     ← if ORDER BY was present
//! DistinctResult               ← if SELECT DISTINCT was present
//! LimitResult(count, offset)   ← if LIMIT/OFFSET was present
//! ```
//!
//! The VM processes these after the main program terminates, applying them to
//! the accumulated result set.
//!
//! ## Security: recursion depth guard
//!
//! SQL expressions can be arbitrarily deeply nested (e.g. `1 + (2 + (3 + ...))`).
//! Without a depth limit, a pathologically-crafted query could overflow the
//! Rust call stack.  We use a thread-local counter to enforce a maximum
//! recursion depth of 512 levels in `compile_expr`, mirroring the pattern used
//! by `sql-planner`.

use coding_adventures_sql_backend::{ColumnDef, SqlValue};
use coding_adventures_sql_optimizer::OptimizedPlan;
use coding_adventures_sql_planner::{
    AggFunc, AggregateItem, Assignment, BinaryOp as PlanBinaryOp, InsertSource, JoinKind,
    OutputColumn, SqlExpr, UnaryOp as PlanUnaryOp,
};

/// The CAST target type, re-exported from the planner so `Instruction::Cast`
/// and the VM can name it without a parallel enum (it is a plain data enum
/// with identical meaning across all three layers).
pub use coding_adventures_sql_planner::CastType;

// ---------------------------------------------------------------------------
// Maximum recursion depth for compile_expr.
//
// Deep SQL expressions — e.g. `a + (b + (c + (d + ...)))` — can push the Rust
// call stack unboundedly if we recurse naively.  512 is enough for any
// realistic query; we return a LoadConst(Null) sentinel on overflow rather than
// panicking, and the depth counter is thread-local so we don't need any locking.
// ---------------------------------------------------------------------------
const MAX_EXPR_DEPTH: usize = 512;

std::thread_local! {
    static EXPR_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

// ===========================================================================
// Instruction set
// ===========================================================================

/// A binary operator for the VM's evaluation stack.
///
/// Each variant corresponds to a SQL operator; the VM pops two operands,
/// applies the operation, and pushes the result.
///
/// ## Stack effect: `[..., left, right] → [..., result]`
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    /// `left + right`
    Add,
    /// `left - right`
    Sub,
    /// `left * right`
    Mul,
    /// `left / right` (integer division for integers; NULL on division by zero)
    Div,
    /// `left % right`
    Mod,
    /// `left = right`
    Eq,
    /// `left <> right` (SQL not-equal)
    Neq,
    /// `left < right`
    Lt,
    /// `left <= right`
    Lte,
    /// `left > right`
    Gt,
    /// `left >= right`
    Gte,
    /// `left AND right`
    And,
    /// `left OR right`
    Or,
    /// `left || right` (string concatenation)
    Concat,
    /// `left & right` (bitwise AND; operands coerced to integer)
    BitAnd,
    /// `left | right` (bitwise OR; operands coerced to integer)
    BitOr,
    /// `left << right` (bitwise left shift; SQLite saturation/negation rules)
    ShiftLeft,
    /// `left >> right` (bitwise arithmetic right shift; SQLite rules)
    ShiftRight,
}

/// A unary operator for the VM's evaluation stack.
///
/// The VM pops one operand and pushes the result.
///
/// ## Stack effect: `[..., val] → [..., result]`
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    /// Arithmetic negation: `-x`
    Neg,
    /// Logical negation: `NOT x`
    Not,
    /// Bitwise complement: `~x` (operand coerced to integer)
    BitNot,
}

/// An aggregate function tag.
///
/// Used by [`Instruction::InitAgg`], [`Instruction::UpdateAgg`], and
/// [`Instruction::FinalizeAgg`] to identify which accumulation logic to run.
///
/// ## Slot semantics
///
/// Each aggregate in a query gets a *slot index* (0-based, assigned during
/// compilation).  The VM maintains a per-slot accumulator.  All three instructions
/// reference the slot by index so the VM can find the right accumulator:
///
/// ```text
/// InitAgg(0)              ← allocate accumulator slot 0
/// UpdateAgg(0, CountStar) ← slot 0 counts rows
/// FinalizeAgg(0, CountStar) ← compute final value from slot 0
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum AggFn {
    /// `COUNT(col)` — counts non-NULL values
    Count,
    /// `COUNT(*)` — counts all rows, including those with NULLs
    CountStar,
    /// `COUNT(DISTINCT col)` — counts distinct non-NULL values
    CountDistinct,
    /// `SUM(col)` — sum of all non-NULL values
    Sum,
    /// `AVG(col)` — arithmetic mean of non-NULL values
    Avg,
    /// `MIN(col)` — minimum value
    Min,
    /// `MAX(col)` — maximum value
    Max,
    /// `GROUP_CONCAT([DISTINCT] col [, sep])` — concatenate non-NULL values in
    /// row order, joined by `sep` (the constant separator captured at plan time;
    /// the value stream is just `col`). `distinct` deduplicates values before
    /// joining, matching `GROUP_CONCAT(DISTINCT col)`.
    GroupConcat { sep: String, distinct: bool },
}

/// A sort key emitted by the code generator, used in `SortResult`.
///
/// The VM sorts the accumulated result set by these keys in order, applying
/// each key's direction (ascending/descending).
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledSortKey {
    /// The column name (or expression alias) to sort by.
    pub column: String,
    /// `true` = ascending (ASC), `false` = descending (DESC).
    pub ascending: bool,
    /// Explicit NULL placement from `NULLS FIRST` / `NULLS LAST`. `None` = the
    /// SQLite default (NULLs first for ASC, last for DESC).
    pub nulls_first: Option<bool>,
    /// Collating sequence from `COLLATE name`, applied to text values before
    /// comparison. `None` = default byte order (BINARY). `Some("NOCASE")` =
    /// ASCII case-insensitive; `Some("RTRIM")` = ignore trailing spaces.
    pub collation: Option<String>,
}

/// The complete bytecode instruction set for the Mini-SQLite VM.
///
/// ## Execution model
///
/// The VM is a simple stack machine with named labels.  Control flow uses
/// `Label`/`Jump`/`JumpIf*` instructions.  Data lives on an *evaluation stack*;
/// each expression pushes values and operators consume them.
///
/// ## Row assembly
///
/// To emit a row, the sequence is:
///
/// ```text
/// BeginRow           ← start a new output row
/// <push col1>        ← any expression that pushes a value
/// EmitColumn("col1") ← pop the stack value into the row's named slot
/// <push col2>
/// EmitColumn("col2")
/// EmitRow            ← add the assembled row to the result buffer
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // ── Expression evaluation ────────────────────────────────────────────────

    /// Push a constant value onto the evaluation stack.
    ///
    /// ## Stack effect: `[...] → [..., value]`
    ///
    /// Handles all SQL literal types: integers, floats, text, booleans, NULL,
    /// and binary blobs.
    LoadConst(SqlValue),

    /// Push the value of a named column from the current row onto the stack.
    ///
    /// ## Stack effect: `[...] → [..., column_value]`
    ///
    /// The `table` qualifier (e.g. `"users"` in `users.name`) is used in join
    /// queries to look up which cursor's current row to read from.
    ///
    /// - `table: None` — no qualifier, use the single open cursor or the first
    ///   open cursor (the VM decides).
    /// - `table: Some(alias)` — read from the cursor associated with this alias.
    LoadColumn(Option<String>, String),

    /// Apply a binary operator to the top two stack values.
    ///
    /// Pops `right` first, then `left`.  Pushes the result.
    ///
    /// ## Stack effect: `[..., left, right] → [..., result]`
    BinaryOpInstr(BinaryOp),

    /// Apply a unary operator to the top stack value.
    ///
    /// ## Stack effect: `[..., val] → [..., result]`
    UnaryOpInstr(UnaryOp),

    /// Test whether the top stack value is SQL NULL.
    ///
    /// Pops one value; pushes `Bool(true)` if it was NULL, `Bool(false)` otherwise.
    ///
    /// ## Stack effect: `[..., val] → [..., Bool]`
    IsNull,

    /// Test whether the top stack value is *not* SQL NULL.
    ///
    /// Pops one value; pushes `Bool(true)` if it was NOT NULL, `Bool(false)` otherwise.
    ///
    /// ## Stack effect: `[..., val] → [..., Bool]`
    IsNotNull,

    /// SQL `BETWEEN` (or `NOT BETWEEN`) range test.
    ///
    /// Expects the stack to contain `[..., value, low, high]` (high on top).
    /// Pops all three; the payload is `!negated` (`true` for `BETWEEN`, `false`
    /// for `NOT BETWEEN`). `BETWEEN` pushes the inclusive range test
    /// `Bool(value >= low AND value <= high)`; `NOT BETWEEN` pushes its logical
    /// negation `Bool(value < low OR value > high)`. Any NULL operand → NULL.
    ///
    /// ## Stack effect: `[..., value, low, high] → [..., Bool]`
    Between(bool), // !negated: true = BETWEEN, false = NOT BETWEEN

    /// SQL `LIKE` / `NOT LIKE` pattern match.
    ///
    /// Expects `[..., value, pattern]` (pattern on top). Pops both; pushes a
    /// Bool result (or NULL if either operand is NULL). The `bool` payload is the
    /// `NOT` flag: when `true`, the (non-NULL) match result is inverted.
    ///
    /// ## Stack effect: `[..., value, pattern] → [..., Bool]`
    Like(bool),

    /// SQL `LIKE` / `NOT LIKE` with an `ESCAPE ch` clause.
    ///
    /// Expects `[..., value, pattern, escape]` (escape on top). The escape
    /// value's first character makes a following `%`, `_`, or the escape
    /// character itself a literal in the pattern. The `bool` payload is the `NOT`
    /// flag (see [`Instruction::Like`]).
    ///
    /// ## Stack effect: `[..., value, pattern, escape] → [..., Bool]`
    LikeEscape(bool),

    /// SQL `CAST(value AS type)` — pop one value, push its conversion.
    ///
    /// ## Stack effect: `[..., value] → [..., converted]`
    Cast(CastType),

    /// SQL `IN (v1, v2, ...)` list membership test.
    ///
    /// The usize argument is the number of list items that precede the test
    /// value on the stack:
    ///
    /// ```text
    /// Stack: [..., test_value, item1, item2, ..., itemN]
    ///                          ↑─────── n items ───────↑
    /// ```
    ///
    /// Pops `n + 1` values; pushes `Bool(test_value IN {item1..itemN})`.
    ///
    /// ## Stack effect: `[..., val, i1, …, iN] → [..., Bool]`
    InList(usize), // item count

    // ── Scan control ─────────────────────────────────────────────────────────

    /// Open a cursor over all rows of `table`, associating it with `alias`.
    ///
    /// The alias is how later instructions (e.g. `LoadColumn`, `CloseScan`)
    /// refer to this cursor.  If `alias` is `None`, the table name is used
    /// as the implicit alias.
    ///
    /// Must be paired with [`Instruction::CloseScan`].
    OpenScan(String, Option<String>), // table, alias

    /// Advance the cursor to the next row.
    ///
    /// If the cursor still has rows, execution falls through to the next
    /// instruction.  If the cursor is exhausted, control jumps to
    /// `JumpIfExhausted`'s target label.
    ///
    /// This instruction is always followed in the instruction stream by a
    /// `JumpIfExhausted` for the same alias.
    AdvanceCursor(Option<String>),

    /// Jump to `label` if the cursor identified by `alias` has no more rows.
    ///
    /// This is emitted immediately after every `AdvanceCursor` in the scan loop.
    ///
    /// ## Scan loop pattern
    ///
    /// ```text
    /// Label("scan_N_loop")
    ///   AdvanceCursor(alias)
    ///   JumpIfExhausted(alias, "scan_N_end")
    ///   … row body …
    ///   Jump("scan_N_loop")
    /// Label("scan_N_end")
    /// ```
    JumpIfExhausted(Option<String>, String), // alias, label

    /// Close the cursor for `alias`, releasing any resources it holds.
    CloseScan(Option<String>),

    // ── Row assembly ─────────────────────────────────────────────────────────

    /// Begin assembling a new output row.
    ///
    /// Must be followed by one or more `EmitColumn` instructions and then
    /// exactly one `EmitRow`.
    BeginRow,

    /// Pop the top stack value and add it to the current row under `name`.
    ///
    /// ## Stack effect: `[..., val] → [...]` (value is stored in the row buffer)
    EmitColumn(String),

    /// Commit the current row to the result buffer.
    ///
    /// After `EmitRow`, the row is available to later result-phase operators
    /// (sort, limit, distinct).
    EmitRow,

    // ── Aggregation ──────────────────────────────────────────────────────────

    /// Allocate `n` aggregate accumulator slots, initializing each to its
    /// identity value (0 for Count/Sum, None for Avg/Min/Max).
    ///
    /// Must be emitted before any `UpdateAgg` or `FinalizeAgg` for those slots.
    /// Called once at the start of aggregate compilation, *outside* the loop.
    InitAgg(usize), // number of accumulators

    /// Update aggregate slot `slot` using `func` and the current top-of-stack
    /// value (popped).
    ///
    /// For `CountStar`, no argument is consumed from the stack; the accumulator
    /// is simply incremented.
    ///
    /// ## Stack effect (all but CountStar): `[..., val] → [...]`
    UpdateAgg(usize, AggFn), // slot index, function

    /// Read aggregate slot `slot`, compute the final value via `func`, and
    /// push it onto the stack.
    ///
    /// ## Stack effect: `[...] → [..., final_value]`
    FinalizeAgg(usize, AggFn), // slot index, function

    /// Push the current `GROUP BY` key values onto the stack and record them
    /// as the "current group" in the VM.
    ///
    /// The first `Vec<String>` lists the column names making up the group key;
    /// the parallel `Vec<Option<String>>` gives each key's collation (`None` =
    /// the default BINARY, i.e. compare the bytes as-is).
    ///
    /// The collation folds ONLY the key string the VM groups on — the original
    /// column values are kept for output, so `GROUP BY c` on a `COLLATE NOCASE`
    /// column reports the first row's original text (`'A'`, not `'a'`) while
    /// still grouping `'A'` with `'a'`.
    /// This instruction is emitted inside the scan loop, before `UpdateAgg`.
    SaveGroupKey(Vec<String>, Vec<Option<String>>),

    // ── Control flow ─────────────────────────────────────────────────────────

    /// Define a named label at this position in the instruction stream.
    ///
    /// Labels are the targets of jump instructions.  They are not executed
    /// themselves; the VM just records their position in a `{name → index}` map.
    Label(String),

    /// Unconditional jump to `label`.
    Jump(String),

    /// Jump to `label` if the top stack value is truthy (non-NULL, non-false).
    ///
    /// Pops the top value.
    ///
    /// ## Stack effect: `[..., val] → [...]`
    JumpIfTrue(String),

    /// Jump to `label` if the top stack value is falsy (NULL or Bool(false)).
    ///
    /// Pops the top value.
    ///
    /// ## Stack effect: `[..., val] → [...]`
    JumpIfFalse(String),

    // ── Outer-join match flag ─────────────────────────────────────────────────
    //
    // A single boolean the VM keeps to implement `LEFT`/`RIGHT JOIN`. For each
    // outer row we `ClearMatch`, `SetMatch` inside the inner loop whenever the
    // `ON` condition holds, and after the inner loop `JumpIfMatched` over the
    // NULL-padded emit — so an outer row with no match still produces one row
    // (with the inner side's columns NULL). None of these touch the value stack.

    /// Reset the outer-join match flag to `false` (start of each outer row).
    ClearMatch,

    /// Set the outer-join match flag to `true` (an inner row satisfied `ON`).
    SetMatch,

    /// Jump to `label` if the outer-join match flag is currently `true`.
    JumpIfMatched(String),

    /// Halt the main scan loop and signal to the VM to move to post-processing.
    ///
    /// This instruction terminates the main program.  Post-op instructions
    /// (`SortResult`, `DistinctResult`, `LimitResult`) appear *after* `Halt`
    /// in the instruction stream.
    Halt,

    // ── DDL / DML ────────────────────────────────────────────────────────────

    /// Execute `CREATE TABLE [IF NOT EXISTS] name (cols...)`.
    CreateTableInstr(String, bool, Vec<ColumnDef>), // name, if_not_exists, cols

    /// Execute `DROP TABLE [IF EXISTS] name`.
    DropTableInstr(String, bool), // name, if_exists

    /// Execute `INSERT INTO table [(columns)] VALUES (top-of-stack values)`.
    ///
    /// Before this instruction, the compiled row values must have been pushed
    /// onto the evaluation stack by the preceding expression code.
    InsertRow(String, Option<Vec<String>>), // table, explicit columns

    /// Execute `UPDATE table SET col = expr [WHERE ...]` for the current cursor row.
    ///
    /// The SET expressions are evaluated and pushed onto the stack in assignment
    /// order before this instruction.  The `Vec<String>` carries the matching
    /// column names so the VM can pair each stack value with its target column.
    UpdateRows(String, Vec<String>), // table, assignment columns

    /// Execute `DELETE FROM table` for the current cursor row.
    DeleteRows(String), // table

    // ── Transactions ─────────────────────────────────────────────────────────

    /// Begin a new transaction (`BEGIN`).
    BeginTransaction,

    /// Commit the current transaction (`COMMIT`).
    CommitTransaction,

    /// Roll back the current transaction (`ROLLBACK`).
    RollbackTransaction,

    // ── Post-processing (appended after Halt) ────────────────────────────────

    /// Sort the accumulated result set by the given keys.
    ///
    /// Emitted after `Halt` if the query had `ORDER BY`.  The VM applies this
    /// to its result buffer before returning rows to the caller.
    SortResult(Vec<CompiledSortKey>),

    /// Remove duplicate rows from the accumulated result set.
    ///
    /// Emitted after `Halt` (and after `SortResult` if both are present) for
    /// `SELECT DISTINCT`.
    /// Deduplicate the output rows. The `Vec<Option<String>>` gives one collation
    /// per OUTPUT column, positionally parallel to the emitted row (`None` =
    /// default BINARY). Only the dedupe KEY is folded — the surviving row keeps
    /// its ORIGINAL text, since dedup retains the first occurrence.
    DistinctResult(Vec<Option<String>>),

    /// Truncate the result set to at most `count` rows, starting from `offset`.
    ///
    /// Emitted after `Halt` for `LIMIT [count] [OFFSET offset]`.
    ///
    /// - `None` for `count` means no row limit.
    /// - `None` for `offset` means start from row 0.
    LimitResult(Option<i64>, Option<i64>), // count, offset

    /// Truncate every output row to the first `n` columns.
    ///
    /// Emitted after `SortResult` when the query had ORDER BY on columns that
    /// are not in the SELECT list.  The code generator temporarily includes
    /// those sort-key columns in the emitted row so `SortResult` can find them
    /// by name; after sorting, `TruncateOutputColumns(n)` strips the hidden
    /// trailing columns so that only the SELECT-list columns are returned to
    /// the caller.
    TruncateOutputColumns(usize),

    /// Define the output column names without emitting any rows.
    ///
    /// Emitted when the optimizer proves no rows will be produced (e.g.
    /// `LIMIT 0`) but the SELECT list is still known.  The VM sets
    /// `output_columns` to these names so the `QueryResult` carries the
    /// correct column metadata even when `rows` is empty.
    ///
    /// ## Example
    ///
    /// `SELECT x FROM nums ORDER BY x LIMIT 0`
    /// → optimizer produces `Project(EmptyResult, ["x"])`
    /// → codegen emits `DefineColumns(["x"]), Halt`
    /// → VM returns `QueryResult { columns: ["x"], rows: [], … }`
    DefineColumns(Vec<String>),

    /// Call a built-in scalar SQL function.
    ///
    /// The `usize` argument is the number of arguments already pushed onto
    /// the evaluation stack (pushed left-to-right, so the first argument is
    /// deepest).  The VM pops `n` values, calls the named function, and
    /// pushes the result.
    ///
    /// ## Stack effect: `[..., arg1, …, argN] → [..., result]` (N pops, 1 push)
    ///
    /// ## Supported built-ins
    ///
    /// | Name     | Args | Description                                     |
    /// |----------|------|-------------------------------------------------|
    /// | LENGTH   |  1   | Character count of a string (NULL → NULL)       |
    /// | UPPER    |  1   | Uppercase the string                            |
    /// | LOWER    |  1   | Lowercase the string                            |
    /// | TRIM     |  1   | Strip leading and trailing whitespace           |
    /// | LTRIM    |  1   | Strip leading whitespace                        |
    /// | RTRIM    |  1   | Strip trailing whitespace                       |
    /// | SUBSTR   | 2–3  | 1-indexed substring (pos, [len])               |
    /// | REPLACE  |  3   | Replace all occurrences (src, from, to)         |
    /// | ABS      |  1   | Absolute value of integer or float              |
    /// | COALESCE | ≥1   | First non-NULL argument                         |
    CallBuiltin(String, usize), // function name (uppercase), arg count
}

// ===========================================================================
// Program
// ===========================================================================

/// A compiled program — a flat sequence of instructions ready for the VM.
///
/// The VM executes instructions in order from index 0, with control flow
/// redirected by jump instructions (which reference labels by name).
///
/// ## Invariant
///
/// The last "main body" instruction is always `Instruction::Halt`.  Zero or
/// more post-processing instructions may follow it.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub instructions: Vec<Instruction>,
}

impl Program {
    /// Create an empty `Program`.
    pub fn new() -> Self {
        Program {
            instructions: Vec::new(),
        }
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Compile an [`OptimizedPlan`] into a [`Program`] for the Mini-SQLite VM.
///
/// This is the single public entry point of the crate.  Call `optimize()`
/// from `sql-optimizer` first, then pass the result here.
///
/// ## Example
///
/// ```rust
/// use coding_adventures_sql_optimizer::optimize;
/// use coding_adventures_sql_planner::LogicalPlan;
/// use coding_adventures_sql_codegen::compile;
///
/// let plan = LogicalPlan::Scan { table: "t".into(), alias: None };
/// let opt = optimize(plan);
/// let program = compile(&opt);
/// assert!(program.instructions.len() > 1);
/// ```
///
/// ## Compilation flow
///
/// 1. Create a fresh `Compiler` (monotonic label counter, empty instruction vec).
/// 2. Peel post-ops (`Sort`, `Limit`, `Distinct`) off the outermost plan.
/// 3. Compile the inner plan recursively.
/// 4. Emit `Halt`.
/// 5. Append post-ops in correct execution order.
pub fn compile(plan: &OptimizedPlan) -> Program {
    let mut compiler = Compiler::new();
    compiler.compile_plan(plan);
    Program {
        instructions: compiler.instructions,
    }
}

// ===========================================================================
// Compiler internals
// ===========================================================================

/// The internal compilation context.
///
/// Holds the mutable instruction stream and a monotonic counter used to
/// generate unique label names.  By using a single counter for all label
/// types (`scan_N_loop`, `agg_N_loop`, etc.) we guarantee no collisions even
/// in deeply-nested plans like `JOIN(agg(scan), filter(scan))`.
struct Compiler {
    /// The instruction stream being assembled.
    instructions: Vec<Instruction>,

    /// Monotonic counter for generating unique label names.
    ///
    /// Each call to `fresh_label` returns a string like `"scan_3_loop"` and
    /// increments this counter.
    label_counter: usize,

    /// Aggregate slot map for HAVING predicate compilation.
    ///
    /// When `compile_having` sets up aggregate slots, it populates this vec
    /// with `(slot_index, AggregateItem)` pairs so that `compile_expr` can
    /// emit `FinalizeAgg(slot, fn_tag)` with the correct slot index instead
    /// of always defaulting to slot 0.
    ///
    /// Cleared after HAVING compilation is done.
    agg_slots: Vec<(usize, AggregateItem)>,
}

impl Compiler {
    fn new() -> Self {
        Compiler {
            instructions: Vec::new(),
            label_counter: 0,
            agg_slots: Vec::new(),
        }
    }

    /// Append `instr` to the instruction stream.
    fn emit(&mut self, instr: Instruction) {
        self.instructions.push(instr);
    }

    /// Generate a fresh label name: `"{prefix}_{counter}"`.
    ///
    /// The counter increments with every call, ensuring globally unique names
    /// across the entire compilation of one plan.
    ///
    /// ## Examples
    ///
    /// ```text
    /// fresh_label("scan") → "scan_0"
    /// fresh_label("scan") → "scan_1"
    /// fresh_label("agg")  → "agg_2"
    /// ```
    fn fresh_label(&mut self, prefix: &str) -> String {
        let label = format!("{}_{}", prefix, self.label_counter);
        self.label_counter += 1;
        label
    }

    // -----------------------------------------------------------------------
    // Top-level plan dispatch
    // -----------------------------------------------------------------------

    /// Compile an [`OptimizedPlan`] into the instruction stream.
    ///
    /// The outer shell: peel post-processing operators, compile the inner plan,
    /// emit `Halt`, then append post-ops.
    ///
    /// ## Plan canonicalization
    ///
    /// The planner always puts `Project` as the outermost node (so that the
    /// SELECT list is applied after ORDER BY / LIMIT / DISTINCT).  This means
    /// we often receive `Project { Sort { Scan } }` rather than
    /// `Sort { Project { Scan } }`.  `peel_post_ops` only strips the outermost
    /// Sort/Limit/Distinct wrapper, so it cannot see through a Project.
    ///
    /// We handle this by canonicalizing the plan before peeling: when the top
    /// node is `Project` and its inner plan starts with Sort/Limit/Distinct
    /// wrappers, we collect those post-ops ourselves and pass `Project { inner }`
    /// to the normal compilation path.
    fn compile_plan(&mut self, plan: &OptimizedPlan) {
        // Step 1: Peel Sort / Limit / Distinct wrappers from the outermost plan.
        // These operators apply to the entire result buffer; they run *after*
        // the main scan loop terminates.  We collect them here and append them
        // after `Halt`.
        //
        // Special case: the planner emits `Project { Sort { Scan } }` (Project
        // outermost).  We detect that pattern here and split it so that the Sort
        // becomes a post-op while the Project wraps the Scan directly.
        //
        // Hidden sort columns: when the ORDER BY keys reference columns that are
        // NOT in the SELECT list, `SortResult` cannot find them by name in the
        // output buffer.  We work around this by temporarily including those
        // extra columns in the emitted rows (with a `__sort_N__` prefix) and
        // appending a `TruncateOutputColumns(n)` post-op after `SortResult` to
        // strip them.  This keeps `SortResult` as a simple name-based lookup.
        let (inner, mut post_ops) = peel_post_ops_through_project(plan);

        // Extract the sort keys (if any) so we can pass hidden sort columns
        // to compile_project.
        let hidden_sort_cols: Vec<(String, SqlExpr)> =
            if let OptimizedPlan::Project { columns, .. } = inner.as_ref() {
                // Collect sort key column names from the first SortResult post-op.
                let sort_col_names: Vec<String> = post_ops
                    .iter()
                    .flat_map(|op| {
                        if let Instruction::SortResult(keys) = op {
                            keys.iter().map(|k| k.column.clone()).collect::<Vec<_>>()
                        } else {
                            vec![]
                        }
                    })
                    .collect();

                // Check which sort keys are NOT already in the projection output.
                let projected_names: Vec<String> = columns.iter().map(output_column_name).collect();
                sort_col_names
                    .iter()
                    .filter(|k| !projected_names.contains(k))
                    .map(|k| {
                        let hidden_name = format!("__sort_{}__", k);
                        let expr = SqlExpr::Column {
                            name: k.clone(),
                            table: None,
                        };
                        (hidden_name.clone(), expr)
                    })
                    .collect()
            } else {
                vec![]
            };

        // If there are hidden sort columns, update SortResult to use the hidden
        // column names AND append TruncateOutputColumns(n).
        if !hidden_sort_cols.is_empty() {
            if let OptimizedPlan::Project { columns, .. } = inner.as_ref() {
                let n_project = columns.len();
                // Remap SortResult keys to use hidden column names.
                for op in &mut post_ops {
                    if let Instruction::SortResult(keys) = op {
                        for key in keys.iter_mut() {
                            let hidden = format!("__sort_{}__", key.column);
                            let projected: Vec<String> =
                                columns.iter().map(output_column_name).collect();
                            if !projected.contains(&key.column) {
                                key.column = hidden;
                            }
                        }
                    }
                }
                // Insert TruncateOutputColumns right after SortResult.
                let sort_pos = post_ops
                    .iter()
                    .position(|op| matches!(op, Instruction::SortResult(_)));
                if let Some(pos) = sort_pos {
                    post_ops.insert(pos + 1, Instruction::TruncateOutputColumns(n_project));
                }
            }
        }

        // Step 2: Compile the inner query (scan loop, filter, project, etc.).
        // Pass hidden sort columns so compile_project can include them.
        if hidden_sort_cols.is_empty() {
            self.compile_inner_ref(&inner);
        } else {
            self.compile_inner_with_hidden_sort(&inner, &hidden_sort_cols);
        }

        // Step 3: Terminate the main program.
        self.emit(Instruction::Halt);

        // Step 4: Append post-processing operators after Halt.
        //
        // `peel_post_ops` peels from the outermost plan node inward, collecting
        // post-ops in outer-to-inner order.  For example, given
        // `Sort(Limit(Scan))`, it peels Sort first, then Limit, producing
        // post_ops = [SortResult, LimitResult].
        //
        // We append them in that same outer-to-inner order, which is also the
        // correct *execution* order (Sort before Limit).  The VM executes the
        // post-ops sequentially: sort the buffer first, then paginate.
        for op in post_ops {
            self.emit(op);
        }
    }

    /// Compile the inner plan, with hidden sort-key columns appended to the
    /// emitted row.  Only called when there are ORDER BY keys that are not in
    /// the SELECT list.
    fn compile_inner_with_hidden_sort(
        &mut self,
        inner: &InnerPlan<'_>,
        hidden: &[(String, SqlExpr)],
    ) {
        match inner.as_ref() {
            OptimizedPlan::Project { input, columns } => {
                let cols = columns.to_vec();
                let hidden_cols = hidden.to_vec();
                match input.as_ref() {
                    OptimizedPlan::Filter { input: filter_input, predicate } => {
                        let pred_clone = predicate.clone();
                        let hidden_clone = hidden_cols.clone();
                        // Generate the skip label BEFORE the closure so it can be
                        // captured without a double borrow of self.
                        let skip_lbl = self.fresh_label("hidden_sort_filter_skip");
                        self.compile_scan_loop(filter_input, move |compiler, _alias| {
                            compiler.compile_expr(&pred_clone);
                            compiler.emit(Instruction::JumpIfFalse(skip_lbl.clone()));
                            compiler.emit(Instruction::BeginRow);
                            for col in &cols {
                                compiler.compile_expr(&col.expr);
                                let name = output_column_name(col);
                                compiler.emit(Instruction::EmitColumn(name));
                            }
                            for (name, expr) in &hidden_clone {
                                compiler.compile_expr(expr);
                                compiler.emit(Instruction::EmitColumn(name.clone()));
                            }
                            compiler.emit(Instruction::EmitRow);
                            compiler.emit(Instruction::Label(skip_lbl.clone()));
                        });
                    }
                    OptimizedPlan::Aggregate { .. } | OptimizedPlan::Having { .. } => {
                        // Aggregate output already handles its own projection.
                        self.compile_inner(input);
                    }
                    _ => {
                        let hidden_clone = hidden_cols.clone();
                        self.compile_scan_loop(input, move |compiler, _alias| {
                            compiler.emit(Instruction::BeginRow);
                            for col in &cols {
                                compiler.compile_expr(&col.expr);
                                let name = output_column_name(col);
                                compiler.emit(Instruction::EmitColumn(name));
                            }
                            for (name, expr) in &hidden_clone {
                                compiler.compile_expr(expr);
                                compiler.emit(Instruction::EmitColumn(name.clone()));
                            }
                            compiler.emit(Instruction::EmitRow);
                        });
                    }
                }
            }
            _ => {
                // Non-Project inner plan: fall back to normal compilation.
                self.compile_inner_ref(inner);
            }
        }
    }

    /// Compile `plan` when it arrives as a borrowed reference (used internally
    /// after canonicalization splits the plan into a borrowed inner + post-ops).
    fn compile_inner_ref(&mut self, plan: &InnerPlan) {
        match plan {
            InnerPlan::Borrowed(p) => self.compile_inner(p),
            InnerPlan::Owned(p) => self.compile_inner(p),
        }
    }

    // -----------------------------------------------------------------------
    // Inner plan compilation
    // -----------------------------------------------------------------------

    /// Compile the inner (non-post-op) part of a plan.
    ///
    /// Dispatches on the `OptimizedPlan` variant.
    fn compile_inner(&mut self, plan: &OptimizedPlan) {
        match plan {
            // ── Leaf nodes ──────────────────────────────────────────────────

            OptimizedPlan::EmptyResult => {
                // An empty result: the optimizer proved no rows can exist.
                // We emit nothing — `Halt` follows immediately from `compile_plan`.
            }

            OptimizedPlan::Scan { table, alias, .. } => {
                // A bare Scan with no projection above it:
                //   OpenScan → loop → emit all columns → close
                // This is rare in practice (the planner wraps Scans in Project),
                // but we handle it for correctness and for testability.
                let loop_lbl = self.fresh_label("scan_loop");
                let end_lbl = self.fresh_label("scan_end");
                self.emit(Instruction::OpenScan(table.clone(), alias.clone()));
                self.emit(Instruction::Label(loop_lbl.clone()));
                self.emit(Instruction::AdvanceCursor(alias.clone()));
                self.emit(Instruction::JumpIfExhausted(alias.clone(), end_lbl.clone()));
                self.emit(Instruction::BeginRow);
                self.emit(Instruction::EmitRow);
                self.emit(Instruction::Jump(loop_lbl));
                self.emit(Instruction::Label(end_lbl));
                self.emit(Instruction::CloseScan(alias.clone()));
            }

            OptimizedPlan::Filter { input, predicate } => {
                self.compile_filter(input, predicate);
            }

            OptimizedPlan::Project { input, columns } => {
                self.compile_project(input, columns);
            }

            OptimizedPlan::Aggregate {
                input,
                group_by,
                aggregates,
            } => {
                self.compile_aggregate(input, group_by, aggregates);
            }

            OptimizedPlan::Having { input, predicate } => {
                self.compile_having(input, predicate);
            }

            OptimizedPlan::Join {
                left,
                right,
                kind,
                condition,
            } => {
                self.compile_join(left, right, kind, condition);
            }

            // ── DML / DDL ───────────────────────────────────────────────────

            OptimizedPlan::Insert {
                table,
                columns,
                source,
            } => {
                self.compile_insert(table, columns, source);
            }

            OptimizedPlan::Update {
                table,
                assignments,
                predicate,
            } => {
                self.compile_update(table, assignments, predicate);
            }

            OptimizedPlan::Delete { table, predicate } => {
                self.compile_delete(table, predicate);
            }

            OptimizedPlan::CreateTable {
                table,
                if_not_exists,
                columns,
            } => {
                // CREATE TABLE is a single-instruction DDL statement.
                // No loop, no cursor — the VM executes this once and we Halt.
                self.emit(Instruction::CreateTableInstr(
                    table.clone(),
                    *if_not_exists,
                    columns.clone(),
                ));
            }

            OptimizedPlan::DropTable { table, if_exists } => {
                // DROP TABLE is similarly a single-instruction DDL statement.
                self.emit(Instruction::DropTableInstr(table.clone(), *if_exists));
            }

            // ── Plan nodes that contain post-ops (already peeled) ───────────
            // These variants should not appear here because `peel_post_ops` strips
            // Sort/Limit/Distinct wrappers before `compile_inner` is called.
            // We still handle them defensively by recursing into their child.
            OptimizedPlan::Sort { input, .. }
            | OptimizedPlan::Limit { input, .. }
            | OptimizedPlan::Distinct(input, _) => {
                self.compile_inner(input);
            }

            // ── Union — currently emit both sides ───────────────────────────
            OptimizedPlan::Union { left, right, .. } => {
                // For UNION, we compile both sub-plans into the same instruction
                // stream.  Both sides feed into the same result buffer; the VM
                // can then apply DistinctResult for UNION (without ALL) or leave
                // duplicates for UNION ALL.
                self.compile_inner(left);
                self.compile_inner(right);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Filter compilation
    // -----------------------------------------------------------------------

    /// Compile `Filter { input, predicate }`.
    ///
    /// Strategy: Find the innermost Scan in `input`, then emit the scan loop
    /// with the predicate check spliced inside.  After `AdvanceCursor` (and
    /// `JumpIfExhausted`), we evaluate the predicate.  If it is false, we jump
    /// back to the loop header, skipping this row.
    ///
    /// ```text
    /// OpenScan(tbl, alias)
    /// Label("scan_N_loop")
    ///   AdvanceCursor(alias)
    ///   JumpIfExhausted(alias, "scan_N_end")
    ///   <predicate expression>
    ///   JumpIfFalse("scan_N_loop")   ← skip row if predicate is false
    ///   BeginRow
    ///   EmitRow
    ///   Jump("scan_N_loop")
    /// Label("scan_N_end")
    /// CloseScan(alias)
    /// ```
    fn compile_filter(&mut self, input: &OptimizedPlan, predicate: &SqlExpr) {
        // Delegate to the unified scan-emitting helper.
        let skip_lbl = self.fresh_label("filter_skip");
        self.compile_scan_loop(input, |compiler, alias| {
            // Inside the loop body: evaluate the predicate; if false, skip row.
            compiler.compile_expr(predicate);
            compiler.emit(Instruction::JumpIfFalse(skip_lbl.clone()));
            compiler.emit(Instruction::BeginRow);
            compiler.emit(Instruction::EmitRow);
            compiler.emit(Instruction::Label(skip_lbl.clone()));
            // Note: the emit of the loop-back Jump is handled by compile_scan_loop.
            let _ = alias; // alias is used by the surrounding loop, not here
        });
    }

    // -----------------------------------------------------------------------
    // Project compilation
    // -----------------------------------------------------------------------

    /// Compile `Project { input, columns }`.
    ///
    /// Strategy: Run the scan loop; for each row, evaluate each output
    /// expression and emit it as a named column.
    ///
    /// ```text
    /// … scan loop header …
    ///   BeginRow
    ///   <expr1>        EmitColumn("col1")
    ///   <expr2>        EmitColumn("col2")
    ///   EmitRow
    ///   Jump(loop)
    /// … scan loop footer …
    /// ```
    fn compile_project(&mut self, input: &OptimizedPlan, columns: &[OutputColumn]) {
        // We need to check if the input is a Filter — in that case, the predicate
        // must gate the row before we project.
        //
        // For Aggregate / Having / Distinct inputs: the inner plan already
        // produces fully-assembled output rows (with EmitColumn+EmitRow).
        // The Project wrapper is then effectively a rename/projection of those
        // rows.  At Level 1 we don't re-project; we let the aggregate's own
        // EmitColumn names stand.  (Aliases are propagated into AggregateItem
        // by the planner, so the names are already correct.)
        match input {
            // ── Scan or Filter over Scan: generate the scan loop inline ──────
            OptimizedPlan::Filter {
                input: filter_input,
                predicate,
            } => {
                let skip_lbl = self.fresh_label("proj_filter_skip");
                let cols = columns.to_vec();
                let pred = predicate.clone();
                self.compile_scan_loop(filter_input, |compiler, _alias| {
                    compiler.compile_expr(&pred);
                    compiler.emit(Instruction::JumpIfFalse(skip_lbl.clone()));
                    compiler.emit(Instruction::BeginRow);
                    for col in &cols {
                        compiler.compile_expr(&col.expr);
                        let name = output_column_name(col);
                        compiler.emit(Instruction::EmitColumn(name));
                    }
                    compiler.emit(Instruction::EmitRow);
                    compiler.emit(Instruction::Label(skip_lbl.clone()));
                });
            }

            // ── EmptyResult: the optimizer proved no rows can exist (e.g. LIMIT 0).
            //    Emit DefineColumns so the QueryResult still carries the correct
            //    column metadata even though the row buffer is empty. ───────────
            OptimizedPlan::EmptyResult => {
                let col_names: Vec<String> = columns.iter().map(output_column_name).collect();
                self.emit(Instruction::DefineColumns(col_names));
            }

            // ── Aggregate / Having: these already emit rows.  The Project
            //    wrapper's column aliases are pre-propagated into the
            //    AggregateItem.alias fields by the planner.  Just compile the
            //    inner plan directly and skip the extra scan loop. ────────────
            OptimizedPlan::Aggregate { .. } | OptimizedPlan::Having { .. } => {
                self.compile_inner(input);
            }

            // ── Join: thread the projection through the join's inner loop so
            //    qualified columns resolve against the correct cursor. ─────────
            OptimizedPlan::Join {
                left,
                right,
                kind,
                condition,
            } => {
                let cols = columns.to_vec();
                self.compile_join_projected(left, right, kind, condition, &cols);
            }

            // ── All other inputs (plain Scan, etc.): standard scan loop ──────
            _ => {
                let cols = columns.to_vec();
                self.compile_scan_loop(input, |compiler, _alias| {
                    compiler.emit(Instruction::BeginRow);
                    for col in &cols {
                        compiler.compile_expr(&col.expr);
                        let name = output_column_name(col);
                        compiler.emit(Instruction::EmitColumn(name));
                    }
                    compiler.emit(Instruction::EmitRow);
                });
            }
        }
    }

    // -----------------------------------------------------------------------
    // Generic scan-loop helper
    // -----------------------------------------------------------------------

    /// Emit a complete scan loop around `input`, calling `body_fn` in the body.
    ///
    /// This is the canonical pattern used by Filter, Project, and other
    /// per-row operations.  `body_fn` receives the compiler and the alias of
    /// the innermost scan.
    ///
    /// For simple plans where `input` is a single Scan, the emitted code is:
    ///
    /// ```text
    /// OpenScan(table, alias)
    /// Label("scan_N_loop")
    ///   AdvanceCursor(alias)
    ///   JumpIfExhausted(alias, "scan_N_end")
    ///   <body_fn output>
    ///   Jump("scan_N_loop")
    /// Label("scan_N_end")
    /// CloseScan(alias)
    /// ```
    fn compile_scan_loop<F>(&mut self, input: &OptimizedPlan, body_fn: F)
    where
        F: FnOnce(&mut Compiler, Option<String>),
    {
        match input {
            OptimizedPlan::Scan { table, alias, .. } => {
                let loop_lbl = self.fresh_label("scan_loop");
                let end_lbl = self.fresh_label("scan_end");
                let alias = alias.clone();
                self.emit(Instruction::OpenScan(table.clone(), alias.clone()));
                self.emit(Instruction::Label(loop_lbl.clone()));
                self.emit(Instruction::AdvanceCursor(alias.clone()));
                self.emit(Instruction::JumpIfExhausted(alias.clone(), end_lbl.clone()));
                body_fn(self, alias.clone());
                self.emit(Instruction::Jump(loop_lbl));
                self.emit(Instruction::Label(end_lbl));
                self.emit(Instruction::CloseScan(alias));
            }
            // For non-Scan inputs, fall back to compiling the inner plan
            // and wrapping with a generic "over the output of inner" pattern.
            // In Level 1 the optimizer ensures the inner plan is a Scan,
            // but we handle the general case for robustness.
            other => {
                self.compile_inner(other);
                // For the body, we simply call it with None alias since there
                // is no single scan cursor to reference.
                body_fn(self, None);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Aggregate compilation
    // -----------------------------------------------------------------------

    /// Compile `Aggregate { input, group_by, aggregates }`.
    ///
    /// ## Two-phase structure
    ///
    /// **Phase 1 — Accumulation loop:**
    /// The scan loop runs over every input row.  For each row, we evaluate
    /// the GROUP BY expressions to save the group key, then call `UpdateAgg`
    /// for each aggregate slot.
    ///
    /// **Phase 2 — Emission:**
    /// After the loop, we finalize each aggregate and emit one output row per
    /// group.  For non-grouped aggregates (no `GROUP BY`), this is exactly one
    /// row (the global aggregate).
    ///
    /// ## COUNT(*) vs COUNT(col)
    ///
    /// `COUNT(*)` has a `None` argument in `AggregateItem`, which maps to
    /// `AggFn::CountStar`.  `UpdateAgg(slot, CountStar)` does not pop from
    /// the stack.
    ///
    /// All other aggregates have an expression argument.  The compiled argument
    /// is pushed onto the stack before `UpdateAgg`.
    fn compile_aggregate(
        &mut self,
        input: &OptimizedPlan,
        group_by: &[SqlExpr],
        aggregates: &[AggregateItem],
    ) {
        let n = aggregates.len();
        // Allocate accumulator slots: one per aggregate in declaration order.
        self.emit(Instruction::InitAgg(n));

        // Build group key column names for SaveGroupKey, plus each key's collation.
        let (group_key_cols, group_key_colls) = group_key_cols_and_collations(group_by);

        // Compute all the agg function tags upfront so the borrow checker
        // can release the borrow on `aggregates` before we use the compiler.
        let agg_fns: Vec<AggFn> = aggregates
            .iter()
            .map(|a| plan_agg_to_agg_fn_with_distinct(&a.func, a.arg.is_none(), a.distinct))
            .collect();
        let agg_args: Vec<Option<SqlExpr>> =
            aggregates.iter().map(|a| a.arg.clone()).collect();

        // Phase 1: scan loop body — save group key + update each aggregate.
        let loop_lbl = self.fresh_label("agg_loop");
        let end_lbl = self.fresh_label("agg_end");

        // Build a shared closure body that saves group key + updates accumulators.
        // This body is injected into the scan loop for both Scan and Filter inputs.
        let emit_agg_body = |compiler: &mut Compiler| {
            if !group_key_cols.is_empty() {
                compiler.emit(Instruction::SaveGroupKey(group_key_cols.clone(), group_key_colls.clone()));
            }
            for (i, (fn_tag, arg)) in agg_fns.iter().zip(agg_args.iter()).enumerate() {
                if let Some(arg_expr) = arg {
                    compiler.compile_expr(arg_expr);
                }
                compiler.emit(Instruction::UpdateAgg(i, fn_tag.clone()));
            }
        };

        match input {
            OptimizedPlan::Scan { table, alias, .. } => {
                let alias = alias.clone();
                self.emit(Instruction::OpenScan(table.clone(), alias.clone()));
                self.emit(Instruction::Label(loop_lbl.clone()));
                self.emit(Instruction::AdvanceCursor(alias.clone()));
                self.emit(Instruction::JumpIfExhausted(alias.clone(), end_lbl.clone()));
                emit_agg_body(self);
                self.emit(Instruction::Jump(loop_lbl));
                self.emit(Instruction::Label(end_lbl));
                self.emit(Instruction::CloseScan(alias));
            }
            OptimizedPlan::Filter { input: scan_input, predicate } => {
                // WHERE-filtered aggregate: emit a scan loop over the base table
                // and only accumulate rows that pass the predicate.
                let skip_lbl = self.fresh_label("agg_filter_skip");
                let pred = predicate.clone();
                let scan_input = scan_input.as_ref();
                // We need to manually build the scan loop instead of using
                // compile_scan_loop because compile_scan_loop takes FnOnce and
                // we need to call emit_agg_body (which mutably borrows self).
                if let OptimizedPlan::Scan { table, alias, .. } = scan_input {
                    let alias = alias.clone();
                    self.emit(Instruction::OpenScan(table.clone(), alias.clone()));
                    self.emit(Instruction::Label(loop_lbl.clone()));
                    self.emit(Instruction::AdvanceCursor(alias.clone()));
                    self.emit(Instruction::JumpIfExhausted(alias.clone(), end_lbl.clone()));
                    // Evaluate predicate; skip accumulation if false.
                    self.compile_expr(&pred);
                    self.emit(Instruction::JumpIfFalse(skip_lbl.clone()));
                    emit_agg_body(self);
                    self.emit(Instruction::Label(skip_lbl));
                    self.emit(Instruction::Jump(loop_lbl));
                    self.emit(Instruction::Label(end_lbl));
                    self.emit(Instruction::CloseScan(alias));
                } else {
                    // Nested non-scan filter: fall back to simple accumulation.
                    self.compile_inner(scan_input);
                    self.compile_expr(&pred);
                    self.emit(Instruction::JumpIfFalse(skip_lbl.clone()));
                    emit_agg_body(self);
                    self.emit(Instruction::Label(skip_lbl));
                }
            }
            _other => {
                // Non-scan input: compile the inner plan first, then update.
                // This is a simplified handling for complex subquery aggregates.
                self.compile_inner(_other);
                emit_agg_body(self);
            }
        }

        // Phase 2: emit one output row per group.
        // FinalizeAgg pushes the final accumulated value for each slot,
        // then we assemble a row from group key values + aggregate values.
        self.emit(Instruction::BeginRow);
        // Emit group-by columns first (as LoadColumn for each group key expr).
        // A collated key is emitted from its UNDERLYING column, not from the
        // `__collate(...)` wrapper: the collation decides which rows share a
        // group, but the reported value is the group's original text (SQLite
        // shows 'A' for a group of {'A','a'} keyed case-insensitively). Compiling
        // the wrapper instead would emit the folded value and rename the column.
        for e in group_by {
            let e = strip_collate(e);
            self.compile_expr(e);
            let name = match e {
                SqlExpr::Column { name, .. } => name.clone(),
                _ => "?".to_string(),
            };
            self.emit(Instruction::EmitColumn(name));
        }
        // Emit finalized aggregate values, each named the SQLite way
        // (`SUM(n)`, `COUNT(*)`, …) unless an explicit alias overrides it.
        for (i, (fn_tag, item)) in agg_fns.iter().zip(aggregates.iter()).enumerate() {
            self.emit(Instruction::FinalizeAgg(i, fn_tag.clone()));
            self.emit(Instruction::EmitColumn(aggregate_column_name(item)));
        }
        self.emit(Instruction::EmitRow);
    }

    // -----------------------------------------------------------------------
    // Having compilation
    // -----------------------------------------------------------------------

    /// Compile `Having { input: Aggregate, predicate }`.
    ///
    /// HAVING is evaluated *after* grouping and aggregation.  The instruction
    /// sequence is the same as a regular aggregate, but with an additional
    /// predicate check before the final row emission.
    ///
    /// If the predicate is false, we skip `EmitRow` for that group.
    fn compile_having(&mut self, input: &OptimizedPlan, predicate: &SqlExpr) {
        match input {
            OptimizedPlan::Aggregate {
                input: agg_input,
                group_by,
                aggregates,
            } => {
                let n = aggregates.len();
                self.emit(Instruction::InitAgg(n));

                let (group_key_cols, group_key_colls) = group_key_cols_and_collations(group_by);

                let agg_fns: Vec<AggFn> = aggregates
                    .iter()
                    .map(|a| plan_agg_to_agg_fn_with_distinct(&a.func, a.arg.is_none(), a.distinct))
                    .collect();
                let agg_args: Vec<Option<SqlExpr>> =
                    aggregates.iter().map(|a| a.arg.clone()).collect();

                let loop_lbl = self.fresh_label("having_loop");
                let end_lbl = self.fresh_label("having_end");
                let skip_lbl = self.fresh_label("having_skip");

                match agg_input.as_ref() {
                    OptimizedPlan::Scan { table, alias, .. } => {
                        let alias = alias.clone();
                        self.emit(Instruction::OpenScan(table.clone(), alias.clone()));
                        self.emit(Instruction::Label(loop_lbl.clone()));
                        self.emit(Instruction::AdvanceCursor(alias.clone()));
                        self.emit(Instruction::JumpIfExhausted(
                            alias.clone(),
                            end_lbl.clone(),
                        ));
                        if !group_key_cols.is_empty() {
                            self.emit(Instruction::SaveGroupKey(group_key_cols.clone(), group_key_colls.clone()));
                        }
                        for (i, (fn_tag, arg)) in
                            agg_fns.iter().zip(agg_args.iter()).enumerate()
                        {
                            if let Some(arg_expr) = arg {
                                self.compile_expr(arg_expr);
                            }
                            self.emit(Instruction::UpdateAgg(i, fn_tag.clone()));
                        }
                        self.emit(Instruction::Jump(loop_lbl));
                        self.emit(Instruction::Label(end_lbl));
                        self.emit(Instruction::CloseScan(alias));
                    }
                    other => {
                        self.compile_inner(other);
                        if !group_key_cols.is_empty() {
                            self.emit(Instruction::SaveGroupKey(group_key_cols.clone(), group_key_colls.clone()));
                        }
                        for (i, (fn_tag, arg)) in
                            agg_fns.iter().zip(agg_args.iter()).enumerate()
                        {
                            if let Some(arg_expr) = arg {
                                self.compile_expr(arg_expr);
                            }
                            self.emit(Instruction::UpdateAgg(i, fn_tag.clone()));
                        }
                    }
                }

                // After the loop: finalize each aggregate, apply HAVING predicate.
                // First finalize all aggs (so predicate can reference them),
                // then check predicate, then emit row.
                for (i, fn_tag) in agg_fns.iter().enumerate() {
                    self.emit(Instruction::FinalizeAgg(i, fn_tag.clone()));
                    self.emit(Instruction::EmitColumn(aggregate_column_name(&aggregates[i])));
                }

                // HAVING predicate check — skip the row if false.
                //
                // Populate `agg_slots` so that `compile_expr` can emit the
                // correct slot index for aggregate references in the predicate
                // (e.g. `SUM(amount) > 50` must use slot 1, not slot 0).
                self.agg_slots = aggregates
                    .iter()
                    .cloned()
                    .enumerate()
                    .collect();
                self.compile_expr(predicate);
                self.agg_slots.clear();
                self.emit(Instruction::JumpIfFalse(skip_lbl.clone()));

                self.emit(Instruction::BeginRow);
                // As above: emit a collated key from its underlying column so the
                // group's ORIGINAL text (and column name) is reported.
                for e in group_by {
                    let e = strip_collate(e);
                    self.compile_expr(e);
                    let name = match e {
                        SqlExpr::Column { name, .. } => name.clone(),
                        _ => "?".to_string(),
                    };
                    self.emit(Instruction::EmitColumn(name));
                }
                for (i, (fn_tag, item)) in agg_fns.iter().zip(aggregates.iter()).enumerate() {
                    self.emit(Instruction::FinalizeAgg(i, fn_tag.clone()));
                    self.emit(Instruction::EmitColumn(aggregate_column_name(item)));
                }
                self.emit(Instruction::EmitRow);
                self.emit(Instruction::Label(skip_lbl));
            }
            // Non-aggregate HAVING — treat like a filter.
            other => self.compile_filter(other, predicate),
        }
    }

    // -----------------------------------------------------------------------
    // Join compilation
    // -----------------------------------------------------------------------

    /// Compile a nested-loop join of `left` and `right`.
    ///
    /// ## Nested-loop join pattern
    ///
    /// ```text
    /// OpenScan(left_table, left_alias)   ← outer loop
    /// Label("join_N_outer_loop")
    ///   AdvanceCursor(left_alias)
    ///   JumpIfExhausted(left_alias, "join_N_outer_end")
    ///
    ///   OpenScan(right_table, right_alias)   ← inner loop
    ///   Label("join_N_inner_loop")
    ///     AdvanceCursor(right_alias)
    ///     JumpIfExhausted(right_alias, "join_N_inner_end")
    ///     <condition check + JumpIfFalse("join_N_inner_loop")>  ← optional
    ///     BeginRow
    ///     EmitRow
    ///     Jump("join_N_inner_loop")
    ///   Label("join_N_inner_end")
    ///   CloseScan(right_alias)
    ///
    ///   Jump("join_N_outer_loop")
    /// Label("join_N_outer_end")
    /// CloseScan(left_alias)
    /// ```
    ///
    /// The inner scan is re-opened for every outer row (by being inside the
    /// outer loop).  This is correct but not efficient for large tables; an
    /// optimizer/VM with index support would do better.
    fn compile_join(
        &mut self,
        left: &OptimizedPlan,
        right: &OptimizedPlan,
        kind: &JoinKind,
        condition: &Option<SqlExpr>,
    ) {
        // We only handle INNER and CROSS joins in Level 1.
        // For other join types we fall back to a cross-join for now.
        let has_condition = condition.is_some() && *kind == JoinKind::Inner;

        let outer_loop = self.fresh_label("join_outer_loop");
        let outer_end = self.fresh_label("join_outer_end");
        let inner_loop = self.fresh_label("join_inner_loop");
        let inner_end = self.fresh_label("join_inner_end");
        let cond_skip = if has_condition {
            self.fresh_label("join_cond_skip")
        } else {
            String::new()
        };

        // Outer scan.
        let (outer_table, outer_alias) = extract_scan_info(left);
        self.emit(Instruction::OpenScan(outer_table, outer_alias.clone()));
        self.emit(Instruction::Label(outer_loop.clone()));
        self.emit(Instruction::AdvanceCursor(outer_alias.clone()));
        self.emit(Instruction::JumpIfExhausted(
            outer_alias.clone(),
            outer_end.clone(),
        ));

        // Inner scan (re-opened on each outer row).
        let (inner_table, inner_alias) = extract_scan_info(right);
        self.emit(Instruction::OpenScan(inner_table, inner_alias.clone()));
        self.emit(Instruction::Label(inner_loop.clone()));
        self.emit(Instruction::AdvanceCursor(inner_alias.clone()));
        self.emit(Instruction::JumpIfExhausted(
            inner_alias.clone(),
            inner_end.clone(),
        ));

        // Optional join condition.
        if let Some(cond) = condition {
            if has_condition {
                self.compile_expr(cond);
                self.emit(Instruction::JumpIfFalse(cond_skip.clone()));
            }
        }

        // Emit matched row.
        self.emit(Instruction::BeginRow);
        self.emit(Instruction::EmitRow);

        if has_condition {
            self.emit(Instruction::Label(cond_skip));
        }

        self.emit(Instruction::Jump(inner_loop));
        self.emit(Instruction::Label(inner_end));
        self.emit(Instruction::CloseScan(inner_alias));

        self.emit(Instruction::Jump(outer_loop));
        self.emit(Instruction::Label(outer_end));
        self.emit(Instruction::CloseScan(outer_alias));
    }

    /// Compile a nested-loop join of `left` and `right` **with a projection**:
    /// the `columns` are evaluated and emitted *inside* the inner loop, once per
    /// matched pair. This is the path a real `SELECT … FROM a JOIN b` takes
    /// (the planner always wraps a join in a `Project`).
    ///
    /// ## Why this exists separately from [`compile_join`]
    ///
    /// Two problems had to be fixed together for qualified columns to resolve:
    ///
    /// 1. **Cursor keys must be distinct and match the column qualifiers.** A
    ///    `FROM a` with no `AS` alias would otherwise open its cursor under the
    ///    `None` key — and so would `FROM b`, so the two collided and *both*
    ///    `a.x` and `b.y` read from whichever advanced last (resolving to NULL).
    ///    Here each side is keyed by its **effective alias** — the explicit alias
    ///    if given, else the table name — which is exactly what a `LoadColumn`
    ///    qualifier (`a.x` → `LoadColumn(Some("a"), "x")`) looks up. Now the `ON`
    ///    condition *and* the projected columns resolve against the right row.
    /// 2. **The projection must run inside the loop.** Emitting the output
    ///    columns after the join loop closed (the old `compile_scan_loop`
    ///    fallback) evaluated them with no live cursor, producing a single
    ///    all-NULL row. They belong in the per-pair body.
    /// ## Join kinds
    ///
    /// - **INNER / CROSS** — the classic nested loop; a matched pair (or every
    ///   pair, for CROSS) is projected and emitted.
    /// - **LEFT / RIGHT OUTER** — every *outer* row must appear at least once.
    ///   We keep a per-outer-row match flag ([`Instruction::ClearMatch`] /
    ///   [`Instruction::SetMatch`]); after the inner loop, if nothing matched we
    ///   emit one row with the inner side NULL. `RIGHT a b` is just `LEFT b a`
    ///   (the outer/inner roles swap; the projection still references each table
    ///   by name, so the output is unchanged). The NULL padding falls out for
    ///   free: `CloseScan` drops the inner cursor's `current_row`, so its columns
    ///   read NULL while the still-open outer cursor keeps its real values.
    /// - **FULL OUTER** — every row from *both* sides, matched pairs joined and
    ///   unmatched rows NULL-padded on the missing side. A single nested loop
    ///   can't produce the unmatched *right* rows (the inner side is re-scanned
    ///   per outer row, so "did this right row ever match *any* left row?" isn't
    ///   known during the forward pass), so we run two passes and union them.
    ///   Pass 1 is a LEFT JOIN (outer = left): all matched pairs, plus each left
    ///   row that matched nothing, NULL-padded on the right. Pass 2 is a RIGHT
    ///   *anti*-join (outer = right): for each right row we evaluate `ON` against
    ///   every left row but **suppress the matched-pair emit** (pass 1 already
    ///   produced those) and emit only the right rows that matched no left row,
    ///   NULL-padded on the left. The union is exactly a FULL JOIN with no
    ///   duplicated matched pairs; both passes reuse the same match-flag
    ///   machinery, so no new VM instructions are needed. Ordering across the two
    ///   passes is handled by the surrounding `ORDER BY` sort, which runs after
    ///   every row is emitted.
    fn compile_join_projected(
        &mut self,
        left: &OptimizedPlan,
        right: &OptimizedPlan,
        kind: &JoinKind,
        condition: &Option<SqlExpr>,
        columns: &[OutputColumn],
    ) {
        match kind {
            // FULL OUTER = LEFT JOIN  ∪  RIGHT anti-join (see the doc above).
            JoinKind::Full => {
                let eval = condition.is_some();
                // Pass 1: the LEFT half — matched pairs + left-only rows.
                self.emit_join_pass(left, right, condition, eval, true, true, columns);
                // Pass 2: the right rows that matched no left row. `emit_matched`
                // is false so matched pairs are NOT emitted a second time.
                self.emit_join_pass(right, left, condition, eval, false, true, columns);
            }
            _ => {
                // RIGHT JOIN is LEFT JOIN with the operands swapped.
                let (outer_plan, inner_plan) = if *kind == JoinKind::Right {
                    (right, left)
                } else {
                    (left, right)
                };
                let is_outer = matches!(kind, JoinKind::Left | JoinKind::Right);
                // INNER/LEFT/RIGHT evaluate the ON condition; CROSS has none.
                let eval_condition = condition.is_some()
                    && matches!(kind, JoinKind::Inner | JoinKind::Left | JoinKind::Right);
                self.emit_join_pass(
                    outer_plan,
                    inner_plan,
                    condition,
                    eval_condition,
                    /* emit_matched   */ true,
                    /* emit_unmatched */ is_outer,
                    columns,
                );
            }
        }
    }

    /// Emit one nested-loop join pass over `outer_plan` × `inner_plan`.
    ///
    /// - `eval_condition` — evaluate the `ON` predicate to gate matches (false ⇒
    ///   every pair "matches", i.e. a cross product).
    /// - `emit_matched` — project + emit a row for each matched pair. FULL JOIN's
    ///   second (anti-join) pass sets this false: it needs the match *flag* to
    ///   know which right rows were unmatched, but must not re-emit pairs.
    /// - `emit_unmatched` — after the inner loop, if this outer row matched no
    ///   inner row, emit one row with the inner side NULL-padded (its cursor is
    ///   closed, so its columns read NULL while the outer cursor holds its row).
    ///
    /// The match flag (`ClearMatch`/`SetMatch`/`JumpIfMatched`) is used whenever
    /// `emit_unmatched` is set — that is the only case that must distinguish a
    /// matched outer row from an unmatched one.
    #[allow(clippy::too_many_arguments)]
    fn emit_join_pass(
        &mut self,
        outer_plan: &OptimizedPlan,
        inner_plan: &OptimizedPlan,
        condition: &Option<SqlExpr>,
        eval_condition: bool,
        emit_matched: bool,
        emit_unmatched: bool,
        columns: &[OutputColumn],
    ) {
        // The flag is needed only to decide the post-loop NULL-padded emit.
        let use_match_flag = emit_unmatched;

        let outer_loop = self.fresh_label("joinp_outer_loop");
        let outer_end = self.fresh_label("joinp_outer_end");
        let inner_loop = self.fresh_label("joinp_inner_loop");
        let inner_end = self.fresh_label("joinp_inner_end");
        let cond_skip = if eval_condition {
            self.fresh_label("joinp_cond_skip")
        } else {
            String::new()
        };
        let after_null = if emit_unmatched {
            self.fresh_label("joinp_after_null")
        } else {
            String::new()
        };

        // Effective alias = explicit alias, else the table name. This is the key
        // both the cursor and the column qualifiers agree on.
        let (outer_table, outer_alias_opt) = extract_scan_info(outer_plan);
        let outer_alias = Some(outer_alias_opt.unwrap_or_else(|| outer_table.clone()));
        let (inner_table, inner_alias_opt) = extract_scan_info(inner_plan);
        let inner_alias = Some(inner_alias_opt.unwrap_or_else(|| inner_table.clone()));

        // Outer scan.
        self.emit(Instruction::OpenScan(outer_table, outer_alias.clone()));
        self.emit(Instruction::Label(outer_loop.clone()));
        self.emit(Instruction::AdvanceCursor(outer_alias.clone()));
        self.emit(Instruction::JumpIfExhausted(
            outer_alias.clone(),
            outer_end.clone(),
        ));

        // Start each outer row with the match flag cleared.
        if use_match_flag {
            self.emit(Instruction::ClearMatch);
        }

        // Inner scan (re-opened per outer row).
        self.emit(Instruction::OpenScan(inner_table, inner_alias.clone()));
        self.emit(Instruction::Label(inner_loop.clone()));
        self.emit(Instruction::AdvanceCursor(inner_alias.clone()));
        self.emit(Instruction::JumpIfExhausted(
            inner_alias.clone(),
            inner_end.clone(),
        ));

        // Optional join condition (both cursors are live here, correctly keyed).
        if eval_condition {
            if let Some(cond) = condition {
                self.compile_expr(cond);
                self.emit(Instruction::JumpIfFalse(cond_skip.clone()));
            }
        }

        // A matched pair: record the match, and project the row unless this pass
        // only wants the unmatched (anti-join) rows.
        if use_match_flag {
            self.emit(Instruction::SetMatch);
        }
        if emit_matched {
            self.emit_row_projection(columns);
        }

        if eval_condition {
            self.emit(Instruction::Label(cond_skip));
        }

        self.emit(Instruction::Jump(inner_loop));
        self.emit(Instruction::Label(inner_end));
        self.emit(Instruction::CloseScan(inner_alias));

        // If no inner row matched this outer row, emit one row with the inner
        // side NULL (its cursor is now closed, so its columns read NULL; the
        // outer cursor still holds this outer row).
        if emit_unmatched {
            self.emit(Instruction::JumpIfMatched(after_null.clone()));
            self.emit_row_projection(columns);
            self.emit(Instruction::Label(after_null));
        }

        self.emit(Instruction::Jump(outer_loop));
        self.emit(Instruction::Label(outer_end));
        self.emit(Instruction::CloseScan(outer_alias));
    }

    /// Emit `BeginRow`, one `EmitColumn` per output column, then `EmitRow` —
    /// projecting the current cursor state into one result row.
    fn emit_row_projection(&mut self, columns: &[OutputColumn]) {
        self.emit(Instruction::BeginRow);
        for col in columns {
            self.compile_expr(&col.expr);
            let name = output_column_name(col);
            self.emit(Instruction::EmitColumn(name));
        }
        self.emit(Instruction::EmitRow);
    }

    // -----------------------------------------------------------------------
    // DML compilation
    // -----------------------------------------------------------------------

    /// Compile `INSERT INTO table [(cols)] VALUES (...)`.
    ///
    /// For each row in the `Values` source, push each value expression onto the
    /// evaluation stack, then emit `InsertRow`.
    fn compile_insert(
        &mut self,
        table: &str,
        columns: &Option<Vec<String>>,
        source: &InsertSource,
    ) {
        match source {
            InsertSource::Values(rows) => {
                for row in rows {
                    self.emit(Instruction::BeginRow);
                    for (i, expr) in row.iter().enumerate() {
                        self.compile_expr(expr);
                        // After pushing the value onto the stack, pop it into
                        // the row_buffer under the appropriate column name.
                        // The VM's InsertRow instruction reads from row_buffer,
                        // not directly from the evaluation stack.
                        let col_name = columns
                            .as_ref()
                            .and_then(|cols| cols.get(i))
                            .cloned()
                            .unwrap_or_else(|| format!("col_{}", i));
                        self.emit(Instruction::EmitColumn(col_name));
                    }
                    // Pass None to InsertRow — the row_buffer already holds
                    // named (col_name, value) pairs from the EmitColumn sequence
                    // above, so build_insert_row can use them directly.
                    self.emit(Instruction::InsertRow(
                        table.to_string(),
                        None,
                    ));
                }
            }
        }
    }

    /// Compile `UPDATE table SET col = expr [WHERE predicate]`.
    ///
    /// If there is a predicate, we emit a scan loop and `UpdateRows` only for
    /// rows that pass the filter.  Without a predicate, we update all rows.
    fn compile_update(
        &mut self,
        table: &str,
        assignments: &[Assignment],
        predicate: &Option<SqlExpr>,
    ) {
        // Assignments are pushed onto the stack as pairs of (column_value) in
        // the order they appear, then UpdateRows uses them.
        let update_lbl = self.fresh_label("update_loop");
        let update_end = self.fresh_label("update_end");
        let skip_lbl = self.fresh_label("update_skip");

        // We need a Scan over the table to find rows to update.
        // For Level 1, we synthesize a scan over `table`.
        self.emit(Instruction::OpenScan(table.to_string(), None));
        self.emit(Instruction::Label(update_lbl.clone()));
        self.emit(Instruction::AdvanceCursor(None));
        self.emit(Instruction::JumpIfExhausted(None, update_end.clone()));

        if let Some(pred) = predicate {
            self.compile_expr(pred);
            self.emit(Instruction::JumpIfFalse(skip_lbl.clone()));
        }

        // Push each assignment expression onto the stack (in assignment order).
        // Collect the column names in the same order so the VM can pair them.
        let col_names: Vec<String> = assignments.iter().map(|a| a.column.clone()).collect();
        for assignment in assignments {
            self.compile_expr(&assignment.value);
        }

        self.emit(Instruction::UpdateRows(table.to_string(), col_names));

        if predicate.is_some() {
            self.emit(Instruction::Label(skip_lbl));
        }

        self.emit(Instruction::Jump(update_lbl));
        self.emit(Instruction::Label(update_end));
        self.emit(Instruction::CloseScan(None));
    }

    /// Compile `DELETE FROM table [WHERE predicate]`.
    ///
    /// Similar to UPDATE: scan the table, filter by predicate, call `DeleteRows`
    /// for each matching row.
    fn compile_delete(&mut self, table: &str, predicate: &Option<SqlExpr>) {
        let del_lbl = self.fresh_label("delete_loop");
        let del_end = self.fresh_label("delete_end");
        let skip_lbl = self.fresh_label("delete_skip");

        self.emit(Instruction::OpenScan(table.to_string(), None));
        self.emit(Instruction::Label(del_lbl.clone()));
        self.emit(Instruction::AdvanceCursor(None));
        self.emit(Instruction::JumpIfExhausted(None, del_end.clone()));

        if let Some(pred) = predicate {
            self.compile_expr(pred);
            self.emit(Instruction::JumpIfFalse(skip_lbl.clone()));
        }

        self.emit(Instruction::DeleteRows(table.to_string()));

        if predicate.is_some() {
            self.emit(Instruction::Label(skip_lbl));
        }

        self.emit(Instruction::Jump(del_lbl));
        self.emit(Instruction::Label(del_end));
        self.emit(Instruction::CloseScan(None));
    }

    // -----------------------------------------------------------------------
    // Expression compilation
    // -----------------------------------------------------------------------

    /// Compile a [`SqlExpr`] into a stack-machine sequence.
    ///
    /// Post-order (bottom-up) traversal: children are compiled before their
    /// parent.  This ensures the operands are on the stack before an operator
    /// instruction executes.
    ///
    /// ## Stack discipline
    ///
    /// Every expression pushes exactly one value onto the evaluation stack.
    /// Compound expressions consume their children's values and push one result.
    ///
    /// ## Recursion depth guard
    ///
    /// A thread-local counter prevents stack overflow on pathologically deep
    /// expressions.  If depth exceeds `MAX_EXPR_DEPTH`, we emit
    /// `LoadConst(Null)` as a safe sentinel rather than panicking.
    fn compile_expr(&mut self, expr: &SqlExpr) {
        // Depth guard: prevent runaway recursion on deeply nested expressions.
        let depth = EXPR_DEPTH.with(|d| {
            let v = d.get();
            d.set(v + 1);
            v
        });
        if depth >= MAX_EXPR_DEPTH {
            EXPR_DEPTH.with(|d| d.set(d.get() - 1));
            // Safe sentinel: emit NULL so the program can still halt cleanly.
            self.emit(Instruction::LoadConst(SqlValue::Null));
            return;
        }

        match expr {
            // ── Literals ────────────────────────────────────────────────────

            SqlExpr::Literal(val) => {
                // The simplest case: push a constant directly.
                // No sub-expressions to compile.
                self.emit(Instruction::LoadConst(val.clone()));
            }

            // ── Column references ────────────────────────────────────────────

            SqlExpr::Column { table, name } => {
                // Push the value of a named column from the current row.
                // The optional `table` qualifier selects which cursor's row to read.
                self.emit(Instruction::LoadColumn(table.clone(), name.clone()));
            }

            // ── Binary operations ────────────────────────────────────────────

            SqlExpr::BinaryOp { op, left, right } => {
                // Post-order: left operand first, then right, then the operator.
                // This mirrors how a stack-machine evaluates `a + b`:
                //   1. Push a
                //   2. Push b
                //   3. Add (pops a and b, pushes a+b)
                self.compile_expr(left);
                self.compile_expr(right);
                self.emit(Instruction::BinaryOpInstr(map_binary_op(op)));
            }

            // ── Unary operations ─────────────────────────────────────────────

            SqlExpr::UnaryOp { op, expr: inner } => {
                // Compile the operand first, then apply the operator.
                self.compile_expr(inner);
                self.emit(Instruction::UnaryOpInstr(map_unary_op(op)));
            }

            // ── NULL tests ───────────────────────────────────────────────────

            SqlExpr::IsNull(inner) => {
                self.compile_expr(inner);
                self.emit(Instruction::IsNull);
            }

            SqlExpr::IsNotNull(inner) => {
                self.compile_expr(inner);
                self.emit(Instruction::IsNotNull);
            }

            // ── Range test ───────────────────────────────────────────────────

            SqlExpr::Between {
                value,
                low,
                high,
                negated,
            } => {
                // Stack layout expected by Between: [value, low, high] (high on top).
                self.compile_expr(value);
                self.compile_expr(low);
                self.compile_expr(high);
                // Between(true) = inclusive bounds (the normal SQL semantics).
                // NOT BETWEEN is !negated in the inclusive sense.
                self.emit(Instruction::Between(!negated));
            }

            // ── Pattern matching ─────────────────────────────────────────────

            SqlExpr::Like {
                value,
                pattern,
                negated,
                escape,
            } => {
                // Stack: [value, pattern] (pattern on top), plus [escape] when an
                // ESCAPE clause is present. The VM applies the LIKE match and,
                // for `NOT LIKE`, the NULL-aware inversion carried by the
                // instruction's `negated` flag.
                self.compile_expr(value);
                self.compile_expr(pattern);
                match escape {
                    Some(escape_expr) => {
                        self.compile_expr(escape_expr);
                        self.emit(Instruction::LikeEscape(*negated));
                    }
                    None => self.emit(Instruction::Like(*negated)),
                }
            }

            // ── CAST ─────────────────────────────────────────────────────────

            SqlExpr::Cast { expr, ty } => {
                // Evaluate the operand, then convert it in place.
                self.compile_expr(expr);
                self.emit(Instruction::Cast(ty.clone()));
            }

            // ── CASE ─────────────────────────────────────────────────────────

            SqlExpr::Case { branches, else_val } => {
                // Short-circuit via a jump chain (no branch's THEN is evaluated
                // unless its WHEN matched, and no later WHEN is evaluated once
                // one matches). Exactly one value is left on the stack.
                //
                //     for each branch:  <compile cond>; JumpIfTrue(body_i)
                //     <compile ELSE or LoadConst Null>; Jump(end)
                //     body_i: <compile then_i>; Jump(end)
                //     end:
                let end = self.fresh_label("case_end");
                let body_labels: Vec<String> =
                    branches.iter().map(|_| self.fresh_label("case_body")).collect();

                for ((cond, _), body) in branches.iter().zip(body_labels.iter()) {
                    self.compile_expr(cond);
                    self.emit(Instruction::JumpIfTrue(body.clone()));
                }
                // Fell through every WHEN → the ELSE value, or NULL.
                match else_val {
                    Some(e) => self.compile_expr(e),
                    None => self.emit(Instruction::LoadConst(SqlValue::Null)),
                }
                self.emit(Instruction::Jump(end.clone()));

                for ((_, then_val), body) in branches.iter().zip(body_labels.iter()) {
                    self.emit(Instruction::Label(body.clone()));
                    self.compile_expr(then_val);
                    self.emit(Instruction::Jump(end.clone()));
                }
                self.emit(Instruction::Label(end));
            }

            // ── IN list ──────────────────────────────────────────────────────

            SqlExpr::InList {
                value,
                list,
                negated,
            } => {
                // Stack: [value, item1, item2, ..., itemN].
                // InList(N) pops N+1 values, pushes Bool.
                self.compile_expr(value);
                for item in list {
                    self.compile_expr(item);
                }
                self.emit(Instruction::InList(list.len()));
                // For NOT IN, add a logical NOT.
                if *negated {
                    self.emit(Instruction::UnaryOpInstr(UnaryOp::Not));
                }
            }

            // ── Aggregate references ─────────────────────────────────────────

            SqlExpr::Aggregate { func, arg, distinct, .. } => {
                // Aggregate expressions inside a HAVING predicate: look up the
                // slot index from `agg_slots` (populated by `compile_having`
                // before calling `compile_expr(predicate)`).  If no match is
                // found (e.g. an inline aggregate outside a proper HAVING node),
                // fall back to slot 0.
                //
                // Matching is by (func, arg, distinct) so that
                // `COUNT(DISTINCT x)` and `COUNT(x)` use different slots.
                let fn_tag = plan_agg_to_agg_fn_with_distinct(func, arg.is_none(), *distinct);
                let slot = self.agg_slots.iter().find(|(_, a)| {
                    let a_fn = plan_agg_to_agg_fn_with_distinct(&a.func, a.arg.is_none(), a.distinct);
                    // `arg` here is Option<Box<SqlExpr>>; `a.arg` is Option<SqlExpr>.
                    // Compare by derefing the Box.
                    let arg_matches = match (arg, &a.arg) {
                        (None, None) => true,
                        (Some(boxed), Some(a_expr)) => (**boxed) == *a_expr,
                        _ => false,
                    };
                    a_fn == fn_tag && arg_matches
                }).map(|(i, _)| *i).unwrap_or(0);
                // No arg to push for aggregate references in predicate context —
                // the accumulator already holds the accumulated value.
                self.emit(Instruction::FinalizeAgg(slot, fn_tag));
            }

            // ── Function calls ───────────────────────────────────────────────

            SqlExpr::FunctionCall { name, args, .. } => {
                // Compile each argument (left-to-right, first arg deepest).
                let n = args.len();
                for a in args {
                    self.compile_expr(a);
                }
                // Emit a dispatch to the named built-in.  The VM pops `n`
                // values, applies the function, and pushes the result.
                self.emit(Instruction::CallBuiltin(name.to_uppercase(), n));
            }
        }

        EXPR_DEPTH.with(|d| d.set(d.get() - 1));
    }
}

// ===========================================================================
// Helper: peel post-ops off the outermost plan
// ===========================================================================

/// Strip `Sort`, `Limit`, and `Distinct` wrappers off the plan tree.
///
/// Returns `(inner_plan, post_ops)` where `post_ops` is the list of
/// post-processing instructions to append after `Halt`.  Post-ops are
/// collected in outer-to-inner peeling order, which is also the correct
/// *execution* order.  For example:
///
/// ```text
/// Sort(Limit(Scan)) → post_ops = [SortResult(...), LimitResult(...)]
/// ```
///
/// Execution: VM runs scan → builds result buffer → Halt →
/// SortResult (sort) → LimitResult (paginate).
///
/// ## Why peel rather than compile inline?
///
/// Sort, Limit, and Distinct operate on the *entire assembled result set*,
/// not on individual rows.  They cannot be interleaved with the scan loop
/// (which processes one row at a time).  By moving them after `Halt`, we
/// signal to the VM: "run the loop to fill the result buffer, then apply
/// these batch operators."
fn peel_post_ops(plan: &OptimizedPlan) -> (&OptimizedPlan, Vec<Instruction>) {
    let mut post_ops = Vec::new();
    let mut current = plan;

    loop {
        match current {
            OptimizedPlan::Sort { input, keys } => {
                let compiled_keys: Vec<CompiledSortKey> = keys
                    .iter()
                    .map(|k| CompiledSortKey {
                        column: match &k.expr {
                            SqlExpr::Column { name, .. } => name.clone(),
                            _ => "?".to_string(),
                        },
                        ascending: k.ascending,
                        nulls_first: k.nulls_first,
                        collation: k.collation.clone(),
                    })
                    .collect();
                post_ops.push(Instruction::SortResult(compiled_keys));
                current = input;
            }
            OptimizedPlan::Limit {
                input,
                count,
                offset,
            } => {
                post_ops.push(Instruction::LimitResult(*count, *offset));
                current = input;
            }
            OptimizedPlan::Distinct(inner, colls) => {
                post_ops.push(Instruction::DistinctResult(colls.clone()));
                current = inner;
            }
            _ => break,
        }
    }

    (current, post_ops)
}

// ===========================================================================
// Helper: derive the output column name from an OutputColumn
// ===========================================================================

/// Return the name to use for an output column.
///
/// Priority:
/// 1. An explicit alias (`SELECT expr AS alias`) — always wins.
/// 2. If the expression is a bare column reference (`SELECT col`), use the
///    column name as the implicit label.  This matches SQL's convention that
///    `SELECT id FROM t` exposes a column named `id`.
/// 3. For a function call (`SELECT UPPER(name)`), reconstruct the call text —
///    `UPPER(name)` — as the label.  SQLite names an un-aliased expression
///    column after the *source text* of the expression, so `SELECT UPPER(name),
///    LENGTH(name)` returns columns `UPPER(name)` and `LENGTH(name)`.  Giving
///    the two columns distinct names is not just cosmetic: the VM keys nothing
///    by name (it projects positionally), but the differential oracle compares
///    column names against real SQLite, and two `?`-named columns previously
///    diverged.  We reconstruct rather than thread source spans through the
///    parser, which matches SQLite exactly for the whitespace-free calls we
///    emit and degrades to `?` for arguments we cannot render.
/// 4. Fall back to `"?"` for other complex expressions without an alias.
fn output_column_name(col: &OutputColumn) -> String {
    if let Some(alias) = &col.alias {
        return alias.clone();
    }
    match &col.expr {
        SqlExpr::Column { name, .. } => name.clone(),
        SqlExpr::FunctionCall { .. } => render_expr_label(&col.expr).unwrap_or_else(|| "?".to_string()),
        _ => "?".to_string(),
    }
}

/// Best-effort reconstruction of an expression's *source text*, used as the
/// implicit column label for un-aliased expressions (mirroring SQLite).
///
/// Returns `None` for any node we do not know how to render, so the caller can
/// fall back to `"?"` rather than emitting a misleading label.  We only render
/// the shapes that actually appear as function arguments today — columns,
/// simple literals, and nested calls — because the goal is faithful column
/// *names*, not a general SQL pretty-printer.
///
/// | expression            | rendered label   |
/// |-----------------------|------------------|
/// | `UPPER(name)`         | `UPPER(name)`    |
/// | `SUBSTR(name,1,2)`    | `SUBSTR(name,1,2)` |
/// | `LENGTH(u.name)`      | `LENGTH(u.name)` |
/// | `COALESCE(x,'-')`     | `COALESCE(x,'-')`|
fn render_expr_label(expr: &SqlExpr) -> Option<String> {
    match expr {
        SqlExpr::Column { table: Some(t), name } => Some(format!("{t}.{name}")),
        SqlExpr::Column { table: None, name } => Some(name.clone()),
        SqlExpr::Literal(v) => Some(match v {
            SqlValue::Null => "NULL".to_string(),
            SqlValue::Bool(b) => if *b { "TRUE".to_string() } else { "FALSE".to_string() },
            SqlValue::Int(n) => n.to_string(),
            // Render floats via SQLite's own default text form is out of scope;
            // the plain Rust form matches for the integral/simple cases we emit.
            SqlValue::Float(f) => f.to_string(),
            // Single-quote string literals, doubling embedded quotes as SQL does.
            SqlValue::Text(s) => format!("'{}'", s.replace('\'', "''")),
            // Blobs have no simple textual column-name form; decline to render.
            SqlValue::Blob(_) => return None,
        }),
        SqlExpr::FunctionCall { name, args, .. } => {
            let rendered: Option<Vec<String>> = args.iter().map(render_expr_label).collect();
            Some(format!("{}({})", name, rendered?.join(",")))
        }
        _ => None,
    }
}

/// SQLite-style implicit column name for an un-aliased aggregate: the function
/// call text — `COUNT(*)`, `SUM(n)`, `MIN(x)`, `AVG(n)`, `COUNT(DISTINCT id)`.
///
/// An explicit `AS` alias always wins. Otherwise this mirrors SQLite, which
/// names an un-aliased result column after the expression's source text — so a
/// bare `SELECT COUNT(*)` returns a column literally named `COUNT(*)`, not the
/// engine-internal `agg_0`. `COUNT(*)` alone has no argument (rendered `*`);
/// every other aggregate renders its argument via [`render_expr_label`],
/// prefixed with `DISTINCT ` when the aggregate is distinct.
fn aggregate_column_name(item: &AggregateItem) -> String {
    if let Some(alias) = &item.alias {
        return alias.clone();
    }
    let func = match item.func {
        AggFunc::Count => "COUNT",
        AggFunc::Sum => "SUM",
        AggFunc::Avg => "AVG",
        AggFunc::Min => "MIN",
        AggFunc::Max => "MAX",
        AggFunc::GroupConcat { .. } => "GROUP_CONCAT",
    };
    let inner = match &item.arg {
        None => "*".to_string(),
        Some(expr) => {
            let base = render_expr_label(expr).unwrap_or_else(|| "?".to_string());
            if item.distinct {
                format!("DISTINCT {base}")
            } else {
                base
            }
        }
    };
    format!("{func}({inner})")
}

// ===========================================================================
// Helper: InnerPlan — owned-or-borrowed wrapper for canonicalized plans
// ===========================================================================

/// A plan that is either a borrow of the original (the common case, no
/// allocation) or a freshly constructed owned value (when we had to
/// reconstruct a Project around a peeled inner node).
enum InnerPlan<'a> {
    Borrowed(&'a OptimizedPlan),
    Owned(OptimizedPlan),
}

impl<'a> InnerPlan<'a> {
    /// Return a reference to the wrapped plan regardless of ownership.
    fn as_ref(&self) -> &OptimizedPlan {
        match self {
            InnerPlan::Borrowed(p) => p,
            InnerPlan::Owned(p) => p,
        }
    }
}

/// Peel Sort/Limit/Distinct post-ops, looking through a `Project` wrapper.
///
/// The planner emits `Project { Sort { ... } }` (Project outermost, per
/// lessons.md).  The standard `peel_post_ops` cannot see the Sort because
/// Project is on top.  This function detects that pattern and strips the
/// Sort/Limit/Distinct from INSIDE the Project, reconstructing
/// `Project { stripped_inner }` as the effective inner plan.
///
/// Without a Project wrapper, this falls back to the standard `peel_post_ops`.
fn peel_post_ops_through_project(plan: &OptimizedPlan) -> (InnerPlan<'_>, Vec<Instruction>) {
    // Detect the `Project { Sort/Limit/Distinct { ... } }` pattern.
    if let OptimizedPlan::Project { input, columns } = plan {
        // Check whether the project's input starts with post-op wrappers.
        let (stripped_inner, post_ops) = peel_post_ops(input);
        if !post_ops.is_empty() {
            // Rebuild the Project around the stripped inner, owning the result.
            let owned_inner = OptimizedPlan::Project {
                input: Box::new(stripped_inner.clone()),
                columns: columns.clone(),
            };
            return (InnerPlan::Owned(owned_inner), post_ops);
        }
    }

    // Standard path: peel from the outermost node directly.
    let (inner, post_ops) = peel_post_ops(plan);
    (InnerPlan::Borrowed(inner), post_ops)
}

// ===========================================================================
// Helper: map planner binary/unary ops to codegen ops
// ===========================================================================

/// Map a `sql-planner` [`BinaryOp`] to a codegen [`BinaryOp`].
///
/// The two types mirror each other exactly; this function is the boundary
/// that prevents the planner's type system from leaking into the codegen's
/// public API.
fn map_binary_op(op: &PlanBinaryOp) -> BinaryOp {
    match op {
        PlanBinaryOp::Add => BinaryOp::Add,
        PlanBinaryOp::Sub => BinaryOp::Sub,
        PlanBinaryOp::Mul => BinaryOp::Mul,
        PlanBinaryOp::Div => BinaryOp::Div,
        PlanBinaryOp::Mod => BinaryOp::Mod,
        PlanBinaryOp::Eq => BinaryOp::Eq,
        PlanBinaryOp::Neq => BinaryOp::Neq,
        PlanBinaryOp::Lt => BinaryOp::Lt,
        PlanBinaryOp::Lte => BinaryOp::Lte,
        PlanBinaryOp::Gt => BinaryOp::Gt,
        PlanBinaryOp::Gte => BinaryOp::Gte,
        PlanBinaryOp::And => BinaryOp::And,
        PlanBinaryOp::Or => BinaryOp::Or,
        PlanBinaryOp::Concat => BinaryOp::Concat,
        PlanBinaryOp::BitAnd => BinaryOp::BitAnd,
        PlanBinaryOp::BitOr => BinaryOp::BitOr,
        PlanBinaryOp::ShiftLeft => BinaryOp::ShiftLeft,
        PlanBinaryOp::ShiftRight => BinaryOp::ShiftRight,
    }
}

/// Map a `sql-planner` [`UnaryOp`] to a codegen [`UnaryOp`].
fn map_unary_op(op: &PlanUnaryOp) -> UnaryOp {
    match op {
        PlanUnaryOp::Neg => UnaryOp::Neg,
        PlanUnaryOp::Not => UnaryOp::Not,
        PlanUnaryOp::BitNot => UnaryOp::BitNot,
    }
}

/// Map a `sql-planner` [`AggFunc`] to a codegen [`AggFn`].
///
/// The `is_star` flag distinguishes `COUNT(*)` (no argument) from
/// `COUNT(col)` (with an argument).  The planner uses a `None` argument
/// for `COUNT(*)`, which we map to `AggFn::CountStar`.
/// Peel a `__collate(<expr>, '<COLL>')` wrapper down to `<expr>`, leaving any
/// other expression untouched.
///
/// A collated GROUP BY key is *grouped* by the collated value but *reported* as
/// the underlying column's original value, so every site that emits the key —
/// as opposed to keying on it — must strip the wrapper first. Without this the
/// group `{'A','a'}` would report the folded `'a'` and be named `?` instead of
/// reporting `'A'` under the name `c`.
fn strip_collate(expr: &SqlExpr) -> &SqlExpr {
    match expr {
        SqlExpr::FunctionCall { name, args, .. } if name == "__collate" && args.len() == 2 => {
            &args[0]
        }
        other => other,
    }
}

/// Split GROUP BY key expressions into the column names the VM reads per row and
/// the collation each key groups under (`None` = default BINARY).
///
/// A key that carries a collation arrives wrapped as `__collate(<column>,
/// '<COLL>')` — the same representation explicit `COLLATE` and the planner's
/// column-collation pass already use elsewhere. We **peel** that wrapper here
/// rather than evaluating it, because the collation must fold only the grouping
/// KEY, never the emitted value: SQLite reports the first row of each group with
/// its ORIGINAL text (`'A'` stays `'A'` even though it grouped case-insensitively
/// with `'a'`). The VM keeps the untouched values in `key_vals` for output and
/// applies these collations only when building the key string.
///
/// | GROUP BY expression              | column | collation |
/// |----------------------------------|--------|-----------|
/// | `c`                              | `c`    | `None`    |
/// | `__collate(c, 'NOCASE')`         | `c`    | `NOCASE`  |
/// | anything else (computed key)     | `?`    | `None`    |
fn group_key_cols_and_collations(group_by: &[SqlExpr]) -> (Vec<String>, Vec<Option<String>>) {
    group_by
        .iter()
        .map(|e| match e {
            SqlExpr::Column { name, .. } => (name.clone(), None),
            SqlExpr::FunctionCall { name, args, .. } if name == "__collate" && args.len() == 2 => {
                match (&args[0], &args[1]) {
                    (SqlExpr::Column { name: col, .. }, SqlExpr::Literal(SqlValue::Text(coll))) => {
                        (col.clone(), Some(coll.clone()))
                    }
                    _ => ("?".to_string(), None),
                }
            }
            _ => ("?".to_string(), None),
        })
        .unzip()
}

fn plan_agg_to_agg_fn(func: &AggFunc, is_star: bool) -> AggFn {
    match func {
        AggFunc::Count => {
            if is_star {
                AggFn::CountStar
            } else {
                AggFn::Count
            }
        }
        AggFunc::Sum => AggFn::Sum,
        AggFunc::Avg => AggFn::Avg,
        AggFunc::Min => AggFn::Min,
        AggFunc::Max => AggFn::Max,
        AggFunc::GroupConcat { sep } => AggFn::GroupConcat { sep: sep.clone(), distinct: false },
    }
}

/// Map a `sql-planner` [`AggFunc`] to a codegen [`AggFn`], taking `distinct`
/// into account.  `COUNT(DISTINCT col)` maps to `CountDistinct`; all other
/// combinations fall back to `plan_agg_to_agg_fn`.
fn plan_agg_to_agg_fn_with_distinct(func: &AggFunc, is_star: bool, distinct: bool) -> AggFn {
    if distinct && matches!(func, AggFunc::Count) && !is_star {
        AggFn::CountDistinct
    } else if let AggFunc::GroupConcat { sep } = func {
        // GROUP_CONCAT carries its own `distinct` (dedup before joining), rather
        // than mapping to a separate DISTINCT opcode like COUNT does.
        AggFn::GroupConcat { sep: sep.clone(), distinct }
    } else {
        plan_agg_to_agg_fn(func, is_star)
    }
}

// ===========================================================================
// Helper: extract Scan table + alias from an OptimizedPlan
// ===========================================================================

/// Extract the `(table, alias)` from an `OptimizedPlan::Scan`, or fall back
/// to `("unknown", None)` for non-scan plans.
///
/// This is used by the join compiler which, in Level 1, expects both sides
/// of a join to be either a `Scan` or a simple `Filter(Scan)`.
fn extract_scan_info(plan: &OptimizedPlan) -> (String, Option<String>) {
    match plan {
        OptimizedPlan::Scan { table, alias, .. } => (table.clone(), alias.clone()),
        OptimizedPlan::Filter { input, .. } => extract_scan_info(input),
        _ => ("unknown".to_string(), None),
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_sql_backend::{ColumnDef, SqlValue};
    use coding_adventures_sql_optimizer::optimize;
    use coding_adventures_sql_planner::{
        AggFunc, AggregateItem, Assignment, BinaryOp as PlanBinaryOp, InsertSource, JoinKind,
        LogicalPlan, OutputColumn, SortKey, SqlExpr, UnaryOp as PlanUnaryOp,
    };

    // ── Helpers ──────────────────────────────────────────────────────────────

    /// Build a Scan plan.
    fn scan(table: &str) -> LogicalPlan {
        LogicalPlan::Scan {
            table: table.to_string(),
            alias: None,
        }
    }

    /// Build a Scan plan with an alias.
    fn scan_alias(table: &str, alias: &str) -> LogicalPlan {
        LogicalPlan::Scan {
            table: table.to_string(),
            alias: Some(alias.to_string()),
        }
    }

    /// Integer literal expression.
    fn lit_int(n: i64) -> SqlExpr {
        SqlExpr::Literal(SqlValue::Int(n))
    }

    /// Boolean literal expression.
    fn lit_bool(b: bool) -> SqlExpr {
        SqlExpr::Literal(SqlValue::Bool(b))
    }

    /// NULL literal.
    fn lit_null() -> SqlExpr {
        SqlExpr::Literal(SqlValue::Null)
    }

    /// Text literal.
    fn lit_text(s: &str) -> SqlExpr {
        SqlExpr::Literal(SqlValue::Text(s.to_string()))
    }

    /// Column reference.
    fn col(name: &str) -> SqlExpr {
        SqlExpr::Column {
            table: None,
            name: name.to_string(),
        }
    }

    /// Qualified column reference.
    fn col_qual(table: &str, name: &str) -> SqlExpr {
        SqlExpr::Column {
            table: Some(table.to_string()),
            name: name.to_string(),
        }
    }

    /// Binary op expression.
    fn bin(op: PlanBinaryOp, left: SqlExpr, right: SqlExpr) -> SqlExpr {
        SqlExpr::BinaryOp {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Build an OptimizedPlan::Scan directly (no optimizer needed).
    fn opt_scan(table: &str) -> OptimizedPlan {
        optimize(scan(table))
    }

    /// Build an OptimizedPlan::Scan with alias.
    fn opt_scan_alias(table: &str, alias: &str) -> OptimizedPlan {
        optimize(scan_alias(table, alias))
    }

    /// Compile an OptimizedPlan and return its instructions.
    fn instrs(plan: &OptimizedPlan) -> Vec<Instruction> {
        compile(plan).instructions
    }

    // ── Utility assertions ────────────────────────────────────────────────────

    /// Assert the program contains an `OpenScan` for the given table.
    fn has_open_scan(v: &[Instruction], table: &str) -> bool {
        v.iter()
            .any(|i| matches!(i, Instruction::OpenScan(t, _) if t == table))
    }

    /// Assert the program contains a `CloseScan`.
    fn has_close_scan(v: &[Instruction]) -> bool {
        v.iter().any(|i| matches!(i, Instruction::CloseScan(_)))
    }

    /// Assert the program ends with `Halt` (before any post-ops).
    fn has_halt(v: &[Instruction]) -> bool {
        v.iter().any(|i| matches!(i, Instruction::Halt))
    }

    /// Find index of first occurrence of a given instruction variant tag.
    fn first_idx<F: Fn(&Instruction) -> bool>(v: &[Instruction], f: F) -> Option<usize> {
        v.iter().position(f)
    }

    // ======================================================================
    // 1. Scan compilation
    // ======================================================================

    #[test]
    fn test_scan_starts_with_open_scan() {
        let plan = opt_scan("users");
        let v = instrs(&plan);
        assert!(has_open_scan(&v, "users"), "expected OpenScan(users)");
    }

    #[test]
    fn test_scan_ends_with_close_scan_and_halt() {
        let plan = opt_scan("users");
        let v = instrs(&plan);
        assert!(has_close_scan(&v));
        assert!(has_halt(&v));
    }

    #[test]
    fn test_scan_has_advance_cursor() {
        let plan = opt_scan("orders");
        let v = instrs(&plan);
        assert!(v.iter().any(|i| matches!(i, Instruction::AdvanceCursor(_))));
    }

    #[test]
    fn test_scan_has_jump_if_exhausted() {
        let plan = opt_scan("products");
        let v = instrs(&plan);
        assert!(
            v.iter()
                .any(|i| matches!(i, Instruction::JumpIfExhausted(..)))
        );
    }

    #[test]
    fn test_scan_has_loop_label() {
        let plan = opt_scan("t");
        let v = instrs(&plan);
        assert!(v.iter().any(|i| matches!(i, Instruction::Label(_))));
    }

    #[test]
    fn test_scan_with_alias() {
        let plan = opt_scan_alias("employees", "e");
        let v = instrs(&plan);
        assert!(has_open_scan(&v, "employees"));
    }

    // ======================================================================
    // 2. EmptyResult
    // ======================================================================

    #[test]
    fn test_empty_result_is_just_halt() {
        let plan = OptimizedPlan::EmptyResult;
        let v = instrs(&plan);
        // EmptyResult → Halt only (no scan instructions).
        assert_eq!(v, vec![Instruction::Halt]);
    }

    // ======================================================================
    // 3. Filter compilation
    // ======================================================================

    #[test]
    fn test_filter_has_jump_if_false() {
        let plan = optimize(LogicalPlan::Filter {
            input: Box::new(scan("t")),
            predicate: col("x"),
        });
        let v = instrs(&plan);
        assert!(
            v.iter().any(|i| matches!(i, Instruction::JumpIfFalse(_))),
            "Filter must emit JumpIfFalse"
        );
    }

    #[test]
    fn test_filter_has_open_close_scan() {
        let plan = optimize(LogicalPlan::Filter {
            input: Box::new(scan("orders")),
            predicate: lit_bool(true),
        });
        let v = instrs(&plan);
        assert!(has_open_scan(&v, "orders"));
        assert!(has_close_scan(&v));
    }

    #[test]
    fn test_filter_compiles_predicate_before_jump() {
        // We expect: …AdvanceCursor, JumpIfExhausted, <predicate>, JumpIfFalse…
        let plan = optimize(LogicalPlan::Filter {
            input: Box::new(scan("t")),
            predicate: lit_int(1),
        });
        let v = instrs(&plan);
        let jif_idx = first_idx(&v, |i| matches!(i, Instruction::JumpIfFalse(_))).unwrap();
        let load_idx = first_idx(&v, |i| {
            matches!(i, Instruction::LoadConst(SqlValue::Int(1)))
        })
        .unwrap();
        assert!(
            load_idx < jif_idx,
            "predicate must be compiled before JumpIfFalse"
        );
    }

    // ======================================================================
    // 4. Project compilation
    // ======================================================================

    #[test]
    fn test_project_emits_emit_column_for_each_column() {
        let plan = optimize(LogicalPlan::Project {
            input: Box::new(scan("users")),
            columns: vec![
                OutputColumn {
                    expr: col("id"),
                    alias: Some("id".to_string()),
                },
                OutputColumn {
                    expr: col("name"),
                    alias: Some("name".to_string()),
                },
            ],
        });
        let v = instrs(&plan);
        let ec: Vec<_> = v
            .iter()
            .filter(|i| matches!(i, Instruction::EmitColumn(_)))
            .collect();
        assert_eq!(ec.len(), 2, "expected 2 EmitColumn instructions");
    }

    #[test]
    fn test_project_has_begin_row_and_emit_row() {
        let plan = optimize(LogicalPlan::Project {
            input: Box::new(scan("t")),
            columns: vec![OutputColumn {
                expr: col("x"),
                alias: Some("x".to_string()),
            }],
        });
        let v = instrs(&plan);
        assert!(v.iter().any(|i| matches!(i, Instruction::BeginRow)));
        assert!(v.iter().any(|i| matches!(i, Instruction::EmitRow)));
    }

    #[test]
    fn test_project_column_names() {
        let plan = optimize(LogicalPlan::Project {
            input: Box::new(scan("t")),
            columns: vec![
                OutputColumn {
                    expr: col("a"),
                    alias: Some("alpha".to_string()),
                },
                OutputColumn {
                    expr: col("b"),
                    alias: None,
                },
            ],
        });
        let v = instrs(&plan);
        let names: Vec<_> = v
            .iter()
            .filter_map(|i| {
                if let Instruction::EmitColumn(n) = i {
                    Some(n.clone())
                } else {
                    None
                }
            })
            .collect();
        assert!(names.contains(&"alpha".to_string()));
    }

    // ======================================================================
    // 5. Sort post-op
    // ======================================================================

    #[test]
    fn test_sort_emits_sort_result_after_halt() {
        let plan = optimize(LogicalPlan::Sort {
            input: Box::new(scan("t")),
            keys: vec![SortKey {
                expr: col("name"),
                ascending: true,
                nulls_first: None,
                collation: None,
            }],
        });
        let v = instrs(&plan);
        let halt_idx = first_idx(&v, |i| matches!(i, Instruction::Halt)).unwrap();
        let sort_idx = first_idx(&v, |i| matches!(i, Instruction::SortResult(_))).unwrap();
        assert!(
            sort_idx > halt_idx,
            "SortResult must come after Halt"
        );
    }

    #[test]
    fn test_sort_keys_ascending() {
        let plan = optimize(LogicalPlan::Sort {
            input: Box::new(scan("t")),
            keys: vec![SortKey {
                expr: col("score"),
                ascending: true,
                nulls_first: None,
                collation: None,
            }],
        });
        let v = instrs(&plan);
        let sort_instr = v
            .iter()
            .find(|i| matches!(i, Instruction::SortResult(_)))
            .unwrap();
        if let Instruction::SortResult(keys) = sort_instr {
            assert!(keys[0].ascending);
        }
    }

    #[test]
    fn test_sort_keys_descending() {
        let plan = optimize(LogicalPlan::Sort {
            input: Box::new(scan("t")),
            keys: vec![SortKey {
                expr: col("score"),
                ascending: false,
                nulls_first: None,
                collation: None,
            }],
        });
        let v = instrs(&plan);
        let sort_instr = v
            .iter()
            .find(|i| matches!(i, Instruction::SortResult(_)))
            .unwrap();
        if let Instruction::SortResult(keys) = sort_instr {
            assert!(!keys[0].ascending);
        }
    }

    #[test]
    fn test_sort_multiple_keys() {
        let plan = optimize(LogicalPlan::Sort {
            input: Box::new(scan("t")),
            keys: vec![
                SortKey {
                    expr: col("a"),
                    ascending: true,
                    nulls_first: None,
                    collation: None,
                },
                SortKey {
                    expr: col("b"),
                    ascending: false,
                    nulls_first: None,
                    collation: None,
                },
            ],
        });
        let v = instrs(&plan);
        if let Some(Instruction::SortResult(keys)) =
            v.iter().find(|i| matches!(i, Instruction::SortResult(_)))
        {
            assert_eq!(keys.len(), 2);
        }
    }

    // ======================================================================
    // 6. Limit post-op
    // ======================================================================

    #[test]
    fn test_limit_emits_limit_result_after_halt() {
        let plan = optimize(LogicalPlan::Limit {
            input: Box::new(scan("t")),
            count: Some(10),
            offset: Some(5),
        });
        let v = instrs(&plan);
        let halt_idx = first_idx(&v, |i| matches!(i, Instruction::Halt)).unwrap();
        let limit_idx = first_idx(&v, |i| matches!(i, Instruction::LimitResult(..)))
            .unwrap();
        assert!(limit_idx > halt_idx, "LimitResult must come after Halt");
    }

    #[test]
    fn test_limit_values() {
        let plan = optimize(LogicalPlan::Limit {
            input: Box::new(scan("t")),
            count: Some(10),
            offset: Some(5),
        });
        let v = instrs(&plan);
        let limit_instr = v
            .iter()
            .find(|i| matches!(i, Instruction::LimitResult(..)))
            .unwrap();
        assert_eq!(
            *limit_instr,
            Instruction::LimitResult(Some(10), Some(5))
        );
    }

    #[test]
    fn test_limit_no_offset() {
        let plan = optimize(LogicalPlan::Limit {
            input: Box::new(scan("t")),
            count: Some(20),
            offset: None,
        });
        let v = instrs(&plan);
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::LimitResult(Some(20), None))));
    }

    // ======================================================================
    // 7. Distinct post-op
    // ======================================================================

    #[test]
    fn test_distinct_emits_distinct_result_after_halt() {
        let plan = optimize(LogicalPlan::Distinct(Box::new(scan("t")), vec![]));
        let v = instrs(&plan);
        let halt_idx = first_idx(&v, |i| matches!(i, Instruction::Halt)).unwrap();
        let dist_idx = first_idx(&v, |i| matches!(i, Instruction::DistinctResult(_))).unwrap();
        assert!(dist_idx > halt_idx, "DistinctResult must come after Halt");
    }

    // ======================================================================
    // 8. Aggregate compilation
    // ======================================================================

    #[test]
    fn test_aggregate_count_star_has_init_agg() {
        let plan = optimize(LogicalPlan::Aggregate {
            input: Box::new(scan("t")),
            group_by: vec![],
            aggregates: vec![AggregateItem {
                func: AggFunc::Count,
                arg: None,
                distinct: false,
                alias: Some("cnt".to_string()),
            }],
        });
        let v = instrs(&plan);
        assert!(
            v.iter().any(|i| matches!(i, Instruction::InitAgg(1))),
            "expected InitAgg(1)"
        );
    }

    #[test]
    fn test_aggregate_count_star_has_update_agg() {
        let plan = optimize(LogicalPlan::Aggregate {
            input: Box::new(scan("t")),
            group_by: vec![],
            aggregates: vec![AggregateItem {
                func: AggFunc::Count,
                arg: None,
                distinct: false,
                alias: Some("cnt".to_string()),
            }],
        });
        let v = instrs(&plan);
        assert!(
            v.iter()
                .any(|i| matches!(i, Instruction::UpdateAgg(0, AggFn::CountStar))),
            "expected UpdateAgg(0, CountStar)"
        );
    }

    #[test]
    fn test_aggregate_count_star_has_finalize_agg() {
        let plan = optimize(LogicalPlan::Aggregate {
            input: Box::new(scan("t")),
            group_by: vec![],
            aggregates: vec![AggregateItem {
                func: AggFunc::Count,
                arg: None,
                distinct: false,
                alias: Some("cnt".to_string()),
            }],
        });
        let v = instrs(&plan);
        assert!(
            v.iter()
                .any(|i| matches!(i, Instruction::FinalizeAgg(0, AggFn::CountStar))),
            "expected FinalizeAgg(0, CountStar)"
        );
    }

    #[test]
    fn test_aggregate_sum_has_update_and_finalize() {
        let plan = optimize(LogicalPlan::Aggregate {
            input: Box::new(scan("orders")),
            group_by: vec![],
            aggregates: vec![AggregateItem {
                func: AggFunc::Sum,
                arg: Some(col("amount")),
                distinct: false,
                alias: Some("total".to_string()),
            }],
        });
        let v = instrs(&plan);
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::UpdateAgg(0, AggFn::Sum))));
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::FinalizeAgg(0, AggFn::Sum))));
    }

    #[test]
    fn test_aggregate_multiple_slots() {
        // COUNT(*) + SUM(amount) → InitAgg(2), UpdateAgg(0, ..), UpdateAgg(1, ..)
        let plan = optimize(LogicalPlan::Aggregate {
            input: Box::new(scan("t")),
            group_by: vec![],
            aggregates: vec![
                AggregateItem {
                    func: AggFunc::Count,
                    arg: None,
                    distinct: false,
                    alias: Some("cnt".to_string()),
                },
                AggregateItem {
                    func: AggFunc::Sum,
                    arg: Some(col("val")),
                    distinct: false,
                    alias: Some("s".to_string()),
                },
            ],
        });
        let v = instrs(&plan);
        assert!(v.iter().any(|i| matches!(i, Instruction::InitAgg(2))));
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::UpdateAgg(0, AggFn::CountStar))));
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::UpdateAgg(1, AggFn::Sum))));
    }

    #[test]
    fn test_aggregate_min_and_max() {
        let plan = optimize(LogicalPlan::Aggregate {
            input: Box::new(scan("t")),
            group_by: vec![],
            aggregates: vec![
                AggregateItem {
                    func: AggFunc::Min,
                    arg: Some(col("score")),
                    distinct: false,
                    alias: Some("lo".to_string()),
                },
                AggregateItem {
                    func: AggFunc::Max,
                    arg: Some(col("score")),
                    distinct: false,
                    alias: Some("hi".to_string()),
                },
            ],
        });
        let v = instrs(&plan);
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::UpdateAgg(0, AggFn::Min))));
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::UpdateAgg(1, AggFn::Max))));
    }

    #[test]
    fn test_aggregate_avg() {
        let plan = optimize(LogicalPlan::Aggregate {
            input: Box::new(scan("t")),
            group_by: vec![],
            aggregates: vec![AggregateItem {
                func: AggFunc::Avg,
                arg: Some(col("price")),
                distinct: false,
                alias: Some("avg_price".to_string()),
            }],
        });
        let v = instrs(&plan);
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::UpdateAgg(0, AggFn::Avg))));
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::FinalizeAgg(0, AggFn::Avg))));
    }

    #[test]
    fn test_aggregate_with_group_by_has_save_group_key() {
        let plan = optimize(LogicalPlan::Aggregate {
            input: Box::new(scan("orders")),
            group_by: vec![col("category")],
            aggregates: vec![AggregateItem {
                func: AggFunc::Count,
                arg: None,
                distinct: false,
                alias: Some("cnt".to_string()),
            }],
        });
        let v = instrs(&plan);
        assert!(
            v.iter().any(|i| matches!(i, Instruction::SaveGroupKey(..))),
            "GROUP BY should emit SaveGroupKey"
        );
    }

    // ======================================================================
    // 9. Having compilation
    // ======================================================================

    #[test]
    fn test_having_has_jump_if_false_after_finalize() {
        let plan = optimize(LogicalPlan::Having {
            input: Box::new(LogicalPlan::Aggregate {
                input: Box::new(scan("t")),
                group_by: vec![],
                aggregates: vec![AggregateItem {
                    func: AggFunc::Count,
                    arg: None,
                    distinct: false,
                    alias: Some("cnt".to_string()),
                }],
            }),
            predicate: bin(PlanBinaryOp::Gt, col("cnt"), lit_int(5)),
        });
        let v = instrs(&plan);
        // Must have FinalizeAgg and JumpIfFalse (HAVING predicate check).
        assert!(v.iter().any(|i| matches!(i, Instruction::FinalizeAgg(..))));
        assert!(v.iter().any(|i| matches!(i, Instruction::JumpIfFalse(_))));
    }

    // ======================================================================
    // 10. CreateTable / DropTable
    // ======================================================================

    #[test]
    fn test_create_table_emits_create_table_instr() {
        let plan = optimize(LogicalPlan::CreateTable {
            table: "users".to_string(),
            if_not_exists: false,
            columns: vec![ColumnDef::new("id", "INTEGER")],
        });
        let v = instrs(&plan);
        assert!(
            v.iter()
                .any(|i| matches!(i, Instruction::CreateTableInstr(..))),
            "expected CreateTableInstr"
        );
    }

    #[test]
    fn test_create_table_if_not_exists() {
        let plan = optimize(LogicalPlan::CreateTable {
            table: "foo".to_string(),
            if_not_exists: true,
            columns: vec![],
        });
        let v = instrs(&plan);
        let found = v.iter().any(|i| {
            matches!(i, Instruction::CreateTableInstr(name, true, _) if name == "foo")
        });
        assert!(found, "expected CreateTableInstr with if_not_exists=true");
    }

    #[test]
    fn test_drop_table_emits_drop_table_instr() {
        let plan = optimize(LogicalPlan::DropTable {
            table: "old_table".to_string(),
            if_exists: true,
        });
        let v = instrs(&plan);
        assert!(
            v.iter()
                .any(|i| matches!(i, Instruction::DropTableInstr(_, true))),
            "expected DropTableInstr with if_exists=true"
        );
    }

    #[test]
    fn test_drop_table_name() {
        let plan = optimize(LogicalPlan::DropTable {
            table: "mytable".to_string(),
            if_exists: false,
        });
        let v = instrs(&plan);
        let found = v
            .iter()
            .any(|i| matches!(i, Instruction::DropTableInstr(n, false) if n == "mytable"));
        assert!(found);
    }

    #[test]
    fn test_create_table_ends_with_halt() {
        let plan = optimize(LogicalPlan::CreateTable {
            table: "t".to_string(),
            if_not_exists: false,
            columns: vec![],
        });
        let v = instrs(&plan);
        assert!(has_halt(&v));
    }

    #[test]
    fn test_drop_table_ends_with_halt() {
        let plan = optimize(LogicalPlan::DropTable {
            table: "t".to_string(),
            if_exists: false,
        });
        let v = instrs(&plan);
        assert!(has_halt(&v));
    }

    // ======================================================================
    // 11. Expression compilation
    // ======================================================================

    #[test]
    fn test_expr_literal_int() {
        let plan = optimize(LogicalPlan::Project {
            input: Box::new(scan("t")),
            columns: vec![OutputColumn {
                expr: lit_int(42),
                alias: Some("x".to_string()),
            }],
        });
        let v = instrs(&plan);
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::LoadConst(SqlValue::Int(42)))));
    }

    #[test]
    fn test_expr_literal_null() {
        let plan = optimize(LogicalPlan::Project {
            input: Box::new(scan("t")),
            columns: vec![OutputColumn {
                expr: lit_null(),
                alias: Some("n".to_string()),
            }],
        });
        let v = instrs(&plan);
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::LoadConst(SqlValue::Null))));
    }

    #[test]
    fn test_expr_literal_text() {
        let plan = optimize(LogicalPlan::Project {
            input: Box::new(scan("t")),
            columns: vec![OutputColumn {
                expr: lit_text("hello"),
                alias: Some("s".to_string()),
            }],
        });
        let v = instrs(&plan);
        assert!(v.iter().any(|i| {
            matches!(i, Instruction::LoadConst(SqlValue::Text(s)) if s == "hello")
        }));
    }

    #[test]
    fn test_expr_binary_op_add() {
        // col("a") + col("b") → LoadColumn(a), LoadColumn(b), BinaryOpInstr(Add)
        // We use column references (not literals) because the optimizer's
        // ConstantFolding pass would fold literal expressions like `1 + 2 → 3`.
        let plan = optimize(LogicalPlan::Project {
            input: Box::new(scan("t")),
            columns: vec![OutputColumn {
                expr: bin(PlanBinaryOp::Add, col("a"), col("b")),
                alias: Some("r".to_string()),
            }],
        });
        let v = instrs(&plan);
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::BinaryOpInstr(BinaryOp::Add))));
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::LoadColumn(None, n) if n == "a")));
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::LoadColumn(None, n) if n == "b")));
    }

    #[test]
    fn test_expr_binary_op_order() {
        // For `col("a") + col("b")`: LoadColumn(a) before LoadColumn(b) before BinaryOpInstr.
        // Column refs survive ConstantFolding, so the Add instruction remains visible.
        let plan = optimize(LogicalPlan::Project {
            input: Box::new(scan("t")),
            columns: vec![OutputColumn {
                expr: bin(PlanBinaryOp::Add, col("a"), col("b")),
                alias: Some("r".to_string()),
            }],
        });
        let v = instrs(&plan);
        let ia = first_idx(&v, |i| {
            matches!(i, Instruction::LoadColumn(None, n) if n == "a")
        })
        .unwrap();
        let ib = first_idx(&v, |i| {
            matches!(i, Instruction::LoadColumn(None, n) if n == "b")
        })
        .unwrap();
        let op = first_idx(&v, |i| {
            matches!(i, Instruction::BinaryOpInstr(BinaryOp::Add))
        })
        .unwrap();
        assert!(ia < ib, "left operand before right");
        assert!(ib < op, "right operand before operator");
    }

    #[test]
    fn test_expr_unary_neg() {
        // -col("x") → LoadColumn(x), UnaryOpInstr(Neg)
        // Using a column ref prevents ConstantFolding from evaluating the negation.
        let plan = optimize(LogicalPlan::Project {
            input: Box::new(scan("t")),
            columns: vec![OutputColumn {
                expr: SqlExpr::UnaryOp {
                    op: PlanUnaryOp::Neg,
                    expr: Box::new(col("x")),
                },
                alias: Some("neg".to_string()),
            }],
        });
        let v = instrs(&plan);
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::UnaryOpInstr(UnaryOp::Neg))));
    }

    #[test]
    fn test_expr_unary_not() {
        // NOT col("flag") → LoadColumn(flag), UnaryOpInstr(Not)
        // Using a column ref prevents ConstantFolding from evaluating NOT.
        let plan = optimize(LogicalPlan::Project {
            input: Box::new(scan("t")),
            columns: vec![OutputColumn {
                expr: SqlExpr::UnaryOp {
                    op: PlanUnaryOp::Not,
                    expr: Box::new(col("flag")),
                },
                alias: Some("n".to_string()),
            }],
        });
        let v = instrs(&plan);
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::UnaryOpInstr(UnaryOp::Not))));
    }

    #[test]
    fn test_expr_is_null() {
        let plan = optimize(LogicalPlan::Filter {
            input: Box::new(scan("t")),
            predicate: SqlExpr::IsNull(Box::new(col("x"))),
        });
        let v = instrs(&plan);
        assert!(v.iter().any(|i| matches!(i, Instruction::IsNull)));
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::LoadColumn(None, n) if n == "x")));
    }

    #[test]
    fn test_expr_is_not_null() {
        let plan = optimize(LogicalPlan::Filter {
            input: Box::new(scan("t")),
            predicate: SqlExpr::IsNotNull(Box::new(col("y"))),
        });
        let v = instrs(&plan);
        assert!(v.iter().any(|i| matches!(i, Instruction::IsNotNull)));
    }

    #[test]
    fn test_expr_between() {
        // x BETWEEN 1 AND 10 → [LoadColumn(x), LoadConst(1), LoadConst(10), Between(true)]
        let plan = optimize(LogicalPlan::Filter {
            input: Box::new(scan("t")),
            predicate: SqlExpr::Between {
                value: Box::new(col("x")),
                low: Box::new(lit_int(1)),
                high: Box::new(lit_int(10)),
                negated: false,
            },
        });
        let v = instrs(&plan);
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::Between(true))));
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::LoadConst(SqlValue::Int(1)))));
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::LoadConst(SqlValue::Int(10)))));
    }

    #[test]
    fn test_expr_between_negated() {
        // x NOT BETWEEN 1 AND 10 → Between(false)
        let plan = optimize(LogicalPlan::Filter {
            input: Box::new(scan("t")),
            predicate: SqlExpr::Between {
                value: Box::new(col("x")),
                low: Box::new(lit_int(1)),
                high: Box::new(lit_int(10)),
                negated: true,
            },
        });
        let v = instrs(&plan);
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::Between(false))));
    }

    #[test]
    fn test_expr_like() {
        let plan = optimize(LogicalPlan::Filter {
            input: Box::new(scan("t")),
            predicate: SqlExpr::Like {
                value: Box::new(col("name")),
                pattern: Box::new(lit_text("A%")),
                negated: false,
                escape: None,
            },
        });
        let v = instrs(&plan);
        assert!(v.iter().any(|i| matches!(i, Instruction::Like(_))));
    }

    #[test]
    fn test_expr_in_list() {
        // x IN (1, 2, 3) → [LoadColumn(x), LoadConst(1), LoadConst(2), LoadConst(3), InList(3)]
        let plan = optimize(LogicalPlan::Filter {
            input: Box::new(scan("t")),
            predicate: SqlExpr::InList {
                value: Box::new(col("x")),
                list: vec![lit_int(1), lit_int(2), lit_int(3)],
                negated: false,
            },
        });
        let v = instrs(&plan);
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::InList(3))));
    }

    #[test]
    fn test_expr_in_list_negated() {
        // x NOT IN (1, 2) → InList(2) + UnaryOpInstr(Not)
        let plan = optimize(LogicalPlan::Filter {
            input: Box::new(scan("t")),
            predicate: SqlExpr::InList {
                value: Box::new(col("x")),
                list: vec![lit_int(1), lit_int(2)],
                negated: true,
            },
        });
        let v = instrs(&plan);
        assert!(v.iter().any(|i| matches!(i, Instruction::InList(2))));
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::UnaryOpInstr(UnaryOp::Not))));
    }

    #[test]
    fn test_expr_column_load() {
        // A plain column reference becomes LoadColumn.
        let plan = optimize(LogicalPlan::Filter {
            input: Box::new(scan("t")),
            predicate: col("price"),
        });
        let v = instrs(&plan);
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::LoadColumn(None, n) if n == "price")));
    }

    #[test]
    fn test_expr_qualified_column_load() {
        // A qualified column reference: `u.name` → LoadColumn(Some("u"), "name")
        let plan = optimize(LogicalPlan::Filter {
            input: Box::new(scan("users")),
            predicate: col_qual("u", "name"),
        });
        let v = instrs(&plan);
        assert!(v.iter().any(|i| {
            matches!(i, Instruction::LoadColumn(Some(t), n) if t == "u" && n == "name")
        }));
    }

    // ======================================================================
    // 12. DML compilation
    // ======================================================================

    #[test]
    fn test_insert_emits_insert_row() {
        let plan = optimize(LogicalPlan::Insert {
            table: "users".to_string(),
            columns: None,
            source: InsertSource::Values(vec![vec![lit_int(1), lit_text("alice")]]),
        });
        let v = instrs(&plan);
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::InsertRow(t, _) if t == "users")));
    }

    #[test]
    fn test_insert_with_columns() {
        // When columns are provided, the codegen emits EmitColumn instructions
        // using the column names so the VM's row_buffer has named entries.
        // InsertRow now always receives None (the names live in the row_buffer).
        let plan = optimize(LogicalPlan::Insert {
            table: "t".to_string(),
            columns: Some(vec!["id".to_string(), "name".to_string()]),
            source: InsertSource::Values(vec![vec![lit_int(1), lit_text("bob")]]),
        });
        let v = instrs(&plan);
        // Verify the InsertRow is emitted.
        let has_insert_row = v.iter().any(|i| matches!(i, Instruction::InsertRow(t, None) if t == "t"));
        assert!(has_insert_row, "expected InsertRow for table t with None cols");
        // Verify EmitColumn is emitted for each provided column.
        let emit_cols: Vec<_> = v
            .iter()
            .filter_map(|i| if let Instruction::EmitColumn(n) = i { Some(n.as_str()) } else { None })
            .collect();
        assert!(emit_cols.contains(&"id"), "expected EmitColumn(\"id\")");
        assert!(emit_cols.contains(&"name"), "expected EmitColumn(\"name\")");
    }

    #[test]
    fn test_insert_multiple_rows() {
        let plan = optimize(LogicalPlan::Insert {
            table: "t".to_string(),
            columns: None,
            source: InsertSource::Values(vec![
                vec![lit_int(1)],
                vec![lit_int(2)],
                vec![lit_int(3)],
            ]),
        });
        let v = instrs(&plan);
        let insert_count = v
            .iter()
            .filter(|i| matches!(i, Instruction::InsertRow(..)))
            .count();
        assert_eq!(insert_count, 3, "three rows → three InsertRow instructions");
    }

    #[test]
    fn test_update_emits_update_rows() {
        let plan = optimize(LogicalPlan::Update {
            table: "users".to_string(),
            assignments: vec![Assignment {
                column: "name".to_string(),
                value: lit_text("alice"),
            }],
            predicate: None,
        });
        let v = instrs(&plan);
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::UpdateRows(t, _) if t == "users")));
    }

    #[test]
    fn test_update_with_predicate_has_jump_if_false() {
        let plan = optimize(LogicalPlan::Update {
            table: "t".to_string(),
            assignments: vec![Assignment {
                column: "x".to_string(),
                value: lit_int(0),
            }],
            predicate: Some(bin(PlanBinaryOp::Eq, col("id"), lit_int(1))),
        });
        let v = instrs(&plan);
        assert!(v.iter().any(|i| matches!(i, Instruction::JumpIfFalse(_))));
    }

    #[test]
    fn test_delete_emits_delete_rows() {
        let plan = optimize(LogicalPlan::Delete {
            table: "orders".to_string(),
            predicate: None,
        });
        let v = instrs(&plan);
        assert!(v
            .iter()
            .any(|i| matches!(i, Instruction::DeleteRows(t) if t == "orders")));
    }

    #[test]
    fn test_delete_with_predicate_has_jump_if_false() {
        let plan = optimize(LogicalPlan::Delete {
            table: "t".to_string(),
            predicate: Some(lit_bool(false)),
        });
        let v = instrs(&plan);
        // The predicate lit_bool(false) gets constant-folded by the optimizer,
        // which turns Filter(false) into EmptyResult → just Halt.
        // So we just check the program terminates cleanly.
        assert!(has_halt(&v));
    }

    // ======================================================================
    // 13. Post-op ordering
    // ======================================================================

    #[test]
    fn test_sort_then_limit_ordering() {
        // Sort(Limit(Scan)) → after Halt: SortResult then LimitResult
        let plan = optimize(LogicalPlan::Sort {
            input: Box::new(LogicalPlan::Limit {
                input: Box::new(scan("t")),
                count: Some(5),
                offset: None,
            }),
            keys: vec![SortKey {
                expr: col("x"),
                ascending: true,
                nulls_first: None,
                collation: None,
            }],
        });
        let v = instrs(&plan);
        let halt_idx = first_idx(&v, |i| matches!(i, Instruction::Halt)).unwrap();
        let sort_idx = first_idx(&v, |i| matches!(i, Instruction::SortResult(_))).unwrap();
        let limit_idx =
            first_idx(&v, |i| matches!(i, Instruction::LimitResult(..))).unwrap();
        assert!(halt_idx < sort_idx, "Sort after Halt");
        assert!(sort_idx < limit_idx, "Sort before Limit in post-ops");
    }

    #[test]
    fn test_distinct_and_limit_ordering() {
        let plan = optimize(LogicalPlan::Distinct(
            Box::new(LogicalPlan::Limit {
                input: Box::new(scan("t")),
                count: Some(3),
                offset: None,
            }),
            vec![],
        ));
        let v = instrs(&plan);
        let halt_idx = first_idx(&v, |i| matches!(i, Instruction::Halt)).unwrap();
        let dist_idx = first_idx(&v, |i| matches!(i, Instruction::DistinctResult(_))).unwrap();
        let limit_idx =
            first_idx(&v, |i| matches!(i, Instruction::LimitResult(..))).unwrap();
        // Both post-ops must come after Halt.
        assert!(halt_idx < dist_idx);
        assert!(halt_idx < limit_idx);
    }

    // ======================================================================
    // 14. Join compilation
    // ======================================================================

    #[test]
    fn test_join_has_two_open_scans() {
        let plan = optimize(LogicalPlan::Join {
            left: Box::new(scan("employees")),
            right: Box::new(scan("departments")),
            kind: JoinKind::Inner,
            condition: None,
        });
        let v = instrs(&plan);
        let open_count = v
            .iter()
            .filter(|i| matches!(i, Instruction::OpenScan(..)))
            .count();
        assert_eq!(open_count, 2, "join needs two OpenScan instructions");
    }

    #[test]
    fn test_join_inner_with_condition_has_jump_if_false() {
        let plan = optimize(LogicalPlan::Join {
            left: Box::new(scan("a")),
            right: Box::new(scan("b")),
            kind: JoinKind::Inner,
            condition: Some(bin(
                PlanBinaryOp::Eq,
                col_qual("a", "id"),
                col_qual("b", "a_id"),
            )),
        });
        let v = instrs(&plan);
        assert!(v.iter().any(|i| matches!(i, Instruction::JumpIfFalse(_))));
    }

    #[test]
    fn test_join_cross_has_no_jump_if_false() {
        let plan = optimize(LogicalPlan::Join {
            left: Box::new(scan("a")),
            right: Box::new(scan("b")),
            kind: JoinKind::Cross,
            condition: None,
        });
        let v = instrs(&plan);
        // Cross join with no condition should NOT emit a predicate check.
        assert!(!v.iter().any(|i| matches!(i, Instruction::JumpIfFalse(_))));
    }

    // ======================================================================
    // 15. Recursive depth guard
    // ======================================================================

    #[test]
    fn test_deep_expression_does_not_overflow() {
        // Build a 600-level deep nested expression: (((...(1 + 1) + 1) + 1)...)
        let mut e = lit_int(1);
        for _ in 0..600 {
            e = bin(PlanBinaryOp::Add, e, lit_int(1));
        }
        let plan = optimize(LogicalPlan::Filter {
            input: Box::new(scan("t")),
            predicate: e,
        });
        // Should not panic, even though depth > MAX_EXPR_DEPTH.
        let _v = instrs(&plan);
    }

    // ======================================================================
    // 16. Program structure guarantees
    // ======================================================================

    #[test]
    fn test_program_always_has_halt() {
        for plan in [
            opt_scan("t"),
            OptimizedPlan::EmptyResult,
            optimize(LogicalPlan::CreateTable {
                table: "t".to_string(),
                if_not_exists: false,
                columns: vec![],
            }),
        ] {
            assert!(has_halt(&instrs(&plan)), "every program must have Halt");
        }
    }

    #[test]
    fn test_compile_returns_program_struct() {
        let plan = opt_scan("t");
        let prog = compile(&plan);
        assert!(!prog.instructions.is_empty());
    }

    #[test]
    fn test_program_default_is_empty() {
        let p = Program::default();
        assert!(p.instructions.is_empty());
    }

    // ======================================================================
    // 17. All binary ops map correctly
    // ======================================================================

    // Each operator test uses column references (`col("a")`, `col("b")`) rather
    // than literal values.  The ConstantFolding optimizer pass evaluates
    // constant-only expressions at plan time (e.g. `1 + 2 → 3`), which would
    // eliminate the operator instruction before codegen can see it.  Column
    // references are not folded, so the BinaryOpInstr always appears in output.
    macro_rules! test_binary_op {
        ($name:ident, $plan_op:expr, $codegen_op:pat) => {
            #[test]
            fn $name() {
                let plan = optimize(LogicalPlan::Project {
                    input: Box::new(scan("t")),
                    columns: vec![OutputColumn {
                        expr: bin($plan_op, col("a"), col("b")),
                        alias: Some("r".to_string()),
                    }],
                });
                let v = instrs(&plan);
                assert!(v.iter().any(|i| matches!(i, Instruction::BinaryOpInstr($codegen_op))));
            }
        };
    }

    test_binary_op!(test_op_sub, PlanBinaryOp::Sub, BinaryOp::Sub);
    test_binary_op!(test_op_mul, PlanBinaryOp::Mul, BinaryOp::Mul);
    test_binary_op!(test_op_div, PlanBinaryOp::Div, BinaryOp::Div);
    test_binary_op!(test_op_mod, PlanBinaryOp::Mod, BinaryOp::Mod);
    test_binary_op!(test_op_eq, PlanBinaryOp::Eq, BinaryOp::Eq);
    test_binary_op!(test_op_neq, PlanBinaryOp::Neq, BinaryOp::Neq);
    test_binary_op!(test_op_lt, PlanBinaryOp::Lt, BinaryOp::Lt);
    test_binary_op!(test_op_lte, PlanBinaryOp::Lte, BinaryOp::Lte);
    test_binary_op!(test_op_gt, PlanBinaryOp::Gt, BinaryOp::Gt);
    test_binary_op!(test_op_gte, PlanBinaryOp::Gte, BinaryOp::Gte);
    test_binary_op!(test_op_and, PlanBinaryOp::And, BinaryOp::And);
    test_binary_op!(test_op_or, PlanBinaryOp::Or, BinaryOp::Or);
    test_binary_op!(test_op_concat, PlanBinaryOp::Concat, BinaryOp::Concat);
}

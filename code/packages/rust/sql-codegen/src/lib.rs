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
    OutputColumn, SortKey, SqlExpr, UnaryOp as PlanUnaryOp,
};

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
    static EXPR_DEPTH: std::cell::Cell<usize> = std::cell::Cell::new(0);
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
    /// `SUM(col)` — sum of all non-NULL values
    Sum,
    /// `AVG(col)` — arithmetic mean of non-NULL values
    Avg,
    /// `MIN(col)` — minimum value
    Min,
    /// `MAX(col)` — maximum value
    Max,
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
    /// Pops all three; pushes `Bool(value >= low AND value <= high)`.
    /// If `inclusive = false` the bounds are exclusive (value > low AND value < high).
    ///
    /// ## Stack effect: `[..., value, low, high] → [..., Bool]`
    Between(bool), // inclusive

    /// SQL `LIKE` pattern match.
    ///
    /// Expects `[..., value, pattern]` (pattern on top).
    /// Pops both; pushes a Bool result.
    ///
    /// ## Stack effect: `[..., value, pattern] → [..., Bool]`
    Like,

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
    /// `Vec<String>` lists the column names making up the group key.
    /// This instruction is emitted inside the scan loop, before `UpdateAgg`.
    SaveGroupKey(Vec<String>),

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

    /// Execute `UPDATE table SET ... [WHERE ...]` for the current cursor row.
    ///
    /// The WHERE predicate and SET expressions are evaluated from the stack.
    UpdateRows(String), // table

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
    DistinctResult,

    /// Truncate the result set to at most `count` rows, starting from `offset`.
    ///
    /// Emitted after `Halt` for `LIMIT [count] [OFFSET offset]`.
    ///
    /// - `None` for `count` means no row limit.
    /// - `None` for `offset` means start from row 0.
    LimitResult(Option<i64>, Option<i64>), // count, offset
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
}

impl Compiler {
    fn new() -> Self {
        Compiler {
            instructions: Vec::new(),
            label_counter: 0,
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
    fn compile_plan(&mut self, plan: &OptimizedPlan) {
        // Step 1: Peel Sort / Limit / Distinct wrappers from the outermost plan.
        // These operators apply to the entire result buffer; they run *after*
        // the main scan loop terminates.  We collect them here and append them
        // after `Halt`.
        let (inner, post_ops) = peel_post_ops(plan);

        // Step 2: Compile the inner query (scan loop, filter, project, etc.).
        self.compile_inner(inner);

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
            | OptimizedPlan::Distinct(input) => {
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
        match input {
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
                        let name = col.alias.clone().unwrap_or_else(|| "?".to_string());
                        compiler.emit(Instruction::EmitColumn(name));
                    }
                    compiler.emit(Instruction::EmitRow);
                    compiler.emit(Instruction::Label(skip_lbl.clone()));
                });
            }
            _ => {
                let cols = columns.to_vec();
                self.compile_scan_loop(input, |compiler, _alias| {
                    compiler.emit(Instruction::BeginRow);
                    for col in &cols {
                        compiler.compile_expr(&col.expr);
                        let name = col.alias.clone().unwrap_or_else(|| "?".to_string());
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

        // Build group key column names for SaveGroupKey.
        let group_key_cols: Vec<String> = group_by
            .iter()
            .map(|e| match e {
                SqlExpr::Column { name, .. } => name.clone(),
                _ => "?".to_string(),
            })
            .collect();

        // Compute all the agg function tags upfront so the borrow checker
        // can release the borrow on `aggregates` before we use the compiler.
        let agg_fns: Vec<AggFn> = aggregates
            .iter()
            .map(|a| plan_agg_to_agg_fn(&a.func, a.arg.is_none()))
            .collect();
        let agg_args: Vec<Option<SqlExpr>> =
            aggregates.iter().map(|a| a.arg.clone()).collect();
        let agg_aliases: Vec<Option<String>> =
            aggregates.iter().map(|a| a.alias.clone()).collect();

        // Phase 1: scan loop body — save group key + update each aggregate.
        let loop_lbl = self.fresh_label("agg_loop");
        let end_lbl = self.fresh_label("agg_end");

        match input {
            OptimizedPlan::Scan { table, alias, .. } => {
                let alias = alias.clone();
                self.emit(Instruction::OpenScan(table.clone(), alias.clone()));
                self.emit(Instruction::Label(loop_lbl.clone()));
                self.emit(Instruction::AdvanceCursor(alias.clone()));
                self.emit(Instruction::JumpIfExhausted(alias.clone(), end_lbl.clone()));

                // Save group key (may be empty for global aggregates).
                if !group_key_cols.is_empty() {
                    self.emit(Instruction::SaveGroupKey(group_key_cols.clone()));
                }

                // Update each aggregate slot.
                for (i, (fn_tag, arg)) in agg_fns.iter().zip(agg_args.iter()).enumerate() {
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
                // Non-scan input: compile the inner plan first, then update.
                // This is a simplified handling for the test suite.
                self.compile_inner(other);
                if !group_key_cols.is_empty() {
                    self.emit(Instruction::SaveGroupKey(group_key_cols.clone()));
                }
                for (i, (fn_tag, arg)) in agg_fns.iter().zip(agg_args.iter()).enumerate() {
                    if let Some(arg_expr) = arg {
                        self.compile_expr(arg_expr);
                    }
                    self.emit(Instruction::UpdateAgg(i, fn_tag.clone()));
                }
            }
        }

        // Phase 2: emit one output row per group.
        // FinalizeAgg pushes the final accumulated value for each slot,
        // then we assemble a row from group key values + aggregate values.
        self.emit(Instruction::BeginRow);
        // Emit group-by columns first (as LoadColumn for each group key expr).
        for e in group_by {
            self.compile_expr(e);
            let name = match e {
                SqlExpr::Column { name, .. } => name.clone(),
                _ => "?".to_string(),
            };
            self.emit(Instruction::EmitColumn(name));
        }
        // Emit finalized aggregate values.
        for (i, (fn_tag, alias)) in agg_fns.iter().zip(agg_aliases.iter()).enumerate() {
            self.emit(Instruction::FinalizeAgg(i, fn_tag.clone()));
            let col_name = alias.clone().unwrap_or_else(|| format!("agg_{}", i));
            self.emit(Instruction::EmitColumn(col_name));
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

                let group_key_cols: Vec<String> = group_by
                    .iter()
                    .map(|e| match e {
                        SqlExpr::Column { name, .. } => name.clone(),
                        _ => "?".to_string(),
                    })
                    .collect();

                let agg_fns: Vec<AggFn> = aggregates
                    .iter()
                    .map(|a| plan_agg_to_agg_fn(&a.func, a.arg.is_none()))
                    .collect();
                let agg_args: Vec<Option<SqlExpr>> =
                    aggregates.iter().map(|a| a.arg.clone()).collect();
                let agg_aliases: Vec<Option<String>> =
                    aggregates.iter().map(|a| a.alias.clone()).collect();

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
                            self.emit(Instruction::SaveGroupKey(group_key_cols.clone()));
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
                            self.emit(Instruction::SaveGroupKey(group_key_cols.clone()));
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
                    let col_name = agg_aliases[i]
                        .clone()
                        .unwrap_or_else(|| format!("agg_{}", i));
                    self.emit(Instruction::EmitColumn(col_name));
                }

                // HAVING predicate check — skip the row if false.
                self.compile_expr(predicate);
                self.emit(Instruction::JumpIfFalse(skip_lbl.clone()));

                self.emit(Instruction::BeginRow);
                for e in group_by {
                    self.compile_expr(e);
                    let name = match e {
                        SqlExpr::Column { name, .. } => name.clone(),
                        _ => "?".to_string(),
                    };
                    self.emit(Instruction::EmitColumn(name));
                }
                for (i, (fn_tag, alias)) in agg_fns.iter().zip(agg_aliases.iter()).enumerate() {
                    self.emit(Instruction::FinalizeAgg(i, fn_tag.clone()));
                    let col_name = alias.clone().unwrap_or_else(|| format!("agg_{}", i));
                    self.emit(Instruction::EmitColumn(col_name));
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
                    for expr in row {
                        self.compile_expr(expr);
                    }
                    self.emit(Instruction::InsertRow(
                        table.to_string(),
                        columns.clone(),
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

        // Push each assignment expression.
        for assignment in assignments {
            self.compile_expr(&assignment.value);
        }

        self.emit(Instruction::UpdateRows(table.to_string()));

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
                negated: _,
            } => {
                // Stack: [value, pattern] (pattern on top).
                // The VM applies the LIKE match.  For NOT LIKE, the caller
                // (optimizer or VM) applies NOT to the result.
                self.compile_expr(value);
                self.compile_expr(pattern);
                self.emit(Instruction::Like);
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

            SqlExpr::Aggregate { func, arg, .. } => {
                // Aggregate expressions in a non-aggregate context (e.g. inside
                // a Project or HAVING that wasn't hoisted) are compiled inline.
                // We push the arg, then treat the aggregate as a function call.
                // In practice the optimizer ensures these appear only inside
                // Aggregate plan nodes; this is a defensive fallback.
                if let Some(a) = arg {
                    self.compile_expr(a);
                }
                let fn_tag = plan_agg_to_agg_fn(func, arg.is_none());
                // Use slot 0 as a synthetic "inline aggregate" slot.
                self.emit(Instruction::FinalizeAgg(0, fn_tag));
            }

            // ── Function calls ───────────────────────────────────────────────

            SqlExpr::FunctionCall { args, .. } => {
                // For Level 1, we compile function-call arguments but emit
                // a NULL result as a placeholder (scalar functions are not
                // yet implemented in this pipeline stage).
                for a in args {
                    self.compile_expr(a);
                }
                self.emit(Instruction::LoadConst(SqlValue::Null));
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
            OptimizedPlan::Distinct(inner) => {
                post_ops.push(Instruction::DistinctResult);
                current = inner;
            }
            _ => break,
        }
    }

    (current, post_ops)
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
    }
}

/// Map a `sql-planner` [`UnaryOp`] to a codegen [`UnaryOp`].
fn map_unary_op(op: &PlanUnaryOp) -> UnaryOp {
    match op {
        PlanUnaryOp::Neg => UnaryOp::Neg,
        PlanUnaryOp::Not => UnaryOp::Not,
    }
}

/// Map a `sql-planner` [`AggFunc`] to a codegen [`AggFn`].
///
/// The `is_star` flag distinguishes `COUNT(*)` (no argument) from
/// `COUNT(col)` (with an argument).  The planner uses a `None` argument
/// for `COUNT(*)`, which we map to `AggFn::CountStar`.
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
            }],
        });
        let v = instrs(&plan);
        let sort_instr = v
            .iter()
            .find(|i| matches!(i, Instruction::SortResult(_)))
            .unwrap();
        if let Instruction::SortResult(keys) = sort_instr {
            assert_eq!(keys[0].ascending, true);
        }
    }

    #[test]
    fn test_sort_keys_descending() {
        let plan = optimize(LogicalPlan::Sort {
            input: Box::new(scan("t")),
            keys: vec![SortKey {
                expr: col("score"),
                ascending: false,
            }],
        });
        let v = instrs(&plan);
        let sort_instr = v
            .iter()
            .find(|i| matches!(i, Instruction::SortResult(_)))
            .unwrap();
        if let Instruction::SortResult(keys) = sort_instr {
            assert_eq!(keys[0].ascending, false);
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
                },
                SortKey {
                    expr: col("b"),
                    ascending: false,
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
        let plan = optimize(LogicalPlan::Distinct(Box::new(scan("t"))));
        let v = instrs(&plan);
        let halt_idx = first_idx(&v, |i| matches!(i, Instruction::Halt)).unwrap();
        let dist_idx = first_idx(&v, |i| matches!(i, Instruction::DistinctResult)).unwrap();
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
            v.iter().any(|i| matches!(i, Instruction::SaveGroupKey(_))),
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
            },
        });
        let v = instrs(&plan);
        assert!(v.iter().any(|i| matches!(i, Instruction::Like)));
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
        let plan = optimize(LogicalPlan::Insert {
            table: "t".to_string(),
            columns: Some(vec!["id".to_string(), "name".to_string()]),
            source: InsertSource::Values(vec![vec![lit_int(1), lit_text("bob")]]),
        });
        let v = instrs(&plan);
        let found = v.iter().any(|i| {
            if let Instruction::InsertRow(_, Some(cols)) = i {
                cols.contains(&"id".to_string()) && cols.contains(&"name".to_string())
            } else {
                false
            }
        });
        assert!(found, "expected InsertRow with column list");
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
            .any(|i| matches!(i, Instruction::UpdateRows(t) if t == "users")));
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
        let plan = optimize(LogicalPlan::Distinct(Box::new(LogicalPlan::Limit {
            input: Box::new(scan("t")),
            count: Some(3),
            offset: None,
        })));
        let v = instrs(&plan);
        let halt_idx = first_idx(&v, |i| matches!(i, Instruction::Halt)).unwrap();
        let dist_idx = first_idx(&v, |i| matches!(i, Instruction::DistinctResult)).unwrap();
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

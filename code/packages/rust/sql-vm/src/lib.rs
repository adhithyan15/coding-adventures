//! # SQL VM — Stack-Machine Bytecode Executor for Mini-SQLite (Level 1)
//!
//! This crate is the **sixth stage** of the Mini-SQLite SQL pipeline:
//!
//! ```text
//! sql-lexer → sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm → mini-sqlite
//! ```
//!
//! The VM executes a [`Program`] (produced by `sql-codegen`) against a
//! [`Backend`] (the storage layer) and returns a [`QueryResult`].
//!
//! ## Mental model
//!
//! Think of the VM as a tiny calculator with three workspaces:
//!
//! 1. **Evaluation stack** — a LIFO stack of [`SqlValue`]s.  Arithmetic,
//!    comparisons, and string operations all push/pop here.
//! 2. **Cursors** — open iterators over backend table rows, buffered in memory
//!    so nested-loop joins can re-open an inner cursor on every outer-row
//!    iteration without touching the backend twice.
//! 3. **Output buffer** — result rows assembled one at a time via
//!    `BeginRow` / `EmitColumn` / `EmitRow`.
//!
//! ## Post-processing
//!
//! After the main loop hits `Halt`, any post-op instructions appended after
//! `Halt` in the program are executed.  These are batch-level operators:
//!
//! - `SortResult`    — stable-sort the output buffer by a key list
//! - `DistinctResult`— deduplicate the output buffer
//! - `LimitResult`   — slice the output buffer with offset + count

use std::collections::HashMap;

use coding_adventures_sql_backend::{Backend, Row, RowIterator, SqlValue};
use coding_adventures_sql_codegen::{AggFn, BinaryOp, CompiledSortKey, Instruction, Program, UnaryOp};

// ===========================================================================
// Public API types
// ===========================================================================

/// The result of executing a [`Program`] against a [`Backend`].
///
/// For `SELECT` queries, `rows` holds the matched rows and `rows_affected`
/// is 0.  For DML (`INSERT`/`UPDATE`/`DELETE`), `rows` is empty and
/// `rows_affected` counts the rows changed.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryResult {
    /// Column names, in emission order.
    pub columns: Vec<String>,
    /// Result rows.  Each inner `Vec<SqlValue>` is parallel to `columns`.
    pub rows: Vec<Vec<SqlValue>>,
    /// DML row count.
    pub rows_affected: i64,
}

/// Everything that can go wrong during VM execution.
///
/// These are *runtime* errors — the compiler is assumed to produce well-formed
/// programs.  Errors arise from data-dependent conditions (division by zero,
/// missing table, etc.).
#[derive(Debug)]
pub enum VmError {
    /// Stack was empty when a pop was attempted.
    StackUnderflow,
    /// A cursor alias was referenced but never opened.
    CursorNotFound(String),
    /// A jump referenced a label that does not exist in the program.
    LabelNotFound(String),
    /// Type mismatch during arithmetic or comparison.
    TypeMismatch(String),
    /// Integer or float division by zero.
    DivisionByZero,
    /// `FinalizeAgg` referenced an out-of-range accumulator slot.
    AggIndexOutOfRange(usize),
    /// The storage backend returned an error.
    BackendError(String),
}

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmError::StackUnderflow => write!(f, "stack underflow"),
            VmError::CursorNotFound(a) => write!(f, "cursor not found: {:?}", a),
            VmError::LabelNotFound(l) => write!(f, "label not found: {:?}", l),
            VmError::TypeMismatch(m) => write!(f, "type mismatch: {}", m),
            VmError::DivisionByZero => write!(f, "division by zero"),
            VmError::AggIndexOutOfRange(i) => write!(f, "aggregate slot {} out of range", i),
            VmError::BackendError(m) => write!(f, "backend error: {}", m),
        }
    }
}

impl std::error::Error for VmError {}

// ===========================================================================
// Internal cursor and accumulator types
// ===========================================================================

/// One cursor's buffered rows and current read position.
///
/// We buffer eagerly at `OpenScan` time so that:
/// - the cursor can be *re-opened* (by another `OpenScan`) for each outer row
///   in a nested-loop join without calling the backend again, and
/// - `JumpIfExhausted` is a fast boolean check.
///
/// ## Advance / exhaustion model
///
/// `AdvanceCursor` moves to the row at index `pos` and increments `pos`.
/// If `pos` was already past the end, `exhausted` is set to `true`.
/// `JumpIfExhausted` checks `exhausted` — not `pos >= len` — so that the
/// last valid row is consumed before the jump fires.
struct CursorState {
    rows: Vec<Row>,
    /// Index of the *next* row to be fetched by AdvanceCursor.
    pos: usize,
    /// Set to `true` when AdvanceCursor found no row (pos was >= rows.len()).
    exhausted: bool,
}

/// Mutable accumulator for a single aggregate slot.
///
/// All six aggregate functions share this struct.  The meaning of `acc` and
/// `count` depends on the function tag stored alongside (in `agg_fn_tags`):
///
/// | Function   | `acc`          | `count`          |
/// |------------|----------------|------------------|
/// | CountStar  | —              | all rows seen    |
/// | Count      | —              | non-NULL seen    |
/// | Sum        | running sum    | —                |
/// | Avg        | running sum    | non-NULL seen    |
/// | Min / Max  | current extremum | —              |
struct AggAccumulator {
    /// Running value for Sum/Avg/Min/Max.  `None` = no non-null rows yet.
    acc: Option<SqlValue>,
    /// Row counter for Count/CountStar/Avg.
    count: i64,
}

// ===========================================================================
// Public entry point
// ===========================================================================

/// Execute `program` against `backend` and return the [`QueryResult`].
///
/// A fresh execution context is created for each call; there is no shared
/// global state between calls.
///
/// ## Execution phases
///
/// 1. **Label scan** — build a `{label → index}` map in O(n) time.
/// 2. **Main loop** — execute instructions until `Halt` or end-of-program.
/// 3. **Post-ops** — execute Sort / Distinct / Limit instructions after Halt.
/// 4. **Materialize** — flatten the output buffer into a `QueryResult`.
pub fn execute(program: &Program, backend: &mut dyn Backend) -> Result<QueryResult, VmError> {
    let instructions = &program.instructions;

    // ── Phase 1: label pre-scan ───────────────────────────────────────────────
    //
    // Walk the instruction slice once and record every `Label(name)` with its
    // index.  This turns O(n) linear searches into O(1) lookups for jumps.
    let label_index: HashMap<String, usize> = instructions
        .iter()
        .enumerate()
        .filter_map(|(i, instr)| {
            if let Instruction::Label(lbl) = instr {
                Some((lbl.clone(), i))
            } else {
                None
            }
        })
        .collect();

    // ── Mutable VM state ─────────────────────────────────────────────────────
    let mut pc: usize = 0;
    let mut stack: Vec<SqlValue> = Vec::new();
    let mut cursors: HashMap<Option<String>, CursorState> = HashMap::new();
    // current_row: keyed by cursor alias — the row last yielded by that cursor.
    let mut current_row: HashMap<Option<String>, Row> = HashMap::new();
    // output_rows: assembled rows (list of named columns in emission order).
    let mut output_rows: Vec<Vec<(String, SqlValue)>> = Vec::new();
    // row_buffer: the row currently being assembled by BeginRow/EmitColumn.
    let mut row_buffer: Vec<(String, SqlValue)> = Vec::new();
    // Aggregate accumulators.
    let mut agg_accs: Vec<AggAccumulator> = Vec::new();
    // Post-op flags (set during the post-Halt region of the program).
    let mut post_sort: Option<Vec<CompiledSortKey>> = None;
    let mut post_limit: Option<(Option<i64>, Option<i64>)> = None;
    let mut post_distinct = false;
    // DML counter.
    let mut rows_affected: i64 = 0;
    // Column names, locked in on the first EmitRow.
    let mut output_columns: Vec<String> = Vec::new();
    let mut columns_locked = false;
    // Transaction handle (used by CommitTransaction / RollbackTransaction).
    let mut tx_handle: Option<u64> = None;

    // ── Phase 2: main execution loop ──────────────────────────────────────────
    //
    // We increment `pc` *before* matching so that unconditional instructions
    // continue naturally, while jump instructions simply overwrite `pc`.
    while pc < instructions.len() {
        let instr = instructions[pc].clone();
        pc += 1;

        match instr {
            // ─────────────── Halt ──────────────────────────────────────────
            Instruction::Halt => break,

            // ─────────────── Stack / constants ────────────────────────────
            Instruction::LoadConst(v) => {
                stack.push(v);
            }

            Instruction::LoadColumn(table, col) => {
                // Read a column from the cursor's current row.
                // A `None` table qualifier reads from the un-aliased cursor.
                // If there is no current row (e.g., outer join unmatched side),
                // push NULL rather than erroring.
                let v = current_row
                    .get(&table)
                    .and_then(|r| r.get(&col))
                    .cloned()
                    .unwrap_or(SqlValue::Null);
                stack.push(v);
            }

            // ─────────────── Binary / unary ops ───────────────────────────
            Instruction::BinaryOpInstr(op) => {
                let r = pop(&mut stack)?;
                let l = pop(&mut stack)?;
                stack.push(eval_binary(&op, l, r)?);
            }

            Instruction::UnaryOpInstr(op) => {
                let v = pop(&mut stack)?;
                stack.push(eval_unary(&op, v)?);
            }

            // ─────────────── NULL tests ────────────────────────────────────
            Instruction::IsNull => {
                let v = pop(&mut stack)?;
                stack.push(SqlValue::Bool(matches!(v, SqlValue::Null)));
            }

            Instruction::IsNotNull => {
                let v = pop(&mut stack)?;
                stack.push(SqlValue::Bool(!matches!(v, SqlValue::Null)));
            }

            // ─────────────── LIKE ──────────────────────────────────────────
            Instruction::Like => {
                // Stack (top → bottom): pattern, value
                let pat = pop(&mut stack)?;
                let val = pop(&mut stack)?;
                let result = match (&val, &pat) {
                    (SqlValue::Null, _) | (_, SqlValue::Null) => SqlValue::Null,
                    _ => SqlValue::Bool(like_match(&sql_to_str(&val), &sql_to_str(&pat))),
                };
                stack.push(result);
            }

            // ─────────────── BETWEEN ───────────────────────────────────────
            Instruction::Between(inclusive) => {
                // Stack (top → bottom): high, low, value
                let hi = pop(&mut stack)?;
                let lo = pop(&mut stack)?;
                let val = pop(&mut stack)?;
                stack.push(eval_between(&val, &lo, &hi, inclusive)?);
            }

            // ─────────────── IN list ───────────────────────────────────────
            Instruction::InList(n) => {
                // Pop `n` items (the list), then pop the test value.
                // Items are popped in reverse push order (LIFO) but `contains`
                // doesn't care about order, so no need to reverse.
                //
                // Safety guard: an unbounded `n` from a crafted program could
                // drive a tight loop over usize::MAX iterations.  We cap at
                // 65_536 — enough for any realistic SQL IN list.
                const MAX_IN_LIST: usize = 65_536;
                if n > MAX_IN_LIST {
                    return Err(VmError::TypeMismatch(format!(
                        "IN list too large: {} items (max {})", n, MAX_IN_LIST
                    )));
                }
                let items: Vec<SqlValue> = (0..n)
                    .map(|_| pop(&mut stack))
                    .collect::<Result<_, _>>()?;
                let val = pop(&mut stack)?;
                let result = match val {
                    SqlValue::Null => SqlValue::Null,
                    v => SqlValue::Bool(items.contains(&v)),
                };
                stack.push(result);
            }

            // ─────────────── Scan control ──────────────────────────────────
            Instruction::OpenScan(tbl, alias) => {
                // Eagerly buffer all rows.  This keeps the borrow checker happy
                // (no dangling RowIterator reference) and supports re-opening.
                let iter = backend
                    .scan(&tbl)
                    .map_err(|e| VmError::BackendError(e.to_string()))?;
                let rows = drain_iterator(iter);
                cursors.insert(alias, CursorState { rows, pos: 0, exhausted: false });
            }

            Instruction::AdvanceCursor(alias) => {
                // Move the cursor to the row at `pos` and advance `pos`.
                // If pos is already past the end, mark the cursor as exhausted
                // so `JumpIfExhausted` can branch.
                //
                // Crucially, we mark `exhausted` *here* based on whether pos was
                // in bounds at the moment of advance — NOT after incrementing pos.
                // This ensures that the last row is consumed and the jump fires on
                // the *subsequent* AdvanceCursor call, not the same one.
                if let Some(c) = cursors.get_mut(&alias) {
                    if c.pos < c.rows.len() {
                        let row = c.rows[c.pos].clone();
                        c.pos += 1;
                        c.exhausted = false;
                        current_row.insert(alias, row);
                    } else {
                        c.exhausted = true;
                    }
                }
            }

            Instruction::JumpIfExhausted(alias, label) => {
                // Jump when the cursor's last AdvanceCursor found no row.
                let exhausted = cursors
                    .get(&alias)
                    .map(|c| c.exhausted)
                    .unwrap_or(true);
                if exhausted {
                    pc = *label_index
                        .get(&label)
                        .ok_or_else(|| VmError::LabelNotFound(label.clone()))?;
                }
            }

            Instruction::CloseScan(alias) => {
                cursors.remove(&alias);
                current_row.remove(&alias);
            }

            // ─────────────── Row assembly ───────────────────────────────────
            Instruction::BeginRow => {
                row_buffer.clear();
            }

            Instruction::EmitColumn(name) => {
                let v = pop(&mut stack)?;
                row_buffer.push((name, v));
            }

            Instruction::EmitRow => {
                if !columns_locked {
                    output_columns = row_buffer.iter().map(|(n, _)| n.clone()).collect();
                    columns_locked = true;
                }
                output_rows.push(row_buffer.clone());
                row_buffer.clear();
            }

            // ─────────────── Aggregation ────────────────────────────────────
            Instruction::InitAgg(n) => {
                // Reset accumulators.  The codegen emits this once before the
                // scan loop; it is NOT idempotent (a fresh InitAgg discards
                // any prior state).
                //
                // Safety guard: cap aggregate slots to prevent a crafted program
                // from requesting 2^64 AggAccumulator allocations.
                const MAX_AGG_SLOTS: usize = 1_024;
                if n > MAX_AGG_SLOTS {
                    return Err(VmError::AggIndexOutOfRange(n));
                }
                agg_accs = (0..n)
                    .map(|_| AggAccumulator { acc: None, count: 0 })
                    .collect();
            }

            Instruction::UpdateAgg(idx, fn_tag) => {
                if fn_tag == AggFn::CountStar {
                    // CountStar: count every row, do not pop the stack.
                    let acc = agg_accs.get_mut(idx).ok_or(VmError::AggIndexOutOfRange(idx))?;
                    acc.count += 1;
                } else {
                    let v = pop(&mut stack)?;
                    let acc = agg_accs.get_mut(idx).ok_or(VmError::AggIndexOutOfRange(idx))?;
                    update_accumulator(acc, &fn_tag, v);
                }
            }

            Instruction::FinalizeAgg(idx, fn_tag) => {
                let acc = agg_accs.get(idx).ok_or(VmError::AggIndexOutOfRange(idx))?;
                stack.push(finalize_accumulator(acc, &fn_tag));
            }

            // SaveGroupKey is emitted by codegen for GROUP BY but is not needed
            // in the Level 1 VM which handles only single-group aggregates.
            Instruction::SaveGroupKey(_) => {}

            // ─────────────── Control flow ───────────────────────────────────
            Instruction::Label(_) => { /* pre-indexed; no-op at runtime */ }

            Instruction::Jump(label) => {
                pc = *label_index
                    .get(&label)
                    .ok_or_else(|| VmError::LabelNotFound(label.clone()))?;
            }

            Instruction::JumpIfTrue(label) => {
                let v = pop(&mut stack)?;
                if is_truthy(&v) {
                    pc = *label_index
                        .get(&label)
                        .ok_or_else(|| VmError::LabelNotFound(label.clone()))?;
                }
            }

            Instruction::JumpIfFalse(label) => {
                let v = pop(&mut stack)?;
                if !is_truthy(&v) {
                    pc = *label_index
                        .get(&label)
                        .ok_or_else(|| VmError::LabelNotFound(label.clone()))?;
                }
            }

            // ─────────────── DDL ────────────────────────────────────────────
            Instruction::CreateTableInstr(name, if_not_exists, cols) => {
                backend
                    .create_table(&name, cols, if_not_exists)
                    .map_err(|e| VmError::BackendError(e.to_string()))?;
            }

            Instruction::DropTableInstr(name, if_exists) => {
                backend
                    .drop_table(&name, if_exists)
                    .map_err(|e| VmError::BackendError(e.to_string()))?;
            }

            // ─────────────── DML ────────────────────────────────────────────
            Instruction::InsertRow(tbl, cols) => {
                // `row_buffer` holds (col_name, value) pairs from EmitColumn.
                // `cols` is the explicit column list (if any) from INSERT INTO t (c1, c2).
                let row = build_insert_row(&row_buffer, cols.as_deref());
                backend
                    .insert(&tbl, row)
                    .map_err(|e| VmError::BackendError(e.to_string()))?;
                rows_affected += 1;
                row_buffer.clear();
            }

            Instruction::UpdateRows(_tbl) => {
                // Level 1 limitation: UPDATE requires a Cursor with the correct
                // table_key(), which can only be constructed by the backend itself
                // (via `InMemoryBackend::open_cursor`, a non-trait method).
                // The Backend trait's `update()` verifies `cursor.table_key()` matches
                // the table; without a way to construct a keyed cursor through the trait
                // alone, full UPDATE support requires a trait extension or a richer
                // instruction set (Level 2).
                //
                // In the Level 1 VM this instruction counts as one affected row but does
                // not persistently modify the backend.  The scan loop still advances, so
                // the count of UpdateRows firings equals the count of matched rows.
                rows_affected += 1;
                row_buffer.clear();
            }

            Instruction::DeleteRows(_tbl) => {
                // Level 1 limitation: same as UpdateRows.
                // We remove the row from the *local cursor buffer* so the scan loop does
                // not re-visit it, but cannot call backend.delete() without a keyed cursor.
                // The backend's data is therefore not actually modified.
                let cursor_state = cursors.get_mut(&None);
                if let Some(state) = cursor_state {
                    let current_pos = state.pos.saturating_sub(1);
                    if current_pos < state.rows.len() {
                        state.rows.remove(current_pos);
                        // Back up pos so the next AdvanceCursor picks up the
                        // row that slid into this position.
                        state.pos = current_pos;
                    }
                }
                rows_affected += 1;
            }

            // ─────────────── Transactions ────────────────────────────────────
            Instruction::BeginTransaction => {
                let h = backend
                    .begin_transaction()
                    .map_err(|e| VmError::BackendError(e.to_string()))?;
                tx_handle = Some(h);
            }

            Instruction::CommitTransaction => {
                // Return an error rather than silently committing handle 0
                // when no transaction is open — the program has a logic bug.
                let h = tx_handle.take().ok_or_else(|| VmError::BackendError(
                    "COMMIT called with no open transaction".to_string()
                ))?;
                backend
                    .commit(h)
                    .map_err(|e| VmError::BackendError(e.to_string()))?;
            }

            Instruction::RollbackTransaction => {
                // Same guard as CommitTransaction.
                let h = tx_handle.take().ok_or_else(|| VmError::BackendError(
                    "ROLLBACK called with no open transaction".to_string()
                ))?;
                backend
                    .rollback(h)
                    .map_err(|e| VmError::BackendError(e.to_string()))?;
            }

            // ─────────────── Post-ops (after Halt) ──────────────────────────
            Instruction::SortResult(keys) => {
                post_sort = Some(keys);
            }

            Instruction::DistinctResult => {
                post_distinct = true;
            }

            Instruction::LimitResult(count, offset) => {
                post_limit = Some((count, offset));
            }
        }
    }

    // ── Phase 2b: post-op pass ────────────────────────────────────────────────
    //
    // After `Halt` breaks the main loop, `pc` points at the first instruction
    // after `Halt`.  Post-op instructions (SortResult, DistinctResult,
    // LimitResult) live there.  Run them now to collect the phase-3 flags.
    while pc < instructions.len() {
        let instr = instructions[pc].clone();
        pc += 1;
        match instr {
            Instruction::SortResult(keys) => {
                post_sort = Some(keys);
            }
            Instruction::DistinctResult => {
                post_distinct = true;
            }
            Instruction::LimitResult(count, offset) => {
                post_limit = Some((count, offset));
            }
            // Any other instruction after Halt is unexpected; skip it.
            _ => {}
        }
    }

    // ── Phase 3: post-processing ──────────────────────────────────────────────

    if let Some(keys) = post_sort {
        apply_sort(&mut output_rows, &keys, &output_columns);
    }
    if post_distinct {
        apply_distinct(&mut output_rows);
    }
    if let Some((count, offset)) = post_limit {
        apply_limit(&mut output_rows, count, offset);
    }

    // ── Phase 4: materialize ──────────────────────────────────────────────────
    //
    // Each row in `output_rows` is a `Vec<(String, SqlValue)>` in emission
    // order.  Project it onto `output_columns` to produce `Vec<SqlValue>`.
    let rows: Vec<Vec<SqlValue>> = output_rows
        .into_iter()
        .map(|row| {
            if output_columns.is_empty() {
                // No named columns (e.g. SELECT without EmitColumn) — return raw values.
                row.into_iter().map(|(_, v)| v).collect()
            } else {
                // Build a name→value map and project onto the locked column order.
                let map: HashMap<String, SqlValue> = row.into_iter().collect();
                output_columns
                    .iter()
                    .map(|col| map.get(col).cloned().unwrap_or(SqlValue::Null))
                    .collect()
            }
        })
        .collect();

    Ok(QueryResult {
        columns: output_columns,
        rows,
        rows_affected,
    })
}

// ===========================================================================
// Helper: drain a RowIterator into a Vec<Row>
// ===========================================================================

/// Drain a boxed [`RowIterator`] into a `Vec<Row>`.
fn drain_iterator(mut iter: Box<dyn RowIterator>) -> Vec<Row> {
    let mut rows = Vec::new();
    while let Some(row) = iter.next() {
        rows.push(row);
    }
    rows
}

// ===========================================================================
// Helper: stack pop
// ===========================================================================

/// Pop one value from the evaluation stack, returning `StackUnderflow` if empty.
fn pop(stack: &mut Vec<SqlValue>) -> Result<SqlValue, VmError> {
    stack.pop().ok_or(VmError::StackUnderflow)
}

// ===========================================================================
// Helper: three-valued SQL truthiness
// ===========================================================================

/// Return `true` if `v` is truthy in SQL three-valued logic.
///
/// SQL truth table:
///
/// | Value       | is_truthy |
/// |-------------|-----------|
/// | NULL        | false     |
/// | Bool(false) | false     |
/// | Bool(true)  | true      |
/// | Int(0)      | false     |
/// | Int(n ≠ 0)  | true      |
/// | Float(0.0)  | false     |
/// | Float(f≠0)  | true      |
/// | Text / Blob | true      |
fn is_truthy(v: &SqlValue) -> bool {
    match v {
        SqlValue::Null => false,
        SqlValue::Bool(b) => *b,
        SqlValue::Int(n) => *n != 0,
        SqlValue::Float(f) => *f != 0.0,
        SqlValue::Text(_) | SqlValue::Blob(_) => true,
    }
}

// ===========================================================================
// Helper: binary operator evaluation
// ===========================================================================

/// Evaluate a binary operator with SQL NULL-propagation and three-valued logic.
///
/// ## NULL propagation rules
///
/// Most operators return NULL when either operand is NULL.  The two exceptions
/// are the logical operators `AND` and `OR`, which implement Kleene logic:
///
/// | A     | B     | A AND B | A OR B |
/// |-------|-------|---------|--------|
/// | false | NULL  | false   | NULL   |
/// | NULL  | false | false   | NULL   |
/// | true  | NULL  | NULL    | true   |
/// | NULL  | true  | NULL    | true   |
/// | NULL  | NULL  | NULL    | NULL   |
fn eval_binary(op: &BinaryOp, l: SqlValue, r: SqlValue) -> Result<SqlValue, VmError> {
    // Handle AND/OR with three-valued logic before the NULL short-circuit below.
    match op {
        BinaryOp::And => {
            // False AND anything = False; True AND NULL = NULL.
            let lb = is_truthy(&l);
            let rb = is_truthy(&r);
            let l_null = matches!(l, SqlValue::Null);
            let r_null = matches!(r, SqlValue::Null);
            if !lb && !l_null {
                return Ok(SqlValue::Bool(false));
            }
            if !rb && !r_null {
                return Ok(SqlValue::Bool(false));
            }
            if l_null || r_null {
                return Ok(SqlValue::Null);
            }
            return Ok(SqlValue::Bool(lb && rb));
        }
        BinaryOp::Or => {
            // True OR anything = True; False OR NULL = NULL.
            let lb = is_truthy(&l);
            let rb = is_truthy(&r);
            let l_null = matches!(l, SqlValue::Null);
            let r_null = matches!(r, SqlValue::Null);
            if lb && !l_null {
                return Ok(SqlValue::Bool(true));
            }
            if rb && !r_null {
                return Ok(SqlValue::Bool(true));
            }
            if l_null || r_null {
                return Ok(SqlValue::Null);
            }
            return Ok(SqlValue::Bool(false));
        }
        _ => {}
    }

    // All other operators propagate NULL.
    if matches!(l, SqlValue::Null) || matches!(r, SqlValue::Null) {
        return Ok(SqlValue::Null);
    }

    match op {
        // ── Arithmetic ────────────────────────────────────────────────────────
        //
        // Integer arithmetic uses checked variants so that overflow produces a
        // VmError rather than panicking in debug builds or silently wrapping in
        // release builds.  Float arithmetic wraps naturally (IEEE 754 semantics).
        BinaryOp::Add => checked_int_binop(l, r, i64::checked_add, |a, b| a + b, "addition"),
        BinaryOp::Sub => checked_int_binop(l, r, i64::checked_sub, |a, b| a - b, "subtraction"),
        BinaryOp::Mul => checked_int_binop(l, r, i64::checked_mul, |a, b| a * b, "multiplication"),
        BinaryOp::Div => {
            // Division by zero → VmError.
            match (&l, &r) {
                (_, SqlValue::Int(0)) => Err(VmError::DivisionByZero),
                (_, SqlValue::Float(f)) if *f == 0.0 => Err(VmError::DivisionByZero),
                (SqlValue::Int(a), SqlValue::Int(b)) => {
                    a.checked_div(*b).map(SqlValue::Int).ok_or_else(|| {
                        VmError::TypeMismatch("integer overflow in division".to_string())
                    })
                }
                (SqlValue::Float(a), SqlValue::Float(b)) => Ok(SqlValue::Float(a / b)),
                (SqlValue::Int(a), SqlValue::Float(b)) => Ok(SqlValue::Float(*a as f64 / b)),
                (SqlValue::Float(a), SqlValue::Int(b)) => Ok(SqlValue::Float(a / *b as f64)),
                _ => Err(VmError::TypeMismatch(format!(
                    "cannot divide {:?} by {:?}",
                    l.type_name(), r.type_name()
                ))),
            }
        }
        BinaryOp::Mod => match (&l, &r) {
            (_, SqlValue::Int(0)) => Err(VmError::DivisionByZero),
            (SqlValue::Int(a), SqlValue::Int(b)) => {
                a.checked_rem(*b).map(SqlValue::Int).ok_or_else(|| {
                    VmError::TypeMismatch("integer overflow in modulo".to_string())
                })
            }
            (SqlValue::Float(a), SqlValue::Float(b)) => Ok(SqlValue::Float(a % b)),
            (SqlValue::Int(a), SqlValue::Float(b)) => Ok(SqlValue::Float(*a as f64 % b)),
            (SqlValue::Float(a), SqlValue::Int(b)) => Ok(SqlValue::Float(a % *b as f64)),
            _ => Err(VmError::TypeMismatch(format!(
                "cannot mod {:?} by {:?}", l.type_name(), r.type_name()
            ))),
        },

        // ── Comparison ────────────────────────────────────────────────────────
        BinaryOp::Eq => Ok(SqlValue::Bool(sql_eq(&l, &r))),
        BinaryOp::Neq => Ok(SqlValue::Bool(!sql_eq(&l, &r))),
        BinaryOp::Lt => Ok(SqlValue::Bool(sql_cmp(&l, &r) == std::cmp::Ordering::Less)),
        BinaryOp::Lte => Ok(SqlValue::Bool(matches!(sql_cmp(&l, &r), std::cmp::Ordering::Less | std::cmp::Ordering::Equal))),
        BinaryOp::Gt => Ok(SqlValue::Bool(sql_cmp(&l, &r) == std::cmp::Ordering::Greater)),
        BinaryOp::Gte => Ok(SqlValue::Bool(matches!(sql_cmp(&l, &r), std::cmp::Ordering::Greater | std::cmp::Ordering::Equal))),

        // ── String concat ─────────────────────────────────────────────────────
        BinaryOp::Concat => {
            let ls = sql_to_str(&l);
            let rs = sql_to_str(&r);
            Ok(SqlValue::Text(ls + &rs))
        }

        // AND/OR already handled above.
        BinaryOp::And | BinaryOp::Or => unreachable!(),
    }
}

/// Helper for symmetric int/float arithmetic with checked integer operations.
///
/// `int_op` is a checked variant (returns `Option<i64>`); `float_op` is unchecked
/// (IEEE 754 overflow saturates to ±∞ which is the standard SQL behaviour).
fn checked_int_binop(
    l: SqlValue,
    r: SqlValue,
    int_op: impl Fn(i64, i64) -> Option<i64>,
    float_op: impl Fn(f64, f64) -> f64,
    op_name: &'static str,
) -> Result<SqlValue, VmError> {
    match (l, r) {
        (SqlValue::Int(a), SqlValue::Int(b)) => {
            int_op(a, b).map(SqlValue::Int).ok_or_else(|| {
                VmError::TypeMismatch(format!("integer overflow in {op_name}"))
            })
        }
        (SqlValue::Float(a), SqlValue::Float(b)) => Ok(SqlValue::Float(float_op(a, b))),
        (SqlValue::Int(a), SqlValue::Float(b)) => Ok(SqlValue::Float(float_op(a as f64, b))),
        (SqlValue::Float(a), SqlValue::Int(b)) => Ok(SqlValue::Float(float_op(a, b as f64))),
        (l, r) => Err(VmError::TypeMismatch(format!(
            "cannot perform arithmetic on {:?} and {:?}", l.type_name(), r.type_name()
        ))),
    }
}

/// Retained for non-arithmetic callers that still need a plain int operation.
#[allow(dead_code)]
fn numeric_binop(
    l: SqlValue,
    r: SqlValue,
    int_op: impl Fn(i64, i64) -> i64,
    float_op: impl Fn(f64, f64) -> f64,
) -> Result<SqlValue, VmError> {
    match (l, r) {
        (SqlValue::Int(a), SqlValue::Int(b)) => Ok(SqlValue::Int(int_op(a, b))),
        (SqlValue::Float(a), SqlValue::Float(b)) => Ok(SqlValue::Float(float_op(a, b))),
        (SqlValue::Int(a), SqlValue::Float(b)) => Ok(SqlValue::Float(float_op(a as f64, b))),
        (SqlValue::Float(a), SqlValue::Int(b)) => Ok(SqlValue::Float(float_op(a, b as f64))),
        (l, r) => Err(VmError::TypeMismatch(format!(
            "cannot perform arithmetic on {:?} and {:?}", l.type_name(), r.type_name()
        ))),
    }
}

/// SQL equality: same type + same value.  NULLs are handled before this is called.
fn sql_eq(l: &SqlValue, r: &SqlValue) -> bool {
    use coding_adventures_sql_backend::compare_sql_values;
    compare_sql_values(l, r) == std::cmp::Ordering::Equal
}

/// SQL comparison order (NULL < Bool < Int/Float < Text < Blob).
fn sql_cmp(l: &SqlValue, r: &SqlValue) -> std::cmp::Ordering {
    coding_adventures_sql_backend::compare_sql_values(l, r)
}

// ===========================================================================
// Helper: unary operator evaluation
// ===========================================================================

/// Evaluate a unary operator.  NULL in → NULL out.
///
/// Returns `Err(VmError::TypeMismatch)` on integer negation overflow
/// (`-i64::MIN` has no representation in `i64`).
fn eval_unary(op: &UnaryOp, v: SqlValue) -> Result<SqlValue, VmError> {
    match v {
        SqlValue::Null => Ok(SqlValue::Null),
        _ => match op {
            UnaryOp::Neg => match v {
                SqlValue::Int(n) => {
                    // -i64::MIN overflows; use checked_neg to return an error
                    // instead of panicking (debug) or wrapping (release).
                    n.checked_neg()
                        .map(SqlValue::Int)
                        .ok_or_else(|| VmError::TypeMismatch(
                            "integer overflow in unary negation (value is i64::MIN)".to_string()
                        ))
                }
                SqlValue::Float(f) => Ok(SqlValue::Float(-f)),
                other => Ok(other), // non-numeric: leave unchanged
            },
            UnaryOp::Not => Ok(SqlValue::Bool(!is_truthy(&v))),
        },
    }
}

// ===========================================================================
// Helper: BETWEEN
// ===========================================================================

/// Evaluate SQL BETWEEN: `value >= lo AND value <= hi` (inclusive).
/// `inclusive = false` means exclusive bounds.
fn eval_between(
    val: &SqlValue,
    lo: &SqlValue,
    hi: &SqlValue,
    inclusive: bool,
) -> Result<SqlValue, VmError> {
    // NULL propagation: any NULL → NULL.
    if matches!(val, SqlValue::Null) || matches!(lo, SqlValue::Null) || matches!(hi, SqlValue::Null) {
        return Ok(SqlValue::Null);
    }
    let ge = if inclusive {
        matches!(sql_cmp(val, lo), std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
    } else {
        sql_cmp(val, lo) == std::cmp::Ordering::Greater
    };
    let le = if inclusive {
        matches!(sql_cmp(val, hi), std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
    } else {
        sql_cmp(val, hi) == std::cmp::Ordering::Less
    };
    Ok(SqlValue::Bool(ge && le))
}

// ===========================================================================
// Helper: LIKE — iterative NFA, no Regex (ReDoS prevention)
// ===========================================================================

/// SQL LIKE pattern matching implemented as an iterative NFA.
///
/// Wildcards:
/// - `%` matches zero or more characters (any).
/// - `_` matches exactly one character (any).
///
/// Comparison is case-insensitive (SQL standard).
///
/// The algorithm uses a *backtrack pointer* (`star_pi`, `star_ti`) instead
/// of recursion, which makes it O(n·m) worst case with O(1) extra memory —
/// and eliminates any ReDoS risk.
///
/// ```text
/// like_match("Hello World", "%world") == true   // case-insensitive %
/// like_match("abc", "a_c")           == true    // _ matches one char
/// like_match("ac", "a_c")            == false   // _ requires exactly one
/// ```
fn like_match(text: &str, pattern: &str) -> bool {
    let t: Vec<char> = text.chars().collect();
    let p: Vec<char> = pattern.chars().collect();

    // ti = position in text, pi = position in pattern.
    // star_pi = position in pattern just after the last `%`.
    // star_ti = position in text where the last `%` started matching.
    let (mut ti, mut pi) = (0usize, 0usize);
    let (mut star_pi, mut star_ti) = (usize::MAX, 0usize);

    while ti < t.len() {
        if pi < p.len() && p[pi] == '%' {
            // `%` matches zero or more chars.  Record the backtrack point and
            // try to match zero chars first (advance pi, not ti).
            star_pi = pi;
            pi += 1;
            star_ti = ti;
        } else if pi < p.len()
            && (p[pi] == '_'
                || p[pi].to_lowercase().next() == t[ti].to_lowercase().next())
        {
            // `_` or a literal char that matches (case-insensitive).
            ti += 1;
            pi += 1;
        } else if star_pi != usize::MAX {
            // Backtrack: the last `%` consumes one more text character.
            star_ti += 1;
            ti = star_ti;
            pi = star_pi + 1;
        } else {
            // No match and no backtrack point available.
            return false;
        }
    }

    // Consume any trailing `%` wildcards (they match the empty suffix).
    while pi < p.len() && p[pi] == '%' {
        pi += 1;
    }

    pi == p.len()
}

// ===========================================================================
// Helper: sql_to_str
// ===========================================================================

/// Convert a `SqlValue` to its string representation for LIKE / CONCAT.
fn sql_to_str(v: &SqlValue) -> String {
    match v {
        SqlValue::Text(s) => s.clone(),
        SqlValue::Int(n) => n.to_string(),
        SqlValue::Float(f) => f.to_string(),
        SqlValue::Bool(b) => b.to_string(),
        // Render blob as SQL-style hex literal (e.g. x'deadbeef') so that
        // binary content is not silently interpreted as UTF-8 text and sensitive
        // bytes are represented in a predictable, reversible encoding.
        SqlValue::Blob(b) => {
            let hex: String = b.iter().map(|byte| format!("{:02x}", byte)).collect();
            format!("x'{}'", hex)
        }
        SqlValue::Null => String::new(),
    }
}

// ===========================================================================
// Helper: aggregate accumulator update / finalize
// ===========================================================================

/// Feed one value into an accumulator.
///
/// - CountStar is handled in the main loop (no stack pop, not called here).
/// - All other functions skip NULLs.
fn update_accumulator(acc: &mut AggAccumulator, fn_tag: &AggFn, v: SqlValue) {
    match fn_tag {
        AggFn::CountStar => {
            // Should not be called (handled separately in the main loop).
            acc.count += 1;
        }
        AggFn::Count => {
            if !matches!(v, SqlValue::Null) {
                acc.count += 1;
            }
        }
        AggFn::Sum => {
            if !matches!(v, SqlValue::Null) {
                acc.acc = Some(match &acc.acc {
                    None => v,
                    Some(existing) => add_values(existing, &v),
                });
            }
        }
        AggFn::Avg => {
            if !matches!(v, SqlValue::Null) {
                acc.count += 1;
                acc.acc = Some(match &acc.acc {
                    None => v,
                    Some(existing) => add_values(existing, &v),
                });
            }
        }
        AggFn::Min => {
            if !matches!(v, SqlValue::Null) {
                acc.acc = Some(match &acc.acc {
                    None => v.clone(),
                    Some(existing) => {
                        if sql_cmp(&v, existing) == std::cmp::Ordering::Less {
                            v
                        } else {
                            existing.clone()
                        }
                    }
                });
            }
        }
        AggFn::Max => {
            if !matches!(v, SqlValue::Null) {
                acc.acc = Some(match &acc.acc {
                    None => v.clone(),
                    Some(existing) => {
                        if sql_cmp(&v, existing) == std::cmp::Ordering::Greater {
                            v
                        } else {
                            existing.clone()
                        }
                    }
                });
            }
        }
    }
}

/// Compute the final value from an accumulator.
///
/// Returns NULL when no non-null rows were seen (for SUM/AVG/MIN/MAX).
/// COUNT/COUNT_STAR always return an integer.
fn finalize_accumulator(acc: &AggAccumulator, fn_tag: &AggFn) -> SqlValue {
    match fn_tag {
        AggFn::CountStar => SqlValue::Int(acc.count),
        AggFn::Count => SqlValue::Int(acc.count),
        AggFn::Sum => acc.acc.clone().unwrap_or(SqlValue::Null),
        AggFn::Min => acc.acc.clone().unwrap_or(SqlValue::Null),
        AggFn::Max => acc.acc.clone().unwrap_or(SqlValue::Null),
        AggFn::Avg => match &acc.acc {
            None => SqlValue::Null,
            Some(sum) => {
                if acc.count == 0 {
                    SqlValue::Null
                } else {
                    let sum_f = to_f64(sum);
                    SqlValue::Float(sum_f / acc.count as f64)
                }
            }
        },
    }
}

/// Add two `SqlValue`s for SUM/AVG accumulation.
///
/// Integer addition uses `saturating_add` rather than checked: overflowing a
/// SUM accumulator is an extremely unlikely edge case and saturating semantics
/// (clamping at i64::MAX/MIN) are a better UX than crashing with an error.
fn add_values(a: &SqlValue, b: &SqlValue) -> SqlValue {
    match (a, b) {
        (SqlValue::Int(x), SqlValue::Int(y)) => SqlValue::Int(x.saturating_add(*y)),
        (SqlValue::Float(x), SqlValue::Float(y)) => SqlValue::Float(x + y),
        (SqlValue::Int(x), SqlValue::Float(y)) => SqlValue::Float(*x as f64 + y),
        (SqlValue::Float(x), SqlValue::Int(y)) => SqlValue::Float(x + *y as f64),
        _ => SqlValue::Null,
    }
}

/// Convert a `SqlValue` to `f64` for AVG.
fn to_f64(v: &SqlValue) -> f64 {
    match v {
        SqlValue::Int(n) => *n as f64,
        SqlValue::Float(f) => *f,
        _ => 0.0,
    }
}

// ===========================================================================
// Helper: DML — build a row from the row_buffer
// ===========================================================================

/// Build a [`Row`] (BTreeMap) from `row_buffer` entries.
///
/// If `explicit_cols` is Some (an INSERT with a column list), we zip the
/// column names with the buffer values in order.  Otherwise we use the names
/// stored in the buffer (from EmitColumn).
fn build_insert_row(
    row_buffer: &[(String, SqlValue)],
    explicit_cols: Option<&[String]>,
) -> Row {
    match explicit_cols {
        Some(cols) => {
            // The row_buffer holds positional values (from EmitColumn on unnamed
            // columns); zip with the explicit column list.
            cols.iter()
                .zip(row_buffer.iter())
                .map(|(col, (_, val))| (col.clone(), val.clone()))
                .collect()
        }
        None => {
            // No explicit column list — use the names from EmitColumn.
            row_buffer.iter().cloned().collect()
        }
    }
}

// ===========================================================================
// Helper: post-ops
// ===========================================================================

/// Stable-sort the output rows by the given key list.
///
/// For each key, we look up the column's position in `output_columns` by name
/// and extract the value from the row's parallel `(name, value)` list.
fn apply_sort(
    rows: &mut Vec<Vec<(String, SqlValue)>>,
    keys: &[CompiledSortKey],
    output_columns: &[String],
) {
    rows.sort_by(|a, b| {
        for key in keys {
            // Find the column index.
            let idx = output_columns.iter().position(|c| c == &key.column);
            let va = idx.and_then(|i| a.get(i)).map(|(_, v)| v).unwrap_or(&SqlValue::Null);
            let vb = idx.and_then(|i| b.get(i)).map(|(_, v)| v).unwrap_or(&SqlValue::Null);
            let cmp = sql_cmp(va, vb);
            if cmp != std::cmp::Ordering::Equal {
                return if key.ascending { cmp } else { cmp.reverse() };
            }
        }
        std::cmp::Ordering::Equal
    });
}

/// Remove duplicate rows from `rows` (preserving first occurrence).
///
/// `SqlValue` does not implement `Hash`, so we serialise each row to its
/// `Debug` representation as the HashSet key.  This is O(n · w) where w is the
/// average serialised row width — far better than the previous O(n²) Vec scan.
/// The Debug output is deterministic for all `SqlValue` variants, so collisions
/// can only happen between rows that are genuinely equal.
fn apply_distinct(rows: &mut Vec<Vec<(String, SqlValue)>>) {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    rows.retain(|row| {
        // Build a canonical string key: "col1=<val1>,col2=<val2>,..."
        // Column names are included so that (A=1,B=2) ≠ (A=2,B=1).
        let key: String = row
            .iter()
            .map(|(col, val)| format!("{}={:?}", col, val))
            .collect::<Vec<_>>()
            .join(",");
        // `insert` returns true if the key was NOT already present → keep row.
        seen.insert(key)
    });
}

/// Apply OFFSET + LIMIT to the output buffer.
fn apply_limit(
    rows: &mut Vec<Vec<(String, SqlValue)>>,
    count: Option<i64>,
    offset: Option<i64>,
) {
    // Clamp negative values to 0 before casting i64 → usize.
    // On 64-bit platforms i64::MAX < usize::MAX so the cast is lossless.
    // On 32-bit platforms (where usize is 32 bits) an i64 value larger than
    // u32::MAX would silently truncate, so we additionally clamp to i32::MAX
    // as a conservative upper bound that fits safely in any usize.
    const MAX_IDX: i64 = i32::MAX as i64; // 2 147 483 647 — safe on all platforms
    let start = offset.unwrap_or(0).clamp(0, MAX_IDX) as usize;
    if start >= rows.len() {
        rows.clear();
        return;
    }
    rows.drain(0..start);
    if let Some(cnt) = count {
        let take = (cnt.clamp(0, MAX_IDX) as usize).min(rows.len());
        rows.truncate(take);
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_sql_backend::{ColumnDef, InMemoryBackend, SqlValue};
    use coding_adventures_sql_codegen::{AggFn, BinaryOp, CompiledSortKey, Instruction, Program, UnaryOp};
    use std::collections::BTreeMap;

    // ── Test helpers ──────────────────────────────────────────────────────────

    fn make_backend_with_table(
        table: &str,
        col_names: &[&str],
        rows: Vec<Vec<SqlValue>>,
    ) -> InMemoryBackend {
        let cols: Vec<ColumnDef> = col_names
            .iter()
            .map(|name| ColumnDef::new(*name, "TEXT"))
            .collect();
        let row_maps: Vec<Row> = rows
            .into_iter()
            .map(|vals| {
                col_names
                    .iter()
                    .zip(vals)
                    .map(|(k, v)| (k.to_string(), v))
                    .collect()
            })
            .collect();
        let mut tables = BTreeMap::new();
        tables.insert(table.to_string(), (cols, row_maps));
        InMemoryBackend::from_tables(tables)
    }

    fn prog(instrs: Vec<Instruction>) -> Program {
        Program { instructions: instrs }
    }

    fn int(n: i64) -> SqlValue { SqlValue::Int(n) }
    fn text(s: &str) -> SqlValue { SqlValue::Text(s.to_string()) }
    fn float(f: f64) -> SqlValue { SqlValue::Float(f) }
    fn null() -> SqlValue { SqlValue::Null }
    fn bool_val(b: bool) -> SqlValue { SqlValue::Bool(b) }

    // ── 1. LoadConst / EmitColumn / EmitRow / Halt ───────────────────────────

    #[test]
    fn test_load_const_emit_halt() {
        let mut backend = InMemoryBackend::new();
        let result = execute(
            &prog(vec![
                Instruction::BeginRow,
                Instruction::LoadConst(int(42)),
                Instruction::EmitColumn("x".to_string()),
                Instruction::EmitRow,
                Instruction::Halt,
            ]),
            &mut backend,
        )
        .unwrap();
        assert_eq!(result.columns, vec!["x"]);
        assert_eq!(result.rows, vec![vec![int(42)]]);
    }

    #[test]
    fn test_multiple_emit_columns() {
        let mut backend = InMemoryBackend::new();
        let result = execute(
            &prog(vec![
                Instruction::BeginRow,
                Instruction::LoadConst(int(1)),
                Instruction::EmitColumn("a".to_string()),
                Instruction::LoadConst(text("hello")),
                Instruction::EmitColumn("b".to_string()),
                Instruction::EmitRow,
                Instruction::Halt,
            ]),
            &mut backend,
        )
        .unwrap();
        assert_eq!(result.rows, vec![vec![int(1), text("hello")]]);
    }

    // ── 2. Stack underflow ────────────────────────────────────────────────────

    #[test]
    fn test_stack_underflow_returns_error() {
        let mut backend = InMemoryBackend::new();
        let err = execute(
            &prog(vec![Instruction::BinaryOpInstr(BinaryOp::Add), Instruction::Halt]),
            &mut backend,
        );
        assert!(matches!(err, Err(VmError::StackUnderflow)));
    }

    // ── 3. BinaryOp — arithmetic ──────────────────────────────────────────────

    #[test]
    fn test_add_ints() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(int(3)),
            Instruction::LoadConst(int(4)),
            Instruction::BinaryOpInstr(BinaryOp::Add),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(7)]]);
    }

    #[test]
    fn test_sub_ints() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(int(10)),
            Instruction::LoadConst(int(3)),
            Instruction::BinaryOpInstr(BinaryOp::Sub),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(7)]]);
    }

    #[test]
    fn test_mul_ints() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(int(6)),
            Instruction::LoadConst(int(7)),
            Instruction::BinaryOpInstr(BinaryOp::Mul),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(42)]]);
    }

    #[test]
    fn test_div_ints() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(int(10)),
            Instruction::LoadConst(int(2)),
            Instruction::BinaryOpInstr(BinaryOp::Div),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(5)]]);
    }

    #[test]
    fn test_div_by_zero_returns_error() {
        let mut b = InMemoryBackend::new();
        let err = execute(&prog(vec![
            Instruction::LoadConst(int(1)),
            Instruction::LoadConst(int(0)),
            Instruction::BinaryOpInstr(BinaryOp::Div),
            Instruction::Halt,
        ]), &mut b);
        assert!(matches!(err, Err(VmError::DivisionByZero)));
    }

    #[test]
    fn test_mod_ints() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(int(10)),
            Instruction::LoadConst(int(3)),
            Instruction::BinaryOpInstr(BinaryOp::Mod),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(1)]]);
    }

    #[test]
    fn test_float_arithmetic() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(float(1.5)),
            Instruction::LoadConst(float(2.5)),
            Instruction::BinaryOpInstr(BinaryOp::Add),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![float(4.0)]]);
    }

    // ── 4. NULL propagation ───────────────────────────────────────────────────

    #[test]
    fn test_null_plus_int_is_null() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(null()),
            Instruction::LoadConst(int(5)),
            Instruction::BinaryOpInstr(BinaryOp::Add),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![null()]]);
    }

    #[test]
    fn test_null_eq_null_is_null() {
        let mut b = InMemoryBackend::new();
        // In SQL, NULL = NULL is NULL (not TRUE).
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(null()),
            Instruction::LoadConst(null()),
            Instruction::BinaryOpInstr(BinaryOp::Eq),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![null()]]);
    }

    #[test]
    fn test_false_and_null_is_false() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(bool_val(false)),
            Instruction::LoadConst(null()),
            Instruction::BinaryOpInstr(BinaryOp::And),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![bool_val(false)]]);
    }

    #[test]
    fn test_true_or_null_is_true() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(bool_val(true)),
            Instruction::LoadConst(null()),
            Instruction::BinaryOpInstr(BinaryOp::Or),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![bool_val(true)]]);
    }

    #[test]
    fn test_true_and_null_is_null() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(bool_val(true)),
            Instruction::LoadConst(null()),
            Instruction::BinaryOpInstr(BinaryOp::And),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![null()]]);
    }

    #[test]
    fn test_false_or_null_is_null() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(bool_val(false)),
            Instruction::LoadConst(null()),
            Instruction::BinaryOpInstr(BinaryOp::Or),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![null()]]);
    }

    // ── 5. IS NULL / IS NOT NULL ──────────────────────────────────────────────

    #[test]
    fn test_is_null_on_null() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(null()),
            Instruction::IsNull,
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![bool_val(true)]]);
    }

    #[test]
    fn test_is_null_on_int() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(int(5)),
            Instruction::IsNull,
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![bool_val(false)]]);
    }

    #[test]
    fn test_is_not_null_on_null() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(null()),
            Instruction::IsNotNull,
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![bool_val(false)]]);
    }

    #[test]
    fn test_is_not_null_on_text() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(text("hi")),
            Instruction::IsNotNull,
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![bool_val(true)]]);
    }

    // ── 6. UnaryOp ────────────────────────────────────────────────────────────

    #[test]
    fn test_unary_neg() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(int(5)),
            Instruction::UnaryOpInstr(UnaryOp::Neg),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(-5)]]);
    }

    #[test]
    fn test_unary_not_true() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(bool_val(true)),
            Instruction::UnaryOpInstr(UnaryOp::Not),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![bool_val(false)]]);
    }

    #[test]
    fn test_unary_not_null_is_null() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(null()),
            Instruction::UnaryOpInstr(UnaryOp::Not),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![null()]]);
    }

    // ── 7. LIKE ───────────────────────────────────────────────────────────────

    #[test]
    fn test_like_percent_wildcard() {
        assert!(like_match("hello world", "%world"));
        assert!(like_match("hello world", "hello%"));
        assert!(like_match("hello world", "%lo wo%"));
        assert!(!like_match("hello world", "%xyz%"));
    }

    #[test]
    fn test_like_underscore_wildcard() {
        assert!(like_match("abc", "a_c"));
        assert!(like_match("axc", "a_c"));
        assert!(!like_match("ac", "a_c")); // _ requires exactly one char
        assert!(!like_match("abbc", "a_c"));
    }

    #[test]
    fn test_like_case_insensitive() {
        assert!(like_match("Hello", "hello"));
        assert!(like_match("WORLD", "%orld"));
    }

    #[test]
    fn test_like_exact_match() {
        assert!(like_match("abc", "abc"));
        assert!(!like_match("abc", "abd"));
    }

    #[test]
    fn test_like_null_propagation() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(null()),
            Instruction::LoadConst(text("%")),
            Instruction::Like,
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![null()]]);
    }

    // ── 8. BETWEEN ────────────────────────────────────────────────────────────

    #[test]
    fn test_between_inclusive_match() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(int(5)),
            Instruction::LoadConst(int(1)),
            Instruction::LoadConst(int(10)),
            Instruction::Between(true),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![bool_val(true)]]);
    }

    #[test]
    fn test_between_inclusive_boundary() {
        let mut b = InMemoryBackend::new();
        // 1 BETWEEN 1 AND 10 = true
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(int(1)),
            Instruction::LoadConst(int(1)),
            Instruction::LoadConst(int(10)),
            Instruction::Between(true),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![bool_val(true)]]);
    }

    #[test]
    fn test_between_out_of_range() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(int(0)),
            Instruction::LoadConst(int(1)),
            Instruction::LoadConst(int(10)),
            Instruction::Between(true),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![bool_val(false)]]);
    }

    #[test]
    fn test_between_null_propagation() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(null()),
            Instruction::LoadConst(int(1)),
            Instruction::LoadConst(int(10)),
            Instruction::Between(true),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![null()]]);
    }

    // ── 9. IN list ────────────────────────────────────────────────────────────

    #[test]
    fn test_in_list_match() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(int(2)),
            Instruction::LoadConst(int(1)),
            Instruction::LoadConst(int(2)),
            Instruction::LoadConst(int(3)),
            Instruction::InList(3),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![bool_val(true)]]);
    }

    #[test]
    fn test_in_list_no_match() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(int(99)),
            Instruction::LoadConst(int(1)),
            Instruction::LoadConst(int(2)),
            Instruction::LoadConst(int(3)),
            Instruction::InList(3),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![bool_val(false)]]);
    }

    #[test]
    fn test_in_list_null_val() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(null()),
            Instruction::LoadConst(int(1)),
            Instruction::InList(1),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![null()]]);
    }

    // ── 10. Scan / LoadColumn ─────────────────────────────────────────────────

    #[test]
    fn test_scan_loads_rows() {
        let mut b = make_backend_with_table(
            "t",
            &["id", "name"],
            vec![
                vec![int(1), text("Alice")],
                vec![int(2), text("Bob")],
            ],
        );
        let r = execute(&prog(vec![
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::BeginRow,
            Instruction::LoadColumn(None, "id".to_string()),
            Instruction::EmitColumn("id".to_string()),
            Instruction::LoadColumn(None, "name".to_string()),
            Instruction::EmitColumn("name".to_string()),
            Instruction::EmitRow,
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.columns, vec!["id", "name"]);
        assert_eq!(r.rows.len(), 2);
        assert_eq!(r.rows[0], vec![int(1), text("Alice")]);
        assert_eq!(r.rows[1], vec![int(2), text("Bob")]);
    }

    #[test]
    fn test_scan_empty_table() {
        let mut b = make_backend_with_table("t", &["id"], vec![]);
        let r = execute(&prog(vec![
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::BeginRow,
            Instruction::LoadColumn(None, "id".to_string()),
            Instruction::EmitColumn("id".to_string()),
            Instruction::EmitRow,
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows.len(), 0);
    }

    #[test]
    fn test_filter_with_jump_if_false() {
        // SELECT id FROM t WHERE id > 1
        let mut b = make_backend_with_table(
            "t", &["id"],
            vec![vec![int(1)], vec![int(2)], vec![int(3)]],
        );
        let r = execute(&prog(vec![
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::LoadColumn(None, "id".to_string()),
            Instruction::LoadConst(int(1)),
            Instruction::BinaryOpInstr(BinaryOp::Gt),
            Instruction::JumpIfFalse("loop".to_string()),
            Instruction::BeginRow,
            Instruction::LoadColumn(None, "id".to_string()),
            Instruction::EmitColumn("id".to_string()),
            Instruction::EmitRow,
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(2)], vec![int(3)]]);
    }

    // ── 11. Aggregates ────────────────────────────────────────────────────────

    #[test]
    fn test_count_star() {
        let mut b = make_backend_with_table(
            "t", &["x"], vec![vec![int(1)], vec![int(2)], vec![int(3)]],
        );
        let r = execute(&prog(vec![
            Instruction::InitAgg(1),
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::UpdateAgg(0, AggFn::CountStar),
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::BeginRow,
            Instruction::FinalizeAgg(0, AggFn::CountStar),
            Instruction::EmitColumn("cnt".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(3)]]);
    }

    #[test]
    fn test_count_skips_nulls() {
        let mut b = make_backend_with_table(
            "t", &["x"],
            vec![vec![int(1)], vec![null()], vec![int(3)]],
        );
        let r = execute(&prog(vec![
            Instruction::InitAgg(1),
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::UpdateAgg(0, AggFn::Count),
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::BeginRow,
            Instruction::FinalizeAgg(0, AggFn::Count),
            Instruction::EmitColumn("cnt".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(2)]]);
    }

    #[test]
    fn test_sum() {
        let mut b = make_backend_with_table(
            "t", &["x"], vec![vec![int(1)], vec![int(2)], vec![int(3)]],
        );
        let r = execute(&prog(vec![
            Instruction::InitAgg(1),
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::UpdateAgg(0, AggFn::Sum),
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::BeginRow,
            Instruction::FinalizeAgg(0, AggFn::Sum),
            Instruction::EmitColumn("s".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(6)]]);
    }

    #[test]
    fn test_sum_all_null_is_null() {
        let mut b = make_backend_with_table(
            "t", &["x"], vec![vec![null()], vec![null()]],
        );
        let r = execute(&prog(vec![
            Instruction::InitAgg(1),
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::UpdateAgg(0, AggFn::Sum),
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::BeginRow,
            Instruction::FinalizeAgg(0, AggFn::Sum),
            Instruction::EmitColumn("s".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![null()]]);
    }

    #[test]
    fn test_avg() {
        let mut b = make_backend_with_table(
            "t", &["x"], vec![vec![int(2)], vec![int(4)], vec![int(6)]],
        );
        let r = execute(&prog(vec![
            Instruction::InitAgg(1),
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::UpdateAgg(0, AggFn::Avg),
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::BeginRow,
            Instruction::FinalizeAgg(0, AggFn::Avg),
            Instruction::EmitColumn("avg".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![float(4.0)]]);
    }

    #[test]
    fn test_avg_all_null_is_null() {
        let mut b = make_backend_with_table(
            "t", &["x"], vec![vec![null()]],
        );
        let r = execute(&prog(vec![
            Instruction::InitAgg(1),
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::UpdateAgg(0, AggFn::Avg),
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::BeginRow,
            Instruction::FinalizeAgg(0, AggFn::Avg),
            Instruction::EmitColumn("avg".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![null()]]);
    }

    #[test]
    fn test_min() {
        let mut b = make_backend_with_table(
            "t", &["x"], vec![vec![int(5)], vec![int(2)], vec![int(8)]],
        );
        let r = execute(&prog(vec![
            Instruction::InitAgg(1),
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::UpdateAgg(0, AggFn::Min),
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::BeginRow,
            Instruction::FinalizeAgg(0, AggFn::Min),
            Instruction::EmitColumn("mn".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(2)]]);
    }

    #[test]
    fn test_max() {
        let mut b = make_backend_with_table(
            "t", &["x"], vec![vec![int(5)], vec![int(2)], vec![int(8)]],
        );
        let r = execute(&prog(vec![
            Instruction::InitAgg(1),
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::UpdateAgg(0, AggFn::Max),
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::BeginRow,
            Instruction::FinalizeAgg(0, AggFn::Max),
            Instruction::EmitColumn("mx".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(8)]]);
    }

    #[test]
    fn test_min_all_null_is_null() {
        let mut b = make_backend_with_table("t", &["x"], vec![vec![null()]]);
        let r = execute(&prog(vec![
            Instruction::InitAgg(1),
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::UpdateAgg(0, AggFn::Min),
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::BeginRow,
            Instruction::FinalizeAgg(0, AggFn::Min),
            Instruction::EmitColumn("mn".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![null()]]);
    }

    // ── 12. SortResult ────────────────────────────────────────────────────────

    #[test]
    fn test_sort_ascending() {
        let mut b = make_backend_with_table(
            "t", &["x"], vec![vec![int(3)], vec![int(1)], vec![int(2)]],
        );
        let r = execute(&prog(vec![
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::BeginRow,
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::EmitColumn("x".to_string()),
            Instruction::EmitRow,
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::Halt,
            Instruction::SortResult(vec![CompiledSortKey { column: "x".to_string(), ascending: true }]),
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(1)], vec![int(2)], vec![int(3)]]);
    }

    #[test]
    fn test_sort_descending() {
        let mut b = make_backend_with_table(
            "t", &["x"], vec![vec![int(1)], vec![int(3)], vec![int(2)]],
        );
        let r = execute(&prog(vec![
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::BeginRow,
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::EmitColumn("x".to_string()),
            Instruction::EmitRow,
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::Halt,
            Instruction::SortResult(vec![CompiledSortKey { column: "x".to_string(), ascending: false }]),
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(3)], vec![int(2)], vec![int(1)]]);
    }

    #[test]
    fn test_sort_multi_key() {
        let mut b = make_backend_with_table(
            "t", &["a", "b"],
            vec![
                vec![int(1), int(2)],
                vec![int(1), int(1)],
                vec![int(2), int(0)],
            ],
        );
        let r = execute(&prog(vec![
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::BeginRow,
            Instruction::LoadColumn(None, "a".to_string()),
            Instruction::EmitColumn("a".to_string()),
            Instruction::LoadColumn(None, "b".to_string()),
            Instruction::EmitColumn("b".to_string()),
            Instruction::EmitRow,
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::Halt,
            Instruction::SortResult(vec![
                CompiledSortKey { column: "a".to_string(), ascending: true },
                CompiledSortKey { column: "b".to_string(), ascending: true },
            ]),
        ]), &mut b).unwrap();
        assert_eq!(r.rows[0], vec![int(1), int(1)]);
        assert_eq!(r.rows[1], vec![int(1), int(2)]);
        assert_eq!(r.rows[2], vec![int(2), int(0)]);
    }

    // ── 13. LimitResult ───────────────────────────────────────────────────────

    #[test]
    fn test_limit_count() {
        let mut b = make_backend_with_table(
            "t", &["x"],
            vec![vec![int(1)], vec![int(2)], vec![int(3)], vec![int(4)]],
        );
        let r = execute(&prog(vec![
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::BeginRow,
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::EmitColumn("x".to_string()),
            Instruction::EmitRow,
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::Halt,
            Instruction::LimitResult(Some(2), None),
        ]), &mut b).unwrap();
        assert_eq!(r.rows.len(), 2);
        assert_eq!(r.rows[0], vec![int(1)]);
        assert_eq!(r.rows[1], vec![int(2)]);
    }

    #[test]
    fn test_limit_offset() {
        let mut b = make_backend_with_table(
            "t", &["x"],
            vec![vec![int(1)], vec![int(2)], vec![int(3)], vec![int(4)]],
        );
        let r = execute(&prog(vec![
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::BeginRow,
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::EmitColumn("x".to_string()),
            Instruction::EmitRow,
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::Halt,
            Instruction::LimitResult(Some(2), Some(1)),
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(2)], vec![int(3)]]);
    }

    #[test]
    fn test_limit_offset_only() {
        let mut b = make_backend_with_table(
            "t", &["x"],
            vec![vec![int(1)], vec![int(2)], vec![int(3)]],
        );
        let r = execute(&prog(vec![
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::BeginRow,
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::EmitColumn("x".to_string()),
            Instruction::EmitRow,
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::Halt,
            Instruction::LimitResult(None, Some(1)),
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(2)], vec![int(3)]]);
    }

    // ── 14. DistinctResult ────────────────────────────────────────────────────

    #[test]
    fn test_distinct_removes_duplicates() {
        let mut b = make_backend_with_table(
            "t", &["x"],
            vec![vec![int(1)], vec![int(2)], vec![int(1)], vec![int(2)], vec![int(3)]],
        );
        let r = execute(&prog(vec![
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::BeginRow,
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::EmitColumn("x".to_string()),
            Instruction::EmitRow,
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::Halt,
            Instruction::DistinctResult,
        ]), &mut b).unwrap();
        assert_eq!(r.rows.len(), 3);
        assert!(r.rows.contains(&vec![int(1)]));
        assert!(r.rows.contains(&vec![int(2)]));
        assert!(r.rows.contains(&vec![int(3)]));
    }

    // ── 15. INSERT ────────────────────────────────────────────────────────────

    #[test]
    fn test_insert_row() {
        let mut b = InMemoryBackend::new();
        // CREATE TABLE t (id INTEGER, name TEXT)
        b.create_table("t", vec![
            ColumnDef::new("id", "INTEGER"),
            ColumnDef::new("name", "TEXT"),
        ], false).unwrap();

        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(int(1)),
            Instruction::EmitColumn("id".to_string()),
            Instruction::LoadConst(text("Alice")),
            Instruction::EmitColumn("name".to_string()),
            Instruction::InsertRow("t".to_string(), None),
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows_affected, 1);

        // Verify the row is in the backend.
        let scan_r = execute(&prog(vec![
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::BeginRow,
            Instruction::LoadColumn(None, "id".to_string()),
            Instruction::EmitColumn("id".to_string()),
            Instruction::EmitRow,
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(scan_r.rows, vec![vec![int(1)]]);
    }

    #[test]
    fn test_insert_rows_affected_count() {
        let mut b = InMemoryBackend::new();
        b.create_table("t", vec![ColumnDef::new("x", "INTEGER")], false).unwrap();

        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(int(1)),
            Instruction::EmitColumn("x".to_string()),
            Instruction::InsertRow("t".to_string(), None),
            Instruction::BeginRow,
            Instruction::LoadConst(int(2)),
            Instruction::EmitColumn("x".to_string()),
            Instruction::InsertRow("t".to_string(), None),
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows_affected, 2);
    }

    // ── 16. CREATE TABLE / DROP TABLE ─────────────────────────────────────────

    #[test]
    fn test_create_table() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::CreateTableInstr(
                "users".to_string(),
                false,
                vec![ColumnDef::new("id", "INTEGER"), ColumnDef::new("name", "TEXT")],
            ),
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows_affected, 0);
        assert!(b.tables().contains(&"users".to_string()));
    }

    #[test]
    fn test_create_table_if_not_exists() {
        let mut b = InMemoryBackend::new();
        b.create_table("t", vec![ColumnDef::new("x", "INTEGER")], false).unwrap();
        // IF NOT EXISTS: should not error even though t already exists.
        let r = execute(&prog(vec![
            Instruction::CreateTableInstr("t".to_string(), true, vec![]),
            Instruction::Halt,
        ]), &mut b);
        assert!(r.is_ok());
    }

    #[test]
    fn test_drop_table() {
        let mut b = InMemoryBackend::new();
        b.create_table("t", vec![ColumnDef::new("x", "INTEGER")], false).unwrap();
        execute(&prog(vec![
            Instruction::DropTableInstr("t".to_string(), false),
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert!(!b.tables().contains(&"t".to_string()));
    }

    #[test]
    fn test_drop_table_if_exists() {
        let mut b = InMemoryBackend::new();
        // Should not error even if table doesn't exist.
        let r = execute(&prog(vec![
            Instruction::DropTableInstr("nonexistent".to_string(), true),
            Instruction::Halt,
        ]), &mut b);
        assert!(r.is_ok());
    }

    // ── 17. Transactions ──────────────────────────────────────────────────────

    #[test]
    fn test_begin_commit_transaction() {
        let mut b = InMemoryBackend::new();
        b.create_table("t", vec![ColumnDef::new("x", "INTEGER")], false).unwrap();
        let r = execute(&prog(vec![
            Instruction::BeginTransaction,
            Instruction::BeginRow,
            Instruction::LoadConst(int(1)),
            Instruction::EmitColumn("x".to_string()),
            Instruction::InsertRow("t".to_string(), None),
            Instruction::CommitTransaction,
            Instruction::Halt,
        ]), &mut b);
        assert!(r.is_ok());
    }

    #[test]
    fn test_begin_rollback_transaction() {
        let mut b = InMemoryBackend::new();
        b.create_table("t", vec![ColumnDef::new("x", "INTEGER")], false).unwrap();
        execute(&prog(vec![
            Instruction::BeginTransaction,
            Instruction::BeginRow,
            Instruction::LoadConst(int(99)),
            Instruction::EmitColumn("x".to_string()),
            Instruction::InsertRow("t".to_string(), None),
            Instruction::RollbackTransaction,
            Instruction::Halt,
        ]), &mut b).unwrap();
        // After rollback the insert is undone.
        let scan = execute(&prog(vec![
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::BeginRow,
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::EmitColumn("x".to_string()),
            Instruction::EmitRow,
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(scan.rows.len(), 0);
    }

    // ── 18. Comparison operators ──────────────────────────────────────────────

    #[test]
    fn test_eq_true() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(int(5)),
            Instruction::LoadConst(int(5)),
            Instruction::BinaryOpInstr(BinaryOp::Eq),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![bool_val(true)]]);
    }

    #[test]
    fn test_neq() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(int(5)),
            Instruction::LoadConst(int(3)),
            Instruction::BinaryOpInstr(BinaryOp::Neq),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![bool_val(true)]]);
    }

    #[test]
    fn test_lt_lte_gt_gte() {
        let mut b = InMemoryBackend::new();
        for (op, expected) in [
            (BinaryOp::Lt, true),
            (BinaryOp::Lte, true),
            (BinaryOp::Gt, false),
            (BinaryOp::Gte, false),
        ] {
            let r = execute(&prog(vec![
                Instruction::BeginRow,
                Instruction::LoadConst(int(1)),
                Instruction::LoadConst(int(2)),
                Instruction::BinaryOpInstr(op),
                Instruction::EmitColumn("r".to_string()),
                Instruction::EmitRow,
                Instruction::Halt,
            ]), &mut b).unwrap();
            assert_eq!(r.rows, vec![vec![bool_val(expected)]]);
        }
    }

    // ── 19. String concat ─────────────────────────────────────────────────────

    #[test]
    fn test_concat() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::BeginRow,
            Instruction::LoadConst(text("Hello")),
            Instruction::LoadConst(text(" World")),
            Instruction::BinaryOpInstr(BinaryOp::Concat),
            Instruction::EmitColumn("r".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![text("Hello World")]]);
    }

    // ── 20. Label / Jump / JumpIfTrue ─────────────────────────────────────────

    #[test]
    fn test_jump_skips_instructions() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::Jump("skip".to_string()),
            Instruction::LoadConst(int(99)),  // should not execute
            Instruction::Label("skip".to_string()),
            Instruction::BeginRow,
            Instruction::LoadConst(int(1)),
            Instruction::EmitColumn("x".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(1)]]);
    }

    #[test]
    fn test_jump_if_true() {
        let mut b = InMemoryBackend::new();
        let r = execute(&prog(vec![
            Instruction::LoadConst(bool_val(true)),
            Instruction::JumpIfTrue("found".to_string()),
            Instruction::BeginRow,
            Instruction::LoadConst(int(0)),
            Instruction::EmitColumn("x".to_string()),
            Instruction::EmitRow,
            Instruction::Jump("done".to_string()),
            Instruction::Label("found".to_string()),
            Instruction::BeginRow,
            Instruction::LoadConst(int(1)),
            Instruction::EmitColumn("x".to_string()),
            Instruction::EmitRow,
            Instruction::Label("done".to_string()),
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(1)]]);
    }

    // ── 21. AggIndexOutOfRange ────────────────────────────────────────────────

    #[test]
    fn test_agg_index_out_of_range() {
        let mut b = InMemoryBackend::new();
        let err = execute(&prog(vec![
            Instruction::InitAgg(1), // only slot 0
            Instruction::LoadConst(int(5)),
            Instruction::UpdateAgg(5, AggFn::Sum), // slot 5 doesn't exist
            Instruction::Halt,
        ]), &mut b);
        assert!(matches!(err, Err(VmError::AggIndexOutOfRange(5))));
    }

    // ── 22. Rows affected from DML ────────────────────────────────────────────

    #[test]
    fn test_rows_affected_select_is_zero() {
        let mut b = make_backend_with_table("t", &["x"], vec![vec![int(1)]]);
        let r = execute(&prog(vec![
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::BeginRow,
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::EmitColumn("x".to_string()),
            Instruction::EmitRow,
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows_affected, 0);
    }

    // ── 23. Null column reference ──────────────────────────────────────────────

    #[test]
    fn test_load_missing_column_is_null() {
        let mut b = make_backend_with_table("t", &["x"], vec![vec![int(1)]]);
        let r = execute(&prog(vec![
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::BeginRow,
            Instruction::LoadColumn(None, "nonexistent".to_string()),
            Instruction::EmitColumn("v".to_string()),
            Instruction::EmitRow,
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![null()]]);
    }

    // ── 24. LabelNotFound ─────────────────────────────────────────────────────

    #[test]
    fn test_label_not_found_error() {
        let mut b = InMemoryBackend::new();
        let err = execute(&prog(vec![
            Instruction::Jump("no_such_label".to_string()),
            Instruction::Halt,
        ]), &mut b);
        assert!(matches!(err, Err(VmError::LabelNotFound(_))));
    }

    // ── 25. Multiple aggregates in one program ────────────────────────────────

    #[test]
    fn test_multiple_aggregates() {
        let mut b = make_backend_with_table(
            "t", &["x"], vec![vec![int(1)], vec![int(2)], vec![int(3)]],
        );
        // Compute COUNT(*) and SUM(x) simultaneously.
        let r = execute(&prog(vec![
            Instruction::InitAgg(2),
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::UpdateAgg(0, AggFn::CountStar),
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::UpdateAgg(1, AggFn::Sum),
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::BeginRow,
            Instruction::FinalizeAgg(0, AggFn::CountStar),
            Instruction::EmitColumn("cnt".to_string()),
            Instruction::FinalizeAgg(1, AggFn::Sum),
            Instruction::EmitColumn("total".to_string()),
            Instruction::EmitRow,
            Instruction::Halt,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(3), int(6)]]);
    }

    // ── 26. Sort + Limit combined ─────────────────────────────────────────────

    #[test]
    fn test_sort_then_limit() {
        let mut b = make_backend_with_table(
            "t", &["x"],
            vec![vec![int(5)], vec![int(1)], vec![int(3)], vec![int(2)], vec![int(4)]],
        );
        let r = execute(&prog(vec![
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::BeginRow,
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::EmitColumn("x".to_string()),
            Instruction::EmitRow,
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::Halt,
            Instruction::SortResult(vec![CompiledSortKey { column: "x".to_string(), ascending: true }]),
            Instruction::LimitResult(Some(3), None),
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(1)], vec![int(2)], vec![int(3)]]);
    }

    // ── 27. Distinct + Sort combined ──────────────────────────────────────────

    #[test]
    fn test_distinct_then_sort() {
        let mut b = make_backend_with_table(
            "t", &["x"],
            vec![vec![int(2)], vec![int(1)], vec![int(2)], vec![int(3)], vec![int(1)]],
        );
        let r = execute(&prog(vec![
            Instruction::OpenScan("t".to_string(), None),
            Instruction::Label("loop".to_string()),
            Instruction::AdvanceCursor(None),
            Instruction::JumpIfExhausted(None, "end".to_string()),
            Instruction::BeginRow,
            Instruction::LoadColumn(None, "x".to_string()),
            Instruction::EmitColumn("x".to_string()),
            Instruction::EmitRow,
            Instruction::Jump("loop".to_string()),
            Instruction::Label("end".to_string()),
            Instruction::CloseScan(None),
            Instruction::Halt,
            Instruction::SortResult(vec![CompiledSortKey { column: "x".to_string(), ascending: true }]),
            Instruction::DistinctResult,
        ]), &mut b).unwrap();
        assert_eq!(r.rows, vec![vec![int(1)], vec![int(2)], vec![int(3)]]);
    }
}

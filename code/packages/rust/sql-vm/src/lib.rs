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

use coding_adventures_sql_backend::{Backend, Cursor, Row, RowIterator, SqlValue};
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
    /// A configurable resource limit was exceeded (e.g. too many GROUP BY groups
    /// or too many distinct values for COUNT(DISTINCT …)).
    ResourceLimit(String),
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
            VmError::ResourceLimit(m) => write!(f, "resource limit exceeded: {}", m),
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

/// A minimal [`Cursor`] implementation backed by the VM's own row buffer.
///
/// This lets us call [`Backend::update`] and [`Backend::delete`] from inside
/// the VM without needing to use the backend's own cursor type (`ListCursor`,
/// which is produced by the non-trait `InMemoryBackend::open_cursor` method).
///
/// The VM knows:
/// - which rows belong to a table (it scanned them at `OpenScan` time), and
/// - which row is "current" (`CursorState.pos - 1` after `AdvanceCursor`).
///
/// Constructing a `VmCursor` with that information satisfies the `Cursor`
/// contract so that `Backend::update()` and `Backend::delete()` can safely
/// locate and modify the correct row in the backend's storage.
struct VmCursor {
    /// All rows from the scanned table (same slice the VM uses).
    rows: Vec<Row>,
    /// Index of the current row (i.e., `CursorState.pos - 1`).
    /// Stored as `isize` so `adjust_after_delete` can underflow to -1 safely.
    index: isize,
    /// Normalized (lowercase) table name, used to satisfy the
    /// `cursor.table_key()` check inside `Backend::update()` / `delete()`.
    table_key: String,
}

impl RowIterator for VmCursor {
    fn next(&mut self) -> Option<Row> {
        // The VM does not use VmCursor as a live iterator — it only uses it to
        // satisfy the Backend::update/delete trait requirement.  This method is
        // never called by the VM internally.
        self.index += 1;
        let idx = usize::try_from(self.index).ok()?;
        self.rows.get(idx).cloned()
    }

    fn close(&mut self) {
        // No external resources to release.
    }
}

impl Cursor for VmCursor {
    fn current_row(&self) -> Option<Row> {
        usize::try_from(self.index)
            .ok()
            .and_then(|i| self.rows.get(i).cloned())
    }

    fn current_index(&self) -> Option<usize> {
        usize::try_from(self.index).ok()
    }

    fn table_key(&self) -> Option<&str> {
        Some(&self.table_key)
    }

    fn adjust_after_delete(&mut self) {
        self.index -= 1;
    }
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
/// | CountDistinct | — | distinct value set |
#[derive(Clone)]
struct AggAccumulator {
    /// Running value for Sum/Avg/Min/Max.  `None` = no non-null rows yet.
    acc: Option<SqlValue>,
    /// Row counter for Count/CountStar/Avg.
    count: i64,
    /// Distinct value set for COUNT(DISTINCT col).
    /// `None` when not in distinct mode; `Some(set)` when tracking distinct values.
    distinct_vals: Option<std::collections::HashSet<String>>,
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
    // Aggregate accumulators (for non-grouped aggregates).
    let mut agg_accs: Vec<AggAccumulator> = Vec::new();
    // Number of aggregate slots (saved from InitAgg for lazy group creation).
    let mut num_agg_slots: usize = 0;
    // ── GROUP BY state ────────────────────────────────────────────────────────
    //
    // When `SaveGroupKey` fires, group_mode activates.  Instead of updating a
    // single flat accumulator array, we maintain one accumulator array per
    // distinct group key.  After the scan loop, `BeginRow` emits all groups.
    //
    // `group_key_order` preserves insertion order so that GROUP BY output is
    // deterministic (matches the scan order, i.e. the order of first
    // occurrence of each distinct group value in the table).
    let mut group_mode = false;
    // Canonical string key → (original SqlValue list for the key columns, accumulators)
    let mut group_data: HashMap<String, (Vec<SqlValue>, Vec<AggAccumulator>)> = HashMap::new();
    let mut group_key_order: Vec<String> = Vec::new(); // insertion-order of distinct keys
    let mut current_group_key: String = String::new(); // set by SaveGroupKey each row
    // Names of the group-by columns (set by first SaveGroupKey call).
    let mut group_col_names: Vec<String> = Vec::new();
    // ── GROUP BY iteration state ───────────────────────────────────────────────
    //
    // When `FinalizeAgg` is first called in group mode, the VM switches to
    // "group iteration" mode: it executes the finalize/predicate/emit block
    // once per group by rewinding `pc` after each `EmitRow`.
    //
    // `group_finalize_pc`: the pc value at the moment of the first FinalizeAgg
    //     in group mode.  After each EmitRow we jump back here to process the
    //     next group.  `None` while not yet in group iteration mode.
    // `group_iter_idx`: index into `group_key_order` of the group currently
    //     being processed.
    let mut group_finalize_pc: Option<usize> = None;
    let mut group_iter_idx: usize = 0;
    // Post-op flags (set during the post-Halt region of the program).
    let mut post_sort: Option<Vec<CompiledSortKey>> = None;
    let mut post_limit: Option<(Option<i64>, Option<i64>)> = None;
    let mut post_distinct = false;
    // TruncateOutputColumns: strip hidden sort-key columns after SortResult.
    let mut post_truncate: Option<usize> = None;
    // DML counter.
    let mut rows_affected: i64 = 0;
    // Column names, locked in on the first EmitRow.
    let mut output_columns: Vec<String> = Vec::new();
    let mut columns_locked = false;
    // Transaction handle (used by CommitTransaction / RollbackTransaction).
    let mut tx_handle: Option<u64> = None;
    // Outer-join match flag: set true when an inner row satisfies the ON
    // condition for the current outer row, so LEFT/RIGHT JOIN can decide whether
    // to emit a NULL-padded row after the inner loop. See ClearMatch/SetMatch/
    // JumpIfMatched.
    let mut join_matched = false;

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
                //
                // Special case: "__dual__" is the implicit single-row virtual
                // table used for `SELECT expr` without a FROM clause (e.g.
                // `SELECT LENGTH('hello') AS n`).  It yields exactly one
                // empty row so the scan loop body executes once.
                let rows = if tbl == "__dual__" {
                    vec![Row::default()] // one empty row — columns evaluated from expressions
                } else {
                    let iter = backend
                        .scan(&tbl)
                        .map_err(|e| VmError::BackendError(e.to_string()))?;
                    drain_iterator(iter)
                };
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
                // GROUP BY: when the scan loop ends, enter group-iteration mode.
                // `pc` now points at the first instruction after CloseScan (the
                // finalize/predicate/emit block).  We save it as the "rewind
                // target" and immediately load the first group's data so that
                // LoadColumn / FinalizeAgg instructions execute correctly for
                // that group.
                if group_mode && !group_data.is_empty() {
                    group_finalize_pc = Some(pc); // save rewind target
                    group_iter_idx = 0;
                    group_mode = false; // disable so FinalizeAgg/LoadColumn run normally
                    // Load first group's data.
                    let key_str = group_key_order[0].clone();
                    if let Some((key_vals, group_accs)) = group_data.get(&key_str) {
                        agg_accs = group_accs.clone();
                        let mut fake_row: Row = Row::default();
                        for (col_name, val) in group_col_names.iter().zip(key_vals.iter()) {
                            fake_row.insert(col_name.clone(), val.clone());
                        }
                        current_row.insert(alias, fake_row);
                    }
                }
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
                // Emit the current row_buffer.
                if !columns_locked {
                    output_columns = row_buffer.iter().map(|(n, _)| n.clone()).collect();
                    columns_locked = true;
                }
                output_rows.push(row_buffer.clone());
                row_buffer.clear();

                // GROUP BY iteration: advance to the next group and rewind pc
                // to re-execute the finalize/predicate/emit block for it.
                if let Some(finalize_start) = group_finalize_pc {
                    group_iter_idx += 1;
                    if group_iter_idx < group_key_order.len() {
                        // Load the next group's data so that LoadColumn /
                        // FinalizeAgg operate on the correct group.
                        let key_str = group_key_order[group_iter_idx].clone();
                        if let Some((key_vals, group_accs)) = group_data.get(&key_str) {
                            agg_accs = group_accs.clone();
                            // Repopulate current_row[None] with this group's key values.
                            let mut fake_row: Row = Row::default();
                            for (col_name, val) in group_col_names.iter().zip(key_vals.iter()) {
                                fake_row.insert(col_name.clone(), val.clone());
                            }
                            // Use None as the cursor alias (no-alias scans store under None).
                            current_row.insert(None, fake_row);
                        }
                        // Clear row_buffer so pre-BeginRow EmitColumn accumulations
                        // from the previous iteration don't carry over.
                        row_buffer.clear();
                        pc = finalize_start; // rewind to first instruction after CloseScan
                    }
                }
            }

            // Lock in the output column schema without emitting any row.
            //
            // Emitted by the codegen when `Project(EmptyResult, cols)` is
            // compiled: the optimizer proved no rows can exist (e.g. LIMIT 0)
            // but we still need to return the correct column names so that
            // `QueryResult.columns` is populated.
            Instruction::DefineColumns(names) => {
                if !columns_locked {
                    output_columns = names;
                    columns_locked = true;
                }
            }

            // ─────────────── Built-in scalar functions ───────────────────────
            Instruction::CallBuiltin(fname, n) => {
                // Pop `n` arguments (last arg on top → pop in reverse order).
                // The codegen pushes args left-to-right so arg1 is deepest.
                let mut args: Vec<SqlValue> = (0..n)
                    .map(|_| pop(&mut stack))
                    .collect::<Result<_, _>>()?;
                args.reverse(); // now args[0] = first arg, args[n-1] = last arg

                let result = call_builtin(&fname, args)?;
                stack.push(result);
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
                num_agg_slots = n;
                // We cannot know the fn_tag for each slot at InitAgg time,
                // so we always allocate `distinct_vals: None` here.  The
                // UpdateAgg handler lazily initialises it to `Some(HashSet::new())`
                // on the first CountDistinct update (see update_accumulator).
                agg_accs = (0..n)
                    .map(|_| AggAccumulator { acc: None, count: 0, distinct_vals: None })
                    .collect();
                // Reset GROUP BY state for each new aggregate operation.
                group_mode = false;
                group_data.clear();
                group_key_order.clear();
                group_col_names.clear();
                group_finalize_pc = None;
                group_iter_idx = 0;
            }

            Instruction::UpdateAgg(idx, fn_tag) => {
                if group_mode {
                    // GROUP BY mode: update the accumulator for the current group.
                    let (_, group_accs) = group_data
                        .get_mut(&current_group_key)
                        .ok_or(VmError::AggIndexOutOfRange(idx))?;
                    if fn_tag == AggFn::CountStar {
                        let acc = group_accs.get_mut(idx).ok_or(VmError::AggIndexOutOfRange(idx))?;
                        acc.count += 1;
                    } else {
                        let v = pop(&mut stack)?;
                        let acc = group_accs.get_mut(idx).ok_or(VmError::AggIndexOutOfRange(idx))?;
                        update_accumulator(acc, &fn_tag, v)?;
                    }
                } else if fn_tag == AggFn::CountStar {
                    // Non-group CountStar: count every row, do not pop the stack.
                    let acc = agg_accs.get_mut(idx).ok_or(VmError::AggIndexOutOfRange(idx))?;
                    acc.count += 1;
                } else {
                    let v = pop(&mut stack)?;
                    let acc = agg_accs.get_mut(idx).ok_or(VmError::AggIndexOutOfRange(idx))?;
                    update_accumulator(acc, &fn_tag, v)?;
                }
            }

            Instruction::FinalizeAgg(idx, fn_tag) => {
                // Group iteration setup is done at CloseScan time, so by the
                // time FinalizeAgg runs, agg_accs already holds this group's
                // accumulators.  Just finalize normally in all cases.
                let acc = agg_accs.get(idx).ok_or(VmError::AggIndexOutOfRange(idx))?;
                stack.push(finalize_accumulator(acc, &fn_tag));
            }

            // SaveGroupKey: activate GROUP BY mode and record the current group.
            //
            // `cols` is the list of group-by column names.  We look up each in
            // `current_row` to build a canonical key string (format
            // "val0\x1Fval1\x1F..." using ASCII unit-separator as delimiter).
            // On the first invocation we record the column names; subsequent
            // invocations must use the same columns.
            Instruction::SaveGroupKey(cols) => {
                // Activate group mode on first SaveGroupKey.
                if !group_mode {
                    group_mode = true;
                    group_col_names = cols.clone();
                }
                // Build the key values by reading each group-by column from the
                // current (un-aliased) cursor row.  Fall back to Null if a column
                // is not present (e.g. outer join unmatched side).
                let key_vals: Vec<SqlValue> = cols.iter().map(|col_name| {
                    current_row
                        .get(&None) // the un-aliased cursor
                        .and_then(|row| row.get(col_name))
                        .cloned()
                        .unwrap_or(SqlValue::Null)
                }).collect();
                // Compute a canonical key string: "type:value\x1Ftype:value..."
                let key_str: String = key_vals.iter().map(|v| match v {
                    SqlValue::Int(n)   => format!("i:{}", n),
                    SqlValue::Float(f) => format!("f:{}", f),
                    SqlValue::Text(s)  => format!("t:{}", s),
                    SqlValue::Bool(b)  => format!("b:{}", b),
                    SqlValue::Null     => "null".to_string(),
                    SqlValue::Blob(bytes) => {
                        let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
                        format!("x:{}", hex)
                    }
                }).collect::<Vec<_>>().join("\x1F");
                current_group_key = key_str.clone();
                // Insert new group if we haven't seen this key before.
                if !group_data.contains_key(&key_str) {
                    // Guard against memory exhaustion from high-cardinality GROUP BY keys.
                    // 1 000 000 distinct groups × (accumulator array + key strings) can
                    // consume gigabytes; cap at a safe default.
                    const MAX_GROUP_KEYS: usize = 1_000_000;
                    if group_data.len() >= MAX_GROUP_KEYS {
                        return Err(VmError::ResourceLimit(format!(
                            "GROUP BY exceeded maximum distinct groups ({})",
                            MAX_GROUP_KEYS
                        )));
                    }
                    group_key_order.push(key_str.clone());
                    let fresh_accs = (0..num_agg_slots)
                        .map(|_| AggAccumulator { acc: None, count: 0, distinct_vals: None })
                        .collect();
                    group_data.insert(key_str, (key_vals, fresh_accs));
                }
            }

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

            // ── Outer-join match flag (no stack effect) ────────────────────
            Instruction::ClearMatch => {
                join_matched = false;
            }

            Instruction::SetMatch => {
                join_matched = true;
            }

            Instruction::JumpIfMatched(label) => {
                if join_matched {
                    pc = *label_index
                        .get(&label)
                        .ok_or_else(|| VmError::LabelNotFound(label.clone()))?;
                }
            }

            Instruction::JumpIfFalse(label) => {
                let v = pop(&mut stack)?;
                if !is_truthy(&v) {
                    // HAVING filter: predicate is false for this group.
                    // In group iteration mode, instead of jumping to the skip
                    // label (which leads to Halt/post-ops), advance to the next
                    // group and rewind to the finalize/predicate/emit block.
                    if let Some(finalize_start) = group_finalize_pc {
                        group_iter_idx += 1;
                        if group_iter_idx < group_key_order.len() {
                            // Load the next group's data.
                            let key_str = group_key_order[group_iter_idx].clone();
                            if let Some((key_vals, group_accs)) = group_data.get(&key_str) {
                                agg_accs = group_accs.clone();
                                let mut fake_row: Row = Row::default();
                                for (col_name, val) in group_col_names.iter().zip(key_vals.iter()) {
                                    fake_row.insert(col_name.clone(), val.clone());
                                }
                                current_row.insert(None, fake_row);
                            }
                            // Clear row_buffer so that pre-BeginRow EmitColumn accumulations
                            // from the previous iteration don't pollute the new one.
                            row_buffer.clear();
                            pc = finalize_start;
                        } else {
                            // No more groups and none passed the HAVING predicate.
                            // Lock in column names even though no rows were emitted,
                            // so that `QueryResult.columns` is correct.
                            if !columns_locked && !group_col_names.is_empty() {
                                let mut col_names: Vec<String> = group_col_names.clone();
                                // Append aggregate column names from row_buffer
                                // (pre-BeginRow EmitColumn accumulated them here).
                                for (name, _) in &row_buffer {
                                    col_names.push(name.clone());
                                }
                                output_columns = col_names;
                                columns_locked = true;
                            }
                            // Jump to the skip label to exit normally.
                            pc = *label_index
                                .get(&label)
                                .ok_or_else(|| VmError::LabelNotFound(label.clone()))?;
                        }
                    } else {
                        pc = *label_index
                            .get(&label)
                            .ok_or_else(|| VmError::LabelNotFound(label.clone()))?;
                    }
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

            Instruction::UpdateRows(tbl, col_names) => {
                // Pop one value per assignment column from the stack (they were
                // pushed in assignment order by compile_update).  Values are
                // popped LIFO, so the last assignment is on top — reverse to
                // restore the original left-to-right order.
                let n = col_names.len();
                let mut values: Vec<SqlValue> = (0..n)
                    .map(|_| pop(&mut stack))
                    .collect::<Result<_, _>>()?;
                values.reverse();

                // Build the assignments Row: { column_name → new_value }.
                let assignments_row: Row = col_names
                    .iter()
                    .zip(values.into_iter())
                    .map(|(c, v)| (c.clone(), v))
                    .collect();

                // The scan loop has already advanced past the current row:
                // cursor.pos = last_fetched_index + 1, so current = pos - 1.
                let current_idx = cursors
                    .get(&None)
                    .map(|c| c.pos.saturating_sub(1))
                    .unwrap_or(0);
                let cursor_rows = cursors
                    .get(&None)
                    .map(|c| c.rows.clone())
                    .unwrap_or_default();

                // Construct a VmCursor positioned at current_idx so that
                // Backend::update() can verify table_key and locate the row.
                let vm_cursor = VmCursor {
                    rows: cursor_rows,
                    index: current_idx as isize,
                    table_key: tbl.to_ascii_lowercase(),
                };

                backend
                    .update(&tbl, &vm_cursor, assignments_row)
                    .map_err(|e| VmError::BackendError(e.to_string()))?;
                rows_affected += 1;
            }

            Instruction::DeleteRows(tbl) => {
                // Get the current cursor position (last row fetched = pos - 1).
                let current_idx = cursors
                    .get(&None)
                    .map(|c| c.pos.saturating_sub(1))
                    .unwrap_or(0);
                let cursor_rows = cursors
                    .get(&None)
                    .map(|c| c.rows.clone())
                    .unwrap_or_default();

                // Construct a VmCursor positioned at current_idx.
                let mut vm_cursor = VmCursor {
                    rows: cursor_rows,
                    index: current_idx as isize,
                    table_key: tbl.to_ascii_lowercase(),
                };

                // Delete from the backend first.  backend.delete() calls
                // adjust_after_delete() on the cursor (adjusting vm_cursor.index),
                // which we can ignore since the VM tracks position via CursorState.
                backend
                    .delete(&tbl, &mut vm_cursor)
                    .map_err(|e| VmError::BackendError(e.to_string()))?;

                // Also remove the row from the VM's local cursor buffer so the
                // scan loop does not re-visit a row that no longer exists in the
                // backend.  Back up pos so the next AdvanceCursor picks up the
                // row that slid into this position.
                if let Some(state) = cursors.get_mut(&None) {
                    if current_idx < state.rows.len() {
                        state.rows.remove(current_idx);
                        state.pos = current_idx;
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

            Instruction::TruncateOutputColumns(n) => {
                post_truncate = Some(n);
            }
        }
    }

    // ── Phase 2b: post-op pass ────────────────────────────────────────────────
    //
    // After `Halt` breaks the main loop, `pc` points at the first instruction
    // after `Halt`.  Post-op instructions (SortResult, DistinctResult,
    // LimitResult, TruncateOutputColumns) live there.  Run them now to collect
    // the phase-3 flags.
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
            Instruction::TruncateOutputColumns(n) => {
                post_truncate = Some(n);
            }
            // Any other instruction after Halt is unexpected; skip it.
            _ => {}
        }
    }

    // ── Phase 3: post-processing ──────────────────────────────────────────────

    if let Some(keys) = post_sort {
        apply_sort(&mut output_rows, &keys, &output_columns);
    }
    // Strip hidden sort-key columns that were appended during compilation so
    // that SortResult could find them by name.  This truncation must happen
    // AFTER sorting and BEFORE distinct/limit so that only the SELECT-list
    // columns remain in the output.
    if let Some(n) = post_truncate {
        for row in &mut output_rows {
            row.truncate(n);
        }
        output_columns.truncate(n);
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
    // order.  Strip the names and keep the values — the row is already POSITIONAL
    // and parallel to `output_columns`, because both are produced by the same
    // `EmitColumn` sequence (the codegen emits one `EmitColumn` per output column,
    // in order; `output_columns` was locked from the first row's buffer; and any
    // hidden sort-key columns are truncated off BOTH the rows and `output_columns`
    // together in Phase 3).  So position `i` of every row is column `i`.
    //
    // We deliberately do NOT rebuild a `name → value` map here.  Two output
    // columns can legitimately share a name — e.g. `SELECT UPPER(x), LENGTH(x)`
    // yields two columns both defaulting to the name `?`, and `SELECT id, id`
    // yields two `id` columns.  Collapsing `(name, value)` pairs into a `HashMap`
    // would drop all but the last value for each repeated name, so both `UPPER(x)`
    // and `LENGTH(x)` would come back as `LENGTH(x)`'s value.  Positional
    // projection is both correct and cheaper.
    let ncols = output_columns.len();
    let rows: Vec<Vec<SqlValue>> = output_rows
        .into_iter()
        .map(|row| {
            let mut vals: Vec<SqlValue> = row.into_iter().map(|(_, v)| v).collect();
            // When column names were locked (the normal SELECT path), keep exactly
            // one value per column so `columns.len() == row.len()`.  `is_empty()`
            // means no `EmitColumn` ran (raw-value path) — return the values as-is.
            if ncols != 0 {
                vals.truncate(ncols);
                while vals.len() < ncols {
                    vals.push(SqlValue::Null);
                }
            }
            vals
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
// Built-in scalar function dispatcher
// ===========================================================================

/// Evaluate a named SQL built-in scalar function with the given arguments.
///
/// Returns the result as a [`SqlValue`], or a [`VmError::TypeMismatch`] if the
/// argument types are wrong for the function.  All functions propagate NULL:
/// if any required argument is NULL the result is NULL (except COALESCE).
///
/// ## Supported functions
///
/// | Name     | Args | Semantics                                             |
/// |----------|------|-------------------------------------------------------|
/// | LENGTH   |  1   | Byte-length of a string (returns Integer or NULL)     |
/// | UPPER    |  1   | ASCII-uppercase the string                            |
/// | LOWER    |  1   | ASCII-lowercase the string                            |
/// | TRIM     |  1   | Strip leading and trailing ASCII whitespace           |
/// | LTRIM    |  1   | Strip leading ASCII whitespace                        |
/// | RTRIM    |  1   | Strip trailing ASCII whitespace                       |
/// | SUBSTR   | 2–3  | 1-indexed substring extraction                        |
/// | REPLACE  |  3   | Replace all occurrences of a pattern with another str |
/// | ABS      |  1   | Absolute value (Integer or Float)                     |
/// | COALESCE | ≥1   | Return the first non-NULL argument                    |
fn call_builtin(name: &str, args: Vec<SqlValue>) -> Result<SqlValue, VmError> {
    match name {
        "LENGTH" => {
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("LENGTH expects 1 arg, got {}", args.len())));
            }
            match &args[0] {
                SqlValue::Null => Ok(SqlValue::Null),
                SqlValue::Text(s) => Ok(SqlValue::Int(s.chars().count() as i64)),
                other => Err(VmError::TypeMismatch(format!("LENGTH expects TEXT, got {:?}", other))),
            }
        }

        "UPPER" => {
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("UPPER expects 1 arg, got {}", args.len())));
            }
            match &args[0] {
                SqlValue::Null => Ok(SqlValue::Null),
                SqlValue::Text(s) => Ok(SqlValue::Text(s.to_uppercase())),
                other => Err(VmError::TypeMismatch(format!("UPPER expects TEXT, got {:?}", other))),
            }
        }

        "LOWER" => {
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("LOWER expects 1 arg, got {}", args.len())));
            }
            match &args[0] {
                SqlValue::Null => Ok(SqlValue::Null),
                SqlValue::Text(s) => Ok(SqlValue::Text(s.to_lowercase())),
                other => Err(VmError::TypeMismatch(format!("LOWER expects TEXT, got {:?}", other))),
            }
        }

        "TRIM" => {
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("TRIM expects 1 arg, got {}", args.len())));
            }
            match &args[0] {
                SqlValue::Null => Ok(SqlValue::Null),
                SqlValue::Text(s) => Ok(SqlValue::Text(s.trim().to_string())),
                other => Err(VmError::TypeMismatch(format!("TRIM expects TEXT, got {:?}", other))),
            }
        }

        "LTRIM" => {
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("LTRIM expects 1 arg, got {}", args.len())));
            }
            match &args[0] {
                SqlValue::Null => Ok(SqlValue::Null),
                SqlValue::Text(s) => Ok(SqlValue::Text(s.trim_start().to_string())),
                other => Err(VmError::TypeMismatch(format!("LTRIM expects TEXT, got {:?}", other))),
            }
        }

        "RTRIM" => {
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("RTRIM expects 1 arg, got {}", args.len())));
            }
            match &args[0] {
                SqlValue::Null => Ok(SqlValue::Null),
                SqlValue::Text(s) => Ok(SqlValue::Text(s.trim_end().to_string())),
                other => Err(VmError::TypeMismatch(format!("RTRIM expects TEXT, got {:?}", other))),
            }
        }

        "SUBSTR" => {
            if args.len() < 2 || args.len() > 3 {
                return Err(VmError::TypeMismatch(format!("SUBSTR expects 2 or 3 args, got {}", args.len())));
            }
            // NULL propagation: NULL string or NULL position → NULL.
            if matches!(args[0], SqlValue::Null) || matches!(args[1], SqlValue::Null) {
                return Ok(SqlValue::Null);
            }
            let s = match &args[0] {
                SqlValue::Text(t) => t.clone(),
                other => return Err(VmError::TypeMismatch(format!("SUBSTR arg1 expects TEXT, got {:?}", other))),
            };
            let pos = match &args[1] {
                SqlValue::Int(n) => *n,
                other => return Err(VmError::TypeMismatch(format!("SUBSTR arg2 expects INTEGER, got {:?}", other))),
            };
            // SQLite SUBSTR is 1-indexed.  pos=1 means the first character.
            // Negative pos counts from the end.
            let chars: Vec<char> = s.chars().collect();
            let len = chars.len() as i64;
            let start = if pos >= 1 {
                (pos - 1).min(len) as usize
            } else {
                (len + pos).max(0) as usize
            };
            let result_chars = if args.len() == 3 {
                if matches!(args[2], SqlValue::Null) { return Ok(SqlValue::Null); }
                let take = match &args[2] {
                    SqlValue::Int(n) => *n,
                    other => return Err(VmError::TypeMismatch(format!("SUBSTR arg3 expects INTEGER, got {:?}", other))),
                };
                let take = take.max(0) as usize;
                &chars[start..start.saturating_add(take).min(chars.len())]
            } else {
                &chars[start..]
            };
            Ok(SqlValue::Text(result_chars.iter().collect()))
        }

        "REPLACE" => {
            if args.len() != 3 {
                return Err(VmError::TypeMismatch(format!("REPLACE expects 3 args, got {}", args.len())));
            }
            // NULL propagation.
            if args.iter().any(|a| matches!(a, SqlValue::Null)) {
                return Ok(SqlValue::Null);
            }
            let (s, from, to) = match (&args[0], &args[1], &args[2]) {
                (SqlValue::Text(a), SqlValue::Text(b), SqlValue::Text(c)) => (a, b, c),
                _ => return Err(VmError::TypeMismatch("REPLACE expects TEXT, TEXT, TEXT".to_string())),
            };
            Ok(SqlValue::Text(s.replace(from.as_str(), to.as_str())))
        }

        "ABS" => {
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("ABS expects 1 arg, got {}", args.len())));
            }
            match &args[0] {
                SqlValue::Null => Ok(SqlValue::Null),
                SqlValue::Int(n) => Ok(SqlValue::Int(n.abs())),
                SqlValue::Float(f) => Ok(SqlValue::Float(f.abs())),
                other => Err(VmError::TypeMismatch(format!("ABS expects numeric, got {:?}", other))),
            }
        }

        "COALESCE" => {
            if args.is_empty() {
                return Err(VmError::TypeMismatch("COALESCE expects at least 1 arg".to_string()));
            }
            // Return the first non-NULL argument; if all are NULL, return NULL.
            for arg in args {
                if !matches!(arg, SqlValue::Null) {
                    return Ok(arg);
                }
            }
            Ok(SqlValue::Null)
        }

        "ROUND" => {
            // ROUND(x) or ROUND(x, digits).
            // SQLite always returns a Float for ROUND.
            if args.is_empty() || args.len() > 2 {
                return Err(VmError::TypeMismatch(format!("ROUND expects 1 or 2 args, got {}", args.len())));
            }
            if matches!(args[0], SqlValue::Null) {
                return Ok(SqlValue::Null);
            }
            let x: f64 = match &args[0] {
                SqlValue::Float(f) => *f,
                SqlValue::Int(n) => *n as f64,
                other => return Err(VmError::TypeMismatch(format!("ROUND arg1 expects numeric, got {:?}", other))),
            };
            let digits: i32 = if args.len() == 2 {
                if matches!(args[1], SqlValue::Null) { return Ok(SqlValue::Null); }
                match &args[1] {
                    SqlValue::Int(n) => *n as i32,
                    SqlValue::Float(f) => *f as i32,
                    other => return Err(VmError::TypeMismatch(format!("ROUND arg2 expects numeric, got {:?}", other))),
                }
            } else {
                0
            };
            // Round half away from zero (SQLite semantics), to `digits` decimal places.
            let factor = 10_f64.powi(digits);
            let rounded = (x * factor).round() / factor;
            Ok(SqlValue::Float(rounded))
        }

        "IFNULL" => {
            // IFNULL(a, b): the two-argument COALESCE — `a` unless it is NULL,
            // in which case `b`.
            if args.len() != 2 {
                return Err(VmError::TypeMismatch(format!("IFNULL expects 2 args, got {}", args.len())));
            }
            let mut it = args.into_iter();
            let a = it.next().unwrap();
            let b = it.next().unwrap();
            Ok(if matches!(a, SqlValue::Null) { b } else { a })
        }

        "NULLIF" => {
            // NULLIF(a, b): NULL when the two arguments are equal, else `a`.
            // Equivalent to `CASE WHEN a = b THEN NULL ELSE a END`, so a NULL
            // first argument still yields NULL.
            if args.len() != 2 {
                return Err(VmError::TypeMismatch(format!("NULLIF expects 2 args, got {}", args.len())));
            }
            if sql_eq(&args[0], &args[1]) {
                Ok(SqlValue::Null)
            } else {
                Ok(args.into_iter().next().unwrap())
            }
        }

        "TYPEOF" => {
            // TYPEOF(x): SQLite's storage-class name for the value.
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("TYPEOF expects 1 arg, got {}", args.len())));
            }
            let t = match &args[0] {
                SqlValue::Null => "null",
                // SQLite has no boolean storage class — booleans are integers.
                SqlValue::Bool(_) | SqlValue::Int(_) => "integer",
                SqlValue::Float(_) => "real",
                SqlValue::Text(_) => "text",
                SqlValue::Blob(_) => "blob",
            };
            Ok(SqlValue::Text(t.to_string()))
        }

        "INSTR" => {
            // INSTR(haystack, needle): 1-based character index of the first
            // occurrence of `needle` in `haystack`, 0 if absent, NULL if either
            // argument is NULL. `instr(x, '')` is 1, matching SQLite. (SQLite
            // also accepts blobs; text covers every current caller, and the
            // engine's other string builtins are likewise text-only.)
            if args.len() != 2 {
                return Err(VmError::TypeMismatch(format!("INSTR expects 2 args, got {}", args.len())));
            }
            if matches!(args[0], SqlValue::Null) || matches!(args[1], SqlValue::Null) {
                return Ok(SqlValue::Null);
            }
            let hay = match &args[0] {
                SqlValue::Text(s) => s,
                other => return Err(VmError::TypeMismatch(format!("INSTR arg1 expects TEXT, got {:?}", other))),
            };
            let needle = match &args[1] {
                SqlValue::Text(s) => s,
                other => return Err(VmError::TypeMismatch(format!("INSTR arg2 expects TEXT, got {:?}", other))),
            };
            let pos = if needle.is_empty() {
                1
            } else {
                match hay.find(needle.as_str()) {
                    // Byte offset → 1-based character offset.
                    Some(byte_idx) => hay[..byte_idx].chars().count() as i64 + 1,
                    None => 0,
                }
            };
            Ok(SqlValue::Int(pos))
        }

        "HEX" => {
            // HEX(x): uppercase hexadecimal of the argument's bytes. SQLite reads
            // the argument as a blob — text uses its UTF-8 bytes, a blob its raw
            // bytes, and an integer its decimal-text bytes (`hex(255)` → "323535").
            // NULL maps to NULL. Floats are declined: their SQLite text form
            // (`2.0`, not Rust's `2`) is subtle enough that we don't guess here.
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("HEX expects 1 arg, got {}", args.len())));
            }
            let bytes: Vec<u8> = match &args[0] {
                SqlValue::Null => return Ok(SqlValue::Null),
                SqlValue::Text(s) => s.as_bytes().to_vec(),
                SqlValue::Blob(b) => b.clone(),
                SqlValue::Int(i) => i.to_string().into_bytes(),
                SqlValue::Bool(b) => (*b as i64).to_string().into_bytes(),
                other => return Err(VmError::TypeMismatch(format!("HEX expects TEXT/BLOB/INTEGER, got {:?}", other))),
            };
            let mut out = String::with_capacity(bytes.len() * 2);
            for byte in bytes {
                out.push_str(&format!("{byte:02X}"));
            }
            Ok(SqlValue::Text(out))
        }

        other => {
            // Unknown function — return NULL rather than crashing.
            // This matches SQLite's behaviour for unrecognised scalar functions.
            Err(VmError::TypeMismatch(format!("unknown built-in function: {:?}", other)))
        }
    }
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
        // SQL standard: NULL propagates through concatenation.
        // If either operand is NULL the result is NULL.
        BinaryOp::Concat => {
            if matches!(l, SqlValue::Null) || matches!(r, SqlValue::Null) {
                return Ok(SqlValue::Null);
            }
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
///
/// Returns `Err(VmError::ResourceLimit)` if a hard per-accumulator memory cap
/// is reached (currently only for `CountDistinct`).
fn update_accumulator(acc: &mut AggAccumulator, fn_tag: &AggFn, v: SqlValue) -> Result<(), VmError> {
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
        AggFn::CountDistinct => {
            // Skip NULLs; insert a canonical string representation of non-NULL
            // values into the distinct set.  The set is lazily initialised here
            // in case the slot was not pre-tagged at InitAgg time.
            //
            // Safety cap: prevent memory exhaustion from high-cardinality columns.
            // COUNT(DISTINCT blob_col) over millions of rows could store megabytes
            // of hex strings per entry; cap at 1 000 000 distinct values.
            const MAX_DISTINCT_VALS: usize = 1_000_000;
            if !matches!(v, SqlValue::Null) {
                let key = match &v {
                    SqlValue::Int(n)   => format!("i:{}", n),
                    SqlValue::Float(f) => format!("f:{}", f),
                    SqlValue::Text(s)  => format!("t:{}", s),
                    SqlValue::Bool(b)  => format!("b:{}", b),
                    SqlValue::Blob(bytes) => {
                        let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
                        format!("x:{}", hex)
                    }
                    SqlValue::Null => unreachable!(),
                };
                let set = acc.distinct_vals.get_or_insert_with(std::collections::HashSet::new);
                if set.len() >= MAX_DISTINCT_VALS && !set.contains(&key) {
                    return Err(VmError::ResourceLimit(format!(
                        "COUNT(DISTINCT) exceeded maximum distinct values ({})",
                        MAX_DISTINCT_VALS
                    )));
                }
                set.insert(key);
            }
        }
    }
    Ok(())
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
        AggFn::CountDistinct => {
            let n = acc.distinct_vals.as_ref().map(|s| s.len()).unwrap_or(0);
            SqlValue::Int(n as i64)
        }
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
    rows: &mut [Vec<(String, SqlValue)>],
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
        // LIMIT -1 (or any negative value) means "no limit" in SQLite.
        // Only truncate when the count is non-negative.
        if cnt >= 0 {
            let take = (cnt.clamp(0, MAX_IDX) as usize).min(rows.len());
            rows.truncate(take);
        }
        // cnt < 0 → keep all remaining rows (no truncation).
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

    #[test]
    fn builtin_ifnull_and_nullif() {
        // IFNULL passes through a non-NULL, substitutes on NULL.
        assert_eq!(
            call_builtin("IFNULL", vec![SqlValue::Int(5), SqlValue::Int(-1)]).unwrap(),
            SqlValue::Int(5)
        );
        assert_eq!(
            call_builtin("IFNULL", vec![SqlValue::Null, SqlValue::Int(-1)]).unwrap(),
            SqlValue::Int(-1)
        );
        // NULLIF collapses equal args to NULL, else returns the first.
        assert_eq!(
            call_builtin("NULLIF", vec![SqlValue::Int(2), SqlValue::Int(2)]).unwrap(),
            SqlValue::Null
        );
        assert_eq!(
            call_builtin("NULLIF", vec![SqlValue::Int(1), SqlValue::Int(2)]).unwrap(),
            SqlValue::Int(1)
        );
    }

    #[test]
    fn builtin_typeof_names_each_storage_class() {
        let t = |v: SqlValue| match call_builtin("TYPEOF", vec![v]).unwrap() {
            SqlValue::Text(s) => s,
            other => panic!("expected text, got {other:?}"),
        };
        assert_eq!(t(SqlValue::Null), "null");
        assert_eq!(t(SqlValue::Int(3)), "integer");
        assert_eq!(t(SqlValue::Float(1.5)), "real");
        assert_eq!(t(SqlValue::Text("x".into())), "text");
        assert_eq!(t(SqlValue::Blob(vec![1])), "blob");
    }

    #[test]
    fn builtin_instr_char_positions_and_nulls() {
        let i = |h: &str, n: &str| {
            call_builtin("INSTR", vec![SqlValue::Text(h.into()), SqlValue::Text(n.into())]).unwrap()
        };
        assert_eq!(i("abc", "b"), SqlValue::Int(2));
        assert_eq!(i("abc", "x"), SqlValue::Int(0));
        assert_eq!(i("abc", ""), SqlValue::Int(1)); // instr(x, '') == 1
        // Multi-byte prefix: 'é' is one character, so the match is at char 2.
        assert_eq!(i("éb", "b"), SqlValue::Int(2));
        // NULL in either argument propagates.
        assert_eq!(
            call_builtin("INSTR", vec![SqlValue::Null, SqlValue::Text("b".into())]).unwrap(),
            SqlValue::Null
        );
    }

    #[test]
    fn builtin_hex_encodes_bytes_uppercase() {
        let h = |v: SqlValue| match call_builtin("HEX", vec![v]).unwrap() {
            SqlValue::Text(s) => s,
            other => panic!("expected text, got {other:?}"),
        };
        assert_eq!(h(SqlValue::Text("abc".into())), "616263");
        assert_eq!(h(SqlValue::Blob(vec![0xde, 0xad, 0xbe, 0xef])), "DEADBEEF");
        assert_eq!(h(SqlValue::Int(255)), "323535"); // hex of the text "255"
        assert_eq!(call_builtin("HEX", vec![SqlValue::Null]).unwrap(), SqlValue::Null);
    }
}

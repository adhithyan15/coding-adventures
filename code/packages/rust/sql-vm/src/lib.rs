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
use coding_adventures_sql_codegen::{
    AggFn, BinaryOp, CastType, CompiledSortKey, Instruction, Program, UnaryOp,
};

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

            // ─────────────── LIKE / NOT LIKE ────────────────────────────────
            Instruction::Like(negated) => {
                // Stack (top → bottom): pattern, value
                let pat = pop(&mut stack)?;
                let val = pop(&mut stack)?;
                let result = match (&val, &pat) {
                    // A NULL operand makes the whole predicate NULL, and `NOT`
                    // leaves NULL unchanged (NULL is neither true nor false).
                    (SqlValue::Null, _) | (_, SqlValue::Null) => SqlValue::Null,
                    _ => {
                        let matched = like_match(&sql_to_str(&val), &sql_to_str(&pat));
                        SqlValue::Bool(matched ^ negated)
                    }
                };
                stack.push(result);
            }

            // ─────────────── LIKE / NOT LIKE … ESCAPE ───────────────────────
            Instruction::LikeEscape(negated) => {
                // Stack (top → bottom): escape, pattern, value
                let esc = pop(&mut stack)?;
                let pat = pop(&mut stack)?;
                let val = pop(&mut stack)?;
                let result = match (&val, &pat, &esc) {
                    // Any NULL operand — including a NULL escape — yields NULL,
                    // which `NOT` leaves unchanged.
                    (SqlValue::Null, _, _) | (_, SqlValue::Null, _) | (_, _, SqlValue::Null) => {
                        SqlValue::Null
                    }
                    _ => {
                        // SQLite requires the ESCAPE string to be exactly one
                        // character; anything else is a runtime error.
                        let esc_str = sql_to_str(&esc);
                        let mut chars = esc_str.chars();
                        match (chars.next(), chars.next()) {
                            (Some(e), None) => {
                                let matched = like_match_escape(
                                    &sql_to_str(&val),
                                    &sql_to_str(&pat),
                                    e,
                                );
                                SqlValue::Bool(matched ^ negated)
                            }
                            _ => {
                                return Err(VmError::TypeMismatch(
                                    "ESCAPE expression must be a single character".to_string(),
                                ))
                            }
                        }
                    }
                };
                stack.push(result);
            }

            Instruction::Cast(ty) => {
                let val = pop(&mut stack)?;
                stack.push(apply_cast(&val, &ty));
            }

            // ─────────────── BETWEEN ───────────────────────────────────────
            Instruction::Between(plain) => {
                // Stack (top → bottom): high, low, value.
                // `plain` is codegen's `!negated`: true for `BETWEEN`, false for
                // `NOT BETWEEN` (see eval_between).
                let hi = pop(&mut stack)?;
                let lo = pop(&mut stack)?;
                let val = pop(&mut stack)?;
                stack.push(eval_between(&val, &lo, &hi, plain)?);
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
                // SQLite IN is three-valued and uses the same equality as `=`:
                //   • test value NULL            → NULL
                //   • any element `=` the value  → 1 (true), even if NULLs present
                //   • else if any element is NULL → NULL (the value *might* equal
                //     the unknown), matching `1 IN (NULL,2)` → NULL
                //   • else                        → 0 (false)
                // `sql_eq` compares by storage class, so `1 IN (1.0)` is true
                // (Int/Float compare numerically) while `'1' IN (1)` is false
                // (text vs integer). This supersedes the old derived-`PartialEq`
                // membership, which missed both numeric equality and NULL logic.
                let result = match val {
                    SqlValue::Null => SqlValue::Null,
                    v => {
                        let mut saw_null = false;
                        let mut matched = false;
                        for item in &items {
                            if matches!(item, SqlValue::Null) {
                                saw_null = true;
                            } else if sql_eq(&v, item) {
                                matched = true;
                                break;
                            }
                        }
                        if matched {
                            SqlValue::Bool(true)
                        } else if saw_null {
                            SqlValue::Null
                        } else {
                            SqlValue::Bool(false)
                        }
                    }
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
                    .zip(values)
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
/// | LENGTH   |  1   | Character count of a string (returns Integer or NULL) |
/// | OCTET_LENGTH | 1 | Byte count of text/blob/integer (Integer or NULL)    |
/// | UPPER    |  1   | ASCII-uppercase the string                            |
/// | LOWER    |  1   | ASCII-lowercase the string                            |
/// | TRIM     | 1–2  | Strip whitespace, or a given character set, from both ends |
/// | LTRIM    | 1–2  | Strip whitespace, or a given character set, from the left  |
/// | RTRIM    | 1–2  | Strip whitespace, or a given character set, from the right |
/// | SUBSTR   | 2–3  | 1-indexed substring extraction (alias: SUBSTRING)     |
/// | CONCAT   | ≥1   | Concatenate all arguments (NULL → empty string)       |
/// | CONCAT_WS| ≥2   | Join value arguments with a separator (NULLs skipped) |
/// | UNHEX    | 1–2  | Decode hex digit pairs into a blob (inverse of HEX)   |
/// | LIKELY / UNLIKELY | 1 | Planner hint; returns the argument unchanged     |
/// | LIKELIHOOD | 2  | Planner hint with a probability; returns arg 1        |
/// | GLOB     |  2   | Case-sensitive wildcard match: GLOB(pattern, subject) |
/// | PRINTF / FORMAT | ≥1 | C-style string formatting (integer/string specifiers) |
/// | REPLACE  |  3   | Replace all occurrences of a pattern with another str |
/// | ABS      |  1   | Absolute value (Integer or Float)                     |
/// | COALESCE | ≥1   | Return the first non-NULL argument                    |
/// Coerce a value to the text form TRIM operates on, matching SQLite's
/// implicit cast: text is itself; an integer or boolean becomes its decimal
/// digits (`trim(12321, '1')` → `"232"`). A NULL argument returns `Ok(None)`,
/// the caller's signal to propagate NULL. Floats and blobs are declined — as
/// with HEX/QUOTE above, their exact SQLite text form is subtle enough that we
/// don't guess here.
fn trim_coerce(name: &str, v: &SqlValue) -> Result<Option<String>, VmError> {
    match v {
        SqlValue::Null => Ok(None),
        SqlValue::Text(s) => Ok(Some(s.clone())),
        SqlValue::Int(i) => Ok(Some(i.to_string())),
        SqlValue::Bool(b) => Ok(Some((*b as i64).to_string())),
        other => Err(VmError::TypeMismatch(format!("{name} expects TEXT, got {other:?}"))),
    }
}

/// The shared body of `TRIM` / `LTRIM` / `RTRIM`, parameterised by which end(s)
/// to strip (`left`, `right`).
///
/// **One argument** keeps the historical behaviour — remove whitespace from the
/// chosen end(s).
///
/// **Two arguments** switch to SQLite's *character-set* trim: the second
/// argument is read as a bag of characters, and any leading (`left`) or
/// trailing (`right`) character that appears in that bag is removed — repeated
/// until a character outside the bag is reached. The bag is a *set of
/// characters*, not a substring: order and repetition inside it don't matter.
///
/// ```text
///   trim('xxhixx', 'x')    -> 'hi'      set = {x}
///   trim('abcHIcba', 'abc') -> 'HI'     set = {a, b, c}
///   ltrim('xyxhi', 'xy')   -> 'hi'      only the left end
///   rtrim('hixyx', 'xy')   -> 'hi'      only the right end
///   trim('héllo', 'h')     -> 'éllo'    operates on Unicode chars, not bytes
///   trim('xhix', '')       -> 'xhix'    empty set removes nothing
///   trim('xxhixx', NULL)   -> NULL      NULL in either argument propagates
/// ```
fn trim_builtin(name: &str, args: &[SqlValue], left: bool, right: bool) -> Result<SqlValue, VmError> {
    // Validate arity *before* indexing. The grammar makes a call's argument list
    // optional, so `TRIM()` parses and reaches here with an empty `args`; without
    // this guard `args[0]` would panic (index out of bounds) — a reachable DoS on
    // any untrusted SQL. Every sibling builtin checks arity first; we match that.
    if args.is_empty() || args.len() > 2 {
        return Err(VmError::TypeMismatch(format!("{name} expects 1 or 2 args, got {}", args.len())));
    }

    // Resolve the subject string, short-circuiting on a NULL argument.
    let subject = match trim_coerce(name, &args[0])? {
        None => return Ok(SqlValue::Null),
        Some(s) => s,
    };

    if args.len() == 1 {
        let trimmed = match (left, right) {
            (true, true) => subject.trim(),
            (true, false) => subject.trim_start(),
            (false, true) => subject.trim_end(),
            (false, false) => subject.as_str(),
        };
        return Ok(SqlValue::Text(trimmed.to_string()));
    }

    // Two-argument (character-set) form.
    let set = match trim_coerce(name, &args[1])? {
        None => return Ok(SqlValue::Null),
        Some(s) => s,
    };
    // An empty trim-set matches nothing, so the subject is returned verbatim —
    // no need to scan every character against the set.
    if set.is_empty() {
        return Ok(SqlValue::Text(subject));
    }
    // Materialise the set into a `HashSet` for O(1) membership. `str::contains`
    // would be a linear scan of the set per subject character, making the whole
    // trim O(N·M) in the subject/set lengths — a quadratic CPU vector an attacker
    // controls (a long subject of set-members against a long set). The set is
    // O(M) to build once, so overall cost drops to O(N + M).
    let set_chars: std::collections::HashSet<char> = set.chars().collect();
    let in_set = |c: char| set_chars.contains(&c);
    let mut slice: &str = &subject;
    if left {
        slice = slice.trim_start_matches(in_set);
    }
    if right {
        slice = slice.trim_end_matches(in_set);
    }
    Ok(SqlValue::Text(slice.to_string()))
}

fn call_builtin(name: &str, args: Vec<SqlValue>) -> Result<SqlValue, VmError> {
    match name {
        "LENGTH" => {
            // LENGTH(X) counts *characters* for text and *bytes* for a blob,
            // matching SQLite: `length('héllo')` = 5 (5 chars, though 6 bytes)
            // but `length(x'0102ff')` = 3 (raw bytes, NOT text-converted). A
            // number is measured as the character count of its decimal-text form
            // (`length(12345)` = 5, `length(-7)` = 2). NULL → NULL. Floats are
            // declined — their SQLite text form (`3.0` vs Rust's `3`, exponent
            // notation, …) is subtle enough that we don't guess here (same stance
            // as OCTET_LENGTH / HEX / QUOTE).
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("LENGTH expects 1 arg, got {}", args.len())));
            }
            match &args[0] {
                SqlValue::Null => Ok(SqlValue::Null),
                SqlValue::Text(s) => Ok(SqlValue::Int(s.chars().count() as i64)),
                SqlValue::Blob(b) => Ok(SqlValue::Int(b.len() as i64)),
                SqlValue::Int(i) => Ok(SqlValue::Int(i.to_string().chars().count() as i64)),
                SqlValue::Bool(b) => Ok(SqlValue::Int((*b as i64).to_string().chars().count() as i64)),
                other => Err(VmError::TypeMismatch(format!("LENGTH expects TEXT/BLOB/INTEGER, got {:?}", other))),
            }
        }

        "OCTET_LENGTH" => {
            // OCTET_LENGTH(x): the number of *bytes*, in contrast to LENGTH's
            // count of characters. Text is measured as its UTF-8 bytes
            // (`octet_length('héllo')` = 6, five characters but `é` is two
            // bytes); a blob as its raw byte count; an integer/boolean as its
            // decimal-text bytes (`octet_length(123)` = 3). NULL → NULL. Floats
            // are declined — their byte length depends on SQLite's exact float
            // text form, which is subtle (see HEX/QUOTE).
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("OCTET_LENGTH expects 1 arg, got {}", args.len())));
            }
            match &args[0] {
                SqlValue::Null => Ok(SqlValue::Null),
                SqlValue::Text(s) => Ok(SqlValue::Int(s.len() as i64)),
                SqlValue::Blob(b) => Ok(SqlValue::Int(b.len() as i64)),
                SqlValue::Int(i) => Ok(SqlValue::Int(i.to_string().len() as i64)),
                SqlValue::Bool(b) => Ok(SqlValue::Int((*b as i64).to_string().len() as i64)),
                other => Err(VmError::TypeMismatch(format!("OCTET_LENGTH expects TEXT/BLOB/INTEGER, got {:?}", other))),
            }
        }

        "LIKELY" | "UNLIKELY" => {
            // Query-planner hints: `likely(x)` / `unlikely(x)` tell SQLite's
            // optimizer that `x` is probably true / probably false, biasing its
            // row-count estimates. They have no effect on the result — they are
            // the *identity* function, returning the argument unchanged (any
            // type, including NULL).
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("{name} expects 1 arg, got {}", args.len())));
            }
            Ok(args.into_iter().next().unwrap())
        }

        "LIKELIHOOD" => {
            // `likelihood(x, p)` is `likely`/`unlikely` with an explicit
            // probability `p` (the fraction of rows for which `x` is expected to
            // be true). It returns `x` unchanged; `p` only hints the planner.
            // SQLite requires `p` to be a constant number in [0.0, 1.0] — we
            // validate the value's range and reject anything else.
            if args.len() != 2 {
                return Err(VmError::TypeMismatch(format!("LIKELIHOOD expects 2 args, got {}", args.len())));
            }
            let p = match &args[1] {
                SqlValue::Float(f) => *f,
                SqlValue::Int(i) => *i as f64,
                other => {
                    return Err(VmError::TypeMismatch(format!(
                        "LIKELIHOOD probability must be a number in [0,1], got {other:?}"
                    )))
                }
            };
            if !(0.0..=1.0).contains(&p) {
                return Err(VmError::TypeMismatch(format!(
                    "LIKELIHOOD probability {p} is out of range [0,1]"
                )));
            }
            Ok(args.into_iter().next().unwrap())
        }

        "GLOB" => {
            // GLOB(pattern, subject) is the function form of `subject GLOB
            // pattern`: a case-sensitive wildcard match returning 1 or 0. NULL in
            // either argument yields NULL. (The infix `GLOB` operator is a
            // separate, grammar-level feature; this is the callable function.)
            if args.len() != 2 {
                return Err(VmError::TypeMismatch(format!("GLOB expects 2 args, got {}", args.len())));
            }
            if matches!(args[0], SqlValue::Null) || matches!(args[1], SqlValue::Null) {
                return Ok(SqlValue::Null);
            }
            let pattern = sql_to_str(&args[0]);
            let subject = sql_to_str(&args[1]);
            Ok(SqlValue::Int(glob_match(&subject, &pattern) as i64))
        }

        "PRINTF" | "FORMAT" => {
            // PRINTF(format, ...) / FORMAT(format, ...): C-style string
            // formatting. The first argument is the format string; the rest are
            // consumed by its conversions. A NULL format yields NULL. See
            // `sql_printf` for the supported conversions and the DoS caps.
            if args.is_empty() {
                return Err(VmError::TypeMismatch(format!("{name} expects at least 1 arg")));
            }
            let format = match &args[0] {
                SqlValue::Null => return Ok(SqlValue::Null),
                SqlValue::Text(s) => s.clone(),
                SqlValue::Int(i) => i.to_string(),
                SqlValue::Bool(b) => (*b as i64).to_string(),
                other => {
                    return Err(VmError::TypeMismatch(format!(
                        "{name} format must be text, got {other:?}"
                    )))
                }
            };
            Ok(SqlValue::Text(sql_printf(name, &format, &args[1..])?))
        }

        "UPPER" => {
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("UPPER expects 1 arg, got {}", args.len())));
            }
            match &args[0] {
                SqlValue::Null => Ok(SqlValue::Null),
                // SQLite's built-in UPPER only case-folds ASCII `a`–`z`; every
                // other byte (accented letters, non-Latin scripts) is left as-is.
                // Rust's `to_uppercase` is full-Unicode, so use the ASCII variant.
                SqlValue::Text(s) => Ok(SqlValue::Text(s.to_ascii_uppercase())),
                other => Err(VmError::TypeMismatch(format!("UPPER expects TEXT, got {:?}", other))),
            }
        }

        "LOWER" => {
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("LOWER expects 1 arg, got {}", args.len())));
            }
            match &args[0] {
                SqlValue::Null => Ok(SqlValue::Null),
                // ASCII-only, mirroring SQLite's built-in LOWER (see UPPER above).
                SqlValue::Text(s) => Ok(SqlValue::Text(s.to_ascii_lowercase())),
                other => Err(VmError::TypeMismatch(format!("LOWER expects TEXT, got {:?}", other))),
            }
        }

        // Internal collation canonicaliser (not user-facing SQL — the planner
        // emits it to lower `x <op> y COLLATE C` onto `canon_C(x) <op> canon_C(y)`).
        // A text value is transformed by the collation (NOCASE → ASCII-lowercase,
        // RTRIM → strip trailing spaces); NULL and every non-text value pass
        // through UNCHANGED, so a numeric comparison keeps its own semantics
        // (`5 = '5' COLLATE NOCASE` stays 0). Reuses `collate_text`.
        "__COLLATE" => {
            if args.len() != 2 {
                return Err(VmError::TypeMismatch(format!(
                    "__collate expects 2 args, got {}",
                    args.len()
                )));
            }
            let collation = match &args[1] {
                SqlValue::Text(s) => s.clone(),
                other => {
                    return Err(VmError::TypeMismatch(format!(
                        "__collate expects a text collation name, got {other:?}"
                    )))
                }
            };
            match &args[0] {
                SqlValue::Text(s) => Ok(SqlValue::Text(collate_text(s, &collation))),
                other => Ok(other.clone()),
            }
        }

        "TRIM" => trim_builtin("TRIM", &args, true, true),

        "LTRIM" => trim_builtin("LTRIM", &args, true, false),

        "RTRIM" => trim_builtin("RTRIM", &args, false, true),

        "CONCAT" => {
            // CONCAT(x, y, …): concatenate every argument's text. A NULL argument
            // contributes the empty string (it does NOT make the result NULL), so
            // `concat('a', NULL, 'c')` = 'ac'. The result is always text; at least
            // one argument is required. `trim_coerce` supplies the Int/Bool→text
            // rule and declines Float/Blob (their SQLite text form is subtle).
            if args.is_empty() {
                return Err(VmError::TypeMismatch("CONCAT expects at least 1 arg".into()));
            }
            let mut out = String::new();
            for a in &args {
                if let Some(s) = trim_coerce("CONCAT", a)? {
                    out.push_str(&s);
                }
            }
            Ok(SqlValue::Text(out))
        }

        "CONCAT_WS" => {
            // CONCAT_WS(sep, x, y, …): join the value arguments with `sep`. Unlike
            // CONCAT, a NULL value argument is SKIPPED entirely (not joined as
            // empty), so `concat_ws('-', 'a', NULL, 'c')` = 'a-c'. A NULL separator
            // makes the whole result NULL. At least two arguments are required
            // (the separator plus one value).
            if args.len() < 2 {
                return Err(VmError::TypeMismatch(format!(
                    "CONCAT_WS expects at least 2 args, got {}",
                    args.len()
                )));
            }
            let sep = match trim_coerce("CONCAT_WS", &args[0])? {
                None => return Ok(SqlValue::Null),
                Some(s) => s,
            };
            let mut parts: Vec<String> = Vec::new();
            for a in &args[1..] {
                if let Some(s) = trim_coerce("CONCAT_WS", a)? {
                    parts.push(s);
                }
            }
            Ok(SqlValue::Text(parts.join(&sep)))
        }

        // `SUBSTRING` is a spelling of `SUBSTR` — identical semantics.
        "SUBSTR" | "SUBSTRING" => {
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
            // SQLite SUBSTR is 1-indexed and counts *characters*. The index
            // arithmetic below reproduces SQLite's `substrFunc` exactly, so the
            // fiddly edge cases match: `pos = 0` (a virtual slot before the
            // first character), a negative `pos` counting from the right, and a
            // negative length that returns the |Z| characters *preceding* the
            // start. See the truth table in `sqlite_substr`.
            let chars: Vec<char> = s.chars().collect();
            let len_arg = if args.len() == 3 {
                if matches!(args[2], SqlValue::Null) {
                    return Ok(SqlValue::Null);
                }
                match &args[2] {
                    SqlValue::Int(n) => Some(*n),
                    other => {
                        return Err(VmError::TypeMismatch(format!(
                            "SUBSTR arg3 expects INTEGER, got {:?}",
                            other
                        )))
                    }
                }
            } else {
                None
            };
            Ok(SqlValue::Text(sqlite_substr(&chars, pos, len_arg)))
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
            // SQLite treats a NEGATIVE digit count as zero — it never rounds to
            // tens/hundreds. `round(2.567, -1)` is `round(2.567, 0)` = `3.0`, not
            // `0.0`. Clamp the low end here (leaving large positive counts alone,
            // where the value is already unchanged within f64 precision).
            let digits = digits.max(0);
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
            // NULL casts to an *empty blob*, so `hex(NULL)` is the empty string
            // `''` (a text value), NOT NULL. Floats are declined: their SQLite
            // text form (`2.0`, not Rust's `2`) is subtle enough that we don't
            // guess here.
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("HEX expects 1 arg, got {}", args.len())));
            }
            let bytes: Vec<u8> = match &args[0] {
                SqlValue::Null => return Ok(SqlValue::Text(String::new())),
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

        "UNHEX" => {
            // UNHEX(x) / UNHEX(x, ignore): the inverse of HEX — decode a string of
            // hexadecimal digit pairs into a blob. Case-insensitive.
            //
            //   unhex('414243')  -> x'414243'  ("ABC")
            //   unhex('')        -> x''          (empty blob)
            //   unhex('abc')     -> NULL         (odd number of digits)
            //   unhex('4g')      -> NULL         (non-hex character)
            //   unhex(12)        -> x'12'        (integer coerces to its digits)
            //
            // The optional second argument is a *set of ignorable characters*.
            // An ignorable character may appear only at a byte boundary — never
            // splitting a hex pair — matching SQLite exactly:
            //
            //   unhex('41.42', '.')   -> x'4142'   ('.' sits between pairs)
            //   unhex('4-1-4-2', '-') -> NULL      ('-' splits the pair '4'…'1')
            //
            // NULL in either argument yields NULL. Integer/boolean `x` coerces to
            // its decimal text (via `trim_coerce`); Float/Blob are declined, as
            // with HEX/QUOTE.
            if args.is_empty() || args.len() > 2 {
                return Err(VmError::TypeMismatch(format!("UNHEX expects 1 or 2 args, got {}", args.len())));
            }
            let x = match trim_coerce("UNHEX", &args[0])? {
                None => return Ok(SqlValue::Null),
                Some(s) => s,
            };
            let ignore: std::collections::HashSet<char> = if args.len() == 2 {
                match trim_coerce("UNHEX", &args[1])? {
                    None => return Ok(SqlValue::Null),
                    Some(s) => s.chars().collect(),
                }
            } else {
                std::collections::HashSet::new()
            };
            // Output is at most half the input length — bounded by the argument,
            // so no unbounded allocation.
            let mut out: Vec<u8> = Vec::with_capacity(x.len() / 2);
            let mut high: Option<u8> = None;
            for c in x.chars() {
                if let Some(v) = c.to_digit(16) {
                    match high {
                        None => high = Some(v as u8),
                        Some(h) => {
                            out.push(h * 16 + v as u8);
                            high = None;
                        }
                    }
                } else if ignore.contains(&c) {
                    // Only allowed at a byte boundary, never mid-pair.
                    if high.is_some() {
                        return Ok(SqlValue::Null);
                    }
                } else {
                    // Any other character invalidates the whole string.
                    return Ok(SqlValue::Null);
                }
            }
            if high.is_some() {
                return Ok(SqlValue::Null); // a trailing, unpaired hex digit
            }
            Ok(SqlValue::Blob(out))
        }

        "SIGN" => {
            // SIGN(x): -1, 0, or +1 for a negative, zero, or positive number;
            // NULL for NULL or a non-numeric argument.
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("SIGN expects 1 arg, got {}", args.len())));
            }
            let s = match &args[0] {
                SqlValue::Int(i) => (*i).signum(),
                SqlValue::Float(f) => {
                    if *f > 0.0 {
                        1
                    } else if *f < 0.0 {
                        -1
                    } else {
                        0
                    }
                }
                _ => return Ok(SqlValue::Null),
            };
            Ok(SqlValue::Int(s))
        }

        "UNICODE" => {
            // UNICODE(s): the code point of the first character of `s`; NULL for a
            // NULL or empty string.
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("UNICODE expects 1 arg, got {}", args.len())));
            }
            match &args[0] {
                SqlValue::Text(s) => match s.chars().next() {
                    Some(c) => Ok(SqlValue::Int(c as i64)),
                    None => Ok(SqlValue::Null),
                },
                SqlValue::Null => Ok(SqlValue::Null),
                other => Err(VmError::TypeMismatch(format!("UNICODE expects TEXT, got {:?}", other))),
            }
        }

        "CHAR" => {
            // CHAR(x1, x2, …): a string built from the characters whose code
            // points are the integer arguments. Non-integer or out-of-range code
            // points contribute nothing (SQLite is lax here); no args → "".
            let mut out = String::with_capacity(args.len());
            for a in &args {
                if let SqlValue::Int(cp) = a {
                    if let Some(c) = u32::try_from(*cp).ok().and_then(char::from_u32) {
                        out.push(c);
                    }
                }
            }
            Ok(SqlValue::Text(out))
        }

        "ZEROBLOB" => {
            // ZEROBLOB(n): a BLOB of `n` zero bytes (n < 0 → empty). NULL → NULL.
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("ZEROBLOB expects 1 arg, got {}", args.len())));
            }
            match &args[0] {
                SqlValue::Null => Ok(SqlValue::Null),
                SqlValue::Int(n) => {
                    let len = (*n).max(0) as usize;
                    // Cap the eager allocation. `n` is any i64 the query names, so
                    // `zeroblob(9999999999)` would otherwise request ~10 GB and
                    // OOM/abort the process. Real SQLite likewise errors past its
                    // SQLITE_MAX_LENGTH; we reuse the engine's 1e6 guard (as with
                    // GROUP BY / COUNT(DISTINCT)) and surface ResourceLimit.
                    const MAX_BLOB_LEN: usize = 1_000_000;
                    if len > MAX_BLOB_LEN {
                        return Err(VmError::ResourceLimit(format!(
                            "ZEROBLOB length {len} exceeds limit {MAX_BLOB_LEN}"
                        )));
                    }
                    Ok(SqlValue::Blob(vec![0u8; len]))
                }
                other => Err(VmError::TypeMismatch(format!("ZEROBLOB expects INTEGER, got {:?}", other))),
            }
        }

        "QUOTE" => {
            // QUOTE(x): the value as an SQL literal — NULL as `NULL`, text
            // single-quoted with doubled inner quotes, a blob as `X'…'` hex, and
            // an integer as its digits. (Floats are declined; their exact SQLite
            // text form is subtle, like HEX above.)
            if args.len() != 1 {
                return Err(VmError::TypeMismatch(format!("QUOTE expects 1 arg, got {}", args.len())));
            }
            let lit = match &args[0] {
                SqlValue::Null => "NULL".to_string(),
                SqlValue::Int(i) => i.to_string(),
                SqlValue::Bool(b) => (*b as i64).to_string(),
                SqlValue::Text(s) => format!("'{}'", s.replace('\'', "''")),
                SqlValue::Blob(b) => {
                    let mut h = String::with_capacity(b.len() * 2 + 3);
                    h.push_str("X'");
                    for byte in b {
                        h.push_str(&format!("{byte:02X}"));
                    }
                    h.push('\'');
                    h
                }
                other => return Err(VmError::TypeMismatch(format!("QUOTE does not support {:?}", other))),
            };
            Ok(SqlValue::Text(lit))
        }

        "MAX" | "MIN" => {
            // The SCALAR forms of MAX/MIN — two-or-more arguments — return the
            // largest / smallest argument, or NULL if ANY argument is NULL
            // (SQLite semantics). The single-argument forms are the AGGREGATE
            // max/min and are compiled to `FinalizeAgg`, never reaching here; the
            // planner routes only the 2+-argument calls to `call_builtin`.
            if args.is_empty() {
                return Err(VmError::TypeMismatch(format!("{name} expects at least 1 arg")));
            }
            if args.iter().any(|a| matches!(a, SqlValue::Null)) {
                return Ok(SqlValue::Null);
            }
            let want_max = name == "MAX";
            let mut best = args[0].clone();
            for a in &args[1..] {
                let ord = sql_cmp(a, &best);
                let take = if want_max {
                    ord == std::cmp::Ordering::Greater
                } else {
                    ord == std::cmp::Ordering::Less
                };
                if take {
                    best = a.clone();
                }
            }
            Ok(best)
        }

        "IIF" => {
            // IIF(x, y, z) — SQLite's function-form conditional, equivalent to
            // `CASE WHEN x THEN y ELSE z END`: `y` when `x` is truthy (SQL
            // three-valued logic — a NULL or falsy `x` picks `z`). Arguments are
            // already evaluated here; since this engine's expressions have no
            // side effects, eagerly evaluating both branches is observationally
            // identical to CASE's short-circuit.
            if args.len() != 3 {
                return Err(VmError::TypeMismatch(format!("IIF expects 3 args, got {}", args.len())));
            }
            let pick_then = is_truthy(&args[0]);
            let mut it = args.into_iter();
            let _cond = it.next();
            let y = it.next().unwrap();
            let z = it.next().unwrap();
            Ok(if pick_then { y } else { z })
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
/// | Text / Blob | numeric affinity ≠ 0 (`'5'`→true, `'abc'`/`'0'`/`''`→false) |
fn is_truthy(v: &SqlValue) -> bool {
    match v {
        SqlValue::Null => false,
        SqlValue::Bool(b) => *b,
        SqlValue::Int(n) => *n != 0,
        SqlValue::Float(f) => *f != 0.0,
        // A text/blob in a boolean context takes NUMERIC AFFINITY first, exactly
        // like SQLite: `WHERE 'abc'` is false (`'abc'`→0), `WHERE '5'` is true,
        // `NOT 'abc'` = 1, `NOT '5'` = 0. Previously every non-NULL text/blob was
        // truthy, which wrongly kept `WHERE <text-column>` rows and inverted
        // `NOT`. `cast_to_f64` takes the leading numeric prefix (0 for non-numeric).
        SqlValue::Text(_) | SqlValue::Blob(_) => cast_to_f64(v) != 0.0,
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
            // SQLite returns NULL for division by zero (integer OR float) — e.g.
            // `SELECT 5/0`, `5.0/0`, and `0/0` all yield NULL, never an error.
            // NULL operands were already short-circuited above, so a zero divisor
            // here is a genuine value. `*f == 0.0` also matches `-0.0`.
            // Numeric affinity applies first, so `5 / '0'` is NULL and `5 / '2'`
            // is 2, matching SQLite.
            let (l, r) = (coerce_arith(l), coerce_arith(r));
            match (&l, &r) {
                (_, SqlValue::Int(0)) => Ok(SqlValue::Null),
                (_, SqlValue::Float(f)) if *f == 0.0 => Ok(SqlValue::Null),
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
        BinaryOp::Mod => {
          // Numeric affinity applies first (`'7' % 3` = 1); then modulo by zero
          // is NULL in SQLite (`5%0`, `5.5%0`, `5%0.0`), not an error.
          let (l, r) = (coerce_arith(l), coerce_arith(r));
          match (&l, &r) {
            (_, SqlValue::Int(0)) => Ok(SqlValue::Null),
            (_, SqlValue::Float(f)) if *f == 0.0 => Ok(SqlValue::Null),
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
          }
        }

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
            // `||` yields TEXT. A BLOB operand contributes its RAW bytes (as
            // text), NOT the `x'…'` hex-literal spelling `sql_to_str` uses for
            // display: SQLite evaluates `X'41' || 'B'` to `'AB'` (0x41 = 'A'),
            // not `"x'41'B"`. Other types stringify as usual.
            let ls = concat_operand_to_str(&l);
            let rs = concat_operand_to_str(&r);
            Ok(SqlValue::Text(ls + &rs))
        }

        // ── Bitwise ───────────────────────────────────────────────────────────
        //
        // Both operands are coerced to integer (SQLite integer affinity: reals
        // truncate toward zero, text prefix-parses). NULL was already handled
        // above, so these never see NULL. `&`/`|` are plain i64 bit ops; shifts
        // follow SQLite's saturate-and-negate rules in `sql_shift`.
        BinaryOp::BitAnd => Ok(SqlValue::Int(cast_to_i64(&l) & cast_to_i64(&r))),
        BinaryOp::BitOr => Ok(SqlValue::Int(cast_to_i64(&l) | cast_to_i64(&r))),
        BinaryOp::ShiftLeft => {
            Ok(SqlValue::Int(sql_shift(cast_to_i64(&l), cast_to_i64(&r), true)))
        }
        BinaryOp::ShiftRight => {
            Ok(SqlValue::Int(sql_shift(cast_to_i64(&l), cast_to_i64(&r), false)))
        }

        // AND/OR already handled above.
        BinaryOp::And | BinaryOp::Or => unreachable!(),
    }
}

/// SQLite's bit-shift semantics for `value << count` / `value >> count`.
///
/// Rust's own `<<`/`>>` are Undefined Behaviour (they panic in debug) once the
/// shift amount reaches the type width, so we cannot use them directly on
/// attacker-controlled counts. SQLite instead defines every count precisely
/// (verified against the C library):
///
/// | input        | result | why                                        |
/// |--------------|--------|--------------------------------------------|
/// | `1 << 64`    | 0      | count ≥ 64 on a left shift → 0             |
/// | `8 >> 100`   | 0      | count ≥ 64, value ≥ 0 → 0                  |
/// | `-1 >> 1`    | -1     | right shift is arithmetic (sign-extending) |
/// | `-4 >> 100`  | -1     | count ≥ 64, value < 0 → -1                 |
/// | `1 << -1`    | 0      | negative count flips direction: `1 >> 1`   |
///
/// The rules: a negative count shifts the *other* direction by its magnitude; a
/// count ≥ 64 saturates (left → 0; right → 0 for a non-negative value, −1 for a
/// negative one because the sign bit fills); otherwise a normal shift, using an
/// unsigned left shift so it can never overflow and an `i64` (arithmetic) right
/// shift so negatives sign-extend — exactly matching SQLite's implementation.
fn sql_shift(value: i64, count: i64, left: bool) -> i64 {
    let mut do_left = left;
    // Magnitude of the shift; a negative count means shift the other way.
    // `i64::MIN.unsigned_abs()` is 2^63, well past 64, so it saturates below.
    let amount: u64 = if count < 0 {
        do_left = !do_left;
        count.unsigned_abs()
    } else {
        count as u64
    };

    if amount >= 64 {
        // Saturated: a left shift (or any shift of a non-negative value) yields
        // 0; a right shift of a negative value fills with sign bits → -1.
        return if value >= 0 || do_left { 0 } else { -1 };
    }
    let amount = amount as u32;
    if do_left {
        // Unsigned shift cannot be UB and matches SQLite's `(i64)((u64)iA<<iB)`.
        (value as u64).wrapping_shl(amount) as i64
    } else {
        // i64 `>>` is arithmetic in Rust, sign-extending like SQLite.
        value >> amount
    }
}

/// Helper for symmetric int/float arithmetic with checked integer operations.
///
/// `int_op` is a checked variant (returns `Option<i64>`); `float_op` is unchecked
/// (IEEE 754 overflow saturates to ±∞ which is the standard SQL behaviour).
/// Apply SQLite numeric affinity to an arithmetic operand. Text/blob take their
/// leading numeric prefix (`'5'`→5, `'5.5'`→5.5, `'12abc'`→12, `'abc'`→0, via
/// [`text_to_numeric`]); a bool is its integer value; INTEGER/REAL pass through.
/// NULL never reaches here — `eval_binary` short-circuits NULL operands before
/// dispatching to arithmetic. Scoped to arithmetic only: comparison and bitwise
/// operators apply their own (different) coercion rules.
fn coerce_arith(v: SqlValue) -> SqlValue {
    match v {
        SqlValue::Int(_) | SqlValue::Float(_) => v,
        SqlValue::Bool(b) => SqlValue::Int(b as i64),
        SqlValue::Text(s) => text_to_numeric(&s),
        SqlValue::Blob(b) => text_to_numeric(&String::from_utf8_lossy(&b)),
        SqlValue::Null => SqlValue::Null,
    }
}

fn checked_int_binop(
    l: SqlValue,
    r: SqlValue,
    int_op: impl Fn(i64, i64) -> Option<i64>,
    float_op: impl Fn(f64, f64) -> f64,
    op_name: &'static str,
) -> Result<SqlValue, VmError> {
    // SQLite applies numeric affinity to arithmetic operands: `'5' + 0` = 5.
    let (l, r) = (coerce_arith(l), coerce_arith(r));
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
            UnaryOp::Neg => {
                // SQLite applies numeric affinity to the operand *before*
                // negating, so `-` on text/blob coerces first:
                //   `-'5'`   = -5      `-'12abc'` = -12    `-'abc'` = 0
                //   `-'3.5'` = -3.5    `-'  7'`   = -7     `-TRUE`  = -1
                // A leading numeric prefix is taken (whitespace-trimmed) and the
                // rest ignored; text with no numeric prefix is 0. `text_to_numeric`
                // is the shared affinity helper (Int when integral, else Float).
                // Known edge left for later: a string in *exponent* form such as
                // `'3e2'` stays REAL in SQLite (`-'3e2'` = -300.0) but collapses to
                // an integer here — see the float-affinity follow-up.
                let num = match v {
                    SqlValue::Int(_) | SqlValue::Float(_) => v,
                    SqlValue::Bool(b) => SqlValue::Int(b as i64),
                    SqlValue::Text(s) => text_to_numeric(&s),
                    SqlValue::Blob(b) => text_to_numeric(&String::from_utf8_lossy(&b)),
                    SqlValue::Null => unreachable!("NULL handled by the outer match"),
                };
                match num {
                    // -i64::MIN overflows; use checked_neg to return an error
                    // instead of panicking (debug) or wrapping (release).
                    SqlValue::Int(n) => n
                        .checked_neg()
                        .map(SqlValue::Int)
                        .ok_or_else(|| VmError::TypeMismatch(
                            "integer overflow in unary negation (value is i64::MIN)".to_string(),
                        )),
                    SqlValue::Float(f) => Ok(SqlValue::Float(-f)),
                    _ => unreachable!("text_to_numeric returns Int or Float"),
                }
            }
            UnaryOp::Not => Ok(SqlValue::Bool(!is_truthy(&v))),
            // `~x` — coerce to integer (SQLite integer affinity) then complement.
            // `~0` = -1, `~-1` = 0. NULL was handled above.
            UnaryOp::BitNot => Ok(SqlValue::Int(!cast_to_i64(&v))),
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
    plain: bool,
) -> Result<SqlValue, VmError> {
    // NULL propagation: any NULL → NULL (three-valued logic).
    if matches!(val, SqlValue::Null) || matches!(lo, SqlValue::Null) || matches!(hi, SqlValue::Null) {
        return Ok(SqlValue::Null);
    }
    // The BETWEEN range test is *inclusive*: `val >= lo AND val <= hi`.
    //
    //   x   BETWEEN a AND c  ≡  a <= x <= c
    //   x NOT BETWEEN a AND c ≡  NOT(a <= x <= c)  ≡  x < a OR x > c
    //
    // `plain` is codegen's `!negated` (the ONLY producer of `Between(false)` is
    // `NOT BETWEEN`). `NOT BETWEEN` is the LOGICAL NEGATION of the inclusive
    // range — it must NOT be computed as a strict/exclusive-bounds range
    // (`val > lo AND val < hi`). The earlier code did exactly that, which
    // inverted the answer for interior values: `5 NOT BETWEEN 1 AND 10` wrongly
    // returned true (5 is *in* [1,10], so `NOT BETWEEN` is false), and
    // `15 NOT BETWEEN 1 AND 10` wrongly returned false. NULL already returned
    // above, so the negation is a plain boolean flip.
    let in_range = matches!(sql_cmp(val, lo), std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
        && matches!(sql_cmp(val, hi), std::cmp::Ordering::Less | std::cmp::Ordering::Equal);
    Ok(SqlValue::Bool(if plain { in_range } else { !in_range }))
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

/// `LIKE` matching with an `ESCAPE` character, per SQLite. Identical to
/// [`like_match`] except that `escape` immediately before a `%`, `_`, or the
/// escape character itself makes that character a **literal** (matched
/// case-insensitively) rather than a wildcard — so `'100%' LIKE '100\%'
/// ESCAPE '\'` is true and matches only a trailing percent sign.
fn like_match_escape(text: &str, pattern: &str, escape: char) -> bool {
    let t: Vec<char> = text.chars().collect();
    let p: Vec<char> = pattern.chars().collect();

    let (mut ti, mut pi) = (0usize, 0usize);
    let (mut star_pi, mut star_ti) = (usize::MAX, 0usize);

    // `chars_eq` folds ASCII/Unicode case exactly as the wildcard-free path does.
    let chars_eq = |a: char, b: char| a.to_lowercase().next() == b.to_lowercase().next();

    while ti < t.len() {
        if pi + 1 < p.len() && p[pi] == escape {
            // Escaped literal: the character after `escape` must match verbatim.
            if chars_eq(p[pi + 1], t[ti]) {
                ti += 1;
                pi += 2;
            } else if star_pi != usize::MAX {
                star_ti += 1;
                ti = star_ti;
                pi = star_pi + 1;
            } else {
                return false;
            }
        } else if pi < p.len() && p[pi] == '%' {
            // `%` matches zero or more; record the backtrack point.
            star_pi = pi;
            pi += 1;
            star_ti = ti;
        } else if pi < p.len() && (p[pi] == '_' || chars_eq(p[pi], t[ti])) {
            ti += 1;
            pi += 1;
        } else if star_pi != usize::MAX {
            star_ti += 1;
            ti = star_ti;
            pi = star_pi + 1;
        } else {
            return false;
        }
    }

    // Only trailing `%` can match the empty suffix; an escaped literal or `_`
    // still demands a character, so the pattern fails to consume fully.
    while pi < p.len() && p[pi] == '%' {
        pi += 1;
    }

    pi == p.len()
}

/// Try to match a GLOB character class `[...]` in `p` starting at `p[start]`
/// (which must be `'['`) against `ch`. Returns `Some((matched, after))` where
/// `after` is the pattern index just past the closing `']'`; or `None` if there
/// is no closing `']'`, in which case the caller treats `'['` as a literal.
///
/// A leading `^` negates the class. A `]` immediately after `[` (or `[^`) is a
/// literal member, not the closer. `a-c` is an inclusive range.
fn glob_class_match(p: &[char], start: usize, ch: char) -> Option<(bool, usize)> {
    let mut i = start + 1;
    let negate = i < p.len() && p[i] == '^';
    if negate {
        i += 1;
    }
    let first = i; // a `]` here is a literal member, not the class close
    let mut matched = false;
    while i < p.len() {
        if p[i] == ']' && i != first {
            let result = if negate { !matched } else { matched };
            return Some((result, i + 1));
        }
        // `x-y` range (but not when `-` is the class's closing-adjacent char).
        if i + 2 < p.len() && p[i + 1] == '-' && p[i + 2] != ']' {
            if p[i] <= ch && ch <= p[i + 2] {
                matched = true;
            }
            i += 3;
        } else {
            if p[i] == ch {
                matched = true;
            }
            i += 1;
        }
    }
    None // unterminated class
}

/// Whether the single GLOB pattern element at `p[pi]` (a literal, `?`, or a
/// `[...]` class) matches `ch`. Returns the pattern index after the element on a
/// match, or `None` on no match. Does not handle `*` (the caller does).
fn glob_single(p: &[char], pi: usize, ch: char) -> Option<usize> {
    match p[pi] {
        '?' => Some(pi + 1),
        '[' => match glob_class_match(p, pi, ch) {
            Some((true, after)) => Some(after),
            Some((false, _)) => None,
            None => (ch == '[').then_some(pi + 1), // literal '['
        },
        c => (c == ch).then_some(pi + 1),
    }
}

/// Coerce a value to the integer a `printf` `%d`/`%x`/… conversion expects.
/// Matches SQLite: an integer is itself; a float truncates toward zero; text
/// contributes its leading integer (`'12ab'` → 12, `'abc'` → 0); NULL and other
/// types are 0.
fn printf_int(v: &SqlValue) -> i64 {
    match v {
        SqlValue::Int(i) => *i,
        SqlValue::Bool(b) => *b as i64,
        SqlValue::Float(f) => *f as i64,
        SqlValue::Text(s) => {
            // Parse an optional sign followed by ASCII digits, stopping at the
            // first non-digit — exactly what SQLite's implicit text→int cast does.
            let t = s.trim_start();
            let mut chars = t.chars().peekable();
            let mut neg = false;
            match chars.peek() {
                Some('-') => {
                    neg = true;
                    chars.next();
                }
                Some('+') => {
                    chars.next();
                }
                _ => {}
            }
            let mut n: i64 = 0;
            for c in chars {
                if let Some(d) = c.to_digit(10) {
                    n = n.saturating_mul(10).saturating_add(d as i64);
                } else {
                    break;
                }
            }
            if neg {
                -n
            } else {
                n
            }
        }
        _ => 0,
    }
}

/// Coerce a value to the string a `printf` `%s`/`%q`/`%c` conversion expects.
/// Text is itself; an integer/boolean becomes its decimal text; NULL becomes the
/// empty string. Floats and blobs are declined (their exact SQLite text form is
/// subtle — same convention as HEX/QUOTE).
fn printf_str(name: &str, v: &SqlValue) -> Result<String, VmError> {
    match v {
        SqlValue::Null => Ok(String::new()),
        SqlValue::Text(s) => Ok(s.clone()),
        SqlValue::Int(i) => Ok(i.to_string()),
        SqlValue::Bool(b) => Ok((*b as i64).to_string()),
        other => Err(VmError::TypeMismatch(format!(
            "{name}: %s/%q of {other:?} is unsupported"
        ))),
    }
}

/// A minimal, DoS-bounded implementation of SQLite's `printf`/`format`.
///
/// Supports the conversions `%d`/`%i`, `%s` (with `.precision` truncation),
/// `%x`/`%X`, `%o`, `%c` (the first character of the argument's text), `%q`
/// (single-quotes doubled, for SQL-literal building) and `%%`; with the flags
/// `-` (left-justify), `0` (zero-pad numbers), `+` and space (sign on positives),
/// and a field width. Float conversions (`%f`/`%g`/`%e`) are declined — their
/// exact SQLite text form is the same subtlety HEX/QUOTE avoid.
///
/// Missing arguments default to `0` (numeric) or `""` (string), and extra
/// arguments are ignored — matching SQLite. Width and precision are capped, and
/// the running output is capped, so a hostile format like `printf('%9999999999d')`
/// cannot drive an unbounded allocation.
fn sql_printf(name: &str, format: &str, args: &[SqlValue]) -> Result<String, VmError> {
    const MAX_FIELD: usize = 1_000_000; // per-field width/precision cap
    const MAX_OUTPUT: usize = 10_000_000; // total output cap

    let fmt: Vec<char> = format.chars().collect();
    let mut out = String::new();
    let mut argi = 0usize;
    let mut i = 0usize;

    let take_field = |i: &mut usize| -> Result<usize, VmError> {
        let mut n = 0usize;
        while *i < fmt.len() && fmt[*i].is_ascii_digit() {
            n = n
                .saturating_mul(10)
                .saturating_add((fmt[*i] as u8 - b'0') as usize);
            *i += 1;
        }
        if n > MAX_FIELD {
            return Err(VmError::ResourceLimit(format!(
                "{name}: field size {n} exceeds limit {MAX_FIELD}"
            )));
        }
        Ok(n)
    };

    while i < fmt.len() {
        if fmt[i] != '%' {
            out.push(fmt[i]);
            i += 1;
            continue;
        }
        i += 1; // consume '%'
        // Flags.
        let (mut left, mut zero, mut plus, mut space) = (false, false, false, false);
        while i < fmt.len() {
            match fmt[i] {
                '-' => left = true,
                '0' => zero = true,
                '+' => plus = true,
                ' ' => space = true,
                '#' => {} // alternate form — accepted and ignored
                _ => break,
            }
            i += 1;
        }
        let width = take_field(&mut i)?;
        let precision = if i < fmt.len() && fmt[i] == '.' {
            i += 1;
            Some(take_field(&mut i)?)
        } else {
            None
        };
        let Some(&spec) = fmt.get(i) else {
            // A trailing, incomplete conversion: emit a literal '%'.
            out.push('%');
            break;
        };
        i += 1;

        if spec == '%' {
            out.push('%');
            continue;
        }

        // Build the converted body and note whether it is a signed number (so a
        // `0` flag pads between the sign and the digits). Arms that `continue`
        // or `return` diverge, so the match is still a `String` expression.
        let mut sign = String::new();
        let body: String = match spec {
            'd' | 'i' => {
                let n = printf_int(args.get(argi).unwrap_or(&SqlValue::Int(0)));
                argi += 1;
                if n < 0 {
                    sign.push('-');
                } else if plus {
                    sign.push('+');
                } else if space {
                    sign.push(' ');
                }
                n.unsigned_abs().to_string()
            }
            'x' => {
                let n = printf_int(args.get(argi).unwrap_or(&SqlValue::Int(0)));
                argi += 1;
                format!("{:x}", n as u64)
            }
            'X' => {
                let n = printf_int(args.get(argi).unwrap_or(&SqlValue::Int(0)));
                argi += 1;
                format!("{:X}", n as u64)
            }
            'o' => {
                let n = printf_int(args.get(argi).unwrap_or(&SqlValue::Int(0)));
                argi += 1;
                format!("{:o}", n as u64)
            }
            's' => {
                let mut s = printf_str(name, args.get(argi).unwrap_or(&SqlValue::Null))?;
                argi += 1;
                if let Some(p) = precision {
                    s = s.chars().take(p).collect();
                }
                s
            }
            'c' => {
                let s = printf_str(name, args.get(argi).unwrap_or(&SqlValue::Null))?;
                argi += 1;
                s.chars().next().map(|c| c.to_string()).unwrap_or_default()
            }
            'q' => {
                let s = printf_str(name, args.get(argi).unwrap_or(&SqlValue::Null))?;
                argi += 1;
                s.replace('\'', "''")
            }
            other => {
                return Err(VmError::TypeMismatch(format!(
                    "{name}: unsupported conversion %{other}"
                )));
            }
        };

        // Assemble sign + body + width padding.
        let content_len = sign.chars().count() + body.chars().count();
        let pad = width.saturating_sub(content_len);
        if left {
            out.push_str(&sign);
            out.push_str(&body);
            out.extend(std::iter::repeat_n(' ', pad));
        } else if zero && matches!(spec, 'd' | 'i' | 'x' | 'X' | 'o') {
            out.push_str(&sign);
            out.extend(std::iter::repeat_n('0', pad));
            out.push_str(&body);
        } else {
            out.extend(std::iter::repeat_n(' ', pad));
            out.push_str(&sign);
            out.push_str(&body);
        }

        if out.len() > MAX_OUTPUT {
            return Err(VmError::ResourceLimit(format!(
                "{name}: output exceeds limit {MAX_OUTPUT}"
            )));
        }
    }

    Ok(out)
}

/// GLOB pattern match: case-sensitive, `*` = any run, `?` = any single char,
/// `[...]` = character class (`[^...]` negated, `a-c` ranges). Unlike LIKE, a
/// backslash is a literal character (GLOB has no escape). Uses the same
/// iterative two-pointer backtracking as [`like_match`], so it is `O(text ×
/// pattern)` — no exponential blow-up on adversarial `*`-heavy patterns.
fn glob_match(text: &str, pattern: &str) -> bool {
    let t: Vec<char> = text.chars().collect();
    let p: Vec<char> = pattern.chars().collect();

    let (mut ti, mut pi) = (0usize, 0usize);
    // Backtrack point: pattern index just after the last `*`, and the text index
    // it started matching from.
    let (mut star_pi, mut star_ti): (Option<usize>, usize) = (None, 0);

    while ti < t.len() {
        if pi < p.len() && p[pi] == '*' {
            star_pi = Some(pi);
            star_ti = ti;
            pi += 1;
        } else if let Some(next_pi) = p.get(pi).and_then(|_| glob_single(&p, pi, t[ti])) {
            pi = next_pi;
            ti += 1;
        } else if let Some(sp) = star_pi {
            // The last `*` consumes one more text character.
            star_ti += 1;
            ti = star_ti;
            pi = sp + 1;
        } else {
            return false;
        }
    }

    // Any trailing `*` wildcards match the empty suffix.
    while pi < p.len() && p[pi] == '*' {
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

/// Stringify an operand of the `||` (concatenate) operator.
///
/// Identical to [`sql_to_str`] EXCEPT for blobs: `||` treats a blob as its raw
/// byte sequence interpreted as text, so `X'41' || 'B'` is `'AB'` (0x41 = 'A'),
/// whereas [`sql_to_str`] renders the reversible display form `x'41'`. Invalid
/// UTF-8 bytes become the replacement character (U+FFFD) — a rare edge for
/// non-textual blobs; the common ASCII/UTF-8 case round-trips exactly. NULL is
/// handled by the caller (it propagates through `||`).
fn concat_operand_to_str(v: &SqlValue) -> String {
    match v {
        SqlValue::Blob(b) => String::from_utf8_lossy(b).into_owned(),
        other => sql_to_str(other),
    }
}

/// Apply a `CAST(value AS type)` conversion, following SQLite's documented
/// rules for the three supported target types. NULL always casts to NULL.
///
/// - **INTEGER**: reals truncate toward zero; text yields the longest leading
///   *integer* prefix (`'3.9'` → 3 because it stops at the `.`, `'12abc'` → 12,
///   `'abc'` → 0), leading whitespace ignored.
/// - **REAL**: text yields the longest leading *real* prefix (`'1e3'` → 1000.0,
///   `'12.5abc'` → 12.5, `'abc'` → 0.0).
/// - **TEXT**: the value's text representation (integers become their decimal
///   string; a boolean — which SQLite has no type for — renders as `1`/`0`).
fn apply_cast(val: &SqlValue, ty: &CastType) -> SqlValue {
    if matches!(val, SqlValue::Null) {
        return SqlValue::Null;
    }
    match ty {
        CastType::Integer => SqlValue::Int(cast_to_i64(val)),
        CastType::Real => SqlValue::Float(cast_to_f64(val)),
        CastType::Text => SqlValue::Text(match val {
            // SQLite has no boolean type; a stored bool casts like the integer
            // 1/0 it stands in for, not the words "true"/"false".
            SqlValue::Bool(b) => (*b as i64).to_string(),
            _ => sql_to_str(val),
        }),
        CastType::Numeric => cast_to_numeric(val),
    }
}

/// Value → INTEGER or REAL for `CAST(… AS NUMERIC)` (SQLite's NUMERIC affinity).
///
/// A number is left unchanged — an INTEGER stays INTEGER and a REAL stays REAL,
/// so `CAST(3.0 AS NUMERIC)` is `3.0`, not `3`. Text and blob are parsed to a
/// number, preferring INTEGER when the value is integral and fits `i64`,
/// otherwise REAL (see [`text_to_numeric`]). NULL was already handled by the
/// caller.
fn cast_to_numeric(val: &SqlValue) -> SqlValue {
    match val {
        SqlValue::Int(i) => SqlValue::Int(*i),
        // A stored bool stands in for the integer 1/0.
        SqlValue::Bool(b) => SqlValue::Int(*b as i64),
        SqlValue::Float(f) => SqlValue::Float(*f),
        SqlValue::Text(s) => text_to_numeric(s),
        SqlValue::Blob(b) => text_to_numeric(&String::from_utf8_lossy(b)),
        SqlValue::Null => SqlValue::Null,
    }
}

/// Parse text to a NUMERIC value, matching SQLite's `sqlite3VdbeMemNumerify`:
/// prefer INTEGER when the leading numeric prefix denotes an integer that fits
/// `i64`, otherwise REAL. Non-numeric text yields `0` (integer), like the other
/// numeric casts.
///
/// | input        | result      | why                                    |
/// |--------------|-------------|----------------------------------------|
/// | `'42'`       | `Int(42)`   | pure integer prefix                    |
/// | `'42abc'`    | `Int(42)`   | integer prefix, trailing junk ignored  |
/// | `'3.0'`      | `Int(3)`    | real syntax, but value is integral     |
/// | `'1e3'`      | `Int(1000)` | exponent, but value is integral        |
/// | `'3.5'`      | `Float(3.5)`| non-integral                           |
/// | `'9e99'`     | `Float(..)` | integral but overflows i64 → real      |
/// | `'abc'`      | `Int(0)`    | no numeric prefix                       |
fn text_to_numeric(s: &str) -> SqlValue {
    let t = s.trim_start();
    let bytes = t.as_bytes();
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let digit_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let has_int_digits = i > digit_start;
    // A `.`/`e`/`E` immediately after the integer digits means the prefix is a
    // real literal, not a plain integer.
    let real_syntax =
        i < bytes.len() && matches!(bytes[i], b'.' | b'e' | b'E');

    if has_int_digits && !real_syntax {
        // Pure integer prefix (e.g. `42`, `-7`, `42abc`). Parse it exactly so an
        // i64 overflow falls through to the real value rather than saturating.
        if let Ok(n) = t[..i].parse::<i64>() {
            return SqlValue::Int(n);
        }
        return SqlValue::Float(parse_real_prefix(s));
    }

    // Real-syntax or digit-less prefix: take the real value, then collapse it to
    // an integer when it is integral and fits i64 (`3.0`→3, `1e3`→1000), else
    // keep it real (`3.5`, overflow). `parse_real_prefix` returns 0.0 for
    // non-numeric text, so `'abc'`→`Int(0)`.
    let r = parse_real_prefix(s);
    // 2^63 as f64; the half-open range keeps `r as i64` exact (no saturation).
    const I64_LIMIT: f64 = 9_223_372_036_854_775_808.0;
    if r.is_finite() && r.fract() == 0.0 && (-I64_LIMIT..I64_LIMIT).contains(&r) {
        SqlValue::Int(r as i64)
    } else {
        SqlValue::Float(r)
    }
}

/// Value → i64 for `CAST(… AS INTEGER)`. Float truncates toward zero (and
/// saturates on overflow, matching Rust's `as i64`); text parses a leading
/// integer prefix.
fn cast_to_i64(val: &SqlValue) -> i64 {
    match val {
        SqlValue::Int(i) => *i,
        SqlValue::Bool(b) => *b as i64,
        SqlValue::Float(f) => *f as i64,
        SqlValue::Text(s) => parse_int_prefix(s),
        SqlValue::Blob(b) => parse_int_prefix(&String::from_utf8_lossy(b)),
        SqlValue::Null => 0,
    }
}

/// Value → f64 for `CAST(… AS REAL)`. Text parses a leading real prefix.
fn cast_to_f64(val: &SqlValue) -> f64 {
    match val {
        SqlValue::Int(i) => *i as f64,
        SqlValue::Bool(b) => *b as i64 as f64,
        SqlValue::Float(f) => *f,
        SqlValue::Text(s) => parse_real_prefix(s),
        SqlValue::Blob(b) => parse_real_prefix(&String::from_utf8_lossy(b)),
        SqlValue::Null => 0.0,
    }
}

/// SQLite's `substr(X, Y, Z)` window over `chars`, returning the selected text.
/// `pos` is `Y` (1-indexed); `len_arg` is `Some(Z)` for the 3-arg form or `None`
/// for the 2-arg form (which runs to the end of the string). This mirrors
/// SQLite's `substrFunc` index arithmetic byte-for-byte, so every edge case
/// agrees:
///
/// | call                    | result   | rule                                    |
/// |-------------------------|----------|-----------------------------------------|
/// | `substr('hello',2,3)`   | `'ell'`  | ordinary 1-indexed slice                |
/// | `substr('hello',-2)`    | `'lo'`   | negative Y counts from the right        |
/// | `substr('hello',0)`     | `'hello'`| Y=0 is a slot before char 1 (2-arg)     |
/// | `substr('hello',0,2)`   | `'h'`    | Y=0 with length consumes one from Z     |
/// | `substr('hello',2,-1)`  | `'h'`    | negative Z: the |Z| chars *before* Y    |
/// | `substr('hello',5,-2)`  | `'ll'`   | negative Z reads leftward               |
fn sqlite_substr(chars: &[char], pos: i64, len_arg: Option<i64>) -> String {
    let len = chars.len() as i64;
    let mut p1 = pos;

    // All arithmetic is saturating: `pos`/`len_arg` are attacker-controlled i64
    // values, so `i64::MIN`/`i64::MAX` must never overflow (debug builds panic
    // on overflow). Saturation only affects out-of-range inputs, which the final
    // `clamp(0, len)` collapses to an empty or full window anyway.
    let (start, end) = match len_arg {
        Some(mut z) => {
            if p1 < 0 {
                // Negative start counts from the right; if it lands before the
                // string, the shortfall eats into the length.
                p1 = p1.saturating_add(len);
                if p1 < 0 {
                    z = z.saturating_add(p1);
                    if z < 0 {
                        z = 0;
                    }
                    p1 = 0;
                }
            } else if p1 > 0 {
                p1 -= 1; // 1-indexed → 0-indexed
            } else if z > 0 {
                // p1 == 0: the virtual slot before char 1 costs one of Z.
                z -= 1;
            }
            if z < 0 {
                // Negative length: return the |Z| characters preceding `p1`.
                p1 = p1.saturating_add(z);
                z = z.saturating_neg(); // `i64::MIN` → `i64::MAX`, no overflow
                if p1 < 0 {
                    z = z.saturating_add(p1);
                    p1 = 0;
                }
            }
            let start = p1.clamp(0, len);
            let end = p1.saturating_add(z.max(0)).clamp(0, len);
            (start, end.max(start))
        }
        None => {
            // 2-arg form: from `p1` to the end of the string.
            if p1 < 0 {
                p1 = p1.saturating_add(len).max(0);
            } else if p1 > 0 {
                p1 -= 1;
            }
            (p1.clamp(0, len), len)
        }
    };

    chars[start as usize..end as usize].iter().collect()
}

/// The longest leading substring of `s` that is a well-formed integer
/// (optional sign + digits, after skipping leading whitespace), parsed to
/// i64. No such prefix → 0; digit overflow saturates to i64::MIN/MAX.
fn parse_int_prefix(s: &str) -> i64 {
    let t = s.trim_start();
    let bytes = t.as_bytes();
    let mut i = 0;
    let mut neg = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        neg = bytes[i] == b'-';
        i += 1;
    }
    let start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == start {
        return 0;
    }
    match t[start..i].parse::<i64>() {
        Ok(n) => {
            if neg {
                -n
            } else {
                n
            }
        }
        Err(_) => {
            if neg {
                i64::MIN
            } else {
                i64::MAX
            }
        }
    }
}

/// The longest leading substring of `s` that is a well-formed real number
/// (optional sign, digits, fractional part, and exponent — after skipping
/// leading whitespace), parsed to f64. No such prefix → 0.0.
fn parse_real_prefix(s: &str) -> f64 {
    let t = s.trim_start();
    let bytes = t.as_bytes();
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let mut saw_digit = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
        saw_digit = true;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            saw_digit = true;
        }
    }
    if !saw_digit {
        return 0.0;
    }
    // Optional exponent — only consumed if it actually has digits.
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
            j += 1;
        }
        let exp_start = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp_start {
            i = j;
        }
    }
    t[..i].parse::<f64>().unwrap_or(0.0)
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

            // NULL placement is handled explicitly so it can be controlled by a
            // `NULLS FIRST`/`NULLS LAST` clause independently of ASC/DESC. The
            // default (no clause) is SQLite's: NULLs first for ASC, last for
            // DESC — i.e. `nulls_first` defaults to `ascending`. Null placement
            // is absolute and is NOT flipped by the ascending/descending
            // reversal below (which only applies to non-NULL comparisons).
            let a_null = matches!(va, SqlValue::Null);
            let b_null = matches!(vb, SqlValue::Null);
            if a_null || b_null {
                if a_null && b_null {
                    continue; // equal on this key; fall through to the next
                }
                let nulls_first = key.nulls_first.unwrap_or(key.ascending);
                // The NULL operand sorts first iff `nulls_first`.
                return if a_null == nulls_first {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
            }

            let cmp = sql_cmp_collated(va, vb, key.collation.as_deref());
            if cmp != std::cmp::Ordering::Equal {
                return if key.ascending { cmp } else { cmp.reverse() };
            }
        }
        std::cmp::Ordering::Equal
    });
}

/// Compare two values honouring an optional `COLLATE` sequence.
///
/// Collation only affects **text-vs-text** comparisons; for every other type
/// pairing SQLite's normal type-ordering (`sql_cmp`) applies unchanged. The two
/// built-in text collations we transform:
///
/// | Collation | Transform before byte comparison        | Example equal pair |
/// |-----------|------------------------------------------|--------------------|
/// | `NOCASE`  | ASCII-lowercase both operands            | `'Apple'` = `'apple'` |
/// | `RTRIM`   | strip trailing spaces from both operands | `'a  '` = `'a'`       |
///
/// `None` or `BINARY` (folded to `None` in the planner) keeps raw byte order.
/// The transform is applied to *copies*; the underlying values are untouched, so
/// the sort is a pure reordering. NOCASE is ASCII-only, matching SQLite's
/// built-in NOCASE (it lowercases A–Z exclusively, leaving non-ASCII bytes as
/// is) — we mirror that with `to_ascii_lowercase` rather than Unicode folding.
fn sql_cmp_collated(
    a: &SqlValue,
    b: &SqlValue,
    collation: Option<&str>,
) -> std::cmp::Ordering {
    if let (Some(coll), SqlValue::Text(sa), SqlValue::Text(sb)) = (collation, a, b) {
        let ta = collate_text(sa, coll);
        let tb = collate_text(sb, coll);
        return ta.cmp(&tb);
    }
    sql_cmp(a, b)
}

/// Produce the collation-normalised form of a text value for comparison.
/// Unknown collation names fall through to the raw string (defensive — the
/// planner already rejects anything other than NOCASE/RTRIM/BINARY).
fn collate_text(s: &str, collation: &str) -> String {
    match collation {
        "NOCASE" => s.to_ascii_lowercase(),
        "RTRIM" => s.trim_end_matches(' ').to_string(),
        _ => s.to_string(),
    }
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
    fn test_arith_text_numeric_affinity() {
        // Binary arithmetic coerces text/blob operands via numeric affinity,
        // matching SQLite: `'5' + 0` = 5, `'abc' + 1` = 1, `'10' - '3'` = 7.
        let bin = |op: BinaryOp, l: SqlValue, r: SqlValue| eval_binary(&op, l, r).unwrap();
        let t = |s: &str| SqlValue::Text(s.to_string());
        assert_eq!(bin(BinaryOp::Add, t("5"), int(0)), int(5));
        assert_eq!(bin(BinaryOp::Add, t("5.5"), int(0)), SqlValue::Float(5.5));
        assert_eq!(bin(BinaryOp::Add, t("abc"), int(1)), int(1)); // no prefix → 0
        assert_eq!(bin(BinaryOp::Mul, t("5"), int(2)), int(10));
        assert_eq!(bin(BinaryOp::Sub, t("10"), t("3")), int(7));
        // Division/modulo coerce too; `5 / '0'` → NULL (affinity → integer 0).
        assert_eq!(bin(BinaryOp::Div, int(5), t("2")), int(2));
        assert_eq!(bin(BinaryOp::Div, int(5), t("0")), null());
        assert_eq!(bin(BinaryOp::Mod, t("7"), int(3)), int(1));
        // NULL still short-circuits to NULL (handled before coercion).
        assert_eq!(bin(BinaryOp::Add, t("5"), null()), null());
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
    fn test_div_and_mod_by_zero_return_null() {
        // SQLite yields NULL (not an error) for `x / 0` and `x % 0`, for both
        // integer and float zero divisors.
        let mut b = InMemoryBackend::new();
        for op in [BinaryOp::Div, BinaryOp::Mod] {
            for divisor in [int(0), SqlValue::Float(0.0)] {
                let r = execute(
                    &prog(vec![
                        Instruction::BeginRow,
                        Instruction::LoadConst(int(5)),
                        Instruction::LoadConst(divisor.clone()),
                        Instruction::BinaryOpInstr(op.clone()),
                        Instruction::EmitColumn("r".to_string()),
                        Instruction::EmitRow,
                        Instruction::Halt,
                    ]),
                    &mut b,
                )
                .unwrap();
                assert_eq!(
                    r.rows,
                    vec![vec![SqlValue::Null]],
                    "{op:?} by {divisor:?} should be NULL"
                );
            }
        }
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
    fn test_cast_numeric_affinity() {
        let num = |v: SqlValue| apply_cast(&v, &CastType::Numeric);

        // Text → INTEGER when integral & fits i64; else REAL.
        assert_eq!(num(SqlValue::Text("42".into())), SqlValue::Int(42));
        assert_eq!(num(SqlValue::Text("3.0".into())), SqlValue::Int(3));
        assert_eq!(num(SqlValue::Text("1e3".into())), SqlValue::Int(1000));
        assert_eq!(num(SqlValue::Text("42abc".into())), SqlValue::Int(42));
        assert_eq!(num(SqlValue::Text("abc".into())), SqlValue::Int(0));
        assert_eq!(num(SqlValue::Text("3.5".into())), SqlValue::Float(3.5));
        // i64-overflowing integer text → REAL.
        match num(SqlValue::Text("99999999999999999999".into())) {
            SqlValue::Float(f) => assert!((f - 1e20).abs() < 1e6),
            other => panic!("expected REAL, got {other:?}"),
        }
        // i64::MAX parses exactly as INTEGER (no f64-rounding surprise).
        assert_eq!(
            num(SqlValue::Text("9223372036854775807".into())),
            SqlValue::Int(i64::MAX)
        );
        // Numbers are a no-op: INTEGER stays INTEGER, REAL stays REAL (even 3.0).
        assert_eq!(num(SqlValue::Int(42)), SqlValue::Int(42));
        assert_eq!(num(SqlValue::Float(3.0)), SqlValue::Float(3.0));
        assert_eq!(num(SqlValue::Float(3.5)), SqlValue::Float(3.5));
        // NULL stays NULL.
        assert_eq!(num(SqlValue::Null), SqlValue::Null);
    }

    #[test]
    fn test_sqlite_substr_edge_cases() {
        let chars: Vec<char> = "hello".chars().collect();
        let s = |pos, z| sqlite_substr(&chars, pos, z);
        // Ordinary and negative start.
        assert_eq!(s(2, Some(3)), "ell");
        assert_eq!(s(-2, None), "lo");
        assert_eq!(s(-3, Some(2)), "ll");
        // Y = 0 (virtual slot before the first char).
        assert_eq!(s(0, None), "hello");
        assert_eq!(s(0, Some(3)), "he");
        assert_eq!(s(0, Some(1)), "");
        assert_eq!(s(0, Some(2)), "h");
        // Negative length reads the |Z| chars preceding Y.
        assert_eq!(s(2, Some(-1)), "h");
        assert_eq!(s(3, Some(-2)), "he");
        assert_eq!(s(1, Some(-1)), "");
        assert_eq!(s(5, Some(-2)), "ll");
        assert_eq!(s(-2, Some(-1)), "l");
        // Out-of-range windows clamp to empty / the string bounds.
        assert_eq!(s(6, Some(2)), "");
        assert_eq!(s(3, Some(0)), "");
        assert_eq!(s(3, Some(10)), "llo");
        assert_eq!(s(-10, None), "hello");
        // Character-based for multibyte text.
        let accented: Vec<char> = "héllo".chars().collect();
        assert_eq!(sqlite_substr(&accented, 2, Some(2)), "él");

        // Extreme i64 arguments must not overflow-panic (attacker-controlled —
        // these are the exact breakers the security review found). We only
        // require a safe, bounded result: saturating arithmetic keeps the window
        // in range, so the output is always some slice of the input. Bug-for-bug
        // parity with SQLite's C integer wrapping on these pathological inputs is
        // out of scope (no real query passes i64::MIN as an offset/length).
        for &pos in &[i64::MIN, i64::MAX, -6, -1, 0, 1] {
            for &z in &[Some(i64::MIN), Some(i64::MAX), Some(0), None] {
                let out = sqlite_substr(&chars, pos, z);
                assert!(
                    out.chars().count() <= chars.len(),
                    "substr({pos},{z:?}) escaped bounds: {out:?}"
                );
            }
        }
    }

    #[test]
    fn test_like_match_escape() {
        // Escaped `%`/`_` become literals; unescaped ones stay wildcards.
        assert!(like_match_escape("a%b", "a#%b", '#'));
        assert!(!like_match_escape("100x", "100#%", '#'));
        assert!(like_match_escape("a_b", "a#_b", '#'));
        assert!(!like_match_escape("axb", "a#_b", '#'));
        // Literal escape then a real wildcard.
        assert!(like_match_escape("50%off", "50#%%", '#'));
        // The escape character escaping itself.
        assert!(like_match_escape("a/b", "a//b", '/'));
        // Plain wildcards still work with an escape char defined.
        assert!(like_match_escape("anything", "%", '#'));
        // Case-insensitive literal match, like the wildcard-free path.
        assert!(like_match_escape("A%C", "a#%c", '#'));
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
    fn test_unary_neg_text_numeric_affinity() {
        // Unary minus coerces a text operand through numeric affinity, then
        // negates — matching SQLite.
        let neg = |s: &str| eval_unary(&UnaryOp::Neg, SqlValue::Text(s.to_string())).unwrap();
        assert_eq!(neg("5"), SqlValue::Int(-5));
        assert_eq!(neg("12abc"), SqlValue::Int(-12));
        assert_eq!(neg("abc"), SqlValue::Int(0)); // no numeric prefix → 0
        assert_eq!(neg("3.5"), SqlValue::Float(-3.5));
        assert_eq!(neg("  7"), SqlValue::Int(-7)); // leading whitespace tolerated
        // Blob operand coerces via its UTF-8 bytes; NULL stays NULL.
        assert_eq!(
            eval_unary(&UnaryOp::Neg, SqlValue::Blob(b"9".to_vec())).unwrap(),
            SqlValue::Int(-9)
        );
        assert_eq!(
            eval_unary(&UnaryOp::Neg, SqlValue::Null).unwrap(),
            SqlValue::Null
        );
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

    #[test]
    fn test_is_truthy_text_numeric_affinity() {
        // Text/blob truthiness takes numeric affinity, matching SQLite.
        assert!(!is_truthy(&SqlValue::Text("abc".into()))); // → 0 → false
        assert!(is_truthy(&SqlValue::Text("5".into()))); // → 5 → true
        assert!(!is_truthy(&SqlValue::Text("0".into())));
        assert!(!is_truthy(&SqlValue::Text("".into())));
        assert!(is_truthy(&SqlValue::Text("5.5".into())));
        assert!(is_truthy(&SqlValue::Text("12abc".into()))); // leading 12 → true
        assert!(!is_truthy(&SqlValue::Blob(b"abc".to_vec())));
        assert!(is_truthy(&SqlValue::Blob(b"9".to_vec())));
        // Numeric/NULL/bool arms unchanged.
        assert!(!is_truthy(&SqlValue::Int(0)));
        assert!(is_truthy(&SqlValue::Int(3)));
        assert!(!is_truthy(&SqlValue::Null));
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
            Instruction::Like(false),
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

    #[test]
    fn test_in_list_numeric_equality() {
        // `1 IN (1.0)` is TRUE — IN uses `=` equality, which compares INTEGER and
        // REAL numerically (not by same-variant identity).
        let run = |val: SqlValue, items: Vec<SqlValue>| {
            let mut b = InMemoryBackend::new();
            let n = items.len();
            let mut prog_v = vec![Instruction::BeginRow, Instruction::LoadConst(val)];
            for it in items {
                prog_v.push(Instruction::LoadConst(it));
            }
            prog_v.push(Instruction::InList(n));
            prog_v.push(Instruction::EmitColumn("r".to_string()));
            prog_v.push(Instruction::EmitRow);
            prog_v.push(Instruction::Halt);
            execute(&prog(prog_v), &mut b).unwrap().rows[0][0].clone()
        };
        assert_eq!(run(int(1), vec![float(1.0)]), bool_val(true));
        assert_eq!(run(float(1.0), vec![int(1)]), bool_val(true));
        assert_eq!(run(int(1), vec![int(2), float(1.0), int(3)]), bool_val(true));
        // Text vs integer do NOT match (no affinity in IN).
        assert_eq!(run(SqlValue::Text("1".into()), vec![int(1)]), bool_val(false));
    }

    #[test]
    fn test_in_list_null_three_valued() {
        let run = |val: SqlValue, items: Vec<SqlValue>| {
            let mut b = InMemoryBackend::new();
            let n = items.len();
            let mut prog_v = vec![Instruction::BeginRow, Instruction::LoadConst(val)];
            for it in items {
                prog_v.push(Instruction::LoadConst(it));
            }
            prog_v.push(Instruction::InList(n));
            prog_v.push(Instruction::EmitColumn("r".to_string()));
            prog_v.push(Instruction::EmitRow);
            prog_v.push(Instruction::Halt);
            execute(&prog(prog_v), &mut b).unwrap().rows[0][0].clone()
        };
        // No match but a NULL element present → NULL.
        assert_eq!(run(int(5), vec![null(), int(2)]), null());
        // A real match wins even with a NULL element present → true.
        assert_eq!(run(int(1), vec![null(), int(1)]), bool_val(true));
        // No match, no NULL element → false.
        assert_eq!(run(int(5), vec![int(1), int(2)]), bool_val(false));
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
            Instruction::SortResult(vec![CompiledSortKey { column: "x".to_string(), ascending: true, nulls_first: None, collation: None }]),
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
            Instruction::SortResult(vec![CompiledSortKey { column: "x".to_string(), ascending: false, nulls_first: None, collation: None }]),
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
                CompiledSortKey { column: "a".to_string(), ascending: true, nulls_first: None, collation: None },
                CompiledSortKey { column: "b".to_string(), ascending: true, nulls_first: None, collation: None },
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

    #[test]
    fn test_concat_blob_uses_raw_bytes() {
        // `||` concatenates a blob as its RAW bytes (as text), not the `x'…'`
        // display form: `X'41' || 'B'` = 'AB'. Result is TEXT; NULL propagates.
        let cat = |l: SqlValue, r: SqlValue| eval_binary(&BinaryOp::Concat, l, r).unwrap();
        assert_eq!(
            cat(SqlValue::Blob(vec![0x41]), SqlValue::Text("B".into())),
            SqlValue::Text("AB".into())
        );
        assert_eq!(
            cat(SqlValue::Text("A".into()), SqlValue::Blob(vec![0x42])),
            SqlValue::Text("AB".into())
        );
        assert_eq!(
            cat(SqlValue::Blob(vec![0x48]), SqlValue::Blob(vec![0x69])),
            SqlValue::Text("Hi".into())
        );
        // `sql_to_str` (the display form) still renders the hex literal — the
        // concat path must NOT regress that helper's behavior.
        assert_eq!(sql_to_str(&SqlValue::Blob(vec![0x41])), "x'41'");
        assert_eq!(
            cat(SqlValue::Blob(vec![0x41]), SqlValue::Null),
            SqlValue::Null
        );
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
            Instruction::SortResult(vec![CompiledSortKey { column: "x".to_string(), ascending: true, nulls_first: None, collation: None }]),
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
            Instruction::SortResult(vec![CompiledSortKey { column: "x".to_string(), ascending: true, nulls_first: None, collation: None }]),
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
        // NULL casts to an empty blob, so HEX(NULL) is the empty string, NOT NULL.
        assert_eq!(call_builtin("HEX", vec![SqlValue::Null]).unwrap(), SqlValue::Text(String::new()));
    }

    #[test]
    fn builtin_sign_and_unicode() {
        let sign = |v: SqlValue| call_builtin("SIGN", vec![v]).unwrap();
        assert_eq!(sign(SqlValue::Int(-7)), SqlValue::Int(-1));
        assert_eq!(sign(SqlValue::Int(0)), SqlValue::Int(0));
        assert_eq!(sign(SqlValue::Float(3.5)), SqlValue::Int(1));
        assert_eq!(sign(SqlValue::Text("x".into())), SqlValue::Null); // non-numeric
        assert_eq!(sign(SqlValue::Null), SqlValue::Null);

        let uni = |s: &str| call_builtin("UNICODE", vec![SqlValue::Text(s.into())]).unwrap();
        assert_eq!(uni("abc"), SqlValue::Int(97));
        assert_eq!(uni("Z"), SqlValue::Int(90));
        assert_eq!(uni(""), SqlValue::Null); // empty → NULL
        assert_eq!(call_builtin("UNICODE", vec![SqlValue::Null]).unwrap(), SqlValue::Null);
    }

    #[test]
    fn builtin_char_and_zeroblob() {
        assert_eq!(
            call_builtin("CHAR", vec![SqlValue::Int(72), SqlValue::Int(105), SqlValue::Int(33)]).unwrap(),
            SqlValue::Text("Hi!".into())
        );
        // No args → empty string.
        assert_eq!(call_builtin("CHAR", vec![]).unwrap(), SqlValue::Text(String::new()));

        assert_eq!(
            call_builtin("ZEROBLOB", vec![SqlValue::Int(3)]).unwrap(),
            SqlValue::Blob(vec![0, 0, 0])
        );
        assert_eq!(
            call_builtin("ZEROBLOB", vec![SqlValue::Int(-1)]).unwrap(),
            SqlValue::Blob(vec![]) // negative length → empty
        );
        assert_eq!(call_builtin("ZEROBLOB", vec![SqlValue::Null]).unwrap(), SqlValue::Null);
        // Adversarial: a huge length is rejected, not eagerly allocated (DoS guard).
        assert!(matches!(
            call_builtin("ZEROBLOB", vec![SqlValue::Int(9_999_999_999)]),
            Err(VmError::ResourceLimit(_))
        ));
    }

    #[test]
    fn builtin_quote_renders_sql_literals() {
        let q = |v: SqlValue| match call_builtin("QUOTE", vec![v]).unwrap() {
            SqlValue::Text(s) => s,
            other => panic!("expected text, got {other:?}"),
        };
        assert_eq!(q(SqlValue::Null), "NULL");
        assert_eq!(q(SqlValue::Int(-7)), "-7");
        assert_eq!(q(SqlValue::Text("it's".into())), "'it''s'"); // inner quote doubled
        assert_eq!(q(SqlValue::Blob(vec![0xde, 0xad])), "X'DEAD'");
    }

    #[test]
    fn builtin_scalar_max_min() {
        let mx = |vs: Vec<SqlValue>| call_builtin("MAX", vs).unwrap();
        let mn = |vs: Vec<SqlValue>| call_builtin("MIN", vs).unwrap();
        assert_eq!(mx(vec![SqlValue::Int(3), SqlValue::Int(9), SqlValue::Int(5)]), SqlValue::Int(9));
        assert_eq!(mn(vec![SqlValue::Int(3), SqlValue::Int(9), SqlValue::Int(5)]), SqlValue::Int(3));
        // Any NULL argument → NULL.
        assert_eq!(mx(vec![SqlValue::Int(1), SqlValue::Null, SqlValue::Int(3)]), SqlValue::Null);
        // Text ordering.
        assert_eq!(
            mn(vec![SqlValue::Text("b".into()), SqlValue::Text("a".into()), SqlValue::Text("c".into())]),
            SqlValue::Text("a".into())
        );
    }

    #[test]
    fn builtin_iif_selects_by_truthiness() {
        let iif = |x: SqlValue| {
            call_builtin("IIF", vec![x, SqlValue::Text("yes".into()), SqlValue::Text("no".into())]).unwrap()
        };
        assert_eq!(iif(SqlValue::Int(1)), SqlValue::Text("yes".into()));
        assert_eq!(iif(SqlValue::Int(0)), SqlValue::Text("no".into()));
        assert_eq!(iif(SqlValue::Null), SqlValue::Text("no".into())); // NULL → falsy
        assert_eq!(iif(SqlValue::Bool(true)), SqlValue::Text("yes".into()));
        // Wrong arity is a type error, not a panic.
        assert!(call_builtin("IIF", vec![SqlValue::Int(1), SqlValue::Int(2)]).is_err());
    }

    #[test]
    fn builtin_trim_one_arg_strips_whitespace() {
        // The single-argument form keeps its historical whitespace behaviour.
        let t = |name: &str, s: &str| call_builtin(name, vec![SqlValue::Text(s.into())]).unwrap();
        assert_eq!(t("TRIM", "  hi  "), SqlValue::Text("hi".into()));
        assert_eq!(t("LTRIM", "  hi  "), SqlValue::Text("hi  ".into()));
        assert_eq!(t("RTRIM", "  hi  "), SqlValue::Text("  hi".into()));
        // NULL propagates; a NULL string trims to NULL.
        assert_eq!(call_builtin("TRIM", vec![SqlValue::Null]).unwrap(), SqlValue::Null);
    }

    #[test]
    fn builtin_trim_two_arg_strips_character_set() {
        let t = |name: &str, s: &str, set: &str| {
            call_builtin(name, vec![SqlValue::Text(s.into()), SqlValue::Text(set.into())]).unwrap()
        };
        // The second argument is a *set* of characters, matched at each end.
        assert_eq!(t("TRIM", "xxhixx", "x"), SqlValue::Text("hi".into()));
        assert_eq!(t("TRIM", "abcHIcba", "abc"), SqlValue::Text("HI".into()));
        assert_eq!(t("LTRIM", "xyxhi", "xy"), SqlValue::Text("hi".into()));
        assert_eq!(t("RTRIM", "hixyx", "xy"), SqlValue::Text("hi".into()));
        // Operates on Unicode characters, not bytes.
        assert_eq!(t("TRIM", "héllo", "h"), SqlValue::Text("éllo".into()));
        assert_eq!(t("TRIM", " oé oé", "é "), SqlValue::Text("oé o".into()));
        // Stripping everything yields the empty string.
        assert_eq!(t("TRIM", "aaa", "a"), SqlValue::Text("".into()));
        // An empty trim-set removes nothing.
        assert_eq!(t("TRIM", "xhix", ""), SqlValue::Text("xhix".into()));
    }

    #[test]
    fn builtin_trim_two_arg_null_and_coercion() {
        // NULL in either argument propagates.
        assert_eq!(
            call_builtin("TRIM", vec![SqlValue::Text("xxhixx".into()), SqlValue::Null]).unwrap(),
            SqlValue::Null
        );
        assert_eq!(
            call_builtin("TRIM", vec![SqlValue::Null, SqlValue::Text("x".into())]).unwrap(),
            SqlValue::Null
        );
        // Numeric arguments coerce to their decimal text, like real SQLite:
        //   trim(12321, '1') -> '232',  trim('5xx', 5) -> 'xx'
        assert_eq!(
            call_builtin("TRIM", vec![SqlValue::Int(12321), SqlValue::Text("1".into())]).unwrap(),
            SqlValue::Text("232".into())
        );
        assert_eq!(
            call_builtin("TRIM", vec![SqlValue::Text("5xx".into()), SqlValue::Int(5)]).unwrap(),
            SqlValue::Text("xx".into())
        );
        // Three arguments is an arity error, not a panic.
        assert!(call_builtin(
            "TRIM",
            vec![SqlValue::Text("a".into()), SqlValue::Text("b".into()), SqlValue::Text("c".into())]
        )
        .is_err());
    }

    #[test]
    fn builtin_trim_zero_args_is_error_not_panic() {
        // The grammar lets `TRIM()` parse (the argument list is optional), so an
        // empty `args` must be a clean error — never an out-of-bounds panic.
        for name in ["TRIM", "LTRIM", "RTRIM"] {
            assert!(call_builtin(name, vec![]).is_err(), "{name}() should error, not panic");
        }
    }

    #[test]
    fn builtin_concat_joins_all_arguments() {
        let c = |vs: Vec<SqlValue>| call_builtin("CONCAT", vs).unwrap();
        assert_eq!(
            c(vec![SqlValue::Text("a".into()), SqlValue::Text("b".into()), SqlValue::Text("c".into())]),
            SqlValue::Text("abc".into())
        );
        // A NULL argument contributes the empty string (does not nullify).
        assert_eq!(
            c(vec![SqlValue::Text("a".into()), SqlValue::Null, SqlValue::Text("c".into())]),
            SqlValue::Text("ac".into())
        );
        // Integers/booleans coerce to their decimal text.
        assert_eq!(
            c(vec![SqlValue::Int(12), SqlValue::Text("x".into())]),
            SqlValue::Text("12x".into())
        );
        // All-NULL concatenation is the empty string, not NULL.
        assert_eq!(c(vec![SqlValue::Null]), SqlValue::Text("".into()));
        // Zero arguments is an arity error.
        assert!(call_builtin("CONCAT", vec![]).is_err());
        // Floats are declined (their SQLite text form is subtle), like HEX/QUOTE.
        assert!(call_builtin("CONCAT", vec![SqlValue::Float(2.5)]).is_err());
    }

    #[test]
    fn builtin_concat_ws_joins_with_separator() {
        let c = |vs: Vec<SqlValue>| call_builtin("CONCAT_WS", vs).unwrap();
        assert_eq!(
            c(vec![
                SqlValue::Text("-".into()),
                SqlValue::Text("a".into()),
                SqlValue::Text("b".into()),
                SqlValue::Text("c".into()),
            ]),
            SqlValue::Text("a-b-c".into())
        );
        // NULL value arguments are SKIPPED entirely (not joined as empty).
        assert_eq!(
            c(vec![
                SqlValue::Text("-".into()),
                SqlValue::Text("a".into()),
                SqlValue::Null,
                SqlValue::Text("c".into()),
            ]),
            SqlValue::Text("a-c".into())
        );
        // All-NULL values → empty string (separator never appears).
        assert_eq!(
            c(vec![SqlValue::Text("-".into()), SqlValue::Null, SqlValue::Null]),
            SqlValue::Text("".into())
        );
        // A NULL separator makes the whole result NULL.
        assert_eq!(
            c(vec![SqlValue::Null, SqlValue::Text("a".into()), SqlValue::Text("b".into())]),
            SqlValue::Null
        );
        // Fewer than two arguments is an arity error.
        assert!(call_builtin("CONCAT_WS", vec![SqlValue::Text("-".into())]).is_err());
    }

    #[test]
    fn builtin_substring_is_an_alias_of_substr() {
        // SUBSTRING must behave identically to SUBSTR for every arity.
        for args in [
            vec![SqlValue::Text("hello".into()), SqlValue::Int(2)],
            vec![SqlValue::Text("hello".into()), SqlValue::Int(2), SqlValue::Int(3)],
            vec![SqlValue::Text("hello".into()), SqlValue::Int(-2), SqlValue::Int(1)],
        ] {
            assert_eq!(
                call_builtin("SUBSTRING", args.clone()).unwrap(),
                call_builtin("SUBSTR", args).unwrap(),
            );
        }
    }

    #[test]
    fn builtin_printf_formats_integers_and_strings() {
        let txt = |s: &str| SqlValue::Text(s.into());
        let pf = |fmt: &str, extra: Vec<SqlValue>| {
            let mut a = vec![txt(fmt)];
            a.extend(extra);
            call_builtin("PRINTF", a).unwrap()
        };
        assert_eq!(pf("%d-%s", vec![SqlValue::Int(5), txt("x")]), txt("5-x"));
        // Width, left-justify, zero-pad, sign.
        assert_eq!(pf("%5d", vec![SqlValue::Int(42)]), txt("   42"));
        assert_eq!(pf("%-5d|", vec![SqlValue::Int(42)]), txt("42   |"));
        assert_eq!(pf("%05d", vec![SqlValue::Int(42)]), txt("00042"));
        assert_eq!(pf("%+d", vec![SqlValue::Int(5)]), txt("+5"));
        assert_eq!(pf("%05d", vec![SqlValue::Int(-7)]), txt("-0007")); // zero-pad after sign
        // Hex / octal / precision / literal percent.
        assert_eq!(pf("%x", vec![SqlValue::Int(255)]), txt("ff"));
        assert_eq!(pf("%X", vec![SqlValue::Int(255)]), txt("FF"));
        assert_eq!(pf("%o", vec![SqlValue::Int(8)]), txt("10"));
        assert_eq!(pf("%.3s", vec![txt("hello")]), txt("hel"));
        assert_eq!(pf("100%%", vec![]), txt("100%"));
        // Coercion + missing/extra args (SQLite's defaults).
        assert_eq!(pf("%d", vec![txt("abc")]), txt("0")); // non-numeric text → 0
        assert_eq!(pf("%d", vec![SqlValue::Null]), txt("0")); // NULL → 0
        assert_eq!(pf("%s", vec![SqlValue::Null]), txt("")); // NULL → ""
        assert_eq!(pf("%d %d", vec![SqlValue::Int(1)]), txt("1 0")); // missing → 0
        assert_eq!(pf("%d", vec![SqlValue::Int(1), SqlValue::Int(2)]), txt("1")); // extra ignored
        // %q doubles single quotes (SQL-literal building).
        assert_eq!(pf("%q", vec![txt("a'b")]), txt("a''b"));
        // FORMAT is an alias.
        assert_eq!(call_builtin("FORMAT", vec![txt("%d"), SqlValue::Int(7)]).unwrap(), txt("7"));
        // A NULL format → NULL; a float conversion is declined; no format errors.
        assert_eq!(call_builtin("PRINTF", vec![SqlValue::Null]).unwrap(), SqlValue::Null);
        assert!(call_builtin("PRINTF", vec![txt("%f"), SqlValue::Float(1.5)]).is_err());
        assert!(call_builtin("PRINTF", vec![]).is_err());
        // A hostile field width is rejected, not allocated (DoS guard).
        assert!(call_builtin("PRINTF", vec![txt("%9999999999d"), SqlValue::Int(1)]).is_err());
    }

    #[test]
    fn builtin_glob_matches_case_sensitively() {
        let g = |pat: &str, subj: &str| {
            call_builtin("GLOB", vec![SqlValue::Text(pat.into()), SqlValue::Text(subj.into())]).unwrap()
        };
        let t = SqlValue::Int(1);
        let f = SqlValue::Int(0);
        // `*` and `?` wildcards; GLOB is case-sensitive.
        assert_eq!(g("a*", "abc"), t);
        assert_eq!(g("A*", "abc"), f); // case-sensitive
        assert_eq!(g("*c", "abc"), t);
        assert_eq!(g("a?c", "abc"), t);
        assert_eq!(g("a?c", "ac"), f);
        assert_eq!(g("h*o", "hello"), t);
        // Character classes, ranges, and negation.
        assert_eq!(g("[a-c]x", "bx"), t);
        assert_eq!(g("[a-c]x", "dx"), f);
        assert_eq!(g("[^a]", "b"), t);
        assert_eq!(g("[0-9]*", "7up"), t);
        // Empty pattern / subject; `*` matches empty.
        assert_eq!(g("", ""), t);
        assert_eq!(g("*", ""), t);
        // Backslash is a LITERAL in GLOB (no escape).
        assert_eq!(g("a\\*b", "a*b"), f);
        // Unicode is matched by character.
        assert_eq!(g("日*", "日本"), t);
        // NULL in either argument → NULL; wrong arity errors, not panics.
        assert_eq!(
            call_builtin("GLOB", vec![SqlValue::Text("a*".into()), SqlValue::Null]).unwrap(),
            SqlValue::Null
        );
        assert!(call_builtin("GLOB", vec![SqlValue::Text("a".into())]).is_err());
    }

    #[test]
    fn builtin_likely_family_is_identity() {
        // likely / unlikely return their single argument unchanged, any type.
        for name in ["LIKELY", "UNLIKELY"] {
            assert_eq!(call_builtin(name, vec![SqlValue::Int(5)]).unwrap(), SqlValue::Int(5));
            assert_eq!(
                call_builtin(name, vec![SqlValue::Text("abc".into())]).unwrap(),
                SqlValue::Text("abc".into())
            );
            assert_eq!(call_builtin(name, vec![SqlValue::Null]).unwrap(), SqlValue::Null);
            assert_eq!(call_builtin(name, vec![SqlValue::Float(2.5)]).unwrap(), SqlValue::Float(2.5));
            // Wrong arity is an error, not a panic.
            assert!(call_builtin(name, vec![]).is_err());
            assert!(call_builtin(name, vec![SqlValue::Int(1), SqlValue::Int(2)]).is_err());
        }
        // likelihood(x, p) returns x when p is a probability in [0, 1].
        assert_eq!(
            call_builtin("LIKELIHOOD", vec![SqlValue::Int(7), SqlValue::Float(0.0625)]).unwrap(),
            SqlValue::Int(7)
        );
        assert_eq!(
            call_builtin("LIKELIHOOD", vec![SqlValue::Null, SqlValue::Float(0.5)]).unwrap(),
            SqlValue::Null
        );
        // A probability outside [0, 1], a non-numeric probability, or wrong arity
        // are all errors.
        assert!(call_builtin("LIKELIHOOD", vec![SqlValue::Int(1), SqlValue::Float(1.5)]).is_err());
        assert!(call_builtin("LIKELIHOOD", vec![SqlValue::Int(1), SqlValue::Text("x".into())]).is_err());
        assert!(call_builtin("LIKELIHOOD", vec![SqlValue::Int(1)]).is_err());
    }

    #[test]
    fn builtin_length_blob_and_number() {
        let len = |v: SqlValue| call_builtin("LENGTH", vec![v]).unwrap();
        // Text → character count; a blob → raw byte count (contrast the text
        // char count); a number → its decimal-text length. NULL propagates.
        assert_eq!(len(SqlValue::Text("héllo".into())), SqlValue::Int(5)); // 5 chars
        assert_eq!(len(SqlValue::Blob(vec![0x01, 0x02, 0xff])), SqlValue::Int(3));
        assert_eq!(len(SqlValue::Blob(vec![])), SqlValue::Int(0));
        assert_eq!(len(SqlValue::Int(12345)), SqlValue::Int(5));
        assert_eq!(len(SqlValue::Int(-7)), SqlValue::Int(2));
        assert_eq!(len(SqlValue::Bool(true)), SqlValue::Int(1));
        assert_eq!(len(SqlValue::Null), SqlValue::Null);
        // Floats are declined (text-form length is subtle); wrong arity errors.
        assert!(call_builtin("LENGTH", vec![SqlValue::Float(3.14)]).is_err());
        assert!(call_builtin("LENGTH", vec![]).is_err());
    }

    #[test]
    fn builtin_octet_length_counts_bytes() {
        let ol = |v: SqlValue| call_builtin("OCTET_LENGTH", vec![v]).unwrap();
        // Text is measured in UTF-8 bytes, not characters (contrast LENGTH).
        assert_eq!(ol(SqlValue::Text("héllo".into())), SqlValue::Int(6)); // 5 chars, 6 bytes
        assert_eq!(
            call_builtin("LENGTH", vec![SqlValue::Text("héllo".into())]).unwrap(),
            SqlValue::Int(5)
        );
        assert_eq!(ol(SqlValue::Text("abc".into())), SqlValue::Int(3));
        assert_eq!(ol(SqlValue::Text("".into())), SqlValue::Int(0));
        assert_eq!(ol(SqlValue::Text("日本".into())), SqlValue::Int(6)); // 2 chars × 3 bytes
        // Blobs measure raw bytes; integers their decimal digits.
        assert_eq!(ol(SqlValue::Blob(vec![0x00, 0xff])), SqlValue::Int(2));
        assert_eq!(ol(SqlValue::Int(123)), SqlValue::Int(3));
        // NULL propagates; wrong arity errors, not panics.
        assert_eq!(ol(SqlValue::Null), SqlValue::Null);
        assert!(call_builtin("OCTET_LENGTH", vec![]).is_err());
    }

    #[test]
    fn builtin_unhex_decodes_hex_pairs() {
        let u1 = |s: &str| call_builtin("UNHEX", vec![SqlValue::Text(s.into())]).unwrap();
        // Even-length hex → blob; case-insensitive.
        assert_eq!(u1("414243"), SqlValue::Blob(vec![0x41, 0x42, 0x43]));
        assert_eq!(u1("abcdef"), SqlValue::Blob(vec![0xab, 0xcd, 0xef]));
        assert_eq!(u1("ABCDEF"), SqlValue::Blob(vec![0xab, 0xcd, 0xef]));
        assert_eq!(u1(""), SqlValue::Blob(vec![])); // empty → empty blob
        // Odd length or a non-hex character → NULL.
        assert_eq!(u1("abc"), SqlValue::Null);
        assert_eq!(u1("4g"), SqlValue::Null);
        assert_eq!(u1("41 42"), SqlValue::Null);
        // NULL propagates; an integer coerces to its decimal digits.
        assert_eq!(call_builtin("UNHEX", vec![SqlValue::Null]).unwrap(), SqlValue::Null);
        assert_eq!(
            call_builtin("UNHEX", vec![SqlValue::Int(12)]).unwrap(),
            SqlValue::Blob(vec![0x12])
        );
        // Result is a blob.
        assert!(matches!(u1("41"), SqlValue::Blob(_)));
    }

    #[test]
    fn builtin_unhex_ignore_set_only_at_byte_boundaries() {
        let u2 = |s: &str, ig: &str| {
            call_builtin("UNHEX", vec![SqlValue::Text(s.into()), SqlValue::Text(ig.into())]).unwrap()
        };
        // An ignorable char between pairs is fine.
        assert_eq!(u2("41.42", "."), SqlValue::Blob(vec![0x41, 0x42]));
        assert_eq!(u2("41", "x"), SqlValue::Blob(vec![0x41])); // ignore char absent
        // An ignorable char that splits a pair invalidates the string.
        assert_eq!(u2("4-1-4-2", "-"), SqlValue::Null);
        // A NULL ignore set yields NULL.
        assert_eq!(
            call_builtin("UNHEX", vec![SqlValue::Text("41".into()), SqlValue::Null]).unwrap(),
            SqlValue::Null
        );
        // Zero args is an arity error, not a panic.
        assert!(call_builtin("UNHEX", vec![]).is_err());
    }

    #[test]
    fn builtin_round_clamps_negative_digits_to_zero() {
        let round = |x: f64, d: Option<i64>| {
            let mut args = vec![SqlValue::Float(x)];
            if let Some(d) = d {
                args.push(SqlValue::Int(d));
            }
            call_builtin("ROUND", args).unwrap()
        };
        // Positive / zero digit counts are unchanged.
        assert_eq!(round(2.567, None), SqlValue::Float(3.0));
        assert_eq!(round(2.567, Some(0)), SqlValue::Float(3.0));
        assert_eq!(round(2.567, Some(2)), SqlValue::Float(2.57));
        // Round half away from zero.
        assert_eq!(round(2.5, None), SqlValue::Float(3.0));
        assert_eq!(round(-2.5, None), SqlValue::Float(-3.0));
        // A NEGATIVE digit count behaves as 0 — NOT tens/hundreds rounding.
        assert_eq!(round(2.567, Some(-1)), SqlValue::Float(3.0));
        assert_eq!(round(2.567, Some(-5)), SqlValue::Float(3.0));
        assert_eq!(round(12.5, Some(-1)), SqlValue::Float(13.0));
        // NULL propagation on either argument.
        assert_eq!(call_builtin("ROUND", vec![SqlValue::Null]).unwrap(), SqlValue::Null);
        assert_eq!(
            call_builtin("ROUND", vec![SqlValue::Float(2.5), SqlValue::Null]).unwrap(),
            SqlValue::Null
        );
    }
}

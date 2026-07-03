package com.codingadventures.sqlvm

// SqlVm.kt — Kotlin stack-machine bytecode VM for the mini-sqlite Level 1 pipeline.
//
// This is the *execution engine* of the pipeline:
//
//   SQL text
//     │  sql-lexer / sql-parser
//     ▼
//   AST
//     │  sql-planner
//     ▼
//   LogicalPlan
//     │  sql-optimizer
//     ▼
//   OptimizedPlan
//     │  sql-codegen
//     ▼
//   Program (flat instruction list)   ← we receive this
//     │  sql-vm  (THIS FILE)
//     ▼
//   QueryResult
//
// Architecture — the dispatch loop
// ─────────────────────────────────
// The VM is a simple while-loop over a flat array of instructions, exactly like
// CPython's ceval.c or SQLite's VDBE.  No tree recursion — every "complex"
// operation is decomposed into a sequence of simple stack steps by the codegen.
//
//   while (pc < instructions.size) {
//       dispatch(instructions[pc])
//       pc++
//   }
//
// The stack holds SqlValue instances (the sealed class from sql-codegen).  Most
// instructions pop one or two values, do something, and push a result.
//
// Value representation
// ────────────────────
// Inside the VM we use the `SqlValue` sealed class from sql-codegen as the
// single currency for all values:
//
//   SqlValue.Null       — SQL NULL
//   SqlValue.IntVal     — 64-bit integer
//   SqlValue.FloatVal   — IEEE-754 double
//   SqlValue.TextVal    — Unicode string
//   SqlValue.BoolVal    — boolean
//
// Key lesson from lessons.md:
//   • layout.buildDirectory = file("gradle-build") — mandatory to avoid
//     case-insensitive collision with the BUILD script on macOS/Windows.
//   • Kotlin when-guard patterns ("is Type when cond") are NOT supported;
//     use nested if inside an "is Type ->" branch.
//   • ArrayDeque is the idiomatic Kotlin stack (addLast / removeLast).
//   • Test method names in backticks must NOT contain "--", ":", etc.

import com.codingadventures.sqlbackend.Backend
import com.codingadventures.sqlbackend.ColumnDef
import com.codingadventures.sqlbackend.Cursor
import com.codingadventures.sqlbackend.CursorBackend
import com.codingadventures.sqlplanner.ColumnDef as PlannerColumnDef
import com.codingadventures.sqlplanner.SqlExpr as PlannerSqlExpr
import com.codingadventures.sqlbackend.IndexDef
import com.codingadventures.sqlbackend.Row
import com.codingadventures.sqlbackend.RowIterator
import com.codingadventures.sqlbackend.TriggerDef
import com.codingadventures.sqlcodegen.AggFn
import com.codingadventures.sqlcodegen.BinaryOp
import com.codingadventures.sqlcodegen.Instruction
import com.codingadventures.sqlcodegen.Program
import com.codingadventures.sqlcodegen.SqlValue
import com.codingadventures.sqlcodegen.UnaryOp

// ── QueryResult ───────────────────────────────────────────────────────────────
//
// The final product of executing a Program.  For SELECT queries, `columns` and
// `rows` carry the result set.  For DML (INSERT / UPDATE / DELETE), `rows` is
// empty and `rowsAffected` is the number of rows changed.

/**
 * The result of executing a [Program].
 *
 * For SELECT queries:
 *   - [columns] — ordered list of output column names (matches row width)
 *   - [rows]    — each row is a positional list of [SqlValue]s; order matches [columns]
 *   - [rowsAffected] — 0 (SELECTs don't mutate data)
 *
 * For DML (INSERT / UPDATE / DELETE):
 *   - [columns] — empty
 *   - [rows]    — empty
 *   - [rowsAffected] — count of rows inserted/updated/deleted
 *
 * For DDL (CREATE TABLE / DROP TABLE):
 *   - All fields are empty/zero (DDL produces no rows)
 */
data class QueryResult(
    val columns: List<String>,
    val rows: List<List<SqlValue>>,
    val rowsAffected: Int,
)

// ── Aggregate state ───────────────────────────────────────────────────────────
//
// SQL aggregates (COUNT, SUM, AVG, MIN, MAX) must track running state per
// group.  We maintain a flat array of AggAccumulator objects indexed by slot
// number.  Within a GROUP BY query there are multiple groups; each group maps
// to its own array of accumulators.

/**
 * Mutable accumulator for a single aggregate slot within a single group.
 *
 * For AVG, both [sum] and [count] are used.  For SUM/MIN/MAX, only [sum] is
 * used (max starts at null, updated via comparison).  For COUNT/COUNT_STAR,
 * only [count] is used.
 */
private data class AggAccumulator(
    val fn: AggFn,
    var count: Long = 0L,
    var sum: SqlValue = SqlValue.Null,     // also serves as min/max accumulator
)

// ── In-memory subquery cursor ─────────────────────────────────────────────────
//
// When the codegen produces a Union (two sequential scan loops writing to the
// same result buffer) or other multi-source scans, intermediate results are
// sometimes materialised.  For Level 1 we don't implement full subqueries, but
// we DO need a general RowIterator over a pre-collected list of rows.

/**
 * A simple [RowIterator] backed by a pre-materialised list of [Row] objects.
 *
 * Used internally by the VM when it needs to iterate a snapshot.
 */
private class ListCursorInternal(private val rows: List<Row>) : RowIterator {
    private var idx = -1
    override fun next(): Row? {
        idx += 1
        return rows.getOrNull(idx)
    }
    override fun close() {}
}

// ── VM state ──────────────────────────────────────────────────────────────────
//
// All mutable execution state for one execute() call is bundled here so the
// public API (SqlVm.execute) is side-effect-free from the caller's perspective.

/**
 * Encapsulates all mutable state for a single [SqlVm.execute] invocation.
 *
 * Keeping state in a dedicated object (rather than as loose local variables)
 * makes helper functions easy to write: every helper receives `state` and can
 * read/write any field without threading every variable through the call chain.
 */
private class VmState(
    val instructions: List<Instruction>,
    val backend: Backend,
    val labelIndex: Map<String, Int>,
) {
    // Program counter — index into `instructions`.
    var pc: Int = 0

    // The operand stack.  We use ArrayDeque as a stack: addLast to push,
    // removeLast to pop.  (Kotlin's ArrayDeque is a resizable array; O(1)
    // at both ends, no boxing overhead for object references.)
    val stack: ArrayDeque<SqlValue> = ArrayDeque()

    // Open cursors, keyed by alias (null = default/anonymous cursor).
    // Each cursor is the RowIterator returned by backend.scan() or openCursor().
    val cursors: MutableMap<String?, RowIterator> = mutableMapOf()

    // Current row for each open cursor, keyed by alias.
    // Updated on every AdvanceCursor; cleared on CloseScan.
    val currentRow: MutableMap<String?, Row> = mutableMapOf()

    // Output row buffer — columns are accumulated here between BeginRow and EmitRow.
    val rowBuffer: MutableList<Pair<String, SqlValue>> = mutableListOf()

    // Final result buffer — rows committed by EmitRow.
    val resultRows: MutableList<List<Pair<String, SqlValue>>> = mutableListOf()

    // Number of rows affected by DML (INSERT / UPDATE / DELETE).
    var rowsAffected: Int = 0

    // ── Aggregate state ────────────────────────────────────────────────────────
    //
    // For GROUP BY queries the VM tracks a separate accumulator array per group.
    // The group key is the tuple of GROUP BY column values, serialised as a list.
    //
    // For non-GROUP BY aggregates, we use an empty-list key (the single implicit
    // group that covers all rows).

    // Current group key (values pushed by SaveGroupKey).
    var groupKey: List<SqlValue> = emptyList()

    // Accumulator table: group key → slot array.
    val aggTable: MutableMap<List<SqlValue>, MutableList<AggAccumulator>> = linkedMapOf()

    // Group ordering (insertion order — so AdvanceGroup is deterministic).
    val groupOrder: MutableList<List<SqlValue>> = mutableListOf()

    // Iterator position for the finalize phase (AdvanceGroup increments this).
    var groupIter: Int = -1

    // ── Transaction state ──────────────────────────────────────────────────────
    var transactionHandle: com.codingadventures.sqlbackend.TransactionHandle? = null

    // ── Stack helpers ──────────────────────────────────────────────────────────

    /** Push a value onto the operand stack. */
    fun push(v: SqlValue) { stack.addLast(v) }

    /**
     * Pop and return the top of the operand stack.
     *
     * Throws [IllegalStateException] if the stack is empty — this indicates
     * a codegen bug, not a user error.
     */
    fun pop(): SqlValue {
        check(stack.isNotEmpty()) { "operand stack underflow" }
        return stack.removeLast()
    }

    /**
     * Pop [n] values and return them in push order (oldest first).
     *
     * Example: if the stack top-to-bottom is [C, B, A] and we popN(3), we
     * return [A, B, C] — the order in which they were pushed.
     *
     * This is important for IN lists and multi-argument aggregate functions
     * where argument order matters.
     */
    fun popN(n: Int): List<SqlValue> {
        if (n == 0) return emptyList()
        check(stack.size >= n) { "operand stack underflow: need $n, have ${stack.size}" }
        val result: MutableList<SqlValue> = MutableList(n) { SqlValue.Null }
        for (i in n - 1 downTo 0) result[i] = stack.removeLast()
        return result
    }
}

// ── SqlVm ─────────────────────────────────────────────────────────────────────
//
// The public entry point.  `execute` is a standalone function that creates a
// fresh VmState, runs the dispatch loop, and returns the QueryResult.

/**
 * Stack-machine VM that executes [Program] bytecode against a [Backend].
 *
 * The VM is a simple dispatch loop — it reads each [Instruction] in sequence,
 * executes it (modifying the [VmState]), and advances the program counter.
 * Control-flow instructions (Jump, JumpIfFalse, JumpIfTrue) rewrite the
 * program counter directly.
 *
 * # Thread safety
 * [execute] creates a fresh [VmState] on each call and does not share mutable
 * state between invocations.  Concurrent calls are safe as long as the
 * [Backend] implementation is itself thread-safe.
 */
object SqlVm {

    /**
     * Execute [program] against [backend] and return the query result.
     *
     * For SELECT queries, the returned [QueryResult] has a non-empty [columns]
     * list and zero or more [rows].  For DML, [rowsAffected] is non-zero and
     * [rows] is empty.  For DDL, all fields are zero/empty.
     */
    fun execute(program: Program, backend: Backend): QueryResult {
        // ── Label resolution ──────────────────────────────────────────────────
        //
        // Before execution, we build a Map<labelName, instructionIndex> by
        // scanning the instruction list once.  This makes all jump targets O(1)
        // to resolve during execution instead of O(n) linear scan per jump.
        val labelIndex = buildLabelIndex(program.instructions)

        val state = VmState(
            instructions = program.instructions,
            backend      = backend,
            labelIndex   = labelIndex,
        )

        // ── Main dispatch loop ────────────────────────────────────────────────
        //
        // We increment `pc` at the top of the loop so that jump instructions can
        // set `pc` to the target index and the loop will execute that instruction
        // next.  Note: Halt breaks the loop, so it does not need to set pc.
        while (state.pc < state.instructions.size) {
            val instr = state.instructions[state.pc]
            state.pc++

            when (instr) {
                // ─ Stack / constants ─────────────────────────────────────────
                is Instruction.LoadConst -> state.push(instr.value)

                is Instruction.LoadColumn -> doLoadColumn(instr, state)

                is Instruction.LoadParam -> {
                    // Reserved for parameterised queries; Level 1 inlines all
                    // literals, so this instruction should not appear in practice.
                    // Push NULL as a safe placeholder.
                    state.push(SqlValue.Null)
                }

                is Instruction.LoadGroupKey -> {
                    // Push the i-th component of the current group key.
                    // During the finalize phase, groupKey holds the values for
                    // the group currently being emitted.
                    val v = state.groupKey.getOrNull(instr.index) ?: SqlValue.Null
                    state.push(v)
                }

                is Instruction.Pop -> state.pop()

                // ─ Binary / unary operators ──────────────────────────────────
                is Instruction.BinaryOpInstr -> {
                    val right = state.pop()
                    val left  = state.pop()
                    state.push(evalBinary(instr.op, left, right))
                }

                is Instruction.UnaryOpInstr -> {
                    val operand = state.pop()
                    state.push(evalUnary(instr.op, operand))
                }

                // ─ Predicate tests ───────────────────────────────────────────
                is Instruction.IsNull -> {
                    val v = state.pop()
                    state.push(SqlValue.BoolVal(v is SqlValue.Null))
                }

                is Instruction.IsNotNull -> {
                    val v = state.pop()
                    state.push(SqlValue.BoolVal(v !is SqlValue.Null))
                }

                is Instruction.Between -> doInstrBetween(instr, state)

                is Instruction.Like -> doLike(state)

                is Instruction.InList -> doInList(instr, state)

                // ─ Scans ─────────────────────────────────────────────────────
                is Instruction.OpenScan -> {
                    // Ask the backend for a cursor over the named table.
                    // We prefer openCursor() (which supports positioned UPDATE /
                    // DELETE) over scan() (read-only iterator).
                    val iter = openCursorOrScan(backend, instr.table)
                    state.cursors[instr.alias] = iter
                }

                is Instruction.AdvanceCursor -> doAdvanceCursor(instr, state)

                is Instruction.JumpIfExhausted -> doJumpIfExhausted(instr, state)

                is Instruction.CloseScan -> {
                    val iter = state.cursors.remove(instr.alias)
                    iter?.close()
                    state.currentRow.remove(instr.alias)
                }

                // ─ Row emission ───────────────────────────────────────────────
                is Instruction.BeginRow -> state.rowBuffer.clear()

                is Instruction.EmitColumn -> {
                    val value = state.pop()
                    state.rowBuffer.add(instr.name to value)
                }

                is Instruction.EmitRow -> {
                    state.resultRows.add(state.rowBuffer.toList())
                    state.rowBuffer.clear()
                }

                // ─ Aggregation ────────────────────────────────────────────────
                is Instruction.InitAgg   -> doInitAgg(instr, state)
                is Instruction.UpdateAgg -> doUpdateAgg(instr, state)
                is Instruction.FinalizeAgg -> doFinalizeAgg(instr, state)

                is Instruction.SaveGroupKey -> {
                    // Pop `keys.size` values from the stack (oldest first) and
                    // store them as the current group key.
                    val n = instr.keys.size
                    val values = state.popN(n)
                    state.groupKey = values
                }

                is Instruction.AdvanceGroup -> doAdvanceGroup(instr, state)

                // ─ Control flow ────────────────────────────────────────────────
                is Instruction.Label -> { /* runtime no-op; used only for pre-scan */ }

                is Instruction.Jump -> {
                    state.pc = resolveLabel(instr.label, state)
                }

                is Instruction.JumpIfTrue -> {
                    val v = state.pop()
                    if (isTruthy(v)) {
                        state.pc = resolveLabel(instr.label, state)
                    }
                }

                is Instruction.JumpIfFalse -> {
                    val v = state.pop()
                    if (!isTruthy(v)) {
                        state.pc = resolveLabel(instr.label, state)
                    }
                }

                is Instruction.Halt -> break

                // ─ DDL ─────────────────────────────────────────────────────────
                is Instruction.CreateTableInstr -> {
                    // instr.columns is List<com.codingadventures.sqlplanner.ColumnDef>;
                    // backend.createTable expects List<com.codingadventures.sqlbackend.ColumnDef>.
                    // We convert each one via convertColumnDef().
                    backend.createTable(instr.name, instr.columns.map { convertColumnDef(it) }, instr.ifNotExists)
                }

                is Instruction.DropTableInstr -> {
                    backend.dropTable(instr.name, instr.ifExists)
                }

                // ─ DML ─────────────────────────────────────────────────────────
                is Instruction.InsertRow  -> doInsert(instr, state)
                is Instruction.UpdateRows -> doUpdate(instr, state)
                is Instruction.DeleteRows -> doDelete(instr, state)

                // ─ Transactions ────────────────────────────────────────────────
                is Instruction.BeginTransaction -> {
                    if (state.transactionHandle == null) {
                        state.transactionHandle = backend.beginTransaction()
                    }
                }

                is Instruction.CommitTransaction -> {
                    val handle = state.transactionHandle
                    if (handle != null) {
                        backend.commit(handle)
                        state.transactionHandle = null
                    }
                }

                is Instruction.RollbackTransaction -> {
                    val handle = state.transactionHandle
                    if (handle != null) {
                        backend.rollback(handle)
                        state.transactionHandle = null
                    }
                }

                // ─ Post-operation instructions ─────────────────────────────────
                is Instruction.SortResult     -> doSortResult(instr, state)
                is Instruction.DistinctResult -> doDistinctResult(state)
                is Instruction.LimitResult    -> doLimitResult(instr, state)
            }
        }

        return buildResult(state)
    }

    // ── Label resolution ──────────────────────────────────────────────────────

    /**
     * Build a map from label name to instruction index by scanning [instructions] once.
     *
     * Labels are [Instruction.Label] objects that act as no-ops at runtime.
     * Pre-scanning them means every jump is O(1) during execution.
     */
    private fun buildLabelIndex(instructions: List<Instruction>): Map<String, Int> {
        val map = HashMap<String, Int>(instructions.size)
        for ((idx, instr) in instructions.withIndex()) {
            if (instr is Instruction.Label) {
                map[instr.name] = idx
            }
        }
        return map
    }

    /**
     * Resolve a label name to its instruction index.
     *
     * Throws [IllegalArgumentException] if the label is unknown — this would
     * indicate a codegen bug, not a user error.
     */
    private fun resolveLabel(label: String, state: VmState): Int =
        state.labelIndex[label]
            ?: error("unknown label: '$label'")

    // ── Backend cursor helpers ────────────────────────────────────────────────

    /**
     * Open a cursor over [table].
     *
     * Prefers [CursorBackend.openCursor] (which returns a positioned [Cursor]
     * supporting UPDATE / DELETE) over the read-only [Backend.scan].  The
     * dispatch uses a Kotlin `is`-check instead of reflection, which is
     * type-safe, obfuscation-friendly, and throws no [ReflectiveOperationException].
     */
    private fun openCursorOrScan(backend: Backend, table: String): RowIterator =
        if (backend is CursorBackend) backend.openCursor(table)
        else backend.scan(table)

    // ── Column load ───────────────────────────────────────────────────────────

    /**
     * Load a column value from the current row of the named cursor.
     *
     * The cursor alias (or null for the default cursor) identifies which
     * open scan to read from.  If the cursor has no current row (e.g., the
     * cursor was already closed) we push NULL.
     */
    private fun doLoadColumn(instr: Instruction.LoadColumn, state: VmState) {
        val row = state.currentRow[instr.table]
        if (row == null) {
            state.push(SqlValue.Null)
            return
        }
        // Column lookup is case-insensitive (SQL standard).
        val rawValue = row.entries
            .find { it.key.equals(instr.column, ignoreCase = true) }
            ?.value
        state.push(backendValueToSqlValue(rawValue))
    }

    /**
     * Convert a backend [Any?] value to an [SqlValue].
     *
     * The backend stores values as untyped [Any?] (null, Long, Double, String,
     * Boolean, Blob).  We box them into the SqlValue sealed class hierarchy.
     */
    private fun backendValueToSqlValue(v: Any?): SqlValue = when (v) {
        null       -> SqlValue.Null
        is Boolean -> SqlValue.BoolVal(v)
        is Long    -> SqlValue.IntVal(v)
        is Int     -> SqlValue.IntVal(v.toLong())
        is Short   -> SqlValue.IntVal(v.toLong())
        is Byte    -> SqlValue.IntVal(v.toLong())
        is Double  -> SqlValue.FloatVal(v)
        is Float   -> SqlValue.FloatVal(v.toDouble())
        is String  -> SqlValue.TextVal(v)
        else       -> SqlValue.TextVal(v.toString())
    }

    /**
     * Convert an [SqlValue] to the native type that the [Backend] expects.
     *
     * The backend accepts null, Long, Double, String, or Boolean.
     */
    private fun sqlValueToBackend(v: SqlValue): Any? = when (v) {
        is SqlValue.Null    -> null
        is SqlValue.IntVal  -> v.v
        is SqlValue.FloatVal -> v.v
        is SqlValue.TextVal -> v.v
        is SqlValue.BoolVal -> v.v
    }

    // ── Cursor advance ────────────────────────────────────────────────────────

    /**
     * Advance the named cursor to the next row.
     *
     * If the cursor is exhausted (no more rows), jump to [instr.label].
     * Otherwise, update [VmState.currentRow] and fall through to the loop body.
     */
    private fun doAdvanceCursor(instr: Instruction.AdvanceCursor, state: VmState) {
        val cursor = state.cursors[instr.alias]
            ?: error("AdvanceCursor: no open cursor for alias '${instr.alias}'")
        val row = cursor.next()
        if (row == null) {
            // Cursor exhausted — jump to end label.
            state.currentRow.remove(instr.alias)
            state.pc = resolveLabel(instr.label, state)
        } else {
            state.currentRow[instr.alias] = row
        }
    }

    /**
     * Jump to [instr.label] if the named cursor is exhausted.
     *
     * This is a variant of [doAdvanceCursor] used in patterns where the advance
     * and the exhaustion check are separate steps.
     */
    private fun doJumpIfExhausted(instr: Instruction.JumpIfExhausted, state: VmState) {
        val cursor = state.cursors[instr.alias]
        if (cursor == null || state.currentRow[instr.alias] == null) {
            state.pc = resolveLabel(instr.label, state)
        }
    }

    // ── Binary operators ──────────────────────────────────────────────────────
    //
    // SQL's three-valued logic: any operation with a NULL input propagates NULL
    // as the output (except for AND/OR which have special truth tables).
    //
    // AND truth table:                 OR truth table:
    //   TRUE  AND TRUE  = TRUE           TRUE  OR FALSE = TRUE
    //   TRUE  AND FALSE = FALSE          TRUE  OR NULL  = TRUE
    //   TRUE  AND NULL  = NULL           FALSE OR FALSE = FALSE
    //   FALSE AND FALSE = FALSE          FALSE OR NULL  = NULL
    //   FALSE AND NULL  = FALSE          NULL  OR NULL  = NULL
    //   NULL  AND NULL  = NULL
    //
    // (Ref: SQL-92 §8.16.  Same semantics as SQLite.)

    /**
     * Evaluate a binary operation on two [SqlValue] operands.
     *
     * For arithmetic / comparison operations, NULL in → NULL out.
     * For AND / OR, SQL three-valued logic applies.
     * For CONCAT, NULL in → NULL out.
     */
    private fun evalBinary(op: BinaryOp, left: SqlValue, right: SqlValue): SqlValue {
        // AND and OR have special NULL handling — check them first.
        if (op == BinaryOp.AND) {
            return evalAnd(left, right)
        }
        if (op == BinaryOp.OR) {
            return evalOr(left, right)
        }

        // All other ops propagate NULL.
        if (left is SqlValue.Null || right is SqlValue.Null) return SqlValue.Null

        return when (op) {
            BinaryOp.ADD    -> evalArith(left, right, { a, b -> a + b }, { a, b -> a + b })
            BinaryOp.SUB    -> evalArith(left, right, { a, b -> a - b }, { a, b -> a - b })
            BinaryOp.MUL    -> evalArith(left, right, { a, b -> a * b }, { a, b -> a * b })
            BinaryOp.DIV    -> evalDiv(left, right)
            BinaryOp.MOD    -> evalMod(left, right)
            BinaryOp.EQ     -> SqlValue.BoolVal(sqlEquals(left, right))
            BinaryOp.NEQ    -> SqlValue.BoolVal(!sqlEquals(left, right))
            BinaryOp.LT     -> SqlValue.BoolVal(sqlCompare(left, right) < 0)
            BinaryOp.LTE    -> SqlValue.BoolVal(sqlCompare(left, right) <= 0)
            BinaryOp.GT     -> SqlValue.BoolVal(sqlCompare(left, right) > 0)
            BinaryOp.GTE    -> SqlValue.BoolVal(sqlCompare(left, right) >= 0)
            BinaryOp.CONCAT -> evalConcat(left, right)
            // AND / OR handled above — unreachable here.
            BinaryOp.AND, BinaryOp.OR -> SqlValue.Null
        }
    }

    /** SQL AND with three-valued logic. */
    private fun evalAnd(left: SqlValue, right: SqlValue): SqlValue {
        val l = toBoolOrNull(left)
        val r = toBoolOrNull(right)
        // FALSE AND anything = FALSE
        if (l == false || r == false) return SqlValue.BoolVal(false)
        // NULL AND TRUE = NULL; NULL AND NULL = NULL
        if (l == null || r == null) return SqlValue.Null
        return SqlValue.BoolVal(true)
    }

    /** SQL OR with three-valued logic. */
    private fun evalOr(left: SqlValue, right: SqlValue): SqlValue {
        val l = toBoolOrNull(left)
        val r = toBoolOrNull(right)
        // TRUE OR anything = TRUE
        if (l == true || r == true) return SqlValue.BoolVal(true)
        // NULL OR FALSE = NULL; NULL OR NULL = NULL
        if (l == null || r == null) return SqlValue.Null
        return SqlValue.BoolVal(false)
    }

    /**
     * Convert [SqlValue] to [Boolean?] for three-valued logic.
     *
     * NULL → null; FALSE/0/0.0 → false; everything else → true.
     */
    private fun toBoolOrNull(v: SqlValue): Boolean? = when (v) {
        is SqlValue.Null    -> null
        is SqlValue.BoolVal -> v.v
        is SqlValue.IntVal  -> v.v != 0L
        is SqlValue.FloatVal -> v.v != 0.0
        is SqlValue.TextVal -> v.v.isNotEmpty()
    }

    /**
     * Evaluate [v] as a truthy SQL value for JumpIfTrue / JumpIfFalse.
     *
     * NULL is falsy (jump is NOT taken for JumpIfTrue; IS taken for JumpIfFalse).
     * False, 0, and 0.0 are also falsy.
     */
    private fun isTruthy(v: SqlValue): Boolean = when (v) {
        is SqlValue.Null    -> false
        is SqlValue.BoolVal -> v.v
        is SqlValue.IntVal  -> v.v != 0L
        is SqlValue.FloatVal -> v.v != 0.0
        is SqlValue.TextVal -> v.v.isNotEmpty()
    }

    /**
     * Evaluate an arithmetic operation.
     *
     * If both operands are integers, the result is an integer (preserving SQL's
     * integer arithmetic for exact precision).  If either is a float, the result
     * is a float.
     *
     * The two lambdas [intOp] and [floatOp] provide the type-specific operation.
     */
    private inline fun evalArith(
        left: SqlValue,
        right: SqlValue,
        intOp: (Long, Long) -> Long,
        floatOp: (Double, Double) -> Double,
    ): SqlValue {
        val lNum = toNumber(left)
        val rNum = toNumber(right)
        if (lNum == null || rNum == null) return SqlValue.Null

        return if (left is SqlValue.FloatVal || right is SqlValue.FloatVal) {
            SqlValue.FloatVal(floatOp(lNum.toDouble(), rNum.toDouble()))
        } else {
            SqlValue.IntVal(intOp(lNum.toLong(), rNum.toLong()))
        }
    }

    /**
     * Integer or floating-point division.
     *
     * SQL: integer / integer = integer (truncated toward zero); divide-by-zero
     * returns NULL (SQLite behaviour — does not throw an exception).
     */
    private fun evalDiv(left: SqlValue, right: SqlValue): SqlValue {
        val lNum = toNumber(left) ?: return SqlValue.Null
        val rNum = toNumber(right) ?: return SqlValue.Null
        return if (left is SqlValue.FloatVal || right is SqlValue.FloatVal) {
            val d = rNum.toDouble()
            if (d == 0.0) SqlValue.Null else SqlValue.FloatVal(lNum.toDouble() / d)
        } else {
            val r = rNum.toLong()
            if (r == 0L) SqlValue.Null else SqlValue.IntVal(lNum.toLong() / r)
        }
    }

    /** Integer modulo; divide-by-zero returns NULL. */
    private fun evalMod(left: SqlValue, right: SqlValue): SqlValue {
        val lNum = toNumber(left) ?: return SqlValue.Null
        val rNum = toNumber(right) ?: return SqlValue.Null
        return if (left is SqlValue.FloatVal || right is SqlValue.FloatVal) {
            val d = rNum.toDouble()
            if (d == 0.0) SqlValue.Null else SqlValue.FloatVal(lNum.toDouble() % d)
        } else {
            val r = rNum.toLong()
            if (r == 0L) SqlValue.Null else SqlValue.IntVal(lNum.toLong() % r)
        }
    }

    /** String concatenation (SQL `||`). */
    private fun evalConcat(left: SqlValue, right: SqlValue): SqlValue {
        val l = sqlToString(left)
        val r = sqlToString(right)
        return SqlValue.TextVal(l + r)
    }

    /**
     * Convert [SqlValue] to a [Number] for arithmetic, or null if not numeric.
     *
     * Strings that represent numbers are coerced (SQLite does this too — e.g.
     * `'3' + 4` evaluates to 7).  Non-numeric strings yield null (which becomes
     * a NULL output via NULL propagation).
     */
    private fun toNumber(v: SqlValue): Number? = when (v) {
        is SqlValue.IntVal  -> v.v
        is SqlValue.FloatVal -> v.v
        is SqlValue.BoolVal -> if (v.v) 1L else 0L
        is SqlValue.TextVal -> v.v.toLongOrNull() ?: v.v.toDoubleOrNull()
        is SqlValue.Null    -> null
    }

    /**
     * SQL equality test — returns a Boolean (not an SqlValue) for use in
     * comparison operators.
     *
     * NULL == NULL is true for DISTINCT deduplication but false for regular
     * SQL comparison.  Here we implement regular equality: NULL is not equal
     * to anything (callers handle the NULL case before reaching this function).
     */
    private fun sqlEquals(left: SqlValue, right: SqlValue): Boolean {
        if (left is SqlValue.Null || right is SqlValue.Null) return false
        // Numeric cross-type comparison: IntVal == FloatVal when values are equal.
        val lNum = toNumber(left)
        val rNum = toNumber(right)
        if (lNum != null && rNum != null) {
            return lNum.toDouble() == rNum.toDouble()
        }
        return left == right
    }

    /**
     * SQL ordering comparison.
     *
     * Type ordering (SQLite affinity order): NULL < BOOL < INTEGER/REAL < TEXT.
     * Within integers and reals, numeric comparison.  Within text, lexicographic.
     */
    private fun sqlCompare(left: SqlValue, right: SqlValue): Int {
        // NULL sorts before everything.
        if (left is SqlValue.Null && right is SqlValue.Null) return 0
        if (left is SqlValue.Null) return -1
        if (right is SqlValue.Null) return 1

        val lNum = toNumber(left)
        val rNum = toNumber(right)
        if (lNum != null && rNum != null) {
            return lNum.toDouble().compareTo(rNum.toDouble())
        }
        // Fall back to string comparison (TEXT > numeric in SQLite affinity).
        return sqlToString(left).compareTo(sqlToString(right))
    }

    /** Convert any [SqlValue] to its SQL string representation. */
    private fun sqlToString(v: SqlValue): String = when (v) {
        is SqlValue.Null    -> ""
        is SqlValue.IntVal  -> v.v.toString()
        is SqlValue.FloatVal -> {
            // Match SQLite's float rendering: omit trailing ".0" for whole numbers.
            if (v.v == v.v.toLong().toDouble() && !v.v.isInfinite() && !v.v.isNaN()) {
                v.v.toLong().toString()
            } else {
                v.v.toString()
            }
        }
        is SqlValue.TextVal -> v.v
        is SqlValue.BoolVal -> if (v.v) "1" else "0"
    }

    // ── Unary operators ───────────────────────────────────────────────────────

    /**
     * Evaluate a unary operation.
     *
     * NEG: arithmetic negation.  -(NULL) = NULL.
     * NOT: boolean NOT.  NOT NULL = NULL; NOT TRUE = FALSE; NOT FALSE = TRUE.
     *      Note: NOT 0 = TRUE; NOT 1 = FALSE (SQLite integer truthiness).
     */
    private fun evalUnary(op: UnaryOp, operand: SqlValue): SqlValue = when (op) {
        UnaryOp.NEG -> {
            when (operand) {
                is SqlValue.Null    -> SqlValue.Null
                is SqlValue.IntVal  -> SqlValue.IntVal(-operand.v)
                is SqlValue.FloatVal -> SqlValue.FloatVal(-operand.v)
                is SqlValue.BoolVal -> SqlValue.IntVal(if (operand.v) -1L else 0L)
                is SqlValue.TextVal -> {
                    val n = toNumber(operand)
                    if (n == null) SqlValue.Null
                    else if (n is Long) SqlValue.IntVal(-n)
                    else SqlValue.FloatVal(-n.toDouble())
                }
            }
        }
        UnaryOp.NOT -> {
            when (operand) {
                is SqlValue.Null    -> SqlValue.Null
                is SqlValue.BoolVal -> SqlValue.BoolVal(!operand.v)
                is SqlValue.IntVal  -> SqlValue.BoolVal(operand.v == 0L)
                is SqlValue.FloatVal -> SqlValue.BoolVal(operand.v == 0.0)
                is SqlValue.TextVal -> {
                    // NOT '0' = TRUE; NOT '' = TRUE; NOT '1' = FALSE
                    val n = toNumber(operand) ?: return SqlValue.Null
                    SqlValue.BoolVal(n.toDouble() == 0.0)
                }
            }
        }
    }

    // ── BETWEEN ───────────────────────────────────────────────────────────────

    /**
     * Evaluate SQL BETWEEN: push TRUE if `low <= value <= high`.
     *
     * Stack layout when this instruction executes (top is last pushed):
     *   ... value  low  high
     *
     * We pop high first, then low, then value.
     *
     * Three-valued logic: any NULL input yields NULL.
     */
    private fun doInstrBetween(@Suppress("UNUSED_PARAMETER") instr: Instruction.Between, state: VmState) {
        val high  = state.pop()
        val low   = state.pop()
        val value = state.pop()

        if (value is SqlValue.Null || low is SqlValue.Null || high is SqlValue.Null) {
            state.push(SqlValue.Null)
            return
        }
        val ge = sqlCompare(value, low)  >= 0
        val le = sqlCompare(value, high) <= 0
        state.push(SqlValue.BoolVal(ge && le))
    }

    // ── LIKE ─────────────────────────────────────────────────────────────────
    //
    // SQL LIKE uses two wildcards:
    //   %   — matches any sequence of zero or more characters
    //   _   — matches any single character
    //
    // Escape character: not supported in Level 1 (the codegen's Like instruction
    // does not carry an escape character field).
    //
    // The algorithm uses a linear-time iterative matcher (classic two-pointer
    // backtracking approach) instead of converting to a regex.  This avoids
    // ReDoS vulnerabilities: a crafted pattern like `%a%a%a%a%a%a%a%b` with a
    // long non-matching text could cause exponential backtracking in a
    // regex-based implementation.
    //
    // The algorithm maintains:
    //   ti     — current position in text
    //   pi     — current position in pattern
    //   starPi — position of the last '%' wildcard in pattern (-1 = none)
    //   starTi — text position where we started trying to match after the last '%'
    //
    // When a mismatch occurs and we have a saved '%' position, we backtrack by
    // advancing starTi one character and replaying from starPi+1.  This is O(n*m)
    // worst-case (n = text length, m = pattern length) but avoids catastrophic
    // exponential blowup.

    /**
     * LIKE predicate.  Stack (top-to-bottom when this executes): pattern, value.
     *
     * We pop pattern first, then value.
     * NULL in → NULL out.
     */
    private fun doLike(state: VmState) {
        val pattern = state.pop()
        val value   = state.pop()

        if (value is SqlValue.Null || pattern is SqlValue.Null) {
            state.push(SqlValue.Null)
            return
        }

        val patStr = sqlToString(pattern)
        val valStr = sqlToString(value)

        state.push(SqlValue.BoolVal(likeMatch(valStr, patStr)))
    }

    /**
     * Match [text] against a SQL LIKE [pattern] using a linear-time iterative
     * matcher.
     *
     * Wildcards:
     *   `%` — matches any sequence of zero or more characters
     *   `_` — matches any single character
     *
     * All other characters match themselves case-insensitively (SQLite NOCASE
     * for ASCII compatibility).
     *
     * This avoids the ReDoS vulnerability inherent in converting `%` to `.*` and
     * delegating to [Regex]: a pattern with many `%` wildcards and a long
     * non-matching text could trigger exponential backtracking in the regex engine.
     */
    internal fun likeMatch(text: String, pattern: String): Boolean {
        var ti = 0      // current index into text
        var pi = 0      // current index into pattern
        var starPi = -1 // index of the most recent '%' in pattern; -1 = none seen
        var starTi = -1 // text index where we began the match attempt after '%'

        while (ti < text.length) {
            val pc = pattern.getOrNull(pi)
            when {
                // '%' matches zero or more characters — save this position and
                // advance the pattern pointer (consuming zero chars of text now).
                pc == '%' -> { starPi = pi++; starTi = ti }

                // '_' matches exactly one character; plain chars match
                // case-insensitively.
                pc == '_' || (pc != null && pc.equals(text[ti], ignoreCase = true)) -> {
                    ti++; pi++
                }

                // Mismatch — but we have a saved '%': backtrack by letting '%'
                // consume one more character of text and replay from starPi+1.
                starPi >= 0 -> { pi = starPi + 1; ti = ++starTi }

                // No '%' to backtrack to — no match.
                else -> return false
            }
        }

        // Consume any trailing '%' wildcards (they match the empty suffix).
        while (pi < pattern.length && pattern[pi] == '%') pi++

        // A match requires the pattern to be fully consumed.
        return pi == pattern.length
    }

    // ── IN list ───────────────────────────────────────────────────────────────
    //
    // Stack layout for `x IN (v1, v2, v3)` with count=3 when this executes:
    //   ... x  v1  v2  v3       (v3 is on top)
    //
    // We pop `count` list items first (they were pushed in order v1..vN so
    // popN returns them in push order), then pop the needle.
    //
    // NULL semantics (SQL standard):
    //   needle IS NULL → push NULL
    //   needle found in non-NULL items → push TRUE
    //   needle not found; no NULL in list → push FALSE
    //   needle not found; a NULL is in the list → push NULL

    /** SQL IN (v1, v2, …) predicate. */
    private fun doInList(instr: Instruction.InList, state: VmState) {
        val items  = state.popN(instr.count)
        val needle = state.pop()

        if (instr.count == 0) {
            state.push(SqlValue.BoolVal(false))
            return
        }

        if (needle is SqlValue.Null) {
            state.push(SqlValue.Null)
            return
        }

        var foundNull = false
        for (item in items) {
            if (item is SqlValue.Null) {
                foundNull = true
                continue
            }
            if (sqlEquals(needle, item)) {
                state.push(SqlValue.BoolVal(true))
                return
            }
        }
        state.push(if (foundNull) SqlValue.Null else SqlValue.BoolVal(false))
    }

    // ── Aggregation ───────────────────────────────────────────────────────────
    //
    // Two-phase aggregate computation:
    //   Phase 1 (scan loop): InitAgg → UpdateAgg  (per input row)
    //   Phase 2 (finalize):  AdvanceGroup → FinalizeAgg → EmitRow  (per group)
    //
    // The `aggTable` maps group keys to arrays of AggAccumulator.
    // The slot index (from InitAgg/UpdateAgg/FinalizeAgg) selects which aggregate
    // within the group we're updating.

    /** Ensure a slot exists for the current group key.  Idempotent. */
    private fun ensureGroup(state: VmState): MutableList<AggAccumulator> {
        return state.aggTable.getOrPut(state.groupKey) {
            state.groupOrder.add(state.groupKey)
            mutableListOf()
        }
    }

    /**
     * InitAgg: ensure aggregate slot [instr.index] exists for the current group.
     *
     * This instruction is emitted once per input row by the codegen (idempotent —
     * if the slot already exists we leave it unchanged so partial sums aren't reset).
     */
    private fun doInitAgg(instr: Instruction.InitAgg, state: VmState) {
        val slots = ensureGroup(state)
        while (slots.size <= instr.index) {
            slots.add(AggAccumulator(fn = instr.fn))
        }
    }

    /**
     * UpdateAgg: pop one value and feed it into aggregate slot [instr.index].
     *
     * NULL values are ignored by all aggregate functions except COUNT(*).
     */
    private fun doUpdateAgg(instr: Instruction.UpdateAgg, state: VmState) {
        val value = state.pop()
        val slots = ensureGroup(state)
        while (slots.size <= instr.index) {
            slots.add(AggAccumulator(fn = instr.fn))
        }
        val acc = slots[instr.index]

        when (acc.fn) {
            AggFn.COUNT_STAR -> acc.count++

            AggFn.COUNT -> {
                if (value !is SqlValue.Null) acc.count++
            }

            AggFn.SUM -> {
                if (value !is SqlValue.Null) {
                    acc.sum = if (acc.sum is SqlValue.Null) value else addSqlValues(acc.sum, value)
                }
            }

            AggFn.AVG -> {
                if (value !is SqlValue.Null) {
                    acc.sum = if (acc.sum is SqlValue.Null) value else addSqlValues(acc.sum, value)
                    acc.count++
                }
            }

            AggFn.MIN -> {
                if (value !is SqlValue.Null) {
                    if (acc.sum is SqlValue.Null || sqlCompare(value, acc.sum) < 0) {
                        acc.sum = value
                    }
                }
            }

            AggFn.MAX -> {
                if (value !is SqlValue.Null) {
                    if (acc.sum is SqlValue.Null || sqlCompare(value, acc.sum) > 0) {
                        acc.sum = value
                    }
                }
            }
        }
    }

    /** Add two non-null SQL numeric values (used by SUM / AVG accumulation). */
    private fun addSqlValues(a: SqlValue, b: SqlValue): SqlValue {
        val aNum = toNumber(a) ?: return b
        val bNum = toNumber(b) ?: return a
        return if (a is SqlValue.FloatVal || b is SqlValue.FloatVal) {
            SqlValue.FloatVal(aNum.toDouble() + bNum.toDouble())
        } else {
            SqlValue.IntVal(aNum.toLong() + bNum.toLong())
        }
    }

    /**
     * FinalizeAgg: compute the final aggregate value and push it.
     *
     * Called during the finalize (group-emit) phase.  The current group key
     * is already set by [doAdvanceGroup].
     */
    private fun doFinalizeAgg(instr: Instruction.FinalizeAgg, state: VmState) {
        // Auto-grow: if InitAgg was never called for this group (happens when
        // the table is empty and GROUP BY emits one implicit group), create a
        // default-zeroed accumulator.
        val slots = ensureGroup(state)
        while (slots.size <= instr.index) {
            slots.add(AggAccumulator(fn = instr.fn))
        }
        val acc = slots[instr.index]

        val result: SqlValue = when (acc.fn) {
            AggFn.COUNT, AggFn.COUNT_STAR -> SqlValue.IntVal(acc.count)
            AggFn.SUM   -> acc.sum                  // NULL if no non-null inputs
            AggFn.AVG   -> {
                if (acc.count == 0L) SqlValue.Null
                else {
                    val s = toNumber(acc.sum)?.toDouble() ?: 0.0
                    SqlValue.FloatVal(s / acc.count.toDouble())
                }
            }
            AggFn.MIN, AggFn.MAX -> acc.sum         // NULL if no non-null inputs
        }
        state.push(result)
    }

    /**
     * AdvanceGroup: move to the next accumulated group for the finalize phase.
     *
     * Increments the group iterator.  If all groups have been emitted, this
     * instruction must jump to a "done" label — but the codegen doesn't embed
     * the label in [Instruction.AdvanceGroup].  Instead, the codegen structure is:
     *
     *   Label("agg_N_finalize")
     *   AdvanceGroup          ← we increment groupIter here
     *   BeginRow
     *   LoadGroupKey(0) ... FinalizeAgg ... EmitRow
     *   Jump("agg_N_finalize")
     *   Label("agg_N_done")
     *
     * The exhaustion check is handled by the pattern: after AdvanceGroup the VM
     * sets groupKey to the next group's key.  If there are no more groups, we
     * need to jump past the finalize loop.
     *
     * To accomplish this without a label in the AdvanceGroup instruction, we
     * scan ahead to find the matching Jump-back instruction and skip past it
     * (jumping to the label after the finalize loop).
     *
     * Simpler alternative: look at the instruction stream and find the Label
     * that follows the finalize loop's closing Jump.  This is deterministic
     * because the codegen always emits:
     *
     *   Label("agg_N_finalize")   ← labelIndex maps this to some index I
     *   AdvanceGroup              ← current pc-1
     *   BeginRow
     *   ...
     *   EmitRow
     *   Jump("agg_N_finalize")    ← jumps back to I
     *   Label("agg_N_done")       ← THIS is where we jump on exhaustion
     *
     * We find "agg_N_done" by scanning forward from pc for the first Label
     * instruction after a Jump that targets the finalize label.
     */
    private fun doAdvanceGroup(instr: Instruction.AdvanceGroup, state: VmState) {
        @Suppress("UNUSED_PARAMETER") val _unused = instr
        state.groupIter++

        // SQL standard: a global aggregate (no GROUP BY) over an empty table
        // must emit exactly one row of NULL/zero values.  Detect this by
        // checking whether the implicit empty-key group was ever added.
        if (state.groupIter == 0 && state.groupOrder.isEmpty()) {
            // Synthesise the implicit single-group with no accumulated rows.
            state.groupOrder.add(emptyList())
            state.aggTable[emptyList()] = mutableListOf()
        }

        if (state.groupIter >= state.groupOrder.size) {
            // All groups emitted — jump past the finalize loop.
            // Find the Label instruction that comes after the next backward Jump.
            val targetPc = findGroupDoneLabel(state)
            state.pc = targetPc
        } else {
            state.groupKey = state.groupOrder[state.groupIter]
        }
    }

    /**
     * Scan forward from the current [VmState.pc] to find the instruction index
     * that the finalize loop should jump to on exhaustion.
     *
     * The codegen emits:
     *   AdvanceGroup         ← state.pc - 1 (we've already incremented)
     *   ...body...
     *   Jump("agg_N_finalize")
     *   Label("agg_N_done") ← this is the target
     *
     * We look for the first Jump instruction followed immediately by a Label.
     */
    private fun findGroupDoneLabel(state: VmState): Int {
        val instrs = state.instructions
        var i = state.pc
        while (i < instrs.size) {
            val curr = instrs[i]
            if (curr is Instruction.Jump) {
                // Found the closing Jump.  The instruction after it is the done label.
                val next = i + 1
                if (next < instrs.size && instrs[next] is Instruction.Label) {
                    return next + 1  // skip the Label itself (it's a no-op)
                }
                // No Label follows — jump to end.
                return instrs.size
            }
            i++
        }
        return instrs.size
    }

    // ── DML ───────────────────────────────────────────────────────────────────

    /**
     * InsertRow: pop values for each column and insert a row into the backend.
     *
     * If [instr.columns] is non-null, values are mapped positionally to column
     * names.  If null, the backend uses its schema column order.
     */
    private fun doInsert(instr: Instruction.InsertRow, state: VmState) {
        val columns = instr.columns
        val row = Row()
        if (columns != null) {
            val values = state.popN(columns.size)
            for ((i, col) in columns.withIndex()) {
                row[col] = sqlValueToBackend(values[i])
            }
        } else {
            // Determine column count from the backend schema.
            val colDefs = try { state.backend.columns(instr.table) } catch (e: Exception) { emptyList<ColumnDef>() }
            val values = state.popN(colDefs.size)
            for ((i, cd) in colDefs.withIndex()) {
                row[cd.name] = sqlValueToBackend(values[i])
            }
        }
        state.backend.insert(instr.table, row)
        state.rowsAffected++
    }

    /**
     * UpdateRows: update the current cursor row with the assignment expressions.
     *
     * The assignment column names come from the plan (stored in the UpdateRows
     * instruction's context).  The codegen pushes one value per assignment
     * expression before emitting UpdateRows.
     *
     * Since UpdateRows doesn't carry the column list in the Instruction, we
     * read the schema and map positionally — the codegen emits values in schema
     * order for UPDATE.  This matches the C# and Python references.
     */
    private fun doUpdate(instr: Instruction.UpdateRows, state: VmState) {
        // The cursor for UPDATE is always the anonymous (null) cursor.
        val cursor = state.cursors[null]
        if (cursor !is Cursor) return
        // Get column count from the schema to know how many values to pop.
        val colDefs = try { state.backend.columns(instr.table) } catch (e: Exception) { emptyList<ColumnDef>() }
        val values = state.popN(colDefs.size)
        val assignments = LinkedHashMap<String, Any?>()
        for ((i, cd) in colDefs.withIndex()) {
            assignments[cd.name] = sqlValueToBackend(values[i])
        }
        state.backend.update(instr.table, cursor, assignments)
        state.rowsAffected++
    }

    /**
     * DeleteRows: delete the row at the current anonymous cursor position.
     */
    private fun doDelete(instr: Instruction.DeleteRows, state: VmState) {
        val cursor = state.cursors[null]
        if (cursor !is Cursor) return
        state.backend.delete(instr.table, cursor)
        state.rowsAffected++
    }

    // ── Post-operation instructions ───────────────────────────────────────────
    //
    // These operate on the completed result buffer *after* all EmitRow
    // instructions have fired.  The codegen emits them just before Halt.

    /**
     * SortResult: sort the result buffer in-place.
     *
     * Each sort key carries:
     *   - expression (used by codegen; the VM sorts by column name at Level 1)
     *   - direction (ASC or DESC)
     *   - nullsOrder (FIRST or LAST)
     *
     * We sort the result rows list directly (Kotlin's sort is stable, so equal
     * keys preserve their insertion order — matching SQL's stable-sort guarantee).
     *
     * The SortKey from sql-planner carries:
     *   - keyExpr: SqlExpr  — usually a Column reference; we evaluate it against the row
     *   - direction: SortDir — ASC or DESC
     *   - nullOrder: NullOrder — NULLS_FIRST or NULLS_LAST
     */
    private fun doSortResult(instr: Instruction.SortResult, state: VmState) {
        if (state.resultRows.isEmpty() || instr.keys.isEmpty()) return

        // Build column-name → position map from the first row's column list.
        val colIndex: Map<String, Int> = buildColIndex(state)

        state.resultRows.sortWith(Comparator { rowA, rowB ->
            for (key in instr.keys) {
                // Evaluate the key expression against both rows.
                // For a Column reference, look up by column name in the result row.
                // For other expressions, we fall back to NULL (can't re-evaluate
                // arbitrary expressions without a full mini-interpreter here).
                val a = evalSortKeyExpr(key.keyExpr, rowA, colIndex)
                val b = evalSortKeyExpr(key.keyExpr, rowB, colIndex)

                val cmp = sortCompare(a, b, key)
                if (cmp != 0) return@Comparator cmp
            }
            0
        })
    }

    /**
     * Evaluate a sort key expression against a result row.
     *
     * For simple column references (the common case), we look up by name in
     * the result row's column list.  For literal constants we return the value
     * directly.  For other expressions we return NULL (sort key can't be
     * evaluated without a full execution context at sort time).
     */
    private fun evalSortKeyExpr(
        expr: com.codingadventures.sqlplanner.SqlExpr,
        row: List<Pair<String, SqlValue>>,
        colIndex: Map<String, Int>,
    ): SqlValue = when (expr) {
        is com.codingadventures.sqlplanner.SqlExpr.Column -> {
            val idx = colIndex[expr.column.lowercase()]
            row.getOrNull(idx ?: -1)?.second ?: SqlValue.Null
        }
        is com.codingadventures.sqlplanner.SqlExpr.Literal -> {
            when (val v = expr.value) {
                null       -> SqlValue.Null
                is Long    -> SqlValue.IntVal(v)
                is Int     -> SqlValue.IntVal(v.toLong())
                is Double  -> SqlValue.FloatVal(v)
                is Boolean -> SqlValue.BoolVal(v)
                is String  -> SqlValue.TextVal(v)
                else       -> SqlValue.TextVal(v.toString())
            }
        }
        else -> SqlValue.Null
    }

    /**
     * Compare two values for ORDER BY, respecting direction and NULL placement.
     *
     * Uses the actual sql-planner types:
     *   - SortDir.ASC / SortDir.DESC
     *   - NullOrder.NULLS_FIRST / NullOrder.NULLS_LAST
     */
    private fun sortCompare(
        a: SqlValue,
        b: SqlValue,
        key: com.codingadventures.sqlplanner.SortKey,
    ): Int {
        // NULL placement.
        val aNull = a is SqlValue.Null
        val bNull = b is SqlValue.Null
        if (aNull && bNull) return 0
        val nullsFirst = key.nullOrder == com.codingadventures.sqlplanner.NullOrder.NULLS_FIRST
        if (aNull) return if (nullsFirst) -1 else 1
        if (bNull) return if (nullsFirst) 1 else -1

        val cmp = sqlCompare(a, b)
        return if (key.direction == com.codingadventures.sqlplanner.SortDir.DESC) -cmp else cmp
    }

    /**
     * Build a column-name → position map from the current result rows.
     *
     * We look at the column names in the first result row (if any).
     */
    private fun buildColIndex(state: VmState): Map<String, Int> {
        val first = state.resultRows.firstOrNull() ?: return emptyMap()
        return first.mapIndexed { i, (name, _) -> name.lowercase() to i }.toMap()
    }

    /**
     * DistinctResult: remove duplicate rows from the result buffer.
     *
     * Two rows are duplicates if every column value is SQL-equal.  For
     * deduplication, NULL == NULL (unlike regular SQL equality).
     */
    private fun doDistinctResult(state: VmState) {
        val seen = linkedSetOf<List<SqlValue>>()
        state.resultRows.retainAll { row ->
            val key = row.map { (_, v) -> v }
            seen.add(key)
        }
    }

    /**
     * LimitResult: apply OFFSET then LIMIT to the result buffer.
     *
     * - [offset] rows are skipped (default 0).
     * - At most [count] rows are kept (null = unlimited).
     */
    private fun doLimitResult(instr: Instruction.LimitResult, state: VmState) {
        // Clamp the offset to [0, Int.MAX_VALUE] before narrowing to Int.
        // A raw `.toInt()` on a Long > Int.MAX_VALUE silently overflows to a
        // negative number, which would corrupt subList indices.
        val start = (instr.offset ?: 0L)
            .coerceAtLeast(0L)
            .coerceAtMost(Int.MAX_VALUE.toLong())
            .toInt()
        val count = instr.count

        val sliced = if (start >= state.resultRows.size) {
            emptyList()
        } else {
            state.resultRows.subList(start, state.resultRows.size)
        }

        val limited = if (count == null) {
            sliced.toList()
        } else {
            // Clamp count to [0, Int.MAX_VALUE] before narrowing for the same reason.
            val safeCount = count
                .coerceAtLeast(0L)
                .coerceAtMost(Int.MAX_VALUE.toLong())
                .toInt()
            sliced.take(safeCount)
        }

        state.resultRows.clear()
        state.resultRows.addAll(limited)
    }

    // ── DDL helpers ───────────────────────────────────────────────────────────

    /**
     * Convert a sql-planner [PlannerColumnDef] to a sql-backend [ColumnDef].
     *
     * The two types are structurally identical except for the `default` field:
     *   - sql-planner carries `default: SqlExpr?` (an AST node)
     *   - sql-backend carries `defaultValue: Any?` + `hasDefault: Boolean` (a raw value)
     *
     * For level-1 we only extract literal defaults; any non-literal SqlExpr
     * default is dropped (rare and out-of-scope for the VM layer).  The backend
     * enforces constraints itself so we just pass through the flags.
     */
    private fun convertColumnDef(src: PlannerColumnDef): ColumnDef {
        val (defaultValue, hasDefault) = when (val d = src.default) {
            null -> null to false
            is PlannerSqlExpr.Literal -> d.value to true
            else -> null to false   // non-literal default: drop (unsupported at L1)
        }
        return ColumnDef(
            name          = src.name,
            typeName      = src.typeName,
            notNull       = src.notNull,
            primaryKey    = src.primaryKey,
            unique        = src.unique,
            defaultValue  = defaultValue,
            hasDefault    = hasDefault,
        )
    }

    // ── Result construction ───────────────────────────────────────────────────

    /**
     * Convert the accumulated [VmState] into a [QueryResult].
     *
     * The result rows were stored as `List<Pair<String, SqlValue>>` during
     * execution (column name + value).  Here we extract the column names
     * (from the first row, or an empty list) and package everything up.
     */
    private fun buildResult(state: VmState): QueryResult {
        val firstRow = state.resultRows.firstOrNull()
        val columns = firstRow?.map { (name, _) -> name } ?: emptyList()
        val rows = state.resultRows.map { row -> row.map { (_, v) -> v } }
        return QueryResult(
            columns = columns,
            rows    = rows,
            rowsAffected = state.rowsAffected,
        )
    }
}

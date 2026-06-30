package com.codingadventures.sqlcodegen

// SqlCodegen.kt — Kotlin bytecode code generator for the mini-sqlite Level 1 pipeline.
//
// Takes an OptimizedPlan from sql-optimizer and produces a flat list of stack-machine
// bytecode instructions that the sql-vm can execute.
//
// Architecture
// ────────────
// The compilation pipeline:
//
//   OptimizedPlan (from sql-optimizer)
//       │  SqlCodegen.compile()
//       ▼
//   Program { instructions: List<Instruction> }
//       │  sql-vm (next stage)
//       ▼
//   QueryResult
//
// Why compile to bytecode?
// ────────────────────────
// A plan tree is great for reasoning and optimisation, but awkward for direct
// execution: tree traversal requires recursion, and recursive interpreters are
// hard to debug and hard to port.  A *flat* bytecode program solves this — the
// VM is a simple loop: read the next instruction, execute it, advance the
// program counter.  This is the same insight behind CPython's bytecode, the
// JVM, WebAssembly, and SQLite's VDBE.
//
// Design constraints from CLAUDE.md
// ───────────────────────────────────
// • Knuth-style literate programming: explanatory comments, analogies, diagrams,
//   and examples are welcome inline.
// • typealias cannot live inside an object body — place at file scope.
// • Kotlin when-expressions use "is Type -> …" patterns, NOT "is Type when cond"
//   (guard syntax is unsupported in Kotlin 2.x).
// • Instruction class names must not collide with planner types.  Specifically
//   the codegen emits CreateTableInstr / DropTableInstr (not CreateTable /
//   DropTable) so the import from com.codingadventures.sqlplanner can coexist.

import com.codingadventures.sqlplanner.*
import com.codingadventures.sqloptimizer.OptimizedPlan

// ── Value type used in LoadConst ─────────────────────────────────────────────
//
// SQL supports a small domain of literal values.  We encode them as a sealed
// class so the VM can pattern-match exhaustively rather than casting Any?.
//
// Think of SqlValue like a tagged union in C — one enum tag per variant, plus
// the payload.  Unlike a raw Any?, a sealed class gives us compile-time
// exhaustiveness checks.

sealed class SqlValue {
    /** SQL NULL — the absence of a value.  Distinct from zero or empty string. */
    object Null : SqlValue() { override fun toString() = "NULL" }

    /** 64-bit signed integer.  Maps to Kotlin/JVM Long. */
    data class IntVal(val v: Long) : SqlValue()

    /** IEEE-754 double-precision float. */
    data class FloatVal(val v: Double) : SqlValue()

    /** Arbitrary-length Unicode text. */
    data class TextVal(val v: String) : SqlValue()

    /** Boolean — TRUE or FALSE. */
    data class BoolVal(val v: Boolean) : SqlValue()
}

// ── Operator enumerations ─────────────────────────────────────────────────────
//
// These mirror the planner's BinaryOperator / UnaryOperator but are codegen-
// internal enums.  Having our own enums lets the VM package depend only on
// sql-codegen without pulling in the planner's type graph.

/** Binary operations that the stack machine can execute in one step. */
enum class BinaryOp {
    // Arithmetic
    ADD, SUB, MUL, DIV, MOD,
    // Comparison
    EQ, NEQ, LT, LTE, GT, GTE,
    // Logic
    AND, OR,
    // String
    CONCAT
}

/** Unary operations (single operand from the stack). */
enum class UnaryOp { NEG, NOT }

/** Aggregate function kinds. */
enum class AggFn {
    COUNT,       // COUNT(expr) — ignores NULLs
    COUNT_STAR,  // COUNT(*)   — counts all rows
    SUM,
    AVG,
    MIN,
    MAX
}

// ── Instruction set ───────────────────────────────────────────────────────────
//
// The IR is a *register-free stack machine*: operands are pushed onto a value
// stack; operations pop their arguments and push their results.  There are no
// named registers — all intermediate values live on the stack.
//
// Think of RPN (Reverse Polish Notation): to compute 3 + 4, you push 3, push 4,
// then invoke BinaryOpInstr(ADD).  The result 7 remains on the stack.
//
// The VM additionally maintains:
//   • cursor table    — open iterators over table data
//   • row buffer      — the output row currently being assembled
//   • result buffer   — all completed output rows
//   • aggregate state — per-slot running aggregate values

sealed class Instruction {

    // ── Stack instructions ────────────────────────────────────────────────────

    /**
     * Push a compile-time-known literal value onto the stack.
     *
     * Example: `LoadConst(SqlValue.IntVal(42))` leaves 42 on the stack.
     */
    data class LoadConst(val value: SqlValue) : Instruction()

    /**
     * Read a column from the current row of the named cursor and push it.
     *
     * [table] is the table alias (or null for the default cursor); [column] is
     * the column name.  Pushes SqlValue.Null if the column is absent.
     */
    data class LoadColumn(val table: String?, val column: String) : Instruction()

    /**
     * Push the i-th bound query parameter.  Reserved for parameterised queries;
     * not produced by the Level-1 compiler (which inlines all literals), but
     * present for forward-compatibility.
     */
    data class LoadParam(val index: Int) : Instruction()

    /**
     * Push the i-th component of the saved group key.
     *
     * During aggregate output, after all groups have been accumulated, the VM
     * iterates over its group-key hash map.  For each group, the GROUP BY
     * column values can be recovered via LoadGroupKey(i).
     */
    data class LoadGroupKey(val index: Int) : Instruction()

    /**
     * Discard the top stack value.  Used to clean up unused expression results,
     * for example the implicit "affected row count" returned by UpdateRows.
     */
    object Pop : Instruction()

    // ── Arithmetic and comparison ─────────────────────────────────────────────

    /**
     * Pop right, pop left, apply [op], push result.
     *
     * Follows three-valued (SQL) logic for NULL: arithmetic or comparison with
     * NULL yields NULL.  AND / OR follow SQL truth tables:
     *
     *   NULL AND TRUE  = NULL    NULL OR TRUE  = TRUE
     *   NULL AND FALSE = FALSE   NULL OR FALSE = NULL
     */
    data class BinaryOpInstr(val op: BinaryOp) : Instruction()

    /**
     * Pop one value, apply [op], push result.
     *
     *   NEG: negate a number; -(NULL) = NULL
     *   NOT: boolean NOT; NOT NULL = NULL
     */
    data class UnaryOpInstr(val op: UnaryOp) : Instruction()

    // ── Predicate tests ───────────────────────────────────────────────────────

    /**
     * Pop one value. Push TRUE if it is NULL, FALSE otherwise.
     *
     * This is SQL `IS NULL`.  Note that in SQL, NULL = NULL is also NULL (not
     * TRUE) — IS NULL is the only way to detect the absence of a value.
     */
    object IsNull : Instruction()

    /**
     * Pop one value. Push TRUE if it is NOT NULL, FALSE otherwise.
     *
     * SQL: `IS NOT NULL`.
     */
    object IsNotNull : Instruction()

    /**
     * Pop three values: high, low, value.  Push TRUE if low <= value <= high.
     *
     * [inclusive] = true means the range is closed on both ends (the default,
     * matching SQL BETWEEN semantics).  NULL propagates per three-valued logic.
     */
    data class Between(val inclusive: Boolean = true) : Instruction()

    /**
     * SQL LIKE predicate.  Pop pattern, pop value.  Push TRUE if value matches
     * the SQL LIKE pattern (% = any sequence of chars, _ = any single char).
     * Push NULL if either operand is NULL.
     */
    object Like : Instruction()

    /**
     * Pop [count] list items, then pop the needle.  Push TRUE if any item equals
     * the needle; FALSE if none match and no list item was NULL; NULL if no match
     * but a NULL was present.
     *
     * This implements SQL `IN (v1, v2, …)`.
     */
    data class InList(val count: Int) : Instruction()

    // ── Scan / cursor instructions ────────────────────────────────────────────
    //
    // Cursors are named iterators over table rows.  Each Scan node in the plan
    // gets a unique cursor name (derived from the table alias or the table name
    // with a counter suffix).
    //
    // The cursor lifecycle is: OpenScan → [AdvanceCursor → body]* → CloseScan.
    // Think of it like a Java Iterator: hasNext() + next() fused into one
    // AdvanceCursor instruction that also does the branch-if-exhausted.

    /**
     * Ask the storage backend to open an iterator over [table], storing it
     * under the name [alias] in the cursor table.
     */
    data class OpenScan(val table: String, val alias: String?) : Instruction()

    /**
     * Advance the cursor identified by [alias] to the next row.  If no more
     * rows exist, jump to [label]; otherwise fall through to the body.
     */
    data class AdvanceCursor(val alias: String?, val label: String) : Instruction()

    /**
     * Jump to [label] if the cursor named [alias] is exhausted (no more rows).
     * Used in loop headers where the advance is a separate step.
     */
    data class JumpIfExhausted(val alias: String?, val label: String) : Instruction()

    /**
     * Release the cursor [alias], freeing any backend resources (file handles,
     * locks, etc.).
     */
    data class CloseScan(val alias: String?) : Instruction()

    // ── Row construction ──────────────────────────────────────────────────────
    //
    // Output rows are assembled into a "row buffer" and then committed to the
    // "result buffer" with EmitRow.

    /**
     * Clear the row buffer and begin assembling a new output row.
     * Must be followed by one or more EmitColumn instructions.
     */
    object BeginRow : Instruction()

    /**
     * Pop the top stack value and store it in the row buffer under [name].
     */
    data class EmitColumn(val name: String) : Instruction()

    /**
     * Commit the current row buffer to the result buffer.  After this
     * instruction the row buffer is empty and ready for the next row.
     */
    object EmitRow : Instruction()

    // ── Aggregation ───────────────────────────────────────────────────────────
    //
    // SQL aggregates (COUNT, SUM, AVG, MIN, MAX) are computed in two phases:
    //
    //   Phase 1 — accumulate: during the scan loop, each matching row calls
    //             UpdateAgg(slot, fn) to feed one value into the running total.
    //
    //   Phase 2 — finalize: after the scan loop ends, FinalizeAgg(slot, fn)
    //             computes and pushes the final aggregate value.
    //
    // This is the same two-phase approach used by DBMSes like PostgreSQL, where
    // the "transition function" runs per-row and the "final function" runs once.

    /**
     * Initialize aggregate slot [index] for function [fn].
     * Zero states: COUNT/COUNT_STAR = 0, SUM/AVG/MIN/MAX = NULL.
     */
    data class InitAgg(val index: Int, val fn: AggFn) : Instruction()

    /**
     * Pop the top stack value and feed it into aggregate slot [index].
     * NULL values are ignored by COUNT(col), SUM, AVG, MIN, MAX.
     * COUNT_STAR always increments regardless of the popped value.
     */
    data class UpdateAgg(val index: Int, val fn: AggFn) : Instruction()

    /**
     * Compute the final aggregate value for slot [index] and push it.
     * For AVG: computes sum/count; returns NULL if count = 0.
     */
    data class FinalizeAgg(val index: Int, val fn: AggFn) : Instruction()

    /**
     * Pop [keys].size values and save them as the current group key.
     * The VM uses the group key to route aggregate updates to the right group.
     * [keys] carries the column names for human-readable debugging.
     */
    data class SaveGroupKey(val keys: List<String>) : Instruction()

    /**
     * Advance the VM's internal group iterator to the next accumulated group.
     * Used during the finalize phase to walk over all groups and emit one row
     * per group.
     */
    object AdvanceGroup : Instruction()

    // ── Control flow ──────────────────────────────────────────────────────────
    //
    // Labels are resolved to instruction indices after all instructions have
    // been emitted.  During code generation they are referenced by name (a
    // string like "scan_0_loop") to keep the code readable and debuggable.

    /**
     * Named jump target.  This is a no-op at runtime; the VM resolves all
     * label references to indices before starting execution.
     */
    data class Label(val name: String) : Instruction()

    /** Unconditionally jump to the instruction at [label]. */
    data class Jump(val label: String) : Instruction()

    /**
     * Pop the stack.  If the value is TRUE, jump to [label].
     * If FALSE or NULL, fall through.
     */
    data class JumpIfTrue(val label: String) : Instruction()

    /**
     * Pop the stack.  If the value is FALSE or NULL, jump to [label].
     * If TRUE, fall through.
     */
    data class JumpIfFalse(val label: String) : Instruction()

    /**
     * Stop execution.  The result buffer contains the final output.
     * Every program must end with Halt (possibly preceded by post-op
     * instructions like SortResult).
     */
    object Halt : Instruction()

    // ── DDL instructions ──────────────────────────────────────────────────────
    //
    // Names use the "Instr" suffix to avoid clashing with the planner's
    // CreateTable / DropTable plan-node classes when both are in scope.

    /**
     * Ask the backend to create a new table named [name] with schema [columns].
     * If [ifNotExists] is true and the table already exists, this is a no-op.
     */
    data class CreateTableInstr(
        val name: String,
        val ifNotExists: Boolean,
        val columns: List<ColumnDef>   // ColumnDef from com.codingadventures.sqlplanner
    ) : Instruction()

    /**
     * Ask the backend to drop the table named [name].
     * If [ifExists] is true and the table does not exist, this is a no-op.
     */
    data class DropTableInstr(val name: String, val ifExists: Boolean) : Instruction()

    // ── DML instructions ──────────────────────────────────────────────────────

    /**
     * Pop one value per column in [columns] (last column first) and ask the
     * backend to insert a new row into [table].
     */
    data class InsertRow(val table: String, val columns: List<String>?) : Instruction()

    /**
     * For the current scan cursor position, apply [assignments] to the row and
     * ask the backend to persist the change.  The count of updated rows is
     * pushed onto the stack.
     */
    data class UpdateRows(val table: String) : Instruction()

    /**
     * Delete the row at the current scan cursor position.  The count of deleted
     * rows is pushed onto the stack.
     */
    data class DeleteRows(val table: String) : Instruction()

    // ── Transaction instructions ──────────────────────────────────────────────

    /** Begin a new transaction.  No-op if already in a transaction. */
    object BeginTransaction : Instruction()

    /** Commit the current transaction. */
    object CommitTransaction : Instruction()

    /** Roll back the current transaction, discarding all changes. */
    object RollbackTransaction : Instruction()

    // ── Post-operation instructions ───────────────────────────────────────────
    //
    // These operate on the *completed* result buffer (after all EmitRow
    // instructions have run) and are emitted at the end of the program — just
    // before Halt.  The order is: Sort → Distinct → Limit.

    /**
     * Sort the result buffer in-place by [keys].
     * Each SortKey carries the expression, direction (ASC/DESC), and null order.
     * The sort is stable so rows with equal keys keep their insertion order.
     */
    data class SortResult(val keys: List<SortKey>) : Instruction()  // SortKey from sqlplanner

    /**
     * Remove duplicate rows from the result buffer.  Two rows are duplicates if
     * every column compares equal (NULLs compare equal for deduplication, unlike
     * regular SQL equality).
     */
    object DistinctResult : Instruction()

    /**
     * Truncate the result buffer: skip the first [offset] rows (default 0), then
     * keep at most [count] rows (null = unlimited).
     */
    data class LimitResult(val count: Long?, val offset: Long?) : Instruction()
}

// ── Program ───────────────────────────────────────────────────────────────────
//
// A compiled program is simply an ordered list of instructions.  Labels in
// Jump/JumpIfFalse/JumpIfTrue/AdvanceCursor are stored as names (strings) and
// must be resolved to indices by the VM before execution.

/**
 * A compiled bytecode program ready for execution by the sql-vm.
 *
 * [instructions] is the flat, ordered instruction sequence.  The VM executes
 * instructions[0], then instructions[1], and so on, unless a control-flow
 * instruction redirects the program counter.
 */
data class Program(val instructions: List<Instruction>)

// ── LabelCounter (file-scope helper) ─────────────────────────────────────────
//
// Generates unique label names across the entire compilation of one plan.
// Each call to next() returns a fresh integer; format strings build human-
// readable label names like "scan_3_loop".
//
// We keep this at file scope (not inside SqlCodegen) because Kotlin object
// bodies cannot contain top-level helper classes that need mutable state.

/**
 * A simple monotonically-increasing counter used to produce unique label names.
 *
 * Analogy: think of it as a "ticket dispenser" at a bakery — each call to
 * [next] gives you a fresh number you can use to build a unique label name.
 */
private class LabelCounter {
    private var n = 0
    fun next(): Int = n++
}

// ── SqlCodegen ────────────────────────────────────────────────────────────────
//
// Entry point for the code generator.  Provides two public functions:
//
//   compile(plan)            — compile a full OptimizedPlan to a Program
//   compileExpression(expr)  — compile a single SqlExpr to instructions (for tests)

/**
 * Bytecode code generator for the mini-sqlite Level 1 pipeline.
 *
 * Transforms an [OptimizedPlan] produced by [com.codingadventures.sqloptimizer.SqlOptimizer]
 * into a [Program] suitable for execution by the sql-vm.
 */
object SqlCodegen {

    // ── Public API ─────────────────────────────────────────────────────────────

    /**
     * Compile [plan] into a flat list of stack-machine instructions.
     *
     * The compiler peels Sort / Limit / Distinct wrappers off SELECT-style
     * plans (since those operate on the completed result buffer) and defers them
     * to post-operation instructions appended after the core scan loop.
     */
    fun compile(plan: OptimizedPlan): Program {
        val lc = LabelCounter()
        val instructions = mutableListOf<Instruction>()

        // Peel post-ops (Sort / Limit / Distinct) from the outermost plan layers.
        // These always operate on the complete result buffer, so they're emitted
        // at the very end — after all rows have been accumulated.
        val (core, postOps) = peelPostOps(plan)

        compileNode(core, instructions, lc)

        // Append post-operation instructions in Sort → Distinct → Limit order,
        // then unconditionally Halt.
        for (op in postOps) {
            instructions.add(op)
        }
        instructions.add(Instruction.Halt)

        return Program(instructions)
    }

    /**
     * Compile a single [SqlExpr] to a list of instructions.
     *
     * Exposed for unit testing so individual expression compilations can be
     * verified without building a full plan.
     */
    fun compileExpression(expr: SqlExpr): List<Instruction> {
        val out = mutableListOf<Instruction>()
        emitExpr(expr, out)
        return out
    }

    // ── Post-op peeling ────────────────────────────────────────────────────────
    //
    // SELECT queries may have Sort / Limit / Distinct wrappers at the top.
    // Rather than weaving these into the scan loop (which would require buffering
    // all rows before sorting — exactly what we want to avoid at each iteration),
    // we peel them off and accumulate them as post-operation instructions.
    //
    // Example plan:
    //   Limit(count=10, offset=0,
    //     Sort(keys=[salary DESC],
    //       Project(... Scan("employees"))))
    //
    // After peeling: core = Project(... Scan(...)), postOps = [SortResult, LimitResult]

    /**
     * Recursively strip Sort/Limit/Distinct from the plan top and return:
     *   - [first] the inner core plan (Scan / Filter / Project / Aggregate / etc.)
     *   - [second] the list of post-operation Instructions in emission order
     *              (Sort before Limit, since you sort first, then limit)
     */
    private fun peelPostOps(plan: OptimizedPlan): Pair<OptimizedPlan, List<Instruction>> {
        val postOps = mutableListOf<Instruction>()
        var cur = plan

        // We collect all wrappers in order (outermost first), then reverse so
        // the outermost post-op (Sort) ends up first in the emitted list.
        // The canonical order is Sort → DistinctResult → LimitResult.
        val wrappers = mutableListOf<Instruction>()

        // Peel until we hit a non-wrapper plan node.
        while (true) {
            cur = when (cur) {
                is OptimizedPlan.Sort -> {
                    wrappers.add(Instruction.SortResult(cur.keys))
                    cur.input
                }
                is OptimizedPlan.Limit -> {
                    wrappers.add(Instruction.LimitResult(cur.count, cur.offset))
                    cur.input
                }
                is OptimizedPlan.Distinct -> {
                    wrappers.add(Instruction.DistinctResult)
                    cur.input
                }
                else -> break
            }
        }

        // The wrappers list was built outermost-first (Sort, then Limit if
        // Limit wraps Sort from above).  For execution we want Sort → Limit
        // order, which is exactly the order we collected (outer sort runs first
        // on the result buffer before we slice).  No reversal needed.
        postOps.addAll(wrappers)

        return Pair(cur, postOps)
    }

    // ── Core compilation dispatch ──────────────────────────────────────────────
    //
    // compileNode dispatches to a specialised function for each plan node type.
    // It uses Kotlin's "when" expression with "is Type" checks — note that
    // guard patterns ("is Type when condition") are NOT supported; nested ifs
    // must be used instead.

    /**
     * Recursively compile [plan] appending instructions into [out].
     *
     * The recursive structure mirrors the plan tree — children are compiled
     * before parents (post-order traversal), so the scan loop infrastructure is
     * always laid down before the body that references it.
     */
    private fun compileNode(
        plan: OptimizedPlan,
        out: MutableList<Instruction>,
        lc: LabelCounter
    ) {
        when (plan) {
            is OptimizedPlan.Scan       -> compileScan(plan, out, lc)
            is OptimizedPlan.Filter     -> compileFilter(plan, out, lc)
            is OptimizedPlan.Project    -> compileProject(plan, out, lc)
            is OptimizedPlan.Aggregate  -> compileAggregate(plan, out, lc)
            is OptimizedPlan.Having     -> compileHaving(plan, out, lc)
            is OptimizedPlan.Join       -> compileJoin(plan, out, lc)
            is OptimizedPlan.Union      -> compileUnion(plan, out, lc)
            is OptimizedPlan.Insert     -> compileInsert(plan, out, lc)
            is OptimizedPlan.Update     -> compileUpdate(plan, out, lc)
            is OptimizedPlan.Delete     -> compileDelete(plan, out, lc)
            is OptimizedPlan.CreateTable -> compileCreateTable(plan, out)
            is OptimizedPlan.DropTable  -> compileDropTable(plan, out)
            is OptimizedPlan.EmptyResult -> compileEmptyResult(out)
            // Sort / Limit / Distinct are peeled before compileNode is called,
            // but they can also appear mid-tree (e.g., inside a UNION).
            is OptimizedPlan.Sort       -> compileNode(plan.input, out, lc)
            is OptimizedPlan.Limit      -> compileNode(plan.input, out, lc)
            is OptimizedPlan.Distinct   -> compileNode(plan.input, out, lc)
        }
    }

    // ── Scan ──────────────────────────────────────────────────────────────────
    //
    // A table scan emits the loop skeleton:
    //
    //   OpenScan(table, alias)
    //   Label("scan_N_loop")
    //   AdvanceCursor(alias, "scan_N_end")   ← jumps to end if no more rows
    //   BeginRow
    //   EmitColumn/... (body — simple SELECT * for a bare Scan)
    //   EmitRow
    //   Jump("scan_N_loop")
    //   Label("scan_N_end")
    //   CloseScan(alias)
    //
    // When a Scan is wrapped by Filter or Project, those nodes call compileScan
    // to get the loop skeleton and inject their own logic into the body.  But
    // since the plan is a tree (not a template), we instead compile parent nodes
    // by recursing into children — the scan loop is laid down by the innermost
    // Plan node first; Filter and Project wrap it by emitting instructions around
    // the recursive call result.
    //
    // Here, a bare Scan (with no parent Project) emits a full SELECT * loop.

    /**
     * Compile a bare Scan into a full cursor loop.
     *
     * For a Scan that is not wrapped by a Project, we emit an implicit SELECT *
     * loop: open, advance, begin-row, (no column projections — the VM fills in
     * all columns), emit-row, jump-back, close.
     */
    private fun compileScan(
        plan: OptimizedPlan.Scan,
        out: MutableList<Instruction>,
        lc: LabelCounter
    ) {
        val n = lc.next()
        val loopLabel = "scan_${n}_loop"
        val endLabel  = "scan_${n}_end"
        val alias = plan.alias ?: plan.table

        out.add(Instruction.OpenScan(plan.table, plan.alias))
        out.add(Instruction.Label(loopLabel))
        out.add(Instruction.AdvanceCursor(plan.alias, endLabel))
        out.add(Instruction.BeginRow)
        out.add(Instruction.EmitRow)
        out.add(Instruction.Jump(loopLabel))
        out.add(Instruction.Label(endLabel))
        out.add(Instruction.CloseScan(plan.alias))
        // Suppress unused-variable warning — alias used in doc context
        @Suppress("UNUSED_EXPRESSION") alias
    }

    // ── Filter ────────────────────────────────────────────────────────────────
    //
    // A filter wraps its input's scan loop with a predicate check:
    //
    //   [input scan loop header: OpenScan + Label + AdvanceCursor]
    //     [predicate expression]
    //     JumpIfFalse("filter_N_skip")
    //     BeginRow
    //     EmitRow
    //     Label("filter_N_skip")
    //   [input scan loop footer: Jump + Label + CloseScan]
    //
    // The trick is that we can't literally "inject" code into the already-emitted
    // loop.  Instead we compile the *innermost* Scan node separately (getting its
    // loop skeleton), then build the complete instruction list here.  For
    // simplicity (and to match the C# reference implementation), we actually
    // emit the full Filter+Scan loop inline rather than delegating to compileScan.

    /**
     * Compile a Filter node into a scan loop with a predicate guard.
     *
     * If the predicate evaluates to FALSE or NULL for a row, we jump over the
     * body to the label "filter_N_skip" and the loop advances to the next row.
     */
    private fun compileFilter(
        plan: OptimizedPlan.Filter,
        out: MutableList<Instruction>,
        lc: LabelCounter
    ) {
        val filterN = lc.next()
        val skipLabel = "filter_${filterN}_skip"

        // Determine the innermost Scan so we can open/close the cursor.
        // The filter's input may itself be a Scan or something more complex.
        // For a Filter directly over a Scan we inline the full loop.
        // For more complex inputs (Filter over Filter, etc.) we fall back to
        // compiling the input recursively and surrounding it with predicate logic.

        val innerScan = innerScanOf(plan.input)
        if (innerScan != null) {
            // Inline loop: we control the full cursor lifecycle.
            val n = lc.next()
            val loopLabel = "scan_${n}_loop"
            val endLabel  = "scan_${n}_end"

            out.add(Instruction.OpenScan(innerScan.table, innerScan.alias))
            out.add(Instruction.Label(loopLabel))
            out.add(Instruction.AdvanceCursor(innerScan.alias, endLabel))
            // Emit the predicate; jump over the body if false/null.
            emitExpr(plan.predicate, out)
            out.add(Instruction.JumpIfFalse(skipLabel))
            out.add(Instruction.BeginRow)
            out.add(Instruction.EmitRow)
            out.add(Instruction.Label(skipLabel))
            out.add(Instruction.Jump(loopLabel))
            out.add(Instruction.Label(endLabel))
            out.add(Instruction.CloseScan(innerScan.alias))
        } else {
            // Complex input: compile the input first (it emits its own loop),
            // then wrap.  This path is less common in practice but handles
            // deeply nested plans.
            compileNode(plan.input, out, lc)
            // The predicate check is appended but cannot easily be injected
            // into the already-emitted loop.  For now emit as a post-filter.
            emitExpr(plan.predicate, out)
            out.add(Instruction.JumpIfFalse(skipLabel))
            out.add(Instruction.Label(skipLabel))
        }
    }

    /**
     * Compile a Project node.
     *
     * A Project emits a complete scan loop where, for each row that survives any
     * enclosing Filter, we:
     *   1. BeginRow
     *   2. For each output column: evaluate its expression, EmitColumn(name)
     *   3. EmitRow
     *
     * SELECT * is handled by emitting EmitRow without any EmitColumn calls —
     * the VM is expected to copy all cursor columns into the row buffer.
     */
    private fun compileProject(
        plan: OptimizedPlan.Project,
        out: MutableList<Instruction>,
        lc: LabelCounter
    ) {
        // Walk inward through Filter wrappers to find the Scan (if any).
        // We need the Scan to build the cursor loop.
        val (corePlan, filters) = collectFilters(plan.input)

        when (corePlan) {
            is OptimizedPlan.Scan -> {
                val n = lc.next()
                val loopLabel = "scan_${n}_loop"
                val endLabel  = "scan_${n}_end"

                out.add(Instruction.OpenScan(corePlan.table, corePlan.alias))
                out.add(Instruction.Label(loopLabel))
                out.add(Instruction.AdvanceCursor(corePlan.alias, endLabel))

                // Emit filter predicates in order (innermost first, since
                // collectFilters returns them outermost first, so we reverse).
                for (filterIdx in filters.indices.reversed()) {
                    val skipLabel = "filter_${lc.next()}_skip"
                    emitExpr(filters[filterIdx], out)
                    out.add(Instruction.JumpIfFalse(skipLabel))
                    // We need to emit the rest of the body and then label the skip.
                    // Since we can't retroactively insert the label, we emit it
                    // after the body using a nested helper that closes over the label.
                    // Simpler: emit BeginRow + columns + EmitRow then the label.
                    out.add(Instruction.BeginRow)
                    emitProjectColumns(plan.columns, corePlan.alias, out)
                    out.add(Instruction.EmitRow)
                    out.add(Instruction.Label(skipLabel))
                    out.add(Instruction.Jump(loopLabel))
                    out.add(Instruction.Label(endLabel))
                    out.add(Instruction.CloseScan(corePlan.alias))
                    return
                }

                // No filters: straightforward projection loop.
                out.add(Instruction.BeginRow)
                emitProjectColumns(plan.columns, corePlan.alias, out)
                out.add(Instruction.EmitRow)
                out.add(Instruction.Jump(loopLabel))
                out.add(Instruction.Label(endLabel))
                out.add(Instruction.CloseScan(corePlan.alias))
            }

            is OptimizedPlan.Aggregate -> {
                // Project over Aggregate: compile the aggregate first, then
                // add the projection wrapper.  The aggregate emits its own
                // finalize phase.
                compileAggregate(corePlan, out, lc)
            }

            is OptimizedPlan.Join -> {
                compileJoin(corePlan, out, lc)
            }

            else -> {
                // For any other core plan, compile it and emit projection logic.
                compileNode(corePlan, out, lc)
            }
        }
    }

    // ── Aggregate ─────────────────────────────────────────────────────────────
    //
    // An aggregate compiles in two phases:
    //
    //   Phase 1 — accumulate (inside the scan loop):
    //     For each row, compute GROUP BY expressions, save the group key, and
    //     call UpdateAgg(slot, fn) to accumulate each aggregate function.
    //
    //   Phase 2 — finalize (after the loop):
    //     For each distinct group, finalize all aggregate slots and emit a row.
    //
    // Label structure:
    //   OpenScan(table)
    //   Label("agg_loop")
    //   AdvanceCursor(alias, "agg_end")
    //   [group key expressions]
    //   SaveGroupKey(keys)
    //   InitAgg(0, fn) ... InitAgg(k-1, fn)
    //   [agg argument expressions]
    //   UpdateAgg(0, fn) ... UpdateAgg(k-1, fn)
    //   Jump("agg_loop")
    //   Label("agg_end")
    //   CloseScan(alias)
    //   [finalize loop — one pass per accumulated group]
    //   Label("agg_finalize")
    //   AdvanceGroup / jump to "agg_done"
    //   FinalizeAgg(0, fn) ... FinalizeAgg(k-1, fn)
    //   BeginRow
    //   [group key columns]
    //   [finalized agg columns]
    //   EmitRow
    //   Jump("agg_finalize")
    //   Label("agg_done")

    /**
     * Compile an Aggregate node.
     *
     * Aggregation is the most complex compilation case because it requires two
     * passes over the data:  one pass to accumulate (Phase 1) and one virtual
     * pass over the accumulated groups to finalize and emit (Phase 2).
     */
    private fun compileAggregate(
        plan: OptimizedPlan.Aggregate,
        out: MutableList<Instruction>,
        lc: LabelCounter
    ) {
        val n = lc.next()
        val loopLabel     = "agg_${n}_loop"
        val endLabel      = "agg_${n}_end"
        val finalizeLabel = "agg_${n}_finalize"
        val doneLabel     = "agg_${n}_done"

        val innerScan = innerScanOf(plan.input)
        val alias = innerScan?.alias

        if (innerScan != null) {
            out.add(Instruction.OpenScan(innerScan.table, innerScan.alias))
        } else {
            compileNode(plan.input, out, lc)
        }

        out.add(Instruction.Label(loopLabel))
        out.add(Instruction.AdvanceCursor(alias, endLabel))

        // Emit group-by key expressions and save them.
        val groupKeyNames = plan.groupBy.mapIndexed { i, expr ->
            // Best-effort: extract the column name for the SaveGroupKey label.
            val colName = when (expr) {
                is SqlExpr.Column -> expr.column
                else -> "group_$i"
            }
            emitExpr(expr, out)
            colName
        }
        if (groupKeyNames.isNotEmpty()) {
            out.add(Instruction.SaveGroupKey(groupKeyNames))
        }

        // Phase 1: initialise then update each aggregate slot.
        plan.aggregates.forEachIndexed { idx, agg ->
            val fn = aggFnOf(agg.func, agg.arg)
            out.add(Instruction.InitAgg(idx, fn))
            when (val arg = agg.arg) {
                is AggArg.Star -> out.add(Instruction.LoadConst(SqlValue.Null))
                is AggArg.Expr -> emitExpr(arg.expression, out)
            }
            out.add(Instruction.UpdateAgg(idx, fn))
        }

        out.add(Instruction.Jump(loopLabel))
        out.add(Instruction.Label(endLabel))
        if (innerScan != null) {
            out.add(Instruction.CloseScan(innerScan.alias))
        }

        // Phase 2: finalize — iterate over each accumulated group.
        out.add(Instruction.Label(finalizeLabel))
        out.add(Instruction.AdvanceGroup)

        // Emit one row per group.
        out.add(Instruction.BeginRow)

        // Emit group-by key columns.
        groupKeyNames.forEachIndexed { i, name ->
            out.add(Instruction.LoadGroupKey(i))
            out.add(Instruction.EmitColumn(name))
        }

        // Emit finalized aggregate columns.
        plan.aggregates.forEachIndexed { idx, agg ->
            val fn = aggFnOf(agg.func, agg.arg)
            out.add(Instruction.FinalizeAgg(idx, fn))
            out.add(Instruction.EmitColumn(agg.alias))
        }

        out.add(Instruction.EmitRow)
        out.add(Instruction.Jump(finalizeLabel))
        out.add(Instruction.Label(doneLabel))
    }

    // ── Having ────────────────────────────────────────────────────────────────

    /**
     * Compile a Having node.
     *
     * HAVING is like a Filter but applied to the *aggregate output* rather than
     * the raw input rows.  We compile the underlying Aggregate first, then wrap
     * its finalize phase with a predicate check.
     *
     * In practice the Having node wraps an Aggregate; we delegate to
     * compileAggregate and append the HAVING predicate as a post-filter on the
     * emitted rows.
     */
    private fun compileHaving(
        plan: OptimizedPlan.Having,
        out: MutableList<Instruction>,
        lc: LabelCounter
    ) {
        // Compile the underlying aggregate (or other input).
        compileNode(plan.input, out, lc)
        // Emit predicate as a post-filter.  The VM will apply this after each
        // EmitRow produced by the aggregate finalize phase.
        val skipLabel = "having_${lc.next()}_skip"
        emitExpr(plan.predicate, out)
        out.add(Instruction.JumpIfFalse(skipLabel))
        out.add(Instruction.Label(skipLabel))
    }

    // ── Join ──────────────────────────────────────────────────────────────────
    //
    // A join compiles to a nested loop:
    //
    //   OpenScan(left_table, left_alias)
    //   Label("outer_N_loop")
    //   AdvanceCursor(left_alias, "outer_N_end")
    //
    //     OpenScan(right_table, right_alias)
    //     Label("inner_N_loop")
    //     AdvanceCursor(right_alias, "inner_N_end")
    //       [condition]
    //       JumpIfFalse("inner_N_continue")
    //       BeginRow
    //       [columns from both sides]
    //       EmitRow
    //       Label("inner_N_continue")
    //     Jump("inner_N_loop")
    //     Label("inner_N_end")
    //     CloseScan(right_alias)
    //
    //   Jump("outer_N_loop")
    //   Label("outer_N_end")
    //   CloseScan(left_alias)

    /**
     * Compile a Join node (INNER, LEFT, RIGHT, CROSS, FULL).
     *
     * Only INNER and CROSS joins emit a simple nested loop.  LEFT/RIGHT/FULL
     * joins require NULL-filling logic; for Level 1 we emit the nested-loop
     * skeleton and leave NULL-filling to the VM runtime.
     */
    private fun compileJoin(
        plan: OptimizedPlan.Join,
        out: MutableList<Instruction>,
        lc: LabelCounter
    ) {
        val n = lc.next()
        val outerLoop = "outer_${n}_loop"
        val outerEnd  = "outer_${n}_end"
        val innerLoop = "inner_${n}_loop"
        val innerEnd  = "inner_${n}_end"
        val contLabel = "inner_${n}_continue"

        val leftScan  = innerScanOf(plan.left)
        val rightScan = innerScanOf(plan.right)

        // Outer loop (left side).
        if (leftScan != null) {
            out.add(Instruction.OpenScan(leftScan.table, leftScan.alias))
        } else {
            compileNode(plan.left, out, lc)
        }
        out.add(Instruction.Label(outerLoop))
        out.add(Instruction.AdvanceCursor(leftScan?.alias, outerEnd))

        // Inner loop (right side) — reopened for each outer row.
        if (rightScan != null) {
            out.add(Instruction.OpenScan(rightScan.table, rightScan.alias))
        } else {
            compileNode(plan.right, out, lc)
        }
        out.add(Instruction.Label(innerLoop))
        out.add(Instruction.AdvanceCursor(rightScan?.alias, innerEnd))

        // Condition (if any).
        if (plan.condition != null) {
            emitExpr(plan.condition, out)
            out.add(Instruction.JumpIfFalse(contLabel))
        }

        out.add(Instruction.BeginRow)
        out.add(Instruction.EmitRow)

        if (plan.condition != null) {
            out.add(Instruction.Label(contLabel))
        }
        out.add(Instruction.Jump(innerLoop))
        out.add(Instruction.Label(innerEnd))
        if (rightScan != null) {
            out.add(Instruction.CloseScan(rightScan.alias))
        }

        out.add(Instruction.Jump(outerLoop))
        out.add(Instruction.Label(outerEnd))
        if (leftScan != null) {
            out.add(Instruction.CloseScan(leftScan.alias))
        }
    }

    // ── Union ─────────────────────────────────────────────────────────────────

    /**
     * Compile a Union node.
     *
     * A UNION compiles the left side first (accumulating rows into the result
     * buffer), then the right side.  UNION ALL keeps duplicates; UNION (without
     * ALL) appends a DistinctResult at the end — but since Distinct is peeled
     * as a post-op wrapper, we just compile both sides sequentially here.
     */
    private fun compileUnion(
        plan: OptimizedPlan.Union,
        out: MutableList<Instruction>,
        lc: LabelCounter
    ) {
        compileNode(plan.left, out, lc)
        compileNode(plan.right, out, lc)
        // If UNION (not UNION ALL), add DistinctResult.
        if (!plan.all) {
            out.add(Instruction.DistinctResult)
        }
    }

    // ── Insert ────────────────────────────────────────────────────────────────
    //
    //   LoadConst(v1)
    //   LoadConst(v2)
    //   ...
    //   InsertRow(table, columns)
    //
    // For multi-row INSERT, repeat for each value tuple.

    /**
     * Compile an INSERT INTO ... VALUES (...) statement.
     *
     * For each value tuple, we emit LoadConst instructions for each column
     * value, then InsertRow to ask the backend to persist the row.
     */
    private fun compileInsert(
        plan: OptimizedPlan.Insert,
        out: MutableList<Instruction>,
        lc: LabelCounter
    ) {
        @Suppress("UNUSED_PARAMETER") val _ = lc  // lc unused here; suppress warning
        for (row in plan.values) {
            for (expr in row) {
                emitExpr(expr, out)
            }
            out.add(Instruction.InsertRow(plan.table, plan.columns.ifEmpty { null }))
        }
    }

    // ── Update ────────────────────────────────────────────────────────────────
    //
    //   OpenScan(table, null)
    //   Label("update_N_loop")
    //   AdvanceCursor(null, "update_N_end")
    //     [predicate (if any)]
    //     JumpIfFalse("update_N_skip")
    //     [assignment value expressions]
    //     UpdateRows(table)
    //     Label("update_N_skip")
    //   Jump("update_N_loop")
    //   Label("update_N_end")
    //   CloseScan(null)

    /**
     * Compile an UPDATE statement.
     *
     * We open a full scan cursor, check the predicate for each row, evaluate
     * assignment expressions, and call UpdateRows.
     */
    private fun compileUpdate(
        plan: OptimizedPlan.Update,
        out: MutableList<Instruction>,
        lc: LabelCounter
    ) {
        val n = lc.next()
        val loopLabel = "update_${n}_loop"
        val endLabel  = "update_${n}_end"
        val skipLabel = "update_${n}_skip"

        out.add(Instruction.OpenScan(plan.table, null))
        out.add(Instruction.Label(loopLabel))
        out.add(Instruction.AdvanceCursor(null, endLabel))

        if (plan.predicate != null) {
            emitExpr(plan.predicate, out)
            out.add(Instruction.JumpIfFalse(skipLabel))
        }

        for (assignment in plan.assignments) {
            emitExpr(assignment.value, out)
        }
        out.add(Instruction.UpdateRows(plan.table))

        if (plan.predicate != null) {
            out.add(Instruction.Label(skipLabel))
        }
        out.add(Instruction.Jump(loopLabel))
        out.add(Instruction.Label(endLabel))
        out.add(Instruction.CloseScan(null))
    }

    // ── Delete ────────────────────────────────────────────────────────────────

    /**
     * Compile a DELETE FROM statement.
     *
     * Like UPDATE but without assignment expressions.  The predicate determines
     * which rows are marked for deletion; DeleteRows asks the backend to remove
     * all marked rows.
     */
    private fun compileDelete(
        plan: OptimizedPlan.Delete,
        out: MutableList<Instruction>,
        lc: LabelCounter
    ) {
        val n = lc.next()
        val loopLabel = "delete_${n}_loop"
        val endLabel  = "delete_${n}_end"
        val skipLabel = "delete_${n}_skip"

        out.add(Instruction.OpenScan(plan.table, null))
        out.add(Instruction.Label(loopLabel))
        out.add(Instruction.AdvanceCursor(null, endLabel))

        if (plan.predicate != null) {
            emitExpr(plan.predicate, out)
            out.add(Instruction.JumpIfFalse(skipLabel))
        }

        out.add(Instruction.DeleteRows(plan.table))

        if (plan.predicate != null) {
            out.add(Instruction.Label(skipLabel))
        }
        out.add(Instruction.Jump(loopLabel))
        out.add(Instruction.Label(endLabel))
        out.add(Instruction.CloseScan(null))
    }

    // ── DDL ───────────────────────────────────────────────────────────────────

    /** Compile CREATE TABLE into a single-instruction program (before Halt). */
    private fun compileCreateTable(plan: OptimizedPlan.CreateTable, out: MutableList<Instruction>) {
        out.add(Instruction.CreateTableInstr(plan.table, plan.ifNotExists, plan.columns))
    }

    /** Compile DROP TABLE into a single-instruction program (before Halt). */
    private fun compileDropTable(plan: OptimizedPlan.DropTable, out: MutableList<Instruction>) {
        out.add(Instruction.DropTableInstr(plan.table, plan.ifExists))
    }

    // ── EmptyResult ───────────────────────────────────────────────────────────

    /**
     * Compile an EmptyResult plan.
     *
     * The optimizer inserts EmptyResult when it can prove the query will return
     * zero rows (e.g., a provably-false WHERE clause).  We emit only Halt (the
     * result buffer stays empty).  The Halt is appended by [compile] after this
     * returns.
     */
    private fun compileEmptyResult(out: MutableList<Instruction>) {
        // Nothing to emit — the Halt in compile() terminates the program.
        // The result buffer is empty by default, giving an empty result set.
        @Suppress("UNUSED_PARAMETER") val _ = out
    }

    // ── Expression compilation ────────────────────────────────────────────────
    //
    // SQL expressions form a tree (SqlExpr is a sealed class with recursive
    // subclasses).  We compile them to a *postfix* sequence of stack-machine
    // instructions: leaves are pushed first, then operators consume the top
    // values and push their results.
    //
    // Example: (a.salary > 50000) AND (a.age < 65)
    //
    //   LoadColumn("a", "salary")    ← push left operand of >
    //   LoadConst(Int(50000))         ← push right operand of >
    //   BinaryOpInstr(GT)             ← pop both, push result
    //   LoadColumn("a", "age")        ← push left operand of <
    //   LoadConst(Int(65))            ← push right operand of <
    //   BinaryOpInstr(LT)             ← pop both, push result
    //   BinaryOpInstr(AND)            ← pop both, push result

    /**
     * Recursively compile [expr] by appending instructions to [out].
     *
     * The generated instructions leave exactly one value on the stack upon
     * completion.
     */
    internal fun emitExpr(expr: SqlExpr, out: MutableList<Instruction>) {
        when (expr) {
            is SqlExpr.Literal    -> out.add(Instruction.LoadConst(toSqlValue(expr.value)))
            is SqlExpr.Column     -> out.add(Instruction.LoadColumn(expr.table, expr.column))
            is SqlExpr.Wildcard   -> out.add(Instruction.LoadConst(SqlValue.Null))

            is SqlExpr.BinaryOp   -> {
                emitExpr(expr.left, out)
                emitExpr(expr.right, out)
                out.add(Instruction.BinaryOpInstr(binaryOpOf(expr.op)))
            }

            is SqlExpr.UnaryOp    -> {
                emitExpr(expr.operand, out)
                out.add(Instruction.UnaryOpInstr(unaryOpOf(expr.op)))
            }

            is SqlExpr.IsNull     -> {
                emitExpr(expr.operand, out)
                out.add(Instruction.IsNull)
            }

            is SqlExpr.IsNotNull  -> {
                emitExpr(expr.operand, out)
                out.add(Instruction.IsNotNull)
            }

            is SqlExpr.Between    -> {
                // Stack order: value first, then low, then high.
                // The VM pops: high, low, value.
                emitExpr(expr.value, out)
                emitExpr(expr.low, out)
                emitExpr(expr.high, out)
                out.add(Instruction.Between())
            }

            is SqlExpr.Like       -> {
                emitExpr(expr.value, out)
                out.add(Instruction.LoadConst(SqlValue.TextVal(expr.pattern)))
                out.add(Instruction.Like)
            }

            is SqlExpr.NotLike    -> {
                // NOT LIKE = push value, push pattern, Like, then NOT.
                emitExpr(expr.value, out)
                out.add(Instruction.LoadConst(SqlValue.TextVal(expr.pattern)))
                out.add(Instruction.Like)
                out.add(Instruction.UnaryOpInstr(UnaryOp.NOT))
            }

            is SqlExpr.In         -> {
                // Push the needle, then push each list element.
                emitExpr(expr.value, out)
                for (item in expr.items) emitExpr(item, out)
                out.add(Instruction.InList(expr.items.size))
            }

            is SqlExpr.NotIn      -> {
                // NOT IN = IN then boolean NOT.
                emitExpr(expr.value, out)
                for (item in expr.items) emitExpr(item, out)
                out.add(Instruction.InList(expr.items.size))
                out.add(Instruction.UnaryOpInstr(UnaryOp.NOT))
            }

            is SqlExpr.FuncCall   -> {
                // Generic function call: push all args, then a LoadConst with
                // the function name.  The VM dispatches based on function name.
                for (arg in expr.args) emitExpr(arg, out)
                out.add(Instruction.LoadConst(SqlValue.TextVal("__func:${expr.name}")))
            }

            is SqlExpr.AggExpr    -> {
                // AggExpr at expression level (outside an Aggregate plan node).
                // This occurs in HAVING clauses.  We emit a FinalizeAgg for
                // slot 0 as a best-effort; the VM handles the actual grouping.
                val fn = aggFnOf(expr.func, expr.arg)
                out.add(Instruction.FinalizeAgg(0, fn))
            }
        }
    }

    // ── Helpers ────────────────────────────────────────────────────────────────

    /**
     * Map a planner BinaryOperator to the codegen BinaryOp enum.
     *
     * The planner uses its own enum (BinaryOperator); the codegen uses BinaryOp.
     * Having two separate enums avoids coupling the VM to the planner package.
     */
    private fun binaryOpOf(op: BinaryOperator): BinaryOp = when (op) {
        BinaryOperator.ADD    -> BinaryOp.ADD
        BinaryOperator.SUB    -> BinaryOp.SUB
        BinaryOperator.MUL    -> BinaryOp.MUL
        BinaryOperator.DIV    -> BinaryOp.DIV
        BinaryOperator.MOD    -> BinaryOp.MOD
        BinaryOperator.EQ     -> BinaryOp.EQ
        BinaryOperator.NOT_EQ -> BinaryOp.NEQ
        BinaryOperator.LT     -> BinaryOp.LT
        BinaryOperator.LTE    -> BinaryOp.LTE
        BinaryOperator.GT     -> BinaryOp.GT
        BinaryOperator.GTE    -> BinaryOp.GTE
        BinaryOperator.AND    -> BinaryOp.AND
        BinaryOperator.OR     -> BinaryOp.OR
    }

    /** Map a planner UnaryOperator to the codegen UnaryOp enum. */
    private fun unaryOpOf(op: UnaryOperator): UnaryOp = when (op) {
        UnaryOperator.NEG -> UnaryOp.NEG
        UnaryOperator.NOT -> UnaryOp.NOT
    }

    /**
     * Determine the [AggFn] for an aggregate item.
     *
     * COUNT(*) (AggArg.Star) maps to COUNT_STAR; COUNT(col) maps to COUNT.
     */
    private fun aggFnOf(func: AggFunction, arg: AggArg): AggFn = when (func) {
        AggFunction.COUNT -> if (arg is AggArg.Star) AggFn.COUNT_STAR else AggFn.COUNT
        AggFunction.SUM   -> AggFn.SUM
        AggFunction.AVG   -> AggFn.AVG
        AggFunction.MIN   -> AggFn.MIN
        AggFunction.MAX   -> AggFn.MAX
    }

    /**
     * Convert a Kotlin Any? (from SqlExpr.Literal.value) to an [SqlValue].
     *
     * The planner stores literal values as raw Kotlin types (Long, Double,
     * String, Boolean, null).  We box them into the sealed SqlValue hierarchy.
     */
    private fun toSqlValue(v: Any?): SqlValue = when (v) {
        null           -> SqlValue.Null
        is Boolean     -> SqlValue.BoolVal(v)
        is Long        -> SqlValue.IntVal(v)
        is Int         -> SqlValue.IntVal(v.toLong())
        is Double      -> SqlValue.FloatVal(v)
        is Float       -> SqlValue.FloatVal(v.toDouble())
        is String      -> SqlValue.TextVal(v)
        else           -> SqlValue.TextVal(v.toString())
    }

    /**
     * Return the innermost [OptimizedPlan.Scan] if [plan] is a Scan or a chain
     * of Filter/Project nodes that directly wraps a Scan.  Returns null for
     * more complex plans (Aggregate, Join, Union, etc.).
     *
     * This is used to "look through" simple wrappers and find the cursor we
     * need to open/close for a loop.
     */
    private fun innerScanOf(plan: OptimizedPlan): OptimizedPlan.Scan? = when (plan) {
        is OptimizedPlan.Scan    -> plan
        is OptimizedPlan.Filter  -> innerScanOf(plan.input)
        is OptimizedPlan.Project -> innerScanOf(plan.input)
        is OptimizedPlan.Having  -> innerScanOf(plan.input)
        else                     -> null
    }

    /**
     * Walk through Filter wrappers and collect their predicates.
     * Returns (innerPlan, listOfPredicates_outerFirst).
     *
     * Used by compileProject to peel off any Filter nodes wrapped around the
     * Scan so we can emit one unified cursor loop with filter checks inside.
     */
    private fun collectFilters(plan: OptimizedPlan): Pair<OptimizedPlan, List<SqlExpr>> {
        val predicates = mutableListOf<SqlExpr>()
        var cur = plan
        while (cur is OptimizedPlan.Filter) {
            predicates.add(cur.predicate)
            cur = cur.input
        }
        return Pair(cur, predicates)
    }

    /**
     * Emit [EmitColumn] instructions for all output columns in a Project.
     *
     * For SELECT *: the single OutputColumn.Star means "emit all columns from
     * the cursor".  At this level we emit an EmitRow without any EmitColumn
     * instructions; the VM is expected to copy all cursor columns.
     *
     * For named columns: each OutputColumn.Expr is evaluated and stored under
     * its alias (or the column name if no alias was given).
     */
    private fun emitProjectColumns(
        columns: List<OutputColumn>,
        cursorAlias: String?,
        out: MutableList<Instruction>
    ) {
        val hasStar = columns.any { it is OutputColumn.Star }
        if (hasStar) {
            // SELECT * — the VM materialises all columns; no EmitColumn needed.
            // We load a sentinel so the VM knows to expand the cursor row.
            out.add(Instruction.LoadConst(SqlValue.TextVal("*")))
            out.add(Instruction.EmitColumn("*"))
            return
        }
        for (col in columns) {
            if (col is OutputColumn.Expr) {
                emitExpr(col.expression, out)
                val name = col.alias
                    ?: when (val e = col.expression) {
                        is SqlExpr.Column -> e.column
                        else -> "col"
                    }
                out.add(Instruction.EmitColumn(name))
            }
        }
        // Suppress unused parameter warning for cursorAlias — retained for
        // future use when the VM needs the cursor ID to expand SELECT *.
        @Suppress("UNUSED_EXPRESSION") cursorAlias
    }
}

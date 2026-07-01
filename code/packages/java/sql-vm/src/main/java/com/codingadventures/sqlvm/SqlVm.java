package com.codingadventures.sqlvm;

// SqlVm.java — stack-machine bytecode interpreter for the SQL VM.
//
// This file is the heart of the sql-vm package.  It executes the flat
// list of Instructions produced by SqlCodegen against a Backend, returning
// a QueryResult.
//
// Architecture overview
// ─────────────────────
//
//   Program (instructions + label map + result schema)
//      │
//      ▼
//   SqlVm.execute(program, backend)
//      │
//      ▼  dispatch loop: while pc < n { dispatch(instructions[pc++]) }
//      │
//      ├─ Stack ops:   LoadConst / LoadColumn / BinaryOp / UnaryOp / IsNull / …
//      ├─ Scan ops:    OpenScan → AdvanceCursor (loop) → CloseScan
//      ├─ Row ops:     BeginRow / EmitColumn / EmitRow
//      ├─ Agg ops:     InitAgg / UpdateAgg / FinalizeAgg
//      ├─ Post ops:    SortResult / LimitResult / DistinctResult
//      ├─ DML ops:     InsertRow / UpdateRows / DeleteRows
//      ├─ DDL ops:     CreateTable / DropTable
//      └─ Control:     Label (no-op) / Jump / JumpIfFalse / JumpIfTrue / Halt
//
// Three-valued logic (SQL NULL semantics)
// ──────────────────────────────────────
// Every value on the stack is `Object`, where `null` represents SQL NULL.
//   • Arithmetic with NULL → NULL
//   • NULL AND FALSE → FALSE  (short-circuit)
//   • NULL OR  TRUE  → TRUE   (short-circuit)
//   • IS NULL(null) → true;  IS NOT NULL(null) → false
//
// Aggregates
// ──────────
// Each aggregate slot is indexed from 0 (matching InitAgg/UpdateAgg/FinalizeAgg).
// A "group key" — the tuple of GROUP BY expression values — partitions rows.
// The agg table maps group-key tuples to lists of AggAccum objects.

import com.codingadventures.sqlbackend.SqlBackend;
import com.codingadventures.sqlbackend.SqlBackend.Backend;
import com.codingadventures.sqlbackend.SqlBackend.Cursor;
import com.codingadventures.sqlbackend.SqlBackend.Row;
import com.codingadventures.sqlbackend.SqlBackend.RowIterator;
import com.codingadventures.sqlcodegen.SqlCodegen;
import com.codingadventures.sqlcodegen.SqlCodegen.AggFunc;
import com.codingadventures.sqlcodegen.SqlCodegen.BinaryOpCode;
import com.codingadventures.sqlcodegen.SqlCodegen.Direction;
import com.codingadventures.sqlcodegen.SqlCodegen.Instruction;
import com.codingadventures.sqlcodegen.SqlCodegen.NullsOrder;
import com.codingadventures.sqlcodegen.SqlCodegen.Program;
import com.codingadventures.sqlcodegen.SqlCodegen.SortKey;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.HashMap;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.LinkedList;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Set;
import java.util.regex.Pattern;
import java.util.stream.Collectors;

/**
 * Stack-machine VM that executes bytecode Programs produced by SqlCodegen.
 *
 * <p>Usage:
 * <pre>{@code
 *   Program prog = SqlCodegen.compile(logicalPlan);
 *   QueryResult result = SqlVm.execute(prog, backend);
 *   System.out.println(result.columns());
 *   result.rows().forEach(row -> System.out.println(row));
 * }</pre>
 *
 * <p>The VM is entirely stateless — a fresh VmState is created for each
 * {@code execute()} call, so the same Program can be executed concurrently
 * without any synchronization.
 */
public final class SqlVm {

    // Private constructor — this class is a static-method namespace only.
    private SqlVm() {}

    // ── QueryResult ───────────────────────────────────────────────────────────
    //
    // The output of every execute() call.
    //
    //   columns     — ordered list of output column names
    //   rows        — result rows; each row is a list of SQL values (may contain null)
    //   rowsAffected — for DML: how many rows were inserted/updated/deleted;
    //                  for SELECT: always 0

    /**
     * Result of a single SQL program execution.
     *
     * @param columns      output column names, in SELECT list order
     * @param rows         result rows; each inner list is one row, same width as columns
     * @param rowsAffected number of rows modified (0 for SELECT)
     */
    public record QueryResult(
        List<String> columns,
        List<List<Object>> rows,
        int rowsAffected
    ) {}

    // ── Public API ────────────────────────────────────────────────────────────

    /**
     * Execute {@code program} against {@code backend} and return the result.
     *
     * <p>This is the VM's single public entry point.  It creates a fresh
     * {@link VmState}, runs the dispatch loop, applies post-processing
     * (sort / limit / distinct), and packages the result.
     *
     * @param program compiled bytecode program from SqlCodegen
     * @param backend storage backend (e.g. InMemoryBackend)
     * @return the query result, never null
     */
    public static QueryResult execute(Program program, Backend backend) {
        VmState st = new VmState(program, backend);
        List<Instruction> instrs = program.instructions();
        int n = instrs.size();

        // Main dispatch loop: advance pc, dispatch the instruction.
        // Halt breaks the loop early; jumps rewrite pc.
        while (st.pc < n) {
            Instruction instr = instrs.get(st.pc);
            st.pc++;

            if (instr instanceof Instruction.Halt) {
                break;
            }
            dispatch(instr, st);
        }

        // Package and return the final result.
        return st.buildResult();
    }

    // ── VM state (private) ────────────────────────────────────────────────────
    //
    // All mutable per-execution state lives here.  The dispatch loop is a set
    // of static methods that mutate a VmState reference — this keeps the
    // "big dispatch switch" readable while avoiding global state.

    private static final class VmState {
        // ── Program being executed ────────────────────────────────────────
        final Program program;
        final Backend backend;

        // ── Instruction pointer ───────────────────────────────────────────
        int pc = 0;

        // ── Operand stack ─────────────────────────────────────────────────
        // Object here means "any SQL value": null (NULL), Boolean, Long,
        // Double, String, or byte[].
        // IMPORTANT: ArrayList (not ArrayDeque) because ArrayDeque.push() rejects
        // null — but SQL NULL is a valid stack value represented as Java null.
        // Top of stack = last element of the list.
        final List<Object> stack = new ArrayList<>();

        // ── Cursors ───────────────────────────────────────────────────────
        // cursorId → open iterator (RowIterator or Cursor for DML).
        final Map<Integer, RowIterator> cursors = new HashMap<>();
        // cursorId → the last row returned by that cursor (for LoadColumn).
        final Map<Integer, Map<String, Object>> currentRow = new HashMap<>();

        // ── Row assembly buffer ───────────────────────────────────────────
        // BeginRow clears this; EmitColumn appends; EmitRow drains it into outputRows.
        final List<Object> rowBuffer = new ArrayList<>();

        // ── Output result set ─────────────────────────────────────────────
        List<String> resultColumns = new ArrayList<>();
        final List<List<Object>> outputRows = new ArrayList<>();

        // ── Aggregate state ───────────────────────────────────────────────
        //
        // Aggregates are partitioned by the current group key (tuple of GROUP BY
        // expression values).  Each group maps to a list of AggAccum, one per slot.
        //
        // group_key:  the tuple of GROUP BY values for the *current input row*.
        // group_order: insertion-ordered list of group keys seen so far
        //              (preserves GROUP BY determinism).
        // aggTable:   group-key tuple → list of AggAccum (indexed by slot).
        // groupIter:  cursor into group_order during the emit phase.
        List<Object> groupKey = List.of();
        final List<List<Object>> groupOrder = new ArrayList<>();
        final Map<List<Object>, List<AggAccum>> aggTable = new LinkedHashMap<>();
        int groupIter = -1;

        // ── DML counter ───────────────────────────────────────────────────
        int rowsAffected = 0;

        // ── LEFT JOIN tracking ────────────────────────────────────────────
        // Each JoinBeginRow pushes false; JoinSetMatched sets top to true;
        // JoinIfMatched pops and conditionally jumps.
        // Boolean (join match tracking) is never null, so LinkedList is fine here.
        final LinkedList<Boolean> joinMatchStack = new LinkedList<>();

        VmState(Program program, Backend backend) {
            this.program = program;
            this.backend = backend;
        }

        // Push a SQL value onto the operand stack.
        // null represents SQL NULL and is a valid stack entry.
        void push(Object value) {
            stack.add(value);  // add to end (= top of stack)
        }

        // Pop the top SQL value from the operand stack.
        // Throws IllegalStateException on underflow (indicates a codegen bug).
        Object pop() {
            if (stack.isEmpty()) {
                throw new IllegalStateException("stack underflow");
            }
            return stack.remove(stack.size() - 1);  // remove from end (= top)
        }

        // Pop n values and return them in push order (oldest first).
        // An n of 0 returns an empty list without touching the stack.
        List<Object> popN(int n) {
            if (n == 0) return new ArrayList<>();
            if (stack.size() < n) {
                throw new IllegalStateException("stack underflow: need " + n + ", have " + stack.size());
            }
            // Stack top is at index (size-1).
            // We want [pushed-first, ..., pushed-last].
            // The last n elements of the list, in order, are exactly that.
            int from = stack.size() - n;
            List<Object> result = new ArrayList<>(stack.subList(from, stack.size()));
            stack.subList(from, stack.size()).clear();
            return result;
        }

        // Resolve a label name to an instruction index.
        int resolve(String label) {
            Integer idx = program.labels().get(label);
            if (idx == null) {
                throw new IllegalStateException("unknown label: " + label);
            }
            return idx;
        }

        // Determine SQL truthiness.
        // In SQL, NULL, false, 0, and 0.0 are all falsy.
        static boolean isTruthy(Object v) {
            if (v == null) return false;
            if (v instanceof Boolean b) return b;
            if (v instanceof Long l) return l != 0L;
            if (v instanceof Double d) return d != 0.0;
            if (v instanceof Integer i) return i != 0;
            // Strings and byte arrays: truthy when non-empty (uncommon in SQL but safe).
            return true;
        }

        // Package final result after the dispatch loop exits.
        QueryResult buildResult() {
            // The SetResultSchema instruction may have set resultColumns;
            // if so, use it.  Otherwise fall back to whatever we collected.
            List<String> cols = resultColumns.isEmpty()
                ? program.resultSchema()
                : new ArrayList<>(resultColumns);
            // Use ArrayList instead of List.copyOf because result rows may
            // contain SQL NULL (Java null), and List.copyOf disallows nulls.
            return new QueryResult(
                cols,
                outputRows.stream()
                    .map(ArrayList::new)
                    .collect(Collectors.toList()),
                rowsAffected
            );
        }
    }

    // ── Aggregate accumulator (per group, per slot) ────────────────────────

    private static final class AggAccum {
        final AggFunc func;
        boolean distinct;
        Set<Object> seen;  // populated only when distinct=true
        int count = 0;
        Object acc = null; // SUM / AVG running total / MIN / MAX extremum
        final List<Object> items = new ArrayList<>(); // GROUP_CONCAT list

        AggAccum(AggFunc func, boolean distinct) {
            this.func = func;
            this.distinct = distinct;
            if (distinct) this.seen = new HashSet<>();
        }
    }

    // ── Main dispatch ─────────────────────────────────────────────────────────

    /**
     * Dispatch a single instruction.
     *
     * <p>Uses Java 21 sealed-interface pattern matching (instanceof with record
     * destructuring) to handle every Instruction variant exhaustively.
     * Each branch is labelled with a short comment explaining the semantic.
     */
    @SuppressWarnings("unchecked")
    private static void dispatch(Instruction instr, VmState st) {

        // ── Stack / constant operations ───────────────────────────────────

        if (instr instanceof Instruction.LoadConst(var value)) {
            // Push a compile-time constant onto the operand stack.
            st.push(value);
            return;
        }

        if (instr instanceof Instruction.LoadColumn(var cursorId, var column)) {
            // Look up the named column in the cursor's current row.
            // If the cursor has no current row (e.g. unmatched LEFT JOIN side),
            // push NULL so the rest of the expression evaluates to NULL.
            Map<String, Object> row = st.currentRow.get(cursorId);
            st.push(row == null ? null : row.get(column));
            return;
        }

        if (instr instanceof Instruction.Pop()) {
            // Discard the top-of-stack value.  Emitted when an expression is
            // evaluated for side effects only (rarely used in SQL bytecode).
            st.pop();
            return;
        }

        // ── Arithmetic and comparison ─────────────────────────────────────

        if (instr instanceof Instruction.BinaryOp(var op)) {
            // Pop right operand first (it was pushed last), then left.
            Object right = st.pop();
            Object left = st.pop();
            st.push(applyBinary(op, left, right));
            return;
        }

        if (instr instanceof Instruction.UnaryOp(var op)) {
            st.push(applyUnary(op, st.pop()));
            return;
        }

        if (instr instanceof Instruction.IsNull()) {
            // IS NULL is immune to three-valued logic: it always returns a Boolean.
            st.push(st.pop() == null);
            return;
        }

        if (instr instanceof Instruction.IsNotNull()) {
            st.push(st.pop() != null);
            return;
        }

        if (instr instanceof Instruction.Between()) {
            doBetween(st);
            return;
        }

        if (instr instanceof Instruction.InList(var n)) {
            doInList(n, st);
            return;
        }

        if (instr instanceof Instruction.Like(var negated)) {
            doLike(negated, st);
            return;
        }

        if (instr instanceof Instruction.CallScalar(var func, var nArgs)) {
            doCallScalar(func, nArgs, st);
            return;
        }

        // ── Cursor / scan operations ──────────────────────────────────────

        if (instr instanceof Instruction.OpenScan(var cursorId, var table)) {
            // Open a positioned cursor (for DML support) if the backend provides one;
            // fall back to a plain RowIterator for read-only access.
            RowIterator it;
            if (st.backend instanceof SqlBackend.InMemoryBackend imb) {
                it = imb.openCursor(table);
            } else {
                it = st.backend.scan(table);
            }
            st.cursors.put(cursorId, it);
            return;
        }

        if (instr instanceof Instruction.AdvanceCursor(var cursorId, var onExhausted)) {
            RowIterator it = st.cursors.get(cursorId);
            if (it == null) {
                throw new IllegalStateException("advance of unknown cursor " + cursorId);
            }
            Row row = it.next();
            if (row == null) {
                // Cursor exhausted — remove stale current-row entry and jump.
                st.currentRow.remove(cursorId);
                st.pc = st.resolve(onExhausted);
            } else {
                // Save a copy of the row so LoadColumn can read it.
                st.currentRow.put(cursorId, new HashMap<>(row));
            }
            return;
        }

        if (instr instanceof Instruction.CloseScan(var cursorId)) {
            RowIterator it = st.cursors.remove(cursorId);
            if (it != null) it.close();
            st.currentRow.remove(cursorId);
            return;
        }

        // ── Row assembly operations ───────────────────────────────────────

        if (instr instanceof Instruction.BeginRow()) {
            // Start a fresh output row.  Any prior partial row is discarded.
            st.rowBuffer.clear();
            return;
        }

        if (instr instanceof Instruction.EmitColumn(var name)) {
            // Pop the top-of-stack value and store it under the column name.
            // The row buffer is positional; name ordering matches resultColumns.
            st.rowBuffer.add(st.pop());
            return;
        }

        if (instr instanceof Instruction.EmitRow()) {
            // Finalise the current row and append it to the result set.
            st.outputRows.add(new ArrayList<>(st.rowBuffer));
            st.rowBuffer.clear();
            return;
        }

        if (instr instanceof Instruction.SetResultSchema(var columns)) {
            // Declare the output column names.  Emitted once near the start of
            // SELECT programs so the VM knows the schema before any rows arrive.
            st.resultColumns = new ArrayList<>(columns);
            return;
        }

        // ── Aggregate operations ──────────────────────────────────────────
        //
        // Two-phase aggregate processing:
        //   1. InitAgg  — ensure the accumulator slot exists for the current group.
        //   2. UpdateAgg — feed the top-of-stack value into the accumulator.
        //   3. FinalizeAgg — push the final aggregate value onto the stack.

        if (instr instanceof Instruction.InitAgg(var slot, var func, var distinct)) {
            doInitAgg(slot, func, distinct, st);
            return;
        }

        if (instr instanceof Instruction.UpdateAgg(var slot)) {
            doUpdateAgg(slot, st);
            return;
        }

        if (instr instanceof Instruction.FinalizeAgg(var slot, var func)) {
            doFinalizeAgg(slot, func, st);
            return;
        }

        // ── Group-key operations ──────────────────────────────────────────
        //
        // SaveGroupKey pops n values and stores them as the current group key.
        // LoadGroupKey pushes the i-th element of the current group key.
        // AdvanceGroupKey advances the group iterator; jumps when exhausted.

        if (instr instanceof Instruction.SaveGroupKey(var n)) {
            // Use new ArrayList (not List.copyOf) because group keys may
            // contain SQL NULL values, and List.copyOf disallows nulls.
            st.groupKey = new ArrayList<>(st.popN(n));
            return;
        }

        if (instr instanceof Instruction.LoadGroupKey(var i)) {
            st.push(st.groupKey.get(i));
            return;
        }

        if (instr instanceof Instruction.AdvanceGroupKey(var onExhausted, var hasGroupBy)) {
            doAdvanceGroupKey(onExhausted, hasGroupBy, st);
            return;
        }

        // ── Post-processing operations ────────────────────────────────────
        //
        // Applied to the full result buffer after the scan loop.
        // Order in the bytecode: Sort → Limit → Distinct.

        if (instr instanceof Instruction.SortResult(var keys)) {
            doSortResult(keys, st);
            return;
        }

        if (instr instanceof Instruction.LimitResult(var count, var offset)) {
            doLimitResult(count, offset, st);
            return;
        }

        if (instr instanceof Instruction.DistinctResult()) {
            doDistinctResult(st);
            return;
        }

        // ── LEFT JOIN tracking ────────────────────────────────────────────

        if (instr instanceof Instruction.JoinBeginRow()) {
            // Record that the current outer row has not yet matched any inner row.
            st.joinMatchStack.push(false);
            return;
        }

        if (instr instanceof Instruction.JoinSetMatched()) {
            // The current outer row found at least one matching inner row.
            if (!st.joinMatchStack.isEmpty()) {
                st.joinMatchStack.pop();
                st.joinMatchStack.push(true);
            }
            return;
        }

        if (instr instanceof Instruction.JoinIfMatched(var label)) {
            // If matched, skip the null-padding path (jump to label).
            boolean matched = st.joinMatchStack.isEmpty() ? false : st.joinMatchStack.pop();
            if (matched) st.pc = st.resolve(label);
            return;
        }

        // ── DML operations ────────────────────────────────────────────────

        if (instr instanceof Instruction.InsertRow(var table, var columns)) {
            doInsertRow(table, columns, st);
            return;
        }

        if (instr instanceof Instruction.UpdateRows(var table, var assignments, var cursorId)) {
            doUpdateRows(table, assignments, cursorId, st);
            return;
        }

        if (instr instanceof Instruction.DeleteRows(var table, var cursorId)) {
            doDeleteRows(table, cursorId, st);
            return;
        }

        // ── DDL operations ────────────────────────────────────────────────

        if (instr instanceof Instruction.CreateTable(var table, var ifNotExists, var columns)) {
            doCreateTable(table, ifNotExists, columns, st);
            return;
        }

        if (instr instanceof Instruction.DropTable(var table, var ifExists)) {
            doDropTable(table, ifExists, st);
            return;
        }

        // ── Control flow ──────────────────────────────────────────────────

        if (instr instanceof Instruction.Label(var ignored)) {
            // Labels are runtime no-ops — they exist only so the pre-scan can build
            // the label → index map.  Nothing to do here.
            return;
        }

        if (instr instanceof Instruction.Jump(var target)) {
            st.pc = st.resolve(target);
            return;
        }

        if (instr instanceof Instruction.JumpIfFalse(var target)) {
            // SQL truthiness: NULL, false, 0, 0.0 are all falsy.
            Object v = st.pop();
            if (!VmState.isTruthy(v)) {
                st.pc = st.resolve(target);
            }
            return;
        }

        if (instr instanceof Instruction.JumpIfTrue(var target)) {
            Object v = st.pop();
            if (VmState.isTruthy(v)) {
                st.pc = st.resolve(target);
            }
            return;
        }

        // Halt is handled in the main loop above; reaching here means an unknown
        // instruction type was added to the sealed hierarchy without updating this
        // dispatch method — that indicates a programming error.
        throw new IllegalStateException("unknown instruction: " + instr.getClass().getSimpleName());
    }

    // ── Binary operator evaluation ────────────────────────────────────────────
    //
    // Three-valued logic applies throughout:
    //   - If either operand is NULL, the result is NULL — EXCEPT for AND/OR,
    //     which have short-circuit rules:
    //       NULL AND FALSE → FALSE
    //       NULL OR  TRUE  → TRUE

    private static Object applyBinary(BinaryOpCode op, Object left, Object right) {
        // AND/OR have special NULL-short-circuit semantics.
        if (op == BinaryOpCode.AND) {
            if (Boolean.FALSE.equals(left) || Boolean.FALSE.equals(right)) return false;
            if (left == null || right == null) return null;
            return VmState.isTruthy(left) && VmState.isTruthy(right);
        }
        if (op == BinaryOpCode.OR) {
            if (Boolean.TRUE.equals(left) || Boolean.TRUE.equals(right)) return true;
            if (left == null || right == null) return null;
            return VmState.isTruthy(left) || VmState.isTruthy(right);
        }

        // For all other operators, NULL propagates.
        if (left == null || right == null) return null;

        return switch (op) {
            case ADD    -> numericBinary(left, right, Double::sum, Long::sum);
            case SUB    -> numericBinary(left, right, (a, b) -> a - b, (a, b) -> a - b);
            case MUL    -> numericBinary(left, right, (a, b) -> a * b, (a, b) -> a * b);
            case DIV    -> divideValues(left, right);
            case MOD    -> moduloValues(left, right);
            case EQ     -> sqlEquals(left, right);
            case NEQ    -> {
                Object eq = sqlEquals(left, right);
                yield eq == null ? null : !((Boolean) eq);
            }
            case LT     -> sqlCompare(left, right) < 0;
            case LTE    -> sqlCompare(left, right) <= 0;
            case GT     -> sqlCompare(left, right) > 0;
            case GTE    -> sqlCompare(left, right) >= 0;
            case CONCAT -> String.valueOf(left) + String.valueOf(right);
            default     -> throw new IllegalStateException("unexpected BinaryOpCode: " + op);
        };
    }

    // Functional interfaces for numeric binary lambdas.
    @FunctionalInterface private interface DoubleBinOp { double apply(double a, double b); }
    @FunctionalInterface private interface LongBinOp   { long   apply(long a, long b);     }

    /**
     * Apply a numeric binary operation, preferring Long arithmetic when both
     * operands are integral (Long/Integer) and falling back to Double when
     * either is floating-point.
     *
     * <p>SQL type coercions:
     * <pre>
     *   INTEGER op INTEGER → INTEGER (Long)
     *   REAL    op REAL    → REAL    (Double)
     *   INTEGER op REAL    → REAL    (Double)
     * </pre>
     */
    private static Object numericBinary(Object left, Object right,
                                        DoubleBinOp dfn, LongBinOp lfn) {
        if (left instanceof Double || right instanceof Double
                || left instanceof Float || right instanceof Float) {
            return dfn.apply(toDouble(left), toDouble(right));
        }
        return lfn.apply(toLong(left), toLong(right));
    }

    private static Object divideValues(Object left, Object right) {
        // Division by zero: SQL returns NULL (mirrors SQLite behaviour).
        if (toDouble(right) == 0.0) return null;
        if (left instanceof Double || right instanceof Double
                || left instanceof Float || right instanceof Float) {
            return toDouble(left) / toDouble(right);
        }
        // Integer division truncates toward zero (SQL semantics).
        long r = toLong(right);
        if (r == 0) return null;
        return toLong(left) / r;
    }

    private static Object moduloValues(Object left, Object right) {
        if (toDouble(right) == 0.0) return null;
        if (left instanceof Double || right instanceof Double
                || left instanceof Float || right instanceof Float) {
            return toDouble(left) % toDouble(right);
        }
        long r = toLong(right);
        if (r == 0) return null;
        return toLong(left) % r;
    }

    /** SQL equality: booleans only match booleans; numbers are compared by value. */
    private static Object sqlEquals(Object left, Object right) {
        if (left instanceof Boolean && right instanceof Boolean) {
            return left.equals(right);
        }
        if (left instanceof Boolean || right instanceof Boolean) {
            // Boolean does not equal a non-boolean.
            return false;
        }
        if (isNumber(left) && isNumber(right)) {
            return Double.compare(toDouble(left), toDouble(right)) == 0;
        }
        return Objects.equals(left, right);
    }

    /**
     * SQL ordering comparison.  Returns negative/zero/positive like Comparator.
     * NULLs must not reach this method (callers guard).
     *
     * <p>SQL type affinity order (matching SQLite):
     * <pre>
     *   NULL &lt; BOOLEAN &lt; NUMBER &lt; TEXT &lt; BLOB
     * </pre>
     */
    @SuppressWarnings({"unchecked", "rawtypes"})
    private static int sqlCompare(Object left, Object right) {
        int lRank = sqlTypeRank(left);
        int rRank = sqlTypeRank(right);
        if (lRank != rRank) return Integer.compare(lRank, rRank);
        if (left instanceof Boolean lb && right instanceof Boolean rb) {
            return Boolean.compare(lb, rb);
        }
        if (isNumber(left) && isNumber(right)) {
            return Double.compare(toDouble(left), toDouble(right));
        }
        if (left instanceof String ls && right instanceof String rs) {
            return ls.compareTo(rs);
        }
        if (left instanceof byte[] lb && right instanceof byte[] rb) {
            for (int i = 0; i < Math.min(lb.length, rb.length); i++) {
                int cmp = Byte.compare(lb[i], rb[i]);
                if (cmp != 0) return cmp;
            }
            return Integer.compare(lb.length, rb.length);
        }
        if (left instanceof Comparable c && right != null && left.getClass().isInstance(right)) {
            return c.compareTo(right);
        }
        return String.valueOf(left).compareTo(String.valueOf(right));
    }

    private static int sqlTypeRank(Object v) {
        if (v == null) return 0;
        if (v instanceof Boolean) return 1;
        if (isNumber(v)) return 2;
        if (v instanceof String) return 3;
        if (v instanceof byte[]) return 4;
        return 5;
    }

    private static boolean isNumber(Object v) {
        return v instanceof Long || v instanceof Integer
            || v instanceof Double || v instanceof Float;
    }

    private static double toDouble(Object v) {
        return ((Number) v).doubleValue();
    }

    private static long toLong(Object v) {
        return ((Number) v).longValue();
    }

    // ── Unary operator evaluation ─────────────────────────────────────────────

    private static Object applyUnary(SqlCodegen.UnaryOpCode op, Object value) {
        if (value == null) return null; // NULL propagates
        return switch (op) {
            case NEG -> {
                if (value instanceof Double d) yield -d;
                if (value instanceof Float f) yield -(double) f;
                yield -toLong(value);
            }
            case NOT -> !VmState.isTruthy(value);
        };
    }

    // ── BETWEEN ───────────────────────────────────────────────────────────────
    //
    // Stack layout (bottom → top when pushed): value, low, high
    // We pop high first, then low, then value (LIFO).
    //
    //   x BETWEEN low AND high ≡ x >= low AND x <= high
    //
    // Three-valued logic: any NULL input yields NULL.

    private static void doBetween(VmState st) {
        Object high = st.pop();
        Object low  = st.pop();
        Object val  = st.pop();
        if (val == null || low == null || high == null) { st.push(null); return; }
        boolean ge = sqlCompare(val, low)  >= 0;
        boolean le = sqlCompare(val, high) <= 0;
        st.push(ge && le);
    }

    // ── IN list ───────────────────────────────────────────────────────────────
    //
    // Stack layout (bottom → top): needle, item0, item1, …, item_{n-1}
    // We pop n items (the IN list), then the needle.
    //
    // SQL NULL semantics:
    //   - Empty list → false (even if needle is NULL)
    //   - needle is NULL → NULL
    //   - needle found (non-null match) → true
    //   - needle not found but list has NULL → NULL (unknown)
    //   - needle not found and no NULL in list → false

    private static void doInList(int n, VmState st) {
        List<Object> items = st.popN(n);
        Object needle = st.pop();
        if (n == 0) { st.push(false); return; }
        if (needle == null) { st.push(null); return; }
        boolean foundNull = false;
        for (Object item : items) {
            if (item == null) { foundNull = true; continue; }
            if (sqlNonNullEquals(needle, item)) { st.push(true); return; }
        }
        st.push(foundNull ? null : false);
    }

    /** Equality check for non-null values, respecting SQL type affinity. */
    private static boolean sqlNonNullEquals(Object a, Object b) {
        if (a instanceof Boolean && b instanceof Boolean) return a.equals(b);
        if (a instanceof Boolean || b instanceof Boolean) return false;
        if (isNumber(a) && isNumber(b)) return Double.compare(toDouble(a), toDouble(b)) == 0;
        return Objects.equals(a, b);
    }

    // ── LIKE ──────────────────────────────────────────────────────────────────
    //
    // Stack layout (bottom → top): value, pattern
    // (pattern was pushed after value, so it's on top)
    //
    // LIKE wildcards:
    //   %  → matches any sequence of zero or more characters
    //   _  → matches exactly one character
    // All other regex metacharacters are escaped.
    //
    // NULL propagation: if either value or pattern is NULL, result is NULL.

    private static void doLike(boolean negated, VmState st) {
        Object pattern = st.pop();
        Object value   = st.pop();
        if (value == null || pattern == null) { st.push(null); return; }
        if (!(value instanceof String sv) || !(pattern instanceof String sp)) {
            // Non-text operands: LIKE is always false in SQL.
            st.push(negated);
            return;
        }
        boolean matched = likeMatch(sv, sp);
        st.push(negated ? !matched : matched);
    }

    /**
     * Convert a SQL LIKE pattern to a Java regex and test the string.
     *
     * <p>Conversion rules:
     * <ol>
     *   <li>Escape all Java regex metacharacters in the pattern string.</li>
     *   <li>Replace escaped {@code \%} with {@code .*} (any characters).</li>
     *   <li>Replace escaped {@code \_} with {@code .}  (any one character).</li>
     * </ol>
     */
    static boolean likeMatch(String value, String sqlPattern) {
        // Build a regex from the SQL LIKE pattern.
        // Strategy: iterate characters; escape metacharacters; handle % and _.
        StringBuilder regex = new StringBuilder("(?s)");
        for (int i = 0; i < sqlPattern.length(); i++) {
            char c = sqlPattern.charAt(i);
            if (c == '%') {
                regex.append(".*");
            } else if (c == '_') {
                regex.append('.');
            } else if ("\\.[]{}()*+?^$|".indexOf(c) >= 0) {
                // Escape Java regex metacharacters.
                regex.append('\\').append(c);
            } else {
                regex.append(c);
            }
        }
        return Pattern.compile(regex.toString()).matcher(value).matches();
    }

    // ── Scalar function calls ─────────────────────────────────────────────────
    //
    // Currently supports a small set of built-in scalar functions.
    // Unknown functions propagate a null (silent NULL semantics) rather than
    // crashing, to maintain robustness in the face of evolving codegen.

    private static void doCallScalar(String func, int nArgs, VmState st) {
        List<Object> args = st.popN(nArgs);
        st.push(callScalar(func.toUpperCase(), args));
    }

    private static Object callScalar(String func, List<Object> args) {
        return switch (func) {
            case "ABS" -> {
                Object v = args.isEmpty() ? null : args.get(0);
                if (v == null) yield null;
                if (v instanceof Double d) yield Math.abs(d);
                yield Math.abs(toLong(v));
            }
            case "LENGTH" -> {
                Object v = args.isEmpty() ? null : args.get(0);
                if (v == null) yield null;
                if (v instanceof String s) yield (long) s.length();
                if (v instanceof byte[] b) yield (long) b.length;
                yield null;
            }
            case "UPPER" -> {
                Object v = args.isEmpty() ? null : args.get(0);
                yield v instanceof String s ? s.toUpperCase() : null;
            }
            case "LOWER" -> {
                Object v = args.isEmpty() ? null : args.get(0);
                yield v instanceof String s ? s.toLowerCase() : null;
            }
            case "TRIM" -> {
                Object v = args.isEmpty() ? null : args.get(0);
                yield v instanceof String s ? s.trim() : null;
            }
            case "LTRIM" -> {
                Object v = args.isEmpty() ? null : args.get(0);
                if (!(v instanceof String s)) yield null;
                int i = 0;
                while (i < s.length() && s.charAt(i) == ' ') i++;
                yield s.substring(i);
            }
            case "RTRIM" -> {
                Object v = args.isEmpty() ? null : args.get(0);
                if (!(v instanceof String s)) yield null;
                int i = s.length();
                while (i > 0 && s.charAt(i - 1) == ' ') i--;
                yield s.substring(0, i);
            }
            case "SUBSTR", "SUBSTRING" -> {
                if (args.size() < 2) yield null;
                Object v = args.get(0);
                if (!(v instanceof String s)) yield null;
                int start = (int) toLong(args.get(1)) - 1; // SQL is 1-based
                if (start < 0) start = 0;
                if (start >= s.length()) yield "";
                if (args.size() >= 3) {
                    int len = (int) toLong(args.get(2));
                    yield s.substring(start, Math.min(start + len, s.length()));
                }
                yield s.substring(start);
            }
            case "COALESCE" -> {
                for (Object a : args) if (a != null) yield a;
                yield null;
            }
            case "NULLIF" -> {
                if (args.size() < 2) yield null;
                Object a = args.get(0), b = args.get(1);
                yield Objects.equals(a, b) ? null : a;
            }
            case "IFNULL", "NVL" -> {
                if (args.size() < 2) yield null;
                yield args.get(0) != null ? args.get(0) : args.get(1);
            }
            case "IIF" -> {
                if (args.size() < 3) yield null;
                yield VmState.isTruthy(args.get(0)) ? args.get(1) : args.get(2);
            }
            case "REPLACE" -> {
                if (args.size() < 3) yield null;
                if (!(args.get(0) instanceof String s)) yield null;
                if (!(args.get(1) instanceof String from)) yield null;
                if (!(args.get(2) instanceof String to)) yield null;
                yield s.replace(from, to);
            }
            case "TYPEOF" -> {
                Object v = args.isEmpty() ? null : args.get(0);
                yield SqlBackend.SqlValues.typeName(v).toLowerCase();
            }
            case "CAST", "ROUND", "CEIL", "CEILING", "FLOOR", "SQRT",
                 "HEX", "RANDOM", "GLOB" -> null; // unsupported — return NULL
            default -> null;
        };
    }

    // ── Aggregate operations ──────────────────────────────────────────────────

    private static void doInitAgg(int slot, AggFunc func, boolean distinct, VmState st) {
        // Ensure an accumulator list exists for the current group key.
        List<AggAccum> slots = st.aggTable.computeIfAbsent(st.groupKey, k -> {
            st.groupOrder.add(k);
            return new ArrayList<>();
        });
        // Extend the list to at least (slot + 1) entries.
        while (slots.size() <= slot) {
            slots.add(new AggAccum(func, distinct));
        }
    }

    private static void doUpdateAgg(int slot, VmState st) {
        Object value = st.pop();
        List<AggAccum> slots = st.aggTable.computeIfAbsent(st.groupKey, k -> {
            st.groupOrder.add(k);
            return new ArrayList<>();
        });
        if (slot >= slots.size()) {
            // Slot not initialized by InitAgg — possible if the group was first
            // seen here. This shouldn't happen with correct codegen but we handle it.
            throw new IllegalStateException("updateAgg: slot " + slot + " not initialized");
        }
        AggAccum agg = slots.get(slot);

        if (agg.func == AggFunc.COUNT_STAR) {
            // COUNT(*) counts every row regardless of the value.
            agg.count++;
            return;
        }
        if (value == null) {
            // SQL: NULL inputs are ignored for all aggregate functions except COUNT(*).
            return;
        }
        // DISTINCT deduplication: skip already-seen values.
        if (agg.distinct && agg.seen != null) {
            if (!agg.seen.add(value)) return; // already seen — skip
        }

        switch (agg.func) {
            case COUNT -> agg.count++;
            case SUM   -> agg.acc = agg.acc == null
                            ? value
                            : numericAdd(agg.acc, value);
            case AVG   -> {
                agg.acc = agg.acc == null ? value : numericAdd(agg.acc, value);
                agg.count++;
            }
            case MIN   -> {
                if (agg.acc == null || sqlCompare(value, agg.acc) < 0) agg.acc = value;
            }
            case MAX   -> {
                if (agg.acc == null || sqlCompare(value, agg.acc) > 0) agg.acc = value;
            }
            default -> throw new IllegalStateException("unexpected AggFunc in updateAgg");
        }
    }

    private static Object numericAdd(Object a, Object b) {
        if (a instanceof Double || b instanceof Double
                || a instanceof Float || b instanceof Float) {
            return toDouble(a) + toDouble(b);
        }
        return toLong(a) + toLong(b);
    }

    private static void doFinalizeAgg(int slot, AggFunc func, VmState st) {
        List<AggAccum> slots = st.aggTable.computeIfAbsent(st.groupKey, k -> {
            st.groupOrder.add(k);
            return new ArrayList<>();
        });
        // Auto-grow: if InitAgg was never called for this group/slot (empty table
        // with an implicit single group), synthesise a default accumulator.
        while (slots.size() <= slot) {
            slots.add(new AggAccum(func, false));
        }
        AggAccum agg = slots.get(slot);
        switch (func) {
            case COUNT, COUNT_STAR -> st.push((long) agg.count);
            case SUM, MIN, MAX     -> st.push(agg.acc); // NULL for empty/all-null groups
            case AVG               -> {
                if (agg.count == 0) { st.push(null); return; }
                st.push(toDouble(agg.acc) / agg.count);
            }
            default -> throw new IllegalStateException("unexpected AggFunc in finalizeAgg");
        }
    }

    // ── Group-key advancement ─────────────────────────────────────────────────

    private static void doAdvanceGroupKey(String onExhausted, boolean hasGroupBy, VmState st) {
        st.groupIter++;
        // SQL standard: a scalar aggregate (no GROUP BY) over an empty table must
        // return exactly one row.  We synthesise an empty group key so the emit
        // loop runs once.
        if (!hasGroupBy && st.groupIter == 0 && st.groupOrder.isEmpty()) {
            List<Object> emptyKey = List.of();
            st.groupOrder.add(emptyKey);
            st.aggTable.put(emptyKey, new ArrayList<>());
        }
        if (st.groupIter >= st.groupOrder.size()) {
            st.pc = st.resolve(onExhausted);
        } else {
            st.groupKey = st.groupOrder.get(st.groupIter);
        }
    }

    // ── Post-processing operations ────────────────────────────────────────────

    /**
     * Sort the output rows using the given sort keys.
     *
     * <p>Each SortKey specifies a column name, a direction (ASC/DESC), and
     * whether NULLs come first or last.  We implement NULL placement
     * independently of direction, matching SQLite's NULLS LAST default for
     * DESC and NULLS LAST for ASC (when not explicitly specified).
     */
    private static void doSortResult(List<SortKey> keys, VmState st) {
        List<String> cols = st.resultColumns.isEmpty() ? st.program.resultSchema() : st.resultColumns;

        st.outputRows.sort(Comparator.comparing(row -> {
            // Build a compound sort key: one entry per SortKey field.
            // Each entry is a (nullRank, value) pair so we can handle NULL
            // placement independent of sort direction.
            Object[] compound = new Object[keys.size() * 2];
            for (int i = 0; i < keys.size(); i++) {
                SortKey sk = keys.get(i);
                int colIdx = cols.indexOf(sk.column());
                Object val = (colIdx >= 0 && colIdx < row.size()) ? row.get(colIdx) : null;
                boolean isNull = val == null;
                // nullRank: 0 = NULLS FIRST, 2 = NULLS LAST, 1 = non-null.
                int nullRank = isNull
                    ? (sk.nullsOrder() == NullsOrder.FIRST ? 0 : 2)
                    : 1;
                compound[i * 2]     = nullRank;
                compound[i * 2 + 1] = val;
            }
            return compound;
        }, (a, b) -> {
            for (int i = 0; i < keys.size(); i++) {
                SortKey sk = keys.get(i);
                int nullRankCmp = Integer.compare((int) a[i * 2], (int) b[i * 2]);
                if (nullRankCmp != 0) return nullRankCmp;
                Object av = a[i * 2 + 1];
                Object bv = b[i * 2 + 1];
                if (av == null && bv == null) continue;
                if (av == null) continue;
                if (bv == null) continue;
                int cmp = sqlCompare(av, bv);
                if (cmp != 0) return sk.direction() == Direction.DESC ? -cmp : cmp;
            }
            return 0;
        }));
    }

    private static void doLimitResult(Long count, Long offset, VmState st) {
        int start = (offset != null) ? offset.intValue() : 0;
        if (start < 0) start = 0;
        if (count == null) {
            // OFFSET only — keep all rows from start onward.
            if (start > 0) {
                st.outputRows.subList(0, Math.min(start, st.outputRows.size())).clear();
            }
        } else {
            // LIMIT (possibly with OFFSET).
            int end = start + count.intValue();
            if (end > st.outputRows.size()) end = st.outputRows.size();
            // Remove tail first (so indices remain valid), then remove head.
            if (end < st.outputRows.size()) {
                st.outputRows.subList(end, st.outputRows.size()).clear();
            }
            if (start > 0 && start <= st.outputRows.size()) {
                st.outputRows.subList(0, start).clear();
            }
        }
    }

    private static void doDistinctResult(VmState st) {
        // Preserve insertion order, deduplicate by tuple equality.
        Set<List<Object>> seen = new HashSet<>();
        st.outputRows.removeIf(row -> !seen.add(row));
    }

    // ── DML operations ────────────────────────────────────────────────────────

    /**
     * INSERT: pop one value per column (in reverse order, since the last column
     * was pushed last and is on top), build a Row, call backend.insert().
     *
     * <p>Stack layout (bottom → top): col0_value, col1_value, …, coln_value
     * (col0 was pushed first, coln last — coln is on top).
     */
    private static void doInsertRow(String table, List<String> columns, VmState st) {
        // Pop values in reverse order to get col0..coln in proper order.
        List<Object> vals = st.popN(columns.size());
        Row row = new Row();
        for (int i = 0; i < columns.size(); i++) {
            row.put(columns.get(i), vals.get(i));
        }
        st.backend.insert(table, row);
        st.rowsAffected++;
    }

    /**
     * UPDATE: pop one value per assignment column, build the assignments map,
     * call backend.update() with the current cursor position.
     *
     * <p>The {@code assignments} list is the ordered list of column names to
     * update.  Stack layout: assignment[0]_value, …, assignment[n-1]_value.
     */
    private static void doUpdateRows(String table, List<String> assignments, int cursorId, VmState st) {
        List<Object> vals = st.popN(assignments.size());
        Map<String, Object> map = new LinkedHashMap<>();
        for (int i = 0; i < assignments.size(); i++) {
            map.put(assignments.get(i), vals.get(i));
        }
        RowIterator it = st.cursors.get(cursorId);
        if (!(it instanceof Cursor cursor)) {
            throw new IllegalStateException("UPDATE requires a positioned cursor");
        }
        st.backend.update(table, cursor, map);
        st.rowsAffected++;
    }

    /**
     * DELETE: call backend.delete() at the current cursor position.
     * No stack values are consumed.
     */
    private static void doDeleteRows(String table, int cursorId, VmState st) {
        RowIterator it = st.cursors.get(cursorId);
        if (!(it instanceof Cursor cursor)) {
            throw new IllegalStateException("DELETE requires a positioned cursor");
        }
        st.backend.delete(table, cursor);
        st.rowsAffected++;
    }

    // ── DDL operations ────────────────────────────────────────────────────────

    private static void doCreateTable(String table, boolean ifNotExists,
                                      List<com.codingadventures.sqlplanner.SqlPlanner.ColumnDef> plannerCols,
                                      VmState st) {
        // Convert SqlPlanner.ColumnDef → SqlBackend.ColumnDef
        List<SqlBackend.ColumnDef> backendCols = plannerCols.stream()
            .map(c -> new SqlBackend.ColumnDef(c.name(), c.typeName(), c.notNull(), c.primaryKey(), c.unique()))
            .collect(Collectors.toList());
        st.backend.createTable(table, backendCols, ifNotExists);
    }

    private static void doDropTable(String table, boolean ifExists, VmState st) {
        st.backend.dropTable(table, ifExists);
    }
}

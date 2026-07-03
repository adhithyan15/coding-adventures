package com.codingadventures.sqlcodegen;

import com.codingadventures.sqlplanner.SqlPlanner;
import com.codingadventures.sqloptimizer.SqlOptimizer;

import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

// SqlCodegen.java — bytecode compiler for the SQL VM.
//
// Transforms an OptimizedPlan (produced by SqlOptimizer) into a flat Program
// consisting of a list of typed Instructions, a label-to-index map, and an
// output column schema.
//
// The compilation model is a stack machine with explicit cursor management:
//
//   OpenScan / AdvanceCursor / CloseScan  — iterate a base table
//   LoadConst / LoadColumn / BinaryOp … — evaluate scalar expressions
//   BeginRow / EmitColumn / EmitRow       — assemble and emit output rows
//   InitAgg / UpdateAgg / FinalizeAgg     — two-phase aggregate processing
//   InsertRow / UpdateRows / DeleteRows   — DML operations
//   CreateTable / DropTable               — DDL operations
//   Label / Jump / JumpIfFalse / Halt     — control flow
//
// Usage:
//   Program prog = SqlCodegen.compile(logicalPlan);
//   Program prog = SqlCodegen.compileOptimized(optimizedPlan);
//   List<Instruction> instrs = SqlCodegen.compileExpr(expr);   // for unit tests

public final class SqlCodegen {

    private SqlCodegen() {} // static-method namespace only

    // ── Bytecode enumerations ─────────────────────────────────────────────────
    //
    // These enumerations define the primitive operations the SQL VM understands.
    // They are deliberately minimal: the VM interprets them, the codegen emits them.

    /** Arithmetic and comparison operators for binary stack operations. */
    public enum BinaryOpCode {
        ADD, SUB, MUL, DIV, MOD,
        EQ, NEQ, LT, LTE, GT, GTE,
        AND, OR,
        CONCAT
    }

    /** Prefix operators for unary stack operations. */
    public enum UnaryOpCode { NEG, NOT }

    /** Aggregate accumulator functions. */
    public enum AggFunc { COUNT, COUNT_STAR, SUM, AVG, MIN, MAX }

    /** Sort direction for ORDER BY keys. */
    public enum Direction { ASC, DESC }

    /** NULL ordering within a sort key. */
    public enum NullsOrder { FIRST, LAST }

    // ── SortKey record ────────────────────────────────────────────────────────
    //
    // Represents one column in an ORDER BY clause, with its sort direction
    // and how NULL values are ordered relative to non-NULL values.

    /** One column in an ORDER BY clause as seen by the VM. */
    public record SortKey(String column, Direction direction, NullsOrder nullsOrder) {}

    // ── Instruction — sealed interface ────────────────────────────────────────
    //
    // Every instruction the VM can execute.  Uses Java 21 sealed interfaces
    // and records to get exhaustive pattern matching.
    //
    // Stack discipline:
    //   LoadConst(v)        pushes one value
    //   LoadColumn(c, col)  pushes one value
    //   BinaryOp(op)        pops two, pushes one
    //   UnaryOp(op)         pops one,  pushes one
    //   IsNull / IsNotNull  pops one,  pushes boolean
    //   Between             pops three (value, low, high), pushes boolean
    //   InList(n)           pops n items then needle, pushes boolean
    //   Like                pops two (value, pattern), pushes boolean
    //   CallScalar(f, n)    pops n args, pushes one result
    //   Pop                 pops one
    //
    // Cursor operations are side-effecting; they do not push values.
    // Row-building operations (BeginRow/EmitColumn/EmitRow) are similarly
    // side-effecting and pop the top-of-stack value for each EmitColumn.
    //
    // Aggregate slots are indexed from 0; each aggregate function has its own slot.
    // The two-phase protocol is: InitAgg (reset) → UpdateAgg (accumulate) ×N
    // → FinalizeAgg (push result).

    public sealed interface Instruction permits
        Instruction.LoadConst, Instruction.LoadColumn, Instruction.Pop,
        Instruction.BinaryOp, Instruction.UnaryOp,
        Instruction.IsNull, Instruction.IsNotNull,
        Instruction.Between, Instruction.InList, Instruction.Like,
        Instruction.CallScalar,
        Instruction.OpenScan, Instruction.AdvanceCursor, Instruction.CloseScan,
        Instruction.BeginRow, Instruction.EmitColumn, Instruction.EmitRow,
        Instruction.SetResultSchema,
        Instruction.InitAgg, Instruction.UpdateAgg, Instruction.FinalizeAgg,
        Instruction.SaveGroupKey, Instruction.LoadGroupKey, Instruction.AdvanceGroupKey,
        Instruction.SortResult, Instruction.LimitResult, Instruction.DistinctResult,
        Instruction.JoinBeginRow, Instruction.JoinSetMatched, Instruction.JoinIfMatched,
        Instruction.InsertRow, Instruction.UpdateRows, Instruction.DeleteRows,
        Instruction.CreateTable, Instruction.DropTable,
        Instruction.Label, Instruction.Jump, Instruction.JumpIfFalse,
        Instruction.JumpIfTrue, Instruction.Halt {

        // ── Stack instructions ─────────────────────────────────────────────

        /** Push a compile-time constant (null, Boolean, Long, Double, String). */
        record LoadConst(Object value)                         implements Instruction {}

        /** Push the value of a named column from the given cursor. */
        record LoadColumn(int cursorId, String column)         implements Instruction {}

        /** Discard the top-of-stack value. */
        record Pop()                                           implements Instruction {}

        /** Pop two values, apply binary operator, push result. */
        record BinaryOp(BinaryOpCode op)                      implements Instruction {}

        /** Pop one value, apply unary operator, push result. */
        record UnaryOp(UnaryOpCode op)                        implements Instruction {}

        /** Pop one value; push true if it is SQL NULL, false otherwise. */
        record IsNull()                                        implements Instruction {}

        /** Pop one value; push true if it is not SQL NULL, false otherwise. */
        record IsNotNull()                                     implements Instruction {}

        /**
         * Pop three values (high, low, value — pushed value first, then low, then high);
         * push true iff value BETWEEN low AND high (inclusive).
         */
        record Between()                                       implements Instruction {}

        /**
         * Pop n items (the IN list), then the needle;
         * push true iff needle equals any item.
         */
        record InList(int n)                                   implements Instruction {}

        /**
         * Pop two values (pattern pushed after value); apply LIKE matching.
         * negated=true implements NOT LIKE.
         */
        record Like(boolean negated)                           implements Instruction {}

        /** Pop nArgs values, call scalar SQL function, push one result. */
        record CallScalar(String func, int nArgs)              implements Instruction {}

        // ── Cursor / scan instructions ─────────────────────────────────────

        /** Open a table scan on the given table; associate it with cursorId. */
        record OpenScan(int cursorId, String table)            implements Instruction {}

        /**
         * Advance the cursor to the next row.
         * If no row is available, jump to the label onExhausted.
         */
        record AdvanceCursor(int cursorId, String onExhausted) implements Instruction {}

        /** Close a previously opened cursor. */
        record CloseScan(int cursorId)                         implements Instruction {}

        // ── Row-building instructions ──────────────────────────────────────

        /** Start assembling a new output row. */
        record BeginRow()                                      implements Instruction {}

        /** Pop one value and store it as the named column of the current output row. */
        record EmitColumn(String name)                         implements Instruction {}

        /** Finalise and emit the current output row to the result set. */
        record EmitRow()                                       implements Instruction {}

        /**
         * Declare the output column schema.
         * Emitted once at the start of the program so the VM knows the result shape
         * before any rows arrive.
         */
        record SetResultSchema(List<String> columns)           implements Instruction {}

        // ── Aggregate instructions ─────────────────────────────────────────

        /**
         * Initialise (reset) aggregate accumulator slot.
         * distinct=true tracks which values have already been seen.
         */
        record InitAgg(int slot, AggFunc func, boolean distinct) implements Instruction {}

        /**
         * Pop one value and feed it into aggregate accumulator slot.
         * For COUNT_STAR the value is ignored (push null before UpdateAgg).
         */
        record UpdateAgg(int slot)                             implements Instruction {}

        /** Push the finalised aggregate value from slot (mean, count, etc.). */
        record FinalizeAgg(int slot, AggFunc func)             implements Instruction {}

        // ── Group-key instructions ─────────────────────────────────────────
        //
        // The VM maintains an internal "group key store".
        // SaveGroupKey pops n values from the stack and saves them.
        // LoadGroupKey pushes the i-th saved value back onto the stack.
        // AdvanceGroupKey moves to the next group; jumps to onExhausted when done.

        /** Pop n values (GROUP BY expressions) and save them as the current group key. */
        record SaveGroupKey(int n)                             implements Instruction {}

        /** Push the i-th element of the current group key onto the stack. */
        record LoadGroupKey(int i)                             implements Instruction {}

        /**
         * Move to the next group.
         * hasGroupBy=true means GROUP BY columns were used; false means scalar aggregate.
         * Jumps to onExhausted when all groups have been emitted.
         */
        record AdvanceGroupKey(String onExhausted, boolean hasGroupBy) implements Instruction {}

        // ── Post-processing instructions ───────────────────────────────────
        //
        // Applied to the full result set after the scan loop completes.
        // The VM buffers all emitted rows, then applies these in order.

        /** Sort all buffered result rows by the given keys. */
        record SortResult(List<SortKey> keys)                  implements Instruction {}

        /** Truncate buffered results to at most count rows, skipping the first offset. */
        record LimitResult(Long count, Long offset)            implements Instruction {}

        /** Remove duplicate rows from the buffered result set. */
        record DistinctResult()                                implements Instruction {}

        // ── LEFT JOIN tracking instructions ────────────────────────────────

        /** Begin tracking whether the current left row found any match. */
        record JoinBeginRow()                                  implements Instruction {}

        /** Mark that the current left row found at least one matching right row. */
        record JoinSetMatched()                                implements Instruction {}

        /**
         * If the current left row found a match, jump to label (skip null-padding).
         * Otherwise fall through so the caller can emit a null-padded row.
         */
        record JoinIfMatched(String label)                     implements Instruction {}

        // ── DML instructions ───────────────────────────────────────────────

        /** Pop values for the given columns (in order) and insert a row. */
        record InsertRow(String table, List<String> columns)   implements Instruction {}

        /** Update rows in table where the given cursor currently points. */
        record UpdateRows(String table, List<String> assignments, int cursorId) implements Instruction {}

        /** Delete the row at which the given cursor currently points. */
        record DeleteRows(String table, int cursorId)          implements Instruction {}

        // ── DDL instructions ───────────────────────────────────────────────

        /** Create a table (optionally skip if it already exists). */
        record CreateTable(String table, boolean ifNotExists,
                           List<SqlPlanner.ColumnDef> columns) implements Instruction {}

        /** Drop a table (optionally ignore if it doesn't exist). */
        record DropTable(String table, boolean ifExists)       implements Instruction {}

        // ── Control-flow instructions ──────────────────────────────────────

        /** Define a jump target at this position in the instruction stream. */
        record Label(String name)                              implements Instruction {}

        /** Unconditional jump to target. */
        record Jump(String target)                             implements Instruction {}

        /**
         * Pop one value; if it is falsy (false or null), jump to target.
         * Otherwise fall through.
         */
        record JumpIfFalse(String target)                      implements Instruction {}

        /**
         * Pop one value; if it is truthy (true), jump to target.
         * Otherwise fall through.
         */
        record JumpIfTrue(String target)                       implements Instruction {}

        /** Terminate the program. */
        record Halt()                                          implements Instruction {}
    }

    // ── Program record ────────────────────────────────────────────────────────
    //
    // The compiled output.  instructions is the flat bytecode list.
    // labels maps each Label.name to its index in instructions (for the VM to
    // implement jumps without linear search).
    // resultSchema lists the output column names in order.

    /**
     * A compiled SQL program ready for the VM to execute.
     *
     * @param instructions flat ordered list of bytecode instructions
     * @param labels       mapping from label name to instruction index
     * @param resultSchema ordered list of output column names
     */
    public record Program(
        List<Instruction> instructions,
        Map<String, Integer> labels,
        List<String> resultSchema
    ) {}

    // ── Compilation context ───────────────────────────────────────────────────
    //
    // Private mutable state threaded through the recursive compilation.
    // Keeps counters for cursor IDs, label suffixes, and aggregate slots.

    private static final class Ctx {
        int c = 0; // cursor ID counter
        int l = 0; // label suffix counter
        int a = 0; // aggregate slot counter
        // Maps table alias → cursor ID so expression compilation can look up the
        // right cursor when evaluating a column reference like "users.id".
        final Map<String, Integer> aliasToId = new HashMap<>();

        /** Allocate a fresh cursor ID (monotonically increasing). */
        int nextCursor() { return c++; }

        /** Generate a unique label like "scan_0_loop_3". */
        String nextLabel(String pfx) { return pfx + "_" + l++; }

        /** Allocate a fresh aggregate accumulator slot. */
        int nextSlot() { return a++; }

        /**
         * Look up the cursor ID for a table alias.
         * Returns 0 as a safe default (single-table queries have cursor 0).
         */
        int cursorOf(String alias) {
            return aliasToId.getOrDefault(alias, 0);
        }
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /**
     * Compile a LogicalPlan by first optimising it, then generating bytecode.
     * Equivalent to {@code compileOptimized(SqlOptimizer.optimize(plan))}.
     *
     * @param plan the logical plan from SqlPlanner
     * @return a bytecode Program ready for the VM
     */
    public static Program compile(SqlPlanner.LogicalPlan plan) {
        return compileOptimized(SqlOptimizer.optimize(plan));
    }

    /**
     * Compile an OptimizedPlan directly into a Program.
     * This is the primary entry point when the optimizer has already been run.
     *
     * @param plan an optimized plan from SqlOptimizer
     * @return a bytecode Program ready for the VM
     */
    public static Program compileOptimized(SqlOptimizer.OptimizedPlan plan) {
        var out = new ArrayList<Instruction>();
        var ctx = new Ctx();

        compilePlan(plan, out, ctx);

        // ── Resolve labels ─────────────────────────────────────────────────
        // Build a label name → instruction index map so the VM can jump in O(1).
        var lblMap = new HashMap<String, Integer>();
        for (int i = 0; i < out.size(); i++) {
            if (out.get(i) instanceof Instruction.Label lb) {
                lblMap.put(lb.name(), i);
            }
        }

        // ── Extract result schema ──────────────────────────────────────────
        // The SetResultSchema instruction appears at most once, near the start.
        List<String> resultSchema = List.of();
        for (var instr : out) {
            if (instr instanceof Instruction.SetResultSchema srs) {
                resultSchema = srs.columns();
                break;
            }
        }

        return new Program(Collections.unmodifiableList(out), lblMap, resultSchema);
    }

    /**
     * Compile a single scalar expression in isolation.
     * Uses a fresh empty context (cursor 0 for any column reference).
     * Useful for unit tests that want to inspect expression bytecode directly.
     *
     * @param expr a SQL scalar expression
     * @return the list of instructions that evaluate the expression
     */
    public static List<Instruction> compileExpr(SqlPlanner.SqlExpr expr) {
        return compileExprCtx(expr, new Ctx());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Plan compilation
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // compilePlan is the top-level dispatcher.  It handles three categories:
    //
    //   1. DDL / DML — simple: emit one instruction + Halt.
    //   2. SELECT — complex: peel Sort/Limit/Distinct wrappers, compile core,
    //               then append post-processing.
    //   3. EmptyResult — emit Halt (zero rows).

    private static void compilePlan(SqlOptimizer.OptimizedPlan plan,
                                    List<Instruction> out,
                                    Ctx ctx) {
        // ── Quick DDL / DML dispatch ───────────────────────────────────────
        switch (plan) {
            case SqlOptimizer.OptimizedPlan.CreateTable ct -> {
                out.add(new Instruction.CreateTable(ct.table(), ct.ifNotExists(), ct.columns()));
                out.add(new Instruction.Halt());
                return;
            }
            case SqlOptimizer.OptimizedPlan.DropTable dt -> {
                out.add(new Instruction.DropTable(dt.table(), dt.ifExists()));
                out.add(new Instruction.Halt());
                return;
            }
            case SqlOptimizer.OptimizedPlan.Insert ins -> {
                compileInsert(ins, out, ctx);
                out.add(new Instruction.Halt());
                return;
            }
            case SqlOptimizer.OptimizedPlan.Update upd -> {
                compileUpdate(upd, out, ctx);
                out.add(new Instruction.Halt());
                return;
            }
            case SqlOptimizer.OptimizedPlan.Delete del -> {
                compileDelete(del, out, ctx);
                out.add(new Instruction.Halt());
                return;
            }
            case SqlOptimizer.OptimizedPlan.EmptyResult e -> {
                // An EmptyResult at the top level means zero rows. Just halt.
                out.add(new Instruction.Halt());
                return;
            }
            default -> {} // fall through to SELECT path
        }

        // ── SELECT path: peel post-processing wrappers ─────────────────────
        //
        // Sort, Limit, and Distinct are applied after the scan loop completes.
        // We peel them from the top of the plan tree outermost-first, collecting
        // each instruction in a list, then append them after the scan body.
        //
        // Execution semantics for Sort(Limit(inner)):
        //   1. Scan produces rows
        //   2. Sort all rows (Sort is semantically outermost — it needs all rows)
        //   3. Limit takes the first N from the sorted result
        //
        // So postOps must be appended in the order they were collected:
        //   [SortResult, LimitResult] — Sort first, then Limit.
        //
        // Do NOT reverse: the outermost wrapper is collected first and must
        // execute first in the post-processing phase.

        var postOps   = new ArrayList<Instruction>();
        var innerPlan = plan;

        while (true) {
            if (innerPlan instanceof SqlOptimizer.OptimizedPlan.Sort s) {
                var keys = buildSortKeys(s.keys());
                postOps.add(new Instruction.SortResult(keys));
                innerPlan = s.input();
            } else if (innerPlan instanceof SqlOptimizer.OptimizedPlan.Limit lim) {
                postOps.add(new Instruction.LimitResult(lim.count(), lim.offset()));
                innerPlan = lim.input();
            } else if (innerPlan instanceof SqlOptimizer.OptimizedPlan.Distinct d) {
                postOps.add(new Instruction.DistinctResult());
                innerPlan = d.input();
            } else {
                break;
            }
        }

        // ── Compile the core plan (Project / Aggregate / bare scan) ────────
        compileCore(innerPlan, out, ctx);

        // Append post-processing in collected order (outermost → innermost).
        // For Sort(Limit(inner)): [SortResult, LimitResult] → Sort then Limit.
        out.addAll(postOps);

        out.add(new Instruction.Halt());
    }

    // ── Core plan compilation ─────────────────────────────────────────────────
    //
    // compileCore handles the inner plan after post-processing wrappers have
    // been peeled.  The innermost plan is one of:
    //
    //   • Project(Aggregate(...))  — aggregate query
    //   • Project(...)             — regular SELECT
    //   • Aggregate(...)           — aggregate without explicit project (rare)
    //   • EmptyResult              — inside a Project wrapper (handled by Project case)
    //
    // The two-phase aggregate protocol is handled inside compileCore when we
    // detect a Project wrapping an Aggregate.

    private static void compileCore(SqlOptimizer.OptimizedPlan plan,
                                    List<Instruction> out,
                                    Ctx ctx) {
        switch (plan) {

            // ── EmptyResult inside core position ──────────────────────────
            case SqlOptimizer.OptimizedPlan.EmptyResult ignored -> {
                // Nothing to emit. compilePlan adds Halt afterwards.
            }

            // ── Project: the typical outer SELECT wrapper ──────────────────
            case SqlOptimizer.OptimizedPlan.Project p -> {
                // Collect output column names.
                var schema = buildSchema(p.columns());
                out.add(new Instruction.SetResultSchema(schema));

                // Special case: Project wrapping Aggregate → two-phase agg.
                if (p.input() instanceof SqlOptimizer.OptimizedPlan.Aggregate agg) {
                    compileProjectAggregate(agg, p.columns(), schema, out, ctx);
                    return;
                }

                // Regular project: scan + filter + row building.
                compileScanBody(p.input(), out, ctx, () -> {
                    out.add(new Instruction.BeginRow());
                    emitProjectColumns(p.columns(), schema, out, ctx);
                    out.add(new Instruction.EmitRow());
                });
            }

            // ── Bare Aggregate (SELECT COUNT(*) without an explicit Project) ─
            case SqlOptimizer.OptimizedPlan.Aggregate agg -> {
                // Build a synthetic schema from the aggregate aliases.
                var schema = new ArrayList<String>();
                for (var item : agg.aggregates()) schema.add(item.alias());
                out.add(new Instruction.SetResultSchema(schema));
                compileAggregateOnly(agg, schema, out, ctx);
            }

            // ── All other plan types at core level — compile as scan body ──
            default -> {
                // e.g. bare Scan or Filter without a Project above it.
                // Emit minimal schema.
                out.add(new Instruction.SetResultSchema(List.of()));
                compileScanBody(plan, out, ctx, () -> {
                    out.add(new Instruction.BeginRow());
                    out.add(new Instruction.EmitRow());
                });
            }
        }
    }

    // ── Scan body compilation ─────────────────────────────────────────────────
    //
    // compileScanBody generates the looping structure for the data-access tier.
    // The body Runnable is called at the innermost point of the loop — it
    // emits whichever instructions should execute for each row that passes all
    // filters.
    //
    // Pattern for a single Scan:
    //
    //   OpenScan(cid, table)
    //   Label("scan_cid_loop_N")
    //   AdvanceCursor(cid, "scan_cid_end_M")
    //     <body>
    //   Jump("scan_cid_loop_N")
    //   Label("scan_cid_end_M")
    //   CloseScan(cid)
    //
    // Filter wraps the body with a predicate guard:
    //
    //   <inner scan loop>
    //     <predicate expression>
    //     JumpIfFalse("filter_skip_N")
    //     <body>
    //     Label("filter_skip_N")
    //
    // INNER JOIN is a nested loop: compile left, and inside the left body
    // compile right, and inside the right body emit the join body.

    private static void compileScanBody(SqlOptimizer.OptimizedPlan plan,
                                        List<Instruction> out,
                                        Ctx ctx,
                                        Runnable body) {
        switch (plan) {

            // ── Leaf: base table scan ─────────────────────────────────────
            case SqlOptimizer.OptimizedPlan.Scan s -> {
                int cid = ctx.nextCursor();
                String alias = s.alias() != null ? s.alias() : s.table();
                ctx.aliasToId.put(alias, cid);

                String loopLbl = ctx.nextLabel("scan_" + cid + "_loop");
                String endLbl  = ctx.nextLabel("scan_" + cid + "_end");

                out.add(new Instruction.OpenScan(cid, s.table()));
                out.add(new Instruction.Label(loopLbl));
                out.add(new Instruction.AdvanceCursor(cid, endLbl));
                body.run();
                out.add(new Instruction.Jump(loopLbl));
                out.add(new Instruction.Label(endLbl));
                out.add(new Instruction.CloseScan(cid));
            }

            // ── Filter: wrap body with predicate check ────────────────────
            case SqlOptimizer.OptimizedPlan.Filter f -> {
                String skipLbl = ctx.nextLabel("filter_skip");
                // The filter surrounds the body inside the inner scan loop.
                compileScanBody(f.input(), out, ctx, () -> {
                    out.addAll(compileExprCtx(f.predicate(), ctx));
                    out.add(new Instruction.JumpIfFalse(skipLbl));
                    body.run();
                    out.add(new Instruction.Label(skipLbl));
                });
            }

            // ── INNER / CROSS JOIN: nested loop ───────────────────────────
            case SqlOptimizer.OptimizedPlan.Join j
                when j.kind() == SqlPlanner.JoinKind.INNER
                  || j.kind() == SqlPlanner.JoinKind.CROSS -> {
                compileScanBody(j.left(), out, ctx, () -> {
                    compileScanBody(j.right(), out, ctx, () -> {
                        if (j.condition() != null
                                && j.kind() == SqlPlanner.JoinKind.INNER) {
                            String skipLbl = ctx.nextLabel("join_skip");
                            out.addAll(compileExprCtx(j.condition(), ctx));
                            out.add(new Instruction.JumpIfFalse(skipLbl));
                            body.run();
                            out.add(new Instruction.Label(skipLbl));
                        } else {
                            body.run();
                        }
                    });
                });
            }

            // ── LEFT OUTER JOIN: nested loop with null-padding ────────────
            //
            // For each left row:
            //   JoinBeginRow()           — clear the "matched" flag
            //   <right scan loop>
            //     [condition check]
            //     JoinSetMatched()       — mark that we found a match
            //     body                   — emit the joined row
            //   JoinIfMatched(matched)   — if a match was found, skip null-padding
            //   body                     — null-padding (right cols are NULL)
            //   Label(matched)
            case SqlOptimizer.OptimizedPlan.Join j
                when j.kind() == SqlPlanner.JoinKind.LEFT -> {
                String matchedLbl = ctx.nextLabel("loj_matched");
                compileScanBody(j.left(), out, ctx, () -> {
                    out.add(new Instruction.JoinBeginRow());
                    compileScanBody(j.right(), out, ctx, () -> {
                        if (j.condition() != null) {
                            String skipLbl = ctx.nextLabel("loj_skip");
                            out.addAll(compileExprCtx(j.condition(), ctx));
                            out.add(new Instruction.JumpIfFalse(skipLbl));
                            out.add(new Instruction.JoinSetMatched());
                            body.run();
                            out.add(new Instruction.Label(skipLbl));
                        } else {
                            out.add(new Instruction.JoinSetMatched());
                            body.run();
                        }
                    });
                    out.add(new Instruction.JoinIfMatched(matchedLbl));
                    // Null-padding row: right columns will be NULL in the VM
                    // because no cursor is positioned.
                    body.run();
                    out.add(new Instruction.Label(matchedLbl));
                });
            }

            // ── RIGHT OUTER JOIN: mirror of LEFT JOIN ─────────────────────
            case SqlOptimizer.OptimizedPlan.Join j
                when j.kind() == SqlPlanner.JoinKind.RIGHT -> {
                // Compile as a left join but swap the sides.
                String matchedLbl = ctx.nextLabel("roj_matched");
                compileScanBody(j.right(), out, ctx, () -> {
                    out.add(new Instruction.JoinBeginRow());
                    compileScanBody(j.left(), out, ctx, () -> {
                        if (j.condition() != null) {
                            String skipLbl = ctx.nextLabel("roj_skip");
                            out.addAll(compileExprCtx(j.condition(), ctx));
                            out.add(new Instruction.JumpIfFalse(skipLbl));
                            out.add(new Instruction.JoinSetMatched());
                            body.run();
                            out.add(new Instruction.Label(skipLbl));
                        } else {
                            out.add(new Instruction.JoinSetMatched());
                            body.run();
                        }
                    });
                    out.add(new Instruction.JoinIfMatched(matchedLbl));
                    body.run();
                    out.add(new Instruction.Label(matchedLbl));
                });
            }

            // ── Transparent wrappers: strip and recurse ───────────────────
            //
            // Having is handled by the Aggregate path above; if it leaks here
            // we strip it so the scan body can still be compiled.
            case SqlOptimizer.OptimizedPlan.Having h ->
                compileScanBody(h.input(), out, ctx, body);

            // ── EmptyResult: body is never called ─────────────────────────
            case SqlOptimizer.OptimizedPlan.EmptyResult ignored -> {
                // No rows are produced; body intentionally not called.
            }

            default ->
                throw new UnsupportedOperationException(
                    "Unsupported plan node in scan body: "
                    + plan.getClass().getSimpleName());
        }
    }

    // ── Two-phase aggregate compilation ──────────────────────────────────────
    //
    // Phase 1 (accumulation loop):
    //   For each row that passes filters:
    //     InitAgg(slot, func, distinct)     — must be called once per group,
    //                                         but for simplicity we emit it inside
    //                                         the loop body; the VM must handle
    //                                         idempotent init (no-op if already init).
    //     SaveGroupKey(n)                   — store GROUP BY values
    //     [for each aggregate argument:]
    //       <expr>
    //       UpdateAgg(slot)
    //
    // Phase 2 (group iteration):
    //   Label("group_start_N")
    //   AdvanceGroupKey("group_end_M", hasGroupBy)
    //   BeginRow()
    //   [for each output column: LoadGroupKey(i) or FinalizeAgg(slot, func)]
    //   EmitRow()
    //   Jump("group_start_N")
    //   Label("group_end_M")

    private static void compileProjectAggregate(
            SqlOptimizer.OptimizedPlan.Aggregate agg,
            List<SqlPlanner.OutputColumn> projCols,
            List<String> schema,
            List<Instruction> out,
            Ctx ctx) {

        int numGroupBy = agg.groupBy().size();
        boolean hasGroupBy = numGroupBy > 0;

        // Allocate an aggregate slot for each aggregate function.
        var aggSlots = new ArrayList<Integer>();
        for (var item : agg.aggregates()) aggSlots.add(ctx.nextSlot());

        // ── Phase 1: scan + accumulate ────────────────────────────────────
        compileScanBody(agg.input(), out, ctx, () -> {
            // Save the GROUP BY key first so the VM can group rows.
            for (var gbExpr : agg.groupBy()) {
                out.addAll(compileExprCtx(gbExpr, ctx));
            }
            if (hasGroupBy) {
                out.add(new Instruction.SaveGroupKey(numGroupBy));
            } else {
                // Scalar aggregate: no GROUP BY — emit SaveGroupKey(0) so
                // the VM knows to produce exactly one output row.
                out.add(new Instruction.SaveGroupKey(0));
            }

            // Init + update each aggregate.
            for (int i = 0; i < agg.aggregates().size(); i++) {
                var item = agg.aggregates().get(i);
                AggFunc func = mapAggFunc(item.func(), item.arg());
                int slot = aggSlots.get(i);
                out.add(new Instruction.InitAgg(slot, func, item.distinct()));
                // Push the aggregate argument.
                if (item.arg() instanceof SqlPlanner.AggArg.Expr e) {
                    out.addAll(compileExprCtx(e.expression(), ctx));
                } else {
                    // COUNT(*) — push null; VM ignores it for COUNT_STAR.
                    out.add(new Instruction.LoadConst(null));
                }
                out.add(new Instruction.UpdateAgg(slot));
            }
        });

        // ── Phase 2: group iteration ──────────────────────────────────────
        String groupStart = ctx.nextLabel("group_start");
        String groupEnd   = ctx.nextLabel("group_end");

        out.add(new Instruction.Label(groupStart));
        out.add(new Instruction.AdvanceGroupKey(groupEnd, hasGroupBy));
        out.add(new Instruction.BeginRow());

        // For each output column, determine whether it maps to a GROUP BY key
        // or an aggregate result.
        // Strategy: match by AggExpr → FinalizeAgg; Column → LoadGroupKey; else LoadConst.
        emitAggProjectColumns(projCols, schema, agg.groupBy(), agg.aggregates(), aggSlots, out, ctx);

        out.add(new Instruction.EmitRow());
        out.add(new Instruction.Jump(groupStart));
        out.add(new Instruction.Label(groupEnd));
    }

    // ── Bare aggregate (no outer Project) ────────────────────────────────────

    private static void compileAggregateOnly(
            SqlOptimizer.OptimizedPlan.Aggregate agg,
            List<String> schema,
            List<Instruction> out,
            Ctx ctx) {

        int numGroupBy = agg.groupBy().size();
        boolean hasGroupBy = numGroupBy > 0;

        var aggSlots = new ArrayList<Integer>();
        for (var item : agg.aggregates()) aggSlots.add(ctx.nextSlot());

        // Phase 1
        compileScanBody(agg.input(), out, ctx, () -> {
            for (var gbExpr : agg.groupBy()) {
                out.addAll(compileExprCtx(gbExpr, ctx));
            }
            out.add(new Instruction.SaveGroupKey(hasGroupBy ? numGroupBy : 0));
            for (int i = 0; i < agg.aggregates().size(); i++) {
                var item = agg.aggregates().get(i);
                AggFunc func = mapAggFunc(item.func(), item.arg());
                int slot = aggSlots.get(i);
                out.add(new Instruction.InitAgg(slot, func, item.distinct()));
                if (item.arg() instanceof SqlPlanner.AggArg.Expr e) {
                    out.addAll(compileExprCtx(e.expression(), ctx));
                } else {
                    out.add(new Instruction.LoadConst(null));
                }
                out.add(new Instruction.UpdateAgg(slot));
            }
        });

        // Phase 2
        String groupStart = ctx.nextLabel("group_start");
        String groupEnd   = ctx.nextLabel("group_end");

        out.add(new Instruction.Label(groupStart));
        out.add(new Instruction.AdvanceGroupKey(groupEnd, hasGroupBy));
        out.add(new Instruction.BeginRow());

        // Emit each aggregate result column.
        for (int i = 0; i < agg.aggregates().size(); i++) {
            var item = agg.aggregates().get(i);
            AggFunc func = mapAggFunc(item.func(), item.arg());
            out.add(new Instruction.FinalizeAgg(aggSlots.get(i), func));
            out.add(new Instruction.EmitColumn(schema.get(i)));
        }

        out.add(new Instruction.EmitRow());
        out.add(new Instruction.Jump(groupStart));
        out.add(new Instruction.Label(groupEnd));
    }

    // ── DML compilation ───────────────────────────────────────────────────────

    private static void compileInsert(SqlOptimizer.OptimizedPlan.Insert ins,
                                      List<Instruction> out, Ctx ctx) {
        // For each value row, push each value and emit InsertRow.
        for (var row : ins.values()) {
            for (var val : row) {
                out.addAll(compileExprCtx(val, ctx));
            }
            out.add(new Instruction.InsertRow(ins.table(), ins.columns()));
        }
    }

    private static void compileUpdate(SqlOptimizer.OptimizedPlan.Update upd,
                                      List<Instruction> out, Ctx ctx) {
        // Scan the table; for each row that passes the predicate, update it.
        int cid = ctx.nextCursor();
        ctx.aliasToId.put(upd.table(), cid);

        String loopLbl = ctx.nextLabel("update_loop");
        String endLbl  = ctx.nextLabel("update_end");
        String skipLbl = ctx.nextLabel("update_skip");

        out.add(new Instruction.OpenScan(cid, upd.table()));
        out.add(new Instruction.Label(loopLbl));
        out.add(new Instruction.AdvanceCursor(cid, endLbl));

        if (upd.predicate() != null) {
            out.addAll(compileExprCtx(upd.predicate(), ctx));
            out.add(new Instruction.JumpIfFalse(skipLbl));
        }

        // Push assignment values and collect column names.
        var cols = new ArrayList<String>();
        for (var assignment : upd.assignments()) {
            out.addAll(compileExprCtx(assignment.value(), ctx));
            cols.add(assignment.column());
        }
        out.add(new Instruction.UpdateRows(upd.table(), cols, cid));

        out.add(new Instruction.Label(skipLbl));
        out.add(new Instruction.Jump(loopLbl));
        out.add(new Instruction.Label(endLbl));
        out.add(new Instruction.CloseScan(cid));
    }

    private static void compileDelete(SqlOptimizer.OptimizedPlan.Delete del,
                                      List<Instruction> out, Ctx ctx) {
        int cid = ctx.nextCursor();
        ctx.aliasToId.put(del.table(), cid);

        String loopLbl = ctx.nextLabel("delete_loop");
        String endLbl  = ctx.nextLabel("delete_end");
        String skipLbl = ctx.nextLabel("delete_skip");

        out.add(new Instruction.OpenScan(cid, del.table()));
        out.add(new Instruction.Label(loopLbl));
        out.add(new Instruction.AdvanceCursor(cid, endLbl));

        if (del.predicate() != null) {
            out.addAll(compileExprCtx(del.predicate(), ctx));
            out.add(new Instruction.JumpIfFalse(skipLbl));
        }

        out.add(new Instruction.DeleteRows(del.table(), cid));

        out.add(new Instruction.Label(skipLbl));
        out.add(new Instruction.Jump(loopLbl));
        out.add(new Instruction.Label(endLbl));
        out.add(new Instruction.CloseScan(cid));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Expression compilation
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // Compiles a SqlExpr into a list of stack-machine instructions.
    // The instructions are appended to the output list of the caller.
    //
    // Binary operations use left-operand then right-operand ordering:
    //   LoadConst(3)
    //   LoadConst(4)
    //   BinaryOp(ADD)   →  stack: [7]
    //
    // Between pushes: value, low, high (in that order) so the VM can pop
    // high first, then low, then value.

    static List<Instruction> compileExprCtx(SqlPlanner.SqlExpr expr, Ctx ctx) {
        return switch (expr) {

            // ── Literal ───────────────────────────────────────────────────
            case SqlPlanner.SqlExpr.Literal lit ->
                List.of(new Instruction.LoadConst(lit.value()));

            // ── Column reference ──────────────────────────────────────────
            case SqlPlanner.SqlExpr.Column col -> {
                int cid = col.table() != null ? ctx.cursorOf(col.table()) : 0;
                yield List.of(new Instruction.LoadColumn(cid, col.column()));
            }

            // ── Binary operation ──────────────────────────────────────────
            case SqlPlanner.SqlExpr.BinaryOp bop -> {
                var result = new ArrayList<Instruction>();
                result.addAll(compileExprCtx(bop.left(), ctx));
                result.addAll(compileExprCtx(bop.right(), ctx));
                result.add(new Instruction.BinaryOp(mapBinaryOp(bop.op())));
                yield result;
            }

            // ── Unary operation ───────────────────────────────────────────
            case SqlPlanner.SqlExpr.UnaryOp uop -> {
                var result = new ArrayList<Instruction>(compileExprCtx(uop.operand(), ctx));
                result.add(new Instruction.UnaryOp(mapUnaryOp(uop.op())));
                yield result;
            }

            // ── IS NULL / IS NOT NULL ─────────────────────────────────────
            case SqlPlanner.SqlExpr.IsNull isn -> {
                var result = new ArrayList<Instruction>(compileExprCtx(isn.operand(), ctx));
                result.add(new Instruction.IsNull());
                yield result;
            }

            case SqlPlanner.SqlExpr.IsNotNull inn -> {
                var result = new ArrayList<Instruction>(compileExprCtx(inn.operand(), ctx));
                result.add(new Instruction.IsNotNull());
                yield result;
            }

            // ── BETWEEN ───────────────────────────────────────────────────
            // Stack order: value, low, high  (VM pops high first, then low, then value)
            case SqlPlanner.SqlExpr.Between b -> {
                var result = new ArrayList<Instruction>();
                result.addAll(compileExprCtx(b.value(), ctx));
                result.addAll(compileExprCtx(b.low(), ctx));
                result.addAll(compileExprCtx(b.high(), ctx));
                result.add(new Instruction.Between());
                yield result;
            }

            // ── IN list ───────────────────────────────────────────────────
            // Stack order: needle first, then items (VM pops n items, then needle)
            case SqlPlanner.SqlExpr.In in -> {
                var result = new ArrayList<Instruction>();
                result.addAll(compileExprCtx(in.value(), ctx));
                for (var item : in.items()) result.addAll(compileExprCtx(item, ctx));
                result.add(new Instruction.InList(in.items().size()));
                yield result;
            }

            // ── NOT IN list ───────────────────────────────────────────────
            // NOT IN is compiled as IN followed by NOT.
            case SqlPlanner.SqlExpr.NotIn notIn -> {
                var result = new ArrayList<Instruction>();
                result.addAll(compileExprCtx(notIn.value(), ctx));
                for (var item : notIn.items()) result.addAll(compileExprCtx(item, ctx));
                result.add(new Instruction.InList(notIn.items().size()));
                result.add(new Instruction.UnaryOp(UnaryOpCode.NOT));
                yield result;
            }

            // ── LIKE ──────────────────────────────────────────────────────
            // Pattern is a String literal in the AST; push it as a LoadConst.
            case SqlPlanner.SqlExpr.Like like -> {
                var result = new ArrayList<Instruction>(compileExprCtx(like.value(), ctx));
                result.add(new Instruction.LoadConst(like.pattern()));
                result.add(new Instruction.Like(false));
                yield result;
            }

            // ── NOT LIKE ─────────────────────────────────────────────────
            case SqlPlanner.SqlExpr.NotLike nl -> {
                var result = new ArrayList<Instruction>(compileExprCtx(nl.value(), ctx));
                result.add(new Instruction.LoadConst(nl.pattern()));
                result.add(new Instruction.Like(true));
                yield result;
            }

            // ── Scalar function call ───────────────────────────────────────
            case SqlPlanner.SqlExpr.FuncCall fc -> {
                var result = new ArrayList<Instruction>();
                for (var arg : fc.args()) result.addAll(compileExprCtx(arg, ctx));
                result.add(new Instruction.CallScalar(fc.name().toLowerCase(), fc.args().size()));
                yield result;
            }

            // ── Aggregate expression ───────────────────────────────────────
            // AggExpr inside a non-aggregate context: compile the argument expression.
            // (The aggregate wrapping is handled at the Aggregate plan node level.)
            case SqlPlanner.SqlExpr.AggExpr agg -> {
                if (agg.arg() instanceof SqlPlanner.AggArg.Expr e) {
                    yield compileExprCtx(e.expression(), ctx);
                }
                yield List.of(new Instruction.LoadConst(null));
            }

            // ── Wildcard: no instructions ─────────────────────────────────
            case SqlPlanner.SqlExpr.Wildcard ignored -> List.of();
        };
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Helpers
    // ═══════════════════════════════════════════════════════════════════════════

    // ── Build schema from OutputColumn list ───────────────────────────────────

    private static List<String> buildSchema(List<SqlPlanner.OutputColumn> columns) {
        var schema = new ArrayList<String>();
        for (var col : columns) {
            switch (col) {
                case SqlPlanner.OutputColumn.Expr e ->
                    schema.add(e.alias() != null ? e.alias()
                        : inferColumnName(e.expression()));
                case SqlPlanner.OutputColumn.Star s -> schema.add("*");
            }
        }
        return Collections.unmodifiableList(schema);
    }

    /** Best-effort column name for an output expression without an alias. */
    private static String inferColumnName(SqlPlanner.SqlExpr expr) {
        if (expr instanceof SqlPlanner.SqlExpr.Column col) return col.column();
        if (expr instanceof SqlPlanner.SqlExpr.AggExpr agg) {
            return agg.func().name().toLowerCase()
                + (agg.arg() instanceof SqlPlanner.AggArg.Expr e
                    ? "_" + inferColumnName(e.expression())
                    : "_star");
        }
        return "?";
    }

    // ── Emit project columns for regular SELECT ───────────────────────────────

    private static void emitProjectColumns(List<SqlPlanner.OutputColumn> columns,
                                           List<String> schema,
                                           List<Instruction> out,
                                           Ctx ctx) {
        for (int i = 0; i < columns.size(); i++) {
            var col = columns.get(i);
            if (col instanceof SqlPlanner.OutputColumn.Expr e) {
                out.addAll(compileExprCtx(e.expression(), ctx));
                out.add(new Instruction.EmitColumn(schema.get(i)));
            }
            // Star columns are not emitted here; the planner should have resolved them.
        }
    }

    // ── Emit project columns for aggregate SELECT ─────────────────────────────
    //
    // Maps each output column to either:
    //   • LoadGroupKey(i) for GROUP BY columns
    //   • FinalizeAgg(slot, func) for aggregate expressions

    private static void emitAggProjectColumns(
            List<SqlPlanner.OutputColumn> projCols,
            List<String> schema,
            List<SqlPlanner.SqlExpr> groupBy,
            List<SqlPlanner.AggregateItem> aggItems,
            List<Integer> aggSlots,
            List<Instruction> out,
            Ctx ctx) {

        for (int i = 0; i < projCols.size(); i++) {
            var col = projCols.get(i);
            if (!(col instanceof SqlPlanner.OutputColumn.Expr e)) continue;

            // Try to match to an aggregate expression.
            int aggIdx = findAggIndex(e.expression(), aggItems);
            if (aggIdx >= 0) {
                var item = aggItems.get(aggIdx);
                AggFunc func = mapAggFunc(item.func(), item.arg());
                out.add(new Instruction.FinalizeAgg(aggSlots.get(aggIdx), func));
                out.add(new Instruction.EmitColumn(schema.get(i)));
                continue;
            }

            // Try to match to a GROUP BY key by column name / position.
            int gbIdx = findGroupByIndex(e.expression(), groupBy);
            if (gbIdx >= 0) {
                out.add(new Instruction.LoadGroupKey(gbIdx));
                out.add(new Instruction.EmitColumn(schema.get(i)));
                continue;
            }

            // Fall back: compile the expression directly.
            out.addAll(compileExprCtx(e.expression(), ctx));
            out.add(new Instruction.EmitColumn(schema.get(i)));
        }
    }

    /** Find the index of an aggregate that matches the given expression. */
    private static int findAggIndex(SqlPlanner.SqlExpr expr,
                                    List<SqlPlanner.AggregateItem> aggItems) {
        // Match by alias (the planner tags aggregates with synthetic aliases like "_agg0").
        // Also match AggExpr nodes by function + arg equivalence.
        if (expr instanceof SqlPlanner.SqlExpr.AggExpr ae) {
            for (int i = 0; i < aggItems.size(); i++) {
                var item = aggItems.get(i);
                if (item.func() == ae.func() && item.distinct() == ae.distinct()) {
                    // Rough arg match
                    boolean argMatch = (item.arg() instanceof SqlPlanner.AggArg.Star
                                         && ae.arg() instanceof SqlPlanner.AggArg.Star)
                        || (item.arg() instanceof SqlPlanner.AggArg.Expr ia
                            && ae.arg() instanceof SqlPlanner.AggArg.Expr ea
                            && ia.expression().equals(ea.expression()));
                    if (argMatch) return i;
                }
            }
        }
        // Match by column reference that looks like an aggregate alias "_aggN".
        if (expr instanceof SqlPlanner.SqlExpr.Column col && col.column() != null) {
            for (int i = 0; i < aggItems.size(); i++) {
                if (col.column().equals(aggItems.get(i).alias())) return i;
            }
        }
        return -1;
    }

    /** Find the GROUP BY index of a given expression. */
    private static int findGroupByIndex(SqlPlanner.SqlExpr expr,
                                        List<SqlPlanner.SqlExpr> groupBy) {
        for (int i = 0; i < groupBy.size(); i++) {
            if (groupBy.get(i).equals(expr)) return i;
            // Also match by column name alone (without table qualifier).
            if (expr instanceof SqlPlanner.SqlExpr.Column c1
                && groupBy.get(i) instanceof SqlPlanner.SqlExpr.Column c2
                && c1.column().equals(c2.column())) {
                return i;
            }
        }
        return -1;
    }

    // ── Map SqlPlanner sort keys → SortKey records ────────────────────────────

    private static List<SortKey> buildSortKeys(List<SqlPlanner.SortKey> keys) {
        var result = new ArrayList<SortKey>();
        for (var k : keys) {
            String col;
            if (k.keyExpr() instanceof SqlPlanner.SqlExpr.Column c) {
                col = c.column();
            } else {
                // Non-column sort key (e.g. expression): use a placeholder.
                col = "?";
            }
            Direction dir = k.direction() == SqlPlanner.SortDir.ASC
                            ? Direction.ASC : Direction.DESC;
            NullsOrder nullsOrd = k.nullOrder() == SqlPlanner.NullOrder.NULLS_FIRST
                                  ? NullsOrder.FIRST : NullsOrder.LAST;
            result.add(new SortKey(col, dir, nullsOrd));
        }
        return Collections.unmodifiableList(result);
    }

    // ── Map BinaryOperator → BinaryOpCode ────────────────────────────────────

    private static BinaryOpCode mapBinaryOp(SqlPlanner.BinaryOperator op) {
        return switch (op) {
            case ADD    -> BinaryOpCode.ADD;
            case SUB    -> BinaryOpCode.SUB;
            case MUL    -> BinaryOpCode.MUL;
            case DIV    -> BinaryOpCode.DIV;
            case MOD    -> BinaryOpCode.MOD;
            case EQ     -> BinaryOpCode.EQ;
            case NOT_EQ -> BinaryOpCode.NEQ;
            case LT     -> BinaryOpCode.LT;
            case LTE    -> BinaryOpCode.LTE;
            case GT     -> BinaryOpCode.GT;
            case GTE    -> BinaryOpCode.GTE;
            case AND    -> BinaryOpCode.AND;
            case OR     -> BinaryOpCode.OR;
        };
    }

    // ── Map UnaryOperator → UnaryOpCode ──────────────────────────────────────

    private static UnaryOpCode mapUnaryOp(SqlPlanner.UnaryOperator op) {
        return switch (op) {
            case NEG -> UnaryOpCode.NEG;
            case NOT -> UnaryOpCode.NOT;
        };
    }

    // ── Map AggFunction → AggFunc ─────────────────────────────────────────────

    private static AggFunc mapAggFunc(SqlPlanner.AggFunction func, SqlPlanner.AggArg arg) {
        return switch (func) {
            case COUNT -> arg instanceof SqlPlanner.AggArg.Star
                          ? AggFunc.COUNT_STAR : AggFunc.COUNT;
            case SUM   -> AggFunc.SUM;
            case AVG   -> AggFunc.AVG;
            case MIN   -> AggFunc.MIN;
            case MAX   -> AggFunc.MAX;
        };
    }
}

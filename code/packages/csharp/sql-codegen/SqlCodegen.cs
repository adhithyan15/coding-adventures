// SqlCodegen.cs — bytecode code generator for the Mini-SQLite Level 1 pipeline.
//
// Compiles an OptimizedPlan tree (from sql-optimizer) into a flat, linear
// sequence of bytecode instructions (Program) that the SQL VM can execute.
//
// The compilation proceeds in three major phases:
//
//   1. Peel wrappers  — Strip Sort/Limit/Distinct nodes from the outer plan,
//                       collecting them as "post-ops" to append after the main
//                       scan body.
//
//   2. CompileCore    — Emit the central computation (scan loops, joins,
//                       aggregation, DML, DDL).
//
//   3. Post-ops       — Append SortResult, LimitResult, DistinctResult in the
//                       correct order (Sort before Limit before Distinct).
//
// The compilation context (Ctx) manages three counters:
//   • CursorId — each OpenScan/CloseScan pair gets a unique integer identifier
//   • Label    — each branch target gets a unique string label
//   • Slot     — each aggregate accumulator gets a unique integer slot
//
// Instructions are collected in a plain List<Instruction>; labels are resolved
// to instruction indices in a final post-pass over the list.
//
// Usage:
//   LogicalPlan  logical  = planner.Plan(stmt);
//   OptimizedPlan opt     = SqlOptimizer.Optimize(logical);
//   Program       program = SqlCodegen.CompileOptimized(opt);
//
// No I/O, no database access — pure in-memory tree → list transformation.

using CodingAdventures.SqlPlanner;
using CodingAdventures.SqlOptimizer;
using Optimizer = CodingAdventures.SqlOptimizer.SqlOptimizer;

// Bring SqlExpr subtypes into scope unqualified so the compiler
// can disambiguate them from the Instruction records that share names.
// The instruction records BinaryOpInstr and UnaryOpInstr are deliberately
// renamed to avoid the clash with SqlExpr.BinaryOp / SqlExpr.UnaryOp.
using PlBinaryOp  = CodingAdventures.SqlPlanner.SqlExpr.BinaryOp;
using PlUnaryOp   = CodingAdventures.SqlPlanner.SqlExpr.UnaryOp;
using PlIsNull    = CodingAdventures.SqlPlanner.SqlExpr.IsNull;
using PlIsNotNull = CodingAdventures.SqlPlanner.SqlExpr.IsNotNull;
using PlBetween   = CodingAdventures.SqlPlanner.SqlExpr.Between;
using PlIn        = CodingAdventures.SqlPlanner.SqlExpr.In;
using PlNotIn     = CodingAdventures.SqlPlanner.SqlExpr.NotIn;
using PlLike      = CodingAdventures.SqlPlanner.SqlExpr.Like;
using PlNotLike   = CodingAdventures.SqlPlanner.SqlExpr.NotLike;

namespace CodingAdventures.SqlCodegen;

// ── Enumerations ──────────────────────────────────────────────────────────────
//
// Each enum mirrors the corresponding concept in the planner/optimizer, but at
// the VM instruction level.  The VM only knows about opcodes and slots — it
// never sees LogicalPlan or OptimizedPlan.

/// <summary>Binary arithmetic/comparison/logical opcodes emitted by the codegen.</summary>
public enum BinaryOpCode
{
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Lte, Gt, Gte,
    And, Or,
    Concat
}

/// <summary>Unary prefix opcodes.</summary>
public enum UnaryOpCode { Neg, Not }

/// <summary>Aggregate accumulator functions.</summary>
public enum AggFunc { Count, CountStar, Sum, Avg, Min, Max }

/// <summary>Sort direction for ORDER BY keys.</summary>
public enum Direction { Asc, Desc }

/// <summary>NULL placement for ORDER BY keys.</summary>
public enum NullsOrder { First, Last }

// ── Supporting records ────────────────────────────────────────────────────────

/// <summary>One sort key in a SortResult instruction.</summary>
public sealed record CodegenSortKey(string Column, Direction Direction, NullsOrder NullsOrder);

// ── Instruction hierarchy ─────────────────────────────────────────────────────
//
// Instructions are abstract records so that tests can pattern-match on them
// and equality is structural (record semantics).  The hierarchy deliberately
// avoids inheriting from Exception or any framework type — it is a plain ADT.

/// <summary>Base type for all bytecode instructions.</summary>
public abstract record Instruction;

// ── Stack / value instructions ────────────────────────────────────────────────

/// <summary>Push a compile-time constant (int, long, double, string, bool, null) onto the stack.</summary>
public sealed record LoadConst(object? Value) : Instruction;

/// <summary>Push the value of a column from the given cursor's current row.</summary>
public sealed record LoadColumn(int CursorId, string Column) : Instruction;

/// <summary>Discard the top stack value.</summary>
public sealed record Pop : Instruction;

// ── Operator instructions ─────────────────────────────────────────────────────

/// <summary>
/// Pop two values, apply the binary operation, push the result.
/// The right operand is the top of the stack, the left operand is below it.
/// </summary>
public sealed record BinaryOpInstr(BinaryOpCode Op) : Instruction;

/// <summary>Pop one value, apply the unary operation, push the result.</summary>
public sealed record UnaryOpInstr(UnaryOpCode Op) : Instruction;

/// <summary>Pop one value, push true if it is SQL NULL, false otherwise.</summary>
public sealed record IsNullInstr : Instruction;

/// <summary>Pop one value, push false if it is SQL NULL, true otherwise.</summary>
public sealed record IsNotNullInstr : Instruction;

/// <summary>
/// Pop three values (high, low, value) from the stack, push true if value BETWEEN low AND high.
/// Stack order: value pushed first, then low, then high.
/// </summary>
public sealed record BetweenInstr : Instruction;

/// <summary>Pop N+1 values (N items then the probe value) and push true if the probe is IN the list.</summary>
public sealed record InListInstr(int N) : Instruction;

/// <summary>Evaluate LIKE / NOT LIKE.  The pattern is embedded at compile time.</summary>
public sealed record LikeInstr(bool Negated) : Instruction;

/// <summary>Pop NArgs values and call a named scalar function, push the result.</summary>
public sealed record CallScalar(string Func, int NArgs) : Instruction;

// ── Cursor instructions ───────────────────────────────────────────────────────

/// <summary>Open a forward-only cursor on the named table.</summary>
public sealed record OpenScan(int CursorId, string Table) : Instruction;

/// <summary>
/// Advance the cursor to the next row.
/// If the cursor is exhausted, jump to <see cref="OnExhausted"/>.
/// </summary>
public sealed record AdvanceCursor(int CursorId, string OnExhausted) : Instruction;

/// <summary>Release the cursor and its associated resources.</summary>
public sealed record CloseScan(int CursorId) : Instruction;

// ── Row construction instructions ─────────────────────────────────────────────

/// <summary>Begin assembling a new output row.</summary>
public sealed record BeginRow : Instruction;

/// <summary>Pop the top stack value and append it as the named column of the current output row.</summary>
public sealed record EmitColumn(string Name) : Instruction;

/// <summary>Finalise the current output row and deliver it to the result set.</summary>
public sealed record EmitRow : Instruction;

/// <summary>Record the schema (column names) of the result set.</summary>
public sealed record SetResultSchema(IReadOnlyList<string> Columns) : Instruction;

// ── Aggregate instructions ────────────────────────────────────────────────────
//
// Two-phase aggregation:
//   Phase 1 (scan body): InitAgg + [arg expression] + UpdateAgg for each group row.
//   Phase 2 (emit body): AdvanceGroupKey + FinalizeAgg + EmitColumn per group.

/// <summary>Initialise (clear) aggregate slot <see cref="Slot"/> for function <see cref="Func"/>.</summary>
public sealed record InitAgg(int Slot, AggFunc Func, bool Distinct) : Instruction;

/// <summary>Pop the top value and feed it into aggregate slot <see cref="Slot"/>.</summary>
public sealed record UpdateAgg(int Slot) : Instruction;

/// <summary>Finalise aggregate slot <see cref="Slot"/> and push its result.</summary>
public sealed record FinalizeAgg(int Slot, AggFunc Func) : Instruction;

/// <summary>Save the top N stack values as the current group key.</summary>
public sealed record SaveGroupKey(int N) : Instruction;

/// <summary>Push the I-th saved group key value onto the stack.</summary>
public sealed record LoadGroupKey(int I) : Instruction;

/// <summary>
/// Advance to the next group.
/// If all groups have been emitted, jump to <see cref="OnExhausted"/>.
/// </summary>
public sealed record AdvanceGroupKey(string OnExhausted, bool HasGroupBy) : Instruction;

// ── Sort/Limit/Distinct post-op instructions ──────────────────────────────────

/// <summary>Sort the entire result set in memory by the given keys.</summary>
public sealed record SortResult(IReadOnlyList<CodegenSortKey> Keys) : Instruction;

/// <summary>Apply LIMIT/OFFSET to the result set.</summary>
public sealed record LimitResult(long? Count, long? Offset) : Instruction;

/// <summary>Deduplicate the result set (DISTINCT).</summary>
public sealed record DistinctResult : Instruction;

// ── Join instructions ─────────────────────────────────────────────────────────

/// <summary>Begin a LEFT JOIN row (marks that no right-side match has been found yet).</summary>
public sealed record JoinBeginRow : Instruction;

/// <summary>Record that a right-side match was found for the current LEFT JOIN row.</summary>
public sealed record JoinSetMatched : Instruction;

/// <summary>Jump to <see cref="Label"/> if the current LEFT JOIN row already matched.</summary>
public sealed record JoinIfMatched(string Label) : Instruction;

// ── DML instructions ──────────────────────────────────────────────────────────

/// <summary>Pop column values from the stack (in order) and insert a new row.</summary>
public sealed record InsertRow(string Table, IReadOnlyList<string> Columns) : Instruction;

/// <summary>Update the current cursor row with new column values popped from the stack.</summary>
public sealed record UpdateRows(string Table, IReadOnlyList<string> Assignments, int CursorId) : Instruction;

/// <summary>Delete the row at the current cursor position.</summary>
public sealed record DeleteRows(string Table, int CursorId) : Instruction;

// ── DDL instructions ──────────────────────────────────────────────────────────

/// <summary>CREATE TABLE DDL instruction.</summary>
public sealed record CreateTableInstr(string Table, bool IfNotExists, IReadOnlyList<ColumnDef> Columns) : Instruction;

/// <summary>DROP TABLE DDL instruction.</summary>
public sealed record DropTableInstr(string Table, bool IfExists) : Instruction;

// ── Control flow ──────────────────────────────────────────────────────────────

/// <summary>Define a label at this position in the instruction stream.</summary>
public sealed record CodegenLabel(string Name) : Instruction;

/// <summary>Unconditional jump to the named label.</summary>
public sealed record Jump(string Target) : Instruction;

/// <summary>Pop the top value; jump to <see cref="Target"/> if it is falsy.</summary>
public sealed record JumpIfFalse(string Target) : Instruction;

/// <summary>Pop the top value; jump to <see cref="Target"/> if it is truthy.</summary>
public sealed record JumpIfTrue(string Target) : Instruction;

/// <summary>Stop execution.  Any pending result rows are flushed to the caller.</summary>
public sealed record Halt : Instruction;

// ── Program ───────────────────────────────────────────────────────────────────

/// <summary>
/// A compiled bytecode program.
/// <list type="bullet">
///   <item><term>Instructions</term><description>flat, ordered list of opcodes</description></item>
///   <item><term>Labels</term><description>mapping from label name to its index in Instructions</description></item>
///   <item><term>ResultSchema</term><description>ordered column names of the result set (empty for DML/DDL)</description></item>
/// </list>
/// </summary>
public sealed record Program(
    IReadOnlyList<Instruction>         Instructions,
    IReadOnlyDictionary<string, int>   Labels,
    IReadOnlyList<string>              ResultSchema);

// ── Compilation context ───────────────────────────────────────────────────────
//
// The Ctx is private to the compilation; tests never touch it directly.

internal sealed class Ctx
{
    // Monotonically increasing counters for unique identifiers.
    private int _cursorCounter;
    private int _labelCounter;
    private int _slotCounter;

    // Maps table alias (or table name when no alias) → cursor id.
    private readonly Dictionary<string, int> _aliasToId = new(StringComparer.OrdinalIgnoreCase);

    /// <summary>Allocate the next cursor id.</summary>
    public int NextCursor() => _cursorCounter++;

    /// <summary>Allocate a unique label with the given prefix.</summary>
    public string NextLabel(string prefix) => $"{prefix}_{_labelCounter++}";

    /// <summary>Allocate the next aggregate slot index.</summary>
    public int NextSlot() => _slotCounter++;

    /// <summary>Bind a table alias to a cursor id so expression compilation can look it up.</summary>
    public void RegisterAlias(string alias, int cursorId) => _aliasToId[alias] = cursorId;

    /// <summary>
    /// Return the cursor id bound to the given alias, or 0 if unknown.
    /// Unknown is safe because the VM resolves column names dynamically;
    /// a 0 cursor just means "first open cursor" which is correct for
    /// single-table queries that omit the table qualifier.
    /// </summary>
    public int CursorOf(string? alias) =>
        alias is null ? 0 : _aliasToId.GetValueOrDefault(alias, 0);
}

// ── Main compiler ─────────────────────────────────────────────────────────────

/// <summary>
/// Compiles OptimizedPlan trees into executable Program bytecode.
/// All methods are pure static transformations — no I/O, no side effects.
/// </summary>
public static class SqlCodegen
{
    // ── Public API ────────────────────────────────────────────────────────────

    /// <summary>
    /// Convenience overload: lift + optimize a LogicalPlan, then compile.
    /// Equivalent to <c>CompileOptimized(SqlOptimizer.Optimize(plan))</c>.
    ///
    /// When the optimizer produces OptEmptyResult (e.g. for LIMIT 0 or WHERE FALSE),
    /// the result schema is normally lost.  We recover it by walking the original
    /// logical plan for a ProjectPlan node before optimization destroys it.
    /// </summary>
    public static Program Compile(LogicalPlan plan)
    {
        var compiled = CompileOptimized(Optimizer.Optimize(plan));

        // If the compiled program has no schema (OptEmptyResult erased it), try to
        // recover the column names from the top-level ProjectPlan in the logical plan.
        if (compiled.ResultSchema.Count == 0)
        {
            var recoveredSchema = ExtractSchemaFromLogical(plan);
            if (recoveredSchema is { Count: > 0 })
            {
                // Inject a SetResultSchema at the front so the VM picks it up.
                var instrWithSchema = new List<Instruction> { new SetResultSchema(recoveredSchema) };
                instrWithSchema.AddRange(compiled.Instructions);
                return compiled with { Instructions = instrWithSchema, ResultSchema = recoveredSchema };
            }
        }

        return compiled;
    }

    // Walk the logical plan tree looking for a ProjectPlan and extract its column names.
    // Returns null if no project is found or the project has no named columns.
    private static IReadOnlyList<string>? ExtractSchemaFromLogical(LogicalPlan plan)
    {
        // Walk down the spine (Sort, Limit, Distinct, Filter wrap Project).
        var current = plan;
        while (true)
        {
            switch (current)
            {
                case ProjectPlan(var input, var cols):
                    return ExtractProjectedNames(cols);
                case SortPlan(var input, _):
                    current = input;
                    break;
                case LimitPlan(var input, _, _):
                    current = input;
                    break;
                case DistinctPlan(var input):
                    current = input;
                    break;
                case FilterPlan(var input, _):
                    current = input;
                    break;
                default:
                    return null;
            }
        }
    }

    /// <summary>
    /// Main entry point.  Compile an already-optimized plan into a Program.
    /// </summary>
    public static Program CompileOptimized(OptimizedPlan plan)
    {
        var ctx  = new Ctx();
        var out_ = new List<Instruction>();

        // ── Phase 1: peel Sort / Limit / Distinct wrappers ────────────────────
        //
        // These operators are post-ops that run on the completed result set,
        // not on individual rows.  We strip them from the outer plan so that
        // CompileCore sees only the "data-producing" inner plan, then append
        // the post-op instructions after the scan body.
        //
        // Canonical order: Sort → Limit → Distinct.

        var postOps = new List<Instruction>();
        var inner   = PeelWrappers(plan, postOps);

        // ── Phase 2: compile the core plan ────────────────────────────────────
        CompileCore(inner, out_, ctx);

        // ── Phase 3: append post-ops, then Halt ───────────────────────────────
        out_.AddRange(postOps);
        out_.Add(new Halt());

        // ── Phase 4: resolve labels ───────────────────────────────────────────
        //
        // Scan the instruction list and collect CodegenLabel positions.
        // The VM uses the dictionary to resolve jump targets.
        var labels = new Dictionary<string, int>(StringComparer.Ordinal);
        for (var i = 0; i < out_.Count; i++)
        {
            if (out_[i] is CodegenLabel lbl)
                labels[lbl.Name] = i;
        }

        // ── Phase 5: extract result schema ────────────────────────────────────
        //
        // The first SetResultSchema instruction (if any) declares the column names.
        var schema = out_
            .OfType<SetResultSchema>()
            .FirstOrDefault()
            ?.Columns
            ?? Array.Empty<string>();

        return new Program(out_, labels, schema);
    }

    /// <summary>
    /// Compile a standalone expression into a list of instructions.
    /// Useful for unit tests and the REPL.
    /// </summary>
    public static IReadOnlyList<Instruction> CompileExpr(SqlExpr expr)
    {
        var ctx = new Ctx();
        return CompileExprInCtx(expr, ctx).ToList();
    }

    // ── Wrapper peeling ───────────────────────────────────────────────────────
    //
    // Walk the outer spine of the plan, collecting Sort/Limit/Distinct
    // post-ops in canonical order.  Stop when we hit any other plan node.

    private static OptimizedPlan PeelWrappers(OptimizedPlan plan, List<Instruction> postOps)
    {
        // Collect all wrappers first so we can emit them in the right order.
        // We process the outer spine and accumulate operations bottom-up.

        var sortOps     = new List<SortResult>();
        var limitOps    = new List<LimitResult>();
        var distinctOps = new List<DistinctResult>();

        var current = plan;
        while (true)
        {
            switch (current)
            {
                case OptSort(var input, var keys):
                    sortOps.Add(new SortResult(keys.Select(k => new CodegenSortKey(
                        ExtractSortKeyColumn(k),
                        k.Direction == SortDir.Asc ? Direction.Asc : Direction.Desc,
                        k.NullOrder  == NullOrder.NullsFirst ? NullsOrder.First : NullsOrder.Last
                    )).ToList()));
                    current = input;
                    break;

                case OptLimit(var input, var count, var offset):
                    limitOps.Add(new LimitResult(count, offset));
                    current = input;
                    break;

                case OptDistinct(var input):
                    distinctOps.Add(new DistinctResult());
                    current = input;
                    break;

                default:
                    goto done;
            }
        }
        done:

        // Emit post-ops: Sort first (must order before limiting), then Limit, then Distinct.
        postOps.AddRange(sortOps);
        postOps.AddRange(limitOps);
        postOps.AddRange(distinctOps);

        return current;
    }

    // Extract a column name string from a SortKey expression.
    // For a bare Column reference we use the column name; for a more complex
    // expression we emit a placeholder that the VM resolves by expression index.
    private static string ExtractSortKeyColumn(SortKey key)
    {
        return key.KeyExpr is SqlExpr.Column(_, var col) ? col : $"__sort_expr_{key.KeyExpr}";
    }

    // ── Core compilation ──────────────────────────────────────────────────────
    //
    // Dispatches on the top-level plan node after wrapper peeling.

    private static void CompileCore(OptimizedPlan plan, List<Instruction> out_, Ctx ctx)
    {
        switch (plan)
        {
            // ── DDL nodes — emit and done ─────────────────────────────────────
            case OptCreateTable(var table, var ifNotExists, var cols):
                out_.Add(new CreateTableInstr(table, ifNotExists, cols));
                return; // Halt appended by caller

            case OptDropTable(var table, var ifExists):
                out_.Add(new DropTableInstr(table, ifExists));
                return;

            // ── DML nodes ─────────────────────────────────────────────────────
            case OptInsert(var table, var colNames, var rows):
                CompileInsert(table, colNames, rows, out_, ctx);
                return;

            case OptUpdate(var table, var assignments, var predicate):
                CompileUpdate(table, assignments, predicate, out_, ctx);
                return;

            case OptDelete(var table, var predicate):
                CompileDelete(table, predicate, out_, ctx);
                return;

            // ── EmptyResult — no rows produced ────────────────────────────────
            case OptEmptyResult:
                // Nothing to emit — Halt will be appended by the caller.
                return;

            // ── Aggregate ─────────────────────────────────────────────────────
            case OptAggregate agg:
                CompileAggregate(agg, null, out_, ctx);
                return;

            // ── Project wrapping Aggregate — common pattern ───────────────────
            case OptProject(OptAggregate innerAgg, var projCols):
            {
                var colNames = ExtractProjectedNames(projCols);
                out_.Add(new SetResultSchema(colNames));
                CompileAggregate(innerAgg, projCols, out_, ctx);
                return;
            }

            // ── Project wrapping HAVING wrapping Aggregate (GROUP BY + HAVING) ──
            //
            // When a query has GROUP BY + HAVING, the planner emits:
            //   ProjectPlan(HavingPlan(AggregatePlan(...), pred), cols)
            // The optimizer lifts this to OptProject(OptHaving(OptAggregate), cols).
            // We must compile the aggregate first, then apply the HAVING predicate
            // as a filter on each emitted group row.
            case OptProject(OptHaving(OptAggregate innerAgg2, var havingPred), var projCols2):
            {
                var colNames = ExtractProjectedNames(projCols2);
                out_.Add(new SetResultSchema(colNames));
                CompileAggregateWithHaving(innerAgg2, havingPred, projCols2, out_, ctx);
                return;
            }

            // ── Project (general case) ────────────────────────────────────────
            case OptProject(var input, var projCols):
            {
                var colNames = ExtractProjectedNames(projCols);
                out_.Add(new SetResultSchema(colNames));
                CompileScanBody(input, out_, ctx, () =>
                {
                    out_.Add(new BeginRow());
                    EmitProjectColumns(projCols, colNames, out_, ctx);
                    out_.Add(new EmitRow());
                });
                return;
            }

            // ── Bare scan (no project) ────────────────────────────────────────
            case OptScan:
            case OptFilter:
            case OptJoin:
            case OptHaving:
                CompileScanBody(plan, out_, ctx, () =>
                {
                    // No project — we cannot emit columns without a schema.
                    // Emit an EmitRow so the VM knows to materialise the row.
                    out_.Add(new EmitRow());
                });
                return;

            default:
                throw new InvalidOperationException(
                    $"CompileCore: unsupported plan node {plan.GetType().Name}");
        }
    }

    // ── Scan body compilation ─────────────────────────────────────────────────
    //
    // Generates the nested-loop scan structure for all data-producing plan nodes.
    // The caller supplies a body Action that is invoked inside every innermost loop.

    private static void CompileScanBody(
        OptimizedPlan         plan,
        List<Instruction>     out_,
        Ctx                   ctx,
        Action                body)
    {
        switch (plan)
        {
            // ── Table scan ────────────────────────────────────────────────────
            //
            // Structure:
            //   OpenScan(cid, table)
            //   loop_label:
            //     AdvanceCursor(cid, end_label)
            //     <body>
            //     Jump(loop_label)
            //   end_label:
            //   CloseScan(cid)
            case OptScan(var table, var alias, _, _):
            {
                var cid      = ctx.NextCursor();
                var scanAlias = alias ?? table;
                ctx.RegisterAlias(scanAlias, cid);

                var loop = ctx.NextLabel($"scan_{cid}_loop");
                var end  = ctx.NextLabel($"scan_{cid}_end");

                out_.Add(new OpenScan(cid, table));
                out_.Add(new CodegenLabel(loop));
                out_.Add(new AdvanceCursor(cid, end));
                body();
                out_.Add(new Jump(loop));
                out_.Add(new CodegenLabel(end));
                out_.Add(new CloseScan(cid));
                break;
            }

            // ── Filter ────────────────────────────────────────────────────────
            //
            // The predicate is evaluated inside the scan loop.  Rows that fail
            // the predicate jump over the body.
            case OptFilter(var input, var predicate):
            {
                var skip = ctx.NextLabel("filter_skip");
                CompileScanBody(input, out_, ctx, () =>
                {
                    out_.AddRange(CompileExprInCtx(predicate, ctx));
                    out_.Add(new JumpIfFalse(skip));
                    body();
                    out_.Add(new CodegenLabel(skip));
                });
                break;
            }

            // ── INNER / CROSS JOIN — nested loops ─────────────────────────────
            //
            // For every row in the left table, iterate all rows in the right table.
            // If there is a join condition, rows that fail it are skipped.
            case OptJoin(var left, var right, var kind, var cond)
                when kind is JoinKind.Inner or JoinKind.Cross:
            {
                CompileScanBody(left, out_, ctx, () =>
                    CompileScanBody(right, out_, ctx, () =>
                    {
                        if (cond is null)
                        {
                            body();
                        }
                        else
                        {
                            var skip = ctx.NextLabel("join_skip");
                            out_.AddRange(CompileExprInCtx(cond, ctx));
                            out_.Add(new JumpIfFalse(skip));
                            body();
                            out_.Add(new CodegenLabel(skip));
                        }
                    })
                );
                break;
            }

            // ── LEFT OUTER JOIN ───────────────────────────────────────────────
            //
            // For every left row, try all right rows.  If at least one right row
            // satisfied the join condition, we emit normal joined rows.  If none
            // did, we emit the left row padded with NULLs on the right side.
            //
            // Structure:
            //   foreach left_row:
            //     JoinBeginRow()           ; reset "matched" flag
            //     foreach right_row:
            //       [optional condition check → skip]
            //       JoinSetMatched()
            //       <body>
            //       [cond_skip:]
            //     JoinIfMatched(matched)   ; if matched, skip null-padded emit
            //     <body>                   ; null-padded emit
            //     matched:
            case OptJoin(var left, var right, JoinKind.Left, var cond):
            {
                var matched = ctx.NextLabel("loj_matched");
                CompileScanBody(left, out_, ctx, () =>
                {
                    out_.Add(new JoinBeginRow());
                    CompileScanBody(right, out_, ctx, () =>
                    {
                        string? condSkip = null;
                        if (cond is not null)
                        {
                            condSkip = ctx.NextLabel("loj_cond_skip");
                            out_.AddRange(CompileExprInCtx(cond, ctx));
                            out_.Add(new JumpIfFalse(condSkip));
                        }
                        out_.Add(new JoinSetMatched());
                        body();
                        if (condSkip is not null)
                            out_.Add(new CodegenLabel(condSkip));
                    });
                    out_.Add(new JoinIfMatched(matched));
                    body(); // null-padded emit
                    out_.Add(new CodegenLabel(matched));
                });
                break;
            }

            // ── EmptyResult — body is never called ────────────────────────────
            case OptEmptyResult:
                break; // intentionally empty

            // ── Having — pass through ─────────────────────────────────────────
            //
            // HAVING is compiled as a filter inside the scan body.
            // (This path is reached when a HavingPlan survives optimization.)
            case OptHaving(var input, var predicate):
            {
                var skip = ctx.NextLabel("having_skip");
                CompileScanBody(input, out_, ctx, () =>
                {
                    out_.AddRange(CompileExprInCtx(predicate, ctx));
                    out_.Add(new JumpIfFalse(skip));
                    body();
                    out_.Add(new CodegenLabel(skip));
                });
                break;
            }

            default:
                throw new InvalidOperationException(
                    $"CompileScanBody: unsupported plan node {plan.GetType().Name}");
        }
    }

    // ── Aggregate compilation ─────────────────────────────────────────────────
    //
    // Two-phase approach:
    //
    //   Phase 1 (accumulation scan):
    //     For each row produced by agg.Input, save the group key and
    //     feed each aggregate function's argument value into its slot.
    //
    //   Phase 2 (emit groups):
    //     Iterate over the accumulated groups.  For each group, emit
    //     the group key columns and the finalised aggregate values.
    //
    // projCols is the Project's output column list — it provides the
    // human-readable names for the emitted columns.  It may be null when
    // the aggregate is compiled stand-alone (no wrapping Project).

    private static void CompileAggregate(
        OptAggregate          agg,
        IReadOnlyList<OutputColumn>? projCols,
        List<Instruction>     out_,
        Ctx                   ctx)
    {
        // Allocate one slot per aggregate function.
        var slots = agg.Aggregates.Select(_ => ctx.NextSlot()).ToList();

        // ── Phase 1: accumulation scan ────────────────────────────────────────
        CompileScanBody(agg.Input, out_, ctx, () =>
        {
            // Evaluate group-by keys and save them.
            foreach (var gb in agg.GroupBy)
                out_.AddRange(CompileExprInCtx(gb, ctx));
            out_.Add(new SaveGroupKey(agg.GroupBy.Count));

            // For each aggregate: initialise, compile argument, accumulate.
            for (var i = 0; i < agg.Aggregates.Count; i++)
            {
                var item = agg.Aggregates[i];

                // COUNT(*) uses AggArg.Star — map it to CountStar so the VM
                // increments the counter unconditionally (even for null values).
                // COUNT(expr) uses AggArg.Expr — map it to Count (skip nulls).
                var func = (item.Func == AggFunction.Count && item.Arg is AggArg.Star)
                    ? AggFunc.CountStar
                    : MapAggFunc(item.Func);

                out_.Add(new InitAgg(slots[i], func, item.Distinct));

                if (item.Arg is AggArg.Expr(var argExpr))
                    out_.AddRange(CompileExprInCtx(argExpr, ctx));
                else
                    out_.Add(new LoadConst(null)); // COUNT(*) — arg is star; value unused by CountStar

                out_.Add(new UpdateAgg(slots[i]));
            }
        });

        // ── Phase 2: emit groups ──────────────────────────────────────────────
        //
        // Build column names for the emitted row.  When a projCols list was
        // supplied, use its aliases; otherwise fall back to generated names.
        var colNames = projCols is not null
            ? ExtractProjectedNames(projCols)
            : BuildDefaultAggNames(agg);

        if (projCols is not null)
            out_.Add(new SetResultSchema(colNames));

        var gStart = ctx.NextLabel("group_start");
        var gEnd   = ctx.NextLabel("group_end");

        out_.Add(new CodegenLabel(gStart));
        out_.Add(new AdvanceGroupKey(gEnd, agg.GroupBy.Count > 0));
        out_.Add(new BeginRow());

        // Emit group-by key columns.
        for (var i = 0; i < agg.GroupBy.Count; i++)
        {
            // The column name: use the project's alias for the i-th column,
            // or a generated name if no project was supplied.
            var name = (projCols is not null && i < projCols.Count)
                ? GetColumnName(projCols[i], $"group_{i}")
                : $"group_{i}";
            out_.Add(new LoadGroupKey(i));
            out_.Add(new EmitColumn(name));
        }

        // Emit finalised aggregate columns.
        for (var i = 0; i < agg.Aggregates.Count; i++)
        {
            var item      = agg.Aggregates[i];
            var projIndex = agg.GroupBy.Count + i;
            var name = (projCols is not null && projIndex < projCols.Count)
                ? GetColumnName(projCols[projIndex], item.Alias)
                : item.Alias;

            // Use the same func mapping as InitAgg: COUNT(*) → CountStar.
            var finalFunc = (item.Func == AggFunction.Count && item.Arg is AggArg.Star)
                ? AggFunc.CountStar
                : MapAggFunc(item.Func);
            out_.Add(new FinalizeAgg(slots[i], finalFunc));
            out_.Add(new EmitColumn(name));
        }

        out_.Add(new EmitRow());
        out_.Add(new Jump(gStart));
        out_.Add(new CodegenLabel(gEnd));
    }

    // ── Aggregate + HAVING compilation ───────────────────────────────────────────
    //
    // For queries like SELECT region, COUNT(*) FROM sales GROUP BY region HAVING COUNT(*) > 1,
    // the plan is OptProject(OptHaving(OptAggregate(...), pred), projCols).
    //
    // Strategy: run the normal two-phase aggregate loop but add a HAVING predicate check
    // inside the emit phase (phase 2), skipping groups that don't pass the predicate.
    // The HAVING expression is re-compiled as part of the group-emit loop; because it
    // references aggregate results, we evaluate it AFTER FinalizeAgg is called.

    private static void CompileAggregateWithHaving(
        OptAggregate               agg,
        SqlExpr                    havingPred,
        IReadOnlyList<OutputColumn> projCols,
        List<Instruction>          out_,
        Ctx                        ctx)
    {
        // Phase 1: same as CompileAggregate — accumulate rows into agg slots.
        var slots = agg.Aggregates.Select(_ => ctx.NextSlot()).ToList();

        CompileScanBody(agg.Input, out_, ctx, () =>
        {
            foreach (var gb in agg.GroupBy)
                out_.AddRange(CompileExprInCtx(gb, ctx));
            out_.Add(new SaveGroupKey(agg.GroupBy.Count));

            for (var i = 0; i < agg.Aggregates.Count; i++)
            {
                var item = agg.Aggregates[i];
                var func = (item.Func == AggFunction.Count && item.Arg is AggArg.Star)
                    ? AggFunc.CountStar
                    : MapAggFunc(item.Func);

                out_.Add(new InitAgg(slots[i], func, item.Distinct));

                if (item.Arg is AggArg.Expr(var argExpr))
                    out_.AddRange(CompileExprInCtx(argExpr, ctx));
                else
                    out_.Add(new LoadConst(null));

                out_.Add(new UpdateAgg(slots[i]));
            }
        });

        // Phase 2: emit groups, applying the HAVING filter per group.
        var colNames = ExtractProjectedNames(projCols);

        var gStart = ctx.NextLabel("hagg_start");
        var gEnd   = ctx.NextLabel("hagg_end");
        var gSkip  = ctx.NextLabel("hagg_skip");

        out_.Add(new CodegenLabel(gStart));
        out_.Add(new AdvanceGroupKey(gEnd, agg.GroupBy.Count > 0));

        // Evaluate the HAVING predicate.
        //
        // We use CompileHavingExpr rather than CompileExprInCtx so that AggExpr
        // nodes (e.g. COUNT(*) in HAVING COUNT(*) > 1) are compiled to
        // FinalizeAgg(slot_i, func_i) instead of LoadConst(null).
        //
        // FinalizeAgg reads the running accumulator without consuming it, so we
        // can call it again in the emit phase below without re-accumulating.
        out_.AddRange(CompileHavingExpr(havingPred, agg, slots, ctx));
        out_.Add(new JumpIfFalse(gSkip));

        // HAVING passed — emit the group row.
        out_.Add(new BeginRow());

        // Emit group-by keys.
        for (var i = 0; i < agg.GroupBy.Count; i++)
        {
            var name = (i < projCols.Count) ? GetColumnName(projCols[i], $"group_{i}") : $"group_{i}";
            out_.Add(new LoadGroupKey(i));
            out_.Add(new EmitColumn(name));
        }

        // Emit finalized aggregate columns.
        //
        // IMPORTANT: agg.Aggregates may contain MORE items than project columns,
        // because the planner collects aggregates from both SELECT and HAVING.
        // For example, SELECT region, COUNT(*) AS n ... HAVING COUNT(*) > 1
        // gives agg.Aggregates = [_agg0 for SELECT, _agg1 for HAVING].
        //
        // We must only emit aggregate columns that map to actual project columns.
        // The number of aggregate-output columns is projCols.Count - agg.GroupBy.Count.
        var aggOutputCount = projCols.Count - agg.GroupBy.Count;
        for (var i = 0; i < aggOutputCount && i < agg.Aggregates.Count; i++)
        {
            var item      = agg.Aggregates[i];
            var projIndex = agg.GroupBy.Count + i;
            var name = (projIndex < projCols.Count)
                ? GetColumnName(projCols[projIndex], item.Alias)
                : item.Alias;

            var finalFunc2 = (item.Func == AggFunction.Count && item.Arg is AggArg.Star)
                ? AggFunc.CountStar
                : MapAggFunc(item.Func);
            out_.Add(new FinalizeAgg(slots[i], finalFunc2));
            out_.Add(new EmitColumn(name));
        }

        out_.Add(new EmitRow());

        out_.Add(new CodegenLabel(gSkip));
        out_.Add(new Jump(gStart));
        out_.Add(new CodegenLabel(gEnd));
    }

    // ── HAVING expression compiler ────────────────────────────────────────────
    //
    // Like CompileExprInCtx but replaces AggExpr nodes with FinalizeAgg instructions
    // by matching them against the AggregateItem list + slot indices from the
    // surrounding CompileAggregateWithHaving call.
    //
    // This is necessary because AggExpr nodes appear in HAVING predicates
    // (e.g. HAVING COUNT(*) > 1) and the general expression compiler has no
    // knowledge of which slot holds which aggregate.
    //
    // Matching: we compare (Func, Arg record equality, Distinct) between the
    // AggExpr in the predicate and the AggregateItem in agg.Aggregates.
    // AggArg.Star and AggArg.Expr are records so == / Equals works structurally.

    private static IEnumerable<Instruction> CompileHavingExpr(
        SqlExpr                    expr,
        OptAggregate               agg,
        IReadOnlyList<int>         slots,
        Ctx                        ctx)
    {
        // AggExpr → look up matching slot and emit FinalizeAgg.
        if (expr is SqlExpr.AggExpr(var aggFunc, var aggArg, var aggDistinct))
        {
            for (var i = 0; i < agg.Aggregates.Count; i++)
            {
                var item = agg.Aggregates[i];
                if (item.Func == aggFunc && Equals(item.Arg, aggArg) && item.Distinct == aggDistinct)
                {
                    var func = (item.Func == AggFunction.Count && item.Arg is AggArg.Star)
                        ? AggFunc.CountStar
                        : MapAggFunc(item.Func);
                    return new Instruction[] { new FinalizeAgg(slots[i], func) };
                }
            }
            // No matching slot found — should not happen if planner is correct.
            // Fall back to null so the predicate evaluates to false and the group is excluded.
            return new Instruction[] { new LoadConst(null) };
        }

        // For compound expressions, recurse into children with the same AggExpr-aware compiler,
        // then apply the top-level operator at the end.  This handles HAVING predicates like
        //   COUNT(*) > 1            → BinaryOp(AggExpr, Literal(1))
        //   SUM(x) > 10 AND y < 5  → BinaryOp(BinaryOp(AggExpr, Literal(10)), BinaryOp(Column, Literal(5)))
        switch (expr)
        {
            case PlBinaryOp(var op, var left, var right):
                return CompileHavingExpr(left, agg, slots, ctx)
                    .Concat(CompileHavingExpr(right, agg, slots, ctx))
                    .Append(new BinaryOpInstr(MapBinaryOp(op)));

            case PlUnaryOp(var op, var operand):
                return CompileHavingExpr(operand, agg, slots, ctx)
                    .Append(new UnaryOpInstr(MapUnaryOp(op)));

            case PlIsNull(var operand):
                return CompileHavingExpr(operand, agg, slots, ctx)
                    .Append((Instruction)new IsNullInstr());

            case PlIsNotNull(var operand):
                return CompileHavingExpr(operand, agg, slots, ctx)
                    .Append((Instruction)new IsNotNullInstr());

            case PlBetween(var value, var low, var high):
                return CompileHavingExpr(value, agg, slots, ctx)
                    .Concat(CompileHavingExpr(low, agg, slots, ctx))
                    .Concat(CompileHavingExpr(high, agg, slots, ctx))
                    .Append(new BetweenInstr());

            case SqlExpr.FuncCall(var name, var args):
            {
                var instrs = new List<Instruction>();
                foreach (var arg in args)
                    instrs.AddRange(CompileHavingExpr(arg, agg, slots, ctx));
                instrs.Add(new CallScalar(name, args.Count));
                return instrs;
            }

            // For leaf expressions with no aggregate children, delegate to the general compiler.
            default:
                return CompileExprInCtx(expr, ctx);
        }
    }

    // ── DML compilation ───────────────────────────────────────────────────────

    private static void CompileInsert(
        string                                       table,
        IReadOnlyList<string>?                       colNames,
        IReadOnlyList<IReadOnlyList<SqlExpr>>        rows,
        List<Instruction>                            out_,
        Ctx                                          ctx)
    {
        // For each row of values: compile each value expression then InsertRow.
        var cols = colNames ?? Array.Empty<string>();
        foreach (var row in rows)
        {
            foreach (var val in row)
                out_.AddRange(CompileExprInCtx(val, ctx));
            out_.Add(new InsertRow(table, cols));
        }
    }

    private static void CompileUpdate(
        string                          table,
        IReadOnlyList<Assignment>       assignments,
        SqlExpr?                        predicate,
        List<Instruction>               out_,
        Ctx                             ctx)
    {
        // Open a scan, apply the optional predicate, then emit assignment
        // values and UpdateRows for each matching row.
        var cid  = ctx.NextCursor();
        ctx.RegisterAlias(table, cid);

        var loop = ctx.NextLabel($"update_{cid}_loop");
        var end  = ctx.NextLabel($"update_{cid}_end");
        var skip = predicate is null ? null : ctx.NextLabel("update_skip");

        out_.Add(new OpenScan(cid, table));
        out_.Add(new CodegenLabel(loop));
        out_.Add(new AdvanceCursor(cid, end));

        if (predicate is not null)
        {
            out_.AddRange(CompileExprInCtx(predicate, ctx));
            out_.Add(new JumpIfFalse(skip!));
        }

        foreach (var assgn in assignments)
            out_.AddRange(CompileExprInCtx(assgn.Value, ctx));

        out_.Add(new UpdateRows(table, assignments.Select(a => a.Column).ToList(), cid));

        if (skip is not null)
            out_.Add(new CodegenLabel(skip));

        out_.Add(new Jump(loop));
        out_.Add(new CodegenLabel(end));
        out_.Add(new CloseScan(cid));
    }

    private static void CompileDelete(
        string            table,
        SqlExpr?          predicate,
        List<Instruction> out_,
        Ctx               ctx)
    {
        var cid  = ctx.NextCursor();
        ctx.RegisterAlias(table, cid);

        var loop = ctx.NextLabel($"delete_{cid}_loop");
        var end  = ctx.NextLabel($"delete_{cid}_end");
        var skip = predicate is null ? null : ctx.NextLabel("delete_skip");

        out_.Add(new OpenScan(cid, table));
        out_.Add(new CodegenLabel(loop));
        out_.Add(new AdvanceCursor(cid, end));

        if (predicate is not null)
        {
            out_.AddRange(CompileExprInCtx(predicate, ctx));
            out_.Add(new JumpIfFalse(skip!));
        }

        out_.Add(new DeleteRows(table, cid));

        if (skip is not null)
            out_.Add(new CodegenLabel(skip));

        out_.Add(new Jump(loop));
        out_.Add(new CodegenLabel(end));
        out_.Add(new CloseScan(cid));
    }

    // ── Expression compiler ───────────────────────────────────────────────────
    //
    // Compiles a single SqlExpr into a sequence of stack-machine instructions.
    // The caller appends these instructions into the main output list.
    //
    // Stack discipline: each compiled expression leaves exactly one value on the
    // stack.

    internal static IEnumerable<Instruction> CompileExprInCtx(SqlExpr expr, Ctx ctx)
    {
        // We use the fully-qualified SqlExpr.xxx names (via aliases) to avoid
        // ambiguity with the identically-named instruction records.
        switch (expr)
        {
            // ── Literals ──────────────────────────────────────────────────────
            case SqlExpr.Literal(var value):
                return new Instruction[] { new LoadConst(ConvertLiteral(value)) };

            // ── Column references ─────────────────────────────────────────────
            case SqlExpr.Column(var table, var col):
                return new Instruction[] { new LoadColumn(ctx.CursorOf(table), col) };

            // ── Binary operators ──────────────────────────────────────────────
            case PlBinaryOp(var op, var left, var right):
                return CompileExprInCtx(left,  ctx)
                    .Concat(CompileExprInCtx(right, ctx))
                    .Append(new BinaryOpInstr(MapBinaryOp(op)));

            // ── Unary operators ───────────────────────────────────────────────
            case PlUnaryOp(var op, var operand):
                return CompileExprInCtx(operand, ctx)
                    .Append(new UnaryOpInstr(MapUnaryOp(op)));

            // ── IS NULL / IS NOT NULL ─────────────────────────────────────────
            case PlIsNull(var operand):
                return CompileExprInCtx(operand, ctx)
                    .Append((Instruction)new IsNullInstr());

            case PlIsNotNull(var operand):
                return CompileExprInCtx(operand, ctx)
                    .Append((Instruction)new IsNotNullInstr());

            // ── BETWEEN ───────────────────────────────────────────────────────
            // Stack order: value (bottom), low, high (top)
            case PlBetween(var value, var low, var high):
                return CompileExprInCtx(value, ctx)
                    .Concat(CompileExprInCtx(low,   ctx))
                    .Concat(CompileExprInCtx(high,  ctx))
                    .Append(new BetweenInstr());

            // ── IN (list) ─────────────────────────────────────────────────────
            // Stack order: probe value (bottom), then each item (top of last item)
            case PlIn(var value, var items):
            {
                var instrs = CompileExprInCtx(value, ctx).ToList();
                foreach (var item in items)
                    instrs.AddRange(CompileExprInCtx(item, ctx));
                instrs.Add(new InListInstr(items.Count));
                return instrs;
            }

            // ── NOT IN (list) ─────────────────────────────────────────────────
            case PlNotIn(var value, var items):
            {
                // Compile as IN then negate.
                var instrs = CompileExprInCtx(value, ctx).ToList();
                foreach (var item in items)
                    instrs.AddRange(CompileExprInCtx(item, ctx));
                instrs.Add(new InListInstr(items.Count));
                instrs.Add(new UnaryOpInstr(UnaryOpCode.Not));
                return instrs;
            }

            // ── LIKE / NOT LIKE ───────────────────────────────────────────────
            case PlLike(var value, _):
                return CompileExprInCtx(value, ctx)
                    .Append(new LikeInstr(Negated: false));

            case PlNotLike(var value, _):
                return CompileExprInCtx(value, ctx)
                    .Append(new LikeInstr(Negated: true));

            // ── Scalar function call ───────────────────────────────────────────
            case SqlExpr.FuncCall(var name, var args):
            {
                var instrs = new List<Instruction>();
                foreach (var arg in args)
                    instrs.AddRange(CompileExprInCtx(arg, ctx));
                instrs.Add(new CallScalar(name, args.Count));
                return instrs;
            }

            // ── Aggregate expressions ─────────────────────────────────────────
            // AggExpr nodes are handled by CompileAggregate; if one leaks into
            // expression compilation it means the planner left a bare aggregate
            // outside a proper AggregatePlan.  Emit a placeholder LoadConst(null).
            case SqlExpr.AggExpr:
                return new Instruction[] { new LoadConst(null) };

            // ── Wildcard ─────────────────────────────────────────────────────
            // SELECT * — not a scalar value; emit a null placeholder.
            case SqlExpr.Wildcard:
                return new Instruction[] { new LoadConst(null) };

            default:
                throw new InvalidOperationException(
                    $"CompileExprInCtx: unsupported expression type {expr.GetType().Name}");
        }
    }

    // ── Helper: project column name extraction ────────────────────────────────

    private static IReadOnlyList<string> ExtractProjectedNames(IReadOnlyList<OutputColumn> cols)
    {
        var names = new List<string>();
        for (var i = 0; i < cols.Count; i++)
        {
            names.Add(cols[i] switch
            {
                OutputColumn.Star         => "*",
                OutputColumn.Expr(_, var alias) when alias is not null => alias,
                OutputColumn.Expr(SqlExpr.Column(_, var col), _)       => col,
                OutputColumn.Expr _                                      => $"col_{i}",
                _                                                        => $"col_{i}",
            });
        }
        return names;
    }

    private static string GetColumnName(OutputColumn col, string fallback)
        => col switch
        {
            OutputColumn.Expr(_, var alias) when alias is not null     => alias,
            OutputColumn.Expr(SqlExpr.Column(_, var c), _)             => c,
            _                                                           => fallback,
        };

    private static void EmitProjectColumns(
        IReadOnlyList<OutputColumn> cols,
        IReadOnlyList<string>       names,
        List<Instruction>           out_,
        Ctx                         ctx)
    {
        for (var i = 0; i < cols.Count; i++)
        {
            var col  = cols[i];
            var name = i < names.Count ? names[i] : $"col_{i}";

            switch (col)
            {
                case OutputColumn.Star:
                    // SELECT * — emit a null placeholder; real expansion happens in VM.
                    out_.Add(new LoadConst(null));
                    out_.Add(new EmitColumn(name));
                    break;

                case OutputColumn.Expr(var expr, _):
                    out_.AddRange(CompileExprInCtx(expr, ctx));
                    out_.Add(new EmitColumn(name));
                    break;
            }
        }
    }

    private static IReadOnlyList<string> BuildDefaultAggNames(OptAggregate agg)
    {
        var names = new List<string>();
        for (var i = 0; i < agg.GroupBy.Count; i++)
            names.Add($"group_{i}");
        foreach (var item in agg.Aggregates)
            names.Add(item.Alias);
        return names;
    }

    // ── Literal conversion ────────────────────────────────────────────────────
    //
    // The planner uses `object?` for literal values.  We pass them through
    // unchanged since the VM's type system accepts the same CLR types.

    private static object? ConvertLiteral(object? value) => value;

    // ── Operator mapping ──────────────────────────────────────────────────────

    private static BinaryOpCode MapBinaryOp(BinaryOperator op)
        => op switch
        {
            BinaryOperator.Add   => BinaryOpCode.Add,
            BinaryOperator.Sub   => BinaryOpCode.Sub,
            BinaryOperator.Mul   => BinaryOpCode.Mul,
            BinaryOperator.Div   => BinaryOpCode.Div,
            BinaryOperator.Mod   => BinaryOpCode.Mod,
            BinaryOperator.Eq    => BinaryOpCode.Eq,
            BinaryOperator.NotEq => BinaryOpCode.Neq,
            BinaryOperator.Lt    => BinaryOpCode.Lt,
            BinaryOperator.Lte   => BinaryOpCode.Lte,
            BinaryOperator.Gt    => BinaryOpCode.Gt,
            BinaryOperator.Gte   => BinaryOpCode.Gte,
            BinaryOperator.And   => BinaryOpCode.And,
            BinaryOperator.Or    => BinaryOpCode.Or,
            _ => throw new InvalidOperationException($"Unknown BinaryOperator: {op}"),
        };

    private static UnaryOpCode MapUnaryOp(UnaryOperator op)
        => op switch
        {
            UnaryOperator.Neg => UnaryOpCode.Neg,
            UnaryOperator.Not => UnaryOpCode.Not,
            _ => throw new InvalidOperationException($"Unknown UnaryOperator: {op}"),
        };

    private static AggFunc MapAggFunc(AggFunction func)
        => func switch
        {
            AggFunction.Count => AggFunc.Count,
            AggFunction.Sum   => AggFunc.Sum,
            AggFunction.Avg   => AggFunc.Avg,
            AggFunction.Min   => AggFunc.Min,
            AggFunction.Max   => AggFunc.Max,
            _ => throw new InvalidOperationException($"Unknown AggFunction: {func}"),
        };
}

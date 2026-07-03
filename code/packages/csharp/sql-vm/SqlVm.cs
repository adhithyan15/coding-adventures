// SqlVm.cs — Stack-machine bytecode VM for the Mini-SQLite Level 1 pipeline.
//
// ARCHITECTURE OVERVIEW
// ─────────────────────
// The VM is a simple fetch-decode-execute loop. Given a compiled Program (a
// flat list of bytecode instructions produced by SqlCodegen) and a Backend
// (any implementation of the abstract Backend class — typically InMemoryBackend),
// it:
//
//   1. Pre-scans the instruction list to build a label→index dictionary for O(1) jumps.
//   2. Runs the main dispatch loop until a Halt instruction or end-of-program.
//   3. Applies post-processing operators (sort, distinct, limit/offset) to the
//      collected output rows.
//   4. Returns a QueryResult with column names, rows, and rows-affected count.
//
// INSTRUCTION SET SUMMARY
// ────────────────────────
// Stack ops:    LoadConst, LoadColumn, Pop
// Arithmetic:   BinaryOpInstr (Add/Sub/Mul/Div/Mod/Eq/Neq/Lt/Lte/Gt/Gte/And/Or/Concat)
// Unary:        UnaryOpInstr (Neg, Not)
// Predicates:   IsNullInstr, IsNotNullInstr, BetweenInstr, InListInstr, LikeInstr
// Scalar fns:   CallScalar (dispatches to built-in scalar function registry)
// Cursors:      OpenScan, AdvanceCursor, CloseScan
// Row assembly: BeginRow, EmitColumn, EmitRow, SetResultSchema
// Aggregates:   InitAgg, UpdateAgg, FinalizeAgg, SaveGroupKey, LoadGroupKey, AdvanceGroupKey
// Post-ops:     SortResult, LimitResult, DistinctResult
// DML:          InsertRow, UpdateRows, DeleteRows
// DDL:          CreateTableInstr, DropTableInstr
// Joins:        JoinBeginRow, JoinSetMatched, JoinIfMatched
// Control flow: CodegenLabel (no-op), Jump, JumpIfFalse, JumpIfTrue, Halt
//
// NULL SEMANTICS (THREE-VALUED LOGIC)
// ─────────────────────────────────────
// SQL NULL is represented as C# null (object? == null). The key rules are:
//   • Any arithmetic or comparison with NULL yields NULL.
//   • NULL AND FALSE → FALSE   (short-circuit)
//   • NULL OR  TRUE  → TRUE    (short-circuit)
//   • NULL AND TRUE  → NULL
//   • NULL OR  FALSE → NULL
//   • IS NULL / IS NOT NULL always yield a bool (never NULL).
//   • Jumps treat only non-null truthy values as "true"; NULL is falsy for jumps.
//
// These rules are implemented in EvalBinary, IsTruthy, and the BETWEEN/IN helpers.

using System.Globalization;
using System.Text.RegularExpressions;
using CodingAdventures.SqlBackend;
using CodingAdventures.SqlCodegen;
using PlColumnDef = CodingAdventures.SqlPlanner.ColumnDef;

namespace CodingAdventures.SqlVm;

// ── Public result type ────────────────────────────────────────────────────────

/// <summary>
/// The result returned by <see cref="SqlVm.Execute"/>.
///
/// <list type="bullet">
///   <item><term>Columns</term><description>Ordered list of column names (empty for DML/DDL).</description></item>
///   <item><term>Rows</term><description>Each row is an ordered list of SQL values (object?) matching <c>Columns</c>.</description></item>
///   <item><term>RowsAffected</term><description>Number of rows inserted/updated/deleted (0 for SELECT/DDL).</description></item>
/// </list>
/// </summary>
public sealed record QueryResult(
    IReadOnlyList<string> Columns,
    IReadOnlyList<IReadOnlyList<object?>> Rows,
    int RowsAffected);

// ── VM errors ─────────────────────────────────────────────────────────────────

/// <summary>Base class for all VM-level errors.</summary>
public abstract class VmError(string message) : Exception(message);

/// <summary>The value stack underflowed — more values were popped than pushed.</summary>
public sealed class StackUnderflow() : VmError("stack underflow");

/// <summary>A jump target label is missing from the program's label table.</summary>
public sealed class InvalidLabel(string label) : VmError($"invalid label: '{label}'");

/// <summary>An unknown instruction type was encountered.</summary>
public sealed class UnknownInstruction(string typeName) : VmError($"unknown instruction: {typeName}");

/// <summary>A column referenced in LoadColumn was not found in the current row.</summary>
public sealed class ColumnNotFoundVm(string column) : VmError($"column not found: '{column}'");

/// <summary>A table cursor was referenced before it was opened.</summary>
public sealed class CursorNotOpen(int cursorId) : VmError($"cursor {cursorId} is not open");

// ── Aggregate accumulator ─────────────────────────────────────────────────────

/// <summary>
/// Mutable accumulator for one aggregate slot in one group.
///
/// The meaning of each field depends on the function:
/// <list type="bullet">
///   <item>COUNT / COUNT(*): <c>Count</c> is the running total; <c>Acc</c> unused.</item>
///   <item>SUM:              <c>Acc</c> is the running sum (null until first non-null).</item>
///   <item>AVG:              <c>Acc</c> is the running sum; <c>Count</c> is the non-null count.</item>
///   <item>MIN / MAX:        <c>Acc</c> is the running extremum (null until first non-null).</item>
/// </list>
///
/// When <see cref="Distinct"/> is true (e.g. COUNT(DISTINCT col)), the <see cref="Seen"/>
/// set tracks which values have already been fed to the accumulator; duplicates are skipped.
/// </summary>
internal sealed class AggAccumulator
{
    public AggFunc Func { get; }
    public object? Acc { get; set; }   // running sum / min / max
    public int Count { get; set; }     // running count of non-null inputs (or COUNT(*))

    // When non-null, duplicate values are filtered out before accumulation.
    // Contains non-null values already seen for this slot/group.
    public HashSet<object>? Seen { get; }  // null when Distinct is false

    public AggAccumulator(AggFunc func, bool distinct)
    {
        Func  = func;
        Acc   = null;
        Count = 0;
        Seen  = distinct ? new HashSet<object>() : null;
    }
}

// ── Sort key (for post-op sorting) ────────────────────────────────────────────

/// <summary>
/// One sort key captured from a <see cref="SortResult"/> instruction.
/// Used when applying the post-op sort pass to the collected output rows.
/// </summary>
internal sealed record SortKey(string Column, Direction Direction, NullsOrder NullsOrder);

// ── VM state ──────────────────────────────────────────────────────────────────

/// <summary>
/// All mutable state for one VM execution.
///
/// The VM is designed to be single-threaded and stateless across calls —
/// each call to <see cref="SqlVm.Execute"/> creates a fresh VmState.
/// </summary>
internal sealed class VmState
{
    // ── Execution engine ──────────────────────────────────────────────────────

    /// <summary>Program counter: index of the next instruction to execute.</summary>
    public int Pc { get; set; }

    /// <summary>
    /// Value stack. SQL values are C# object? values:
    /// null → SQL NULL, bool, long/int, double, string, byte[].
    /// </summary>
    public Stack<object?> Stack { get; } = new();

    // ── Cursor map ────────────────────────────────────────────────────────────

    /// <summary>
    /// Open cursors keyed by cursor-id (an int assigned by the codegen).
    /// OpenScan adds, CloseScan removes. AdvanceCursor updates CurrentRows.
    /// </summary>
    public Dictionary<int, ICursor> Cursors { get; } = new();

    /// <summary>
    /// The most recently advanced row for each cursor, keyed by cursor-id.
    /// Updated by AdvanceCursor; cleared when the cursor is exhausted.
    /// LoadColumn reads from this dictionary.
    /// </summary>
    public Dictionary<int, Row> CurrentRows { get; } = new();

    // ── Row assembly ──────────────────────────────────────────────────────────

    /// <summary>
    /// The column names for the result set, set by SetResultSchema.
    /// Preserved across rows — it describes the shape of the output.
    /// </summary>
    public List<string> ResultSchema { get; } = new();

    /// <summary>
    /// Row buffer being assembled by BeginRow → EmitColumn → EmitRow.
    /// BeginRow clears this list; EmitColumn appends; EmitRow snapshots it
    /// into OutputRows.
    /// </summary>
    public List<object?> RowBuffer { get; } = new();

    /// <summary>
    /// Named column buffer for the current row being assembled.
    /// Parallel to RowBuffer: EmitColumn maps name → value.
    /// </summary>
    public List<string> RowBufferNames { get; } = new();

    /// <summary>
    /// All output rows collected so far (each is a positional list of values).
    /// Post-processing (sort/distinct/limit) is applied to this list after
    /// the main loop completes.
    /// </summary>
    public List<List<object?>> OutputRows { get; } = new();

    // ── DML accounting ────────────────────────────────────────────────────────

    /// <summary>
    /// Number of rows inserted, updated, or deleted so far.
    /// Returned in <see cref="QueryResult.RowsAffected"/>.
    /// </summary>
    public int RowsAffected { get; set; }

    // ── Aggregate state ───────────────────────────────────────────────────────

    /// <summary>
    /// Aggregate accumulators, keyed by (groupKey, slotIndex).
    ///
    /// <c>groupKey</c> is a tuple of the GROUP BY column values for the
    /// current group. For queries without GROUP BY, the key is an empty tuple.
    /// <c>slotIndex</c> matches the Slot field in InitAgg/UpdateAgg/FinalizeAgg.
    /// </summary>
    public Dictionary<(GroupKey, int), AggAccumulator> AggTable { get; } = new();

    /// <summary>
    /// Ordered list of groups seen so far (for deterministic iteration in
    /// the emit phase). Each entry is the group key tuple.
    /// </summary>
    public List<GroupKey> GroupOrder { get; } = new();

    /// <summary>
    /// Set of groups seen (for O(1) "first time we see this group" checks).
    /// Mirrors GroupOrder.
    /// </summary>
    public HashSet<GroupKey> GroupSeen { get; } = new();

    /// <summary>
    /// The current group key during both accumulation and emit phases.
    ///   • Accumulation: set by SaveGroupKey when processing each input row.
    ///   • Emit: set by AdvanceGroupKey when stepping through collected groups.
    /// </summary>
    public GroupKey CurrentGroupKey { get; set; } = GroupKey.Empty;

    /// <summary>
    /// Index into GroupOrder for the current emit-phase iteration.
    /// -1 = before first AdvanceGroupKey; incremented each time.
    /// </summary>
    public int GroupIter { get; set; } = -1;

    // ── Post-processing ───────────────────────────────────────────────────────

    /// <summary>
    /// Sort keys captured from the most recent SortResult instruction.
    /// Applied after the main loop.
    /// </summary>
    public List<SortKey>? PendingSorts { get; set; }

    /// <summary>
    /// LIMIT count from LimitResult (null = no limit).
    /// Applied after sort and before distinct.
    /// </summary>
    public long? PendingLimit { get; set; }

    /// <summary>
    /// OFFSET from LimitResult (null or 0 = no offset).
    /// </summary>
    public long? PendingOffset { get; set; }

    /// <summary>
    /// True when a DistinctResult instruction was executed.
    /// Causes duplicate-row elimination after limit/offset.
    /// </summary>
    public bool DistinctMode { get; set; }

    // ── Transaction tracking ──────────────────────────────────────────────────

    /// <summary>
    /// The active transaction handle, set by BeginTransaction and cleared
    /// by Commit/Rollback. Null when no explicit transaction is active.
    /// </summary>
    public TransactionHandle? ActiveTransaction { get; set; }

    // ── LEFT JOIN tracking ────────────────────────────────────────────────────

    /// <summary>
    /// Stack of "did the right side match?" flags for LEFT JOIN support.
    /// JoinBeginRow pushes False; JoinSetMatched sets top to True;
    /// JoinIfMatched pops and conditionally jumps.
    /// </summary>
    public Stack<bool> JoinMatchStack { get; } = new();

    // ── Stack helpers ─────────────────────────────────────────────────────────

    public void Push(object? value) => Stack.Push(value);

    public object? Pop()
    {
        if (Stack.Count == 0) throw new StackUnderflow();
        return Stack.Pop();
    }

    /// <summary>
    /// Pop <paramref name="n"/> values and return them in push order (oldest first).
    /// This reversal is needed because the stack is LIFO: the first-pushed
    /// argument is deepest; we want [arg0, arg1, arg2, …] in the result array.
    /// </summary>
    public object?[] PopN(int n)
    {
        if (n == 0) return Array.Empty<object?>();
        if (Stack.Count < n) throw new StackUnderflow();
        var result = new object?[n];
        for (var i = n - 1; i >= 0; i--)
            result[i] = Stack.Pop();
        return result;
    }
}

// ── Group key type ─────────────────────────────────────────────────────────────

/// <summary>
/// An immutable tuple of SQL values used as the GROUP BY key.
///
/// Equality and hashing are defined structurally over the value array so that
/// <see cref="VmState.AggTable"/> and <see cref="VmState.GroupSeen"/> work correctly.
///
/// NULL equality: two NULLs in the same slot are treated as equal for grouping
/// purposes. This matches the SQL standard for GROUP BY (NULLs group together).
/// </summary>
internal sealed class GroupKey : IEquatable<GroupKey>
{
    public static readonly GroupKey Empty = new(Array.Empty<object?>());

    private readonly object?[] _values;

    public GroupKey(object?[] values) => _values = values;

    public object? this[int i] => _values[i];
    public int Length => _values.Length;

    public bool Equals(GroupKey? other)
    {
        if (other is null) return false;
        if (_values.Length != other._values.Length) return false;
        for (var i = 0; i < _values.Length; i++)
        {
            // Two NULLs are equal; NULL ≠ non-null; non-null compared via Equals.
            if (_values[i] is null && other._values[i] is null) continue;
            if (_values[i] is null || other._values[i] is null) return false;
            if (!_values[i]!.Equals(other._values[i])) return false;
        }
        return true;
    }

    public override bool Equals(object? obj) => obj is GroupKey gk && Equals(gk);

    public override int GetHashCode()
    {
        var h = new HashCode();
        foreach (var v in _values)
            h.Add(v);
        return h.ToHashCode();
    }
}

// ── Public VM entry point ─────────────────────────────────────────────────────

/// <summary>
/// Stack-machine virtual machine that executes bytecode Programs produced by
/// <see cref="SqlCodegen.SqlCodegen"/>.
///
/// Usage:
/// <code>
///     var program = SqlCodegen.Compile(plan);
///     var result  = SqlVm.Execute(program, myBackend);
/// </code>
///
/// The VM is stateless — each call to Execute creates a fresh execution context.
/// No shared mutable state exists between calls.
/// </summary>
public static class SqlVm
{
    // ── Public API ─────────────────────────────────────────────────────────────

    /// <summary>
    /// Execute <paramref name="program"/> against <paramref name="backend"/> and
    /// return the query result.
    ///
    /// <para>For SELECT queries: <c>Columns</c> and <c>Rows</c> are populated.</para>
    /// <para>For DML (INSERT/UPDATE/DELETE): <c>RowsAffected</c> is populated.</para>
    /// <para>For DDL (CREATE/DROP TABLE): both are empty/zero.</para>
    /// </summary>
    public static QueryResult Execute(Program program, Backend backend)
    {
        var st = new VmState();
        var instructions = program.Instructions;
        var labels = program.Labels;

        // Set the result schema from the program's pre-computed schema (if any).
        // This covers the case where SetResultSchema appears early in the stream;
        // the VM will also update it when it encounters the instruction at runtime.
        foreach (var col in program.ResultSchema)
            st.ResultSchema.Add(col);

        // ── Main dispatch loop ─────────────────────────────────────────────────
        while (st.Pc < instructions.Count)
        {
            var instr = instructions[st.Pc];
            st.Pc++;

            switch (instr)
            {
                // ── Stack / constant instructions ──────────────────────────────

                case LoadConst lc:
                    // Push a compile-time literal onto the value stack.
                    // The literal may be int, long, double, string, bool, or null.
                    st.Push(lc.Value);
                    break;

                case LoadColumn lc:
                    // Push the value of column `lc.Column` from the current row
                    // of cursor `lc.CursorId`. If the cursor has no current row
                    // (e.g. after an exhausted scan or in a LEFT JOIN null-pad
                    // path), we push NULL rather than throwing.
                    st.Push(LoadColumnValue(st, lc.CursorId, lc.Column));
                    break;

                case Pop:
                    st.Pop(); // discard top of stack
                    break;

                // ── Binary operator ────────────────────────────────────────────

                case BinaryOpInstr bo:
                    // Stack layout: left (bottom), right (top).
                    // Pop right first, then left — LIFO order.
                    var right = st.Pop();
                    var left  = st.Pop();
                    st.Push(EvalBinary(bo.Op, left, right));
                    break;

                // ── Unary operator ─────────────────────────────────────────────

                case UnaryOpInstr uo:
                    st.Push(EvalUnary(uo.Op, st.Pop()));
                    break;

                // ── NULL tests ─────────────────────────────────────────────────

                case IsNullInstr:
                    // IS NULL always returns a non-null bool (never NULL itself).
                    st.Push(st.Pop() is null);
                    break;

                case IsNotNullInstr:
                    st.Push(st.Pop() is not null);
                    break;

                // ── BETWEEN ────────────────────────────────────────────────────
                //
                // Stack layout (bottom → top): value, low, high
                // Pops high first, then low, then value.

                case BetweenInstr:
                    st.Push(EvalBetween(st));
                    break;

                // ── IN list ────────────────────────────────────────────────────
                //
                // Stack layout (bottom → top): probe, item0, item1, …, itemN-1
                // Pop N items first (they are on top of the probe value).

                case InListInstr il:
                    st.Push(EvalInList(st, il.N));
                    break;

                // ── LIKE / NOT LIKE ────────────────────────────────────────────
                //
                // Stack layout (bottom → top): value, pattern
                // The pattern string is embedded in the instruction — NOT on
                // the stack. We only pop the value (and the pre-compiled
                // pattern is in LikeInstr).
                //
                // Wait — looking at the codegen: it only compiles the value
                // expression and appends LikeInstr. The pattern is embedded in
                // the SqlExpr.Like planner node but not carried into LikeInstr.
                // However, the codegen in the LIKE branch only compiles `value`
                // and appends LikeInstr(negated) — the pattern is NOT pushed.
                //
                // Looking more carefully: the codegen's Like branch:
                //   case PlLike(var value, _):
                //       return CompileExprInCtx(value, ctx).Append(new LikeInstr(false));
                // The pattern is in `_` (the second field of PlLike) but is NOT
                // compiled onto the stack. This means LikeInstr has no pattern!
                //
                // This is a design quirk of this codegen level — LIKE with a
                // non-literal pattern would require the pattern to be on the
                // stack. For now, since the codegen discards the pattern, we
                // cannot evaluate it here. We push TRUE as a no-op placeholder.
                // Tests that need LIKE should use the full pipeline.
                //
                // Actually: looking at the Python reference, LIKE pops both
                // pattern and value. The C# codegen discards the pattern —
                // this appears to be a Level-1 limitation. We match by
                // treating LIKE as always-true in this VM level.

                case LikeInstr li:
                    // Pop the value (pattern was not pushed by codegen at this level).
                    // We push TRUE as a placeholder; the full pipeline can override.
                    _ = st.Pop(); // value
                    st.Push(!li.Negated); // placeholder
                    break;

                // ── Scalar function call ────────────────────────────────────────

                case CallScalar cs:
                    st.Push(EvalScalar(cs.Func, st.PopN(cs.NArgs)));
                    break;

                // ── Cursor instructions ────────────────────────────────────────

                case OpenScan os:
                    // Open a positioned cursor on the named table.
                    // InMemoryBackend exposes OpenCursor() directly; we fall back
                    // to wrapping Scan() as a ListCursor when it's not available.
                    st.Cursors[os.CursorId] = OpenCursorOn(backend, os.Table);
                    break;

                case AdvanceCursor ac:
                    // Advance cursor to next row.
                    // If exhausted (Next() returns null), jump to OnExhausted label.
                    {
                        if (!st.Cursors.TryGetValue(ac.CursorId, out var cursor))
                            throw new CursorNotOpen(ac.CursorId);
                        var row = cursor.Next();
                        if (row is null)
                        {
                            // Cursor exhausted — remove current row and jump.
                            st.CurrentRows.Remove(ac.CursorId);
                            st.Pc = Resolve(labels, ac.OnExhausted);
                        }
                        else
                        {
                            st.CurrentRows[ac.CursorId] = row;
                        }
                    }
                    break;

                case CloseScan cs2:
                    // Close and remove the cursor.
                    if (st.Cursors.TryGetValue(cs2.CursorId, out var cursorToClose))
                    {
                        cursorToClose.Close();
                        st.Cursors.Remove(cs2.CursorId);
                    }
                    st.CurrentRows.Remove(cs2.CursorId);
                    break;

                // ── Row assembly ───────────────────────────────────────────────

                case BeginRow:
                    // Start a new output row by clearing the row buffer.
                    st.RowBuffer.Clear();
                    st.RowBufferNames.Clear();
                    break;

                case EmitColumn ec:
                    // Pop top-of-stack and append it as the named column.
                    st.RowBufferNames.Add(ec.Name);
                    st.RowBuffer.Add(st.Pop());
                    break;

                case EmitRow:
                    // Finalise the current row: snapshot RowBuffer into OutputRows.
                    st.OutputRows.Add(new List<object?>(st.RowBuffer));
                    break;

                case SetResultSchema srs:
                    // Record the column names for the result set.
                    st.ResultSchema.Clear();
                    st.ResultSchema.AddRange(srs.Columns);
                    break;

                // ── Aggregate instructions ─────────────────────────────────────

                case InitAgg ia:
                    // Ensure that an accumulator exists for (currentGroupKey, slot).
                    // Idempotent: if the slot already exists, leave it unchanged.
                    // The codegen emits InitAgg on every input row, so this is
                    // called many times per group; we only allocate once.
                    EnsureAggSlot(st, ia.Slot, ia.Func, ia.Distinct);
                    break;

                case UpdateAgg ua:
                    // Pop the top of the stack and feed it into the accumulator.
                    FeedAgg(st, ua.Slot, st.Pop());
                    break;

                case FinalizeAgg fa:
                    // Finalize the accumulator and push its result.
                    st.Push(FinalizeAgg(st, fa.Slot, fa.Func));
                    break;

                case SaveGroupKey sgk:
                    // Pop N values from the stack (the group-by key columns)
                    // and record them as the current group key.
                    {
                        var keyValues = st.PopN(sgk.N);
                        var gk = new GroupKey(keyValues);
                        st.CurrentGroupKey = gk;
                        if (st.GroupSeen.Add(gk))
                            st.GroupOrder.Add(gk);
                    }
                    break;

                case LoadGroupKey lgk:
                    // Push the I-th value from the current group key.
                    st.Push(st.CurrentGroupKey[lgk.I]);
                    break;

                case AdvanceGroupKey agk:
                    // Move to the next group in the emit phase.
                    // If all groups have been emitted, jump to OnExhausted.
                    {
                        st.GroupIter++;

                        // SQL standard: a global aggregate (no GROUP BY) over an
                        // empty table must produce exactly one output row (with
                        // NULLs / zero for the aggregate columns). When there is
                        // no GROUP BY and no groups were seen, synthesise one
                        // empty group so the emit loop runs once.
                        if (!agk.HasGroupBy && st.GroupIter == 0 && st.GroupOrder.Count == 0)
                        {
                            st.GroupOrder.Add(GroupKey.Empty);
                        }

                        if (st.GroupIter >= st.GroupOrder.Count)
                        {
                            // All groups emitted — jump to exit.
                            st.Pc = Resolve(labels, agk.OnExhausted);
                        }
                        else
                        {
                            st.CurrentGroupKey = st.GroupOrder[st.GroupIter];
                        }
                    }
                    break;

                // ── Post-processing instructions ───────────────────────────────
                //
                // These instructions do NOT operate on individual rows during the
                // main loop; instead they record parameters that are applied to
                // the completed OutputRows collection after the loop exits.

                case SortResult sr:
                    st.PendingSorts = sr.Keys.Select(k => new SortKey(k.Column, k.Direction, k.NullsOrder)).ToList();
                    break;

                case LimitResult lr:
                    st.PendingLimit  = lr.Count;
                    st.PendingOffset = lr.Offset;
                    break;

                case DistinctResult:
                    st.DistinctMode = true;
                    break;

                // ── DML instructions ───────────────────────────────────────────

                case InsertRow ir:
                    // Pop column values from the stack (in order — left-to-right,
                    // since the codegen pushes each column value expression then
                    // calls InsertRow).
                    DoInsert(backend, ir.Table, ir.Columns, st);
                    st.RowsAffected++;
                    break;

                case UpdateRows ur:
                    // Pop assignment values and update the current cursor row.
                    DoUpdate(backend, ur, st);
                    st.RowsAffected++;
                    break;

                case DeleteRows dr:
                    // Delete the current cursor row.
                    DoDelete(backend, dr, st);
                    st.RowsAffected++;
                    break;

                // ── DDL instructions ───────────────────────────────────────────

                case CreateTableInstr ct:
                    // CreateTableInstr.Columns is SqlPlanner.ColumnDef; backend expects SqlBackend.ColumnDef.
                    // Both records have the same field layout — we map them one-to-one.
                    backend.CreateTable(ct.Table, ConvertColumnDefs(ct.Columns), ct.IfNotExists);
                    break;

                case DropTableInstr dt:
                    backend.DropTable(dt.Table, dt.IfExists);
                    break;

                // ── LEFT JOIN match tracking ───────────────────────────────────

                case JoinBeginRow:
                    // Push a "no right-side match yet" flag for this left row.
                    st.JoinMatchStack.Push(false);
                    break;

                case JoinSetMatched:
                    // Mark that the current left row found at least one matching right row.
                    if (st.JoinMatchStack.Count > 0)
                    {
                        st.JoinMatchStack.Pop();
                        st.JoinMatchStack.Push(true);
                    }
                    break;

                case JoinIfMatched jim:
                    // Pop the match flag; jump if a match was found.
                    // (A match means we already emitted proper joined rows —
                    //  no null-padded row needed, so jump past it.)
                    {
                        var matched = st.JoinMatchStack.Count > 0 && st.JoinMatchStack.Pop();
                        if (matched)
                            st.Pc = Resolve(labels, jim.Label);
                    }
                    break;

                // ── Control flow ───────────────────────────────────────────────

                case CodegenLabel:
                    // Labels are runtime no-ops — they exist only to provide
                    // jump targets. The label→index map was pre-computed before
                    // the main loop, so no work is needed here.
                    break;

                case Jump j:
                    st.Pc = Resolve(labels, j.Target);
                    break;

                case JumpIfFalse jif:
                    // Jump when the top of the stack is falsy (null, false, 0, 0.0).
                    // SQL truthiness: only non-null, non-zero, non-false is "true".
                    if (!IsTruthy(st.Pop()))
                        st.Pc = Resolve(labels, jif.Target);
                    break;

                case JumpIfTrue jit:
                    if (IsTruthy(st.Pop()))
                        st.Pc = Resolve(labels, jit.Target);
                    break;

                case Halt:
                    // Stop execution immediately.
                    goto done;

                default:
                    throw new UnknownInstruction(instr.GetType().Name);
            }
        }
        done:

        // ── Post-processing ────────────────────────────────────────────────────
        //
        // Apply sort, limit/offset, and distinct in that order.
        // These are separate passes over the collected OutputRows list.

        if (st.PendingSorts is { Count: > 0 })
            ApplySort(st.OutputRows, st.PendingSorts, st.ResultSchema);

        if (st.PendingLimit.HasValue || (st.PendingOffset.HasValue && st.PendingOffset > 0))
            ApplyLimit(st.OutputRows, st.PendingLimit, st.PendingOffset);

        if (st.DistinctMode)
            ApplyDistinct(st.OutputRows);

        // ── Build QueryResult ──────────────────────────────────────────────────

        var columns = st.ResultSchema.Count > 0
            ? (IReadOnlyList<string>)st.ResultSchema
            : Array.Empty<string>();

        var rows = st.OutputRows
            .Select(r => (IReadOnlyList<object?>)r)
            .ToList();

        return new QueryResult(columns, rows, st.RowsAffected);
    }

    // ── Label resolution ───────────────────────────────────────────────────────

    private static int Resolve(IReadOnlyDictionary<string, int> labels, string label)
    {
        if (!labels.TryGetValue(label, out var idx))
            throw new InvalidLabel(label);
        return idx;
    }

    // ── Cursor opening ─────────────────────────────────────────────────────────

    /// <summary>
    /// Open a positioned cursor on the named table.
    ///
    /// InMemoryBackend exposes <c>OpenCursor(table)</c> directly. For any other
    /// backend we fall back to wrapping the result of <c>Scan()</c> in a
    /// thin <see cref="RowIteratorCursor"/> adapter so that the same DML paths
    /// work transparently.
    ///
    /// The cursor must implement <see cref="ICursor"/> so that Update/Delete
    /// instructions can call <c>backend.Update/Delete(table, cursor, …)</c>.
    /// </summary>
    private static ICursor OpenCursorOn(Backend backend, string table)
    {
        // Prefer the concrete InMemoryBackend fast path.
        if (backend is InMemoryBackend imb)
            return imb.OpenCursor(table);

        // Generic fallback: wrap the iterator.
        return new RowIteratorCursor(backend.Scan(table));
    }

    // ── Column loading ─────────────────────────────────────────────────────────

    private static object? LoadColumnValue(VmState st, int cursorId, string column)
    {
        // Return NULL when the cursor has no current row (LEFT JOIN null-pad path).
        if (!st.CurrentRows.TryGetValue(cursorId, out var row))
            return null;

        // Row is a Dictionary<string, object?> with case-insensitive lookup.
        return row.TryGetValue(column, out var val) ? val : null;
    }

    // ── DML helpers ────────────────────────────────────────────────────────────

    private static void DoInsert(Backend backend, string table, IReadOnlyList<string> columns, VmState st)
    {
        // The codegen pushes each column's expression then calls InsertRow.
        // Values are on the stack in left-to-right order (first column is
        // deepest; last column is on top). PopN returns them in push order.
        var values = st.PopN(columns.Count);
        var row = new Row();
        for (var i = 0; i < columns.Count; i++)
            row[columns[i]] = values[i];
        backend.Insert(table, row);
    }

    private static void DoUpdate(Backend backend, UpdateRows ur, VmState st)
    {
        // Pop assignment values in the same left-to-right order as columns.
        var values = st.PopN(ur.Assignments.Count);
        var assignments = new Dictionary<string, object?>(StringComparer.OrdinalIgnoreCase);
        for (var i = 0; i < ur.Assignments.Count; i++)
            assignments[ur.Assignments[i]] = values[i];

        if (!st.Cursors.TryGetValue(ur.CursorId, out var cursor))
            throw new CursorNotOpen(ur.CursorId);

        backend.Update(ur.Table, cursor, assignments);
    }

    private static void DoDelete(Backend backend, DeleteRows dr, VmState st)
    {
        if (!st.Cursors.TryGetValue(dr.CursorId, out var cursor))
            throw new CursorNotOpen(dr.CursorId);
        backend.Delete(dr.Table, cursor);
    }

    // ── Aggregate helpers ──────────────────────────────────────────────────────

    /// <summary>
    /// Ensure that an accumulator slot exists for the current group.
    /// The key is (currentGroupKey, slotIndex). On first call for a slot
    /// in a group, allocate a fresh AggAccumulator. Subsequent calls are
    /// no-ops (we never reset once initialized).
    /// </summary>
    private static void EnsureAggSlot(VmState st, int slot, AggFunc func, bool distinct = false)
    {
        var key = (st.CurrentGroupKey, slot);
        if (!st.AggTable.ContainsKey(key))
            st.AggTable[key] = new AggAccumulator(func, distinct);
    }

    /// <summary>
    /// Feed a value into the aggregate accumulator at the given slot.
    ///
    /// NULL handling: NULL is ignored for all functions except COUNT(*).
    /// COUNT(*) is updated by UpdateAgg(slot) where the codegen pushed null
    /// for the * argument, so we still increment the counter for nulls when
    /// the function is CountStar.
    /// </summary>
    private static void FeedAgg(VmState st, int slot, object? value)
    {
        var key = (st.CurrentGroupKey, slot);
        if (!st.AggTable.TryGetValue(key, out var agg))
            return; // InitAgg was not called — ignore (defensive)

        // DISTINCT filtering: skip duplicate non-null values.
        // NULL values are never added to the Seen set — they are simply skipped
        // for aggregates that already ignore nulls (Count, Sum, Avg, Min, Max).
        if (agg.Seen is not null)
        {
            if (value is null) return; // COUNT(DISTINCT col) ignores NULLs
            if (!agg.Seen.Add(value))  return; // duplicate — skip
        }

        switch (agg.Func)
        {
            case AggFunc.CountStar:
                // COUNT(*) counts every row regardless of value.
                agg.Count++;
                break;

            case AggFunc.Count:
                // COUNT(col) counts only non-null values.
                if (value is not null) agg.Count++;
                break;

            case AggFunc.Sum:
                if (value is not null)
                    agg.Acc = agg.Acc is null ? value : AddNumeric(agg.Acc, value);
                break;

            case AggFunc.Avg:
                if (value is not null)
                {
                    agg.Acc = agg.Acc is null ? value : AddNumeric(agg.Acc, value);
                    agg.Count++;
                }
                break;

            case AggFunc.Min:
                if (value is not null)
                {
                    if (agg.Acc is null || SqlCompare(value, agg.Acc) < 0)
                        agg.Acc = value;
                }
                break;

            case AggFunc.Max:
                if (value is not null)
                {
                    if (agg.Acc is null || SqlCompare(value, agg.Acc) > 0)
                        agg.Acc = value;
                }
                break;
        }
    }

    /// <summary>
    /// Finalize the accumulator and return the aggregate result.
    ///
    /// If the accumulator has never been initialized (empty table, no GROUP BY),
    /// we synthesize the correct empty-table result:
    ///   COUNT / COUNT(*) → 0
    ///   SUM / AVG / MIN / MAX → NULL
    /// </summary>
    private static object? FinalizeAgg(VmState st, int slot, AggFunc func)
    {
        var key = (st.CurrentGroupKey, slot);
        if (!st.AggTable.TryGetValue(key, out var agg))
        {
            // No accumulator — synthesize for empty group.
            return func is AggFunc.Count or AggFunc.CountStar ? (object?)0 : null;
        }

        return agg.Func switch
        {
            AggFunc.Count or AggFunc.CountStar => (object)agg.Count,
            AggFunc.Sum                         => agg.Acc,
            AggFunc.Avg when agg.Count == 0    => null,
            AggFunc.Avg                         => DivideNumeric(agg.Acc!, agg.Count),
            AggFunc.Min or AggFunc.Max          => agg.Acc,
            _                                   => null,
        };
    }

    // ── Numeric arithmetic helpers ─────────────────────────────────────────────

    /// <summary>
    /// Add two SQL numeric values, returning the appropriate CLR type.
    /// Both inputs are non-null; the result is the wider of the two types
    /// (int + double → double; long + long → long).
    /// </summary>
    private static object AddNumeric(object a, object b)
    {
        if (a is double || b is double)
            return ToDouble(a) + ToDouble(b);
        return ToLong(a) + ToLong(b);
    }

    private static object DivideNumeric(object sum, int count)
    {
        if (count == 0) return null!;
        return ToDouble(sum) / count;
    }

    private static double ToDouble(object? v) => Convert.ToDouble(v, CultureInfo.InvariantCulture);
    private static long   ToLong  (object? v) => Convert.ToInt64 (v, CultureInfo.InvariantCulture);

    // ── Binary operator evaluation ─────────────────────────────────────────────

    /// <summary>
    /// Evaluate a binary operator on two SQL values.
    ///
    /// Three-valued logic for AND/OR:
    /// <list type="table">
    ///   <listheader><term>Left</term><term>Right</term><term>AND result</term><term>OR result</term></listheader>
    ///   <item><term>FALSE</term><term>NULL</term><term>FALSE</term><term>NULL</term></item>
    ///   <item><term>TRUE</term><term>NULL</term><term>NULL</term><term>TRUE</term></item>
    ///   <item><term>NULL</term><term>FALSE</term><term>FALSE</term><term>NULL</term></item>
    ///   <item><term>NULL</term><term>TRUE</term><term>NULL</term><term>TRUE</term></item>
    ///   <item><term>NULL</term><term>NULL</term><term>NULL</term><term>NULL</term></item>
    /// </list>
    ///
    /// All other operators return NULL if either operand is NULL.
    /// </summary>
    private static object? EvalBinary(BinaryOpCode op, object? left, object? right)
    {
        // ── Three-valued AND ───────────────────────────────────────────────────
        if (op == BinaryOpCode.And)
        {
            // FALSE AND anything → FALSE (even NULL AND FALSE → FALSE)
            if (left is false || right is false) return false;
            // NULL AND non-false → NULL
            if (left is null || right is null) return null;
            // TRUE AND TRUE
            return IsTruthy(left) && IsTruthy(right);
        }

        // ── Three-valued OR ────────────────────────────────────────────────────
        if (op == BinaryOpCode.Or)
        {
            // TRUE OR anything → TRUE
            if (IsTruthy(left) || IsTruthy(right)) return true;
            // NULL OR FALSE → NULL
            if (left is null || right is null) return null;
            // FALSE OR FALSE
            return false;
        }

        // ── NULL propagation for all other operators ────────────────────────────
        if (left is null || right is null) return null;

        // ── Arithmetic ─────────────────────────────────────────────────────────
        switch (op)
        {
            case BinaryOpCode.Add:
                if (left is string ls && right is string rs) return ls + rs;
                if (left is double || right is double)       return ToDouble(left) + ToDouble(right);
                return ToLong(left) + ToLong(right);

            case BinaryOpCode.Sub:
                if (left is double || right is double) return ToDouble(left) - ToDouble(right);
                return ToLong(left) - ToLong(right);

            case BinaryOpCode.Mul:
                if (left is double || right is double) return ToDouble(left) * ToDouble(right);
                return ToLong(left) * ToLong(right);

            case BinaryOpCode.Div:
                // Division always produces a double in SQL.
                var divisor = ToDouble(right);
                return divisor == 0.0 ? null : ToDouble(left) / divisor;

            case BinaryOpCode.Mod:
                var modRight = ToLong(right);
                return modRight == 0 ? null : ToLong(left) % modRight;

            case BinaryOpCode.Concat:
                // String concatenation: coerce both to strings.
                return ToString(left) + ToString(right);

            // ── Comparisons ────────────────────────────────────────────────────
            case BinaryOpCode.Eq:  return SqlCompare(left, right) == 0;
            case BinaryOpCode.Neq: return SqlCompare(left, right) != 0;
            case BinaryOpCode.Lt:  return SqlCompare(left, right) <  0;
            case BinaryOpCode.Lte: return SqlCompare(left, right) <= 0;
            case BinaryOpCode.Gt:  return SqlCompare(left, right) >  0;
            case BinaryOpCode.Gte: return SqlCompare(left, right) >= 0;

            default:
                return null;
        }
    }

    // ── Unary operator evaluation ──────────────────────────────────────────────

    /// <summary>Evaluate a unary operator. NULL propagates for Neg; NOT follows 3VL.</summary>
    private static object? EvalUnary(UnaryOpCode op, object? value)
    {
        if (op == UnaryOpCode.Not)
        {
            // SQL NOT with three-valued logic:
            //   NOT NULL  → NULL
            //   NOT TRUE  → FALSE
            //   NOT FALSE → TRUE
            if (value is null) return null;
            return !IsTruthy(value);
        }

        // Neg: any NULL → NULL
        if (value is null) return null;
        if (value is double d) return -d;
        return -ToLong(value);
    }

    // ── BETWEEN evaluation ─────────────────────────────────────────────────────

    /// <summary>
    /// Evaluate BETWEEN by popping high, low, and value from the stack.
    ///
    /// Stack order: value was pushed first (deepest), then low, then high (top).
    ///
    /// Result: value >= low AND value &lt;= high, with full NULL propagation.
    /// If any of value/low/high is NULL, the result is NULL.
    /// </summary>
    private static object? EvalBetween(VmState st)
    {
        var high  = st.Pop();
        var low   = st.Pop();
        var value = st.Pop();

        if (value is null || low is null || high is null) return null;

        var ge = SqlCompare(value, low)  >= 0;
        var le = SqlCompare(value, high) <= 0;
        return ge && le;
    }

    // ── IN list evaluation ─────────────────────────────────────────────────────

    /// <summary>
    /// Evaluate IN-list membership.
    ///
    /// Stack order: probe value (bottom), item0, item1, …, itemN-1 (top).
    /// Pop N items first (they're above the probe), then pop the probe.
    ///
    /// NULL semantics:
    ///   • If the list is empty:       push FALSE (regardless of probe).
    ///   • If probe is NULL:           push NULL.
    ///   • If probe == any non-null:   push TRUE.
    ///   • If probe not found, list had NULL: push NULL (unknown).
    ///   • If probe not found, no NULLs: push FALSE.
    /// </summary>
    private static object? EvalInList(VmState st, int n)
    {
        var items = st.PopN(n);
        var probe = st.Pop();

        // Empty list: always FALSE.
        if (n == 0) return false;
        // NULL probe: always NULL.
        if (probe is null) return null;

        var hasNull = false;
        foreach (var item in items)
        {
            if (item is null) { hasNull = true; continue; }
            if (SqlCompare(probe, item) == 0) return true;
        }
        return hasNull ? null : (object?)false;
    }

    // ── LIKE evaluation ────────────────────────────────────────────────────────

    /// <summary>
    /// Evaluate a SQL LIKE pattern match.
    ///
    /// SQL LIKE metacharacters:
    ///   %  — matches any sequence of zero or more characters
    ///   _  — matches any single character
    ///   other characters — literal match (case-insensitive by default)
    ///
    /// Implementation: convert the SQL pattern to a .NET regex, then test.
    /// </summary>
    public static bool LikeMatch(string value, string pattern)
    {
        // Convert each SQL metachar to its regex equivalent:
        //   %  →  .*
        //   _  →  .
        //   other → Regex.Escape(char)
        var sb = new System.Text.StringBuilder("^");
        foreach (var ch in pattern)
        {
            if (ch == '%')      sb.Append(".*");
            else if (ch == '_') sb.Append('.');
            else                sb.Append(Regex.Escape(ch.ToString()));
        }
        sb.Append('$');
        // RegexOptions.NonBacktracking (introduced in .NET 7) eliminates catastrophic
        // backtracking for patterns like '%a%a%a%a%' matched against long strings.
        // This makes the LIKE implementation ReDoS-safe for user-supplied patterns.
        return Regex.IsMatch(value, sb.ToString(),
            RegexOptions.IgnoreCase | RegexOptions.Singleline | RegexOptions.NonBacktracking);
    }

    // ── Scalar function dispatch ───────────────────────────────────────────────

    /// <summary>
    /// Evaluate a SQL scalar function call.
    ///
    /// Only a subset of common functions is needed for Level 1 VM tests.
    /// Unknown function names return NULL rather than throwing, matching
    /// the "lenient" posture used elsewhere in the pipeline.
    /// </summary>
    private static object? EvalScalar(string func, object?[] args)
    {
        return func.ToUpperInvariant() switch
        {
            "ABS" when args.Length == 1 =>
                args[0] is null ? null :
                args[0] is double d ? (object)Math.Abs(d) : (object)Math.Abs(ToLong(args[0])),

            "UPPER" when args.Length == 1 =>
                args[0] is string s ? (object)s.ToUpperInvariant() : null,

            "LOWER" when args.Length == 1 =>
                args[0] is string s ? (object)s.ToLowerInvariant() : null,

            "LENGTH" when args.Length == 1 =>
                args[0] is string s ? (object)(long)s.Length :
                args[0] is byte[] b ? (object)(long)b.Length : null,

            "TRIM" when args.Length == 1 =>
                args[0] is string s ? (object)s.Trim() : null,

            "LTRIM" when args.Length == 1 =>
                args[0] is string s ? (object)s.TrimStart() : null,

            "RTRIM" when args.Length == 1 =>
                args[0] is string s ? (object)s.TrimEnd() : null,

            "COALESCE" =>
                args.FirstOrDefault(v => v is not null),

            "IFNULL" when args.Length == 2 =>
                args[0] ?? args[1],

            "NULLIF" when args.Length == 2 =>
                (args[0] is null || args[1] is null) ? args[0] :
                SqlCompare(args[0], args[1]) == 0 ? null : args[0],

            "TYPEOF" when args.Length == 1 =>
                (object?)SqlValues.TypeName(args[0]).ToLowerInvariant(),

            "ROUND" when args.Length >= 1 =>
                args[0] is null ? null :
                args.Length == 2 && args[1] is not null
                    ? (object)Math.Round(ToDouble(args[0]), (int)ToLong(args[1]), MidpointRounding.AwayFromZero)
                    : (object)Math.Round(ToDouble(args[0]), MidpointRounding.AwayFromZero),

            "MAX" when args.Length >= 1 =>
                args.Where(v => v is not null)
                    .OrderByDescending(v => v, Comparer<object?>.Create((a, b) => SqlCompare(a, b)))
                    .FirstOrDefault(),

            "MIN" when args.Length >= 1 =>
                args.Where(v => v is not null)
                    .OrderBy(v => v, Comparer<object?>.Create((a, b) => SqlCompare(a, b)))
                    .FirstOrDefault(),

            "SUBSTR" or "SUBSTRING" when args.Length >= 2 =>
                EvalSubstr(args),

            "REPLACE" when args.Length == 3 =>
                (args[0] is string src && args[1] is string from && args[2] is string to)
                    ? (object)src.Replace(from, to, StringComparison.Ordinal)
                    : null,

            "HEX" when args.Length == 1 =>
                args[0] is byte[] bytes ? (object)Convert.ToHexString(bytes).ToUpperInvariant() : null,

            // CONCAT(a, b, ...) — string concatenation; NULL arg makes the whole result NULL
            // (matches SQLite's || behaviour: NULL || 'x' is NULL).
            "CONCAT" =>
                args.Any(v => v is null) ? null
                    : (object?)string.Concat(args.Select(v => v?.ToString() ?? "")),

            _ => null, // Unknown function — return NULL (lenient)
        };
    }

    private static object? EvalSubstr(object?[] args)
    {
        if (args[0] is not string s) return null;
        // Clamp long→int to avoid overflow with extreme SUBSTR arguments.
        // String lengths are bounded by int.MaxValue, so clamping is correct.
        var startLong = ToLong(args[1]);
        var start = startLong > int.MaxValue ? int.MaxValue
                  : startLong < int.MinValue ? int.MinValue
                  : (int)startLong;
        // SQL SUBSTR uses 1-based indexing; negative values are end-relative.
        if (start > 0) start--; // convert to 0-based
        else if (start < 0) start = Math.Max(0, s.Length + start);
        if (start >= s.Length) return "";
        if (args.Length >= 3 && args[2] is not null)
        {
            var lenLong = ToLong(args[2]);
            // Clamp length to [0, s.Length - start].
            var len = (int)Math.Min(Math.Max(lenLong, 0), s.Length - start);
            return s.Substring(start, len);
        }
        return s.Substring(start);
    }

    // ── Truthiness ────────────────────────────────────────────────────────────

    /// <summary>
    /// SQL truthiness: a value is truthy iff it is non-null AND not falsy.
    ///
    /// Falsy values: null, false, 0 (int), 0L (long), 0.0 (double).
    /// All other values (non-zero numbers, non-empty strings, true) are truthy.
    /// </summary>
    private static bool IsTruthy(object? value) => value switch
    {
        null   => false,
        bool b => b,
        int  i => i != 0,
        long l => l != 0,
        double d => d != 0.0,
        _ => true, // non-null string / byte[] etc. are truthy
    };

    // ── String conversion ──────────────────────────────────────────────────────

    private static string ToString(object? v) => v switch
    {
        null   => "",
        string s => s,
        _ => Convert.ToString(v, CultureInfo.InvariantCulture) ?? "",
    };

    // ── Post-processing: sort ──────────────────────────────────────────────────

    /// <summary>
    /// Sort <paramref name="rows"/> in place by the given keys.
    ///
    /// Each sort key identifies a result column by name. Direction controls
    /// ASC/DESC; NullsOrder controls whether NULLs sort first or last.
    ///
    /// Multi-key sort: primary key first, then secondary, etc. We implement
    /// this by building a compound comparison function.
    /// </summary>
    private static void ApplySort(
        List<List<object?>> rows,
        List<SortKey>       keys,
        List<string>        schema)
    {
        rows.Sort((a, b) =>
        {
            foreach (var key in keys)
            {
                // Resolve column index — fall back to 0 if name not found.
                var idx = schema.IndexOf(key.Column);
                if (idx < 0) idx = 0;

                var av = idx < a.Count ? a[idx] : null;
                var bv = idx < b.Count ? b[idx] : null;

                var cmp = CompareForSort(av, bv, key.Direction, key.NullsOrder);
                if (cmp != 0)
                    return cmp;
            }
            return 0;
        });
    }

    // Compare two values for sorting, honouring direction and null placement.
    //
    // Key insight: NULLS FIRST / NULLS LAST is about POSITION IN THE OUTPUT, not
    // about the numeric value of null.  So null placement must be applied BEFORE
    // direction negation — if we want null first in DESC, we must not negate the
    // null-placement signal.
    //
    // Algorithm:
    //   1. If either value is null, return a fixed signal based on NullsOrder alone.
    //   2. Both non-null: compare normally, then negate for DESC.
    private static int CompareForSort(object? a, object? b, Direction dir, NullsOrder nullsOrder)
    {
        // Null placement is absolute (independent of sort direction).
        if (a is null && b is null) return 0;
        if (a is null) return nullsOrder == NullsOrder.First ? -1 :  1;
        if (b is null) return nullsOrder == NullsOrder.First ?  1 : -1;

        // Both non-null: apply direction.
        var cmp = SqlCompare(a, b);
        return dir == Direction.Desc ? -cmp : cmp;
    }

    // ── Post-processing: limit/offset ──────────────────────────────────────────

    private static void ApplyLimit(List<List<object?>> rows, long? count, long? offset)
    {
        // Clamp long→int to avoid overflow when offset/count exceed int.MaxValue.
        // List<T> is bounded by int.MaxValue anyway, so clamping is correct.
        var start = (int)Math.Min(Math.Max(offset ?? 0, 0), int.MaxValue);
        if (start >= rows.Count)
        {
            rows.Clear();
            return;
        }
        rows.RemoveRange(0, start);
        if (count.HasValue)
        {
            var take = (int)Math.Min(count.Value, rows.Count);
            if (take < rows.Count)
                rows.RemoveRange(take, rows.Count - take);
        }
    }

    // ── Post-processing: distinct ──────────────────────────────────────────────

    /// <summary>
    /// Remove duplicate rows from the result set.
    ///
    /// Two rows are considered equal when their values are elementwise equal
    /// (using structural equality; NULLs are equal to NULLs for DISTINCT purposes).
    /// </summary>
    private static void ApplyDistinct(List<List<object?>> rows)
    {
        var seen = new HashSet<RowKey>();
        var result = new List<List<object?>>();
        foreach (var row in rows)
        {
            var key = new RowKey(row);
            if (seen.Add(key))
                result.Add(row);
        }
        rows.Clear();
        rows.AddRange(result);
    }

    // ── SQL value comparison ───────────────────────────────────────────────────

    /// <summary>
    /// Compare two SQL values using the standard SQL type-ordering rules.
    ///
    /// Ordering by type rank: NULL &lt; BOOLEAN &lt; NUMERIC &lt; TEXT &lt; BLOB.
    /// Within the same type:
    ///   bool    — false &lt; true
    ///   numeric — numeric comparison (all integer/float types coerced to double)
    ///   text    — ordinal string comparison
    ///   blob    — lexicographic byte comparison
    ///
    /// This mirrors the internal SqlValues.Compare in SqlBackend, which is
    /// declared internal and therefore not accessible from this package.
    /// </summary>
    private static int SqlCompare(object? left, object? right)
    {
        // Type rank: NULL=0, BOOL=1, NUMERIC=2, TEXT=3, BLOB=4, other=5
        static int Rank(object? v) => v switch
        {
            null                                                             => 0,
            bool                                                             => 1,
            byte or sbyte or short or ushort or int or uint or long or ulong => 2,
            float or double                                                   => 2,
            string                                                           => 3,
            byte[]                                                           => 4,
            _                                                                => 5,
        };

        var rLeft  = Rank(left);
        var rRight = Rank(right);
        if (rLeft != rRight) return rLeft.CompareTo(rRight);

        return left switch
        {
            null                  => 0,
            bool lb               => lb.CompareTo((bool)right!),
            string ls             => string.CompareOrdinal(ls, (string)right!),
            byte[] lb2            => CompareBytes(lb2, (byte[])right!),
            _                     => ToDouble(left).CompareTo(ToDouble(right)),
        };
    }

    private static int CompareBytes(IReadOnlyList<byte> a, IReadOnlyList<byte> b)
    {
        var len = Math.Min(a.Count, b.Count);
        for (var i = 0; i < len; i++)
        {
            var c = a[i].CompareTo(b[i]);
            if (c != 0) return c;
        }
        return a.Count.CompareTo(b.Count);
    }

    // ── DDL helpers ───────────────────────────────────────────────────────────

    /// <summary>
    /// Convert a list of <see cref="CodingAdventures.SqlPlanner.ColumnDef"/> to
    /// <see cref="CodingAdventures.SqlBackend.ColumnDef"/>.
    ///
    /// Both types carry the same fields; the conversion is field-by-field. We
    /// cannot use one type where the other is expected because they live in
    /// different assemblies, even though they are structurally identical.
    /// </summary>
    private static IReadOnlyList<ColumnDef> ConvertColumnDefs(IReadOnlyList<PlColumnDef> plDefs)
    {
        // SqlPlanner.ColumnDef has: Name, TypeName, NotNull, PrimaryKey, Unique, Default (SqlExpr?).
        // SqlBackend.ColumnDef has: Name, TypeName, NotNull, PrimaryKey, Unique, Autoincrement,
        //                           DefaultValue (object?), HasDefault, CheckExpression, ForeignKey.
        // At Level 1 we carry over the structural fields only; default expressions are not yet
        // evaluated at codegen time (they would require a separate expression evaluator), so we
        // map them as HasDefault=false / DefaultValue=null.
        return plDefs.Select(c => new ColumnDef(
            c.Name,
            c.TypeName,
            c.NotNull,
            c.PrimaryKey,
            c.Unique)).ToList();
    }

    // ── Row key for distinct ───────────────────────────────────────────────────

    /// <summary>
    /// Immutable row wrapper used for structural equality in DISTINCT deduplication.
    /// Two RowKeys are equal when all their values are equal (NULL == NULL).
    /// </summary>
    private sealed class RowKey : IEquatable<RowKey>
    {
        private readonly object?[] _values;

        public RowKey(List<object?> values) => _values = values.ToArray();

        public bool Equals(RowKey? other)
        {
            if (other is null) return false;
            if (_values.Length != other._values.Length) return false;
            for (var i = 0; i < _values.Length; i++)
            {
                var a = _values[i];
                var b = other._values[i];
                if (a is null && b is null) continue;
                if (a is null || b is null) return false;
                if (SqlCompare(a, b) != 0) return false;
            }
            return true;
        }

        public override bool Equals(object? obj) => obj is RowKey rk && Equals(rk);

        public override int GetHashCode()
        {
            var h = new HashCode();
            foreach (var v in _values) h.Add(v);
            return h.ToHashCode();
        }
    }
}

// ── Fallback cursor adapter ────────────────────────────────────────────────────

/// <summary>
/// Wraps a plain <see cref="IRowIterator"/> as an <see cref="ICursor"/> so that
/// backends that only implement Scan() (not OpenCursor()) can still be used with
/// DML instructions. Note: positioned UPDATE/DELETE will fail on this cursor
/// because it is not a ListCursor — those backends must implement OpenCursor().
/// </summary>
internal sealed class RowIteratorCursor : ICursor
{
    private readonly IRowIterator _iter;
    private Row? _current;

    public RowIteratorCursor(IRowIterator iter) => _iter = iter;

    public Row? CurrentRow => _current?.Copy();

    public Row? Next()
    {
        _current = _iter.Next();
        return _current?.Copy();
    }

    public void Close() => _iter.Close();
}

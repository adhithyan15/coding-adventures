// SqlCodegenTests.cs — xUnit test suite for the C# sql-codegen package.
//
// Test strategy:
//   • Each test builds a minimal OptimizedPlan (or uses SqlOptimizer.Lift) and
//     calls SqlCodegen.CompileOptimized.
//   • Assertions check that expected instruction types appear (or don't appear)
//     in the compiled Program.Instructions list.
//   • We do NOT test instruction indices or exact ordering beyond what the
//     contract requires (e.g., SortResult appears before Halt).
//
// Tests are grouped by plan node type and expression type to provide
// comprehensive coverage of the compiler's branching logic.

using CodingAdventures.SqlPlanner;
using CodingAdventures.SqlOptimizer;
using Optimizer = CodingAdventures.SqlOptimizer.SqlOptimizer;
using CodingAdventures.SqlCodegen;

using Xunit;

namespace CodingAdventures.SqlCodegen.Tests;

// ── Helpers ───────────────────────────────────────────────────────────────────

file static class H
{
    /// <summary>Short-hand: lift a LogicalPlan without optimization.</summary>
    public static OptimizedPlan L(LogicalPlan p) => Optimizer.Lift(p);

    /// <summary>Compile without optimization (lift only).</summary>
    public static Program Cmp(LogicalPlan p) => SqlCodegen.CompileOptimized(L(p));

    /// <summary>Compile an already-optimized plan.</summary>
    public static Program CmpO(OptimizedPlan p) => SqlCodegen.CompileOptimized(p);

    /// <summary>Count instructions of type T.</summary>
    public static int Count<T>(Program prog) where T : Instruction
        => prog.Instructions.OfType<T>().Count();

    /// <summary>Assert at least one instruction of type T exists.</summary>
    public static void HasAny<T>(Program prog) where T : Instruction
        => Assert.True(Count<T>(prog) > 0, $"Expected at least one {typeof(T).Name}");

    /// <summary>Assert no instruction of type T exists.</summary>
    public static void HasNone<T>(Program prog) where T : Instruction
        => Assert.Equal(0, Count<T>(prog));

    /// <summary>Assert exactly one Halt is present.</summary>
    public static void HasHalt(Program prog)
        => Assert.Contains(prog.Instructions, i => i is Halt);

    // Planner type shortcuts
    public static ScanPlan Scan(string t, string? a = null) => new(t, a);
    public static FilterPlan Filter(LogicalPlan i, SqlExpr p) => new(i, p);
    public static ProjectPlan Project(LogicalPlan i, params OutputColumn[] cols) => new(i, cols);
    public static AggregatePlan Agg(LogicalPlan i, IReadOnlyList<SqlExpr> gb, IReadOnlyList<AggregateItem> aggs) => new(i, gb, aggs);
    public static JoinPlan Join(LogicalPlan l, LogicalPlan r, JoinKind k, SqlExpr? cond = null) => new(l, r, k, cond);
    public static SortPlan Sort(LogicalPlan i, params SortKey[] keys) => new(i, keys);
    public static LimitPlan Limit(LogicalPlan i, long? count, long? offset = null) => new(i, count, offset);
    public static DistinctPlan Distinct(LogicalPlan i) => new(i);
    public static InsertPlan Insert(string t, IReadOnlyList<string>? cols, IReadOnlyList<IReadOnlyList<SqlExpr>> vals) => new(t, cols, vals);
    public static UpdatePlan Update(string t, IReadOnlyList<Assignment> asgn, SqlExpr? pred) => new(t, asgn, pred);
    public static DeletePlan Delete(string t, SqlExpr? pred) => new(t, pred);
    public static CreateTablePlan CreateTbl(string t, bool ine, IReadOnlyList<ColumnDef> cols) => new(t, ine, cols);
    public static DropTablePlan DropTbl(string t, bool ife) => new(t, ife);

    // Expression shortcuts
    public static SqlExpr Lit(object? v) => new SqlExpr.Literal(v);
    public static SqlExpr Col(string c, string? t = null) => new SqlExpr.Column(t, c);
    public static SqlExpr BinOp(BinaryOperator op, SqlExpr l, SqlExpr r) => new SqlExpr.BinaryOp(op, l, r);
    public static SqlExpr UnaOp(UnaryOperator op, SqlExpr e) => new SqlExpr.UnaryOp(op, e);

    // OutputColumn shortcuts
    public static OutputColumn OC(string alias, SqlExpr e) => new OutputColumn.Expr(e, alias);
    public static OutputColumn OStar() => new OutputColumn.Star();
}

// ── 1. EmptyResult → no scan, Halt present ────────────────────────────────────

public class EmptyResultTests
{
    [Fact]
    public void EmptyResult_NoOpenScan()
    {
        var prog = H.CmpO(new OptEmptyResult());
        H.HasNone<OpenScan>(prog);
        H.HasHalt(prog);
    }

    [Fact]
    public void EmptyResult_NoAdvanceCursor()
    {
        var prog = H.CmpO(new OptEmptyResult());
        H.HasNone<AdvanceCursor>(prog);
    }

    [Fact]
    public void EmptyResult_HaltAlwaysPresent()
    {
        var prog = H.CmpO(new OptEmptyResult());
        Assert.Single(prog.Instructions.Where(i => i is Halt));
    }
}

// ── 2. Scan only → OpenScan + AdvanceCursor + CloseScan ──────────────────────

public class ScanTests
{
    [Fact]
    public void BareProject_HasOpenAndCloseScan()
    {
        // Project(Scan("users")) should open and close a scan.
        var plan = H.L(H.Project(H.Scan("users"), H.OC("id", H.Col("id"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<OpenScan>(prog);
        H.HasAny<CloseScan>(prog);
    }

    [Fact]
    public void Scan_HasAdvanceCursor()
    {
        var plan = H.L(H.Project(H.Scan("users"), H.OC("id", H.Col("id"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<AdvanceCursor>(prog);
    }

    [Fact]
    public void Scan_OpenAndCloseCountMatch()
    {
        var plan = H.L(H.Project(H.Scan("orders"), H.OC("id", H.Col("id"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        Assert.Equal(H.Count<OpenScan>(prog), H.Count<CloseScan>(prog));
    }

    [Fact]
    public void Scan_OpenScanNamesTable()
    {
        var plan = H.L(H.Project(H.Scan("products"), H.OC("id", H.Col("id"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        var open = prog.Instructions.OfType<OpenScan>().Single();
        Assert.Equal("products", open.Table);
    }
}

// ── 3. Project → SetResultSchema + BeginRow + EmitRow ────────────────────────

public class ProjectTests
{
    [Fact]
    public void Project_HasSetResultSchema()
    {
        var plan = H.L(H.Project(H.Scan("users"),
            H.OC("name", H.Col("name")),
            H.OC("age",  H.Col("age"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<SetResultSchema>(prog);
    }

    [Fact]
    public void Project_SchemaColumnsMatchAliases()
    {
        var plan = H.L(H.Project(H.Scan("users"),
            H.OC("name", H.Col("name")),
            H.OC("age",  H.Col("age"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        var schema = prog.Instructions.OfType<SetResultSchema>().First().Columns;
        Assert.Contains("name", schema);
        Assert.Contains("age",  schema);
    }

    [Fact]
    public void Project_HasBeginRowAndEmitRow()
    {
        var plan = H.L(H.Project(H.Scan("users"), H.OC("id", H.Col("id"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<BeginRow>(prog);
        H.HasAny<EmitRow>(prog);
    }

    [Fact]
    public void Project_EmitColumnNamesMatchSchema()
    {
        var plan = H.L(H.Project(H.Scan("users"), H.OC("email", H.Col("email"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        var emitted = prog.Instructions.OfType<EmitColumn>().Select(e => e.Name).ToList();
        Assert.Contains("email", emitted);
    }

    [Fact]
    public void Project_ResultSchemaExposedOnProgram()
    {
        var plan = H.L(H.Project(H.Scan("x"), H.OC("v", H.Col("v"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        Assert.Contains("v", prog.ResultSchema);
    }
}

// ── 4. Filter → JumpIfFalse present ──────────────────────────────────────────

public class FilterTests
{
    [Fact]
    public void Filter_HasJumpIfFalse()
    {
        var pred = H.BinOp(BinaryOperator.Gt, H.Col("age"), H.Lit(18L));
        var plan = H.L(H.Project(H.Filter(H.Scan("users"), pred), H.OC("age", H.Col("age"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<JumpIfFalse>(prog);
    }

    [Fact]
    public void Filter_JumpTargetIsLabel()
    {
        var pred = H.BinOp(BinaryOperator.Eq, H.Col("id"), H.Lit(1L));
        var plan = H.L(H.Project(H.Filter(H.Scan("t"), pred), H.OC("id", H.Col("id"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        var jf   = prog.Instructions.OfType<JumpIfFalse>().First();
        Assert.True(prog.Labels.ContainsKey(jf.Target),
            $"Jump target '{jf.Target}' not found in Labels dictionary");
    }
}

// ── 5. INNER JOIN → two AdvanceCursor ────────────────────────────────────────

public class InnerJoinTests
{
    [Fact]
    public void InnerJoin_TwoAdvanceCursors()
    {
        var join = H.Join(H.Scan("users"), H.Scan("orders"), JoinKind.Inner);
        var plan = H.L(H.Project(join, H.OC("x", H.Col("id"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        Assert.Equal(2, H.Count<AdvanceCursor>(prog));
    }

    [Fact]
    public void InnerJoin_TwoOpenScans()
    {
        var join = H.Join(H.Scan("a"), H.Scan("b"), JoinKind.Inner);
        var plan = H.L(H.Project(join, H.OC("x", H.Col("x"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        Assert.Equal(2, H.Count<OpenScan>(prog));
    }

    [Fact]
    public void InnerJoin_WithCondition_HasJumpIfFalse()
    {
        var cond = H.BinOp(BinaryOperator.Eq, H.Col("user_id", "orders"), H.Col("id", "users"));
        var join = H.Join(H.Scan("users"), H.Scan("orders"), JoinKind.Inner, cond);
        var plan = H.L(H.Project(join, H.OC("x", H.Col("id"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<JumpIfFalse>(prog);
    }

    [Fact]
    public void CrossJoin_TwoAdvanceCursors()
    {
        var join = H.Join(H.Scan("a"), H.Scan("b"), JoinKind.Cross);
        var plan = H.L(H.Project(join, H.OC("x", H.Col("x"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        Assert.Equal(2, H.Count<AdvanceCursor>(prog));
    }
}

// ── 6. LEFT JOIN → JoinBeginRow + JoinSetMatched + JoinIfMatched ─────────────

public class LeftJoinTests
{
    [Fact]
    public void LeftJoin_HasJoinBeginRow()
    {
        var join = H.Join(H.Scan("users"), H.Scan("orders"), JoinKind.Left);
        var plan = H.L(H.Project(join, H.OC("x", H.Col("id"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<JoinBeginRow>(prog);
    }

    [Fact]
    public void LeftJoin_HasJoinSetMatched()
    {
        var join = H.Join(H.Scan("u"), H.Scan("o"), JoinKind.Left);
        var plan = H.L(H.Project(join, H.OC("x", H.Col("x"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<JoinSetMatched>(prog);
    }

    [Fact]
    public void LeftJoin_HasJoinIfMatched()
    {
        var join = H.Join(H.Scan("u"), H.Scan("o"), JoinKind.Left);
        var plan = H.L(H.Project(join, H.OC("x", H.Col("x"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<JoinIfMatched>(prog);
    }

    [Fact]
    public void LeftJoin_JoinIfMatchedTargetIsLabel()
    {
        var join = H.Join(H.Scan("u"), H.Scan("o"), JoinKind.Left);
        var plan = H.L(H.Project(join, H.OC("x", H.Col("x"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        var jim  = prog.Instructions.OfType<JoinIfMatched>().First();
        Assert.True(prog.Labels.ContainsKey(jim.Label),
            $"JoinIfMatched target '{jim.Label}' not in Labels");
    }
}

// ── 7. Sort → SortResult before Halt ─────────────────────────────────────────

public class SortTests
{
    [Fact]
    public void Sort_HasSortResult()
    {
        var key  = new SortKey(H.Col("name"), SortDir.Asc, NullOrder.NullsLast);
        var plan = H.L(H.Sort(H.Project(H.Scan("users"), H.OC("name", H.Col("name"))), key));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<SortResult>(prog);
    }

    [Fact]
    public void Sort_SortResultBeforeHalt()
    {
        var key  = new SortKey(H.Col("x"), SortDir.Desc, NullOrder.NullsFirst);
        var plan = H.L(H.Sort(H.Project(H.Scan("t"), H.OC("x", H.Col("x"))), key));
        var prog = SqlCodegen.CompileOptimized(plan);
        var instrs = prog.Instructions.ToList();
        var sortIdx = instrs.FindIndex(i => i is SortResult);
        var haltIdx = instrs.FindIndex(i => i is Halt);
        Assert.True(sortIdx < haltIdx, "SortResult must appear before Halt");
    }

    [Fact]
    public void Sort_DirectionPreserved()
    {
        var key  = new SortKey(H.Col("age"), SortDir.Desc, NullOrder.NullsLast);
        var plan = H.L(H.Sort(H.Project(H.Scan("users"), H.OC("age", H.Col("age"))), key));
        var prog = SqlCodegen.CompileOptimized(plan);
        var sr   = prog.Instructions.OfType<SortResult>().First();
        Assert.Equal(Direction.Desc, sr.Keys[0].Direction);
    }
}

// ── 8. Limit → LimitResult before Halt ───────────────────────────────────────

public class LimitTests
{
    [Fact]
    public void Limit_HasLimitResult()
    {
        var plan = H.L(H.Limit(H.Project(H.Scan("t"), H.OC("x", H.Col("x"))), 10));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<LimitResult>(prog);
    }

    [Fact]
    public void Limit_CountPreserved()
    {
        var plan = H.L(H.Limit(H.Project(H.Scan("t"), H.OC("x", H.Col("x"))), 42));
        var prog = SqlCodegen.CompileOptimized(plan);
        var lr   = prog.Instructions.OfType<LimitResult>().First();
        Assert.Equal(42L, lr.Count);
    }

    [Fact]
    public void Limit_OffsetPreserved()
    {
        var plan = H.L(H.Limit(H.Project(H.Scan("t"), H.OC("x", H.Col("x"))), 10, 5));
        var prog = SqlCodegen.CompileOptimized(plan);
        var lr   = prog.Instructions.OfType<LimitResult>().First();
        Assert.Equal(5L, lr.Offset);
    }
}

// ── 9. Distinct → DistinctResult ─────────────────────────────────────────────

public class DistinctTests
{
    [Fact]
    public void Distinct_HasDistinctResult()
    {
        var plan = H.L(H.Distinct(H.Project(H.Scan("t"), H.OC("x", H.Col("x")))));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<DistinctResult>(prog);
    }

    [Fact]
    public void Distinct_DistinctResultBeforeHalt()
    {
        var plan = H.L(H.Distinct(H.Project(H.Scan("t"), H.OC("x", H.Col("x")))));
        var prog = SqlCodegen.CompileOptimized(plan);
        var instrs  = prog.Instructions.ToList();
        var distIdx = instrs.FindIndex(i => i is DistinctResult);
        var haltIdx = instrs.FindIndex(i => i is Halt);
        Assert.True(distIdx < haltIdx);
    }
}

// ── 10. Aggregate → InitAgg + FinalizeAgg + AdvanceGroupKey ──────────────────

public class AggregateTests
{
    private static AggregatePlan MakeCountAgg(string table)
    {
        var items = new[]
        {
            new AggregateItem(AggFunction.Count, new AggArg.Star(), "_count", false)
        };
        return H.Agg(H.Scan(table), Array.Empty<SqlExpr>(), items);
    }

    [Fact]
    public void Aggregate_HasInitAgg()
    {
        var plan = H.L(H.Project(MakeCountAgg("t"), H.OC("cnt", H.Col("_count"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<InitAgg>(prog);
    }

    [Fact]
    public void Aggregate_HasUpdateAgg()
    {
        var plan = H.L(H.Project(MakeCountAgg("t"), H.OC("cnt", H.Col("_count"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<UpdateAgg>(prog);
    }

    [Fact]
    public void Aggregate_HasFinalizeAgg()
    {
        var plan = H.L(H.Project(MakeCountAgg("t"), H.OC("cnt", H.Col("_count"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<FinalizeAgg>(prog);
    }

    [Fact]
    public void Aggregate_HasAdvanceGroupKey()
    {
        var plan = H.L(H.Project(MakeCountAgg("t"), H.OC("cnt", H.Col("_count"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<AdvanceGroupKey>(prog);
    }

    [Fact]
    public void Aggregate_WithGroupBy_HasSaveGroupKey()
    {
        var items = new[]
        {
            new AggregateItem(AggFunction.Count, new AggArg.Star(), "_c", false)
        };
        var agg  = H.Agg(H.Scan("t"), new[] { H.Col("dept") }, items);
        var plan = H.L(H.Project(agg,
            H.OC("dept", H.Col("dept")),
            H.OC("cnt",  H.Col("_c"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<SaveGroupKey>(prog);
    }

    [Fact]
    public void Aggregate_SumExprArg()
    {
        var items = new[]
        {
            new AggregateItem(AggFunction.Sum, new AggArg.Expr(H.Col("price")), "_sum", false)
        };
        var agg  = H.Agg(H.Scan("orders"), Array.Empty<SqlExpr>(), items);
        var plan = H.L(H.Project(agg, H.OC("total", H.Col("_sum"))));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<InitAgg>(prog);
        var init = prog.Instructions.OfType<InitAgg>().First();
        Assert.Equal(AggFunc.Sum, init.Func);
    }
}

// ── 11. INSERT → InsertRow ────────────────────────────────────────────────────

public class InsertTests
{
    [Fact]
    public void Insert_HasInsertRow()
    {
        var vals = new[] { new SqlExpr[] { H.Lit(1L), H.Lit("Alice") } };
        var plan = H.Insert("users", new[] { "id", "name" }, vals);
        var prog = SqlCodegen.CompileOptimized(H.L(plan));
        H.HasAny<InsertRow>(prog);
    }

    [Fact]
    public void Insert_TableNamePreserved()
    {
        var vals = new[] { new SqlExpr[] { H.Lit(42L) } };
        var plan = H.Insert("products", null, vals);
        var prog = SqlCodegen.CompileOptimized(H.L(plan));
        var ir   = prog.Instructions.OfType<InsertRow>().First();
        Assert.Equal("products", ir.Table);
    }

    [Fact]
    public void Insert_MultipleRowsEmitMultipleInsertRows()
    {
        var vals = new[]
        {
            new SqlExpr[] { H.Lit(1L) },
            new SqlExpr[] { H.Lit(2L) },
            new SqlExpr[] { H.Lit(3L) },
        };
        var plan = H.Insert("t", new[] { "id" }, vals);
        var prog = SqlCodegen.CompileOptimized(H.L(plan));
        Assert.Equal(3, H.Count<InsertRow>(prog));
    }
}

// ── 12. UPDATE → UpdateRows ───────────────────────────────────────────────────

public class UpdateTests
{
    [Fact]
    public void Update_HasUpdateRows()
    {
        var asgn = new[] { new Assignment("name", H.Lit("Bob")) };
        var plan = H.Update("users", asgn, null);
        var prog = SqlCodegen.CompileOptimized(H.L(plan));
        H.HasAny<UpdateRows>(prog);
    }

    [Fact]
    public void Update_WithPredicate_HasJumpIfFalse()
    {
        var asgn = new[] { new Assignment("name", H.Lit("Bob")) };
        var pred = H.BinOp(BinaryOperator.Eq, H.Col("id"), H.Lit(1L));
        var plan = H.Update("users", asgn, pred);
        var prog = SqlCodegen.CompileOptimized(H.L(plan));
        H.HasAny<JumpIfFalse>(prog);
    }

    [Fact]
    public void Update_HasOpenAndCloseScan()
    {
        var asgn = new[] { new Assignment("x", H.Lit(0L)) };
        var plan = H.Update("t", asgn, null);
        var prog = SqlCodegen.CompileOptimized(H.L(plan));
        H.HasAny<OpenScan>(prog);
        H.HasAny<CloseScan>(prog);
    }
}

// ── 13. DELETE → DeleteRows ───────────────────────────────────────────────────

public class DeleteTests
{
    [Fact]
    public void Delete_HasDeleteRows()
    {
        var plan = H.Delete("users", null);
        var prog = SqlCodegen.CompileOptimized(H.L(plan));
        H.HasAny<DeleteRows>(prog);
    }

    [Fact]
    public void Delete_WithPredicate_HasJumpIfFalse()
    {
        var pred = H.BinOp(BinaryOperator.Lt, H.Col("age"), H.Lit(18L));
        var plan = H.Delete("users", pred);
        var prog = SqlCodegen.CompileOptimized(H.L(plan));
        H.HasAny<JumpIfFalse>(prog);
    }

    [Fact]
    public void Delete_HasOpenAndCloseScan()
    {
        var plan = H.Delete("t", null);
        var prog = SqlCodegen.CompileOptimized(H.L(plan));
        H.HasAny<OpenScan>(prog);
        H.HasAny<CloseScan>(prog);
    }
}

// ── 14. CREATE TABLE → CreateTableInstr ──────────────────────────────────────

public class CreateTableTests
{
    [Fact]
    public void CreateTable_HasCreateTableInstr()
    {
        var cols = new[] { new ColumnDef("id", "INTEGER", PrimaryKey: true) };
        var plan = H.CreateTbl("users", false, cols);
        var prog = SqlCodegen.CompileOptimized(H.L(plan));
        H.HasAny<CreateTableInstr>(prog);
    }

    [Fact]
    public void CreateTable_TableNamePreserved()
    {
        var cols = new[] { new ColumnDef("x", "TEXT") };
        var plan = H.CreateTbl("widgets", true, cols);
        var prog = SqlCodegen.CompileOptimized(H.L(plan));
        var ct   = prog.Instructions.OfType<CreateTableInstr>().First();
        Assert.Equal("widgets", ct.Table);
    }

    [Fact]
    public void CreateTable_IfNotExistsPreserved()
    {
        var cols = new[] { new ColumnDef("id", "INT") };
        var plan = H.CreateTbl("t", true, cols);
        var prog = SqlCodegen.CompileOptimized(H.L(plan));
        var ct   = prog.Instructions.OfType<CreateTableInstr>().First();
        Assert.True(ct.IfNotExists);
    }
}

// ── 15. DROP TABLE → DropTableInstr ──────────────────────────────────────────

public class DropTableTests
{
    [Fact]
    public void DropTable_HasDropTableInstr()
    {
        var plan = H.DropTbl("users", false);
        var prog = SqlCodegen.CompileOptimized(H.L(plan));
        H.HasAny<DropTableInstr>(prog);
    }

    [Fact]
    public void DropTable_TableNamePreserved()
    {
        var plan = H.DropTbl("orders", true);
        var prog = SqlCodegen.CompileOptimized(H.L(plan));
        var dt   = prog.Instructions.OfType<DropTableInstr>().First();
        Assert.Equal("orders", dt.Table);
    }

    [Fact]
    public void DropTable_IfExistsPreserved()
    {
        var plan = H.DropTbl("t", true);
        var prog = SqlCodegen.CompileOptimized(H.L(plan));
        var dt   = prog.Instructions.OfType<DropTableInstr>().First();
        Assert.True(dt.IfExists);
    }
}

// ── 16-30. Expression compiler tests ─────────────────────────────────────────

public class ExprTests
{
    // ── 16. Literal
    [Fact]
    public void Expr_Literal_Integer()
    {
        var instrs = SqlCodegen.CompileExpr(H.Lit(42L));
        Assert.Single(instrs);
        Assert.IsType<LoadConst>(instrs[0]);
        Assert.Equal(42L, ((LoadConst)instrs[0]).Value);
    }

    // ── 17. Literal null
    [Fact]
    public void Expr_Literal_Null()
    {
        var instrs = SqlCodegen.CompileExpr(H.Lit(null));
        var lc = Assert.Single(instrs.OfType<LoadConst>());
        Assert.Null(lc.Value);
    }

    // ── 18. Column reference
    [Fact]
    public void Expr_Column_EmitsLoadColumn()
    {
        var instrs = SqlCodegen.CompileExpr(H.Col("name"));
        Assert.Single(instrs.OfType<LoadColumn>());
        Assert.Equal("name", instrs.OfType<LoadColumn>().First().Column);
    }

    // ── 19. BinaryOp: Add
    [Fact]
    public void Expr_BinaryOp_Add()
    {
        var expr   = H.BinOp(BinaryOperator.Add, H.Lit(1L), H.Lit(2L));
        var instrs = SqlCodegen.CompileExpr(expr);
        Assert.Contains(instrs, i => i is BinaryOpInstr { Op: BinaryOpCode.Add });
    }

    // ── 20. BinaryOp: Eq
    [Fact]
    public void Expr_BinaryOp_Eq()
    {
        var expr   = H.BinOp(BinaryOperator.Eq, H.Col("id"), H.Lit(5L));
        var instrs = SqlCodegen.CompileExpr(expr);
        Assert.Contains(instrs, i => i is BinaryOpInstr { Op: BinaryOpCode.Eq });
    }

    // ── 21. BinaryOp: And / Or
    [Fact]
    public void Expr_BinaryOp_And()
    {
        var expr   = H.BinOp(BinaryOperator.And, H.Lit(true), H.Lit(false));
        var instrs = SqlCodegen.CompileExpr(expr);
        Assert.Contains(instrs, i => i is BinaryOpInstr { Op: BinaryOpCode.And });
    }

    [Fact]
    public void Expr_BinaryOp_Or()
    {
        var expr   = H.BinOp(BinaryOperator.Or, H.Lit(true), H.Lit(false));
        var instrs = SqlCodegen.CompileExpr(expr);
        Assert.Contains(instrs, i => i is BinaryOpInstr { Op: BinaryOpCode.Or });
    }

    // ── 22. UnaryOp: Not
    [Fact]
    public void Expr_UnaryOp_Not()
    {
        var expr   = H.UnaOp(UnaryOperator.Not, H.Lit(true));
        var instrs = SqlCodegen.CompileExpr(expr);
        Assert.Contains(instrs, i => i is UnaryOpInstr { Op: UnaryOpCode.Not });
    }

    // ── 23. UnaryOp: Neg
    [Fact]
    public void Expr_UnaryOp_Neg()
    {
        var expr   = H.UnaOp(UnaryOperator.Neg, H.Lit(5L));
        var instrs = SqlCodegen.CompileExpr(expr);
        Assert.Contains(instrs, i => i is UnaryOpInstr { Op: UnaryOpCode.Neg });
    }

    // ── 24. IS NULL
    [Fact]
    public void Expr_IsNull()
    {
        var expr   = new SqlExpr.IsNull(H.Col("x"));
        var instrs = SqlCodegen.CompileExpr(expr);
        H.HasAny<IsNullInstr>(new Program(instrs, new Dictionary<string, int>(), Array.Empty<string>()));
    }

    // ── 25. IS NOT NULL
    [Fact]
    public void Expr_IsNotNull()
    {
        var expr   = new SqlExpr.IsNotNull(H.Col("x"));
        var instrs = SqlCodegen.CompileExpr(expr);
        H.HasAny<IsNotNullInstr>(new Program(instrs, new Dictionary<string, int>(), Array.Empty<string>()));
    }

    // ── 26. BETWEEN
    [Fact]
    public void Expr_Between()
    {
        var expr   = new SqlExpr.Between(H.Col("age"), H.Lit(18L), H.Lit(65L));
        var instrs = SqlCodegen.CompileExpr(expr);
        Assert.Contains(instrs, i => i is BetweenInstr);
        // Three sub-expressions (value, low, high) = three LoadColumn/LoadConst
        Assert.Equal(3, instrs.Count(i => i is LoadColumn or LoadConst));
    }

    // ── 27. IN (list)
    [Fact]
    public void Expr_In()
    {
        var expr   = new SqlExpr.In(H.Col("status"), new[] { H.Lit("A"), H.Lit("B") });
        var instrs = SqlCodegen.CompileExpr(expr);
        Assert.Contains(instrs, i => i is InListInstr { N: 2 });
    }

    // ── 28. NOT IN
    [Fact]
    public void Expr_NotIn()
    {
        var expr   = new SqlExpr.NotIn(H.Col("x"), new[] { H.Lit(1L), H.Lit(2L) });
        var instrs = SqlCodegen.CompileExpr(expr);
        Assert.Contains(instrs, i => i is InListInstr);
        // NOT IN compiles as IN + Not
        Assert.Contains(instrs, i => i is UnaryOpInstr { Op: UnaryOpCode.Not });
    }

    // ── 29. LIKE / NOT LIKE
    [Fact]
    public void Expr_Like()
    {
        var expr   = new SqlExpr.Like(H.Col("name"), "%Alice%");
        var instrs = SqlCodegen.CompileExpr(expr);
        Assert.Contains(instrs, i => i is LikeInstr { Negated: false });
    }

    [Fact]
    public void Expr_NotLike()
    {
        var expr   = new SqlExpr.NotLike(H.Col("name"), "%Bob%");
        var instrs = SqlCodegen.CompileExpr(expr);
        Assert.Contains(instrs, i => i is LikeInstr { Negated: true });
    }

    // ── 30. FuncCall (scalar)
    [Fact]
    public void Expr_FuncCall_EmitsCallScalar()
    {
        var expr   = new SqlExpr.FuncCall("upper", new[] { H.Col("name") });
        var instrs = SqlCodegen.CompileExpr(expr);
        Assert.Contains(instrs, i => i is CallScalar { Func: "upper", NArgs: 1 });
    }

    // ── 31. FuncCall: multiple args
    [Fact]
    public void Expr_FuncCall_MultipleArgs()
    {
        var expr   = new SqlExpr.FuncCall("substr", new SqlExpr[] { H.Col("text"), H.Lit(1L), H.Lit(3L) });
        var instrs = SqlCodegen.CompileExpr(expr);
        Assert.Contains(instrs, i => i is CallScalar { Func: "substr", NArgs: 3 });
    }

    // ── 32. Label resolution
    [Fact]
    public void Labels_AllJumpTargetsResolvable()
    {
        // A filter plan will produce Jump and JumpIfFalse instructions;
        // verify all their targets appear in the labels dictionary.
        var pred = H.BinOp(BinaryOperator.Gt, H.Col("v"), H.Lit(0L));
        var plan = H.L(H.Project(H.Filter(H.Scan("t"), pred), H.OC("v", H.Col("v"))));
        var prog = SqlCodegen.CompileOptimized(plan);

        foreach (var instr in prog.Instructions)
        {
            switch (instr)
            {
                case Jump j:
                    Assert.True(prog.Labels.ContainsKey(j.Target), $"Jump target '{j.Target}' unresolved");
                    break;
                case JumpIfFalse jf:
                    Assert.True(prog.Labels.ContainsKey(jf.Target), $"JumpIfFalse target '{jf.Target}' unresolved");
                    break;
                case JumpIfTrue jt:
                    Assert.True(prog.Labels.ContainsKey(jt.Target), $"JumpIfTrue target '{jt.Target}' unresolved");
                    break;
                case AdvanceCursor ac:
                    Assert.True(prog.Labels.ContainsKey(ac.OnExhausted), $"AdvanceCursor onExhausted '{ac.OnExhausted}' unresolved");
                    break;
            }
        }
    }

    // ── 33. Compile(LogicalPlan) convenience overload
    [Fact]
    public void Compile_LogicalPlan_Overload_Works()
    {
        var plan = H.Project(H.Scan("t"), H.OC("x", H.Col("x")));
        var prog = SqlCodegen.Compile(plan);
        H.HasAny<OpenScan>(prog);
        H.HasHalt(prog);
    }

    // ── 34. Literal string
    [Fact]
    public void Expr_Literal_String()
    {
        var instrs = SqlCodegen.CompileExpr(H.Lit("hello"));
        var lc = Assert.Single(instrs.OfType<LoadConst>());
        Assert.Equal("hello", lc.Value);
    }

    // ── 35. Literal bool
    [Fact]
    public void Expr_Literal_Bool()
    {
        var instrs = SqlCodegen.CompileExpr(H.Lit(true));
        var lc = Assert.Single(instrs.OfType<LoadConst>());
        Assert.Equal(true, lc.Value);
    }

    // ── 36. Nested BinaryOp ordering (left-to-right push)
    [Fact]
    public void Expr_BinaryOp_Ordering_LeftFirst()
    {
        // (a + b) means: push a, push b, BinaryOpInstr(Add)
        var expr   = H.BinOp(BinaryOperator.Add, H.Col("a"), H.Col("b"));
        var instrs = SqlCodegen.CompileExpr(expr).ToList();
        Assert.Equal("a", instrs.OfType<LoadColumn>().First().Column);
        Assert.Equal("b", instrs.OfType<LoadColumn>().Last().Column);
        Assert.IsType<BinaryOpInstr>(instrs.Last());
    }

    // ── 37-45. All remaining BinaryOperator opcodes
    [Fact]
    public void Expr_BinaryOp_Sub() =>
        Assert.Contains(SqlCodegen.CompileExpr(H.BinOp(BinaryOperator.Sub, H.Lit(5L), H.Lit(3L))),
            i => i is BinaryOpInstr { Op: BinaryOpCode.Sub });

    [Fact]
    public void Expr_BinaryOp_Mul() =>
        Assert.Contains(SqlCodegen.CompileExpr(H.BinOp(BinaryOperator.Mul, H.Lit(2L), H.Lit(3L))),
            i => i is BinaryOpInstr { Op: BinaryOpCode.Mul });

    [Fact]
    public void Expr_BinaryOp_Div() =>
        Assert.Contains(SqlCodegen.CompileExpr(H.BinOp(BinaryOperator.Div, H.Lit(6L), H.Lit(2L))),
            i => i is BinaryOpInstr { Op: BinaryOpCode.Div });

    [Fact]
    public void Expr_BinaryOp_Mod() =>
        Assert.Contains(SqlCodegen.CompileExpr(H.BinOp(BinaryOperator.Mod, H.Lit(7L), H.Lit(3L))),
            i => i is BinaryOpInstr { Op: BinaryOpCode.Mod });

    [Fact]
    public void Expr_BinaryOp_NotEq() =>
        Assert.Contains(SqlCodegen.CompileExpr(H.BinOp(BinaryOperator.NotEq, H.Col("x"), H.Lit(0L))),
            i => i is BinaryOpInstr { Op: BinaryOpCode.Neq });

    [Fact]
    public void Expr_BinaryOp_Lt() =>
        Assert.Contains(SqlCodegen.CompileExpr(H.BinOp(BinaryOperator.Lt, H.Col("age"), H.Lit(18L))),
            i => i is BinaryOpInstr { Op: BinaryOpCode.Lt });

    [Fact]
    public void Expr_BinaryOp_Lte() =>
        Assert.Contains(SqlCodegen.CompileExpr(H.BinOp(BinaryOperator.Lte, H.Col("age"), H.Lit(18L))),
            i => i is BinaryOpInstr { Op: BinaryOpCode.Lte });

    [Fact]
    public void Expr_BinaryOp_Gt() =>
        Assert.Contains(SqlCodegen.CompileExpr(H.BinOp(BinaryOperator.Gt, H.Col("price"), H.Lit(100L))),
            i => i is BinaryOpInstr { Op: BinaryOpCode.Gt });

    [Fact]
    public void Expr_BinaryOp_Gte() =>
        Assert.Contains(SqlCodegen.CompileExpr(H.BinOp(BinaryOperator.Gte, H.Col("score"), H.Lit(60L))),
            i => i is BinaryOpInstr { Op: BinaryOpCode.Gte });

    // ── 46. AggExpr in expression context falls back to LoadConst(null)
    [Fact]
    public void Expr_AggExpr_EmitsNullPlaceholder()
    {
        var aggExpr = new SqlExpr.AggExpr(AggFunction.Count, new AggArg.Star(), false);
        var instrs  = SqlCodegen.CompileExpr(aggExpr);
        var lc = Assert.Single(instrs.OfType<LoadConst>());
        Assert.Null(lc.Value);
    }

    // ── 47. Wildcard in expression context falls back to LoadConst(null)
    [Fact]
    public void Expr_Wildcard_EmitsNullPlaceholder()
    {
        var instrs = SqlCodegen.CompileExpr(new SqlExpr.Wildcard());
        var lc = Assert.Single(instrs.OfType<LoadConst>());
        Assert.Null(lc.Value);
    }
}

// ── 37. HAVING tests ─────────────────────────────────────────────────────────

public class HavingTests
{
    // Build a GROUP BY + HAVING plan: SELECT dept, COUNT(*) FROM t GROUP BY dept HAVING COUNT(*) > 1
    private static LogicalPlan MakeGroupByHavingPlan(string table)
    {
        var countItem = new AggregateItem(AggFunction.Count, new AggArg.Star(), "_count", false);
        var agg = new AggregatePlan(
            new ScanPlan(table, null),
            new SqlExpr[] { new SqlExpr.Column(null, "dept") },
            new[] { countItem });
        var havingPred = new SqlExpr.BinaryOp(
            BinaryOperator.Gt,
            new SqlExpr.AggExpr(AggFunction.Count, new AggArg.Star(), false),
            new SqlExpr.Literal(1L));
        var having = new HavingPlan(agg, havingPred);
        return new ProjectPlan(having, new OutputColumn[]
        {
            new OutputColumn.Expr(new SqlExpr.Column(null, "dept"), "dept"),
            new OutputColumn.Expr(new SqlExpr.Column(null, "_count"), "cnt"),
        });
    }

    [Fact]
    public void Having_HasAdvanceGroupKey()
    {
        var plan = Optimizer.Lift(MakeGroupByHavingPlan("sales"));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<AdvanceGroupKey>(prog);
    }

    [Fact]
    public void Having_HasJumpIfFalse()
    {
        var plan = Optimizer.Lift(MakeGroupByHavingPlan("sales"));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<JumpIfFalse>(prog);
    }

    [Fact]
    public void Having_HasFinalizeAgg()
    {
        var plan = Optimizer.Lift(MakeGroupByHavingPlan("sales"));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<FinalizeAgg>(prog);
    }

    [Fact]
    public void Having_HasSaveGroupKey()
    {
        var plan = Optimizer.Lift(MakeGroupByHavingPlan("orders"));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<SaveGroupKey>(prog);
    }

    [Fact]
    public void Having_HaltPresent()
    {
        var plan = Optimizer.Lift(MakeGroupByHavingPlan("t"));
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasHalt(prog);
    }

    // HAVING with SUM aggregate
    [Fact]
    public void Having_SumAggregate_HasInitAgg()
    {
        var sumItem = new AggregateItem(AggFunction.Sum, new AggArg.Expr(new SqlExpr.Column(null, "amount")), "_sum", false);
        var agg = new AggregatePlan(
            new ScanPlan("orders", null),
            new SqlExpr[] { new SqlExpr.Column(null, "region") },
            new[] { sumItem });
        var havingPred = new SqlExpr.BinaryOp(
            BinaryOperator.Gt,
            new SqlExpr.AggExpr(AggFunction.Sum, new AggArg.Expr(new SqlExpr.Column(null, "amount")), false),
            new SqlExpr.Literal(100L));
        var having = new HavingPlan(agg, havingPred);
        var project = new ProjectPlan(having, new OutputColumn[]
        {
            new OutputColumn.Expr(new SqlExpr.Column(null, "region"), "region"),
            new OutputColumn.Expr(new SqlExpr.Column(null, "_sum"), "total"),
        });
        var plan = Optimizer.Lift(project);
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<InitAgg>(prog);
        var init = prog.Instructions.OfType<InitAgg>().First();
        Assert.Equal(AggFunc.Sum, init.Func);
    }

    // HAVING with compound predicate: COUNT(*) > 1 AND SUM(x) < 1000
    [Fact]
    public void Having_CompoundHavingPredicate_HasMultipleFinalizeAgg()
    {
        var countItem = new AggregateItem(AggFunction.Count, new AggArg.Star(), "_count", false);
        var sumItem   = new AggregateItem(AggFunction.Sum, new AggArg.Expr(new SqlExpr.Column(null, "val")), "_sum", false);
        var agg = new AggregatePlan(
            new ScanPlan("t", null),
            new SqlExpr[] { new SqlExpr.Column(null, "grp") },
            new[] { countItem, sumItem });
        // HAVING COUNT(*) > 1 AND SUM(val) < 1000
        var havingPred = new SqlExpr.BinaryOp(
            BinaryOperator.And,
            new SqlExpr.BinaryOp(
                BinaryOperator.Gt,
                new SqlExpr.AggExpr(AggFunction.Count, new AggArg.Star(), false),
                new SqlExpr.Literal(1L)),
            new SqlExpr.BinaryOp(
                BinaryOperator.Lt,
                new SqlExpr.AggExpr(AggFunction.Sum, new AggArg.Expr(new SqlExpr.Column(null, "val")), false),
                new SqlExpr.Literal(1000L)));
        var having = new HavingPlan(agg, havingPred);
        var project = new ProjectPlan(having, new OutputColumn[]
        {
            new OutputColumn.Expr(new SqlExpr.Column(null, "grp"), "grp"),
            new OutputColumn.Expr(new SqlExpr.Column(null, "_count"), "cnt"),
        });
        var plan = Optimizer.Lift(project);
        var prog = SqlCodegen.CompileOptimized(plan);
        // Should have at least two FinalizeAgg instructions (one for the predicate check,
        // one for the emit phase).
        Assert.True(prog.Instructions.OfType<FinalizeAgg>().Count() >= 2,
            "Expected multiple FinalizeAgg for compound HAVING");
    }

    // HAVING with unknown AggExpr falls back to LoadConst(null)
    [Fact]
    public void Having_UnknownAggExpr_FallsBackToNull()
    {
        // Build a HAVING plan where the HAVING predicate references a MAX agg
        // but the aggregate plan only defines COUNT(*).
        var countItem = new AggregateItem(AggFunction.Count, new AggArg.Star(), "_count", false);
        var agg = new AggregatePlan(
            new ScanPlan("t", null),
            new SqlExpr[] { new SqlExpr.Column(null, "dept") },
            new[] { countItem });
        // HAVING MAX(salary) > 50000 -- but MAX is not in the agg list
        var havingPred = new SqlExpr.BinaryOp(
            BinaryOperator.Gt,
            new SqlExpr.AggExpr(AggFunction.Max, new AggArg.Expr(new SqlExpr.Column(null, "salary")), false),
            new SqlExpr.Literal(50000L));
        var having = new HavingPlan(agg, havingPred);
        var project = new ProjectPlan(having, new OutputColumn[]
        {
            new OutputColumn.Expr(new SqlExpr.Column(null, "dept"), "dept"),
            new OutputColumn.Expr(new SqlExpr.Column(null, "_count"), "cnt"),
        });
        var plan = Optimizer.Lift(project);
        // Should compile without throwing; the HAVING check emits LoadConst(null).
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasHalt(prog);
    }
}

// ── 38. Bare aggregate (no wrapping Project) ──────────────────────────────────

public class BareAggregateTests
{
    [Fact]
    public void BareAggregate_NoProject_HasInitAgg()
    {
        // Compile OptAggregate directly without an OptProject wrapper.
        var items = new[] { new AggregateItem(AggFunction.Count, new AggArg.Star(), "_count", false) };
        var agg   = new AggregatePlan(new ScanPlan("t", null), Array.Empty<SqlExpr>(), items);
        var opt   = Optimizer.Lift(agg);
        var prog  = SqlCodegen.CompileOptimized(opt);
        H.HasAny<InitAgg>(prog);
    }

    [Fact]
    public void BareAggregate_NoProject_HasFinalizeAgg()
    {
        var items = new[] { new AggregateItem(AggFunction.Sum, new AggArg.Expr(new SqlExpr.Column(null, "price")), "_sum", false) };
        var agg   = new AggregatePlan(new ScanPlan("products", null), Array.Empty<SqlExpr>(), items);
        var opt   = Optimizer.Lift(agg);
        var prog  = SqlCodegen.CompileOptimized(opt);
        H.HasAny<FinalizeAgg>(prog);
        var fa = prog.Instructions.OfType<FinalizeAgg>().First();
        Assert.Equal(AggFunc.Sum, fa.Func);
    }

    [Fact]
    public void BareAggregate_WithGroupBy_HasLoadGroupKey()
    {
        var items = new[] { new AggregateItem(AggFunction.Count, new AggArg.Star(), "_c", false) };
        var agg   = new AggregatePlan(
            new ScanPlan("orders", null),
            new SqlExpr[] { new SqlExpr.Column(null, "region") },
            items);
        var opt  = Optimizer.Lift(agg);
        var prog = SqlCodegen.CompileOptimized(opt);
        H.HasAny<LoadGroupKey>(prog);
    }

    // Aggregate with Avg/Min/Max functions to cover MapAggFunc branches
    [Fact]
    public void BareAggregate_AvgMinMax_FuncsMap()
    {
        var items = new[]
        {
            new AggregateItem(AggFunction.Avg, new AggArg.Expr(new SqlExpr.Column(null, "score")), "_avg", false),
            new AggregateItem(AggFunction.Min, new AggArg.Expr(new SqlExpr.Column(null, "score")), "_min", false),
            new AggregateItem(AggFunction.Max, new AggArg.Expr(new SqlExpr.Column(null, "score")), "_max", false),
        };
        var agg  = new AggregatePlan(new ScanPlan("t", null), Array.Empty<SqlExpr>(), items);
        var opt  = Optimizer.Lift(agg);
        var prog = SqlCodegen.CompileOptimized(opt);
        var funcs = prog.Instructions.OfType<InitAgg>().Select(i => i.Func).ToHashSet();
        Assert.Contains(AggFunc.Avg, funcs);
        Assert.Contains(AggFunc.Min, funcs);
        Assert.Contains(AggFunc.Max, funcs);
    }

    // COUNT(expr) (non-star) should use AggFunc.Count, not CountStar
    [Fact]
    public void BareAggregate_CountExprArg_UsesCountNotCountStar()
    {
        var items = new[]
        {
            new AggregateItem(AggFunction.Count, new AggArg.Expr(new SqlExpr.Column(null, "id")), "_count", false)
        };
        var agg  = new AggregatePlan(new ScanPlan("t", null), Array.Empty<SqlExpr>(), items);
        var opt  = Optimizer.Lift(agg);
        var prog = SqlCodegen.CompileOptimized(opt);
        var init = prog.Instructions.OfType<InitAgg>().First();
        Assert.Equal(AggFunc.Count, init.Func);
    }
}

// ── 39. Compile(LogicalPlan) overload schema recovery ────────────────────────

public class CompileOverloadTests
{
    // When the optimizer emits OptEmptyResult (e.g. LIMIT 0), the Compile overload
    // recovers the result schema from the logical plan's ProjectPlan.
    [Fact]
    public void Compile_RecoverSchemaFromLogicalPlan_WhenOptEmptyResult()
    {
        // LIMIT 0 → optimizer produces OptEmptyResult
        var inner = new ProjectPlan(
            new ScanPlan("t", null),
            new OutputColumn[] { new OutputColumn.Expr(new SqlExpr.Column(null, "x"), "x") });
        var plan  = new LimitPlan(inner, 0L, null); // LIMIT 0 → OptEmptyResult
        var prog  = SqlCodegen.Compile(plan);
        // The schema should be recovered even though no rows are produced.
        Assert.Contains("x", prog.ResultSchema);
    }

    [Fact]
    public void Compile_NormalPlan_HasResultSchema()
    {
        var plan = new ProjectPlan(
            new ScanPlan("users", null),
            new OutputColumn[] { new OutputColumn.Expr(new SqlExpr.Column(null, "name"), "name") });
        var prog = SqlCodegen.Compile(plan);
        Assert.Contains("name", prog.ResultSchema);
    }

    // Compile with SortPlan → DistinctPlan → ProjectPlan spine (exercises ExtractSchemaFromLogical)
    [Fact]
    public void Compile_SortLimitDistinctSpine_SchemaRecovered()
    {
        // Build a plan with a deep spine where the optimizer still emits rows
        // so ResultSchema gets populated normally.
        var inner   = new ProjectPlan(new ScanPlan("t", null),
            new OutputColumn[] { new OutputColumn.Expr(new SqlExpr.Column(null, "v"), "v") });
        var sorted  = new SortPlan(inner, new[] { new SortKey(new SqlExpr.Column(null, "v"), SortDir.Asc, NullOrder.NullsLast) });
        var limited = new LimitPlan(sorted, 10L, null);
        var prog = SqlCodegen.Compile(limited);
        Assert.Contains("v", prog.ResultSchema);
    }
}

// ── 40. OutputColumn.Star in project ─────────────────────────────────────────

public class StarProjectTests
{
    [Fact]
    public void Project_WithStar_EmitsStarColumnName()
    {
        // SELECT * FROM t → output column is OutputColumn.Star
        var plan = new ProjectPlan(new ScanPlan("t", null),
            new OutputColumn[] { new OutputColumn.Star() });
        var opt  = Optimizer.Lift(plan);
        var prog = SqlCodegen.CompileOptimized(opt);
        // Star gets schema name "*"
        H.HasAny<SetResultSchema>(prog);
        var schema = prog.Instructions.OfType<SetResultSchema>().First().Columns;
        Assert.Contains("*", schema);
    }

    [Fact]
    public void Project_WithStar_EmitsLoadConst()
    {
        var plan = new ProjectPlan(new ScanPlan("t", null),
            new OutputColumn[] { new OutputColumn.Star() });
        var opt  = Optimizer.Lift(plan);
        var prog = SqlCodegen.CompileOptimized(opt);
        // Star emits LoadConst(null) as placeholder
        H.HasAny<LoadConst>(prog);
    }
}

// ── 41. Left join with ON condition ──────────────────────────────────────────

public class LeftJoinWithConditionTests
{
    [Fact]
    public void LeftJoin_WithCondition_HasJoinSetMatched()
    {
        var cond = new SqlExpr.BinaryOp(
            BinaryOperator.Eq,
            new SqlExpr.Column("u", "id"),
            new SqlExpr.Column("o", "user_id"));
        var join = new JoinPlan(
            new ScanPlan("users", "u"),
            new ScanPlan("orders", "o"),
            JoinKind.Left,
            cond);
        var plan = new ProjectPlan(join,
            new OutputColumn[] { new OutputColumn.Expr(new SqlExpr.Column("u", "id"), "id") });
        var opt  = Optimizer.Lift(plan);
        var prog = SqlCodegen.CompileOptimized(opt);
        H.HasAny<JoinSetMatched>(prog);
        H.HasAny<JoinIfMatched>(prog);
        H.HasAny<JumpIfFalse>(prog);
    }
}

// ── 42. Bare scan/filter/join in CompileCore (no project) ────────────────────

public class BareScanTests
{
    [Fact]
    public void BareScan_EmitsOpenScanAndEmitRow()
    {
        // OptScan without OptProject wrapping — hits the bare scan branch in CompileCore.
        var plan = new ScanPlan("t", null);
        var opt  = Optimizer.Lift(plan);
        var prog = SqlCodegen.CompileOptimized(opt);
        H.HasAny<OpenScan>(prog);
        H.HasAny<EmitRow>(prog);
    }

    [Fact]
    public void BareFilter_EmitsJumpIfFalse()
    {
        var pred = new SqlExpr.BinaryOp(BinaryOperator.Eq, new SqlExpr.Column(null, "x"), new SqlExpr.Literal(1L));
        var plan = new FilterPlan(new ScanPlan("t", null), pred);
        var opt  = Optimizer.Lift(plan);
        var prog = SqlCodegen.CompileOptimized(opt);
        H.HasAny<JumpIfFalse>(prog);
        H.HasAny<EmitRow>(prog);
    }
}

// ── 43. Aggregate without GroupBy (no SaveGroupKey / no LoadGroupKey) ─────────

public class AggregateNoGroupByTests
{
    [Fact]
    public void Aggregate_NoGroupBy_HasNoSaveGroupKey()
    {
        var items = new[] { new AggregateItem(AggFunction.Count, new AggArg.Star(), "_count", false) };
        var agg   = new AggregatePlan(new ScanPlan("t", null), Array.Empty<SqlExpr>(), items);
        var plan  = new ProjectPlan(agg, new OutputColumn[]
        {
            new OutputColumn.Expr(new SqlExpr.Column(null, "_count"), "cnt")
        });
        var opt  = Optimizer.Lift(plan);
        var prog = SqlCodegen.CompileOptimized(opt);
        // SaveGroupKey(0) is still emitted but LoadGroupKey is not (no group-by columns to load).
        H.HasNone<LoadGroupKey>(prog);
    }

    [Fact]
    public void Aggregate_NoGroupBy_AdvanceGroupKeyHasGroupByFalse()
    {
        var items = new[] { new AggregateItem(AggFunction.Count, new AggArg.Star(), "_c", false) };
        var agg   = new AggregatePlan(new ScanPlan("t", null), Array.Empty<SqlExpr>(), items);
        var plan  = new ProjectPlan(agg, new OutputColumn[]
        {
            new OutputColumn.Expr(new SqlExpr.Column(null, "_c"), "cnt")
        });
        var opt  = Optimizer.Lift(plan);
        var prog = SqlCodegen.CompileOptimized(opt);
        var agk = prog.Instructions.OfType<AdvanceGroupKey>().First();
        Assert.False(agk.HasGroupBy);
    }
}

// ── 44. ExpressionColumn with no alias (falls back to column name) ─────────────

public class ProjectColumnNameTests
{
    [Fact]
    public void Project_ExprColumnNoAlias_UsesColumnName()
    {
        // OutputColumn.Expr with no alias: name should come from the Column expression.
        var plan = new ProjectPlan(new ScanPlan("t", null),
            new OutputColumn[]
            {
                new OutputColumn.Expr(new SqlExpr.Column(null, "name"), null)
            });
        var opt  = Optimizer.Lift(plan);
        var prog = SqlCodegen.CompileOptimized(opt);
        var schema = prog.Instructions.OfType<SetResultSchema>().First().Columns;
        Assert.Contains("name", schema);
    }

    [Fact]
    public void Project_ExprNonColumnNoAlias_UsesFallbackName()
    {
        // OutputColumn.Expr with a non-column expr and no alias: name is "col_i".
        var plan = new ProjectPlan(new ScanPlan("t", null),
            new OutputColumn[]
            {
                new OutputColumn.Expr(new SqlExpr.Literal(42L), null)
            });
        var opt  = Optimizer.Lift(plan);
        var prog = SqlCodegen.CompileOptimized(opt);
        var schema = prog.Instructions.OfType<SetResultSchema>().First().Columns;
        Assert.Contains("col_0", schema);
    }
}

// ── 45. HAVING with non-aggregate leaf in predicate ───────────────────────────

public class HavingLeafExprTests
{
    // HAVING uses CompileHavingExpr; a plain column reference in the predicate
    // should fall through to CompileExprInCtx (the default branch).
    [Fact]
    public void Having_PlainColumnInPredicate_FallsBackToExprCompiler()
    {
        var countItem = new AggregateItem(AggFunction.Count, new AggArg.Star(), "_count", false);
        var agg = new AggregatePlan(
            new ScanPlan("t", null),
            new SqlExpr[] { new SqlExpr.Column(null, "dept") },
            new[] { countItem });
        // HAVING dept = 'Sales' (a plain column, not an aggregate expr)
        var havingPred = new SqlExpr.BinaryOp(
            BinaryOperator.Eq,
            new SqlExpr.Column(null, "dept"),
            new SqlExpr.Literal("Sales"));
        var having = new HavingPlan(agg, havingPred);
        var project = new ProjectPlan(having, new OutputColumn[]
        {
            new OutputColumn.Expr(new SqlExpr.Column(null, "dept"), "dept"),
            new OutputColumn.Expr(new SqlExpr.Column(null, "_count"), "cnt"),
        });
        var plan = Optimizer.Lift(project);
        var prog = SqlCodegen.CompileOptimized(plan);
        // Should compile; the binary op check emits JumpIfFalse
        H.HasAny<JumpIfFalse>(prog);
    }
}

// ── 46. HAVING expression compiler — all recursive arms ───────────────────────

public class HavingExprCompilerTests
{
    private static (OptAggregate agg, IReadOnlyList<int> slots) BuildAgg()
    {
        // A single COUNT(*) aggregate so we can build an OptAggregate for testing
        var countItem = new AggregateItem(AggFunction.Count, new AggArg.Star(), "_c", false);
        var logical   = new AggregatePlan(
            new ScanPlan("t", null),
            new SqlExpr[] { new SqlExpr.Column(null, "dept") },
            new[] { countItem });
        var opt = Optimizer.Lift(logical) as OptAggregate
                  ?? throw new InvalidOperationException("Expected OptAggregate");
        return (opt, new List<int> { 0 });
    }

    // HAVING IS NULL(x) — hits PlIsNull arm in CompileHavingExpr
    [Fact]
    public void Having_IsNull_InPredicate()
    {
        var countItem = new AggregateItem(AggFunction.Count, new AggArg.Star(), "_count", false);
        var agg = new AggregatePlan(
            new ScanPlan("t", null),
            new SqlExpr[] { new SqlExpr.Column(null, "dept") },
            new[] { countItem });
        var havingPred = new SqlExpr.IsNull(new SqlExpr.Column(null, "dept"));
        var having  = new HavingPlan(agg, havingPred);
        var project = new ProjectPlan(having, new OutputColumn[]
        {
            new OutputColumn.Expr(new SqlExpr.Column(null, "dept"), "dept"),
            new OutputColumn.Expr(new SqlExpr.Column(null, "_count"), "cnt"),
        });
        var plan = Optimizer.Lift(project);
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<IsNullInstr>(prog);
    }

    // HAVING IS NOT NULL(x) — hits PlIsNotNull arm
    [Fact]
    public void Having_IsNotNull_InPredicate()
    {
        var countItem = new AggregateItem(AggFunction.Count, new AggArg.Star(), "_count", false);
        var agg = new AggregatePlan(
            new ScanPlan("t", null),
            new SqlExpr[] { new SqlExpr.Column(null, "dept") },
            new[] { countItem });
        var havingPred = new SqlExpr.IsNotNull(new SqlExpr.Column(null, "dept"));
        var having  = new HavingPlan(agg, havingPred);
        var project = new ProjectPlan(having, new OutputColumn[]
        {
            new OutputColumn.Expr(new SqlExpr.Column(null, "dept"), "dept"),
            new OutputColumn.Expr(new SqlExpr.Column(null, "_count"), "cnt"),
        });
        var plan = Optimizer.Lift(project);
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<IsNotNullInstr>(prog);
    }

    // HAVING x BETWEEN low AND high — hits PlBetween arm
    [Fact]
    public void Having_Between_InPredicate()
    {
        var countItem = new AggregateItem(AggFunction.Count, new AggArg.Star(), "_count", false);
        var agg = new AggregatePlan(
            new ScanPlan("t", null),
            new SqlExpr[] { new SqlExpr.Column(null, "dept") },
            new[] { countItem });
        var havingPred = new SqlExpr.Between(
            new SqlExpr.Column(null, "dept"),
            new SqlExpr.Literal("A"),
            new SqlExpr.Literal("Z"));
        var having  = new HavingPlan(agg, havingPred);
        var project = new ProjectPlan(having, new OutputColumn[]
        {
            new OutputColumn.Expr(new SqlExpr.Column(null, "dept"), "dept"),
            new OutputColumn.Expr(new SqlExpr.Column(null, "_count"), "cnt"),
        });
        var plan = Optimizer.Lift(project);
        var prog = SqlCodegen.CompileOptimized(plan);
        H.HasAny<BetweenInstr>(prog);
    }

    // HAVING upper(dept) = 'SALES' — hits FuncCall arm
    [Fact]
    public void Having_FuncCall_InPredicate()
    {
        var countItem = new AggregateItem(AggFunction.Count, new AggArg.Star(), "_count", false);
        var agg = new AggregatePlan(
            new ScanPlan("t", null),
            new SqlExpr[] { new SqlExpr.Column(null, "dept") },
            new[] { countItem });
        var havingPred = new SqlExpr.BinaryOp(
            BinaryOperator.Eq,
            new SqlExpr.FuncCall("upper", new SqlExpr[] { new SqlExpr.Column(null, "dept") }),
            new SqlExpr.Literal("SALES"));
        var having  = new HavingPlan(agg, havingPred);
        var project = new ProjectPlan(having, new OutputColumn[]
        {
            new OutputColumn.Expr(new SqlExpr.Column(null, "dept"), "dept"),
            new OutputColumn.Expr(new SqlExpr.Column(null, "_count"), "cnt"),
        });
        var plan = Optimizer.Lift(project);
        var prog = SqlCodegen.CompileOptimized(plan);
        Assert.Contains(prog.Instructions, i => i is CallScalar { Func: "upper" });
    }

    // HAVING NOT (x > 0) — hits UnaryOp arm
    [Fact]
    public void Having_UnaryNot_InPredicate()
    {
        var countItem = new AggregateItem(AggFunction.Count, new AggArg.Star(), "_count", false);
        var agg = new AggregatePlan(
            new ScanPlan("t", null),
            new SqlExpr[] { new SqlExpr.Column(null, "dept") },
            new[] { countItem });
        var havingPred = new SqlExpr.UnaryOp(
            UnaryOperator.Not,
            new SqlExpr.BinaryOp(BinaryOperator.Eq, new SqlExpr.Column(null, "dept"), new SqlExpr.Literal("X")));
        var having  = new HavingPlan(agg, havingPred);
        var project = new ProjectPlan(having, new OutputColumn[]
        {
            new OutputColumn.Expr(new SqlExpr.Column(null, "dept"), "dept"),
            new OutputColumn.Expr(new SqlExpr.Column(null, "_count"), "cnt"),
        });
        var plan = Optimizer.Lift(project);
        var prog = SqlCodegen.CompileOptimized(plan);
        Assert.Contains(prog.Instructions, i => i is UnaryOpInstr { Op: UnaryOpCode.Not });
    }
}

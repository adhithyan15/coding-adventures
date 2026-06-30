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
}

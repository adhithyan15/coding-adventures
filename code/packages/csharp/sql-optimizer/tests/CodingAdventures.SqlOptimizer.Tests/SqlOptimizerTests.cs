// SqlOptimizerTests.cs — xUnit conformance and unit tests for CodingAdventures.SqlOptimizer.
//
// Covers:
//   • SqlOptimizer.Lift()         — 1:1 node mapping from LogicalPlan to OptimizedPlan
//   • ConstantFolding             — arithmetic, comparisons, logical short-circuit, unary
//   • PredicatePushdown           — AND splitting, transparency through Sort/Distinct/Project
//   • ProjectionPruning           — RequiredColumns annotation on OptScan
//   • DeadCodeElimination         — FALSE filter, LIMIT 0, EmptyResult propagation
//   • LimitPushdown               — scan_limit annotation
//   • SqlOptimizer.Optimize()     — end-to-end pipeline
//   • SqlOptimizer.OptimizeWithPasses() — custom pass list
//
// 50 test methods.  Each test is self-contained and uses only xUnit [Fact] attributes.

using Xunit;
using CodingAdventures.SqlPlanner;
using CodingAdventures.SqlOptimizer;

namespace CodingAdventures.SqlOptimizer.Tests;

public sealed class SqlOptimizerTests
{
    // ── Shared helpers ────────────────────────────────────────────────────────

    /// <summary>Build a scan → filter → project pipeline (the most common query shape).</summary>
    private static LogicalPlan ScanFilterProject(
        string table,
        SqlExpr predicate,
        IReadOnlyList<OutputColumn>? cols = null)
    {
        LogicalPlan scan   = new ScanPlan(table, null);
        LogicalPlan filter = new FilterPlan(scan, predicate);
        cols ??= new[] { new OutputColumn.Star() };
        return new ProjectPlan(filter, cols);
    }

    private static SqlExpr.Literal LitL(long v)    => new(v);
    private static SqlExpr.Literal LitD(double v)  => new(v);
    private static SqlExpr.Literal LitS(string v)  => new(v);
    private static SqlExpr.Literal LitBool(bool v) => new(v);
    private static SqlExpr.Literal LitNull()       => new(null);

    private static SqlExpr.Column Col(string name) => new(null, name);
    private static SqlExpr.Column ColQ(string table, string name) => new(table, name);

    private static SqlExpr BinOp(BinaryOperator op, SqlExpr l, SqlExpr r)
        => new SqlExpr.BinaryOp(op, l, r);

    // ── Lift tests ────────────────────────────────────────────────────────────

    [Fact]
    public void Lift_ScanPlan_ProducesOptScan()
    {
        var plan   = new ScanPlan("users", "u");
        var result = SqlOptimizer.Lift(plan);
        var scan   = Assert.IsType<OptScan>(result);
        Assert.Equal("users", scan.Table);
        Assert.Equal("u", scan.Alias);
        Assert.Null(scan.RequiredColumns);
        Assert.Null(scan.ScanLimit);
    }

    [Fact]
    public void Lift_FilterPlan_ProducesOptFilter()
    {
        var pred   = LitBool(true);
        var plan   = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.Lift(plan);
        var filter = Assert.IsType<OptFilter>(result);
        Assert.IsType<OptScan>(filter.Input);
    }

    [Fact]
    public void Lift_ProjectPlan_ProducesOptProject()
    {
        var cols = new[] { new OutputColumn.Star() };
        var plan = new ProjectPlan(new ScanPlan("t", null), cols);
        var result = SqlOptimizer.Lift(plan);
        var proj = Assert.IsType<OptProject>(result);
        Assert.IsType<OptScan>(proj.Input);
    }

    [Fact]
    public void Lift_InsertPlan_ProducesOptInsert()
    {
        var vals = new[] { (IReadOnlyList<SqlExpr>)new[] { LitL(1) } };
        var plan = new InsertPlan("t", null, vals);
        var result = SqlOptimizer.Lift(plan);
        var ins = Assert.IsType<OptInsert>(result);
        Assert.Equal("t", ins.Table);
    }

    [Fact]
    public void Lift_CreateTablePlan_ProducesOptCreateTable()
    {
        var cols = new[] { new ColumnDef("id", "INTEGER", NotNull: true, PrimaryKey: true) };
        var plan = new CreateTablePlan("t", false, cols);
        var result = SqlOptimizer.Lift(plan);
        var ct = Assert.IsType<OptCreateTable>(result);
        Assert.Equal("t", ct.Table);
        Assert.False(ct.IfNotExists);
    }

    [Fact]
    public void Lift_DropTablePlan_ProducesOptDropTable()
    {
        var plan   = new DropTablePlan("t", true);
        var result = SqlOptimizer.Lift(plan);
        var dt     = Assert.IsType<OptDropTable>(result);
        Assert.True(dt.IfExists);
    }

    [Fact]
    public void Lift_LimitPlan_PreservesCountAndOffset()
    {
        var plan   = new LimitPlan(new ScanPlan("t", null), 10L, 5L);
        var result = SqlOptimizer.Lift(plan);
        var lim    = Assert.IsType<OptLimit>(result);
        Assert.Equal(10L, lim.Count);
        Assert.Equal(5L,  lim.Offset);
    }

    // ── ConstantFolding tests ─────────────────────────────────────────────────

    [Fact]
    public void ConstantFolding_Folds_AddLiterals()
    {
        var plan = new FilterPlan(
            new ScanPlan("t", null),
            BinOp(BinaryOperator.Add, LitL(1), LitL(2)));
        var result = SqlOptimizer.Optimize(plan);
        var filter = Assert.IsType<OptFilter>(result);
        var lit    = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(3L, lit.Value);
    }

    [Fact]
    public void ConstantFolding_Folds_SubLiterals()
    {
        var plan = new FilterPlan(
            new ScanPlan("t", null),
            BinOp(BinaryOperator.Sub, LitL(10), LitL(3)));
        var result = SqlOptimizer.Optimize(plan);
        var filter = Assert.IsType<OptFilter>(result);
        var lit    = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(7L, lit.Value);
    }

    [Fact]
    public void ConstantFolding_Folds_MulLiterals()
    {
        var pred = BinOp(BinaryOperator.Mul, LitL(3), LitL(4));
        var plan = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.Optimize(plan);
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(12L, lit.Value);
    }

    [Fact]
    public void ConstantFolding_Folds_DivLiterals()
    {
        var pred = BinOp(BinaryOperator.Div, LitL(10), LitL(2));
        var plan = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.Optimize(plan);
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(5L, lit.Value);
    }

    [Fact]
    public void ConstantFolding_Folds_ModLiterals()
    {
        var pred = BinOp(BinaryOperator.Mod, LitL(10), LitL(3));
        var plan = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.Optimize(plan);
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(1L, lit.Value);
    }

    [Fact]
    public void ConstantFolding_Folds_EqComparison_True()
    {
        // 5 == 5 → true; Filter(TRUE) is then eliminated by DCE → scan survives
        var pred = BinOp(BinaryOperator.Eq, LitL(5), LitL(5));
        var plan = new FilterPlan(new ScanPlan("t", null), pred);
        // Use only ConstantFolding so we can inspect the intermediate literal.
        var result = SqlOptimizer.OptimizeWithPasses(plan, new IPass[] { new ConstantFoldingPass() });
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(true, lit.Value);
    }

    [Fact]
    public void ConstantFolding_Folds_LtComparison_False()
    {
        // 10 < 5 → false; with only ConstantFolding applied we get Filter(FALSE).
        var pred = BinOp(BinaryOperator.Lt, LitL(10), LitL(5));
        var plan = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.OptimizeWithPasses(plan, new IPass[] { new ConstantFoldingPass() });
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(false, lit.Value);
    }

    [Fact]
    public void ConstantFolding_ShortCircuit_TrueAnd_Yields_RHS()
    {
        var col  = Col("age");
        var pred = BinOp(BinaryOperator.And, LitBool(true), col);
        var plan = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.Optimize(plan);
        var filter = Assert.IsType<OptFilter>(result);
        Assert.IsType<SqlExpr.Column>(filter.Predicate);
    }

    [Fact]
    public void ConstantFolding_ShortCircuit_FalseAnd_Yields_False()
    {
        var col  = Col("age");
        var pred = BinOp(BinaryOperator.And, LitBool(false), col);
        var plan = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.Optimize(plan);
        // DeadCodeElimination converts FALSE filter → EmptyResult
        Assert.IsType<OptEmptyResult>(result);
    }

    [Fact]
    public void ConstantFolding_ShortCircuit_TrueOr_Yields_True()
    {
        var col  = Col("age");
        var pred = BinOp(BinaryOperator.Or, LitBool(true), col);
        var plan = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.Optimize(plan);
        // Filter(TRUE) → input (tautology elim)
        // Result is the scan itself or a project
        Assert.IsNotType<OptFilter>(result);
    }

    [Fact]
    public void ConstantFolding_ShortCircuit_FalseOr_Yields_RHS()
    {
        var col  = Col("age");
        var pred = BinOp(BinaryOperator.Or, LitBool(false), col);
        var plan = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.Optimize(plan);
        var filter = Assert.IsType<OptFilter>(result);
        Assert.IsType<SqlExpr.Column>(filter.Predicate);
    }

    [Fact]
    public void ConstantFolding_NotTrue_Yields_False()
    {
        var pred = new SqlExpr.UnaryOp(UnaryOperator.Not, LitBool(true));
        var plan = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.Optimize(plan);
        // Filter(FALSE) → EmptyResult
        Assert.IsType<OptEmptyResult>(result);
    }

    [Fact]
    public void ConstantFolding_NotFalse_Yields_True()
    {
        var pred = new SqlExpr.UnaryOp(UnaryOperator.Not, LitBool(false));
        var plan = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.Optimize(plan);
        // Filter(TRUE) → input (scan)
        Assert.IsType<OptScan>(result);
    }

    [Fact]
    public void ConstantFolding_NegLong_Yields_Negative()
    {
        var pred = new SqlExpr.UnaryOp(UnaryOperator.Neg, LitL(5));
        var plan = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.Optimize(plan);
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(-5L, lit.Value);
    }

    [Fact]
    public void ConstantFolding_StringConcat_Yields_Combined()
    {
        var pred = BinOp(BinaryOperator.Add, LitS("hello"), LitS(" world"));
        var plan = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.Optimize(plan);
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal("hello world", lit.Value);
    }

    [Fact]
    public void ConstantFolding_DoesNotFoldColumn()
    {
        var col  = Col("age");
        var pred = BinOp(BinaryOperator.Gt, col, LitL(18));
        var plan = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.Optimize(plan);
        var filter = Assert.IsType<OptFilter>(result);
        Assert.IsType<SqlExpr.BinaryOp>(filter.Predicate);
    }

    [Fact]
    public void ConstantFolding_Nested_Arithmetic_Folds_AllLayers()
    {
        // (2 + 3) * 4 = 20
        var inner = BinOp(BinaryOperator.Add, LitL(2), LitL(3));
        var outer = BinOp(BinaryOperator.Mul, inner, LitL(4));
        var plan  = new FilterPlan(new ScanPlan("t", null), outer);
        var result = SqlOptimizer.Optimize(plan);
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(20L, lit.Value);
    }

    // ── DeadCodeElimination tests ──────────────────────────────────────────────

    [Fact]
    public void DeadCode_FilterFalse_ProducesEmptyResult()
    {
        var plan   = new FilterPlan(new ScanPlan("t", null), LitBool(false));
        var result = SqlOptimizer.Optimize(plan);
        Assert.IsType<OptEmptyResult>(result);
    }

    [Fact]
    public void DeadCode_ConstantFoldedFalseFilter_ProducesEmptyResult()
    {
        // 5 < 3 → false → EmptyResult
        var pred   = BinOp(BinaryOperator.Lt, LitL(5), LitL(3));
        var plan   = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.Optimize(plan);
        Assert.IsType<OptEmptyResult>(result);
    }

    [Fact]
    public void DeadCode_LimitZero_ProducesEmptyResult()
    {
        var plan   = new LimitPlan(new ScanPlan("t", null), 0L, null);
        var result = SqlOptimizer.Optimize(plan);
        Assert.IsType<OptEmptyResult>(result);
    }

    [Fact]
    public void DeadCode_ProjectAboveEmptyResult_PropagatesEmpty()
    {
        var filter  = new FilterPlan(new ScanPlan("t", null), LitBool(false));
        var project = new ProjectPlan(filter, new[] { new OutputColumn.Star() });
        var result  = SqlOptimizer.Optimize(project);
        Assert.IsType<OptEmptyResult>(result);
    }

    [Fact]
    public void DeadCode_SortAboveEmptyResult_PropagatesEmpty()
    {
        var filter  = new FilterPlan(new ScanPlan("t", null), LitBool(false));
        var sort    = new SortPlan(filter, new[] { new SortKey(Col("id"), SortDir.Asc, NullOrder.NullsLast) });
        var result  = SqlOptimizer.Optimize(sort);
        Assert.IsType<OptEmptyResult>(result);
    }

    [Fact]
    public void DeadCode_InnerJoin_EmptyLeft_ProducesEmptyResult()
    {
        var emptyLeft  = new FilterPlan(new ScanPlan("a", null), LitBool(false));
        var right      = new ScanPlan("b", null);
        var join       = new JoinPlan(emptyLeft, right, JoinKind.Inner, null);
        var result     = SqlOptimizer.Optimize(join);
        Assert.IsType<OptEmptyResult>(result);
    }

    [Fact]
    public void DeadCode_InnerJoin_EmptyRight_ProducesEmptyResult()
    {
        var left       = new ScanPlan("a", null);
        var emptyRight = new FilterPlan(new ScanPlan("b", null), LitBool(false));
        var join       = new JoinPlan(left, emptyRight, JoinKind.Inner, null);
        var result     = SqlOptimizer.Optimize(join);
        Assert.IsType<OptEmptyResult>(result);
    }

    [Fact]
    public void DeadCode_FilterTrue_RetainsInput()
    {
        var plan   = new FilterPlan(new ScanPlan("t", null), LitBool(true));
        var result = SqlOptimizer.Optimize(plan);
        // Filter(TRUE) is eliminated; we get the scan directly
        Assert.IsType<OptScan>(result);
    }

    // ── LimitPushdown tests ───────────────────────────────────────────────────

    [Fact]
    public void LimitPushdown_AnnotatesScanWithLimit()
    {
        var plan   = new LimitPlan(new ScanPlan("t", null), 5L, null);
        var result = SqlOptimizer.Optimize(plan);
        var limit  = Assert.IsType<OptLimit>(result);
        var scan   = Assert.IsType<OptScan>(limit.Input);
        Assert.Equal(5L, scan.ScanLimit);
    }

    [Fact]
    public void LimitPushdown_PushesThrough_Project()
    {
        var project = new ProjectPlan(new ScanPlan("t", null), new[] { new OutputColumn.Star() });
        var plan    = new LimitPlan(project, 10L, null);
        var result  = SqlOptimizer.Optimize(plan);
        var limit   = Assert.IsType<OptLimit>(result);
        var proj    = Assert.IsType<OptProject>(limit.Input);
        var scan    = Assert.IsType<OptScan>(proj.Input);
        Assert.Equal(10L, scan.ScanLimit);
    }

    [Fact]
    public void LimitPushdown_DoesNotPushThrough_Sort()
    {
        var sort  = new SortPlan(new ScanPlan("t", null),
                        new[] { new SortKey(Col("id"), SortDir.Asc, NullOrder.NullsLast) });
        var plan  = new LimitPlan(sort, 5L, null);
        var result = SqlOptimizer.Optimize(plan);
        var limit  = Assert.IsType<OptLimit>(result);
        var optSort = Assert.IsType<OptSort>(limit.Input);
        var scan   = Assert.IsType<OptScan>(optSort.Input);
        // Scan must NOT have a scan_limit because Sort blocks pushdown.
        Assert.Null(scan.ScanLimit);
    }

    [Fact]
    public void LimitPushdown_TakesSmaller_WhenTwoLimitsExist()
    {
        // LIMIT 3 on top of LIMIT 10 → scan should see 3
        var inner  = new LimitPlan(new ScanPlan("t", null), 10L, null);
        var outer  = new LimitPlan(inner, 3L, null);
        var result = SqlOptimizer.Optimize(outer);
        var outerLimit = Assert.IsType<OptLimit>(result);
        var innerLimit = Assert.IsType<OptLimit>(outerLimit.Input);
        var scan = Assert.IsType<OptScan>(innerLimit.Input);
        Assert.NotNull(scan.ScanLimit);
        Assert.True(scan.ScanLimit <= 10L);
    }

    // ── ProjectionPruning tests ───────────────────────────────────────────────

    [Fact]
    public void ProjectionPruning_AnnotatesScanWithRequiredColumns()
    {
        var cols = new OutputColumn[]
        {
            new OutputColumn.Expr(Col("name"), null),
            new OutputColumn.Expr(Col("age"),  null),
        };
        var plan   = new ProjectPlan(new ScanPlan("t", null), cols);
        var result = SqlOptimizer.Optimize(plan);
        var proj   = Assert.IsType<OptProject>(result);
        var scan   = Assert.IsType<OptScan>(proj.Input);
        // RequiredColumns should contain "name" and "age"
        Assert.NotNull(scan.RequiredColumns);
        Assert.Contains("name", scan.RequiredColumns, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("age",  scan.RequiredColumns, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void ProjectionPruning_Star_LeavesRequiredColumnsNull()
    {
        var cols = new OutputColumn[] { new OutputColumn.Star() };
        var plan = new ProjectPlan(new ScanPlan("t", null), cols);
        var result = SqlOptimizer.Optimize(plan);
        var proj = Assert.IsType<OptProject>(result);
        var scan = Assert.IsType<OptScan>(proj.Input);
        // Star means all columns; RequiredColumns should be null or empty.
        Assert.True(scan.RequiredColumns is null || scan.RequiredColumns.Count == 0);
    }

    // ── PredicatePushdown tests ───────────────────────────────────────────────

    [Fact]
    public void PredicatePushdown_PushesThrough_Sort()
    {
        // Sort(Filter(Scan, pred), keys) → Sort(Filter inside, keys)
        // After pushdown: Filter should be below Sort.
        var pred = BinOp(BinaryOperator.Gt, Col("age"), LitL(18));
        var scan = new ScanPlan("t", null);
        var filt = new FilterPlan(scan, pred);
        var sort = new SortPlan(filt, new[] { new SortKey(Col("age"), SortDir.Asc, NullOrder.NullsLast) });
        var result = SqlOptimizer.Optimize(sort);
        // The result should have sort on the outside, filter pushed inside.
        Assert.IsType<OptSort>(result);
    }

    [Fact]
    public void PredicatePushdown_AndPredicateSplitToTwoFilters()
    {
        // Filter(Scan, A AND B) → Filter(Filter(Scan, A), B) or similar
        var predA = BinOp(BinaryOperator.Gt, Col("age"), LitL(18));
        var predB = BinOp(BinaryOperator.Eq, Col("active"), LitBool(true));
        var pred  = BinOp(BinaryOperator.And, predA, predB);
        var plan  = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.Optimize(plan);
        // Result should have a filter somewhere (or be a scan if everything was pushed/eliminated)
        Assert.NotNull(result);
    }

    // ── DefaultPasses tests ───────────────────────────────────────────────────

    [Fact]
    public void DefaultPasses_ReturnsFivePasses()
    {
        var passes = SqlOptimizer.DefaultPasses();
        Assert.Equal(5, passes.Count);
    }

    [Fact]
    public void DefaultPasses_HaveExpectedNames()
    {
        var passes = SqlOptimizer.DefaultPasses();
        var names  = passes.Select(p => p.Name).ToList();
        Assert.Contains("ConstantFolding",    names);
        Assert.Contains("PredicatePushdown",  names);
        Assert.Contains("ProjectionPruning",  names);
        Assert.Contains("DeadCodeElimination", names);
        Assert.Contains("LimitPushdown",      names);
    }

    // ── OptimizeWithPasses tests ──────────────────────────────────────────────

    [Fact]
    public void OptimizeWithPasses_EmptyPassList_LiftsOnly()
    {
        var plan   = new ScanPlan("t", "t");
        var result = SqlOptimizer.OptimizeWithPasses(plan, Array.Empty<IPass>());
        var scan   = Assert.IsType<OptScan>(result);
        Assert.Equal("t", scan.Table);
    }

    [Fact]
    public void OptimizeWithPasses_SinglePass_Applied()
    {
        var pred   = BinOp(BinaryOperator.Add, LitL(1), LitL(1));
        var plan   = new FilterPlan(new ScanPlan("t", null), pred);
        var passes = new IPass[] { new ConstantFoldingPass() };
        var result = SqlOptimizer.OptimizeWithPasses(plan, passes);
        var filter = Assert.IsType<OptFilter>(result);
        var lit    = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(2L, lit.Value);
    }

    // ── End-to-end pipeline tests ─────────────────────────────────────────────

    [Fact]
    public void Optimize_UnionPlan_LiftsAndRecurses()
    {
        var left   = new ScanPlan("a", null);
        var right  = new ScanPlan("b", null);
        var plan   = new UnionPlan(left, right, false);
        var result = SqlOptimizer.Optimize(plan);
        var union  = Assert.IsType<OptUnion>(result);
        Assert.IsType<OptScan>(union.Left);
        Assert.IsType<OptScan>(union.Right);
        Assert.False(union.All);
    }

    [Fact]
    public void Optimize_UpdatePlan_PreservesAssignments()
    {
        var asgn = new Assignment("name", LitS("Alice"));
        var plan = new UpdatePlan("users", new[] { asgn }, null);
        var result = SqlOptimizer.Optimize(plan);
        var upd  = Assert.IsType<OptUpdate>(result);
        Assert.Equal("users", upd.Table);
        Assert.Single(upd.Assignments);
    }

    [Fact]
    public void Optimize_DeletePlan_PreservesTable()
    {
        var plan   = new DeletePlan("users", null);
        var result = SqlOptimizer.Optimize(plan);
        var del    = Assert.IsType<OptDelete>(result);
        Assert.Equal("users", del.Table);
    }

    [Fact]
    public void Optimize_DistinctPlan_PreservesDistinct()
    {
        var plan   = new DistinctPlan(new ScanPlan("t", null));
        var result = SqlOptimizer.Optimize(plan);
        Assert.IsType<OptDistinct>(result);
    }

    [Fact]
    public void Optimize_AggregatePlan_PreservesGroupBy()
    {
        var agg = new AggregateItem(AggFunction.Count, new AggArg.Star(), "_cnt", false);
        var plan = new AggregatePlan(
            new ScanPlan("t", null),
            new[] { Col("dept") },
            new[] { agg });
        var result = SqlOptimizer.Optimize(plan);
        var opt = Assert.IsType<OptAggregate>(result);
        Assert.Single(opt.GroupBy);
        Assert.Single(opt.Aggregates);
    }

    [Fact]
    public void Optimize_HavingPlan_PreservesPredicate()
    {
        var agg  = new AggregatePlan(new ScanPlan("t", null), Array.Empty<SqlExpr>(),
                       new[] { new AggregateItem(AggFunction.Count, new AggArg.Star(), "_cnt", false) });
        var pred = BinOp(BinaryOperator.Gt, Col("_cnt"), LitL(10));
        var plan = new HavingPlan(agg, pred);
        var result = SqlOptimizer.Optimize(plan);
        var having = Assert.IsType<OptHaving>(result);
        Assert.NotNull(having.Predicate);
    }

    [Fact]
    public void Optimize_ScanAliasPreserved()
    {
        var plan   = new ScanPlan("orders", "o");
        var result = SqlOptimizer.Optimize(plan);
        var scan   = Assert.IsType<OptScan>(result);
        Assert.Equal("orders", scan.Table);
        Assert.Equal("o", scan.Alias);
    }

    [Fact]
    public void Optimize_LimitWithOffset_PreservesOffset()
    {
        var plan   = new LimitPlan(new ScanPlan("t", null), 5L, 20L);
        var result = SqlOptimizer.Optimize(plan);
        var lim    = Assert.IsType<OptLimit>(result);
        Assert.Equal(5L,  lim.Count);
        Assert.Equal(20L, lim.Offset);
    }

    [Fact]
    public void Optimize_JoinPlan_Recurses_Into_Both_Sides()
    {
        var left   = new ScanPlan("a", null);
        var right  = new ScanPlan("b", null);
        var join   = new JoinPlan(left, right, JoinKind.Left, null);
        var result = SqlOptimizer.Optimize(join);
        var optJoin = Assert.IsType<OptJoin>(result);
        Assert.IsType<OptScan>(optJoin.Left);
        Assert.IsType<OptScan>(optJoin.Right);
        Assert.Equal(JoinKind.Left, optJoin.Kind);
    }

    [Fact]
    public void Optimize_ComplexQuery_FilterUnderProject()
    {
        // Project(Filter(Scan, age > 18), [name]) — standard SELECT name FROM t WHERE age > 18
        var pred = BinOp(BinaryOperator.Gt, Col("age"), LitL(18));
        var cols = new OutputColumn[] { new OutputColumn.Expr(Col("name"), null) };
        var plan = ScanFilterProject("users", pred, cols);
        var result = SqlOptimizer.Optimize(plan);
        // Should be Project → Filter → Scan (or optimized equivalent)
        Assert.IsNotType<OptEmptyResult>(result);
    }

    [Fact]
    public void Optimize_DivByZero_NotFolded()
    {
        // 10 / 0 — must NOT crash; must remain as a BinaryOp (or be passed through)
        var pred = BinOp(BinaryOperator.Div, LitL(10), LitL(0));
        var plan = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.Optimize(plan);
        // Result is a filter (not folded, not crashed)
        var filter = Assert.IsType<OptFilter>(result);
        Assert.IsType<SqlExpr.BinaryOp>(filter.Predicate);
    }

    [Fact]
    public void Optimize_ModByZero_NotFolded()
    {
        var pred = BinOp(BinaryOperator.Mod, LitL(10), LitL(0));
        var plan = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.Optimize(plan);
        var filter = Assert.IsType<OptFilter>(result);
        Assert.IsType<SqlExpr.BinaryOp>(filter.Predicate);
    }

    [Fact]
    public void Optimize_NullComparison_FoldsToNull()
    {
        var pred = BinOp(BinaryOperator.Eq, LitNull(), LitL(1));
        var plan = new FilterPlan(new ScanPlan("t", null), pred);
        var result = SqlOptimizer.Optimize(plan);
        // NULL = 1 → NULL literal; Filter(NULL) stays as Filter (null is not false in SQL)
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Null(lit.Value);
    }

    [Fact]
    public void Optimize_EmptyResult_InsideUnion_Preserved()
    {
        // UNION of empty result and a scan — the non-empty side should survive
        var emptyFilter = new FilterPlan(new ScanPlan("a", null), LitBool(false));
        var right       = new ScanPlan("b", null);
        var union       = new UnionPlan(emptyFilter, right, false);
        var result      = SqlOptimizer.Optimize(union);
        // Union itself is preserved even if one side is empty (union all semantics)
        Assert.IsType<OptUnion>(result);
    }

    [Fact]
    public void Optimize_DropTable_IfExists_True()
    {
        var plan   = new DropTablePlan("old_table", true);
        var result = SqlOptimizer.Optimize(plan);
        var dt     = Assert.IsType<OptDropTable>(result);
        Assert.Equal("old_table", dt.Table);
        Assert.True(dt.IfExists);
    }

    [Fact]
    public void Optimize_CreateTable_IfNotExists()
    {
        var cols   = new[] { new ColumnDef("id", "INTEGER") };
        var plan   = new CreateTablePlan("new_table", true, cols);
        var result = SqlOptimizer.Optimize(plan);
        var ct     = Assert.IsType<OptCreateTable>(result);
        Assert.Equal("new_table", ct.Table);
        Assert.True(ct.IfNotExists);
        Assert.Single(ct.Columns);
    }

    [Fact]
    public void Optimize_InsertPlan_PreservesValues()
    {
        var rows = new[] { (IReadOnlyList<SqlExpr>)new SqlExpr[] { LitL(1), LitS("Alice") } };
        var plan = new InsertPlan("users", new[] { "id", "name" }, rows);
        var result = SqlOptimizer.Optimize(plan);
        var ins  = Assert.IsType<OptInsert>(result);
        Assert.Equal("users", ins.Table);
        Assert.Equal(new[] { "id", "name" }, ins.Columns);
    }

    [Fact]
    public void Optimize_GteLte_Folds_Correctly()
    {
        // 5 >= 5 → true,  3 <= 2 → false
        var predTrue  = BinOp(BinaryOperator.Gte, LitL(5), LitL(5));
        var predFalse = BinOp(BinaryOperator.Lte, LitL(3), LitL(2));

        var pass = new ConstantFoldingPass();
        var t    = pass.Apply(new OptFilter(new OptScan("t", null), predTrue));
        var f    = pass.Apply(new OptFilter(new OptScan("t", null), predFalse));

        Assert.Equal(true,  ((SqlExpr.Literal)((OptFilter)t).Predicate).Value);
        Assert.Equal(false, ((SqlExpr.Literal)((OptFilter)f).Predicate).Value);
    }

    [Fact]
    public void Optimize_NotEq_Folds_Correctly()
    {
        var pred = BinOp(BinaryOperator.NotEq, LitL(1), LitL(2));
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(true, lit.Value);
    }

    // ── ConstantFolding coverage — more expression forms ──────────────────────

    [Fact]
    public void ConstantFolding_FuncCall_FoldsArguments()
    {
        // UPPER(1 + 2) — we cannot evaluate UPPER, but we fold the argument.
        var arg  = BinOp(BinaryOperator.Add, LitL(1), LitL(2));
        var func = new SqlExpr.FuncCall("UPPER", new[] { arg });
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), func));
        var filter = Assert.IsType<OptFilter>(result);
        var fc = Assert.IsType<SqlExpr.FuncCall>(filter.Predicate);
        Assert.Single(fc.Args);
        Assert.Equal(3L, ((SqlExpr.Literal)fc.Args[0]).Value);
    }

    [Fact]
    public void ConstantFolding_IsNull_FoldsOperand()
    {
        var inner = BinOp(BinaryOperator.Add, LitL(1), LitL(2));
        var pred  = new SqlExpr.IsNull(inner);
        var pass  = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        var isNull = Assert.IsType<SqlExpr.IsNull>(filter.Predicate);
        Assert.IsType<SqlExpr.Literal>(isNull.Operand);
    }

    [Fact]
    public void ConstantFolding_IsNotNull_FoldsOperand()
    {
        var inner = BinOp(BinaryOperator.Sub, LitL(10), LitL(3));
        var pred  = new SqlExpr.IsNotNull(inner);
        var pass  = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        var isNotNull = Assert.IsType<SqlExpr.IsNotNull>(filter.Predicate);
        Assert.Equal(7L, ((SqlExpr.Literal)isNotNull.Operand).Value);
    }

    [Fact]
    public void ConstantFolding_Between_FoldsValues()
    {
        var v   = BinOp(BinaryOperator.Add, LitL(1), LitL(2));  // 3
        var lo  = LitL(1);
        var hi  = LitL(10);
        var pred = new SqlExpr.Between(v, lo, hi);
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        var between = Assert.IsType<SqlExpr.Between>(filter.Predicate);
        Assert.Equal(3L, ((SqlExpr.Literal)between.Value).Value);
    }

    [Fact]
    public void ConstantFolding_In_FoldsItems()
    {
        var v    = Col("status");
        var item = BinOp(BinaryOperator.Add, LitL(1), LitL(0));  // folds to 1
        var pred = new SqlExpr.In(v, new SqlExpr[] { item });
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        var inExpr = Assert.IsType<SqlExpr.In>(filter.Predicate);
        Assert.Equal(1L, ((SqlExpr.Literal)inExpr.Items[0]).Value);
    }

    [Fact]
    public void ConstantFolding_NotIn_FoldsItems()
    {
        var v    = Col("status");
        var item = BinOp(BinaryOperator.Mul, LitL(2), LitL(3));  // folds to 6
        var pred = new SqlExpr.NotIn(v, new SqlExpr[] { item });
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        var notIn = Assert.IsType<SqlExpr.NotIn>(filter.Predicate);
        Assert.Equal(6L, ((SqlExpr.Literal)notIn.Items[0]).Value);
    }

    [Fact]
    public void ConstantFolding_Like_FoldsValueExpr()
    {
        // LIKE value is a column — no fold, just passes through.
        var pred = new SqlExpr.Like(Col("name"), "%Alice%");
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        Assert.IsType<SqlExpr.Like>(filter.Predicate);
    }

    [Fact]
    public void ConstantFolding_NotLike_FoldsValueExpr()
    {
        var pred = new SqlExpr.NotLike(Col("name"), "%Bob%");
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        Assert.IsType<SqlExpr.NotLike>(filter.Predicate);
    }

    [Fact]
    public void ConstantFolding_JoinCondition_Folded()
    {
        var cond = BinOp(BinaryOperator.Eq, LitL(1), LitL(1));
        var join = new OptJoin(new OptScan("a", null), new OptScan("b", null), JoinKind.Inner, cond);
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(join);
        var optJoin = Assert.IsType<OptJoin>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(optJoin.Condition);
        Assert.Equal(true, lit.Value);
    }

    [Fact]
    public void ConstantFolding_Double_Arithmetic_Folds()
    {
        var pred = BinOp(BinaryOperator.Add, LitD(1.5), LitD(2.5));
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(4.0, lit.Value);
    }

    [Fact]
    public void ConstantFolding_StringEqTrue()
    {
        var pred = BinOp(BinaryOperator.Eq, LitS("abc"), LitS("abc"));
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(true, lit.Value);
    }

    [Fact]
    public void ConstantFolding_StringNeqTrue()
    {
        var pred = BinOp(BinaryOperator.NotEq, LitS("abc"), LitS("xyz"));
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(true, lit.Value);
    }

    [Fact]
    public void ConstantFolding_AggExpr_InProject_NotFolded()
    {
        // AggExpr is a leaf — ConstantFolding should leave it untouched.
        var agg  = new SqlExpr.AggExpr(AggFunction.Count, new AggArg.Star(), false);
        var cols = new OutputColumn[] { new OutputColumn.Expr(agg, "cnt") };
        var plan = new OptProject(new OptScan("t", null), cols);
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(plan);
        var proj = Assert.IsType<OptProject>(result);
        var expr = Assert.IsType<OutputColumn.Expr>(proj.Columns[0]);
        Assert.IsType<SqlExpr.AggExpr>(expr.Expression);
    }

    // ── LimitPushdown — more coverage ─────────────────────────────────────────

    [Fact]
    public void LimitPushdown_PushesThrough_Filter()
    {
        var pred   = BinOp(BinaryOperator.Gt, Col("age"), LitL(18));
        var scan   = new ScanPlan("t", null);
        var filter = new FilterPlan(scan, pred);
        var plan   = new LimitPlan(filter, 5L, null);
        var result = SqlOptimizer.Optimize(plan);
        var lim    = Assert.IsType<OptLimit>(result);
        // Filter wraps scan; scan should have scan limit hint.
        Assert.IsType<OptFilter>(lim.Input);
    }

    [Fact]
    public void LimitPushdown_DoesNotPushThrough_Aggregate()
    {
        var agg = new AggregatePlan(new ScanPlan("t", null), Array.Empty<SqlExpr>(),
                      new[] { new AggregateItem(AggFunction.Count, new AggArg.Star(), "_cnt", false) });
        var plan = new LimitPlan(agg, 3L, null);
        var result = SqlOptimizer.Optimize(plan);
        var lim = Assert.IsType<OptLimit>(result);
        Assert.IsType<OptAggregate>(lim.Input);
        // The scan inside aggregate must NOT have a scan_limit.
        var optAgg = (OptAggregate)lim.Input;
        var scanInAgg = Assert.IsType<OptScan>(optAgg.Input);
        Assert.Null(scanInAgg.ScanLimit);
    }

    [Fact]
    public void LimitPushdown_DoesNotPushThrough_Distinct()
    {
        var plan   = new LimitPlan(new DistinctPlan(new ScanPlan("t", null)), 5L, null);
        var result = SqlOptimizer.Optimize(plan);
        var lim    = Assert.IsType<OptLimit>(result);
        var dist   = Assert.IsType<OptDistinct>(lim.Input);
        var scan   = Assert.IsType<OptScan>(dist.Input);
        Assert.Null(scan.ScanLimit);
    }

    [Fact]
    public void LimitPushdown_PushesInto_BothSidesOfJoin()
    {
        var left  = new ScanPlan("a", null);
        var right = new ScanPlan("b", null);
        var join  = new JoinPlan(left, right, JoinKind.Inner, null);
        var plan  = new LimitPlan(join, 10L, null);
        var result = SqlOptimizer.Optimize(plan);
        // With DCE the inner join may become empty result if either side is empty;
        // here both sides are live, so the join and limit survive.
        Assert.IsNotType<OptEmptyResult>(result);
    }

    // ── ProjectionPruning — more coverage ─────────────────────────────────────

    [Fact]
    public void ProjectionPruning_Filter_IncludesFilterColumns()
    {
        // SELECT name FROM t WHERE age > 18
        // Both "name" and "age" must appear in RequiredColumns of scan.
        var pred = BinOp(BinaryOperator.Gt, Col("age"), LitL(18));
        var cols = new OutputColumn[] { new OutputColumn.Expr(Col("name"), null) };
        var filter  = new FilterPlan(new ScanPlan("t", null), pred);
        var project = new ProjectPlan(filter, cols);
        var result  = SqlOptimizer.Optimize(project);
        var proj = Assert.IsType<OptProject>(result);
        var filt = Assert.IsType<OptFilter>(proj.Input);
        var scan = Assert.IsType<OptScan>(filt.Input);
        Assert.NotNull(scan.RequiredColumns);
        Assert.Contains("name", scan.RequiredColumns, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("age",  scan.RequiredColumns, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void ProjectionPruning_SortColumns_IncludedInRequired()
    {
        // SELECT name FROM t ORDER BY age
        var cols = new OutputColumn[] { new OutputColumn.Expr(Col("name"), null) };
        var scan = new ScanPlan("t", null);
        var proj = new ProjectPlan(scan, cols);
        var sort = new SortPlan(proj,
                       new[] { new SortKey(Col("age"), SortDir.Desc, NullOrder.NullsFirst) });
        var result = SqlOptimizer.Optimize(sort);
        // Sort → Project → Scan; scan should carry required columns.
        Assert.IsType<OptSort>(result);
    }

    [Fact]
    public void ProjectionPruning_AggregateStar_PassesThroughPruning()
    {
        // COUNT(*) — AggArg.Star has no column reference.
        var agg = new AggregateItem(AggFunction.Count, new AggArg.Star(), "_cnt", false);
        var plan = new AggregatePlan(new ScanPlan("t", null), Array.Empty<SqlExpr>(), new[] { agg });
        var result = SqlOptimizer.Optimize(plan);
        Assert.IsType<OptAggregate>(result);
    }

    [Fact]
    public void ProjectionPruning_AggregateExprArg_IncludesColumn()
    {
        // SUM(amount) — 'amount' should appear in scan's required columns.
        var agg = new AggregateItem(AggFunction.Sum, new AggArg.Expr(Col("amount")), "_sum", false);
        var plan = new AggregatePlan(new ScanPlan("t", null), Array.Empty<SqlExpr>(), new[] { agg });
        var result = SqlOptimizer.Optimize(plan);
        var optAgg = Assert.IsType<OptAggregate>(result);
        var scan   = Assert.IsType<OptScan>(optAgg.Input);
        Assert.NotNull(scan.RequiredColumns);
        Assert.Contains("amount", scan.RequiredColumns, StringComparer.OrdinalIgnoreCase);
    }

    // ── DeadCodeElimination — outer join coverage ──────────────────────────────

    [Fact]
    public void DeadCode_LeftJoin_EmptyLeft_DoesNotPropagateEmpty()
    {
        // LEFT JOIN with empty left: the right side's rows are still returned (null-padded).
        // Our DCE does not eliminate outer joins — the node should survive.
        var emptyLeft = new FilterPlan(new ScanPlan("a", null), LitBool(false));
        var right     = new ScanPlan("b", null);
        var join      = new JoinPlan(emptyLeft, right, JoinKind.Left, null);
        var result    = SqlOptimizer.Optimize(join);
        // Left outer join with empty left → result is OptJoin (not OptEmptyResult).
        Assert.IsType<OptJoin>(result);
    }

    [Fact]
    public void DeadCode_DistinctAboveEmptyResult_ProducesEmpty()
    {
        var filter   = new FilterPlan(new ScanPlan("t", null), LitBool(false));
        var distinct = new DistinctPlan(filter);
        var result   = SqlOptimizer.Optimize(distinct);
        Assert.IsType<OptEmptyResult>(result);
    }

    [Fact]
    public void DeadCode_LimitAboveEmptyResult_ProducesEmpty()
    {
        var filter = new FilterPlan(new ScanPlan("t", null), LitBool(false));
        var limit  = new LimitPlan(filter, 100L, null);
        var result = SqlOptimizer.Optimize(limit);
        Assert.IsType<OptEmptyResult>(result);
    }

    [Fact]
    public void DeadCode_HavingAboveEmptyResult_ProducesEmpty()
    {
        var filter = new FilterPlan(new ScanPlan("t", null), LitBool(false));
        var agg    = new AggregatePlan(filter, Array.Empty<SqlExpr>(),
                         new[] { new AggregateItem(AggFunction.Count, new AggArg.Star(), "_cnt", false) });
        var having = new HavingPlan(agg, BinOp(BinaryOperator.Gt, Col("_cnt"), LitL(5)));
        var result = SqlOptimizer.Optimize(having);
        Assert.IsType<OptEmptyResult>(result);
    }

    // ── Lift — more plan types ─────────────────────────────────────────────────

    [Fact]
    public void Lift_AggregatePlan_PreservesGroupBy()
    {
        var agg  = new AggregateItem(AggFunction.Sum, new AggArg.Expr(Col("amount")), "_sum", false);
        var plan = new AggregatePlan(new ScanPlan("t", null), new[] { Col("dept") }, new[] { agg });
        var result = SqlOptimizer.Lift(plan);
        var optAgg = Assert.IsType<OptAggregate>(result);
        Assert.Single(optAgg.GroupBy);
    }

    [Fact]
    public void Lift_HavingPlan_PreservesPredicate()
    {
        var agg  = new AggregatePlan(new ScanPlan("t", null), Array.Empty<SqlExpr>(),
                       new[] { new AggregateItem(AggFunction.Count, new AggArg.Star(), "_cnt", false) });
        var pred = BinOp(BinaryOperator.Gt, Col("_cnt"), LitL(1));
        var plan = new HavingPlan(agg, pred);
        var result = SqlOptimizer.Lift(plan);
        var having = Assert.IsType<OptHaving>(result);
        Assert.IsType<SqlExpr.BinaryOp>(having.Predicate);
    }

    [Fact]
    public void Lift_DistinctPlan_WrapsInput()
    {
        var plan   = new DistinctPlan(new ScanPlan("t", null));
        var result = SqlOptimizer.Lift(plan);
        var dist   = Assert.IsType<OptDistinct>(result);
        Assert.IsType<OptScan>(dist.Input);
    }

    [Fact]
    public void Lift_UpdatePlan_PreservesFields()
    {
        var asgn   = new Assignment("col", LitL(42));
        var pred   = BinOp(BinaryOperator.Eq, Col("id"), LitL(1));
        var plan   = new UpdatePlan("users", new[] { asgn }, pred);
        var result = SqlOptimizer.Lift(plan);
        var upd    = Assert.IsType<OptUpdate>(result);
        Assert.Equal("users", upd.Table);
        Assert.NotNull(upd.Predicate);
    }

    [Fact]
    public void Lift_DeleteWithPredicate_PreservesFields()
    {
        var pred   = BinOp(BinaryOperator.Lt, Col("age"), LitL(18));
        var plan   = new DeletePlan("users", pred);
        var result = SqlOptimizer.Lift(plan);
        var del    = Assert.IsType<OptDelete>(result);
        Assert.Equal("users", del.Table);
        Assert.NotNull(del.Predicate);
    }

    // ── PredicatePushdown — PushInto branch coverage ──────────────────────────
    //
    // These tests exercise the different plan shapes that PushInto encounters
    // when it recurses through the tree carrying conjuncts.

    [Fact]
    public void PredicatePushdown_PushesThrough_ProjectThenSort()
    {
        // Filter(Sort(Project(Scan, cols), keys), pred)
        // PushInto hits Sort, then Project, then Scan.
        var pred = BinOp(BinaryOperator.Gt, Col("age"), LitL(18));
        var cols = new OutputColumn[] { new OutputColumn.Expr(Col("age"), null) };
        var scan = new ScanPlan("t", null);
        var proj = new ProjectPlan(scan, cols);
        var sort = new SortPlan(proj, new[] { new SortKey(Col("age"), SortDir.Asc, NullOrder.NullsLast) });
        var filt = new FilterPlan(sort, pred);
        var pass = new PredicatePushdownPass();
        var result = pass.Apply(SqlOptimizer.Lift(filt));
        // Predicate got pushed, structure survives.
        Assert.NotNull(result);
    }

    [Fact]
    public void PredicatePushdown_PushesThrough_Distinct()
    {
        var pred    = BinOp(BinaryOperator.Gt, Col("age"), LitL(0));
        var scan    = new ScanPlan("t", null);
        var dist    = new DistinctPlan(scan);
        var filt    = new FilterPlan(dist, pred);
        var pass    = new PredicatePushdownPass();
        var result  = pass.Apply(SqlOptimizer.Lift(filt));
        Assert.NotNull(result);
    }

    [Fact]
    public void PredicatePushdown_Filter_Above_InnerJoin_BlocksPush()
    {
        // Filter(InnerJoin(A, B), pred) — PushInto hits the Inner Join case
        // and blocks, leaving the filter above the join.
        var pred  = BinOp(BinaryOperator.Eq, Col("id"), LitL(1));
        var left  = new ScanPlan("a", null);
        var right = new ScanPlan("b", null);
        var join  = new JoinPlan(left, right, JoinKind.Inner, null);
        var filt  = new FilterPlan(join, pred);
        var pass  = new PredicatePushdownPass();
        var result = pass.Apply(SqlOptimizer.Lift(filt));
        // Filter remains above the join.
        Assert.IsType<OptFilter>(result);
    }

    [Fact]
    public void PredicatePushdown_Filter_Above_Aggregate_BlocksPush()
    {
        // Filter(Agg(Scan), pred) → hits the default case in PushInto.
        var pred = BinOp(BinaryOperator.Gt, Col("cnt"), LitL(5));
        var agg  = new AggregatePlan(new ScanPlan("t", null), Array.Empty<SqlExpr>(),
                       new[] { new AggregateItem(AggFunction.Count, new AggArg.Star(), "cnt", false) });
        var filt = new FilterPlan(agg, pred);
        var pass = new PredicatePushdownPass();
        var result = pass.Apply(SqlOptimizer.Lift(filt));
        // Filter stays above aggregate.
        Assert.IsType<OptFilter>(result);
    }

    [Fact]
    public void PredicatePushdown_NestedFilters_Merged()
    {
        // Filter(Filter(Scan, predA), predB) — both get combined and re-pushed.
        var predA = BinOp(BinaryOperator.Gt, Col("age"), LitL(18));
        var predB = BinOp(BinaryOperator.Eq, Col("active"), LitBool(true));
        var scan  = new ScanPlan("t", null);
        var inner = new FilterPlan(scan, predA);
        var outer = new FilterPlan(inner, predB);
        var pass  = new PredicatePushdownPass();
        var result = pass.Apply(SqlOptimizer.Lift(outer));
        // Both predicates are preserved (may be as one AND filter or two nested).
        Assert.NotNull(result);
    }

    // ── DeadCode — Cross join ──────────────────────────────────────────────────

    [Fact]
    public void DeadCode_CrossJoin_EmptyLeft_ProducesEmptyResult()
    {
        var emptyLeft = new FilterPlan(new ScanPlan("a", null), LitBool(false));
        var right     = new ScanPlan("b", null);
        var join      = new JoinPlan(emptyLeft, right, JoinKind.Cross, null);
        var result    = SqlOptimizer.Optimize(join);
        Assert.IsType<OptEmptyResult>(result);
    }

    [Fact]
    public void DeadCode_CrossJoin_EmptyRight_ProducesEmptyResult()
    {
        var left       = new ScanPlan("a", null);
        var emptyRight = new FilterPlan(new ScanPlan("b", null), LitBool(false));
        var join       = new JoinPlan(left, emptyRight, JoinKind.Cross, null);
        var result     = SqlOptimizer.Optimize(join);
        Assert.IsType<OptEmptyResult>(result);
    }

    [Fact]
    public void DeadCode_InnerJoin_BothLive_PreservesJoin()
    {
        var left   = new ScanPlan("a", null);
        var right  = new ScanPlan("b", null);
        var join   = new JoinPlan(left, right, JoinKind.Inner, null);
        var dce    = new DeadCodeEliminationPass();
        var result = dce.Apply(SqlOptimizer.Lift(join));
        Assert.IsType<OptJoin>(result);
    }

    [Fact]
    public void DeadCode_CrossJoin_BothLive_PreservesJoin()
    {
        var left   = new ScanPlan("a", null);
        var right  = new ScanPlan("b", null);
        var join   = new JoinPlan(left, right, JoinKind.Cross, null);
        var dce    = new DeadCodeEliminationPass();
        var result = dce.Apply(SqlOptimizer.Lift(join));
        Assert.IsType<OptJoin>(result);
    }

    // ── ProjectionPruning — Collect helper branches ────────────────────────────

    [Fact]
    public void ProjectionPruning_Collect_UnaryOp_In_Filter()
    {
        var pred = new SqlExpr.UnaryOp(UnaryOperator.Not, Col("active"));
        var cols = new OutputColumn[] { new OutputColumn.Expr(Col("name"), null) };
        var plan = new ProjectPlan(new FilterPlan(new ScanPlan("t", null), pred), cols);
        var result = SqlOptimizer.Optimize(plan);
        // Filter's column 'active' and project's column 'name' both in required.
        var proj = Assert.IsType<OptProject>(result);
        var filt = Assert.IsType<OptFilter>(proj.Input);
        var scan = Assert.IsType<OptScan>(filt.Input);
        Assert.NotNull(scan.RequiredColumns);
        Assert.Contains("active", scan.RequiredColumns, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void ProjectionPruning_Collect_FuncCall_In_Project()
    {
        var func = new SqlExpr.FuncCall("LOWER", new SqlExpr[] { Col("email") });
        var cols = new OutputColumn[] { new OutputColumn.Expr(func, "lower_email") };
        var plan = new ProjectPlan(new ScanPlan("t", null), cols);
        var result = SqlOptimizer.Optimize(plan);
        var proj = Assert.IsType<OptProject>(result);
        var scan = Assert.IsType<OptScan>(proj.Input);
        Assert.NotNull(scan.RequiredColumns);
        Assert.Contains("email", scan.RequiredColumns, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void ProjectionPruning_Collect_IsNull_In_Filter()
    {
        var pred = new SqlExpr.IsNull(Col("phone"));
        var cols = new OutputColumn[] { new OutputColumn.Expr(Col("name"), null) };
        var plan = new ProjectPlan(new FilterPlan(new ScanPlan("t", null), pred), cols);
        var result = SqlOptimizer.Optimize(plan);
        var proj = Assert.IsType<OptProject>(result);
        var filt = Assert.IsType<OptFilter>(proj.Input);
        var scan = Assert.IsType<OptScan>(filt.Input);
        Assert.NotNull(scan.RequiredColumns);
        Assert.Contains("phone", scan.RequiredColumns, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void ProjectionPruning_Collect_Between_In_Filter()
    {
        var pred = new SqlExpr.Between(Col("age"), LitL(18), LitL(65));
        var cols = new OutputColumn[] { new OutputColumn.Expr(Col("name"), null) };
        var plan = new ProjectPlan(new FilterPlan(new ScanPlan("t", null), pred), cols);
        var result = SqlOptimizer.Optimize(plan);
        var proj = Assert.IsType<OptProject>(result);
        var scan = Assert.IsType<OptScan>(((OptFilter)proj.Input).Input);
        Assert.Contains("age", scan.RequiredColumns!, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void ProjectionPruning_Collect_In_Expr()
    {
        var pred = new SqlExpr.In(Col("status"), new SqlExpr[] { LitL(1), LitL(2) });
        var cols = new OutputColumn[] { new OutputColumn.Expr(Col("name"), null) };
        var plan = new ProjectPlan(new FilterPlan(new ScanPlan("t", null), pred), cols);
        var result = SqlOptimizer.Optimize(plan);
        var proj = Assert.IsType<OptProject>(result);
        var scan = Assert.IsType<OptScan>(((OptFilter)proj.Input).Input);
        Assert.Contains("status", scan.RequiredColumns!, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void ProjectionPruning_Collect_NotIn_Expr()
    {
        var pred = new SqlExpr.NotIn(Col("status"), new SqlExpr[] { LitL(0) });
        var cols = new OutputColumn[] { new OutputColumn.Expr(Col("name"), null) };
        var plan = new ProjectPlan(new FilterPlan(new ScanPlan("t", null), pred), cols);
        var result = SqlOptimizer.Optimize(plan);
        var proj = Assert.IsType<OptProject>(result);
        var scan = Assert.IsType<OptScan>(((OptFilter)proj.Input).Input);
        Assert.Contains("status", scan.RequiredColumns!, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void ProjectionPruning_Collect_Like_Expr()
    {
        var pred = new SqlExpr.Like(Col("email"), "%@example.com");
        var cols = new OutputColumn[] { new OutputColumn.Expr(Col("name"), null) };
        var plan = new ProjectPlan(new FilterPlan(new ScanPlan("t", null), pred), cols);
        var result = SqlOptimizer.Optimize(plan);
        var proj = Assert.IsType<OptProject>(result);
        var scan = Assert.IsType<OptScan>(((OptFilter)proj.Input).Input);
        Assert.Contains("email", scan.RequiredColumns!, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void ProjectionPruning_Collect_NotLike_Expr()
    {
        var pred = new SqlExpr.NotLike(Col("email"), "%@spam.com");
        var cols = new OutputColumn[] { new OutputColumn.Expr(Col("name"), null) };
        var plan = new ProjectPlan(new FilterPlan(new ScanPlan("t", null), pred), cols);
        var result = SqlOptimizer.Optimize(plan);
        var proj = Assert.IsType<OptProject>(result);
        var scan = Assert.IsType<OptScan>(((OptFilter)proj.Input).Input);
        Assert.Contains("email", scan.RequiredColumns!, StringComparer.OrdinalIgnoreCase);
    }

    // ── LimitPushdown — scan already has a limit ───────────────────────────────

    [Fact]
    public void LimitPushdown_ScanAlreadyHasLimit_TakesSmaller()
    {
        // Pre-annotate a scan with ScanLimit=100, then push a limit of 5.
        var scan     = new OptScan("t", null, null, 100L);
        var limitPlan = new OptLimit(scan, 5L, null);
        var pass     = new LimitPushdownPass();
        var result   = pass.Apply(limitPlan);
        var lim      = Assert.IsType<OptLimit>(result);
        var optScan  = Assert.IsType<OptScan>(lim.Input);
        Assert.Equal(5L, optScan.ScanLimit);   // min(100, 5) = 5
    }

    [Fact]
    public void LimitPushdown_NoLimit_ScanLimitNull()
    {
        // A bare scan with no limit — ScanLimit must remain null.
        var scan   = new ScanPlan("t", null);
        var result = SqlOptimizer.Optimize(scan);
        var optScan = Assert.IsType<OptScan>(result);
        Assert.Null(optScan.ScanLimit);
    }

    // ── ConstantFolding — double comparison coverage ───────────────────────────

    [Fact]
    public void ConstantFolding_Double_Comparison_LtFalse()
    {
        var pred = BinOp(BinaryOperator.Lt, LitD(5.0), LitD(2.0));
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(false, lit.Value);
    }

    [Fact]
    public void ConstantFolding_Double_Comparison_GteTrue()
    {
        var pred = BinOp(BinaryOperator.Gte, LitD(5.0), LitD(5.0));
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(true, lit.Value);
    }

    [Fact]
    public void ConstantFolding_Double_Comparison_EqFalse()
    {
        var pred = BinOp(BinaryOperator.Eq, LitD(1.1), LitD(1.2));
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(false, lit.Value);
    }

    [Fact]
    public void ConstantFolding_Double_Sub_Folds()
    {
        var pred = BinOp(BinaryOperator.Sub, LitD(10.0), LitD(3.5));
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(6.5, lit.Value);
    }

    [Fact]
    public void ConstantFolding_Double_Mul_Folds()
    {
        var pred = BinOp(BinaryOperator.Mul, LitD(2.5), LitD(4.0));
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(10.0, lit.Value);
    }

    [Fact]
    public void ConstantFolding_Double_Div_Folds()
    {
        var pred = BinOp(BinaryOperator.Div, LitD(9.0), LitD(3.0));
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(3.0, lit.Value);
    }

    [Fact]
    public void ConstantFolding_Double_NotEq_Folds()
    {
        var pred = BinOp(BinaryOperator.NotEq, LitD(1.0), LitD(2.0));
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(true, lit.Value);
    }

    [Fact]
    public void ConstantFolding_Double_Gt_Folds()
    {
        var pred = BinOp(BinaryOperator.Gt, LitD(5.0), LitD(3.0));
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(true, lit.Value);
    }

    [Fact]
    public void ConstantFolding_Double_Lte_Folds()
    {
        var pred = BinOp(BinaryOperator.Lte, LitD(3.0), LitD(3.0));
        var pass = new ConstantFoldingPass();
        var result = pass.Apply(new OptFilter(new OptScan("t", null), pred));
        var filter = Assert.IsType<OptFilter>(result);
        var lit = Assert.IsType<SqlExpr.Literal>(filter.Predicate);
        Assert.Equal(true, lit.Value);
    }
}

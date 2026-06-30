// SqlPlannerTests.cs — conformance tests for the C# sql-planner.
//
// Covers all 13 spec conformance points plus expression-type and error-path coverage.

namespace CodingAdventures.SqlPlanner.Tests;

public sealed class SqlPlannerTests
{
    // ── Shared schema ─────────────────────────────────────────────────────────

    /// <summary>users(id, name, age)  orders(id, user_id, amount)</summary>
    private static readonly InMemorySchemaProvider Schema = new(new Dictionary<string, IReadOnlyList<string>>
    {
        ["users"]  = new[] { "id", "name", "age" },
        ["orders"] = new[] { "id", "user_id", "amount" },
    });

    private static SqlPlanner Planner() => new(Schema);

    // ── Helpers ───────────────────────────────────────────────────────────────

    private static SelectStatement SimpleSelect(
        IReadOnlyList<OutputColumn> cols,
        string table,
        string? alias = null) => new(
            Distinct: false,
            Columns:  cols,
            From:     new[] { (table, alias) },
            Joins:    Array.Empty<JoinClause>(),
            Where:    null,
            GroupBy:  Array.Empty<SqlExpr>(),
            Having:   null,
            OrderBy:  Array.Empty<SortKey>(),
            Limit:    null);

    private static OutputColumn Col(string? tbl, string name) =>
        new OutputColumn.Expr(new SqlExpr.Column(tbl, name), null);

    // ──────────────────────────────────────────────────────────────────────────
    // Conformance tests (C1–C13)
    // ──────────────────────────────────────────────────────────────────────────

    [Fact]
    public void C1_SimpleSelect_ProducesProjectAboveScan()
    {
        var plan = Planner().Plan(new SelectStatement(
            false,
            new[] { Col(null, "id"), Col(null, "name") },
            new[] { ("users", (string?)null) },
            Array.Empty<JoinClause>(), null,
            Array.Empty<SqlExpr>(), null,
            Array.Empty<SortKey>(), null));

        var proj = Assert.IsType<ProjectPlan>(plan);
        Assert.Equal(2, proj.Columns.Count);
        var scan = Assert.IsType<ScanPlan>(proj.Input);
        Assert.Equal("users", scan.Table);
    }

    [Fact]
    public void C2_Where_InsertsFilterBetweenScanAndProject()
    {
        var pred = new SqlExpr.BinaryOp(BinaryOperator.Gt,
                       new SqlExpr.Column(null, "age"),
                       new SqlExpr.Literal(18L));

        var stmt = SimpleSelect(new[] { Col(null, "name") }, "users") with { Where = pred };
        var plan = Planner().Plan(stmt);

        var proj = Assert.IsType<ProjectPlan>(plan);
        var filt = Assert.IsType<FilterPlan>(proj.Input);
        Assert.IsType<ScanPlan>(filt.Input);
    }

    [Fact]
    public void C3_GroupBy_ProducesAggregateThenHavingThenProject()
    {
        var aggExpr = new SqlExpr.AggExpr(AggFunction.Count, new AggArg.Star(), false);
        var stmt = new SelectStatement(
            false,
            new OutputColumn[]
            {
                Col(null, "name"),
                new OutputColumn.Expr(aggExpr, "cnt"),
            },
            new[] { ("users", (string?)null) },
            Array.Empty<JoinClause>(),
            null,
            new[] { new SqlExpr.Column(null, "name") },
            new SqlExpr.BinaryOp(BinaryOperator.Gt, aggExpr, new SqlExpr.Literal(1L)),
            Array.Empty<SortKey>(),
            null);

        var plan = Planner().Plan(stmt);

        var proj = Assert.IsType<ProjectPlan>(plan);
        var hav  = Assert.IsType<HavingPlan>(proj.Input);
        var agg  = Assert.IsType<AggregatePlan>(hav.Input);
        Assert.IsType<ScanPlan>(agg.Input);
        Assert.Equal(1, agg.GroupBy.Count);
        Assert.NotEmpty(agg.Aggregates);
    }

    [Fact]
    public void C4_Join_ProducesJoinNodeAboveScans()
    {
        var onCond = new SqlExpr.BinaryOp(BinaryOperator.Eq,
                         new SqlExpr.Column("users",  "id"),
                         new SqlExpr.Column("orders", "user_id"));
        var stmt = new SelectStatement(
            false,
            new OutputColumn[] { Col("users", "name"), Col("orders", "amount") },
            new[] { ("users", (string?)null) },
            new[] { new JoinClause(JoinKind.Inner, "orders", null, onCond) },
            null, Array.Empty<SqlExpr>(), null, Array.Empty<SortKey>(), null);

        var plan = Planner().Plan(stmt);

        var proj = Assert.IsType<ProjectPlan>(plan);
        var join = Assert.IsType<JoinPlan>(proj.Input);
        Assert.IsType<ScanPlan>(join.Left);
        Assert.IsType<ScanPlan>(join.Right);
        Assert.Equal(JoinKind.Inner, join.Kind);
        Assert.NotNull(join.Condition);
    }

    [Fact]
    public void C5_OrderBy_ProducesSortAtTop()
    {
        var stmt = SimpleSelect(new[] { Col(null, "name") }, "users") with
        {
            OrderBy = new[] { new SortKey(new SqlExpr.Column(null, "age"), SortDir.Desc, NullOrder.NullsLast) },
        };

        var plan = Planner().Plan(stmt);

        var sort = Assert.IsType<SortPlan>(plan);
        Assert.Equal(1, sort.Keys.Count);
    }

    [Fact]
    public void C6_Limit_ProducesLimitAtTop()
    {
        var stmt = SimpleSelect(new[] { Col(null, "id") }, "users") with
        {
            Limit = new LimitClause(10L, 20L),
        };

        var plan = Planner().Plan(stmt);

        var lim = Assert.IsType<LimitPlan>(plan);
        Assert.Equal(10L, lim.Count);
        Assert.Equal(20L, lim.Offset);
    }

    [Fact]
    public void C7_Distinct_WrapsProjectInDistinct()
    {
        var stmt = SimpleSelect(new[] { Col(null, "name") }, "users") with { Distinct = true };
        var plan = Planner().Plan(stmt);

        var dist = Assert.IsType<DistinctPlan>(plan);
        Assert.IsType<ProjectPlan>(dist.Input);
    }

    [Fact]
    public void C8_Insert_ProducesInsertPlan()
    {
        var stmt = new InsertStatement(
            "users",
            new[] { "id", "name", "age" },
            new[] { new SqlExpr[] { new SqlExpr.Literal(1L), new SqlExpr.Literal("Alice"), new SqlExpr.Literal(30L) } });

        var plan = Planner().Plan(stmt);

        var ins = Assert.IsType<InsertPlan>(plan);
        Assert.Equal("users", ins.Table);
        Assert.Equal(3, ins.Columns!.Count);
        Assert.Equal(1, ins.Values.Count);
    }

    [Fact]
    public void C9_Update_ProducesUpdatePlan()
    {
        var stmt = new UpdateStatement(
            "users",
            new[] { new Assignment("name", new SqlExpr.Literal("Bob")) },
            new SqlExpr.BinaryOp(BinaryOperator.Eq, new SqlExpr.Column(null, "id"), new SqlExpr.Literal(1L)));

        var plan = Planner().Plan(stmt);

        var upd = Assert.IsType<UpdatePlan>(plan);
        Assert.Equal("users", upd.Table);
        Assert.Equal(1, upd.Assignments.Count);
        Assert.Equal("name", upd.Assignments[0].Column);
        Assert.NotNull(upd.Predicate);
    }

    [Fact]
    public void C10_Delete_ProducesDeletePlan()
    {
        var stmt = new DeleteStatement("users",
            new SqlExpr.BinaryOp(BinaryOperator.Eq, new SqlExpr.Column(null, "id"), new SqlExpr.Literal(2L)));

        var plan = Planner().Plan(stmt);

        var del = Assert.IsType<DeletePlan>(plan);
        Assert.Equal("users", del.Table);
        Assert.NotNull(del.Predicate);
    }

    [Fact]
    public void C11a_CreateTable_ProducesCreateTablePlan()
    {
        var stmt = new CreateTableStatement("products", true, new ColumnDef[]
        {
            new("id",    "INTEGER", NotNull: true,  PrimaryKey: true),
            new("name",  "TEXT",    NotNull: true),
            new("price", "REAL"),
        });

        var plan = Planner().Plan(stmt);

        var ct = Assert.IsType<CreateTablePlan>(plan);
        Assert.Equal("products", ct.Table);
        Assert.True(ct.IfNotExists);
        Assert.Equal(3, ct.Columns.Count);
    }

    [Fact]
    public void C11b_DropTable_ProducesDropTablePlan()
    {
        var plan = Planner().Plan(new DropTableStatement("users", false));

        var dt = Assert.IsType<DropTablePlan>(plan);
        Assert.Equal("users", dt.Table);
        Assert.False(dt.IfExists);
    }

    [Fact]
    public void C12_AmbiguousColumn_ThrowsAmbiguousColumnException()
    {
        // "id" exists in both users and orders.
        var stmt = new SelectStatement(
            false,
            new[] { Col(null, "id") },
            new[] { ("users", (string?)null) },
            new[] { new JoinClause(JoinKind.Inner, "orders", null, null) },
            null, Array.Empty<SqlExpr>(), null, Array.Empty<SortKey>(), null);

        var ex = Assert.Throws<AmbiguousColumnException>(() => Planner().Plan(stmt));
        Assert.Equal("id", ex.Column);
        Assert.Equal(2, ex.Tables.Count);
    }

    [Fact]
    public void C13_UnknownTable_ThrowsUnknownTableException()
    {
        var stmt = SimpleSelect(new[] { Col(null, "id") }, "ghost");

        var ex = Assert.Throws<UnknownTableException>(() => Planner().Plan(stmt));
        Assert.Equal("ghost", ex.Table);
    }

    // ──────────────────────────────────────────────────────────────────────────
    // PlanAll
    // ──────────────────────────────────────────────────────────────────────────

    [Fact]
    public void PlanAll_ReturnsList_FailsOnFirstError()
    {
        var stmts = new Statement[]
        {
            new InsertStatement("users", null, Array.Empty<IReadOnlyList<SqlExpr>>()),
            new DeleteStatement("users", null),
        };

        var plans = Planner().PlanAll(stmts);
        Assert.Equal(2, plans.Count);

        var badStmt = SimpleSelect(new[] { Col(null, "x") }, "ghost");
        Assert.Throws<UnknownTableException>(() => Planner().PlanAll(new Statement[] { badStmt }));
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Expression type coverage
    // ──────────────────────────────────────────────────────────────────────────

    [Fact]
    public void Where_FuncCall_IsResolved()
    {
        var pred = new SqlExpr.FuncCall("upper", new[] { new SqlExpr.Column(null, "name") });
        var stmt = SimpleSelect(new[] { Col(null, "id") }, "users") with { Where = pred };
        var plan = Planner().Plan(stmt);

        var proj = Assert.IsType<ProjectPlan>(plan);
        var filt = Assert.IsType<FilterPlan>(proj.Input);
        Assert.IsType<SqlExpr.FuncCall>(filt.Predicate);
    }

    [Fact]
    public void Where_IsNull_IsResolved()
    {
        var pred = new SqlExpr.IsNull(new SqlExpr.Column(null, "age"));
        var stmt = SimpleSelect(new[] { Col(null, "id") }, "users") with { Where = pred };
        var plan = Planner().Plan(stmt);

        var proj = Assert.IsType<ProjectPlan>(plan);
        var filt = Assert.IsType<FilterPlan>(proj.Input);
        Assert.IsType<SqlExpr.IsNull>(filt.Predicate);
    }

    [Fact]
    public void Where_IsNotNull_IsResolved()
    {
        var pred = new SqlExpr.IsNotNull(new SqlExpr.Column(null, "name"));
        var stmt = SimpleSelect(new[] { Col(null, "id") }, "users") with { Where = pred };
        var plan = Planner().Plan(stmt);

        var proj = Assert.IsType<ProjectPlan>(plan);
        var filt = Assert.IsType<FilterPlan>(proj.Input);
        Assert.IsType<SqlExpr.IsNotNull>(filt.Predicate);
    }

    [Fact]
    public void Where_Between_IsResolved()
    {
        var pred = new SqlExpr.Between(
            new SqlExpr.Column(null, "age"),
            new SqlExpr.Literal(18L),
            new SqlExpr.Literal(65L));
        var stmt = SimpleSelect(new[] { Col(null, "id") }, "users") with { Where = pred };
        var plan = Planner().Plan(stmt);

        var proj = Assert.IsType<ProjectPlan>(plan);
        var filt = Assert.IsType<FilterPlan>(proj.Input);
        Assert.IsType<SqlExpr.Between>(filt.Predicate);
    }

    [Fact]
    public void Where_In_IsResolved()
    {
        var pred = new SqlExpr.In(new SqlExpr.Column(null, "id"),
            new[] { new SqlExpr.Literal(1L), new SqlExpr.Literal(2L) });
        var stmt = SimpleSelect(new[] { Col(null, "name") }, "users") with { Where = pred };
        var plan = Planner().Plan(stmt);

        var proj = Assert.IsType<ProjectPlan>(plan);
        var filt = Assert.IsType<FilterPlan>(proj.Input);
        Assert.IsType<SqlExpr.In>(filt.Predicate);
    }

    [Fact]
    public void Where_NotIn_IsResolved()
    {
        var pred = new SqlExpr.NotIn(new SqlExpr.Column(null, "id"),
            new[] { new SqlExpr.Literal(1L) });
        var stmt = SimpleSelect(new[] { Col(null, "name") }, "users") with { Where = pred };
        var plan = Planner().Plan(stmt);

        var proj = Assert.IsType<ProjectPlan>(plan);
        var filt = Assert.IsType<FilterPlan>(proj.Input);
        Assert.IsType<SqlExpr.NotIn>(filt.Predicate);
    }

    [Fact]
    public void Where_Like_IsResolved()
    {
        var pred = new SqlExpr.Like(new SqlExpr.Column(null, "name"), "A%");
        var stmt = SimpleSelect(new[] { Col(null, "id") }, "users") with { Where = pred };
        var plan = Planner().Plan(stmt);

        var proj = Assert.IsType<ProjectPlan>(plan);
        var filt = Assert.IsType<FilterPlan>(proj.Input);
        var like = Assert.IsType<SqlExpr.Like>(filt.Predicate);
        Assert.Equal("A%", like.Pattern);
    }

    [Fact]
    public void Where_NotLike_IsResolved()
    {
        var pred = new SqlExpr.NotLike(new SqlExpr.Column(null, "name"), "B%");
        var stmt = SimpleSelect(new[] { Col(null, "id") }, "users") with { Where = pred };
        var plan = Planner().Plan(stmt);

        var proj = Assert.IsType<ProjectPlan>(plan);
        var filt = Assert.IsType<FilterPlan>(proj.Input);
        Assert.IsType<SqlExpr.NotLike>(filt.Predicate);
    }

    [Fact]
    public void Where_UnaryOp_IsResolved()
    {
        var pred = new SqlExpr.UnaryOp(UnaryOperator.Not,
            new SqlExpr.BinaryOp(BinaryOperator.Eq,
                new SqlExpr.Column(null, "age"),
                new SqlExpr.Literal(0L)));
        var stmt = SimpleSelect(new[] { Col(null, "id") }, "users") with { Where = pred };
        var plan = Planner().Plan(stmt);

        var proj = Assert.IsType<ProjectPlan>(plan);
        var filt = Assert.IsType<FilterPlan>(proj.Input);
        Assert.IsType<SqlExpr.UnaryOp>(filt.Predicate);
    }

    [Fact]
    public void Where_QualifiedColumn_IsResolved()
    {
        var pred = new SqlExpr.BinaryOp(BinaryOperator.Gt,
                       new SqlExpr.Column("users", "age"),
                       new SqlExpr.Literal(21L));
        var stmt = SimpleSelect(new[] { Col(null, "id") }, "users") with { Where = pred };
        var plan = Planner().Plan(stmt);

        var proj = Assert.IsType<ProjectPlan>(plan);
        var filt = Assert.IsType<FilterPlan>(proj.Input);
        var bin  = Assert.IsType<SqlExpr.BinaryOp>(filt.Predicate);
        var col  = Assert.IsType<SqlExpr.Column>(bin.Left);
        Assert.Equal("users", col.Table);
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Error paths
    // ──────────────────────────────────────────────────────────────────────────

    [Fact]
    public void Insert_UnknownTable_Throws()
    {
        var ex = Assert.Throws<UnknownTableException>(() =>
            Planner().Plan(new InsertStatement("ghost", null, Array.Empty<IReadOnlyList<SqlExpr>>())));
        Assert.Equal("ghost", ex.Table);
    }

    [Fact]
    public void Update_UnknownTable_Throws()
    {
        var ex = Assert.Throws<UnknownTableException>(() =>
            Planner().Plan(new UpdateStatement("ghost", Array.Empty<Assignment>(), null)));
        Assert.Equal("ghost", ex.Table);
    }

    [Fact]
    public void Delete_UnknownTable_Throws()
    {
        var ex = Assert.Throws<UnknownTableException>(() =>
            Planner().Plan(new DeleteStatement("ghost", null)));
        Assert.Equal("ghost", ex.Table);
    }

    [Fact]
    public void Where_UnknownColumn_Throws()
    {
        var pred = new SqlExpr.Column(null, "no_col");
        var stmt = SimpleSelect(new[] { Col(null, "id") }, "users") with { Where = pred };

        var ex = Assert.Throws<UnknownColumnException>(() => Planner().Plan(stmt));
        Assert.Equal("no_col", ex.Column);
        Assert.Null(ex.QualifyingTable);
    }

    [Fact]
    public void Where_QualifiedUnknownTable_Throws()
    {
        var pred = new SqlExpr.Column("no_such", "id");
        var stmt = SimpleSelect(new[] { Col(null, "id") }, "users") with { Where = pred };

        Assert.Throws<UnknownTableException>(() => Planner().Plan(stmt));
    }

    [Fact]
    public void Where_QualifiedUnknownColumn_Throws()
    {
        var pred = new SqlExpr.Column("users", "no_col");
        var stmt = SimpleSelect(new[] { Col(null, "id") }, "users") with { Where = pred };

        var ex = Assert.Throws<UnknownColumnException>(() => Planner().Plan(stmt));
        Assert.Equal("no_col", ex.Column);
        Assert.Equal("users", ex.QualifyingTable);
    }

    [Fact]
    public void Project_UnknownColumn_Throws()
    {
        var stmt = SimpleSelect(new[] { Col(null, "no_col") }, "users");
        Assert.Throws<UnknownColumnException>(() => Planner().Plan(stmt));
    }

    [Fact]
    public void Join_UnknownTable_Throws()
    {
        var stmt = new SelectStatement(
            false, new[] { Col(null, "id") },
            new[] { ("users", (string?)null) },
            new[] { new JoinClause(JoinKind.Inner, "ghost", null, null) },
            null, Array.Empty<SqlExpr>(), null, Array.Empty<SortKey>(), null);

        Assert.Throws<UnknownTableException>(() => Planner().Plan(stmt));
    }

    [Fact]
    public void MultiFrom_SecondUnknownTable_Throws()
    {
        var stmt = new SelectStatement(
            false, new[] { Col("users", "id") },
            new[] { ("users", (string?)null), ("ghost", (string?)null) },
            Array.Empty<JoinClause>(),
            null, Array.Empty<SqlExpr>(), null, Array.Empty<SortKey>(), null);

        Assert.Throws<UnknownTableException>(() => Planner().Plan(stmt));
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Structural tests
    // ──────────────────────────────────────────────────────────────────────────

    [Fact]
    public void TableAlias_IsThreadedThroughScope()
    {
        var stmt = new SelectStatement(
            false,
            new OutputColumn[] { new OutputColumn.Expr(new SqlExpr.Column("u", "name"), null) },
            new[] { ("users", (string?)"u") },
            Array.Empty<JoinClause>(),
            new SqlExpr.BinaryOp(BinaryOperator.Eq, new SqlExpr.Column("u", "id"), new SqlExpr.Literal(1L)),
            Array.Empty<SqlExpr>(), null, Array.Empty<SortKey>(), null);

        var plan = Planner().Plan(stmt);

        var proj = Assert.IsType<ProjectPlan>(plan);
        var filt = Assert.IsType<FilterPlan>(proj.Input);
        var scan = Assert.IsType<ScanPlan>(filt.Input);
        Assert.Equal("u", scan.Alias);
    }

    [Fact]
    public void SelectStar_IsPreservedInProject()
    {
        var stmt = SimpleSelect(new OutputColumn[] { new OutputColumn.Star() }, "users");
        var plan = Planner().Plan(stmt);

        var proj = Assert.IsType<ProjectPlan>(plan);
        Assert.IsType<OutputColumn.Star>(proj.Columns[0]);
    }

    [Fact]
    public void MultiFrom_ProducesCrossJoin()
    {
        var stmt = new SelectStatement(
            false,
            new OutputColumn[] { Col("users", "id"), Col("orders", "amount") },
            new[] { ("users", (string?)null), ("orders", (string?)null) },
            Array.Empty<JoinClause>(),
            null, Array.Empty<SqlExpr>(), null, Array.Empty<SortKey>(), null);

        var plan = Planner().Plan(stmt);

        var proj = Assert.IsType<ProjectPlan>(plan);
        var join = Assert.IsType<JoinPlan>(proj.Input);
        Assert.Equal(JoinKind.Cross, join.Kind);
    }

    [Fact]
    public void DistinctOrderByLimit_Stack_Correctly()
    {
        var stmt = new SelectStatement(
            true,
            new[] { Col(null, "name") },
            new[] { ("users", (string?)null) },
            Array.Empty<JoinClause>(), null,
            Array.Empty<SqlExpr>(), null,
            new[] { new SortKey(new SqlExpr.Column(null, "name"), SortDir.Asc, NullOrder.NullsFirst) },
            new LimitClause(5L, null));

        var plan = Planner().Plan(stmt);

        var lim  = Assert.IsType<LimitPlan>(plan);
        var sort = Assert.IsType<SortPlan>(lim.Input);
        var dist = Assert.IsType<DistinctPlan>(sort.Input);
        Assert.IsType<ProjectPlan>(dist.Input);
    }

    [Fact]
    public void AggregateVariants_SumAvgMinMax_Collected()
    {
        var exprs = new[]
        {
            new SqlExpr.AggExpr(AggFunction.Sum, new AggArg.Expr(new SqlExpr.Column(null, "age")), false),
            new SqlExpr.AggExpr(AggFunction.Avg, new AggArg.Expr(new SqlExpr.Column(null, "age")), false),
            new SqlExpr.AggExpr(AggFunction.Min, new AggArg.Expr(new SqlExpr.Column(null, "age")), false),
            new SqlExpr.AggExpr(AggFunction.Max, new AggArg.Expr(new SqlExpr.Column(null, "age")), false),
        };

        var cols  = exprs.Select((e, i) => (OutputColumn)new OutputColumn.Expr(e, $"a{i}")).ToList();
        var stmt  = SimpleSelect(cols, "users");
        var plan  = Planner().Plan(stmt);

        var proj = Assert.IsType<ProjectPlan>(plan);
        var agg  = Assert.IsType<AggregatePlan>(proj.Input);
        Assert.Equal(4, agg.Aggregates.Count);
    }

    [Fact]
    public void CountDistinct_SetsDistinctFlag()
    {
        var aggExpr = new SqlExpr.AggExpr(AggFunction.Count,
            new AggArg.Expr(new SqlExpr.Column(null, "name")), true);
        var stmt = SimpleSelect(
            new[] { (OutputColumn)new OutputColumn.Expr(aggExpr, "cnt") }, "users");
        var plan = Planner().Plan(stmt);

        var proj = Assert.IsType<ProjectPlan>(plan);
        var agg  = Assert.IsType<AggregatePlan>(proj.Input);
        Assert.True(agg.Aggregates.Any(a => a.Distinct));
    }
}

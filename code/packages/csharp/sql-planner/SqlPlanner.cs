// SqlPlanner.cs — logical query plan builder for SQL statements.
//
// Transforms a Statement into a LogicalPlan tree using an 8-step bottom-up
// SELECT pipeline:
//
//   Scan → Filter → Aggregate → Having → Project → Distinct → Sort → Limit
//
// No I/O, no database access — pure in-memory data transformation.
// Errors are reported as PlanException subclasses (consistent with the
// C# sql-backend's exception-based error style).
//
// Usage:
//   var schema = new InMemorySchemaProvider(new Dictionary<string, IReadOnlyList<string>>
//   {
//       ["users"] = new[] { "id", "name", "age" }
//   });
//   var planner = new SqlPlanner(schema);
//   var plan = planner.Plan(stmt);   // throws PlanException on error

namespace CodingAdventures.SqlPlanner;

// ── Enumerations ─────────────────────────────────────────────────────────────

/// <summary>Binary infix SQL operators.</summary>
public enum BinaryOperator
{
    Eq, NotEq, Lt, Lte, Gt, Gte,
    And, Or,
    Add, Sub, Mul, Div, Mod,
}

/// <summary>Unary prefix SQL operators.</summary>
public enum UnaryOperator { Not, Neg }

/// <summary>Aggregate functions.</summary>
public enum AggFunction { Count, Sum, Avg, Min, Max }

/// <summary>Sort direction for ORDER BY.</summary>
public enum SortDir { Asc, Desc }

/// <summary>NULL placement in ORDER BY.</summary>
public enum NullOrder { NullsFirst, NullsLast }

/// <summary>JOIN type.</summary>
public enum JoinKind { Inner, Left, Right, Full, Cross }

// ── Aggregate argument ────────────────────────────────────────────────────────

/// <summary>Argument to an aggregate function — either * (COUNT(*)) or an expression.</summary>
public abstract record AggArg
{
    /// <summary>The * wildcard argument (COUNT(*)).</summary>
    public sealed record Star : AggArg;

    /// <summary>An expression argument (SUM(price), AVG(age), …).</summary>
    public sealed record Expr(SqlExpr Expression) : AggArg;
}

// ── Scalar expressions ────────────────────────────────────────────────────────

/// <summary>
/// A scalar expression in a SQL query plan.
/// Use the derived record types for each variant.
/// </summary>
public abstract record SqlExpr
{
    /// <summary>A SQL literal value (NULL, integer, real, text, boolean).</summary>
    public sealed record Literal(object? Value) : SqlExpr;

    /// <summary>A column reference, optionally qualified by table or alias.</summary>
    public sealed record Column(string? Table, string ColName) : SqlExpr;

    /// <summary>Binary operation (left op right).</summary>
    public sealed record BinaryOp(BinaryOperator Op, SqlExpr Left, SqlExpr Right) : SqlExpr;

    /// <summary>Unary prefix operation.</summary>
    public sealed record UnaryOp(UnaryOperator Op, SqlExpr Operand) : SqlExpr;

    /// <summary>Scalar function call.</summary>
    public sealed record FuncCall(string Name, IReadOnlyList<SqlExpr> Args) : SqlExpr;

    /// <summary>IS NULL predicate.</summary>
    public sealed record IsNull(SqlExpr Operand) : SqlExpr;

    /// <summary>IS NOT NULL predicate.</summary>
    public sealed record IsNotNull(SqlExpr Operand) : SqlExpr;

    /// <summary>BETWEEN low AND high (inclusive).</summary>
    public sealed record Between(SqlExpr Value, SqlExpr Low, SqlExpr High) : SqlExpr;

    /// <summary>value IN (items…).</summary>
    public sealed record In(SqlExpr Value, IReadOnlyList<SqlExpr> Items) : SqlExpr;

    /// <summary>value NOT IN (items…).</summary>
    public sealed record NotIn(SqlExpr Value, IReadOnlyList<SqlExpr> Items) : SqlExpr;

    /// <summary>String LIKE pattern.</summary>
    public sealed record Like(SqlExpr Value, string Pattern) : SqlExpr;

    /// <summary>String NOT LIKE pattern.</summary>
    public sealed record NotLike(SqlExpr Value, string Pattern) : SqlExpr;

    /// <summary>The * wildcard in SELECT *.</summary>
    public sealed record Wildcard : SqlExpr;

    /// <summary>Aggregate expression (COUNT, SUM, …).</summary>
    public sealed record AggExpr(AggFunction Func, AggArg Arg, bool Distinct) : SqlExpr;
}

// ── Output column ─────────────────────────────────────────────────────────────

/// <summary>One item in a SELECT list.</summary>
public abstract record OutputColumn
{
    /// <summary>The bare * wildcard (SELECT *).</summary>
    public sealed record Star : OutputColumn;

    /// <summary>A named expression with an optional alias.</summary>
    public sealed record Expr(SqlExpr Expression, string? Alias) : OutputColumn;
}

// ── Structural types ──────────────────────────────────────────────────────────

/// <summary>A JOIN clause.</summary>
public sealed record JoinClause(JoinKind Kind, string Table, string? Alias, SqlExpr? On);

/// <summary>DDL column definition.</summary>
public sealed record ColumnDef(
    string Name,
    string TypeName,
    bool NotNull    = false,
    bool PrimaryKey = false,
    bool Unique     = false,
    SqlExpr? Default = null);

/// <summary>A SET assignment in UPDATE.</summary>
public sealed record Assignment(string Column, SqlExpr Value);

/// <summary>LIMIT / OFFSET pair.</summary>
public sealed record LimitClause(long? Count, long? Offset);

/// <summary>One key in an ORDER BY clause.</summary>
public sealed record SortKey(SqlExpr KeyExpr, SortDir Direction, NullOrder NullOrder);

// ── Statement AST ─────────────────────────────────────────────────────────────
// The C# sql-parser is a stub, so this package defines its own statement types.

/// <summary>A SQL statement. Use the derived record types for each variant.</summary>
public abstract record Statement;

/// <summary>A SELECT statement.</summary>
public sealed record SelectStatement(
    bool Distinct,
    IReadOnlyList<OutputColumn> Columns,
    IReadOnlyList<(string Table, string? Alias)> From,
    IReadOnlyList<JoinClause> Joins,
    SqlExpr? Where,
    IReadOnlyList<SqlExpr> GroupBy,
    SqlExpr? Having,
    IReadOnlyList<SortKey> OrderBy,
    LimitClause? Limit) : Statement;

/// <summary>An INSERT … VALUES statement.</summary>
public sealed record InsertStatement(
    string Table,
    IReadOnlyList<string>? Columns,
    IReadOnlyList<IReadOnlyList<SqlExpr>> Values) : Statement;

/// <summary>An UPDATE statement.</summary>
public sealed record UpdateStatement(
    string Table,
    IReadOnlyList<Assignment> Assignments,
    SqlExpr? Where) : Statement;

/// <summary>A DELETE statement.</summary>
public sealed record DeleteStatement(string Table, SqlExpr? Where) : Statement;

/// <summary>CREATE TABLE DDL.</summary>
public sealed record CreateTableStatement(
    string Table,
    bool IfNotExists,
    IReadOnlyList<ColumnDef> Columns) : Statement;

/// <summary>DROP TABLE DDL.</summary>
public sealed record DropTableStatement(string Table, bool IfExists) : Statement;

// ── Aggregate item ────────────────────────────────────────────────────────────

/// <summary>One aggregate function applied during grouping.</summary>
public sealed record AggregateItem(AggFunction Func, AggArg Arg, string Alias, bool Distinct);

// ── Logical plan nodes ────────────────────────────────────────────────────────

/// <summary>A node in the logical query plan tree.</summary>
public abstract record LogicalPlan;

/// <summary>Full table scan.</summary>
public sealed record ScanPlan(string Table, string? Alias) : LogicalPlan;

/// <summary>Filter rows matching predicate.</summary>
public sealed record FilterPlan(LogicalPlan Input, SqlExpr Predicate) : LogicalPlan;

/// <summary>Project (compute) a list of output columns.</summary>
public sealed record ProjectPlan(LogicalPlan Input, IReadOnlyList<OutputColumn> Columns) : LogicalPlan;

/// <summary>Join two inputs.</summary>
public sealed record JoinPlan(LogicalPlan Left, LogicalPlan Right, JoinKind Kind, SqlExpr? Condition) : LogicalPlan;

/// <summary>Group and aggregate.</summary>
public sealed record AggregatePlan(LogicalPlan Input, IReadOnlyList<SqlExpr> GroupBy, IReadOnlyList<AggregateItem> Aggregates) : LogicalPlan;

/// <summary>Filter after grouping (HAVING).</summary>
public sealed record HavingPlan(LogicalPlan Input, SqlExpr Predicate) : LogicalPlan;

/// <summary>Sort the result.</summary>
public sealed record SortPlan(LogicalPlan Input, IReadOnlyList<SortKey> Keys) : LogicalPlan;

/// <summary>Limit and/or offset rows.</summary>
public sealed record LimitPlan(LogicalPlan Input, long? Count, long? Offset) : LogicalPlan;

/// <summary>Deduplicate rows (DISTINCT).</summary>
public sealed record DistinctPlan(LogicalPlan Input) : LogicalPlan;

/// <summary>Set UNION.</summary>
public sealed record UnionPlan(LogicalPlan Left, LogicalPlan Right, bool All) : LogicalPlan;

/// <summary>INSERT rows.</summary>
public sealed record InsertPlan(
    string Table,
    IReadOnlyList<string>? Columns,
    IReadOnlyList<IReadOnlyList<SqlExpr>> Values) : LogicalPlan;

/// <summary>UPDATE rows.</summary>
public sealed record UpdatePlan(string Table, IReadOnlyList<Assignment> Assignments, SqlExpr? Predicate) : LogicalPlan;

/// <summary>DELETE rows.</summary>
public sealed record DeletePlan(string Table, SqlExpr? Predicate) : LogicalPlan;

/// <summary>CREATE TABLE DDL.</summary>
public sealed record CreateTablePlan(string Table, bool IfNotExists, IReadOnlyList<ColumnDef> Columns) : LogicalPlan;

/// <summary>DROP TABLE DDL.</summary>
public sealed record DropTablePlan(string Table, bool IfExists) : LogicalPlan;

// ── Plan exceptions ───────────────────────────────────────────────────────────

/// <summary>Base class for all planning errors.</summary>
public abstract class PlanException : Exception
{
    protected PlanException(string message) : base(message) { }
}

/// <summary>A column name matches more than one table in scope.</summary>
public sealed class AmbiguousColumnException : PlanException
{
    /// <summary>The ambiguous column name.</summary>
    public string Column { get; }

    /// <summary>The tables that each own a column with this name.</summary>
    public IReadOnlyList<string> Tables { get; }

    public AmbiguousColumnException(string column, IReadOnlyList<string> tables)
        : base($"Ambiguous column '{column}' — found in: {string.Join(", ", tables)}")
    {
        Column = column;
        Tables = tables;
    }
}

/// <summary>A FROM / JOIN clause names a table that is not in the schema.</summary>
public sealed class UnknownTableException : PlanException
{
    /// <summary>The table name that could not be resolved.</summary>
    public string Table { get; }

    public UnknownTableException(string table)
        : base($"Unknown table '{table}'")
    {
        Table = table;
    }
}

/// <summary>An expression references a column that cannot be resolved in scope.</summary>
public sealed class UnknownColumnException : PlanException
{
    /// <summary>The optional table qualifier from the original expression.</summary>
    public string? QualifyingTable { get; }

    /// <summary>The column name that could not be resolved.</summary>
    public string Column { get; }

    public UnknownColumnException(string? table, string column)
        : base(table is null ? $"Unknown column '{column}'" : $"Unknown column '{table}.{column}'")
    {
        QualifyingTable = table;
        Column = column;
    }
}

/// <summary>An aggregate function appears in an illegal position.</summary>
public sealed class InvalidAggregateException : PlanException
{
    public InvalidAggregateException(string message) : base(message) { }
}

/// <summary>The statement type is not supported by this planner.</summary>
public sealed class UnsupportedStatementException : PlanException
{
    public UnsupportedStatementException(string kind) : base($"Unsupported statement: {kind}") { }
}

// ── Schema provider ───────────────────────────────────────────────────────────

/// <summary>Returns the ordered list of column names for a table.</summary>
public interface ISchemaProvider
{
    /// <summary>
    /// Returns the column names for <paramref name="table"/>.
    /// </summary>
    /// <exception cref="UnknownTableException">If the table is not in the schema.</exception>
    IReadOnlyList<string> Columns(string table);
}

/// <summary>An in-memory schema backed by a dictionary of table → column list.</summary>
public sealed class InMemorySchemaProvider : ISchemaProvider
{
    private readonly Dictionary<string, IReadOnlyList<string>> _tables;

    public InMemorySchemaProvider(Dictionary<string, IReadOnlyList<string>> tables)
        => _tables = tables;

    /// <inheritdoc/>
    public IReadOnlyList<string> Columns(string table)
        => _tables.TryGetValue(table, out var cols)
            ? cols
            : throw new UnknownTableException(table);
}

// ── Planner ───────────────────────────────────────────────────────────────────

/// <summary>
/// Transforms SQL statements into logical query plan trees.
/// Errors are reported as <see cref="PlanException"/> subclasses.
/// </summary>
public sealed class SqlPlanner
{
    private readonly ISchemaProvider _schema;

    public SqlPlanner(ISchemaProvider schema)
        => _schema = schema;

    // ── Scope entry ───────────────────────────────────────────────────────────

    private sealed class ScopeEntry
    {
        public string Alias { get; }
        public string Table { get; }
        public IReadOnlyList<string> Cols { get; }

        public ScopeEntry(string alias, string table, IReadOnlyList<string> cols)
        {
            Alias = alias;
            Table = table;
            Cols  = cols;
        }
    }

    // ── Scope building ────────────────────────────────────────────────────────

    private List<ScopeEntry> BuildScope(
        IReadOnlyList<(string Table, string? Alias)> from,
        IReadOnlyList<JoinClause> joins)
    {
        var scope = new List<ScopeEntry>();

        foreach (var (tbl, aliasOpt) in from)
        {
            var cols  = _schema.Columns(tbl);          // throws UnknownTableException
            scope.Add(new ScopeEntry(aliasOpt ?? tbl, tbl, cols));
        }

        foreach (var j in joins)
        {
            var cols  = _schema.Columns(j.Table);      // throws UnknownTableException
            scope.Add(new ScopeEntry(j.Alias ?? j.Table, j.Table, cols));
        }

        return scope;
    }

    // ── Column resolution ─────────────────────────────────────────────────────

    private static SqlExpr ResolveColumn(List<ScopeEntry> scope, string? tableOpt, string col)
    {
        if (tableOpt is not null)
        {
            var entry = scope.Find(e => e.Alias == tableOpt)
                ?? throw new UnknownTableException(tableOpt);

            if (!entry.Cols.Any(c => string.Equals(c, col, StringComparison.OrdinalIgnoreCase)))
                throw new UnknownColumnException(tableOpt, col);

            return new SqlExpr.Column(entry.Alias, col);
        }
        else
        {
            var matches = scope
                .Where(e => e.Cols.Any(c => string.Equals(c, col, StringComparison.OrdinalIgnoreCase)))
                .ToList();

            return matches.Count switch
            {
                0 => throw new UnknownColumnException(null, col),
                1 => new SqlExpr.Column(matches[0].Alias, col),
                _ => throw new AmbiguousColumnException(col, matches.Select(e => e.Alias).ToList()),
            };
        }
    }

    // ── Expression resolution ─────────────────────────────────────────────────

    private static SqlExpr ResolveExpr(List<ScopeEntry> scope, SqlExpr expr)
    {
        return expr switch
        {
            SqlExpr.Column(var tbl, var col) => ResolveColumn(scope, tbl, col),
            SqlExpr.Literal _    => expr,
            SqlExpr.Wildcard _   => expr,
            SqlExpr.AggExpr _    => expr,
            SqlExpr.BinaryOp(var op, var l, var r) =>
                new SqlExpr.BinaryOp(op, ResolveExpr(scope, l), ResolveExpr(scope, r)),
            SqlExpr.UnaryOp(var op, var operand) =>
                new SqlExpr.UnaryOp(op, ResolveExpr(scope, operand)),
            SqlExpr.FuncCall(var name, var args) =>
                new SqlExpr.FuncCall(name, args.Select(a => ResolveExpr(scope, a)).ToList()),
            SqlExpr.IsNull(var operand) =>
                new SqlExpr.IsNull(ResolveExpr(scope, operand)),
            SqlExpr.IsNotNull(var operand) =>
                new SqlExpr.IsNotNull(ResolveExpr(scope, operand)),
            SqlExpr.Between(var v, var lo, var hi) =>
                new SqlExpr.Between(ResolveExpr(scope, v), ResolveExpr(scope, lo), ResolveExpr(scope, hi)),
            SqlExpr.In(var v, var items) =>
                new SqlExpr.In(ResolveExpr(scope, v), items.Select(i => ResolveExpr(scope, i)).ToList()),
            SqlExpr.NotIn(var v, var items) =>
                new SqlExpr.NotIn(ResolveExpr(scope, v), items.Select(i => ResolveExpr(scope, i)).ToList()),
            SqlExpr.Like(var v, var pattern) =>
                new SqlExpr.Like(ResolveExpr(scope, v), pattern),
            SqlExpr.NotLike(var v, var pattern) =>
                new SqlExpr.NotLike(ResolveExpr(scope, v), pattern),
            _ => throw new InvalidOperationException($"Unexpected expression type: {expr.GetType().Name}"),
        };
    }

    // Resolve, returning null instead of throwing UnknownColumnException.
    private static SqlExpr? TryResolveExpr(List<ScopeEntry> scope, SqlExpr expr)
    {
        try  { return ResolveExpr(scope, expr); }
        catch (UnknownColumnException) { return null; }
    }

    // ── Aggregate detection / collection ──────────────────────────────────────

    private static bool ContainsAgg(SqlExpr expr)
    {
        return expr switch
        {
            SqlExpr.AggExpr _                   => true,
            SqlExpr.BinaryOp(_, var l, var r)   => ContainsAgg(l) || ContainsAgg(r),
            SqlExpr.UnaryOp(_, var op)          => ContainsAgg(op),
            SqlExpr.FuncCall(_, var args)        => args.Any(ContainsAgg),
            SqlExpr.IsNull(var op)              => ContainsAgg(op),
            SqlExpr.IsNotNull(var op)           => ContainsAgg(op),
            SqlExpr.Between(var v, var lo, var hi) => ContainsAgg(v) || ContainsAgg(lo) || ContainsAgg(hi),
            SqlExpr.In(var v, var items)         => ContainsAgg(v) || items.Any(ContainsAgg),
            SqlExpr.NotIn(var v, var items)      => ContainsAgg(v) || items.Any(ContainsAgg),
            SqlExpr.Like(var v, _)              => ContainsAgg(v),
            SqlExpr.NotLike(var v, _)           => ContainsAgg(v),
            _                                   => false,
        };
    }

    private static List<AggregateItem> CollectAggregates(IEnumerable<SqlExpr> exprs)
    {
        var found   = new List<AggregateItem>();
        var counter = 0;

        void Walk(SqlExpr e)
        {
            switch (e)
            {
                case SqlExpr.AggExpr(var func, var arg, var distinct):
                    found.Add(new AggregateItem(func, arg, $"_agg{counter++}", distinct));
                    break;
                case SqlExpr.BinaryOp(_, var l, var r): Walk(l); Walk(r); break;
                case SqlExpr.UnaryOp(_, var op):         Walk(op);         break;
                case SqlExpr.FuncCall(_, var args):      foreach (var a in args) Walk(a); break;
                case SqlExpr.IsNull(var op):             Walk(op);         break;
                case SqlExpr.IsNotNull(var op):          Walk(op);         break;
                case SqlExpr.Between(var v, var lo, var hi): Walk(v); Walk(lo); Walk(hi); break;
                case SqlExpr.In(var v, var items):       Walk(v); foreach (var i in items) Walk(i); break;
                case SqlExpr.NotIn(var v, var items):    Walk(v); foreach (var i in items) Walk(i); break;
                case SqlExpr.Like(var v, _):             Walk(v); break;
                case SqlExpr.NotLike(var v, _):          Walk(v); break;
            }
        }

        foreach (var expr in exprs) Walk(expr);
        return found;
    }

    // ── FROM tree construction ────────────────────────────────────────────────

    private LogicalPlan BuildFromTree(
        IReadOnlyList<(string Table, string? Alias)> from,
        IReadOnlyList<JoinClause> joins)
    {
        if (from.Count == 0)
            throw new UnsupportedStatementException("SELECT without FROM");

        var (tbl0, alias0) = from[0];
        _schema.Columns(tbl0);   // validate table exists
        LogicalPlan plan = new ScanPlan(tbl0, alias0);

        for (var i = 1; i < from.Count; i++)
        {
            var (tbl, aliasOpt) = from[i];
            _schema.Columns(tbl);  // validate table exists
            plan = new JoinPlan(plan, new ScanPlan(tbl, aliasOpt), JoinKind.Cross, null);
        }

        foreach (var j in joins)
        {
            _schema.Columns(j.Table);  // validate table exists
            plan = new JoinPlan(plan, new ScanPlan(j.Table, j.Alias), j.Kind, j.On);
        }

        return plan;
    }

    // ── SELECT planner (8-step bottom-up pipeline) ────────────────────────────

    private LogicalPlan PlanSelect(SelectStatement s)
    {
        // Build scope first (validates all table references).
        var scope    = BuildScope(s.From, s.Joins);
        var fromPlan = BuildFromTree(s.From, s.Joins);

        // Step 1: WHERE → Filter
        var plan = s.Where is null
            ? fromPlan
            : (LogicalPlan)new FilterPlan(fromPlan, ResolveExpr(scope, s.Where));

        // Determine whether aggregation is required.
        var colExprs    = s.Columns.Select(c => c is OutputColumn.Expr(var e, _) ? e : new SqlExpr.Wildcard()).ToList();
        var havingExprs = s.Having is null ? Enumerable.Empty<SqlExpr>() : new[] { s.Having };

        var needsAgg = s.GroupBy.Count > 0
            || colExprs.Any(ContainsAgg)
            || havingExprs.Any(ContainsAgg);

        // Step 2: GROUP BY + Aggregate
        if (needsAgg)
        {
            var aggs = CollectAggregates(colExprs.Concat(havingExprs));
            var resolvedGroupBy = s.GroupBy.Select(e => ResolveExpr(scope, e)).ToList();
            plan = new AggregatePlan(plan, resolvedGroupBy, aggs);
        }

        // Step 3: HAVING
        if (s.Having is not null)
        {
            var rHaving = TryResolveExpr(scope, s.Having) ?? s.Having;
            plan = new HavingPlan(plan, rHaving);
        }

        // Step 4: PROJECT
        var projectedCols = s.Columns.Select(c =>
        {
            if (c is OutputColumn.Star)
                return (OutputColumn)new OutputColumn.Star();
            var ec = (OutputColumn.Expr)c;
            var resolved = needsAgg
                ? TryResolveExpr(scope, ec.Expression) ?? ec.Expression
                : ResolveExpr(scope, ec.Expression);
            return (OutputColumn)new OutputColumn.Expr(resolved, ec.Alias);
        }).ToList();
        plan = new ProjectPlan(plan, projectedCols);

        // Step 5: DISTINCT
        if (s.Distinct)
            plan = new DistinctPlan(plan);

        // Step 6: ORDER BY
        if (s.OrderBy.Count > 0)
        {
            var resolvedKeys = s.OrderBy.Select(key =>
            {
                var resolved = TryResolveExpr(scope, key.KeyExpr) ?? key.KeyExpr;
                return key with { KeyExpr = resolved };
            }).ToList();
            plan = new SortPlan(plan, resolvedKeys);
        }

        // Step 7: LIMIT / OFFSET
        if (s.Limit is not null)
            plan = new LimitPlan(plan, s.Limit.Count, s.Limit.Offset);

        return plan;
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// <summary>
    /// Transform a single statement into a logical plan.
    /// </summary>
    /// <exception cref="PlanException">On any planning error.</exception>
    public LogicalPlan Plan(Statement stmt)
    {
        return stmt switch
        {
            SelectStatement s =>
                PlanSelect(s),

            InsertStatement i =>
                (_schema.Columns(i.Table),   // validate table
                 (LogicalPlan)new InsertPlan(i.Table, i.Columns, i.Values)).Item2,

            UpdateStatement u =>
                (_schema.Columns(u.Table),
                 (LogicalPlan)new UpdatePlan(u.Table, u.Assignments, u.Where)).Item2,

            DeleteStatement d =>
                (_schema.Columns(d.Table),
                 (LogicalPlan)new DeletePlan(d.Table, d.Where)).Item2,

            CreateTableStatement ct =>
                new CreateTablePlan(ct.Table, ct.IfNotExists, ct.Columns),

            DropTableStatement dt =>
                new DropTablePlan(dt.Table, dt.IfExists),

            _ => throw new UnsupportedStatementException(stmt.GetType().Name),
        };
    }

    /// <summary>
    /// Plan every statement in the sequence.
    /// </summary>
    /// <exception cref="PlanException">On the first planning error encountered.</exception>
    public IReadOnlyList<LogicalPlan> PlanAll(IEnumerable<Statement> stmts)
        => stmts.Select(Plan).ToList();
}

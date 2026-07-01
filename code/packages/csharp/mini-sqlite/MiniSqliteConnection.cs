// MiniSqliteConnection.cs — Level 1 pipeline facade for mini-sqlite.
//
// This module wires the full Level 1 pipeline:
//
//   SQL text  →  Statement AST  →  LogicalPlan  →  OptimizedPlan
//       →  Program (bytecode)  →  QueryResult (via SqlVm)
//
// The pipeline packages it depends on:
//   • sql-backend    – InMemoryBackend, BackendAdapters
//   • sql-planner    – SqlPlanner, ISchemaProvider, Statement types
//   • sql-optimizer  – SqlOptimizer.Optimize
//   • sql-codegen    – SqlCodegen.Compile
//   • sql-vm         – SqlVm.Execute, QueryResult
//
// PUBLIC API
// ──────────
//   var conn = new MiniSqliteConnection();
//   conn.Execute("CREATE TABLE users (id INT, name TEXT)");
//   conn.Execute("INSERT INTO users VALUES (1, 'Alice')");
//   QueryResult r = conn.Execute("SELECT * FROM users WHERE id > 0");
//   // r.Columns → ["id", "name"]
//   // r.Rows[0] → [1L, "Alice"]
//
// PARAMETER BINDING
// ─────────────────
// Positional ? parameters are bound to C# values before parsing:
//   conn.Execute("SELECT * FROM users WHERE id = ?", new object?[] { 1 });
// NULL, bool, long, int, double, and string are the supported parameter types.
//
// TEXT-TO-STATEMENT BRIDGE
// ─────────────────────────
// The C# sql-parser package is currently a placeholder stub, so this module
// contains its own hand-rolled SQL parser that emits planner Statement objects.
// The parser handles the full Level 1 SQL surface:
//   SELECT … FROM … [JOIN] [WHERE] [GROUP BY] [HAVING] [ORDER BY] [LIMIT/OFFSET]
//   INSERT INTO … VALUES …
//   UPDATE … SET … [WHERE]
//   DELETE FROM … [WHERE]
//   CREATE TABLE … (…)
//   DROP TABLE …
//
// This is deliberately literate code — every non-obvious parsing step is
// annotated with the SQL grammar fragment it implements.

using System.Globalization;
using System.Text.RegularExpressions;
using CodingAdventures.SqlBackend;
using Optimizer = CodingAdventures.SqlOptimizer.SqlOptimizer;
using SqlVmNs = CodingAdventures.SqlVm;

// Planner class — use fully qualified name to disambiguate from the namespace.
using PlSqlPlanner = CodingAdventures.SqlPlanner.SqlPlanner;
using PlSchemaProvider = CodingAdventures.SqlPlanner.ISchemaProvider;

// Codegen class — use fully qualified name to avoid namespace collision.
using PlSqlCodegen = CodingAdventures.SqlCodegen.SqlCodegen;

// Bring planner types into scope without prefix so the parser reads naturally.
using PlStatement = CodingAdventures.SqlPlanner.Statement;
using PlColumnDef = CodingAdventures.SqlPlanner.ColumnDef;
using PlAssignment = CodingAdventures.SqlPlanner.Assignment;
using PlSortKey = CodingAdventures.SqlPlanner.SortKey;
using PlLimitClause = CodingAdventures.SqlPlanner.LimitClause;
using PlOutputColumn = CodingAdventures.SqlPlanner.OutputColumn;
using PlJoinClause = CodingAdventures.SqlPlanner.JoinClause;
using PlSqlExpr = CodingAdventures.SqlPlanner.SqlExpr;
using PlAggArg = CodingAdventures.SqlPlanner.AggArg;

// Planner enum types — imported as type aliases so the parser body doesn't need
// the CodingAdventures.SqlPlanner prefix everywhere.
using BinaryOperator = CodingAdventures.SqlPlanner.BinaryOperator;
using UnaryOperator  = CodingAdventures.SqlPlanner.UnaryOperator;
using AggFunction    = CodingAdventures.SqlPlanner.AggFunction;
using SortDir        = CodingAdventures.SqlPlanner.SortDir;
using NullOrder      = CodingAdventures.SqlPlanner.NullOrder;
using JoinKind       = CodingAdventures.SqlPlanner.JoinKind;

// Planner concrete statement types.
using PlSelectStatement      = CodingAdventures.SqlPlanner.SelectStatement;
using PlInsertStatement      = CodingAdventures.SqlPlanner.InsertStatement;
using PlUpdateStatement      = CodingAdventures.SqlPlanner.UpdateStatement;
using PlDeleteStatement      = CodingAdventures.SqlPlanner.DeleteStatement;
using PlCreateTableStatement = CodingAdventures.SqlPlanner.CreateTableStatement;
using PlDropTableStatement   = CodingAdventures.SqlPlanner.DropTableStatement;

// Planner exception types.
using PlUnknownTableException    = CodingAdventures.SqlPlanner.UnknownTableException;
using PlUnknownColumnException   = CodingAdventures.SqlPlanner.UnknownColumnException;
using PlAmbiguousColumnException = CodingAdventures.SqlPlanner.AmbiguousColumnException;
using PlPlanException            = CodingAdventures.SqlPlanner.PlanException;

namespace CodingAdventures.MiniSqlite;

// ── Public result type ─────────────────────────────────────────────────────────

/// <summary>
/// The result of executing a SQL statement through the Level 1 pipeline.
///
/// <list type="bullet">
///   <item><term>Columns</term><description>Ordered output column names (empty for DML/DDL).</description></item>
///   <item><term>Rows</term><description>Result rows; each row is a positional list of SQL values (C# object? or null).</description></item>
///   <item><term>RowsAffected</term><description>Rows inserted/updated/deleted (0 for SELECT/DDL, -1 when not applicable).</description></item>
/// </list>
/// </summary>
public sealed record QueryResult(
    IReadOnlyList<string> Columns,
    IReadOnlyList<IReadOnlyList<object?>> Rows,
    int RowsAffected);

// ── Top-level connection ───────────────────────────────────────────────────────

/// <summary>
/// A Level 1 in-memory SQL connection that runs every statement through the
/// full sql-backend → sql-planner → sql-optimizer → sql-codegen → sql-vm pipeline.
///
/// <para>Usage:</para>
/// <code>
/// var conn = new MiniSqliteConnection();
/// conn.Execute("CREATE TABLE t (id INT, name TEXT)");
/// conn.Execute("INSERT INTO t VALUES (1, 'Alice')");
/// QueryResult r = conn.Execute("SELECT * FROM t");
/// </code>
///
/// Parameter binding with positional '?' placeholders:
/// <code>
/// conn.Execute("INSERT INTO t VALUES (?, ?)", new object?[] { 2, "Bob" });
/// conn.Execute("SELECT * FROM t WHERE id = ?", new object?[] { 1 });
/// </code>
/// </summary>
public sealed class MiniSqliteConnection
{
    // The in-memory backend stores tables, rows, indexes, and handles
    // positioned-update/delete via ListCursor.
    private readonly InMemoryBackend _backend = new();

    // ── Transaction state ──────────────────────────────────────────────────────

    // We track an active transaction handle so that Commit() and Rollback()
    // can be called explicitly between Execute() calls.
    private CodingAdventures.SqlBackend.TransactionHandle? _txHandle;

    // ── Public Execute API ─────────────────────────────────────────────────────

    /// <summary>
    /// Execute a SQL statement and return the result.
    ///
    /// <para>
    /// For SELECT: result contains Columns and Rows.
    /// For DML: result contains RowsAffected (Columns/Rows are empty).
    /// For DDL: result has all fields zero/empty.
    /// </para>
    /// </summary>
    /// <param name="sql">The SQL text to execute.</param>
    /// <param name="parameters">Optional positional '?' parameters.</param>
    /// <exception cref="MiniSqliteException">On any SQL error.</exception>
    public QueryResult Execute(string sql, params object?[] parameters)
        => Execute(sql, (IReadOnlyList<object?>)parameters);

    /// <summary>Execute with a typed parameter list.</summary>
    public QueryResult Execute(string sql, IReadOnlyList<object?> parameters)
    {
        try
        {
            // Step 1: bind '?' parameters into the SQL text so the rest of the
            // pipeline works on a fully-resolved statement.
            var bound = BindParameters(sql, parameters);

            // Step 2: parse the bound SQL text into a planner Statement.
            var stmt = SqlStatementParser.Parse(bound);

            // Step 2b: Expand column-list-free INSERTs.
            //
            // The SQL standard allows INSERT INTO t VALUES (…) with no explicit
            // column list — values are assigned positionally to all table columns
            // in their declaration order.  The codegen (via OptInsert) receives
            // the column list from the planner; when it is null/empty it emits
            // InsertRow with an empty column array, and the VM's DoInsert then
            // pops 0 values — so nothing is actually inserted.
            //
            // The fix: after parsing, when the parsed statement is an INSERT with
            // no column list, look up the current table schema and substitute the
            // full ordered column list.  This expands "INSERT INTO t VALUES (1,2)"
            // into the equivalent of "INSERT INTO t (col1, col2) VALUES (1, 2)".
            if (stmt is PlInsertStatement ins && ins.Columns is null)
            {
                IReadOnlyList<string> expandedCols;
                try
                {
                    expandedCols = _backend.Columns(ins.Table).Select(c => c.Name).ToArray();
                }
                catch (CodingAdventures.SqlBackend.TableNotFound ex)
                {
                    // Let the planner throw the proper typed exception.
                    throw new PlUnknownTableException(ex.Table);
                }
                stmt = new PlInsertStatement(ins.Table, expandedCols, ins.Values);
            }

            // Step 2c: Handle SELECT without FROM (scalar "from dual" queries).
            //
            // The planner rejects SELECT without a FROM clause with
            // UnsupportedStatementException("SELECT without FROM").  SQLite allows
            // SELECT <expr> without FROM as a way to evaluate scalar expressions:
            //   SELECT LENGTH('hello')  →  [5]
            //   SELECT UPPER('hi'), 42  →  ['HI', 42]
            //
            // We short-circuit before the planner by detecting this pattern and
            // evaluating each projection column as a constant expression.  Only
            // bare constant expressions (literals, arithmetic on literals, built-in
            // scalar function calls on literals) are supported — any column
            // reference would be meaningless without a FROM table anyway.
            if (stmt is PlSelectStatement sel && sel.From.Count == 0
                && sel.Joins.Count == 0
                && sel.Where is null
                && sel.GroupBy.Count == 0
                && sel.Having is null)
            {
                return EvalScalarSelect(sel);
            }

            // Step 2d: Expand SELECT * into explicit column lists.
            //
            // The codegen emits a null placeholder for OutputColumn.Star and the VM
            // doesn't expand star to real column names — it just emits a column
            // called "*" with value null.  We expand SELECT * into explicit named
            // columns by looking up the schema here, before the planner runs.
            //
            // When multiple FROM tables are involved (cross-join or FROM t1, t2),
            // we enumerate each table's columns in FROM-list order.
            if (stmt is PlSelectStatement selStar
                && selStar.Columns.Any(c => c is CodingAdventures.SqlPlanner.OutputColumn.Star))
            {
                stmt = ExpandStarColumns(selStar);
            }

            // Step 2e: Hoist ORDER BY columns into the projection.
            //
            // The codegen emits SortPlan AFTER ProjectPlan, so sort keys that
            // reference columns not in the SELECT output are not visible to the
            // VM's ApplySort (it resolves by column name in the result schema).
            // Fix: for non-aggregate SELECTs with ORDER BY, temporarily add any
            // missing sort-key columns to the output, run the pipeline, then
            // strip those extra columns from the result.
            //
            // We track which columns were added so we can strip them after.
            int extraSortCols = 0;
            if (stmt is PlSelectStatement selOrd && selOrd.OrderBy.Count > 0
                && selOrd.GroupBy.Count == 0
                && selOrd.From.Count > 0
                && !selOrd.Columns.Any(c => c is CodingAdventures.SqlPlanner.OutputColumn.Star))
            {
                var result2 = AddMissingSortColumns(selOrd);
                stmt = result2.stmt;
                extraSortCols = result2.extraCols;
            }

            // Step 3: wire the InMemoryBackend as a live schema provider so the
            // planner can resolve column names against the current table catalog.
            // The planner uses its own ISchemaProvider interface (not the backend's),
            // so we wrap the backend in a BackendSchemaAdapter that bridges the two.
            var schema = new BackendSchemaAdapter(_backend);

            // Step 4: plan → optimize → compile → execute.
            // SqlCodegen.Compile already calls Optimizer.Optimize internally,
            // so we go: Parse → Plan → Compile (which optimizes) → Execute.
            var planner  = new PlSqlPlanner(schema);
            var logical  = planner.Plan(stmt);
            var program  = PlSqlCodegen.Compile(logical);
            var vmResult = SqlVmNs.SqlVm.Execute(program, _backend);

            // Step 5: translate the VM's QueryResult into our public QueryResult.
            // If we hoisted extra sort-key columns in step 2e, strip them now.
            var resultCols = vmResult.Columns;
            var resultRows = vmResult.Rows;
            if (extraSortCols > 0 && vmResult.Columns.Count > extraSortCols)
            {
                var keep = vmResult.Columns.Count - extraSortCols;
                resultCols = vmResult.Columns.Take(keep).ToArray();
                resultRows = vmResult.Rows.Select(r => (IReadOnlyList<object?>)r.Take(keep).ToArray()).ToList();
            }

            return new QueryResult(
                resultCols,
                resultRows,
                vmResult.RowsAffected);
        }
        catch (MiniSqliteException)
        {
            throw;   // already wrapped, pass through
        }
        catch (PlUnknownTableException ex)
        {
            throw new MiniSqliteException("OperationalError", ex.Message);
        }
        catch (PlUnknownColumnException ex)
        {
            throw new MiniSqliteException("OperationalError", ex.Message);
        }
        catch (PlAmbiguousColumnException ex)
        {
            throw new MiniSqliteException("OperationalError", ex.Message);
        }
        catch (PlPlanException ex)
        {
            throw new MiniSqliteException("OperationalError", ex.Message);
        }
        catch (BackendError ex)
        {
            throw new MiniSqliteException("OperationalError", ex.Message);
        }
        catch (SqlVmNs.VmError ex)
        {
            throw new MiniSqliteException("OperationalError", ex.Message);
        }
        catch (ParseException ex)
        {
            throw new MiniSqliteException("ProgrammingError", ex.Message);
        }
        catch (Exception ex)
        {
            throw new MiniSqliteException("OperationalError", ex.Message);
        }
    }

    // ── ORDER BY column hoisting ──────────────────────────────────────────────────
    //
    // The planner wraps ProjectPlan with SortPlan (sort comes after project), so
    // ORDER BY columns not in the SELECT list are invisible to the VM sorter.
    // We fix this by adding missing ORDER BY key columns as trailing output columns,
    // running the query through the pipeline, then stripping those extra columns.
    //
    // Returns the (possibly rewritten) statement and how many extra columns were added.

    private (PlSelectStatement stmt, int extraCols) AddMissingSortColumns(PlSelectStatement sel)
    {
        // Collect names of columns that are already projected.
        var projectedNames = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var col in sel.Columns)
        {
            if (col is CodingAdventures.SqlPlanner.OutputColumn.Expr exprCol)
            {
                if (exprCol.Alias is not null)
                    projectedNames.Add(exprCol.Alias);
                else if (exprCol.Expression is PlSqlExpr.Column(_, var cname))
                    projectedNames.Add(cname);
            }
        }

        // Find ORDER BY keys referencing bare column names not already projected.
        var extra = new List<CodingAdventures.SqlPlanner.OutputColumn.Expr>();
        foreach (var key in sel.OrderBy)
        {
            if (key.KeyExpr is PlSqlExpr.Column(var tbl, var colName)
                && !projectedNames.Contains(colName))
            {
                extra.Add(new CodingAdventures.SqlPlanner.OutputColumn.Expr(
                    new PlSqlExpr.Column(tbl, colName), null));
                projectedNames.Add(colName); // don't add the same column twice
            }
        }

        if (extra.Count == 0)
            return (sel, 0);

        var newCols = sel.Columns.Concat(extra).ToList();
        return (sel with { Columns = newCols }, extra.Count);
    }

    // ── SELECT * expansion ────────────────────────────────────────────────────────
    //
    // The codegen and VM don't expand SELECT * — the codegen emits LoadConst(null) +
    // EmitColumn("*") as a placeholder, which causes the result to contain a "*" column
    // with null values.  We fix this by rewriting the SelectStatement before planning:
    // replace each OutputColumn.Star with an explicit Expr per schema column.
    //
    // Expansion order follows the FROM-list order.  A star in a multi-table FROM
    // (FROM t1, t2) expands to all columns of t1, then all columns of t2.

    private PlSelectStatement ExpandStarColumns(PlSelectStatement sel)
    {
        // Collect all schema columns from the FROM tables (in order).
        // For SELECT t.* (table-qualified star) the star expands only that table's cols.
        // Our parser emits bare OutputColumn.Star for *, so we expand all tables.
        var allCols = new List<CodingAdventures.SqlPlanner.OutputColumn>();
        foreach (var col in sel.Columns)
        {
            if (col is CodingAdventures.SqlPlanner.OutputColumn.Star)
            {
                // Expand star to all columns of all FROM/JOIN tables.
                var expanded = GetAllTableColumns(sel);
                allCols.AddRange(expanded);
            }
            else
            {
                allCols.Add(col);
            }
        }
        return sel with { Columns = allCols };
    }

    // Build the list of OutputColumn.Expr for every column in all FROM/JOIN tables.
    private IReadOnlyList<CodingAdventures.SqlPlanner.OutputColumn.Expr> GetAllTableColumns(PlSelectStatement sel)
    {
        var result = new List<CodingAdventures.SqlPlanner.OutputColumn.Expr>();

        // Tables from the main FROM list.
        foreach (var (tbl, alias) in sel.From)
        {
            IReadOnlyList<string> cols;
            try { cols = _backend.Columns(tbl).Select(c => c.Name).ToArray(); }
            catch (CodingAdventures.SqlBackend.TableNotFound ex)
            { throw new PlUnknownTableException(ex.Table); }

            var qualifier = alias ?? tbl;
            foreach (var c in cols)
                result.Add(new CodingAdventures.SqlPlanner.OutputColumn.Expr(
                    new PlSqlExpr.Column(qualifier, c), null));
        }

        // Tables from JOIN clauses.
        foreach (var join in sel.Joins)
        {
            IReadOnlyList<string> cols;
            try { cols = _backend.Columns(join.Table).Select(c => c.Name).ToArray(); }
            catch (CodingAdventures.SqlBackend.TableNotFound ex)
            { throw new PlUnknownTableException(ex.Table); }

            var qualifier = join.Alias ?? join.Table;
            foreach (var c in cols)
                result.Add(new CodingAdventures.SqlPlanner.OutputColumn.Expr(
                    new PlSqlExpr.Column(qualifier, c), null));
        }

        return result;
    }

    // ── Scalar SELECT (FROM-less) evaluation ──────────────────────────────────────
    //
    // SELECT <expr>, ... without a FROM clause evaluates each projection expression
    // as a constant and returns exactly one row.  This is the "from dual" shorthand
    // supported by SQLite, MySQL, and others.  We evaluate expressions here rather
    // than in the VM pipeline because the planner rejects SELECT without FROM.
    //
    // Only constant-foldable expressions are meaningful without a table:
    // literals, arithmetic on literals, and calls to built-in scalar functions.
    // Column references would refer to nothing and will throw at eval time.

    private QueryResult EvalScalarSelect(PlSelectStatement sel)
    {
        // Evaluate each projection column expression in a context with no current row.
        // OutputColumn is an abstract record with two concrete subtypes:
        //   OutputColumn.Star  — SELECT *  (illegal without FROM; skip/error)
        //   OutputColumn.Expr  — SELECT <expr> [AS alias]
        var cols  = new List<string>();
        var vals  = new List<object?>();

        foreach (var col in sel.Columns)
        {
            if (col is CodingAdventures.SqlPlanner.OutputColumn.Star)
                throw new MiniSqliteException("OperationalError",
                    "SELECT * requires a FROM clause");

            var exprCol = (CodingAdventures.SqlPlanner.OutputColumn.Expr)col;
            var expr    = exprCol.Expression;
            var alias   = exprCol.Alias;

            var name = alias
                ?? (expr is PlSqlExpr.Column(_, var cname)    ? cname
                    : expr is PlSqlExpr.FuncCall(var fn, _) ? fn.ToLowerInvariant()
                    : "expr");

            cols.Add(name);
            vals.Add(EvalConstExpr(expr));
        }

        var rowList = new List<object?>(vals);
        return new QueryResult(
            cols,
            new List<IReadOnlyList<object?>> { rowList },
            0);
    }

    // Evaluate a constant SQL expression that requires no row context.
    // Supports: Literal (including Literal(null) for SQL NULL),
    //           UnaryOp(Neg/Not), BinaryOp, FuncCall.
    // Column references throw — they require a row context from a FROM table.
    private static object? EvalConstExpr(PlSqlExpr expr) => expr switch
    {
        // ── Literals — Literal(null) represents SQL NULL ──────────────────────
        PlSqlExpr.Literal(var v)               => v,

        // ── Unary operators ───────────────────────────────────────────────────
        PlSqlExpr.UnaryOp(UnaryOperator.Neg, var operand) =>
            ToDouble(EvalConstExpr(operand)) is double d ? -d : null,
        PlSqlExpr.UnaryOp(UnaryOperator.Not, var operand) =>
            EvalConstExpr(operand) is null ? null
            : (object?)(IsTruthy(EvalConstExpr(operand)) ? (long)0 : (long)1),

        // ── Binary operators ──────────────────────────────────────────────────
        PlSqlExpr.BinaryOp(var op, var left, var right) =>
            EvalBinaryOp(op, EvalConstExpr(left), EvalConstExpr(right)),

        // ── Scalar function calls ─────────────────────────────────────────────
        // FuncCall.Args is IReadOnlyList<SqlExpr> — evaluate each arg recursively.
        PlSqlExpr.FuncCall(var name, var args) =>
            EvalBuiltinScalar(name, args.Select(a => EvalConstExpr(a)).ToArray()),

        // ── Anything else — cannot evaluate without a row context ─────────────
        _ => throw new MiniSqliteException("OperationalError",
                 $"Cannot evaluate expression '{expr.GetType().Name}' without a FROM clause"),
    };

    private static bool IsTruthy(object? v) =>
        v is not null && (v is not long l || l != 0);

    private static double? ToDouble(object? v) => v switch
    {
        long l   => (double)l,
        double d => d,
        string s => double.TryParse(s, System.Globalization.NumberStyles.Float,
                        System.Globalization.CultureInfo.InvariantCulture, out var r) ? r : null,
        _ => null,
    };

    private static object? EvalBinaryOp(BinaryOperator op, object? left, object? right)
    {
        // NULL propagates through all operators (three-valued logic).
        if (left is null || right is null)
            return null;

        switch (op)
        {
            case BinaryOperator.Add:
            case BinaryOperator.Sub:
            case BinaryOperator.Mul:
            case BinaryOperator.Div:
            case BinaryOperator.Mod:
            {
                double l = ToDouble(left) ?? 0, r = ToDouble(right) ?? 0;
                double result = op switch
                {
                    BinaryOperator.Add => l + r,
                    BinaryOperator.Sub => l - r,
                    BinaryOperator.Mul => l * r,
                    BinaryOperator.Div => r == 0 ? double.NaN : l / r,
                    BinaryOperator.Mod => r == 0 ? double.NaN : l % r,
                    _ => 0,
                };
                // Return integer when both inputs are integer and result is whole.
                if (left is long && right is long && result == Math.Floor(result)
                    && op != BinaryOperator.Div)
                    return (long)result;
                return result;
            }
            case BinaryOperator.Eq:    return SqlEq(left, right) ? 1L : 0L;
            case BinaryOperator.NotEq: return SqlEq(left, right) ? 0L : 1L;
            case BinaryOperator.Lt:    return SqlCmp(left, right) < 0  ? 1L : 0L;
            case BinaryOperator.Lte:   return SqlCmp(left, right) <= 0 ? 1L : 0L;
            case BinaryOperator.Gt:    return SqlCmp(left, right) > 0  ? 1L : 0L;
            case BinaryOperator.Gte:   return SqlCmp(left, right) >= 0 ? 1L : 0L;
            case BinaryOperator.And:   return (IsTruthy(left) && IsTruthy(right)) ? 1L : 0L;
            case BinaryOperator.Or:    return (IsTruthy(left) || IsTruthy(right)) ? 1L : 0L;
            default: return null;
        }
    }

    private static bool SqlEq(object? a, object? b) =>
        (a, b) switch
        {
            (long la, long lb)     => la == lb,
            (double da, double db) => Math.Abs(da - db) < 1e-10,
            (long la2, double db2) => Math.Abs((double)la2 - db2) < 1e-10,
            (double da2, long lb2) => Math.Abs(da2 - (double)lb2) < 1e-10,
            (string sa, string sb) => string.Equals(sa, sb, StringComparison.Ordinal),
            _ => false,
        };

    private static int SqlCmp(object? a, object? b) =>
        (a, b) switch
        {
            (long la, long lb)     => la.CompareTo(lb),
            (double da, double db) => da.CompareTo(db),
            (long la2, double db2) => ((double)la2).CompareTo(db2),
            (double da2, long lb2) => da2.CompareTo((double)lb2),
            (string sa, string sb) => string.Compare(sa, sb, StringComparison.Ordinal),
            _ => 0,
        };

    private static object? EvalBuiltinScalar(string name, object?[] args) =>
        name.ToUpperInvariant() switch
        {
            "LENGTH" when args.Length == 1 =>
                args[0] is null ? null : (object?)(long)args[0].ToString()!.Length,
            "UPPER"  when args.Length == 1 =>
                args[0] is null ? null : args[0].ToString()!.ToUpperInvariant(),
            "LOWER"  when args.Length == 1 =>
                args[0] is null ? null : args[0].ToString()!.ToLowerInvariant(),
            "TRIM"   when args.Length == 1 =>
                args[0] is null ? null : args[0].ToString()!.Trim(),
            "LTRIM"  when args.Length == 1 =>
                args[0] is null ? null : args[0].ToString()!.TrimStart(),
            "RTRIM"  when args.Length == 1 =>
                args[0] is null ? null : args[0].ToString()!.TrimEnd(),
            "ABS" when args.Length == 1 =>
                args[0] switch
                {
                    null   => null,
                    long l => (object?)(l < 0 ? -l : l),
                    double d => Math.Abs(d),
                    _ => null,
                },
            "SUBSTR" when args.Length >= 2 =>
                EvalSubstr(args),
            "REPLACE" when args.Length == 3 =>
                args[0] is null || args[1] is null || args[2] is null ? null
                    : args[0].ToString()!.Replace(args[1].ToString()!, args[2].ToString()),
            "INSTR" when args.Length == 2 =>
                args[0] is null || args[1] is null ? null
                    : (object?)(long)(args[0].ToString()!.IndexOf(
                          args[1].ToString()!, StringComparison.Ordinal) + 1),
            // SQLite uses "round half away from zero" (not banker's rounding).
            // e.g. ROUND(2.5) = 3.0, ROUND(-2.5) = -3.0.
            "ROUND" when args.Length == 1 =>
                args[0] is null ? null
                    : (object?)Math.Round(ToDouble(args[0]) ?? 0, 0, MidpointRounding.AwayFromZero),
            "ROUND" when args.Length == 2 =>
                args[0] is null || args[1] is null ? null
                    : (object?)Math.Round(ToDouble(args[0]) ?? 0, (int)(Convert.ToInt64(args[1])),
                          MidpointRounding.AwayFromZero),
            "TYPEOF" when args.Length == 1 =>
                args[0] switch
                {
                    null   => "null",
                    long   => "integer",
                    double => "real",
                    string => "text",
                    _ => "blob",
                },
            "COALESCE" =>
                args.FirstOrDefault(a => a is not null),
            "IFNULL" when args.Length == 2 =>
                args[0] ?? args[1],
            "NULLIF" when args.Length == 2 =>
                SqlEq(args[0], args[1]) ? null : args[0],
            "IIF" when args.Length == 3 =>
                IsTruthy(args[0]) ? args[1] : args[2],
            "PRINTF" or "FORMAT" when args.Length >= 1 =>
                EvalPrintf(args),
            // CONCAT(a, b, ...) — string concatenation.
            // NULL propagates: any NULL argument makes the result NULL (matches SQLite || semantics).
            "CONCAT" =>
                args.Any(a => a is null) ? null
                    : (object?)string.Concat(args.Select(a => a?.ToString() ?? "")),
            _ => throw new MiniSqliteException("OperationalError",
                     $"Unknown scalar function: {name}"),
        };

    private static object? EvalSubstr(object?[] args)
    {
        if (args[0] is null) return null;
        var s     = args[0].ToString()!;
        var start = (int)(Convert.ToInt64(args[1]));
        // SQLite SUBSTR uses 1-based indexing; negative start counts from end.
        var startIdx = start > 0 ? start - 1 : Math.Max(0, s.Length + start);
        if (startIdx >= s.Length) return "";
        if (args.Length < 3 || args[2] is null)
            return s[startIdx..];
        var len = (int)Math.Max(0, Convert.ToInt64(args[2]));
        return s.Substring(startIdx, Math.Min(len, s.Length - startIdx));
    }

    private static object? EvalCast(object? value, string typeName) =>
        typeName.ToUpperInvariant() switch
        {
            "INTEGER" or "INT" => value is null ? null : (object?)Convert.ToInt64(value),
            "REAL" or "FLOAT"  => value is null ? null : (object?)Convert.ToDouble(value),
            "TEXT" or "VARCHAR" or "CHAR" => value?.ToString(),
            _ => value,
        };

    private static object? EvalPrintf(object?[] args)
    {
        // Very minimal printf: just return the format string with %s/%d/%f substitutions.
        if (args[0] is null) return null;
        var fmt = args[0].ToString()!;
        var sb  = new System.Text.StringBuilder();
        var ai  = 1;
        for (var i = 0; i < fmt.Length; i++)
        {
            if (fmt[i] == '%' && i + 1 < fmt.Length && ai < args.Length)
            {
                var spec = fmt[i + 1];
                var arg  = args[ai++];
                sb.Append(spec switch
                {
                    'd' or 'i' => Convert.ToInt64(arg).ToString(System.Globalization.CultureInfo.InvariantCulture),
                    'f'        => Convert.ToDouble(arg).ToString("F6", System.Globalization.CultureInfo.InvariantCulture),
                    's'        => arg?.ToString() ?? "NULL",
                    _          => arg?.ToString() ?? "",
                });
                i++;
            }
            else
            {
                sb.Append(fmt[i]);
            }
        }
        return sb.ToString();
    }

    // ── Transaction management ─────────────────────────────────────────────────
    //
    // Transaction model: the connection starts in "auto-commit" mode — every
    // DML statement is immediately visible.  When the caller calls Commit(),
    // we take a new snapshot (baseline), so that a subsequent Rollback() can
    // restore to the post-commit state.  Rollback() restores the backend to
    // the snapshot taken at the most recent Commit().
    //
    // If Rollback() is called before any Commit(), we restore to the state
    // at connection-open time (the earliest snapshot we have).

    /// <summary>
    /// Commit any pending changes: take a new snapshot so a later Rollback()
    /// can restore to this point.
    /// </summary>
    public void Commit()
    {
        // Discard any previous snapshot by committing it, then take a fresh one.
        if (_txHandle.HasValue)
        {
            _backend.Commit(_txHandle.Value);
        }
        // Begin a new snapshot so the next Rollback() knows where to return to.
        _txHandle = _backend.BeginTransaction();
    }

    /// <summary>
    /// Roll back any pending changes and restore the state at the last Commit().
    /// If no Commit() was called, restores to the connection-open baseline.
    /// </summary>
    public void Rollback()
    {
        if (_txHandle.HasValue)
        {
            _backend.Rollback(_txHandle.Value);
            _txHandle = null;
            // After rollback the snapshot is consumed; start fresh so the
            // caller can continue using the connection.
        }
        // If no snapshot exists there is nothing to roll back — the connection
        // is already at the "beginning" with no deferred changes.
    }

    // Ensure a transaction is active; begin one if not yet started.
    private void EnsureTransaction()
    {
        if (!_txHandle.HasValue)
            _txHandle = _backend.BeginTransaction();
    }

    // ── Parameter binding ──────────────────────────────────────────────────────

    // Replaces every unquoted '?' in sql with the SQL literal for
    // parameters[i]. Mirrors SqlText.BindParameters from the Level 0 module.
    private static string BindParameters(string sql, IReadOnlyList<object?> parameters)
    {
        var sb    = new System.Text.StringBuilder();
        var idx   = 0;
        var i     = 0;
        while (i < sql.Length)
        {
            var ch = sql[i];
            if (ch is '\'' or '"')
            {
                // Skip quoted string literal — '?' inside quotes is not a placeholder.
                var end = i + 1;
                while (end < sql.Length)
                {
                    if (sql[end] == ch)
                    {
                        end++;
                        if (end < sql.Length && sql[end] == ch) { end++; } // doubled quote
                        else { break; }
                    }
                    else { end++; }
                }
                sb.Append(sql, i, end - i);
                i = end;
            }
            else if (ch == '-' && i + 1 < sql.Length && sql[i + 1] == '-')
            {
                // Skip line comment.
                var end = i;
                while (end < sql.Length && sql[end] != '\n') end++;
                sb.Append(sql, i, end - i);
                i = end;
            }
            else if (ch == '?')
            {
                if (idx >= parameters.Count)
                    throw new MiniSqliteException("ProgrammingError", "not enough parameters for SQL statement");
                sb.Append(ToSqlLiteral(parameters[idx++]));
                i++;
            }
            else
            {
                sb.Append(ch);
                i++;
            }
        }

        if (idx < parameters.Count)
            throw new MiniSqliteException("ProgrammingError", "too many parameters for SQL statement");

        return sb.ToString();
    }

    // Converts a C# value to the SQL literal representation it should occupy in
    // the bound-text SQL so the parser can parse it back as a typed literal.
    private static string ToSqlLiteral(object? value) => value switch
    {
        null                    => "NULL",
        bool b                  => b ? "TRUE" : "FALSE",
        string s                => $"'{s.Replace("'", "''")}'",
        char c                  => $"'{c.ToString().Replace("'", "''")}'",
        byte or sbyte or short or ushort or int or uint or long or ulong
                                => Convert.ToString(value, CultureInfo.InvariantCulture) ?? "NULL",
        float or double or decimal
                                => Convert.ToString(value, CultureInfo.InvariantCulture) ?? "NULL",
        _                       => throw new MiniSqliteException("ProgrammingError",
                                       $"unsupported parameter type: {value.GetType().Name}"),
    };
}

// ── Backend → Planner schema bridge ───────────────────────────────────────────
//
// The sql-planner and sql-backend packages each define their own ISchemaProvider
// interface.  Both have the same method signature:
//     IReadOnlyList<string> Columns(string table);
// but they are different CLR types.  This adapter bridges from the backend's live
// table catalog to the planner's interface, so the planner always sees up-to-date
// column lists (important after CREATE TABLE and DROP TABLE).

internal sealed class BackendSchemaAdapter(CodingAdventures.SqlBackend.Backend backend)
    : PlSchemaProvider
{
    public IReadOnlyList<string> Columns(string table)
    {
        try
        {
            return backend.Columns(table).Select(c => c.Name).ToArray();
        }
        catch (CodingAdventures.SqlBackend.TableNotFound ex)
        {
            throw new PlUnknownTableException(ex.Table);
        }
    }
}

// ── Parse exception ────────────────────────────────────────────────────────────

/// <summary>Thrown by SqlStatementParser when the SQL text cannot be parsed.</summary>
internal sealed class ParseException(string message) : Exception(message);

// ── SQL text → Statement parser ────────────────────────────────────────────────
//
// This is the bridge between raw SQL text and the planner's Statement type
// hierarchy. The C# sql-parser package is a stub, so we implement a full
// hand-rolled recursive-descent parser here.
//
// Grammar coverage (abbreviated BNF):
//
//   statement ::= select_stmt | insert_stmt | update_stmt | delete_stmt
//               | create_stmt | drop_stmt
//
//   select_stmt ::= SELECT [DISTINCT] select_list FROM table_ref
//                   [join_clause*] [WHERE expr] [GROUP BY expr_list]
//                   [HAVING expr] [ORDER BY sort_key_list] [LIMIT n [OFFSET m]]
//
//   insert_stmt ::= INSERT INTO table [(col_list)] VALUES (val_list) [, ...]
//
//   update_stmt ::= UPDATE table SET col=expr [, ...] [WHERE expr]
//
//   delete_stmt ::= DELETE FROM table [WHERE expr]
//
//   create_stmt ::= CREATE TABLE [IF NOT EXISTS] table (col_def [, ...])
//
//   drop_stmt   ::= DROP TABLE [IF EXISTS] table
//
// Expression grammar:
//   expr        ::= or_expr
//   or_expr     ::= and_expr (OR and_expr)*
//   and_expr    ::= not_expr (AND not_expr)*
//   not_expr    ::= NOT not_expr | cmp_expr
//   cmp_expr    ::= add_expr [(= | <> | != | < | <= | > | >=) add_expr]
//                 | add_expr [NOT? BETWEEN add_expr AND add_expr]
//                 | add_expr [NOT? IN (expr_list)]
//                 | add_expr IS [NOT] NULL
//                 | add_expr [NOT] LIKE string
//   add_expr    ::= mul_expr ((+ | - | ||) mul_expr)*
//   mul_expr    ::= unary_expr ((* | / | %) unary_expr)*
//   unary_expr  ::= - unary_expr | primary_expr
//   primary_expr ::= literal | identifier | func_call | (expr) | aggregate
//   aggregate   ::= COUNT(*) | COUNT(DISTINCT? expr) | SUM(expr) | AVG(expr)
//                 | MIN(expr) | MAX(expr)

internal static class SqlStatementParser
{
    // ── Entry point ────────────────────────────────────────────────────────────

    public static PlStatement Parse(string sql)
    {
        var tokens = Tokenize(sql.Trim());
        var p      = new Parser(tokens);
        var stmt   = p.ParseStatement();
        p.ExpectEnd();
        return stmt;
    }

    // ── Tokenizer ──────────────────────────────────────────────────────────────
    //
    // Produces a flat list of typed tokens.  The grammar does not need a
    // streaming tokenizer — all tokens are collected upfront so the parser
    // can look ahead freely.

    private enum TK
    {
        Word,        // unquoted identifier or keyword
        Number,      // integer or real literal
        Str,         // single-quoted string literal
        LParen,      // (
        RParen,      // )
        Comma,       // ,
        Dot,         // .
        Star,        // *
        Eq,          // =
        Neq,         // <> or !=
        Lt,          // <
        Lte,         // <=
        Gt,          // >
        Gte,         // >=
        Plus,        // +
        Minus,       // -
        Slash,       // /
        Percent,     // %
        PipePipe,    // ||  (string concat)
        Semi,        // ;
        EOF,
    }

    private sealed record Token(TK Kind, string Text, int Pos);

    private static IReadOnlyList<Token> Tokenize(string sql)
    {
        var tokens = new List<Token>();
        var i      = 0;

        while (i < sql.Length)
        {
            // Skip whitespace.
            if (char.IsWhiteSpace(sql[i])) { i++; continue; }

            // Line comment  -- …
            if (sql[i] == '-' && i + 1 < sql.Length && sql[i + 1] == '-')
            {
                while (i < sql.Length && sql[i] != '\n') i++;
                continue;
            }

            // Block comment  /* … */
            if (sql[i] == '/' && i + 1 < sql.Length && sql[i + 1] == '*')
            {
                i += 2;
                while (i + 1 < sql.Length && !(sql[i] == '*' && sql[i + 1] == '/')) i++;
                i += 2;
                continue;
            }

            var start = i;

            // Single-quoted string  'text' or ''  (doubled-quote escape)
            if (sql[i] == '\'')
            {
                i++;
                while (i < sql.Length)
                {
                    if (sql[i] == '\'')
                    {
                        i++;
                        if (i < sql.Length && sql[i] == '\'') i++; // escaped ''
                        else break;
                    }
                    else i++;
                }
                tokens.Add(new Token(TK.Str, sql[start..i], start));
                continue;
            }

            // Backtick or double-quoted identifier
            if (sql[i] is '"' or '`')
            {
                var q = sql[i++];
                while (i < sql.Length && sql[i] != q) i++;
                i++; // closing quote
                // Return as Word (strip outer quotes).
                tokens.Add(new Token(TK.Word, sql[(start + 1)..(i - 1)], start));
                continue;
            }

            // Number:  integer or real
            if (char.IsDigit(sql[i]) || (sql[i] == '.' && i + 1 < sql.Length && char.IsDigit(sql[i + 1])))
            {
                while (i < sql.Length && (char.IsDigit(sql[i]) || sql[i] == '.')) i++;
                // Optional exponent
                if (i < sql.Length && sql[i] is 'e' or 'E')
                {
                    i++;
                    if (i < sql.Length && sql[i] is '+' or '-') i++;
                    while (i < sql.Length && char.IsDigit(sql[i])) i++;
                }
                tokens.Add(new Token(TK.Number, sql[start..i], start));
                continue;
            }

            // Identifier or keyword
            if (char.IsLetter(sql[i]) || sql[i] == '_')
            {
                while (i < sql.Length && (char.IsLetterOrDigit(sql[i]) || sql[i] == '_')) i++;
                tokens.Add(new Token(TK.Word, sql[start..i], start));
                continue;
            }

            // Two-character tokens
            if (i + 1 < sql.Length)
            {
                var two = sql[i..(i + 2)];
                switch (two)
                {
                    case "<>": tokens.Add(new Token(TK.Neq, two, start)); i += 2; continue;
                    case "!=": tokens.Add(new Token(TK.Neq, two, start)); i += 2; continue;
                    case "<=": tokens.Add(new Token(TK.Lte, two, start)); i += 2; continue;
                    case ">=": tokens.Add(new Token(TK.Gte, two, start)); i += 2; continue;
                    case "||": tokens.Add(new Token(TK.PipePipe, two, start)); i += 2; continue;
                }
            }

            // Single-character tokens
            TK? single = sql[i] switch
            {
                '(' => TK.LParen,
                ')' => TK.RParen,
                ',' => TK.Comma,
                '.' => TK.Dot,
                '*' => TK.Star,
                '=' => TK.Eq,
                '<' => TK.Lt,
                '>' => TK.Gt,
                '+' => TK.Plus,
                '-' => TK.Minus,
                '/' => TK.Slash,
                '%' => TK.Percent,
                ';' => TK.Semi,
                _   => null,
            };
            if (single is not null)
            {
                tokens.Add(new Token(single.Value, sql[i..(i + 1)], start));
                i++;
                continue;
            }

            throw new ParseException($"unexpected character '{sql[i]}' at position {i}");
        }

        tokens.Add(new Token(TK.EOF, "", sql.Length));
        return tokens;
    }

    // ── Recursive-descent parser ───────────────────────────────────────────────

    private sealed class Parser(IReadOnlyList<Token> tokens)
    {
        private int _pos;

        private Token Peek(int ahead = 0) =>
            _pos + ahead < tokens.Count ? tokens[_pos + ahead] : tokens[^1];

        private Token Consume() => tokens[_pos++];

        private Token Expect(TK kind)
        {
            var t = Consume();
            if (t.Kind != kind)
                throw new ParseException($"expected {kind} but got '{t.Text}'");
            return t;
        }

        private bool Match(TK kind)
        {
            if (Peek().Kind == kind) { Consume(); return true; }
            return false;
        }

        // Case-insensitive keyword match without consuming.
        private bool PeekWord(string kw, int ahead = 0)
            => Peek(ahead).Kind == TK.Word &&
               string.Equals(Peek(ahead).Text, kw, StringComparison.OrdinalIgnoreCase);

        // Consume keyword (case-insensitive); throw if not present.
        private Token ExpectWord(string kw)
        {
            var t = Consume();
            if (t.Kind != TK.Word || !string.Equals(t.Text, kw, StringComparison.OrdinalIgnoreCase))
                throw new ParseException($"expected keyword '{kw}' but got '{t.Text}'");
            return t;
        }

        // Match keyword without consuming; return false otherwise.
        private bool MatchWord(string kw)
        {
            if (PeekWord(kw)) { Consume(); return true; }
            return false;
        }

        // Verify we've consumed everything (ignoring trailing semicolon).
        public void ExpectEnd()
        {
            Match(TK.Semi); // optional trailing semicolon
            if (Peek().Kind != TK.EOF)
                throw new ParseException($"unexpected token '{Peek().Text}' after statement");
        }

        // ── Statement dispatch ─────────────────────────────────────────────────

        public PlStatement ParseStatement()
        {
            var t = Peek();
            if (t.Kind != TK.Word)
                throw new ParseException($"expected SQL keyword, got '{t.Text}'");

            return t.Text.ToUpperInvariant() switch
            {
                "SELECT" => ParseSelect(),
                "INSERT" => ParseInsert(),
                "UPDATE" => ParseUpdate(),
                "DELETE" => ParseDelete(),
                "CREATE" => ParseCreate(),
                "DROP"   => ParseDrop(),
                _        => throw new ParseException($"unsupported statement type: {t.Text}"),
            };
        }

        // ── SELECT ─────────────────────────────────────────────────────────────
        //
        // SELECT [DISTINCT] col_list
        // FROM table [AS alias] [join*]
        // [WHERE expr]
        // [GROUP BY expr [, expr]*]
        // [HAVING expr]
        // [ORDER BY sort_key [, sort_key]*]
        // [LIMIT n [OFFSET m]]

        private PlStatement ParseSelect()
        {
            ExpectWord("SELECT");
            var distinct = MatchWord("DISTINCT");

            // Column list (may include *, aliases, aggregate calls)
            var columns = ParseSelectList();

            // FROM clause (optional – bare "SELECT expr" with no table)
            var from    = new List<(string Table, string? Alias)>();
            var joins   = new List<PlJoinClause>();
            if (MatchWord("FROM"))
            {
                // First table reference
                var (tbl0, a0) = ParseTableRef();
                from.Add((tbl0, a0));

                // Additional comma-separated tables (implicit cross join)
                while (Match(TK.Comma))
                {
                    var (tblN, aN) = ParseTableRef();
                    from.Add((tblN, aN));
                }

                // Explicit JOIN clauses
                while (TryParseJoin() is { } j)
                    joins.Add(j);
            }

            // WHERE
            PlSqlExpr? where = null;
            if (MatchWord("WHERE"))
                where = ParseExpr();

            // GROUP BY
            var groupBy = new List<PlSqlExpr>();
            if (MatchWord("GROUP") && ExpectWord("BY") is not null)
            {
                groupBy.Add(ParseExpr());
                while (Match(TK.Comma))
                    groupBy.Add(ParseExpr());
            }

            // HAVING
            PlSqlExpr? having = null;
            if (MatchWord("HAVING"))
                having = ParseExpr();

            // ORDER BY
            var orderBy = new List<PlSortKey>();
            if (MatchWord("ORDER") && ExpectWord("BY") is not null)
            {
                orderBy.Add(ParseSortKey());
                while (Match(TK.Comma))
                    orderBy.Add(ParseSortKey());
            }

            // LIMIT / OFFSET
            //
            // SQLite allows LIMIT -1 as a special value meaning "no limit —
            // return all rows after any OFFSET is applied".  We parse an optional
            // leading minus before the integer literal and propagate -1 upward;
            // the VM's ApplyLimit skips the take-count when it is null or < 0.
            PlLimitClause? limit = null;
            if (MatchWord("LIMIT"))
            {
                bool limitNeg = Match(TK.Minus);
                var countTok  = Expect(TK.Number);
                var rawCount  = long.Parse(countTok.Text, CultureInfo.InvariantCulture);
                // SQLite semantics: a negative LIMIT means "no row limit".
                // Represent that as null so the VM skips the take-count pass.
                long? count = limitNeg ? null : rawCount;
                long? offset = null;
                if (MatchWord("OFFSET"))
                {
                    bool offNeg = Match(TK.Minus);
                    var offTok  = Expect(TK.Number);
                    var rawOff  = long.Parse(offTok.Text, CultureInfo.InvariantCulture);
                    offset = offNeg ? (long?)null : rawOff;
                }
                limit = new PlLimitClause(count, offset);
            }

            return new PlSelectStatement(distinct, columns, from, joins, where, groupBy, having, orderBy, limit);
        }

        // Parse the comma-separated SELECT column list:  * | col [AS alias] | expr [AS alias]
        private IReadOnlyList<PlOutputColumn> ParseSelectList()
        {
            var cols = new List<PlOutputColumn>();
            do
            {
                cols.Add(ParseOneSelectColumn());
            }
            while (Match(TK.Comma));
            return cols;
        }

        private PlOutputColumn ParseOneSelectColumn()
        {
            // Bare star — SELECT *
            if (Peek().Kind == TK.Star)
            {
                Consume();
                return new PlOutputColumn.Star();
            }

            var expr = ParseExpr();

            // Optional alias:  expr AS name  |  expr name  (implicit)
            string? alias = null;
            if (MatchWord("AS"))
                alias = ExpectIdentifier();
            else if (Peek().Kind == TK.Word && !IsReservedKeyword(Peek().Text))
            {
                // Implicit alias (only for simple column references, not expressions)
                // We only consume the implicit alias if the next token is a bare identifier
                // that is NOT followed by a binary operator — otherwise it is part of the
                // next expression in the list or a keyword.
                // Conservative: only take an implicit alias if the next two tokens are
                // Word + (Comma | FROM | WHERE | GROUP | HAVING | ORDER | LIMIT | EOF | RParen | Semi).
                if (IsImplicitAliasContext())
                    alias = Consume().Text;
            }

            return new PlOutputColumn.Expr(expr, alias);
        }

        private bool IsImplicitAliasContext()
        {
            // The word at current position could be an implicit alias if the word
            // after it is a statement boundary or column separator.
            if (Peek().Kind != TK.Word || IsReservedKeyword(Peek().Text)) return false;
            var afterAlias = Peek(1);
            return afterAlias.Kind is TK.Comma or TK.EOF or TK.Semi
                || (afterAlias.Kind == TK.Word && IsSelectSectionKeyword(afterAlias.Text));
        }

        private static bool IsSelectSectionKeyword(string w) =>
            w.ToUpperInvariant() is "FROM" or "WHERE" or "GROUP" or "HAVING"
                or "ORDER" or "LIMIT" or "UNION" or "INTERSECT" or "EXCEPT";

        // Parse  table [AS alias]
        private (string Table, string? Alias) ParseTableRef()
        {
            var table = ExpectIdentifier();
            string? alias = null;
            if (MatchWord("AS"))
                alias = ExpectIdentifier();
            else if (Peek().Kind == TK.Word && !IsReservedKeyword(Peek().Text))
                alias = Consume().Text;
            return (table, alias);
        }

        // Parse one optional JOIN clause; returns null if the next token is not a join keyword.
        private PlJoinClause? TryParseJoin()
        {
            JoinKind kind;
            if (MatchWord("JOIN") || (PeekWord("INNER") && Peek(1).Kind == TK.Word && PeekWord("JOIN", 1) && Consume() is not null && Consume() is not null))
            {
                kind = JoinKind.Inner;
            }
            else if (MatchWord("LEFT"))
            {
                MatchWord("OUTER"); // optional
                ExpectWord("JOIN");
                kind = JoinKind.Left;
            }
            else if (MatchWord("RIGHT"))
            {
                MatchWord("OUTER");
                ExpectWord("JOIN");
                kind = JoinKind.Right;
            }
            else if (MatchWord("FULL"))
            {
                MatchWord("OUTER");
                ExpectWord("JOIN");
                kind = JoinKind.Full;
            }
            else if (MatchWord("CROSS"))
            {
                ExpectWord("JOIN");
                kind = JoinKind.Cross;
            }
            else
            {
                return null;
            }

            var (table, alias) = ParseTableRef();
            PlSqlExpr? on = null;
            if (MatchWord("ON"))
                on = ParseExpr();

            return new PlJoinClause(kind, table, alias, on);
        }

        // Parse one ORDER BY sort key:  expr [ASC|DESC] [NULLS FIRST|LAST]
        private PlSortKey ParseSortKey()
        {
            var expr = ParseExpr();
            var dir  = PeekWord("DESC") ? (Consume() is not null ? SortDir.Desc : SortDir.Asc) : (MatchWord("ASC") ? SortDir.Asc : SortDir.Asc);
            // SQLite NULL ordering: NULLs are treated as less than any value.
            // For ASC  → NULLs sort first (smallest value = NULL = first in ASC).
            // For DESC → NULLs sort last  (smallest value = NULL = last in DESC).
            // An explicit NULLS FIRST / NULLS LAST overrides the default.
            var nullOrder = dir == SortDir.Asc ? NullOrder.NullsFirst : NullOrder.NullsLast;
            if (MatchWord("NULLS"))
            {
                if (MatchWord("FIRST"))  nullOrder = NullOrder.NullsFirst;
                else if (MatchWord("LAST")) nullOrder = NullOrder.NullsLast;
            }
            return new PlSortKey(expr, dir, nullOrder);
        }

        // ── INSERT ─────────────────────────────────────────────────────────────

        private PlStatement ParseInsert()
        {
            ExpectWord("INSERT");
            ExpectWord("INTO");
            var table = ExpectIdentifier();

            // Optional column list
            IReadOnlyList<string>? columns = null;
            if (Peek().Kind == TK.LParen && !PeekWord("VALUES", 1))
            {
                // Check if the next token after '(' is a word (column name) not a number/string
                // This distinguishes  INSERT INTO t (col, ...) from  INSERT INTO t VALUES (...)
                // The PeekWord check handles the edge where first col could look like VALUES.
                var after = Peek(1);
                if (after.Kind == TK.Word && !string.Equals(after.Text, "VALUES", StringComparison.OrdinalIgnoreCase))
                {
                    Expect(TK.LParen);
                    var cols = new List<string>();
                    cols.Add(ExpectIdentifier());
                    while (Match(TK.Comma))
                        cols.Add(ExpectIdentifier());
                    Expect(TK.RParen);
                    columns = cols;
                }
            }

            ExpectWord("VALUES");

            var rows = new List<IReadOnlyList<PlSqlExpr>>();
            do
            {
                Expect(TK.LParen);
                var vals = new List<PlSqlExpr>();
                vals.Add(ParseExpr());
                while (Match(TK.Comma))
                    vals.Add(ParseExpr());
                Expect(TK.RParen);
                rows.Add(vals);
            }
            while (Match(TK.Comma));

            return new PlInsertStatement(table, columns, rows);
        }

        // ── UPDATE ─────────────────────────────────────────────────────────────

        private PlStatement ParseUpdate()
        {
            ExpectWord("UPDATE");
            var table = ExpectIdentifier();
            ExpectWord("SET");

            var assignments = new List<PlAssignment>();
            assignments.Add(ParseAssignment());
            while (Match(TK.Comma))
                assignments.Add(ParseAssignment());

            PlSqlExpr? where = null;
            if (MatchWord("WHERE"))
                where = ParseExpr();

            return new PlUpdateStatement(table, assignments, where);
        }

        private PlAssignment ParseAssignment()
        {
            var col  = ExpectIdentifier();
            Expect(TK.Eq);
            var expr = ParseExpr();
            return new PlAssignment(col, expr);
        }

        // ── DELETE ─────────────────────────────────────────────────────────────

        private PlStatement ParseDelete()
        {
            ExpectWord("DELETE");
            ExpectWord("FROM");
            var table = ExpectIdentifier();

            PlSqlExpr? where = null;
            if (MatchWord("WHERE"))
                where = ParseExpr();

            return new PlDeleteStatement(table, where);
        }

        // ── CREATE TABLE ───────────────────────────────────────────────────────

        private PlStatement ParseCreate()
        {
            ExpectWord("CREATE");
            ExpectWord("TABLE");

            bool ifNotExists = false;
            if (MatchWord("IF"))
            {
                ExpectWord("NOT");
                ExpectWord("EXISTS");
                ifNotExists = true;
            }

            var table = ExpectIdentifier();
            Expect(TK.LParen);

            var cols = new List<PlColumnDef>();
            cols.Add(ParseColumnDef());
            while (Match(TK.Comma))
            {
                // Handle optional trailing comma before )
                if (Peek().Kind == TK.RParen) break;
                cols.Add(ParseColumnDef());
            }

            Expect(TK.RParen);
            return new PlCreateTableStatement(table, ifNotExists, cols);
        }

        // Parse one column definition:  name [type] [NOT NULL] [PRIMARY KEY] [UNIQUE]
        private PlColumnDef ParseColumnDef()
        {
            var name     = ExpectIdentifier();
            // Type name is optional in SQLite style
            var typeName = "TEXT";
            if (Peek().Kind == TK.Word && !IsColumnConstraintKeyword(Peek().Text))
            {
                typeName = Consume().Text.ToUpperInvariant();
                // Allow compound type names like "NOT NULL" or "PRIMARY KEY"
                // We stop consuming type words when we hit a constraint keyword.
                while (Peek().Kind == TK.Word && !IsColumnConstraintKeyword(Peek().Text)
                       && !string.Equals(Peek().Text, "NOT", StringComparison.OrdinalIgnoreCase))
                {
                    typeName += " " + Consume().Text.ToUpperInvariant();
                }
            }

            bool notNull    = false;
            bool primaryKey = false;
            bool unique     = false;

            // Parse optional inline constraints
            while (true)
            {
                if (MatchWord("NOT"))
                {
                    ExpectWord("NULL");
                    notNull = true;
                }
                else if (MatchWord("PRIMARY"))
                {
                    ExpectWord("KEY");
                    primaryKey = true;
                }
                else if (MatchWord("UNIQUE"))
                {
                    unique = true;
                }
                else if (MatchWord("DEFAULT"))
                {
                    // Consume the default value (literal or parenthesized expression)
                    ParseExpr(); // value consumed and discarded for now
                }
                else if (MatchWord("REFERENCES"))
                {
                    ExpectIdentifier(); // foreign table
                    if (Peek().Kind == TK.LParen)
                    {
                        Expect(TK.LParen);
                        ExpectIdentifier();
                        while (Match(TK.Comma)) ExpectIdentifier();
                        Expect(TK.RParen);
                    }
                }
                else
                {
                    break;
                }
            }

            return new PlColumnDef(name, typeName, notNull, primaryKey, unique);
        }

        private static bool IsColumnConstraintKeyword(string w) =>
            w.ToUpperInvariant() is "NOT" or "PRIMARY" or "UNIQUE" or "DEFAULT"
                or "REFERENCES" or "CHECK" or "AUTOINCREMENT" or "AUTO_INCREMENT"
                or "ON" or "CONSTRAINT";

        // ── DROP TABLE ─────────────────────────────────────────────────────────

        private PlStatement ParseDrop()
        {
            ExpectWord("DROP");
            ExpectWord("TABLE");

            bool ifExists = false;
            if (MatchWord("IF"))
            {
                ExpectWord("EXISTS");
                ifExists = true;
            }

            var table = ExpectIdentifier();
            return new PlDropTableStatement(table, ifExists);
        }

        // ── Expression parser (recursive descent) ─────────────────────────────
        //
        // Precedence ladder (lowest → highest):
        //   OR → AND → NOT → comparison → addition → multiplication → unary → primary

        internal PlSqlExpr ParseExpr() => ParseOr();

        private PlSqlExpr ParseOr()
        {
            var left = ParseAnd();
            while (MatchWord("OR"))
            {
                var right = ParseAnd();
                left = new PlSqlExpr.BinaryOp(BinaryOperator.Or, left, right);
            }
            return left;
        }

        private PlSqlExpr ParseAnd()
        {
            var left = ParseNot();
            while (MatchWord("AND"))
            {
                var right = ParseNot();
                left = new PlSqlExpr.BinaryOp(BinaryOperator.And, left, right);
            }
            return left;
        }

        private PlSqlExpr ParseNot()
        {
            if (MatchWord("NOT"))
            {
                var operand = ParseNot();
                return new PlSqlExpr.UnaryOp(UnaryOperator.Not, operand);
            }
            return ParseComparison();
        }

        private PlSqlExpr ParseComparison()
        {
            var left = ParseAddition();

            // IS [NOT] NULL
            if (MatchWord("IS"))
            {
                if (MatchWord("NOT"))
                {
                    ExpectWord("NULL");
                    return new PlSqlExpr.IsNotNull(left);
                }
                ExpectWord("NULL");
                return new PlSqlExpr.IsNull(left);
            }

            // [NOT] BETWEEN
            var negated = MatchWord("NOT");
            if (MatchWord("BETWEEN"))
            {
                var lo = ParseAddition();
                ExpectWord("AND");
                var hi = ParseAddition();
                var between = new PlSqlExpr.Between(left, lo, hi);
                return negated ? new PlSqlExpr.UnaryOp(UnaryOperator.Not, between) : between;
            }

            // [NOT] IN (...)
            if (MatchWord("IN") || (!negated && false))
            {
                Expect(TK.LParen);
                var items = new List<PlSqlExpr>();
                if (Peek().Kind != TK.RParen)
                {
                    items.Add(ParseExpr());
                    while (Match(TK.Comma))
                        items.Add(ParseExpr());
                }
                Expect(TK.RParen);
                if (negated)
                    return new PlSqlExpr.NotIn(left, items);
                return new PlSqlExpr.In(left, items);
            }

            if (negated)
            {
                // "NOT" followed by something other than BETWEEN/IN — put NOT back
                // by wrapping; but we already consumed NOT. Let's handle LIKE.
                if (MatchWord("LIKE"))
                {
                    var pattern = ParseStringLiteral();
                    return new PlSqlExpr.NotLike(left, pattern);
                }
                // Unrecognised: treat NOT as a unary prefix on the left expression
                // (unusual but graceful fallback).
                return new PlSqlExpr.UnaryOp(UnaryOperator.Not, left);
            }

            // LIKE
            if (MatchWord("LIKE"))
            {
                var pattern = ParseStringLiteral();
                return new PlSqlExpr.Like(left, pattern);
            }

            // Standard comparison operators
            BinaryOperator? op = Peek().Kind switch
            {
                TK.Eq  => BinaryOperator.Eq,
                TK.Neq => BinaryOperator.NotEq,
                TK.Lt  => BinaryOperator.Lt,
                TK.Lte => BinaryOperator.Lte,
                TK.Gt  => BinaryOperator.Gt,
                TK.Gte => BinaryOperator.Gte,
                _      => null,
            };
            if (op is not null)
            {
                Consume();
                var right = ParseAddition();
                return new PlSqlExpr.BinaryOp(op.Value, left, right);
            }

            return left;
        }

        private string ParseStringLiteral()
        {
            var tok = Peek();
            if (tok.Kind == TK.Str)
            {
                Consume();
                // Strip surrounding single quotes and unescape doubled-quote.
                var raw = tok.Text[1..^1].Replace("''", "'");
                return raw;
            }
            throw new ParseException($"expected string literal for LIKE pattern, got '{tok.Text}'");
        }

        private PlSqlExpr ParseAddition()
        {
            var left = ParseMultiplication();
            while (true)
            {
                BinaryOperator? op = Peek().Kind switch
                {
                    TK.Plus     => BinaryOperator.Add,
                    TK.Minus    => BinaryOperator.Sub,
                    TK.PipePipe => BinaryOperator.Add, // we'll handle Concat separately below
                    _           => null,
                };
                if (op is null) break;
                var tok = Consume();
                var right = ParseMultiplication();
                // ||  maps to Concat, not Add.
                left = tok.Kind == TK.PipePipe
                    ? new PlSqlExpr.FuncCall("CONCAT", new[] { left, right })
                    : new PlSqlExpr.BinaryOp(op.Value, left, right);
            }
            return left;
        }

        private PlSqlExpr ParseMultiplication()
        {
            var left = ParseUnary();
            while (true)
            {
                BinaryOperator? op = Peek().Kind switch
                {
                    TK.Star    => BinaryOperator.Mul,
                    TK.Slash   => BinaryOperator.Div,
                    TK.Percent => BinaryOperator.Mod,
                    _          => null,
                };
                if (op is null) break;
                Consume();
                var right = ParseUnary();
                left = new PlSqlExpr.BinaryOp(op.Value, left, right);
            }
            return left;
        }

        private PlSqlExpr ParseUnary()
        {
            if (Peek().Kind == TK.Minus)
            {
                Consume();
                // Unary minus on a numeric literal — fold into the literal
                // so that the planner sees a typed constant, not an expression.
                if (Peek().Kind == TK.Number)
                {
                    var raw = Consume().Text;
                    return raw.Contains('.') || raw.Contains('e') || raw.Contains('E')
                        ? new PlSqlExpr.Literal(-double.Parse(raw, CultureInfo.InvariantCulture))
                        : new PlSqlExpr.Literal(-long.Parse(raw, CultureInfo.InvariantCulture));
                }
                return new PlSqlExpr.UnaryOp(UnaryOperator.Neg, ParseUnary());
            }
            return ParsePrimary();
        }

        // Primary: literals, identifiers, function calls, sub-expressions, aggregates.
        private PlSqlExpr ParsePrimary()
        {
            var tok = Peek();

            // Parenthesised expression
            if (tok.Kind == TK.LParen)
            {
                Consume();
                var inner = ParseExpr();
                Expect(TK.RParen);
                return inner;
            }

            // NULL literal
            if (tok.Kind == TK.Word && string.Equals(tok.Text, "NULL", StringComparison.OrdinalIgnoreCase))
            {
                Consume();
                return new PlSqlExpr.Literal(null);
            }

            // TRUE / FALSE
            if (tok.Kind == TK.Word && string.Equals(tok.Text, "TRUE", StringComparison.OrdinalIgnoreCase))
            {
                Consume();
                return new PlSqlExpr.Literal(true);
            }
            if (tok.Kind == TK.Word && string.Equals(tok.Text, "FALSE", StringComparison.OrdinalIgnoreCase))
            {
                Consume();
                return new PlSqlExpr.Literal(false);
            }

            // Numeric literal
            if (tok.Kind == TK.Number)
            {
                Consume();
                if (tok.Text.Contains('.') || tok.Text.Contains('e') || tok.Text.Contains('E'))
                    return new PlSqlExpr.Literal(double.Parse(tok.Text, CultureInfo.InvariantCulture));
                return new PlSqlExpr.Literal(long.Parse(tok.Text, CultureInfo.InvariantCulture));
            }

            // String literal
            if (tok.Kind == TK.Str)
            {
                Consume();
                var value = tok.Text[1..^1].Replace("''", "'");
                return new PlSqlExpr.Literal(value);
            }

            // Star in SELECT (handled at select-list level, but can appear in COUNT(*))
            if (tok.Kind == TK.Star)
            {
                Consume();
                return new PlSqlExpr.Wildcard();
            }

            // Word: identifier, qualified column, function call, or aggregate
            if (tok.Kind == TK.Word)
            {
                Consume();
                var name = tok.Text;

                // Function call or aggregate — next token is '('
                if (Peek().Kind == TK.LParen)
                    return ParseFunctionCall(name);

                // Qualified column ref:  table.column
                if (Peek().Kind == TK.Dot)
                {
                    Consume(); // consume '.'
                    var col = ExpectIdentifier();
                    return new PlSqlExpr.Column(name, col);
                }

                // Bare identifier — unqualified column reference
                return new PlSqlExpr.Column(null, name);
            }

            throw new ParseException($"unexpected token '{tok.Text}' at position {tok.Pos}");
        }

        // Parse a function or aggregate call.  name is already consumed.
        private PlSqlExpr ParseFunctionCall(string name)
        {
            Expect(TK.LParen);
            var upper = name.ToUpperInvariant();

            // COUNT(*)
            if (upper == "COUNT" && Peek().Kind == TK.Star)
            {
                Consume(); // consume *
                Expect(TK.RParen);
                return new PlSqlExpr.AggExpr(AggFunction.Count, new PlAggArg.Star(), false);
            }

            // Aggregate functions: COUNT(expr), SUM(expr), AVG(expr), MIN(expr), MAX(expr)
            if (upper is "COUNT" or "SUM" or "AVG" or "MIN" or "MAX")
            {
                var func = upper switch
                {
                    "COUNT" => AggFunction.Count,
                    "SUM"   => AggFunction.Sum,
                    "AVG"   => AggFunction.Avg,
                    "MIN"   => AggFunction.Min,
                    "MAX"   => AggFunction.Max,
                    _       => AggFunction.Count,
                };
                var distinct = MatchWord("DISTINCT");
                var argExpr  = ParseExpr();
                Expect(TK.RParen);
                return new PlSqlExpr.AggExpr(func, new PlAggArg.Expr(argExpr), distinct);
            }

            // Scalar function call: LENGTH, UPPER, LOWER, SUBSTR, TRIM, etc.
            if (Peek().Kind == TK.RParen)
            {
                Consume();
                return new PlSqlExpr.FuncCall(upper, Array.Empty<PlSqlExpr>());
            }

            var args = new List<PlSqlExpr>();
            args.Add(ParseExpr());
            while (Match(TK.Comma))
                args.Add(ParseExpr());
            Expect(TK.RParen);
            return new PlSqlExpr.FuncCall(upper, args);
        }

        // Expect and return an unquoted identifier.
        private string ExpectIdentifier()
        {
            var t = Consume();
            if (t.Kind == TK.Word)
                return t.Text;
            throw new ParseException($"expected identifier but got '{t.Text}'");
        }

        // Reserved SQL keywords that cannot be used as implicit aliases or bare identifiers.
        private static bool IsReservedKeyword(string w) =>
            w.ToUpperInvariant() is
                "SELECT" or "FROM" or "WHERE" or "GROUP" or "HAVING" or "ORDER" or "BY"
                or "LIMIT" or "OFFSET" or "JOIN" or "INNER" or "LEFT" or "RIGHT" or "FULL"
                or "CROSS" or "OUTER" or "ON" or "AS" or "AND" or "OR" or "NOT" or "NULL"
                or "IN" or "BETWEEN" or "LIKE" or "IS" or "DISTINCT" or "UNION"
                or "INSERT" or "INTO" or "VALUES" or "UPDATE" or "SET" or "DELETE"
                or "CREATE" or "TABLE" or "DROP" or "IF" or "EXISTS" or "ASC" or "DESC"
                or "TRUE" or "FALSE" or "CASE" or "WHEN" or "THEN" or "ELSE" or "END"
                or "NULLS" or "FIRST" or "LAST" or "PRIMARY" or "KEY" or "UNIQUE"
                or "DEFAULT" or "REFERENCES" or "AUTOINCREMENT" or "AUTO_INCREMENT";
    }
}

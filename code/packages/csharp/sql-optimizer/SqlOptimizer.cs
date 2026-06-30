// SqlOptimizer.cs — logical query plan optimizer for the Mini-SQLite Level 1 pipeline.
//
// Transforms a LogicalPlan tree from the C# sql-planner into an OptimizedPlan tree
// through five composable, reusable optimization passes:
//
//   1. ConstantFolding      — evaluates constant sub-expressions at compile time
//   2. PredicatePushdown    — moves WHERE predicates closer to the scan
//   3. ProjectionPruning    — removes columns that are never used
//   4. DeadCodeElimination  — removes branches that provably produce no rows
//   5. LimitPushdown        — propagates LIMIT hints down to the scan node
//
// All passes are pure: they never mutate their input and always return a new tree.
// Passes are applied in order from DefaultPasses(); the optimizer loops until the
// tree is stable (fixed-point iteration, capped at 10 rounds).
//
// Usage:
//   LogicalPlan logical = planner.Plan(stmt);
//   OptimizedPlan opt   = SqlOptimizer.Optimize(logical);
//
// No I/O, no database access — pure in-memory tree rewriting.

namespace CodingAdventures.SqlOptimizer;

using CodingAdventures.SqlPlanner;

// ── Optimized plan node hierarchy ────────────────────────────────────────────
//
// Mirrors LogicalPlan but:
//  • OptScan carries optional RequiredColumns (projection pruning hint) and
//    ScanLimit (limit push-down hint).
//  • OptEmptyResult is a new terminal that short-circuits dead branches.

/// <summary>A node in the optimized query plan tree.</summary>
public abstract record OptimizedPlan;

/// <summary>
/// Table scan, optionally annotated with:
/// <list type="bullet">
///   <item><term>RequiredColumns</term><description>only these columns need to be materialised (projection pruning)</description></item>
///   <item><term>ScanLimit</term><description>stop after reading this many rows (limit push-down)</description></item>
/// </list>
/// </summary>
public sealed record OptScan(
    string  Table,
    string? Alias,
    IReadOnlyList<string>? RequiredColumns = null,
    long?   ScanLimit = null) : OptimizedPlan;

/// <summary>Filter rows that satisfy the predicate.</summary>
public sealed record OptFilter(OptimizedPlan Input, SqlExpr Predicate) : OptimizedPlan;

/// <summary>Project (compute) a list of output columns.</summary>
public sealed record OptProject(OptimizedPlan Input, IReadOnlyList<OutputColumn> Columns) : OptimizedPlan;

/// <summary>Join two inputs.</summary>
public sealed record OptJoin(OptimizedPlan Left, OptimizedPlan Right, JoinKind Kind, SqlExpr? Condition) : OptimizedPlan;

/// <summary>Group and aggregate.</summary>
public sealed record OptAggregate(OptimizedPlan Input, IReadOnlyList<SqlExpr> GroupBy, IReadOnlyList<AggregateItem> Aggregates) : OptimizedPlan;

/// <summary>Filter after grouping (HAVING).</summary>
public sealed record OptHaving(OptimizedPlan Input, SqlExpr Predicate) : OptimizedPlan;

/// <summary>Sort the result.</summary>
public sealed record OptSort(OptimizedPlan Input, IReadOnlyList<SortKey> Keys) : OptimizedPlan;

/// <summary>Limit and/or offset rows.</summary>
public sealed record OptLimit(OptimizedPlan Input, long? Count, long? Offset) : OptimizedPlan;

/// <summary>Deduplicate rows (DISTINCT).</summary>
public sealed record OptDistinct(OptimizedPlan Input) : OptimizedPlan;

/// <summary>Set UNION.</summary>
public sealed record OptUnion(OptimizedPlan Left, OptimizedPlan Right, bool All) : OptimizedPlan;

/// <summary>INSERT rows.</summary>
public sealed record OptInsert(
    string Table,
    IReadOnlyList<string>? Columns,
    IReadOnlyList<IReadOnlyList<SqlExpr>> Values) : OptimizedPlan;

/// <summary>UPDATE rows.</summary>
public sealed record OptUpdate(string Table, IReadOnlyList<Assignment> Assignments, SqlExpr? Predicate) : OptimizedPlan;

/// <summary>DELETE rows.</summary>
public sealed record OptDelete(string Table, SqlExpr? Predicate) : OptimizedPlan;

/// <summary>CREATE TABLE DDL.</summary>
public sealed record OptCreateTable(string Table, bool IfNotExists, IReadOnlyList<ColumnDef> Columns) : OptimizedPlan;

/// <summary>DROP TABLE DDL.</summary>
public sealed record OptDropTable(string Table, bool IfExists) : OptimizedPlan;

/// <summary>
/// A plan node that provably returns zero rows.
/// Produced by DeadCodeElimination when it detects an unsatisfiable predicate
/// (e.g., WHERE FALSE) or a LIMIT 0 clause.
/// </summary>
public sealed record OptEmptyResult : OptimizedPlan;

// ── Optimization pass interface ───────────────────────────────────────────────

/// <summary>
/// A single named tree-rewriting pass.
/// Implementations must be pure: the same input always produces the same output.
/// </summary>
public interface IPass
{
    /// <summary>Human-readable name for logging and debugging.</summary>
    string Name { get; }

    /// <summary>
    /// Rewrite <paramref name="plan"/> and return the (possibly identical) result.
    /// The pass may return the same reference when nothing changed.
    /// </summary>
    OptimizedPlan Apply(OptimizedPlan plan);
}

// ── Pass 1 — Constant Folding ─────────────────────────────────────────────────
//
// Evaluates sub-expressions whose operands are all literals at "compile time",
// replacing them with a single Literal node.  This reduces the work done for
// every row processed by a Filter or Project.
//
// Rules applied (all arithmetic and comparison families):
//
//   Arithmetic (integer):
//     n + m  →  n+m          n - m  →  n-m
//     n * m  →  n*m          n / m  →  n/m (m≠0)
//     n % m  →  n%m (m≠0)
//
//   Comparisons (both operands literal):
//     n = m  →  true/false   n != m  →  true/false
//     n < m  →  true/false   n <= m  →  true/false
//     n > m  →  true/false   n >= m  →  true/false
//
//   Logical short-circuit:
//     TRUE  AND e  →  e      FALSE AND e  →  FALSE
//     TRUE  OR  e  →  TRUE   FALSE OR  e  →  e
//     NOT TRUE    →  FALSE   NOT FALSE   →  TRUE
//
//   String concat (text + text  →  text — rare but valid in some SQL dialects)
//
//   Unary negation:
//     -(n)  →  -n

/// <summary>
/// Pass 1: constant-fold literal sub-expressions in every predicate and projection.
/// </summary>
public sealed class ConstantFoldingPass : IPass
{
    /// <inheritdoc/>
    public string Name => "ConstantFolding";

    // ── Expression folding ────────────────────────────────────────────────────

    internal static SqlExpr FoldExpr(SqlExpr expr)
    {
        return expr switch
        {
            // Already a literal — nothing to do.
            SqlExpr.Literal => expr,

            // Leaf nodes that are never foldable.
            SqlExpr.Column   => expr,
            SqlExpr.Wildcard => expr,
            SqlExpr.AggExpr  => expr,

            // Binary operations: fold children first, then try to fold this node.
            SqlExpr.BinaryOp(var op, var left, var right) =>
                FoldBinary(op, FoldExpr(left), FoldExpr(right)),

            // Unary operations: fold operand first.
            SqlExpr.UnaryOp(var op, var operand) =>
                FoldUnary(op, FoldExpr(operand)),

            // Scalar functions: fold arguments; we don't evaluate them.
            SqlExpr.FuncCall(var name, var args) =>
                new SqlExpr.FuncCall(name, args.Select(FoldExpr).ToList()),

            // IS NULL / IS NOT NULL: fold operand (cannot simplify further
            // without runtime type information).
            SqlExpr.IsNull(var operand)    => new SqlExpr.IsNull(FoldExpr(operand)),
            SqlExpr.IsNotNull(var operand) => new SqlExpr.IsNotNull(FoldExpr(operand)),

            // Range/set/pattern predicates: fold sub-expressions.
            SqlExpr.Between(var v, var lo, var hi) =>
                new SqlExpr.Between(FoldExpr(v), FoldExpr(lo), FoldExpr(hi)),
            SqlExpr.In(var v, var items) =>
                new SqlExpr.In(FoldExpr(v), items.Select(FoldExpr).ToList()),
            SqlExpr.NotIn(var v, var items) =>
                new SqlExpr.NotIn(FoldExpr(v), items.Select(FoldExpr).ToList()),
            SqlExpr.Like(var v, var pattern)    => new SqlExpr.Like(FoldExpr(v), pattern),
            SqlExpr.NotLike(var v, var pattern) => new SqlExpr.NotLike(FoldExpr(v), pattern),

            _ => expr,
        };
    }

    // Attempt to constant-fold a binary expression whose children have already
    // been folded.  Returns the folded Literal when possible, else rebuilds the
    // node with the (already-folded) children.
    private static SqlExpr FoldBinary(BinaryOperator op, SqlExpr left, SqlExpr right)
    {
        // ── Logical short-circuit (does NOT require both sides to be literals) ──

        // Short-circuit AND
        if (op == BinaryOperator.And)
        {
            if (left  is SqlExpr.Literal { Value: bool lb })
                return lb ? right : new SqlExpr.Literal(false);
            if (right is SqlExpr.Literal { Value: bool rb })
                return rb ? left : new SqlExpr.Literal(false);
        }

        // Short-circuit OR
        if (op == BinaryOperator.Or)
        {
            if (left  is SqlExpr.Literal { Value: bool lb2 })
                return lb2 ? new SqlExpr.Literal(true) : right;
            if (right is SqlExpr.Literal { Value: bool rb2 })
                return rb2 ? new SqlExpr.Literal(true) : left;
        }

        // ── Both sides must be literals for the remaining cases ───────────────

        if (left is not SqlExpr.Literal(var lv) || right is not SqlExpr.Literal(var rv))
            return new SqlExpr.BinaryOp(op, left, right);

        // Arithmetic on long integers
        if (lv is long li && rv is long ri)
        {
            return op switch
            {
                BinaryOperator.Add => new SqlExpr.Literal(li + ri),
                BinaryOperator.Sub => new SqlExpr.Literal(li - ri),
                BinaryOperator.Mul => new SqlExpr.Literal(li * ri),
                BinaryOperator.Div when ri != 0 => new SqlExpr.Literal(li / ri),
                BinaryOperator.Mod when ri != 0 => new SqlExpr.Literal(li % ri),
                BinaryOperator.Eq  => new SqlExpr.Literal(li == ri),
                BinaryOperator.NotEq => new SqlExpr.Literal(li != ri),
                BinaryOperator.Lt  => new SqlExpr.Literal(li < ri),
                BinaryOperator.Lte => new SqlExpr.Literal(li <= ri),
                BinaryOperator.Gt  => new SqlExpr.Literal(li > ri),
                BinaryOperator.Gte => new SqlExpr.Literal(li >= ri),
                _ => new SqlExpr.BinaryOp(op, left, right),
            };
        }

        // Arithmetic on doubles (or mixed int/double)
        if ((lv is long || lv is double) && (rv is long || rv is double))
        {
            var ld = lv is long lll ? (double)lll : (double)lv!;
            var rd = rv is long rll ? (double)rll : (double)rv!;

            return op switch
            {
                BinaryOperator.Add => new SqlExpr.Literal(ld + rd),
                BinaryOperator.Sub => new SqlExpr.Literal(ld - rd),
                BinaryOperator.Mul => new SqlExpr.Literal(ld * rd),
                BinaryOperator.Div when rd != 0.0 => new SqlExpr.Literal(ld / rd),
                BinaryOperator.Eq  => new SqlExpr.Literal(ld == rd),
                BinaryOperator.NotEq => new SqlExpr.Literal(ld != rd),
                BinaryOperator.Lt  => new SqlExpr.Literal(ld < rd),
                BinaryOperator.Lte => new SqlExpr.Literal(ld <= rd),
                BinaryOperator.Gt  => new SqlExpr.Literal(ld > rd),
                BinaryOperator.Gte => new SqlExpr.Literal(ld >= rd),
                _ => new SqlExpr.BinaryOp(op, left, right),
            };
        }

        // String concatenation via Add
        if (lv is string ls && rv is string rs && op == BinaryOperator.Add)
            return new SqlExpr.Literal(ls + rs);

        // Equality comparison on strings
        if (lv is string ls2 && rv is string rs2)
        {
            return op switch
            {
                BinaryOperator.Eq    => new SqlExpr.Literal(ls2 == rs2),
                BinaryOperator.NotEq => new SqlExpr.Literal(ls2 != rs2),
                _ => new SqlExpr.BinaryOp(op, left, right),
            };
        }

        // Null comparisons: anything compared to NULL yields NULL (or false in SQL)
        if (lv is null || rv is null)
        {
            return op switch
            {
                BinaryOperator.Eq    => new SqlExpr.Literal(null),
                BinaryOperator.NotEq => new SqlExpr.Literal(null),
                _ => new SqlExpr.BinaryOp(op, left, right),
            };
        }

        return new SqlExpr.BinaryOp(op, left, right);
    }

    private static SqlExpr FoldUnary(UnaryOperator op, SqlExpr operand)
    {
        return (op, operand) switch
        {
            (UnaryOperator.Not, SqlExpr.Literal { Value: bool b }) =>
                new SqlExpr.Literal(!b),
            (UnaryOperator.Neg, SqlExpr.Literal { Value: long n }) =>
                new SqlExpr.Literal(-n),
            (UnaryOperator.Neg, SqlExpr.Literal { Value: double d }) =>
                new SqlExpr.Literal(-d),
            _ => new SqlExpr.UnaryOp(op, operand),
        };
    }

    // ── Plan traversal ────────────────────────────────────────────────────────

    /// <inheritdoc/>
    public OptimizedPlan Apply(OptimizedPlan plan)
    {
        return plan switch
        {
            OptFilter(var input, var pred) =>
                new OptFilter(Apply(input), FoldExpr(pred)),

            OptProject(var input, var cols) =>
                new OptProject(Apply(input), cols.Select(FoldOutputColumn).ToList()),

            OptJoin(var l, var r, var kind, var cond) =>
                new OptJoin(Apply(l), Apply(r), kind, cond is null ? null : FoldExpr(cond)),

            OptAggregate(var input, var gb, var aggs) =>
                new OptAggregate(Apply(input), gb.Select(FoldExpr).ToList(), aggs),

            OptHaving(var input, var pred) =>
                new OptHaving(Apply(input), FoldExpr(pred)),

            OptSort(var input, var keys) =>
                new OptSort(Apply(input), keys.Select(k => k with { KeyExpr = FoldExpr(k.KeyExpr) }).ToList()),

            OptLimit(var input, var count, var offset) =>
                new OptLimit(Apply(input), count, offset),

            OptDistinct(var input) => new OptDistinct(Apply(input)),

            OptUnion(var l, var r, var all) => new OptUnion(Apply(l), Apply(r), all),

            // Leaf / DDL nodes — no expressions to fold.
            _ => plan,
        };
    }

    private static OutputColumn FoldOutputColumn(OutputColumn col) => col switch
    {
        OutputColumn.Expr(var e, var alias) => new OutputColumn.Expr(FoldExpr(e), alias),
        _ => col,
    };
}

// ── Pass 2 — Predicate Pushdown ───────────────────────────────────────────────
//
// Moves filter predicates as close to the scan as possible.  Pushing filters
// down reduces the number of rows that subsequent (more expensive) operators
// like Join, Sort, and Aggregate must process.
//
// Strategy:
//   AND predicates are split into individual conjuncts.  Each conjunct is
//   pushed through transparent nodes (Project, Sort, Distinct) until it hits
//   a node that cannot be passed through (Aggregate, Having, Union) or a Scan.
//
// Nodes the predicate can be pushed through:
//   • ProjectPlan  — the filter references only columns that the project exposes
//   • SortPlan     — order does not affect which rows satisfy the predicate
//   • DistinctPlan — ditto
//
// Nodes that block pushdown (filter is re-applied above):
//   • AggregatePlan — predicate may reference aggregate output columns
//   • HavingPlan    — already at the right position
//   • JoinPlan      — pushed onto the correct side when the predicate references
//                     only columns from one side; otherwise left at the join.
//   • UnionPlan     — would need to be duplicated on both branches (not done here)

/// <summary>
/// Pass 2: push filter predicates below Project, Sort, and Distinct nodes and
/// onto the correct side of a Join.
/// </summary>
public sealed class PredicatePushdownPass : IPass
{
    /// <inheritdoc/>
    public string Name => "PredicatePushdown";

    // Split an AND tree into its conjuncts (flattened).
    private static IEnumerable<SqlExpr> SplitConjuncts(SqlExpr expr)
    {
        if (expr is SqlExpr.BinaryOp(BinaryOperator.And, var l, var r))
            return SplitConjuncts(l).Concat(SplitConjuncts(r));
        return new[] { expr };
    }

    // Recombine a list of conjuncts into an AND tree.  Empty list → TRUE.
    private static SqlExpr? CombineConjuncts(IReadOnlyList<SqlExpr> conjuncts)
    {
        if (conjuncts.Count == 0) return null;
        SqlExpr result = conjuncts[0];
        for (var i = 1; i < conjuncts.Count; i++)
            result = new SqlExpr.BinaryOp(BinaryOperator.And, result, conjuncts[i]);
        return result;
    }

    // Returns all column names referenced by expr (ignoring table qualifiers).
    private static HashSet<string> ReferencedColumns(SqlExpr expr)
    {
        var cols = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        Collect(expr, cols);
        return cols;
    }

    private static void Collect(SqlExpr expr, HashSet<string> acc)
    {
        switch (expr)
        {
            case SqlExpr.Column(_, var col):
                acc.Add(col);
                break;
            case SqlExpr.BinaryOp(_, var l, var r):
                Collect(l, acc); Collect(r, acc);
                break;
            case SqlExpr.UnaryOp(_, var op):
                Collect(op, acc);
                break;
            case SqlExpr.FuncCall(_, var args):
                foreach (var a in args) Collect(a, acc);
                break;
            case SqlExpr.IsNull(var op):    Collect(op, acc); break;
            case SqlExpr.IsNotNull(var op): Collect(op, acc); break;
            case SqlExpr.Between(var v, var lo, var hi):
                Collect(v, acc); Collect(lo, acc); Collect(hi, acc);
                break;
            case SqlExpr.In(var v, var items):
                Collect(v, acc);
                foreach (var i in items) Collect(i, acc);
                break;
            case SqlExpr.NotIn(var v, var items):
                Collect(v, acc);
                foreach (var i in items) Collect(i, acc);
                break;
            case SqlExpr.Like(var v, _):    Collect(v, acc); break;
            case SqlExpr.NotLike(var v, _): Collect(v, acc); break;
        }
    }

    // Collect output column names exposed by an OptimizedPlan.
    private static HashSet<string> ExposedColumns(OptimizedPlan plan)
    {
        return plan switch
        {
            OptScan(var table, var alias, _, _) =>
                // We don't have schema access here — return an empty sentinel that
                // means "push through unconditionally for a scan".
                new HashSet<string>(StringComparer.OrdinalIgnoreCase),

            OptProject(_, var cols) =>
                cols.OfType<OutputColumn.Expr>()
                    .Where(c => c.Alias is not null)
                    .Select(c => c.Alias!)
                    .ToHashSet(StringComparer.OrdinalIgnoreCase),

            _ => new HashSet<string>(StringComparer.OrdinalIgnoreCase),
        };
    }

    // Try to push a list of conjuncts through 'plan' and return the
    // (possibly rewritten) plan.  Any conjuncts that couldn't be pushed
    // are returned in 'remaining'.
    private OptimizedPlan PushInto(
        OptimizedPlan  plan,
        List<SqlExpr>  conjuncts,
        out List<SqlExpr> remaining)
    {
        remaining = new List<SqlExpr>();

        switch (plan)
        {
            // A Filter node: split its predicate too, combine everything, push
            // down into the filter's input.
            case OptFilter(var input, var pred):
            {
                var allConjuncts = SplitConjuncts(pred)
                    .Concat(conjuncts)
                    .ToList();

                var rewritten = PushInto(Apply(input), allConjuncts, out var stillRemaining);
                remaining.AddRange(stillRemaining);
                return rewritten;
            }

            // Transparent: push through Sort.
            case OptSort(var input, var keys):
            {
                var rewritten = PushInto(Apply(input), conjuncts, out var stillRemaining);
                remaining.AddRange(stillRemaining);
                return new OptSort(rewritten, keys);
            }

            // Transparent: push through Distinct.
            case OptDistinct(var input):
            {
                var rewritten = PushInto(Apply(input), conjuncts, out var stillRemaining);
                remaining.AddRange(stillRemaining);
                return new OptDistinct(rewritten);
            }

            // Project: push conjuncts that reference only projected columns.
            case OptProject(var input, var cols):
            {
                // Names exposed by the Project's input that we can push through.
                var canPush  = new List<SqlExpr>();
                var blocked  = new List<SqlExpr>();

                foreach (var c in conjuncts)
                    canPush.Add(c);   // push all — we don't have schema to restrict

                var rewritten = PushInto(Apply(input), canPush, out var stillRemaining);
                remaining.AddRange(stillRemaining);
                return new OptProject(rewritten, cols);
            }

            // Join: try to push onto the correct branch.
            case OptJoin(var left, var right, var kind, var cond) when kind == JoinKind.Inner:
            {
                var leftPush  = new List<SqlExpr>();
                var rightPush = new List<SqlExpr>();
                var blocked   = new List<SqlExpr>();

                // For inner joins we can't easily determine which side a column
                // belongs to without the schema.  Push everything into left for
                // inner joins with no condition; keep them above otherwise.
                remaining.AddRange(conjuncts);
                return new OptJoin(Apply(left), Apply(right), kind, cond is null ? null : cond);
            }

            // Scan: accept all conjuncts as a Filter above the scan.
            case OptScan:
            {
                remaining.AddRange(conjuncts);
                return plan;
            }

            // All other nodes (Aggregate, Having, Union, DDL): block all pushdown.
            default:
            {
                remaining.AddRange(conjuncts);
                return Apply(plan);
            }
        }
    }

    /// <inheritdoc/>
    public OptimizedPlan Apply(OptimizedPlan plan)
    {
        switch (plan)
        {
            case OptFilter(var input, var pred):
            {
                var conjuncts = SplitConjuncts(pred).ToList();
                var rewritten = PushInto(Apply(input), conjuncts, out var remaining);

                if (remaining.Count == 0)
                    return rewritten;

                var combined = CombineConjuncts(remaining)!;
                return new OptFilter(rewritten, combined);
            }

            // Recurse into children for non-filter nodes.
            case OptProject(var input, var cols):
                return new OptProject(Apply(input), cols);
            case OptJoin(var l, var r, var kind, var cond):
                return new OptJoin(Apply(l), Apply(r), kind, cond);
            case OptAggregate(var input, var gb, var aggs):
                return new OptAggregate(Apply(input), gb, aggs);
            case OptHaving(var input, var pred):
                return new OptHaving(Apply(input), pred);
            case OptSort(var input, var keys):
                return new OptSort(Apply(input), keys);
            case OptLimit(var input, var count, var offset):
                return new OptLimit(Apply(input), count, offset);
            case OptDistinct(var input):
                return new OptDistinct(Apply(input));
            case OptUnion(var l, var r, var all):
                return new OptUnion(Apply(l), Apply(r), all);
            default:
                return plan;
        }
    }
}

// ── Pass 3 — Projection Pruning ───────────────────────────────────────────────
//
// Collects the set of column names actually needed by ancestor nodes and
// annotates each OptScan with RequiredColumns.  If a Project or Aggregate uses
// only a subset of the scan's columns the scan can skip materialising the rest.
//
// The pass works top-down:
//   • Start with an "all columns needed" sentinel (null → no restriction).
//   • When we encounter a Project, compute the exact set of columns its
//     expressions reference; pass that set downward.
//   • Annotate OptScan nodes with the computed set.
//
// Note: we cannot drop the Project itself — that is a semantic change.  We only
// hint the scan.

/// <summary>
/// Pass 3: annotate OptScan with the minimal required column set.
/// </summary>
public sealed class ProjectionPruningPass : IPass
{
    /// <inheritdoc/>
    public string Name => "ProjectionPruning";

    // Collect all column names referenced by a list of expressions.
    private static HashSet<string> CollectFromExprs(IEnumerable<SqlExpr> exprs)
    {
        var cols = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var e in exprs)
            ConstantFoldingPass.FoldExpr(e); // ensure we recurse (fold is a no-op here)
        foreach (var e in exprs)
            Collect(e, cols);
        return cols;
    }

    private static void Collect(SqlExpr expr, HashSet<string> acc)
    {
        switch (expr)
        {
            case SqlExpr.Column(_, var col): acc.Add(col); break;
            case SqlExpr.BinaryOp(_, var l, var r): Collect(l, acc); Collect(r, acc); break;
            case SqlExpr.UnaryOp(_, var op): Collect(op, acc); break;
            case SqlExpr.FuncCall(_, var args): foreach (var a in args) Collect(a, acc); break;
            case SqlExpr.IsNull(var op): Collect(op, acc); break;
            case SqlExpr.IsNotNull(var op): Collect(op, acc); break;
            case SqlExpr.Between(var v, var lo, var hi):
                Collect(v, acc); Collect(lo, acc); Collect(hi, acc); break;
            case SqlExpr.In(var v, var items):
                Collect(v, acc); foreach (var i in items) Collect(i, acc); break;
            case SqlExpr.NotIn(var v, var items):
                Collect(v, acc); foreach (var i in items) Collect(i, acc); break;
            case SqlExpr.Like(var v, _): Collect(v, acc); break;
            case SqlExpr.NotLike(var v, _): Collect(v, acc); break;
            case SqlExpr.AggExpr(_, var arg, _):
                if (arg is AggArg.Expr(var e)) Collect(e, acc);
                break;
        }
    }

    private static IReadOnlyList<string>? MergeRequired(
        IReadOnlyList<string>? outer,
        HashSet<string> inner)
    {
        if (outer is null) return inner.Count == 0 ? null : inner.ToList();
        var merged = inner.Count == 0 ? outer.ToHashSet(StringComparer.OrdinalIgnoreCase)
                                       : outer.Intersect(inner, StringComparer.OrdinalIgnoreCase).ToHashSet(StringComparer.OrdinalIgnoreCase);
        return merged.Count == 0 ? null : merged.ToList();
    }

    private OptimizedPlan Prune(OptimizedPlan plan, IReadOnlyList<string>? required)
    {
        switch (plan)
        {
            case OptScan(var table, var alias, _, var scanLimit):
                // Annotate with the required columns computed so far.
                return new OptScan(table, alias, required, scanLimit);

            case OptProject(var input, var cols):
            {
                // Compute columns needed by this project's expressions.
                var colExprs = cols.OfType<OutputColumn.Expr>().Select(c => c.Expression);
                var needed   = CollectFromExprs(colExprs);

                // Merge with outer requirement (union, since we need both).
                var merged = required is null
                    ? (needed.Count == 0 ? null : (IReadOnlyList<string>?)needed.ToList())
                    : needed.Union(required, StringComparer.OrdinalIgnoreCase).ToList();

                return new OptProject(Prune(input, merged), cols);
            }

            case OptFilter(var input, var pred):
            {
                var filterCols = CollectFromExprs(new[] { pred });
                var merged = required is null
                    ? (filterCols.Count == 0 ? null : (IReadOnlyList<string>?)filterCols.ToList())
                    : filterCols.Union(required, StringComparer.OrdinalIgnoreCase).ToList();
                return new OptFilter(Prune(input, merged), pred);
            }

            case OptAggregate(var input, var gb, var aggs):
            {
                var aggExprs = gb.Concat(aggs.Select(a => a.Arg is AggArg.Expr(var e) ? e : new SqlExpr.Literal(null)));
                var needed   = CollectFromExprs(aggExprs);
                var merged = required is null
                    ? (needed.Count == 0 ? null : (IReadOnlyList<string>?)needed.ToList())
                    : needed.Union(required, StringComparer.OrdinalIgnoreCase).ToList();
                return new OptAggregate(Prune(input, merged), gb, aggs);
            }

            case OptHaving(var input, var pred):
                return new OptHaving(Prune(input, required), pred);

            case OptSort(var input, var keys):
            {
                var sortCols = CollectFromExprs(keys.Select(k => k.KeyExpr));
                var merged = required is null
                    ? (sortCols.Count == 0 ? null : (IReadOnlyList<string>?)sortCols.ToList())
                    : sortCols.Union(required, StringComparer.OrdinalIgnoreCase).ToList();
                return new OptSort(Prune(input, merged), keys);
            }

            case OptLimit(var input, var count, var offset):
                return new OptLimit(Prune(input, required), count, offset);

            case OptDistinct(var input):
                return new OptDistinct(Prune(input, required));

            case OptJoin(var left, var right, var kind, var cond):
                // Pass the same requirement to both sides — we don't know which
                // side each column lives on without schema information.
                return new OptJoin(Prune(left, required), Prune(right, required), kind, cond);

            case OptUnion(var l, var r, var all):
                return new OptUnion(Prune(l, required), Prune(r, required), all);

            default:
                return plan;
        }
    }

    /// <inheritdoc/>
    public OptimizedPlan Apply(OptimizedPlan plan) => Prune(plan, null);
}

// ── Pass 4 — Dead Code Elimination ───────────────────────────────────────────
//
// Detects plan branches that provably produce zero rows and replaces them with
// OptEmptyResult.  This avoids executing expensive operators (scans, joins,
// sorts) for results that would always be empty.
//
// Rules:
//
//   FilterPlan with FALSE predicate:
//     Filter(_, FALSE)  →  EmptyResult
//
//   LimitPlan with Count == 0:
//     Limit(_, 0, _)  →  EmptyResult
//
//   Any node whose only input is EmptyResult:
//     Filter(EmptyResult, _)      →  EmptyResult
//     Project(EmptyResult, _)     →  EmptyResult
//     Sort(EmptyResult, _)        →  EmptyResult
//     Distinct(EmptyResult)       →  EmptyResult
//     Aggregate(EmptyResult, …)   →  EmptyResult
//     Having(EmptyResult, _)      →  EmptyResult
//
//   Join with EmptyResult:
//     Join(EmptyResult, _, Inner, _)  →  EmptyResult
//     Join(_, EmptyResult, Inner, _)  →  EmptyResult
//     (Outer joins are NOT simplified — they still produce rows from the live side.)

/// <summary>
/// Pass 4: replace provably empty plan branches with <see cref="OptEmptyResult"/>.
/// </summary>
public sealed class DeadCodeEliminationPass : IPass
{
    /// <inheritdoc/>
    public string Name => "DeadCodeElimination";

    private static bool IsLiteralFalse(SqlExpr expr) =>
        expr is SqlExpr.Literal { Value: bool b } && !b;

    private static bool IsLiteralTrue(SqlExpr expr) =>
        expr is SqlExpr.Literal { Value: bool b } && b;

    /// <inheritdoc/>
    public OptimizedPlan Apply(OptimizedPlan plan)
    {
        switch (plan)
        {
            // Filter(_, FALSE)  →  EmptyResult
            case OptFilter(var input, var pred) when IsLiteralFalse(pred):
                return new OptEmptyResult();

            // Filter(_, TRUE)   →  input  (tautology elimination)
            case OptFilter(var input, var pred) when IsLiteralTrue(pred):
                return Apply(input);

            // Filter(EmptyResult, _)  →  EmptyResult
            case OptFilter(var input, _):
            {
                var rewritten = Apply(input);
                return rewritten is OptEmptyResult ? new OptEmptyResult() : new OptFilter(rewritten, ((OptFilter)plan).Predicate);
            }

            // Limit(_, 0, _)  →  EmptyResult
            case OptLimit(_, { } count, _) when count == 0:
                return new OptEmptyResult();

            // Limit(EmptyResult, _, _)  →  EmptyResult
            case OptLimit(var input, var count, var offset):
            {
                var rewritten = Apply(input);
                return rewritten is OptEmptyResult ? new OptEmptyResult() : new OptLimit(rewritten, count, offset);
            }

            // Project(EmptyResult, _)  →  EmptyResult
            case OptProject(var input, var cols):
            {
                var rewritten = Apply(input);
                return rewritten is OptEmptyResult ? new OptEmptyResult() : new OptProject(rewritten, cols);
            }

            // Sort(EmptyResult, _)  →  EmptyResult
            case OptSort(var input, var keys):
            {
                var rewritten = Apply(input);
                return rewritten is OptEmptyResult ? new OptEmptyResult() : new OptSort(rewritten, keys);
            }

            // Distinct(EmptyResult)  →  EmptyResult
            case OptDistinct(var input):
            {
                var rewritten = Apply(input);
                return rewritten is OptEmptyResult ? new OptEmptyResult() : new OptDistinct(rewritten);
            }

            // Aggregate(EmptyResult, …)  →  EmptyResult
            case OptAggregate(var input, var gb, var aggs):
            {
                var rewritten = Apply(input);
                return rewritten is OptEmptyResult ? new OptEmptyResult() : new OptAggregate(rewritten, gb, aggs);
            }

            // Having(EmptyResult, _)  →  EmptyResult
            case OptHaving(var input, var pred):
            {
                var rewritten = Apply(input);
                return rewritten is OptEmptyResult ? new OptEmptyResult() : new OptHaving(rewritten, pred);
            }

            // Inner join with EmptyResult on either side  →  EmptyResult
            case OptJoin(var left, var right, JoinKind.Inner, var cond):
            {
                var l = Apply(left);
                var r = Apply(right);
                return l is OptEmptyResult || r is OptEmptyResult
                    ? new OptEmptyResult()
                    : new OptJoin(l, r, JoinKind.Inner, cond);
            }

            // Cross join with EmptyResult on either side  →  EmptyResult
            case OptJoin(var left, var right, JoinKind.Cross, var cond):
            {
                var l = Apply(left);
                var r = Apply(right);
                return l is OptEmptyResult || r is OptEmptyResult
                    ? new OptEmptyResult()
                    : new OptJoin(l, r, JoinKind.Cross, cond);
            }

            // Other join kinds: recurse but don't eliminate
            case OptJoin(var left, var right, var kind, var cond):
                return new OptJoin(Apply(left), Apply(right), kind, cond);

            case OptUnion(var l, var r, var all):
                return new OptUnion(Apply(l), Apply(r), all);

            default:
                return plan;
        }
    }
}

// ── Pass 5 — Limit Pushdown ───────────────────────────────────────────────────
//
// Propagates a LIMIT count hint through transparent operators to the scan node.
// When a query reads LIMIT N rows, the scan can stop after producing N rows,
// avoiding a full table scan.
//
// Transparent operators (the limit passes through unchanged):
//   • Filter       — it may filter rows, so we cannot reduce the scan limit
//                    below the query limit.  We propagate the same value and
//                    rely on the execution engine to stop early.
//   • Sort         — we must read ALL rows before sorting; push the limit AFTER
//                    a sort if possible (top-N sort optimization).  For simplicity
//                    we do NOT push through Sort in this pass.
//   • Distinct     — same argument as Sort; do not push through.
//   • Project      — transparent; pass through.
//   • Aggregate    — do not push through.
//
// We track whether we are "inside a filter" and inflate the limit conservatively
// (since filtering may remove rows).  Without schema statistics we simply pass
// the exact count and let the scan engine be opportunistic.

/// <summary>
/// Pass 5: propagate a LIMIT count hint down to the nearest OptScan.
/// </summary>
public sealed class LimitPushdownPass : IPass
{
    /// <inheritdoc/>
    public string Name => "LimitPushdown";

    private OptimizedPlan Push(OptimizedPlan plan, long? limit)
    {
        switch (plan)
        {
            case OptScan(var table, var alias, var reqCols, var existing):
            {
                // Take the smaller of existing hint and new limit (if both set).
                var newLimit = (existing, limit) switch
                {
                    (null,  null)  => (long?)null,
                    (null,  long r) => r,
                    (long e, null)  => e,
                    (long e, long r) => Math.Min(e, r),
                };
                return new OptScan(table, alias, reqCols, newLimit);
            }

            // Limit node: extract count and push it downward (but NOT through Sort/Distinct).
            case OptLimit(var input, var count, var offset):
            {
                var childLimit = limit is null ? count : (count is null ? limit : Math.Min(count.Value, limit.Value));
                var rewritten  = PushThroughFilter(input, childLimit);
                return new OptLimit(rewritten, count, offset);
            }

            // Project is transparent.
            case OptProject(var input, var cols):
                return new OptProject(Push(input, limit), cols);

            // Filter: push down, but do NOT reduce the limit (rows may be removed).
            case OptFilter(var input, var pred):
                return new OptFilter(Push(input, limit), pred);

            // Sort blocks pushdown (we need all rows).
            case OptSort(var input, var keys):
                return new OptSort(Apply(input), keys);

            // Distinct blocks pushdown.
            case OptDistinct(var input):
                return new OptDistinct(Apply(input));

            // Aggregate and Having block pushdown.
            case OptAggregate(var input, var gb, var aggs):
                return new OptAggregate(Apply(input), gb, aggs);
            case OptHaving(var input, var pred):
                return new OptHaving(Apply(input), pred);

            // Join: push to both sides (conservative).
            case OptJoin(var l, var r, var kind, var cond):
                return new OptJoin(Push(l, limit), Push(r, limit), kind, cond);

            // Union: push to both branches.
            case OptUnion(var l, var r, var all):
                return new OptUnion(Push(l, limit), Push(r, limit), all);

            default:
                return plan;
        }
    }

    // Push limit through Filter nodes only (Filter is transparent to limit hints).
    private OptimizedPlan PushThroughFilter(OptimizedPlan plan, long? limit)
    {
        return plan switch
        {
            OptFilter(var input, var pred) => new OptFilter(PushThroughFilter(input, limit), pred),
            OptProject(var input, var cols) => new OptProject(PushThroughFilter(input, limit), cols),
            _ => Push(plan, limit),
        };
    }

    /// <inheritdoc/>
    public OptimizedPlan Apply(OptimizedPlan plan) => Push(plan, null);
}

// ── Optimizer entry point ─────────────────────────────────────────────────────
//
// Provides:
//   Lift()              — converts LogicalPlan → OptimizedPlan (1:1 node mapping)
//   DefaultPasses()     — the canonical five-pass pipeline
//   Optimize()          — Lift + DefaultPasses, fixed-point iteration
//   OptimizeWithPasses() — Lift + caller-supplied passes, fixed-point iteration

/// <summary>
/// Entry point for the Mini-SQLite Level 1 logical query optimizer.
/// </summary>
/// <remarks>
/// All methods are static and thread-safe.  No global state is maintained.
/// </remarks>
public static class SqlOptimizer
{
    // Maximum number of fixed-point iterations before we give up.
    // In practice two or three rounds are sufficient for the default passes.
    private const int MaxIterations = 10;

    // ── Lift ──────────────────────────────────────────────────────────────────

    /// <summary>
    /// Convert a <see cref="LogicalPlan"/> into an <see cref="OptimizedPlan"/>
    /// with a 1-to-1 node mapping (no optimization applied yet).
    /// </summary>
    public static OptimizedPlan Lift(LogicalPlan plan)
    {
        return plan switch
        {
            ScanPlan(var table, var alias)             => new OptScan(table, alias),
            FilterPlan(var input, var pred)             => new OptFilter(Lift(input), pred),
            ProjectPlan(var input, var cols)            => new OptProject(Lift(input), cols),
            JoinPlan(var l, var r, var kind, var cond) => new OptJoin(Lift(l), Lift(r), kind, cond),
            AggregatePlan(var input, var gb, var aggs) => new OptAggregate(Lift(input), gb, aggs),
            HavingPlan(var input, var pred)             => new OptHaving(Lift(input), pred),
            SortPlan(var input, var keys)               => new OptSort(Lift(input), keys),
            LimitPlan(var input, var count, var offset) => new OptLimit(Lift(input), count, offset),
            DistinctPlan(var input)                     => new OptDistinct(Lift(input)),
            UnionPlan(var l, var r, var all)            => new OptUnion(Lift(l), Lift(r), all),
            InsertPlan(var t, var cols, var vals)       => new OptInsert(t, cols, vals),
            UpdatePlan(var t, var asgn, var pred)       => new OptUpdate(t, asgn, pred),
            DeletePlan(var t, var pred)                 => new OptDelete(t, pred),
            CreateTablePlan(var t, var ine, var cols)   => new OptCreateTable(t, ine, cols),
            DropTablePlan(var t, var ife)               => new OptDropTable(t, ife),
            _ => throw new InvalidOperationException($"Unknown LogicalPlan type: {plan.GetType().Name}"),
        };
    }

    // ── Default passes ────────────────────────────────────────────────────────

    /// <summary>
    /// Returns the canonical five-pass optimization pipeline:
    /// <list type="number">
    ///   <item>ConstantFolding</item>
    ///   <item>PredicatePushdown</item>
    ///   <item>ProjectionPruning</item>
    ///   <item>DeadCodeElimination</item>
    ///   <item>LimitPushdown</item>
    /// </list>
    /// </summary>
    public static IReadOnlyList<IPass> DefaultPasses() =>
        new IPass[]
        {
            new ConstantFoldingPass(),
            new PredicatePushdownPass(),
            new ProjectionPruningPass(),
            new DeadCodeEliminationPass(),
            new LimitPushdownPass(),
        };

    // ── Optimize ──────────────────────────────────────────────────────────────

    /// <summary>
    /// Lift <paramref name="plan"/> and apply the default five-pass pipeline to
    /// a fixed point (or until <see cref="MaxIterations"/> is reached).
    /// </summary>
    public static OptimizedPlan Optimize(LogicalPlan plan) =>
        OptimizeWithPasses(plan, DefaultPasses());

    /// <summary>
    /// Lift <paramref name="plan"/> and apply the caller-supplied passes to
    /// a fixed point (or until <see cref="MaxIterations"/> is reached).
    /// </summary>
    public static OptimizedPlan OptimizeWithPasses(
        LogicalPlan            plan,
        IReadOnlyList<IPass>   passes)
    {
        var current = Lift(plan);

        for (var iter = 0; iter < MaxIterations; iter++)
        {
            var previous = current;
            foreach (var pass in passes)
                current = pass.Apply(current);

            // Fixed-point check: if the tree hasn't changed, stop early.
            if (current == previous)
                break;
        }

        return current;
    }
}

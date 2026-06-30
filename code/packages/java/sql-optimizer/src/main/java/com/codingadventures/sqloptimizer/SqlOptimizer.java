package com.codingadventures.sqloptimizer;

import com.codingadventures.sqlplanner.SqlPlanner;

import java.util.ArrayList;
import java.util.Collections;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

// SqlOptimizer.java — logical query plan optimizer for SQL.
//
// The optimizer transforms a LogicalPlan (produced by SqlPlanner) into an
// OptimizedPlan by running a fixed sequence of analysis passes.  Each pass
// is a pure function: it takes an OptimizedPlan and returns a new (possibly
// smaller, flatter, or annotated) OptimizedPlan.
//
// The five default passes, in order:
//
//   1. ConstantFolding      — evaluate compile-time constants, propagate NULL
//   2. PredicatePushdown    — move filters closer to the data they filter
//   3. ProjectionPruning    — annotate Scans with only the columns they need
//   4. DeadCodeElimination  — replace provably-empty subtrees with EmptyResult
//   5. LimitPushdown        — annotate Scans with an early-stop row count
//
// All passes compose cleanly: the output of one is the input of the next.
// Passes are idempotent: running them twice produces the same result as once.
//
// Usage:
//   OptimizedPlan opt = SqlOptimizer.optimize(planner.plan(stmt));

public final class SqlOptimizer {

    // Private constructor: this class is a static-method namespace only.
    private SqlOptimizer() {}

    // ── OptimizedPlan sealed interface ────────────────────────────────────────
    //
    // OptimizedPlan mirrors LogicalPlan but:
    //   • Scan gains two nullable annotations set by the optimizer passes:
    //       - requiredColumns: column names this scan must actually emit
    //       - scanLimit:       the optimizer has proven the caller needs at
    //                          most this many rows
    //   • EmptyResult is a new leaf indicating zero rows will ever be produced.
    //     Dead-code elimination introduces it wherever a predicate is statically
    //     false or a LIMIT 0 is encountered.

    public sealed interface OptimizedPlan
        permits OptimizedPlan.Scan, OptimizedPlan.Filter, OptimizedPlan.Project,
                OptimizedPlan.Join, OptimizedPlan.Aggregate, OptimizedPlan.Having,
                OptimizedPlan.Sort, OptimizedPlan.Limit, OptimizedPlan.Distinct,
                OptimizedPlan.Union, OptimizedPlan.Insert, OptimizedPlan.Update,
                OptimizedPlan.Delete, OptimizedPlan.CreateTable, OptimizedPlan.DropTable,
                OptimizedPlan.EmptyResult {

        /**
         * A base table scan.
         * <p>
         * {@code requiredColumns} — when non-null, only these column names need to
         * be read from storage.  Null means "all columns" (SELECT *).
         * <p>
         * {@code scanLimit} — when non-null, the engine may stop reading after this
         * many rows.  Only correct when there is no ordering requirement above.
         */
        record Scan(String table, String alias,
                    List<String> requiredColumns, Long scanLimit)
            implements OptimizedPlan {

            // Convenience constructor used by lift() and tests.
            public Scan(String table, String alias) {
                this(table, alias, null, null);
            }
        }

        record Filter(OptimizedPlan input, SqlPlanner.SqlExpr predicate)
            implements OptimizedPlan {}

        record Project(OptimizedPlan input, List<SqlPlanner.OutputColumn> columns)
            implements OptimizedPlan {}

        record Join(OptimizedPlan left, OptimizedPlan right,
                    SqlPlanner.JoinKind kind, SqlPlanner.SqlExpr condition)
            implements OptimizedPlan {}

        record Aggregate(OptimizedPlan input,
                         List<SqlPlanner.SqlExpr> groupBy,
                         List<SqlPlanner.AggregateItem> aggregates)
            implements OptimizedPlan {}

        record Having(OptimizedPlan input, SqlPlanner.SqlExpr predicate)
            implements OptimizedPlan {}

        record Sort(OptimizedPlan input, List<SqlPlanner.SortKey> keys)
            implements OptimizedPlan {}

        record Limit(OptimizedPlan input, Long count, Long offset)
            implements OptimizedPlan {}

        record Distinct(OptimizedPlan input) implements OptimizedPlan {}

        record Union(OptimizedPlan left, OptimizedPlan right, boolean all)
            implements OptimizedPlan {}

        record Insert(String table, List<String> columns,
                      List<List<SqlPlanner.SqlExpr>> values)
            implements OptimizedPlan {}

        record Update(String table, List<SqlPlanner.Assignment> assignments,
                      SqlPlanner.SqlExpr predicate)
            implements OptimizedPlan {}

        record Delete(String table, SqlPlanner.SqlExpr predicate)
            implements OptimizedPlan {}

        record CreateTable(String table, boolean ifNotExists,
                           List<SqlPlanner.ColumnDef> columns)
            implements OptimizedPlan {}

        record DropTable(String table, boolean ifExists)
            implements OptimizedPlan {}

        /**
         * A sentinel node meaning "this subtree produces zero rows".
         * Introduced by DeadCodeElimination; propagated upward through operators
         * that cannot manufacture rows from nothing (Filter, Sort, Project, etc.).
         */
        record EmptyResult() implements OptimizedPlan {}
    }

    // ── Pass interface ────────────────────────────────────────────────────────
    //
    // A Pass is any tree transformation.  Passes are composable and named so
    // that a pipeline can be described in log output.

    public interface Pass {
        /** A short human-readable label for this pass, e.g. "ConstantFolding". */
        String name();

        /**
         * Transform {@code plan} into a (possibly new) OptimizedPlan.
         * Must not modify the input; must return a structurally equal plan
         * if no transformation applies.
         */
        OptimizedPlan apply(OptimizedPlan plan);
    }

    // ── Public API ─────────────────────────────────────────────────────────────

    /**
     * Convert a LogicalPlan to an OptimizedPlan and run all five default passes.
     * This is the primary entry point for callers.
     */
    public static OptimizedPlan optimize(SqlPlanner.LogicalPlan plan) {
        return optimizeWithPasses(plan, defaultPasses());
    }

    /**
     * Convert a LogicalPlan to an OptimizedPlan and run the supplied passes
     * in order.  An empty list is valid (returns the lifted plan unchanged).
     */
    public static OptimizedPlan optimizeWithPasses(SqlPlanner.LogicalPlan plan,
                                                    List<Pass> passes) {
        OptimizedPlan current = lift(plan);
        for (var pass : passes) {
            current = pass.apply(current);
        }
        return current;
    }

    /**
     * Returns the five default optimization passes in their canonical order:
     *   ConstantFolding → PredicatePushdown → ProjectionPruning →
     *   DeadCodeElimination → LimitPushdown
     */
    public static List<Pass> defaultPasses() {
        return List.of(
            new ConstantFolding(),
            new PredicatePushdown(),
            new ProjectionPruning(),
            new DeadCodeElimination(),
            new LimitPushdown()
        );
    }

    /**
     * Convert a LogicalPlan tree to an OptimizedPlan tree without running any
     * optimization passes.  Scan nodes are annotated with (null, null) for
     * requiredColumns and scanLimit.
     */
    public static OptimizedPlan lift(SqlPlanner.LogicalPlan plan) {
        return switch (plan) {
            case SqlPlanner.LogicalPlan.Scan(var t, var a) ->
                new OptimizedPlan.Scan(t, a, null, null);

            case SqlPlanner.LogicalPlan.Filter(var input, var pred) ->
                new OptimizedPlan.Filter(lift(input), pred);

            case SqlPlanner.LogicalPlan.Project(var input, var cols) ->
                new OptimizedPlan.Project(lift(input), cols);

            case SqlPlanner.LogicalPlan.Join(var l, var r, var kind, var cond) ->
                new OptimizedPlan.Join(lift(l), lift(r), kind, cond);

            case SqlPlanner.LogicalPlan.Aggregate(var input, var gb, var aggs) ->
                new OptimizedPlan.Aggregate(lift(input), gb, aggs);

            case SqlPlanner.LogicalPlan.Having(var input, var pred) ->
                new OptimizedPlan.Having(lift(input), pred);

            case SqlPlanner.LogicalPlan.Sort(var input, var keys) ->
                new OptimizedPlan.Sort(lift(input), keys);

            case SqlPlanner.LogicalPlan.Limit(var input, var count, var offset) ->
                new OptimizedPlan.Limit(lift(input), count, offset);

            case SqlPlanner.LogicalPlan.Distinct(var input) ->
                new OptimizedPlan.Distinct(lift(input));

            case SqlPlanner.LogicalPlan.Union(var l, var r, var all) ->
                new OptimizedPlan.Union(lift(l), lift(r), all);

            case SqlPlanner.LogicalPlan.Insert(var tbl, var cols, var vals) ->
                new OptimizedPlan.Insert(tbl, cols, vals);

            case SqlPlanner.LogicalPlan.Update(var tbl, var asgns, var pred) ->
                new OptimizedPlan.Update(tbl, asgns, pred);

            case SqlPlanner.LogicalPlan.Delete(var tbl, var pred) ->
                new OptimizedPlan.Delete(tbl, pred);

            case SqlPlanner.LogicalPlan.CreateTable(var tbl, var ine, var colDefs) ->
                new OptimizedPlan.CreateTable(tbl, ine, colDefs);

            case SqlPlanner.LogicalPlan.DropTable(var tbl, var ie) ->
                new OptimizedPlan.DropTable(tbl, ie);
        };
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Pass 1 — Constant Folding
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // Evaluates expressions that can be resolved at compile time:
    //
    //   • Arithmetic on two literal numbers    (1 + 2 → 3)
    //   • Comparison of two literal values     (1 < 2 → true)
    //   • Short-circuit Boolean logic          (x AND false → false)
    //   • NULL propagation                     (NULL + 5 → NULL)
    //   • Unary NOT / NEG on literals          (NOT true → false)
    //   • IS NULL / IS NOT NULL on literals    (IS NULL(null) → true)
    //
    // The pass is bottom-up: children are folded before parents, so folded
    // sub-expressions become available for further folding above them.

    public static final class ConstantFolding implements Pass {
        @Override public String name() { return "ConstantFolding"; }

        @Override
        public OptimizedPlan apply(OptimizedPlan plan) {
            return transformPlan(plan);
        }

        // ── Plan-level traversal ─────────────────────────────────────────────

        private OptimizedPlan transformPlan(OptimizedPlan plan) {
            return switch (plan) {
                case OptimizedPlan.Filter(var input, var pred) ->
                    new OptimizedPlan.Filter(transformPlan(input), foldExpr(pred));

                case OptimizedPlan.Project(var input, var cols) ->
                    new OptimizedPlan.Project(transformPlan(input), foldOutputCols(cols));

                case OptimizedPlan.Join(var l, var r, var kind, var cond) ->
                    new OptimizedPlan.Join(transformPlan(l), transformPlan(r), kind,
                                           cond != null ? foldExpr(cond) : null);

                case OptimizedPlan.Aggregate(var input, var gb, var aggs) ->
                    new OptimizedPlan.Aggregate(transformPlan(input),
                                                gb.stream().map(this::foldExpr).toList(),
                                                aggs);

                case OptimizedPlan.Having(var input, var pred) ->
                    new OptimizedPlan.Having(transformPlan(input), foldExpr(pred));

                case OptimizedPlan.Sort(var input, var keys) ->
                    new OptimizedPlan.Sort(transformPlan(input),
                                           keys.stream().map(k ->
                                               new SqlPlanner.SortKey(foldExpr(k.keyExpr()),
                                                                       k.direction(),
                                                                       k.nullOrder())).toList());

                case OptimizedPlan.Limit(var input, var count, var offset) ->
                    new OptimizedPlan.Limit(transformPlan(input), count, offset);

                case OptimizedPlan.Distinct(var input) ->
                    new OptimizedPlan.Distinct(transformPlan(input));

                case OptimizedPlan.Union(var l, var r, var all) ->
                    new OptimizedPlan.Union(transformPlan(l), transformPlan(r), all);

                // Leaf nodes and DML — no scalar expressions in their "body"
                // that benefit from folding (INSERT values could be folded but
                // it is not required for optimizer correctness).
                default -> plan;
            };
        }

        private List<SqlPlanner.OutputColumn> foldOutputCols(List<SqlPlanner.OutputColumn> cols) {
            var out = new ArrayList<SqlPlanner.OutputColumn>(cols.size());
            for (var c : cols) {
                out.add(switch (c) {
                    case SqlPlanner.OutputColumn.Star s -> s;
                    case SqlPlanner.OutputColumn.Expr(var expr, var alias) ->
                        new SqlPlanner.OutputColumn.Expr(foldExpr(expr), alias);
                });
            }
            return out;
        }

        // ── Expression folding (bottom-up) ───────────────────────────────────

        SqlPlanner.SqlExpr foldExpr(SqlPlanner.SqlExpr expr) {
            return switch (expr) {
                // ── Recurse and then try to fold the result ──────────────────
                case SqlPlanner.SqlExpr.BinaryOp(var op, var l, var r) -> {
                    var fl = foldExpr(l);
                    var fr = foldExpr(r);
                    yield foldBinary(op, fl, fr);
                }

                case SqlPlanner.SqlExpr.UnaryOp(var op, var operand) -> {
                    var fo = foldExpr(operand);
                    yield foldUnary(op, fo);
                }

                case SqlPlanner.SqlExpr.IsNull(var operand) -> {
                    var fo = foldExpr(operand);
                    if (fo instanceof SqlPlanner.SqlExpr.Literal(var v))
                        yield new SqlPlanner.SqlExpr.Literal(v == null);
                    yield new SqlPlanner.SqlExpr.IsNull(fo);
                }

                case SqlPlanner.SqlExpr.IsNotNull(var operand) -> {
                    var fo = foldExpr(operand);
                    if (fo instanceof SqlPlanner.SqlExpr.Literal(var v))
                        yield new SqlPlanner.SqlExpr.Literal(v != null);
                    yield new SqlPlanner.SqlExpr.IsNotNull(fo);
                }

                case SqlPlanner.SqlExpr.FuncCall(var name, var args) ->
                    new SqlPlanner.SqlExpr.FuncCall(name, args.stream().map(this::foldExpr).toList());

                case SqlPlanner.SqlExpr.Between(var v, var lo, var hi) ->
                    new SqlPlanner.SqlExpr.Between(foldExpr(v), foldExpr(lo), foldExpr(hi));

                case SqlPlanner.SqlExpr.In(var v, var items) ->
                    new SqlPlanner.SqlExpr.In(foldExpr(v), items.stream().map(this::foldExpr).toList());

                case SqlPlanner.SqlExpr.NotIn(var v, var items) ->
                    new SqlPlanner.SqlExpr.NotIn(foldExpr(v), items.stream().map(this::foldExpr).toList());

                // Leaves — nothing to fold.
                default -> expr;
            };
        }

        // ── Binary folding ───────────────────────────────────────────────────

        private SqlPlanner.SqlExpr foldBinary(SqlPlanner.BinaryOperator op,
                                               SqlPlanner.SqlExpr fl,
                                               SqlPlanner.SqlExpr fr) {
            // Short-circuit Boolean rules that don't require BOTH sides to be literals.
            //
            //   TRUE  AND x  → x        FALSE AND x → FALSE
            //   TRUE  OR  x  → TRUE     FALSE OR  x → x
            //   x     AND TRUE → x      x     AND FALSE → FALSE
            //   x     OR  TRUE → TRUE   x     OR  FALSE → x

            if (op == SqlPlanner.BinaryOperator.AND) {
                if (isLiteral(fl, Boolean.FALSE)) return lit(false);
                if (isLiteral(fr, Boolean.FALSE)) return lit(false);
                if (isLiteral(fl, Boolean.TRUE))  return fr;
                if (isLiteral(fr, Boolean.TRUE))  return fl;
            }

            if (op == SqlPlanner.BinaryOperator.OR) {
                if (isLiteral(fl, Boolean.TRUE))  return lit(true);
                if (isLiteral(fr, Boolean.TRUE))  return lit(true);
                if (isLiteral(fl, Boolean.FALSE)) return fr;
                if (isLiteral(fr, Boolean.FALSE)) return fl;
            }

            // NULL propagation — anything op NULL → NULL
            // (after AND/OR short-circuit checks above so that TRUE OR NULL → TRUE).
            if (isNull(fl) || isNull(fr)) return lit(null);

            // From here both sides must be non-null literals for folding to apply.
            if (!(fl instanceof SqlPlanner.SqlExpr.Literal(var lv)) ||
                !(fr instanceof SqlPlanner.SqlExpr.Literal(var rv))) {
                return new SqlPlanner.SqlExpr.BinaryOp(op, fl, fr);
            }

            return switch (op) {
                case ADD  -> numericOp(lv, rv, (a, b) -> a + b, (a, b) -> a + b);
                case SUB  -> numericOp(lv, rv, (a, b) -> a - b, (a, b) -> a - b);
                case MUL  -> numericOp(lv, rv, (a, b) -> a * b, (a, b) -> a * b);
                case DIV  -> {
                    // Division by zero is left for the VM to handle at runtime.
                    if ((rv instanceof Long && (Long) rv == 0L) ||
                        (rv instanceof Double && (Double) rv == 0.0))
                        yield new SqlPlanner.SqlExpr.BinaryOp(op, fl, fr);
                    yield numericOp(lv, rv,
                        (a, b) -> a / b,
                        (a, b) -> a / b);
                }
                case MOD  -> {
                    if ((rv instanceof Long && (Long) rv == 0L) ||
                        (rv instanceof Double && (Double) rv == 0.0))
                        yield new SqlPlanner.SqlExpr.BinaryOp(op, fl, fr);
                    yield numericOp(lv, rv,
                        (a, b) -> a % b,
                        (a, b) -> a % b);
                }
                case EQ    -> lit(compareValues(lv, rv) == 0);
                case NOT_EQ -> lit(compareValues(lv, rv) != 0);
                case LT    -> lit(compareValues(lv, rv) < 0);
                case LTE   -> lit(compareValues(lv, rv) <= 0);
                case GT    -> lit(compareValues(lv, rv) > 0);
                case GTE   -> lit(compareValues(lv, rv) >= 0);
                // AND / OR with two non-null, non-boolean literals shouldn't
                // reach here after the short-circuit guard above, but be safe.
                case AND   -> {
                    if (lv instanceof Boolean lb && rv instanceof Boolean rb)
                        yield lit(lb && rb);
                    yield new SqlPlanner.SqlExpr.BinaryOp(op, fl, fr);
                }
                case OR    -> {
                    if (lv instanceof Boolean lb && rv instanceof Boolean rb)
                        yield lit(lb || rb);
                    yield new SqlPlanner.SqlExpr.BinaryOp(op, fl, fr);
                }
            };
        }

        // ── Unary folding ────────────────────────────────────────────────────

        private SqlPlanner.SqlExpr foldUnary(SqlPlanner.UnaryOperator op,
                                              SqlPlanner.SqlExpr fo) {
            if (!(fo instanceof SqlPlanner.SqlExpr.Literal(var v)))
                return new SqlPlanner.SqlExpr.UnaryOp(op, fo);

            if (v == null) return lit(null);    // NULL propagation

            return switch (op) {
                case NOT -> {
                    if (v instanceof Boolean b) yield lit(!b);
                    yield new SqlPlanner.SqlExpr.UnaryOp(op, fo);
                }
                case NEG -> {
                    if (v instanceof Long l) yield lit(-l);
                    if (v instanceof Double d) yield lit(-d);
                    if (v instanceof Integer i) yield lit((long) -i);
                    yield new SqlPlanner.SqlExpr.UnaryOp(op, fo);
                }
            };
        }

        // ── Arithmetic helpers ────────────────────────────────────────────────

        @FunctionalInterface interface LongBinaryOp  { long  apply(long  a, long  b); }
        @FunctionalInterface interface DoubleBinaryOp { double apply(double a, double b); }

        private SqlPlanner.SqlExpr numericOp(Object lv, Object rv,
                                              LongBinaryOp longOp,
                                              DoubleBinaryOp doubleOp) {
            if (lv instanceof Long l && rv instanceof Long r)
                return lit(longOp.apply(l, r));
            if (lv instanceof Integer li && rv instanceof Integer ri)
                return lit(longOp.apply(li.longValue(), ri.longValue()));
            if (lv instanceof Integer li && rv instanceof Long r)
                return lit(longOp.apply(li.longValue(), r));
            if (lv instanceof Long l && rv instanceof Integer ri)
                return lit(longOp.apply(l, ri.longValue()));

            double dl = toDouble(lv), dr = toDouble(rv);
            return lit(doubleOp.apply(dl, dr));
        }

        private static double toDouble(Object v) {
            if (v instanceof Double d) return d;
            if (v instanceof Float f)  return f.doubleValue();
            if (v instanceof Long l)   return l.doubleValue();
            if (v instanceof Integer i) return i.doubleValue();
            throw new IllegalArgumentException("Not a number: " + v);
        }

        // ── Comparison helper ─────────────────────────────────────────────────
        //
        // Compares two non-null literal values.  Numbers are compared by
        // promoting both to double so that comparing Long and Double works.

        @SuppressWarnings("unchecked")
        private static int compareValues(Object a, Object b) {
            if (a instanceof Number na && b instanceof Number nb)
                return Double.compare(na.doubleValue(), nb.doubleValue());
            if (a instanceof Comparable ca) {
                // Guard against mixed-type comparison (e.g. String vs Long): the
                // unchecked cast inside compareTo would throw ClassCastException
                // with an opaque message.  We detect the mismatch early and
                // report it clearly.
                if (!a.getClass().isInstance(b))
                    throw new IllegalArgumentException(
                        "Cannot compare values of different types: "
                        + a.getClass().getSimpleName() + " vs "
                        + b.getClass().getSimpleName());
                return ca.compareTo(b);
            }
            throw new IllegalArgumentException("Cannot compare " + a + " with " + b);
        }

        // ── Convenience constructors ─────────────────────────────────────────

        private static SqlPlanner.SqlExpr lit(Object v) {
            return new SqlPlanner.SqlExpr.Literal(v);
        }

        private static boolean isLiteral(SqlPlanner.SqlExpr e, Object expected) {
            return e instanceof SqlPlanner.SqlExpr.Literal(var v) && expected.equals(v);
        }

        private static boolean isNull(SqlPlanner.SqlExpr e) {
            return e instanceof SqlPlanner.SqlExpr.Literal(var v) && v == null;
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Pass 2 — Predicate Pushdown
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // Moves Filter nodes downward (closer to the Scan they filter).
    // A filter pushed closer to its data source reduces the number of rows that
    // flow upward through the rest of the plan.
    //
    // The pass splits AND conjuncts and tries to place each one as low as
    // possible.  The routing rules are:
    //
    //   • Filter → Sort or Distinct   : always safe; push through
    //   • Filter → Project            : push through (conservative: only when the
    //                                   predicate can be resolved against the
    //                                   scan aliases below the project)
    //   • Filter → Join               : push conjunct to whichever side owns all
    //                                   of the referenced column aliases:
    //                                     LEFT  JOIN  : only push to left side
    //                                     RIGHT JOIN  : only push to right side
    //                                     FULL  JOIN  : keep above (push neither)
    //                                     INNER/CROSS : push to either/both
    //   • Filter → Aggregate, Limit,
    //              Union, Having      : do NOT push (semantics change or it's wrong)

    public static final class PredicatePushdown implements Pass {
        @Override public String name() { return "PredicatePushdown"; }

        @Override
        public OptimizedPlan apply(OptimizedPlan plan) {
            return transformPlan(plan);
        }

        // ── Plan traversal ───────────────────────────────────────────────────

        private OptimizedPlan transformPlan(OptimizedPlan plan) {
            return switch (plan) {
                // The interesting case: try to push the filter downward.
                case OptimizedPlan.Filter(var input, var pred) -> {
                    var pushedInput = transformPlan(input);   // recurse first
                    yield pushFilter(pushedInput, pred);
                }

                // Non-filter nodes: just recurse into their children.
                case OptimizedPlan.Project(var input, var cols) ->
                    new OptimizedPlan.Project(transformPlan(input), cols);

                case OptimizedPlan.Join(var l, var r, var kind, var cond) ->
                    new OptimizedPlan.Join(transformPlan(l), transformPlan(r), kind, cond);

                case OptimizedPlan.Aggregate(var input, var gb, var aggs) ->
                    new OptimizedPlan.Aggregate(transformPlan(input), gb, aggs);

                case OptimizedPlan.Having(var input, var pred) ->
                    new OptimizedPlan.Having(transformPlan(input), pred);

                case OptimizedPlan.Sort(var input, var keys) ->
                    new OptimizedPlan.Sort(transformPlan(input), keys);

                case OptimizedPlan.Limit(var input, var count, var offset) ->
                    new OptimizedPlan.Limit(transformPlan(input), count, offset);

                case OptimizedPlan.Distinct(var input) ->
                    new OptimizedPlan.Distinct(transformPlan(input));

                case OptimizedPlan.Union(var l, var r, var all) ->
                    new OptimizedPlan.Union(transformPlan(l), transformPlan(r), all);

                default -> plan;
            };
        }

        // ── Core pushdown logic ───────────────────────────────────────────────

        /**
         * Attempt to push {@code pred} through {@code input}.
         * If it cannot be pushed, wraps it in a new Filter node.
         */
        private OptimizedPlan pushFilter(OptimizedPlan input, SqlPlanner.SqlExpr pred) {
            // Split the predicate into AND conjuncts; each is pushed independently.
            var conjuncts = splitAnd(pred);
            return pushConjuncts(input, conjuncts);
        }

        private OptimizedPlan pushConjuncts(OptimizedPlan input,
                                             List<SqlPlanner.SqlExpr> conjuncts) {
            return switch (input) {
                // Push through Sort: Sort(Filter(x)) = Filter(Sort(x)) because
                // ordering does not depend on row count.
                case OptimizedPlan.Sort(var child, var keys) -> {
                    var pushed = pushConjuncts(child, conjuncts);
                    yield new OptimizedPlan.Sort(pushed, keys);
                }

                // Push through Distinct: same reasoning — predicate doesn't affect
                // which distinct values exist, only how many rows we keep.
                case OptimizedPlan.Distinct(var child) -> {
                    var pushed = pushConjuncts(child, conjuncts);
                    yield new OptimizedPlan.Distinct(pushed);
                }

                // Push through Project conservatively: push only if the predicate's
                // column references can be satisfied by the aliases visible below
                // the project.
                case OptimizedPlan.Project(var child, var cols) -> {
                    var belowAliases = collectAliases(child);
                    var canPush  = new ArrayList<SqlPlanner.SqlExpr>();
                    var mustKeep = new ArrayList<SqlPlanner.SqlExpr>();
                    for (var c : conjuncts) {
                        var refs = columnAliases(c);
                        // If all referenced aliases exist below the project, push.
                        if (refs.isEmpty() || belowAliases.containsAll(refs))
                            canPush.add(c);
                        else
                            mustKeep.add(c);
                    }
                    OptimizedPlan result = new OptimizedPlan.Project(
                        pushConjuncts(child, canPush), cols);
                    result = wrapFilters(result, mustKeep);
                    yield result;
                }

                // Push through Join — conjuncts are routed by alias ownership.
                case OptimizedPlan.Join(var left, var right, var kind, var cond) -> {
                    var leftAliases  = collectAliases(left);
                    var rightAliases = collectAliases(right);

                    var toLeft   = new ArrayList<SqlPlanner.SqlExpr>();
                    var toRight  = new ArrayList<SqlPlanner.SqlExpr>();
                    var keepHere = new ArrayList<SqlPlanner.SqlExpr>();

                    for (var c : conjuncts) {
                        var refs = columnAliases(c);
                        boolean refsLeft  = !Collections.disjoint(refs, leftAliases)  || (refs.isEmpty());
                        boolean refsRight = !Collections.disjoint(refs, rightAliases) || (refs.isEmpty());

                        // A conjunct that references no columns (e.g. literal) can go
                        // to either side; we route it left by convention.
                        if (refs.isEmpty()) { toLeft.add(c); continue; }

                        boolean onlyLeft  = leftAliases.containsAll(refs);
                        boolean onlyRight = rightAliases.containsAll(refs);
                        boolean bothSides = refsLeft && refsRight && !onlyLeft && !onlyRight;

                        if (bothSides) {
                            keepHere.add(c);
                        } else if (onlyLeft) {
                            // Outer join safety: don't push to left of RIGHT JOIN or either of FULL.
                            if (kind == SqlPlanner.JoinKind.RIGHT || kind == SqlPlanner.JoinKind.FULL)
                                keepHere.add(c);
                            else
                                toLeft.add(c);
                        } else if (onlyRight) {
                            // Don't push to right of LEFT JOIN or either of FULL.
                            if (kind == SqlPlanner.JoinKind.LEFT || kind == SqlPlanner.JoinKind.FULL)
                                keepHere.add(c);
                            else
                                toRight.add(c);
                        } else {
                            keepHere.add(c);
                        }
                    }

                    var newLeft  = pushConjuncts(left,  toLeft);
                    var newRight = pushConjuncts(right, toRight);
                    OptimizedPlan result = new OptimizedPlan.Join(newLeft, newRight, kind, cond);
                    result = wrapFilters(result, keepHere);
                    yield result;
                }

                // Cannot push through Aggregate, Limit, Having, Union, or leaf nodes.
                default -> wrapFilters(input, conjuncts);
            };
        }

        // ── Helpers ──────────────────────────────────────────────────────────

        /** Wrap {@code plan} in Filter nodes for each conjunct in order. */
        private static OptimizedPlan wrapFilters(OptimizedPlan plan,
                                                  List<SqlPlanner.SqlExpr> conjuncts) {
            var result = plan;
            for (var c : conjuncts)
                result = new OptimizedPlan.Filter(result, c);
            return result;
        }

        /** Split an expression on AND into its top-level conjuncts. */
        static List<SqlPlanner.SqlExpr> splitAnd(SqlPlanner.SqlExpr expr) {
            var parts = new ArrayList<SqlPlanner.SqlExpr>();
            splitAndInto(expr, parts);
            return parts;
        }

        private static void splitAndInto(SqlPlanner.SqlExpr expr,
                                          List<SqlPlanner.SqlExpr> acc) {
            if (expr instanceof SqlPlanner.SqlExpr.BinaryOp(var op, var l, var r)
                && op == SqlPlanner.BinaryOperator.AND) {
                splitAndInto(l, acc);
                splitAndInto(r, acc);
            } else {
                acc.add(expr);
            }
        }

        /**
         * Collect all table-alias names that appear in Scan nodes under {@code plan}.
         * These are the aliases that predicates can reference to be pushed below.
         */
        static Set<String> collectAliases(OptimizedPlan plan) {
            var aliases = new HashSet<String>();
            collectAliasesInto(plan, aliases);
            return aliases;
        }

        private static void collectAliasesInto(OptimizedPlan plan, Set<String> acc) {
            switch (plan) {
                case OptimizedPlan.Scan(var t, var a, var rc, var sl) -> {
                    acc.add(a != null ? a : t);
                }
                case OptimizedPlan.Filter(var input, var p) -> collectAliasesInto(input, acc);
                case OptimizedPlan.Project(var input, var c) -> collectAliasesInto(input, acc);
                case OptimizedPlan.Join(var l, var r, var k, var c) -> {
                    collectAliasesInto(l, acc);
                    collectAliasesInto(r, acc);
                }
                case OptimizedPlan.Aggregate(var input, var g, var ag) -> collectAliasesInto(input, acc);
                case OptimizedPlan.Having(var input, var p) -> collectAliasesInto(input, acc);
                case OptimizedPlan.Sort(var input, var k) -> collectAliasesInto(input, acc);
                case OptimizedPlan.Limit(var input, var c, var o) -> collectAliasesInto(input, acc);
                case OptimizedPlan.Distinct(var input) -> collectAliasesInto(input, acc);
                case OptimizedPlan.Union(var l, var r, var al) -> {
                    collectAliasesInto(l, acc);
                    collectAliasesInto(r, acc);
                }
                default -> {} // leaf DML nodes
            }
        }

        /**
         * Return the set of table qualifiers (the "table" field of Column nodes)
         * used anywhere in {@code expr}.  An empty set means the expression has
         * no Column references at all.
         */
        static Set<String> columnAliases(SqlPlanner.SqlExpr expr) {
            var refs = new HashSet<String>();
            collectColumnAliasesInto(expr, refs);
            return refs;
        }

        private static void collectColumnAliasesInto(SqlPlanner.SqlExpr expr,
                                                       Set<String> acc) {
            switch (expr) {
                case SqlPlanner.SqlExpr.Column(var t, var c) -> { if (t != null) acc.add(t); }
                case SqlPlanner.SqlExpr.BinaryOp(var op, var l, var r) -> {
                    collectColumnAliasesInto(l, acc);
                    collectColumnAliasesInto(r, acc);
                }
                case SqlPlanner.SqlExpr.UnaryOp(var op, var o) -> collectColumnAliasesInto(o, acc);
                case SqlPlanner.SqlExpr.FuncCall(var n, var args) ->
                    args.forEach(a -> collectColumnAliasesInto(a, acc));
                case SqlPlanner.SqlExpr.IsNull(var o)    -> collectColumnAliasesInto(o, acc);
                case SqlPlanner.SqlExpr.IsNotNull(var o) -> collectColumnAliasesInto(o, acc);
                case SqlPlanner.SqlExpr.Between(var v, var lo, var hi) -> {
                    collectColumnAliasesInto(v, acc);
                    collectColumnAliasesInto(lo, acc);
                    collectColumnAliasesInto(hi, acc);
                }
                case SqlPlanner.SqlExpr.In(var v, var items) -> {
                    collectColumnAliasesInto(v, acc);
                    items.forEach(i -> collectColumnAliasesInto(i, acc));
                }
                case SqlPlanner.SqlExpr.NotIn(var v, var items) -> {
                    collectColumnAliasesInto(v, acc);
                    items.forEach(i -> collectColumnAliasesInto(i, acc));
                }
                case SqlPlanner.SqlExpr.Like(var v, var p) -> collectColumnAliasesInto(v, acc);
                case SqlPlanner.SqlExpr.NotLike(var v, var p) -> collectColumnAliasesInto(v, acc);
                default -> {} // Literal, Wildcard, AggExpr — no column refs
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Pass 3 — Projection Pruning
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // Determines which columns each Scan actually needs to produce.  Columns
    // that are never referenced above a Scan can be dropped from storage reads.
    //
    // The pass is top-down: it threads a "required" set of (table, column) pairs
    // downward.  At each Project it recomputes required = union of all column refs
    // in the project expressions; at each Filter / Sort / Join it adds the column
    // refs in the predicate / sort keys / join condition to the required set.
    //
    // A null required set means "all columns needed" (SELECT *, or we have no
    // information).  A Wildcard() anywhere in the project list also disables
    // pruning.

    public static final class ProjectionPruning implements Pass {

        // A simple pair record to track (table-alias, column-name) requirements.
        record ColRef(String table, String column) {}

        @Override public String name() { return "ProjectionPruning"; }

        @Override
        public OptimizedPlan apply(OptimizedPlan plan) {
            // null required → "all columns needed"
            return transformPlan(plan, null);
        }

        private OptimizedPlan transformPlan(OptimizedPlan plan, Set<ColRef> required) {
            return switch (plan) {
                // At a Project: compute required from the project expressions
                // (everything the project references from below).
                case OptimizedPlan.Project(var input, var cols) -> {
                    if (hasWildcard(cols)) {
                        // SELECT * — we can't prune anything.
                        yield new OptimizedPlan.Project(transformPlan(input, null), cols);
                    }
                    var newRequired = new HashSet<ColRef>();
                    for (var c : cols) {
                        if (c instanceof SqlPlanner.OutputColumn.Expr(var expr, var alias))
                            collectColRefs(expr, newRequired);
                    }
                    // Union with any required passed down from above.
                    if (required != null) newRequired.addAll(required);
                    yield new OptimizedPlan.Project(transformPlan(input, newRequired), cols);
                }

                // At a Filter: add predicate column refs to required.
                case OptimizedPlan.Filter(var input, var pred) -> {
                    var newRequired = required == null ? null : new HashSet<>(required);
                    if (newRequired != null) collectColRefs(pred, newRequired);
                    yield new OptimizedPlan.Filter(transformPlan(input, newRequired), pred);
                }

                // At a Sort: add sort-key column refs to required.
                case OptimizedPlan.Sort(var input, var keys) -> {
                    var newRequired = required == null ? null : new HashSet<>(required);
                    if (newRequired != null)
                        for (var k : keys) collectColRefs(k.keyExpr(), newRequired);
                    yield new OptimizedPlan.Sort(transformPlan(input, newRequired), keys);
                }

                // At a Join: add condition column refs, then split required by alias.
                case OptimizedPlan.Join(var left, var right, var kind, var cond) -> {
                    var newRequired = required == null ? null : new HashSet<>(required);
                    if (newRequired != null && cond != null)
                        collectColRefs(cond, newRequired);

                    var leftAliases  = collectAliasSet(left);
                    var rightAliases = collectAliasSet(right);

                    Set<ColRef> leftReq  = splitRequired(newRequired, leftAliases);
                    Set<ColRef> rightReq = splitRequired(newRequired, rightAliases);

                    yield new OptimizedPlan.Join(
                        transformPlan(left,  leftReq),
                        transformPlan(right, rightReq),
                        kind, cond);
                }

                // At an Aggregate: required = GROUP BY refs + aggregate arg refs.
                case OptimizedPlan.Aggregate(var input, var gb, var aggs) -> {
                    var newRequired = required == null ? null : new HashSet<ColRef>();
                    if (newRequired != null) {
                        for (var e : gb) collectColRefs(e, newRequired);
                        for (var agg : aggs) {
                            if (agg.arg() instanceof SqlPlanner.AggArg.Expr(var expr))
                                collectColRefs(expr, newRequired);
                        }
                    }
                    yield new OptimizedPlan.Aggregate(transformPlan(input, newRequired), gb, aggs);
                }

                // At Having, Limit, Distinct: pass through unchanged.
                case OptimizedPlan.Having(var input, var pred) -> {
                    var newRequired = required == null ? null : new HashSet<>(required);
                    if (newRequired != null) collectColRefs(pred, newRequired);
                    yield new OptimizedPlan.Having(transformPlan(input, newRequired), pred);
                }

                case OptimizedPlan.Limit(var input, var count, var offset) ->
                    new OptimizedPlan.Limit(transformPlan(input, required), count, offset);

                case OptimizedPlan.Distinct(var input) ->
                    new OptimizedPlan.Distinct(transformPlan(input, required));

                case OptimizedPlan.Union(var l, var r, var all) ->
                    new OptimizedPlan.Union(transformPlan(l, required), transformPlan(r, required), all);

                // At a Scan: annotate with the columns required by this scan's alias.
                case OptimizedPlan.Scan(var tbl, var alias, var existingRC, var sl) -> {
                    if (required == null) {
                        yield new OptimizedPlan.Scan(tbl, alias, null, sl);
                    }
                    var scanAlias = alias != null ? alias : tbl;
                    var cols = new ArrayList<String>();
                    for (var cr : required) {
                        if (scanAlias.equals(cr.table()))
                            cols.add(cr.column());
                    }
                    if (cols.isEmpty()) {
                        // No specific columns needed → might be a COUNT(*) situation;
                        // keep null to avoid confusing the engine.
                        yield new OptimizedPlan.Scan(tbl, alias, null, sl);
                    }
                    Collections.sort(cols);
                    yield new OptimizedPlan.Scan(tbl, alias, cols, sl);
                }

                default -> plan;
            };
        }

        // ── Helpers ──────────────────────────────────────────────────────────

        private static boolean hasWildcard(List<SqlPlanner.OutputColumn> cols) {
            for (var c : cols)
                if (c instanceof SqlPlanner.OutputColumn.Star) return true;
            return false;
        }

        static void collectColRefs(SqlPlanner.SqlExpr expr, Set<ColRef> acc) {
            switch (expr) {
                case SqlPlanner.SqlExpr.Column(var t, var c) -> {
                    if (t != null) acc.add(new ColRef(t, c));
                }
                case SqlPlanner.SqlExpr.BinaryOp(var op, var l, var r) -> {
                    collectColRefs(l, acc);
                    collectColRefs(r, acc);
                }
                case SqlPlanner.SqlExpr.UnaryOp(var op, var o) -> collectColRefs(o, acc);
                case SqlPlanner.SqlExpr.FuncCall(var n, var args) ->
                    args.forEach(a -> collectColRefs(a, acc));
                case SqlPlanner.SqlExpr.IsNull(var o)    -> collectColRefs(o, acc);
                case SqlPlanner.SqlExpr.IsNotNull(var o) -> collectColRefs(o, acc);
                case SqlPlanner.SqlExpr.Between(var v, var lo, var hi) -> {
                    collectColRefs(v, acc);
                    collectColRefs(lo, acc);
                    collectColRefs(hi, acc);
                }
                case SqlPlanner.SqlExpr.In(var v, var items) -> {
                    collectColRefs(v, acc);
                    items.forEach(i -> collectColRefs(i, acc));
                }
                case SqlPlanner.SqlExpr.NotIn(var v, var items) -> {
                    collectColRefs(v, acc);
                    items.forEach(i -> collectColRefs(i, acc));
                }
                case SqlPlanner.SqlExpr.Like(var v, var p) -> collectColRefs(v, acc);
                case SqlPlanner.SqlExpr.NotLike(var v, var p) -> collectColRefs(v, acc);
                default -> {} // Literal, Wildcard, AggExpr.Star — no col refs
            }
        }

        private static Set<String> collectAliasSet(OptimizedPlan plan) {
            var aliases = new HashSet<String>();
            collectAliasSetInto(plan, aliases);
            return aliases;
        }

        private static void collectAliasSetInto(OptimizedPlan plan, Set<String> acc) {
            switch (plan) {
                case OptimizedPlan.Scan(var t, var a, var rc, var sl) ->
                    acc.add(a != null ? a : t);
                case OptimizedPlan.Filter(var input, var p) -> collectAliasSetInto(input, acc);
                case OptimizedPlan.Project(var input, var c) -> collectAliasSetInto(input, acc);
                case OptimizedPlan.Join(var l, var r, var k, var c) -> {
                    collectAliasSetInto(l, acc);
                    collectAliasSetInto(r, acc);
                }
                case OptimizedPlan.Aggregate(var input, var g, var ag) ->
                    collectAliasSetInto(input, acc);
                case OptimizedPlan.Having(var input, var p) -> collectAliasSetInto(input, acc);
                case OptimizedPlan.Sort(var input, var k) -> collectAliasSetInto(input, acc);
                case OptimizedPlan.Limit(var input, var c, var o) -> collectAliasSetInto(input, acc);
                case OptimizedPlan.Distinct(var input) -> collectAliasSetInto(input, acc);
                case OptimizedPlan.Union(var l, var r, var al) -> {
                    collectAliasSetInto(l, acc);
                    collectAliasSetInto(r, acc);
                }
                default -> {}
            }
        }

        private static Set<ColRef> splitRequired(Set<ColRef> required, Set<String> aliases) {
            if (required == null) return null;
            var out = new HashSet<ColRef>();
            for (var cr : required)
                if (aliases.contains(cr.table()))
                    out.add(cr);
            return out;
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Pass 4 — Dead Code Elimination
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // Replaces subtrees that provably produce zero rows with EmptyResult,
    // then propagates EmptyResult upward through operators that cannot conjure
    // rows from nothing.
    //
    // Rules (bottom-up):
    //
    //   Filter(EmptyResult, _)          → EmptyResult
    //   Filter(child, Literal(false))   → EmptyResult
    //   Filter(child, Literal(null))    → EmptyResult
    //   Filter(child, Literal(true))    → child          (remove useless filter)
    //   Limit(child, count=0, _)        → EmptyResult
    //   Project(EmptyResult, _)         → EmptyResult
    //   Sort(EmptyResult, _)            → EmptyResult
    //   Limit(EmptyResult, _, _)        → EmptyResult
    //   Distinct(EmptyResult)           → EmptyResult
    //   Having(EmptyResult, _)          → EmptyResult
    //   Join(EmptyResult, _, INNER/CROSS) → EmptyResult
    //   Join(_, EmptyResult, INNER/CROSS) → EmptyResult
    //   Union(EmptyResult, right)       → right
    //   Union(left, EmptyResult)        → left
    //
    // NOTE: Aggregate(EmptyResult) is intentionally NOT collapsed.
    // "SELECT COUNT(*) FROM empty_table" must return one row with count 0.

    public static final class DeadCodeElimination implements Pass {
        @Override public String name() { return "DeadCodeElimination"; }

        @Override
        public OptimizedPlan apply(OptimizedPlan plan) {
            return transformPlan(plan);
        }

        private OptimizedPlan transformPlan(OptimizedPlan plan) {
            return switch (plan) {
                case OptimizedPlan.Filter(var input, var pred) -> {
                    var child = transformPlan(input);

                    // If the child is already empty, propagate.
                    if (child instanceof OptimizedPlan.EmptyResult) yield child;

                    // If the predicate is statically false or null, no rows pass.
                    if (isFalsy(pred)) yield new OptimizedPlan.EmptyResult();

                    // If the predicate is statically true, the filter is a no-op.
                    if (isTrue(pred)) yield child;

                    yield new OptimizedPlan.Filter(child, pred);
                }

                case OptimizedPlan.Project(var input, var cols) -> {
                    var child = transformPlan(input);
                    if (child instanceof OptimizedPlan.EmptyResult) yield child;
                    yield new OptimizedPlan.Project(child, cols);
                }

                case OptimizedPlan.Sort(var input, var keys) -> {
                    var child = transformPlan(input);
                    if (child instanceof OptimizedPlan.EmptyResult) yield child;
                    yield new OptimizedPlan.Sort(child, keys);
                }

                case OptimizedPlan.Limit(var input, var count, var offset) -> {
                    var child = transformPlan(input);
                    if (child instanceof OptimizedPlan.EmptyResult) yield child;
                    // LIMIT 0 → no rows will ever be produced.
                    if (count != null && count == 0L) yield new OptimizedPlan.EmptyResult();
                    yield new OptimizedPlan.Limit(child, count, offset);
                }

                case OptimizedPlan.Distinct(var input) -> {
                    var child = transformPlan(input);
                    if (child instanceof OptimizedPlan.EmptyResult) yield child;
                    yield new OptimizedPlan.Distinct(child);
                }

                case OptimizedPlan.Having(var input, var pred) -> {
                    var child = transformPlan(input);
                    if (child instanceof OptimizedPlan.EmptyResult) yield child;
                    yield new OptimizedPlan.Having(child, pred);
                }

                case OptimizedPlan.Join(var left, var right, var kind, var cond) -> {
                    var lc = transformPlan(left);
                    var rc = transformPlan(right);
                    // INNER and CROSS joins produce no rows when either side is empty.
                    boolean innerLike = kind == SqlPlanner.JoinKind.INNER
                                     || kind == SqlPlanner.JoinKind.CROSS;
                    if (innerLike &&
                        (lc instanceof OptimizedPlan.EmptyResult ||
                         rc instanceof OptimizedPlan.EmptyResult))
                        yield new OptimizedPlan.EmptyResult();

                    yield new OptimizedPlan.Join(lc, rc, kind, cond);
                }

                case OptimizedPlan.Union(var left, var right, var all) -> {
                    var lc = transformPlan(left);
                    var rc = transformPlan(right);
                    // Union with one empty side collapses to the other side.
                    if (lc instanceof OptimizedPlan.EmptyResult) yield rc;
                    if (rc instanceof OptimizedPlan.EmptyResult) yield lc;
                    yield new OptimizedPlan.Union(lc, rc, all);
                }

                // Aggregate(EmptyResult) is NOT collapsed — see class-level note.
                case OptimizedPlan.Aggregate(var input, var gb, var aggs) -> {
                    var child = transformPlan(input);
                    yield new OptimizedPlan.Aggregate(child, gb, aggs);
                }

                case OptimizedPlan.Scan s -> s;
                case OptimizedPlan.EmptyResult e -> e;
                default -> plan;
            };
        }

        // ── Predicate helpers ─────────────────────────────────────────────────

        /** Returns true if the expression is statically false or null. */
        private static boolean isFalsy(SqlPlanner.SqlExpr expr) {
            if (expr instanceof SqlPlanner.SqlExpr.Literal(var v))
                return Boolean.FALSE.equals(v) || v == null;
            return false;
        }

        /** Returns true if the expression is statically true. */
        private static boolean isTrue(SqlPlanner.SqlExpr expr) {
            if (expr instanceof SqlPlanner.SqlExpr.Literal(var v))
                return Boolean.TRUE.equals(v);
            return false;
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Pass 5 — Limit Pushdown
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // Propagates LIMIT counts down to Scan nodes so the storage layer can
    // short-circuit after reading enough rows.
    //
    // A LIMIT N with no offset (or offset == 0) tells us the plan needs at most
    // N rows from its source.  We push N downward through Project and Filter
    // until we reach a Scan.  On the way we set Scan.scanLimit = min(N, existing).
    //
    // We must NOT push through:
    //   • Sort      — needs all rows to determine the first N
    //   • Aggregate — GROUP BY changes cardinality
    //   • Join      — cartesian product may expand rows
    //   • Distinct  — duplicates may collapse rows
    //   • Union     — row counts come from two sources
    //
    // When offset > 0 the effective early-stop is count + offset rows, but
    // pushing that is more complex and we simply skip the push to be safe.

    public static final class LimitPushdown implements Pass {
        @Override public String name() { return "LimitPushdown"; }

        @Override
        public OptimizedPlan apply(OptimizedPlan plan) {
            return transformPlan(plan, null);
        }

        /**
         * @param plan    the node to transform
         * @param limit   the maximum row count to propagate (null = no limit known)
         */
        private OptimizedPlan transformPlan(OptimizedPlan plan, Long limit) {
            return switch (plan) {
                // When we encounter a Limit node, extract its count (if offset == 0)
                // and propagate downward.
                case OptimizedPlan.Limit(var input, var count, var offset) -> {
                    // Only push when there is no offset (offset = null or 0).
                    boolean noOffset = offset == null || offset == 0L;
                    Long childLimit;
                    if (noOffset && count != null) {
                        // Avoid mixing Long/long in ternary (auto-unboxing NPE).
                        if (limit == null) {
                            childLimit = count;
                        } else {
                            childLimit = Math.min(limit, count);  // both non-null
                        }
                    } else {
                        childLimit = null;
                    }
                    yield new OptimizedPlan.Limit(transformPlan(input, childLimit),
                                                   count, offset);
                }

                // Project and Filter are transparent to row count — pass limit through.
                case OptimizedPlan.Project(var input, var cols) ->
                    new OptimizedPlan.Project(transformPlan(input, limit), cols);

                case OptimizedPlan.Filter(var input, var pred) ->
                    new OptimizedPlan.Filter(transformPlan(input, limit), pred);

                // At a Scan: apply the limit annotation.
                // Care: avoid mixing Long and long in ternaries (auto-unboxing NPE).
                case OptimizedPlan.Scan(var tbl, var alias, var rc, var existingSL) -> {
                    Long newSL;
                    if (existingSL == null) {
                        newSL = limit;                                   // may be null
                    } else if (limit == null) {
                        newSL = existingSL;
                    } else {
                        newSL = Math.min(existingSL, limit);             // both non-null
                    }
                    yield new OptimizedPlan.Scan(tbl, alias, rc, newSL);
                }

                // Barriers: recurse but reset limit to null so children are not
                // contaminated by an outer limit that doesn't apply to them.
                case OptimizedPlan.Sort(var input, var keys) ->
                    new OptimizedPlan.Sort(transformPlan(input, null), keys);

                case OptimizedPlan.Aggregate(var input, var gb, var aggs) ->
                    new OptimizedPlan.Aggregate(transformPlan(input, null), gb, aggs);

                case OptimizedPlan.Having(var input, var pred) ->
                    new OptimizedPlan.Having(transformPlan(input, null), pred);

                case OptimizedPlan.Distinct(var input) ->
                    new OptimizedPlan.Distinct(transformPlan(input, null));

                case OptimizedPlan.Join(var l, var r, var kind, var cond) ->
                    new OptimizedPlan.Join(transformPlan(l, null), transformPlan(r, null),
                                           kind, cond);

                case OptimizedPlan.Union(var l, var r, var all) ->
                    new OptimizedPlan.Union(transformPlan(l, null), transformPlan(r, null), all);

                // Leaf nodes / DML — nothing to propagate into.
                default -> plan;
            };
        }
    }
}

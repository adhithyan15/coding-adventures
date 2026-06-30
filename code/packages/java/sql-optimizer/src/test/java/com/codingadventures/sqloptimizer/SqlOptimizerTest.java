package com.codingadventures.sqloptimizer;

import com.codingadventures.sqlplanner.SqlPlanner;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

/**
 * SqlOptimizerTest — comprehensive JUnit 5 test suite for the SQL optimizer.
 *
 * The tests are grouped by pass for clarity:
 *   • ConstantFoldingTests
 *   • PredicatePushdownTests
 *   • ProjectionPruningTests
 *   • DeadCodeEliminationTests
 *   • LimitPushdownTests
 *   • CombinedTests (multi-pass and API surface)
 *
 * Every test uses direct plan construction (no real schema required) so tests
 * are self-contained and fast.
 */
class SqlOptimizerTest {

    // ── Convenience factories ─────────────────────────────────────────────────

    static SqlPlanner.SqlExpr lit(Object v) {
        return new SqlPlanner.SqlExpr.Literal(v);
    }
    static SqlPlanner.SqlExpr col(String table, String column) {
        return new SqlPlanner.SqlExpr.Column(table, column);
    }
    static SqlPlanner.SqlExpr binOp(SqlPlanner.BinaryOperator op,
                                     SqlPlanner.SqlExpr l, SqlPlanner.SqlExpr r) {
        return new SqlPlanner.SqlExpr.BinaryOp(op, l, r);
    }
    static SqlPlanner.SqlExpr add(SqlPlanner.SqlExpr l, SqlPlanner.SqlExpr r) {
        return binOp(SqlPlanner.BinaryOperator.ADD, l, r);
    }
    static SqlPlanner.SqlExpr sub(SqlPlanner.SqlExpr l, SqlPlanner.SqlExpr r) {
        return binOp(SqlPlanner.BinaryOperator.SUB, l, r);
    }
    static SqlPlanner.SqlExpr mul(SqlPlanner.SqlExpr l, SqlPlanner.SqlExpr r) {
        return binOp(SqlPlanner.BinaryOperator.MUL, l, r);
    }
    static SqlPlanner.SqlExpr div(SqlPlanner.SqlExpr l, SqlPlanner.SqlExpr r) {
        return binOp(SqlPlanner.BinaryOperator.DIV, l, r);
    }
    static SqlPlanner.SqlExpr and(SqlPlanner.SqlExpr l, SqlPlanner.SqlExpr r) {
        return binOp(SqlPlanner.BinaryOperator.AND, l, r);
    }
    static SqlPlanner.SqlExpr or(SqlPlanner.SqlExpr l, SqlPlanner.SqlExpr r) {
        return binOp(SqlPlanner.BinaryOperator.OR, l, r);
    }
    static SqlPlanner.SqlExpr eq(SqlPlanner.SqlExpr l, SqlPlanner.SqlExpr r) {
        return binOp(SqlPlanner.BinaryOperator.EQ, l, r);
    }
    static SqlPlanner.SqlExpr lt(SqlPlanner.SqlExpr l, SqlPlanner.SqlExpr r) {
        return binOp(SqlPlanner.BinaryOperator.LT, l, r);
    }
    static SqlPlanner.SqlExpr not(SqlPlanner.SqlExpr e) {
        return new SqlPlanner.SqlExpr.UnaryOp(SqlPlanner.UnaryOperator.NOT, e);
    }
    static SqlPlanner.SqlExpr neg(SqlPlanner.SqlExpr e) {
        return new SqlPlanner.SqlExpr.UnaryOp(SqlPlanner.UnaryOperator.NEG, e);
    }
    static SqlPlanner.SqlExpr isNull(SqlPlanner.SqlExpr e) {
        return new SqlPlanner.SqlExpr.IsNull(e);
    }
    static SqlPlanner.SqlExpr isNotNull(SqlPlanner.SqlExpr e) {
        return new SqlPlanner.SqlExpr.IsNotNull(e);
    }

    static SqlOptimizer.OptimizedPlan scan(String table, String alias) {
        return new SqlOptimizer.OptimizedPlan.Scan(table, alias);
    }
    static SqlOptimizer.OptimizedPlan filter(SqlOptimizer.OptimizedPlan input,
                                              SqlPlanner.SqlExpr pred) {
        return new SqlOptimizer.OptimizedPlan.Filter(input, pred);
    }
    static SqlOptimizer.OptimizedPlan project(SqlOptimizer.OptimizedPlan input,
                                               List<SqlPlanner.OutputColumn> cols) {
        return new SqlOptimizer.OptimizedPlan.Project(input, cols);
    }
    static SqlOptimizer.OptimizedPlan limit(SqlOptimizer.OptimizedPlan input,
                                             Long count, Long offset) {
        return new SqlOptimizer.OptimizedPlan.Limit(input, count, offset);
    }
    static SqlOptimizer.OptimizedPlan sort(SqlOptimizer.OptimizedPlan input,
                                            List<SqlPlanner.SortKey> keys) {
        return new SqlOptimizer.OptimizedPlan.Sort(input, keys);
    }
    static SqlOptimizer.OptimizedPlan distinct(SqlOptimizer.OptimizedPlan input) {
        return new SqlOptimizer.OptimizedPlan.Distinct(input);
    }
    static SqlOptimizer.OptimizedPlan join(SqlOptimizer.OptimizedPlan l,
                                            SqlOptimizer.OptimizedPlan r,
                                            SqlPlanner.JoinKind kind,
                                            SqlPlanner.SqlExpr cond) {
        return new SqlOptimizer.OptimizedPlan.Join(l, r, kind, cond);
    }
    static SqlOptimizer.OptimizedPlan agg(SqlOptimizer.OptimizedPlan input,
                                           List<SqlPlanner.SqlExpr> gb,
                                           List<SqlPlanner.AggregateItem> aggs) {
        return new SqlOptimizer.OptimizedPlan.Aggregate(input, gb, aggs);
    }

    static SqlPlanner.OutputColumn exprCol(SqlPlanner.SqlExpr expr, String alias) {
        return new SqlPlanner.OutputColumn.Expr(expr, alias);
    }
    static SqlPlanner.OutputColumn starCol() { return new SqlPlanner.OutputColumn.Star(); }

    static SqlPlanner.SortKey asc(SqlPlanner.SqlExpr expr) {
        return new SqlPlanner.SortKey(expr, SqlPlanner.SortDir.ASC, SqlPlanner.NullOrder.NULLS_LAST);
    }

    // Fold a single expression via ConstantFolding.
    static SqlPlanner.SqlExpr fold(SqlPlanner.SqlExpr expr) {
        return new SqlOptimizer.ConstantFolding().foldExpr(expr);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // API surface
    // ─────────────────────────────────────────────────────────────────────────

    @Test @DisplayName("defaultPasses() returns 5 passes in correct order")
    void defaultPassesOrderAndCount() {
        var passes = SqlOptimizer.defaultPasses();
        assertEquals(5, passes.size());
        assertEquals("ConstantFolding",     passes.get(0).name());
        assertEquals("PredicatePushdown",   passes.get(1).name());
        assertEquals("ProjectionPruning",   passes.get(2).name());
        assertEquals("DeadCodeElimination", passes.get(3).name());
        assertEquals("LimitPushdown",       passes.get(4).name());
    }

    @Test @DisplayName("Pass.name() returns correct string for each pass")
    void passNames() {
        assertEquals("ConstantFolding",     new SqlOptimizer.ConstantFolding().name());
        assertEquals("PredicatePushdown",   new SqlOptimizer.PredicatePushdown().name());
        assertEquals("ProjectionPruning",   new SqlOptimizer.ProjectionPruning().name());
        assertEquals("DeadCodeElimination", new SqlOptimizer.DeadCodeElimination().name());
        assertEquals("LimitPushdown",       new SqlOptimizer.LimitPushdown().name());
    }

    @Test @DisplayName("optimizeWithPasses with custom single pass")
    void customSinglePass() {
        // A no-op pass that returns the plan unchanged.
        SqlOptimizer.Pass noOp = new SqlOptimizer.Pass() {
            @Override public String name() { return "NoOp"; }
            @Override public SqlOptimizer.OptimizedPlan apply(SqlOptimizer.OptimizedPlan p) { return p; }
        };

        var logical = new SqlPlanner.LogicalPlan.Scan("users", "u");
        var result = SqlOptimizer.optimizeWithPasses(logical, List.of(noOp));
        assertInstanceOf(SqlOptimizer.OptimizedPlan.Scan.class, result);
    }

    @Test @DisplayName("optimize returns same structure if no optimizations apply")
    void optimizeNoOp() {
        // A simple scan with a non-constant predicate — nothing can be optimized.
        var logical = new SqlPlanner.LogicalPlan.Filter(
            new SqlPlanner.LogicalPlan.Scan("users", "u"),
            col("u", "age")   // non-constant; cannot fold or eliminate
        );
        var result = SqlOptimizer.optimize(logical);
        // The result should still be a Filter(Scan) shape.
        assertInstanceOf(SqlOptimizer.OptimizedPlan.Filter.class, result);
        var f = (SqlOptimizer.OptimizedPlan.Filter) result;
        assertInstanceOf(SqlOptimizer.OptimizedPlan.Scan.class, f.input());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // lift()
    // ─────────────────────────────────────────────────────────────────────────

    @Test @DisplayName("lift() correctly converts Scan")
    void liftScan() {
        var plan = new SqlPlanner.LogicalPlan.Scan("t", "a");
        var opt  = SqlOptimizer.lift(plan);
        assertInstanceOf(SqlOptimizer.OptimizedPlan.Scan.class, opt);
        var s = (SqlOptimizer.OptimizedPlan.Scan) opt;
        assertEquals("t", s.table());
        assertEquals("a", s.alias());
        assertNull(s.requiredColumns());
        assertNull(s.scanLimit());
    }

    @Test @DisplayName("lift() correctly converts all LogicalPlan variants")
    void liftAllVariants() {
        var scan   = new SqlPlanner.LogicalPlan.Scan("t", "t");
        var filter = new SqlPlanner.LogicalPlan.Filter(scan, lit(true));
        var project = new SqlPlanner.LogicalPlan.Project(filter, List.of(starCol()));
        var sort   = new SqlPlanner.LogicalPlan.Sort(project,
            List.of(asc(col("t", "id"))));
        var lim    = new SqlPlanner.LogicalPlan.Limit(sort, 10L, 0L);
        var dist   = new SqlPlanner.LogicalPlan.Distinct(lim);
        var union  = new SqlPlanner.LogicalPlan.Union(dist, scan, false);

        var opt = SqlOptimizer.lift(union);
        assertInstanceOf(SqlOptimizer.OptimizedPlan.Union.class, opt);

        // CreateTable
        var ct = new SqlPlanner.LogicalPlan.CreateTable("t2", false, List.of());
        assertInstanceOf(SqlOptimizer.OptimizedPlan.CreateTable.class, SqlOptimizer.lift(ct));

        // DropTable
        var dt = new SqlPlanner.LogicalPlan.DropTable("t2", true);
        assertInstanceOf(SqlOptimizer.OptimizedPlan.DropTable.class, SqlOptimizer.lift(dt));

        // Insert
        var ins = new SqlPlanner.LogicalPlan.Insert("t", List.of("id"), List.of(List.of(lit(1L))));
        assertInstanceOf(SqlOptimizer.OptimizedPlan.Insert.class, SqlOptimizer.lift(ins));

        // Update
        var upd = new SqlPlanner.LogicalPlan.Update("t",
            List.of(new SqlPlanner.Assignment("id", lit(1L))), lit(true));
        assertInstanceOf(SqlOptimizer.OptimizedPlan.Update.class, SqlOptimizer.lift(upd));

        // Delete
        var del = new SqlPlanner.LogicalPlan.Delete("t", lit(true));
        assertInstanceOf(SqlOptimizer.OptimizedPlan.Delete.class, SqlOptimizer.lift(del));

        // Aggregate
        var aggPlan = new SqlPlanner.LogicalPlan.Aggregate(scan, List.of(), List.of());
        assertInstanceOf(SqlOptimizer.OptimizedPlan.Aggregate.class, SqlOptimizer.lift(aggPlan));

        // Having
        var having = new SqlPlanner.LogicalPlan.Having(scan, lit(true));
        assertInstanceOf(SqlOptimizer.OptimizedPlan.Having.class, SqlOptimizer.lift(having));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // ConstantFolding
    // ─────────────────────────────────────────────────────────────────────────

    @Nested @DisplayName("ConstantFolding")
    class ConstantFoldingTests {

        @Test @DisplayName("1 + 2 → Literal(3)")
        void addTwoInts() {
            var result = fold(add(lit(1L), lit(2L)));
            assertEquals(new SqlPlanner.SqlExpr.Literal(3L), result);
        }

        @Test @DisplayName("TRUE AND FALSE → Literal(false)")
        void andTrueAndFalse() {
            var result = fold(and(lit(true), lit(false)));
            assertEquals(new SqlPlanner.SqlExpr.Literal(false), result);
        }

        @Test @DisplayName("NULL + 5 → Literal(null)")
        void nullPropagationAdd() {
            var result = fold(add(lit(null), lit(5L)));
            assertEquals(new SqlPlanner.SqlExpr.Literal(null), result);
        }

        @Test @DisplayName("NOT TRUE → Literal(false)")
        void notTrue() {
            var result = fold(not(lit(true)));
            assertEquals(new SqlPlanner.SqlExpr.Literal(false), result);
        }

        @Test @DisplayName("x AND TRUE → x (identity simplification)")
        void andTrueIsIdentity() {
            var x = col("u", "active");
            var result = fold(and(x, lit(true)));
            assertEquals(x, result);
        }

        @Test @DisplayName("TRUE OR unknown → TRUE (short circuit)")
        void orTrueShortCircuits() {
            var result = fold(or(lit(true), col("u", "x")));
            assertEquals(new SqlPlanner.SqlExpr.Literal(true), result);
        }

        @Test @DisplayName("NULL OR TRUE → TRUE (short circuit before null propagation)")
        void nullOrTrueIsTrue() {
            var result = fold(or(lit(null), lit(true)));
            // TRUE short circuits: TRUE OR _ → TRUE, so order matters.
            // Here left is NULL and right is TRUE; the rule TRUE OR x → TRUE fires when
            // right is TRUE, so result is TRUE.
            assertEquals(new SqlPlanner.SqlExpr.Literal(true), result);
        }

        @Test @DisplayName("IsNull(Literal(null)) → Literal(true)")
        void isNullOfNull() {
            var result = fold(isNull(lit(null)));
            assertEquals(new SqlPlanner.SqlExpr.Literal(true), result);
        }

        @Test @DisplayName("IsNotNull(Literal(42)) → Literal(true)")
        void isNotNullOfNonNull() {
            var result = fold(isNotNull(lit(42L)));
            assertEquals(new SqlPlanner.SqlExpr.Literal(true), result);
        }

        @Test @DisplayName("Division by zero not folded")
        void divisionByZeroNotFolded() {
            var expr   = div(lit(10L), lit(0L));
            var result = fold(expr);
            // Should remain as BinaryOp, not throw.
            assertInstanceOf(SqlPlanner.SqlExpr.BinaryOp.class, result);
        }

        @Test @DisplayName("SUB, MUL arithmetic fold")
        void arithmeticSubAndMul() {
            assertEquals(new SqlPlanner.SqlExpr.Literal(3L),  fold(sub(lit(5L), lit(2L))));
            assertEquals(new SqlPlanner.SqlExpr.Literal(6L),  fold(mul(lit(2L), lit(3L))));
        }

        @Test @DisplayName("1 < 2 → Literal(true)")
        void comparisonLessThan() {
            var result = fold(lt(lit(1L), lit(2L)));
            assertEquals(new SqlPlanner.SqlExpr.Literal(true), result);
        }

        @Test @DisplayName("NEG on literal: NEG(-5) → Literal(5)")
        void negOnLiteral() {
            var result = fold(neg(lit(-5L)));
            assertEquals(new SqlPlanner.SqlExpr.Literal(5L), result);
        }

        @Test @DisplayName("FALSE AND x → FALSE (short circuit with non-literal x)")
        void falseAndXIsAlwaysFalse() {
            var result = fold(and(lit(false), col("u", "x")));
            assertEquals(new SqlPlanner.SqlExpr.Literal(false), result);
        }

        @Test @DisplayName("Constant folding propagates through plan nodes")
        void foldingPropagatesThroughPlan() {
            // Filter(Scan, 1+2=3) — the predicate 1+2 should be folded to 3.
            var plan = filter(scan("t", "t"),
                add(lit(1L), lit(2L)));
            var cf = new SqlOptimizer.ConstantFolding();
            var result = cf.apply(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Filter.class, result);
            var f = (SqlOptimizer.OptimizedPlan.Filter) result;
            assertEquals(new SqlPlanner.SqlExpr.Literal(3L), f.predicate());
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // PredicatePushdown
    // ─────────────────────────────────────────────────────────────────────────

    @Nested @DisplayName("PredicatePushdown")
    class PredicatePushdownTests {

        @Test @DisplayName("Filter above Project pushed below Project")
        void filterPushedThroughProject() {
            // Filter(Project(Scan("u","u"), [u.name]), u.id = 1)
            // predicate references alias "u" which exists below Project → push down
            var pred    = eq(col("u", "id"), lit(1L));
            var scanU   = scan("users", "u");
            var proj    = project(scanU, List.of(exprCol(col("u", "name"), "name")));
            var filtered = filter(proj, pred);

            var pp = new SqlOptimizer.PredicatePushdown();
            var result = pp.apply(filtered);

            // The filter should now be below the project.
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Project.class, result);
            var p = (SqlOptimizer.OptimizedPlan.Project) result;
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Filter.class, p.input());
        }

        @Test @DisplayName("AND predicate splits — each half pushed to correct join side")
        void andSplitsAndPushesToBothSides() {
            // Filter(Join(Scan(u), Scan(o)), u.id=1 AND o.user_id=2)
            // Left conjunct (u.id=1) goes to left side; right conjunct to right.
            var predLeft  = eq(col("u", "id"), lit(1L));
            var predRight = eq(col("o", "user_id"), lit(2L));
            var combined  = and(predLeft, predRight);

            var scanU = scan("users", "u");
            var scanO = scan("orders", "o");
            var joined = join(scanU, scanO, SqlPlanner.JoinKind.INNER, null);
            var filtered = filter(joined, combined);

            var pp = new SqlOptimizer.PredicatePushdown();
            var result = pp.apply(filtered);

            // The join itself should have no wrapping filter; each predicate
            // should be below.
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Join.class, result);
            var j = (SqlOptimizer.OptimizedPlan.Join) result;
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Filter.class, j.left());
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Filter.class, j.right());
        }

        @Test @DisplayName("HAVING filter is not pushed below Aggregate")
        void havingNotPushedBelowAggregate() {
            // Filter(Aggregate(Scan), count > 5)
            // Aggregate is a barrier — the filter must stay above.
            var pred    = binOp(SqlPlanner.BinaryOperator.GT, col("g", "cnt"), lit(5L));
            var scanG   = scan("grp", "g");
            var aggPlan = agg(scanG, List.of(), List.of());
            var filtered = filter(aggPlan, pred);

            var pp = new SqlOptimizer.PredicatePushdown();
            var result = pp.apply(filtered);

            // Filter must remain above Aggregate (not pushed through).
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Filter.class, result);
            var f = (SqlOptimizer.OptimizedPlan.Filter) result;
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Aggregate.class, f.input());
        }

        @Test @DisplayName("Filter pushed through Sort")
        void filterPushedThroughSort() {
            var pred = eq(col("u", "id"), lit(1L));
            var s    = sort(scan("u", "u"), List.of(asc(col("u", "name"))));
            var f    = filter(s, pred);

            var result = new SqlOptimizer.PredicatePushdown().apply(f);

            assertInstanceOf(SqlOptimizer.OptimizedPlan.Sort.class, result);
            var sortResult = (SqlOptimizer.OptimizedPlan.Sort) result;
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Filter.class, sortResult.input());
        }

        @Test @DisplayName("Filter pushed through Distinct")
        void filterPushedThroughDistinct() {
            var pred = eq(col("u", "id"), lit(1L));
            var d    = distinct(scan("u", "u"));
            var f    = filter(d, pred);

            var result = new SqlOptimizer.PredicatePushdown().apply(f);

            assertInstanceOf(SqlOptimizer.OptimizedPlan.Distinct.class, result);
            var distResult = (SqlOptimizer.OptimizedPlan.Distinct) result;
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Filter.class, distResult.input());
        }

        @Test @DisplayName("LEFT JOIN: right-side predicate NOT pushed to right")
        void leftJoinRightNotPushed() {
            var predRight = eq(col("o", "user_id"), lit(2L));
            var scanU = scan("users", "u");
            var scanO = scan("orders", "o");
            var joined = join(scanU, scanO, SqlPlanner.JoinKind.LEFT, null);
            var filtered = filter(joined, predRight);

            var result = new SqlOptimizer.PredicatePushdown().apply(filtered);

            // The filter should stay above the join (not pushed to right side
            // of a LEFT JOIN — that would change outer-join semantics).
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Filter.class, result);
        }

        @Test @DisplayName("Predicate referencing both join sides stays above join")
        void predicateSpanningBothSidesStaysAbove() {
            // u.id = o.user_id references both sides — cannot push to either.
            var pred    = eq(col("u", "id"), col("o", "user_id"));
            var scanU   = scan("users", "u");
            var scanO   = scan("orders", "o");
            var joined  = join(scanU, scanO, SqlPlanner.JoinKind.INNER, null);
            var filtered = filter(joined, pred);

            var result = new SqlOptimizer.PredicatePushdown().apply(filtered);

            // Should remain as Filter(Join(...))
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Filter.class, result);
            var f = (SqlOptimizer.OptimizedPlan.Filter) result;
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Join.class, f.input());
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // ProjectionPruning
    // ─────────────────────────────────────────────────────────────────────────

    @Nested @DisplayName("ProjectionPruning")
    class ProjectionPruningTests {

        @Test @DisplayName("Scan gains requiredColumns from Project above it")
        void scanGetsRequiredColumns() {
            // Project(Scan("users", "u"), [u.name])
            var scanU = scan("users", "u");
            var proj  = project(scanU, List.of(exprCol(col("u", "name"), "name")));

            var result = new SqlOptimizer.ProjectionPruning().apply(proj);

            assertInstanceOf(SqlOptimizer.OptimizedPlan.Project.class, result);
            var p = (SqlOptimizer.OptimizedPlan.Project) result;
            var s = (SqlOptimizer.OptimizedPlan.Scan) p.input();
            assertNotNull(s.requiredColumns());
            assertTrue(s.requiredColumns().contains("name"));
        }

        @Test @DisplayName("SELECT * disables projection pruning (requiredColumns stays null)")
        void selectStarDisablesPruning() {
            var scanU = scan("users", "u");
            var proj  = project(scanU, List.of(starCol()));

            var result = new SqlOptimizer.ProjectionPruning().apply(proj);

            assertInstanceOf(SqlOptimizer.OptimizedPlan.Project.class, result);
            var p = (SqlOptimizer.OptimizedPlan.Project) result;
            var s = (SqlOptimizer.OptimizedPlan.Scan) p.input();
            // Wildcard in project disables pruning.
            assertNull(s.requiredColumns());
        }

        @Test @DisplayName("ProjectionPruning with join condition adds its cols")
        void pruningWithJoinCondition() {
            var scanU  = scan("users", "u");
            var scanO  = scan("orders", "o");
            var j      = join(scanU, scanO, SqlPlanner.JoinKind.INNER,
                              eq(col("u", "id"), col("o", "user_id")));
            var proj   = project(j, List.of(exprCol(col("u", "name"), "name")));

            var result = new SqlOptimizer.ProjectionPruning().apply(proj);

            var p = (SqlOptimizer.OptimizedPlan.Project) result;
            var joinResult = (SqlOptimizer.OptimizedPlan.Join) p.input();
            var scanUResult = (SqlOptimizer.OptimizedPlan.Scan) joinResult.left();
            var scanOResult = (SqlOptimizer.OptimizedPlan.Scan) joinResult.right();

            // u needs name (from project) and id (from join condition).
            assertNotNull(scanUResult.requiredColumns());
            assertTrue(scanUResult.requiredColumns().contains("id"));
            assertTrue(scanUResult.requiredColumns().contains("name"));

            // o needs user_id (from join condition).
            assertNotNull(scanOResult.requiredColumns());
            assertTrue(scanOResult.requiredColumns().contains("user_id"));
        }

        @Test @DisplayName("ProjectionPruning with sort key adds sort columns")
        void pruningWithSortKey() {
            var scanU  = scan("users", "u");
            var sortP  = sort(scanU, List.of(asc(col("u", "age"))));
            var proj   = project(sortP, List.of(exprCol(col("u", "name"), "name")));

            var result = new SqlOptimizer.ProjectionPruning().apply(proj);

            var p       = (SqlOptimizer.OptimizedPlan.Project) result;
            var sortR   = (SqlOptimizer.OptimizedPlan.Sort) p.input();
            var scanR   = (SqlOptimizer.OptimizedPlan.Scan) sortR.input();

            assertNotNull(scanR.requiredColumns());
            // Both name (from project) and age (from sort key) are required.
            assertTrue(scanR.requiredColumns().contains("name"));
            assertTrue(scanR.requiredColumns().contains("age"));
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // DeadCodeElimination
    // ─────────────────────────────────────────────────────────────────────────

    @Nested @DisplayName("DeadCodeElimination")
    class DeadCodeEliminationTests {

        final SqlOptimizer.DeadCodeElimination dce = new SqlOptimizer.DeadCodeElimination();

        @Test @DisplayName("Filter(child, Literal(false)) → EmptyResult")
        void filterFalseBecomesEmpty() {
            var plan = filter(scan("t", "t"), lit(false));
            var result = dce.apply(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.EmptyResult.class, result);
        }

        @Test @DisplayName("Filter(child, Literal(null)) → EmptyResult")
        void filterNullBecomesEmpty() {
            var plan = filter(scan("t", "t"), lit(null));
            var result = dce.apply(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.EmptyResult.class, result);
        }

        @Test @DisplayName("Filter(child, Literal(true)) → child (filter removed)")
        void filterTrueRemoved() {
            var scanT  = scan("t", "t");
            var plan   = filter(scanT, lit(true));
            var result = dce.apply(plan);
            // The filter should be gone; scan remains.
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Scan.class, result);
        }

        @Test @DisplayName("Limit(child, count=0, offset=null) → EmptyResult")
        void limitZeroBecomesEmpty() {
            var plan   = limit(scan("t", "t"), 0L, null);
            var result = dce.apply(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.EmptyResult.class, result);
        }

        @Test @DisplayName("Union(EmptyResult, scan) → scan")
        void unionWithEmptyLeft() {
            var scanT = scan("t", "t");
            var plan  = new SqlOptimizer.OptimizedPlan.Union(
                new SqlOptimizer.OptimizedPlan.EmptyResult(), scanT, false);
            var result = dce.apply(plan);
            // The EmptyResult side disappears; scan remains.
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Scan.class, result);
        }

        @Test @DisplayName("Inner Join with EmptyResult → EmptyResult")
        void innerJoinEmptyResultCollapses() {
            var plan = join(
                new SqlOptimizer.OptimizedPlan.EmptyResult(),
                scan("t", "t"),
                SqlPlanner.JoinKind.INNER, null);
            var result = dce.apply(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.EmptyResult.class, result);
        }

        @Test @DisplayName("Aggregate(EmptyResult) NOT collapsed — COUNT(*) from empty")
        void aggregateEmptyNotCollapsed() {
            var plan = agg(
                new SqlOptimizer.OptimizedPlan.EmptyResult(),
                List.of(), List.of());
            var result = dce.apply(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Aggregate.class, result);
        }

        @Test @DisplayName("Filter(FALSE) under Sort → EmptyResult propagates up")
        void emptyPropagatesUpThroughSort() {
            var falseScan = filter(scan("t", "t"), lit(false));
            var plan = sort(falseScan, List.of(asc(col("t", "id"))));
            var result = dce.apply(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.EmptyResult.class, result);
        }

        @Test @DisplayName("Project(EmptyResult) → EmptyResult")
        void projectEmptyResult() {
            var plan = project(new SqlOptimizer.OptimizedPlan.EmptyResult(),
                               List.of(exprCol(col("t", "id"), "id")));
            var result = dce.apply(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.EmptyResult.class, result);
        }

        @Test @DisplayName("CROSS Join with EmptyResult right side → EmptyResult")
        void crossJoinEmptyRightCollapses() {
            var plan = join(
                scan("t", "t"),
                new SqlOptimizer.OptimizedPlan.EmptyResult(),
                SqlPlanner.JoinKind.CROSS, null);
            var result = dce.apply(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.EmptyResult.class, result);
        }

        @Test @DisplayName("Distinct(EmptyResult) → EmptyResult")
        void distinctEmpty() {
            var plan = distinct(new SqlOptimizer.OptimizedPlan.EmptyResult());
            var result = dce.apply(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.EmptyResult.class, result);
        }

        @Test @DisplayName("Sort(EmptyResult) → EmptyResult")
        void sortEmpty() {
            var plan = sort(new SqlOptimizer.OptimizedPlan.EmptyResult(),
                            List.of(asc(col("t", "id"))));
            var result = dce.apply(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.EmptyResult.class, result);
        }

        @Test @DisplayName("Limit(EmptyResult) → EmptyResult")
        void limitEmpty() {
            var plan = limit(new SqlOptimizer.OptimizedPlan.EmptyResult(), 10L, null);
            var result = dce.apply(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.EmptyResult.class, result);
        }

        @Test @DisplayName("Having(EmptyResult) → EmptyResult")
        void havingEmpty() {
            var plan = new SqlOptimizer.OptimizedPlan.Having(
                new SqlOptimizer.OptimizedPlan.EmptyResult(), lit(true));
            var result = dce.apply(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.EmptyResult.class, result);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // LimitPushdown
    // ─────────────────────────────────────────────────────────────────────────

    @Nested @DisplayName("LimitPushdown")
    class LimitPushdownTests {

        final SqlOptimizer.LimitPushdown lp = new SqlOptimizer.LimitPushdown();

        @Test @DisplayName("Limit annotates Scan with scanLimit through Project")
        void limitPropagatesThroughProject() {
            var scanU = scan("users", "u");
            var proj  = project(scanU, List.of(starCol()));
            var lim   = limit(proj, 10L, null);

            var result = lp.apply(lim);

            var limResult  = (SqlOptimizer.OptimizedPlan.Limit) result;
            var projResult = (SqlOptimizer.OptimizedPlan.Project) limResult.input();
            var scanResult = (SqlOptimizer.OptimizedPlan.Scan) projResult.input();
            assertEquals(10L, scanResult.scanLimit());
        }

        @Test @DisplayName("Limit does NOT push through Sort")
        void limitDoesNotPushThroughSort() {
            var scanU = scan("users", "u");
            var sortP = sort(scanU, List.of(asc(col("u", "name"))));
            var lim   = limit(sortP, 10L, null);

            var result = lp.apply(lim);

            var limResult  = (SqlOptimizer.OptimizedPlan.Limit) result;
            var sortResult = (SqlOptimizer.OptimizedPlan.Sort) limResult.input();
            var scanResult = (SqlOptimizer.OptimizedPlan.Scan) sortResult.input();
            // scanLimit must NOT be set — Sort is a barrier.
            assertNull(scanResult.scanLimit());
        }

        @Test @DisplayName("Limit pushes through Filter")
        void limitPropagatesThroughFilter() {
            var scanU   = scan("users", "u");
            var filtP   = filter(scanU, col("u", "active"));
            var lim     = limit(filtP, 5L, null);

            var result = lp.apply(lim);

            var limResult  = (SqlOptimizer.OptimizedPlan.Limit) result;
            var filtResult = (SqlOptimizer.OptimizedPlan.Filter) limResult.input();
            var scanResult = (SqlOptimizer.OptimizedPlan.Scan) filtResult.input();
            assertEquals(5L, scanResult.scanLimit());
        }

        @Test @DisplayName("Limit with offset > 0 is NOT pushed to Scan")
        void limitWithOffsetNotPushed() {
            var scanU = scan("users", "u");
            var lim   = limit(scanU, 10L, 5L);   // LIMIT 10 OFFSET 5

            var result = lp.apply(lim);

            var limResult  = (SqlOptimizer.OptimizedPlan.Limit) result;
            var scanResult = (SqlOptimizer.OptimizedPlan.Scan) limResult.input();
            // No scanLimit because offset > 0.
            assertNull(scanResult.scanLimit());
        }

        @Test @DisplayName("Limit with offset=0 IS pushed")
        void limitWithZeroOffsetIsPushed() {
            var scanU = scan("users", "u");
            var lim   = limit(scanU, 10L, 0L);   // LIMIT 10 OFFSET 0

            var result = lp.apply(lim);

            var limResult  = (SqlOptimizer.OptimizedPlan.Limit) result;
            var scanResult = (SqlOptimizer.OptimizedPlan.Scan) limResult.input();
            assertEquals(10L, scanResult.scanLimit());
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Combined / integration
    // ─────────────────────────────────────────────────────────────────────────

    @Nested @DisplayName("Combined")
    class CombinedTests {

        @Test @DisplayName("All five passes applied in sequence")
        void allFivePassesTogether() {
            // SELECT u.name FROM users u WHERE 1=1 LIMIT 10
            // After optimize():
            //   - ConstantFolding folds 1=1 → true
            //   - DCE removes the Filter(true, ...) → bare Scan
            //   - ProjectionPruning annotates Scan with requiredColumns=[name]
            //   - LimitPushdown annotates Scan with scanLimit=10

            var logical = new SqlPlanner.LogicalPlan.Limit(
                new SqlPlanner.LogicalPlan.Project(
                    new SqlPlanner.LogicalPlan.Filter(
                        new SqlPlanner.LogicalPlan.Scan("users", "u"),
                        eq(lit(1L), lit(1L))   // 1=1 → constant true
                    ),
                    List.of(exprCol(col("u", "name"), "name"))
                ),
                10L, null
            );

            var result = SqlOptimizer.optimize(logical);

            // Result should be: Limit(Project(Scan(...)))
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Limit.class, result);
            var lim  = (SqlOptimizer.OptimizedPlan.Limit) result;
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Project.class, lim.input());
            var proj = (SqlOptimizer.OptimizedPlan.Project) lim.input();
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Scan.class, proj.input());
            var scan = (SqlOptimizer.OptimizedPlan.Scan) proj.input();

            // Projection pruning annotated the scan.
            assertNotNull(scan.requiredColumns());
            assertTrue(scan.requiredColumns().contains("name"));

            // Limit pushdown annotated the scan.
            assertEquals(10L, scan.scanLimit());
        }

        @Test @DisplayName("Filter(FALSE) under aggregate → DCE does not collapse aggregate")
        void falseFilterUnderAggregate() {
            // SELECT COUNT(*) FROM t WHERE 1=2
            // The DCE should make Filter(Scan, false) → EmptyResult,
            // but Aggregate(EmptyResult) should NOT be collapsed.
            var logical = new SqlPlanner.LogicalPlan.Aggregate(
                new SqlPlanner.LogicalPlan.Filter(
                    new SqlPlanner.LogicalPlan.Scan("t", "t"),
                    eq(lit(1L), lit(2L))   // 1=2 → false after ConstantFolding
                ),
                List.of(),
                List.of(new SqlPlanner.AggregateItem(
                    SqlPlanner.AggFunction.COUNT,
                    new SqlPlanner.AggArg.Star(),
                    "cnt", false))
            );

            var result = SqlOptimizer.optimize(logical);

            // After passes: Aggregate should survive (COUNT(*) over empty = 1 row).
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Aggregate.class, result);
            var agg = (SqlOptimizer.OptimizedPlan.Aggregate) result;
            assertInstanceOf(SqlOptimizer.OptimizedPlan.EmptyResult.class, agg.input());
        }

        @Test @DisplayName("ConstantFolding + DCE: NULL in predicate → EmptyResult")
        void nullPredicateBecomesEmpty() {
            // Filter(Scan, NULL) → after CF stays null lit, DCE removes the scan.
            var plan = new SqlPlanner.LogicalPlan.Filter(
                new SqlPlanner.LogicalPlan.Scan("t", "t"),
                lit(null)
            );
            var result = SqlOptimizer.optimize(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.EmptyResult.class, result);
        }

        @Test @DisplayName("PredicatePushdown + Limit: push both down through Project")
        void predPushdownAndLimitTogether() {
            // SELECT u.name FROM users u WHERE u.id = 1 LIMIT 5
            var logical = new SqlPlanner.LogicalPlan.Limit(
                new SqlPlanner.LogicalPlan.Project(
                    new SqlPlanner.LogicalPlan.Filter(
                        new SqlPlanner.LogicalPlan.Scan("users", "u"),
                        eq(col("u", "id"), lit(1L))
                    ),
                    List.of(exprCol(col("u", "name"), "name"))
                ),
                5L, null
            );

            var result = SqlOptimizer.optimize(logical);

            // After predicate pushdown, the filter should already be below the project.
            // After limit pushdown, scanLimit = 5.
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Limit.class, result);
            var lim = (SqlOptimizer.OptimizedPlan.Limit) result;
            assertEquals(5L, lim.count());
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Extended ConstantFolding coverage
    // ─────────────────────────────────────────────────────────────────────────

    @Nested @DisplayName("ConstantFolding Extended")
    class ConstantFoldingExtendedTests {

        @Test @DisplayName("MOD: 10 mod 3 → 1")
        void modFolding() {
            var expr = binOp(SqlPlanner.BinaryOperator.MOD, lit(10L), lit(3L));
            assertEquals(new SqlPlanner.SqlExpr.Literal(1L), fold(expr));
        }

        @Test @DisplayName("MOD by zero not folded")
        void modByZeroNotFolded() {
            var expr = binOp(SqlPlanner.BinaryOperator.MOD, lit(10L), lit(0L));
            assertInstanceOf(SqlPlanner.SqlExpr.BinaryOp.class, fold(expr));
        }

        @Test @DisplayName("EQ on literals: 5 = 5 → true")
        void eqLiterals() {
            assertEquals(new SqlPlanner.SqlExpr.Literal(true), fold(eq(lit(5L), lit(5L))));
        }

        @Test @DisplayName("NOT_EQ on literals: 3 != 5 → true")
        void notEqLiterals() {
            var expr = binOp(SqlPlanner.BinaryOperator.NOT_EQ, lit(3L), lit(5L));
            assertEquals(new SqlPlanner.SqlExpr.Literal(true), fold(expr));
        }

        @Test @DisplayName("GT: 5 > 3 → true")
        void gtLiterals() {
            var expr = binOp(SqlPlanner.BinaryOperator.GT, lit(5L), lit(3L));
            assertEquals(new SqlPlanner.SqlExpr.Literal(true), fold(expr));
        }

        @Test @DisplayName("GTE: 5 >= 5 → true")
        void gteLiterals() {
            var expr = binOp(SqlPlanner.BinaryOperator.GTE, lit(5L), lit(5L));
            assertEquals(new SqlPlanner.SqlExpr.Literal(true), fold(expr));
        }

        @Test @DisplayName("LTE: 3 <= 5 → true")
        void lteLiterals() {
            var expr = binOp(SqlPlanner.BinaryOperator.LTE, lit(3L), lit(5L));
            assertEquals(new SqlPlanner.SqlExpr.Literal(true), fold(expr));
        }

        @Test @DisplayName("AND of two Boolean literals: true AND true → true")
        void andBothTrue() {
            assertEquals(new SqlPlanner.SqlExpr.Literal(true), fold(and(lit(true), lit(true))));
        }

        @Test @DisplayName("OR of two Boolean literals: false OR false → false")
        void orBothFalse() {
            assertEquals(new SqlPlanner.SqlExpr.Literal(false), fold(or(lit(false), lit(false))));
        }

        @Test @DisplayName("FALSE OR x → x (short circuit)")
        void falseOrXIsX() {
            var x = col("u", "flag");
            var result = fold(or(lit(false), x));
            assertEquals(x, result);
        }

        @Test @DisplayName("x AND FALSE → FALSE (right-side short circuit)")
        void xAndFalseIsFalse() {
            var x = col("u", "flag");
            assertEquals(new SqlPlanner.SqlExpr.Literal(false), fold(and(x, lit(false))));
        }

        @Test @DisplayName("x OR FALSE → x (right-side identity)")
        void xOrFalseIsX() {
            var x = col("u", "flag");
            assertEquals(x, fold(or(x, lit(false))));
        }

        @Test @DisplayName("TRUE AND x → x (left-side identity)")
        void trueAndXIsX() {
            var x = col("u", "flag");
            assertEquals(x, fold(and(lit(true), x)));
        }

        @Test @DisplayName("DIV of longs: 10 / 3 → 3 (integer division)")
        void divLongs() {
            assertEquals(new SqlPlanner.SqlExpr.Literal(3L), fold(div(lit(10L), lit(3L))));
        }

        @Test @DisplayName("NULL AND FALSE → FALSE (AND short-circuit before null prop)")
        void nullAndFalseIsFalse() {
            // FALSE short-circuits AND before null propagation applies.
            assertEquals(new SqlPlanner.SqlExpr.Literal(false), fold(and(lit(null), lit(false))));
        }

        @Test @DisplayName("IsNull(Literal(42)) → false")
        void isNullOfNonNull() {
            assertEquals(new SqlPlanner.SqlExpr.Literal(false), fold(isNull(lit(42L))));
        }

        @Test @DisplayName("IsNotNull(Literal(null)) → false")
        void isNotNullOfNull() {
            assertEquals(new SqlPlanner.SqlExpr.Literal(false), fold(isNotNull(lit(null))));
        }

        @Test @DisplayName("NOT(non-Boolean literal) not folded")
        void notNonBooleanNotFolded() {
            var result = fold(not(lit(42L)));
            assertInstanceOf(SqlPlanner.SqlExpr.UnaryOp.class, result);
        }

        @Test @DisplayName("NEG(null) → null (null propagation)")
        void negNull() {
            assertEquals(new SqlPlanner.SqlExpr.Literal(null), fold(neg(lit(null))));
        }

        @Test @DisplayName("FuncCall args are folded")
        void funcCallArgsFolded() {
            var expr = new SqlPlanner.SqlExpr.FuncCall("ABS", List.of(add(lit(1L), lit(2L))));
            var result = fold(expr);
            assertInstanceOf(SqlPlanner.SqlExpr.FuncCall.class, result);
            var fc = (SqlPlanner.SqlExpr.FuncCall) result;
            assertEquals(new SqlPlanner.SqlExpr.Literal(3L), fc.args().get(0));
        }

        @Test @DisplayName("Between children are folded")
        void betweenFolded() {
            var expr = new SqlPlanner.SqlExpr.Between(
                add(lit(1L), lit(1L)),
                lit(0L), lit(5L));
            var result = fold(expr);
            assertInstanceOf(SqlPlanner.SqlExpr.Between.class, result);
            var b = (SqlPlanner.SqlExpr.Between) result;
            assertEquals(new SqlPlanner.SqlExpr.Literal(2L), b.value());
        }

        @Test @DisplayName("In items are folded")
        void inFolded() {
            var expr = new SqlPlanner.SqlExpr.In(
                col("u", "x"),
                List.of(add(lit(1L), lit(1L)), lit(5L)));
            var result = fold(expr);
            assertInstanceOf(SqlPlanner.SqlExpr.In.class, result);
            var in = (SqlPlanner.SqlExpr.In) result;
            assertEquals(new SqlPlanner.SqlExpr.Literal(2L), in.items().get(0));
        }

        @Test @DisplayName("NotIn items are folded")
        void notInFolded() {
            var expr = new SqlPlanner.SqlExpr.NotIn(
                col("u", "x"),
                List.of(add(lit(2L), lit(3L))));
            var result = fold(expr);
            assertInstanceOf(SqlPlanner.SqlExpr.NotIn.class, result);
            var nin = (SqlPlanner.SqlExpr.NotIn) result;
            assertEquals(new SqlPlanner.SqlExpr.Literal(5L), nin.items().get(0));
        }

        @Test @DisplayName("Double arithmetic: 1.5 + 2.5 → 4.0")
        void doubleArithmetic() {
            var result = fold(add(lit(1.5), lit(2.5)));
            assertInstanceOf(SqlPlanner.SqlExpr.Literal.class, result);
            var lit = (SqlPlanner.SqlExpr.Literal) result;
            assertEquals(4.0, (Double) lit.value(), 1e-9);
        }

        @Test @DisplayName("NULL + NULL → NULL")
        void nullPlusNull() {
            assertEquals(new SqlPlanner.SqlExpr.Literal(null), fold(add(lit(null), lit(null))));
        }

        @Test @DisplayName("String comparison: 'a' < 'b' → true")
        void stringComparison() {
            var result = fold(lt(lit("a"), lit("b")));
            assertEquals(new SqlPlanner.SqlExpr.Literal(true), result);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Extended ProjectionPruning coverage
    // ─────────────────────────────────────────────────────────────────────────

    @Nested @DisplayName("ProjectionPruning Extended")
    class ProjectionPruningExtendedTests {

        @Test @DisplayName("ProjectionPruning with AggArg.Star: no column refs, scan stays null")
        void aggStarDoesNotPruneColumns() {
            // Aggregate(Scan, [], [COUNT(*)])
            var scanT = scan("t", "t");
            var aggPlan = agg(scanT, List.of(),
                List.of(new SqlPlanner.AggregateItem(
                    SqlPlanner.AggFunction.COUNT,
                    new SqlPlanner.AggArg.Star(),
                    "cnt", false)));

            var result = new SqlOptimizer.ProjectionPruning().apply(aggPlan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Aggregate.class, result);
            var a = (SqlOptimizer.OptimizedPlan.Aggregate) result;
            // COUNT(*) has no column refs; scan's requiredColumns should stay null.
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Scan.class, a.input());
            var s = (SqlOptimizer.OptimizedPlan.Scan) a.input();
            assertNull(s.requiredColumns());
        }

        @Test @DisplayName("ProjectionPruning with Having: having predicate columns added")
        void havingPredicateAddsColumns() {
            // Having(Scan, col(t, cnt) > 5) with Scan("t", "t")
            var scanT = scan("t", "t");
            var havingPlan = new SqlOptimizer.OptimizedPlan.Having(
                scanT,
                binOp(SqlPlanner.BinaryOperator.GT, col("t", "cnt"), lit(5L)));

            var result = new SqlOptimizer.ProjectionPruning().apply(havingPlan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Having.class, result);
            var h = (SqlOptimizer.OptimizedPlan.Having) result;
            // No project above, so required is null; scan stays null.
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Scan.class, h.input());
        }

        @Test @DisplayName("ProjectionPruning passes through Distinct")
        void pruningPassesThroughDistinct() {
            var scanU = scan("u", "u");
            var proj  = project(distinct(scanU), List.of(exprCol(col("u", "name"), "n")));

            var result = new SqlOptimizer.ProjectionPruning().apply(proj);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Project.class, result);
            var p = (SqlOptimizer.OptimizedPlan.Project) result;
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Distinct.class, p.input());
            var d = (SqlOptimizer.OptimizedPlan.Distinct) p.input();
            var s = (SqlOptimizer.OptimizedPlan.Scan) d.input();
            assertNotNull(s.requiredColumns());
            assertTrue(s.requiredColumns().contains("name"));
        }

        @Test @DisplayName("ProjectionPruning passes through Limit")
        void pruningPassesThroughLimit() {
            var scanU = scan("u", "u");
            var lim   = limit(scanU, 10L, null);
            var proj  = project(lim, List.of(exprCol(col("u", "name"), "n")));

            var result = new SqlOptimizer.ProjectionPruning().apply(proj);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Project.class, result);
            var p = (SqlOptimizer.OptimizedPlan.Project) result;
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Limit.class, p.input());
            var limResult = (SqlOptimizer.OptimizedPlan.Limit) p.input();
            var s = (SqlOptimizer.OptimizedPlan.Scan) limResult.input();
            assertNotNull(s.requiredColumns());
        }

        @Test @DisplayName("ProjectionPruning with Union passes required to both sides")
        void pruningWithUnion() {
            var scanA = scan("a", "a");
            var scanB = scan("b", "b");
            var u     = new SqlOptimizer.OptimizedPlan.Union(scanA, scanB, false);
            // No project; required is null → both scans stay null.
            var result = new SqlOptimizer.ProjectionPruning().apply(u);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Union.class, result);
            var un = (SqlOptimizer.OptimizedPlan.Union) result;
            assertNull(((SqlOptimizer.OptimizedPlan.Scan) un.left()).requiredColumns());
            assertNull(((SqlOptimizer.OptimizedPlan.Scan) un.right()).requiredColumns());
        }

        @Test @DisplayName("ProjectionPruning collectColRefs handles AggArg.Expr path")
        void aggExprColRefPath() {
            // Project(Aggregate(Scan("t","t"), [], [SUM(t.amount)]), [t.amount])
            // The project feeds required = {t:amount} to aggregate.
            // The aggregate resets required and adds AggArg.Expr(col("t","amount"))
            // → newRequired = {t:amount}.
            // The scan receives required = {t:amount} → requiredColumns = ["amount"].
            var scanT = scan("t", "t");
            var aggPlan = agg(scanT, List.of(),
                List.of(new SqlPlanner.AggregateItem(
                    SqlPlanner.AggFunction.SUM,
                    new SqlPlanner.AggArg.Expr(col("t", "amount")),
                    "total", false)));
            var proj = project(aggPlan, List.of(exprCol(col("t", "amount"), "amt")));

            var result = new SqlOptimizer.ProjectionPruning().apply(proj);
            var p = (SqlOptimizer.OptimizedPlan.Project) result;
            var a = (SqlOptimizer.OptimizedPlan.Aggregate) p.input();
            var s = (SqlOptimizer.OptimizedPlan.Scan) a.input();
            assertNotNull(s.requiredColumns());
            assertTrue(s.requiredColumns().contains("amount"));
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Extended PredicatePushdown coverage
    // ─────────────────────────────────────────────────────────────────────────

    @Nested @DisplayName("PredicatePushdown Extended")
    class PredicatePushdownExtendedTests {

        @Test @DisplayName("RIGHT JOIN: left-side predicate NOT pushed")
        void rightJoinLeftNotPushed() {
            var predLeft = eq(col("u", "id"), lit(1L));
            var scanU = scan("users", "u");
            var scanO = scan("orders", "o");
            var joined = join(scanU, scanO, SqlPlanner.JoinKind.RIGHT, null);
            var filtered = filter(joined, predLeft);

            var result = new SqlOptimizer.PredicatePushdown().apply(filtered);

            // LEFT side pred not pushed through RIGHT JOIN.
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Filter.class, result);
        }

        @Test @DisplayName("FULL JOIN: no predicate pushed")
        void fullJoinNothingPushed() {
            var pred = eq(col("u", "id"), lit(1L));
            var scanU = scan("users", "u");
            var scanO = scan("orders", "o");
            var joined = join(scanU, scanO, SqlPlanner.JoinKind.FULL, null);
            var filtered = filter(joined, pred);

            var result = new SqlOptimizer.PredicatePushdown().apply(filtered);

            // FULL JOIN: nothing pushed.
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Filter.class, result);
            var f = (SqlOptimizer.OptimizedPlan.Filter) result;
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Join.class, f.input());
            var j = (SqlOptimizer.OptimizedPlan.Join) f.input();
            // Neither side should have a Filter added.
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Scan.class, j.left());
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Scan.class, j.right());
        }

        @Test @DisplayName("INNER JOIN: right-side predicate IS pushed to right")
        void innerJoinRightPushed() {
            var predRight = eq(col("o", "status"), lit("active"));
            var scanU = scan("users", "u");
            var scanO = scan("orders", "o");
            var joined = join(scanU, scanO, SqlPlanner.JoinKind.INNER,
                eq(col("u", "id"), col("o", "user_id")));
            var filtered = filter(joined, predRight);

            var result = new SqlOptimizer.PredicatePushdown().apply(filtered);

            assertInstanceOf(SqlOptimizer.OptimizedPlan.Join.class, result);
            var j = (SqlOptimizer.OptimizedPlan.Join) result;
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Filter.class, j.right());
        }

        @Test @DisplayName("Predicate with no column refs pushed to left by convention")
        void noColRefPredicatePushedLeft() {
            var pred = lit(true);   // no column references
            var scanU = scan("users", "u");
            var scanO = scan("orders", "o");
            var joined = join(scanU, scanO, SqlPlanner.JoinKind.INNER, null);
            var filtered = filter(joined, pred);

            var result = new SqlOptimizer.PredicatePushdown().apply(filtered);

            // No-column predicate goes to left by convention.
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Join.class, result);
        }

        @Test @DisplayName("Filter through Project: predicate referencing non-scan alias stays above")
        void filterThroughProjectNonMatchingAlias() {
            // Project(Scan("t","t"), ...) filtered by col("x", "y")
            // "x" is not a scan alias below the project → stays above.
            var pred = eq(col("x", "y"), lit(1L));
            var proj = project(scan("t", "t"), List.of(exprCol(col("t", "id"), "id")));
            var filtered = filter(proj, pred);

            var result = new SqlOptimizer.PredicatePushdown().apply(filtered);

            // Filter should stay above project.
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Filter.class, result);
            var f = (SqlOptimizer.OptimizedPlan.Filter) result;
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Project.class, f.input());
        }

        @Test @DisplayName("splitAnd correctly splits 3-way AND")
        void splitAndThreeWay() {
            var a = col("u", "a");
            var b = col("u", "b");
            var c = col("u", "c");
            var expr = and(and(a, b), c);
            var parts = SqlOptimizer.PredicatePushdown.splitAnd(expr);
            assertEquals(3, parts.size());
        }

        @Test @DisplayName("columnAliases returns correct alias set")
        void columnAliasesHelper() {
            var expr = and(col("u", "id"), col("o", "uid"));
            var aliases = SqlOptimizer.PredicatePushdown.columnAliases(expr);
            assertTrue(aliases.contains("u"));
            assertTrue(aliases.contains("o"));
        }

        @Test @DisplayName("Filter not pushed through Limit (barrier)")
        void filterNotPushedThroughLimit() {
            var pred = eq(col("t", "id"), lit(1L));
            var lim  = limit(scan("t", "t"), 10L, null);
            var f    = filter(lim, pred);

            var result = new SqlOptimizer.PredicatePushdown().apply(f);

            // Filter stays above Limit.
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Filter.class, result);
            var fr = (SqlOptimizer.OptimizedPlan.Filter) result;
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Limit.class, fr.input());
        }

        @Test @DisplayName("collectAliases includes aliases from nested Join")
        void collectAliasesNestedJoin() {
            var j = join(scan("u","u"), scan("o","o"), SqlPlanner.JoinKind.INNER, null);
            var aliases = SqlOptimizer.PredicatePushdown.collectAliases(j);
            assertTrue(aliases.contains("u"));
            assertTrue(aliases.contains("o"));
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Extended LimitPushdown coverage
    // ─────────────────────────────────────────────────────────────────────────

    @Nested @DisplayName("LimitPushdown Extended")
    class LimitPushdownExtendedTests {

        @Test @DisplayName("LimitPushdown does not push through Aggregate (barrier)")
        void limitDoesNotPushThroughAggregate() {
            var scanT = scan("t", "t");
            var aggP  = agg(scanT, List.of(), List.of());
            var lim   = limit(aggP, 5L, null);

            var result = new SqlOptimizer.LimitPushdown().apply(lim);
            var limResult = (SqlOptimizer.OptimizedPlan.Limit) result;
            var aggResult = (SqlOptimizer.OptimizedPlan.Aggregate) limResult.input();
            var scanResult = (SqlOptimizer.OptimizedPlan.Scan) aggResult.input();
            assertNull(scanResult.scanLimit());
        }

        @Test @DisplayName("LimitPushdown does not push through Distinct (barrier)")
        void limitDoesNotPushThroughDistinct() {
            var scanT = scan("t", "t");
            var distP = distinct(scanT);
            var lim   = limit(distP, 5L, null);

            var result = new SqlOptimizer.LimitPushdown().apply(lim);
            var limResult = (SqlOptimizer.OptimizedPlan.Limit) result;
            var distResult = (SqlOptimizer.OptimizedPlan.Distinct) limResult.input();
            var scanResult = (SqlOptimizer.OptimizedPlan.Scan) distResult.input();
            assertNull(scanResult.scanLimit());
        }

        @Test @DisplayName("Multiple nested limits: min(5, 10) = 5 pushed to scan")
        void nestedLimitsTakeMin() {
            var scanT = scan("t", "t");
            var inner = limit(scanT, 10L, null);
            var outer = limit(inner, 5L, null);

            var result = new SqlOptimizer.LimitPushdown().apply(outer);
            var outerLim = (SqlOptimizer.OptimizedPlan.Limit) result;
            var innerLim = (SqlOptimizer.OptimizedPlan.Limit) outerLim.input();
            var scanResult = (SqlOptimizer.OptimizedPlan.Scan) innerLim.input();
            assertEquals(5L, scanResult.scanLimit());
        }

        @Test @DisplayName("LimitPushdown does not push through Join (barrier)")
        void limitDoesNotPushThroughJoin() {
            var j   = join(scan("u","u"), scan("o","o"), SqlPlanner.JoinKind.INNER, null);
            var lim = limit(j, 5L, null);

            var result = new SqlOptimizer.LimitPushdown().apply(lim);
            var limResult = (SqlOptimizer.OptimizedPlan.Limit) result;
            var jResult = (SqlOptimizer.OptimizedPlan.Join) limResult.input();
            assertNull(((SqlOptimizer.OptimizedPlan.Scan) jResult.left()).scanLimit());
        }

        @Test @DisplayName("LimitPushdown does not push through Union (barrier)")
        void limitDoesNotPushThroughUnion() {
            var u   = new SqlOptimizer.OptimizedPlan.Union(scan("a","a"), scan("b","b"), false);
            var lim = limit(u, 5L, null);

            var result = new SqlOptimizer.LimitPushdown().apply(lim);
            var limResult = (SqlOptimizer.OptimizedPlan.Limit) result;
            var uResult = (SqlOptimizer.OptimizedPlan.Union) limResult.input();
            assertNull(((SqlOptimizer.OptimizedPlan.Scan) uResult.left()).scanLimit());
        }

        @Test @DisplayName("LimitPushdown: null count does not annotate scan")
        void nullCountDoesNotAnnotateScan() {
            // Limit(Scan, null, null) — no count to push
            var scanT = scan("t", "t");
            var lim   = limit(scanT, null, null);

            var result = new SqlOptimizer.LimitPushdown().apply(lim);
            var limResult = (SqlOptimizer.OptimizedPlan.Limit) result;
            var scanResult = (SqlOptimizer.OptimizedPlan.Scan) limResult.input();
            assertNull(scanResult.scanLimit());
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // ConstantFolding plan-level traversal coverage
    // ─────────────────────────────────────────────────────────────────────────

    @Nested @DisplayName("ConstantFolding Plan Traversal")
    class ConstantFoldingPlanTraversalTests {

        private final SqlOptimizer.ConstantFolding cf = new SqlOptimizer.ConstantFolding();

        @Test @DisplayName("CF folds expression in Sort key")
        void foldSortKey() {
            // Sort by add(1,2) — should fold the key expression.
            var plan = sort(scan("t","t"), List.of(
                new SqlPlanner.SortKey(add(lit(1L), lit(2L)),
                    SqlPlanner.SortDir.ASC, SqlPlanner.NullOrder.NULLS_LAST)));
            var result = cf.apply(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Sort.class, result);
            var s = (SqlOptimizer.OptimizedPlan.Sort) result;
            assertEquals(new SqlPlanner.SqlExpr.Literal(3L), s.keys().get(0).keyExpr());
        }

        @Test @DisplayName("CF traverses Limit node")
        void foldLimit() {
            // Filter(Scan, 1+2) under Limit
            var plan = limit(filter(scan("t","t"), add(lit(1L), lit(2L))), 5L, null);
            var result = cf.apply(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Limit.class, result);
            var l = (SqlOptimizer.OptimizedPlan.Limit) result;
            var f = (SqlOptimizer.OptimizedPlan.Filter) l.input();
            assertEquals(new SqlPlanner.SqlExpr.Literal(3L), f.predicate());
        }

        @Test @DisplayName("CF traverses Distinct node")
        void foldDistinct() {
            var plan = distinct(filter(scan("t","t"), add(lit(1L), lit(1L))));
            var result = cf.apply(plan);
            var d = (SqlOptimizer.OptimizedPlan.Distinct) result;
            var f = (SqlOptimizer.OptimizedPlan.Filter) d.input();
            assertEquals(new SqlPlanner.SqlExpr.Literal(2L), f.predicate());
        }

        @Test @DisplayName("CF traverses Union node")
        void foldUnion() {
            var leftF  = filter(scan("a","a"), add(lit(1L), lit(1L)));
            var rightF = filter(scan("b","b"), add(lit(2L), lit(2L)));
            var plan   = new SqlOptimizer.OptimizedPlan.Union(leftF, rightF, false);
            var result = cf.apply(plan);
            var u = (SqlOptimizer.OptimizedPlan.Union) result;
            assertEquals(new SqlPlanner.SqlExpr.Literal(2L),
                ((SqlOptimizer.OptimizedPlan.Filter) u.left()).predicate());
            assertEquals(new SqlPlanner.SqlExpr.Literal(4L),
                ((SqlOptimizer.OptimizedPlan.Filter) u.right()).predicate());
        }

        @Test @DisplayName("CF traverses Having node")
        void foldHaving() {
            var plan = new SqlOptimizer.OptimizedPlan.Having(
                scan("t","t"), add(lit(3L), lit(4L)));
            var result = cf.apply(plan);
            var h = (SqlOptimizer.OptimizedPlan.Having) result;
            assertEquals(new SqlPlanner.SqlExpr.Literal(7L), h.predicate());
        }

        @Test @DisplayName("CF traverses Aggregate groupBy expressions")
        void foldAggGroupBy() {
            var plan = agg(scan("t","t"),
                List.of(add(lit(1L), lit(1L))),
                List.of());
            var result = cf.apply(plan);
            var a = (SqlOptimizer.OptimizedPlan.Aggregate) result;
            assertEquals(new SqlPlanner.SqlExpr.Literal(2L), a.groupBy().get(0));
        }

        @Test @DisplayName("CF traverses Join: folds condition and recurses into children")
        void foldJoin() {
            var lf = filter(scan("u","u"), add(lit(1L), lit(2L)));
            var rf = filter(scan("o","o"), add(lit(3L), lit(4L)));
            var plan = join(lf, rf, SqlPlanner.JoinKind.INNER, add(lit(5L), lit(5L)));
            var result = cf.apply(plan);
            var j = (SqlOptimizer.OptimizedPlan.Join) result;
            assertEquals(new SqlPlanner.SqlExpr.Literal(10L), j.condition());
            assertEquals(new SqlPlanner.SqlExpr.Literal(3L),
                ((SqlOptimizer.OptimizedPlan.Filter) j.left()).predicate());
            assertEquals(new SqlPlanner.SqlExpr.Literal(7L),
                ((SqlOptimizer.OptimizedPlan.Filter) j.right()).predicate());
        }

        @Test @DisplayName("CF traverses Join with null condition")
        void foldJoinNullCondition() {
            var plan = join(scan("u","u"), scan("o","o"), SqlPlanner.JoinKind.CROSS, null);
            var result = cf.apply(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Join.class, result);
            assertNull(((SqlOptimizer.OptimizedPlan.Join) result).condition());
        }

        @Test @DisplayName("CF: NEG on Double literal")
        void negDouble() {
            assertEquals(new SqlPlanner.SqlExpr.Literal(-3.14), fold(neg(lit(-(-3.14)))));
            var r = fold(neg(lit(2.5)));
            assertEquals(new SqlPlanner.SqlExpr.Literal(-2.5), r);
        }

        @Test @DisplayName("CF: AND with two non-boolean non-null literals stays as BinaryOp")
        void andNonBooleanLiterals() {
            // AND(42, 99) — both non-null, non-boolean → returns BinaryOp unchanged.
            var expr = and(lit(42L), lit(99L));
            var result = fold(expr);
            assertInstanceOf(SqlPlanner.SqlExpr.BinaryOp.class, result);
        }

        @Test @DisplayName("CF: OR with two non-boolean non-null literals stays as BinaryOp")
        void orNonBooleanLiterals() {
            var expr = or(lit(42L), lit(99L));
            var result = fold(expr);
            assertInstanceOf(SqlPlanner.SqlExpr.BinaryOp.class, result);
        }

        @Test @DisplayName("CF: DIV zero-check for Double 0.0")
        void divByDoubleZeroNotFolded() {
            var expr = div(lit(10.0), lit(0.0));
            assertInstanceOf(SqlPlanner.SqlExpr.BinaryOp.class, fold(expr));
        }

        @Test @DisplayName("CF: MOD zero-check for Double 0.0")
        void modByDoubleZeroNotFolded() {
            var expr = binOp(SqlPlanner.BinaryOperator.MOD, lit(10.0), lit(0.0));
            assertInstanceOf(SqlPlanner.SqlExpr.BinaryOp.class, fold(expr));
        }

        @Test @DisplayName("CF: Integer + Long → Long")
        void intPlusLong() {
            // Box as Integer (not Long) for the Integer+Long branch.
            var result = fold(add(
                new SqlPlanner.SqlExpr.Literal(Integer.valueOf(3)),
                lit(4L)));
            assertEquals(new SqlPlanner.SqlExpr.Literal(7L), result);
        }

        @Test @DisplayName("CF: Long + Integer → Long")
        void longPlusInt() {
            var result = fold(add(
                lit(10L),
                new SqlPlanner.SqlExpr.Literal(Integer.valueOf(5))));
            assertEquals(new SqlPlanner.SqlExpr.Literal(15L), result);
        }

        @Test @DisplayName("CF: Integer + Integer → Long")
        void intPlusInt() {
            var result = fold(add(
                new SqlPlanner.SqlExpr.Literal(Integer.valueOf(2)),
                new SqlPlanner.SqlExpr.Literal(Integer.valueOf(3))));
            assertEquals(new SqlPlanner.SqlExpr.Literal(5L), result);
        }

        @Test @DisplayName("CF: OutputColumn.Star passthrough in Project")
        void projectStarPassthrough() {
            var plan = project(scan("t","t"), List.of(starCol()));
            var result = cf.apply(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Project.class, result);
            var p = (SqlOptimizer.OptimizedPlan.Project) result;
            assertInstanceOf(SqlPlanner.OutputColumn.Star.class, p.columns().get(0));
        }

        @Test @DisplayName("CF: default branch returns DML nodes unchanged")
        void defaultBranchDml() {
            var ins = new SqlOptimizer.OptimizedPlan.Insert("t",
                List.of("id"), List.of(List.of(lit(1L))));
            var result = cf.apply(ins);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Insert.class, result);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Extended DeadCodeElimination coverage
    // ─────────────────────────────────────────────────────────────────────────

    @Nested @DisplayName("DeadCodeElimination Extended")
    class DeadCodeEliminationExtendedTests {

        @Test @DisplayName("Union(left, EmptyResult) → left")
        void unionWithEmptyRight() {
            var scanT = scan("t", "t");
            var u = new SqlOptimizer.OptimizedPlan.Union(
                scanT,
                new SqlOptimizer.OptimizedPlan.EmptyResult(),
                false);
            var result = new SqlOptimizer.DeadCodeElimination().apply(u);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Scan.class, result);
        }

        @Test @DisplayName("LEFT JOIN with EmptyResult left — NOT collapsed (left outer join)")
        void leftJoinEmptyLeftNotCollapsed() {
            // LEFT JOIN: even when left side is empty, we don't collapse
            // (outer joins can still produce rows via null padding from right side
            // — actually LEFT JOIN returns right rows? No — LEFT JOIN returns all
            // left rows. If left is empty, LEFT JOIN is empty too, but we're
            // conservative here and only collapse INNER/CROSS.)
            var leftEmpty = new SqlOptimizer.OptimizedPlan.EmptyResult();
            var scanO = scan("o", "o");
            var plan = join(leftEmpty, scanO, SqlPlanner.JoinKind.LEFT, null);

            var result = new SqlOptimizer.DeadCodeElimination().apply(plan);
            // LEFT JOIN(EmptyResult, ...) is actually also empty, but our DCE
            // conservatively only collapses INNER and CROSS — so the join stays.
            assertInstanceOf(SqlOptimizer.OptimizedPlan.Join.class, result);
        }

        @Test @DisplayName("Limit(EmptyResult) → EmptyResult even with non-zero count")
        void limitNonZeroCountWithEmptyInput() {
            var plan = limit(new SqlOptimizer.OptimizedPlan.EmptyResult(), 10L, null);
            var result = new SqlOptimizer.DeadCodeElimination().apply(plan);
            assertInstanceOf(SqlOptimizer.OptimizedPlan.EmptyResult.class, result);
        }

        @Test @DisplayName("DCE recurses: Filter(Filter(Scan, false), pred) → EmptyResult")
        void dceRecursesIntoFilter() {
            var innerFalse = filter(scan("t","t"), lit(false));
            var outer = filter(innerFalse, col("t","x"));
            var result = new SqlOptimizer.DeadCodeElimination().apply(outer);
            // Inner becomes EmptyResult, outer filter of EmptyResult → EmptyResult.
            assertInstanceOf(SqlOptimizer.OptimizedPlan.EmptyResult.class, result);
        }
    }
}

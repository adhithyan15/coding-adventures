package com.codingadventures.sqlplanner;

// SqlPlannerTest.java — conformance and unit tests for the Java sql-planner.
//
// Test organisation mirrors the F# and C# test suites:
//   C1–C13  Conformance tests (the canonical 13-statement battery)
//   Struct   Structural tests (JOIN, aliases, DISTINCT/SORT/LIMIT stacking)
//   Error    Error-path tests (unknown table, unknown column, ambiguous column)
//   Expr     Expression-type tests (IS NULL, BETWEEN, IN, LIKE, …)

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;

import java.util.List;
import java.util.Map;
import java.util.Optional;

import static org.junit.jupiter.api.Assertions.*;

import com.codingadventures.sqlplanner.SqlPlanner.*;

class SqlPlannerTest {

    // ── Schema fixture ────────────────────────────────────────────────────────

    private static final InMemorySchemaProvider SCHEMA = new InMemorySchemaProvider(Map.of(
        "users",    List.of("id", "name", "age", "email"),
        "orders",   List.of("id", "user_id", "amount", "status"),
        "products", List.of("id", "name", "price", "category")
    ));

    private static SqlPlanner planner() { return new SqlPlanner(SCHEMA); }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /** Build a bare SELECT * FROM <table>. */
    private static Statement.Select selectStar(String table) {
        return new Statement.Select(
            false,
            List.of(new OutputColumn.Star()),
            List.of(new TableRef(table, null)),
            List.of(), null, List.of(), null, List.of(), null);
    }

    private static Statement.Select selectStarWhere(String table, SqlExpr where) {
        return new Statement.Select(
            false,
            List.of(new OutputColumn.Star()),
            List.of(new TableRef(table, null)),
            List.of(), where, List.of(), null, List.of(), null);
    }

    private static OutputColumn col(String column) {
        return new OutputColumn.Expr(new SqlExpr.Column(null, column), null);
    }

    private static OutputColumn colAs(String column, String alias) {
        return new OutputColumn.Expr(new SqlExpr.Column(null, column), alias);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C1 — SELECT * FROM users
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C1: SELECT * FROM users")
    void c1_selectStarFromUsers() {
        var plan = planner().plan(selectStar("users"));
        var proj = assertInstanceOf(LogicalPlan.Project.class, plan);
        assertEquals(1, proj.columns().size());
        assertInstanceOf(OutputColumn.Star.class, proj.columns().get(0));
        assertInstanceOf(LogicalPlan.Scan.class, proj.input());
        assertEquals("users", ((LogicalPlan.Scan) proj.input()).table());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C2 — SELECT * FROM users WHERE age > 18
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C2: SELECT * FROM users WHERE age > 18")
    void c2_selectStarWhereAge() {
        var where = new SqlExpr.BinaryOp(
            BinaryOperator.GT,
            new SqlExpr.Column(null, "age"),
            new SqlExpr.Literal(18L));
        var plan  = planner().plan(selectStarWhere("users", where));
        var proj  = assertInstanceOf(LogicalPlan.Project.class, plan);
        var filt  = assertInstanceOf(LogicalPlan.Filter.class, proj.input());
        var pred  = assertInstanceOf(SqlExpr.BinaryOp.class, filt.predicate());
        assertEquals(BinaryOperator.GT, pred.op());
        var col   = assertInstanceOf(SqlExpr.Column.class, pred.left());
        assertEquals("age", col.column());
        assertInstanceOf(LogicalPlan.Scan.class, filt.input());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C3 — SELECT id, name FROM users
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C3: SELECT id, name FROM users")
    void c3_selectColumns() {
        var stmt = new Statement.Select(
            false,
            List.of(col("id"), col("name")),
            List.of(new TableRef("users", null)),
            List.of(), null, List.of(), null, List.of(), null);
        var plan = planner().plan(stmt);
        var proj = assertInstanceOf(LogicalPlan.Project.class, plan);
        assertEquals(2, proj.columns().size());
        var c0   = assertInstanceOf(OutputColumn.Expr.class, proj.columns().get(0));
        var c1   = assertInstanceOf(OutputColumn.Expr.class, proj.columns().get(1));
        assertEquals("id",   ((SqlExpr.Column) c0.expression()).column());
        assertEquals("name", ((SqlExpr.Column) c1.expression()).column());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C4 — SELECT name AS n FROM users
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C4: SELECT name AS n FROM users")
    void c4_selectAlias() {
        var stmt = new Statement.Select(
            false,
            List.of(colAs("name", "n")),
            List.of(new TableRef("users", null)),
            List.of(), null, List.of(), null, List.of(), null);
        var plan = planner().plan(stmt);
        var proj = assertInstanceOf(LogicalPlan.Project.class, plan);
        var c0   = assertInstanceOf(OutputColumn.Expr.class, proj.columns().get(0));
        assertEquals("n", c0.alias());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C5 — SELECT * FROM users ORDER BY name ASC
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C5: SELECT * FROM users ORDER BY name ASC")
    void c5_orderBy() {
        var stmt = new Statement.Select(
            false,
            List.of(new OutputColumn.Star()),
            List.of(new TableRef("users", null)),
            List.of(), null, List.of(), null,
            List.of(new SortKey(new SqlExpr.Column(null, "name"), SortDir.ASC, NullOrder.NULLS_LAST)),
            null);
        var plan = planner().plan(stmt);
        var proj = assertInstanceOf(LogicalPlan.Project.class, plan);
        var sort = assertInstanceOf(LogicalPlan.Sort.class, proj.input());
        assertEquals(1, sort.keys().size());
        assertEquals(SortDir.ASC, sort.keys().get(0).direction());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C6 — SELECT * FROM users LIMIT 10
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C6: SELECT * FROM users LIMIT 10")
    void c6_limit() {
        var stmt = new Statement.Select(
            false,
            List.of(new OutputColumn.Star()),
            List.of(new TableRef("users", null)),
            List.of(), null, List.of(), null, List.of(),
            new LimitClause(10L, null));
        var plan  = planner().plan(stmt);
        var proj  = assertInstanceOf(LogicalPlan.Project.class, plan);
        var limit = assertInstanceOf(LogicalPlan.Limit.class, proj.input());
        assertEquals(10L, limit.count());
        assertNull(limit.offset());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C7 — SELECT DISTINCT name FROM users
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C7: SELECT DISTINCT name FROM users")
    void c7_distinct() {
        var stmt = new Statement.Select(
            true,
            List.of(col("name")),
            List.of(new TableRef("users", null)),
            List.of(), null, List.of(), null, List.of(), null);
        var plan  = planner().plan(stmt);
        var proj  = assertInstanceOf(LogicalPlan.Project.class, plan);
        var dist  = assertInstanceOf(LogicalPlan.Distinct.class, proj.input());
        assertInstanceOf(LogicalPlan.Scan.class, dist.input());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C8 — SELECT COUNT(*) FROM users GROUP BY age
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C8: SELECT COUNT(*) FROM users GROUP BY age")
    void c8_aggregate() {
        var countStar = new SqlExpr.AggExpr(AggFunction.COUNT, new AggArg.Star(), false);
        var stmt = new Statement.Select(
            false,
            List.of(new OutputColumn.Expr(countStar, null)),
            List.of(new TableRef("users", null)),
            List.of(), null,
            List.of(new SqlExpr.Column(null, "age")),
            null, List.of(), null);
        var plan = planner().plan(stmt);
        var proj = assertInstanceOf(LogicalPlan.Project.class, plan);
        var agg  = assertInstanceOf(LogicalPlan.Aggregate.class, proj.input());
        assertEquals(1, agg.groupBy().size());
        assertFalse(agg.aggregates().isEmpty());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C9 — SELECT … GROUP BY age HAVING COUNT(*) > 5
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C9: HAVING clause generates Having node")
    void c9_having() {
        var countStar = new SqlExpr.AggExpr(AggFunction.COUNT, new AggArg.Star(), false);
        var having    = new SqlExpr.BinaryOp(
            BinaryOperator.GT, countStar, new SqlExpr.Literal(5L));
        var stmt = new Statement.Select(
            false,
            List.of(col("age")),
            List.of(new TableRef("users", null)),
            List.of(), null,
            List.of(new SqlExpr.Column(null, "age")),
            having, List.of(), null);
        var plan = planner().plan(stmt);
        var proj = assertInstanceOf(LogicalPlan.Project.class, plan);
        var hav  = assertInstanceOf(LogicalPlan.Having.class, proj.input());
        assertInstanceOf(LogicalPlan.Aggregate.class, hav.input());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C10 — INSERT INTO users VALUES (…)
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C10: INSERT into known table produces Insert plan")
    void c10_insert() {
        var stmt = new Statement.Insert(
            "users",
            List.of("id", "name", "age", "email"),
            List.of(List.of(
                new SqlExpr.Literal(1L),
                new SqlExpr.Literal("Alice"),
                new SqlExpr.Literal(30L),
                new SqlExpr.Literal("alice@example.com"))));
        var plan = planner().plan(stmt);
        var ins  = assertInstanceOf(LogicalPlan.Insert.class, plan);
        assertEquals("users", ins.table());
        assertEquals(4, ins.columns().size());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C11 — UPDATE users SET age = 31 WHERE id = 1
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C11: UPDATE produces Update plan")
    void c11_update() {
        var stmt = new Statement.Update(
            "users",
            List.of(new Assignment("age", new SqlExpr.Literal(31L))),
            new SqlExpr.BinaryOp(BinaryOperator.EQ, new SqlExpr.Column(null, "id"), new SqlExpr.Literal(1L)));
        var plan = planner().plan(stmt);
        var upd  = assertInstanceOf(LogicalPlan.Update.class, plan);
        assertEquals("users", upd.table());
        assertEquals(1, upd.assignments().size());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C12 — DELETE FROM users WHERE id = 1
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C12: DELETE produces Delete plan")
    void c12_delete() {
        var stmt = new Statement.Delete(
            "users",
            new SqlExpr.BinaryOp(BinaryOperator.EQ, new SqlExpr.Column(null, "id"), new SqlExpr.Literal(1L)));
        var plan = planner().plan(stmt);
        var del  = assertInstanceOf(LogicalPlan.Delete.class, plan);
        assertEquals("users", del.table());
        assertNotNull(del.predicate());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C13 — CREATE TABLE + DROP TABLE
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C13a: CREATE TABLE produces CreateTable plan")
    void c13a_createTable() {
        var stmt = new Statement.CreateTable(
            "logs", false,
            List.of(new ColumnDef("id", "INTEGER", true, true, false, null)));
        var plan = planner().plan(stmt);
        var ct   = assertInstanceOf(LogicalPlan.CreateTable.class, plan);
        assertEquals("logs", ct.table());
        assertFalse(ct.ifNotExists());
    }

    @Test @DisplayName("C13b: DROP TABLE produces DropTable plan")
    void c13b_dropTable() {
        var stmt = new Statement.DropTable("logs", true);
        var plan = planner().plan(stmt);
        var dt   = assertInstanceOf(LogicalPlan.DropTable.class, plan);
        assertEquals("logs", dt.table());
        assertTrue(dt.ifExists());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Structural tests
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("Struct: multi-FROM cross join")
    void struct_multiFromCrossJoin() {
        var stmt = new Statement.Select(
            false,
            List.of(new OutputColumn.Star()),
            List.of(new TableRef("users", null), new TableRef("orders", null)),
            List.of(), null, List.of(), null, List.of(), null);
        var plan = planner().plan(stmt);
        var proj = assertInstanceOf(LogicalPlan.Project.class, plan);
        var join = assertInstanceOf(LogicalPlan.Join.class, proj.input());
        assertEquals(JoinKind.CROSS, join.kind());
    }

    @Test @DisplayName("Struct: INNER JOIN on condition")
    void struct_innerJoin() {
        var on   = new SqlExpr.BinaryOp(
            BinaryOperator.EQ,
            new SqlExpr.Column("users", "id"),
            new SqlExpr.Column("orders", "user_id"));
        var jc   = new JoinClause(JoinKind.INNER, "orders", null, on);
        var stmt = new Statement.Select(
            false,
            List.of(new OutputColumn.Star()),
            List.of(new TableRef("users", null)),
            List.of(jc), null, List.of(), null, List.of(), null);
        var plan = planner().plan(stmt);
        var proj = assertInstanceOf(LogicalPlan.Project.class, plan);
        var join = assertInstanceOf(LogicalPlan.Join.class, proj.input());
        assertEquals(JoinKind.INNER, join.kind());
    }

    @Test @DisplayName("Struct: table alias resolves correctly")
    void struct_tableAlias() {
        var stmt = new Statement.Select(
            false,
            List.of(new OutputColumn.Expr(new SqlExpr.Column("u", "name"), null)),
            List.of(new TableRef("users", "u")),
            List.of(), null, List.of(), null, List.of(), null);
        var plan = planner().plan(stmt);
        var proj = assertInstanceOf(LogicalPlan.Project.class, plan);
        var c0   = assertInstanceOf(OutputColumn.Expr.class, proj.columns().get(0));
        var col  = assertInstanceOf(SqlExpr.Column.class, c0.expression());
        assertEquals("u", col.table());
        assertEquals("name", col.column());
    }

    @Test @DisplayName("Struct: DISTINCT + ORDER BY + LIMIT stacking")
    void struct_distinctSortLimit() {
        var stmt = new Statement.Select(
            true,
            List.of(col("name")),
            List.of(new TableRef("users", null)),
            List.of(), null, List.of(), null,
            List.of(new SortKey(new SqlExpr.Column(null, "name"), SortDir.DESC, NullOrder.NULLS_LAST)),
            new LimitClause(5L, 2L));
        var plan  = planner().plan(stmt);
        var proj  = assertInstanceOf(LogicalPlan.Project.class, plan);
        var limit = assertInstanceOf(LogicalPlan.Limit.class, proj.input());
        var sort  = assertInstanceOf(LogicalPlan.Sort.class, limit.input());
        var dist  = assertInstanceOf(LogicalPlan.Distinct.class, sort.input());
        assertInstanceOf(LogicalPlan.Scan.class, dist.input());
        assertEquals(5L, limit.count());
        assertEquals(2L, limit.offset());
    }

    @Test @DisplayName("Struct: planAll returns one plan per statement")
    void struct_planAll() {
        var stmts = List.of(
            (Statement) selectStar("users"),
            new Statement.DropTable("nonexistent_but_ifExists", true));
        var plans = planner().planAll(stmts);
        assertEquals(2, plans.size());
        assertInstanceOf(LogicalPlan.Project.class, plans.get(0));
        assertInstanceOf(LogicalPlan.DropTable.class, plans.get(1));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Error tests
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("Error: unknown table in FROM throws UnknownTableException")
    void error_unknownTable() {
        var ex = assertThrows(UnknownTableException.class,
            () -> planner().plan(selectStar("ghost")));
        assertEquals("ghost", ex.table());
    }

    @Test @DisplayName("Error: unknown column in WHERE throws UnknownColumnException")
    void error_unknownColumn() {
        var where = new SqlExpr.BinaryOp(
            BinaryOperator.EQ,
            new SqlExpr.Column(null, "no_such_col"),
            new SqlExpr.Literal(1L));
        assertThrows(UnknownColumnException.class,
            () -> planner().plan(selectStarWhere("users", where)));
    }

    @Test @DisplayName("Error: ambiguous unqualified column throws AmbiguousColumnException")
    void error_ambiguousColumn() {
        // Both users.id and orders.id exist — selecting unqualified "id" is ambiguous.
        var stmt = new Statement.Select(
            false,
            List.of(new OutputColumn.Expr(new SqlExpr.Column(null, "id"), null)),
            List.of(new TableRef("users", null), new TableRef("orders", null)),
            List.of(), null, List.of(), null, List.of(), null);
        var ex = assertThrows(AmbiguousColumnException.class,
            () -> planner().plan(stmt));
        assertEquals("id", ex.column());
        assertTrue(ex.tables().size() >= 2);
    }

    @Test @DisplayName("Error: qualified column against unknown alias throws UnknownTableException")
    void error_unknownAlias() {
        var stmt = new Statement.Select(
            false,
            List.of(new OutputColumn.Expr(new SqlExpr.Column("x", "id"), null)),
            List.of(new TableRef("users", null)),
            List.of(), null, List.of(), null, List.of(), null);
        assertThrows(UnknownTableException.class,
            () -> planner().plan(stmt));
    }

    @Test @DisplayName("Error: INSERT into unknown table throws UnknownTableException")
    void error_insertUnknownTable() {
        var stmt = new Statement.Insert("nope", List.of("id"), List.of(List.of(new SqlExpr.Literal(1L))));
        assertThrows(UnknownTableException.class, () -> planner().plan(stmt));
    }

    @Test @DisplayName("Error: UPDATE on unknown table throws UnknownTableException")
    void error_updateUnknownTable() {
        var stmt = new Statement.Update(
            "nope",
            List.of(new Assignment("id", new SqlExpr.Literal(1L))),
            null);
        assertThrows(UnknownTableException.class, () -> planner().plan(stmt));
    }

    @Test @DisplayName("Error: DELETE from unknown table throws UnknownTableException")
    void error_deleteUnknownTable() {
        var stmt = new Statement.Delete("nope", null);
        assertThrows(UnknownTableException.class, () -> planner().plan(stmt));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Expression-type tests
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("Expr: IS NULL predicate resolves inner column")
    void expr_isNull() {
        var where = new SqlExpr.IsNull(new SqlExpr.Column(null, "email"));
        var plan  = planner().plan(selectStarWhere("users", where));
        var proj  = assertInstanceOf(LogicalPlan.Project.class, plan);
        var filt  = assertInstanceOf(LogicalPlan.Filter.class, proj.input());
        var isNull = assertInstanceOf(SqlExpr.IsNull.class, filt.predicate());
        var col    = assertInstanceOf(SqlExpr.Column.class, isNull.operand());
        assertEquals("email", col.column());
    }

    @Test @DisplayName("Expr: IS NOT NULL predicate resolves inner column")
    void expr_isNotNull() {
        var where  = new SqlExpr.IsNotNull(new SqlExpr.Column(null, "email"));
        var plan   = planner().plan(selectStarWhere("users", where));
        var proj   = assertInstanceOf(LogicalPlan.Project.class, plan);
        var filt   = assertInstanceOf(LogicalPlan.Filter.class, proj.input());
        assertInstanceOf(SqlExpr.IsNotNull.class, filt.predicate());
    }

    @Test @DisplayName("Expr: BETWEEN resolves value, lo, hi columns")
    void expr_between() {
        var where = new SqlExpr.Between(
            new SqlExpr.Column(null, "age"),
            new SqlExpr.Literal(18L),
            new SqlExpr.Literal(65L));
        var plan  = planner().plan(selectStarWhere("users", where));
        var proj  = assertInstanceOf(LogicalPlan.Project.class, plan);
        var filt  = assertInstanceOf(LogicalPlan.Filter.class, proj.input());
        var bet   = assertInstanceOf(SqlExpr.Between.class, filt.predicate());
        assertEquals("age", ((SqlExpr.Column) bet.value()).column());
    }

    @Test @DisplayName("Expr: IN resolves value and all items")
    void expr_in() {
        var where = new SqlExpr.In(
            new SqlExpr.Column(null, "age"),
            List.of(new SqlExpr.Literal(20L), new SqlExpr.Literal(30L)));
        var plan  = planner().plan(selectStarWhere("users", where));
        var proj  = assertInstanceOf(LogicalPlan.Project.class, plan);
        var filt  = assertInstanceOf(LogicalPlan.Filter.class, proj.input());
        var in    = assertInstanceOf(SqlExpr.In.class, filt.predicate());
        assertEquals(2, in.items().size());
    }

    @Test @DisplayName("Expr: NOT IN resolves value and all items")
    void expr_notIn() {
        var where = new SqlExpr.NotIn(
            new SqlExpr.Column(null, "age"),
            List.of(new SqlExpr.Literal(0L)));
        var plan  = planner().plan(selectStarWhere("users", where));
        var proj  = assertInstanceOf(LogicalPlan.Project.class, plan);
        var filt  = assertInstanceOf(LogicalPlan.Filter.class, proj.input());
        assertInstanceOf(SqlExpr.NotIn.class, filt.predicate());
    }

    @Test @DisplayName("Expr: LIKE resolves value column")
    void expr_like() {
        var where = new SqlExpr.Like(new SqlExpr.Column(null, "name"), "%Alice%");
        var plan  = planner().plan(selectStarWhere("users", where));
        var proj  = assertInstanceOf(LogicalPlan.Project.class, plan);
        var filt  = assertInstanceOf(LogicalPlan.Filter.class, proj.input());
        var like  = assertInstanceOf(SqlExpr.Like.class, filt.predicate());
        assertEquals("%Alice%", like.pattern());
    }

    @Test @DisplayName("Expr: NOT LIKE resolves value column")
    void expr_notLike() {
        var where = new SqlExpr.NotLike(new SqlExpr.Column(null, "name"), "%Bob%");
        var plan  = planner().plan(selectStarWhere("users", where));
        var proj  = assertInstanceOf(LogicalPlan.Project.class, plan);
        var filt  = assertInstanceOf(LogicalPlan.Filter.class, proj.input());
        assertInstanceOf(SqlExpr.NotLike.class, filt.predicate());
    }

    @Test @DisplayName("Expr: unary NOT resolves operand column")
    void expr_unaryNot() {
        var where = new SqlExpr.UnaryOp(
            UnaryOperator.NOT,
            new SqlExpr.BinaryOp(
                BinaryOperator.EQ,
                new SqlExpr.Column(null, "age"),
                new SqlExpr.Literal(0L)));
        var plan  = planner().plan(selectStarWhere("users", where));
        var proj  = assertInstanceOf(LogicalPlan.Project.class, plan);
        var filt  = assertInstanceOf(LogicalPlan.Filter.class, proj.input());
        assertInstanceOf(SqlExpr.UnaryOp.class, filt.predicate());
    }

    @Test @DisplayName("Expr: FuncCall resolves all argument columns")
    void expr_funcCall() {
        var where = new SqlExpr.BinaryOp(
            BinaryOperator.GT,
            new SqlExpr.FuncCall("LENGTH", List.of(new SqlExpr.Column(null, "name"))),
            new SqlExpr.Literal(3L));
        var plan  = planner().plan(selectStarWhere("users", where));
        var proj  = assertInstanceOf(LogicalPlan.Project.class, plan);
        var filt  = assertInstanceOf(LogicalPlan.Filter.class, proj.input());
        var bop   = assertInstanceOf(SqlExpr.BinaryOp.class, filt.predicate());
        var fn    = assertInstanceOf(SqlExpr.FuncCall.class, bop.left());
        assertEquals("LENGTH", fn.name());
    }

    // ─── Error propagation inside expressions ─────────────────────────────────

    @Test @DisplayName("Expr error: BETWEEN with bad value column throws")
    void exprErr_betweenValue() {
        var where = new SqlExpr.Between(
            new SqlExpr.Column(null, "ghost_col"),
            new SqlExpr.Literal(1L),
            new SqlExpr.Literal(10L));
        assertThrows(UnknownColumnException.class,
            () -> planner().plan(selectStarWhere("users", where)));
    }

    @Test @DisplayName("Expr error: BETWEEN with bad lo column throws")
    void exprErr_betweenLo() {
        var where = new SqlExpr.Between(
            new SqlExpr.Column(null, "age"),
            new SqlExpr.Column(null, "ghost_lo"),
            new SqlExpr.Literal(10L));
        assertThrows(UnknownColumnException.class,
            () -> planner().plan(selectStarWhere("users", where)));
    }

    @Test @DisplayName("Expr error: BETWEEN with bad hi column throws")
    void exprErr_betweenHi() {
        var where = new SqlExpr.Between(
            new SqlExpr.Column(null, "age"),
            new SqlExpr.Literal(1L),
            new SqlExpr.Column(null, "ghost_hi"));
        assertThrows(UnknownColumnException.class,
            () -> planner().plan(selectStarWhere("users", where)));
    }

    @Test @DisplayName("Expr error: IN with bad item column throws")
    void exprErr_inItem() {
        var where = new SqlExpr.In(
            new SqlExpr.Column(null, "age"),
            List.of(new SqlExpr.Column(null, "ghost_col")));
        assertThrows(UnknownColumnException.class,
            () -> planner().plan(selectStarWhere("users", where)));
    }

    @Test @DisplayName("Expr error: NOT IN with bad item column throws")
    void exprErr_notInItem() {
        var where = new SqlExpr.NotIn(
            new SqlExpr.Column(null, "age"),
            List.of(new SqlExpr.Column(null, "ghost_col")));
        assertThrows(UnknownColumnException.class,
            () -> planner().plan(selectStarWhere("users", where)));
    }

    @Test @DisplayName("Expr error: FuncCall with bad arg column throws")
    void exprErr_funcCallArg() {
        var where = new SqlExpr.BinaryOp(
            BinaryOperator.GT,
            new SqlExpr.FuncCall("LENGTH", List.of(new SqlExpr.Column(null, "ghost_col"))),
            new SqlExpr.Literal(3L));
        assertThrows(UnknownColumnException.class,
            () -> planner().plan(selectStarWhere("users", where)));
    }

    @Test @DisplayName("Expr error: IS NULL with bad inner column throws")
    void exprErr_isNullBadCol() {
        var where = new SqlExpr.IsNull(new SqlExpr.Column(null, "ghost_col"));
        assertThrows(UnknownColumnException.class,
            () -> planner().plan(selectStarWhere("users", where)));
    }

    @Test @DisplayName("Expr error: IS NOT NULL with bad inner column throws")
    void exprErr_isNotNullBadCol() {
        var where = new SqlExpr.IsNotNull(new SqlExpr.Column(null, "ghost_col"));
        assertThrows(UnknownColumnException.class,
            () -> planner().plan(selectStarWhere("users", where)));
    }

    @Test @DisplayName("Expr error: LIKE with bad value column throws")
    void exprErr_likeBadCol() {
        var where = new SqlExpr.Like(new SqlExpr.Column(null, "ghost_col"), "%x%");
        assertThrows(UnknownColumnException.class,
            () -> planner().plan(selectStarWhere("users", where)));
    }

    @Test @DisplayName("Expr error: NOT LIKE with bad value column throws")
    void exprErr_notLikeBadCol() {
        var where = new SqlExpr.NotLike(new SqlExpr.Column(null, "ghost_col"), "%x%");
        assertThrows(UnknownColumnException.class,
            () -> planner().plan(selectStarWhere("users", where)));
    }

    // ─── Literal / passthrough ─────────────────────────────────────────────────

    @Test @DisplayName("Expr: Literal null value is preserved")
    void expr_literalNull() {
        var where = new SqlExpr.IsNull(new SqlExpr.Literal(null));
        assertDoesNotThrow(() -> planner().plan(selectStarWhere("users", where)));
    }

    @Test @DisplayName("Aggregate: SUM(amount) with no GROUP BY still produces Aggregate node")
    void agg_sumNoGroupBy() {
        var sum  = new SqlExpr.AggExpr(AggFunction.SUM, new AggArg.Expr(new SqlExpr.Column(null, "amount")), false);
        var stmt = new Statement.Select(
            false,
            List.of(new OutputColumn.Expr(sum, null)),
            List.of(new TableRef("orders", null)),
            List.of(), null, List.of(), null, List.of(), null);
        var plan = planner().plan(stmt);
        var proj = assertInstanceOf(LogicalPlan.Project.class, plan);
        assertInstanceOf(LogicalPlan.Aggregate.class, proj.input());
    }

    @Test @DisplayName("Aggregate: COUNT DISTINCT")
    void agg_countDistinct() {
        var cd   = new SqlExpr.AggExpr(AggFunction.COUNT, new AggArg.Expr(new SqlExpr.Column(null, "name")), true);
        var stmt = new Statement.Select(
            false,
            List.of(new OutputColumn.Expr(cd, null)),
            List.of(new TableRef("users", null)),
            List.of(), null, List.of(), null, List.of(), null);
        var plan = planner().plan(stmt);
        var proj = assertInstanceOf(LogicalPlan.Project.class, plan);
        var agg  = assertInstanceOf(LogicalPlan.Aggregate.class, proj.input());
        assertTrue(agg.aggregates().get(0).distinct());
    }
}

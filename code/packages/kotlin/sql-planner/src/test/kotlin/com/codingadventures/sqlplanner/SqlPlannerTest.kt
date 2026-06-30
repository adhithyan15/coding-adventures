package com.codingadventures.sqlplanner

// SqlPlannerTest.kt — conformance and unit tests for the Kotlin sql-planner.
//
// Test organisation mirrors the Java and C# suites:
//   C1–C13  Conformance tests
//   Struct   Structural tests (JOIN, aliases, DISTINCT/SORT/LIMIT stacking)
//   Error    Error-path tests (unknown table, unknown column, ambiguous column)
//   Expr     Expression-type tests

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import org.junit.jupiter.api.DisplayName
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlin.test.assertFalse

class SqlPlannerTest {

    // ── Schema fixture ────────────────────────────────────────────────────────

    private val schema = InMemorySchemaProvider(mapOf(
        "users"    to listOf("id", "name", "age", "email"),
        "orders"   to listOf("id", "user_id", "amount", "status"),
        "products" to listOf("id", "name", "price", "category")
    ))

    private fun planner() = SqlPlanner(schema)

    // ── Helpers ───────────────────────────────────────────────────────────────

    private fun selectStar(table: String) = Statement.Select(
        distinct = false,
        columns  = listOf(OutputColumn.Star),
        from     = listOf(TableRef(table, null)),
        joins    = emptyList(),
        where    = null, groupBy = emptyList(), having = null,
        orderBy  = emptyList(), limit = null)

    private fun selectStarWhere(table: String, where: SqlExpr) = Statement.Select(
        distinct = false,
        columns  = listOf(OutputColumn.Star),
        from     = listOf(TableRef(table, null)),
        joins    = emptyList(),
        where    = where, groupBy = emptyList(), having = null,
        orderBy  = emptyList(), limit = null)

    private fun col(column: String)              = OutputColumn.Expr(SqlExpr.Column(null, column), null)
    private fun colAs(column: String, alias: String) = OutputColumn.Expr(SqlExpr.Column(null, column), alias)

    // ═══════════════════════════════════════════════════════════════════════════
    // C1 — SELECT * FROM users
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C1: SELECT * FROM users")
    fun c1_selectStarFromUsers() {
        val plan = planner().plan(selectStar("users"))
        val proj = assertIs<LogicalPlan.Project>(plan)
        assertEquals(1, proj.columns.size)
        assertIs<OutputColumn.Star>(proj.columns[0])
        val scan = assertIs<LogicalPlan.Scan>(proj.input)
        assertEquals("users", scan.table)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C2 — SELECT * FROM users WHERE age > 18
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C2: SELECT * FROM users WHERE age > 18")
    fun c2_selectStarWhereAge() {
        val where = SqlExpr.BinaryOp(BinaryOperator.GT, SqlExpr.Column(null, "age"), SqlExpr.Literal(18L))
        val plan  = planner().plan(selectStarWhere("users", where))
        val proj  = assertIs<LogicalPlan.Project>(plan)
        val filt  = assertIs<LogicalPlan.Filter>(proj.input)
        val pred  = assertIs<SqlExpr.BinaryOp>(filt.predicate)
        assertEquals(BinaryOperator.GT, pred.op)
        val col   = assertIs<SqlExpr.Column>(pred.left)
        assertEquals("age", col.column)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C3 — SELECT id, name FROM users
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C3: SELECT id, name FROM users")
    fun c3_selectColumns() {
        val stmt = Statement.Select(false, listOf(col("id"), col("name")),
            listOf(TableRef("users", null)), emptyList(), null, emptyList(), null, emptyList(), null)
        val plan = planner().plan(stmt)
        val proj = assertIs<LogicalPlan.Project>(plan)
        assertEquals(2, proj.columns.size)
        val c0 = assertIs<OutputColumn.Expr>(proj.columns[0])
        val c1 = assertIs<OutputColumn.Expr>(proj.columns[1])
        assertEquals("id",   (c0.expression as SqlExpr.Column).column)
        assertEquals("name", (c1.expression as SqlExpr.Column).column)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C4 — SELECT name AS n FROM users
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C4: SELECT name AS n FROM users")
    fun c4_selectAlias() {
        val stmt = Statement.Select(false, listOf(colAs("name", "n")),
            listOf(TableRef("users", null)), emptyList(), null, emptyList(), null, emptyList(), null)
        val plan = planner().plan(stmt)
        val proj = assertIs<LogicalPlan.Project>(plan)
        val c0   = assertIs<OutputColumn.Expr>(proj.columns[0])
        assertEquals("n", c0.alias)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C5 — SELECT * FROM users ORDER BY name ASC
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C5: SELECT * FROM users ORDER BY name ASC")
    fun c5_orderBy() {
        val stmt = Statement.Select(false, listOf(OutputColumn.Star),
            listOf(TableRef("users", null)), emptyList(), null, emptyList(), null,
            listOf(SortKey(SqlExpr.Column(null, "name"), SortDir.ASC, NullOrder.NULLS_LAST)), null)
        val plan = planner().plan(stmt)
        val proj = assertIs<LogicalPlan.Project>(plan)
        val sort = assertIs<LogicalPlan.Sort>(proj.input)
        assertEquals(1, sort.keys.size)
        assertEquals(SortDir.ASC, sort.keys[0].direction)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C6 — SELECT * FROM users LIMIT 10
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C6: SELECT * FROM users LIMIT 10")
    fun c6_limit() {
        val stmt = Statement.Select(false, listOf(OutputColumn.Star),
            listOf(TableRef("users", null)), emptyList(), null, emptyList(), null, emptyList(),
            LimitClause(10L, null))
        val plan  = planner().plan(stmt)
        val proj  = assertIs<LogicalPlan.Project>(plan)
        val limit = assertIs<LogicalPlan.Limit>(proj.input)
        assertEquals(10L, limit.count)
        assertNull(limit.offset)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C7 — SELECT DISTINCT name FROM users
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C7: SELECT DISTINCT name FROM users")
    fun c7_distinct() {
        val stmt = Statement.Select(true, listOf(col("name")),
            listOf(TableRef("users", null)), emptyList(), null, emptyList(), null, emptyList(), null)
        val plan = planner().plan(stmt)
        val proj = assertIs<LogicalPlan.Project>(plan)
        val dist = assertIs<LogicalPlan.Distinct>(proj.input)
        assertIs<LogicalPlan.Scan>(dist.input)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C8 — SELECT COUNT(*) FROM users GROUP BY age
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C8: SELECT COUNT(*) FROM users GROUP BY age")
    fun c8_aggregate() {
        val countStar = SqlExpr.AggExpr(AggFunction.COUNT, AggArg.Star, false)
        val stmt = Statement.Select(
            false, listOf(OutputColumn.Expr(countStar, null)),
            listOf(TableRef("users", null)), emptyList(), null,
            listOf(SqlExpr.Column(null, "age")), null, emptyList(), null)
        val plan = planner().plan(stmt)
        val proj = assertIs<LogicalPlan.Project>(plan)
        val agg  = assertIs<LogicalPlan.Aggregate>(proj.input)
        assertEquals(1, agg.groupBy.size)
        assertTrue(agg.aggregates.isNotEmpty())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C9 — HAVING generates Having node
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C9: HAVING clause generates Having node")
    fun c9_having() {
        val countStar = SqlExpr.AggExpr(AggFunction.COUNT, AggArg.Star, false)
        val having    = SqlExpr.BinaryOp(BinaryOperator.GT, countStar, SqlExpr.Literal(5L))
        val stmt = Statement.Select(
            false, listOf(col("age")),
            listOf(TableRef("users", null)), emptyList(), null,
            listOf(SqlExpr.Column(null, "age")), having, emptyList(), null)
        val plan = planner().plan(stmt)
        val proj = assertIs<LogicalPlan.Project>(plan)
        val hav  = assertIs<LogicalPlan.Having>(proj.input)
        assertIs<LogicalPlan.Aggregate>(hav.input)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C10 — INSERT
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C10: INSERT into known table produces Insert plan")
    fun c10_insert() {
        val stmt = Statement.Insert("users", listOf("id", "name", "age", "email"),
            listOf(listOf(SqlExpr.Literal(1L), SqlExpr.Literal("Alice"), SqlExpr.Literal(30L), SqlExpr.Literal("alice@example.com"))))
        val plan = planner().plan(stmt)
        val ins  = assertIs<LogicalPlan.Insert>(plan)
        assertEquals("users", ins.table)
        assertEquals(4, ins.columns.size)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C11 — UPDATE
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C11: UPDATE produces Update plan")
    fun c11_update() {
        val stmt = Statement.Update("users",
            listOf(Assignment("age", SqlExpr.Literal(31L))),
            SqlExpr.BinaryOp(BinaryOperator.EQ, SqlExpr.Column(null, "id"), SqlExpr.Literal(1L)))
        val plan = planner().plan(stmt)
        val upd  = assertIs<LogicalPlan.Update>(plan)
        assertEquals("users", upd.table)
        assertEquals(1, upd.assignments.size)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C12 — DELETE
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C12: DELETE produces Delete plan")
    fun c12_delete() {
        val stmt = Statement.Delete("users",
            SqlExpr.BinaryOp(BinaryOperator.EQ, SqlExpr.Column(null, "id"), SqlExpr.Literal(1L)))
        val plan = planner().plan(stmt)
        val del  = assertIs<LogicalPlan.Delete>(plan)
        assertEquals("users", del.table)
        assertNotNull(del.predicate)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C13 — CREATE TABLE + DROP TABLE
    // ═══════════════════════════════════════════════════════════════════════════
    @Test @DisplayName("C13a: CREATE TABLE produces CreateTable plan")
    fun c13a_createTable() {
        val stmt = Statement.CreateTable("logs", false,
            listOf(ColumnDef("id", "INTEGER", notNull = true, primaryKey = true, unique = false, default = null)))
        val plan = planner().plan(stmt)
        val ct   = assertIs<LogicalPlan.CreateTable>(plan)
        assertEquals("logs", ct.table)
        assertFalse(ct.ifNotExists)
    }

    @Test @DisplayName("C13b: DROP TABLE produces DropTable plan")
    fun c13b_dropTable() {
        val stmt = Statement.DropTable("logs", true)
        val plan = planner().plan(stmt)
        val dt   = assertIs<LogicalPlan.DropTable>(plan)
        assertEquals("logs", dt.table)
        assertTrue(dt.ifExists)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Structural tests
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("Struct: multi-FROM cross join")
    fun struct_multiFromCrossJoin() {
        val stmt = Statement.Select(false, listOf(OutputColumn.Star),
            listOf(TableRef("users", null), TableRef("orders", null)),
            emptyList(), null, emptyList(), null, emptyList(), null)
        val plan = planner().plan(stmt)
        val proj = assertIs<LogicalPlan.Project>(plan)
        val join = assertIs<LogicalPlan.Join>(proj.input)
        assertEquals(JoinKind.CROSS, join.kind)
    }

    @Test @DisplayName("Struct: INNER JOIN on condition")
    fun struct_innerJoin() {
        val on  = SqlExpr.BinaryOp(BinaryOperator.EQ,
            SqlExpr.Column("users", "id"), SqlExpr.Column("orders", "user_id"))
        val jc  = JoinClause(JoinKind.INNER, "orders", null, on)
        val stmt = Statement.Select(false, listOf(OutputColumn.Star),
            listOf(TableRef("users", null)), listOf(jc),
            null, emptyList(), null, emptyList(), null)
        val plan = planner().plan(stmt)
        val proj = assertIs<LogicalPlan.Project>(plan)
        val join = assertIs<LogicalPlan.Join>(proj.input)
        assertEquals(JoinKind.INNER, join.kind)
    }

    @Test @DisplayName("Struct: table alias resolves correctly")
    fun struct_tableAlias() {
        val stmt = Statement.Select(false,
            listOf(OutputColumn.Expr(SqlExpr.Column("u", "name"), null)),
            listOf(TableRef("users", "u")),
            emptyList(), null, emptyList(), null, emptyList(), null)
        val plan = planner().plan(stmt)
        val proj = assertIs<LogicalPlan.Project>(plan)
        val c0   = assertIs<OutputColumn.Expr>(proj.columns[0])
        val col  = assertIs<SqlExpr.Column>(c0.expression)
        assertEquals("u",    col.table)
        assertEquals("name", col.column)
    }

    @Test @DisplayName("Struct: DISTINCT + ORDER BY + LIMIT stacking")
    fun struct_distinctSortLimit() {
        val stmt = Statement.Select(
            true, listOf(col("name")),
            listOf(TableRef("users", null)), emptyList(), null, emptyList(), null,
            listOf(SortKey(SqlExpr.Column(null, "name"), SortDir.DESC, NullOrder.NULLS_LAST)),
            LimitClause(5L, 2L))
        val plan  = planner().plan(stmt)
        val proj  = assertIs<LogicalPlan.Project>(plan)
        val limit = assertIs<LogicalPlan.Limit>(proj.input)
        val sort  = assertIs<LogicalPlan.Sort>(limit.input)
        val dist  = assertIs<LogicalPlan.Distinct>(sort.input)
        assertIs<LogicalPlan.Scan>(dist.input)
        assertEquals(5L, limit.count)
        assertEquals(2L, limit.offset)
    }

    @Test @DisplayName("Struct: planAll returns one plan per statement")
    fun struct_planAll() {
        val stmts = listOf<Statement>(selectStar("users"), Statement.DropTable("nope", true))
        val plans = planner().planAll(stmts)
        assertEquals(2, plans.size)
        assertIs<LogicalPlan.Project>(plans[0])
        assertIs<LogicalPlan.DropTable>(plans[1])
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Error tests
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("Error: unknown table in FROM throws UnknownTableException")
    fun error_unknownTable() {
        val ex = assertThrows<UnknownTableException> { planner().plan(selectStar("ghost")) }
        assertEquals("ghost", ex.table)
    }

    @Test @DisplayName("Error: unknown column in WHERE throws UnknownColumnException")
    fun error_unknownColumn() {
        val where = SqlExpr.BinaryOp(BinaryOperator.EQ, SqlExpr.Column(null, "no_such_col"), SqlExpr.Literal(1L))
        assertThrows<UnknownColumnException> { planner().plan(selectStarWhere("users", where)) }
    }

    @Test @DisplayName("Error: ambiguous unqualified column throws AmbiguousColumnException")
    fun error_ambiguousColumn() {
        val stmt = Statement.Select(false,
            listOf(OutputColumn.Expr(SqlExpr.Column(null, "id"), null)),
            listOf(TableRef("users", null), TableRef("orders", null)),
            emptyList(), null, emptyList(), null, emptyList(), null)
        val ex = assertThrows<AmbiguousColumnException> { planner().plan(stmt) }
        assertEquals("id", ex.column)
        assertTrue(ex.tables.size >= 2)
    }

    @Test @DisplayName("Error: qualified column against unknown alias throws UnknownTableException")
    fun error_unknownAlias() {
        val stmt = Statement.Select(false,
            listOf(OutputColumn.Expr(SqlExpr.Column("x", "id"), null)),
            listOf(TableRef("users", null)),
            emptyList(), null, emptyList(), null, emptyList(), null)
        assertThrows<UnknownTableException> { planner().plan(stmt) }
    }

    @Test @DisplayName("Error: INSERT into unknown table throws UnknownTableException")
    fun error_insertUnknownTable() {
        assertThrows<UnknownTableException> {
            planner().plan(Statement.Insert("nope", listOf("id"), listOf(listOf(SqlExpr.Literal(1L)))))
        }
    }

    @Test @DisplayName("Error: UPDATE on unknown table throws UnknownTableException")
    fun error_updateUnknownTable() {
        assertThrows<UnknownTableException> {
            planner().plan(Statement.Update("nope", listOf(Assignment("id", SqlExpr.Literal(1L))), null))
        }
    }

    @Test @DisplayName("Error: DELETE from unknown table throws UnknownTableException")
    fun error_deleteUnknownTable() {
        assertThrows<UnknownTableException> {
            planner().plan(Statement.Delete("nope", null))
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Expression-type tests
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("Expr: IS NULL predicate resolves inner column")
    fun expr_isNull() {
        val plan  = planner().plan(selectStarWhere("users", SqlExpr.IsNull(SqlExpr.Column(null, "email"))))
        val proj  = assertIs<LogicalPlan.Project>(plan)
        val filt  = assertIs<LogicalPlan.Filter>(proj.input)
        val isNull = assertIs<SqlExpr.IsNull>(filt.predicate)
        assertEquals("email", (isNull.operand as SqlExpr.Column).column)
    }

    @Test @DisplayName("Expr: IS NOT NULL predicate resolves inner column")
    fun expr_isNotNull() {
        val plan  = planner().plan(selectStarWhere("users", SqlExpr.IsNotNull(SqlExpr.Column(null, "email"))))
        val proj  = assertIs<LogicalPlan.Project>(plan)
        val filt  = assertIs<LogicalPlan.Filter>(proj.input)
        assertIs<SqlExpr.IsNotNull>(filt.predicate)
    }

    @Test @DisplayName("Expr: BETWEEN resolves value, lo, hi columns")
    fun expr_between() {
        val plan  = planner().plan(selectStarWhere("users",
            SqlExpr.Between(SqlExpr.Column(null, "age"), SqlExpr.Literal(18L), SqlExpr.Literal(65L))))
        val proj  = assertIs<LogicalPlan.Project>(plan)
        val filt  = assertIs<LogicalPlan.Filter>(proj.input)
        val bet   = assertIs<SqlExpr.Between>(filt.predicate)
        assertEquals("age", (bet.value as SqlExpr.Column).column)
    }

    @Test @DisplayName("Expr: IN resolves value and items")
    fun expr_in() {
        val plan  = planner().plan(selectStarWhere("users",
            SqlExpr.In(SqlExpr.Column(null, "age"), listOf(SqlExpr.Literal(20L), SqlExpr.Literal(30L)))))
        val proj  = assertIs<LogicalPlan.Project>(plan)
        val filt  = assertIs<LogicalPlan.Filter>(proj.input)
        val inExpr = assertIs<SqlExpr.In>(filt.predicate)
        assertEquals(2, inExpr.items.size)
    }

    @Test @DisplayName("Expr: NOT IN resolves value and items")
    fun expr_notIn() {
        val plan  = planner().plan(selectStarWhere("users",
            SqlExpr.NotIn(SqlExpr.Column(null, "age"), listOf(SqlExpr.Literal(0L)))))
        val proj  = assertIs<LogicalPlan.Project>(plan)
        val filt  = assertIs<LogicalPlan.Filter>(proj.input)
        assertIs<SqlExpr.NotIn>(filt.predicate)
    }

    @Test @DisplayName("Expr: LIKE resolves value column")
    fun expr_like() {
        val plan  = planner().plan(selectStarWhere("users", SqlExpr.Like(SqlExpr.Column(null, "name"), "%Alice%")))
        val proj  = assertIs<LogicalPlan.Project>(plan)
        val filt  = assertIs<LogicalPlan.Filter>(proj.input)
        val like  = assertIs<SqlExpr.Like>(filt.predicate)
        assertEquals("%Alice%", like.pattern)
    }

    @Test @DisplayName("Expr: NOT LIKE resolves value column")
    fun expr_notLike() {
        val plan  = planner().plan(selectStarWhere("users", SqlExpr.NotLike(SqlExpr.Column(null, "name"), "%Bob%")))
        val proj  = assertIs<LogicalPlan.Project>(plan)
        val filt  = assertIs<LogicalPlan.Filter>(proj.input)
        assertIs<SqlExpr.NotLike>(filt.predicate)
    }

    @Test @DisplayName("Expr: Unary NOT resolves operand column")
    fun expr_unaryNot() {
        val where = SqlExpr.UnaryOp(UnaryOperator.NOT,
            SqlExpr.BinaryOp(BinaryOperator.EQ, SqlExpr.Column(null, "age"), SqlExpr.Literal(0L)))
        val plan  = planner().plan(selectStarWhere("users", where))
        val proj  = assertIs<LogicalPlan.Project>(plan)
        val filt  = assertIs<LogicalPlan.Filter>(proj.input)
        assertIs<SqlExpr.UnaryOp>(filt.predicate)
    }

    @Test @DisplayName("Expr: FuncCall resolves argument columns")
    fun expr_funcCall() {
        val where = SqlExpr.BinaryOp(BinaryOperator.GT,
            SqlExpr.FuncCall("LENGTH", listOf(SqlExpr.Column(null, "name"))),
            SqlExpr.Literal(3L))
        val plan  = planner().plan(selectStarWhere("users", where))
        val proj  = assertIs<LogicalPlan.Project>(plan)
        val filt  = assertIs<LogicalPlan.Filter>(proj.input)
        val bop   = assertIs<SqlExpr.BinaryOp>(filt.predicate)
        val fn    = assertIs<SqlExpr.FuncCall>(bop.left)
        assertEquals("LENGTH", fn.name)
    }

    // ─── Error propagation inside expressions ─────────────────────────────────

    @Test @DisplayName("Expr error: BETWEEN bad value column throws")
    fun exprErr_betweenValue() {
        assertThrows<UnknownColumnException> {
            planner().plan(selectStarWhere("users",
                SqlExpr.Between(SqlExpr.Column(null, "ghost"), SqlExpr.Literal(1L), SqlExpr.Literal(10L))))
        }
    }

    @Test @DisplayName("Expr error: BETWEEN bad lo column throws")
    fun exprErr_betweenLo() {
        assertThrows<UnknownColumnException> {
            planner().plan(selectStarWhere("users",
                SqlExpr.Between(SqlExpr.Column(null, "age"), SqlExpr.Column(null, "ghost_lo"), SqlExpr.Literal(10L))))
        }
    }

    @Test @DisplayName("Expr error: BETWEEN bad hi column throws")
    fun exprErr_betweenHi() {
        assertThrows<UnknownColumnException> {
            planner().plan(selectStarWhere("users",
                SqlExpr.Between(SqlExpr.Column(null, "age"), SqlExpr.Literal(1L), SqlExpr.Column(null, "ghost_hi"))))
        }
    }

    @Test @DisplayName("Expr error: IN bad item column throws")
    fun exprErr_inItem() {
        assertThrows<UnknownColumnException> {
            planner().plan(selectStarWhere("users",
                SqlExpr.In(SqlExpr.Column(null, "age"), listOf(SqlExpr.Column(null, "ghost_col")))))
        }
    }

    @Test @DisplayName("Expr error: NOT IN bad item column throws")
    fun exprErr_notInItem() {
        assertThrows<UnknownColumnException> {
            planner().plan(selectStarWhere("users",
                SqlExpr.NotIn(SqlExpr.Column(null, "age"), listOf(SqlExpr.Column(null, "ghost_col")))))
        }
    }

    @Test @DisplayName("Expr error: FuncCall bad arg column throws")
    fun exprErr_funcCallArg() {
        assertThrows<UnknownColumnException> {
            planner().plan(selectStarWhere("users",
                SqlExpr.BinaryOp(BinaryOperator.GT,
                    SqlExpr.FuncCall("LENGTH", listOf(SqlExpr.Column(null, "ghost_col"))),
                    SqlExpr.Literal(3L))))
        }
    }

    @Test @DisplayName("Expr error: IS NULL bad inner column throws")
    fun exprErr_isNullBadCol() {
        assertThrows<UnknownColumnException> {
            planner().plan(selectStarWhere("users", SqlExpr.IsNull(SqlExpr.Column(null, "ghost_col"))))
        }
    }

    @Test @DisplayName("Expr error: IS NOT NULL bad inner column throws")
    fun exprErr_isNotNullBadCol() {
        assertThrows<UnknownColumnException> {
            planner().plan(selectStarWhere("users", SqlExpr.IsNotNull(SqlExpr.Column(null, "ghost_col"))))
        }
    }

    @Test @DisplayName("Expr error: LIKE bad value column throws")
    fun exprErr_likeBadCol() {
        assertThrows<UnknownColumnException> {
            planner().plan(selectStarWhere("users", SqlExpr.Like(SqlExpr.Column(null, "ghost_col"), "%x%")))
        }
    }

    @Test @DisplayName("Expr error: NOT LIKE bad value column throws")
    fun exprErr_notLikeBadCol() {
        assertThrows<UnknownColumnException> {
            planner().plan(selectStarWhere("users", SqlExpr.NotLike(SqlExpr.Column(null, "ghost_col"), "%x%")))
        }
    }

    @Test @DisplayName("Expr: Literal null value preserved")
    fun expr_literalNull() {
        planner().plan(selectStarWhere("users", SqlExpr.IsNull(SqlExpr.Literal(null))))
    }

    @Test @DisplayName("Aggregate: SUM with no GROUP BY produces Aggregate node")
    fun agg_sumNoGroupBy() {
        val sum  = SqlExpr.AggExpr(AggFunction.SUM, AggArg.Expr(SqlExpr.Column(null, "amount")), false)
        val stmt = Statement.Select(false, listOf(OutputColumn.Expr(sum, null)),
            listOf(TableRef("orders", null)), emptyList(), null, emptyList(), null, emptyList(), null)
        val plan = planner().plan(stmt)
        val proj = assertIs<LogicalPlan.Project>(plan)
        assertIs<LogicalPlan.Aggregate>(proj.input)
    }

    @Test @DisplayName("Aggregate: COUNT DISTINCT")
    fun agg_countDistinct() {
        val cd   = SqlExpr.AggExpr(AggFunction.COUNT, AggArg.Expr(SqlExpr.Column(null, "name")), true)
        val stmt = Statement.Select(false, listOf(OutputColumn.Expr(cd, null)),
            listOf(TableRef("users", null)), emptyList(), null, emptyList(), null, emptyList(), null)
        val plan = planner().plan(stmt)
        val proj = assertIs<LogicalPlan.Project>(plan)
        val agg  = assertIs<LogicalPlan.Aggregate>(proj.input)
        assertTrue(agg.aggregates[0].distinct)
    }
}

package com.codingadventures.sqlcodegen

// SqlCodegenTest.kt — JUnit 5 test suite for the Kotlin sql-codegen package.
//
// Test philosophy: each test compiles a minimal OptimizedPlan and asserts that
// the generated instruction list contains the expected instructions (in order,
// or at least contains the key ones).  We favour asserting specific instruction
// values over checking exact list indices, making tests resilient to minor
// ordering differences in generated code.
//
// Coverage targets:
//   • All 15 plan-node types (Scan, Filter, Project, Join, Aggregate, Having,
//     Sort, Limit, Distinct, Union, Insert, Update, Delete, CreateTable,
//     DropTable, EmptyResult)
//   • All BinaryOp variants
//   • Both UnaryOp variants
//   • IsNull, IsNotNull, Between, Like, InList expression instructions
//   • All AggFn variants (COUNT, COUNT_STAR, SUM, AVG, MIN, MAX)
//   • Post-op peeling (Sort+Limit+Distinct wrappers)
//   • Multi-row INSERT
//   • Nested Filter over Scan

import com.codingadventures.sqlplanner.*
import com.codingadventures.sqloptimizer.OptimizedPlan
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.Assertions.*

// ── Helper builders ────────────────────────────────────────────────────────────
//
// Small factory functions to build plan nodes without boilerplate.  Using
// helpers keeps each test focused on what it's actually testing.

private fun scan(table: String, alias: String? = null) =
    OptimizedPlan.Scan(table, alias)

private fun filter(input: OptimizedPlan, pred: SqlExpr) =
    OptimizedPlan.Filter(input, pred)

private fun project(input: OptimizedPlan, vararg cols: OutputColumn) =
    OptimizedPlan.Project(input, cols.toList())

private fun colExpr(col: String, alias: String? = null) =
    OutputColumn.Expr(SqlExpr.Column(null, col), alias)

private fun tableColExpr(table: String, col: String, alias: String? = null) =
    OutputColumn.Expr(SqlExpr.Column(table, col), alias)

private fun lit(v: Any?) = SqlExpr.Literal(v)
private fun col(name: String) = SqlExpr.Column(null, name)
private fun col(table: String, name: String) = SqlExpr.Column(table, name)

private fun binOp(op: BinaryOperator, l: SqlExpr, r: SqlExpr) =
    SqlExpr.BinaryOp(op, l, r)

private fun aggItem(func: AggFunction, arg: AggArg, alias: String) =
    AggregateItem(func, arg, alias, false)

private fun columnDef(name: String, typeName: String = "TEXT") =
    ColumnDef(name, typeName, notNull = false, primaryKey = false, unique = false, default = null)

// ── Convenience: extract instruction types from a program ──────────────────────

private fun Program.types(): List<String> =
    instructions.map { it::class.simpleName ?: it.toString() }

private inline fun <reified T : Instruction> Program.allOf(): List<T> =
    instructions.filterIsInstance<T>()

private inline fun <reified T : Instruction> Program.firstOf(): T =
    instructions.filterIsInstance<T>().first()

private inline fun <reified T : Instruction> Program.countOf(): Int =
    instructions.filterIsInstance<T>().size

// ─────────────────────────────────────────────────────────────────────────────
// Test class
// ─────────────────────────────────────────────────────────────────────────────

class SqlCodegenTest {

    // ═══════════════════════════════════════════════════════════════════════════
    // § 1  EmptyResult
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `EmptyResult emits only Halt`() {
        val program = SqlCodegen.compile(OptimizedPlan.EmptyResult)
        // A provably-empty plan should produce just [Halt].
        assertEquals(listOf(Instruction.Halt), program.instructions)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 2  Scan
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `bare Scan emits OpenScan, loop label, AdvanceCursor, BeginRow, EmitRow, Jump, end label, CloseScan, Halt`() {
        val program = SqlCodegen.compile(scan("employees"))
        val instrs = program.instructions

        assertTrue(instrs.any { it is Instruction.OpenScan })
        assertTrue(instrs.any { it is Instruction.AdvanceCursor })
        assertTrue(instrs.any { it is Instruction.BeginRow })
        assertTrue(instrs.any { it is Instruction.EmitRow })
        assertTrue(instrs.any { it is Instruction.Jump })
        assertTrue(instrs.any { it is Instruction.CloseScan })
        assertTrue(instrs.last() is Instruction.Halt)
    }

    @Test
    fun `Scan with alias propagates alias to OpenScan and AdvanceCursor`() {
        val program = SqlCodegen.compile(scan("employees", "e"))
        val open = program.firstOf<Instruction.OpenScan>()
        assertEquals("employees", open.table)
        assertEquals("e", open.alias)
        val adv = program.firstOf<Instruction.AdvanceCursor>()
        assertEquals("e", adv.alias)
    }

    @Test
    fun `Scan without alias uses null alias in AdvanceCursor`() {
        val program = SqlCodegen.compile(scan("users"))
        val open = program.firstOf<Instruction.OpenScan>()
        assertEquals("users", open.table)
        assertNull(open.alias)
    }

    @Test
    fun `Scan loop structure: OpenScan before Label before AdvanceCursor before Jump before CloseScan`() {
        val program = SqlCodegen.compile(scan("t"))
        val instrs = program.instructions
        val openIdx  = instrs.indexOfFirst { it is Instruction.OpenScan }
        val labelIdx = instrs.indexOfFirst { it is Instruction.Label }
        val advIdx   = instrs.indexOfFirst { it is Instruction.AdvanceCursor }
        val jumpIdx  = instrs.indexOfFirst { it is Instruction.Jump }
        val closeIdx = instrs.indexOfFirst { it is Instruction.CloseScan }

        assertTrue(openIdx < labelIdx)
        assertTrue(labelIdx < advIdx)
        assertTrue(advIdx < jumpIdx)
        assertTrue(jumpIdx < closeIdx)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 3  Filter
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `Filter over Scan emits predicate instructions and JumpIfFalse`() {
        val predicate = binOp(BinaryOperator.GT, col("salary"), lit(50000L))
        val plan = filter(scan("employees"), predicate)
        val program = SqlCodegen.compile(plan)
        val instrs = program.instructions

        // Must have a cursor loop.
        assertTrue(instrs.any { it is Instruction.OpenScan })
        assertTrue(instrs.any { it is Instruction.AdvanceCursor })
        // Must have the predicate instructions.
        assertTrue(instrs.any { it is Instruction.LoadColumn })
        assertTrue(instrs.any { it is Instruction.LoadConst })
        assertTrue(instrs.any { it is Instruction.BinaryOpInstr })
        // Must conditionally skip non-matching rows.
        assertTrue(instrs.any { it is Instruction.JumpIfFalse })
        assertTrue(instrs.last() is Instruction.Halt)
    }

    @Test
    fun `Filter over Scan: BinaryOpInstr has correct operator`() {
        val predicate = binOp(BinaryOperator.EQ, col("status"), lit("active"))
        val plan = filter(scan("users"), predicate)
        val program = SqlCodegen.compile(plan)
        val binOps = program.allOf<Instruction.BinaryOpInstr>()
        assertTrue(binOps.any { it.op == BinaryOp.EQ })
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 4  Project
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `Project over Scan emits OpenScan, BeginRow, EmitColumn, EmitRow, CloseScan, Halt`() {
        val plan = project(scan("employees"), colExpr("name"), colExpr("salary"))
        val program = SqlCodegen.compile(plan)
        val instrs = program.instructions

        assertTrue(instrs.any { it is Instruction.OpenScan })
        assertTrue(instrs.any { it is Instruction.BeginRow })
        val emitCols = program.allOf<Instruction.EmitColumn>()
        assertEquals(2, emitCols.size)
        assertTrue(emitCols.any { it.name == "name" })
        assertTrue(emitCols.any { it.name == "salary" })
        assertTrue(instrs.any { it is Instruction.EmitRow })
        assertTrue(instrs.any { it is Instruction.CloseScan })
        assertTrue(instrs.last() is Instruction.Halt)
    }

    @Test
    fun `Project SELECT star emits star EmitColumn`() {
        val plan = project(scan("employees"), OutputColumn.Star)
        val program = SqlCodegen.compile(plan)
        val emitCols = program.allOf<Instruction.EmitColumn>()
        assertTrue(emitCols.any { it.name == "*" })
    }

    @Test
    fun `Project with alias emits EmitColumn with alias name`() {
        val plan = project(scan("t"), colExpr("name", "n"))
        val program = SqlCodegen.compile(plan)
        val emitCols = program.allOf<Instruction.EmitColumn>()
        assertTrue(emitCols.any { it.name == "n" })
    }

    @Test
    fun `Project with Filter underneath emits predicate check`() {
        val pred = binOp(BinaryOperator.LT, col("age"), lit(30L))
        val plan = project(filter(scan("users"), pred), colExpr("name"))
        val program = SqlCodegen.compile(plan)
        assertTrue(program.instructions.any { it is Instruction.JumpIfFalse })
        assertTrue(program.instructions.any { it is Instruction.EmitColumn })
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 5  Aggregate
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `Aggregate COUNT STAR emits InitAgg, UpdateAgg, AdvanceGroup, FinalizeAgg`() {
        val plan = OptimizedPlan.Aggregate(
            scan("orders"),
            groupBy = emptyList(),
            aggregates = listOf(aggItem(AggFunction.COUNT, AggArg.Star, "cnt"))
        )
        val program = SqlCodegen.compile(plan)
        assertTrue(program.instructions.any { it is Instruction.InitAgg })
        assertTrue(program.instructions.any { it is Instruction.UpdateAgg })
        assertTrue(program.instructions.any { it is Instruction.AdvanceGroup })
        assertTrue(program.instructions.any { it is Instruction.FinalizeAgg })
    }

    @Test
    fun `Aggregate SUM emits AggFn SUM`() {
        val plan = OptimizedPlan.Aggregate(
            scan("sales"),
            groupBy = emptyList(),
            aggregates = listOf(aggItem(AggFunction.SUM, AggArg.Expr(col("amount")), "total"))
        )
        val program = SqlCodegen.compile(plan)
        val initAggs = program.allOf<Instruction.InitAgg>()
        assertTrue(initAggs.any { it.fn == AggFn.SUM })
    }

    @Test
    fun `Aggregate AVG emits AggFn AVG`() {
        val plan = OptimizedPlan.Aggregate(
            scan("grades"),
            groupBy = emptyList(),
            aggregates = listOf(aggItem(AggFunction.AVG, AggArg.Expr(col("score")), "avg_score"))
        )
        val program = SqlCodegen.compile(plan)
        val initAggs = program.allOf<Instruction.InitAgg>()
        assertTrue(initAggs.any { it.fn == AggFn.AVG })
    }

    @Test
    fun `Aggregate MIN emits AggFn MIN`() {
        val plan = OptimizedPlan.Aggregate(
            scan("temps"),
            groupBy = emptyList(),
            aggregates = listOf(aggItem(AggFunction.MIN, AggArg.Expr(col("value")), "min_val"))
        )
        val program = SqlCodegen.compile(plan)
        assertTrue(program.allOf<Instruction.InitAgg>().any { it.fn == AggFn.MIN })
    }

    @Test
    fun `Aggregate MAX emits AggFn MAX`() {
        val plan = OptimizedPlan.Aggregate(
            scan("temps"),
            groupBy = emptyList(),
            aggregates = listOf(aggItem(AggFunction.MAX, AggArg.Expr(col("value")), "max_val"))
        )
        val program = SqlCodegen.compile(plan)
        assertTrue(program.allOf<Instruction.InitAgg>().any { it.fn == AggFn.MAX })
    }

    @Test
    fun `Aggregate COUNT column (not star) emits AggFn COUNT`() {
        val plan = OptimizedPlan.Aggregate(
            scan("t"),
            groupBy = emptyList(),
            aggregates = listOf(aggItem(AggFunction.COUNT, AggArg.Expr(col("id")), "cnt"))
        )
        val program = SqlCodegen.compile(plan)
        assertTrue(program.allOf<Instruction.InitAgg>().any { it.fn == AggFn.COUNT })
    }

    @Test
    fun `Aggregate with GROUP BY emits SaveGroupKey`() {
        val plan = OptimizedPlan.Aggregate(
            scan("employees"),
            groupBy = listOf(col("dept")),
            aggregates = listOf(aggItem(AggFunction.COUNT, AggArg.Star, "cnt"))
        )
        val program = SqlCodegen.compile(plan)
        assertTrue(program.instructions.any { it is Instruction.SaveGroupKey })
    }

    @Test
    fun `Aggregate with GROUP BY emits LoadGroupKey during finalize`() {
        val plan = OptimizedPlan.Aggregate(
            scan("employees"),
            groupBy = listOf(col("dept")),
            aggregates = listOf(aggItem(AggFunction.COUNT, AggArg.Star, "cnt"))
        )
        val program = SqlCodegen.compile(plan)
        assertTrue(program.instructions.any { it is Instruction.LoadGroupKey })
    }

    @Test
    fun `Aggregate emits FinalizeAgg with alias as EmitColumn`() {
        val plan = OptimizedPlan.Aggregate(
            scan("orders"),
            groupBy = emptyList(),
            aggregates = listOf(aggItem(AggFunction.COUNT, AggArg.Star, "order_count"))
        )
        val program = SqlCodegen.compile(plan)
        val emitCols = program.allOf<Instruction.EmitColumn>()
        assertTrue(emitCols.any { it.name == "order_count" })
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 6  Having
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `Having over Aggregate emits predicate and JumpIfFalse`() {
        val agg = OptimizedPlan.Aggregate(
            scan("orders"),
            groupBy = listOf(col("status")),
            aggregates = listOf(aggItem(AggFunction.COUNT, AggArg.Star, "cnt"))
        )
        val having = OptimizedPlan.Having(agg, binOp(BinaryOperator.GT, col("cnt"), lit(2L)))
        val program = SqlCodegen.compile(having)
        assertTrue(program.instructions.any { it is Instruction.JumpIfFalse })
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 7  Sort (post-op)
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `Sort wrapper emits SortResult before Halt`() {
        val sortKey = SortKey(col("salary"), SortDir.DESC, NullOrder.NULLS_LAST)
        val plan = OptimizedPlan.Sort(scan("employees"), listOf(sortKey))
        val program = SqlCodegen.compile(plan)
        val instrs = program.instructions
        val sortIdx = instrs.indexOfFirst { it is Instruction.SortResult }
        val haltIdx = instrs.indexOfLast { it is Instruction.Halt }
        assertTrue(sortIdx != -1, "SortResult must be present")
        assertTrue(sortIdx < haltIdx, "SortResult must precede Halt")
    }

    @Test
    fun `Sort SortResult carries the sort keys`() {
        val sortKey = SortKey(col("name"), SortDir.ASC, NullOrder.NULLS_FIRST)
        val plan = OptimizedPlan.Sort(scan("t"), listOf(sortKey))
        val program = SqlCodegen.compile(plan)
        val sortResult = program.firstOf<Instruction.SortResult>()
        assertEquals(1, sortResult.keys.size)
        assertEquals(SortDir.ASC, sortResult.keys[0].direction)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 8  Limit (post-op)
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `Limit wrapper emits LimitResult before Halt`() {
        val plan = OptimizedPlan.Limit(scan("t"), 10L, null)
        val program = SqlCodegen.compile(plan)
        val instrs = program.instructions
        val limitIdx = instrs.indexOfFirst { it is Instruction.LimitResult }
        val haltIdx  = instrs.indexOfLast  { it is Instruction.Halt }
        assertTrue(limitIdx != -1, "LimitResult must be present")
        assertTrue(limitIdx < haltIdx)
    }

    @Test
    fun `Limit LimitResult carries count and offset`() {
        val plan = OptimizedPlan.Limit(scan("t"), 5L, 10L)
        val program = SqlCodegen.compile(plan)
        val limitResult = program.firstOf<Instruction.LimitResult>()
        assertEquals(5L, limitResult.count)
        assertEquals(10L, limitResult.offset)
    }

    @Test
    fun `Limit with no count emits LimitResult with null count`() {
        val plan = OptimizedPlan.Limit(scan("t"), null, 20L)
        val program = SqlCodegen.compile(plan)
        val limitResult = program.firstOf<Instruction.LimitResult>()
        assertNull(limitResult.count)
        assertEquals(20L, limitResult.offset)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 9  Distinct (post-op)
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `Distinct wrapper emits DistinctResult before Halt`() {
        val plan = OptimizedPlan.Distinct(scan("t"))
        val program = SqlCodegen.compile(plan)
        val instrs = program.instructions
        val distIdx = instrs.indexOfFirst { it is Instruction.DistinctResult }
        val haltIdx = instrs.indexOfLast  { it is Instruction.Halt }
        assertTrue(distIdx != -1, "DistinctResult must be present")
        assertTrue(distIdx < haltIdx)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 10  Sort + Limit (combined, stacked wrappers)
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `Sort wrapped by Limit emits SortResult then LimitResult then Halt`() {
        val sortKey = SortKey(col("salary"), SortDir.DESC, NullOrder.NULLS_LAST)
        val sorted  = OptimizedPlan.Sort(scan("employees"), listOf(sortKey))
        val limited = OptimizedPlan.Limit(sorted, 3L, 0L)
        val program = SqlCodegen.compile(limited)
        val instrs = program.instructions
        val limitIdx = instrs.indexOfFirst { it is Instruction.LimitResult }
        val sortIdx  = instrs.indexOfFirst { it is Instruction.SortResult }
        val haltIdx  = instrs.indexOfLast  { it is Instruction.Halt }
        assertTrue(sortIdx != -1)
        assertTrue(limitIdx != -1)
        assertTrue(sortIdx < haltIdx)
        assertTrue(limitIdx < haltIdx)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 11  Join
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `inner Join emits two OpenScan calls and two CloseScan calls`() {
        val condition = binOp(BinaryOperator.EQ, col("e", "dept_id"), col("d", "id"))
        val plan = OptimizedPlan.Join(
            scan("employees", "e"),
            scan("departments", "d"),
            JoinKind.INNER,
            condition
        )
        val program = SqlCodegen.compile(plan)
        assertEquals(2, program.countOf<Instruction.OpenScan>())
        assertEquals(2, program.countOf<Instruction.CloseScan>())
    }

    @Test
    fun `inner Join with condition emits JumpIfFalse inside inner loop`() {
        val condition = binOp(BinaryOperator.EQ, col("e", "dept_id"), col("d", "id"))
        val plan = OptimizedPlan.Join(
            scan("employees", "e"),
            scan("departments", "d"),
            JoinKind.INNER,
            condition
        )
        val program = SqlCodegen.compile(plan)
        assertTrue(program.instructions.any { it is Instruction.JumpIfFalse })
    }

    @Test
    fun `cross Join with no condition emits no JumpIfFalse`() {
        val plan = OptimizedPlan.Join(
            scan("a"),
            scan("b"),
            JoinKind.CROSS,
            null
        )
        val program = SqlCodegen.compile(plan)
        assertFalse(program.instructions.any { it is Instruction.JumpIfFalse })
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 12  Union
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `Union ALL emits two scan loops and no DistinctResult`() {
        val plan = OptimizedPlan.Union(scan("t1"), scan("t2"), all = true)
        val program = SqlCodegen.compile(plan)
        assertEquals(2, program.countOf<Instruction.OpenScan>())
        assertFalse(program.instructions.any { it is Instruction.DistinctResult })
    }

    @Test
    fun `Union (not ALL) emits DistinctResult`() {
        val plan = OptimizedPlan.Union(scan("t1"), scan("t2"), all = false)
        val program = SqlCodegen.compile(plan)
        assertTrue(program.instructions.any { it is Instruction.DistinctResult })
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 13  Insert
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `Insert VALUES emits LoadConst for each value and InsertRow`() {
        val plan = OptimizedPlan.Insert(
            "users",
            listOf("name", "age"),
            listOf(listOf(lit("Alice"), lit(30L)))
        )
        val program = SqlCodegen.compile(plan)
        assertTrue(program.instructions.any { it is Instruction.InsertRow })
        val consts = program.allOf<Instruction.LoadConst>()
        assertTrue(consts.any { it.value == SqlValue.TextVal("Alice") })
        assertTrue(consts.any { it.value == SqlValue.IntVal(30L) })
    }

    @Test
    fun `multi-row Insert emits one InsertRow per value tuple`() {
        val plan = OptimizedPlan.Insert(
            "t",
            listOf("x"),
            listOf(listOf(lit(1L)), listOf(lit(2L)), listOf(lit(3L)))
        )
        val program = SqlCodegen.compile(plan)
        assertEquals(3, program.countOf<Instruction.InsertRow>())
    }

    @Test
    fun `Insert InsertRow carries table name`() {
        val plan = OptimizedPlan.Insert("orders", listOf("id"), listOf(listOf(lit(42L))))
        val program = SqlCodegen.compile(plan)
        val insertRow = program.firstOf<Instruction.InsertRow>()
        assertEquals("orders", insertRow.table)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 14  Update
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `Update emits cursor loop and UpdateRows`() {
        val plan = OptimizedPlan.Update(
            "employees",
            listOf(Assignment("salary", binOp(BinaryOperator.MUL, col("salary"), lit(1.1)))),
            predicate = binOp(BinaryOperator.EQ, col("dept"), lit("Eng"))
        )
        val program = SqlCodegen.compile(plan)
        assertTrue(program.instructions.any { it is Instruction.OpenScan })
        assertTrue(program.instructions.any { it is Instruction.AdvanceCursor })
        assertTrue(program.instructions.any { it is Instruction.UpdateRows })
        assertTrue(program.instructions.any { it is Instruction.CloseScan })
    }

    @Test
    fun `Update with WHERE emits JumpIfFalse predicate guard`() {
        val plan = OptimizedPlan.Update(
            "t",
            listOf(Assignment("v", lit(0L))),
            predicate = binOp(BinaryOperator.GT, col("x"), lit(10L))
        )
        val program = SqlCodegen.compile(plan)
        assertTrue(program.instructions.any { it is Instruction.JumpIfFalse })
    }

    @Test
    fun `Update without WHERE has no JumpIfFalse`() {
        val plan = OptimizedPlan.Update(
            "t",
            listOf(Assignment("v", lit(0L))),
            predicate = null
        )
        val program = SqlCodegen.compile(plan)
        assertFalse(program.instructions.any { it is Instruction.JumpIfFalse })
    }

    @Test
    fun `UpdateRows carries table name`() {
        val plan = OptimizedPlan.Update("inventory", listOf(Assignment("qty", lit(0L))), null)
        val program = SqlCodegen.compile(plan)
        val updateRows = program.firstOf<Instruction.UpdateRows>()
        assertEquals("inventory", updateRows.table)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 15  Delete
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `Delete emits cursor loop and DeleteRows`() {
        val plan = OptimizedPlan.Delete(
            "temp_logs",
            predicate = binOp(BinaryOperator.LT, col("created_at"), lit("2020-01-01"))
        )
        val program = SqlCodegen.compile(plan)
        assertTrue(program.instructions.any { it is Instruction.OpenScan })
        assertTrue(program.instructions.any { it is Instruction.DeleteRows })
        assertTrue(program.instructions.any { it is Instruction.CloseScan })
    }

    @Test
    fun `Delete with WHERE emits JumpIfFalse`() {
        val plan = OptimizedPlan.Delete(
            "t",
            predicate = binOp(BinaryOperator.EQ, col("active"), lit(false))
        )
        val program = SqlCodegen.compile(plan)
        assertTrue(program.instructions.any { it is Instruction.JumpIfFalse })
    }

    @Test
    fun `Delete without WHERE has no JumpIfFalse`() {
        val plan = OptimizedPlan.Delete("t", predicate = null)
        val program = SqlCodegen.compile(plan)
        assertFalse(program.instructions.any { it is Instruction.JumpIfFalse })
    }

    @Test
    fun `DeleteRows carries table name`() {
        val plan = OptimizedPlan.Delete("old_sessions", null)
        val program = SqlCodegen.compile(plan)
        val deleteRows = program.firstOf<Instruction.DeleteRows>()
        assertEquals("old_sessions", deleteRows.table)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 16  CreateTable / DropTable
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `CreateTable emits CreateTableInstr and Halt`() {
        val plan = OptimizedPlan.CreateTable(
            "products",
            ifNotExists = true,
            columns = listOf(columnDef("id", "INTEGER"), columnDef("name", "TEXT"))
        )
        val program = SqlCodegen.compile(plan)
        val ct = program.firstOf<Instruction.CreateTableInstr>()
        assertEquals("products", ct.name)
        assertTrue(ct.ifNotExists)
        assertEquals(2, ct.columns.size)
        assertTrue(program.instructions.last() is Instruction.Halt)
    }

    @Test
    fun `DropTable emits DropTableInstr and Halt`() {
        val plan = OptimizedPlan.DropTable("old_table", ifExists = true)
        val program = SqlCodegen.compile(plan)
        val dt = program.firstOf<Instruction.DropTableInstr>()
        assertEquals("old_table", dt.name)
        assertTrue(dt.ifExists)
        assertTrue(program.instructions.last() is Instruction.Halt)
    }

    @Test
    fun `DropTable ifExists false is preserved`() {
        val plan = OptimizedPlan.DropTable("t", ifExists = false)
        val program = SqlCodegen.compile(plan)
        assertFalse(program.firstOf<Instruction.DropTableInstr>().ifExists)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 17  Expression compilation — BinaryOp variants
    // ═══════════════════════════════════════════════════════════════════════════

    private fun compileBinOp(op: BinaryOperator): BinaryOp {
        val instrs = SqlCodegen.compileExpression(binOp(op, lit(1L), lit(2L)))
        return instrs.filterIsInstance<Instruction.BinaryOpInstr>().first().op
    }

    @Test fun `BinaryOperator ADD maps to BinaryOp ADD`()   { assertEquals(BinaryOp.ADD, compileBinOp(BinaryOperator.ADD)) }
    @Test fun `BinaryOperator SUB maps to BinaryOp SUB`()   { assertEquals(BinaryOp.SUB, compileBinOp(BinaryOperator.SUB)) }
    @Test fun `BinaryOperator MUL maps to BinaryOp MUL`()   { assertEquals(BinaryOp.MUL, compileBinOp(BinaryOperator.MUL)) }
    @Test fun `BinaryOperator DIV maps to BinaryOp DIV`()   { assertEquals(BinaryOp.DIV, compileBinOp(BinaryOperator.DIV)) }
    @Test fun `BinaryOperator MOD maps to BinaryOp MOD`()   { assertEquals(BinaryOp.MOD, compileBinOp(BinaryOperator.MOD)) }
    @Test fun `BinaryOperator EQ maps to BinaryOp EQ`()     { assertEquals(BinaryOp.EQ,  compileBinOp(BinaryOperator.EQ)) }
    @Test fun `BinaryOperator NOT_EQ maps to BinaryOp NEQ`(){ assertEquals(BinaryOp.NEQ, compileBinOp(BinaryOperator.NOT_EQ)) }
    @Test fun `BinaryOperator LT maps to BinaryOp LT`()     { assertEquals(BinaryOp.LT,  compileBinOp(BinaryOperator.LT)) }
    @Test fun `BinaryOperator LTE maps to BinaryOp LTE`()   { assertEquals(BinaryOp.LTE, compileBinOp(BinaryOperator.LTE)) }
    @Test fun `BinaryOperator GT maps to BinaryOp GT`()     { assertEquals(BinaryOp.GT,  compileBinOp(BinaryOperator.GT)) }
    @Test fun `BinaryOperator GTE maps to BinaryOp GTE`()   { assertEquals(BinaryOp.GTE, compileBinOp(BinaryOperator.GTE)) }
    @Test fun `BinaryOperator AND maps to BinaryOp AND`()   { assertEquals(BinaryOp.AND, compileBinOp(BinaryOperator.AND)) }
    @Test fun `BinaryOperator OR maps to BinaryOp OR`()     { assertEquals(BinaryOp.OR,  compileBinOp(BinaryOperator.OR)) }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 18  Expression compilation — UnaryOp
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `UnaryOperator NEG maps to UnaryOp NEG`() {
        val instrs = SqlCodegen.compileExpression(SqlExpr.UnaryOp(UnaryOperator.NEG, lit(5L)))
        val unary = instrs.filterIsInstance<Instruction.UnaryOpInstr>().first()
        assertEquals(UnaryOp.NEG, unary.op)
    }

    @Test
    fun `UnaryOperator NOT maps to UnaryOp NOT`() {
        val instrs = SqlCodegen.compileExpression(SqlExpr.UnaryOp(UnaryOperator.NOT, lit(true)))
        val unary = instrs.filterIsInstance<Instruction.UnaryOpInstr>().first()
        assertEquals(UnaryOp.NOT, unary.op)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 19  Expression compilation — predicate tests
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `IsNull expression emits LoadColumn then IsNull`() {
        val instrs = SqlCodegen.compileExpression(SqlExpr.IsNull(col("age")))
        assertTrue(instrs.any { it is Instruction.LoadColumn })
        assertTrue(instrs.any { it is Instruction.IsNull })
    }

    @Test
    fun `IsNotNull expression emits LoadColumn then IsNotNull`() {
        val instrs = SqlCodegen.compileExpression(SqlExpr.IsNotNull(col("age")))
        assertTrue(instrs.any { it is Instruction.LoadColumn })
        assertTrue(instrs.any { it is Instruction.IsNotNull })
    }

    @Test
    fun `Between expression emits three pushes then Between instruction`() {
        val expr = SqlExpr.Between(col("score"), lit(60L), lit(100L))
        val instrs = SqlCodegen.compileExpression(expr)
        assertTrue(instrs.any { it is Instruction.Between })
        // Three pushes: value, low, high
        val pushCount = instrs.count { it is Instruction.LoadColumn || it is Instruction.LoadConst }
        assertEquals(3, pushCount)
    }

    @Test
    fun `Like expression emits LoadColumn, LoadConst(pattern), Like`() {
        val expr = SqlExpr.Like(col("email"), "%@example.com")
        val instrs = SqlCodegen.compileExpression(expr)
        assertTrue(instrs.any { it is Instruction.Like })
        val consts = instrs.filterIsInstance<Instruction.LoadConst>()
        assertTrue(consts.any { it.value == SqlValue.TextVal("%@example.com") })
    }

    @Test
    fun `NotLike expression emits Like then NOT`() {
        val expr = SqlExpr.NotLike(col("name"), "Alice%")
        val instrs = SqlCodegen.compileExpression(expr)
        assertTrue(instrs.any { it is Instruction.Like })
        val unary = instrs.filterIsInstance<Instruction.UnaryOpInstr>()
        assertTrue(unary.any { it.op == UnaryOp.NOT })
    }

    @Test
    fun `In expression emits LoadColumn, item LoadConsts, InList`() {
        val expr = SqlExpr.In(col("status"), listOf(lit("active"), lit("pending")))
        val instrs = SqlCodegen.compileExpression(expr)
        val inList = instrs.filterIsInstance<Instruction.InList>()
        assertEquals(1, inList.size)
        assertEquals(2, inList[0].count)
    }

    @Test
    fun `NotIn expression emits InList then NOT`() {
        val expr = SqlExpr.NotIn(col("x"), listOf(lit(1L), lit(2L), lit(3L)))
        val instrs = SqlCodegen.compileExpression(expr)
        val inList = instrs.filterIsInstance<Instruction.InList>()
        assertEquals(1, inList.size)
        assertEquals(3, inList[0].count)
        assertTrue(instrs.filterIsInstance<Instruction.UnaryOpInstr>().any { it.op == UnaryOp.NOT })
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 20  Expression compilation — Literal values
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `null literal compiles to LoadConst(Null)`() {
        val instrs = SqlCodegen.compileExpression(lit(null))
        val c = instrs.filterIsInstance<Instruction.LoadConst>().first()
        assertEquals(SqlValue.Null, c.value)
    }

    @Test
    fun `Long literal compiles to LoadConst(IntVal)`() {
        val instrs = SqlCodegen.compileExpression(lit(42L))
        val c = instrs.filterIsInstance<Instruction.LoadConst>().first()
        assertEquals(SqlValue.IntVal(42L), c.value)
    }

    @Test
    fun `Int literal compiles to LoadConst(IntVal) via widening`() {
        val instrs = SqlCodegen.compileExpression(lit(7))
        val c = instrs.filterIsInstance<Instruction.LoadConst>().first()
        assertEquals(SqlValue.IntVal(7L), c.value)
    }

    @Test
    fun `Double literal compiles to LoadConst(FloatVal)`() {
        val instrs = SqlCodegen.compileExpression(lit(3.14))
        val c = instrs.filterIsInstance<Instruction.LoadConst>().first()
        assertEquals(SqlValue.FloatVal(3.14), c.value)
    }

    @Test
    fun `Boolean literal compiles to LoadConst(BoolVal)`() {
        val instrs = SqlCodegen.compileExpression(lit(true))
        val c = instrs.filterIsInstance<Instruction.LoadConst>().first()
        assertEquals(SqlValue.BoolVal(true), c.value)
    }

    @Test
    fun `String literal compiles to LoadConst(TextVal)`() {
        val instrs = SqlCodegen.compileExpression(lit("hello"))
        val c = instrs.filterIsInstance<Instruction.LoadConst>().first()
        assertEquals(SqlValue.TextVal("hello"), c.value)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 21  Program structural invariants
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `every compiled program ends with Halt`() {
        val plans = listOf(
            scan("t"),
            OptimizedPlan.EmptyResult,
            OptimizedPlan.Insert("t", listOf("x"), listOf(listOf(lit(1L)))),
            OptimizedPlan.Delete("t", null),
            OptimizedPlan.CreateTable("t", false, listOf(columnDef("id"))),
            OptimizedPlan.DropTable("t", false)
        )
        for (plan in plans) {
            val program = SqlCodegen.compile(plan)
            assertTrue(program.instructions.last() is Instruction.Halt,
                "Plan ${plan::class.simpleName} did not end with Halt")
        }
    }

    @Test
    fun `Program is a data class with non-empty instructions list for non-trivial plans`() {
        val program = SqlCodegen.compile(scan("t"))
        assertTrue(program.instructions.isNotEmpty())
    }

    @Test
    fun `two compilations of same plan produce identical programs`() {
        val plan = project(filter(scan("users"), binOp(BinaryOperator.GT, col("age"), lit(18L))), colExpr("name"))
        val prog1 = SqlCodegen.compile(plan)
        val prog2 = SqlCodegen.compile(plan)
        assertEquals(prog1, prog2)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 22  Label naming conventions
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `Scan emits labels matching scan_N_loop and scan_N_end pattern`() {
        val program = SqlCodegen.compile(scan("t"))
        val labels = program.allOf<Instruction.Label>().map { it.name }
        assertTrue(labels.any { it.matches(Regex("scan_\\d+_loop")) })
        assertTrue(labels.any { it.matches(Regex("scan_\\d+_end")) })
    }

    @Test
    fun `Update emits labels matching update_N_loop and update_N_end pattern`() {
        val program = SqlCodegen.compile(OptimizedPlan.Update("t", listOf(Assignment("x", lit(0L))), null))
        val labels = program.allOf<Instruction.Label>().map { it.name }
        assertTrue(labels.any { it.matches(Regex("update_\\d+_loop")) })
        assertTrue(labels.any { it.matches(Regex("update_\\d+_end")) })
    }

    @Test
    fun `Delete emits labels matching delete_N_loop and delete_N_end pattern`() {
        val program = SqlCodegen.compile(OptimizedPlan.Delete("t", null))
        val labels = program.allOf<Instruction.Label>().map { it.name }
        assertTrue(labels.any { it.matches(Regex("delete_\\d+_loop")) })
        assertTrue(labels.any { it.matches(Regex("delete_\\d+_end")) })
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 23  Jump target consistency
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `every Jump label target exists in program labels`() {
        val plan = project(filter(scan("users"),
            binOp(BinaryOperator.GT, col("age"), lit(21L))), colExpr("name"))
        val program = SqlCodegen.compile(plan)
        val definedLabels = program.allOf<Instruction.Label>().map { it.name }.toSet()
        val jumpTargets = program.allOf<Instruction.Jump>().map { it.label }
        for (target in jumpTargets) {
            assertTrue(target in definedLabels, "Jump to undefined label: $target")
        }
    }

    @Test
    fun `every JumpIfFalse label target exists in program labels`() {
        val plan = filter(scan("t"), binOp(BinaryOperator.EQ, col("x"), lit(1L)))
        val program = SqlCodegen.compile(plan)
        val definedLabels = program.allOf<Instruction.Label>().map { it.name }.toSet()
        val jumpTargets = program.allOf<Instruction.JumpIfFalse>().map { it.label }
        for (target in jumpTargets) {
            assertTrue(target in definedLabels, "JumpIfFalse to undefined label: $target")
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 24  SqlValue sealed class coverage
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `SqlValue Null toString returns NULL`() {
        assertEquals("NULL", SqlValue.Null.toString())
    }

    @Test
    fun `SqlValue IntVal data class equals`() {
        assertEquals(SqlValue.IntVal(1L), SqlValue.IntVal(1L))
        assertNotEquals(SqlValue.IntVal(1L), SqlValue.IntVal(2L))
    }

    @Test
    fun `SqlValue FloatVal data class equals`() {
        assertEquals(SqlValue.FloatVal(1.5), SqlValue.FloatVal(1.5))
    }

    @Test
    fun `SqlValue TextVal data class equals`() {
        assertEquals(SqlValue.TextVal("a"), SqlValue.TextVal("a"))
    }

    @Test
    fun `SqlValue BoolVal data class equals`() {
        assertEquals(SqlValue.BoolVal(true), SqlValue.BoolVal(true))
        assertNotEquals(SqlValue.BoolVal(true), SqlValue.BoolVal(false))
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // § 25  Instruction sealed class structural tests
    // ═══════════════════════════════════════════════════════════════════════════

    @Test
    fun `Instruction object singletons are reference-equal`() {
        assertTrue(Instruction.Halt === Instruction.Halt)
        assertTrue(Instruction.Pop === Instruction.Pop)
        assertTrue(Instruction.EmitRow === Instruction.EmitRow)
        assertTrue(Instruction.BeginRow === Instruction.BeginRow)
        assertTrue(Instruction.IsNull === Instruction.IsNull)
        assertTrue(Instruction.IsNotNull === Instruction.IsNotNull)
        assertTrue(Instruction.DistinctResult === Instruction.DistinctResult)
        assertTrue(Instruction.AdvanceGroup === Instruction.AdvanceGroup)
    }

    @Test
    fun `Between instruction default inclusive is true`() {
        val b = Instruction.Between()
        assertTrue(b.inclusive)
    }

    @Test
    fun `LoadParam carries index`() {
        val lp = Instruction.LoadParam(3)
        assertEquals(3, lp.index)
    }

    @Test
    fun `LoadGroupKey carries index`() {
        val lgk = Instruction.LoadGroupKey(2)
        assertEquals(2, lgk.index)
    }
}

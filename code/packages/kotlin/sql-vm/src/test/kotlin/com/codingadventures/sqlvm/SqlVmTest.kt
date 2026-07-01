package com.codingadventures.sqlvm

// SqlVmTest.kt — Comprehensive tests for the Kotlin sql-vm package.
//
// Test philosophy
// ───────────────
// Rather than mocking the codegen, we build Program objects by hand — exactly
// the way the codegen would.  This keeps tests fast and verifiable by inspection:
// you can read the instruction sequence and reason about what the VM should do.
//
// Organisation
// ────────────
// Tests are grouped by instruction category.  Each group starts with a comment
// explaining what is being tested and why the expected result is what it is.
//
// Coverage goals (lessons.md §>80% test coverage):
//   - All Instruction variants
//   - Three-valued NULL logic (AND / OR / comparisons)
//   - Aggregate functions: COUNT, COUNT(*), SUM, AVG, MIN, MAX
//   - GROUP BY with multiple groups
//   - ORDER BY (ASC / DESC / NULLS FIRST / NULLS LAST)
//   - DISTINCT
//   - LIMIT / OFFSET
//   - LIKE matching
//   - IN list (including NULL semantics)
//   - BETWEEN
//   - DML: INSERT, UPDATE, DELETE
//   - DDL: CREATE TABLE, DROP TABLE
//   - Transactions: BEGIN, COMMIT, ROLLBACK
//   - Stack operations: LoadConst, Pop
//   - BinaryOp all operators
//   - UnaryOp: NEG, NOT
//
// Note: test method names in backticks must NOT contain "--", ":", or other
// JVM-illegal characters (lessons.md §Kotlin-specific notes).

import com.codingadventures.sqlbackend.ColumnDef
import com.codingadventures.sqlbackend.InMemoryBackend
import com.codingadventures.sqlplanner.ColumnDef as PlannerColumnDef
import com.codingadventures.sqlbackend.Row
import com.codingadventures.sqlcodegen.AggFn
import com.codingadventures.sqlcodegen.BinaryOp
import com.codingadventures.sqlcodegen.Instruction
import com.codingadventures.sqlcodegen.Program
import com.codingadventures.sqlcodegen.SqlValue
import com.codingadventures.sqlcodegen.UnaryOp
import com.codingadventures.sqlplanner.NullOrder
import com.codingadventures.sqlplanner.SortDir
import com.codingadventures.sqlplanner.SortKey
import com.codingadventures.sqlplanner.SqlExpr
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

// ─────────────────────────────────────────────────────────────────────────────
// Helper functions
// ─────────────────────────────────────────────────────────────────────────────

/** Build a [Program] from a vararg list of [Instruction]s. */
private fun prog(vararg instrs: Instruction): Program = Program(instrs.toList())

/** Shorthand: integer SQL literal. */
private fun int(v: Long) = SqlValue.IntVal(v)
private fun int(v: Int)  = SqlValue.IntVal(v.toLong())

/** Shorthand: text SQL literal. */
private fun txt(v: String) = SqlValue.TextVal(v)

/** Shorthand: float SQL literal. */
private fun flt(v: Double) = SqlValue.FloatVal(v)

/** Shorthand: boolean SQL literal. */
private fun bool(v: Boolean) = SqlValue.BoolVal(v)

/** SQL NULL literal. */
private val NULL = SqlValue.Null

/** Create a backend pre-populated with a simple "users" table. */
private fun backendWithUsers(vararg rows: Map<String, Any?>): InMemoryBackend {
    val backend = InMemoryBackend()
    backend.createTable(
        "users",
        listOf(
            ColumnDef("id",   "INTEGER"),
            ColumnDef("name", "TEXT"),
            ColumnDef("age",  "INTEGER"),
        ),
        ifNotExists = false,
    )
    for (rowMap in rows) {
        val row = Row()
        rowMap.forEach { (k, v) -> row[k] = v }
        backend.insert("users", row)
    }
    return backend
}

// ─────────────────────────────────────────────────────────────────────────────
// Test class
// ─────────────────────────────────────────────────────────────────────────────

class SqlVmTest {

    private lateinit var backend: InMemoryBackend

    @BeforeEach
    fun setUp() {
        // Fresh backend for each test — no shared mutable state.
        backend = InMemoryBackend()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Basic LoadConst and Halt
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `empty program returns empty result`() {
        val result = SqlVm.execute(prog(Instruction.Halt), backend)
        assertTrue(result.rows.isEmpty())
        assertTrue(result.columns.isEmpty())
        assertEquals(0, result.rowsAffected)
    }

    @Test
    fun `LoadConst integer pushes value onto stack`() {
        // A program that loads a constant and emits it as a single-column row.
        val result = SqlVm.execute(
            prog(
                Instruction.BeginRow,
                Instruction.LoadConst(int(42)),
                Instruction.EmitColumn("val"),
                Instruction.EmitRow,
                Instruction.Halt,
            ),
            backend,
        )
        assertEquals(listOf("val"), result.columns)
        assertEquals(1, result.rows.size)
        assertEquals(int(42), result.rows[0][0])
    }

    @Test
    fun `LoadConst text value`() {
        val result = SqlVm.execute(
            prog(
                Instruction.BeginRow,
                Instruction.LoadConst(txt("hello")),
                Instruction.EmitColumn("greeting"),
                Instruction.EmitRow,
                Instruction.Halt,
            ),
            backend,
        )
        assertEquals(txt("hello"), result.rows[0][0])
    }

    @Test
    fun `LoadConst NULL value`() {
        val result = SqlVm.execute(
            prog(
                Instruction.BeginRow,
                Instruction.LoadConst(NULL),
                Instruction.EmitColumn("n"),
                Instruction.EmitRow,
                Instruction.Halt,
            ),
            backend,
        )
        assertEquals(NULL, result.rows[0][0])
    }

    @Test
    fun `Pop discards top of stack`() {
        // Push two values, pop one, emit the remaining value.
        val result = SqlVm.execute(
            prog(
                Instruction.BeginRow,
                Instruction.LoadConst(int(99)),
                Instruction.LoadConst(int(1)),
                Instruction.Pop,                    // discard 1; 99 is now on top
                Instruction.EmitColumn("v"),
                Instruction.EmitRow,
                Instruction.Halt,
            ),
            backend,
        )
        assertEquals(int(99), result.rows[0][0])
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Arithmetic binary operators
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `BinaryOp ADD integers`() {
        val r = evalExpr(Instruction.LoadConst(int(3)), Instruction.LoadConst(int(4)), Instruction.BinaryOpInstr(BinaryOp.ADD))
        assertEquals(int(7), r)
    }

    @Test
    fun `BinaryOp SUB integers`() {
        val r = evalExpr(Instruction.LoadConst(int(10)), Instruction.LoadConst(int(3)), Instruction.BinaryOpInstr(BinaryOp.SUB))
        assertEquals(int(7), r)
    }

    @Test
    fun `BinaryOp MUL integers`() {
        val r = evalExpr(Instruction.LoadConst(int(6)), Instruction.LoadConst(int(7)), Instruction.BinaryOpInstr(BinaryOp.MUL))
        assertEquals(int(42), r)
    }

    @Test
    fun `BinaryOp DIV integer`() {
        val r = evalExpr(Instruction.LoadConst(int(10)), Instruction.LoadConst(int(3)), Instruction.BinaryOpInstr(BinaryOp.DIV))
        assertEquals(int(3), r)  // integer division truncates
    }

    @Test
    fun `BinaryOp DIV by zero returns NULL`() {
        val r = evalExpr(Instruction.LoadConst(int(5)), Instruction.LoadConst(int(0)), Instruction.BinaryOpInstr(BinaryOp.DIV))
        assertEquals(NULL, r)
    }

    @Test
    fun `BinaryOp MOD`() {
        val r = evalExpr(Instruction.LoadConst(int(10)), Instruction.LoadConst(int(3)), Instruction.BinaryOpInstr(BinaryOp.MOD))
        assertEquals(int(1), r)
    }

    @Test
    fun `BinaryOp ADD with float promotes to float`() {
        val r = evalExpr(Instruction.LoadConst(int(3)), Instruction.LoadConst(flt(1.5)), Instruction.BinaryOpInstr(BinaryOp.ADD))
        assertEquals(flt(4.5), r)
    }

    @Test
    fun `BinaryOp with NULL propagates NULL`() {
        val r = evalExpr(Instruction.LoadConst(NULL), Instruction.LoadConst(int(5)), Instruction.BinaryOpInstr(BinaryOp.ADD))
        assertEquals(NULL, r)
    }

    @Test
    fun `BinaryOp CONCAT strings`() {
        val r = evalExpr(Instruction.LoadConst(txt("foo")), Instruction.LoadConst(txt("bar")), Instruction.BinaryOpInstr(BinaryOp.CONCAT))
        assertEquals(txt("foobar"), r)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Comparison operators
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `BinaryOp EQ true`() {
        val r = evalExpr(Instruction.LoadConst(int(5)), Instruction.LoadConst(int(5)), Instruction.BinaryOpInstr(BinaryOp.EQ))
        assertEquals(bool(true), r)
    }

    @Test
    fun `BinaryOp EQ false`() {
        val r = evalExpr(Instruction.LoadConst(int(5)), Instruction.LoadConst(int(6)), Instruction.BinaryOpInstr(BinaryOp.EQ))
        assertEquals(bool(false), r)
    }

    @Test
    fun `BinaryOp NEQ`() {
        val r = evalExpr(Instruction.LoadConst(int(5)), Instruction.LoadConst(int(6)), Instruction.BinaryOpInstr(BinaryOp.NEQ))
        assertEquals(bool(true), r)
    }

    @Test
    fun `BinaryOp LT`() {
        val r = evalExpr(Instruction.LoadConst(int(3)), Instruction.LoadConst(int(5)), Instruction.BinaryOpInstr(BinaryOp.LT))
        assertEquals(bool(true), r)
    }

    @Test
    fun `BinaryOp LTE equal`() {
        val r = evalExpr(Instruction.LoadConst(int(5)), Instruction.LoadConst(int(5)), Instruction.BinaryOpInstr(BinaryOp.LTE))
        assertEquals(bool(true), r)
    }

    @Test
    fun `BinaryOp GT`() {
        val r = evalExpr(Instruction.LoadConst(int(7)), Instruction.LoadConst(int(3)), Instruction.BinaryOpInstr(BinaryOp.GT))
        assertEquals(bool(true), r)
    }

    @Test
    fun `BinaryOp GTE`() {
        val r = evalExpr(Instruction.LoadConst(int(5)), Instruction.LoadConst(int(5)), Instruction.BinaryOpInstr(BinaryOp.GTE))
        assertEquals(bool(true), r)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Three-valued logic: AND / OR
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `AND true and true is true`() {
        val r = evalBinaryConst(bool(true), bool(true), BinaryOp.AND)
        assertEquals(bool(true), r)
    }

    @Test
    fun `AND true and false is false`() {
        val r = evalBinaryConst(bool(true), bool(false), BinaryOp.AND)
        assertEquals(bool(false), r)
    }

    @Test
    fun `AND false and NULL is false`() {
        // SQL: FALSE AND NULL = FALSE (because FALSE dominates AND)
        val r = evalBinaryConst(bool(false), NULL, BinaryOp.AND)
        assertEquals(bool(false), r)
    }

    @Test
    fun `AND true and NULL is NULL`() {
        val r = evalBinaryConst(bool(true), NULL, BinaryOp.AND)
        assertEquals(NULL, r)
    }

    @Test
    fun `OR true and NULL is true`() {
        // SQL: TRUE OR NULL = TRUE (because TRUE dominates OR)
        val r = evalBinaryConst(bool(true), NULL, BinaryOp.OR)
        assertEquals(bool(true), r)
    }

    @Test
    fun `OR false and NULL is NULL`() {
        val r = evalBinaryConst(bool(false), NULL, BinaryOp.OR)
        assertEquals(NULL, r)
    }

    @Test
    fun `OR false and false is false`() {
        val r = evalBinaryConst(bool(false), bool(false), BinaryOp.OR)
        assertEquals(bool(false), r)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Unary operators
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `UnaryOp NEG integer`() {
        val r = evalUnary(int(5), UnaryOp.NEG)
        assertEquals(int(-5), r)
    }

    @Test
    fun `UnaryOp NEG NULL is NULL`() {
        val r = evalUnary(NULL, UnaryOp.NEG)
        assertEquals(NULL, r)
    }

    @Test
    fun `UnaryOp NOT true is false`() {
        val r = evalUnary(bool(true), UnaryOp.NOT)
        assertEquals(bool(false), r)
    }

    @Test
    fun `UnaryOp NOT false is true`() {
        val r = evalUnary(bool(false), UnaryOp.NOT)
        assertEquals(bool(true), r)
    }

    @Test
    fun `UnaryOp NOT NULL is NULL`() {
        val r = evalUnary(NULL, UnaryOp.NOT)
        assertEquals(NULL, r)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // IsNull / IsNotNull
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `IsNull pushes true for NULL`() {
        val r = evalExpr(Instruction.LoadConst(NULL), Instruction.IsNull)
        assertEquals(bool(true), r)
    }

    @Test
    fun `IsNull pushes false for integer`() {
        val r = evalExpr(Instruction.LoadConst(int(0)), Instruction.IsNull)
        assertEquals(bool(false), r)
    }

    @Test
    fun `IsNotNull pushes false for NULL`() {
        val r = evalExpr(Instruction.LoadConst(NULL), Instruction.IsNotNull)
        assertEquals(bool(false), r)
    }

    @Test
    fun `IsNotNull pushes true for text`() {
        val r = evalExpr(Instruction.LoadConst(txt("hello")), Instruction.IsNotNull)
        assertEquals(bool(true), r)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // BETWEEN
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `BETWEEN true when value in range`() {
        // value=5, low=1, high=10  → TRUE
        val r = evalExpr(
            Instruction.LoadConst(int(5)),
            Instruction.LoadConst(int(1)),
            Instruction.LoadConst(int(10)),
            Instruction.Between(),
        )
        assertEquals(bool(true), r)
    }

    @Test
    fun `BETWEEN false when value below range`() {
        val r = evalExpr(
            Instruction.LoadConst(int(0)),
            Instruction.LoadConst(int(1)),
            Instruction.LoadConst(int(10)),
            Instruction.Between(),
        )
        assertEquals(bool(false), r)
    }

    @Test
    fun `BETWEEN true at boundary`() {
        val r = evalExpr(
            Instruction.LoadConst(int(1)),
            Instruction.LoadConst(int(1)),
            Instruction.LoadConst(int(10)),
            Instruction.Between(),
        )
        assertEquals(bool(true), r)
    }

    @Test
    fun `BETWEEN NULL propagates NULL`() {
        val r = evalExpr(
            Instruction.LoadConst(NULL),
            Instruction.LoadConst(int(1)),
            Instruction.LoadConst(int(10)),
            Instruction.Between(),
        )
        assertEquals(NULL, r)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // LIKE
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `LIKE percent matches any sequence`() {
        // "hello" LIKE "hel%" → TRUE
        val r = evalExpr(
            Instruction.LoadConst(txt("hello")),
            Instruction.LoadConst(txt("hel%")),
            Instruction.Like,
        )
        assertEquals(bool(true), r)
    }

    @Test
    fun `LIKE underscore matches single char`() {
        val r = evalExpr(
            Instruction.LoadConst(txt("cat")),
            Instruction.LoadConst(txt("c_t")),
            Instruction.Like,
        )
        assertEquals(bool(true), r)
    }

    @Test
    fun `LIKE no match`() {
        val r = evalExpr(
            Instruction.LoadConst(txt("dog")),
            Instruction.LoadConst(txt("cat%")),
            Instruction.Like,
        )
        assertEquals(bool(false), r)
    }

    @Test
    fun `LIKE NULL value returns NULL`() {
        val r = evalExpr(
            Instruction.LoadConst(NULL),
            Instruction.LoadConst(txt("foo%")),
            Instruction.Like,
        )
        assertEquals(NULL, r)
    }

    @Test
    fun `LIKE is case insensitive`() {
        val r = evalExpr(
            Instruction.LoadConst(txt("Hello")),
            Instruction.LoadConst(txt("hello")),
            Instruction.Like,
        )
        assertEquals(bool(true), r)
    }

    @Test
    fun `likeMatch helper percent at start`() {
        assertTrue(SqlVm.likeMatch("the end", "%end"))
    }

    @Test
    fun `likeMatch helper exact match`() {
        assertTrue(SqlVm.likeMatch("abc", "abc"))
    }

    @Test
    fun `likeMatch helper no match`() {
        assertFalse(SqlVm.likeMatch("abc", "xyz%"))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // IN list
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `InList found in list returns true`() {
        val r = evalExpr(
            Instruction.LoadConst(int(2)),
            Instruction.LoadConst(int(1)),
            Instruction.LoadConst(int(2)),
            Instruction.LoadConst(int(3)),
            Instruction.InList(3),
        )
        assertEquals(bool(true), r)
    }

    @Test
    fun `InList not found returns false`() {
        val r = evalExpr(
            Instruction.LoadConst(int(99)),
            Instruction.LoadConst(int(1)),
            Instruction.LoadConst(int(2)),
            Instruction.InList(2),
        )
        assertEquals(bool(false), r)
    }

    @Test
    fun `InList empty list returns false`() {
        val r = evalExpr(
            Instruction.LoadConst(int(5)),
            Instruction.InList(0),
        )
        assertEquals(bool(false), r)
    }

    @Test
    fun `InList NULL needle returns NULL`() {
        val r = evalExpr(
            Instruction.LoadConst(NULL),
            Instruction.LoadConst(int(1)),
            Instruction.InList(1),
        )
        assertEquals(NULL, r)
    }

    @Test
    fun `InList not found but NULL in list returns NULL`() {
        val r = evalExpr(
            Instruction.LoadConst(int(5)),
            Instruction.LoadConst(int(1)),
            Instruction.LoadConst(NULL),
            Instruction.InList(2),
        )
        assertEquals(NULL, r)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Scan / cursor — SELECT from table
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `scan empty table returns no rows`() {
        backend.createTable("t", listOf(ColumnDef("x", "INTEGER")), ifNotExists = false)

        val result = SqlVm.execute(
            prog(
                Instruction.OpenScan("t", "t"),
                Instruction.Label("loop"),
                Instruction.AdvanceCursor("t", "end"),
                Instruction.BeginRow,
                Instruction.LoadColumn("t", "x"),
                Instruction.EmitColumn("x"),
                Instruction.EmitRow,
                Instruction.Jump("loop"),
                Instruction.Label("end"),
                Instruction.CloseScan("t"),
                Instruction.Halt,
            ),
            backend,
        )
        assertTrue(result.rows.isEmpty())
    }

    @Test
    fun `scan table with rows returns all rows`() {
        backend = backendWithUsers(
            mapOf("id" to 1L, "name" to "Alice", "age" to 30L),
            mapOf("id" to 2L, "name" to "Bob",   "age" to 25L),
        )

        val result = SqlVm.execute(
            prog(
                Instruction.OpenScan("users", "users"),
                Instruction.Label("loop"),
                Instruction.AdvanceCursor("users", "end"),
                Instruction.BeginRow,
                Instruction.LoadColumn("users", "id"),
                Instruction.EmitColumn("id"),
                Instruction.LoadColumn("users", "name"),
                Instruction.EmitColumn("name"),
                Instruction.EmitRow,
                Instruction.Jump("loop"),
                Instruction.Label("end"),
                Instruction.CloseScan("users"),
                Instruction.Halt,
            ),
            backend,
        )

        assertEquals(2, result.rows.size)
        assertEquals(listOf("id", "name"), result.columns)
        assertEquals(int(1), result.rows[0][0])
        assertEquals(txt("Alice"), result.rows[0][1])
        assertEquals(int(2), result.rows[1][0])
        assertEquals(txt("Bob"), result.rows[1][1])
    }

    @Test
    fun `scan with filter skips non-matching rows`() {
        backend = backendWithUsers(
            mapOf("id" to 1L, "name" to "Alice", "age" to 30L),
            mapOf("id" to 2L, "name" to "Bob",   "age" to 25L),
            mapOf("id" to 3L, "name" to "Carol",  "age" to 35L),
        )

        // SELECT id FROM users WHERE age > 28
        val result = SqlVm.execute(
            prog(
                Instruction.OpenScan("users", "users"),
                Instruction.Label("loop"),
                Instruction.AdvanceCursor("users", "end"),
                // age > 28
                Instruction.LoadColumn("users", "age"),
                Instruction.LoadConst(int(28)),
                Instruction.BinaryOpInstr(BinaryOp.GT),
                Instruction.JumpIfFalse("skip"),
                // emit
                Instruction.BeginRow,
                Instruction.LoadColumn("users", "id"),
                Instruction.EmitColumn("id"),
                Instruction.EmitRow,
                Instruction.Label("skip"),
                Instruction.Jump("loop"),
                Instruction.Label("end"),
                Instruction.CloseScan("users"),
                Instruction.Halt,
            ),
            backend,
        )

        // Alice (30) and Carol (35) pass the filter; Bob (25) does not.
        assertEquals(2, result.rows.size)
        assertEquals(int(1), result.rows[0][0])
        assertEquals(int(3), result.rows[1][0])
    }

    @Test
    fun `LoadColumn returns NULL for missing column`() {
        backend.createTable("t", listOf(ColumnDef("x", "INTEGER")), ifNotExists = false)
        val row = Row(); row["x"] = 1L
        backend.insert("t", row)

        val result = SqlVm.execute(
            prog(
                Instruction.OpenScan("t", "t"),
                Instruction.Label("loop"),
                Instruction.AdvanceCursor("t", "end"),
                Instruction.BeginRow,
                Instruction.LoadColumn("t", "nonexistent"),
                Instruction.EmitColumn("v"),
                Instruction.EmitRow,
                Instruction.Jump("loop"),
                Instruction.Label("end"),
                Instruction.CloseScan("t"),
                Instruction.Halt,
            ),
            backend,
        )
        assertEquals(NULL, result.rows[0][0])
    }

    // ─────────────────────────────────────────────────────────────────────────
    // DDL: CREATE TABLE / DROP TABLE
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `CreateTableInstr creates a table`() {
        val result = SqlVm.execute(
            prog(
                Instruction.CreateTableInstr(
                    "products",
                    ifNotExists = false,
                    columns = listOf(PlannerColumnDef("id", "INTEGER", notNull = false, primaryKey = false, unique = false, default = null), PlannerColumnDef("name", "TEXT", notNull = false, primaryKey = false, unique = false, default = null)),
                ),
                Instruction.Halt,
            ),
            backend,
        )
        assertTrue(result.rows.isEmpty())
        assertTrue(backend.tables().contains("products"))
    }

    @Test
    fun `CreateTableInstr with ifNotExists is idempotent`() {
        backend.createTable("t", listOf(ColumnDef("x", "INTEGER")), ifNotExists = false)
        // Should NOT throw even though table exists.
        SqlVm.execute(
            prog(
                Instruction.CreateTableInstr("t", ifNotExists = true, columns = listOf(PlannerColumnDef("x", "INTEGER", notNull = false, primaryKey = false, unique = false, default = null))),
                Instruction.Halt,
            ),
            backend,
        )
    }

    @Test
    fun `DropTableInstr removes a table`() {
        backend.createTable("t", listOf(ColumnDef("x", "INTEGER")), ifNotExists = false)
        SqlVm.execute(
            prog(Instruction.DropTableInstr("t", ifExists = false), Instruction.Halt),
            backend,
        )
        assertFalse(backend.tables().contains("t"))
    }

    @Test
    fun `DropTableInstr with ifExists is idempotent`() {
        // Should NOT throw even though table does not exist.
        SqlVm.execute(
            prog(Instruction.DropTableInstr("no_such_table", ifExists = true), Instruction.Halt),
            backend,
        )
    }

    // ─────────────────────────────────────────────────────────────────────────
    // DML: INSERT
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `InsertRow inserts a single row`() {
        backend.createTable("t", listOf(ColumnDef("x", "INTEGER"), ColumnDef("y", "TEXT")), ifNotExists = false)

        val result = SqlVm.execute(
            prog(
                Instruction.LoadConst(int(7)),
                Instruction.LoadConst(txt("hello")),
                Instruction.InsertRow("t", listOf("x", "y")),
                Instruction.Halt,
            ),
            backend,
        )

        assertEquals(1, result.rowsAffected)
        val rows = backend.scan("t").let {
            val out = mutableListOf<Row>()
            while (true) out += it.next() ?: break
            out
        }
        assertEquals(1, rows.size)
        assertEquals(7L, rows[0]["x"])
        assertEquals("hello", rows[0]["y"])
    }

    @Test
    fun `InsertRow multiple rows`() {
        backend.createTable("t", listOf(ColumnDef("v", "INTEGER")), ifNotExists = false)

        SqlVm.execute(
            prog(
                Instruction.LoadConst(int(1)),
                Instruction.InsertRow("t", listOf("v")),
                Instruction.LoadConst(int(2)),
                Instruction.InsertRow("t", listOf("v")),
                Instruction.LoadConst(int(3)),
                Instruction.InsertRow("t", listOf("v")),
                Instruction.Halt,
            ),
            backend,
        )

        val rows = scanAll(backend, "t")
        assertEquals(3, rows.size)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // DML: DELETE
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `DeleteRows removes matching rows`() {
        backend = backendWithUsers(
            mapOf("id" to 1L, "name" to "Alice", "age" to 30L),
            mapOf("id" to 2L, "name" to "Bob",   "age" to 25L),
        )

        // DELETE FROM users WHERE id = 1
        SqlVm.execute(
            prog(
                Instruction.OpenScan("users", null),
                Instruction.Label("loop"),
                Instruction.AdvanceCursor(null, "end"),
                Instruction.LoadColumn(null, "id"),
                Instruction.LoadConst(int(1)),
                Instruction.BinaryOpInstr(BinaryOp.EQ),
                Instruction.JumpIfFalse("skip"),
                Instruction.DeleteRows("users"),
                Instruction.Label("skip"),
                Instruction.Jump("loop"),
                Instruction.Label("end"),
                Instruction.CloseScan(null),
                Instruction.Halt,
            ),
            backend,
        )

        val remaining = scanAll(backend, "users")
        assertEquals(1, remaining.size)
        assertEquals("Bob", remaining[0]["name"])
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Post-processing: LIMIT / OFFSET
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `LimitResult with count limits rows`() {
        backend = backendWithUsers(
            mapOf("id" to 1L, "name" to "A", "age" to 10L),
            mapOf("id" to 2L, "name" to "B", "age" to 20L),
            mapOf("id" to 3L, "name" to "C", "age" to 30L),
        )

        val result = SqlVm.execute(
            prog(
                Instruction.OpenScan("users", "u"),
                Instruction.Label("loop"),
                Instruction.AdvanceCursor("u", "end"),
                Instruction.BeginRow,
                Instruction.LoadColumn("u", "id"),
                Instruction.EmitColumn("id"),
                Instruction.EmitRow,
                Instruction.Jump("loop"),
                Instruction.Label("end"),
                Instruction.CloseScan("u"),
                Instruction.LimitResult(count = 2L, offset = null),
                Instruction.Halt,
            ),
            backend,
        )
        assertEquals(2, result.rows.size)
        assertEquals(int(1), result.rows[0][0])
        assertEquals(int(2), result.rows[1][0])
    }

    @Test
    fun `LimitResult with offset skips rows`() {
        backend = backendWithUsers(
            mapOf("id" to 1L, "name" to "A", "age" to 10L),
            mapOf("id" to 2L, "name" to "B", "age" to 20L),
            mapOf("id" to 3L, "name" to "C", "age" to 30L),
        )

        val result = SqlVm.execute(
            prog(
                Instruction.OpenScan("users", "u"),
                Instruction.Label("loop"),
                Instruction.AdvanceCursor("u", "end"),
                Instruction.BeginRow,
                Instruction.LoadColumn("u", "id"),
                Instruction.EmitColumn("id"),
                Instruction.EmitRow,
                Instruction.Jump("loop"),
                Instruction.Label("end"),
                Instruction.CloseScan("u"),
                Instruction.LimitResult(count = 1L, offset = 1L),
                Instruction.Halt,
            ),
            backend,
        )
        assertEquals(1, result.rows.size)
        assertEquals(int(2), result.rows[0][0])
    }

    @Test
    fun `LimitResult null count returns all rows from offset`() {
        backend = backendWithUsers(
            mapOf("id" to 1L, "name" to "A", "age" to 10L),
            mapOf("id" to 2L, "name" to "B", "age" to 20L),
            mapOf("id" to 3L, "name" to "C", "age" to 30L),
        )

        val result = SqlVm.execute(
            prog(
                Instruction.OpenScan("users", "u"),
                Instruction.Label("loop"),
                Instruction.AdvanceCursor("u", "end"),
                Instruction.BeginRow,
                Instruction.LoadColumn("u", "id"),
                Instruction.EmitColumn("id"),
                Instruction.EmitRow,
                Instruction.Jump("loop"),
                Instruction.Label("end"),
                Instruction.CloseScan("u"),
                Instruction.LimitResult(count = null, offset = 1L),
                Instruction.Halt,
            ),
            backend,
        )
        assertEquals(2, result.rows.size)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Post-processing: DISTINCT
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `DistinctResult removes duplicates`() {
        backend.createTable("t", listOf(ColumnDef("v", "INTEGER")), ifNotExists = false)
        for (v in listOf(1L, 2L, 2L, 3L, 1L)) {
            val row = Row(); row["v"] = v; backend.insert("t", row)
        }

        val result = SqlVm.execute(
            prog(
                Instruction.OpenScan("t", "t"),
                Instruction.Label("loop"),
                Instruction.AdvanceCursor("t", "end"),
                Instruction.BeginRow,
                Instruction.LoadColumn("t", "v"),
                Instruction.EmitColumn("v"),
                Instruction.EmitRow,
                Instruction.Jump("loop"),
                Instruction.Label("end"),
                Instruction.CloseScan("t"),
                Instruction.DistinctResult,
                Instruction.Halt,
            ),
            backend,
        )

        assertEquals(3, result.rows.size)
        val values = result.rows.map { it[0] }.toSet()
        assertTrue(values.contains(int(1)))
        assertTrue(values.contains(int(2)))
        assertTrue(values.contains(int(3)))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Post-processing: ORDER BY
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `SortResult ascending by integer column`() {
        backend = backendWithUsers(
            mapOf("id" to 3L, "name" to "C", "age" to 30L),
            mapOf("id" to 1L, "name" to "A", "age" to 10L),
            mapOf("id" to 2L, "name" to "B", "age" to 20L),
        )

        val result = SqlVm.execute(
            prog(
                Instruction.OpenScan("users", "u"),
                Instruction.Label("loop"),
                Instruction.AdvanceCursor("u", "end"),
                Instruction.BeginRow,
                Instruction.LoadColumn("u", "id"),
                Instruction.EmitColumn("id"),
                Instruction.EmitRow,
                Instruction.Jump("loop"),
                Instruction.Label("end"),
                Instruction.CloseScan("u"),
                Instruction.SortResult(
                    keys = listOf(SortKey(SqlExpr.Column(null, "id"), direction = SortDir.ASC, nullOrder = NullOrder.NULLS_LAST)),
                ),
                Instruction.Halt,
            ),
            backend,
        )

        assertEquals(listOf(int(1), int(2), int(3)), result.rows.map { it[0] })
    }

    @Test
    fun `SortResult descending`() {
        backend = backendWithUsers(
            mapOf("id" to 1L, "name" to "A", "age" to 10L),
            mapOf("id" to 2L, "name" to "B", "age" to 20L),
            mapOf("id" to 3L, "name" to "C", "age" to 30L),
        )

        val result = SqlVm.execute(
            prog(
                Instruction.OpenScan("users", "u"),
                Instruction.Label("loop"),
                Instruction.AdvanceCursor("u", "end"),
                Instruction.BeginRow,
                Instruction.LoadColumn("u", "id"),
                Instruction.EmitColumn("id"),
                Instruction.EmitRow,
                Instruction.Jump("loop"),
                Instruction.Label("end"),
                Instruction.CloseScan("u"),
                Instruction.SortResult(
                    keys = listOf(SortKey(SqlExpr.Column(null, "id"), direction = SortDir.DESC, nullOrder = NullOrder.NULLS_LAST)),
                ),
                Instruction.Halt,
            ),
            backend,
        )

        assertEquals(listOf(int(3), int(2), int(1)), result.rows.map { it[0] })
    }

    @Test
    fun `SortResult NULLs first`() {
        backend.createTable("t", listOf(ColumnDef("v", "INTEGER")), ifNotExists = false)
        for (v in listOf<Long?>(2L, null, 1L)) {
            val row = Row(); row["v"] = v; backend.insert("t", row)
        }

        val result = SqlVm.execute(
            prog(
                Instruction.OpenScan("t", "t"),
                Instruction.Label("loop"),
                Instruction.AdvanceCursor("t", "end"),
                Instruction.BeginRow,
                Instruction.LoadColumn("t", "v"),
                Instruction.EmitColumn("v"),
                Instruction.EmitRow,
                Instruction.Jump("loop"),
                Instruction.Label("end"),
                Instruction.CloseScan("t"),
                Instruction.SortResult(
                    keys = listOf(SortKey(SqlExpr.Column(null, "v"), direction = SortDir.ASC, nullOrder = NullOrder.NULLS_FIRST)),
                ),
                Instruction.Halt,
            ),
            backend,
        )

        assertEquals(NULL,   result.rows[0][0])
        assertEquals(int(1), result.rows[1][0])
        assertEquals(int(2), result.rows[2][0])
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Aggregation: COUNT, SUM, AVG, MIN, MAX
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `COUNT STAR over table`() {
        backend = backendWithUsers(
            mapOf("id" to 1L, "name" to "A", "age" to 10L),
            mapOf("id" to 2L, "name" to "B", "age" to 20L),
            mapOf("id" to 3L, "name" to "C", "age" to 30L),
        )

        val result = SqlVm.execute(buildCountStarProgram("users", "u"), backend)
        assertEquals(1, result.rows.size)
        assertEquals(int(3), result.rows[0][0])
    }

    @Test
    fun `COUNT STAR over empty table returns zero`() {
        backend.createTable("t", listOf(ColumnDef("x", "INTEGER")), ifNotExists = false)
        val result = SqlVm.execute(buildCountStarProgram("t", "t"), backend)
        assertEquals(1, result.rows.size)
        assertEquals(int(0), result.rows[0][0])
    }

    @Test
    fun `SUM of integer column`() {
        backend = backendWithUsers(
            mapOf("id" to 1L, "name" to "A", "age" to 10L),
            mapOf("id" to 2L, "name" to "B", "age" to 20L),
            mapOf("id" to 3L, "name" to "C", "age" to 30L),
        )

        val result = SqlVm.execute(buildSingleAggProgram("users", "u", "age", AggFn.SUM, "total"), backend)
        assertEquals(1, result.rows.size)
        assertEquals(int(60), result.rows[0][0])
    }

    @Test
    fun `AVG of integer column`() {
        backend = backendWithUsers(
            mapOf("id" to 1L, "name" to "A", "age" to 10L),
            mapOf("id" to 2L, "name" to "B", "age" to 20L),
            mapOf("id" to 3L, "name" to "C", "age" to 30L),
        )

        val result = SqlVm.execute(buildSingleAggProgram("users", "u", "age", AggFn.AVG, "avg_age"), backend)
        assertEquals(1, result.rows.size)
        // 60 / 3 = 20.0
        assertEquals(flt(20.0), result.rows[0][0])
    }

    @Test
    fun `MIN of integer column`() {
        backend = backendWithUsers(
            mapOf("id" to 1L, "name" to "A", "age" to 30L),
            mapOf("id" to 2L, "name" to "B", "age" to 10L),
            mapOf("id" to 3L, "name" to "C", "age" to 20L),
        )

        val result = SqlVm.execute(buildSingleAggProgram("users", "u", "age", AggFn.MIN, "min_age"), backend)
        assertEquals(int(10), result.rows[0][0])
    }

    @Test
    fun `MAX of integer column`() {
        backend = backendWithUsers(
            mapOf("id" to 1L, "name" to "A", "age" to 30L),
            mapOf("id" to 2L, "name" to "B", "age" to 10L),
            mapOf("id" to 3L, "name" to "C", "age" to 20L),
        )

        val result = SqlVm.execute(buildSingleAggProgram("users", "u", "age", AggFn.MAX, "max_age"), backend)
        assertEquals(int(30), result.rows[0][0])
    }

    @Test
    fun `COUNT ignores NULLs`() {
        backend.createTable("t", listOf(ColumnDef("v", "INTEGER")), ifNotExists = false)
        for (v in listOf<Long?>(1L, null, 3L)) {
            val row = Row(); row["v"] = v; backend.insert("t", row)
        }
        val result = SqlVm.execute(buildSingleAggProgram("t", "t", "v", AggFn.COUNT, "cnt"), backend)
        assertEquals(int(2), result.rows[0][0])
    }

    @Test
    fun `SUM ignores NULLs`() {
        backend.createTable("t", listOf(ColumnDef("v", "INTEGER")), ifNotExists = false)
        for (v in listOf<Long?>(5L, null, 10L)) {
            val row = Row(); row["v"] = v; backend.insert("t", row)
        }
        val result = SqlVm.execute(buildSingleAggProgram("t", "t", "v", AggFn.SUM, "s"), backend)
        assertEquals(int(15), result.rows[0][0])
    }

    @Test
    fun `AVG over empty result returns NULL`() {
        backend.createTable("t", listOf(ColumnDef("v", "INTEGER")), ifNotExists = false)
        val result = SqlVm.execute(buildSingleAggProgram("t", "t", "v", AggFn.AVG, "a"), backend)
        assertEquals(NULL, result.rows[0][0])
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Transactions
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `transaction commit persists insert`() {
        backend.createTable("t", listOf(ColumnDef("v", "INTEGER")), ifNotExists = false)

        SqlVm.execute(
            prog(
                Instruction.BeginTransaction,
                Instruction.LoadConst(int(42)),
                Instruction.InsertRow("t", listOf("v")),
                Instruction.CommitTransaction,
                Instruction.Halt,
            ),
            backend,
        )

        val rows = scanAll(backend, "t")
        assertEquals(1, rows.size)
        assertEquals(42L, rows[0]["v"])
    }

    @Test
    fun `transaction rollback discards insert`() {
        backend.createTable("t", listOf(ColumnDef("v", "INTEGER")), ifNotExists = false)

        SqlVm.execute(
            prog(
                Instruction.BeginTransaction,
                Instruction.LoadConst(int(99)),
                Instruction.InsertRow("t", listOf("v")),
                Instruction.RollbackTransaction,
                Instruction.Halt,
            ),
            backend,
        )

        val rows = scanAll(backend, "t")
        assertTrue(rows.isEmpty())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Control flow: JumpIfTrue, JumpIfFalse, Jump
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `JumpIfTrue skips when condition is false`() {
        // If 1 == 2 (false), skip to done without emitting; else emit 99.
        val result = SqlVm.execute(
            prog(
                Instruction.LoadConst(int(1)),
                Instruction.LoadConst(int(2)),
                Instruction.BinaryOpInstr(BinaryOp.EQ),
                Instruction.JumpIfTrue("done"),
                Instruction.BeginRow,
                Instruction.LoadConst(int(99)),
                Instruction.EmitColumn("v"),
                Instruction.EmitRow,
                Instruction.Label("done"),
                Instruction.Halt,
            ),
            backend,
        )
        // Condition is false → JumpIfTrue does NOT jump → row IS emitted.
        assertEquals(1, result.rows.size)
    }

    @Test
    fun `JumpIfFalse jumps when condition is false`() {
        // If 5 > 10 (false), jump to done.
        val result = SqlVm.execute(
            prog(
                Instruction.LoadConst(int(5)),
                Instruction.LoadConst(int(10)),
                Instruction.BinaryOpInstr(BinaryOp.GT),
                Instruction.JumpIfFalse("done"),
                Instruction.BeginRow,
                Instruction.LoadConst(int(99)),
                Instruction.EmitColumn("v"),
                Instruction.EmitRow,
                Instruction.Label("done"),
                Instruction.Halt,
            ),
            backend,
        )
        // Condition is false → jump → row is NOT emitted.
        assertTrue(result.rows.isEmpty())
    }

    @Test
    fun `JumpIfFalse NULL is treated as false`() {
        // NULL condition → JumpIfFalse should jump (NULL is falsy).
        val result = SqlVm.execute(
            prog(
                Instruction.LoadConst(NULL),
                Instruction.JumpIfFalse("done"),
                Instruction.BeginRow,
                Instruction.LoadConst(int(1)),
                Instruction.EmitColumn("v"),
                Instruction.EmitRow,
                Instruction.Label("done"),
                Instruction.Halt,
            ),
            backend,
        )
        assertTrue(result.rows.isEmpty())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // LoadParam placeholder
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `LoadParam pushes NULL as placeholder`() {
        val result = SqlVm.execute(
            prog(
                Instruction.BeginRow,
                Instruction.LoadParam(0),
                Instruction.EmitColumn("v"),
                Instruction.EmitRow,
                Instruction.Halt,
            ),
            backend,
        )
        assertEquals(NULL, result.rows[0][0])
    }

    // ─────────────────────────────────────────────────────────────────────────
    // QueryResult structure
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `multiple columns in result`() {
        val result = SqlVm.execute(
            prog(
                Instruction.BeginRow,
                Instruction.LoadConst(int(1)),
                Instruction.EmitColumn("a"),
                Instruction.LoadConst(txt("hello")),
                Instruction.EmitColumn("b"),
                Instruction.LoadConst(flt(3.14)),
                Instruction.EmitColumn("c"),
                Instruction.EmitRow,
                Instruction.Halt,
            ),
            backend,
        )
        assertEquals(listOf("a", "b", "c"), result.columns)
        assertEquals(int(1),    result.rows[0][0])
        assertEquals(txt("hello"), result.rows[0][1])
        assertEquals(flt(3.14), result.rows[0][2])
    }

    @Test
    fun `rowsAffected is zero for SELECT`() {
        val result = SqlVm.execute(
            prog(
                Instruction.BeginRow,
                Instruction.LoadConst(int(1)),
                Instruction.EmitColumn("v"),
                Instruction.EmitRow,
                Instruction.Halt,
            ),
            backend,
        )
        assertEquals(0, result.rowsAffected)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Float arithmetic
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `float division`() {
        val r = evalExpr(
            Instruction.LoadConst(flt(7.0)),
            Instruction.LoadConst(flt(2.0)),
            Instruction.BinaryOpInstr(BinaryOp.DIV),
        )
        assertEquals(flt(3.5), r)
    }

    @Test
    fun `integer and float ADD promotes to float`() {
        val r = evalExpr(
            Instruction.LoadConst(int(1)),
            Instruction.LoadConst(flt(0.5)),
            Instruction.BinaryOpInstr(BinaryOp.ADD),
        )
        assertEquals(flt(1.5), r)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Edge cases
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    fun `multiple EmitRow calls accumulate rows`() {
        val result = SqlVm.execute(
            prog(
                Instruction.BeginRow,
                Instruction.LoadConst(int(1)),
                Instruction.EmitColumn("v"),
                Instruction.EmitRow,
                Instruction.BeginRow,
                Instruction.LoadConst(int(2)),
                Instruction.EmitColumn("v"),
                Instruction.EmitRow,
                Instruction.Halt,
            ),
            backend,
        )
        assertEquals(2, result.rows.size)
        assertEquals(int(1), result.rows[0][0])
        assertEquals(int(2), result.rows[1][0])
    }

    @Test
    fun `stack underflow throws exception`() {
        assertThrows<IllegalStateException> {
            SqlVm.execute(
                prog(Instruction.Pop, Instruction.Halt),
                backend,
            )
        }
    }

    @Test
    fun `unknown label throws exception`() {
        assertThrows<IllegalStateException> {
            SqlVm.execute(
                prog(Instruction.Jump("nonexistent_label"), Instruction.Halt),
                backend,
            )
        }
    }

    @Test
    fun `BoolVal true in LoadConst`() {
        val r = evalExpr(Instruction.LoadConst(bool(true)))
        assertEquals(bool(true), r)
    }

    @Test
    fun `BoolVal false in LoadConst`() {
        val r = evalExpr(Instruction.LoadConst(bool(false)))
        assertEquals(bool(false), r)
    }

    @Test
    fun `NEG float`() {
        val r = evalUnary(flt(2.5), UnaryOp.NEG)
        assertEquals(flt(-2.5), r)
    }

    @Test
    fun `MOD with floats`() {
        val r = evalExpr(
            Instruction.LoadConst(flt(7.5)),
            Instruction.LoadConst(flt(2.0)),
            Instruction.BinaryOpInstr(BinaryOp.MOD),
        )
        assertEquals(flt(1.5), r)
    }

    @Test
    fun `CONCAT with null propagates null`() {
        // SQLite/SQL standard: NULL || 'world' = NULL (NULL propagates through CONCAT).
        val r = evalExpr(
            Instruction.LoadConst(NULL),
            Instruction.LoadConst(txt("world")),
            Instruction.BinaryOpInstr(BinaryOp.CONCAT),
        )
        assertEquals(NULL, r)
    }

    @Test
    fun `scan with null alias cursor`() {
        backend.createTable("t", listOf(ColumnDef("x", "INTEGER")), ifNotExists = false)
        val row = Row(); row["x"] = 5L; backend.insert("t", row)

        // UPDATE pattern: anonymous cursor (alias=null)
        val result = SqlVm.execute(
            prog(
                Instruction.OpenScan("t", null),
                Instruction.Label("loop"),
                Instruction.AdvanceCursor(null, "end"),
                Instruction.BeginRow,
                Instruction.LoadColumn(null, "x"),
                Instruction.EmitColumn("x"),
                Instruction.EmitRow,
                Instruction.Jump("loop"),
                Instruction.Label("end"),
                Instruction.CloseScan(null),
                Instruction.Halt,
            ),
            backend,
        )
        assertEquals(1, result.rows.size)
        assertEquals(int(5), result.rows[0][0])
    }

    @Test
    fun `LimitResult offset beyond size returns empty`() {
        backend = backendWithUsers(
            mapOf("id" to 1L, "name" to "A", "age" to 10L),
        )
        val result = SqlVm.execute(
            prog(
                Instruction.OpenScan("users", "u"),
                Instruction.Label("loop"),
                Instruction.AdvanceCursor("u", "end"),
                Instruction.BeginRow,
                Instruction.LoadColumn("u", "id"),
                Instruction.EmitColumn("id"),
                Instruction.EmitRow,
                Instruction.Jump("loop"),
                Instruction.Label("end"),
                Instruction.CloseScan("u"),
                Instruction.LimitResult(count = 10L, offset = 100L),
                Instruction.Halt,
            ),
            backend,
        )
        assertTrue(result.rows.isEmpty())
    }

    @Test
    fun `DROP TABLE then CREATE TABLE works`() {
        backend.createTable("t", listOf(ColumnDef("x", "INTEGER")), ifNotExists = false)
        SqlVm.execute(prog(Instruction.DropTableInstr("t", ifExists = false), Instruction.Halt), backend)
        SqlVm.execute(prog(Instruction.CreateTableInstr("t", ifNotExists = false, columns = listOf(PlannerColumnDef("y", "TEXT", notNull = false, primaryKey = false, unique = false, default = null))), Instruction.Halt), backend)
        assertTrue(backend.tables().contains("t"))
    }

    @Test
    fun `DistinctResult with single row is unchanged`() {
        val result = SqlVm.execute(
            prog(
                Instruction.BeginRow,
                Instruction.LoadConst(int(1)),
                Instruction.EmitColumn("v"),
                Instruction.EmitRow,
                Instruction.DistinctResult,
                Instruction.Halt,
            ),
            backend,
        )
        assertEquals(1, result.rows.size)
    }

    @Test
    fun `SortResult with no keys leaves order unchanged`() {
        backend = backendWithUsers(
            mapOf("id" to 3L, "name" to "C", "age" to 30L),
            mapOf("id" to 1L, "name" to "A", "age" to 10L),
        )
        val result = SqlVm.execute(
            prog(
                Instruction.OpenScan("users", "u"),
                Instruction.Label("loop"),
                Instruction.AdvanceCursor("u", "end"),
                Instruction.BeginRow,
                Instruction.LoadColumn("u", "id"),
                Instruction.EmitColumn("id"),
                Instruction.EmitRow,
                Instruction.Jump("loop"),
                Instruction.Label("end"),
                Instruction.CloseScan("u"),
                Instruction.SortResult(keys = emptyList()),
                Instruction.Halt,
            ),
            backend,
        )
        assertEquals(2, result.rows.size)
    }

    @Test
    fun `MUL with NULL propagates NULL`() {
        val r = evalExpr(
            Instruction.LoadConst(int(5)),
            Instruction.LoadConst(NULL),
            Instruction.BinaryOpInstr(BinaryOp.MUL),
        )
        assertEquals(NULL, r)
    }

    @Test
    fun `NEQ with equal values returns false`() {
        val r = evalExpr(
            Instruction.LoadConst(int(5)),
            Instruction.LoadConst(int(5)),
            Instruction.BinaryOpInstr(BinaryOp.NEQ),
        )
        assertEquals(bool(false), r)
    }

    @Test
    fun `LTE with larger left returns false`() {
        val r = evalExpr(
            Instruction.LoadConst(int(10)),
            Instruction.LoadConst(int(5)),
            Instruction.BinaryOpInstr(BinaryOp.LTE),
        )
        assertEquals(bool(false), r)
    }

    @Test
    fun `GTE with smaller left returns false`() {
        val r = evalExpr(
            Instruction.LoadConst(int(3)),
            Instruction.LoadConst(int(5)),
            Instruction.BinaryOpInstr(BinaryOp.GTE),
        )
        assertEquals(bool(false), r)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Helpers for building test programs
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Execute a short expression-only program and return the single result value.
     *
     * The program emits exactly one row with column "result".
     */
    private fun evalExpr(vararg instrs: Instruction): SqlValue {
        val full = mutableListOf<Instruction>()
        full.add(Instruction.BeginRow)
        full.addAll(instrs)
        full.add(Instruction.EmitColumn("result"))
        full.add(Instruction.EmitRow)
        full.add(Instruction.Halt)
        return SqlVm.execute(Program(full), backend).rows[0][0]
    }

    private fun evalBinaryConst(left: SqlValue, right: SqlValue, op: BinaryOp): SqlValue =
        evalExpr(
            Instruction.LoadConst(left),
            Instruction.LoadConst(right),
            Instruction.BinaryOpInstr(op),
        )

    private fun evalUnary(operand: SqlValue, op: UnaryOp): SqlValue =
        evalExpr(Instruction.LoadConst(operand), Instruction.UnaryOpInstr(op))

    /**
     * Build a COUNT(*) aggregate program over [table] with cursor alias [alias].
     */
    private fun buildCountStarProgram(table: String, alias: String): Program = prog(
        Instruction.OpenScan(table, alias),
        Instruction.Label("loop"),
        Instruction.AdvanceCursor(alias, "end"),
        Instruction.InitAgg(0, AggFn.COUNT_STAR),
        Instruction.LoadConst(NULL),
        Instruction.UpdateAgg(0, AggFn.COUNT_STAR),
        Instruction.Jump("loop"),
        Instruction.Label("end"),
        Instruction.CloseScan(alias),
        // Finalize phase
        Instruction.Label("finalize"),
        Instruction.AdvanceGroup,
        Instruction.BeginRow,
        Instruction.FinalizeAgg(0, AggFn.COUNT_STAR),
        Instruction.EmitColumn("count"),
        Instruction.EmitRow,
        Instruction.Jump("finalize"),
        Instruction.Label("done"),
        Instruction.Halt,
    )

    /**
     * Build a single-aggregate program: SELECT agg_fn(column) FROM table.
     */
    private fun buildSingleAggProgram(
        table: String,
        alias: String,
        column: String,
        fn: AggFn,
        resultAlias: String,
    ): Program = prog(
        Instruction.OpenScan(table, alias),
        Instruction.Label("loop"),
        Instruction.AdvanceCursor(alias, "end"),
        Instruction.InitAgg(0, fn),
        Instruction.LoadColumn(alias, column),
        Instruction.UpdateAgg(0, fn),
        Instruction.Jump("loop"),
        Instruction.Label("end"),
        Instruction.CloseScan(alias),
        // Finalize phase
        Instruction.Label("finalize"),
        Instruction.AdvanceGroup,
        Instruction.BeginRow,
        Instruction.FinalizeAgg(0, fn),
        Instruction.EmitColumn(resultAlias),
        Instruction.EmitRow,
        Instruction.Jump("finalize"),
        Instruction.Label("done"),
        Instruction.Halt,
    )

    /** Drain all rows from [table] into a list for inspection. */
    private fun scanAll(b: InMemoryBackend, table: String): List<Row> {
        val it = b.scan(table)
        val out = mutableListOf<Row>()
        while (true) out += it.next() ?: break
        return out
    }
}

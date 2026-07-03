package com.codingadventures.sqlvm;

// SqlVmTest.java — integration tests for the SQL VM.
//
// All tests use InMemoryBackend so they are self-contained and reproducible.
// Test structure:
//   1. Build a Program manually (bypassing the parser/planner/optimizer) to
//      test the VM in isolation.
//   2. For each SQL operation, assemble the exact bytecode the codegen would
//      emit, execute it, and assert on the QueryResult.
//
// Coverage targets (JaCoCo ≥ 80%):
//   - LoadConst, LoadColumn, BinaryOp, UnaryOp, IsNull, IsNotNull
//   - Between, InList, Like
//   - OpenScan, AdvanceCursor, CloseScan
//   - BeginRow, EmitColumn, EmitRow, SetResultSchema
//   - InitAgg, UpdateAgg, FinalizeAgg (COUNT, SUM, AVG, MIN, MAX, COUNT_STAR)
//   - SaveGroupKey, LoadGroupKey, AdvanceGroupKey
//   - SortResult, LimitResult, DistinctResult
//   - JoinBeginRow, JoinSetMatched, JoinIfMatched
//   - InsertRow, UpdateRows, DeleteRows
//   - CreateTable, DropTable
//   - Label, Jump, JumpIfFalse, JumpIfTrue, Halt
//   - NULL arithmetic propagation
//   - Three-valued logic (AND, OR)
//   - LIKE pattern matching
//   - Empty table aggregates

import com.codingadventures.sqlbackend.SqlBackend;
import com.codingadventures.sqlbackend.SqlBackend.ColumnDef;
import com.codingadventures.sqlbackend.SqlBackend.InMemoryBackend;
import com.codingadventures.sqlbackend.SqlBackend.Row;
import com.codingadventures.sqlbackend.SqlBackend.RowIterator;
import com.codingadventures.sqlcodegen.SqlCodegen;
import com.codingadventures.sqlcodegen.SqlCodegen.AggFunc;
import com.codingadventures.sqlcodegen.SqlCodegen.BinaryOpCode;
import com.codingadventures.sqlcodegen.SqlCodegen.Direction;
import com.codingadventures.sqlcodegen.SqlCodegen.Instruction;
import com.codingadventures.sqlcodegen.SqlCodegen.NullsOrder;
import com.codingadventures.sqlcodegen.SqlCodegen.Program;
import com.codingadventures.sqlcodegen.SqlCodegen.SortKey;
import com.codingadventures.sqlplanner.SqlPlanner;
import com.codingadventures.sqlvm.SqlVm.QueryResult;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.stream.Collectors;

import static org.junit.jupiter.api.Assertions.*;

class SqlVmTest {

    // ── Test fixture ──────────────────────────────────────────────────────────
    //
    // A fresh InMemoryBackend is created before each test.
    // Helper methods build the pre-populated tables used by multiple tests.

    private InMemoryBackend backend;

    @BeforeEach
    void setUp() {
        backend = new InMemoryBackend();
        createUsersTable();
        createOrdersTable();
        createNullsTable();
    }

    // Populate the `users` table with 4 rows.
    private void createUsersTable() {
        backend.createTable("users", List.of(
            new ColumnDef("id", "INTEGER"),
            new ColumnDef("name", "TEXT"),
            new ColumnDef("age", "INTEGER"),
            new ColumnDef("city", "TEXT")
        ), false);
        backend.insert("users", row("id", 1L, "name", "Alice", "age", 30L, "city", "NYC"));
        backend.insert("users", row("id", 2L, "name", "Bob",   "age", 25L, "city", "LA"));
        backend.insert("users", row("id", 3L, "name", "Carol", "age", 35L, "city", "NYC"));
        backend.insert("users", row("id", 4L, "name", "Dave",  "age", 28L, "city", "LA"));
    }

    // Populate the `orders` table with 3 rows.
    private void createOrdersTable() {
        backend.createTable("orders", List.of(
            new ColumnDef("order_id", "INTEGER"),
            new ColumnDef("user_id", "INTEGER"),
            new ColumnDef("amount", "REAL")
        ), false);
        backend.insert("orders", row("order_id", 1L, "user_id", 1L, "amount", 100.0));
        backend.insert("orders", row("order_id", 2L, "user_id", 2L, "amount", 200.0));
        backend.insert("orders", row("order_id", 3L, "user_id", 1L, "amount", 150.0));
    }

    // Populate the `nulls` table with mixed-NULL data.
    private void createNullsTable() {
        backend.createTable("nulls", List.of(
            new ColumnDef("x", "INTEGER"),
            new ColumnDef("y", "INTEGER")
        ), false);
        backend.insert("nulls", row("x", 1L, "y", null));
        backend.insert("nulls", row("x", null, "y", 2L));
        backend.insert("nulls", row("x", 3L, "y", 4L));
    }

    // ── Test 1: SELECT * (all rows) ───────────────────────────────────────────

    @Test
    void testSelectAllRows() {
        // SELECT id, name FROM users
        //
        // Bytecode:
        //   SetResultSchema [id, name]
        //   OpenScan 0 users
        //   Label loop
        //   AdvanceCursor 0 done
        //   BeginRow
        //   LoadColumn 0 id  → EmitColumn id
        //   LoadColumn 0 name → EmitColumn name
        //   EmitRow
        //   Jump loop
        //   Label done
        //   CloseScan 0
        //   Halt

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("id", "name")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            new Instruction.BeginRow(),
            new Instruction.LoadColumn(0, "id"),
            new Instruction.EmitColumn("id"),
            new Instruction.LoadColumn(0, "name"),
            new Instruction.EmitColumn("name"),
            new Instruction.EmitRow(),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(List.of("id", "name"), result.columns());
        assertEquals(4, result.rows().size());
        assertEquals(List.of(1L, "Alice"), result.rows().get(0));
        assertEquals(List.of(2L, "Bob"),   result.rows().get(1));
        assertEquals(List.of(3L, "Carol"), result.rows().get(2));
        assertEquals(List.of(4L, "Dave"),  result.rows().get(3));
        assertEquals(0, result.rowsAffected());
    }

    // ── Test 2: SELECT with WHERE filter ─────────────────────────────────────

    @Test
    void testSelectWithWhereFilter() {
        // SELECT id, name FROM users WHERE city = 'NYC'
        //
        // Extra instructions after AdvanceCursor:
        //   LoadColumn 0 city
        //   LoadConst 'NYC'
        //   BinaryOp EQ
        //   JumpIfFalse loop   ← skip row if condition is false

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("id", "name")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            // WHERE city = 'NYC'
            new Instruction.LoadColumn(0, "city"),
            new Instruction.LoadConst("NYC"),
            new Instruction.BinaryOp(BinaryOpCode.EQ),
            new Instruction.JumpIfFalse("loop"),
            // Emit matching row
            new Instruction.BeginRow(),
            new Instruction.LoadColumn(0, "id"),
            new Instruction.EmitColumn("id"),
            new Instruction.LoadColumn(0, "name"),
            new Instruction.EmitColumn("name"),
            new Instruction.EmitRow(),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(2, result.rows().size());
        // Alice (NYC) and Carol (NYC) should match.
        assertEquals(List.of(1L, "Alice"), result.rows().get(0));
        assertEquals(List.of(3L, "Carol"), result.rows().get(1));
    }

    // ── Test 3: SELECT COUNT(*) ───────────────────────────────────────────────

    @Test
    void testSelectCountStar() {
        // SELECT COUNT(*) FROM users
        //
        // Aggregate pattern:
        //   scan loop → InitAgg slot=0 COUNT_STAR
        //               LoadConst NULL  (ignored by COUNT_STAR)
        //               UpdateAgg slot=0
        //   emit loop → AdvanceGroupKey onExhausted=done hasGroupBy=false
        //               FinalizeAgg slot=0 COUNT_STAR
        //               BeginRow / EmitColumn / EmitRow
        //               Jump emit_loop
        //   done

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("count")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("scan_loop"),
            new Instruction.AdvanceCursor(0, "scan_done"),
            new Instruction.InitAgg(0, AggFunc.COUNT_STAR, false),
            new Instruction.LoadConst(null),
            new Instruction.UpdateAgg(0),
            new Instruction.Jump("scan_loop"),
            new Instruction.Label("scan_done"),
            new Instruction.CloseScan(0),
            // emit loop
            new Instruction.Label("emit_loop"),
            new Instruction.AdvanceGroupKey("done", false),
            new Instruction.FinalizeAgg(0, AggFunc.COUNT_STAR),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("count"),
            new Instruction.EmitRow(),
            new Instruction.Jump("emit_loop"),
            new Instruction.Label("done"),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(1, result.rows().size());
        assertEquals(4L, result.rows().get(0).get(0));
    }

    // ── Test 4: SELECT SUM ────────────────────────────────────────────────────

    @Test
    void testSelectSum() {
        // SELECT SUM(amount) FROM orders

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("total")),
            new Instruction.OpenScan(0, "orders"),
            new Instruction.Label("scan_loop"),
            new Instruction.AdvanceCursor(0, "scan_done"),
            new Instruction.InitAgg(0, AggFunc.SUM, false),
            new Instruction.LoadColumn(0, "amount"),
            new Instruction.UpdateAgg(0),
            new Instruction.Jump("scan_loop"),
            new Instruction.Label("scan_done"),
            new Instruction.CloseScan(0),
            new Instruction.Label("emit_loop"),
            new Instruction.AdvanceGroupKey("done", false),
            new Instruction.FinalizeAgg(0, AggFunc.SUM),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("total"),
            new Instruction.EmitRow(),
            new Instruction.Jump("emit_loop"),
            new Instruction.Label("done"),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(1, result.rows().size());
        assertEquals(450.0, (double) result.rows().get(0).get(0), 1e-9);
    }

    // ── Test 5: SELECT AVG ────────────────────────────────────────────────────

    @Test
    void testSelectAvg() {
        // SELECT AVG(age) FROM users → (30+25+35+28)/4 = 29.5

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("avg_age")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("scan_loop"),
            new Instruction.AdvanceCursor(0, "scan_done"),
            new Instruction.InitAgg(0, AggFunc.AVG, false),
            new Instruction.LoadColumn(0, "age"),
            new Instruction.UpdateAgg(0),
            new Instruction.Jump("scan_loop"),
            new Instruction.Label("scan_done"),
            new Instruction.CloseScan(0),
            new Instruction.Label("emit_loop"),
            new Instruction.AdvanceGroupKey("done", false),
            new Instruction.FinalizeAgg(0, AggFunc.AVG),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("avg_age"),
            new Instruction.EmitRow(),
            new Instruction.Jump("emit_loop"),
            new Instruction.Label("done"),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(29.5, (double) result.rows().get(0).get(0), 1e-9);
    }

    // ── Test 6: SELECT MIN and MAX ────────────────────────────────────────────

    @Test
    void testSelectMinMax() {
        // SELECT MIN(age), MAX(age) FROM users → 25, 35

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("min_age", "max_age")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("scan_loop"),
            new Instruction.AdvanceCursor(0, "scan_done"),
            new Instruction.InitAgg(0, AggFunc.MIN, false),
            new Instruction.LoadColumn(0, "age"),
            new Instruction.UpdateAgg(0),
            new Instruction.InitAgg(1, AggFunc.MAX, false),
            new Instruction.LoadColumn(0, "age"),
            new Instruction.UpdateAgg(1),
            new Instruction.Jump("scan_loop"),
            new Instruction.Label("scan_done"),
            new Instruction.CloseScan(0),
            new Instruction.Label("emit_loop"),
            new Instruction.AdvanceGroupKey("done", false),
            // Each FinalizeAgg pushes its result; each EmitColumn immediately
            // pops it.  Interleaving prevents ordering confusion on the stack.
            new Instruction.BeginRow(),
            new Instruction.FinalizeAgg(0, AggFunc.MIN),
            new Instruction.EmitColumn("min_age"),
            new Instruction.FinalizeAgg(1, AggFunc.MAX),
            new Instruction.EmitColumn("max_age"),
            new Instruction.EmitRow(),
            new Instruction.Jump("emit_loop"),
            new Instruction.Label("done"),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(25L, result.rows().get(0).get(0));
        assertEquals(35L, result.rows().get(0).get(1));
    }

    // ── Test 7: SELECT with ORDER BY ─────────────────────────────────────────

    @Test
    void testSelectOrderBy() {
        // SELECT name FROM users ORDER BY age ASC

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("name", "age")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            new Instruction.BeginRow(),
            new Instruction.LoadColumn(0, "name"),
            new Instruction.EmitColumn("name"),
            new Instruction.LoadColumn(0, "age"),
            new Instruction.EmitColumn("age"),
            new Instruction.EmitRow(),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.SortResult(List.of(
                new SortKey("age", Direction.ASC, NullsOrder.LAST)
            )),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        // Expected order: Bob(25), Dave(28), Alice(30), Carol(35)
        List<Object> names = result.rows().stream()
            .map(r -> r.get(0))
            .collect(Collectors.toList());
        assertEquals(List.of("Bob", "Dave", "Alice", "Carol"), names);
    }

    // ── Test 8: ORDER BY DESC ─────────────────────────────────────────────────

    @Test
    void testSelectOrderByDesc() {
        // SELECT name FROM users ORDER BY age DESC

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("name", "age")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            new Instruction.BeginRow(),
            new Instruction.LoadColumn(0, "name"),
            new Instruction.EmitColumn("name"),
            new Instruction.LoadColumn(0, "age"),
            new Instruction.EmitColumn("age"),
            new Instruction.EmitRow(),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.SortResult(List.of(
                new SortKey("age", Direction.DESC, NullsOrder.LAST)
            )),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        List<Object> names = result.rows().stream()
            .map(r -> r.get(0))
            .collect(Collectors.toList());
        assertEquals(List.of("Carol", "Alice", "Dave", "Bob"), names);
    }

    // ── Test 9: SELECT with LIMIT ─────────────────────────────────────────────

    @Test
    void testSelectLimit() {
        // SELECT id FROM users LIMIT 2

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("id")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            new Instruction.BeginRow(),
            new Instruction.LoadColumn(0, "id"),
            new Instruction.EmitColumn("id"),
            new Instruction.EmitRow(),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.LimitResult(2L, null),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(2, result.rows().size());
        assertEquals(1L, result.rows().get(0).get(0));
        assertEquals(2L, result.rows().get(1).get(0));
    }

    // ── Test 10: SELECT with OFFSET ───────────────────────────────────────────

    @Test
    void testSelectOffset() {
        // SELECT id FROM users LIMIT 2 OFFSET 2

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("id")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            new Instruction.BeginRow(),
            new Instruction.LoadColumn(0, "id"),
            new Instruction.EmitColumn("id"),
            new Instruction.EmitRow(),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.LimitResult(2L, 2L),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(2, result.rows().size());
        assertEquals(3L, result.rows().get(0).get(0));
        assertEquals(4L, result.rows().get(1).get(0));
    }

    // ── Test 11: SELECT DISTINCT ──────────────────────────────────────────────

    @Test
    void testSelectDistinct() {
        // SELECT DISTINCT city FROM users → NYC, LA

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("city")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            new Instruction.BeginRow(),
            new Instruction.LoadColumn(0, "city"),
            new Instruction.EmitColumn("city"),
            new Instruction.EmitRow(),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.DistinctResult(),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(2, result.rows().size());
        // Insertion order: Alice/NYC first, then Bob/LA.
        assertEquals("NYC", result.rows().get(0).get(0));
        assertEquals("LA",  result.rows().get(1).get(0));
    }

    // ── Test 12: INSERT → row added ───────────────────────────────────────────

    @Test
    void testInsertRow() {
        // INSERT INTO users (id, name, age, city) VALUES (5, 'Eve', 22, 'Chicago')

        Program prog = buildProgram(List.of(
            new Instruction.LoadConst(5L),
            new Instruction.LoadConst("Eve"),
            new Instruction.LoadConst(22L),
            new Instruction.LoadConst("Chicago"),
            new Instruction.InsertRow("users", List.of("id", "name", "age", "city")),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(1, result.rowsAffected());
        // Verify row was actually inserted.
        assertEquals(5, countRows("users"));
    }

    // ── Test 13: UPDATE with WHERE ────────────────────────────────────────────

    @Test
    void testUpdateWithWhere() {
        // UPDATE users SET age = 99 WHERE name = 'Alice'

        Program prog = buildProgram(List.of(
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            new Instruction.LoadColumn(0, "name"),
            new Instruction.LoadConst("Alice"),
            new Instruction.BinaryOp(BinaryOpCode.EQ),
            new Instruction.JumpIfFalse("loop"),
            new Instruction.LoadConst(99L),
            new Instruction.UpdateRows("users", List.of("age"), 0),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(1, result.rowsAffected());
        // Verify age was updated.
        Long alice_age = readValue("users", "name", "Alice", "age");
        assertEquals(99L, alice_age);
    }

    // ── Test 14: DELETE with WHERE ────────────────────────────────────────────

    @Test
    void testDeleteWithWhere() {
        // DELETE FROM users WHERE city = 'LA'  → removes Bob and Dave

        Program prog = buildProgram(List.of(
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            new Instruction.LoadColumn(0, "city"),
            new Instruction.LoadConst("LA"),
            new Instruction.BinaryOp(BinaryOpCode.EQ),
            new Instruction.JumpIfFalse("loop"),
            new Instruction.DeleteRows("users", 0),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(2, result.rowsAffected());
        assertEquals(2, countRows("users"));
    }

    // ── Test 15: CREATE TABLE ─────────────────────────────────────────────────

    @Test
    void testCreateTable() {
        Program prog = buildProgram(List.of(
            new Instruction.CreateTable("products", false, List.of(
                new SqlPlanner.ColumnDef("sku", "TEXT", true, false, false, null),
                new SqlPlanner.ColumnDef("price", "REAL", false, false, false, null)
            )),
            new Instruction.Halt()
        ));

        SqlVm.execute(prog, backend);
        assertTrue(backend.tables().contains("products"));
        assertEquals(2, backend.columns("products").size());
    }

    // ── Test 16: CREATE TABLE IF NOT EXISTS ───────────────────────────────────

    @Test
    void testCreateTableIfNotExists() {
        // Should not throw even though users already exists.
        Program prog = buildProgram(List.of(
            new Instruction.CreateTable("users", true, List.of(
                new SqlPlanner.ColumnDef("id", "INTEGER", false, false, false, null)
            )),
            new Instruction.Halt()
        ));
        assertDoesNotThrow(() -> SqlVm.execute(prog, backend));
    }

    // ── Test 17: DROP TABLE ───────────────────────────────────────────────────

    @Test
    void testDropTable() {
        Program prog = buildProgram(List.of(
            new Instruction.DropTable("orders", false),
            new Instruction.Halt()
        ));

        SqlVm.execute(prog, backend);
        assertFalse(backend.tables().contains("orders"));
    }

    // ── Test 18: DROP TABLE IF EXISTS ────────────────────────────────────────

    @Test
    void testDropTableIfExists() {
        // Dropping a non-existent table with IF EXISTS should not throw.
        Program prog = buildProgram(List.of(
            new Instruction.DropTable("nonexistent", true),
            new Instruction.Halt()
        ));
        assertDoesNotThrow(() -> SqlVm.execute(prog, backend));
    }

    // ── Test 19: NULL arithmetic propagation ─────────────────────────────────

    @Test
    void testNullArithmeticPropagation() {
        // SELECT x + y FROM nulls
        // Row 1: 1 + NULL = NULL
        // Row 2: NULL + 2 = NULL
        // Row 3: 3 + 4 = 7

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("sum")),
            new Instruction.OpenScan(0, "nulls"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            new Instruction.BeginRow(),
            new Instruction.LoadColumn(0, "x"),
            new Instruction.LoadColumn(0, "y"),
            new Instruction.BinaryOp(BinaryOpCode.ADD),
            new Instruction.EmitColumn("sum"),
            new Instruction.EmitRow(),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(3, result.rows().size());
        assertNull(result.rows().get(0).get(0));  // 1 + NULL = NULL
        assertNull(result.rows().get(1).get(0));  // NULL + 2 = NULL
        assertEquals(7L, result.rows().get(2).get(0)); // 3 + 4 = 7
    }

    // ── Test 20: IS NULL ──────────────────────────────────────────────────────

    @Test
    void testIsNull() {
        // SELECT x, x IS NULL FROM nulls

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("x", "x_is_null")),
            new Instruction.OpenScan(0, "nulls"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            new Instruction.BeginRow(),
            new Instruction.LoadColumn(0, "x"),
            new Instruction.EmitColumn("x"),
            new Instruction.LoadColumn(0, "x"),
            new Instruction.IsNull(),
            new Instruction.EmitColumn("x_is_null"),
            new Instruction.EmitRow(),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(false, result.rows().get(0).get(1)); // x=1 IS NULL → false
        assertEquals(true,  result.rows().get(1).get(1)); // x=NULL IS NULL → true
        assertEquals(false, result.rows().get(2).get(1)); // x=3 IS NULL → false
    }

    // ── Test 21: IS NOT NULL ─────────────────────────────────────────────────

    @Test
    void testIsNotNull() {
        // WHERE x IS NOT NULL → rows with x=1 and x=3

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("x")),
            new Instruction.OpenScan(0, "nulls"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            new Instruction.LoadColumn(0, "x"),
            new Instruction.IsNotNull(),
            new Instruction.JumpIfFalse("loop"),
            new Instruction.BeginRow(),
            new Instruction.LoadColumn(0, "x"),
            new Instruction.EmitColumn("x"),
            new Instruction.EmitRow(),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(2, result.rows().size());
        assertEquals(1L, result.rows().get(0).get(0));
        assertEquals(3L, result.rows().get(1).get(0));
    }

    // ── Test 22: LIKE — percent wildcard ─────────────────────────────────────

    @Test
    void testLikePercentWildcard() {
        // SELECT name FROM users WHERE name LIKE 'A%'  → Alice

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("name")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            new Instruction.LoadColumn(0, "name"),
            new Instruction.LoadConst("A%"),
            new Instruction.Like(false),
            new Instruction.JumpIfFalse("loop"),
            new Instruction.BeginRow(),
            new Instruction.LoadColumn(0, "name"),
            new Instruction.EmitColumn("name"),
            new Instruction.EmitRow(),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(1, result.rows().size());
        assertEquals("Alice", result.rows().get(0).get(0));
    }

    // ── Test 23: LIKE — underscore wildcard ──────────────────────────────────

    @Test
    void testLikeUnderscoreWildcard() {
        // WHERE name LIKE 'Bo_'  → Bob

        assertTrue(SqlVm.likeMatch("Bob", "Bo_"));
        assertFalse(SqlVm.likeMatch("Bobby", "Bo_"));
    }

    // ── Test 24: NOT LIKE ─────────────────────────────────────────────────────

    @Test
    void testNotLike() {
        // SELECT name FROM users WHERE name NOT LIKE '%o%'
        // Excludes Bob (contains o) and Carol (contains o).

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("name")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            new Instruction.LoadColumn(0, "name"),
            new Instruction.LoadConst("%o%"),
            new Instruction.Like(true),  // negated=true → NOT LIKE
            new Instruction.JumpIfFalse("loop"),
            new Instruction.BeginRow(),
            new Instruction.LoadColumn(0, "name"),
            new Instruction.EmitColumn("name"),
            new Instruction.EmitRow(),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        // Alice and Dave don't contain 'o'
        assertEquals(2, result.rows().size());
        List<Object> names = result.rows().stream().map(r -> r.get(0)).collect(Collectors.toList());
        assertTrue(names.contains("Alice"));
        assertTrue(names.contains("Dave"));
    }

    // ── Test 25: BETWEEN ─────────────────────────────────────────────────────

    @Test
    void testBetween() {
        // SELECT name FROM users WHERE age BETWEEN 28 AND 32

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("name")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            // Stack: age, 28, 32 — Between pops high, low, value
            new Instruction.LoadColumn(0, "age"),
            new Instruction.LoadConst(28L),
            new Instruction.LoadConst(32L),
            new Instruction.Between(),
            new Instruction.JumpIfFalse("loop"),
            new Instruction.BeginRow(),
            new Instruction.LoadColumn(0, "name"),
            new Instruction.EmitColumn("name"),
            new Instruction.EmitRow(),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        // Alice(30) and Dave(28) match; Bob(25) and Carol(35) do not.
        assertEquals(2, result.rows().size());
        List<Object> names = result.rows().stream().map(r -> r.get(0)).collect(Collectors.toList());
        assertTrue(names.contains("Alice"));
        assertTrue(names.contains("Dave"));
    }

    // ── Test 26: IN list ─────────────────────────────────────────────────────

    @Test
    void testInList() {
        // SELECT name FROM users WHERE city IN ('NYC', 'Chicago')

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("name")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            new Instruction.LoadColumn(0, "city"),
            new Instruction.LoadConst("NYC"),
            new Instruction.LoadConst("Chicago"),
            new Instruction.InList(2),
            new Instruction.JumpIfFalse("loop"),
            new Instruction.BeginRow(),
            new Instruction.LoadColumn(0, "name"),
            new Instruction.EmitColumn("name"),
            new Instruction.EmitRow(),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        // Alice and Carol are in NYC.
        assertEquals(2, result.rows().size());
    }

    // ── Test 27: Empty IN list → always false ────────────────────────────────

    @Test
    void testEmptyInList() {
        // x IN () → always false
        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("id")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            new Instruction.LoadColumn(0, "id"),
            new Instruction.InList(0), // empty IN list
            new Instruction.JumpIfFalse("loop"),
            new Instruction.BeginRow(),
            new Instruction.LoadColumn(0, "id"),
            new Instruction.EmitColumn("id"),
            new Instruction.EmitRow(),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(0, result.rows().size());
    }

    // ── Test 28: Empty table aggregates ──────────────────────────────────────

    @Test
    void testEmptyTableAggregates() {
        // Create an empty table, then SELECT COUNT(*), SUM(x) FROM empty
        backend.createTable("empty_t", List.of(new ColumnDef("x", "INTEGER")), false);

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("cnt", "total")),
            new Instruction.OpenScan(0, "empty_t"),
            new Instruction.Label("scan_loop"),
            new Instruction.AdvanceCursor(0, "scan_done"),
            new Instruction.InitAgg(0, AggFunc.COUNT_STAR, false),
            new Instruction.LoadConst(null),
            new Instruction.UpdateAgg(0),
            new Instruction.InitAgg(1, AggFunc.SUM, false),
            new Instruction.LoadColumn(0, "x"),
            new Instruction.UpdateAgg(1),
            new Instruction.Jump("scan_loop"),
            new Instruction.Label("scan_done"),
            new Instruction.CloseScan(0),
            new Instruction.Label("emit_loop"),
            new Instruction.AdvanceGroupKey("done", false),
            new Instruction.BeginRow(),
            new Instruction.FinalizeAgg(0, AggFunc.COUNT_STAR),
            new Instruction.EmitColumn("cnt"),
            new Instruction.FinalizeAgg(1, AggFunc.SUM),
            new Instruction.EmitColumn("total"),
            new Instruction.EmitRow(),
            new Instruction.Jump("emit_loop"),
            new Instruction.Label("done"),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(1, result.rows().size());
        // COUNT(*) over empty table → 0.
        assertEquals(0L, result.rows().get(0).get(0));
        // SUM(x) over empty table → NULL (SQL standard).
        assertNull(result.rows().get(0).get(1));
    }

    // ── Test 29: COUNT(col) — skips NULLs ────────────────────────────────────

    @Test
    void testCountColumnSkipsNulls() {
        // SELECT COUNT(x) FROM nulls → 2 (x=NULL is skipped)

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("cnt")),
            new Instruction.OpenScan(0, "nulls"),
            new Instruction.Label("scan_loop"),
            new Instruction.AdvanceCursor(0, "scan_done"),
            new Instruction.InitAgg(0, AggFunc.COUNT, false),
            new Instruction.LoadColumn(0, "x"),
            new Instruction.UpdateAgg(0),
            new Instruction.Jump("scan_loop"),
            new Instruction.Label("scan_done"),
            new Instruction.CloseScan(0),
            new Instruction.Label("emit_loop"),
            new Instruction.AdvanceGroupKey("done", false),
            new Instruction.FinalizeAgg(0, AggFunc.COUNT),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("cnt"),
            new Instruction.EmitRow(),
            new Instruction.Jump("emit_loop"),
            new Instruction.Label("done"),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(2L, result.rows().get(0).get(0));
    }

    // ── Test 30: Three-valued logic — AND/OR with NULL ────────────────────────

    @Test
    void testThreeValuedLogicAnd() {
        // NULL AND FALSE → FALSE (short-circuit)
        // NULL AND TRUE  → NULL
        // NULL OR  TRUE  → TRUE (short-circuit)
        // NULL OR  FALSE → NULL

        // We test via a SELECT WHERE clause that mixes NULL booleans.
        // NULL AND FALSE (= false) → row excluded.
        // First, test the AND/OR operator helper directly via binary ops.

        Program andFalse = buildProgram(List.of(
            new Instruction.LoadConst(null),
            new Instruction.LoadConst(false),
            new Instruction.BinaryOp(BinaryOpCode.AND),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("result"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult r1 = SqlVm.execute(andFalse, backend);
        assertEquals(false, r1.rows().get(0).get(0)); // NULL AND FALSE = FALSE

        Program andTrue = buildProgram(List.of(
            new Instruction.LoadConst(null),
            new Instruction.LoadConst(true),
            new Instruction.BinaryOp(BinaryOpCode.AND),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("result"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult r2 = SqlVm.execute(andTrue, backend);
        assertNull(r2.rows().get(0).get(0)); // NULL AND TRUE = NULL
    }

    @Test
    void testThreeValuedLogicOr() {
        // NULL OR TRUE → TRUE
        Program orTrue = buildProgram(List.of(
            new Instruction.LoadConst(null),
            new Instruction.LoadConst(true),
            new Instruction.BinaryOp(BinaryOpCode.OR),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("result"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult r1 = SqlVm.execute(orTrue, backend);
        assertEquals(true, r1.rows().get(0).get(0));

        // NULL OR FALSE → NULL
        Program orFalse = buildProgram(List.of(
            new Instruction.LoadConst(null),
            new Instruction.LoadConst(false),
            new Instruction.BinaryOp(BinaryOpCode.OR),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("result"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult r2 = SqlVm.execute(orFalse, backend);
        assertNull(r2.rows().get(0).get(0));
    }

    // ── Test 31: Unary NEG ────────────────────────────────────────────────────

    @Test
    void testUnaryNeg() {
        Program prog = buildProgram(List.of(
            new Instruction.LoadConst(42L),
            new Instruction.UnaryOp(SqlCodegen.UnaryOpCode.NEG),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(-42L, result.rows().get(0).get(0));
    }

    // ── Test 32: Unary NOT ────────────────────────────────────────────────────

    @Test
    void testUnaryNot() {
        Program prog = buildProgram(List.of(
            new Instruction.LoadConst(true),
            new Instruction.UnaryOp(SqlCodegen.UnaryOpCode.NOT),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(false, result.rows().get(0).get(0));
    }

    // ── Test 33: BinaryOp CONCAT ─────────────────────────────────────────────

    @Test
    void testBinaryConcat() {
        Program prog = buildProgram(List.of(
            new Instruction.LoadConst("Hello"),
            new Instruction.LoadConst(", World"),
            new Instruction.BinaryOp(BinaryOpCode.CONCAT),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals("Hello, World", result.rows().get(0).get(0));
    }

    // ── Test 34: Arithmetic operators ────────────────────────────────────────

    @Test
    void testArithmeticOperators() {
        assertBinaryOp(10L, BinaryOpCode.ADD, 3L, 13L);
        assertBinaryOp(10L, BinaryOpCode.SUB, 3L, 7L);
        assertBinaryOp(10L, BinaryOpCode.MUL, 3L, 30L);
        assertBinaryOp(10L, BinaryOpCode.DIV, 3L, 3L);  // integer division
        assertBinaryOp(10L, BinaryOpCode.MOD, 3L, 1L);
    }

    @Test
    void testDivisionByZeroReturnsNull() {
        Program prog = buildProgram(List.of(
            new Instruction.LoadConst(5L),
            new Instruction.LoadConst(0L),
            new Instruction.BinaryOp(BinaryOpCode.DIV),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result = SqlVm.execute(prog, backend);
        assertNull(result.rows().get(0).get(0));
    }

    // ── Test 35: Comparison operators ────────────────────────────────────────

    @Test
    void testComparisonOperators() {
        assertBinaryOp(3L, BinaryOpCode.LT, 5L, true);
        assertBinaryOp(5L, BinaryOpCode.LT, 3L, false);
        assertBinaryOp(3L, BinaryOpCode.LTE, 3L, true);
        assertBinaryOp(3L, BinaryOpCode.GT, 5L, false);
        assertBinaryOp(5L, BinaryOpCode.GTE, 5L, true);
        assertBinaryOp(3L, BinaryOpCode.EQ, 3L, true);
        assertBinaryOp(3L, BinaryOpCode.NEQ, 5L, true);
    }

    // ── Test 36: JumpIfTrue ───────────────────────────────────────────────────

    @Test
    void testJumpIfTrue() {
        // Push true, JumpIfTrue to "target", push 99 (should be skipped), target: push 42.
        Program prog = buildProgram(List.of(
            new Instruction.LoadConst(true),
            new Instruction.JumpIfTrue("target"),
            new Instruction.LoadConst(99L),  // should be skipped
            new Instruction.Label("target"),
            new Instruction.LoadConst(42L),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result = SqlVm.execute(prog, backend);
        // Only 42 should be on the stack (99 was skipped).
        assertEquals(42L, result.rows().get(0).get(0));
    }

    // ── Test 37: Halt terminates early ───────────────────────────────────────

    @Test
    void testHaltTerminatesEarly() {
        // The program emits a row, then hits Halt before scanning more rows.
        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("id")),
            new Instruction.LoadConst(999L),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("id"),
            new Instruction.EmitRow(),
            new Instruction.Halt(),
            // Instructions below should never execute.
            new Instruction.LoadConst(0L),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("id"),
            new Instruction.EmitRow()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(1, result.rows().size());
        assertEquals(999L, result.rows().get(0).get(0));
    }

    // ── Test 38: Pop instruction ──────────────────────────────────────────────

    @Test
    void testPopInstruction() {
        // Push 1, push 2, pop 2, emit 1.
        Program prog = buildProgram(List.of(
            new Instruction.LoadConst(1L),
            new Instruction.LoadConst(2L),
            new Instruction.Pop(),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(1L, result.rows().get(0).get(0));
    }

    // ── Test 39: BETWEEN with NULL → NULL ────────────────────────────────────

    @Test
    void testBetweenWithNull() {
        // NULL BETWEEN 1 AND 10 → NULL
        Program prog = buildProgram(List.of(
            new Instruction.LoadConst(null),
            new Instruction.LoadConst(1L),
            new Instruction.LoadConst(10L),
            new Instruction.Between(),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result = SqlVm.execute(prog, backend);
        assertNull(result.rows().get(0).get(0));
    }

    // ── Test 40: LIKE with NULL → NULL ───────────────────────────────────────

    @Test
    void testLikeWithNull() {
        // NULL LIKE 'A%' → NULL
        Program prog = buildProgram(List.of(
            new Instruction.LoadConst(null),
            new Instruction.LoadConst("A%"),
            new Instruction.Like(false),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result = SqlVm.execute(prog, backend);
        assertNull(result.rows().get(0).get(0));
    }

    // ── Test 41: LEFT JOIN tracking (JoinBeginRow / JoinSetMatched / JoinIfMatched) ──

    @Test
    void testJoinInstructions() {
        // Simulate a LEFT JOIN where the join match tracking is used.
        // We push false (unmatched), then set it to matched (JoinSetMatched),
        // then JoinIfMatched jumps to "matched" label.

        Program prog = buildProgram(List.of(
            new Instruction.JoinBeginRow(),
            new Instruction.JoinSetMatched(),
            new Instruction.JoinIfMatched("yes"),
            // Not-matched path: push 0 and jump to done.
            new Instruction.LoadConst(0L),
            new Instruction.Jump("done"),
            // Matched path:
            new Instruction.Label("yes"),
            new Instruction.LoadConst(1L),
            new Instruction.Label("done"),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(1L, result.rows().get(0).get(0)); // matched path taken
    }

    // ── Test 42: JoinIfMatched when no match ─────────────────────────────────

    @Test
    void testJoinNoMatch() {
        // JoinBeginRow (false), JoinIfMatched should NOT jump → fall-through to 0.
        Program prog = buildProgram(List.of(
            new Instruction.JoinBeginRow(),
            // No JoinSetMatched — left row has no match
            new Instruction.JoinIfMatched("yes"),
            new Instruction.LoadConst(0L),
            new Instruction.Jump("done"),
            new Instruction.Label("yes"),
            new Instruction.LoadConst(1L),
            new Instruction.Label("done"),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(0L, result.rows().get(0).get(0)); // not-matched path taken
    }

    // ── Test 43: CallScalar — ABS ────────────────────────────────────────────

    @Test
    void testCallScalarAbs() {
        Program prog = buildProgram(List.of(
            new Instruction.LoadConst(-7L),
            new Instruction.CallScalar("abs", 1),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(7L, result.rows().get(0).get(0));
    }

    // ── Test 44: CallScalar — LENGTH ─────────────────────────────────────────

    @Test
    void testCallScalarLength() {
        Program prog = buildProgram(List.of(
            new Instruction.LoadConst("hello"),
            new Instruction.CallScalar("length", 1),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(5L, result.rows().get(0).get(0));
    }

    // ── Test 45: CallScalar — UPPER / LOWER ──────────────────────────────────

    @Test
    void testCallScalarUpperLower() {
        Program prog = buildProgram(List.of(
            new Instruction.LoadConst("hello"),
            new Instruction.CallScalar("upper", 1),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals("HELLO", result.rows().get(0).get(0));
    }

    // ── Test 46: CallScalar — COALESCE ───────────────────────────────────────

    @Test
    void testCallScalarCoalesce() {
        Program prog = buildProgram(List.of(
            new Instruction.LoadConst(null),
            new Instruction.LoadConst(null),
            new Instruction.LoadConst(42L),
            new Instruction.CallScalar("coalesce", 3),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(42L, result.rows().get(0).get(0));
    }

    // ── Test 47: MIN over all-NULL column → NULL ──────────────────────────────

    @Test
    void testMinOverAllNulls() {
        backend.createTable("allnull", List.of(new ColumnDef("v", "INTEGER")), false);
        backend.insert("allnull", row("v", null));
        backend.insert("allnull", row("v", null));

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("mn")),
            new Instruction.OpenScan(0, "allnull"),
            new Instruction.Label("sl"),
            new Instruction.AdvanceCursor(0, "sd"),
            new Instruction.InitAgg(0, AggFunc.MIN, false),
            new Instruction.LoadColumn(0, "v"),
            new Instruction.UpdateAgg(0),
            new Instruction.Jump("sl"),
            new Instruction.Label("sd"),
            new Instruction.CloseScan(0),
            new Instruction.Label("el"),
            new Instruction.AdvanceGroupKey("done", false),
            new Instruction.FinalizeAgg(0, AggFunc.MIN),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("mn"),
            new Instruction.EmitRow(),
            new Instruction.Jump("el"),
            new Instruction.Label("done"),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertNull(result.rows().get(0).get(0));
    }

    // ── Test 48: Multiple aggregate slots ────────────────────────────────────

    @Test
    void testMultipleAggregateSlots() {
        // SELECT COUNT(*), MIN(age), MAX(age) FROM users

        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("cnt", "min_age", "max_age")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("sl"),
            new Instruction.AdvanceCursor(0, "sd"),
            new Instruction.InitAgg(0, AggFunc.COUNT_STAR, false),
            new Instruction.LoadConst(null),
            new Instruction.UpdateAgg(0),
            new Instruction.InitAgg(1, AggFunc.MIN, false),
            new Instruction.LoadColumn(0, "age"),
            new Instruction.UpdateAgg(1),
            new Instruction.InitAgg(2, AggFunc.MAX, false),
            new Instruction.LoadColumn(0, "age"),
            new Instruction.UpdateAgg(2),
            new Instruction.Jump("sl"),
            new Instruction.Label("sd"),
            new Instruction.CloseScan(0),
            new Instruction.Label("el"),
            new Instruction.AdvanceGroupKey("done", false),
            new Instruction.BeginRow(),
            new Instruction.FinalizeAgg(0, AggFunc.COUNT_STAR),
            new Instruction.EmitColumn("cnt"),
            new Instruction.FinalizeAgg(1, AggFunc.MIN),
            new Instruction.EmitColumn("min_age"),
            new Instruction.FinalizeAgg(2, AggFunc.MAX),
            new Instruction.EmitColumn("max_age"),
            new Instruction.EmitRow(),
            new Instruction.Jump("el"),
            new Instruction.Label("done"),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(4L, result.rows().get(0).get(0));
        assertEquals(25L, result.rows().get(0).get(1));
        assertEquals(35L, result.rows().get(0).get(2));
    }

    // ── Test 49: Floating-point arithmetic ───────────────────────────────────

    @Test
    void testFloatingPointArithmetic() {
        Program prog = buildProgram(List.of(
            new Instruction.LoadConst(10.0),
            new Instruction.LoadConst(3.0),
            new Instruction.BinaryOp(BinaryOpCode.DIV),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(10.0 / 3.0, (double) result.rows().get(0).get(0), 1e-9);
    }

    // ── Test 50: LIKE pattern with regex metacharacters ──────────────────────

    @Test
    void testLikePatternWithRegexMetachars() {
        // The LIKE pattern "3.14" should NOT match "3X14" because . is a literal.
        assertFalse(SqlVm.likeMatch("3X14", "3.14"));
        // But "3.14" should match "3.14".
        assertTrue(SqlVm.likeMatch("3.14", "3.14"));
        // A percent-only pattern matches anything.
        assertTrue(SqlVm.likeMatch("anything", "%"));
        assertTrue(SqlVm.likeMatch("", "%"));
    }

    // ── Test 51: LIMIT with no rows to remove ────────────────────────────────

    @Test
    void testLimitBeyondResultSize() {
        // LIMIT 100 on a 4-row table → all 4 rows
        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("id")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            new Instruction.BeginRow(),
            new Instruction.LoadColumn(0, "id"),
            new Instruction.EmitColumn("id"),
            new Instruction.EmitRow(),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.LimitResult(100L, null),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(4, result.rows().size());
    }

    // ── Test 52: DISTINCT on already-distinct data ────────────────────────────

    @Test
    void testDistinctOnUniqueData() {
        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("id")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            new Instruction.BeginRow(),
            new Instruction.LoadColumn(0, "id"),
            new Instruction.EmitColumn("id"),
            new Instruction.EmitRow(),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.DistinctResult(),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(4, result.rows().size()); // no duplicates to remove
    }

    // ── Test 53: NULL in IN list → NULL when no match ─────────────────────────

    @Test
    void testInListWithNullElement() {
        // 5 IN (1, NULL, 3) → NULL (not found, but NULL in list)
        Program prog = buildProgram(List.of(
            new Instruction.LoadConst(5L),
            new Instruction.LoadConst(1L),
            new Instruction.LoadConst(null),
            new Instruction.LoadConst(3L),
            new Instruction.InList(3),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result = SqlVm.execute(prog, backend);
        assertNull(result.rows().get(0).get(0));
    }

    // ── Test 54: CallScalar NULLIF ───────────────────────────────────────────

    @Test
    void testCallScalarNullif() {
        // NULLIF(5, 5) → NULL
        Program prog = buildProgram(List.of(
            new Instruction.LoadConst(5L),
            new Instruction.LoadConst(5L),
            new Instruction.CallScalar("nullif", 2),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result = SqlVm.execute(prog, backend);
        assertNull(result.rows().get(0).get(0));

        // NULLIF(5, 6) → 5
        Program prog2 = buildProgram(List.of(
            new Instruction.LoadConst(5L),
            new Instruction.LoadConst(6L),
            new Instruction.CallScalar("nullif", 2),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result2 = SqlVm.execute(prog2, backend);
        assertEquals(5L, result2.rows().get(0).get(0));
    }

    // ── Test 55: OFFSET-only (null count) ────────────────────────────────────

    @Test
    void testOffsetOnly() {
        // LimitResult(null, 3) → skip first 3 rows → only row 4
        Program prog = buildProgram(List.of(
            new Instruction.SetResultSchema(List.of("id")),
            new Instruction.OpenScan(0, "users"),
            new Instruction.Label("loop"),
            new Instruction.AdvanceCursor(0, "done"),
            new Instruction.BeginRow(),
            new Instruction.LoadColumn(0, "id"),
            new Instruction.EmitColumn("id"),
            new Instruction.EmitRow(),
            new Instruction.Jump("loop"),
            new Instruction.Label("done"),
            new Instruction.CloseScan(0),
            new Instruction.LimitResult(null, 3L),
            new Instruction.Halt()
        ));

        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(1, result.rows().size());
        assertEquals(4L, result.rows().get(0).get(0));
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /** Build a Program from a list of instructions, computing the label map. */
    private static Program buildProgram(List<Instruction> instrs) {
        Map<String, Integer> labels = new HashMap<>();
        for (int i = 0; i < instrs.size(); i++) {
            if (instrs.get(i) instanceof Instruction.Label(var name)) {
                labels.put(name, i);
            }
        }
        return new Program(Collections.unmodifiableList(instrs), labels, List.of());
    }

    /** Convenience: build a Row from alternating key/value pairs. */
    private static Row row(Object... pairs) {
        Row r = new Row();
        for (int i = 0; i < pairs.length; i += 2) {
            r.put((String) pairs[i], pairs[i + 1]);
        }
        return r;
    }

    /** Count all rows in a table by doing a full scan. */
    private int countRows(String table) {
        int n = 0;
        RowIterator it = backend.scan(table);
        while (it.next() != null) n++;
        it.close();
        return n;
    }

    /**
     * Read a single column value from the first row where filterCol = filterVal.
     * Returns null if no such row exists.
     */
    @SuppressWarnings("unchecked")
    private <T> T readValue(String table, String filterCol, Object filterVal, String col) {
        RowIterator it = backend.scan(table);
        Row row;
        while ((row = it.next()) != null) {
            if (Objects.equals(row.get(filterCol), filterVal)) {
                it.close();
                return (T) row.get(col);
            }
        }
        it.close();
        return null;
    }

    /** Assert a binary operation between two constants produces the expected value. */
    private void assertBinaryOp(Object left, BinaryOpCode op, Object right, Object expected) {
        Program prog = buildProgram(List.of(
            new Instruction.LoadConst(left),
            new Instruction.LoadConst(right),
            new Instruction.BinaryOp(op),
            new Instruction.BeginRow(),
            new Instruction.EmitColumn("v"),
            new Instruction.EmitRow(),
            new Instruction.Halt()
        ));
        QueryResult result = SqlVm.execute(prog, backend);
        assertEquals(expected, result.rows().get(0).get(0),
            String.format("expected %s %s %s = %s", left, op, right, expected));
    }
}

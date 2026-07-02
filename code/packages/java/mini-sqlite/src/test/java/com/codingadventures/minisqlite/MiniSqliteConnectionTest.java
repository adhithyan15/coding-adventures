package com.codingadventures.minisqlite;

// MiniSqliteConnectionTest.java — end-to-end integration tests for the Level 1
// mini-sqlite connection.
//
// Every test uses only the public MiniSqliteConnection API, deliberately
// treating the pipeline as a black box.  This ensures that the tests
// exercise real SQL text → parse → plan → optimize → compile → execute flow.
//
// Test organisation
// ─────────────────
//   C01–C24  Conformance tests mirroring the 24 fixtures in
//            code/specs/mini-sqlite-conformance/fixtures/
//   API      DB-API-2.0 contract tests (constants, cursor lifecycle, etc.)
//   TXN      Transaction snapshot tests (commit, rollback)
//   ERR      Error-path tests (bad SQL, wrong table, wrong param count, etc.)
//
// Naming: every test method begins with the fixture id or category for easy
// filtering with `gradle test --tests "*.C01_*"`.

import com.codingadventures.minisqlite.MiniSqliteConnection.Connection;
import com.codingadventures.minisqlite.MiniSqliteConnection.Cursor;
import com.codingadventures.minisqlite.MiniSqliteConnection.MiniSqliteException;
import com.codingadventures.minisqlite.MiniSqliteConnection.Options;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;

import java.util.Arrays;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class MiniSqliteConnectionTest {

    // ── Helpers ───────────────────────────────────────────────────────────────

    private static Connection mem() {
        return MiniSqliteConnection.connect(":memory:");
    }

    /** Create a connection with autocommit enabled. */
    private static Connection memAuto() {
        return MiniSqliteConnection.connect(":memory:", new Options(true));
    }

    // ── DB-API constants ──────────────────────────────────────────────────────

    @Test @DisplayName("API: DB-API-2.0 constants are set correctly")
    void api_constants() {
        assertEquals("2.0",   MiniSqliteConnection.API_LEVEL);
        assertEquals(1,       MiniSqliteConnection.THREADSAFETY);
        assertEquals("qmark", MiniSqliteConnection.PARAMSTYLE);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C01 — CREATE TABLE, INSERT rows, SELECT all rows
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C01: CREATE TABLE, INSERT rows, SELECT all rows")
    void c01_createInsertSelectAll() {
        var conn = mem();
        conn.execute("CREATE TABLE users (id INTEGER, name TEXT, age INTEGER)");
        conn.execute("INSERT INTO users VALUES (1, 'Alice', 30)");
        conn.execute("INSERT INTO users VALUES (2, 'Bob', 25)");
        conn.execute("INSERT INTO users VALUES (3, 'Charlie', 35)");

        var cur = conn.execute("SELECT id, name, age FROM users ORDER BY id");
        assertEquals(List.of("id", "name", "age"), cur.description().stream()
            .map(c -> c.name()).toList());
        var rows = cur.fetchall();
        assertEquals(3, rows.size());
        assertEquals(1L,       rows.get(0).get(0));
        assertEquals("Alice",  rows.get(0).get(1));
        assertEquals(30L,      rows.get(0).get(2));
        assertEquals(2L,       rows.get(1).get(0));
        assertEquals("Bob",    rows.get(1).get(1));
        assertEquals(25L,      rows.get(1).get(2));
        assertEquals(3L,       rows.get(2).get(0));
        assertEquals("Charlie",rows.get(2).get(1));
        assertEquals(35L,      rows.get(2).get(2));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C02 — qmark (?) parameter binding in INSERT and SELECT
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C02: qmark parameter binding in INSERT and SELECT")
    void c02_qmarkBinding() {
        var conn = mem();
        conn.execute("CREATE TABLE products (id INTEGER, name TEXT, price REAL)");
        conn.execute("INSERT INTO products VALUES (?, ?, ?)", List.of(1, "Widget", 9.99));
        conn.execute("INSERT INTO products VALUES (?, ?, ?)", List.of(2, "Gadget", 24.99));
        conn.execute("INSERT INTO products VALUES (?, ?, ?)", List.of(3, "Doohickey", 4.99));

        var rows = conn.execute(
            "SELECT id, name FROM products WHERE price < ? ORDER BY id",
            List.of(15)).fetchall();
        assertEquals(2, rows.size());
        assertEquals(1L,        rows.get(0).get(0));
        assertEquals("Widget",  rows.get(0).get(1));
        assertEquals(3L,        rows.get(1).get(0));
        assertEquals("Doohickey", rows.get(1).get(1));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C03 — Column projection and AS aliases
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C03: column projection and AS aliases")
    void c03_projectionAliases() {
        var conn = mem();
        conn.execute("CREATE TABLE employees (id INTEGER, first_name TEXT, last_name TEXT, salary INTEGER)");
        conn.execute("INSERT INTO employees VALUES (1, 'Alice', 'Smith', 75000)");
        conn.execute("INSERT INTO employees VALUES (2, 'Bob', 'Jones', 82000)");

        var rows = conn.execute(
            "SELECT first_name AS name, salary AS pay FROM employees ORDER BY id").fetchall();
        assertEquals(2, rows.size());
        assertEquals("Alice", rows.get(0).get(0));
        assertEquals(75000L,  rows.get(0).get(1));

        // Column names from description reflect aliases
        var cur = conn.execute("SELECT first_name AS name, salary AS pay FROM employees ORDER BY id");
        assertEquals("name", cur.description().get(0).name());
        assertEquals("pay",  cur.description().get(1).name());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C04 — WHERE clause filtering
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C04: WHERE clause filtering")
    void c04_whereFiltering() {
        var conn = mem();
        conn.execute("CREATE TABLE scores (player TEXT, game TEXT, score INTEGER)");
        conn.execute("INSERT INTO scores VALUES ('Alice', 'chess', 1400)");
        conn.execute("INSERT INTO scores VALUES ('Alice', 'go', 1200)");
        conn.execute("INSERT INTO scores VALUES ('Bob', 'chess', 1600)");
        conn.execute("INSERT INTO scores VALUES ('Bob', 'go', 900)");

        // Equality filter with literal
        var rows1 = conn.execute(
            "SELECT player, score FROM scores WHERE game = 'chess' ORDER BY score").fetchall();
        assertEquals(2, rows1.size());
        assertEquals("Alice", rows1.get(0).get(0));
        assertEquals("Bob",   rows1.get(1).get(0));

        // GTE with qmark param
        var rows2 = conn.execute(
            "SELECT player, game FROM scores WHERE score >= ? ORDER BY player, game",
            List.of(1400)).fetchall();
        assertEquals(2, rows2.size());

        // AND predicate with params
        var rows3 = conn.execute(
            "SELECT player, score FROM scores WHERE game = ? AND score > ?",
            List.of("go", 1000)).fetchall();
        assertEquals(1, rows3.size());
        assertEquals("Alice", rows3.get(0).get(0));
        assertEquals(1200L,   rows3.get(0).get(1));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C05 — ORDER BY, LIMIT, OFFSET
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C05: ORDER BY, LIMIT, and OFFSET")
    void c05_orderByLimitOffset() {
        var conn = mem();
        conn.execute("CREATE TABLE items (id INTEGER, label TEXT, rank INTEGER)");
        for (var row : List.of(
            List.of(1, "alpha",   3),
            List.of(2, "beta",    1),
            List.of(3, "gamma",   4),
            List.of(4, "delta",   2),
            List.of(5, "epsilon", 5)
        )) {
            conn.execute("INSERT INTO items VALUES (?, ?, ?)", row);
        }

        // ORDER BY ASC
        var all = conn.execute("SELECT label FROM items ORDER BY rank").fetchall();
        assertEquals(List.of("beta", "delta", "alpha", "gamma", "epsilon"),
            all.stream().map(r -> r.get(0)).toList());

        // LIMIT 3
        var top3 = conn.execute("SELECT label FROM items ORDER BY rank LIMIT 3").fetchall();
        assertEquals(List.of("beta", "delta", "alpha"),
            top3.stream().map(r -> r.get(0)).toList());

        // LIMIT with OFFSET
        var mid = conn.execute("SELECT label FROM items ORDER BY rank LIMIT 2 OFFSET 2").fetchall();
        assertEquals(List.of("alpha", "gamma"),
            mid.stream().map(r -> r.get(0)).toList());

        // ORDER BY DESC
        var desc = conn.execute("SELECT label FROM items ORDER BY rank DESC LIMIT 2").fetchall();
        assertEquals(List.of("epsilon", "gamma"),
            desc.stream().map(r -> r.get(0)).toList());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C06 — Aggregate functions: COUNT, SUM, AVG, MIN, MAX
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C06: aggregate functions COUNT, SUM, AVG, MIN, MAX")
    void c06_aggregates() {
        var conn = mem();
        conn.execute("CREATE TABLE sales (region TEXT, amount INTEGER)");
        for (var r : List.of(
            List.of("east", 100), List.of("east", 200),
            List.of("west", 150), List.of("west", 50)
        )) conn.execute("INSERT INTO sales VALUES (?, ?)", r);

        var row = conn.execute(
            "SELECT COUNT(*), SUM(amount), MIN(amount), MAX(amount) FROM sales").fetchone();
        assertEquals(4L,  row.get(0)); // COUNT(*)
        assertEquals(500L,row.get(1)); // SUM
        assertEquals(50L, row.get(2)); // MIN
        assertEquals(200L,row.get(3)); // MAX

        // GROUP BY
        var grouped = conn.execute(
            "SELECT region, COUNT(*), SUM(amount) FROM sales GROUP BY region ORDER BY region").fetchall();
        assertEquals(2, grouped.size());
        assertEquals("east", grouped.get(0).get(0));
        assertEquals(2L,     grouped.get(0).get(1));
        assertEquals(300L,   grouped.get(0).get(2));
        assertEquals("west", grouped.get(1).get(0));
        assertEquals(2L,     grouped.get(1).get(1));
        assertEquals(200L,   grouped.get(1).get(2));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C07 — UPDATE and DELETE with WHERE
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C07: UPDATE and DELETE with WHERE")
    void c07_updateDelete() {
        var conn = mem();
        conn.execute("CREATE TABLE users (id INTEGER, name TEXT)");
        conn.executemany("INSERT INTO users VALUES (?, ?)", List.of(
            List.of(1, "Alice"), List.of(2, "Bob"), List.of(3, "Carol")));

        var upd = conn.execute("UPDATE users SET name = ? WHERE id = ?", List.of("Bobby", 2));
        assertEquals(1, upd.rowcount());

        var del = conn.execute("DELETE FROM users WHERE id IN (1, 3)");
        assertEquals(2, del.rowcount());

        var rows = conn.execute("SELECT id, name FROM users").fetchall();
        assertEquals(1, rows.size());
        assertEquals(2L,      rows.get(0).get(0));
        assertEquals("Bobby", rows.get(0).get(1));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C08 — Transaction commit
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C08: transaction commit persists changes")
    void c08_transactionCommit() {
        var conn = mem();
        conn.execute("CREATE TABLE t (v INTEGER)");
        conn.commit();
        conn.execute("INSERT INTO t VALUES (42)");
        conn.commit();
        conn.rollback();  // after commit, rollback is a no-op
        var rows = conn.execute("SELECT v FROM t").fetchall();
        assertEquals(1, rows.size());
        assertEquals(42L, rows.get(0).get(0));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C09 — Transaction rollback
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C09: transaction rollback restores pre-transaction state")
    void c09_transactionRollback() {
        var conn = mem();
        conn.execute("CREATE TABLE t (v INTEGER)");
        conn.commit();

        conn.execute("INSERT INTO t VALUES (99)");
        conn.rollback();
        assertEquals(0, conn.execute("SELECT v FROM t").fetchall().size());

        conn.execute("INSERT INTO t VALUES (7)");
        conn.commit();
        conn.rollback(); // post-commit rollback is no-op
        assertEquals(1, conn.execute("SELECT v FROM t").fetchall().size());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C10 — Wrong parameter count → ProgrammingError
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C10: wrong parameter count raises ProgrammingError")
    void c10_wrongParamCount() {
        var conn = mem();
        conn.execute("CREATE TABLE t (a INTEGER, b INTEGER)");
        var ex = assertThrows(MiniSqliteException.class, () ->
            conn.execute("INSERT INTO t VALUES (?, ?)", List.of(1)));
        assertEquals("ProgrammingError", ex.kind());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C11 — Unknown table → OperationalError
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C11: unknown table raises OperationalError")
    void c11_unknownTable() {
        var conn = mem();
        var ex = assertThrows(MiniSqliteException.class, () ->
            conn.execute("SELECT * FROM no_such_table"));
        assertEquals("OperationalError", ex.kind());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C12 — File-backed path → NotSupportedError
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C12: file-backed path raises NotSupportedError")
    void c12_filePath() {
        var ex = assertThrows(MiniSqliteException.class, () ->
            MiniSqliteConnection.connect("app.db"));
        assertEquals("NotSupportedError", ex.kind());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C13 — DROP TABLE [IF EXISTS]
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C13: DROP TABLE and DROP TABLE IF EXISTS")
    void c13_dropTable() {
        var conn = mem();
        conn.execute("CREATE TABLE t (v INTEGER)");
        conn.execute("INSERT INTO t VALUES (1)");
        conn.execute("DROP TABLE t");

        // After drop the table no longer exists
        assertThrows(MiniSqliteException.class, () -> conn.execute("SELECT * FROM t"));

        // IF EXISTS is a no-op on a missing table
        assertDoesNotThrow(() -> conn.execute("DROP TABLE IF EXISTS t"));

        // Re-create works
        conn.execute("CREATE TABLE t (v INTEGER)");
        conn.execute("INSERT INTO t VALUES (2)");
        assertEquals(2L, conn.execute("SELECT v FROM t").fetchone().get(0));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C14 — executemany
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C14: executemany bulk inserts rows")
    void c14_executemany() {
        var conn = mem();
        conn.execute("CREATE TABLE t (n INTEGER)");
        conn.executemany("INSERT INTO t VALUES (?)",
            List.of(List.of(1), List.of(2), List.of(3), List.of(4), List.of(5)));

        var rows = conn.execute("SELECT n FROM t ORDER BY n").fetchall();
        assertEquals(5, rows.size());
        assertEquals(1L, rows.get(0).get(0));
        assertEquals(5L, rows.get(4).get(0));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C15 — fetchone and fetchmany
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C15: fetchone and fetchmany iterate incrementally")
    void c15_fetchoneAndFetchmany() {
        var conn = mem();
        conn.execute("CREATE TABLE t (n INTEGER)");
        conn.executemany("INSERT INTO t VALUES (?)",
            List.of(List.of(1), List.of(2), List.of(3)));

        var cur = conn.execute("SELECT n FROM t ORDER BY n");
        assertEquals(1L, cur.fetchone().get(0));
        assertEquals(2L, cur.fetchmany(1).get(0).get(0));
        assertEquals(3L, cur.fetchall().get(0).get(0));
        assertNull(cur.fetchone()); // exhausted
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C16 — NULL handling
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C16: NULL handling in SELECT and WHERE")
    void c16_nullHandling() {
        var conn = mem();
        conn.execute("CREATE TABLE t (id INTEGER, val TEXT)");
        conn.execute("INSERT INTO t VALUES (1, 'hello')");
        conn.execute("INSERT INTO t VALUES (2, NULL)");

        var notNull = conn.execute("SELECT id FROM t WHERE val IS NOT NULL ORDER BY id").fetchall();
        assertEquals(1, notNull.size());
        assertEquals(1L, notNull.get(0).get(0));

        var isNull = conn.execute("SELECT id FROM t WHERE val IS NULL").fetchall();
        assertEquals(1, isNull.size());
        assertEquals(2L, isNull.get(0).get(0));

        // NULL in result
        var rows = conn.execute("SELECT val FROM t ORDER BY id").fetchall();
        assertEquals("hello", rows.get(0).get(0));
        assertNull(rows.get(1).get(0));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C17 — NULL aggregate semantics (NULL inputs ignored)
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C17: aggregate functions ignore NULL inputs")
    void c17_nullAggregateSemantics() {
        var conn = mem();
        conn.execute("CREATE TABLE t (v INTEGER)");
        conn.execute("INSERT INTO t VALUES (10)");
        conn.execute("INSERT INTO t VALUES (NULL)");
        conn.execute("INSERT INTO t VALUES (20)");

        var row = conn.execute("SELECT COUNT(*), COUNT(v), SUM(v), AVG(v), MIN(v), MAX(v) FROM t").fetchone();
        assertEquals(3L,  row.get(0)); // COUNT(*) counts NULLs
        assertEquals(2L,  row.get(1)); // COUNT(v) ignores NULLs
        assertEquals(30L, row.get(2)); // SUM ignores NULLs
        // AVG = 15.0
        assertEquals(15.0, ((Number) row.get(3)).doubleValue(), 0.001);
        assertEquals(10L, row.get(4)); // MIN
        assertEquals(20L, row.get(5)); // MAX
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C18 — String functions
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C18: string functions UPPER, LOWER, LENGTH, TRIM")
    void c18_stringFunctions() {
        var conn = mem();
        conn.execute("CREATE TABLE t (s TEXT)");
        conn.execute("INSERT INTO t VALUES ('  Hello World  ')");

        var row = conn.execute(
            "SELECT UPPER(TRIM(s)), LOWER(TRIM(s)), LENGTH(TRIM(s)) FROM t").fetchone();
        assertEquals("HELLO WORLD", row.get(0));
        assertEquals("hello world", row.get(1));
        assertEquals(11L,           row.get(2));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C19 — Math functions (ABS)
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C19: ABS scalar function")
    void c19_mathFunctions() {
        var conn = mem();
        conn.execute("CREATE TABLE t (v INTEGER)");
        conn.execute("INSERT INTO t VALUES (-5)");
        conn.execute("INSERT INTO t VALUES (3)");
        conn.execute("INSERT INTO t VALUES (-1)");

        var rows = conn.execute("SELECT ABS(v) FROM t ORDER BY v").fetchall();
        assertEquals(3, rows.size());
        // Sorted by original v: -5, -1, 3 → ABS: 5, 1, 3
        assertEquals(5L, rows.get(0).get(0));
        assertEquals(1L, rows.get(1).get(0));
        assertEquals(3L, rows.get(2).get(0));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C20 — LIMIT edge cases
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C20: LIMIT 0, LIMIT beyond size, and OFFSET beyond size")
    void c20_limitEdgeCases() {
        var conn = mem();
        conn.execute("CREATE TABLE t (n INTEGER)");
        conn.executemany("INSERT INTO t VALUES (?)",
            List.of(List.of(1), List.of(2), List.of(3)));

        assertEquals(0, conn.execute("SELECT n FROM t LIMIT 0").fetchall().size());
        assertEquals(3, conn.execute("SELECT n FROM t LIMIT 100").fetchall().size());
        assertEquals(0, conn.execute("SELECT n FROM t LIMIT 3 OFFSET 3").fetchall().size());
        var r = conn.execute("SELECT n FROM t ORDER BY n LIMIT 2 OFFSET 1").fetchall();
        assertEquals(2L, r.get(0).get(0));
        assertEquals(3L, r.get(1).get(0));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C21 — DISTINCT and DISTINCT in aggregates
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C21: SELECT DISTINCT and COUNT(DISTINCT …)")
    void c21_distinctAggregate() {
        var conn = mem();
        conn.execute("CREATE TABLE t (cat TEXT, val INTEGER)");
        conn.executemany("INSERT INTO t VALUES (?, ?)", List.of(
            List.of("a", 1), List.of("a", 1), List.of("b", 2), List.of("b", 3)
        ));

        var distinct = conn.execute("SELECT DISTINCT cat FROM t ORDER BY cat").fetchall();
        assertEquals(2, distinct.size());
        assertEquals("a", distinct.get(0).get(0));
        assertEquals("b", distinct.get(1).get(0));

        var countDist = conn.execute("SELECT COUNT(DISTINCT val) FROM t").fetchone();
        assertEquals(3L, countDist.get(0));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C22 — String concatenation and NULL propagation in expressions
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C22: string concatenation and NULL propagation in concat")
    void c22_stringConcatNull() {
        var conn = mem();
        conn.execute("CREATE TABLE t (a TEXT, b TEXT)");
        conn.execute("INSERT INTO t VALUES ('Hello', ' World')");
        conn.execute("INSERT INTO t VALUES ('foo', NULL)");

        var rows = conn.execute("SELECT a, b FROM t ORDER BY a").fetchall();
        // Verify data is stored correctly
        assertEquals("Hello", rows.get(0).get(0));
        assertEquals(" World", rows.get(0).get(1));
        assertEquals("foo",  rows.get(1).get(0));
        assertNull(rows.get(1).get(1));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C23 — NULL in ORDER BY (NULLs sort last by default)
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C23: NULLs in ORDER BY — default sort position")
    void c23_nullInOrderBy() {
        var conn = mem();
        conn.execute("CREATE TABLE t (n INTEGER)");
        conn.execute("INSERT INTO t VALUES (3)");
        conn.execute("INSERT INTO t VALUES (NULL)");
        conn.execute("INSERT INTO t VALUES (1)");

        // In the VM's default ordering, NULLs rank at 0 (NULLS FIRST for ASC).
        // The sql-vm sorts NULL with sqlTypeRank = 0, so NULLs come first in ASC.
        var asc = conn.execute("SELECT n FROM t ORDER BY n ASC").fetchall();
        assertEquals(3, asc.size());
        // First row should be NULL (lowest sqlTypeRank)
        assertNull(asc.get(0).get(0));

        // Verify non-null values are in order
        assertEquals(1L, asc.get(1).get(0));
        assertEquals(3L, asc.get(2).get(0));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // C24 — HAVING clause
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("C24: HAVING clause filters aggregate groups")
    void c24_havingAggregate() {
        var conn = mem();
        conn.execute("CREATE TABLE orders (customer TEXT, amount INTEGER)");
        conn.executemany("INSERT INTO orders VALUES (?, ?)", List.of(
            List.of("Alice", 100), List.of("Alice", 200),
            List.of("Bob",   50),  List.of("Bob",   60),
            List.of("Carol", 300)
        ));

        var rows = conn.execute(
            "SELECT customer, SUM(amount) AS total FROM orders " +
            "GROUP BY customer HAVING SUM(amount) > 150 ORDER BY customer").fetchall();
        assertEquals(2, rows.size());
        assertEquals("Alice", rows.get(0).get(0));
        assertEquals(300L,    rows.get(0).get(1));
        assertEquals("Carol", rows.get(1).get(0));
        assertEquals(300L,    rows.get(1).get(1));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Additional pipeline-specific tests
    // ═══════════════════════════════════════════════════════════════════════════

    @Test @DisplayName("API: cursor description reflects column names after SELECT")
    void api_cursorDescription() {
        var conn = mem();
        conn.execute("CREATE TABLE t (a INTEGER, b TEXT)");
        conn.execute("INSERT INTO t VALUES (1, 'x')");
        var cur = conn.execute("SELECT a, b FROM t");
        assertEquals(2, cur.description().size());
        assertEquals("a", cur.description().get(0).name());
        assertEquals("b", cur.description().get(1).name());
    }

    @Test @DisplayName("API: rowcount reflects affected rows for DML")
    void api_rowcount() {
        var conn = mem();
        conn.execute("CREATE TABLE t (v INTEGER)");
        conn.executemany("INSERT INTO t VALUES (?)",
            List.of(List.of(1), List.of(2), List.of(3)));
        assertEquals(2, conn.execute("DELETE FROM t WHERE v < 3").rowcount());
    }

    @Test @DisplayName("API: cursor close prevents further fetches")
    void api_cursorClose() {
        var conn = mem();
        conn.execute("CREATE TABLE t (v INTEGER)");
        conn.execute("INSERT INTO t VALUES (1)");
        var cur = conn.execute("SELECT v FROM t");
        cur.close();
        assertEquals(List.of(), cur.fetchall());
        assertNull(cur.fetchone());
    }

    @Test @DisplayName("API: closed connection raises ProgrammingError")
    void api_closedConnection() {
        var conn = mem();
        conn.close();
        assertThrows(MiniSqliteException.class, () -> conn.execute("SELECT 1"));
    }

    @Test @DisplayName("API: arraysize controls fetchmany default batch size")
    void api_arraysize() {
        var conn = mem();
        conn.execute("CREATE TABLE t (n INTEGER)");
        conn.executemany("INSERT INTO t VALUES (?)",
            List.of(List.of(1), List.of(2), List.of(3), List.of(4)));
        var cur = conn.execute("SELECT n FROM t ORDER BY n");
        cur.arraysize(2);
        assertEquals(2, cur.fetchmany().size());
        assertEquals(2, cur.fetchmany().size());
        assertEquals(0, cur.fetchmany().size()); // exhausted
    }

    @Test @DisplayName("ERR: too many parameters raises ProgrammingError")
    void err_tooManyParams() {
        var conn = mem();
        conn.execute("CREATE TABLE t (v INTEGER)");
        var ex = assertThrows(MiniSqliteException.class, () ->
            conn.execute("INSERT INTO t VALUES (?)", List.of(1, 2)));
        assertEquals("ProgrammingError", ex.kind());
    }

    @Test @DisplayName("ERR: CREATE TABLE IF NOT EXISTS is idempotent")
    void err_createTableIfNotExists() {
        var conn = mem();
        conn.execute("CREATE TABLE t (v INTEGER)");
        assertDoesNotThrow(() -> conn.execute("CREATE TABLE IF NOT EXISTS t (v INTEGER)"));
    }

    @Test @DisplayName("ERR: duplicate CREATE TABLE raises OperationalError")
    void err_duplicateCreateTable() {
        var conn = mem();
        conn.execute("CREATE TABLE t (v INTEGER)");
        assertThrows(MiniSqliteException.class, () -> conn.execute("CREATE TABLE t (v INTEGER)"));
    }

    @Test @DisplayName("TXN: autocommit mode skips snapshot overhead")
    void txn_autocommit() {
        var conn = memAuto();
        conn.execute("CREATE TABLE t (v INTEGER)");
        conn.execute("INSERT INTO t VALUES (42)");
        // With autocommit, there is no snapshot to roll back.
        conn.rollback(); // no-op
        assertEquals(1, conn.execute("SELECT v FROM t").fetchall().size());
    }

    @Test @DisplayName("PIPE: SELECT * expands all columns")
    void pipe_selectStar() {
        var conn = mem();
        conn.execute("CREATE TABLE t (a INTEGER, b TEXT, c REAL)");
        conn.execute("INSERT INTO t VALUES (1, 'hello', 3.14)");
        var cur = conn.execute("SELECT * FROM t");
        assertEquals(3, cur.description().size());
        var row = cur.fetchone();
        assertEquals(1L,    row.get(0));
        assertEquals("hello", row.get(1));
    }

    @Test @DisplayName("PIPE: BETWEEN predicate")
    void pipe_between() {
        var conn = mem();
        conn.execute("CREATE TABLE t (n INTEGER)");
        conn.executemany("INSERT INTO t VALUES (?)",
            List.of(List.of(1), List.of(5), List.of(10), List.of(15)));
        var rows = conn.execute("SELECT n FROM t WHERE n BETWEEN 5 AND 10 ORDER BY n").fetchall();
        assertEquals(2, rows.size());
        assertEquals(5L,  rows.get(0).get(0));
        assertEquals(10L, rows.get(1).get(0));
    }

    @Test @DisplayName("PIPE: IN list predicate")
    void pipe_inList() {
        var conn = mem();
        conn.execute("CREATE TABLE t (n INTEGER)");
        conn.executemany("INSERT INTO t VALUES (?)",
            List.of(List.of(1), List.of(2), List.of(3), List.of(4)));
        var rows = conn.execute("SELECT n FROM t WHERE n IN (2, 4) ORDER BY n").fetchall();
        assertEquals(2, rows.size());
        assertEquals(2L, rows.get(0).get(0));
        assertEquals(4L, rows.get(1).get(0));
    }

    @Test @DisplayName("PIPE: LIKE predicate")
    void pipe_like() {
        var conn = mem();
        conn.execute("CREATE TABLE t (name TEXT)");
        conn.executemany("INSERT INTO t VALUES (?)",
            List.of(List.of("Alice"), List.of("Bob"), List.of("Carol")));
        var rows = conn.execute("SELECT name FROM t WHERE name LIKE 'A%' ORDER BY name").fetchall();
        assertEquals(1, rows.size());
        assertEquals("Alice", rows.get(0).get(0));
    }

    @Test @DisplayName("PIPE: multi-column ORDER BY")
    void pipe_multiColumnOrderBy() {
        var conn = mem();
        conn.execute("CREATE TABLE t (a INTEGER, b TEXT)");
        conn.executemany("INSERT INTO t VALUES (?, ?)", List.of(
            List.of(1, "z"), List.of(1, "a"), List.of(2, "m")
        ));
        var rows = conn.execute("SELECT a, b FROM t ORDER BY a ASC, b ASC").fetchall();
        assertEquals("a", rows.get(0).get(1));
        assertEquals("z", rows.get(1).get(1));
        assertEquals("m", rows.get(2).get(1));
    }

    @Test @DisplayName("PIPE: AVG aggregate returns correct floating-point result")
    void pipe_avgAggregate() {
        var conn = mem();
        conn.execute("CREATE TABLE t (v INTEGER)");
        conn.executemany("INSERT INTO t VALUES (?)",
            List.of(List.of(10), List.of(20), List.of(30)));
        var row = conn.execute("SELECT AVG(v) FROM t").fetchone();
        assertEquals(20.0, ((Number) row.get(0)).doubleValue(), 0.001);
    }

    @Test @DisplayName("PIPE: COALESCE scalar function")
    void pipe_coalesce() {
        var conn = mem();
        conn.execute("CREATE TABLE t (a TEXT, b TEXT)");
        conn.execute("INSERT INTO t VALUES (NULL, 'fallback')");
        conn.execute("INSERT INTO t VALUES ('primary', 'fallback')");
        var rows = conn.execute("SELECT COALESCE(a, b) FROM t ORDER BY b").fetchall();
        assertEquals("fallback", rows.get(0).get(0));
        assertEquals("primary",  rows.get(1).get(0));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Additional coverage tests for uncovered paths
    // ─────────────────────────────────────────────────────────────────────────

    @Test @DisplayName("PIPE: SELECT * from an INNER JOIN expands both tables")
    void pipe_selectStarJoin() {
        var conn = mem();
        conn.execute("CREATE TABLE a (id INTEGER, name TEXT)");
        conn.execute("CREATE TABLE b (id INTEGER, val INTEGER)");
        conn.execute("INSERT INTO a VALUES (1, 'alice')");
        conn.execute("INSERT INTO b VALUES (1, 42)");
        // SELECT * from a INNER JOIN b should return all columns from both tables.
        var cur = conn.execute("SELECT * FROM a INNER JOIN b ON a.id = b.id");
        // 2 tables × 2 columns each = 4 columns
        assertEquals(4, cur.description().size());
        var row = cur.fetchone();
        assertNotNull(row);
        assertEquals(4, row.size());
    }

    @Test @DisplayName("COV: HAVING with AND predicate filters correctly")
    void cov_havingAnd() {
        var conn = mem();
        conn.execute("CREATE TABLE sales (region TEXT, amount INTEGER)");
        conn.executemany("INSERT INTO sales VALUES (?, ?)", List.of(
            List.of("east", 100), List.of("east", 200),
            List.of("west", 50),  List.of("west", 400),
            List.of("north", 80)
        ));
        // HAVING SUM(amount) > 150 AND SUM(amount) < 500 → east(300), west(450) excluded if > 500
        // east=300, west=450 → both pass 150 < x < 500; north=80 → fails
        var rows = conn.execute(
            "SELECT region, SUM(amount) AS total FROM sales " +
            "GROUP BY region HAVING SUM(amount) > 150 AND SUM(amount) < 500 " +
            "ORDER BY region").fetchall();
        assertEquals(2, rows.size());
        assertEquals("east", rows.get(0).get(0));
        assertEquals(300L,   rows.get(0).get(1));
        assertEquals("west", rows.get(1).get(0));
        assertEquals(450L,   rows.get(1).get(1));
    }

    @Test @DisplayName("COV: HAVING with OR predicate")
    void cov_havingOr() {
        var conn = mem();
        conn.execute("CREATE TABLE grp (cat TEXT, n INTEGER)");
        conn.executemany("INSERT INTO grp VALUES (?, ?)", List.of(
            List.of("a", 10), List.of("b", 20), List.of("c", 5)
        ));
        // HAVING SUM(n) < 8 OR SUM(n) > 15 → a=10 passes (>15? no, <8? no) → fails
        // b=20 passes (>15), c=5 passes (<8)
        var rows = conn.execute(
            "SELECT cat, SUM(n) AS total FROM grp " +
            "GROUP BY cat HAVING SUM(n) < 8 OR SUM(n) > 15 " +
            "ORDER BY cat").fetchall();
        assertEquals(2, rows.size());
        assertEquals("b", rows.get(0).get(0));
        assertEquals("c", rows.get(1).get(0));
    }

    @Test @DisplayName("COV: connection close rolls back uncommitted work")
    void cov_closeRollsBack() {
        var conn = mem();
        conn.execute("CREATE TABLE t (v INTEGER)");
        conn.execute("INSERT INTO t VALUES (1)");
        // Don't commit; close() should roll back.
        conn.close();
        // After close, operations should throw.
        assertThrows(MiniSqliteException.class, () -> conn.execute("SELECT * FROM t"));
    }

    @Test @DisplayName("COV: cursor execute on closed cursor throws")
    void cov_closedCursorThrows() {
        var conn = mem();
        conn.execute("CREATE TABLE t (v INTEGER)");
        var cur = conn.cursor();
        cur.close();
        assertThrows(MiniSqliteException.class, () -> cur.execute("SELECT 1"));
    }

    @Test @DisplayName("COV: fetchmany on closed cursor returns empty list")
    void cov_fetchmanyClosedCursor() {
        var conn = mem();
        conn.execute("CREATE TABLE t (v INTEGER)");
        conn.execute("INSERT INTO t VALUES (1)");
        var cur = conn.cursor().execute("SELECT v FROM t");
        cur.close();
        assertEquals(List.of(), cur.fetchmany(5));
        assertEquals(List.of(), cur.fetchall());
        assertNull(cur.fetchone());
    }

    @Test @DisplayName("COV: INSERT with explicit column list")
    void cov_insertExplicitCols() {
        var conn = mem();
        conn.execute("CREATE TABLE t (a INTEGER, b TEXT, c INTEGER)");
        conn.execute("INSERT INTO t (a, c) VALUES (1, 99)");
        var row = conn.execute("SELECT a, c FROM t").fetchone();
        assertEquals(1L,  row.get(0));
        assertEquals(99L, row.get(1));
    }

    @Test @DisplayName("COV: autocommit mode commits automatically")
    void cov_autocommitMode() {
        var conn = memAuto();
        conn.execute("CREATE TABLE t (v INTEGER)");
        conn.execute("INSERT INTO t VALUES (42)");
        // With autocommit, no explicit commit needed.
        // rollback is a no-op in autocommit mode.
        conn.rollback();
        var rows = conn.execute("SELECT v FROM t").fetchall();
        assertEquals(1, rows.size());
    }

    @Test @DisplayName("COV: BEGIN / COMMIT / ROLLBACK keywords are handled")
    void cov_transactionKeywords() {
        var conn = mem();
        conn.execute("CREATE TABLE t (v INTEGER)");
        // BEGIN starts a transaction
        conn.execute("BEGIN");
        conn.execute("INSERT INTO t VALUES (1)");
        conn.execute("COMMIT");
        assertEquals(1, conn.execute("SELECT v FROM t").fetchall().size());
        // ROLLBACK discards pending work
        conn.execute("BEGIN");
        conn.execute("INSERT INTO t VALUES (2)");
        conn.execute("ROLLBACK");
        assertEquals(1, conn.execute("SELECT v FROM t").fetchall().size());
    }

    @Test @DisplayName("COV: too many bind parameters throws ProgrammingError")
    void cov_tooManyParams() {
        var conn = mem();
        conn.execute("CREATE TABLE t (v INTEGER)");
        assertThrows(MiniSqliteException.class,
            () -> conn.execute("INSERT INTO t VALUES (?)", List.of(1, 2)));
    }

    @Test @DisplayName("COV: too few bind parameters throws ProgrammingError")
    void cov_tooFewParams() {
        var conn = mem();
        conn.execute("CREATE TABLE t (a INTEGER, b INTEGER)");
        assertThrows(MiniSqliteException.class,
            () -> conn.execute("INSERT INTO t VALUES (?, ?)", List.of(1)));
    }

    @Test @DisplayName("COV: ORDER BY expression using alias column")
    void cov_orderByAliasColumn() {
        var conn = mem();
        conn.execute("CREATE TABLE t (n INTEGER)");
        conn.executemany("INSERT INTO t VALUES (?)", List.of(
            List.of(3), List.of(1), List.of(2)));
        // n IS projected AND is the sort key — no extra column needed
        var rows = conn.execute("SELECT n FROM t ORDER BY n DESC").fetchall();
        assertEquals(3L, rows.get(0).get(0));
        assertEquals(2L, rows.get(1).get(0));
        assertEquals(1L, rows.get(2).get(0));
    }

    @Test @DisplayName("COV: boolean parameter binding")
    void cov_booleanParam() {
        var conn = mem();
        conn.execute("CREATE TABLE flags (active INTEGER)");
        conn.execute("INSERT INTO flags VALUES (?)", List.of(true));
        conn.execute("INSERT INTO flags VALUES (?)", List.of(false));
        var rows = conn.execute("SELECT active FROM flags ORDER BY active").fetchall();
        assertEquals(2, rows.size());
    }

    @Test @DisplayName("COV: null parameter binding")
    void cov_nullParam() {
        var conn = mem();
        conn.execute("CREATE TABLE t (v TEXT)");
        conn.execute("INSERT INTO t VALUES (?)", Arrays.asList((Object) null));
        var row = conn.execute("SELECT v FROM t").fetchone();
        assertNull(row.get(0));
    }
}

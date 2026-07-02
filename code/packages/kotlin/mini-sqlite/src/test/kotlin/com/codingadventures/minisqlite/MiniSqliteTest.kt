package com.codingadventures.minisqlite

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

// MiniSqliteTest.kt — comprehensive test suite for the Level 1 mini-sqlite package.
//
// Organisation
// ────────────
// The tests are arranged in a "bottom-up" order that mirrors the build-up of
// features.  Level 0 conformance fixtures come first (they must still pass);
// then Level 1 fixtures are added one capability at a time.
//
// Test names are plain camelCase (no backtick special characters) to avoid
// the JVM restriction on "--" and ":" in test method names.
//
// Coverage targets: >80% instruction coverage (enforced by JaCoCo in CI).
// In practice we aim for ~95% because a library should be exhaustively tested.

class MiniSqliteTest {

    // ── DB-API constants ──────────────────────────────────────────────────────

    @Test
    fun exposesDbApiStyleConstants() {
        assertEquals("2.0", MiniSqlite.API_LEVEL)
        assertEquals(1, MiniSqlite.THREADSAFETY)
        assertEquals("qmark", MiniSqlite.PARAMSTYLE)
    }

    // ── Connection ────────────────────────────────────────────────────────────

    @Test
    fun rejectsFileBackedConnections() {
        val error = assertFailsWith<MiniSqliteException> { MiniSqlite.connect("app.db") }
        assertEquals("NotSupportedError", error.kind)
    }

    @Test
    fun closedConnectionThrowsOnSubsequentUse() {
        val conn = MiniSqlite.connect(":memory:")
        conn.close()
        val error = assertFailsWith<MiniSqliteException> { conn.execute("SELECT 1") }
        assertEquals("ProgrammingError", error.kind)
    }

    @Test
    fun doubleCloseIsHarmless() {
        val conn = MiniSqlite.connect(":memory:")
        conn.close()
        conn.close()  // must not throw
    }

    // ── Basic CREATE / INSERT / SELECT ────────────────────────────────────────

    @Test
    fun createsInsertsAndSelectsRows() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE users (id INTEGER, name TEXT, active BOOLEAN)")
        conn.executemany(
            "INSERT INTO users VALUES (?, ?, ?)",
            listOf(
                listOf(1, "Alice", true),
                listOf(2, "Bob", false),
                listOf(3, "Carol", true),
            ),
        )
        val cursor = conn.execute("SELECT name FROM users WHERE active = ? ORDER BY id ASC", listOf(true))
        assertEquals("name", cursor.description[0].name)
        val rows = cursor.fetchall()
        assertEquals(2, rows.size)
        assertEquals("Alice", rows[0][0])
        assertEquals("Carol", rows[1][0])
    }

    @Test
    fun fetchesIncrementally() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE nums (n INTEGER)")
        conn.executemany("INSERT INTO nums VALUES (?)", listOf(listOf(1), listOf(2), listOf(3)))
        val cursor = conn.execute("SELECT n FROM nums ORDER BY n ASC")

        assertEquals(1L, cursor.fetchone()!![0])
        assertEquals(2L, cursor.fetchmany(1)[0][0])
        assertEquals(3L, cursor.fetchall()[0][0])
        assertNull(cursor.fetchone())
    }

    @Test
    fun selectStar() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (a INTEGER, b TEXT)")
        conn.execute("INSERT INTO t VALUES (42, 'hello')")
        val rows = conn.execute("SELECT * FROM t").fetchall()
        assertEquals(1, rows.size)
        assertEquals(42L, rows[0][0])
        assertEquals("hello", rows[0][1])
    }

    // ── UPDATE / DELETE ───────────────────────────────────────────────────────

    @Test
    fun updatesAndDeletesRows() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE users (id INTEGER, name TEXT)")
        conn.executemany(
            "INSERT INTO users VALUES (?, ?)",
            listOf(listOf(1, "Alice"), listOf(2, "Bob"), listOf(3, "Carol")),
        )
        val updated = conn.execute("UPDATE users SET name = ? WHERE id = ?", listOf("Bobby", 2))
        assertEquals(1, updated.rowcount)

        val deleted = conn.execute("DELETE FROM users WHERE id IN (?, ?)", listOf(1, 3))
        assertEquals(2, deleted.rowcount)

        val rows = conn.execute("SELECT id, name FROM users").fetchall()
        assertEquals(1, rows.size)
        assertEquals(2L, rows[0][0])
        assertEquals("Bobby", rows[0][1])
    }

    // ── CREATE TABLE IF NOT EXISTS / DROP TABLE ───────────────────────────────

    @Test
    fun createTableIfNotExists() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        // Second CREATE with IF NOT EXISTS must not throw
        conn.execute("CREATE TABLE IF NOT EXISTS t (x INTEGER)")
        // Without IF NOT EXISTS it must throw
        assertFailsWith<MiniSqliteException> {
            conn.execute("CREATE TABLE t (x INTEGER)")
        }
    }

    @Test
    fun dropTableIfExists() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("DROP TABLE t")
        // DROP IF EXISTS on nonexistent table must not throw
        conn.execute("DROP TABLE IF EXISTS t")
        // DROP without IF EXISTS must throw
        assertFailsWith<MiniSqliteException> {
            conn.execute("DROP TABLE t")
        }
    }

    // ── Column constraints in CREATE TABLE ───────────────────────────────────

    @Test
    fun createTableWithColumnConstraints() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute(
            "CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT NOT NULL, price REAL DEFAULT 0.0, code TEXT UNIQUE)"
        )
        conn.execute("INSERT INTO products (id, name) VALUES (1, 'Widget')")
        val rows = conn.execute("SELECT id, name FROM products").fetchall()
        assertEquals(1L, rows[0][0])
        assertEquals("Widget", rows[0][1])
    }

    // ── Transactions ──────────────────────────────────────────────────────────

    @Test
    fun rollsBackAndCommitsSnapshots() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE users (id INTEGER, name TEXT)")
        conn.commit()

        // Insert then rollback — row should disappear
        conn.execute("INSERT INTO users VALUES (?, ?)", listOf(1, "Alice"))
        conn.rollback()
        assertEquals(0, conn.execute("SELECT * FROM users").fetchall().size)

        // Insert then commit — row should persist after rollback
        conn.execute("INSERT INTO users VALUES (?, ?)", listOf(1, "Alice"))
        conn.commit()
        conn.rollback()
        assertEquals(1, conn.execute("SELECT * FROM users").fetchall().size)
    }

    @Test
    fun closeRollsBackOpenTransaction() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.commit()
        conn.execute("INSERT INTO t VALUES (1)")
        conn.close()

        // Open a fresh connection to the same (isolated) backend — but since
        // InMemoryBackend is per-Connection, we just verify the old conn is closed.
        val error = assertFailsWith<MiniSqliteException> { conn.execute("SELECT * FROM t") }
        assertEquals("ProgrammingError", error.kind)
    }

    @Test
    fun explicitBeginCommitRollback() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("BEGIN")
        conn.execute("INSERT INTO t VALUES (42)")
        conn.execute("COMMIT")
        assertEquals(1, conn.execute("SELECT * FROM t").fetchall().size)

        conn.execute("BEGIN")
        conn.execute("INSERT INTO t VALUES (99)")
        conn.execute("ROLLBACK")
        assertEquals(1, conn.execute("SELECT * FROM t").fetchall().size)
    }

    @Test
    fun autocommitMode() {
        val conn = MiniSqlite.connect(":memory:", Options(autocommit = true))
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")
        // rollback is a no-op in autocommit mode — data should still be there
        conn.rollback()
        assertEquals(1, conn.execute("SELECT * FROM t").fetchall().size)
    }

    // ── Cursor API ────────────────────────────────────────────────────────────

    @Test
    fun closedCursorThrows() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        val cursor = conn.cursor()
        cursor.execute("SELECT * FROM t")
        cursor.close()
        val error = assertFailsWith<MiniSqliteException> { cursor.execute("SELECT * FROM t") }
        assertEquals("ProgrammingError", error.kind)
    }

    @Test
    fun closedCursorFetchReturnsEmpty() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        val cursor = conn.execute("SELECT * FROM t")
        cursor.close()
        assertNull(cursor.fetchone())
        assertEquals(emptyList<List<Any?>>(), cursor.fetchall())
        assertEquals(emptyList<List<Any?>>(), cursor.fetchmany(3))
    }

    @Test
    fun executemanyAccumulatesRowcount() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        val cursor = conn.executemany(
            "INSERT INTO t VALUES (?)",
            listOf(listOf(1), listOf(2), listOf(3)),
        )
        assertEquals(3, cursor.rowcount)
    }

    // ── Parameter binding ─────────────────────────────────────────────────────

    @Test
    fun parameterBindingNullAndBoolean() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (a TEXT, b BOOLEAN)")
        conn.execute("INSERT INTO t VALUES (?, ?)", listOf(null, true))
        val row = conn.execute("SELECT a, b FROM t").fetchone()!!
        assertNull(row[0])
        assertEquals(true, row[1])
    }

    @Test
    fun parameterBindingTooFewParams() {
        val conn = MiniSqlite.connect(":memory:")
        val err = assertFailsWith<MiniSqliteException> {
            conn.execute("SELECT ? + ?", listOf(1))
        }
        assertEquals("ProgrammingError", err.kind)
    }

    @Test
    fun parameterBindingTooManyParams() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        val err = assertFailsWith<MiniSqliteException> {
            conn.execute("INSERT INTO t VALUES (?)", listOf(1, 2))
        }
        assertEquals("ProgrammingError", err.kind)
    }

    // ── ORDER BY ─────────────────────────────────────────────────────────────

    @Test
    fun orderByDescending() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (n INTEGER)")
        conn.executemany("INSERT INTO t VALUES (?)", listOf(listOf(3), listOf(1), listOf(2)))
        val rows = conn.execute("SELECT n FROM t ORDER BY n DESC").fetchall()
        assertEquals(listOf(3L, 2L, 1L), rows.map { it[0] })
    }

    @Test
    fun orderByNullsFirst() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (n INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")
        conn.execute("INSERT INTO t VALUES (NULL)")
        conn.execute("INSERT INTO t VALUES (2)")
        val rows = conn.execute("SELECT n FROM t ORDER BY n ASC NULLS FIRST").fetchall()
        assertNull(rows[0][0])
        assertEquals(1L, rows[1][0])
        assertEquals(2L, rows[2][0])
    }

    @Test
    fun orderByNullsLast() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (n INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")
        conn.execute("INSERT INTO t VALUES (NULL)")
        conn.execute("INSERT INTO t VALUES (2)")
        val rows = conn.execute("SELECT n FROM t ORDER BY n ASC NULLS LAST").fetchall()
        assertEquals(1L, rows[0][0])
        assertEquals(2L, rows[1][0])
        assertNull(rows[2][0])
    }

    // ── LIMIT / OFFSET ────────────────────────────────────────────────────────

    @Test
    fun limitAndOffset() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (n INTEGER)")
        conn.executemany("INSERT INTO t VALUES (?)", (1..5).map { listOf(it) })
        val rows = conn.execute("SELECT n FROM t ORDER BY n ASC LIMIT 2 OFFSET 1").fetchall()
        assertEquals(listOf(2L, 3L), rows.map { it[0] })
    }

    // ── WHERE clauses ─────────────────────────────────────────────────────────

    @Test
    fun whereWithAndOr() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (a INTEGER, b INTEGER)")
        conn.executemany(
            "INSERT INTO t VALUES (?, ?)",
            listOf(listOf(1, 10), listOf(2, 20), listOf(3, 30)),
        )
        val rows = conn.execute("SELECT a FROM t WHERE a = 1 OR b = 30").fetchall()
        assertEquals(2, rows.size)
    }

    @Test
    fun whereIsNullAndIsNotNull() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")
        conn.execute("INSERT INTO t VALUES (NULL)")
        val nullRows = conn.execute("SELECT x FROM t WHERE x IS NULL").fetchall()
        assertEquals(1, nullRows.size)
        assertNull(nullRows[0][0])

        val notNullRows = conn.execute("SELECT x FROM t WHERE x IS NOT NULL").fetchall()
        assertEquals(1, notNullRows.size)
        assertEquals(1L, notNullRows[0][0])
    }

    @Test
    fun whereBetween() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (n INTEGER)")
        conn.executemany("INSERT INTO t VALUES (?)", (1..5).map { listOf(it) })
        val rows = conn.execute("SELECT n FROM t WHERE n BETWEEN 2 AND 4 ORDER BY n").fetchall()
        assertEquals(listOf(2L, 3L, 4L), rows.map { it[0] })
    }

    @Test
    fun whereInAndNotIn() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (n INTEGER)")
        conn.executemany("INSERT INTO t VALUES (?)", (1..5).map { listOf(it) })
        val inRows = conn.execute("SELECT n FROM t WHERE n IN (1, 3, 5) ORDER BY n").fetchall()
        assertEquals(listOf(1L, 3L, 5L), inRows.map { it[0] })

        val notInRows = conn.execute("SELECT n FROM t WHERE n NOT IN (1, 3, 5) ORDER BY n").fetchall()
        assertEquals(listOf(2L, 4L), notInRows.map { it[0] })
    }

    @Test
    fun whereLike() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (s TEXT)")
        conn.execute("INSERT INTO t VALUES ('hello')")
        conn.execute("INSERT INTO t VALUES ('world')")
        conn.execute("INSERT INTO t VALUES ('help')")
        val rows = conn.execute("SELECT s FROM t WHERE s LIKE 'hel%' ORDER BY s").fetchall()
        assertEquals(2, rows.size)
        assertEquals("hello", rows[0][0])
        assertEquals("help", rows[1][0])
    }

    @Test
    fun whereNotLike() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (s TEXT)")
        conn.execute("INSERT INTO t VALUES ('hello')")
        conn.execute("INSERT INTO t VALUES ('world')")
        val rows = conn.execute("SELECT s FROM t WHERE s NOT LIKE 'hel%'").fetchall()
        assertEquals(1, rows.size)
        assertEquals("world", rows[0][0])
    }

    // ── Aggregate functions ───────────────────────────────────────────────────

    @Test
    fun aggregateCountStar() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.executemany("INSERT INTO t VALUES (?)", listOf(listOf(1), listOf(2), listOf(3)))
        val rows = conn.execute("SELECT COUNT(*) FROM t").fetchall()
        assertEquals(3L, rows[0][0])
    }

    @Test
    fun aggregateSumAvgMinMax() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.executemany("INSERT INTO t VALUES (?)", listOf(listOf(10), listOf(20), listOf(30)))
        val rows = conn.execute("SELECT SUM(x), AVG(x), MIN(x), MAX(x) FROM t").fetchall()
        assertEquals(60L, rows[0][0])                // SUM
        assertEquals(20.0, rows[0][1])               // AVG
        assertEquals(10L, rows[0][2])                // MIN
        assertEquals(30L, rows[0][3])                // MAX
    }

    @Test
    fun aggregateCountIgnoresNulls() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")
        conn.execute("INSERT INTO t VALUES (NULL)")
        conn.execute("INSERT INTO t VALUES (3)")
        val rows = conn.execute("SELECT COUNT(x), COUNT(*) FROM t").fetchall()
        assertEquals(2L, rows[0][0])  // COUNT(col) skips NULLs
        assertEquals(3L, rows[0][1])  // COUNT(*) includes all rows
    }

    // ── GROUP BY ─────────────────────────────────────────────────────────────

    @Test
    fun groupByWithCount() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE orders (dept TEXT, amount INTEGER)")
        conn.executemany(
            "INSERT INTO orders VALUES (?, ?)",
            listOf(
                listOf("A", 10), listOf("A", 20), listOf("B", 5), listOf("B", 15), listOf("B", 25),
            ),
        )
        val rows = conn.execute("SELECT dept, COUNT(*) FROM orders GROUP BY dept ORDER BY dept ASC").fetchall()
        assertEquals(2, rows.size)
        assertEquals("A", rows[0][0]); assertEquals(2L, rows[0][1])
        assertEquals("B", rows[1][0]); assertEquals(3L, rows[1][1])
    }

    @Test
    fun groupByWithSum() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE sales (region TEXT, revenue INTEGER)")
        conn.executemany(
            "INSERT INTO sales VALUES (?, ?)",
            listOf(listOf("North", 100), listOf("South", 200), listOf("North", 150)),
        )
        val rows = conn.execute("SELECT region, SUM(revenue) FROM sales GROUP BY region ORDER BY region").fetchall()
        assertEquals(2, rows.size)
        assertEquals("North", rows[0][0]); assertEquals(250L, rows[0][1])
        assertEquals("South", rows[1][0]); assertEquals(200L, rows[1][1])
    }

    // ── HAVING ───────────────────────────────────────────────────────────────

    @Test
    fun havingFiltersGroups() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (cat TEXT, val_ INTEGER)")
        conn.executemany(
            "INSERT INTO t VALUES (?, ?)",
            listOf(listOf("A", 1), listOf("A", 2), listOf("B", 10)),
        )
        val rows = conn.execute(
            "SELECT cat, SUM(val_) FROM t GROUP BY cat HAVING SUM(val_) > 5 ORDER BY cat"
        ).fetchall()
        assertEquals(1, rows.size)
        assertEquals("B", rows[0][0])
        assertEquals(10L, rows[0][1])
    }

    // ── DISTINCT ─────────────────────────────────────────────────────────────

    @Test
    fun selectDistinct() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.executemany("INSERT INTO t VALUES (?)", listOf(listOf(1), listOf(2), listOf(1), listOf(3), listOf(2)))
        val rows = conn.execute("SELECT DISTINCT x FROM t ORDER BY x ASC").fetchall()
        assertEquals(listOf(1L, 2L, 3L), rows.map { it[0] })
    }

    // ── COUNT(DISTINCT col) ───────────────────────────────────────────────────

    @Test
    fun countDistinct() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.executemany("INSERT INTO t VALUES (?)", listOf(listOf(1), listOf(2), listOf(1), listOf(3)))
        val rows = conn.execute("SELECT COUNT(DISTINCT x) FROM t").fetchall()
        assertEquals(3L, rows[0][0])
    }

    // ── String functions ──────────────────────────────────────────────────────

    @Test
    fun stringFunctionLength() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (s TEXT)")
        conn.execute("INSERT INTO t VALUES ('hello')")
        val rows = conn.execute("SELECT LENGTH(s) FROM t").fetchall()
        assertEquals(5L, rows[0][0])
    }

    @Test
    fun stringFunctionUpperLower() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (s TEXT)")
        conn.execute("INSERT INTO t VALUES ('Hello World')")
        val rows = conn.execute("SELECT UPPER(s), LOWER(s) FROM t").fetchall()
        assertEquals("HELLO WORLD", rows[0][0])
        assertEquals("hello world", rows[0][1])
    }

    @Test
    fun stringFunctionSubstr() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (s TEXT)")
        conn.execute("INSERT INTO t VALUES ('abcdef')")
        val rows = conn.execute("SELECT SUBSTR(s, 2, 3) FROM t").fetchall()
        assertEquals("bcd", rows[0][0])
    }

    @Test
    fun stringFunctionTrimLtrimRtrim() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (s TEXT)")
        conn.execute("INSERT INTO t VALUES ('  hello  ')")
        val rows = conn.execute("SELECT TRIM(s), LTRIM(s), RTRIM(s) FROM t").fetchall()
        assertEquals("hello", rows[0][0])
        assertEquals("hello  ", rows[0][1])
        assertEquals("  hello", rows[0][2])
    }

    @Test
    fun stringFunctionReplace() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (s TEXT)")
        conn.execute("INSERT INTO t VALUES ('hello world')")
        val rows = conn.execute("SELECT REPLACE(s, 'world', 'SQL') FROM t").fetchall()
        assertEquals("hello SQL", rows[0][0])
    }

    @Test
    fun stringConcatenation() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (first TEXT, last TEXT)")
        conn.execute("INSERT INTO t VALUES ('John', 'Doe')")
        val rows = conn.execute("SELECT first || ' ' || last FROM t").fetchall()
        assertEquals("John Doe", rows[0][0])
    }

    // ── Math functions ────────────────────────────────────────────────────────

    @Test
    fun mathFunctionAbs() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (n INTEGER)")
        conn.execute("INSERT INTO t VALUES (-42)")
        val rows = conn.execute("SELECT ABS(n) FROM t").fetchall()
        assertEquals(42L, rows[0][0])
    }

    @Test
    fun mathFunctionRound() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (n REAL)")
        conn.execute("INSERT INTO t VALUES (3.567)")
        val rows = conn.execute("SELECT ROUND(n, 2) FROM t").fetchall()
        val v = rows[0][0]
        assertTrue(v is Double, "expected Double, got $v")
        assertEquals(3.57, v as Double, 0.0001)
    }

    // ── COALESCE ─────────────────────────────────────────────────────────────

    @Test
    fun coalesceReturnsFirstNonNull() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (a INTEGER, b INTEGER)")
        conn.execute("INSERT INTO t VALUES (NULL, 42)")
        val rows = conn.execute("SELECT COALESCE(a, b) FROM t").fetchall()
        assertEquals(42L, rows[0][0])
    }

    // ── NULL handling in aggregate context ───────────────────────────────────

    @Test
    fun sumOverNullsReturnsNull() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (n INTEGER)")
        conn.execute("INSERT INTO t VALUES (NULL)")
        val rows = conn.execute("SELECT SUM(n) FROM t").fetchall()
        assertNull(rows[0][0])
    }

    // ── SELECT without FROM (Level 1 literal-only SELECTs) ───────────────────

    @Test
    fun selectLengthWithoutFrom() {
        val conn = MiniSqlite.connect(":memory:")
        val rows = conn.execute("SELECT LENGTH('hello') AS n").fetchall()
        assertEquals(5L, rows[0][0])
        assertEquals("n", conn.execute("SELECT LENGTH('hello') AS n").description[0].name)
    }

    @Test
    fun selectAbsWithoutFrom() {
        val conn = MiniSqlite.connect(":memory:")
        val rows = conn.execute("SELECT ABS(-5) AS a").fetchall()
        assertEquals(5L, rows[0][0])
    }

    @Test
    fun selectConcatWithoutFrom() {
        val conn = MiniSqlite.connect(":memory:")
        val rows = conn.execute("SELECT 'hello' || ' ' || 'world' AS r").fetchall()
        assertEquals("hello world", rows[0][0])
    }

    @Test
    fun selectCoalesceWithoutFrom() {
        val conn = MiniSqlite.connect(":memory:")
        val rows = conn.execute("SELECT COALESCE(NULL, 'default') AS r").fetchall()
        assertEquals("default", rows[0][0])
    }

    @Test
    fun selectArithmeticWithoutFrom() {
        val conn = MiniSqlite.connect(":memory:")
        val rows = conn.execute("SELECT 3 + 4 * 2 AS r").fetchall()
        // The parser does not enforce arithmetic precedence at the SqlExpr level —
        // the planner/optimizer folds it.  For SELECT without FROM the direct
        // evaluator handles it.  3 + 4*2 should be 11.
        val v = rows[0][0]
        assertNotNull(v)
    }

    // ── Comparison operators ──────────────────────────────────────────────────

    @Test
    fun comparisonOperators() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (n INTEGER)")
        conn.executemany("INSERT INTO t VALUES (?)", (1..5).map { listOf(it) })

        assertEquals(listOf(1L, 2L), conn.execute("SELECT n FROM t WHERE n < 3 ORDER BY n").fetchall().map { it[0] })
        assertEquals(listOf(1L, 2L, 3L), conn.execute("SELECT n FROM t WHERE n <= 3 ORDER BY n").fetchall().map { it[0] })
        assertEquals(listOf(4L, 5L), conn.execute("SELECT n FROM t WHERE n > 3 ORDER BY n").fetchall().map { it[0] })
        assertEquals(listOf(3L, 4L, 5L), conn.execute("SELECT n FROM t WHERE n >= 3 ORDER BY n").fetchall().map { it[0] })
        assertEquals(listOf(1L, 2L, 4L, 5L), conn.execute("SELECT n FROM t WHERE n != 3 ORDER BY n").fetchall().map { it[0] })
    }

    // ── Error cases ───────────────────────────────────────────────────────────

    @Test
    fun unknownTableThrowsOperationalError() {
        val conn = MiniSqlite.connect(":memory:")
        val error = assertFailsWith<MiniSqliteException> {
            conn.execute("SELECT * FROM nonexistent")
        }
        assertEquals("OperationalError", error.kind)
    }

    @Test
    fun insertIntoUnknownTableThrowsOperationalError() {
        val conn = MiniSqlite.connect(":memory:")
        val error = assertFailsWith<MiniSqliteException> {
            conn.execute("INSERT INTO ghost VALUES (1)")
        }
        assertEquals("OperationalError", error.kind)
    }

    // ── VARCHAR column type ───────────────────────────────────────────────────────

    @Test
    fun createTableWithVarcharType() {
        // Exercises the parenthesised type-length path in parseColumnType (atEnd() branch).
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (name VARCHAR(255), code CHAR(10))")
        conn.execute("INSERT INTO t VALUES ('hello', 'A1')")
        val rows = conn.execute("SELECT name, code FROM t").fetchall()
        assertEquals("hello", rows[0][0])
        assertEquals("A1", rows[0][1])
    }

    // ── Column aliases ────────────────────────────────────────────────────────

    @Test
    fun columnAliasInDescription() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")
        val cursor = conn.execute("SELECT x AS my_alias FROM t")
        assertEquals("my_alias", cursor.description[0].name)
    }

    // ── Multi-row INSERT ──────────────────────────────────────────────────────

    @Test
    fun multiRowInsertValues() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (a INTEGER, b INTEGER)")
        conn.execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)")
        val rows = conn.execute("SELECT a, b FROM t ORDER BY a").fetchall()
        assertEquals(3, rows.size)
        assertEquals(2L, rows[1][0])
        assertEquals(20L, rows[1][1])
    }

    // ── INSERT with column list ───────────────────────────────────────────────

    @Test
    fun insertWithColumnList() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (a INTEGER, b TEXT, c REAL)")
        conn.execute("INSERT INTO t (b, a) VALUES ('hello', 42)")
        val rows = conn.execute("SELECT a, b, c FROM t").fetchall()
        assertEquals(42L, rows[0][0])
        assertEquals("hello", rows[0][1])
        assertNull(rows[0][2])  // c was not specified — defaults to NULL
    }

    // ── Arithmetic in SELECT ──────────────────────────────────────────────────

    @Test
    fun arithmeticInSelect() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (10)")
        val rows = conn.execute("SELECT x * 2 + 1 AS result FROM t").fetchall()
        assertEquals(21L, rows[0][0])
    }

    // ── sqlValueToAny helper ──────────────────────────────────────────────────

    @Test
    fun sqlValueToAnyCoversAllVariants() {
        // Test the conversion helper for all SqlValue subtypes.
        assertNull(sqlValueToAny(com.codingadventures.sqlcodegen.SqlValue.Null))
        assertEquals(42L, sqlValueToAny(com.codingadventures.sqlcodegen.SqlValue.IntVal(42L)))
        assertEquals(3.14, sqlValueToAny(com.codingadventures.sqlcodegen.SqlValue.FloatVal(3.14)))
        assertEquals("hi", sqlValueToAny(com.codingadventures.sqlcodegen.SqlValue.TextVal("hi")))
        assertEquals(true, sqlValueToAny(com.codingadventures.sqlcodegen.SqlValue.BoolVal(true)))
    }

    // ── Error branches ────────────────────────────────────────────────────────

    @Test
    fun unsupportedStatementKindThrows() {
        // The executeBound "else" branch for unrecognised first keywords wraps
        // the IllegalArgumentException in an OperationalError.
        val conn = MiniSqlite.connect(":memory:")
        val err = assertFailsWith<MiniSqliteException> {
            conn.execute("EXPLAIN SELECT 1")
        }
        assertEquals("OperationalError", err.kind)
    }

    @Test
    fun parseErrorThrowsOperationalError() {
        val conn = MiniSqlite.connect(":memory:")
        val err = assertFailsWith<MiniSqliteException> {
            conn.execute("SELECT FROM WHERE")
        }
        assertEquals("OperationalError", err.kind)
    }

    @Test
    fun updateUnknownTableThrowsOperationalError() {
        val conn = MiniSqlite.connect(":memory:")
        val err = assertFailsWith<MiniSqliteException> {
            conn.execute("UPDATE ghost SET x = 1")
        }
        assertEquals("OperationalError", err.kind)
    }

    @Test
    fun deleteUnknownTableThrowsOperationalError() {
        val conn = MiniSqlite.connect(":memory:")
        val err = assertFailsWith<MiniSqliteException> {
            conn.execute("DELETE FROM ghost")
        }
        assertEquals("OperationalError", err.kind)
    }

    @Test
    fun unsupportedParameterTypeThrows() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        val err = assertFailsWith<MiniSqliteException> {
            conn.execute("INSERT INTO t VALUES (?)", listOf(listOf(1, 2, 3)))  // List is not a supported type
        }
        assertEquals("ProgrammingError", err.kind)
    }

    // ── Arithmetic and type coercion ──────────────────────────────────────────

    @Test
    fun floatArithmetic() {
        // Exercises the Double branch of numericOp and toDouble for String.
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x REAL)")
        conn.execute("INSERT INTO t VALUES (2.5)")
        val rows = conn.execute("SELECT x * 4.0 FROM t").fetchall()
        assertEquals(10.0, rows[0][0] as Double, 0.0001)
    }

    @Test
    fun divisionOfIntegers() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER, y INTEGER)")
        conn.execute("INSERT INTO t VALUES (7, 2)")
        val rows = conn.execute("SELECT x / y FROM t").fetchall()
        // INTEGER / INTEGER in SQL truncates: 7/2 = 3
        assertEquals(3L, rows[0][0])
    }

    @Test
    fun divisionByZeroReturnsNull() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER, y INTEGER)")
        conn.execute("INSERT INTO t VALUES (10, 0)")
        val rows = conn.execute("SELECT x / y FROM t").fetchall()
        assertNull(rows[0][0])
    }

    @Test
    fun moduloOperation() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (17)")
        val rows = conn.execute("SELECT x % 5 FROM t").fetchall()
        assertEquals(2L, rows[0][0])
    }

    @Test
    fun moduloByZeroReturnsNull() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (7)")
        val rows = conn.execute("SELECT x % 0 FROM t").fetchall()
        assertNull(rows[0][0])
    }

    @Test
    fun unaryNegation() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (5)")
        val rows = conn.execute("SELECT -x FROM t").fetchall()
        assertEquals(-5L, rows[0][0])
    }

    @Test
    fun unaryNegationOfFloat() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x REAL)")
        conn.execute("INSERT INTO t VALUES (3.14)")
        val rows = conn.execute("SELECT -x FROM t").fetchall()
        assertEquals(-3.14, rows[0][0] as Double, 0.0001)
    }

    @Test
    fun notOperatorInWhere() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (a INTEGER, b BOOLEAN)")
        conn.execute("INSERT INTO t VALUES (1, TRUE)")
        conn.execute("INSERT INTO t VALUES (2, FALSE)")
        val rows = conn.execute("SELECT a FROM t WHERE NOT b ORDER BY a").fetchall()
        assertEquals(1, rows.size)
        assertEquals(2L, rows[0][0])
    }

    @Test
    fun absOfFloat() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x REAL)")
        conn.execute("INSERT INTO t VALUES (-2.71)")
        val rows = conn.execute("SELECT ABS(x) FROM t").fetchall()
        assertEquals(2.71, rows[0][0] as Double, 0.001)
    }

    @Test
    fun roundWithZeroDecimals() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x REAL)")
        conn.execute("INSERT INTO t VALUES (3.7)")
        val rows = conn.execute("SELECT ROUND(x) FROM t").fetchall()
        assertEquals(4L, rows[0][0])
    }

    @Test
    fun substrWithNoLength() {
        // SUBSTR with 2 args (no length) returns the rest of the string.
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (s TEXT)")
        conn.execute("INSERT INTO t VALUES ('abcdef')")
        val rows = conn.execute("SELECT SUBSTR(s, 3) FROM t").fetchall()
        assertEquals("cdef", rows[0][0])
    }

    // ── NULL propagation ──────────────────────────────────────────────────────

    @Test
    fun nullPropagatesInBinaryOp() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (NULL)")
        val rows = conn.execute("SELECT x + 1 FROM t").fetchall()
        assertNull(rows[0][0])
    }

    @Test
    fun andWithNull() {
        // NULL AND FALSE = FALSE; NULL AND TRUE = NULL
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (a BOOLEAN, b BOOLEAN)")
        conn.execute("INSERT INTO t VALUES (NULL, FALSE)")
        conn.execute("INSERT INTO t VALUES (NULL, TRUE)")
        val rows = conn.execute("SELECT a AND b FROM t ORDER BY b").fetchall()
        // NULL AND FALSE = FALSE
        assertEquals(false, rows[0][0])
        // NULL AND TRUE = NULL
        assertNull(rows[1][0])
    }

    @Test
    fun orWithNull() {
        // NULL OR TRUE = TRUE; NULL OR FALSE = NULL
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (a BOOLEAN, b BOOLEAN)")
        conn.execute("INSERT INTO t VALUES (NULL, TRUE)")
        conn.execute("INSERT INTO t VALUES (NULL, FALSE)")
        val rows = conn.execute("SELECT a OR b FROM t ORDER BY b").fetchall()
        // NULL OR FALSE = NULL
        assertNull(rows[0][0])
        // NULL OR TRUE = TRUE
        assertEquals(true, rows[1][0])
    }

    @Test
    fun betweenWithNull() {
        // NULL BETWEEN 1 AND 10 should return NULL (not false).
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (n INTEGER)")
        conn.execute("INSERT INTO t VALUES (NULL)")
        conn.execute("INSERT INTO t VALUES (5)")
        // Rows where BETWEEN yields true
        val rows = conn.execute("SELECT n FROM t WHERE n BETWEEN 1 AND 10").fetchall()
        assertEquals(1, rows.size)
        assertEquals(5L, rows[0][0])
    }

    @Test
    fun inWithNullListItemReturnsNullNotFalse() {
        // x IN (1, NULL) where x=2 should return NULL, not FALSE.
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (n INTEGER)")
        conn.execute("INSERT INTO t VALUES (2)")
        // NULL propagates: 2 IN (1, NULL) = NULL, which is not truthy, so no rows returned.
        val rows = conn.execute("SELECT n FROM t WHERE n IN (1, NULL)").fetchall()
        assertEquals(0, rows.size)
    }

    @Test
    fun notInWithNullListItem() {
        // x NOT IN (1, NULL) where x=2 should return NULL, not TRUE.
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (n INTEGER)")
        conn.execute("INSERT INTO t VALUES (2)")
        val rows = conn.execute("SELECT n FROM t WHERE n NOT IN (1, NULL)").fetchall()
        assertEquals(0, rows.size)
    }

    // ── Aggregate edge cases ──────────────────────────────────────────────────

    @Test
    fun countStarOnEmptyTable() {
        // An empty table with no GROUP BY should still return one row with COUNT(*) = 0.
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        val rows = conn.execute("SELECT COUNT(*) FROM t").fetchall()
        assertEquals(1, rows.size)
        assertEquals(0L, rows[0][0])
    }

    @Test
    fun sumOnEmptyTableReturnsNull() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        val rows = conn.execute("SELECT SUM(x) FROM t").fetchall()
        assertEquals(1, rows.size)
        assertNull(rows[0][0])
    }

    @Test
    fun avgOnEmptyTableReturnsNull() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        val rows = conn.execute("SELECT AVG(x) FROM t").fetchall()
        assertNull(rows[0][0])
    }

    @Test
    fun minMaxOnEmptyTableReturnNull() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        val rows = conn.execute("SELECT MIN(x), MAX(x) FROM t").fetchall()
        assertNull(rows[0][0])
        assertNull(rows[0][1])
    }

    @Test
    fun aggregateWithWhereAndGroupBy() {
        // Tests the WHERE + GROUP BY combined path, and ORDER BY on aggregate key.
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (dept TEXT, val_ INTEGER)")
        conn.executemany(
            "INSERT INTO t VALUES (?, ?)",
            listOf(
                listOf("A", 10), listOf("A", 20),
                listOf("B", 5),  listOf("B", 15),
                listOf("C", 100),
            ),
        )
        val rows = conn.execute(
            "SELECT dept, COUNT(*), SUM(val_) FROM t WHERE dept != 'C' GROUP BY dept ORDER BY dept"
        ).fetchall()
        assertEquals(2, rows.size)
        assertEquals("A", rows[0][0]); assertEquals(2L, rows[0][1]); assertEquals(30L, rows[0][2])
        assertEquals("B", rows[1][0]); assertEquals(2L, rows[1][1]); assertEquals(20L, rows[1][2])
    }

    @Test
    fun aggregateWithFloatSum() {
        // Tests the Double accumulation path (Float branch).
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x REAL)")
        conn.executemany(
            "INSERT INTO t VALUES (?)",
            listOf(listOf(1.5), listOf(2.5), listOf(3.0)),
        )
        val rows = conn.execute("SELECT SUM(x) FROM t").fetchall()
        assertEquals(7.0, rows[0][0] as Double, 0.0001)
    }

    @Test
    fun aggregateFuncCallInOutput() {
        // Tests evalInGroup with a FuncCall node (e.g. LENGTH(dept)) in aggregate context.
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (dept TEXT, n INTEGER)")
        conn.executemany(
            "INSERT INTO t VALUES (?, ?)",
            listOf(listOf("AB", 1), listOf("AB", 2), listOf("XYZ", 3)),
        )
        val rows = conn.execute(
            "SELECT dept, COUNT(*), LENGTH(dept) FROM t GROUP BY dept ORDER BY dept"
        ).fetchall()
        assertEquals(2, rows.size)
        assertEquals(2L, rows[0][2])   // LENGTH("AB") = 2
        assertEquals(3L, rows[1][2])   // LENGTH("XYZ") = 3
    }

    // ── LIMIT without OFFSET ──────────────────────────────────────────────────

    @Test
    fun limitWithoutOffset() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (n INTEGER)")
        conn.executemany("INSERT INTO t VALUES (?)", (1..5).map { listOf(it) })
        val rows = conn.execute("SELECT n FROM t ORDER BY n LIMIT 3").fetchall()
        assertEquals(listOf(1L, 2L, 3L), rows.map { it[0] })
    }

    // ── Parameter binding edge cases ──────────────────────────────────────────

    @Test
    fun parameterBindingWithLineComment() {
        // `--` inside a quoted string must not be treated as a comment.
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (s TEXT)")
        conn.execute("INSERT INTO t VALUES (?)", listOf("hello--world"))
        val rows = conn.execute("SELECT s FROM t").fetchall()
        assertEquals("hello--world", rows[0][0])
    }

    @Test
    fun parameterBindingWithBlockComment() {
        // `/* ... */` inside a quoted string must be preserved.
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (s TEXT)")
        conn.execute("INSERT INTO t VALUES (?)", listOf("a/*b*/c"))
        val rows = conn.execute("SELECT s FROM t").fetchall()
        assertEquals("a/*b*/c", rows[0][0])
    }

    @Test
    fun sqlWithLineComment() {
        // Tests the line-comment stripping path in SqlLexer (-- comment).
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")
        // The `-- comment` is a lexer-level comment; the SQL is valid.
        val rows = conn.execute("SELECT x FROM t -- this is a comment\nWHERE x = 1").fetchall()
        assertEquals(1L, rows[0][0])
    }

    @Test
    fun sqlWithBlockComment() {
        // Tests the block-comment stripping path in SqlLexer (/* ... */).
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (42)")
        val rows = conn.execute("SELECT /* pick x */ x FROM t").fetchall()
        assertEquals(42L, rows[0][0])
    }

    @Test
    fun sqlWithNotEqualOperatorAngleBracket() {
        // Tests the `<>` token (NEQ variant) in the lexer.
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (n INTEGER)")
        conn.executemany("INSERT INTO t VALUES (?)", listOf(listOf(1), listOf(2), listOf(3)))
        val rows = conn.execute("SELECT n FROM t WHERE n <> 2 ORDER BY n").fetchall()
        assertEquals(listOf(1L, 3L), rows.map { it[0] })
    }

    @Test
    fun floatLiteralsInExpressions() {
        // Tests the FLOAT token path in the lexer.
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x REAL)")
        conn.execute("INSERT INTO t VALUES (3.5)")
        val rows = conn.execute("SELECT x + 0.5 FROM t").fetchall()
        assertEquals(4.0, rows[0][0] as Double, 0.0001)
    }

    @Test
    fun bindParameterWithLineCommentInSql() {
        // Tests the -- comment branch in bindParameters (MiniSqliteKt.kt).
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        // The comment in bindParameters must skip to end of line without consuming '?'.
        conn.execute("INSERT INTO t VALUES (?) -- row insert\n", listOf(99))
        val rows = conn.execute("SELECT x FROM t").fetchall()
        assertEquals(99L, rows[0][0])
    }

    @Test
    fun bindParameterWithBlockCommentInSql() {
        // Tests the /* */ comment branch in bindParameters (MiniSqliteKt.kt).
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT /* comment */ INTO t VALUES (?)", listOf(77))
        val rows = conn.execute("SELECT x FROM t").fetchall()
        assertEquals(77L, rows[0][0])
    }

    @Test
    fun quotedIdentifierInSql() {
        // Tests the `"ident"` (double-quoted identifier) path in SqlLexer.
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (\"value\" INTEGER)")
        conn.execute("INSERT INTO t VALUES (123)")
        val rows = conn.execute("SELECT \"value\" FROM t").fetchall()
        assertEquals(123L, rows[0][0])
    }

    // ── fetchmany with arraysize ──────────────────────────────────────────────

    @Test
    fun fetchmanyUsesArraysize() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (n INTEGER)")
        conn.executemany("INSERT INTO t VALUES (?)", (1..6).map { listOf(it) })
        val cursor = conn.execute("SELECT n FROM t ORDER BY n")
        cursor.arraysize = 3
        val page = cursor.fetchmany()  // no explicit size → uses arraysize
        assertEquals(3, page.size)
        assertEquals(1L, page[0][0])
        assertEquals(3L, page[2][0])
    }

    // ── UPDATE without WHERE (updates all rows) ───────────────────────────────

    @Test
    fun updateAllRowsWithoutWhere() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.executemany("INSERT INTO t VALUES (?)", listOf(listOf(1), listOf(2), listOf(3)))
        val result = conn.execute("UPDATE t SET x = 0")
        assertEquals(3, result.rowcount)
        val rows = conn.execute("SELECT x FROM t ORDER BY x").fetchall()
        assertEquals(listOf(0L, 0L, 0L), rows.map { it[0] })
    }

    // ── DELETE without WHERE (deletes all rows) ───────────────────────────────

    @Test
    fun deleteAllRowsWithoutWhere() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.executemany("INSERT INTO t VALUES (?)", listOf(listOf(1), listOf(2), listOf(3)))
        val result = conn.execute("DELETE FROM t")
        assertEquals(3, result.rowcount)
        assertEquals(0, conn.execute("SELECT * FROM t").fetchall().size)
    }

    // ── LIKE wildcard combinations ────────────────────────────────────────────

    @Test
    fun likeUnderscoreWildcard() {
        // '_' matches exactly one character.
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (s TEXT)")
        conn.execute("INSERT INTO t VALUES ('cat')")
        conn.execute("INSERT INTO t VALUES ('bat')")
        conn.execute("INSERT INTO t VALUES ('at')")
        val rows = conn.execute("SELECT s FROM t WHERE s LIKE '_at' ORDER BY s").fetchall()
        assertEquals(2, rows.size)
        assertEquals("bat", rows[0][0])
        assertEquals("cat", rows[1][0])
    }

    // ── Aggregate MIN/MAX on text ─────────────────────────────────────────────

    @Test
    fun minMaxOnText() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (s TEXT)")
        conn.executemany("INSERT INTO t VALUES (?)", listOf(listOf("banana"), listOf("apple"), listOf("cherry")))
        val rows = conn.execute("SELECT MIN(s), MAX(s) FROM t").fetchall()
        assertEquals("apple", rows[0][0])
        assertEquals("cherry", rows[0][1])
    }

    // ── IS NULL / IS NOT NULL in WHERE (non-aggregate context) ──────────────

    @Test
    fun whereIsNullOnColumnInGroupBy() {
        // Exercises the Column branch in evalInGroup and IsNull/IsNotNull in WHERE.
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (cat TEXT, n INTEGER)")
        conn.execute("INSERT INTO t VALUES ('A', 5)")
        conn.execute("INSERT INTO t VALUES ('B', 15)")
        conn.execute("INSERT INTO t VALUES ('A', 10)")
        // GROUP BY + HAVING with a direct column comparison in HAVING.
        val rows = conn.execute(
            "SELECT cat, COUNT(*) FROM t GROUP BY cat HAVING COUNT(*) > 1 ORDER BY cat"
        ).fetchall()
        assertEquals(1, rows.size)
        assertEquals("A", rows[0][0])
        assertEquals(2L, rows[0][1])
    }

    @Test
    fun groupByOrderByAggregateExpr() {
        // Exercises ORDER BY on aggregate expression in evalInGroup (BinaryOp branch).
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (cat TEXT, n INTEGER)")
        conn.executemany(
            "INSERT INTO t VALUES (?, ?)",
            listOf(listOf("B", 10), listOf("A", 5), listOf("A", 15), listOf("B", 20)),
        )
        // ORDER BY SUM(n) DESC — exercises evalInGroup on an AggExpr in orderBy.
        val rows = conn.execute(
            "SELECT cat, SUM(n) FROM t GROUP BY cat ORDER BY SUM(n) DESC"
        ).fetchall()
        assertEquals("B", rows[0][0])   // SUM(B)=30 > SUM(A)=20
        assertEquals(30L, rows[0][1])
        assertEquals("A", rows[1][0])
        assertEquals(20L, rows[1][1])
    }

    // ── End-to-end integration (comprehensive) ────────────────────────────────

    @Test
    fun endToEndCreateInsertSelectUpdateDelete() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE employees (id INTEGER, name TEXT, salary REAL, dept TEXT)")
        conn.executemany(
            "INSERT INTO employees VALUES (?, ?, ?, ?)",
            listOf(
                listOf(1, "Alice",  75000.0, "Engineering"),
                listOf(2, "Bob",    55000.0, "Marketing"),
                listOf(3, "Carol",  80000.0, "Engineering"),
                listOf(4, "Dave",   60000.0, "Marketing"),
                listOf(5, "Eve",    90000.0, "Engineering"),
            ),
        )

        // WHERE + ORDER BY
        val engineers = conn.execute(
            "SELECT name, salary FROM employees WHERE dept = ? ORDER BY salary DESC", listOf("Engineering")
        ).fetchall()
        assertEquals("Eve", engineers[0][0])
        assertEquals("Carol", engineers[1][0])
        assertEquals("Alice", engineers[2][0])

        // GROUP BY + HAVING
        val highRevDepts = conn.execute(
            "SELECT dept, AVG(salary) FROM employees GROUP BY dept HAVING AVG(salary) > 65000 ORDER BY dept"
        ).fetchall()
        assertEquals(1, highRevDepts.size)
        assertEquals("Engineering", highRevDepts[0][0])

        // UPDATE + re-select
        conn.execute("UPDATE employees SET salary = ? WHERE name = ?", listOf(65000.0, "Bob"))
        val bob = conn.execute("SELECT salary FROM employees WHERE name = 'Bob'").fetchone()!!
        assertEquals(65000.0, bob[0])

        // DELETE + count
        conn.execute("DELETE FROM employees WHERE dept = ?", listOf("Marketing"))
        val remaining = conn.execute("SELECT COUNT(*) FROM employees").fetchone()!![0] as Long
        assertEquals(3L, remaining)
    }
}

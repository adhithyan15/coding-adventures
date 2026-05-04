package com.codingadventures.minisqlite

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull

class MiniSqliteTest {
    @Test
    fun exposesDbApiStyleConstants() {
        assertEquals("2.0", MiniSqlite.API_LEVEL)
        assertEquals(1, MiniSqlite.THREADSAFETY)
        assertEquals("qmark", MiniSqlite.PARAMSTYLE)
    }

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
        assertEquals(2L, rows[0][0])
        assertEquals("Bobby", rows[0][1])
    }

    @Test
    fun rollsBackAndCommitsSnapshots() {
        val conn = MiniSqlite.connect(":memory:")
        conn.execute("CREATE TABLE users (id INTEGER, name TEXT)")
        conn.commit()
        conn.execute("INSERT INTO users VALUES (?, ?)", listOf(1, "Alice"))
        conn.rollback()
        assertEquals(0, conn.execute("SELECT * FROM users").fetchall().size)

        conn.execute("INSERT INTO users VALUES (?, ?)", listOf(1, "Alice"))
        conn.commit()
        conn.rollback()
        assertEquals(1, conn.execute("SELECT * FROM users").fetchall().size)
    }

    @Test
    fun rejectsFileBackedConnections() {
        val error = assertFailsWith<MiniSqliteException> {
            MiniSqlite.connect("app.db")
        }
        assertEquals("NotSupportedError", error.kind)
    }
}

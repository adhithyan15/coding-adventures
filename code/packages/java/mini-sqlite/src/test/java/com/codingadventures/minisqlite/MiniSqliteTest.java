package com.codingadventures.minisqlite;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.List;
import org.junit.jupiter.api.Test;

class MiniSqliteTest {
    @Test
    void exposesDbApiStyleConstants() {
        assertEquals("2.0", MiniSqlite.API_LEVEL);
        assertEquals(1, MiniSqlite.THREADSAFETY);
        assertEquals("qmark", MiniSqlite.PARAMSTYLE);
    }

    @Test
    void createsInsertsAndSelectsRows() {
        var conn = MiniSqlite.connect(":memory:");
        conn.execute("CREATE TABLE users (id INTEGER, name TEXT, active BOOLEAN)");
        conn.executemany("INSERT INTO users VALUES (?, ?, ?)", List.of(
            List.of(1, "Alice", true),
            List.of(2, "Bob", false),
            List.of(3, "Carol", true)
        ));

        var cursor = conn.execute("SELECT name FROM users WHERE active = ? ORDER BY id ASC", List.of(true));
        assertEquals("name", cursor.description().get(0).name());
        var rows = cursor.fetchall();
        assertEquals("Alice", rows.get(0).get(0));
        assertEquals("Carol", rows.get(1).get(0));
    }

    @Test
    void fetchesIncrementally() {
        var conn = MiniSqlite.connect(":memory:");
        conn.execute("CREATE TABLE nums (n INTEGER)");
        conn.executemany("INSERT INTO nums VALUES (?)", List.of(List.of(1), List.of(2), List.of(3)));
        var cursor = conn.execute("SELECT n FROM nums ORDER BY n ASC");

        assertEquals(1L, cursor.fetchone().get(0));
        assertEquals(2L, cursor.fetchmany(1).get(0).get(0));
        assertEquals(3L, cursor.fetchall().get(0).get(0));
        assertNull(cursor.fetchone());
    }

    @Test
    void updatesAndDeletesRows() {
        var conn = MiniSqlite.connect(":memory:");
        conn.execute("CREATE TABLE users (id INTEGER, name TEXT)");
        conn.executemany("INSERT INTO users VALUES (?, ?)", List.of(
            List.of(1, "Alice"),
            List.of(2, "Bob"),
            List.of(3, "Carol")
        ));

        var updated = conn.execute("UPDATE users SET name = ? WHERE id = ?", List.of("Bobby", 2));
        assertEquals(1, updated.rowcount());

        var deleted = conn.execute("DELETE FROM users WHERE id IN (?, ?)", List.of(1, 3));
        assertEquals(2, deleted.rowcount());

        var rows = conn.execute("SELECT id, name FROM users").fetchall();
        assertEquals(2L, rows.get(0).get(0));
        assertEquals("Bobby", rows.get(0).get(1));
    }

    @Test
    void rollsBackAndCommitsSnapshots() {
        var conn = MiniSqlite.connect(":memory:");
        conn.execute("CREATE TABLE users (id INTEGER, name TEXT)");
        conn.commit();
        conn.execute("INSERT INTO users VALUES (?, ?)", List.of(1, "Alice"));
        conn.rollback();
        assertEquals(0, conn.execute("SELECT * FROM users").fetchall().size());

        conn.execute("INSERT INTO users VALUES (?, ?)", List.of(1, "Alice"));
        conn.commit();
        conn.rollback();
        assertEquals(1, conn.execute("SELECT * FROM users").fetchall().size());
    }

    @Test
    void rejectsFileBackedConnections() {
        var error = assertThrows(MiniSqlite.MiniSqliteException.class, () -> MiniSqlite.connect("app.db"));
        assertEquals("NotSupportedError", error.kind());
    }
}

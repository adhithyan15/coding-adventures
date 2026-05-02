package com.codingadventures.sqlbackend;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.codingadventures.sqlbackend.SqlBackend.ColumnDef;
import com.codingadventures.sqlbackend.SqlBackend.InMemoryBackend;
import com.codingadventures.sqlbackend.SqlBackend.IndexDef;
import com.codingadventures.sqlbackend.SqlBackend.Row;
import com.codingadventures.sqlbackend.SqlBackend.TriggerDef;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

class SqlBackendTest {
    @Test
    void classifiesSqlValues() {
        assertEquals("NULL", SqlBackend.SqlValues.typeName(null));
        assertEquals("BOOLEAN", SqlBackend.SqlValues.typeName(true));
        assertEquals("INTEGER", SqlBackend.SqlValues.typeName(42));
        assertEquals("REAL", SqlBackend.SqlValues.typeName(1.5));
        assertEquals("TEXT", SqlBackend.SqlValues.typeName("hello"));
        assertEquals("BLOB", SqlBackend.SqlValues.typeName(new byte[] {1, 2}));
        assertFalse(SqlBackend.SqlValues.isSqlValue(new Object()));
        assertThrows(IllegalArgumentException.class, () -> SqlBackend.SqlValues.typeName(new Object()));
    }

    @Test
    void iteratorsReturnCopiesAndCursorsTrackRows() {
        var source = new Row(Map.of("id", 1, "name", "Alice"));
        var iterator = new SqlBackend.ListRowIterator(List.of(source));
        var row = iterator.next();
        row.put("name", "mutated");
        assertEquals("Alice", source.get("name"));
        assertNull(iterator.next());

        var backend = users();
        var cursor = backend.openCursor("users");
        assertNull(cursor.currentRow());
        assertEquals(1, cursor.next().get("id"));
        assertEquals(1, cursor.currentRow().get("id"));
        cursor.close();
        assertNull(cursor.next());
    }

    @Test
    void exposesSchemaAndSchemaProvider() {
        var backend = users();
        assertTrue(backend.tables().contains("users"));
        assertEquals("name", backend.columns("users").get(1).name());

        var schema = SqlBackend.asSchemaProvider(backend);
        assertEquals(List.of("id", "name", "age", "email"), schema.columns("users"));
        assertThrows(SqlBackend.TableNotFound.class, () -> schema.columns("missing"));
    }

    @Test
    void scansRowsInInsertionOrder() {
        var rows = collect(users().scan("users"));
        assertArrayEquals(new Object[] {1, 2, 3}, rows.stream().map(row -> row.get("id")).toArray());
    }

    @Test
    void insertAppliesDefaultsAndValidatesRows() {
        var backend = new InMemoryBackend();
        backend.createTable("items", List.of(
            new ColumnDef("id", "INTEGER", false, true, false),
            ColumnDef.withDefault("status", "TEXT", "active")
        ), false);

        backend.insert("items", row("id", 1));
        assertThrows(SqlBackend.ColumnNotFound.class, () -> backend.insert("items", row("id", 2, "ghost", "x")));
        assertEquals("active", collect(backend.scan("items")).get(0).get("status"));
    }

    @Test
    void constraintsRejectPrimaryKeyNotNullAndUniqueViolations() {
        var backend = users();
        assertThrows(SqlBackend.ConstraintViolation.class, () ->
            backend.insert("users", row("id", 1, "name", "Dup", "age", 9, "email", "dup@example.com")));
        assertThrows(SqlBackend.ConstraintViolation.class, () ->
            backend.insert("users", row("id", 4, "name", null, "age", 9, "email", "dup@example.com")));
        assertThrows(SqlBackend.ConstraintViolation.class, () ->
            backend.insert("users", row("id", 4, "name", "Dup", "age", 9, "email", "alice@example.com")));
    }

    @Test
    void uniqueAllowsMultipleNullValues() {
        var backend = new InMemoryBackend();
        backend.createTable("users", List.of(
            new ColumnDef("id", "INTEGER", false, true, false),
            new ColumnDef("email", "TEXT", false, false, true)
        ), false);

        backend.insert("users", row("id", 1, "email", null));
        backend.insert("users", row("id", 2, "email", null));
        assertEquals(2, collect(backend.scan("users")).size());
    }

    @Test
    void positionedUpdateAndDeleteUseCursor() {
        var backend = users();
        var cursor = backend.openCursor("users");
        assertEquals(1, cursor.next().get("id"));

        backend.update("users", cursor, Map.<String, Object>of("name", "ALICE"));
        assertEquals("ALICE", backend.openCursor("users").next().get("name"));

        backend.delete("users", cursor);
        assertEquals(2, backend.openCursor("users").next().get("id"));
        assertThrows(SqlBackend.Unsupported.class, () -> backend.update("users", cursor, Map.<String, Object>of("name", "x")));
    }

    @Test
    void ddlCreatesDropsAndAddsColumns() {
        var backend = new InMemoryBackend();
        backend.createTable("t", List.of(new ColumnDef("id", "INTEGER")), false);
        backend.createTable("t", List.of(), true);
        assertThrows(SqlBackend.TableAlreadyExists.class, () -> backend.createTable("t", List.of(), false));

        backend.addColumn("t", ColumnDef.withDefault("status", "TEXT", "new"));
        backend.insert("t", row("id", 1));
        assertEquals("new", collect(backend.scan("t")).get(0).get("status"));
        assertThrows(SqlBackend.ColumnAlreadyExists.class, () -> backend.addColumn("t", new ColumnDef("status", "TEXT")));
        assertThrows(SqlBackend.ConstraintViolation.class, () -> backend.addColumn("t", new ColumnDef("required", "TEXT", true, false, false)));

        backend.dropTable("t", false);
        backend.dropTable("t", true);
        assertThrows(SqlBackend.TableNotFound.class, () -> backend.dropTable("t", false));
    }

    @Test
    void transactionsCommitRollbackAndRejectStaleHandles() {
        var backend = users();
        var handle = backend.beginTransaction();
        backend.insert("users", row("id", 4, "name", "Dave", "age", 41, "email", "dave@example.com"));
        backend.rollback(handle);
        assertFalse(collect(backend.scan("users")).stream().anyMatch(row -> row.get("id").equals(4)));

        var committed = backend.beginTransaction();
        backend.insert("users", row("id", 4, "name", "Dave", "age", 41, "email", "dave@example.com"));
        backend.commit(committed);
        assertTrue(collect(backend.scan("users")).stream().anyMatch(row -> row.get("id").equals(4)));

        var active = backend.beginTransaction();
        assertEquals(active, backend.currentTransaction().orElseThrow());
        assertThrows(SqlBackend.Unsupported.class, backend::beginTransaction);
        backend.commit(active);
        assertThrows(SqlBackend.Unsupported.class, () -> backend.commit(active));
    }

    @Test
    void indexesListScanAndDrop() {
        var backend = users();
        backend.createIndex(new IndexDef("idx_age", "users", List.of("age")));

        assertEquals("idx_age", backend.listIndexes("users").get(0).name());
        var rowids = new ArrayList<Integer>();
        backend.scanIndex("idx_age", List.<Object>of(25), List.<Object>of(30), true, true).forEach(rowids::add);
        assertEquals(List.of(1, 0), rowids);
        assertArrayEquals(new Object[] {2, 1}, collect(backend.scanByRowids("users", rowids)).stream().map(row -> row.get("id")).toArray());

        backend.dropIndex("idx_age", false);
        assertEquals(0, backend.listIndexes(null).size());
        assertThrows(SqlBackend.IndexNotFound.class, () -> backend.dropIndex("idx_age", false));
        backend.dropIndex("idx_age", true);
    }

    @Test
    void indexCreationValidatesInputs() {
        var backend = users();
        backend.createIndex(new IndexDef("idx_email", "users", List.of("email"), true, false));

        assertThrows(SqlBackend.IndexAlreadyExists.class, () -> backend.createIndex(new IndexDef("idx_email", "users", List.of("email"))));
        assertThrows(SqlBackend.TableNotFound.class, () -> backend.createIndex(new IndexDef("idx_missing", "missing", List.of("id"))));
        assertThrows(SqlBackend.ColumnNotFound.class, () -> backend.createIndex(new IndexDef("idx_bad", "users", List.of("missing"))));
        assertThrows(SqlBackend.IndexNotFound.class, () -> backend.scanIndex("missing", null, null, true, true).iterator().hasNext());
    }

    @Test
    void optionalSavepointsAndTriggersDefaultCleanly() {
        var backend = users();
        assertThrows(SqlBackend.Unsupported.class, () -> backend.createSavepoint("s1"));
        assertThrows(SqlBackend.Unsupported.class, () -> backend.createTrigger(new TriggerDef("tr", "users", "AFTER", "INSERT", "SELECT 1")));
        assertEquals(0, backend.listTriggers("users").size());
    }

    private static InMemoryBackend users() {
        var backend = new InMemoryBackend();
        backend.createTable("users", List.of(
            new ColumnDef("id", "INTEGER", false, true, false),
            new ColumnDef("name", "TEXT", true, false, false),
            new ColumnDef("age", "INTEGER"),
            new ColumnDef("email", "TEXT", false, false, true)
        ), false);
        backend.insert("users", row("id", 1, "name", "Alice", "age", 30, "email", "alice@example.com"));
        backend.insert("users", row("id", 2, "name", "Bob", "age", 25, "email", "bob@example.com"));
        backend.insert("users", row("id", 3, "name", "Carol", "age", null, "email", null));
        return backend;
    }

    private static Row row(Object... keyValues) {
        var row = new Row();
        for (int i = 0; i < keyValues.length; i += 2) {
            row.put((String) keyValues[i], keyValues[i + 1]);
        }
        return row;
    }

    private static List<Row> collect(SqlBackend.RowIterator iterator) {
        var rows = new ArrayList<Row>();
        try {
            Row row;
            while ((row = iterator.next()) != null) {
                rows.add(row);
            }
        } finally {
            iterator.close();
        }
        return rows;
    }
}

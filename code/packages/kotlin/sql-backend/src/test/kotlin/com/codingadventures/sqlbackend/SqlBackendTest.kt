package com.codingadventures.sqlbackend

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class SqlBackendTest {
    @Test
    fun classifiesAndComparesSqlValues() {
        assertTrue(SqlValues.isSqlValue(null))
        assertTrue(SqlValues.isSqlValue(true))
        assertTrue(SqlValues.isSqlValue(42))
        assertTrue(SqlValues.isSqlValue(1.5))
        assertTrue(SqlValues.isSqlValue("text"))
        assertTrue(SqlValues.isSqlValue(Blob("abc")))
        assertFalse(SqlValues.isSqlValue(Any()))

        assertEquals("NULL", SqlValues.typeName(null))
        assertEquals("BOOLEAN", SqlValues.typeName(false))
        assertEquals("INTEGER", SqlValues.typeName(1))
        assertEquals("REAL", SqlValues.typeName(1.0))
        assertEquals("TEXT", SqlValues.typeName("x"))
        assertEquals("BLOB", SqlValues.typeName(Blob("x")))

        assertTrue(SqlValues.compare(null, 1) < 0)
        assertTrue(SqlValues.compare(false, true) < 0)
        assertTrue(SqlValues.compare(1, 2) < 0)
        assertTrue(SqlValues.compare("b", "a") > 0)
        assertEquals(0, SqlValues.compare(Blob("a"), Blob("a")))
    }

    @Test
    fun iteratorsAndCursorsReturnCopies() {
        val rows = listOf(row("id", 1, "name", "Ada"), row("id", 2, "name", "Grace"))
        val iterator = ListRowIterator(rows)
        val first = iterator.next()!!
        first["name"] = "mutated"
        assertEquals("Grace", iterator.next()!!["name"])
        assertNull(iterator.next())

        val cursor = ListCursor(rows, "users")
        assertEquals("Ada", cursor.next()!!["name"])
        val current = cursor.currentRow()!!
        current["name"] = "mutated"
        assertEquals("Ada", cursor.currentRow()!!["name"])
        assertEquals(0, cursor.currentIndex())
    }

    @Test
    fun createsInsertsScansAndAdaptsSchema() {
        val backend = users()
        assertEquals(listOf("users"), backend.tables())
        assertEquals(listOf("id", "name", "email"), backend.columns("USERS").map { it.name })
        assertEquals(listOf("id", "name", "email"), backendAsSchemaProvider(backend).columns("users"))

        val rows = backend.scan("users").toList()
        assertEquals(2, rows.size)
        assertEquals("Ada", rows[0]["name"])
        assertNull(rows[1]["email"])
    }

    @Test
    fun constraintsRejectBadRows() {
        val backend = users()
        assertFailsWith<ConstraintViolation> { backend.insert("users", row("id", 2)) }
        assertFailsWith<ConstraintViolation> { backend.insert("users", row("id", 1, "name", "Ada Again")) }
        assertFailsWith<ColumnNotFound> { backend.insert("users", row("id", 3, "name", "Lin", "missing", 1)) }
        backend.insert("users", row("id", 3, "name", "Lin", "email", "lin@example.test"))
        assertFailsWith<ConstraintViolation> {
            backend.insert("users", row("id", 4, "name", "Other Lin", "email", "lin@example.test"))
        }
    }

    @Test
    fun updatesAndDeletesPositionedRows() {
        val backend = users()
        val cursor = backend.openCursor("users")
        cursor.next()
        backend.update("users", cursor, mapOf("name" to "Augusta Ada"))
        assertEquals("Augusta Ada", backend.scan("users").toList()[0]["name"])

        cursor.next()
        backend.delete("users", cursor)
        val rows = backend.scan("users").toList()
        assertEquals(1, rows.size)
        assertEquals("Augusta Ada", rows[0]["name"])
    }

    @Test
    fun ddlCreatesAddsAndDropsTables() {
        val backend = users()
        assertFailsWith<TableAlreadyExists> { backend.createTable("users", emptyList(), ifNotExists = false) }
        backend.createTable("users", emptyList(), ifNotExists = true)
        backend.addColumn("users", ColumnDef.withDefault("active", "BOOLEAN", true))
        assertEquals(true, backend.scan("users").toList()[0]["active"])
        assertFailsWith<ColumnAlreadyExists> { backend.addColumn("users", ColumnDef("ACTIVE", "BOOLEAN")) }

        backend.dropTable("users", ifExists = false)
        assertFailsWith<TableNotFound> { backend.columns("users") }
        backend.dropTable("users", ifExists = true)
    }

    @Test
    fun indexesScanRowidsAndEnforceUniqueness() {
        val backend = users()
        backend.insert("users", row("id", 3, "name", "Lin"))
        backend.createIndex(IndexDef("idx_users_name", "users", listOf("name")))

        val rowids = backend.scanIndex("idx_users_name", listOf("G"), listOf("M"), loInclusive = false, hiInclusive = false)
        assertEquals(listOf("Grace", "Lin"), backend.scanByRowids("users", rowids).toList().map { it["name"] })
        assertEquals("idx_users_name", backend.listIndexes("users")[0].name)
        assertFailsWith<IndexAlreadyExists> { backend.createIndex(IndexDef("idx_users_name", "users", listOf("id"))) }
        backend.dropIndex("idx_users_name")
        assertEquals(emptyList(), backend.listIndexes())
        backend.dropIndex("idx_users_name", ifExists = true)
        assertFailsWith<IndexNotFound> { backend.scanIndex("missing") }

        backend.createIndex(IndexDef("idx_email", "users", listOf("email"), unique = true))
        assertFailsWith<ConstraintViolation> {
            backend.insert("users", row("id", 4, "name", "Other Ada", "email", "ada@example.test"))
        }
    }

    @Test
    fun transactionsAndSavepointsRestoreSnapshots() {
        val backend = users()
        val handle = backend.beginTransaction()
        backend.insert("users", row("id", 3, "name", "Lin"))
        assertEquals(handle, backend.currentTransaction())
        backend.rollback(handle)
        assertEquals(2, backend.scan("users").toList().size)

        val committed = backend.beginTransaction()
        backend.insert("users", row("id", 3, "name", "Lin"))
        backend.createSavepoint("after_lin")
        backend.insert("users", row("id", 4, "name", "Katherine"))
        backend.rollbackToSavepoint("after_lin")
        assertEquals(3, backend.scan("users").toList().size)
        backend.releaseSavepoint("after_lin")
        backend.commit(committed)
        assertNull(backend.currentTransaction())
    }

    @Test
    fun triggersAndVersionsAreStored() {
        val backend = users()
        val initial = backend.schemaVersion
        val trigger = TriggerDef("users_ai", "users", "after", "insert", "SELECT 1")
        backend.createTrigger(trigger)

        assertTrue(backend.schemaVersion > initial)
        assertEquals("users_ai", backend.listTriggers("users")[0].name)
        assertEquals("AFTER", backend.listTriggers("users")[0].timing)
        assertFailsWith<TriggerAlreadyExists> { backend.createTrigger(trigger) }
        backend.userVersion = 7
        assertEquals(7, backend.userVersion)
        backend.dropTrigger("users_ai")
        assertEquals(emptyList(), backend.listTriggers("users"))
        backend.dropTrigger("users_ai", ifExists = true)
        assertFailsWith<TriggerNotFound> { backend.dropTrigger("users_ai") }
    }

    private fun users(): InMemoryBackend {
        val backend = InMemoryBackend()
        backend.createTable(
            "users",
            listOf(
                ColumnDef("id", "INTEGER", primaryKey = true),
                ColumnDef("name", "TEXT", notNull = true),
                ColumnDef("email", "TEXT", unique = true),
            ),
            ifNotExists = false,
        )
        backend.insert("users", row("id", 1, "name", "Ada", "email", "ada@example.test"))
        backend.insert("users", row("id", 2, "name", "Grace"))
        return backend
    }

    private fun row(vararg values: Any?): Row {
        val row = Row()
        var index = 0
        while (index < values.size) {
            row[values[index] as String] = values[index + 1]
            index += 2
        }
        return row
    }

    private fun RowIterator.toList(): List<Row> {
        val rows = mutableListOf<Row>()
        use {
            while (true) {
                rows += it.next() ?: break
            }
        }
        return rows
    }
}

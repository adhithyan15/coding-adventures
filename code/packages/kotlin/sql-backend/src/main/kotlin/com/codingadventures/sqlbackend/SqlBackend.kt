package com.codingadventures.sqlbackend

import java.util.TreeMap

data class TransactionHandle(val value: Int)

data class Blob(val bytes: List<Byte>) {
    constructor(bytes: ByteArray) : this(bytes.toList())
    constructor(text: String) : this(text.encodeToByteArray().toList())
}

object SqlValues {
    fun isSqlValue(value: Any?): Boolean =
        value == null ||
            value is Boolean ||
            value is Byte ||
            value is Short ||
            value is Int ||
            value is Long ||
            value is Float ||
            value is Double ||
            value is String ||
            value is Blob

    fun typeName(value: Any?): String =
        when (value) {
            null -> "NULL"
            is Boolean -> "BOOLEAN"
            is Byte, is Short, is Int, is Long -> "INTEGER"
            is Float, is Double -> "REAL"
            is String -> "TEXT"
            is Blob -> "BLOB"
            else -> throw IllegalArgumentException("not a SQL value: ${value::class.simpleName}")
        }

    fun compare(left: Any?, right: Any?): Int {
        val rank = rank(left).compareTo(rank(right))
        if (rank != 0) return rank
        return when {
            left == null && right == null -> 0
            left is Boolean && right is Boolean -> left.compareTo(right)
            left is Number && right is Number -> left.toDouble().compareTo(right.toDouble())
            left is String && right is String -> left.compareTo(right)
            left is Blob && right is Blob -> compareBytes(left.bytes, right.bytes)
            else -> left.toString().compareTo(right.toString())
        }
    }

    private fun rank(value: Any?): Int =
        when (value) {
            null -> 0
            is Boolean -> 1
            is Number -> 2
            is String -> 3
            is Blob -> 4
            else -> 5
        }

    private fun compareBytes(left: List<Byte>, right: List<Byte>): Int {
        val count = minOf(left.size, right.size)
        for (i in 0 until count) {
            val comparison = left[i].compareTo(right[i])
            if (comparison != 0) return comparison
        }
        return left.size.compareTo(right.size)
    }
}

class Row(values: Map<String, Any?> = emptyMap()) : LinkedHashMap<String, Any?>(values) {
    fun copy(): Row = Row(this)
}

interface RowIterator : AutoCloseable {
    fun next(): Row?
    override fun close() {}
}

interface Cursor : RowIterator {
    fun currentRow(): Row?
    fun currentIndex(): Int
    fun adjustAfterDelete() {}
}

class ListRowIterator(rows: List<Row>) : RowIterator {
    private val rows = rows.map { it.copy() }
    private var index = 0
    private var closed = false

    override fun next(): Row? {
        if (closed || index >= rows.size) return null
        return rows[index++].copy()
    }

    override fun close() {
        closed = true
    }

    fun toList(): List<Row> {
        val out = mutableListOf<Row>()
        while (true) {
            out += next() ?: break
        }
        close()
        return out
    }
}

class ListCursor(rows: List<Row>, val tableKey: String? = null) : Cursor {
    private val rows = rows.map { it.copy() }
    private var index = -1

    override fun next(): Row? {
        index += 1
        return currentRow()
    }

    override fun currentRow(): Row? = rows.getOrNull(index)?.copy()
    override fun currentIndex(): Int = index
    override fun adjustAfterDelete() {
        if (index >= 0) index -= 1
    }
}

private class TableCursor(val tableKey: String, private val state: TableState) : Cursor {
    private var index = -1

    override fun next(): Row? {
        index += 1
        return currentRow()
    }

    override fun currentRow(): Row? = currentRecord()?.row?.copy()
    override fun currentIndex(): Int = index
    override fun adjustAfterDelete() {
        if (index >= 0) index -= 1
    }

    fun currentRecord(): StoredRow? = state.rows.getOrNull(index)
}

data class ColumnDef(
    val name: String,
    val typeName: String,
    val notNull: Boolean = false,
    val primaryKey: Boolean = false,
    val unique: Boolean = false,
    val autoincrement: Boolean = false,
    val defaultValue: Any? = null,
    val hasDefault: Boolean = false,
    val checkExpression: Any? = null,
    val foreignKey: Any? = null,
) {
    fun effectiveNotNull(): Boolean = notNull || primaryKey
    fun effectiveUnique(): Boolean = unique || primaryKey

    companion object {
        fun withDefault(name: String, typeName: String, defaultValue: Any?): ColumnDef =
            ColumnDef(name, typeName, defaultValue = defaultValue, hasDefault = true)
    }
}

data class IndexDef(
    val name: String,
    val table: String,
    val columns: List<String> = emptyList(),
    val unique: Boolean = false,
    val auto: Boolean = false,
)

data class TriggerDef(
    val name: String,
    val table: String,
    val timing: String,
    val event: String,
    val body: String,
)

sealed class BackendError(message: String) : RuntimeException(message)
class TableNotFound(table: String) : BackendError("table not found: '$table'")
class TableAlreadyExists(table: String) : BackendError("table already exists: '$table'")
class ColumnNotFound(table: String, column: String) : BackendError("column not found: '$table.$column'")
class ColumnAlreadyExists(table: String, column: String) : BackendError("column already exists: '$table.$column'")
class ConstraintViolation(table: String, column: String, message: String) : BackendError(message)
class Unsupported(operation: String) : BackendError("operation not supported: $operation")
class Internal(message: String) : BackendError(message)
class IndexAlreadyExists(index: String) : BackendError("index already exists: '$index'")
class IndexNotFound(index: String) : BackendError("index not found: '$index'")
class TriggerAlreadyExists(trigger: String) : BackendError("trigger already exists: '$trigger'")
class TriggerNotFound(trigger: String) : BackendError("trigger not found: '$trigger'")

abstract class Backend {
    abstract fun tables(): List<String>
    abstract fun columns(table: String): List<ColumnDef>
    abstract fun scan(table: String): RowIterator
    abstract fun insert(table: String, row: Row)
    abstract fun update(table: String, cursor: Cursor, assignments: Map<String, Any?>)
    abstract fun delete(table: String, cursor: Cursor)
    abstract fun createTable(table: String, columns: List<ColumnDef>, ifNotExists: Boolean)
    abstract fun dropTable(table: String, ifExists: Boolean)
    abstract fun addColumn(table: String, column: ColumnDef)
    abstract fun createIndex(index: IndexDef)
    abstract fun dropIndex(name: String, ifExists: Boolean = false)
    abstract fun listIndexes(table: String? = null): List<IndexDef>
    abstract fun scanIndex(
        indexName: String,
        lo: List<Any?>? = null,
        hi: List<Any?>? = null,
        loInclusive: Boolean = true,
        hiInclusive: Boolean = true,
    ): List<Int>
    abstract fun scanByRowids(table: String, rowids: List<Int>): RowIterator
    abstract fun beginTransaction(): TransactionHandle
    abstract fun commit(handle: TransactionHandle)
    abstract fun rollback(handle: TransactionHandle)

    open fun currentTransaction(): TransactionHandle? = null
    open fun createSavepoint(name: String): Unit = throw Unsupported("savepoints")
    open fun releaseSavepoint(name: String): Unit = throw Unsupported("savepoints")
    open fun rollbackToSavepoint(name: String): Unit = throw Unsupported("savepoints")
    open fun createTrigger(definition: TriggerDef): Unit = throw Unsupported("triggers")
    open fun dropTrigger(name: String, ifExists: Boolean = false): Unit = throw Unsupported("triggers")
    open fun listTriggers(table: String): List<TriggerDef> = emptyList()
}

interface SchemaProvider {
    fun columns(table: String): List<String>
    fun listIndexes(table: String): List<IndexDef> = emptyList()
}

fun backendAsSchemaProvider(backend: Backend): SchemaProvider =
    object : SchemaProvider {
        override fun columns(table: String): List<String> = backend.columns(table).map { it.name }
        override fun listIndexes(table: String): List<IndexDef> = backend.listIndexes(table)
    }

private data class StoredRow(val rowid: Int, var row: Row)
private data class TableState(
    val name: String,
    val columns: MutableList<ColumnDef>,
    val rows: MutableList<StoredRow> = mutableListOf(),
    var nextRowid: Int = 0,
) {
    fun copy(): TableState =
        TableState(
            name,
            columns.toMutableList(),
            rows.map { StoredRow(it.rowid, it.row.copy()) }.toMutableList(),
            nextRowid,
        )
}

private data class Snapshot(
    val tables: Map<String, TableState>,
    val indexes: Map<String, IndexDef>,
    val triggers: Map<String, TriggerDef>,
    val triggersByTable: Map<String, List<String>>,
    val userVersion: Int,
    val schemaVersion: Int,
)

private data class Savepoint(val name: String, val snapshot: Snapshot)
private data class KeyedRow(val key: List<Any?>, val rowid: Int)

class InMemoryBackend : Backend() {
    private val tablesByKey = linkedMapOf<String, TableState>()
    private val indexesByKey = linkedMapOf<String, IndexDef>()
    private val triggersByKey = linkedMapOf<String, TriggerDef>()
    private val triggersByTable = linkedMapOf<String, MutableList<String>>()
    private var transactionSnapshot: Snapshot? = null
    private var activeHandle: TransactionHandle? = null
    private var nextHandle = 1
    private val savepoints = mutableListOf<Savepoint>()

    var userVersion: Int = 0
    var schemaVersion: Int = 0
        private set

    override fun tables(): List<String> = tablesByKey.values.map { it.name }

    override fun columns(table: String): List<ColumnDef> = tableState(table).columns.toList()

    override fun scan(table: String): RowIterator =
        ListRowIterator(tableState(table).rows.map { it.row })

    fun openCursor(table: String): Cursor {
        val key = normalizeName(table)
        return TableCursor(key, tableState(table))
    }

    override fun insert(table: String, row: Row) {
        val state = tableState(table)
        val candidate = materializeRow(state, row)
        validateRow(state, candidate)
        state.rows += StoredRow(state.nextRowid++, candidate)
    }

    override fun update(table: String, cursor: Cursor, assignments: Map<String, Any?>) {
        val state = tableState(table)
        val record = currentRecordFor(state, cursor)
        val candidate = record.row.copy()
        for ((name, value) in assignments) {
            val column = findColumn(state, name) ?: throw ColumnNotFound(state.name, name)
            require(SqlValues.isSqlValue(value)) { "not a SQL value: $value" }
            candidate[column.name] = copyValue(value)
        }
        validateRow(state, candidate, record.rowid)
        record.row = candidate
    }

    override fun delete(table: String, cursor: Cursor) {
        val state = tableState(table)
        val record = currentRecordFor(state, cursor)
        state.rows.remove(record)
        cursor.adjustAfterDelete()
    }

    override fun createTable(table: String, columns: List<ColumnDef>, ifNotExists: Boolean) {
        val key = normalizeName(table)
        if (tablesByKey.containsKey(key)) {
            if (ifNotExists) return
            throw TableAlreadyExists(table)
        }
        val seen = mutableSetOf<String>()
        for (column in columns) {
            if (!seen.add(normalizeName(column.name))) throw ColumnAlreadyExists(table, column.name)
        }
        tablesByKey[key] = TableState(table, columns.toMutableList())
        bumpSchemaVersion()
    }

    override fun dropTable(table: String, ifExists: Boolean) {
        val key = normalizeName(table)
        if (tablesByKey.remove(key) == null) {
            if (ifExists) return
            throw TableNotFound(table)
        }
        indexesByKey.entries.removeIf { normalizeName(it.value.table) == key }
        triggersByTable.remove(key)
        triggersByKey.entries.removeIf { normalizeName(it.value.table) == key }
        bumpSchemaVersion()
    }

    override fun addColumn(table: String, column: ColumnDef) {
        val state = tableState(table)
        if (findColumn(state, column.name) != null) throw ColumnAlreadyExists(state.name, column.name)
        if (column.effectiveNotNull() && !column.hasDefault && state.rows.isNotEmpty()) {
            throw ConstraintViolation(state.name, column.name, "NOT NULL constraint failed: ${state.name}.${column.name}")
        }
        state.columns += column
        for (record in state.rows) record.row[column.name] = copyValue(column.defaultValue)
        bumpSchemaVersion()
    }

    override fun createIndex(index: IndexDef) {
        val key = normalizeName(index.name)
        if (indexesByKey.containsKey(key)) throw IndexAlreadyExists(index.name)
        val state = tableState(index.table)
        for (column in index.columns) realColumnName(state, column)
        if (index.unique) validateUniqueIndex(state, index)
        indexesByKey[key] = index.copy(columns = index.columns.toList())
        bumpSchemaVersion()
    }

    override fun dropIndex(name: String, ifExists: Boolean) {
        if (indexesByKey.remove(normalizeName(name)) == null) {
            if (ifExists) return
            throw IndexNotFound(name)
        }
        bumpSchemaVersion()
    }

    override fun listIndexes(table: String?): List<IndexDef> {
        val tableKey = table?.let(::normalizeName)
        return indexesByKey.values
            .filter { tableKey == null || normalizeName(it.table) == tableKey }
            .map { it.copy(columns = it.columns.toList()) }
    }

    override fun scanIndex(
        indexName: String,
        lo: List<Any?>?,
        hi: List<Any?>?,
        loInclusive: Boolean,
        hiInclusive: Boolean,
    ): List<Int> {
        val index = indexesByKey[normalizeName(indexName)] ?: throw IndexNotFound(indexName)
        val state = tableState(index.table)
        val entries = state.rows.map { KeyedRow(indexKey(state, it.row, index.columns), it.rowid) }
            .sortedWith { left, right ->
                val cmp = compareKeys(left.key, right.key)
                if (cmp != 0) cmp else left.rowid.compareTo(right.rowid)
            }
        return entries.filter { entry ->
            val afterLo = lo == null || comparePrefix(entry.key, lo).let { it > 0 || (it == 0 && loInclusive) }
            val beforeHi = hi == null || comparePrefix(entry.key, hi).let { it < 0 || (it == 0 && hiInclusive) }
            afterLo && beforeHi
        }.map { it.rowid }
    }

    override fun scanByRowids(table: String, rowids: List<Int>): RowIterator {
        val state = tableState(table)
        val byRowid = state.rows.associateBy { it.rowid }
        return ListRowIterator(rowids.mapNotNull { byRowid[it]?.row })
    }

    override fun beginTransaction(): TransactionHandle {
        if (activeHandle != null) throw Unsupported("nested transactions")
        transactionSnapshot = snapshot()
        val handle = TransactionHandle(nextHandle++)
        activeHandle = handle
        return handle
    }

    override fun commit(handle: TransactionHandle) {
        requireActive(handle)
        transactionSnapshot = null
        activeHandle = null
        savepoints.clear()
    }

    override fun rollback(handle: TransactionHandle) {
        requireActive(handle)
        restore(transactionSnapshot ?: throw Internal("missing transaction snapshot"))
        transactionSnapshot = null
        activeHandle = null
        savepoints.clear()
    }

    override fun currentTransaction(): TransactionHandle? = activeHandle

    override fun createSavepoint(name: String) {
        if (activeHandle == null) throw Unsupported("savepoints outside transaction")
        savepoints += Savepoint(name, snapshot())
    }

    override fun releaseSavepoint(name: String) {
        val index = savepointIndex(name)
        savepoints.subList(index, savepoints.size).clear()
    }

    override fun rollbackToSavepoint(name: String) {
        val index = savepointIndex(name)
        restore(savepoints[index].snapshot)
        savepoints.subList(index + 1, savepoints.size).clear()
    }

    override fun createTrigger(definition: TriggerDef) {
        val key = normalizeName(definition.name)
        if (triggersByKey.containsKey(key)) throw TriggerAlreadyExists(definition.name)
        val state = tableState(definition.table)
        val trigger = definition.copy(timing = definition.timing.uppercase(), event = definition.event.uppercase())
        triggersByKey[key] = trigger
        triggersByTable.getOrPut(normalizeName(state.name)) { mutableListOf() } += key
        bumpSchemaVersion()
    }

    override fun dropTrigger(name: String, ifExists: Boolean) {
        val key = normalizeName(name)
        val trigger = triggersByKey.remove(key)
        if (trigger == null) {
            if (ifExists) return
            throw TriggerNotFound(name)
        }
        triggersByTable[normalizeName(trigger.table)]?.remove(key)
        bumpSchemaVersion()
    }

    override fun listTriggers(table: String): List<TriggerDef> =
        triggersByTable[normalizeName(table)].orEmpty().mapNotNull { triggersByKey[it] }

    private fun tableState(table: String): TableState =
        tablesByKey[normalizeName(table)] ?: throw TableNotFound(table)

    private fun materializeRow(state: TableState, row: Row): Row {
        val candidate = Row()
        for (column in state.columns) {
            val entry = row.entries.find { normalizeName(it.key) == normalizeName(column.name) }
            val value = when {
                entry != null -> entry.value
                column.autoincrement && column.primaryKey -> nextAutoincrementValue(state, column)
                column.hasDefault -> column.defaultValue
                else -> null
            }
            require(SqlValues.isSqlValue(value)) { "not a SQL value: $value" }
            candidate[column.name] = copyValue(value)
        }
        for (name in row.keys) if (findColumn(state, name) == null) throw ColumnNotFound(state.name, name)
        return candidate
    }

    private fun nextAutoincrementValue(state: TableState, column: ColumnDef): Long =
        (state.rows.mapNotNull { (it.row[column.name] as? Number)?.toLong() }.maxOrNull() ?: 0L) + 1L

    private fun validateRow(state: TableState, candidate: Row, skipRowid: Int? = null) {
        for (column in state.columns) {
            val value = candidate[column.name]
            if (column.effectiveNotNull() && value == null) {
                throw ConstraintViolation(state.name, column.name, "NOT NULL constraint failed: ${state.name}.${column.name}")
            }
            if (column.effectiveUnique() && value != null) {
                for (record in state.rows) {
                    if (record.rowid == skipRowid) continue
                    if (SqlValues.compare(record.row[column.name], value) == 0) {
                        val label = if (column.primaryKey) "PRIMARY KEY" else "UNIQUE"
                        throw ConstraintViolation(state.name, column.name, "$label constraint failed: ${state.name}.${column.name}")
                    }
                }
            }
        }
        for (index in indexesByKey.values) {
            if (index.unique && normalizeName(index.table) == normalizeName(state.name)) {
                validateUniqueIndex(state, index, candidate, skipRowid)
            }
        }
    }

    private fun validateUniqueIndex(state: TableState, index: IndexDef, candidate: Row? = null, skipRowid: Int? = null) {
        if (candidate != null) {
            val candidateKey = indexKey(state, candidate, index.columns)
            if (candidateKey.any { it == null }) return
            for (record in state.rows) {
                if (record.rowid == skipRowid) continue
                if (compareKeys(indexKey(state, record.row, index.columns), candidateKey) == 0) {
                    throw ConstraintViolation(state.name, index.columns.joinToString(","), "UNIQUE constraint failed: ${state.name}.${index.columns.joinToString(",")}")
                }
            }
            return
        }
        val seen = mutableSetOf<List<Any?>>()
        for (record in state.rows) {
            val key = indexKey(state, record.row, index.columns)
            if (key.any { it == null }) continue
            if (!seen.add(key)) {
                throw ConstraintViolation(state.name, index.columns.joinToString(","), "UNIQUE constraint failed: ${state.name}.${index.columns.joinToString(",")}")
            }
        }
    }

    private fun currentRecordFor(state: TableState, cursor: Cursor): StoredRow {
        if (cursor !is TableCursor || cursor.tableKey != normalizeName(state.name)) {
            throw Internal("cursor does not belong to table ${state.name}")
        }
        return cursor.currentRecord() ?: throw Internal("cursor is not positioned on a row")
    }

    private fun findColumn(state: TableState, name: String): ColumnDef? =
        state.columns.find { normalizeName(it.name) == normalizeName(name) }

    private fun realColumnName(state: TableState, name: String): String =
        findColumn(state, name)?.name ?: throw ColumnNotFound(state.name, name)

    private fun indexKey(state: TableState, row: Row, columns: List<String>): List<Any?> =
        columns.map { row[realColumnName(state, it)] }

    private fun requireActive(handle: TransactionHandle) {
        if (activeHandle == null) throw Unsupported("no active transaction")
        if (activeHandle != handle) throw Unsupported("stale transaction handle")
    }

    private fun savepointIndex(name: String): Int {
        if (activeHandle == null) throw Unsupported("savepoints outside transaction")
        val index = savepoints.indexOfLast { it.name == name }
        if (index < 0) throw Internal("savepoint not found: $name")
        return index
    }

    private fun snapshot(): Snapshot =
        Snapshot(
            tablesByKey.mapValues { it.value.copy() },
            indexesByKey.mapValues { it.value.copy(columns = it.value.columns.toList()) },
            triggersByKey.mapValues { it.value.copy() },
            triggersByTable.mapValues { it.value.toList() },
            userVersion,
            schemaVersion,
        )

    private fun restore(snapshot: Snapshot) {
        tablesByKey.clear()
        tablesByKey.putAll(snapshot.tables.mapValues { it.value.copy() })
        indexesByKey.clear()
        indexesByKey.putAll(snapshot.indexes.mapValues { it.value.copy(columns = it.value.columns.toList()) })
        triggersByKey.clear()
        triggersByKey.putAll(snapshot.triggers.mapValues { it.value.copy() })
        triggersByTable.clear()
        triggersByTable.putAll(snapshot.triggersByTable.mapValues { it.value.toMutableList() })
        userVersion = snapshot.userVersion
        schemaVersion = snapshot.schemaVersion
    }

    private fun bumpSchemaVersion() {
        schemaVersion += 1
    }
}

private fun normalizeName(name: String): String = name.lowercase()

private fun copyValue(value: Any?): Any? =
    when (value) {
        is Blob -> value.copy(bytes = value.bytes.toList())
        else -> value
    }

private fun compareKeys(left: List<Any?>, right: List<Any?>): Int {
    val count = minOf(left.size, right.size)
    for (i in 0 until count) {
        val comparison = SqlValues.compare(left[i], right[i])
        if (comparison != 0) return comparison
    }
    return left.size.compareTo(right.size)
}

private fun comparePrefix(key: List<Any?>, bound: List<Any?>): Int {
    for (i in bound.indices) {
        val comparison = SqlValues.compare(key.getOrNull(i), bound[i])
        if (comparison != 0) return comparison
    }
    return 0
}

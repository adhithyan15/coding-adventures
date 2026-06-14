package com.codingadventures.minisqlite

object MiniSqlite {
    const val API_LEVEL = "2.0"
    const val THREADSAFETY = 1
    const val PARAMSTYLE = "qmark"

    fun connect(database: String, options: Options = Options()): Connection {
        if (database != ":memory:") {
            throw MiniSqliteException("NotSupportedError", "Kotlin mini-sqlite supports only :memory: in Level 0")
        }
        return Connection(options)
    }
}

data class Options(val autocommit: Boolean = false)
data class Column(val name: String)

class MiniSqliteException(val kind: String, message: String) : RuntimeException(message)

class Connection internal constructor(options: Options) : AutoCloseable {
    private var db = Database()
    private val autocommit = options.autocommit
    private var snapshot: Database? = null
    private var closed = false

    fun cursor(): Cursor {
        assertOpen()
        return Cursor(this)
    }

    fun execute(sql: String, params: List<Any?> = emptyList()): Cursor = cursor().execute(sql, params)

    fun executemany(sql: String, paramsSeq: List<List<Any?>>): Cursor = cursor().executemany(sql, paramsSeq)

    fun commit() {
        assertOpen()
        snapshot = null
    }

    fun rollback() {
        assertOpen()
        val saved = snapshot
        if (saved != null) {
            db = saved.copy()
            snapshot = null
        }
    }

    override fun close() {
        if (closed) {
            return
        }
        val saved = snapshot
        if (saved != null) {
            db = saved.copy()
        }
        snapshot = null
        closed = true
    }

    private fun assertOpen() {
        if (closed) {
            throw MiniSqliteException("ProgrammingError", "connection is closed")
        }
    }

    private fun ensureSnapshot() {
        if (!autocommit && snapshot == null) {
            snapshot = db.copy()
        }
    }

    internal fun executeBound(sql: String, params: List<Any?>): Result {
        assertOpen()
        val bound = bindParameters(sql, params)
        return try {
            when (firstKeyword(bound)) {
                "BEGIN" -> {
                    ensureSnapshot()
                    Result.empty(0)
                }
                "COMMIT" -> {
                    snapshot = null
                    Result.empty(0)
                }
                "ROLLBACK" -> {
                    val saved = snapshot
                    if (saved != null) {
                        db = saved.copy()
                        snapshot = null
                    }
                    Result.empty(0)
                }
                "SELECT" -> db.select(bound)
                "CREATE" -> {
                    ensureSnapshot()
                    db.create(parseCreate(bound))
                }
                "DROP" -> {
                    ensureSnapshot()
                    db.drop(parseDrop(bound))
                }
                "INSERT" -> {
                    ensureSnapshot()
                    db.insert(parseInsert(bound))
                }
                "UPDATE" -> {
                    ensureSnapshot()
                    db.update(parseUpdate(bound))
                }
                "DELETE" -> {
                    ensureSnapshot()
                    db.delete(parseDelete(bound))
                }
                else -> throw IllegalArgumentException("unsupported SQL statement")
            }
        } catch (ex: MiniSqliteException) {
            throw ex
        } catch (ex: RuntimeException) {
            throw MiniSqliteException("OperationalError", ex.message ?: ex.javaClass.simpleName)
        }
    }
}

class Cursor internal constructor(private val connection: Connection) : AutoCloseable {
    var description: List<Column> = emptyList()
        private set
    var rowcount: Int = -1
        private set
    var lastrowid: Any? = null
        private set
    var arraysize: Int = 1
    private var rows: List<List<Any?>> = emptyList()
    private var offset = 0
    private var closed = false

    fun execute(sql: String, params: List<Any?> = emptyList()): Cursor {
        if (closed) {
            throw MiniSqliteException("ProgrammingError", "cursor is closed")
        }
        val result = connection.executeBound(sql, params)
        rows = result.rows
        offset = 0
        rowcount = result.rowsAffected
        description = result.columns.map { Column(it) }
        return this
    }

    fun executemany(sql: String, paramsSeq: List<List<Any?>>): Cursor {
        var total = 0
        for (params in paramsSeq) {
            execute(sql, params)
            if (rowcount > 0) {
                total += rowcount
            }
        }
        if (paramsSeq.isNotEmpty()) {
            rowcount = total
        }
        return this
    }

    fun fetchone(): List<Any?>? {
        if (closed || offset >= rows.size) {
            return null
        }
        val row = rows[offset]
        offset += 1
        return row
    }

    fun fetchmany(size: Int = arraysize): List<List<Any?>> {
        if (closed) {
            return emptyList()
        }
        val out = mutableListOf<List<Any?>>()
        repeat(size) {
            val row = fetchone() ?: return@repeat
            out += row
        }
        return out
    }

    fun fetchall(): List<List<Any?>> {
        if (closed) {
            return emptyList()
        }
        val out = mutableListOf<List<Any?>>()
        while (true) {
            val row = fetchone() ?: break
            out += row
        }
        return out
    }

    override fun close() {
        closed = true
        rows = emptyList()
        description = emptyList()
    }
}

internal data class Result(
    val columns: List<String>,
    val rows: List<List<Any?>>,
    val rowsAffected: Int,
) {
    companion object {
        fun empty(rowsAffected: Int) = Result(emptyList(), emptyList(), rowsAffected)
    }
}

private data class CreateStmt(val table: String, val columns: List<String>, val ifNotExists: Boolean)
private data class DropStmt(val table: String, val ifExists: Boolean)
private data class InsertStmt(val table: String, val columns: List<String>, val rows: List<List<Any?>>)
private data class Assignment(val column: String, val value: Any?)
private data class UpdateStmt(val table: String, val assignments: List<Assignment>, val where: String)
private data class DeleteStmt(val table: String, val where: String)
private data class Split(val left: String, val right: String)
private data class OperatorAt(val index: Int, val operator: String)

private class Table(
    val columns: MutableList<String>,
    val rows: MutableList<MutableMap<String, Any?>> = mutableListOf(),
) {
    fun copy(): Table {
        val copied = Table(columns.toMutableList())
        for (row in rows) {
            copied.rows += LinkedHashMap(row)
        }
        return copied
    }
}

private class Database {
    private val tables = linkedMapOf<String, Table>()

    fun copy(): Database {
        val copied = Database()
        for ((name, table) in tables) {
            copied.tables[name] = table.copy()
        }
        return copied
    }

    fun create(stmt: CreateStmt): Result {
        val key = normalizeName(stmt.table)
        if (tables.containsKey(key)) {
            if (stmt.ifNotExists) {
                return Result.empty(0)
            }
            throw IllegalArgumentException("table already exists: ${stmt.table}")
        }
        val seen = linkedSetOf<String>()
        for (column in stmt.columns) {
            if (!seen.add(normalizeName(column))) {
                throw IllegalArgumentException("duplicate column: $column")
            }
        }
        tables[key] = Table(stmt.columns.toMutableList())
        return Result.empty(0)
    }

    fun drop(stmt: DropStmt): Result {
        val key = normalizeName(stmt.table)
        if (!tables.containsKey(key)) {
            if (stmt.ifExists) {
                return Result.empty(0)
            }
            throw IllegalArgumentException("no such table: ${stmt.table}")
        }
        tables.remove(key)
        return Result.empty(0)
    }

    fun insert(stmt: InsertStmt): Result {
        val table = table(stmt.table)
        val columns = if (stmt.columns.isEmpty()) {
            table.columns.toList()
        } else {
            stmt.columns.map { canonicalColumn(table, it) }
        }
        for (values in stmt.rows) {
            if (values.size != columns.size) {
                throw IllegalArgumentException("INSERT expected ${columns.size} values, got ${values.size}")
            }
            val row = linkedMapOf<String, Any?>()
            for (tableColumn in table.columns) {
                row[tableColumn] = null
            }
            for (i in columns.indices) {
                row[columns[i]] = values[i]
            }
            table.rows += row
        }
        return Result.empty(stmt.rows.size)
    }

    fun update(stmt: UpdateStmt): Result {
        val table = table(stmt.table)
        val matches = matchingRows(table, stmt.where)
        val assignments = stmt.assignments.map { Assignment(canonicalColumn(table, it.column), it.value) }
        for (row in matches) {
            for (assignment in assignments) {
                row[assignment.column] = assignment.value
            }
        }
        return Result.empty(matches.size)
    }

    fun delete(stmt: DeleteStmt): Result {
        val table = table(stmt.table)
        val matches = matchingRows(table, stmt.where).toSet()
        table.rows.removeAll { row -> matches.contains(row) }
        return Result.empty(matches.size)
    }

    fun select(sql: String): Result {
        val body = stripTrailingSemicolon(sql).replace(Regex("^\\s*SELECT\\s+", RegexOption.IGNORE_CASE), "")
        val fromSplit = splitTopLevelKeyword(body, "FROM")
        if (fromSplit.right.isEmpty()) {
            throw IllegalArgumentException("invalid SELECT statement")
        }
        val columnSql = fromSplit.left
        var rest = fromSplit.right
        val tableName = identifierAtStart(rest) ?: throw IllegalArgumentException("invalid SELECT statement")
        rest = trim(rest.substring(tableName.length))
        val orderSplit = splitTopLevelKeyword(rest, "ORDER BY")
        val whereSplit = splitTopLevelKeyword(orderSplit.left, "WHERE")

        val table = table(tableName)
        val ordered = matchingRows(table, whereSplit.right).toMutableList()
        applyOrder(table, ordered, orderSplit.right)

        val selectedColumns = parseSelectedColumns(table, columnSql)
        val outRows = ordered.map { row ->
            selectedColumns.map { column -> valueOfColumn(table, row, column) }
        }
        return Result(selectedColumns, outRows, -1)
    }

    private fun table(tableName: String): Table {
        return tables[normalizeName(tableName)] ?: throw IllegalArgumentException("no such table: $tableName")
    }

    private fun matchingRows(table: Table, whereSql: String): List<MutableMap<String, Any?>> {
        if (trim(whereSql).isEmpty()) {
            return table.rows.toList()
        }
        return table.rows.filter { row -> matchesWhere(table, row, whereSql) }
    }
}

private fun trim(value: String?): String = value?.trim() ?: ""
private fun normalizeName(name: String): String = name.lowercase()
private fun stripTrailingSemicolon(sql: String): String = trim(sql).replace(Regex(";\\s*$"), "")

private fun firstKeyword(sql: String): String {
    val value = trim(sql)
    var end = 0
    while (end < value.length && (value[end].isLetter() || value[end] == '_')) {
        end += 1
    }
    return value.substring(0, end).uppercase()
}

private fun isBoundaryChar(ch: Char): Boolean = !ch.isLetterOrDigit() && ch != '_'

private fun quoteSqlString(value: Any?): String = "'" + value.toString().replace("'", "''") + "'"

private fun toSqlLiteral(value: Any?): String {
    return when (value) {
        null -> "NULL"
        is Boolean -> if (value) "TRUE" else "FALSE"
        is Number -> value.toString()
        is CharSequence -> quoteSqlString(value)
        else -> throw MiniSqliteException("ProgrammingError", "unsupported parameter type: ${value::class.qualifiedName}")
    }
}

private fun readQuoted(sql: String, index: Int, quote: Char): Int {
    var i = index + 1
    while (i < sql.length) {
        val ch = sql[i]
        if (ch == quote) {
            if (i + 1 < sql.length && sql[i + 1] == quote) {
                i += 2
            } else {
                return i + 1
            }
        } else {
            i += 1
        }
    }
    return sql.length
}

private fun bindParameters(sql: String, params: List<Any?>): String {
    val out = StringBuilder()
    var index = 0
    var i = 0
    while (i < sql.length) {
        val ch = sql[i]
        when {
            ch == '\'' || ch == '"' -> {
                val next = readQuoted(sql, i, ch)
                out.append(sql.substring(i, next))
                i = next
            }
            ch == '-' && i + 1 < sql.length && sql[i + 1] == '-' -> {
                var next = i + 2
                while (next < sql.length && sql[next] != '\n') {
                    next += 1
                }
                out.append(sql.substring(i, next))
                i = next
            }
            ch == '/' && i + 1 < sql.length && sql[i + 1] == '*' -> {
                var next = i + 2
                while (next + 1 < sql.length && sql.substring(next, next + 2) != "*/") {
                    next += 1
                }
                next = minOf(next + 2, sql.length)
                out.append(sql.substring(i, next))
                i = next
            }
            ch == '?' -> {
                if (index >= params.size) {
                    throw MiniSqliteException("ProgrammingError", "not enough parameters for SQL statement")
                }
                out.append(toSqlLiteral(params[index]))
                index += 1
                i += 1
            }
            else -> {
                out.append(ch)
                i += 1
            }
        }
    }
    if (index < params.size) {
        throw MiniSqliteException("ProgrammingError", "too many parameters for SQL statement")
    }
    return out.toString()
}

private fun splitTopLevel(text: String, delimiter: Char): List<String> {
    val parts = mutableListOf<String>()
    val current = StringBuilder()
    var depth = 0
    var quote: Char? = null
    var i = 0
    while (i < text.length) {
        val ch = text[i]
        if (quote != null) {
            current.append(ch)
            if (ch == quote) {
                if (i + 1 < text.length && text[i + 1] == quote) {
                    i += 1
                    current.append(text[i])
                } else {
                    quote = null
                }
            }
        } else if (ch == '\'' || ch == '"') {
            quote = ch
            current.append(ch)
        } else if (ch == '(') {
            depth += 1
            current.append(ch)
        } else if (ch == ')') {
            depth = maxOf(0, depth - 1)
            current.append(ch)
        } else if (depth == 0 && ch == delimiter) {
            val part = trim(current.toString())
            if (part.isNotEmpty()) {
                parts += part
            }
            current.setLength(0)
        } else {
            current.append(ch)
        }
        i += 1
    }
    val part = trim(current.toString())
    if (part.isNotEmpty()) {
        parts += part
    }
    return parts
}

private fun splitTopLevelKeyword(text: String, keyword: String): Split {
    val upper = text.uppercase()
    val keyLength = keyword.length
    var depth = 0
    var quote: Char? = null
    var i = 0
    while (i < text.length) {
        val ch = text[i]
        if (quote != null) {
            if (ch == quote) {
                if (i + 1 < text.length && text[i + 1] == quote) {
                    i += 1
                } else {
                    quote = null
                }
            }
        } else if (ch == '\'' || ch == '"') {
            quote = ch
        } else if (ch == '(') {
            depth += 1
        } else if (ch == ')') {
            depth = maxOf(0, depth - 1)
        } else if (
            depth == 0 &&
            i + keyLength <= text.length &&
            upper.substring(i, i + keyLength) == keyword &&
            (i == 0 || isBoundaryChar(text[i - 1])) &&
            (i + keyLength == text.length || isBoundaryChar(text[i + keyLength]))
        ) {
            return Split(trim(text.substring(0, i)), trim(text.substring(i + keyLength)))
        }
        i += 1
    }
    return Split(trim(text), "")
}

private fun findMatchingParen(text: String, openIndex: Int): Int {
    var depth = 0
    var quote: Char? = null
    var i = openIndex
    while (i < text.length) {
        val ch = text[i]
        if (quote != null) {
            if (ch == quote) {
                if (i + 1 < text.length && text[i + 1] == quote) {
                    i += 1
                } else {
                    quote = null
                }
            }
        } else if (ch == '\'' || ch == '"') {
            quote = ch
        } else if (ch == '(') {
            depth += 1
        } else if (ch == ')') {
            depth -= 1
            if (depth == 0) {
                return i
            }
        }
        i += 1
    }
    return -1
}

private fun parseLiteral(text: String): Any? {
    val value = trim(text)
    val upper = value.uppercase()
    if (upper == "NULL") {
        return null
    }
    if (upper == "TRUE") {
        return true
    }
    if (upper == "FALSE") {
        return false
    }
    if (value.length >= 2 && value.first() == '\'' && value.last() == '\'') {
        return value.substring(1, value.length - 1).replace("''", "'")
    }
    if (Regex("[-+]?(?:\\d+(?:\\.\\d*)?|\\.\\d+)").matches(value)) {
        return if (value.contains('.')) value.toDouble() else value.toLong()
    }
    throw IllegalArgumentException("expected literal value, got: $text")
}

private fun identifierAtStart(text: String): String? {
    val value = trim(text)
    if (value.isEmpty() || (!value[0].isLetter() && value[0] != '_')) {
        return null
    }
    var end = 1
    while (end < value.length && (value[end].isLetterOrDigit() || value[end] == '_')) {
        end += 1
    }
    return value.substring(0, end)
}

private fun parseCreate(sql: String): CreateStmt {
    val stripped = stripTrailingSemicolon(sql)
    val ifNotExists = Regex("^\\s*CREATE\\s+TABLE\\s+IF\\s+NOT\\s+EXISTS\\s+.*", RegexOption.IGNORE_CASE).matches(stripped)
    val prefix = if (ifNotExists) {
        Regex("^\\s*CREATE\\s+TABLE\\s+IF\\s+NOT\\s+EXISTS\\s+", RegexOption.IGNORE_CASE)
    } else {
        Regex("^\\s*CREATE\\s+TABLE\\s+", RegexOption.IGNORE_CASE)
    }
    val restStart = stripped.replace(prefix, "")
    val table = identifierAtStart(restStart) ?: throw IllegalArgumentException("invalid CREATE TABLE statement")
    val rest = trim(restStart.substring(table.length))
    if (!rest.startsWith("(") || !rest.endsWith(")")) {
        throw IllegalArgumentException("invalid CREATE TABLE statement")
    }
    val columns = splitTopLevel(rest.substring(1, rest.length - 1), ',').mapNotNull { identifierAtStart(it) }
    if (columns.isEmpty()) {
        throw IllegalArgumentException("CREATE TABLE requires at least one column")
    }
    return CreateStmt(table, columns, ifNotExists)
}

private fun parseDrop(sql: String): DropStmt {
    val stripped = stripTrailingSemicolon(sql)
    val ifExists = Regex("^\\s*DROP\\s+TABLE\\s+IF\\s+EXISTS\\s+.*", RegexOption.IGNORE_CASE).matches(stripped)
    val prefix = if (ifExists) {
        Regex("^\\s*DROP\\s+TABLE\\s+IF\\s+EXISTS\\s+", RegexOption.IGNORE_CASE)
    } else {
        Regex("^\\s*DROP\\s+TABLE\\s+", RegexOption.IGNORE_CASE)
    }
    val rest = stripped.replace(prefix, "")
    val table = identifierAtStart(rest)
    if (table == null || trim(rest.substring(table.length)).isNotEmpty()) {
        throw IllegalArgumentException("invalid DROP TABLE statement")
    }
    return DropStmt(table, ifExists)
}

private fun parseValueRows(sql: String): List<List<Any?>> {
    var rest = trim(sql)
    val rows = mutableListOf<List<Any?>>()
    while (rest.isNotEmpty()) {
        if (!rest.startsWith("(")) {
            throw IllegalArgumentException("INSERT VALUES rows must be parenthesized")
        }
        val close = findMatchingParen(rest, 0)
        if (close < 0) {
            throw IllegalArgumentException("unterminated INSERT VALUES row")
        }
        val row = splitTopLevel(rest.substring(1, close), ',').map { parseLiteral(it) }
        if (row.isEmpty()) {
            throw IllegalArgumentException("INSERT row requires at least one value")
        }
        rows += row
        rest = trim(rest.substring(close + 1))
        if (rest.startsWith(",")) {
            rest = trim(rest.substring(1))
        } else if (rest.isNotEmpty()) {
            throw IllegalArgumentException("invalid text after INSERT row")
        }
    }
    if (rows.isEmpty()) {
        throw IllegalArgumentException("INSERT requires at least one row")
    }
    return rows
}

private fun parseInsert(sql: String): InsertStmt {
    val stripped = stripTrailingSemicolon(sql)
    val prefix = Regex("^\\s*INSERT\\s+INTO\\s+", RegexOption.IGNORE_CASE)
    val start = stripped.replace(prefix, "")
    if (start == stripped) {
        throw IllegalArgumentException("invalid INSERT statement")
    }
    val table = identifierAtStart(start) ?: throw IllegalArgumentException("invalid INSERT statement")
    var rest = trim(start.substring(table.length))
    val columns = mutableListOf<String>()
    if (rest.startsWith("(")) {
        val close = findMatchingParen(rest, 0)
        if (close < 0) {
            throw IllegalArgumentException("invalid INSERT statement")
        }
        columns += splitTopLevel(rest.substring(1, close), ',').map { trim(it) }
        rest = trim(rest.substring(close + 1))
    }
    if (!rest.uppercase().startsWith("VALUES")) {
        throw IllegalArgumentException("invalid INSERT statement")
    }
    return InsertStmt(table, columns, parseValueRows(rest.substring("VALUES".length)))
}

private fun parseUpdate(sql: String): UpdateStmt {
    val stripped = stripTrailingSemicolon(sql)
    val prefix = Regex("^\\s*UPDATE\\s+", RegexOption.IGNORE_CASE)
    val start = stripped.replace(prefix, "")
    if (start == stripped) {
        throw IllegalArgumentException("invalid UPDATE statement")
    }
    val table = identifierAtStart(start) ?: throw IllegalArgumentException("invalid UPDATE statement")
    var rest = trim(start.substring(table.length))
    if (!rest.uppercase().startsWith("SET")) {
        throw IllegalArgumentException("invalid UPDATE statement")
    }
    rest = trim(rest.substring("SET".length))
    val whereSplit = splitTopLevelKeyword(rest, "WHERE")
    val assignments = splitTopLevel(whereSplit.left, ',').map { assignment ->
        val op = findTopLevelOperator(assignment, listOf("=")) ?: throw IllegalArgumentException("invalid assignment: $assignment")
        val column = trim(assignment.substring(0, op.index))
        if (!Regex("[A-Za-z_][A-Za-z0-9_]*").matches(column)) {
            throw IllegalArgumentException("invalid identifier: $column")
        }
        Assignment(column, parseLiteral(assignment.substring(op.index + op.operator.length)))
    }
    if (assignments.isEmpty()) {
        throw IllegalArgumentException("UPDATE requires at least one assignment")
    }
    return UpdateStmt(table, assignments, whereSplit.right)
}

private fun parseDelete(sql: String): DeleteStmt {
    val stripped = stripTrailingSemicolon(sql)
    val prefix = Regex("^\\s*DELETE\\s+FROM\\s+", RegexOption.IGNORE_CASE)
    val start = stripped.replace(prefix, "")
    if (start == stripped) {
        throw IllegalArgumentException("invalid DELETE statement")
    }
    val table = identifierAtStart(start) ?: throw IllegalArgumentException("invalid DELETE statement")
    val rest = trim(start.substring(table.length))
    var where = ""
    if (rest.isNotEmpty()) {
        val whereSplit = splitTopLevelKeyword(rest, "WHERE")
        if (whereSplit.left.isNotEmpty() || whereSplit.right.isEmpty()) {
            throw IllegalArgumentException("invalid DELETE statement")
        }
        where = whereSplit.right
    }
    return DeleteStmt(table, where)
}

private fun parseSelectedColumns(table: Table, columnSql: String): List<String> {
    if (trim(columnSql) == "*") {
        return table.columns.toList()
    }
    return splitTopLevel(columnSql, ',').map { canonicalColumn(table, it) }
}

private fun canonicalColumn(table: Table, column: String): String {
    val wanted = normalizeName(trim(column))
    for (candidate in table.columns) {
        if (normalizeName(candidate) == wanted) {
            return candidate
        }
    }
    throw IllegalArgumentException("no such column: ${trim(column)}")
}

private fun valueOfColumn(table: Table, row: Map<String, Any?>, column: String): Any? {
    return row[canonicalColumn(table, column)]
}

private fun matchesWhere(table: Table, row: Map<String, Any?>, whereSql: String): Boolean {
    val where = trim(whereSql)
    if (where.isEmpty()) {
        return true
    }
    val orSplit = splitTopLevelKeyword(where, "OR")
    if (orSplit.right.isNotEmpty()) {
        return matchesWhere(table, row, orSplit.left) || matchesWhere(table, row, orSplit.right)
    }
    val andSplit = splitTopLevelKeyword(where, "AND")
    if (andSplit.right.isNotEmpty()) {
        return matchesWhere(table, row, andSplit.left) && matchesWhere(table, row, andSplit.right)
    }
    val upper = where.uppercase()
    if (upper.endsWith(" IS NOT NULL")) {
        val column = trim(where.substring(0, where.length - " IS NOT NULL".length))
        return valueOfColumn(table, row, column) != null
    }
    if (upper.endsWith(" IS NULL")) {
        val column = trim(where.substring(0, where.length - " IS NULL".length))
        return valueOfColumn(table, row, column) == null
    }
    val inIndex = upper.indexOf(" IN ")
    if (inIndex > 0) {
        val column = trim(where.substring(0, inIndex))
        val valuesSql = trim(where.substring(inIndex + 4))
        if (!valuesSql.startsWith("(") || !valuesSql.endsWith(")")) {
            throw IllegalArgumentException("invalid IN predicate")
        }
        val actual = valueOfColumn(table, row, column)
        return splitTopLevel(valuesSql.substring(1, valuesSql.length - 1), ',').any {
            valuesEqual(actual, parseLiteral(it))
        }
    }
    val op = findTopLevelOperator(where, listOf("<=", ">=", "!=", "<>", "=", "<", ">"))
        ?: throw IllegalArgumentException("unsupported WHERE predicate: $where")
    val actual = valueOfColumn(table, row, where.substring(0, op.index))
    val expected = parseLiteral(where.substring(op.index + op.operator.length))
    return comparePredicate(actual, expected, op.operator)
}

private fun findTopLevelOperator(text: String, operators: List<String>): OperatorAt? {
    var depth = 0
    var quote: Char? = null
    var i = 0
    while (i < text.length) {
        val ch = text[i]
        if (quote != null) {
            if (ch == quote) {
                if (i + 1 < text.length && text[i + 1] == quote) {
                    i += 1
                } else {
                    quote = null
                }
            }
        } else if (ch == '\'' || ch == '"') {
            quote = ch
        } else if (ch == '(') {
            depth += 1
        } else if (ch == ')') {
            depth = maxOf(0, depth - 1)
        } else if (depth == 0) {
            for (operator in operators) {
                if (i + operator.length <= text.length && text.substring(i, i + operator.length) == operator) {
                    return OperatorAt(i, operator)
                }
            }
        }
        i += 1
    }
    return null
}

private fun comparePredicate(actual: Any?, expected: Any?, operator: String): Boolean {
    if (operator == "=") {
        return valuesEqual(actual, expected)
    }
    if (operator == "!=" || operator == "<>") {
        return !valuesEqual(actual, expected)
    }
    if (actual == null || expected == null) {
        return false
    }
    val comparison = compareValues(actual, expected)
    return when (operator) {
        "<" -> comparison < 0
        ">" -> comparison > 0
        "<=" -> comparison <= 0
        ">=" -> comparison >= 0
        else -> false
    }
}

private fun valuesEqual(left: Any?, right: Any?): Boolean {
    if (left == null || right == null) {
        return left == right
    }
    if (left is Number && right is Number) {
        return left.toDouble().compareTo(right.toDouble()) == 0
    }
    return left == right
}

private fun compareValues(left: Any, right: Any): Int {
    if (left is Number && right is Number) {
        return left.toDouble().compareTo(right.toDouble())
    }
    return left.toString().compareTo(right.toString())
}

private fun applyOrder(table: Table, rows: MutableList<MutableMap<String, Any?>>, orderSql: String) {
    if (trim(orderSql).isEmpty()) {
        return
    }
    val parts = trim(orderSql).split(Regex("\\s+"))
    val column = canonicalColumn(table, parts[0])
    val desc = parts.size > 1 && parts[1].equals("DESC", ignoreCase = true)
    rows.sortWith { left, right ->
        val l = left[column]
        val r = right[column]
        val cmp = when {
            l == null && r == null -> 0
            l == null -> 1
            r == null -> -1
            else -> compareValues(l, r)
        }
        if (desc) -cmp else cmp
    }
}

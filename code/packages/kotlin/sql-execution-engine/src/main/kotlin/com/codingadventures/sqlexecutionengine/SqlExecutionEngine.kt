package com.codingadventures.sqlexecutionengine

import java.util.LinkedHashMap
import java.util.LinkedHashSet
import java.util.Locale
import java.util.regex.Pattern

interface DataSource {
    fun schema(tableName: String): List<String>
    fun scan(tableName: String): List<Map<String, Any?>>
}

data class QueryResult(val columns: List<String>, val rows: List<List<Any?>>)

data class ExecutionResult(val ok: Boolean, val result: QueryResult?, val error: String?) {
    companion object {
        fun success(result: QueryResult) = ExecutionResult(true, result, null)
        fun failure(error: String) = ExecutionResult(false, null, error)
    }
}

class SqlExecutionException(message: String, cause: Throwable? = null) : RuntimeException(message, cause)

class InMemoryDataSource : DataSource {
    private val schemas = LinkedHashMap<String, List<String>>()
    private val tables = LinkedHashMap<String, List<Map<String, Any?>>>()

    fun addTable(name: String, schema: List<String>, rows: List<Map<String, Any?>>): InMemoryDataSource {
        schemas[name] = schema.toList()
        tables[name] = rows.map { LinkedHashMap(it) }
        return this
    }

    override fun schema(tableName: String): List<String> =
        schemas[tableName]?.toList() ?: throw SqlExecutionException("table not found: $tableName")

    override fun scan(tableName: String): List<Map<String, Any?>> =
        tables[tableName]?.map { LinkedHashMap(it) } ?: throw SqlExecutionException("table not found: $tableName")
}

object SqlExecutionEngine {
    private val keywords = setOf(
        "SELECT", "FROM", "WHERE", "GROUP", "BY", "HAVING", "ORDER", "LIMIT", "OFFSET",
        "DISTINCT", "ALL", "JOIN", "INNER", "LEFT", "RIGHT", "FULL", "OUTER", "CROSS",
        "ON", "AS", "AND", "OR", "NOT", "IS", "NULL", "IN", "BETWEEN", "LIKE", "TRUE",
        "FALSE", "ASC", "DESC", "COUNT", "SUM", "AVG", "MIN", "MAX", "UPPER", "LOWER", "LENGTH"
    )

    fun execute(sql: String, dataSource: DataSource): QueryResult {
        return try {
            executeSelect(Parser(tokenize(sql)).parseStatement(), dataSource)
        } catch (ex: SqlExecutionException) {
            throw ex
        } catch (ex: RuntimeException) {
            throw SqlExecutionException(ex.message ?: "SQL execution failed", ex)
        }
    }

    fun tryExecute(sql: String, dataSource: DataSource): ExecutionResult =
        try {
            ExecutionResult.success(execute(sql, dataSource))
        } catch (ex: RuntimeException) {
            ExecutionResult.failure(ex.message ?: "SQL execution failed")
        }

    private fun executeSelect(statement: SelectStatement, dataSource: DataSource): QueryResult {
        var rows = scanTable(dataSource, statement.from.name, statement.from.alias)
        for (join in statement.joins) {
            rows = applyJoin(rows, scanTable(dataSource, join.table.name, join.table.alias), join)
        }

        statement.where?.let { where ->
            rows = rows.filter { truthy(eval(where, it.values, null)) }
        }

        var frames = makeFrames(rows, statement)
        statement.having?.let { having ->
            frames = frames.filter { truthy(eval(having, it.row.values, it.groupRows)) }
        }

        if (statement.orderBy.isNotEmpty()) {
            frames = frames.sortedWith { left, right -> compareOrder(left, right, statement.orderBy) }
        }

        val projected = project(frames, statement)
        var projectedRows = projected.rows

        if (statement.distinct) {
            val seen = LinkedHashSet<List<Any?>>()
            projectedRows = projectedRows.filter { seen.add(it) }
        }

        val from = maxOf(0, statement.offset ?: 0)
        projectedRows = if (from >= projectedRows.size) {
            emptyList()
        } else {
            val count = statement.limit?.let { maxOf(0, it) } ?: (projectedRows.size - from)
            projectedRows.subList(from, minOf(projectedRows.size, from + count))
        }

        return QueryResult(projected.columns, projectedRows)
    }

    private fun scanTable(dataSource: DataSource, tableName: String, alias: String): List<RowContext> {
        val schema = dataSource.schema(tableName)
        return dataSource.scan(tableName).map { raw ->
            val values = LinkedHashMap<String, Any?>()
            for (column in schema) {
                val value = raw[column]
                values[column] = value
                values["$alias.$column"] = value
                values["$tableName.$column"] = value
            }
            RowContext(values)
        }
    }

    private fun applyJoin(leftRows: List<RowContext>, rightRows: List<RowContext>, join: JoinDef): List<RowContext> {
        val joined = mutableListOf<RowContext>()
        if (join.type == "CROSS") {
            for (left in leftRows) {
                for (right in rightRows) joined += left.merge(right)
            }
            return joined
        }

        for (left in leftRows) {
            var matched = false
            for (right in rightRows) {
                val merged = left.merge(right)
                if (join.on == null || truthy(eval(join.on, merged.values, null))) {
                    joined += merged
                    matched = true
                }
            }
            if (!matched && join.type == "LEFT") joined += left
        }
        return joined
    }

    private fun makeFrames(rows: List<RowContext>, statement: SelectStatement): List<RowFrame> {
        val grouped = statement.groupBy.isNotEmpty()
        val aggregated = statement.selectItems.any { hasAggregate(it.expression) } ||
            (statement.having?.let { hasAggregate(it) } ?: false)
        if (!grouped && !aggregated) return rows.map { RowFrame(it, null) }

        if (!grouped) {
            val row = rows.firstOrNull() ?: RowContext(LinkedHashMap())
            return listOf(RowFrame(row, rows))
        }

        val groups = LinkedHashMap<List<Any?>, MutableList<RowContext>>()
        for (row in rows) {
            val key = statement.groupBy.map { eval(it, row.values, null) }
            groups.getOrPut(key) { mutableListOf() } += row
        }
        return groups.values.map { RowFrame(it.first(), it) }
    }

    private fun project(frames: List<RowFrame>, statement: SelectStatement): Projection {
        if (statement.selectItems.size == 1 && statement.selectItems[0].expression is Expr.Star) {
            val columns = frames.firstOrNull()?.row?.values?.keys
                ?.filter { !it.contains(".") }
                ?.sorted()
                ?: emptyList()
            val rows = frames.map { frame -> columns.map { frame.row.values[it] } }
            return Projection(columns, rows)
        }

        val columns = statement.selectItems.map { it.alias ?: expressionLabel(it.expression) }
        val rows = frames.map { frame ->
            statement.selectItems.map { eval(it.expression, frame.row.values, frame.groupRows) }
        }
        return Projection(columns, rows)
    }

    private fun compareOrder(left: RowFrame, right: RowFrame, orderBy: List<OrderItem>): Int {
        for (item in orderBy) {
            val comparison = compareSql(
                eval(item.expression, left.row.values, left.groupRows),
                eval(item.expression, right.row.values, right.groupRows)
            )
            if (comparison != 0) return if (item.descending) -comparison else comparison
        }
        return 0
    }

    private fun eval(expression: Expr, row: Map<String, Any?>, groupRows: List<RowContext>?): Any? {
        return when (expression) {
            is Expr.Literal -> expression.value
            Expr.NullValue -> null
            is Expr.Column -> {
                if (expression.table != null) {
                    row["${expression.table}.${expression.name}"]
                } else if (row.containsKey(expression.name)) {
                    row[expression.name]
                } else {
                    row.entries.firstOrNull { it.key.endsWith(".${expression.name}") }?.value
                }
            }
            Expr.Star -> row
            is Expr.Unary -> {
                val value = eval(expression.expression, row, groupRows)
                when (expression.operator) {
                    "NOT" -> if (value == null) null else !truthy(value)
                    "-" -> if (value == null) null else -asDouble(value)
                    else -> throw SqlExecutionException("unknown unary operator: ${expression.operator}")
                }
            }
            is Expr.Binary -> evalBinary(expression, row, groupRows)
            is Expr.IsNull -> {
                val result = eval(expression.expression, row, groupRows) == null
                if (expression.negated) !result else result
            }
            is Expr.Between -> {
                val value = eval(expression.expression, row, groupRows)
                val lower = eval(expression.lower, row, groupRows)
                val upper = eval(expression.upper, row, groupRows)
                if (value == null || lower == null || upper == null) {
                    null
                } else {
                    val result = compareSql(value, lower) >= 0 && compareSql(value, upper) <= 0
                    if (expression.negated) !result else result
                }
            }
            is Expr.InList -> {
                val value = eval(expression.expression, row, groupRows)
                if (value == null) {
                    null
                } else {
                    val found = expression.values.any {
                        val option = eval(it, row, groupRows)
                        option != null && sqlEquals(value, option)
                    }
                    if (expression.negated) !found else found
                }
            }
            is Expr.Like -> {
                val value = eval(expression.expression, row, groupRows)
                val pattern = eval(expression.pattern, row, groupRows)
                if (value == null || pattern == null) null
                else {
                    val result = like(value.toString(), pattern.toString())
                    if (expression.negated) !result else result
                }
            }
            is Expr.Function -> evalFunction(expression, row, groupRows)
        }
    }

    private fun evalBinary(binary: Expr.Binary, row: Map<String, Any?>, groupRows: List<RowContext>?): Any? {
        if (binary.operator == "AND") {
            val left = eval(binary.left, row, groupRows)
            if (left != null && !truthy(left)) return false
            val right = eval(binary.right, row, groupRows)
            if (right != null && !truthy(right)) return false
            return if (left == null || right == null) null else true
        }
        if (binary.operator == "OR") {
            val left = eval(binary.left, row, groupRows)
            if (left != null && truthy(left)) return true
            val right = eval(binary.right, row, groupRows)
            if (right != null && truthy(right)) return true
            return if (left == null || right == null) null else false
        }

        val left = eval(binary.left, row, groupRows)
        val right = eval(binary.right, row, groupRows)
        if (left == null || right == null) return null
        return when (binary.operator) {
            "+" -> asDouble(left) + asDouble(right)
            "-" -> asDouble(left) - asDouble(right)
            "*" -> asDouble(left) * asDouble(right)
            "/" -> asDouble(left) / asDouble(right)
            "%" -> asDouble(left) % asDouble(right)
            "=" -> sqlEquals(left, right)
            "!=", "<>" -> !sqlEquals(left, right)
            "<" -> compareSql(left, right) < 0
            ">" -> compareSql(left, right) > 0
            "<=" -> compareSql(left, right) <= 0
            ">=" -> compareSql(left, right) >= 0
            else -> throw SqlExecutionException("unknown operator: ${binary.operator}")
        }
    }

    private fun evalFunction(function: Expr.Function, row: Map<String, Any?>, groupRows: List<RowContext>?): Any? {
        val name = function.name.uppercase(Locale.ROOT)
        if (name in setOf("COUNT", "SUM", "AVG", "MIN", "MAX")) {
            val rows = groupRows ?: throw SqlExecutionException("aggregate used outside grouped context: $name")
            if (name == "COUNT") {
                if (function.args.size == 1 && function.args[0] is Expr.Star) return rows.size
                val arg = function.args.firstOrNull() ?: return rows.size
                return rows.count { eval(arg, it.values, null) != null }
            }
            val arg = function.args.firstOrNull() ?: throw SqlExecutionException("aggregate requires an argument: $name")
            val values = rows.mapNotNull { eval(arg, it.values, null) }
            if (values.isEmpty()) return null
            return when (name) {
                "SUM" -> values.fold(0.0) { acc, value -> acc + asDouble(value) }
                "AVG" -> values.fold(0.0) { acc, value -> acc + asDouble(value) } / values.size
                "MIN" -> values.reduce { best, next -> if (compareSql(next, best) < 0) next else best }
                "MAX" -> values.reduce { best, next -> if (compareSql(next, best) > 0) next else best }
                else -> throw SqlExecutionException("unknown aggregate: $name")
            }
        }

        val value = function.args.firstOrNull()?.let { eval(it, row, groupRows) } ?: return null
        return when (name) {
            "UPPER" -> value.toString().uppercase(Locale.ROOT)
            "LOWER" -> value.toString().lowercase(Locale.ROOT)
            "LENGTH" -> value.toString().length
            else -> throw SqlExecutionException("unknown function: $name")
        }
    }

    private fun hasAggregate(expression: Expr): Boolean {
        return when (expression) {
            is Expr.Function ->
                expression.name.uppercase(Locale.ROOT) in setOf("COUNT", "SUM", "AVG", "MIN", "MAX") ||
                    expression.args.any { hasAggregate(it) }
            is Expr.Binary -> hasAggregate(expression.left) || hasAggregate(expression.right)
            is Expr.Unary -> hasAggregate(expression.expression)
            is Expr.IsNull -> hasAggregate(expression.expression)
            is Expr.Between -> hasAggregate(expression.expression) || hasAggregate(expression.lower) || hasAggregate(expression.upper)
            is Expr.InList -> hasAggregate(expression.expression) || expression.values.any { hasAggregate(it) }
            is Expr.Like -> hasAggregate(expression.expression) || hasAggregate(expression.pattern)
            else -> false
        }
    }

    private fun truthy(value: Any?): Boolean =
        when (value) {
            null -> false
            is Boolean -> value
            is Number -> value.toDouble() != 0.0
            is String -> value.isNotEmpty()
            else -> true
        }

    private fun sqlEquals(left: Any?, right: Any?): Boolean =
        when {
            left == null || right == null -> left == right
            left is Number && right is Number -> left.toDouble().compareTo(right.toDouble()) == 0
            else -> left == right
        }

    private fun compareSql(left: Any?, right: Any?): Int {
        val rank = rank(left).compareTo(rank(right))
        if (rank != 0) return rank
        return when {
            left == null && right == null -> 0
            left is Boolean && right is Boolean -> left.compareTo(right)
            left is Number && right is Number -> left.toDouble().compareTo(right.toDouble())
            left is String && right is String -> left.compareTo(right)
            else -> left.toString().compareTo(right.toString())
        }
    }

    private fun rank(value: Any?): Int =
        when (value) {
            null -> 0
            is Boolean -> 1
            is Number -> 2
            is String -> 3
            else -> 4
        }

    private fun asDouble(value: Any?): Double =
        when (value) {
            is Number -> value.toDouble()
            else -> value.toString().toDouble()
        }

    private fun expressionLabel(expression: Expr): String =
        when (expression) {
            is Expr.Column -> expression.name
            is Expr.Function ->
                if (expression.args.size == 1 && expression.args[0] is Expr.Star) {
                    expression.name.uppercase(Locale.ROOT) + "(*)"
                } else {
                    expression.name.uppercase(Locale.ROOT) + "(...)"
                }
            is Expr.Literal -> expression.value.toString()
            else -> "?"
        }

    private fun like(value: String, sqlPattern: String): Boolean {
        val regex = StringBuilder()
        for (ch in sqlPattern) {
            when (ch) {
                '%' -> regex.append(".*")
                '_' -> regex.append('.')
                else -> regex.append(Pattern.quote(ch.toString()))
            }
        }
        return value.matches(Regex(regex.toString()))
    }

    private fun tokenize(sql: String): List<Token> {
        val tokens = mutableListOf<Token>()
        var index = 0
        while (index < sql.length) {
            val ch = sql[index]
            when {
                ch.isWhitespace() -> index += 1
                ch == '-' && index + 1 < sql.length && sql[index + 1] == '-' -> {
                    index += 2
                    while (index < sql.length && sql[index] != '\n') index += 1
                }
                ch == '\'' -> {
                    val value = StringBuilder()
                    index += 1
                    while (index < sql.length) {
                        val current = sql[index]
                        if (current == '\'' && index + 1 < sql.length && sql[index + 1] == '\'') {
                            value.append('\'')
                            index += 2
                        } else if (current == '\'') {
                            index += 1
                            break
                        } else {
                            value.append(current)
                            index += 1
                        }
                    }
                    tokens += Token(TokenKind.STRING, value.toString())
                }
                ch.isDigit() || (ch == '.' && index + 1 < sql.length && sql[index + 1].isDigit()) -> {
                    val start = index
                    index += 1
                    while (index < sql.length && (sql[index].isDigit() || sql[index] == '.')) index += 1
                    tokens += Token(TokenKind.NUMBER, sql.substring(start, index))
                }
                ch.isLetter() || ch == '_' -> {
                    val start = index
                    index += 1
                    while (index < sql.length && (sql[index].isLetterOrDigit() || sql[index] == '_')) index += 1
                    val value = sql.substring(start, index)
                    val upper = value.uppercase(Locale.ROOT)
                    tokens += Token(if (upper in keywords) TokenKind.KEYWORD else TokenKind.IDENT, value)
                }
                ch == '"' || ch == '`' -> {
                    val quote = ch
                    val start = index + 1
                    index = start
                    while (index < sql.length && sql[index] != quote) index += 1
                    tokens += Token(TokenKind.IDENT, sql.substring(start, index))
                    if (index < sql.length) index += 1
                }
                else -> {
                    if (index + 1 < sql.length) {
                        val two = sql.substring(index, index + 2)
                        if (two in setOf("!=", "<>", "<=", ">=")) {
                            tokens += Token(TokenKind.SYMBOL, two)
                            index += 2
                            continue
                        }
                    }
                    if ("=<>+-*/%(),.;".contains(ch)) tokens += Token(TokenKind.SYMBOL, ch.toString())
                    index += 1
                }
            }
        }
        tokens += Token(TokenKind.EOF, "")
        return tokens
    }

    private class Parser(private val tokens: List<Token>) {
        private var position = 0
        private fun peek(): Token = tokens[position]
        private fun advance(): Token {
            val token = tokens[position]
            if (token.kind != TokenKind.EOF) position += 1
            return token
        }
        private fun error(message: String) = SqlExecutionException("$message near token $position")

        fun parseStatement(): SelectStatement {
            expectKeyword("SELECT")
            val distinct = matchKeyword("DISTINCT")
            matchKeyword("ALL")
            val selectItems = parseSelectList()
            expectKeyword("FROM")
            val from = parseTableRef()
            val joins = parseJoins()
            val where = if (matchKeyword("WHERE")) parseExpression() else null
            val groupBy = if (matchKeyword("GROUP")) {
                expectKeyword("BY")
                parseExpressionList()
            } else emptyList()
            val having = if (matchKeyword("HAVING")) parseExpression() else null
            val orderBy = if (matchKeyword("ORDER")) {
                expectKeyword("BY")
                parseOrderList()
            } else emptyList()
            val limit = if (matchKeyword("LIMIT")) numberAsInt(advance()) else null
            val offset = if (matchKeyword("OFFSET")) numberAsInt(advance()) else null
            matchSymbol(";")
            expect(TokenKind.EOF)
            return SelectStatement(distinct, selectItems, from, joins, where, groupBy, having, orderBy, limit, offset)
        }

        private fun parseSelectList(): List<SelectItem> {
            val items = mutableListOf<SelectItem>()
            do {
                if (matchSymbol("*")) {
                    items += SelectItem(Expr.Star, null)
                } else {
                    val expression = parseExpression()
                    val alias = if (matchKeyword("AS")) expectIdentifier()
                    else if (peek().kind == TokenKind.IDENT) advance().value
                    else null
                    items += SelectItem(expression, alias)
                }
            } while (matchSymbol(","))
            return items
        }

        private fun parseTableRef(): TableRef {
            val name = expectIdentifier()
            val alias = if (matchKeyword("AS")) expectIdentifier()
            else if (peek().kind == TokenKind.IDENT) advance().value
            else name
            return TableRef(name, alias)
        }

        private fun parseJoins(): List<JoinDef> {
            val joins = mutableListOf<JoinDef>()
            while (true) {
                val type = when {
                    matchKeyword("INNER") -> { expectKeyword("JOIN"); "INNER" }
                    matchKeyword("LEFT") -> { matchKeyword("OUTER"); expectKeyword("JOIN"); "LEFT" }
                    matchKeyword("CROSS") -> { expectKeyword("JOIN"); "CROSS" }
                    matchKeyword("JOIN") -> "INNER"
                    else -> null
                } ?: break
                val table = parseTableRef()
                val on = if (type == "CROSS") null else { expectKeyword("ON"); parseExpression() }
                joins += JoinDef(type, table, on)
            }
            return joins
        }

        private fun parseExpressionList(): List<Expr> {
            val expressions = mutableListOf<Expr>()
            do {
                expressions += parseExpression()
            } while (matchSymbol(","))
            return expressions
        }

        private fun parseOrderList(): List<OrderItem> {
            val items = mutableListOf<OrderItem>()
            do {
                val expression = parseExpression()
                val descending = if (matchKeyword("ASC")) false else matchKeyword("DESC")
                items += OrderItem(expression, descending)
            } while (matchSymbol(","))
            return items
        }

        private fun parseExpression(): Expr = parseOr()

        private fun parseOr(): Expr {
            var left = parseAnd()
            while (matchKeyword("OR")) left = Expr.Binary("OR", left, parseAnd())
            return left
        }

        private fun parseAnd(): Expr {
            var left = parseNot()
            while (matchKeyword("AND")) left = Expr.Binary("AND", left, parseNot())
            return left
        }

        private fun parseNot(): Expr =
            if (matchKeyword("NOT")) Expr.Unary("NOT", parseNot()) else parseComparison()

        private fun parseComparison(): Expr {
            val left = parseAdditive()
            if (matchKeyword("IS")) {
                val negated = matchKeyword("NOT")
                expectKeyword("NULL")
                return Expr.IsNull(left, negated)
            }
            if (matchKeyword("NOT")) {
                return when {
                    matchKeyword("BETWEEN") -> {
                        val lower = parseAdditive()
                        expectKeyword("AND")
                        Expr.Between(left, lower, parseAdditive(), true)
                    }
                    matchKeyword("IN") -> Expr.InList(left, parseInValues(), true)
                    matchKeyword("LIKE") -> Expr.Like(left, parseAdditive(), true)
                    else -> throw error("expected BETWEEN, IN, or LIKE after NOT")
                }
            }
            return when {
                matchKeyword("BETWEEN") -> {
                    val lower = parseAdditive()
                    expectKeyword("AND")
                    Expr.Between(left, lower, parseAdditive(), false)
                }
                matchKeyword("IN") -> Expr.InList(left, parseInValues(), false)
                matchKeyword("LIKE") -> Expr.Like(left, parseAdditive(), false)
                peek().kind == TokenKind.SYMBOL && peek().value in setOf("=", "!=", "<>", "<", ">", "<=", ">=") -> {
                    val operator = advance().value
                    Expr.Binary(operator, left, parseAdditive())
                }
                else -> left
            }
        }

        private fun parseInValues(): List<Expr> {
            expectSymbol("(")
            val values = parseExpressionList()
            expectSymbol(")")
            return values
        }

        private fun parseAdditive(): Expr {
            var left = parseMultiplicative()
            while (peek().kind == TokenKind.SYMBOL && peek().value in setOf("+", "-")) {
                val operator = advance().value
                left = Expr.Binary(operator, left, parseMultiplicative())
            }
            return left
        }

        private fun parseMultiplicative(): Expr {
            var left = parseUnary()
            while (peek().kind == TokenKind.SYMBOL && peek().value in setOf("*", "/", "%")) {
                val operator = advance().value
                left = Expr.Binary(operator, left, parseUnary())
            }
            return left
        }

        private fun parseUnary(): Expr =
            if (matchSymbol("-")) Expr.Unary("-", parseUnary()) else parsePrimary()

        private fun parsePrimary(): Expr {
            val token = peek()
            if (matchSymbol("(")) {
                val expression = parseExpression()
                expectSymbol(")")
                return expression
            }
            if (token.kind == TokenKind.NUMBER) {
                advance()
                return if (token.value.contains(".")) Expr.Literal(token.value.toDouble()) else Expr.Literal(token.value.toLong())
            }
            if (token.kind == TokenKind.STRING) {
                advance()
                return Expr.Literal(token.value)
            }
            if (matchKeyword("NULL")) return Expr.NullValue
            if (matchKeyword("TRUE")) return Expr.Literal(true)
            if (matchKeyword("FALSE")) return Expr.Literal(false)
            if (matchSymbol("*")) return Expr.Star
            if (token.kind == TokenKind.IDENT || token.kind == TokenKind.KEYWORD) {
                val name = advance().value
                if (matchSymbol("(")) {
                    val args = mutableListOf<Expr>()
                    if (!matchSymbol(")")) {
                        if (matchSymbol("*")) args += Expr.Star
                        else {
                            args += parseExpression()
                            while (matchSymbol(",")) args += parseExpression()
                        }
                        expectSymbol(")")
                    }
                    return Expr.Function(name, args)
                }
                if (matchSymbol(".")) return Expr.Column(name, expectIdentifier())
                return Expr.Column(null, name)
            }
            throw error("unexpected token: ${token.value}")
        }

        private fun expectIdentifier(): String {
            val token = advance()
            if (token.kind == TokenKind.IDENT || token.kind == TokenKind.KEYWORD) return token.value
            throw error("expected identifier")
        }

        private fun numberAsInt(token: Token): Int {
            if (token.kind != TokenKind.NUMBER) throw error("expected number")
            return token.value.toInt()
        }

        private fun expect(kind: TokenKind) {
            val token = advance()
            if (token.kind != kind) throw error("expected $kind, got ${token.kind}")
        }

        private fun expectKeyword(value: String) {
            if (!matchKeyword(value)) throw error("expected $value")
        }

        private fun matchKeyword(value: String): Boolean {
            if (peek().kind == TokenKind.KEYWORD && peek().value.equals(value, ignoreCase = true)) {
                advance()
                return true
            }
            return false
        }

        private fun expectSymbol(value: String) {
            if (!matchSymbol(value)) throw error("expected $value")
        }

        private fun matchSymbol(value: String): Boolean {
            if (peek().kind == TokenKind.SYMBOL && peek().value == value) {
                advance()
                return true
            }
            return false
        }
    }
}

private enum class TokenKind { IDENT, KEYWORD, NUMBER, STRING, SYMBOL, EOF }
private data class Token(val kind: TokenKind, val value: String)

private data class SelectStatement(
    val distinct: Boolean,
    val selectItems: List<SelectItem>,
    val from: TableRef,
    val joins: List<JoinDef>,
    val where: Expr?,
    val groupBy: List<Expr>,
    val having: Expr?,
    val orderBy: List<OrderItem>,
    val limit: Int?,
    val offset: Int?
)
private data class SelectItem(val expression: Expr, val alias: String?)
private data class TableRef(val name: String, val alias: String)
private data class JoinDef(val type: String, val table: TableRef, val on: Expr?)
private data class OrderItem(val expression: Expr, val descending: Boolean)
private data class RowContext(val values: LinkedHashMap<String, Any?>) {
    fun merge(other: RowContext): RowContext {
        val merged = LinkedHashMap(values)
        merged.putAll(other.values)
        return RowContext(merged)
    }
}
private data class RowFrame(val row: RowContext, val groupRows: List<RowContext>?)
private data class Projection(val columns: List<String>, val rows: List<List<Any?>>)

private sealed class Expr {
    data class Literal(val value: Any?) : Expr()
    object NullValue : Expr()
    data class Column(val table: String?, val name: String) : Expr()
    object Star : Expr()
    data class Unary(val operator: String, val expression: Expr) : Expr()
    data class Binary(val operator: String, val left: Expr, val right: Expr) : Expr()
    data class IsNull(val expression: Expr, val negated: Boolean) : Expr()
    data class Between(val expression: Expr, val lower: Expr, val upper: Expr, val negated: Boolean) : Expr()
    data class InList(val expression: Expr, val values: List<Expr>, val negated: Boolean) : Expr()
    data class Like(val expression: Expr, val pattern: Expr, val negated: Boolean) : Expr()
    data class Function(val name: String, val args: List<Expr>) : Expr()
}

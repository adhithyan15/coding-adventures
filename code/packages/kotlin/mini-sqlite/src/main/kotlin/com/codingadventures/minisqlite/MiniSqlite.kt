package com.codingadventures.minisqlite

// MiniSqlite.kt — Level 1 implementation of the mini-sqlite DB-API facade.
//
// Architecture overview
// ─────────────────────
// The Level 1 pipeline replaces the hand-rolled Level 0 interpreter with a
// proper compilation stack:
//
//   SQL text
//     │  bindParameters()          substitute ? placeholders with literal values
//     ▼
//   bound SQL string
//     │  MiniSqliteParser.parse()  recursive-descent parser → Statement AST
//     ▼
//   Statement  (sql-planner types)
//     │  SqlPlanner.plan()         resolves columns, builds LogicalPlan tree
//     ▼
//   LogicalPlan
//     │  SqlOptimizer.optimize()   constant-folding, predicate pushdown, etc.
//     ▼
//   OptimizedPlan
//     │  SqlCodegen.compile()      emits flat bytecode instructions
//     ▼
//   Program
//     │  SqlVm.execute()           stack-machine execution against InMemoryBackend
//     ▼
//   QueryResult
//     │  queryResultToResult()     map SqlValue rows → List<Any?> rows
//     ▼
//   Result (internal type consumed by Cursor)
//
// Special cases
// ─────────────
// • SELECT without FROM (e.g. `SELECT LENGTH('hello') AS n`):
//   The sql-planner throws UnsupportedStatementException.  We detect this
//   up-front and evaluate the expressions directly.
//
// • SELECT with function calls (e.g. `SELECT LENGTH(s) FROM t`):
//   The sql-codegen emits FuncCall nodes as a `LoadConst("__func:NAME")`
//   sentinel followed by arguments — the VM does NOT dispatch function calls.
//   We detect the presence of FuncCalls in SELECT output columns and route
//   these queries through a row-level direct evaluator instead of the VM.
//
// • Transaction control (BEGIN / COMMIT / ROLLBACK): handled by forwarding
//   to InMemoryBackend.beginTransaction / commit / rollback.

import com.codingadventures.sqlbackend.InMemoryBackend
import com.codingadventures.sqlbackend.Row
import com.codingadventures.sqlbackend.TableNotFound
import com.codingadventures.sqlcodegen.SqlCodegen
import com.codingadventures.sqlcodegen.SqlValue
import com.codingadventures.sqloptimizer.SqlOptimizer
import com.codingadventures.sqlplanner.*
import com.codingadventures.sqlvm.SqlVm

// ── Public constants and entry point ──────────────────────────────────────────

object MiniSqlite {
    /** DB-API 2.0 version string. */
    const val API_LEVEL = "2.0"
    /** Thread safety level: 1 = threads may share the module, not connections. */
    const val THREADSAFETY = 1
    /** Parameter style used for placeholders: '?' positional. */
    const val PARAMSTYLE = "qmark"

    /**
     * Open an in-memory database connection.
     *
     * Only `":memory:"` is supported; passing any other string raises
     * a `NotSupportedError`.
     */
    fun connect(database: String, options: Options = Options()): Connection {
        if (database != ":memory:") {
            throw MiniSqliteException("NotSupportedError", "Kotlin mini-sqlite supports only :memory:")
        }
        return Connection(options)
    }
}

// ── Data model ────────────────────────────────────────────────────────────────

data class Options(val autocommit: Boolean = false)
data class Column(val name: String)

class MiniSqliteException(val kind: String, message: String) : RuntimeException(message)

// ── Connection ────────────────────────────────────────────────────────────────
//
// A Connection wraps a single InMemoryBackend instance.  Transactions are
// managed at the backend level:
//
//   • autocommit=true:  no transaction is ever opened; each statement is
//     immediately visible after execution.
//
//   • autocommit=false (default): a transaction is opened lazily before the
//     first write.  It remains open until the caller calls commit() or
//     rollback().  On close() any open transaction is rolled back.

class Connection internal constructor(options: Options) : AutoCloseable {
    internal val db = InMemoryBackend()
    private val autocommit = options.autocommit
    private var txHandle: com.codingadventures.sqlbackend.TransactionHandle? = null
    private var closed = false

    // ── Public API ─────────────────────────────────────────────────────────

    fun cursor(): Cursor {
        assertOpen()
        return Cursor(this)
    }

    fun execute(sql: String, params: List<Any?> = emptyList()): Cursor =
        cursor().execute(sql, params)

    fun executemany(sql: String, paramsSeq: List<List<Any?>>): Cursor =
        cursor().executemany(sql, paramsSeq)

    /** Commit the current transaction.  No-op if no transaction is open. */
    fun commit() {
        assertOpen()
        val h = txHandle
        if (h != null) {
            db.commit(h)
            txHandle = null
        }
    }

    /** Roll back the current transaction.  No-op if no transaction is open. */
    fun rollback() {
        assertOpen()
        val h = txHandle
        if (h != null) {
            db.rollback(h)
            txHandle = null
        }
    }

    /**
     * Close the connection.  Any uncommitted transaction is rolled back.
     *
     * Calling close() on an already-closed connection is a no-op.
     */
    override fun close() {
        if (closed) return
        val h = txHandle
        if (h != null) {
            db.rollback(h)
            txHandle = null
        }
        closed = true
    }

    // ── Internal helpers ──────────────────────────────────────────────────

    private fun assertOpen() {
        if (closed) throw MiniSqliteException("ProgrammingError", "connection is closed")
    }

    /**
     * Lazily begin a transaction before the first write.
     *
     * In autocommit mode this is a no-op.  In manual mode a transaction is
     * opened the first time a DML or DDL statement runs.
     */
    internal fun ensureTransaction() {
        if (autocommit) return
        if (txHandle == null) {
            txHandle = db.beginTransaction()
        }
    }

    /**
     * Execute a parameter-bound SQL string and return an internal Result.
     *
     * Routes each statement to the appropriate execution path:
     *   • Transaction control: BEGIN / COMMIT / ROLLBACK
     *   • SELECT without FROM: direct expression evaluator
     *   • SELECT with function calls: row-level direct evaluator
     *   • Everything else: full pipeline (parse → plan → optimize → codegen → vm)
     */
    internal fun executeBound(sql: String, params: List<Any?>): Result {
        assertOpen()
        val bound = bindParameters(sql, params)
        return try {
            val kw = firstKeyword(bound)
            when (kw) {
                // ── Transaction control ────────────────────────────────────────
                "BEGIN" -> {
                    ensureTransaction()
                    Result.empty(0)
                }
                "COMMIT" -> {
                    commit()
                    Result.empty(0)
                }
                "ROLLBACK" -> {
                    rollback()
                    Result.empty(0)
                }

                // ── SELECT ────────────────────────────────────────────────────
                "SELECT" -> {
                    if (MiniSqliteParser.isSelectWithoutFrom(bound)) {
                        executeSelectWithoutFrom(bound)
                    } else {
                        executeSelectParsed(bound)
                    }
                }

                // ── DDL (pipeline is correct for these) ──────────────────────
                "CREATE", "DROP", "INSERT" -> {
                    ensureTransaction()
                    runPipeline(bound)
                }

                // ── DML — direct path (VM pipeline has bugs with IN/UPDATE) ──
                "UPDATE" -> {
                    ensureTransaction()
                    val stmt = MiniSqliteParser.parse(bound) as? Statement.Update
                        ?: throw MiniSqliteException("OperationalError", "expected UPDATE")
                    executeUpdateDirect(stmt)
                }

                "DELETE" -> {
                    ensureTransaction()
                    val stmt = MiniSqliteParser.parse(bound) as? Statement.Delete
                        ?: throw MiniSqliteException("OperationalError", "expected DELETE")
                    executeDeleteDirect(stmt)
                }

                else -> throw IllegalArgumentException("unsupported SQL statement: $kw")
            }
        } catch (ex: MiniSqliteException) {
            throw ex
        } catch (ex: TableNotFound) {
            throw MiniSqliteException("OperationalError", ex.message ?: ex.javaClass.simpleName)
        } catch (ex: com.codingadventures.sqlbackend.BackendError) {
            throw MiniSqliteException("OperationalError", ex.message ?: ex.javaClass.simpleName)
        } catch (ex: UnknownTableException) {
            throw MiniSqliteException("OperationalError", ex.message ?: ex.javaClass.simpleName)
        } catch (ex: RuntimeException) {
            throw MiniSqliteException("OperationalError", ex.message ?: ex.javaClass.simpleName)
        }
    }

    // ── SELECT dispatcher ─────────────────────────────────────────────────────
    //
    // All SELECT queries are handled by the direct evaluator.
    // The sql-vm pipeline has multiple known issues:
    //   • SELECT * emits a sentinel string "*" instead of expanding columns
    //   • FuncCall emits a sentinel "__func:NAME" that the VM doesn't dispatch
    //   • GROUP BY result rows have null values for group key columns
    //   • COUNT(DISTINCT col) doesn't deduplicate
    //
    // The direct evaluator implements all SQL operations correctly in Kotlin,
    // including aggregation (GROUP BY, HAVING, COUNT/SUM/AVG/MIN/MAX).

    private fun executeSelectParsed(sql: String): Result {
        val stmt: Statement = try {
            MiniSqliteParser.parse(sql)
        } catch (ex: ParseException) {
            throw MiniSqliteException("OperationalError", ex.message ?: "parse error")
        }
        val select = stmt as? Statement.Select
            ?: throw MiniSqliteException("OperationalError", "expected SELECT")

        // Check if the query has aggregate functions.
        val hasAgg = select.columns.any { col ->
            col is OutputColumn.Expr && containsAgg(col.expression)
        }

        return if (hasAgg) {
            executeAggregateDirect(select)
        } else {
            executeSelectDirect(select)
        }
    }

    // ── Full pipeline execution ────────────────────────────────────────────────

    /**
     * Run [sql] through the full parse → plan → optimize → codegen → vm pipeline.
     */
    private fun runPipeline(sql: String): Result {
        val stmt: Statement = try {
            MiniSqliteParser.parse(sql)
        } catch (ex: ParseException) {
            throw MiniSqliteException("OperationalError", ex.message ?: "parse error")
        }
        return runPipelineWithStmt(stmt, sql)
    }

    private fun runPipelineWithStmt(stmt: Statement, originalSql: String): Result {
        val schemaProvider = backendSchemaProvider(db)
        val planner = SqlPlanner(schemaProvider)
        val logicalPlan: LogicalPlan = try {
            planner.plan(stmt)
        } catch (ex: PlanException) {
            throw MiniSqliteException("OperationalError", ex.message ?: "plan error")
        }
        val optimizedPlan = SqlOptimizer.optimize(logicalPlan)
        val program = SqlCodegen.compile(optimizedPlan)
        val queryResult = SqlVm.execute(program, db)
        return queryResultToResult(queryResult)
    }

    // ── Direct SELECT evaluator ───────────────────────────────────────────────
    //
    // Used for SELECT queries that contain function calls in output columns,
    // because the VM does not dispatch functions.
    //
    // Algorithm:
    //   1. Scan the FROM table(s) via the backend.
    //   2. For each row, evaluate the WHERE predicate (if any).
    //   3. Evaluate each output column expression against the row.
    //   4. Apply ORDER BY, LIMIT, OFFSET.
    //
    // Limitations (acceptable for Level 1):
    //   • Only single-table FROM (no JOINs in direct mode).
    //   • GROUP BY / HAVING not supported in direct mode (these go through the VM).
    //   • DISTINCT not supported in direct mode (goes through VM).
    //
    // For queries that mix functions AND aggregates, the VM path is used (those
    // queries are not in the Level 1 conformance fixtures).

    private fun executeSelectDirect(select: Statement.Select): Result {
        if (select.from.isEmpty()) {
            throw MiniSqliteException("OperationalError", "SELECT without FROM not supported here")
        }

        val tableRef = select.from[0]
        val tableName = tableRef.table
        val tableAlias = tableRef.alias ?: tableName

        // Validate the table exists.
        val colDefs: List<com.codingadventures.sqlbackend.ColumnDef> = try {
            db.columns(tableName)
        } catch (ex: TableNotFound) {
            throw MiniSqliteException("OperationalError", "no such table: $tableName")
        }
        val schemaColumns = colDefs.map { it.name }

        // Scan all rows.
        val allRows = mutableListOf<Row>()
        val iter = db.scan(tableName)
        while (true) { allRows.add(iter.next() ?: break) }
        iter.close()

        // Filter by WHERE.
        val whereExpr = select.where
        val filtered = if (whereExpr != null) {
            allRows.filter { row ->
                val v = evalRowExpr(whereExpr, row, tableAlias, schemaColumns)
                isTruthy(v)
            }
        } else {
            allRows
        }

        // Build output column names.
        val outColumns = mutableListOf<String>()
        for (col in select.columns) {
            when (col) {
                is OutputColumn.Star -> outColumns.addAll(schemaColumns)
                is OutputColumn.Expr -> outColumns.add(col.alias ?: exprName(col.expression))
            }
        }

        // Build output rows.
        var outRows = filtered.map { row ->
            val values = mutableListOf<Any?>()
            for (col in select.columns) {
                when (col) {
                    is OutputColumn.Star -> schemaColumns.forEach { c -> values.add(row[c]) }
                    is OutputColumn.Expr -> values.add(evalRowExpr(col.expression, row, tableAlias, schemaColumns))
                }
            }
            values.toList()
        }

        // Apply ORDER BY.
        if (select.orderBy.isNotEmpty()) {
            outRows = applyOrderBy(outRows, outColumns, select.orderBy, filtered, tableAlias, schemaColumns, select)
        }

        // Apply DISTINCT.
        if (select.distinct) {
            val seen = linkedSetOf<List<Any?>>()
            outRows = outRows.filter { seen.add(it) }
        }

        // Apply LIMIT / OFFSET.
        val limit = select.limit
        if (limit != null) {
            val start = (limit.offset ?: 0L).toInt().coerceAtLeast(0)
            val count = limit.count?.toInt()
            outRows = if (count == null) outRows.drop(start)
                      else outRows.drop(start).take(count)
        }

        return Result(outColumns, outRows, -1)
    }

    // ── Direct UPDATE / DELETE evaluators ─────────────────────────────────────
    //
    // The sql-vm pipeline has bugs with IN predicates in UPDATE/DELETE (stack
    // underflow).  We implement these operations directly using the backend's
    // positioned-update / positioned-delete cursor API.

    /**
     * Execute UPDATE table SET col=expr [, ...] [WHERE predicate].
     *
     * Uses `backend.openCursor()` for positioned update.
     * Returns a Result with rowsAffected set.
     */
    private fun executeUpdateDirect(stmt: Statement.Update): Result {
        // Validate the table exists and get column list.
        val colDefs = try { db.columns(stmt.table) } catch (ex: TableNotFound) {
            throw MiniSqliteException("OperationalError", "no such table: ${stmt.table}")
        }
        val schemaColumns = colDefs.map { it.name }

        val cursor = db.openCursor(stmt.table)
        var affected = 0
        try {
            // Use next() to advance; currentRow() gives the current row for
            // positioned update.  We must call next() BEFORE reading currentRow().
            while (true) {
                val row = cursor.next() ?: break
                // Evaluate WHERE predicate against current row.
                val whereExpr = stmt.where
                if (whereExpr != null) {
                    val v = evalRowExpr(whereExpr, row, stmt.table, schemaColumns)
                    if (!isTruthy(v)) continue
                }
                // Build the assignment map.
                val assignments = stmt.assignments.associate { a ->
                    a.column to evalRowExpr(a.value, row, stmt.table, schemaColumns)
                }
                db.update(stmt.table, cursor, assignments)
                affected++
            }
        } finally {
            cursor.close()
        }
        return Result(emptyList(), emptyList(), affected)
    }

    /**
     * Execute DELETE FROM table [WHERE predicate].
     *
     * Uses `backend.openCursor()` for positioned delete.
     * Returns a Result with rowsAffected set.
     */
    private fun executeDeleteDirect(stmt: Statement.Delete): Result {
        // Validate the table exists and get column list.
        val colDefs = try { db.columns(stmt.table) } catch (ex: TableNotFound) {
            throw MiniSqliteException("OperationalError", "no such table: ${stmt.table}")
        }
        val schemaColumns = colDefs.map { it.name }

        val cursor = db.openCursor(stmt.table)
        var affected = 0
        try {
            while (true) {
                val row = cursor.next() ?: break
                val whereExpr = stmt.where
                if (whereExpr != null) {
                    val v = evalRowExpr(whereExpr, row, stmt.table, schemaColumns)
                    if (!isTruthy(v)) continue
                }
                db.delete(stmt.table, cursor)
                affected++
            }
        } finally {
            cursor.close()
        }
        return Result(emptyList(), emptyList(), affected)
    }

    // ── Direct aggregate evaluator ────────────────────────────────────────────
    //
    // Handles GROUP BY, HAVING, COUNT(*), SUM, AVG, MIN, MAX, COUNT(DISTINCT).
    //
    // Algorithm:
    //   1. Scan the FROM table, filtering by WHERE.
    //   2. Group rows by GROUP BY key expressions (if any).
    //   3. For each group, accumulate aggregate values.
    //   4. Apply HAVING filter on aggregated rows.
    //   5. Project output columns (which may mix group keys and aggregate exprs).
    //   6. Apply ORDER BY, LIMIT, OFFSET.
    //
    // For queries with no GROUP BY, all rows form a single group.
    // This handles conformance fixtures 08 (aggregates), 09 (GROUP BY), and
    // also 24 (HAVING with aggregates).

    // Accumulator state for one group.
    private inner class AggAccum {
        var countStar: Long = 0
        val counts     = mutableMapOf<String, Long>()
        // sums stores Long for pure-integer inputs, Double for float-mixed inputs.
        val sums       = mutableMapOf<String, Any?>()
        val sumNull    = mutableMapOf<String, Boolean>()  // true if all inputs were null
        val mins       = mutableMapOf<String, Any?>()
        val maxs       = mutableMapOf<String, Any?>()
        val distinctSets = mutableMapOf<String, LinkedHashSet<Any?>>()
        // Source rows in this group (needed for scalar exprs like the group key).
        val rows       = mutableListOf<Row>()
    }

    /**
     * A stable key for grouping that compares values by SQL equality semantics.
     * We use a List<Any?> as the key, relying on Kotlin's default equals/hashCode
     * which works correctly for Long, Double, String, Boolean, and null.
     */
    private fun makeGroupKey(row: Row, groupBy: List<SqlExpr>, tableName: String, schemaColumns: List<String>): List<Any?> =
        groupBy.map { expr -> evalRowExpr(expr, row, tableName, schemaColumns) }

    private fun executeAggregateDirect(select: Statement.Select): Result {
        if (select.from.isEmpty()) {
            throw MiniSqliteException("OperationalError", "aggregate SELECT requires FROM")
        }

        val tableRef = select.from[0]
        val tableName = tableRef.table
        val tableAlias = tableRef.alias ?: tableName

        val colDefs = try { db.columns(tableName) } catch (ex: TableNotFound) {
            throw MiniSqliteException("OperationalError", "no such table: $tableName")
        }
        val schemaColumns = colDefs.map { it.name }

        // 1. Scan + WHERE filter.
        val allRows = mutableListOf<Row>()
        val iter = db.scan(tableName)
        while (true) { allRows.add(iter.next() ?: break) }
        iter.close()

        val whereExpr = select.where
        val filtered = if (whereExpr != null) {
            allRows.filter { row ->
                isTruthy(evalRowExpr(whereExpr, row, tableAlias, schemaColumns))
            }
        } else allRows

        // 2. Group rows.
        val groupOrder = mutableListOf<List<Any?>>()
        val groups = linkedMapOf<List<Any?>, AggAccum>()
        for (row in filtered) {
            val key = makeGroupKey(row, select.groupBy, tableAlias, schemaColumns)
            val accum = groups.getOrPut(key) { groupOrder.add(key); AggAccum() }
            accum.rows.add(row)
        }

        // If no rows matched and no GROUP BY, still produce one aggregate row (SQL standard).
        if (groups.isEmpty() && select.groupBy.isEmpty()) {
            val key = emptyList<Any?>()
            groupOrder.add(key)
            groups[key] = AggAccum()
        }

        // 3. For each group, accumulate aggregates.
        for ((_, accum) in groups) {
            accum.countStar = accum.rows.size.toLong()
            // Walk all output columns and HAVING to find all AggExprs to accumulate.
            val aggExprs = mutableListOf<SqlExpr.AggExpr>()
            for (col in select.columns) {
                if (col is OutputColumn.Expr) collectAggExprs(col.expression, aggExprs)
            }
            val havingExpr = select.having
            if (havingExpr != null) collectAggExprs(havingExpr, aggExprs)
            for (key in select.orderBy) collectAggExprs(key.keyExpr, aggExprs)

            for (aggExpr in aggExprs) {
                val argKey = aggExprKey(aggExpr)
                when (aggExpr.func) {
                    AggFunction.COUNT -> {
                        // COUNT(*): arg is AggArg.Star — count all rows in group.
                        // COUNT(col): count non-null values.
                        // COUNT(DISTINCT col): count distinct non-null values.
                        if (aggExpr.arg is AggArg.Star) {
                            accum.counts[argKey] = accum.countStar
                        } else {
                            var c = 0L
                            val seen = if (aggExpr.distinct) LinkedHashSet<Any?>() else null
                            for (row in accum.rows) {
                                val v = evalAggArg(aggExpr.arg, row, tableAlias, schemaColumns)
                                if (v == null) continue
                                if (seen != null && !seen.add(v)) continue
                                c++
                            }
                            accum.counts[argKey] = c
                        }
                    }
                    AggFunction.SUM -> {
                        // Use Long arithmetic for pure-integer inputs to preserve type.
                        var longSum = 0L
                        var doubleSum = 0.0
                        var hasFloat = false
                        var hasNonNull = false
                        for (row in accum.rows) {
                            val v = evalAggArg(aggExpr.arg, row, tableAlias, schemaColumns) ?: continue
                            when (v) {
                                is Long   -> { longSum += v; doubleSum += v; hasNonNull = true }
                                is Int    -> { longSum += v; doubleSum += v; hasNonNull = true }
                                is Double -> { doubleSum += v; hasFloat = true; hasNonNull = true }
                                is Float  -> { doubleSum += v; hasFloat = true; hasNonNull = true }
                                else -> { val d = toDouble2(v) ?: continue; doubleSum += d; hasNonNull = true }
                            }
                        }
                        accum.sums[argKey] = if (!hasNonNull) null else if (hasFloat) doubleSum else longSum
                        accum.sumNull[argKey] = !hasNonNull
                    }
                    AggFunction.AVG -> {
                        var sum = 0.0
                        var count = 0L
                        for (row in accum.rows) {
                            val v = evalAggArg(aggExpr.arg, row, tableAlias, schemaColumns) ?: continue
                            sum += toDouble2(v) ?: continue
                            count++
                        }
                        accum.sums[argKey] = if (count > 0) sum / count else null
                        accum.sumNull[argKey] = count == 0L
                    }
                    AggFunction.MIN -> {
                        var minVal: Any? = null
                        for (row in accum.rows) {
                            val v = evalAggArg(aggExpr.arg, row, tableAlias, schemaColumns) ?: continue
                            minVal = if (minVal == null) v else if (compareAny(v, minVal!!) < 0) v else minVal
                        }
                        accum.mins[argKey] = minVal
                    }
                    AggFunction.MAX -> {
                        var maxVal: Any? = null
                        for (row in accum.rows) {
                            val v = evalAggArg(aggExpr.arg, row, tableAlias, schemaColumns) ?: continue
                            maxVal = if (maxVal == null) v else if (compareAny(v, maxVal!!) > 0) v else maxVal
                        }
                        accum.maxs[argKey] = maxVal
                    }
                }
            }
        }

        // Helper to evaluate an expression in the aggregate context (group key or aggregate result).
        fun evalInGroup(expr: SqlExpr, accum: AggAccum, groupKey: List<Any?>): Any? = when {
            expr is SqlExpr.AggExpr -> {
                val argKey = aggExprKey(expr)
                when (expr.func) {
                    AggFunction.COUNT      -> accum.counts[argKey] ?: 0L
                    AggFunction.SUM        -> if (accum.sumNull[argKey] == true) null else accum.sums[argKey]
                    AggFunction.AVG        -> if (accum.sumNull[argKey] == true) null else accum.sums[argKey]
                    AggFunction.MIN        -> accum.mins[argKey]
                    AggFunction.MAX        -> accum.maxs[argKey]
                }
            }
            expr is SqlExpr.Column -> {
                // Look up in group key (from GROUP BY) or in first row of group.
                val colName = expr.column.lowercase()
                val gbIdx = select.groupBy.indexOfFirst { gbExpr ->
                    gbExpr is SqlExpr.Column && gbExpr.column.lowercase() == colName
                }
                if (gbIdx >= 0) {
                    groupKey.getOrNull(gbIdx)
                } else {
                    val firstRow = accum.rows.firstOrNull()
                    firstRow?.entries?.find { it.key.equals(colName, ignoreCase = true) }?.value
                }
            }
            expr is SqlExpr.Literal -> expr.value
            expr is SqlExpr.BinaryOp -> {
                val l = evalInGroup(expr.left, accum, groupKey)
                val r = evalInGroup(expr.right, accum, groupKey)
                evalBinaryLiteral(expr.op, l, r)
            }
            expr is SqlExpr.FuncCall -> {
                val args = expr.args.map { evalInGroup(it, accum, groupKey) }
                evalFunctionLiteral(expr.name, args)
            }
            expr is SqlExpr.IsNull    -> evalInGroup(expr.operand, accum, groupKey) == null
            expr is SqlExpr.IsNotNull -> evalInGroup(expr.operand, accum, groupKey) != null
            else -> null
        }

        // 4. Apply HAVING.
        val filteredGroups = groupOrder.filter { key ->
            val accum = groups[key] ?: return@filter false
            val havingExpr = select.having
            if (havingExpr == null) true
            else isTruthy(evalInGroup(havingExpr, accum, key))
        }

        // 5. Project output columns.
        val outColumns = mutableListOf<String>()
        for (col in select.columns) {
            when (col) {
                is OutputColumn.Star -> outColumns.addAll(schemaColumns)
                is OutputColumn.Expr -> outColumns.add(col.alias ?: exprName(col.expression))
            }
        }

        var outRows = filteredGroups.map { key ->
            val accum = groups[key]!!
            val row = mutableListOf<Any?>()
            for (col in select.columns) {
                when (col) {
                    is OutputColumn.Star -> {
                        val firstRow = accum.rows.firstOrNull() ?: Row()
                        schemaColumns.forEach { c -> row.add(firstRow[c]) }
                    }
                    is OutputColumn.Expr -> row.add(evalInGroup(col.expression, accum, key))
                }
            }
            row as List<Any?>
        }

        // 6. Apply ORDER BY.
        if (select.orderBy.isNotEmpty()) {
            val groupsWithAccum = filteredGroups.zip(outRows)
            val sorted = groupsWithAccum.sortedWith(Comparator { (keyA, _), (keyB, _) ->
                val accumA = groups[keyA]!!
                val accumB = groups[keyB]!!
                for (sk in select.orderBy) {
                    val a = evalInGroup(sk.keyExpr, accumA, keyA)
                    val b = evalInGroup(sk.keyExpr, accumB, keyB)
                    val cmp = sortCompare(a, b, sk.direction, sk.nullOrder)
                    if (cmp != 0) return@Comparator cmp
                }
                0
            })
            outRows = sorted.map { (_, row) -> row }
        }

        // 7. Apply DISTINCT (rare with aggregates, but supported).
        if (select.distinct) {
            val seen = linkedSetOf<List<Any?>>()
            outRows = outRows.filter { seen.add(it) }
        }

        // 8. Apply LIMIT / OFFSET.
        val limit = select.limit
        if (limit != null) {
            val start = (limit.offset ?: 0L).toInt().coerceAtLeast(0)
            val count = limit.count?.toInt()
            outRows = if (count == null) outRows.drop(start)
                      else outRows.drop(start).take(count)
        }

        return Result(outColumns, outRows, -1)
    }

    /** Collect all AggExpr nodes from an expression tree. */
    private fun collectAggExprs(expr: SqlExpr, out: MutableList<SqlExpr.AggExpr>) {
        when (expr) {
            is SqlExpr.AggExpr   -> out.add(expr)
            is SqlExpr.BinaryOp  -> { collectAggExprs(expr.left, out); collectAggExprs(expr.right, out) }
            is SqlExpr.UnaryOp   -> collectAggExprs(expr.operand, out)
            is SqlExpr.FuncCall  -> expr.args.forEach { collectAggExprs(it, out) }
            is SqlExpr.IsNull    -> collectAggExprs(expr.operand, out)
            is SqlExpr.IsNotNull -> collectAggExprs(expr.operand, out)
            is SqlExpr.Between   -> { collectAggExprs(expr.value, out); collectAggExprs(expr.low, out); collectAggExprs(expr.high, out) }
            is SqlExpr.In        -> { collectAggExprs(expr.value, out); expr.items.forEach { collectAggExprs(it, out) } }
            is SqlExpr.NotIn     -> { collectAggExprs(expr.value, out); expr.items.forEach { collectAggExprs(it, out) } }
            else -> { /* leaf: no agg */ }
        }
    }

    /** A stable string key for an AggExpr, used to look up accumulated results. */
    private fun aggExprKey(expr: SqlExpr.AggExpr): String {
        val base = when (val arg = expr.arg) {
            is AggArg.Star   -> "*"
            is AggArg.Expr   -> when (val e = arg.expression) {
                is SqlExpr.Column -> e.column
                else -> e.toString()
            }
        }
        return "${expr.func}:$base:${expr.distinct}"
    }

    /** Evaluate an AggArg expression against a row. */
    private fun evalAggArg(arg: AggArg, row: Row, tableAlias: String, schemaColumns: List<String>): Any? = when (arg) {
        is AggArg.Star   -> 1L  // COUNT(*): non-null sentinel
        is AggArg.Expr   -> evalRowExpr(arg.expression, row, tableAlias, schemaColumns)
    }

    private fun toDouble2(v: Any?): Double? = when (v) {
        null      -> null
        is Long   -> v.toDouble()
        is Int    -> v.toDouble()
        is Double -> v
        is Float  -> v.toDouble()
        is String -> v.toDoubleOrNull()
        else -> null
    }

    /**
     * Apply ORDER BY to a list of output rows.
     *
     * We sort by evaluating the sort key expressions against the original
     * (pre-projection) rows so column references resolve correctly.
     */
    private fun applyOrderBy(
        outRows: List<List<Any?>>,
        outColumns: List<String>,
        orderBy: List<SortKey>,
        sourceRows: List<Row>,
        tableAlias: String,
        schemaColumns: List<String>,
        select: Statement.Select,
    ): List<List<Any?>> {
        // Pair each output row with its source row for sort key evaluation.
        val paired = outRows.zip(sourceRows)
        val sorted = paired.sortedWith(Comparator { (_, rowA), (_, rowB) ->
            for (key in orderBy) {
                val a = evalRowExpr(key.keyExpr, rowA, tableAlias, schemaColumns)
                val b = evalRowExpr(key.keyExpr, rowB, tableAlias, schemaColumns)
                val cmp = sortCompare(a, b, key.direction, key.nullOrder)
                if (cmp != 0) return@Comparator cmp
            }
            0
        })
        return sorted.map { (row, _) -> row }
    }

    private fun sortCompare(a: Any?, b: Any?, dir: SortDir, nullOrder: NullOrder): Int {
        val aNull = a == null
        val bNull = b == null
        if (aNull && bNull) return 0
        val nullsFirst = nullOrder == NullOrder.NULLS_FIRST
        if (aNull) return if (nullsFirst) -1 else 1
        if (bNull) return if (nullsFirst) 1 else -1
        val cmp = compareAny(a!!, b!!)
        return if (dir == SortDir.DESC) -cmp else cmp
    }

    // ── Row expression evaluator ──────────────────────────────────────────────
    //
    // Evaluates a SqlExpr against a single backend Row.  Handles all expression
    // types needed by Level 1: literals, column refs, binary ops, unary ops,
    // function calls, IS NULL, IS NOT NULL, BETWEEN, IN, NOT IN, LIKE, NOT LIKE.

    private fun evalRowExpr(expr: SqlExpr, row: Row, tableAlias: String, schemaColumns: List<String>): Any? {
        return when (expr) {
            is SqlExpr.Literal -> expr.value

            is SqlExpr.Column -> {
                // Look up the column in the row (case-insensitive).
                val col = expr.column
                row.entries.find { it.key.equals(col, ignoreCase = true) }?.value
            }

            is SqlExpr.Wildcard -> null

            is SqlExpr.UnaryOp -> {
                val v = evalRowExpr(expr.operand, row, tableAlias, schemaColumns)
                when (expr.op) {
                    UnaryOperator.NEG -> negateValue(v)
                    UnaryOperator.NOT -> notValue(v)
                }
            }

            is SqlExpr.BinaryOp -> {
                val l = evalRowExpr(expr.left, row, tableAlias, schemaColumns)
                val r = evalRowExpr(expr.right, row, tableAlias, schemaColumns)
                evalBinaryLiteral(expr.op, l, r)
            }

            is SqlExpr.FuncCall -> {
                val args = expr.args.map { evalRowExpr(it, row, tableAlias, schemaColumns) }
                evalFunctionLiteral(expr.name, args)
            }

            is SqlExpr.IsNull    -> evalRowExpr(expr.operand, row, tableAlias, schemaColumns) == null
            is SqlExpr.IsNotNull -> evalRowExpr(expr.operand, row, tableAlias, schemaColumns) != null

            is SqlExpr.Between -> {
                val v   = evalRowExpr(expr.value, row, tableAlias, schemaColumns)
                val lo  = evalRowExpr(expr.low,   row, tableAlias, schemaColumns)
                val hi  = evalRowExpr(expr.high,  row, tableAlias, schemaColumns)
                if (v == null || lo == null || hi == null) null
                else compareAny(v, lo) >= 0 && compareAny(v, hi) <= 0
            }

            is SqlExpr.In -> {
                val v = evalRowExpr(expr.value, row, tableAlias, schemaColumns)
                if (v == null) { null } else {
                    var foundNull = false
                    var found = false
                    for (item in expr.items) {
                        val iv = evalRowExpr(item, row, tableAlias, schemaColumns)
                        if (iv == null) foundNull = true
                        else if (compareAny(v, iv) == 0) { found = true; break }
                    }
                    if (found) true else if (foundNull) null else false
                }
            }

            is SqlExpr.NotIn -> {
                val v = evalRowExpr(expr.value, row, tableAlias, schemaColumns)
                if (v == null) { null } else {
                    var foundNull = false
                    for (item in expr.items) {
                        val iv = evalRowExpr(item, row, tableAlias, schemaColumns)
                        if (iv == null) foundNull = true
                        else if (compareAny(v, iv) == 0) return@evalRowExpr false
                    }
                    if (foundNull) null else true
                }
            }

            is SqlExpr.Like -> {
                val v = evalRowExpr(expr.value, row, tableAlias, schemaColumns) ?: return null
                likeMatch(v.toString(), expr.pattern)
            }

            is SqlExpr.NotLike -> {
                val v = evalRowExpr(expr.value, row, tableAlias, schemaColumns) ?: return null
                !likeMatch(v.toString(), expr.pattern)
            }

            is SqlExpr.AggExpr -> null  // aggregates not supported in direct evaluator
        }
    }

    // ── SELECT without FROM ────────────────────────────────────────────────────
    //
    // For `SELECT expr1, expr2 AS alias` with no FROM clause.  Evaluates each
    // output expression as a literal and returns one synthetic row.

    private fun executeSelectWithoutFrom(sql: String): Result {
        val stmt: Statement = try {
            MiniSqliteParser.parse(sql)
        } catch (ex: ParseException) {
            throw MiniSqliteException("OperationalError", ex.message ?: "parse error")
        }
        val select = stmt as? Statement.Select
            ?: throw MiniSqliteException("OperationalError", "expected SELECT statement")

        val columns = mutableListOf<String>()
        val values = mutableListOf<Any?>()

        for (col in select.columns) {
            if (col is OutputColumn.Expr) {
                columns.add(col.alias ?: exprName(col.expression))
                values.add(evalLiteralExpr(col.expression))
            }
        }
        return Result(columns, listOf(values), -1)
    }

    // ── Shared expression utilities ───────────────────────────────────────────

    /**
     * Evaluate a scalar expression that does not reference any table columns.
     * Used for SELECT without FROM.
     */
    private fun evalLiteralExpr(expr: SqlExpr): Any? {
        // Delegate to the row evaluator with an empty row.
        return evalRowExpr(expr, Row(), "", emptyList())
    }

    /** Evaluate a binary operation on two Kotlin Any? values. */
    private fun evalBinaryLiteral(op: BinaryOperator, l: Any?, r: Any?): Any? {
        if (op == BinaryOperator.AND) {
            val lb = toBool(l); val rb = toBool(r)
            return if (lb == false || rb == false) false
                   else if (lb == null || rb == null) null
                   else true
        }
        if (op == BinaryOperator.OR) {
            val lb = toBool(l); val rb = toBool(r)
            return if (lb == true || rb == true) true
                   else if (lb == null || rb == null) null
                   else false
        }
        if (l == null || r == null) return null
        return when (op) {
            BinaryOperator.ADD -> numericOp(l, r, Long::plus, Double::plus)
            BinaryOperator.SUB -> numericOp(l, r, Long::minus, Double::minus)
            BinaryOperator.MUL -> numericOp(l, r, Long::times, Double::times)
            BinaryOperator.DIV -> {
                val ld = toDouble(l) ?: return null
                val rd = toDouble(r) ?: return null
                if (rd == 0.0) null else {
                    if (l is Long && r is Long) (ld / rd).toLong() else ld / rd
                }
            }
            BinaryOperator.MOD -> {
                val ll = toLong(l) ?: return null
                val rl = toLong(r) ?: return null
                if (rl == 0L) null else ll % rl
            }
            BinaryOperator.EQ     -> compareAny(l, r) == 0
            BinaryOperator.NOT_EQ -> compareAny(l, r) != 0
            BinaryOperator.LT     -> compareAny(l, r) < 0
            BinaryOperator.LTE    -> compareAny(l, r) <= 0
            BinaryOperator.GT     -> compareAny(l, r) > 0
            BinaryOperator.GTE    -> compareAny(l, r) >= 0
            else -> null
        }
    }

    /** Evaluate a built-in SQL function against literal or row arguments. */
    private fun evalFunctionLiteral(name: String, args: List<Any?>): Any? {
        return when (name.uppercase()) {
            // String concatenation (represented as FuncCall("||") by the parser).
            "||" -> {
                if (args.size < 2) return null
                val sb = StringBuilder()
                for (a in args) {
                    sb.append(a?.toString() ?: return null)
                }
                sb.toString()
            }

            "LENGTH" -> args.getOrNull(0)?.toString()?.length?.toLong()

            "UPPER" -> args.getOrNull(0)?.toString()?.uppercase()
            "LOWER" -> args.getOrNull(0)?.toString()?.lowercase()
            "TRIM"  -> args.getOrNull(0)?.toString()?.trim()
            "LTRIM" -> args.getOrNull(0)?.toString()?.trimStart()
            "RTRIM" -> args.getOrNull(0)?.toString()?.trimEnd()

            "SUBSTR", "SUBSTRING" -> {
                val str   = args.getOrNull(0)?.toString() ?: return null
                val start = toLong(args.getOrNull(1)) ?: return null
                val len   = toLong(args.getOrNull(2))
                // SQL SUBSTR is 1-based; start=1 means first character.
                val s0 = (start.toInt() - 1).coerceAtLeast(0).coerceAtMost(str.length)
                if (len == null) str.substring(s0)
                else str.substring(s0, (s0 + len.toInt()).coerceAtMost(str.length))
            }

            "REPLACE" -> {
                val str  = args.getOrNull(0)?.toString() ?: return null
                val from = args.getOrNull(1)?.toString() ?: return null
                val to   = args.getOrNull(2)?.toString() ?: return null
                str.replace(from, to)
            }

            "ABS" -> {
                val v = args.getOrNull(0) ?: return null
                when (v) {
                    is Long   -> if (v < 0) -v else v
                    is Double -> kotlin.math.abs(v)
                    is Int    -> kotlin.math.abs(v).toLong()
                    else -> null
                }
            }

            "ROUND" -> {
                val v = toDouble(args.getOrNull(0)) ?: return null
                val decimals = toLong(args.getOrNull(1))?.toInt() ?: 0
                val factor = Math.pow(10.0, decimals.toDouble())
                val rounded = kotlin.math.round(v * factor).toDouble() / factor
                if (decimals == 0) rounded.toLong() else rounded
            }

            "COALESCE" -> args.firstOrNull { it != null }

            // Unknown function — return NULL.
            else -> null
        }
    }

    // ── Predicate helpers ─────────────────────────────────────────────────────

    /**
     * SQL LIKE pattern matching.
     *
     * Wildcards:
     *   %  — matches any sequence of zero or more characters
     *   _  — matches any single character
     *
     * Uses an iterative O(n*m) algorithm to avoid ReDoS vulnerabilities.
     */
    private fun likeMatch(text: String, pattern: String): Boolean {
        var ti = 0; var pi = 0; var starPi = -1; var starTi = -1
        while (ti < text.length) {
            val pc = pattern.getOrNull(pi)
            when {
                pc == '%' -> { starPi = pi++; starTi = ti }
                pc == '_' || (pc != null && pc.equals(text[ti], ignoreCase = true)) -> { ti++; pi++ }
                starPi >= 0 -> { pi = starPi + 1; ti = ++starTi }
                else -> return false
            }
        }
        while (pi < pattern.length && pattern[pi] == '%') pi++
        return pi == pattern.length
    }

    private fun isTruthy(v: Any?): Boolean = when (v) {
        null -> false
        is Boolean -> v
        is Long    -> v != 0L
        is Double  -> v != 0.0
        is String  -> v.isNotEmpty()
        is Int     -> v != 0
        else -> true
    }

    // ── Arithmetic helpers ────────────────────────────────────────────────────

    private fun negateValue(v: Any?): Any? = when (v) {
        null -> null
        is Long   -> -v
        is Double -> -v
        is Int    -> -v.toLong()
        else -> null
    }

    private fun notValue(v: Any?): Any? = when (v) {
        null      -> null
        is Boolean -> !v
        is Long    -> v == 0L
        is Double  -> v == 0.0
        else -> false
    }

    private fun numericOp(l: Any, r: Any, intOp: (Long, Long) -> Long, floatOp: (Double, Double) -> Double): Any? {
        val ld = toDouble(l) ?: return null
        val rd = toDouble(r) ?: return null
        return if (l is Long && r is Long) intOp(l, r) else floatOp(ld, rd)
    }

    private fun toDouble(v: Any?): Double? = when (v) {
        null -> null
        is Long   -> v.toDouble()
        is Int    -> v.toDouble()
        is Double -> v
        is Float  -> v.toDouble()
        is String -> v.toDoubleOrNull()
        else -> null
    }

    private fun toLong(v: Any?): Long? = when (v) {
        null -> null
        is Long   -> v
        is Int    -> v.toLong()
        is Double -> v.toLong()
        is String -> v.toLongOrNull()
        else -> null
    }

    private fun toBool(v: Any?): Boolean? = when (v) {
        null      -> null
        is Boolean -> v
        is Long    -> v != 0L
        is Double  -> v != 0.0
        is String  -> v.isNotEmpty()
        else -> null
    }

    private fun compareAny(l: Any, r: Any): Int {
        if (l is Number && r is Number) return l.toDouble().compareTo(r.toDouble())
        if (l is String && r is String) return l.compareTo(r)
        return l.toString().compareTo(r.toString())
    }

    /** Derive a display name for an expression (used when no alias is provided). */
    private fun exprName(expr: SqlExpr): String = when (expr) {
        is SqlExpr.Column   -> expr.column
        is SqlExpr.FuncCall -> expr.name
        else -> "col"
    }

    // ── Expression type inspection helpers ────────────────────────────────────

    /** True if [expr] contains at least one AggExpr node anywhere in the tree. */
    private fun containsAgg(expr: SqlExpr): Boolean = when (expr) {
        is SqlExpr.AggExpr   -> true
        is SqlExpr.BinaryOp  -> containsAgg(expr.left) || containsAgg(expr.right)
        is SqlExpr.UnaryOp   -> containsAgg(expr.operand)
        is SqlExpr.FuncCall  -> expr.args.any { containsAgg(it) }
        is SqlExpr.IsNull    -> containsAgg(expr.operand)
        is SqlExpr.IsNotNull -> containsAgg(expr.operand)
        is SqlExpr.Between   -> containsAgg(expr.value) || containsAgg(expr.low) || containsAgg(expr.high)
        is SqlExpr.In        -> containsAgg(expr.value) || expr.items.any { containsAgg(it) }
        is SqlExpr.NotIn     -> containsAgg(expr.value) || expr.items.any { containsAgg(it) }
        else -> false
    }

    // ── SchemaProvider adapter ─────────────────────────────────────────────────
    //
    // The sql-planner defines its own SchemaProvider interface which differs from
    // the one in sql-backend.  We create an anonymous object that wraps the
    // InMemoryBackend and adapts it to the planner's interface.

    private fun backendSchemaProvider(backend: InMemoryBackend): com.codingadventures.sqlplanner.SchemaProvider =
        object : com.codingadventures.sqlplanner.SchemaProvider {
            override fun columns(table: String): List<String> {
                return try {
                    backend.columns(table).map { it.name }
                } catch (ex: TableNotFound) {
                    throw UnknownTableException(table)
                }
            }
        }
}

// ── Cursor ────────────────────────────────────────────────────────────────────

class Cursor internal constructor(private val connection: Connection) : AutoCloseable {
    var description: List<Column> = emptyList()
        private set
    var rowcount: Int = -1
        private set
    @Suppress("unused")
    var lastrowid: Any? = null
        private set
    var arraysize: Int = 1
    private var rows: List<List<Any?>> = emptyList()
    private var offset = 0
    private var closed = false

    fun execute(sql: String, params: List<Any?> = emptyList()): Cursor {
        if (closed) throw MiniSqliteException("ProgrammingError", "cursor is closed")
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
            if (rowcount > 0) total += rowcount
        }
        if (paramsSeq.isNotEmpty()) rowcount = total
        return this
    }

    fun fetchone(): List<Any?>? {
        if (closed || offset >= rows.size) return null
        return rows[offset++]
    }

    fun fetchmany(size: Int = arraysize): List<List<Any?>> {
        if (closed) return emptyList()
        val out = mutableListOf<List<Any?>>()
        repeat(size) { val row = fetchone() ?: return@repeat; out += row }
        return out
    }

    fun fetchall(): List<List<Any?>> {
        if (closed) return emptyList()
        val out = mutableListOf<List<Any?>>()
        while (true) out += fetchone() ?: break
        return out
    }

    override fun close() {
        closed = true
        rows = emptyList()
        description = emptyList()
    }
}

// ── Internal Result type ──────────────────────────────────────────────────────

internal data class Result(
    val columns: List<String>,
    val rows: List<List<Any?>>,
    val rowsAffected: Int,
) {
    companion object {
        fun empty(rowsAffected: Int) = Result(emptyList(), emptyList(), rowsAffected)
    }
}

/** Convert a SqlVm QueryResult into our internal Result (SqlValue → Any?). */
private fun queryResultToResult(qr: com.codingadventures.sqlvm.QueryResult): Result {
    val anyRows = qr.rows.map { row -> row.map { v -> sqlValueToAny(v) } }
    return Result(qr.columns, anyRows, qr.rowsAffected)
}

/**
 * Convert a single [SqlValue] to the corresponding Kotlin/JVM type.
 *
 * Mapping:
 *   SqlValue.Null     → null
 *   SqlValue.IntVal   → Long
 *   SqlValue.FloatVal → Double
 *   SqlValue.TextVal  → String
 *   SqlValue.BoolVal  → Boolean
 */
internal fun sqlValueToAny(v: SqlValue): Any? = when (v) {
    is SqlValue.Null     -> null
    is SqlValue.IntVal   -> v.v
    is SqlValue.FloatVal -> v.v
    is SqlValue.TextVal  -> v.v
    is SqlValue.BoolVal  -> v.v
}

// ── Parameter binding ──────────────────────────────────────────────────────────
//
// Substitutes `?` placeholders in [sql] with the corresponding literal values.

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
                while (next < sql.length && sql[next] != '\n') next++
                out.append(sql.substring(i, next))
                i = next
            }
            ch == '/' && i + 1 < sql.length && sql[i + 1] == '*' -> {
                var next = i + 2
                while (next + 1 < sql.length && sql.substring(next, next + 2) != "*/") next++
                next = minOf(next + 2, sql.length)
                out.append(sql.substring(i, next))
                i = next
            }
            ch == '?' -> {
                if (index >= params.size) {
                    throw MiniSqliteException("ProgrammingError", "not enough parameters for SQL statement")
                }
                out.append(toSqlLiteral(params[index]))
                index++
                i++
            }
            else -> { out.append(ch); i++ }
        }
    }
    if (index < params.size) {
        throw MiniSqliteException("ProgrammingError", "too many parameters for SQL statement")
    }
    return out.toString()
}

private fun readQuoted(sql: String, index: Int, quote: Char): Int {
    var i = index + 1
    while (i < sql.length) {
        val ch = sql[i]
        if (ch == quote) {
            if (i + 1 < sql.length && sql[i + 1] == quote) { i += 2 } else { return i + 1 }
        } else { i++ }
    }
    return sql.length
}

/**
 * Convert a parameter value to a SQL literal string.
 *
 * Security note: we accept only concrete numeric types whose toString() is
 * guaranteed to produce a numeric literal (digits, sign, decimal point, 'E').
 * We intentionally do NOT accept `is Number` as a catch-all because a caller
 * could provide a custom Number subclass with a malicious toString() that
 * injects arbitrary SQL text into the bound statement.  The allowlist below
 * restricts binding to types whose string representations are safe.
 */
private fun toSqlLiteral(value: Any?): String = when (value) {
    null                 -> "NULL"
    is Boolean           -> if (value) "TRUE" else "FALSE"
    // Explicit allowlist of numeric types whose toString() is safe.
    is Int               -> value.toString()
    is Long              -> value.toString()
    is Float             -> value.toString()
    is Double            -> value.toString()
    is java.math.BigDecimal -> value.toPlainString()  // avoids scientific notation
    is java.math.BigInteger -> value.toString()
    is CharSequence      -> "'" + value.toString().replace("'", "''") + "'"
    else -> throw MiniSqliteException("ProgrammingError", "unsupported parameter type: ${value::class.qualifiedName}")
}

private fun firstKeyword(sql: String): String {
    // Use isWhitespace() (Unicode-aware) rather than trim() (ASCII-only) so that
    // SQL strings with leading Unicode whitespace (e.g. U+00A0 non-breaking space)
    // don't cause a spurious "unsupported statement" error.
    val value = sql.trimStart { it.isWhitespace() }
    var end = 0
    while (end < value.length && (value[end].isLetter() || value[end] == '_')) end++
    return value.substring(0, end).uppercase()
}

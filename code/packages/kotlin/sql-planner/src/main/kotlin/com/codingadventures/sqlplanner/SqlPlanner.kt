package com.codingadventures.sqlplanner

// SqlPlanner.kt — logical query plan builder for SQL statements.
//
// Transforms a Statement into a LogicalPlan tree using an 8-step bottom-up
// SELECT pipeline:
//
//   Scan → Filter → Aggregate → Having → Project → Distinct → Sort → Limit
//
// No I/O, no database connections — pure in-memory data transformation.
// Errors are reported as PlanException subclasses (consistent with the
// Kotlin sql-backend's exception-based error style).
//
// Usage:
//   val schema = InMemorySchemaProvider(mapOf("users" to listOf("id", "name", "age")))
//   val planner = SqlPlanner(schema)
//   val plan: LogicalPlan = planner.plan(stmt)   // throws PlanException on error

// ── Enumerations ──────────────────────────────────────────────────────────────

enum class BinaryOperator { EQ, NOT_EQ, LT, LTE, GT, GTE, AND, OR, ADD, SUB, MUL, DIV, MOD }
enum class UnaryOperator  { NOT, NEG }
enum class AggFunction    { COUNT, SUM, AVG, MIN, MAX }
enum class SortDir        { ASC, DESC }
enum class NullOrder      { NULLS_FIRST, NULLS_LAST }
enum class JoinKind       { INNER, LEFT, RIGHT, FULL, CROSS }

// ── Aggregate argument ────────────────────────────────────────────────────────

sealed class AggArg {
    object Star : AggArg()
    data class Expr(val expression: SqlExpr) : AggArg()
}

// ── Scalar expressions ────────────────────────────────────────────────────────

sealed class SqlExpr {
    data class Literal(val value: Any?) : SqlExpr()
    data class Column(val table: String?, val column: String) : SqlExpr()
    data class BinaryOp(val op: BinaryOperator, val left: SqlExpr, val right: SqlExpr) : SqlExpr()
    data class UnaryOp(val op: UnaryOperator, val operand: SqlExpr) : SqlExpr()
    data class FuncCall(val name: String, val args: List<SqlExpr>) : SqlExpr()
    data class IsNull(val operand: SqlExpr) : SqlExpr()
    data class IsNotNull(val operand: SqlExpr) : SqlExpr()
    data class Between(val value: SqlExpr, val low: SqlExpr, val high: SqlExpr) : SqlExpr()
    data class In(val value: SqlExpr, val items: List<SqlExpr>) : SqlExpr()
    data class NotIn(val value: SqlExpr, val items: List<SqlExpr>) : SqlExpr()
    data class Like(val value: SqlExpr, val pattern: String) : SqlExpr()
    data class NotLike(val value: SqlExpr, val pattern: String) : SqlExpr()
    object Wildcard : SqlExpr()
    data class AggExpr(val func: AggFunction, val arg: AggArg, val distinct: Boolean) : SqlExpr()
}

// ── Output column ─────────────────────────────────────────────────────────────

sealed class OutputColumn {
    object Star : OutputColumn()
    data class Expr(val expression: SqlExpr, val alias: String?) : OutputColumn()
}

// ── Structural types ──────────────────────────────────────────────────────────

data class JoinClause(val kind: JoinKind, val table: String, val alias: String?, val on: SqlExpr?)
data class ColumnDef(val name: String, val typeName: String, val notNull: Boolean, val primaryKey: Boolean, val unique: Boolean, val default: SqlExpr?)
data class Assignment(val column: String, val value: SqlExpr)
data class LimitClause(val count: Long?, val offset: Long?)
data class SortKey(val keyExpr: SqlExpr, val direction: SortDir, val nullOrder: NullOrder)
data class AggregateItem(val func: AggFunction, val arg: AggArg, val alias: String, val distinct: Boolean)
data class TableRef(val table: String, val alias: String?)

// ── Statement AST ─────────────────────────────────────────────────────────────

sealed class Statement {
    data class Select(
        val distinct: Boolean,
        val columns: List<OutputColumn>,
        val from: List<TableRef>,
        val joins: List<JoinClause>,
        val where: SqlExpr?,
        val groupBy: List<SqlExpr>,
        val having: SqlExpr?,
        val orderBy: List<SortKey>,
        val limit: LimitClause?
    ) : Statement()

    data class Insert(
        val table: String,
        val columns: List<String>,
        val values: List<List<SqlExpr>>
    ) : Statement()

    data class Update(
        val table: String,
        val assignments: List<Assignment>,
        val where: SqlExpr?
    ) : Statement()

    data class Delete(val table: String, val where: SqlExpr?) : Statement()

    data class CreateTable(
        val table: String,
        val ifNotExists: Boolean,
        val columns: List<ColumnDef>
    ) : Statement()

    data class DropTable(val table: String, val ifExists: Boolean) : Statement()
}

// ── Logical plan nodes ────────────────────────────────────────────────────────

sealed class LogicalPlan {
    data class Scan(val table: String, val alias: String?) : LogicalPlan()
    data class Filter(val input: LogicalPlan, val predicate: SqlExpr) : LogicalPlan()
    data class Project(val input: LogicalPlan, val columns: List<OutputColumn>) : LogicalPlan()
    data class Join(val left: LogicalPlan, val right: LogicalPlan, val kind: JoinKind, val condition: SqlExpr?) : LogicalPlan()
    data class Aggregate(val input: LogicalPlan, val groupBy: List<SqlExpr>, val aggregates: List<AggregateItem>) : LogicalPlan()
    data class Having(val input: LogicalPlan, val predicate: SqlExpr) : LogicalPlan()
    data class Sort(val input: LogicalPlan, val keys: List<SortKey>) : LogicalPlan()
    data class Limit(val input: LogicalPlan, val count: Long?, val offset: Long?) : LogicalPlan()
    data class Distinct(val input: LogicalPlan) : LogicalPlan()
    data class Union(val left: LogicalPlan, val right: LogicalPlan, val all: Boolean) : LogicalPlan()
    data class Insert(val table: String, val columns: List<String>, val values: List<List<SqlExpr>>) : LogicalPlan()
    data class Update(val table: String, val assignments: List<Assignment>, val predicate: SqlExpr?) : LogicalPlan()
    data class Delete(val table: String, val predicate: SqlExpr?) : LogicalPlan()
    data class CreateTable(val table: String, val ifNotExists: Boolean, val columns: List<ColumnDef>) : LogicalPlan()
    data class DropTable(val table: String, val ifExists: Boolean) : LogicalPlan()
}

// ── Plan exceptions ───────────────────────────────────────────────────────────

abstract class PlanException(message: String) : RuntimeException(message)

class AmbiguousColumnException(val column: String, val tables: List<String>) :
    PlanException("Ambiguous column '$column' — found in: ${tables.joinToString(", ")}")

class UnknownTableException(val table: String) :
    PlanException("Unknown table '$table'")

class UnknownColumnException(val qualifyingTable: String?, val column: String) :
    PlanException(if (qualifyingTable != null) "Unknown column '$qualifyingTable.$column'" else "Unknown column '$column'")

class InvalidAggregateException(message: String) : PlanException(message)

class UnsupportedStatementException(kind: String) : PlanException("Unsupported statement: $kind")

// ── Schema provider ───────────────────────────────────────────────────────────

interface SchemaProvider {
    /** Returns column names for [table], or throws [UnknownTableException]. */
    fun columns(table: String): List<String>
}

class InMemorySchemaProvider(private val tables: Map<String, List<String>>) : SchemaProvider {
    override fun columns(table: String): List<String> =
        tables[table] ?: throw UnknownTableException(table)
}

// ── Planner ───────────────────────────────────────────────────────────────────

class SqlPlanner(private val schema: SchemaProvider) {

    // Scope entry: one source in FROM/JOIN with its resolved alias and columns.
    private data class ScopeEntry(val alias: String, val table: String, val cols: List<String>)

    private fun buildScope(from: List<TableRef>, joins: List<JoinClause>): List<ScopeEntry> {
        val scope = mutableListOf<ScopeEntry>()
        for (ref in from) {
            val cols = schema.columns(ref.table)    // throws UnknownTableException
            scope.add(ScopeEntry(ref.alias ?: ref.table, ref.table, cols))
        }
        for (j in joins) {
            val cols = schema.columns(j.table)      // throws UnknownTableException
            scope.add(ScopeEntry(j.alias ?: j.table, j.table, cols))
        }
        return scope
    }

    private fun resolveColumn(scope: List<ScopeEntry>, tableOpt: String?, col: String): SqlExpr {
        if (tableOpt != null) {
            val entry = scope.find { it.alias == tableOpt }
                ?: throw UnknownTableException(tableOpt)
            if (!entry.cols.any { it.equals(col, ignoreCase = true) })
                throw UnknownColumnException(tableOpt, col)
            return SqlExpr.Column(entry.alias, col)
        } else {
            val matches = scope.filter { e -> e.cols.any { it.equals(col, ignoreCase = true) } }
            if (matches.isEmpty()) throw UnknownColumnException(null, col)
            if (matches.size > 1)  throw AmbiguousColumnException(col, matches.map { it.alias })
            return SqlExpr.Column(matches[0].alias, col)
        }
    }

    private fun resolveExpr(scope: List<ScopeEntry>, expr: SqlExpr): SqlExpr = when (expr) {
        is SqlExpr.Column    -> resolveColumn(scope, expr.table, expr.column)
        is SqlExpr.Literal   -> expr
        is SqlExpr.Wildcard  -> expr
        is SqlExpr.AggExpr   -> expr
        is SqlExpr.BinaryOp  -> SqlExpr.BinaryOp(expr.op, resolveExpr(scope, expr.left), resolveExpr(scope, expr.right))
        is SqlExpr.UnaryOp   -> SqlExpr.UnaryOp(expr.op, resolveExpr(scope, expr.operand))
        is SqlExpr.FuncCall  -> SqlExpr.FuncCall(expr.name, expr.args.map { resolveExpr(scope, it) })
        is SqlExpr.IsNull    -> SqlExpr.IsNull(resolveExpr(scope, expr.operand))
        is SqlExpr.IsNotNull -> SqlExpr.IsNotNull(resolveExpr(scope, expr.operand))
        is SqlExpr.Between   -> SqlExpr.Between(resolveExpr(scope, expr.value), resolveExpr(scope, expr.low), resolveExpr(scope, expr.high))
        is SqlExpr.In        -> SqlExpr.In(resolveExpr(scope, expr.value), expr.items.map { resolveExpr(scope, it) })
        is SqlExpr.NotIn     -> SqlExpr.NotIn(resolveExpr(scope, expr.value), expr.items.map { resolveExpr(scope, it) })
        is SqlExpr.Like      -> SqlExpr.Like(resolveExpr(scope, expr.value), expr.pattern)
        is SqlExpr.NotLike   -> SqlExpr.NotLike(resolveExpr(scope, expr.value), expr.pattern)
    }

    private fun tryResolveExpr(scope: List<ScopeEntry>, expr: SqlExpr): SqlExpr? =
        try { resolveExpr(scope, expr) }
        catch (_: UnknownColumnException) { null }

    private fun containsAggExpr(e: SqlExpr): Boolean = when (e) {
        is SqlExpr.AggExpr   -> true
        is SqlExpr.BinaryOp  -> containsAggExpr(e.left) || containsAggExpr(e.right)
        is SqlExpr.UnaryOp   -> containsAggExpr(e.operand)
        is SqlExpr.FuncCall  -> e.args.any { containsAggExpr(it) }
        is SqlExpr.IsNull    -> containsAggExpr(e.operand)
        is SqlExpr.IsNotNull -> containsAggExpr(e.operand)
        is SqlExpr.Between   -> containsAggExpr(e.value) || containsAggExpr(e.low) || containsAggExpr(e.high)
        is SqlExpr.In        -> containsAggExpr(e.value) || e.items.any { containsAggExpr(it) }
        is SqlExpr.NotIn     -> containsAggExpr(e.value) || e.items.any { containsAggExpr(it) }
        is SqlExpr.Like      -> containsAggExpr(e.value)
        is SqlExpr.NotLike   -> containsAggExpr(e.value)
        else                 -> false
    }

    private fun collectAggregates(exprs: List<SqlExpr>): List<AggregateItem> {
        val found   = mutableListOf<AggregateItem>()
        var counter = 0
        fun walk(e: SqlExpr) {
            when (e) {
                is SqlExpr.AggExpr   -> { found.add(AggregateItem(e.func, e.arg, "_agg${counter++}", e.distinct)) }
                is SqlExpr.BinaryOp  -> { walk(e.left); walk(e.right) }
                is SqlExpr.UnaryOp   -> walk(e.operand)
                is SqlExpr.FuncCall  -> e.args.forEach { walk(it) }
                is SqlExpr.IsNull    -> walk(e.operand)
                is SqlExpr.IsNotNull -> walk(e.operand)
                is SqlExpr.Between   -> { walk(e.value); walk(e.low); walk(e.high) }
                is SqlExpr.In        -> { walk(e.value); e.items.forEach { walk(it) } }
                is SqlExpr.NotIn     -> { walk(e.value); e.items.forEach { walk(it) } }
                is SqlExpr.Like      -> walk(e.value)
                is SqlExpr.NotLike   -> walk(e.value)
                else                 -> {}
            }
        }
        exprs.forEach { walk(it) }
        return found
    }

    private fun buildFromTree(from: List<TableRef>, joins: List<JoinClause>): LogicalPlan {
        if (from.isEmpty()) throw UnsupportedStatementException("SELECT without FROM")
        schema.columns(from[0].table)   // validate
        var plan: LogicalPlan = LogicalPlan.Scan(from[0].table, from[0].alias)
        for (i in 1 until from.size) {
            schema.columns(from[i].table)  // validate
            plan = LogicalPlan.Join(plan, LogicalPlan.Scan(from[i].table, from[i].alias), JoinKind.CROSS, null)
        }
        for (j in joins) {
            schema.columns(j.table)  // validate
            plan = LogicalPlan.Join(plan, LogicalPlan.Scan(j.table, j.alias), j.kind, j.on)
        }
        return plan
    }

    private fun planSelect(s: Statement.Select): LogicalPlan {
        val scope    = buildScope(s.from, s.joins)
        val fromPlan = buildFromTree(s.from, s.joins)

        // Step 1: WHERE → Filter
        var plan: LogicalPlan = if (s.where != null)
            LogicalPlan.Filter(fromPlan, resolveExpr(scope, s.where))
        else fromPlan

        // Determine whether aggregation is required.
        val colExprs    = s.columns.map { if (it is OutputColumn.Expr) it.expression else SqlExpr.Wildcard }
        val havingExprs = listOfNotNull(s.having)
        val needsAgg    = s.groupBy.isNotEmpty() ||
            colExprs.any    { containsAggExpr(it) } ||
            havingExprs.any { containsAggExpr(it) }

        // Step 2: GROUP BY + Aggregate
        if (needsAgg) {
            val aggs    = collectAggregates(colExprs + havingExprs)
            val groupBy = s.groupBy.map { resolveExpr(scope, it) }
            plan = LogicalPlan.Aggregate(plan, groupBy, aggs)
        }

        // Step 3: HAVING
        if (s.having != null) {
            val rHaving = tryResolveExpr(scope, s.having) ?: s.having
            plan = LogicalPlan.Having(plan, rHaving)
        }

        // Step 4: PROJECT
        val projCols = s.columns.map { col ->
            when (col) {
                is OutputColumn.Star -> OutputColumn.Star
                is OutputColumn.Expr -> {
                    val resolved = if (needsAgg)
                        (tryResolveExpr(scope, col.expression) ?: col.expression)
                    else resolveExpr(scope, col.expression)
                    OutputColumn.Expr(resolved, col.alias)
                }
            }
        }
        plan = LogicalPlan.Project(plan, projCols)

        // Step 5: DISTINCT
        if (s.distinct) plan = LogicalPlan.Distinct(plan)

        // Step 6: ORDER BY
        if (s.orderBy.isNotEmpty()) {
            val keys = s.orderBy.map { key ->
                val r = tryResolveExpr(scope, key.keyExpr)
                SortKey(r ?: key.keyExpr, key.direction, key.nullOrder)
            }
            plan = LogicalPlan.Sort(plan, keys)
        }

        // Step 7: LIMIT / OFFSET
        if (s.limit != null) {
            plan = LogicalPlan.Limit(plan, s.limit.count, s.limit.offset)
        }

        return plan
    }

    /** Transform a single statement into a logical plan. Throws [PlanException] on error. */
    fun plan(stmt: Statement): LogicalPlan = when (stmt) {
        is Statement.Select      -> planSelect(stmt)
        is Statement.Insert      -> { schema.columns(stmt.table); LogicalPlan.Insert(stmt.table, stmt.columns, stmt.values) }
        is Statement.Update      -> { schema.columns(stmt.table); LogicalPlan.Update(stmt.table, stmt.assignments, stmt.where) }
        is Statement.Delete      -> { schema.columns(stmt.table); LogicalPlan.Delete(stmt.table, stmt.where) }
        is Statement.CreateTable -> LogicalPlan.CreateTable(stmt.table, stmt.ifNotExists, stmt.columns)
        is Statement.DropTable   -> LogicalPlan.DropTable(stmt.table, stmt.ifExists)
    }

    /** Plan every statement in the list; throws on the first error. */
    fun planAll(stmts: List<Statement>): List<LogicalPlan> = stmts.map { plan(it) }
}

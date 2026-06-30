package com.codingadventures.sqloptimizer

// SqlOptimizer.kt — logical query plan optimizer for SQL statements.
//
// Takes a LogicalPlan produced by SqlPlanner and applies a pipeline of
// rule-based optimization passes to produce an OptimizedPlan, which mirrors
// the LogicalPlan node set but adds optimizer-specific fields:
//
//   • Scan.requiredColumns — column pruning: only fetch columns actually used
//   • Scan.scanLimit       — pushed-down LIMIT so storage can stop early
//   • EmptyResult          — sentinel for provably-empty subtrees
//
// Architecture
// ────────────
// Each pass is a stateless function implementing the Pass interface. Passes
// are composed via optimizeWithPasses, which applies them in order over the
// same OptimizedPlan tree. The default ordering is:
//
//   1. ConstantFolding     — fold literal arithmetic/logic at compile time
//   2. PredicatePushdown   — move Filter nodes as close to Scan as possible
//   3. ProjectionPruning   — annotate Scans with only the columns they need
//   4. DeadCodeElimination — replace provably-empty subtrees with EmptyResult
//   5. LimitPushdown       — push Limit counts down to Scan.scanLimit
//
// No I/O, no database connections — pure in-memory tree transformation.
//
// Usage:
//   val logical = SqlPlanner(schema).plan(stmt)
//   val optimized = SqlOptimizer.optimize(logical)

import com.codingadventures.sqlplanner.*

// ── Optimized plan nodes ──────────────────────────────────────────────────────
//
// Mirrors LogicalPlan exactly, but:
//  - Scan gains requiredColumns (null = all) and scanLimit (null = no limit)
//  - EmptyResult is a new terminal signifying a provably-empty relation

sealed class OptimizedPlan {
    /** Table scan. requiredColumns=null means "all columns". */
    data class Scan(
        val table: String,
        val alias: String?,
        val requiredColumns: List<String>? = null,
        val scanLimit: Long? = null
    ) : OptimizedPlan()

    data class Filter(val input: OptimizedPlan, val predicate: SqlExpr) : OptimizedPlan()
    data class Project(val input: OptimizedPlan, val columns: List<OutputColumn>) : OptimizedPlan()
    data class Join(val left: OptimizedPlan, val right: OptimizedPlan, val kind: JoinKind, val condition: SqlExpr?) : OptimizedPlan()
    data class Aggregate(val input: OptimizedPlan, val groupBy: List<SqlExpr>, val aggregates: List<AggregateItem>) : OptimizedPlan()
    data class Having(val input: OptimizedPlan, val predicate: SqlExpr) : OptimizedPlan()
    data class Sort(val input: OptimizedPlan, val keys: List<SortKey>) : OptimizedPlan()
    data class Limit(val input: OptimizedPlan, val count: Long?, val offset: Long?) : OptimizedPlan()
    data class Distinct(val input: OptimizedPlan) : OptimizedPlan()
    data class Union(val left: OptimizedPlan, val right: OptimizedPlan, val all: Boolean) : OptimizedPlan()
    data class Insert(val table: String, val columns: List<String>, val values: List<List<SqlExpr>>) : OptimizedPlan()
    data class Update(val table: String, val assignments: List<Assignment>, val predicate: SqlExpr?) : OptimizedPlan()
    data class Delete(val table: String, val predicate: SqlExpr?) : OptimizedPlan()
    data class CreateTable(val table: String, val ifNotExists: Boolean, val columns: List<ColumnDef>) : OptimizedPlan()
    data class DropTable(val table: String, val ifExists: Boolean) : OptimizedPlan()

    /** Sentinel for a subtree that is provably empty (zero rows). */
    object EmptyResult : OptimizedPlan()
}

// ── Pass interface ────────────────────────────────────────────────────────────

/** A single-responsibility, stateless optimization pass. */
interface Pass {
    val name: String
    fun apply(plan: OptimizedPlan): OptimizedPlan
}

// ── Pass 1: Constant Folding ──────────────────────────────────────────────────
//
// Evaluates expressions that involve only literal (compile-time-known) values:
//
//   1 + 1            → Literal(2)
//   TRUE AND FALSE   → Literal(false)
//   NULL IS NULL     → Literal(true)
//   NOT TRUE         → Literal(false)
//
// Short-circuit rules (applies even when one side is non-literal):
//   x AND false  → false
//   x AND true   → x
//   x OR  true   → true
//   x OR  false  → x
//
// NULL propagation: arithmetic/comparison with NULL yields NULL.
// Division by zero: NOT folded (let the runtime handle it to avoid masking errors).

object ConstantFoldingPass : Pass {
    override val name = "ConstantFolding"

    override fun apply(plan: OptimizedPlan): OptimizedPlan = foldPlan(plan)

    private fun foldPlan(plan: OptimizedPlan): OptimizedPlan = when (plan) {
        is OptimizedPlan.Filter    -> OptimizedPlan.Filter(foldPlan(plan.input), foldExpr(plan.predicate))
        is OptimizedPlan.Project   -> OptimizedPlan.Project(foldPlan(plan.input), plan.columns.map { foldOutputCol(it) })
        is OptimizedPlan.Join      -> OptimizedPlan.Join(foldPlan(plan.left), foldPlan(plan.right), plan.kind, plan.condition?.let { foldExpr(it) })
        is OptimizedPlan.Aggregate -> OptimizedPlan.Aggregate(foldPlan(plan.input), plan.groupBy.map { foldExpr(it) }, plan.aggregates)
        is OptimizedPlan.Having    -> OptimizedPlan.Having(foldPlan(plan.input), foldExpr(plan.predicate))
        is OptimizedPlan.Sort      -> OptimizedPlan.Sort(foldPlan(plan.input), plan.keys.map { SortKey(foldExpr(it.keyExpr), it.direction, it.nullOrder) })
        is OptimizedPlan.Limit     -> OptimizedPlan.Limit(foldPlan(plan.input), plan.count, plan.offset)
        is OptimizedPlan.Distinct  -> OptimizedPlan.Distinct(foldPlan(plan.input))
        is OptimizedPlan.Union     -> OptimizedPlan.Union(foldPlan(plan.left), foldPlan(plan.right), plan.all)
        else                       -> plan
    }

    private fun foldOutputCol(col: OutputColumn): OutputColumn = when (col) {
        is OutputColumn.Star -> col
        is OutputColumn.Expr -> OutputColumn.Expr(foldExpr(col.expression), col.alias)
    }

    // Recursively fold an expression. Returns a new SqlExpr (or the same one if no folding applies).
    internal fun foldExpr(expr: SqlExpr): SqlExpr = when (expr) {
        is SqlExpr.UnaryOp  -> foldUnary(expr)
        is SqlExpr.BinaryOp -> foldBinary(expr)
        is SqlExpr.IsNull   -> {
            val op = foldExpr(expr.operand)
            if (op is SqlExpr.Literal) SqlExpr.Literal(op.value == null) else SqlExpr.IsNull(op)
        }
        is SqlExpr.IsNotNull -> {
            val op = foldExpr(expr.operand)
            if (op is SqlExpr.Literal) SqlExpr.Literal(op.value != null) else SqlExpr.IsNotNull(op)
        }
        is SqlExpr.Between -> {
            val v = foldExpr(expr.value); val lo = foldExpr(expr.low); val hi = foldExpr(expr.high)
            if (v is SqlExpr.Literal && lo is SqlExpr.Literal && hi is SqlExpr.Literal) {
                if (v.value == null || lo.value == null || hi.value == null) SqlExpr.Literal(null)
                else {
                    val vd = toDouble(v.value); val lod = toDouble(lo.value); val hid = toDouble(hi.value)
                    if (vd != null && lod != null && hid != null) SqlExpr.Literal(vd >= lod && vd <= hid)
                    else SqlExpr.Between(v, lo, hi)
                }
            } else SqlExpr.Between(v, lo, hi)
        }
        is SqlExpr.In -> {
            val v = foldExpr(expr.value)
            val items = expr.items.map { foldExpr(it) }
            if (v is SqlExpr.Literal && items.all { it is SqlExpr.Literal }) {
                SqlExpr.Literal(items.any { (it as SqlExpr.Literal).value == v.value })
            } else SqlExpr.In(v, items)
        }
        is SqlExpr.NotIn -> {
            val v = foldExpr(expr.value)
            val items = expr.items.map { foldExpr(it) }
            if (v is SqlExpr.Literal && items.all { it is SqlExpr.Literal }) {
                SqlExpr.Literal(items.none { (it as SqlExpr.Literal).value == v.value })
            } else SqlExpr.NotIn(v, items)
        }
        is SqlExpr.FuncCall -> SqlExpr.FuncCall(expr.name, expr.args.map { foldExpr(it) })
        else -> expr
    }

    private fun foldUnary(expr: SqlExpr.UnaryOp): SqlExpr {
        val operand = foldExpr(expr.operand)
        if (operand is SqlExpr.Literal) {
            val v = operand.value
            return when (expr.op) {
                UnaryOperator.NOT -> when (v) {
                    null  -> SqlExpr.Literal(null)
                    true  -> SqlExpr.Literal(false)
                    false -> SqlExpr.Literal(true)
                    else  -> SqlExpr.UnaryOp(expr.op, operand)
                }
                UnaryOperator.NEG -> when {
                    v == null -> SqlExpr.Literal(null)
                    v is Long -> SqlExpr.Literal(-v)
                    v is Int  -> SqlExpr.Literal(-v.toLong())
                    v is Double -> SqlExpr.Literal(-v)
                    else -> SqlExpr.UnaryOp(expr.op, operand)
                }
            }
        }
        return SqlExpr.UnaryOp(expr.op, operand)
    }

    private fun foldBinary(expr: SqlExpr.BinaryOp): SqlExpr {
        val left  = foldExpr(expr.left)
        val right = foldExpr(expr.right)

        // Short-circuit for AND / OR before null checks
        if (expr.op == BinaryOperator.AND) {
            if (left  == SqlExpr.Literal(false) || right == SqlExpr.Literal(false)) return SqlExpr.Literal(false)
            if (left  == SqlExpr.Literal(true))  return right
            if (right == SqlExpr.Literal(true))  return left
        }
        if (expr.op == BinaryOperator.OR) {
            if (left  == SqlExpr.Literal(true) || right == SqlExpr.Literal(true)) return SqlExpr.Literal(true)
            if (left  == SqlExpr.Literal(false)) return right
            if (right == SqlExpr.Literal(false)) return left
        }

        // Both sides literal
        if (left is SqlExpr.Literal && right is SqlExpr.Literal) {
            val lv = left.value; val rv = right.value
            // NULL propagation for arithmetic and comparison
            if ((lv == null || rv == null) && expr.op !in setOf(BinaryOperator.AND, BinaryOperator.OR)) {
                return SqlExpr.Literal(null)
            }
            return foldLiterals(expr.op, lv, rv) ?: SqlExpr.BinaryOp(expr.op, left, right)
        }

        return SqlExpr.BinaryOp(expr.op, left, right)
    }

    private fun foldLiterals(op: BinaryOperator, lv: Any?, rv: Any?): SqlExpr? {
        // Numeric folding
        val ld = toDouble(lv); val rd = toDouble(rv)
        if (ld != null && rd != null) {
            return when (op) {
                BinaryOperator.ADD -> SqlExpr.Literal(foldNumeric(lv, rv) { a, b -> a + b })
                BinaryOperator.SUB -> SqlExpr.Literal(foldNumeric(lv, rv) { a, b -> a - b })
                BinaryOperator.MUL -> SqlExpr.Literal(foldNumeric(lv, rv) { a, b -> a * b })
                // Do NOT fold DIV — let the runtime handle div-by-zero
                BinaryOperator.DIV -> null
                BinaryOperator.MOD -> if (rd == 0.0) null else SqlExpr.Literal(foldNumeric(lv, rv) { a, b -> a % b })
                BinaryOperator.EQ     -> SqlExpr.Literal(ld == rd)
                BinaryOperator.NOT_EQ -> SqlExpr.Literal(ld != rd)
                BinaryOperator.LT     -> SqlExpr.Literal(ld < rd)
                BinaryOperator.LTE    -> SqlExpr.Literal(ld <= rd)
                BinaryOperator.GT     -> SqlExpr.Literal(ld > rd)
                BinaryOperator.GTE    -> SqlExpr.Literal(ld >= rd)
                BinaryOperator.AND    -> null  // handled above
                BinaryOperator.OR     -> null
            }
        }
        // String equality
        if (lv is String && rv is String) {
            return when (op) {
                BinaryOperator.EQ     -> SqlExpr.Literal(lv == rv)
                BinaryOperator.NOT_EQ -> SqlExpr.Literal(lv != rv)
                BinaryOperator.LT     -> SqlExpr.Literal(lv < rv)
                BinaryOperator.LTE    -> SqlExpr.Literal(lv <= rv)
                BinaryOperator.GT     -> SqlExpr.Literal(lv > rv)
                BinaryOperator.GTE    -> SqlExpr.Literal(lv >= rv)
                BinaryOperator.ADD    -> SqlExpr.Literal(lv + rv)
                else -> null
            }
        }
        // Boolean logic (already short-circuited above, but handle AND/OR of non-null booleans)
        if (lv is Boolean && rv is Boolean) {
            return when (op) {
                BinaryOperator.AND -> SqlExpr.Literal(lv && rv)
                BinaryOperator.OR  -> SqlExpr.Literal(lv || rv)
                BinaryOperator.EQ  -> SqlExpr.Literal(lv == rv)
                BinaryOperator.NOT_EQ -> SqlExpr.Literal(lv != rv)
                else -> null
            }
        }
        // Equality for any comparable types
        return when (op) {
            BinaryOperator.EQ     -> SqlExpr.Literal(lv == rv)
            BinaryOperator.NOT_EQ -> SqlExpr.Literal(lv != rv)
            else -> null
        }
    }

    private fun foldNumeric(lv: Any?, rv: Any?, op: (Double, Double) -> Double): Any {
        val ld = toDouble(lv)!!; val rd = toDouble(rv)!!
        val result = op(ld, rd)
        // Preserve Long type if both inputs are integral and result is whole
        return if ((lv is Long || lv is Int) && (rv is Long || rv is Int) && result == result.toLong().toDouble())
            result.toLong()
        else result
    }

    private fun toDouble(v: Any?): Double? = when (v) {
        is Long   -> v.toDouble()
        is Int    -> v.toDouble()
        is Double -> v
        is Float  -> v.toDouble()
        else      -> null
    }
}

// ── Pass 2: Predicate Pushdown ────────────────────────────────────────────────
//
// Moves Filter nodes as close to their data source (Scan) as possible.
// This reduces the number of rows that flow through expensive operations like
// Sort, Distinct, or Project.
//
// Rules (applied recursively, bottom-up):
//
//   Filter(Project, p)     → Project(Filter(input, p))    if p references only
//                                                          columns in the input
//   Filter(Sort, p)        → Sort(Filter(input, p))       always
//   Filter(Distinct, p)    → Distinct(Filter(input, p))   always
//   Filter(Join INNER, p)  → push conjuncts to the side they reference
//   Filter(Join LEFT, p)   → push left-only conjuncts to left; keep right as-is
//   Filter(Join RIGHT, p)  → push right-only conjuncts to right; keep left as-is
//   Filter(Aggregate, p)   → stop (predicate may reference aggregate output)
//   Filter(Limit, p)       → stop (limit semantics change if rows removed)

object PredicatePushdownPass : Pass {
    override val name = "PredicatePushdown"

    override fun apply(plan: OptimizedPlan): OptimizedPlan = pushPlan(plan)

    private fun pushPlan(plan: OptimizedPlan): OptimizedPlan = when (plan) {
        is OptimizedPlan.Filter    -> pushFilter(plan.predicate, pushPlan(plan.input))
        is OptimizedPlan.Project   -> OptimizedPlan.Project(pushPlan(plan.input), plan.columns)
        is OptimizedPlan.Join      -> OptimizedPlan.Join(pushPlan(plan.left), pushPlan(plan.right), plan.kind, plan.condition)
        is OptimizedPlan.Aggregate -> OptimizedPlan.Aggregate(pushPlan(plan.input), plan.groupBy, plan.aggregates)
        is OptimizedPlan.Having    -> OptimizedPlan.Having(pushPlan(plan.input), plan.predicate)
        is OptimizedPlan.Sort      -> OptimizedPlan.Sort(pushPlan(plan.input), plan.keys)
        is OptimizedPlan.Limit     -> OptimizedPlan.Limit(pushPlan(plan.input), plan.count, plan.offset)
        is OptimizedPlan.Distinct  -> OptimizedPlan.Distinct(pushPlan(plan.input))
        is OptimizedPlan.Union     -> OptimizedPlan.Union(pushPlan(plan.left), pushPlan(plan.right), plan.all)
        else                       -> plan
    }

    // Try to push a predicate into the input plan. Returns a new plan node.
    private fun pushFilter(predicate: SqlExpr, input: OptimizedPlan): OptimizedPlan {
        val conjuncts = splitConjuncts(predicate)

        return when (input) {
            // Push through Sort — sort doesn't change which rows survive a filter
            is OptimizedPlan.Sort ->
                OptimizedPlan.Sort(pushFilter(predicate, input.input), input.keys)

            // Push through Distinct — distinct doesn't change row membership
            is OptimizedPlan.Distinct ->
                OptimizedPlan.Distinct(pushFilter(predicate, input.input))

            // Push through Project — rewrap with filter on the project's own input
            is OptimizedPlan.Project ->
                OptimizedPlan.Project(pushFilter(predicate, input.input), input.columns)

            // Inner join: split conjuncts to left, right, or keep on the join
            is OptimizedPlan.Join when input.kind == JoinKind.INNER -> {
                val leftAlias  = tableAliasOf(input.left)
                val rightAlias = tableAliasOf(input.right)
                val (leftConj, rightConj, keep) = partitionConjuncts(conjuncts, leftAlias, rightAlias)
                val newLeft  = if (leftConj.isEmpty())  input.left  else pushFilter(conjoin(leftConj),  input.left)
                val newRight = if (rightConj.isEmpty()) input.right else pushFilter(conjoin(rightConj), input.right)
                val join = OptimizedPlan.Join(newLeft, newRight, input.kind, input.condition)
                if (keep.isEmpty()) join else OptimizedPlan.Filter(join, conjoin(keep))
            }

            // Left join: push left-only conjuncts to left side; keep everything else
            is OptimizedPlan.Join when input.kind == JoinKind.LEFT -> {
                val leftAlias = tableAliasOf(input.left)
                val (leftConj, keep) = conjuncts.partition { refsOnlyTable(it, leftAlias) }
                val newLeft = if (leftConj.isEmpty()) input.left else pushFilter(conjoin(leftConj), input.left)
                val join = OptimizedPlan.Join(newLeft, input.right, input.kind, input.condition)
                if (keep.isEmpty()) join else OptimizedPlan.Filter(join, conjoin(keep))
            }

            // Right join: push right-only conjuncts to right side; keep everything else
            is OptimizedPlan.Join when input.kind == JoinKind.RIGHT -> {
                val rightAlias = tableAliasOf(input.right)
                val (rightConj, keep) = conjuncts.partition { refsOnlyTable(it, rightAlias) }
                val newRight = if (rightConj.isEmpty()) input.right else pushFilter(conjoin(rightConj), input.right)
                val join = OptimizedPlan.Join(input.left, newRight, input.kind, input.condition)
                if (keep.isEmpty()) join else OptimizedPlan.Filter(join, conjoin(keep))
            }

            // Stop at Aggregate, Limit, Scan, EmptyResult, DML
            else -> OptimizedPlan.Filter(input, predicate)
        }
    }

    /** Split a predicate on AND boundaries into a flat list of conjuncts. */
    private fun splitConjuncts(expr: SqlExpr): List<SqlExpr> {
        return if (expr is SqlExpr.BinaryOp && expr.op == BinaryOperator.AND)
            splitConjuncts(expr.left) + splitConjuncts(expr.right)
        else listOf(expr)
    }

    /** Re-join a list of conjuncts with AND. List must be non-empty. */
    private fun conjoin(exprs: List<SqlExpr>): SqlExpr =
        exprs.reduce { acc, e -> SqlExpr.BinaryOp(BinaryOperator.AND, acc, e) }

    /** Get the table/alias string for a scan-like plan (best-effort; null if complex). */
    private fun tableAliasOf(plan: OptimizedPlan): String? = when (plan) {
        is OptimizedPlan.Scan -> plan.alias ?: plan.table
        else -> null
    }

    /** Check whether an expression references only a specific table alias (or no table at all). */
    private fun refsOnlyTable(expr: SqlExpr, alias: String?): Boolean {
        if (alias == null) return false
        return collectTableRefs(expr).let { refs ->
            refs.isEmpty() || refs.all { it == null || it == alias }
        }
    }

    /** Collect all table references in an expression. */
    private fun collectTableRefs(expr: SqlExpr): Set<String?> = when (expr) {
        is SqlExpr.Column   -> setOf(expr.table)
        is SqlExpr.BinaryOp -> collectTableRefs(expr.left) + collectTableRefs(expr.right)
        is SqlExpr.UnaryOp  -> collectTableRefs(expr.operand)
        is SqlExpr.FuncCall -> expr.args.flatMap { collectTableRefs(it) }.toSet()
        is SqlExpr.IsNull   -> collectTableRefs(expr.operand)
        is SqlExpr.IsNotNull -> collectTableRefs(expr.operand)
        is SqlExpr.Between  -> collectTableRefs(expr.value) + collectTableRefs(expr.low) + collectTableRefs(expr.high)
        is SqlExpr.In       -> collectTableRefs(expr.value) + expr.items.flatMap { collectTableRefs(it) }
        is SqlExpr.NotIn    -> collectTableRefs(expr.value) + expr.items.flatMap { collectTableRefs(it) }
        is SqlExpr.Like     -> collectTableRefs(expr.value)
        is SqlExpr.NotLike  -> collectTableRefs(expr.value)
        else                -> emptySet()
    }

    /**
     * Partition conjuncts into (left-only, right-only, keep-on-join).
     * A conjunct that only references columns from the left alias goes left;
     * same logic for right; anything else stays on the join.
     */
    private fun partitionConjuncts(
        conjuncts: List<SqlExpr>,
        leftAlias: String?,
        rightAlias: String?
    ): Triple<List<SqlExpr>, List<SqlExpr>, List<SqlExpr>> {
        val left  = mutableListOf<SqlExpr>()
        val right = mutableListOf<SqlExpr>()
        val keep  = mutableListOf<SqlExpr>()
        for (c in conjuncts) {
            when {
                leftAlias != null && refsOnlyTable(c, leftAlias)   -> left.add(c)
                rightAlias != null && refsOnlyTable(c, rightAlias) -> right.add(c)
                else -> keep.add(c)
            }
        }
        return Triple(left, right, keep)
    }
}

// ── Pass 3: Projection Pruning ────────────────────────────────────────────────
//
// Annotates each Scan node with only the columns that are actually referenced
// by the query above it. This allows storage layers to skip fetching unreferenced
// columns (columnar stores, covering indices, etc.).
//
// Strategy: top-down traversal carries a required-column set as
// Set<Pair<String?, String>> (table-alias, column-name). When we reach a Scan
// we can intersect that set with the scan's alias to produce requiredColumns.
//
// Wildcard (SELECT *) disables pruning for that branch — all columns are needed.

object ProjectionPruningPass : Pass {
    override val name = "ProjectionPruning"

    // Pair<table alias or null, column name>
    private typealias ColRef = Pair<String?, String>

    override fun apply(plan: OptimizedPlan): OptimizedPlan = prunePlan(plan, null)

    /**
     * [required] = null means "all columns needed" (e.g., after SELECT * or unknown ref).
     * [required] = non-null set means "only these (table,col) pairs are needed".
     */
    private fun prunePlan(plan: OptimizedPlan, required: Set<ColRef>?): OptimizedPlan = when (plan) {
        is OptimizedPlan.Scan -> {
            val cols = if (required == null) null else {
                val scanAlias = plan.alias ?: plan.table
                val matched = required
                    .filter { (t, _) -> t == null || t == scanAlias }
                    .map { (_, c) -> c }
                    .distinct()
                if (matched.isEmpty()) null else matched
            }
            plan.copy(requiredColumns = cols)
        }

        is OptimizedPlan.Project -> {
            // Collect columns referenced by the project's output expressions
            val projRequired = collectFromOutputCols(plan.columns)
            // Merge with what was required above us (intersection not needed — pass union down)
            val downstream = if (required == null || projRequired == null) null
            else required + projRequired
            OptimizedPlan.Project(prunePlan(plan.input, downstream ?: projRequired), plan.columns)
        }

        is OptimizedPlan.Filter ->
            OptimizedPlan.Filter(
                prunePlan(plan.input, mergeRequired(required, collectFromExpr(plan.predicate))),
                plan.predicate
            )

        is OptimizedPlan.Join ->
            OptimizedPlan.Join(
                prunePlan(plan.left, required),
                prunePlan(plan.right, required),
                plan.kind,
                plan.condition
            )

        is OptimizedPlan.Aggregate ->
            OptimizedPlan.Aggregate(
                prunePlan(plan.input, null),  // aggregates may need all cols
                plan.groupBy,
                plan.aggregates
            )

        is OptimizedPlan.Having ->
            OptimizedPlan.Having(
                prunePlan(plan.input, mergeRequired(required, collectFromExpr(plan.predicate))),
                plan.predicate
            )

        is OptimizedPlan.Sort ->
            OptimizedPlan.Sort(
                prunePlan(plan.input, mergeRequired(required, plan.keys.flatMap { collectFromExpr(it.keyExpr) }.toSet())),
                plan.keys
            )

        is OptimizedPlan.Limit    -> OptimizedPlan.Limit(prunePlan(plan.input, required), plan.count, plan.offset)
        is OptimizedPlan.Distinct -> OptimizedPlan.Distinct(prunePlan(plan.input, required))

        is OptimizedPlan.Union ->
            OptimizedPlan.Union(prunePlan(plan.left, required), prunePlan(plan.right, required), plan.all)

        else -> plan
    }

    private fun mergeRequired(a: Set<ColRef>?, b: Set<ColRef>?): Set<ColRef>? =
        if (a == null || b == null) null else a + b

    private fun collectFromOutputCols(cols: List<OutputColumn>): Set<ColRef>? {
        if (cols.any { it is OutputColumn.Star }) return null  // SELECT * — need all
        return cols.filterIsInstance<OutputColumn.Expr>()
            .flatMap { collectFromExpr(it.expression) }
            .toSet()
    }

    private fun collectFromExpr(expr: SqlExpr): Set<ColRef> = when (expr) {
        is SqlExpr.Column   -> setOf(Pair(expr.table, expr.column))
        is SqlExpr.BinaryOp -> collectFromExpr(expr.left) + collectFromExpr(expr.right)
        is SqlExpr.UnaryOp  -> collectFromExpr(expr.operand)
        is SqlExpr.FuncCall -> expr.args.flatMap { collectFromExpr(it) }.toSet()
        is SqlExpr.IsNull   -> collectFromExpr(expr.operand)
        is SqlExpr.IsNotNull -> collectFromExpr(expr.operand)
        is SqlExpr.Between  -> collectFromExpr(expr.value) + collectFromExpr(expr.low) + collectFromExpr(expr.high)
        is SqlExpr.In       -> collectFromExpr(expr.value) + expr.items.flatMap { collectFromExpr(it) }
        is SqlExpr.NotIn    -> collectFromExpr(expr.value) + expr.items.flatMap { collectFromExpr(it) }
        is SqlExpr.Like     -> collectFromExpr(expr.value)
        is SqlExpr.NotLike  -> collectFromExpr(expr.value)
        is SqlExpr.AggExpr  -> when (val arg = expr.arg) {
            is AggArg.Star -> emptySet()
            is AggArg.Expr -> collectFromExpr(arg.expression)
        }
        else -> emptySet()
    }
}

// ── Pass 4: Dead Code Elimination ────────────────────────────────────────────
//
// Replaces subtrees that are provably empty with EmptyResult, and propagates
// EmptyResult upward when possible.
//
// Trigger conditions:
//
//   Filter(plan, FALSE)         → EmptyResult
//   Filter(plan, NULL)          → EmptyResult
//   Limit(plan, 0)              → EmptyResult
//   Filter(EmptyResult, _)      → EmptyResult
//   Project(EmptyResult, _)     → EmptyResult
//   Sort(EmptyResult, _)        → EmptyResult
//   Distinct(EmptyResult)       → EmptyResult
//   Limit(EmptyResult, _)       → EmptyResult
//   Having(EmptyResult, _)      → EmptyResult
//   Join(EmptyResult, _, INNER) → EmptyResult
//   Join(_, EmptyResult, INNER) → EmptyResult
//   Union(EmptyResult, r, _)    → r
//   Union(l, EmptyResult, _)    → l
//
// NOT eliminated:
//   Aggregate(EmptyResult)      — aggregates over empty still produce a row (COUNT(*) = 0)

object DeadCodeEliminationPass : Pass {
    override val name = "DeadCodeElimination"

    override fun apply(plan: OptimizedPlan): OptimizedPlan = elimPlan(plan)

    private fun elimPlan(plan: OptimizedPlan): OptimizedPlan = when (plan) {
        is OptimizedPlan.Filter -> {
            val input = elimPlan(plan.input)
            when {
                input is OptimizedPlan.EmptyResult -> OptimizedPlan.EmptyResult
                isFalsy(plan.predicate)            -> OptimizedPlan.EmptyResult
                else -> OptimizedPlan.Filter(input, plan.predicate)
            }
        }

        is OptimizedPlan.Project -> {
            val input = elimPlan(plan.input)
            if (input is OptimizedPlan.EmptyResult) OptimizedPlan.EmptyResult
            else OptimizedPlan.Project(input, plan.columns)
        }

        is OptimizedPlan.Sort -> {
            val input = elimPlan(plan.input)
            if (input is OptimizedPlan.EmptyResult) OptimizedPlan.EmptyResult
            else OptimizedPlan.Sort(input, plan.keys)
        }

        is OptimizedPlan.Distinct -> {
            val input = elimPlan(plan.input)
            if (input is OptimizedPlan.EmptyResult) OptimizedPlan.EmptyResult
            else OptimizedPlan.Distinct(input)
        }

        is OptimizedPlan.Limit -> {
            val input = elimPlan(plan.input)
            when {
                input is OptimizedPlan.EmptyResult -> OptimizedPlan.EmptyResult
                plan.count == 0L                   -> OptimizedPlan.EmptyResult
                else -> OptimizedPlan.Limit(input, plan.count, plan.offset)
            }
        }

        is OptimizedPlan.Having -> {
            val input = elimPlan(plan.input)
            when {
                input is OptimizedPlan.EmptyResult -> OptimizedPlan.EmptyResult
                isFalsy(plan.predicate)            -> OptimizedPlan.EmptyResult
                else -> OptimizedPlan.Having(input, plan.predicate)
            }
        }

        is OptimizedPlan.Join -> {
            val left  = elimPlan(plan.left)
            val right = elimPlan(plan.right)
            if (plan.kind == JoinKind.INNER &&
                (left is OptimizedPlan.EmptyResult || right is OptimizedPlan.EmptyResult)) {
                OptimizedPlan.EmptyResult
            } else OptimizedPlan.Join(left, right, plan.kind, plan.condition)
        }

        is OptimizedPlan.Union -> {
            val left  = elimPlan(plan.left)
            val right = elimPlan(plan.right)
            when {
                left  is OptimizedPlan.EmptyResult -> right
                right is OptimizedPlan.EmptyResult -> left
                else -> OptimizedPlan.Union(left, right, plan.all)
            }
        }

        // Aggregate over EmptyResult is NOT eliminated — COUNT(*) of 0 rows = 0
        is OptimizedPlan.Aggregate ->
            OptimizedPlan.Aggregate(elimPlan(plan.input), plan.groupBy, plan.aggregates)

        else -> plan
    }

    /** Returns true if the expression is provably false or NULL (i.e., no rows survive). */
    private fun isFalsy(expr: SqlExpr): Boolean = when {
        expr == SqlExpr.Literal(false) -> true
        expr == SqlExpr.Literal(null)  -> true
        expr is SqlExpr.Literal && expr.value == null -> true
        else -> false
    }
}

// ── Pass 5: Limit Pushdown ────────────────────────────────────────────────────
//
// When a LIMIT N (with no OFFSET, or OFFSET=0) appears above a sequence of
// row-preserving operators (Project, Filter), the limit can be propagated
// down to the Scan node so the storage layer can stop early.
//
// Propagation stops at:
//   • Sort      — sort must see all rows before it can return the top N
//   • Aggregate — same reason
//   • Join      — limit at the outer level doesn't constrain individual sides
//   • Distinct  — distinct must see all rows to eliminate duplicates
//
// Multiple nested limits are reconciled with min(outer, inner).

object LimitPushdownPass : Pass {
    override val name = "LimitPushdown"

    override fun apply(plan: OptimizedPlan): OptimizedPlan = pushPlan(plan, null)

    /**
     * [limitHint] = the count pushed from above (null = no active hint).
     * Returns a rewritten plan that has Scan.scanLimit set where appropriate.
     */
    private fun pushPlan(plan: OptimizedPlan, limitHint: Long?): OptimizedPlan = when (plan) {
        is OptimizedPlan.Limit -> {
            // Reconcile our own count with what came from above
            val effectiveCount = when {
                plan.count == null -> limitHint
                limitHint == null  -> plan.count
                else               -> minOf(plan.count, limitHint)
            }
            // Only push down when offset is absent or zero
            val canPush = plan.offset == null || plan.offset == 0L
            val downstream = if (canPush) effectiveCount else null
            OptimizedPlan.Limit(pushPlan(plan.input, downstream), plan.count, plan.offset)
        }

        is OptimizedPlan.Project ->
            OptimizedPlan.Project(pushPlan(plan.input, limitHint), plan.columns)

        is OptimizedPlan.Filter ->
            // Filter may reduce rows, so the limit still applies downstream
            OptimizedPlan.Filter(pushPlan(plan.input, limitHint), plan.predicate)

        is OptimizedPlan.Having ->
            OptimizedPlan.Having(pushPlan(plan.input, limitHint), plan.predicate)

        is OptimizedPlan.Scan -> {
            if (limitHint == null) plan
            else plan.copy(scanLimit = if (plan.scanLimit == null) limitHint else minOf(plan.scanLimit, limitHint))
        }

        // Stop propagation at Sort, Aggregate, Join, Distinct
        is OptimizedPlan.Sort      -> OptimizedPlan.Sort(pushPlan(plan.input, null), plan.keys)
        is OptimizedPlan.Aggregate -> OptimizedPlan.Aggregate(pushPlan(plan.input, null), plan.groupBy, plan.aggregates)
        is OptimizedPlan.Distinct  -> OptimizedPlan.Distinct(pushPlan(plan.input, null))
        is OptimizedPlan.Join      -> OptimizedPlan.Join(pushPlan(plan.left, null), pushPlan(plan.right, null), plan.kind, plan.condition)

        is OptimizedPlan.Union ->
            OptimizedPlan.Union(pushPlan(plan.left, limitHint), pushPlan(plan.right, limitHint), plan.all)

        else -> plan
    }
}

// ── SqlOptimizer ──────────────────────────────────────────────────────────────
//
// Entry point. Converts a LogicalPlan to an OptimizedPlan, then runs the
// default pass pipeline.

object SqlOptimizer {

    /** Convert a LogicalPlan to an OptimizedPlan (1-to-1 structural lift). */
    fun lift(plan: LogicalPlan): OptimizedPlan = when (plan) {
        is LogicalPlan.Scan        -> OptimizedPlan.Scan(plan.table, plan.alias)
        is LogicalPlan.Filter      -> OptimizedPlan.Filter(lift(plan.input), plan.predicate)
        is LogicalPlan.Project     -> OptimizedPlan.Project(lift(plan.input), plan.columns)
        is LogicalPlan.Join        -> OptimizedPlan.Join(lift(plan.left), lift(plan.right), plan.kind, plan.condition)
        is LogicalPlan.Aggregate   -> OptimizedPlan.Aggregate(lift(plan.input), plan.groupBy, plan.aggregates)
        is LogicalPlan.Having      -> OptimizedPlan.Having(lift(plan.input), plan.predicate)
        is LogicalPlan.Sort        -> OptimizedPlan.Sort(lift(plan.input), plan.keys)
        is LogicalPlan.Limit       -> OptimizedPlan.Limit(lift(plan.input), plan.count, plan.offset)
        is LogicalPlan.Distinct    -> OptimizedPlan.Distinct(lift(plan.input))
        is LogicalPlan.Union       -> OptimizedPlan.Union(lift(plan.left), lift(plan.right), plan.all)
        is LogicalPlan.Insert      -> OptimizedPlan.Insert(plan.table, plan.columns, plan.values)
        is LogicalPlan.Update      -> OptimizedPlan.Update(plan.table, plan.assignments, plan.predicate)
        is LogicalPlan.Delete      -> OptimizedPlan.Delete(plan.table, plan.predicate)
        is LogicalPlan.CreateTable -> OptimizedPlan.CreateTable(plan.table, plan.ifNotExists, plan.columns)
        is LogicalPlan.DropTable   -> OptimizedPlan.DropTable(plan.table, plan.ifExists)
    }

    /** The ordered default pass pipeline. */
    fun defaultPasses(): List<Pass> = listOf(
        ConstantFoldingPass,
        PredicatePushdownPass,
        ProjectionPruningPass,
        DeadCodeEliminationPass,
        LimitPushdownPass
    )

    /** Lift then run all default passes. */
    fun optimize(plan: LogicalPlan): OptimizedPlan =
        optimizeWithPasses(plan, defaultPasses())

    /** Lift then run a custom pass list. */
    fun optimizeWithPasses(plan: LogicalPlan, passes: List<Pass>): OptimizedPlan {
        var result = lift(plan)
        for (pass in passes) {
            result = pass.apply(result)
        }
        return result
    }
}

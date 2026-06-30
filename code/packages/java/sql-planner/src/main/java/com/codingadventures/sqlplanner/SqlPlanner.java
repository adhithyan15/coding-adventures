package com.codingadventures.sqlplanner;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Objects;
import java.util.function.Function;

// SqlPlanner.java — logical query plan builder for SQL statements.
//
// Transforms a Statement into a LogicalPlan tree using an 8-step bottom-up
// SELECT pipeline:
//
//   Scan → Filter → Aggregate → Having → Project → Distinct → Sort → Limit
//
// No I/O, no database connections — pure in-memory data transformation.
// Errors are reported as PlanException subclasses (consistent with the
// Java sql-backend's exception-based error style).
//
// Usage:
//   var schema = new InMemorySchemaProvider(Map.of("users", List.of("id", "name", "age")));
//   var planner = new SqlPlanner(schema);
//   LogicalPlan plan = planner.plan(stmt);  // throws PlanException on error

public final class SqlPlanner {

    // ── Enumerations ──────────────────────────────────────────────────────────

    /** Binary infix SQL operators. */
    public enum BinaryOperator {
        EQ, NOT_EQ, LT, LTE, GT, GTE,
        AND, OR,
        ADD, SUB, MUL, DIV, MOD
    }

    /** Unary prefix SQL operators. */
    public enum UnaryOperator { NOT, NEG }

    /** Aggregate functions. */
    public enum AggFunction { COUNT, SUM, AVG, MIN, MAX }

    /** Sort direction for ORDER BY. */
    public enum SortDir { ASC, DESC }

    /** NULL ordering for ORDER BY. */
    public enum NullOrder { NULLS_FIRST, NULLS_LAST }

    /** JOIN type. */
    public enum JoinKind { INNER, LEFT, RIGHT, FULL, CROSS }

    // ── Aggregate argument ────────────────────────────────────────────────────

    /** Argument to an aggregate function — either * (COUNT(*)) or an expression. */
    public sealed interface AggArg permits AggArg.Star, AggArg.Expr {
        /** The * wildcard argument (COUNT(*)). */
        record Star() implements AggArg {}
        /** An expression argument (SUM(price), AVG(age), …). */
        record Expr(SqlExpr expression) implements AggArg {}
    }

    // ── Scalar expressions ────────────────────────────────────────────────────

    /**
     * A scalar expression in a SQL query plan.
     * All variants are records implementing this sealed interface.
     */
    public sealed interface SqlExpr
        permits SqlExpr.Literal, SqlExpr.Column, SqlExpr.BinaryOp, SqlExpr.UnaryOp,
                SqlExpr.FuncCall, SqlExpr.IsNull, SqlExpr.IsNotNull, SqlExpr.Between,
                SqlExpr.In, SqlExpr.NotIn, SqlExpr.Like, SqlExpr.NotLike,
                SqlExpr.Wildcard, SqlExpr.AggExpr {

        /** A SQL literal value (null, long, double, String, boolean, byte[]). */
        record Literal(Object value) implements SqlExpr {}

        /** A column reference, optionally qualified by table name or alias. */
        record Column(String table, String column) implements SqlExpr {}

        /** Binary infix operation. */
        record BinaryOp(BinaryOperator op, SqlExpr left, SqlExpr right) implements SqlExpr {}

        /** Unary prefix operation. */
        record UnaryOp(UnaryOperator op, SqlExpr operand) implements SqlExpr {}

        /** Scalar function call. */
        record FuncCall(String name, List<SqlExpr> args) implements SqlExpr {}

        /** IS NULL predicate. */
        record IsNull(SqlExpr operand) implements SqlExpr {}

        /** IS NOT NULL predicate. */
        record IsNotNull(SqlExpr operand) implements SqlExpr {}

        /** BETWEEN low AND high (inclusive). */
        record Between(SqlExpr value, SqlExpr low, SqlExpr high) implements SqlExpr {}

        /** value IN (items…). */
        record In(SqlExpr value, List<SqlExpr> items) implements SqlExpr {}

        /** value NOT IN (items…). */
        record NotIn(SqlExpr value, List<SqlExpr> items) implements SqlExpr {}

        /** String LIKE pattern. */
        record Like(SqlExpr value, String pattern) implements SqlExpr {}

        /** String NOT LIKE pattern. */
        record NotLike(SqlExpr value, String pattern) implements SqlExpr {}

        /** The bare * wildcard in SELECT *. */
        record Wildcard() implements SqlExpr {}

        /** Aggregate expression (COUNT, SUM, …). */
        record AggExpr(AggFunction func, AggArg arg, boolean distinct) implements SqlExpr {}
    }

    // ── Output column ─────────────────────────────────────────────────────────

    /** One item in a SELECT list. */
    public sealed interface OutputColumn
        permits OutputColumn.Star, OutputColumn.Expr {
        /** The bare * wildcard (SELECT *). */
        record Star() implements OutputColumn {}
        /** A named expression with an optional alias. */
        record Expr(SqlExpr expression, String alias) implements OutputColumn {}
    }

    // ── Structural types ──────────────────────────────────────────────────────

    /** A JOIN clause. */
    public record JoinClause(JoinKind kind, String table, String alias, SqlExpr on) {}

    /** DDL column definition. */
    public record ColumnDef(
        String name,
        String typeName,
        boolean notNull,
        boolean primaryKey,
        boolean unique,
        SqlExpr defaultValue) {}

    /** A SET assignment in UPDATE. */
    public record Assignment(String column, SqlExpr value) {}

    /** LIMIT / OFFSET pair. */
    public record LimitClause(Long count, Long offset) {}

    /** One key in an ORDER BY clause. */
    public record SortKey(SqlExpr keyExpr, SortDir direction, NullOrder nullOrder) {}

    /** One aggregate function applied during grouping. */
    public record AggregateItem(AggFunction func, AggArg arg, String alias, boolean distinct) {}

    // ── Table reference (from-clause entry) ───────────────────────────────────

    /** A (table, alias) pair in a FROM clause. */
    public record TableRef(String table, String alias) {}

    // ── Statement AST ─────────────────────────────────────────────────────────

    /**
     * A SQL statement.
     * The Java sql-parser is a stub, so callers build Statement instances directly.
     */
    public sealed interface Statement
        permits Statement.Select, Statement.Insert, Statement.Update,
                Statement.Delete, Statement.CreateTable, Statement.DropTable {

        record Select(
            boolean distinct,
            List<OutputColumn> columns,
            List<TableRef> from,
            List<JoinClause> joins,
            SqlExpr where,
            List<SqlExpr> groupBy,
            SqlExpr having,
            List<SortKey> orderBy,
            LimitClause limit) implements Statement {}

        record Insert(
            String table,
            List<String> columns,
            List<List<SqlExpr>> values) implements Statement {}

        record Update(
            String table,
            List<Assignment> assignments,
            SqlExpr where) implements Statement {}

        record Delete(String table, SqlExpr where) implements Statement {}

        record CreateTable(
            String table,
            boolean ifNotExists,
            List<ColumnDef> columns) implements Statement {}

        record DropTable(String table, boolean ifExists) implements Statement {}
    }

    // ── Logical plan nodes ────────────────────────────────────────────────────

    /** A node in the logical query plan tree. */
    public sealed interface LogicalPlan
        permits LogicalPlan.Scan, LogicalPlan.Filter, LogicalPlan.Project,
                LogicalPlan.Join, LogicalPlan.Aggregate, LogicalPlan.Having,
                LogicalPlan.Sort, LogicalPlan.Limit, LogicalPlan.Distinct,
                LogicalPlan.Union, LogicalPlan.Insert, LogicalPlan.Update,
                LogicalPlan.Delete, LogicalPlan.CreateTable, LogicalPlan.DropTable {

        record Scan(String table, String alias) implements LogicalPlan {}
        record Filter(LogicalPlan input, SqlExpr predicate) implements LogicalPlan {}
        record Project(LogicalPlan input, List<OutputColumn> columns) implements LogicalPlan {}
        record Join(LogicalPlan left, LogicalPlan right, JoinKind kind, SqlExpr condition) implements LogicalPlan {}
        record Aggregate(LogicalPlan input, List<SqlExpr> groupBy, List<AggregateItem> aggregates) implements LogicalPlan {}
        record Having(LogicalPlan input, SqlExpr predicate) implements LogicalPlan {}
        record Sort(LogicalPlan input, List<SortKey> keys) implements LogicalPlan {}
        record Limit(LogicalPlan input, Long count, Long offset) implements LogicalPlan {}
        record Distinct(LogicalPlan input) implements LogicalPlan {}
        record Union(LogicalPlan left, LogicalPlan right, boolean all) implements LogicalPlan {}
        record Insert(String table, List<String> columns, List<List<SqlExpr>> values) implements LogicalPlan {}
        record Update(String table, List<Assignment> assignments, SqlExpr predicate) implements LogicalPlan {}
        record Delete(String table, SqlExpr predicate) implements LogicalPlan {}
        record CreateTable(String table, boolean ifNotExists, List<ColumnDef> columns) implements LogicalPlan {}
        record DropTable(String table, boolean ifExists) implements LogicalPlan {}
    }

    // ── Plan exceptions ───────────────────────────────────────────────────────

    /** Base class for all planning errors. */
    public abstract static class PlanException extends RuntimeException {
        protected PlanException(String message) { super(message); }
    }

    /** A column name matches more than one table in scope. */
    public static final class AmbiguousColumnException extends PlanException {
        private final String column;
        private final List<String> tables;

        public AmbiguousColumnException(String column, List<String> tables) {
            super("Ambiguous column '" + column + "' — found in: " + String.join(", ", tables));
            this.column = column;
            this.tables = Collections.unmodifiableList(tables);
        }

        public String column()  { return column; }
        public List<String> tables() { return tables; }
    }

    /** A FROM / JOIN clause names a table not in the schema. */
    public static final class UnknownTableException extends PlanException {
        private final String table;

        public UnknownTableException(String table) {
            super("Unknown table '" + table + "'");
            this.table = table;
        }

        public String table() { return table; }
    }

    /** An expression references a column that cannot be resolved in scope. */
    public static final class UnknownColumnException extends PlanException {
        private final String qualifyingTable;
        private final String column;

        public UnknownColumnException(String qualifyingTable, String column) {
            super(qualifyingTable == null
                ? "Unknown column '" + column + "'"
                : "Unknown column '" + qualifyingTable + "." + column + "'");
            this.qualifyingTable = qualifyingTable;
            this.column = column;
        }

        public String qualifyingTable() { return qualifyingTable; }
        public String column()          { return column; }
    }

    /** An aggregate function appears in an illegal position. */
    public static final class InvalidAggregateException extends PlanException {
        public InvalidAggregateException(String message) { super(message); }
    }

    /** The statement type is not supported by this planner. */
    public static final class UnsupportedStatementException extends PlanException {
        public UnsupportedStatementException(String kind) { super("Unsupported statement: " + kind); }
    }

    // ── Schema provider ───────────────────────────────────────────────────────

    /** Returns the ordered list of column names for a table. */
    public interface SchemaProvider {
        /**
         * Returns column names for {@code table}.
         * @throws UnknownTableException if the table is not in the schema.
         */
        List<String> columns(String table);
    }

    /** An in-memory schema backed by a map of table → column list. */
    public static final class InMemorySchemaProvider implements SchemaProvider {
        private final Map<String, List<String>> tables;

        public InMemorySchemaProvider(Map<String, List<String>> tables) {
            this.tables = Map.copyOf(tables);
        }

        @Override
        public List<String> columns(String table) {
            var cols = tables.get(table);
            if (cols == null) throw new UnknownTableException(table);
            return cols;
        }
    }

    // ── Planner internals ─────────────────────────────────────────────────────

    private record ScopeEntry(String alias, String table, List<String> cols) {}

    private final SchemaProvider schema;

    public SqlPlanner(SchemaProvider schema) {
        this.schema = Objects.requireNonNull(schema);
    }

    /** Build scope from FROM + JOIN sources. */
    private List<ScopeEntry> buildScope(List<TableRef> from, List<JoinClause> joins) {
        var scope = new ArrayList<ScopeEntry>();
        for (var ref : from) {
            var cols = schema.columns(ref.table());   // throws UnknownTableException
            scope.add(new ScopeEntry(ref.alias() != null ? ref.alias() : ref.table(), ref.table(), cols));
        }
        for (var j : joins) {
            var cols = schema.columns(j.table());     // throws UnknownTableException
            scope.add(new ScopeEntry(j.alias() != null ? j.alias() : j.table(), j.table(), cols));
        }
        return scope;
    }

    /** Resolve a column reference against scope. */
    private static SqlExpr resolveColumn(List<ScopeEntry> scope, String tableOpt, String col) {
        if (tableOpt != null) {
            ScopeEntry entry = null;
            for (var e : scope) if (e.alias().equals(tableOpt)) { entry = e; break; }
            if (entry == null) throw new UnknownTableException(tableOpt);
            boolean found = false;
            for (var c : entry.cols()) if (c.equalsIgnoreCase(col)) { found = true; break; }
            if (!found) throw new UnknownColumnException(tableOpt, col);
            return new SqlExpr.Column(entry.alias(), col);
        } else {
            var matches = new ArrayList<ScopeEntry>();
            for (var e : scope) {
                for (var c : e.cols()) {
                    if (c.equalsIgnoreCase(col)) { matches.add(e); break; }
                }
            }
            if (matches.isEmpty()) throw new UnknownColumnException(null, col);
            if (matches.size() > 1) {
                var names = new ArrayList<String>();
                for (var m : matches) names.add(m.alias());
                throw new AmbiguousColumnException(col, names);
            }
            return new SqlExpr.Column(matches.get(0).alias(), col);
        }
    }

    /** Recursively resolve column references inside an expression. */
    private static SqlExpr resolveExpr(List<ScopeEntry> scope, SqlExpr expr) {
        return switch (expr) {
            case SqlExpr.Column(var tbl, var col) -> resolveColumn(scope, tbl, col);
            case SqlExpr.Literal ignored -> expr;
            case SqlExpr.Wildcard ignored -> expr;
            case SqlExpr.AggExpr ignored -> expr;
            case SqlExpr.BinaryOp(var op, var l, var r) ->
                new SqlExpr.BinaryOp(op, resolveExpr(scope, l), resolveExpr(scope, r));
            case SqlExpr.UnaryOp(var op, var operand) ->
                new SqlExpr.UnaryOp(op, resolveExpr(scope, operand));
            case SqlExpr.FuncCall(var name, var args) ->
                new SqlExpr.FuncCall(name, args.stream().map(a -> resolveExpr(scope, a)).toList());
            case SqlExpr.IsNull(var operand) ->
                new SqlExpr.IsNull(resolveExpr(scope, operand));
            case SqlExpr.IsNotNull(var operand) ->
                new SqlExpr.IsNotNull(resolveExpr(scope, operand));
            case SqlExpr.Between(var v, var lo, var hi) ->
                new SqlExpr.Between(resolveExpr(scope, v), resolveExpr(scope, lo), resolveExpr(scope, hi));
            case SqlExpr.In(var v, var items) ->
                new SqlExpr.In(resolveExpr(scope, v), items.stream().map(i -> resolveExpr(scope, i)).toList());
            case SqlExpr.NotIn(var v, var items) ->
                new SqlExpr.NotIn(resolveExpr(scope, v), items.stream().map(i -> resolveExpr(scope, i)).toList());
            case SqlExpr.Like(var v, var pattern) ->
                new SqlExpr.Like(resolveExpr(scope, v), pattern);
            case SqlExpr.NotLike(var v, var pattern) ->
                new SqlExpr.NotLike(resolveExpr(scope, v), pattern);
        };
    }

    /** Try to resolve; returns null instead of throwing UnknownColumnException. */
    private static SqlExpr tryResolveExpr(List<ScopeEntry> scope, SqlExpr expr) {
        try { return resolveExpr(scope, expr); }
        catch (UnknownColumnException e) { return null; }
    }

    /** Check whether any expression in a list contains an aggregate call. */
    private static boolean containsAgg(List<SqlExpr> exprs) {
        for (var e : exprs) if (containsAggExpr(e)) return true;
        return false;
    }

    private static boolean containsAggExpr(SqlExpr e) {
        return switch (e) {
            case SqlExpr.AggExpr ignored            -> true;
            case SqlExpr.BinaryOp(var op2, var l, var r) -> containsAggExpr(l) || containsAggExpr(r);
            case SqlExpr.UnaryOp(var op2, var op)   -> containsAggExpr(op);
            case SqlExpr.FuncCall(var nm, var args)  -> args.stream().anyMatch(SqlPlanner::containsAggExpr);
            case SqlExpr.IsNull(var op)              -> containsAggExpr(op);
            case SqlExpr.IsNotNull(var op)           -> containsAggExpr(op);
            case SqlExpr.Between(var v, var lo, var hi) -> containsAggExpr(v) || containsAggExpr(lo) || containsAggExpr(hi);
            case SqlExpr.In(var v, var items)        -> containsAggExpr(v) || items.stream().anyMatch(SqlPlanner::containsAggExpr);
            case SqlExpr.NotIn(var v, var items)     -> containsAggExpr(v) || items.stream().anyMatch(SqlPlanner::containsAggExpr);
            case SqlExpr.Like(var v, var pat2)       -> containsAggExpr(v);
            case SqlExpr.NotLike(var v, var pat2)    -> containsAggExpr(v);
            default -> false;
        };
    }

    /** Collect all AggExpr nodes from a list of expressions. */
    private static List<AggregateItem> collectAggregates(List<SqlExpr> exprs) {
        var found   = new ArrayList<AggregateItem>();
        int[] counter = {0};
        for (var e : exprs) walkAgg(e, found, counter);
        return found;
    }

    private static void walkAgg(SqlExpr e, List<AggregateItem> out, int[] counter) {
        switch (e) {
            case SqlExpr.AggExpr(var func, var arg, var distinct) ->
                out.add(new AggregateItem(func, arg, "_agg" + counter[0]++, distinct));
            case SqlExpr.BinaryOp(var op2, var l, var r) -> { walkAgg(l, out, counter); walkAgg(r, out, counter); }
            case SqlExpr.UnaryOp(var op2, var op)   -> walkAgg(op, out, counter);
            case SqlExpr.FuncCall(var nm, var args)  -> args.forEach(a -> walkAgg(a, out, counter));
            case SqlExpr.IsNull(var op)              -> walkAgg(op, out, counter);
            case SqlExpr.IsNotNull(var op)           -> walkAgg(op, out, counter);
            case SqlExpr.Between(var v, var lo, var hi) -> { walkAgg(v, out, counter); walkAgg(lo, out, counter); walkAgg(hi, out, counter); }
            case SqlExpr.In(var v, var items)        -> { walkAgg(v, out, counter); items.forEach(i -> walkAgg(i, out, counter)); }
            case SqlExpr.NotIn(var v, var items)     -> { walkAgg(v, out, counter); items.forEach(i -> walkAgg(i, out, counter)); }
            case SqlExpr.Like(var v, var pat2)       -> walkAgg(v, out, counter);
            case SqlExpr.NotLike(var v, var pat2)    -> walkAgg(v, out, counter);
            default -> {} // Literal, Column, Wildcard — no agg inside
        }
    }

    /** Build the FROM + JOIN plan tree left-associatively. */
    private LogicalPlan buildFromTree(List<TableRef> from, List<JoinClause> joins) {
        if (from.isEmpty()) throw new UnsupportedStatementException("SELECT without FROM");

        var first = from.get(0);
        schema.columns(first.table());   // validate table exists
        LogicalPlan plan = new LogicalPlan.Scan(first.table(), first.alias());

        for (int i = 1; i < from.size(); i++) {
            var ref = from.get(i);
            schema.columns(ref.table());  // validate table exists
            plan = new LogicalPlan.Join(plan, new LogicalPlan.Scan(ref.table(), ref.alias()), JoinKind.CROSS, null);
        }

        for (var j : joins) {
            schema.columns(j.table());    // validate table exists
            plan = new LogicalPlan.Join(plan, new LogicalPlan.Scan(j.table(), j.alias()), j.kind(), j.on());
        }

        return plan;
    }

    /** Plan a SELECT statement using the 8-step bottom-up pipeline. */
    private LogicalPlan planSelect(Statement.Select s) {
        var scope    = buildScope(s.from(), s.joins());
        var fromPlan = buildFromTree(s.from(), s.joins());

        // Step 1: WHERE → Filter
        LogicalPlan plan = s.where() != null
            ? new LogicalPlan.Filter(fromPlan, resolveExpr(scope, s.where()))
            : fromPlan;

        // Determine whether aggregation is required.
        var colExprs    = s.columns().stream().map(
            c -> c instanceof OutputColumn.Expr oe ? oe.expression() : new SqlExpr.Wildcard()).toList();
        var havingExprs = s.having() != null ? List.of(s.having()) : List.<SqlExpr>of();

        boolean needsAgg = !s.groupBy().isEmpty() || containsAgg(colExprs) || containsAgg(havingExprs);

        // Step 2: GROUP BY + Aggregate
        if (needsAgg) {
            var aggs       = collectAggregates(concatLists(colExprs, havingExprs));
            var groupBy    = s.groupBy().stream().map(e -> resolveExpr(scope, e)).toList();
            plan = new LogicalPlan.Aggregate(plan, groupBy, aggs);
        }

        // Step 3: HAVING
        if (s.having() != null) {
            var rHaving = tryResolveExpr(scope, s.having());
            if (rHaving == null) rHaving = s.having();
            plan = new LogicalPlan.Having(plan, rHaving);
        }

        // Step 4: DISTINCT
        if (s.distinct()) plan = new LogicalPlan.Distinct(plan);

        // Step 5: ORDER BY
        if (!s.orderBy().isEmpty()) {
            var keys = new ArrayList<SortKey>();
            for (var key : s.orderBy()) {
                var r = tryResolveExpr(scope, key.keyExpr());
                keys.add(new SortKey(r != null ? r : key.keyExpr(), key.direction(), key.nullOrder()));
            }
            plan = new LogicalPlan.Sort(plan, keys);
        }

        // Step 6: LIMIT / OFFSET
        if (s.limit() != null) {
            plan = new LogicalPlan.Limit(plan, s.limit().count(), s.limit().offset());
        }

        // Step 7: PROJECT (outermost — applied last so ORDER BY / LIMIT can see raw column refs)
        var projCols = new ArrayList<OutputColumn>();
        for (var c : s.columns()) {
            if (c instanceof OutputColumn.Star) {
                projCols.add(new OutputColumn.Star());
            } else {
                var oe = (OutputColumn.Expr) c;
                SqlExpr resolved = needsAgg
                    ? (tryResolveExpr(scope, oe.expression()) != null
                        ? tryResolveExpr(scope, oe.expression())
                        : oe.expression())
                    : resolveExpr(scope, oe.expression());
                projCols.add(new OutputColumn.Expr(resolved, oe.alias()));
            }
        }
        plan = new LogicalPlan.Project(plan, projCols);

        return plan;
    }

    @SafeVarargs
    private static <T> List<T> concatLists(List<T>... lists) {
        var out = new ArrayList<T>();
        for (var l : lists) out.addAll(l);
        return out;
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /**
     * Transform a single statement into a logical plan.
     * @throws PlanException on any planning error.
     */
    public LogicalPlan plan(Statement stmt) {
        return switch (stmt) {
            case Statement.Select s    -> planSelect(s);
            case Statement.Insert i    -> {
                schema.columns(i.table());  // validate
                yield new LogicalPlan.Insert(i.table(), i.columns(), i.values());
            }
            case Statement.Update u    -> {
                schema.columns(u.table());  // validate
                yield new LogicalPlan.Update(u.table(), u.assignments(), u.where());
            }
            case Statement.Delete d    -> {
                schema.columns(d.table());  // validate
                yield new LogicalPlan.Delete(d.table(), d.where());
            }
            case Statement.CreateTable ct ->
                new LogicalPlan.CreateTable(ct.table(), ct.ifNotExists(), ct.columns());
            case Statement.DropTable dt ->
                new LogicalPlan.DropTable(dt.table(), dt.ifExists());
        };
    }

    /**
     * Plan every statement in the list.
     * @throws PlanException on the first error encountered.
     */
    public List<LogicalPlan> planAll(List<Statement> stmts) {
        var out = new ArrayList<LogicalPlan>(stmts.size());
        for (var s : stmts) out.add(plan(s));
        return Collections.unmodifiableList(out);
    }
}

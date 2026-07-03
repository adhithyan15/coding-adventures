package com.codingadventures.minisqlite;

// MiniSqliteConnection.java — Level 1 mini-sqlite connection.
//
// This class is the main public entry point for the Level 1 graduation.
// It wires the full five-package SQL pipeline together:
//
//   SQL text
//     │
//     ▼  SqlTextParser.parse(sql)        [in this package]
//   SqlPlanner.Statement
//     │
//     ▼  SqlPlanner.plan(stmt)           [sql-planner]
//   SqlPlanner.LogicalPlan
//     │
//     ▼  SqlOptimizer.optimize(plan)     [sql-optimizer]
//   SqlOptimizer.OptimizedPlan
//     │
//     ▼  SqlCodegen.compile(optimized)   [sql-codegen]
//   SqlCodegen.Program
//     │
//     ▼  SqlVm.execute(program, backend) [sql-vm]
//   SqlVm.QueryResult
//
// The public API follows the same DB-API-2.0-inspired shape as MiniSqlite
// (Level 0) so tests for the new connection can reuse the same idioms:
//
//   var conn = MiniSqliteConnection.connect(":memory:");
//   conn.execute("CREATE TABLE users (id INTEGER, name TEXT)");
//   conn.execute("INSERT INTO users VALUES (?, ?)", List.of(1, "Alice"));
//   var cur  = conn.execute("SELECT name FROM users");
//   List<List<Object>> rows = cur.fetchall();
//
// Parameter binding
// ─────────────────
// Qmark (?) placeholders in the SQL text are substituted with literal
// SQL representations before the text is parsed.  This matches the Level 0
// behaviour and avoids having to thread typed parameters through the planner.
//
// Transaction snapshots
// ─────────────────────
// When autocommit is false (the default), the first mutation after a
// commit/rollback takes a deep copy of the InMemoryBackend state.  A
// subsequent rollback() restores that snapshot.
//
// Error mapping
// ─────────────
// All pipeline exceptions (PlanException, IllegalArgumentException, etc.) are
// caught and rethrown as MiniSqliteException with an appropriate kind.

import com.codingadventures.sqlbackend.SqlBackend;
import com.codingadventures.sqlbackend.SqlBackend.ColumnDef;
import com.codingadventures.sqlbackend.SqlBackend.InMemoryBackend;
import com.codingadventures.sqlbackend.SqlBackend.Row;
import com.codingadventures.sqlbackend.SqlBackend.TransactionHandle;
import com.codingadventures.sqlcodegen.SqlCodegen;
import com.codingadventures.sqloptimizer.SqlOptimizer;
import com.codingadventures.sqlplanner.SqlPlanner;
import com.codingadventures.sqlplanner.SqlPlanner.Statement;
import com.codingadventures.sqlvm.SqlVm;
import com.codingadventures.sqlvm.SqlVm.QueryResult;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Objects;
import java.util.function.UnaryOperator;

/**
 * Level-1 mini-sqlite connection that runs every SQL statement through the
 * full five-package pipeline:
 * {@link SqlTextParser} → {@link SqlPlanner} → {@link SqlOptimizer}
 * → {@link SqlCodegen} → {@link SqlVm}.
 *
 * <p>Create a connection via {@link #connect(String)}.  The only supported
 * database name at Level 1 is {@code ":memory:"}.
 */
public final class MiniSqliteConnection {

    // ── Public constants (DB-API-2.0 style) ──────────────────────────────────

    /** DB-API 2.0 API level identifier. */
    public static final String API_LEVEL = "2.0";
    /** Thread safety level (1 = connection-level). */
    public static final int THREADSAFETY = 1;
    /** Parameter style (qmark = ?). */
    public static final String PARAMSTYLE = "qmark";

    // ── Exception ─────────────────────────────────────────────────────────────

    /**
     * Exception type used by mini-sqlite Level 1.
     *
     * <p>The {@code kind} mirrors the Python DB-API-2.0 exception hierarchy:
     * {@code ProgrammingError}, {@code OperationalError}, {@code InterfaceError},
     * {@code NotSupportedError}.
     */
    public static final class MiniSqliteException extends RuntimeException {
        private final String kind;

        public MiniSqliteException(String kind, String message) {
            super(message);
            this.kind = kind;
        }

        /** The DB-API error category. */
        public String kind() { return kind; }
    }

    // ── Factory method ────────────────────────────────────────────────────────

    /**
     * Open a connection to an in-memory database.
     *
     * @param database must be {@code ":memory:"}; any other value throws
     *                 {@link MiniSqliteException} with kind {@code NotSupportedError}
     * @return a new {@link Connection}
     */
    public static Connection connect(String database) {
        return connect(database, Options.defaults());
    }

    /**
     * Open a connection with explicit options.
     *
     * @param database must be {@code ":memory:"}
     * @param options  connection options (null → {@link Options#defaults()})
     * @return a new {@link Connection}
     */
    public static Connection connect(String database, Options options) {
        if (!":memory:".equals(database)) {
            throw new MiniSqliteException(
                "NotSupportedError",
                "Java mini-sqlite supports only :memory: at Level 1");
        }
        return new Connection(options == null ? Options.defaults() : options);
    }

    // ── Options ───────────────────────────────────────────────────────────────

    /** Connection configuration. */
    public record Options(boolean autocommit) {
        /** Default options: autocommit disabled. */
        public static Options defaults() { return new Options(false); }
    }

    // ── Column metadata ───────────────────────────────────────────────────────

    /** Column descriptor returned by {@link Cursor#description()}. */
    public record Column(String name) {}

    // ── Connection ────────────────────────────────────────────────────────────

    /**
     * A single database session backed by an {@link InMemoryBackend}.
     *
     * <p>Each {@code Connection} owns one backend instance and an optional
     * snapshot for rollback support.  It is not thread-safe (THREADSAFETY = 1).
     */
    public static final class Connection implements AutoCloseable {

        private final InMemoryBackend backend = new InMemoryBackend();
        private final boolean autocommit;
        // The active transaction handle (null if no transaction is in progress).
        private TransactionHandle txHandle;
        private boolean closed;

        // The planner needs the schema on every plan() call.
        // We derive it from the live backend so DDL changes are visible immediately.
        private SqlPlanner.SchemaProvider schemaProvider() {
            return table -> backend.columns(table).stream()
                                   .map(ColumnDef::name)
                                   .toList();
        }

        private Connection(Options options) {
            this.autocommit = options.autocommit();
        }

        // ── Cursor factory ─────────────────────────────────────────────────

        /**
         * Create a new {@link Cursor} for this connection.
         *
         * @throws MiniSqliteException if the connection is closed
         */
        public Cursor cursor() {
            assertOpen();
            return new Cursor(this);
        }

        // ── Direct execute helpers ─────────────────────────────────────────

        /** Execute {@code sql} with no parameters and return a Cursor. */
        public Cursor execute(String sql) {
            return execute(sql, List.of());
        }

        /** Execute {@code sql} with qmark-bound {@code params} and return a Cursor. */
        public Cursor execute(String sql, List<?> params) {
            return cursor().execute(sql, params);
        }

        /**
         * Execute {@code sql} once for each parameter list in {@code paramsSeq}.
         *
         * @return the Cursor of the last execution
         */
        public Cursor executemany(String sql, List<List<?>> paramsSeq) {
            return cursor().executemany(sql, paramsSeq);
        }

        // ── Transaction control ────────────────────────────────────────────

        /**
         * Commit the current transaction.
         */
        public void commit() {
            assertOpen();
            if (txHandle != null) {
                backend.commit(txHandle);
                txHandle = null;
            }
        }

        /**
         * Roll back to the beginning of the current transaction.
         * If no transaction has been started, this is a no-op.
         */
        public void rollback() {
            assertOpen();
            if (txHandle != null) {
                backend.rollback(txHandle);
                txHandle = null;
            }
        }

        /** Close the connection.  Any uncommitted changes are rolled back. */
        @Override
        public void close() {
            if (closed) return;
            if (txHandle != null) {
                backend.rollback(txHandle);
                txHandle = null;
            }
            closed = true;
        }

        // ── Internal helpers ───────────────────────────────────────────────

        private void assertOpen() {
            if (closed) {
                throw new MiniSqliteException("ProgrammingError", "connection is closed");
            }
        }

        /**
         * Begin a transaction if we are in non-autocommit mode and no transaction
         * is currently active.
         */
        private void ensureTransaction() {
            if (!autocommit && txHandle == null) {
                txHandle = backend.beginTransaction();
            }
        }

        /**
         * Bind parameters, parse, plan, optimise, compile, and execute the SQL.
         *
         * <p>This is the core Level-1 execution path.  Errors from any pipeline
         * stage are mapped to {@link MiniSqliteException}.
         */
        QueryResult executeSql(String sql, List<?> params) {
            assertOpen();
            String bound = bindParameters(sql, params == null ? List.of() : params);

            // Short-circuit for transaction-control statements that don't go
            // through the planner/VM.
            String firstKw = firstKeyword(bound);
            switch (firstKw) {
                case "BEGIN" -> {
                    ensureTransaction();
                    return emptyResult(0);
                }
                case "COMMIT" -> {
                    commit();
                    return emptyResult(0);
                }
                case "ROLLBACK" -> {
                    rollback();
                    return emptyResult(0);
                }
            }

            // Open a transaction before any mutation.
            if (isMutation(firstKw)) ensureTransaction();

            // ── Full pipeline ──────────────────────────────────────────────
            try {
                Statement stmt = SqlTextParser.parse(bound);

                // INSERT without an explicit column list: expand columns from
                // the backend schema so the planner and codegen receive correct
                // column names and emit InsertRow with a non-empty column list.
                stmt = expandInsertColumns(stmt);

                // SELECT *: expand the Star wildcard to explicit column references
                // for each table in the FROM clause.  SqlCodegen skips Star output
                // columns ("the planner should have resolved them"), so we resolve
                // them here before planning.
                stmt = expandSelectStar(stmt);

                SqlPlanner planner = new SqlPlanner(schemaProvider());
                SqlPlanner.LogicalPlan logicalPlan = planner.plan(stmt);

                // SqlPlanner puts Project OUTERMOST (Project wraps Sort/Limit/Distinct).
                // SqlCodegen.compilePlan() peels Sort/Limit/Distinct from the TOP, so
                // it needs Sort/Limit/Distinct to be above Project.  We normalize the
                // plan here to lift Sort/Limit/Distinct above Project before handing
                // off to the codegen.  Sort key columns not in the SELECT list are
                // injected into the Project as hidden trailing columns; the count of
                // such extras is returned so we can strip them from the result.
                NormalizedPlan np = normalizePlan(logicalPlan);

                // compile() internally calls SqlOptimizer.optimize(); we pass
                // the normalized LogicalPlan directly.
                SqlCodegen.Program program = SqlCodegen.compile(np.plan());

                QueryResult raw = SqlVm.execute(program, backend);

                // Strip any extra (hidden sort-key) columns from the result.
                QueryResult result = raw;
                if (np.extraColumns() > 0 && !raw.columns().isEmpty()) {
                    int keep = raw.columns().size() - np.extraColumns();
                    List<String> cols = raw.columns().subList(0, keep);
                    List<List<Object>> rows = raw.rows().stream()
                        .map(r -> r.subList(0, keep))
                        .toList();
                    result = new QueryResult(cols, rows, raw.rowsAffected());
                }

                // Post-filter rows using the HAVING predicate if one was extracted.
                if (np.havingPred() != null) {
                    result = applyHavingFilter(result, np.havingPred(), np.projCols());
                }
                return result;

            } catch (MiniSqliteException ex) {
                // Re-throw our own exceptions unchanged.
                throw ex;
            } catch (SqlPlanner.PlanException ex) {
                throw new MiniSqliteException("OperationalError", ex.getMessage());
            } catch (SqlBackend.BackendError ex) {
                throw new MiniSqliteException("OperationalError", ex.getMessage());
            } catch (IllegalArgumentException | IllegalStateException ex) {
                throw new MiniSqliteException("OperationalError", ex.getMessage());
            } catch (RuntimeException ex) {
                throw new MiniSqliteException("OperationalError",
                    ex.getClass().getSimpleName() + ": " + ex.getMessage());
            }
        }

        private static QueryResult emptyResult(int rowsAffected) {
            return new QueryResult(List.of(), List.of(), rowsAffected);
        }

        // SELECT *: expand Star wildcard to explicit column references.
        //
        // SqlCodegen.emitProjectColumns() silently skips OutputColumn.Star nodes
        // ("the planner should have resolved them"), resulting in no columns emitted
        // and an empty result schema.  We expand Star here by querying each FROM-
        // clause table's schema and injecting OutputColumn.Expr entries.
        private Statement expandSelectStar(Statement stmt) {
            if (!(stmt instanceof Statement.Select sel)) return stmt;
            // Only act if there's at least one Star in the column list.
            boolean hasStar = sel.columns().stream()
                .anyMatch(c -> c instanceof SqlPlanner.OutputColumn.Star);
            if (!hasStar) return stmt;

            var expanded = new ArrayList<SqlPlanner.OutputColumn>();
            for (var col : sel.columns()) {
                if (!(col instanceof SqlPlanner.OutputColumn.Star)) {
                    expanded.add(col);
                    continue;
                }
                // Expand * : for every table/alias in FROM + JOINs, emit all columns.
                for (var tr : sel.from()) {
                    String alias = tr.alias() != null ? tr.alias() : tr.table();
                    try {
                        for (var cd : backend.columns(tr.table())) {
                            expanded.add(new SqlPlanner.OutputColumn.Expr(
                                new SqlPlanner.SqlExpr.Column(alias, cd.name()), null));
                        }
                    } catch (SqlBackend.BackendError ignored) { /* table not found */ }
                }
                for (var jc : sel.joins()) {
                    String alias = jc.alias() != null ? jc.alias() : jc.table();
                    try {
                        for (var cd : backend.columns(jc.table())) {
                            expanded.add(new SqlPlanner.OutputColumn.Expr(
                                new SqlPlanner.SqlExpr.Column(alias, cd.name()), null));
                        }
                    } catch (SqlBackend.BackendError ignored) { /* table not found */ }
                }
            }
            return new Statement.Select(sel.distinct(), expanded, sel.from(), sel.joins(),
                sel.where(), sel.groupBy(), sel.having(), sel.orderBy(), sel.limit());
        }

        // When INSERT INTO table VALUES (...) has no explicit column list, the
        // planner and codegen receive an empty column list and the InsertRow
        // instruction pops zero values from the stack — leaving data unset.
        // We resolve this by asking the live backend for the ordered schema
        // columns and injecting them into the Statement before planning.
        private Statement expandInsertColumns(Statement stmt) {
            if (!(stmt instanceof Statement.Insert ins)) return stmt;
            if (!ins.columns().isEmpty()) return stmt; // already has explicit columns
            try {
                List<String> cols = backend.columns(ins.table())
                                           .stream()
                                           .map(ColumnDef::name)
                                           .toList();
                if (cols.isEmpty()) return stmt; // table has no columns (shouldn't happen)
                return new Statement.Insert(ins.table(), cols, ins.values());
            } catch (SqlBackend.BackendError ex) {
                // Table doesn't exist yet — let the planner fail with a better message.
                return stmt;
            }
        }

        private static boolean isMutation(String keyword) {
            return switch (keyword) {
                case "INSERT", "UPDATE", "DELETE", "CREATE", "DROP", "ALTER" -> true;
                default -> false;
            };
        }
    }

    // ── Cursor ────────────────────────────────────────────────────────────────

    /**
     * A stateful cursor over a result set.
     *
     * <p>Execute one or more statements via {@link #execute}/{@link #executemany},
     * then fetch rows with {@link #fetchone()}, {@link #fetchmany()}, or
     * {@link #fetchall()}.
     */
    public static final class Cursor implements AutoCloseable {

        private final Connection connection;
        private List<Column>        description  = List.of();
        private int                 rowcount     = -1;
        private Object              lastrowid    = null;
        private int                 arraysize    = 1;
        private List<List<Object>>  rows         = List.of();
        private int                 offset       = 0;
        private boolean             closed       = false;

        private Cursor(Connection connection) {
            this.connection = connection;
        }

        // ── Execution ──────────────────────────────────────────────────────

        /** Execute {@code sql} with no parameters. */
        public Cursor execute(String sql) {
            return execute(sql, List.of());
        }

        /** Execute {@code sql} with qmark-bound {@code params}. */
        public Cursor execute(String sql, List<?> params) {
            if (closed) throw new MiniSqliteException("ProgrammingError", "cursor is closed");
            QueryResult result = connection.executeSql(sql, params);
            rows        = result.rows();
            offset      = 0;
            rowcount    = result.rowsAffected();
            description = result.columns().stream().map(Column::new).toList();
            return this;
        }

        /** Execute {@code sql} for each parameter list in {@code paramsSeq}. */
        public Cursor executemany(String sql, List<List<?>> paramsSeq) {
            int total = 0;
            for (List<?> p : paramsSeq == null ? List.<List<?>>of() : paramsSeq) {
                execute(sql, p);
                if (rowcount > 0) total += rowcount;
            }
            if (paramsSeq != null && !paramsSeq.isEmpty()) rowcount = total;
            return this;
        }

        // ── Fetch methods ──────────────────────────────────────────────────

        /**
         * Return the next row, or {@code null} if no more rows remain.
         */
        public List<Object> fetchone() {
            if (closed || offset >= rows.size()) return null;
            return rows.get(offset++);
        }

        /**
         * Return up to {@link #arraysize()} rows.
         */
        public List<List<Object>> fetchmany() {
            return fetchmany(arraysize);
        }

        /**
         * Return up to {@code size} rows.
         */
        public List<List<Object>> fetchmany(int size) {
            if (closed) return List.of();
            List<List<Object>> out = new ArrayList<>();
            for (int i = 0; i < size; i++) {
                List<Object> row = fetchone();
                if (row == null) break;
                out.add(row);
            }
            return out;
        }

        /**
         * Return all remaining rows.
         */
        public List<List<Object>> fetchall() {
            if (closed) return List.of();
            List<List<Object>> out = new ArrayList<>();
            while (true) {
                List<Object> row = fetchone();
                if (row == null) break;
                out.add(row);
            }
            return out;
        }

        // ── Close ──────────────────────────────────────────────────────────

        @Override
        public void close() {
            closed      = true;
            rows        = List.of();
            description = List.of();
        }

        // ── Metadata ───────────────────────────────────────────────────────

        /** Column descriptors for the most recent SELECT. */
        public List<Column>  description() { return description; }
        /** Number of rows affected by the last DML statement; -1 for SELECT. */
        public int           rowcount()    { return rowcount;    }
        /** The rowid of the last INSERT (not yet populated at Level 1). */
        public Object        lastrowid()   { return lastrowid;   }
        /** The default fetch size for {@link #fetchmany()}. */
        public int           arraysize()   { return arraysize;   }
        /** Set the default fetch size. */
        public void          arraysize(int n) { this.arraysize = n; }
    }

    // ── Parameter binding ─────────────────────────────────────────────────────
    //
    // We substitute ? placeholders with their SQL literal representations
    // BEFORE parsing, keeping the parser stateless and simple.

    /**
     * Replace each {@code ?} placeholder in {@code sql} with the SQL literal
     * for the corresponding element of {@code params}.
     */
    static String bindParameters(String sql, List<?> params) {
        StringBuilder out = new StringBuilder();
        int index = 0;
        int i = 0;
        while (i < sql.length()) {
            char c = sql.charAt(i);
            if (c == '\'' || c == '"') {
                int next = readQuoted(sql, i, c);
                out.append(sql, i, next);
                i = next;
            } else if (c == '-' && i + 1 < sql.length() && sql.charAt(i + 1) == '-') {
                int next = i + 2;
                while (next < sql.length() && sql.charAt(next) != '\n') next++;
                out.append(sql, i, next);
                i = next;
            } else if (c == '/' && i + 1 < sql.length() && sql.charAt(i + 1) == '*') {
                int next = i + 2;
                while (next + 1 < sql.length()
                       && !(sql.charAt(next) == '*' && sql.charAt(next + 1) == '/')) next++;
                next = Math.min(next + 2, sql.length());
                out.append(sql, i, next);
                i = next;
            } else if (c == '?') {
                if (index >= params.size()) {
                    throw new MiniSqliteException("ProgrammingError",
                        "not enough parameters for SQL statement");
                }
                out.append(toSqlLiteral(params.get(index++)));
                i++;
            } else {
                out.append(c);
                i++;
            }
        }
        if (index < params.size()) {
            throw new MiniSqliteException("ProgrammingError",
                "too many parameters for SQL statement");
        }
        return out.toString();
    }

    private static int readQuoted(String sql, int index, char quote) {
        int i = index + 1;
        while (i < sql.length()) {
            char c = sql.charAt(i);
            if (c == quote) {
                if (i + 1 < sql.length() && sql.charAt(i + 1) == quote) {
                    i += 2;
                } else {
                    return i + 1;
                }
            } else {
                i++;
            }
        }
        return sql.length();
    }

    private static String toSqlLiteral(Object value) {
        if (value == null)                   return "NULL";
        if (value instanceof Boolean b)      return b ? "TRUE" : "FALSE";
        if (value instanceof Number)         return value.toString();
        if (value instanceof CharSequence)   return "'" + Objects.toString(value).replace("'", "''") + "'";
        throw new MiniSqliteException("ProgrammingError",
            "unsupported parameter type: " + value.getClass().getName());
    }

    // ── Plan normalisation ────────────────────────────────────────────────────
    //
    // SqlPlanner.planSelect() wraps the plan in this order (innermost → outermost):
    //
    //   Scan → Filter → Aggregate → Having → Distinct → Sort → Limit → Project
    //
    // This is semantically correct: Project is the last (outermost) node because
    // ORDER BY and LIMIT must be evaluated before column aliasing is applied.
    //
    // However, SqlCodegen.compilePlan() expects Sort/Limit/Distinct to sit ABOVE
    // Project in the plan tree: it peels those post-processing wrappers off the
    // top in a while-loop, then compiles the inner Project core.  If Project is
    // outermost and Sort sits inside it, codegen calls compileScanBody(Sort, ...)
    // which throws UnsupportedOperationException.
    //
    // The fix: rotate the tree so that Sort/Limit/Distinct sit above Project:
    //
    //   Before: Project([a])(Sort([b])(core))
    //   After:  Sort([b])(Project([a, b_hidden])(core))
    //
    // When Sort key columns (e.g. "b") are not in the projected output, we inject
    // them as extra OutputColumn.Expr entries at the end of the Project so the
    // Sort post-op can find them in the result schema.  The caller strips the
    // extra columns from the final QueryResult.
    //
    // The return value is a NormalizedPlan record carrying both the rewritten plan
    // and the count of extra (hidden) columns appended to the project output.

    /**
     * Result of plan normalization.
     *
     * @param plan         the rewritten LogicalPlan, ready for SqlCodegen.compile()
     * @param extraColumns count of hidden sort-key columns appended to Project output;
     *                     the caller strips them from the final QueryResult
     * @param havingPred   Having predicate (may be null) to post-filter aggregate result
     *                     rows; null when HAVING is absent or fully handled by codegen
     * @param projCols     project output columns used to map AggExpr → result column idx
     *                     when evaluating havingPred; null when havingPred is null
     */
    private record NormalizedPlan(
        SqlPlanner.LogicalPlan plan,
        int extraColumns,
        SqlPlanner.SqlExpr havingPred,
        List<SqlPlanner.OutputColumn> projCols) {

        NormalizedPlan(SqlPlanner.LogicalPlan plan, int extraColumns) {
            this(plan, extraColumns, null, null);
        }
    }

    private static NormalizedPlan normalizePlan(SqlPlanner.LogicalPlan plan) {
        // Only act when Project is the outermost node.
        if (!(plan instanceof SqlPlanner.LogicalPlan.Project p)) {
            return new NormalizedPlan(plan, 0);
        }

        // Collect Sort / Limit / Distinct wrappers sitting directly under Project
        // (between Project and the first non-wrapper node).
        // We use addFirst so that when we replay the deque front-to-back we apply
        // the innermost wrapper first, preserving the original nesting order.
        var wrappers = new ArrayDeque<UnaryOperator<SqlPlanner.LogicalPlan>>();
        var sortKeys = new ArrayList<SqlPlanner.SortKey>(); // all sort keys found
        SqlPlanner.LogicalPlan inner = p.input();

        while (true) {
            if (inner instanceof SqlPlanner.LogicalPlan.Sort s) {
                final var keys = s.keys();
                sortKeys.addAll(keys);
                wrappers.addLast(x -> new SqlPlanner.LogicalPlan.Sort(x, keys));
                inner = s.input();
            } else if (inner instanceof SqlPlanner.LogicalPlan.Limit lim) {
                final Long count = lim.count();
                final Long offset = lim.offset();
                wrappers.addLast(x -> new SqlPlanner.LogicalPlan.Limit(x, count, offset));
                inner = lim.input();
            } else if (inner instanceof SqlPlanner.LogicalPlan.Distinct d) {
                wrappers.addLast(x -> new SqlPlanner.LogicalPlan.Distinct(x));
                inner = d.input();
            } else {
                break;
            }
        }

        if (wrappers.isEmpty()) return new NormalizedPlan(plan, 0);

        // ── HAVING stripping ──────────────────────────────────────────────────
        //
        // SqlCodegen.compileCore handles Project(Aggregate(...)) as a two-phase
        // aggregate build, but ONLY when Aggregate is the direct child of Project.
        // If Having wraps the Aggregate (Project(Having(Aggregate(core)))),
        // compileCore falls through to compileScanBody(Having(Aggregate(core)))
        // which strips Having and then throws "Unsupported: Aggregate".
        //
        // Fix: detect Having(Aggregate(core)) as the inner node, strip the Having,
        // and record its predicate for post-execution filtering.

        SqlPlanner.SqlExpr havingPred = null;
        if (inner instanceof SqlPlanner.LogicalPlan.Having h
                && h.input() instanceof SqlPlanner.LogicalPlan.Aggregate) {
            havingPred = h.predicate();
            inner      = h.input(); // Project now sees Aggregate directly
        }

        // Determine which sort-key column names are already in the project output.
        // For each sort key that references a column not already projected, we
        // append an extra OutputColumn.Expr so the SortResult instruction can
        // find it by name in the result schema.
        var projectedNames = new HashSet<String>();
        for (var col : p.columns()) {
            if (col instanceof SqlPlanner.OutputColumn.Expr e) {
                if (e.alias() != null) {
                    projectedNames.add(e.alias());
                } else if (e.expression() instanceof SqlPlanner.SqlExpr.Column c) {
                    projectedNames.add(c.column());
                }
            } else {
                // Star column — all columns are projected, sort will find its key
                projectedNames.add("*");
            }
        }

        var augmentedCols = new ArrayList<>(p.columns());
        int extraCount = 0;
        for (var sk : sortKeys) {
            if (sk.keyExpr() instanceof SqlPlanner.SqlExpr.Column c) {
                String name = c.column();
                if (!projectedNames.contains(name) && !projectedNames.contains("*")) {
                    // Add this column as a hidden extra projection so Sort can use it.
                    augmentedCols.add(new SqlPlanner.OutputColumn.Expr(
                        new SqlPlanner.SqlExpr.Column(c.table(), name), null));
                    projectedNames.add(name); // avoid duplicates for multi-key sorts
                    extraCount++;
                }
            }
        }

        // Rebuild: Project (possibly augmented) wraps the core, then wrappers above.
        SqlPlanner.LogicalPlan result =
            new SqlPlanner.LogicalPlan.Project(inner, augmentedCols);
        for (var wrapper : wrappers) {
            result = wrapper.apply(result);
        }

        // Pass havingPred and project columns so executeSql can post-filter.
        return new NormalizedPlan(result, extraCount,
            havingPred, havingPred != null ? List.copyOf(p.columns()) : null);
    }

    // ── HAVING post-filter ────────────────────────────────────────────────────
    //
    // SqlCodegen.compileProjectAggregate does not emit a HAVING filter in
    // Phase 2.  When normalizePlan strips Having from the plan tree (to allow
    // Project to see Aggregate directly), it saves the Having predicate here
    // so we can apply it as a post-execution row filter.
    //
    // The evaluator handles the subset of SqlExpr nodes that appear in typical
    // HAVING clauses: BinaryOp (arithmetic/comparison/logical), Literal, Column
    // (references projected column by name), and AggExpr (matched to the project
    // output column whose expression is the same aggregate call).

    private static QueryResult applyHavingFilter(
            QueryResult qr,
            SqlPlanner.SqlExpr pred,
            List<SqlPlanner.OutputColumn> projCols) {
        List<String> colNames = qr.columns();
        List<List<Object>> kept = new ArrayList<>();
        for (var row : qr.rows()) {
            Object val = evalHavingExpr(pred, row, colNames, projCols);
            if (Boolean.TRUE.equals(val) || (val instanceof Number n && n.doubleValue() != 0)) {
                kept.add(row);
            }
        }
        return new QueryResult(colNames, kept, qr.rowsAffected());
    }

    /**
     * Evaluate a scalar HAVING predicate against one result row.
     *
     * <p>{@code colNames} is the ordered list of projected column names (matching
     * the row's value positions).  {@code projCols} maps aggregate expressions to
     * those column positions.
     */
    @SuppressWarnings("unchecked")
    private static Object evalHavingExpr(
            SqlPlanner.SqlExpr expr,
            List<Object> row,
            List<String> colNames,
            List<SqlPlanner.OutputColumn> projCols) {
        return switch (expr) {

            // ── Literal ───────────────────────────────────────────────────────
            case SqlPlanner.SqlExpr.Literal(var v) -> v;

            // ── Column ref: look up by projected column name ──────────────────
            case SqlPlanner.SqlExpr.Column(var tbl, var col) -> {
                int idx = colNames.indexOf(col);
                yield idx >= 0 ? row.get(idx) : null;
            }

            // ── AggExpr: find the matching project output column ──────────────
            //
            // e.g. HAVING SUM(amount) > 150 — AggExpr(SUM, amount) must map to
            // the column whose expression is the same SUM(amount) call.
            case SqlPlanner.SqlExpr.AggExpr agg -> {
                // Walk project columns to find the one whose expression matches.
                for (int i = 0; i < projCols.size(); i++) {
                    if (projCols.get(i) instanceof SqlPlanner.OutputColumn.Expr oe) {
                        if (aggExprStructurallyEquals(agg, oe.expression())) {
                            yield i < row.size() ? row.get(i) : null;
                        }
                    }
                }
                // Fallback: try matching by function + alias name.
                for (int i = 0; i < colNames.size(); i++) {
                    String name = colNames.get(i);
                    if (name != null && name.startsWith(
                            agg.func().name().toLowerCase(Locale.ROOT))) {
                        yield i < row.size() ? row.get(i) : null;
                    }
                }
                yield null;
            }

            // ── Binary operations ─────────────────────────────────────────────
            case SqlPlanner.SqlExpr.BinaryOp(var op, var left, var right) -> {
                Object lv = evalHavingExpr(left,  row, colNames, projCols);
                Object rv = evalHavingExpr(right, row, colNames, projCols);
                yield evalBinaryOp(op, lv, rv);
            }

            // ── Unary NOT ─────────────────────────────────────────────────────
            case SqlPlanner.SqlExpr.UnaryOp(var op, var operand) -> {
                Object v = evalHavingExpr(operand, row, colNames, projCols);
                if (op == SqlPlanner.UnaryOperator.NOT) {
                    if (v == null) yield null;
                    yield !(Boolean.TRUE.equals(v) || (v instanceof Number n && n.doubleValue() != 0));
                }
                if (op == SqlPlanner.UnaryOperator.NEG) {
                    if (v instanceof Long l)   yield -l;
                    if (v instanceof Double d) yield -d;
                }
                yield null;
            }

            // ── IsNull / IsNotNull ────────────────────────────────────────────
            case SqlPlanner.SqlExpr.IsNull(var operand) ->
                evalHavingExpr(operand, row, colNames, projCols) == null;

            case SqlPlanner.SqlExpr.IsNotNull(var operand) ->
                evalHavingExpr(operand, row, colNames, projCols) != null;

            // ── Anything else: unsupported, treat as true (pass-through) ──────
            default -> true;
        };
    }

    /** Structural equality check for two AggExpr nodes (same function and argument). */
    private static boolean aggExprStructurallyEquals(
            SqlPlanner.SqlExpr.AggExpr a, SqlPlanner.SqlExpr candidate) {
        if (!(candidate instanceof SqlPlanner.SqlExpr.AggExpr b)) return false;
        if (a.func() != b.func()) return false;
        // Both have same function — compare args (best-effort, column name level).
        if (a.arg() instanceof SqlPlanner.AggArg.Star && b.arg() instanceof SqlPlanner.AggArg.Star)
            return true;
        if (a.arg() instanceof SqlPlanner.AggArg.Expr ae
                && b.arg() instanceof SqlPlanner.AggArg.Expr be) {
            return ae.expression().toString().equals(be.expression().toString());
        }
        return false;
    }

    /** Evaluate a binary SQL operator against two Java values. */
    private static Object evalBinaryOp(
            SqlPlanner.BinaryOperator op, Object lv, Object rv) {
        if (op == SqlPlanner.BinaryOperator.AND) {
            if (Boolean.FALSE.equals(lv) || Boolean.FALSE.equals(rv)) return false;
            if (lv == null || rv == null) return null;
            return isTruthy(lv) && isTruthy(rv);
        }
        if (op == SqlPlanner.BinaryOperator.OR) {
            if (isTruthy(lv) || isTruthy(rv)) return true;
            if (lv == null || rv == null) return null;
            return false;
        }
        if (lv == null || rv == null) return null;

        if (op == SqlPlanner.BinaryOperator.ADD
         || op == SqlPlanner.BinaryOperator.SUB
         || op == SqlPlanner.BinaryOperator.MUL
         || op == SqlPlanner.BinaryOperator.DIV
         || op == SqlPlanner.BinaryOperator.MOD) {
            double l = toDouble(lv), r = toDouble(rv);
            double res = switch (op) {
                case ADD -> l + r;
                case SUB -> l - r;
                case MUL -> l * r;
                case DIV -> r == 0 ? 0 : l / r;
                case MOD -> r == 0 ? 0 : l % r;
                default  -> 0;
            };
            // Return long if both operands are longs and result is exact.
            if (lv instanceof Long && rv instanceof Long && res == (long) res) {
                return switch (op) {
                    case ADD -> ((Long) lv) + ((Long) rv);
                    case SUB -> ((Long) lv) - ((Long) rv);
                    case MUL -> ((Long) lv) * ((Long) rv);
                    case DIV -> rv.equals(0L) ? null : ((Long) lv) / ((Long) rv);
                    case MOD -> rv.equals(0L) ? null : ((Long) lv) % ((Long) rv);
                    default  -> (long) res;
                };
            }
            return res;
        }

        int cmp = sqlCompareValues(lv, rv);
        return switch (op) {
            case EQ     -> cmp == 0;
            case NOT_EQ -> cmp != 0;
            case LT     -> cmp <  0;
            case LTE    -> cmp <= 0;
            case GT     -> cmp >  0;
            case GTE    -> cmp >= 0;
            default -> null;
        };
    }

    private static boolean isTruthy(Object v) {
        if (v == null) return false;
        if (v instanceof Boolean b) return b;
        if (v instanceof Number n) return n.doubleValue() != 0;
        return true;
    }

    private static double toDouble(Object v) {
        if (v instanceof Number n) return n.doubleValue();
        return 0;
    }

    /** SQL-style value comparison: numbers compared numerically, strings lexicographically. */
    @SuppressWarnings({"rawtypes", "unchecked"})
    private static int sqlCompareValues(Object a, Object b) {
        if (a instanceof Number na && b instanceof Number nb) {
            return Double.compare(na.doubleValue(), nb.doubleValue());
        }
        if (a instanceof Comparable ca && b.getClass().isAssignableFrom(a.getClass())) {
            try { return ca.compareTo(b); } catch (ClassCastException ignored) {}
        }
        return a.toString().compareTo(b.toString());
    }

    private static String firstKeyword(String sql) {
        String t = sql == null ? "" : sql.trim();
        int end = 0;
        while (end < t.length() && (Character.isLetter(t.charAt(end)) || t.charAt(end) == '_')) end++;
        return t.substring(0, end).toUpperCase(Locale.ROOT);
    }
}

package com.codingadventures.minisqlite;

import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Objects;
import java.util.Set;

public final class MiniSqlite {
    public static final String API_LEVEL = "2.0";
    public static final int THREADSAFETY = 1;
    public static final String PARAMSTYLE = "qmark";

    private MiniSqlite() {
    }

    public static Connection connect(String database) {
        return connect(database, Options.defaults());
    }

    public static Connection connect(String database, Options options) {
        if (!":memory:".equals(database)) {
            throw new MiniSqliteException(
                "NotSupportedError",
                "Java mini-sqlite supports only :memory: in Level 0"
            );
        }
        return new Connection(options == null ? Options.defaults() : options);
    }

    public record Options(boolean autocommit) {
        public static Options defaults() {
            return new Options(false);
        }
    }

    public record Column(String name) {
    }

    public static final class MiniSqliteException extends RuntimeException {
        private final String kind;

        public MiniSqliteException(String kind, String message) {
            super(message);
            this.kind = kind;
        }

        public String kind() {
            return kind;
        }
    }

    public static final class Connection implements AutoCloseable {
        private Database db = new Database();
        private final boolean autocommit;
        private Database snapshot;
        private boolean closed;

        private Connection(Options options) {
            this.autocommit = options.autocommit();
        }

        public Cursor cursor() {
            assertOpen();
            return new Cursor(this);
        }

        public Cursor execute(String sql) {
            return execute(sql, List.of());
        }

        public Cursor execute(String sql, List<?> params) {
            return cursor().execute(sql, params);
        }

        public Cursor executemany(String sql, List<List<?>> paramsSeq) {
            return cursor().executemany(sql, paramsSeq);
        }

        public void commit() {
            assertOpen();
            snapshot = null;
        }

        public void rollback() {
            assertOpen();
            if (snapshot != null) {
                db = snapshot.copy();
                snapshot = null;
            }
        }

        @Override
        public void close() {
            if (closed) {
                return;
            }
            if (snapshot != null) {
                db = snapshot.copy();
            }
            snapshot = null;
            closed = true;
        }

        private void assertOpen() {
            if (closed) {
                throw new MiniSqliteException("ProgrammingError", "connection is closed");
            }
        }

        private void ensureSnapshot() {
            if (!autocommit && snapshot == null) {
                snapshot = db.copy();
            }
        }

        private Result executeBound(String sql, List<?> params) {
            assertOpen();
            String bound = bindParameters(sql, params == null ? List.of() : params);
            String keyword = firstKeyword(bound);
            try {
                return switch (keyword) {
                    case "BEGIN" -> {
                        ensureSnapshot();
                        yield Result.empty(0);
                    }
                    case "COMMIT" -> {
                        snapshot = null;
                        yield Result.empty(0);
                    }
                    case "ROLLBACK" -> {
                        if (snapshot != null) {
                            db = snapshot.copy();
                            snapshot = null;
                        }
                        yield Result.empty(0);
                    }
                    case "SELECT" -> db.select(bound);
                    case "CREATE" -> {
                        ensureSnapshot();
                        yield db.create(parseCreate(bound));
                    }
                    case "DROP" -> {
                        ensureSnapshot();
                        yield db.drop(parseDrop(bound));
                    }
                    case "INSERT" -> {
                        ensureSnapshot();
                        yield db.insert(parseInsert(bound));
                    }
                    case "UPDATE" -> {
                        ensureSnapshot();
                        yield db.update(parseUpdate(bound));
                    }
                    case "DELETE" -> {
                        ensureSnapshot();
                        yield db.delete(parseDelete(bound));
                    }
                    default -> throw new IllegalArgumentException("unsupported SQL statement");
                };
            } catch (MiniSqliteException ex) {
                throw ex;
            } catch (RuntimeException ex) {
                throw new MiniSqliteException("OperationalError", ex.getMessage());
            }
        }
    }

    public static final class Cursor implements AutoCloseable {
        private final Connection connection;
        private List<Column> description = List.of();
        private int rowcount = -1;
        private Object lastrowid;
        private int arraysize = 1;
        private List<List<Object>> rows = List.of();
        private int offset;
        private boolean closed;

        private Cursor(Connection connection) {
            this.connection = connection;
        }

        public Cursor execute(String sql) {
            return execute(sql, List.of());
        }

        public Cursor execute(String sql, List<?> params) {
            if (closed) {
                throw new MiniSqliteException("ProgrammingError", "cursor is closed");
            }
            Result result = connection.executeBound(sql, params);
            rows = result.rows();
            offset = 0;
            rowcount = result.rowsAffected();
            description = result.columns().stream().map(Column::new).toList();
            return this;
        }

        public Cursor executemany(String sql, List<List<?>> paramsSeq) {
            int total = 0;
            for (List<?> params : paramsSeq == null ? List.<List<?>>of() : paramsSeq) {
                execute(sql, params);
                if (rowcount > 0) {
                    total += rowcount;
                }
            }
            if (paramsSeq != null && !paramsSeq.isEmpty()) {
                rowcount = total;
            }
            return this;
        }

        public List<Object> fetchone() {
            if (closed || offset >= rows.size()) {
                return null;
            }
            return rows.get(offset++);
        }

        public List<List<Object>> fetchmany() {
            return fetchmany(arraysize);
        }

        public List<List<Object>> fetchmany(int size) {
            if (closed) {
                return List.of();
            }
            List<List<Object>> out = new ArrayList<>();
            for (int i = 0; i < size; i++) {
                List<Object> row = fetchone();
                if (row == null) {
                    break;
                }
                out.add(row);
            }
            return out;
        }

        public List<List<Object>> fetchall() {
            if (closed) {
                return List.of();
            }
            List<List<Object>> out = new ArrayList<>();
            while (true) {
                List<Object> row = fetchone();
                if (row == null) {
                    break;
                }
                out.add(row);
            }
            return out;
        }

        @Override
        public void close() {
            closed = true;
            rows = List.of();
            description = List.of();
        }

        public List<Column> description() {
            return description;
        }

        public int rowcount() {
            return rowcount;
        }

        public Object lastrowid() {
            return lastrowid;
        }

        public int arraysize() {
            return arraysize;
        }

        public void arraysize(int arraysize) {
            this.arraysize = arraysize;
        }
    }

    private record Result(List<String> columns, List<List<Object>> rows, int rowsAffected) {
        static Result empty(int rowsAffected) {
            return new Result(List.of(), List.of(), rowsAffected);
        }
    }

    private record CreateStmt(String table, List<String> columns, boolean ifNotExists) {
    }

    private record DropStmt(String table, boolean ifExists) {
    }

    private record InsertStmt(String table, List<String> columns, List<List<Object>> rows) {
    }

    private record Assignment(String column, Object value) {
    }

    private record UpdateStmt(String table, List<Assignment> assignments, String where) {
    }

    private record DeleteStmt(String table, String where) {
    }

    private static final class Table {
        private final List<String> columns;
        private final List<Map<String, Object>> rows;

        private Table(List<String> columns) {
            this.columns = new ArrayList<>(columns);
            this.rows = new ArrayList<>();
        }

        private Table copy() {
            Table copy = new Table(columns);
            for (Map<String, Object> row : rows) {
                copy.rows.add(new LinkedHashMap<>(row));
            }
            return copy;
        }
    }

    private static final class Database {
        private final Map<String, Table> tables = new LinkedHashMap<>();

        private Database copy() {
            Database copy = new Database();
            for (Map.Entry<String, Table> entry : tables.entrySet()) {
                copy.tables.put(entry.getKey(), entry.getValue().copy());
            }
            return copy;
        }

        private Result create(CreateStmt stmt) {
            String key = normalizeName(stmt.table());
            if (tables.containsKey(key)) {
                if (stmt.ifNotExists()) {
                    return Result.empty(0);
                }
                throw new IllegalArgumentException("table already exists: " + stmt.table());
            }
            Set<String> seen = new LinkedHashSet<>();
            for (String column : stmt.columns()) {
                if (!seen.add(normalizeName(column))) {
                    throw new IllegalArgumentException("duplicate column: " + column);
                }
            }
            tables.put(key, new Table(stmt.columns()));
            return Result.empty(0);
        }

        private Result drop(DropStmt stmt) {
            String key = normalizeName(stmt.table());
            if (!tables.containsKey(key)) {
                if (stmt.ifExists()) {
                    return Result.empty(0);
                }
                throw new IllegalArgumentException("no such table: " + stmt.table());
            }
            tables.remove(key);
            return Result.empty(0);
        }

        private Result insert(InsertStmt stmt) {
            Table table = table(stmt.table());
            List<String> columns = stmt.columns().isEmpty()
                ? new ArrayList<>(table.columns)
                : stmt.columns().stream().map(column -> canonicalColumn(table, column)).toList();
            for (List<Object> values : stmt.rows()) {
                if (values.size() != columns.size()) {
                    throw new IllegalArgumentException(
                        "INSERT expected " + columns.size() + " values, got " + values.size()
                    );
                }
                Map<String, Object> row = new LinkedHashMap<>();
                for (String tableColumn : table.columns) {
                    row.put(tableColumn, null);
                }
                for (int i = 0; i < columns.size(); i++) {
                    row.put(columns.get(i), values.get(i));
                }
                table.rows.add(row);
            }
            return Result.empty(stmt.rows().size());
        }

        private Result update(UpdateStmt stmt) {
            Table table = table(stmt.table());
            List<Map<String, Object>> matches = matchingRows(table, stmt.where());
            List<Assignment> assignments = stmt.assignments().stream()
                .map(assignment -> new Assignment(canonicalColumn(table, assignment.column()), assignment.value()))
                .toList();
            for (Map<String, Object> row : matches) {
                for (Assignment assignment : assignments) {
                    row.put(assignment.column(), assignment.value());
                }
            }
            return Result.empty(matches.size());
        }

        private Result delete(DeleteStmt stmt) {
            Table table = table(stmt.table());
            List<Map<String, Object>> matches = matchingRows(table, stmt.where());
            table.rows.removeAll(matches);
            return Result.empty(matches.size());
        }

        private Result select(String sql) {
            String stripped = stripTrailingSemicolon(sql);
            String body = stripped.replaceFirst("(?is)^\\s*SELECT\\s+", "");
            Split fromSplit = splitTopLevelKeyword(body, "FROM");
            if (fromSplit.right().isEmpty()) {
                throw new IllegalArgumentException("invalid SELECT statement");
            }
            String columnSql = fromSplit.left();
            String rest = fromSplit.right();
            String tableName = identifierAtStart(rest);
            if (tableName == null) {
                throw new IllegalArgumentException("invalid SELECT statement");
            }
            rest = trim(rest.substring(tableName.length()));
            Split orderSplit = splitTopLevelKeyword(rest, "ORDER BY");
            Split whereSplit = splitTopLevelKeyword(orderSplit.left(), "WHERE");

            Table table = table(tableName);
            List<Map<String, Object>> filtered = matchingRows(table, whereSplit.right());
            List<Map<String, Object>> ordered = new ArrayList<>(filtered);
            applyOrder(table, ordered, orderSplit.right());

            List<String> selectedColumns = parseSelectedColumns(table, columnSql);
            List<List<Object>> outRows = new ArrayList<>();
            for (Map<String, Object> row : ordered) {
                List<Object> out = new ArrayList<>();
                for (String column : selectedColumns) {
                    out.add(valueOfColumn(table, row, column));
                }
                outRows.add(Collections.unmodifiableList(out));
            }
            return new Result(List.copyOf(selectedColumns), List.copyOf(outRows), -1);
        }

        private Table table(String tableName) {
            Table table = tables.get(normalizeName(tableName));
            if (table == null) {
                throw new IllegalArgumentException("no such table: " + tableName);
            }
            return table;
        }

        private List<Map<String, Object>> matchingRows(Table table, String whereSql) {
            if (trim(whereSql).isEmpty()) {
                return new ArrayList<>(table.rows);
            }
            List<Map<String, Object>> matches = new ArrayList<>();
            for (Map<String, Object> row : table.rows) {
                if (matchesWhere(table, row, whereSql)) {
                    matches.add(row);
                }
            }
            return matches;
        }
    }

    private record Split(String left, String right) {
    }

    private record OperatorAt(int index, String operator) {
    }

    private static String trim(String value) {
        return value == null ? "" : value.trim();
    }

    private static String normalizeName(String name) {
        return name.toLowerCase(Locale.ROOT);
    }

    private static String stripTrailingSemicolon(String sql) {
        return trim(sql).replaceFirst(";\\s*$", "");
    }

    private static String firstKeyword(String sql) {
        String trimmed = trim(sql);
        int end = 0;
        while (end < trimmed.length()) {
            char ch = trimmed.charAt(end);
            if (!Character.isLetter(ch) && ch != '_') {
                break;
            }
            end++;
        }
        return trimmed.substring(0, end).toUpperCase(Locale.ROOT);
    }

    private static boolean isBoundaryChar(char ch) {
        return !Character.isLetterOrDigit(ch) && ch != '_';
    }

    private static String quoteSqlString(Object value) {
        return "'" + Objects.toString(value).replace("'", "''") + "'";
    }

    private static String toSqlLiteral(Object value) {
        if (value == null) {
            return "NULL";
        }
        if (value instanceof Boolean bool) {
            return bool ? "TRUE" : "FALSE";
        }
        if (value instanceof Number) {
            return value.toString();
        }
        if (value instanceof CharSequence) {
            return quoteSqlString(value);
        }
        throw new MiniSqliteException(
            "ProgrammingError",
            "unsupported parameter type: " + value.getClass().getName()
        );
    }

    private static int readQuoted(String sql, int index, char quote) {
        int i = index + 1;
        while (i < sql.length()) {
            char ch = sql.charAt(i);
            if (ch == quote) {
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

    private static String bindParameters(String sql, List<?> params) {
        StringBuilder out = new StringBuilder();
        int index = 0;
        int i = 0;
        while (i < sql.length()) {
            char ch = sql.charAt(i);
            if (ch == '\'' || ch == '"') {
                int next = readQuoted(sql, i, ch);
                out.append(sql, i, next);
                i = next;
            } else if (ch == '-' && i + 1 < sql.length() && sql.charAt(i + 1) == '-') {
                int next = i + 2;
                while (next < sql.length() && sql.charAt(next) != '\n') {
                    next++;
                }
                out.append(sql, i, next);
                i = next;
            } else if (ch == '/' && i + 1 < sql.length() && sql.charAt(i + 1) == '*') {
                int next = i + 2;
                while (next + 1 < sql.length() && !sql.substring(next, next + 2).equals("*/")) {
                    next++;
                }
                next = Math.min(next + 2, sql.length());
                out.append(sql, i, next);
                i = next;
            } else if (ch == '?') {
                if (index >= params.size()) {
                    throw new MiniSqliteException("ProgrammingError", "not enough parameters for SQL statement");
                }
                out.append(toSqlLiteral(params.get(index++)));
                i++;
            } else {
                out.append(ch);
                i++;
            }
        }
        if (index < params.size()) {
            throw new MiniSqliteException("ProgrammingError", "too many parameters for SQL statement");
        }
        return out.toString();
    }

    private static List<String> splitTopLevel(String text, char delimiter) {
        List<String> parts = new ArrayList<>();
        StringBuilder current = new StringBuilder();
        int depth = 0;
        Character quote = null;
        for (int i = 0; i < text.length(); i++) {
            char ch = text.charAt(i);
            if (quote != null) {
                current.append(ch);
                if (ch == quote) {
                    if (i + 1 < text.length() && text.charAt(i + 1) == quote) {
                        current.append(text.charAt(++i));
                    } else {
                        quote = null;
                    }
                }
            } else if (ch == '\'' || ch == '"') {
                quote = ch;
                current.append(ch);
            } else if (ch == '(') {
                depth++;
                current.append(ch);
            } else if (ch == ')') {
                depth = Math.max(0, depth - 1);
                current.append(ch);
            } else if (depth == 0 && ch == delimiter) {
                String part = trim(current.toString());
                if (!part.isEmpty()) {
                    parts.add(part);
                }
                current.setLength(0);
            } else {
                current.append(ch);
            }
        }
        String part = trim(current.toString());
        if (!part.isEmpty()) {
            parts.add(part);
        }
        return parts;
    }

    private static Split splitTopLevelKeyword(String text, String keyword) {
        String upper = text.toUpperCase(Locale.ROOT);
        int keyLength = keyword.length();
        int depth = 0;
        Character quote = null;
        for (int i = 0; i < text.length(); i++) {
            char ch = text.charAt(i);
            if (quote != null) {
                if (ch == quote) {
                    if (i + 1 < text.length() && text.charAt(i + 1) == quote) {
                        i++;
                    } else {
                        quote = null;
                    }
                }
            } else if (ch == '\'' || ch == '"') {
                quote = ch;
            } else if (ch == '(') {
                depth++;
            } else if (ch == ')') {
                depth = Math.max(0, depth - 1);
            } else if (
                depth == 0
                    && i + keyLength <= text.length()
                    && upper.substring(i, i + keyLength).equals(keyword)
                    && (i == 0 || isBoundaryChar(text.charAt(i - 1)))
                    && (i + keyLength == text.length() || isBoundaryChar(text.charAt(i + keyLength)))
            ) {
                return new Split(trim(text.substring(0, i)), trim(text.substring(i + keyLength)));
            }
        }
        return new Split(trim(text), "");
    }

    private static int findMatchingParen(String text, int openIndex) {
        int depth = 0;
        Character quote = null;
        for (int i = openIndex; i < text.length(); i++) {
            char ch = text.charAt(i);
            if (quote != null) {
                if (ch == quote) {
                    if (i + 1 < text.length() && text.charAt(i + 1) == quote) {
                        i++;
                    } else {
                        quote = null;
                    }
                }
            } else if (ch == '\'' || ch == '"') {
                quote = ch;
            } else if (ch == '(') {
                depth++;
            } else if (ch == ')') {
                depth--;
                if (depth == 0) {
                    return i;
                }
            }
        }
        return -1;
    }

    private static Object parseLiteral(String text) {
        String value = trim(text);
        String upper = value.toUpperCase(Locale.ROOT);
        if (upper.equals("NULL")) {
            return null;
        }
        if (upper.equals("TRUE")) {
            return Boolean.TRUE;
        }
        if (upper.equals("FALSE")) {
            return Boolean.FALSE;
        }
        if (value.length() >= 2 && value.charAt(0) == '\'' && value.charAt(value.length() - 1) == '\'') {
            return value.substring(1, value.length() - 1).replace("''", "'");
        }
        if (value.matches("[-+]?(?:\\d+(?:\\.\\d*)?|\\.\\d+)")) {
            if (value.contains(".")) {
                return Double.parseDouble(value);
            }
            return Long.parseLong(value);
        }
        throw new IllegalArgumentException("expected literal value, got: " + text);
    }

    private static String identifierAtStart(String text) {
        String value = trim(text);
        if (value.isEmpty() || (!Character.isLetter(value.charAt(0)) && value.charAt(0) != '_')) {
            return null;
        }
        int end = 1;
        while (end < value.length()) {
            char ch = value.charAt(end);
            if (!Character.isLetterOrDigit(ch) && ch != '_') {
                break;
            }
            end++;
        }
        return value.substring(0, end);
    }

    private static CreateStmt parseCreate(String sql) {
        String stripped = stripTrailingSemicolon(sql);
        boolean ifNotExists = stripped.matches("(?is)^\\s*CREATE\\s+TABLE\\s+IF\\s+NOT\\s+EXISTS\\s+.*");
        String prefix = ifNotExists
            ? "(?is)^\\s*CREATE\\s+TABLE\\s+IF\\s+NOT\\s+EXISTS\\s+"
            : "(?is)^\\s*CREATE\\s+TABLE\\s+";
        String rest = stripped.replaceFirst(prefix, "");
        String table = identifierAtStart(rest);
        if (table == null) {
            throw new IllegalArgumentException("invalid CREATE TABLE statement");
        }
        rest = trim(rest.substring(table.length()));
        if (!rest.startsWith("(") || !rest.endsWith(")")) {
            throw new IllegalArgumentException("invalid CREATE TABLE statement");
        }
        String defs = rest.substring(1, rest.length() - 1);
        List<String> columns = new ArrayList<>();
        for (String part : splitTopLevel(defs, ',')) {
            String name = identifierAtStart(part);
            if (name != null) {
                columns.add(name);
            }
        }
        if (columns.isEmpty()) {
            throw new IllegalArgumentException("CREATE TABLE requires at least one column");
        }
        return new CreateStmt(table, columns, ifNotExists);
    }

    private static DropStmt parseDrop(String sql) {
        String stripped = stripTrailingSemicolon(sql);
        boolean ifExists = stripped.matches("(?is)^\\s*DROP\\s+TABLE\\s+IF\\s+EXISTS\\s+.*");
        String prefix = ifExists
            ? "(?is)^\\s*DROP\\s+TABLE\\s+IF\\s+EXISTS\\s+"
            : "(?is)^\\s*DROP\\s+TABLE\\s+";
        String rest = stripped.replaceFirst(prefix, "");
        String table = identifierAtStart(rest);
        if (table == null || !trim(rest.substring(table.length())).isEmpty()) {
            throw new IllegalArgumentException("invalid DROP TABLE statement");
        }
        return new DropStmt(table, ifExists);
    }

    private static List<List<Object>> parseValueRows(String sql) {
        String rest = trim(sql);
        List<List<Object>> rows = new ArrayList<>();
        while (!rest.isEmpty()) {
            if (!rest.startsWith("(")) {
                throw new IllegalArgumentException("INSERT VALUES rows must be parenthesized");
            }
            int close = findMatchingParen(rest, 0);
            if (close < 0) {
                throw new IllegalArgumentException("unterminated INSERT VALUES row");
            }
            String inside = rest.substring(1, close);
            List<Object> row = splitTopLevel(inside, ',').stream().map(MiniSqlite::parseLiteral).toList();
            if (row.isEmpty()) {
                throw new IllegalArgumentException("INSERT row requires at least one value");
            }
            rows.add(row);
            rest = trim(rest.substring(close + 1));
            if (rest.startsWith(",")) {
                rest = trim(rest.substring(1));
            } else if (!rest.isEmpty()) {
                throw new IllegalArgumentException("invalid text after INSERT row");
            }
        }
        if (rows.isEmpty()) {
            throw new IllegalArgumentException("INSERT requires at least one row");
        }
        return rows;
    }

    private static InsertStmt parseInsert(String sql) {
        String stripped = stripTrailingSemicolon(sql);
        String rest = stripped.replaceFirst("(?is)^\\s*INSERT\\s+INTO\\s+", "");
        if (rest.equals(stripped)) {
            throw new IllegalArgumentException("invalid INSERT statement");
        }
        String table = identifierAtStart(rest);
        if (table == null) {
            throw new IllegalArgumentException("invalid INSERT statement");
        }
        rest = trim(rest.substring(table.length()));
        List<String> columns = new ArrayList<>();
        if (rest.startsWith("(")) {
            int close = findMatchingParen(rest, 0);
            if (close < 0) {
                throw new IllegalArgumentException("invalid INSERT statement");
            }
            columns.addAll(splitTopLevel(rest.substring(1, close), ',').stream().map(MiniSqlite::trim).toList());
            rest = trim(rest.substring(close + 1));
        }
        if (!rest.toUpperCase(Locale.ROOT).startsWith("VALUES")) {
            throw new IllegalArgumentException("invalid INSERT statement");
        }
        return new InsertStmt(table, columns, parseValueRows(rest.substring("VALUES".length())));
    }

    private static UpdateStmt parseUpdate(String sql) {
        String stripped = stripTrailingSemicolon(sql);
        String rest = stripped.replaceFirst("(?is)^\\s*UPDATE\\s+", "");
        if (rest.equals(stripped)) {
            throw new IllegalArgumentException("invalid UPDATE statement");
        }
        String table = identifierAtStart(rest);
        if (table == null) {
            throw new IllegalArgumentException("invalid UPDATE statement");
        }
        rest = trim(rest.substring(table.length()));
        if (!rest.toUpperCase(Locale.ROOT).startsWith("SET")) {
            throw new IllegalArgumentException("invalid UPDATE statement");
        }
        rest = trim(rest.substring("SET".length()));
        Split whereSplit = splitTopLevelKeyword(rest, "WHERE");
        List<Assignment> assignments = new ArrayList<>();
        for (String assignment : splitTopLevel(whereSplit.left(), ',')) {
            OperatorAt op = findTopLevelOperator(assignment, List.of("="));
            if (op == null) {
                throw new IllegalArgumentException("invalid assignment: " + assignment);
            }
            String column = trim(assignment.substring(0, op.index()));
            if (!column.matches("[A-Za-z_][A-Za-z0-9_]*")) {
                throw new IllegalArgumentException("invalid identifier: " + column);
            }
            assignments.add(new Assignment(column, parseLiteral(assignment.substring(op.index() + 1))));
        }
        if (assignments.isEmpty()) {
            throw new IllegalArgumentException("UPDATE requires at least one assignment");
        }
        return new UpdateStmt(table, assignments, whereSplit.right());
    }

    private static DeleteStmt parseDelete(String sql) {
        String stripped = stripTrailingSemicolon(sql);
        String rest = stripped.replaceFirst("(?is)^\\s*DELETE\\s+FROM\\s+", "");
        if (rest.equals(stripped)) {
            throw new IllegalArgumentException("invalid DELETE statement");
        }
        String table = identifierAtStart(rest);
        if (table == null) {
            throw new IllegalArgumentException("invalid DELETE statement");
        }
        rest = trim(rest.substring(table.length()));
        String where = "";
        if (!rest.isEmpty()) {
            Split whereSplit = splitTopLevelKeyword(rest, "WHERE");
            if (!whereSplit.left().isEmpty() || whereSplit.right().isEmpty()) {
                throw new IllegalArgumentException("invalid DELETE statement");
            }
            where = whereSplit.right();
        }
        return new DeleteStmt(table, where);
    }

    private static List<String> parseSelectedColumns(Table table, String columnSql) {
        if (trim(columnSql).equals("*")) {
            return List.copyOf(table.columns);
        }
        return splitTopLevel(columnSql, ',').stream()
            .map(MiniSqlite::trim)
            .map(column -> canonicalColumn(table, column))
            .toList();
    }

    private static String canonicalColumn(Table table, String column) {
        String wanted = normalizeName(trim(column));
        for (String candidate : table.columns) {
            if (normalizeName(candidate).equals(wanted)) {
                return candidate;
            }
        }
        throw new IllegalArgumentException("no such column: " + trim(column));
    }

    private static Object valueOfColumn(Table table, Map<String, Object> row, String column) {
        return row.get(canonicalColumn(table, column));
    }

    private static boolean matchesWhere(Table table, Map<String, Object> row, String whereSql) {
        String where = trim(whereSql);
        if (where.isEmpty()) {
            return true;
        }
        Split orSplit = splitTopLevelKeyword(where, "OR");
        if (!orSplit.right().isEmpty()) {
            return matchesWhere(table, row, orSplit.left()) || matchesWhere(table, row, orSplit.right());
        }
        Split andSplit = splitTopLevelKeyword(where, "AND");
        if (!andSplit.right().isEmpty()) {
            return matchesWhere(table, row, andSplit.left()) && matchesWhere(table, row, andSplit.right());
        }
        String upper = where.toUpperCase(Locale.ROOT);
        if (upper.endsWith(" IS NOT NULL")) {
            String column = trim(where.substring(0, where.length() - " IS NOT NULL".length()));
            return valueOfColumn(table, row, column) != null;
        }
        if (upper.endsWith(" IS NULL")) {
            String column = trim(where.substring(0, where.length() - " IS NULL".length()));
            return valueOfColumn(table, row, column) == null;
        }
        int inIndex = upper.indexOf(" IN ");
        if (inIndex > 0) {
            String column = trim(where.substring(0, inIndex));
            String valuesSql = trim(where.substring(inIndex + 4));
            if (!valuesSql.startsWith("(") || !valuesSql.endsWith(")")) {
                throw new IllegalArgumentException("invalid IN predicate");
            }
            Object actual = valueOfColumn(table, row, column);
            for (String part : splitTopLevel(valuesSql.substring(1, valuesSql.length() - 1), ',')) {
                if (valuesEqual(actual, parseLiteral(part))) {
                    return true;
                }
            }
            return false;
        }
        OperatorAt op = findTopLevelOperator(where, List.of("<=", ">=", "!=", "<>", "=", "<", ">"));
        if (op == null) {
            throw new IllegalArgumentException("unsupported WHERE predicate: " + where);
        }
        Object actual = valueOfColumn(table, row, where.substring(0, op.index()));
        Object expected = parseLiteral(where.substring(op.index() + op.operator().length()));
        return comparePredicate(actual, expected, op.operator());
    }

    private static OperatorAt findTopLevelOperator(String text, List<String> operators) {
        int depth = 0;
        Character quote = null;
        for (int i = 0; i < text.length(); i++) {
            char ch = text.charAt(i);
            if (quote != null) {
                if (ch == quote) {
                    if (i + 1 < text.length() && text.charAt(i + 1) == quote) {
                        i++;
                    } else {
                        quote = null;
                    }
                }
            } else if (ch == '\'' || ch == '"') {
                quote = ch;
            } else if (ch == '(') {
                depth++;
            } else if (ch == ')') {
                depth = Math.max(0, depth - 1);
            } else if (depth == 0) {
                for (String operator : operators) {
                    if (i + operator.length() <= text.length()
                        && text.substring(i, i + operator.length()).equals(operator)) {
                        return new OperatorAt(i, operator);
                    }
                }
            }
        }
        return null;
    }

    private static boolean comparePredicate(Object actual, Object expected, String operator) {
        if (operator.equals("=")) {
            return valuesEqual(actual, expected);
        }
        if (operator.equals("!=") || operator.equals("<>")) {
            return !valuesEqual(actual, expected);
        }
        if (actual == null || expected == null) {
            return false;
        }
        int comparison = compareValues(actual, expected);
        return switch (operator) {
            case "<" -> comparison < 0;
            case ">" -> comparison > 0;
            case "<=" -> comparison <= 0;
            case ">=" -> comparison >= 0;
            default -> false;
        };
    }

    private static boolean valuesEqual(Object left, Object right) {
        if (left == null || right == null) {
            return left == right;
        }
        if (left instanceof Number leftNumber && right instanceof Number rightNumber) {
            return Double.compare(leftNumber.doubleValue(), rightNumber.doubleValue()) == 0;
        }
        return Objects.equals(left, right);
    }

    private static int compareValues(Object left, Object right) {
        if (left instanceof Number leftNumber && right instanceof Number rightNumber) {
            return Double.compare(leftNumber.doubleValue(), rightNumber.doubleValue());
        }
        return Objects.toString(left).compareTo(Objects.toString(right));
    }

    private static void applyOrder(Table table, List<Map<String, Object>> rows, String orderSql) {
        if (trim(orderSql).isEmpty()) {
            return;
        }
        String[] parts = trim(orderSql).split("\\s+");
        String column = canonicalColumn(table, parts[0]);
        boolean desc = parts.length > 1 && parts[1].equalsIgnoreCase("DESC");
        Comparator<Map<String, Object>> comparator = (left, right) -> {
            Object l = left.get(column);
            Object r = right.get(column);
            if (l == null && r == null) {
                return 0;
            }
            if (l == null) {
                return 1;
            }
            if (r == null) {
                return -1;
            }
            return compareValues(l, r);
        };
        if (desc) {
            comparator = comparator.reversed();
        }
        rows.sort(comparator);
    }
}

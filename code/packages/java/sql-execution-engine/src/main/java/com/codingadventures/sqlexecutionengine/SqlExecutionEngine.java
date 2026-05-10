package com.codingadventures.sqlexecutionengine;

import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.HashMap;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Objects;
import java.util.Set;
import java.util.regex.Pattern;

public final class SqlExecutionEngine {
    private SqlExecutionEngine() {
    }

    public interface DataSource {
        List<String> schema(String tableName);

        List<Map<String, Object>> scan(String tableName);
    }

    public record QueryResult(List<String> columns, List<List<Object>> rows) {
        public QueryResult {
            columns = List.copyOf(columns);
            rows = rows.stream()
                .map(row -> Collections.unmodifiableList(new ArrayList<>(row)))
                .toList();
        }
    }

    public record ExecutionResult(boolean ok, QueryResult result, String error) {
        public static ExecutionResult success(QueryResult result) {
            return new ExecutionResult(true, result, null);
        }

        public static ExecutionResult failure(String error) {
            return new ExecutionResult(false, null, error);
        }
    }

    public static final class SqlExecutionException extends RuntimeException {
        public SqlExecutionException(String message) {
            super(message);
        }

        public SqlExecutionException(String message, Throwable cause) {
            super(message, cause);
        }
    }

    public static QueryResult execute(String sql, DataSource dataSource) {
        try {
            Parser parser = new Parser(tokenize(sql));
            SelectStatement statement = parser.parseStatement();
            return executeSelect(statement, dataSource);
        } catch (SqlExecutionException ex) {
            throw ex;
        } catch (RuntimeException ex) {
            throw new SqlExecutionException(ex.getMessage(), ex);
        }
    }

    public static ExecutionResult tryExecute(String sql, DataSource dataSource) {
        try {
            return ExecutionResult.success(execute(sql, dataSource));
        } catch (RuntimeException ex) {
            return ExecutionResult.failure(ex.getMessage());
        }
    }

    public static final class InMemoryDataSource implements DataSource {
        private final Map<String, List<String>> schemas = new LinkedHashMap<>();
        private final Map<String, List<Map<String, Object>>> tables = new LinkedHashMap<>();

        public InMemoryDataSource addTable(String name, List<String> schema, List<Map<String, Object>> rows) {
            schemas.put(name, List.copyOf(schema));
            List<Map<String, Object>> copied = new ArrayList<>();
            for (Map<String, Object> row : rows) {
                copied.add(new LinkedHashMap<>(row));
            }
            tables.put(name, copied);
            return this;
        }

        @Override
        public List<String> schema(String tableName) {
            List<String> schema = schemas.get(tableName);
            if (schema == null) {
                throw new SqlExecutionException("table not found: " + tableName);
            }
            return schema;
        }

        @Override
        public List<Map<String, Object>> scan(String tableName) {
            List<Map<String, Object>> rows = tables.get(tableName);
            if (rows == null) {
                throw new SqlExecutionException("table not found: " + tableName);
            }
            List<Map<String, Object>> copied = new ArrayList<>();
            for (Map<String, Object> row : rows) {
                copied.add(new LinkedHashMap<>(row));
            }
            return copied;
        }
    }

    private static QueryResult executeSelect(SelectStatement statement, DataSource dataSource) {
        List<RowContext> rows = scanTable(dataSource, statement.from.name, statement.from.alias);
        for (Join join : statement.joins) {
            List<RowContext> right = scanTable(dataSource, join.table.name, join.table.alias);
            rows = applyJoin(rows, right, join);
        }

        if (statement.where != null) {
            rows = rows.stream()
                .filter(row -> truthy(eval(statement.where, row.values, null)))
                .toList();
        }

        List<RowFrame> frames = makeFrames(rows, statement);
        if (statement.having != null) {
            frames = frames.stream()
                .filter(frame -> truthy(eval(statement.having, frame.row.values, frame.groupRows)))
                .toList();
        }

        if (!statement.orderBy.isEmpty()) {
            frames = new ArrayList<>(frames);
            frames.sort((left, right) -> compareOrder(left, right, statement.orderBy));
        }

        Projection projection = project(frames, statement);
        List<List<Object>> projectedRows = projection.rows;

        if (statement.distinct) {
            Set<List<Object>> seen = new LinkedHashSet<>();
            List<List<Object>> unique = new ArrayList<>();
            for (List<Object> row : projectedRows) {
                if (seen.add(row)) {
                    unique.add(row);
                }
            }
            projectedRows = unique;
        }

        int from = Math.max(0, statement.offset == null ? 0 : statement.offset);
        int to = projectedRows.size();
        if (statement.limit != null) {
            to = Math.min(to, from + Math.max(0, statement.limit));
        }
        if (from > projectedRows.size()) {
            projectedRows = List.of();
        } else {
            projectedRows = new ArrayList<>(projectedRows.subList(from, to));
        }

        return new QueryResult(projection.columns, projectedRows);
    }

    private static List<RowContext> scanTable(DataSource dataSource, String tableName, String alias) {
        List<String> schema = dataSource.schema(tableName);
        List<Map<String, Object>> rawRows = dataSource.scan(tableName);
        List<RowContext> rows = new ArrayList<>();
        for (Map<String, Object> raw : rawRows) {
            LinkedHashMap<String, Object> values = new LinkedHashMap<>();
            for (String column : schema) {
                Object value = raw.get(column);
                values.put(column, value);
                values.put(alias + "." + column, value);
                values.put(tableName + "." + column, value);
            }
            rows.add(new RowContext(values));
        }
        return rows;
    }

    private static List<RowContext> applyJoin(List<RowContext> leftRows, List<RowContext> rightRows, Join join) {
        List<RowContext> joined = new ArrayList<>();
        if (join.type.equals("CROSS")) {
            for (RowContext left : leftRows) {
                for (RowContext right : rightRows) {
                    joined.add(left.merge(right));
                }
            }
            return joined;
        }

        for (RowContext left : leftRows) {
            boolean matched = false;
            for (RowContext right : rightRows) {
                RowContext merged = left.merge(right);
                if (join.on == null || truthy(eval(join.on, merged.values, null))) {
                    joined.add(merged);
                    matched = true;
                }
            }
            if (!matched && join.type.equals("LEFT")) {
                joined.add(left);
            }
        }
        return joined;
    }

    private static List<RowFrame> makeFrames(List<RowContext> rows, SelectStatement statement) {
        boolean grouped = !statement.groupBy.isEmpty();
        boolean aggregated = hasAggregate(statement.selectItems) || hasAggregate(statement.having);
        if (!grouped && !aggregated) {
            return rows.stream().map(row -> new RowFrame(row, null)).toList();
        }

        if (!grouped) {
            RowContext row = rows.isEmpty() ? new RowContext(new LinkedHashMap<>()) : rows.get(0);
            return List.of(new RowFrame(row, rows));
        }

        Map<List<Object>, List<RowContext>> groups = new LinkedHashMap<>();
        for (RowContext row : rows) {
            List<Object> key = new ArrayList<>();
            for (Expr expression : statement.groupBy) {
                key.add(eval(expression, row.values, null));
            }
            groups.computeIfAbsent(key, ignored -> new ArrayList<>()).add(row);
        }

        List<RowFrame> frames = new ArrayList<>();
        for (List<RowContext> groupRows : groups.values()) {
            frames.add(new RowFrame(groupRows.get(0), groupRows));
        }
        return frames;
    }

    private static Projection project(List<RowFrame> frames, SelectStatement statement) {
        if (statement.selectItems.size() == 1 && statement.selectItems.get(0).expression instanceof StarExpr) {
            List<String> columns = new ArrayList<>();
            if (!frames.isEmpty()) {
                for (String key : frames.get(0).row.values.keySet()) {
                    if (!key.contains(".")) {
                        columns.add(key);
                    }
                }
                Collections.sort(columns);
            }
            List<List<Object>> rows = new ArrayList<>();
            for (RowFrame frame : frames) {
                List<Object> out = new ArrayList<>();
                for (String column : columns) {
                    out.add(frame.row.values.get(column));
                }
                rows.add(out);
            }
            return new Projection(columns, rows);
        }

        List<String> columns = new ArrayList<>();
        for (SelectItem item : statement.selectItems) {
            columns.add(item.alias != null ? item.alias : expressionLabel(item.expression));
        }

        List<List<Object>> rows = new ArrayList<>();
        for (RowFrame frame : frames) {
            List<Object> out = new ArrayList<>();
            for (SelectItem item : statement.selectItems) {
                out.add(eval(item.expression, frame.row.values, frame.groupRows));
            }
            rows.add(out);
        }
        return new Projection(columns, rows);
    }

    private static int compareOrder(RowFrame left, RowFrame right, List<OrderItem> orderBy) {
        for (OrderItem item : orderBy) {
            Object leftValue = eval(item.expression, left.row.values, left.groupRows);
            Object rightValue = eval(item.expression, right.row.values, right.groupRows);
            int cmp = compareSql(leftValue, rightValue);
            if (cmp != 0) {
                return item.descending ? -cmp : cmp;
            }
        }
        return 0;
    }

    private static Object eval(Expr expression, Map<String, Object> row, List<RowContext> groupRows) {
        if (expression instanceof LiteralExpr literal) {
            return literal.value;
        }
        if (expression instanceof NullExpr) {
            return null;
        }
        if (expression instanceof ColumnExpr column) {
            if (column.table != null) {
                return row.get(column.table + "." + column.name);
            }
            if (row.containsKey(column.name)) {
                return row.get(column.name);
            }
            for (Map.Entry<String, Object> entry : row.entrySet()) {
                if (entry.getKey().endsWith("." + column.name)) {
                    return entry.getValue();
                }
            }
            return null;
        }
        if (expression instanceof UnaryExpr unary) {
            Object value = eval(unary.expression, row, groupRows);
            if (unary.operator.equals("NOT")) {
                return value == null ? null : !truthy(value);
            }
            if (unary.operator.equals("-")) {
                return value == null ? null : -asDouble(value);
            }
        }
        if (expression instanceof BinaryExpr binary) {
            return evalBinary(binary, row, groupRows);
        }
        if (expression instanceof IsNullExpr isNull) {
            boolean result = eval(isNull.expression, row, groupRows) == null;
            return isNull.negated ? !result : result;
        }
        if (expression instanceof BetweenExpr between) {
            Object value = eval(between.expression, row, groupRows);
            Object lower = eval(between.lower, row, groupRows);
            Object upper = eval(between.upper, row, groupRows);
            if (value == null || lower == null || upper == null) {
                return null;
            }
            boolean result = compareSql(value, lower) >= 0 && compareSql(value, upper) <= 0;
            return between.negated ? !result : result;
        }
        if (expression instanceof InExpr in) {
            Object value = eval(in.expression, row, groupRows);
            if (value == null) {
                return null;
            }
            boolean found = false;
            for (Expr option : in.values) {
                Object optionValue = eval(option, row, groupRows);
                if (optionValue != null && sqlEquals(value, optionValue)) {
                    found = true;
                    break;
                }
            }
            return in.negated ? !found : found;
        }
        if (expression instanceof LikeExpr like) {
            Object value = eval(like.expression, row, groupRows);
            Object pattern = eval(like.pattern, row, groupRows);
            if (value == null || pattern == null) {
                return null;
            }
            boolean result = like(String.valueOf(value), String.valueOf(pattern));
            return like.negated ? !result : result;
        }
        if (expression instanceof FunctionExpr function) {
            return evalFunction(function, row, groupRows);
        }
        if (expression instanceof StarExpr) {
            return row;
        }
        throw new SqlExecutionException("unknown expression");
    }

    private static Object evalBinary(BinaryExpr binary, Map<String, Object> row, List<RowContext> groupRows) {
        if (binary.operator.equals("AND")) {
            Object left = eval(binary.left, row, groupRows);
            if (left != null && !truthy(left)) {
                return false;
            }
            Object right = eval(binary.right, row, groupRows);
            if (right != null && !truthy(right)) {
                return false;
            }
            return left == null || right == null ? null : true;
        }
        if (binary.operator.equals("OR")) {
            Object left = eval(binary.left, row, groupRows);
            if (left != null && truthy(left)) {
                return true;
            }
            Object right = eval(binary.right, row, groupRows);
            if (right != null && truthy(right)) {
                return true;
            }
            return left == null || right == null ? null : false;
        }

        Object left = eval(binary.left, row, groupRows);
        Object right = eval(binary.right, row, groupRows);
        if (left == null || right == null) {
            return null;
        }
        return switch (binary.operator) {
            case "+" -> asDouble(left) + asDouble(right);
            case "-" -> asDouble(left) - asDouble(right);
            case "*" -> asDouble(left) * asDouble(right);
            case "/" -> asDouble(left) / asDouble(right);
            case "%" -> asDouble(left) % asDouble(right);
            case "=" -> sqlEquals(left, right);
            case "!=", "<>" -> !sqlEquals(left, right);
            case "<" -> compareSql(left, right) < 0;
            case ">" -> compareSql(left, right) > 0;
            case "<=" -> compareSql(left, right) <= 0;
            case ">=" -> compareSql(left, right) >= 0;
            default -> throw new SqlExecutionException("unknown operator: " + binary.operator);
        };
    }

    private static Object evalFunction(FunctionExpr function, Map<String, Object> row, List<RowContext> groupRows) {
        String name = function.name.toUpperCase(Locale.ROOT);
        if (Set.of("COUNT", "SUM", "AVG", "MIN", "MAX").contains(name)) {
            if (groupRows == null) {
                throw new SqlExecutionException("aggregate used outside grouped context: " + name);
            }
            if (name.equals("COUNT")) {
                if (function.args.size() == 1 && function.args.get(0) instanceof StarExpr) {
                    return groupRows.size();
                }
                long count = groupRows.stream()
                    .filter(groupRow -> eval(function.args.get(0), groupRow.values, null) != null)
                    .count();
                return (int) count;
            }
            List<Object> values = groupRows.stream()
                .map(groupRow -> eval(function.args.get(0), groupRow.values, null))
                .filter(Objects::nonNull)
                .toList();
            if (values.isEmpty()) {
                return null;
            }
            return switch (name) {
                case "SUM" -> values.stream().mapToDouble(SqlExecutionEngine::asDouble).sum();
                case "AVG" -> values.stream().mapToDouble(SqlExecutionEngine::asDouble).average().orElse(0.0);
                case "MIN" -> values.stream().min(SqlExecutionEngine::compareSql).orElse(null);
                case "MAX" -> values.stream().max(SqlExecutionEngine::compareSql).orElse(null);
                default -> throw new SqlExecutionException("unknown aggregate: " + name);
            };
        }

        Object value = function.args.isEmpty() ? null : eval(function.args.get(0), row, groupRows);
        if (value == null) {
            return null;
        }
        return switch (name) {
            case "UPPER" -> String.valueOf(value).toUpperCase(Locale.ROOT);
            case "LOWER" -> String.valueOf(value).toLowerCase(Locale.ROOT);
            case "LENGTH" -> String.valueOf(value).length();
            default -> throw new SqlExecutionException("unknown function: " + name);
        };
    }

    private static boolean hasAggregate(List<SelectItem> selectItems) {
        return selectItems.stream().anyMatch(item -> hasAggregate(item.expression));
    }

    private static boolean hasAggregate(Expr expression) {
        if (expression == null) {
            return false;
        }
        if (expression instanceof FunctionExpr function
            && Set.of("COUNT", "SUM", "AVG", "MIN", "MAX").contains(function.name.toUpperCase(Locale.ROOT))) {
            return true;
        }
        if (expression instanceof BinaryExpr binary) {
            return hasAggregate(binary.left) || hasAggregate(binary.right);
        }
        if (expression instanceof UnaryExpr unary) {
            return hasAggregate(unary.expression);
        }
        if (expression instanceof IsNullExpr isNull) {
            return hasAggregate(isNull.expression);
        }
        if (expression instanceof BetweenExpr between) {
            return hasAggregate(between.expression) || hasAggregate(between.lower) || hasAggregate(between.upper);
        }
        if (expression instanceof InExpr in) {
            return hasAggregate(in.expression) || in.values.stream().anyMatch(SqlExecutionEngine::hasAggregate);
        }
        if (expression instanceof LikeExpr like) {
            return hasAggregate(like.expression) || hasAggregate(like.pattern);
        }
        if (expression instanceof FunctionExpr function) {
            return function.args.stream().anyMatch(SqlExecutionEngine::hasAggregate);
        }
        return false;
    }

    private static boolean truthy(Object value) {
        if (value == null) {
            return false;
        }
        if (value instanceof Boolean bool) {
            return bool;
        }
        if (value instanceof Number number) {
            return number.doubleValue() != 0.0;
        }
        return !String.valueOf(value).isEmpty();
    }

    private static boolean sqlEquals(Object left, Object right) {
        if (left instanceof Number && right instanceof Number) {
            return Double.compare(asDouble(left), asDouble(right)) == 0;
        }
        return Objects.equals(left, right);
    }

    @SuppressWarnings({"unchecked", "rawtypes"})
    private static int compareSql(Object left, Object right) {
        if (left == null && right == null) {
            return 0;
        }
        if (left == null) {
            return 1;
        }
        if (right == null) {
            return -1;
        }
        if (left instanceof Number && right instanceof Number) {
            return Double.compare(asDouble(left), asDouble(right));
        }
        if (left instanceof Comparable comparable && left.getClass().isInstance(right)) {
            return comparable.compareTo(right);
        }
        return String.valueOf(left).compareTo(String.valueOf(right));
    }

    private static double asDouble(Object value) {
        if (value instanceof Number number) {
            return number.doubleValue();
        }
        return Double.parseDouble(String.valueOf(value));
    }

    private static String expressionLabel(Expr expression) {
        if (expression instanceof ColumnExpr column) {
            return column.name;
        }
        if (expression instanceof FunctionExpr function) {
            if (function.args.size() == 1 && function.args.get(0) instanceof StarExpr) {
                return function.name.toUpperCase(Locale.ROOT) + "(*)";
            }
            return function.name.toUpperCase(Locale.ROOT) + "(...)";
        }
        if (expression instanceof LiteralExpr literal) {
            return String.valueOf(literal.value);
        }
        return "?";
    }

    private static boolean like(String value, String sqlPattern) {
        StringBuilder regex = new StringBuilder();
        for (int i = 0; i < sqlPattern.length(); i++) {
            char ch = sqlPattern.charAt(i);
            if (ch == '%') {
                regex.append(".*");
            } else if (ch == '_') {
                regex.append('.');
            } else {
                regex.append(Pattern.quote(String.valueOf(ch)));
            }
        }
        return value.matches(regex.toString());
    }

    private static List<Token> tokenize(String sql) {
        List<Token> tokens = new ArrayList<>();
        int index = 0;
        while (index < sql.length()) {
            char ch = sql.charAt(index);
            if (Character.isWhitespace(ch)) {
                index++;
                continue;
            }
            if (ch == '-' && index + 1 < sql.length() && sql.charAt(index + 1) == '-') {
                index += 2;
                while (index < sql.length() && sql.charAt(index) != '\n') {
                    index++;
                }
                continue;
            }
            if (ch == '\'') {
                StringBuilder value = new StringBuilder();
                index++;
                while (index < sql.length()) {
                    char current = sql.charAt(index);
                    if (current == '\'' && index + 1 < sql.length() && sql.charAt(index + 1) == '\'') {
                        value.append('\'');
                        index += 2;
                    } else if (current == '\'') {
                        index++;
                        break;
                    } else {
                        value.append(current);
                        index++;
                    }
                }
                tokens.add(new Token(TokenType.STRING, value.toString()));
                continue;
            }
            if (Character.isDigit(ch) || (ch == '.' && index + 1 < sql.length() && Character.isDigit(sql.charAt(index + 1)))) {
                int start = index;
                index++;
                while (index < sql.length() && (Character.isDigit(sql.charAt(index)) || sql.charAt(index) == '.')) {
                    index++;
                }
                tokens.add(new Token(TokenType.NUMBER, sql.substring(start, index)));
                continue;
            }
            if (Character.isLetter(ch) || ch == '_') {
                int start = index;
                index++;
                while (index < sql.length() && (Character.isLetterOrDigit(sql.charAt(index)) || sql.charAt(index) == '_')) {
                    index++;
                }
                String value = sql.substring(start, index);
                String upper = value.toUpperCase(Locale.ROOT);
                tokens.add(new Token(KEYWORDS.contains(upper) ? TokenType.KEYWORD : TokenType.IDENT, value));
                continue;
            }
            if (ch == '"' || ch == '`') {
                char quote = ch;
                int start = ++index;
                while (index < sql.length() && sql.charAt(index) != quote) {
                    index++;
                }
                tokens.add(new Token(TokenType.IDENT, sql.substring(start, index)));
                index++;
                continue;
            }
            if (index + 1 < sql.length()) {
                String two = sql.substring(index, index + 2);
                if (Set.of("!=", "<>", "<=", ">=").contains(two)) {
                    tokens.add(new Token(TokenType.SYMBOL, two));
                    index += 2;
                    continue;
                }
            }
            if ("=<>+-*/%(),.;".indexOf(ch) >= 0) {
                tokens.add(new Token(TokenType.SYMBOL, String.valueOf(ch)));
                index++;
                continue;
            }
            index++;
        }
        tokens.add(new Token(TokenType.EOF, ""));
        return tokens;
    }

    private static final Set<String> KEYWORDS = Set.of(
        "SELECT", "FROM", "WHERE", "GROUP", "BY", "HAVING", "ORDER", "LIMIT", "OFFSET",
        "DISTINCT", "ALL", "JOIN", "INNER", "LEFT", "RIGHT", "FULL", "OUTER", "CROSS",
        "ON", "AS", "AND", "OR", "NOT", "IS", "NULL", "IN", "BETWEEN", "LIKE", "TRUE",
        "FALSE", "ASC", "DESC", "COUNT", "SUM", "AVG", "MIN", "MAX", "UPPER", "LOWER", "LENGTH"
    );

    private static final class Parser {
        private final List<Token> tokens;
        private int position;

        Parser(List<Token> tokens) {
            this.tokens = tokens;
        }

        SelectStatement parseStatement() {
            expectKeyword("SELECT");
            boolean distinct = matchKeyword("DISTINCT");
            matchKeyword("ALL");
            List<SelectItem> selectItems = parseSelectList();
            expectKeyword("FROM");
            TableRef from = parseTableRef();
            List<Join> joins = parseJoins();
            Expr where = matchKeyword("WHERE") ? parseExpression() : null;
            List<Expr> groupBy = new ArrayList<>();
            if (matchKeyword("GROUP")) {
                expectKeyword("BY");
                groupBy = parseExpressionList();
            }
            Expr having = matchKeyword("HAVING") ? parseExpression() : null;
            List<OrderItem> orderBy = new ArrayList<>();
            if (matchKeyword("ORDER")) {
                expectKeyword("BY");
                orderBy = parseOrderList();
            }
            Integer limit = matchKeyword("LIMIT") ? numberAsInt(advance()) : null;
            Integer offset = matchKeyword("OFFSET") ? numberAsInt(advance()) : null;
            matchSymbol(";");
            expect(TokenType.EOF);
            return new SelectStatement(distinct, selectItems, from, joins, where, groupBy, having, orderBy, limit, offset);
        }

        private List<SelectItem> parseSelectList() {
            List<SelectItem> items = new ArrayList<>();
            do {
                if (matchSymbol("*")) {
                    items.add(new SelectItem(new StarExpr(), null));
                } else {
                    Expr expression = parseExpression();
                    String alias = null;
                    if (matchKeyword("AS")) {
                        alias = expectIdentifier();
                    } else if (peek().type == TokenType.IDENT) {
                        alias = advance().value;
                    }
                    items.add(new SelectItem(expression, alias));
                }
            } while (matchSymbol(","));
            return items;
        }

        private TableRef parseTableRef() {
            String name = expectIdentifier();
            String alias = name;
            if (matchKeyword("AS")) {
                alias = expectIdentifier();
            } else if (peek().type == TokenType.IDENT) {
                alias = advance().value;
            }
            return new TableRef(name, alias);
        }

        private List<Join> parseJoins() {
            List<Join> joins = new ArrayList<>();
            while (true) {
                String type = null;
                if (matchKeyword("INNER")) {
                    type = "INNER";
                    expectKeyword("JOIN");
                } else if (matchKeyword("LEFT")) {
                    type = "LEFT";
                    matchKeyword("OUTER");
                    expectKeyword("JOIN");
                } else if (matchKeyword("CROSS")) {
                    type = "CROSS";
                    expectKeyword("JOIN");
                } else if (matchKeyword("JOIN")) {
                    type = "INNER";
                }
                if (type == null) {
                    break;
                }
                TableRef table = parseTableRef();
                Expr on = null;
                if (!type.equals("CROSS")) {
                    expectKeyword("ON");
                    on = parseExpression();
                }
                joins.add(new Join(type, table, on));
            }
            return joins;
        }

        private List<Expr> parseExpressionList() {
            List<Expr> expressions = new ArrayList<>();
            do {
                expressions.add(parseExpression());
            } while (matchSymbol(","));
            return expressions;
        }

        private List<OrderItem> parseOrderList() {
            List<OrderItem> items = new ArrayList<>();
            do {
                Expr expression = parseExpression();
                boolean descending = false;
                if (matchKeyword("ASC")) {
                    descending = false;
                } else if (matchKeyword("DESC")) {
                    descending = true;
                }
                items.add(new OrderItem(expression, descending));
            } while (matchSymbol(","));
            return items;
        }

        private Expr parseExpression() {
            return parseOr();
        }

        private Expr parseOr() {
            Expr left = parseAnd();
            while (matchKeyword("OR")) {
                left = new BinaryExpr("OR", left, parseAnd());
            }
            return left;
        }

        private Expr parseAnd() {
            Expr left = parseNot();
            while (matchKeyword("AND")) {
                left = new BinaryExpr("AND", left, parseNot());
            }
            return left;
        }

        private Expr parseNot() {
            if (matchKeyword("NOT")) {
                return new UnaryExpr("NOT", parseNot());
            }
            return parseComparison();
        }

        private Expr parseComparison() {
            Expr left = parseAdditive();
            if (matchKeyword("IS")) {
                boolean negated = matchKeyword("NOT");
                expectKeyword("NULL");
                return new IsNullExpr(left, negated);
            }
            if (matchKeyword("NOT")) {
                if (matchKeyword("BETWEEN")) {
                    Expr lower = parseAdditive();
                    expectKeyword("AND");
                    return new BetweenExpr(left, lower, parseAdditive(), true);
                }
                if (matchKeyword("IN")) {
                    return new InExpr(left, parseInValues(), true);
                }
                if (matchKeyword("LIKE")) {
                    return new LikeExpr(left, parseAdditive(), true);
                }
                throw error("expected BETWEEN, IN, or LIKE after NOT");
            }
            if (matchKeyword("BETWEEN")) {
                Expr lower = parseAdditive();
                expectKeyword("AND");
                return new BetweenExpr(left, lower, parseAdditive(), false);
            }
            if (matchKeyword("IN")) {
                return new InExpr(left, parseInValues(), false);
            }
            if (matchKeyword("LIKE")) {
                return new LikeExpr(left, parseAdditive(), false);
            }
            if (peek().type == TokenType.SYMBOL && Set.of("=", "!=", "<>", "<", ">", "<=", ">=").contains(peek().value)) {
                String operator = advance().value;
                return new BinaryExpr(operator, left, parseAdditive());
            }
            return left;
        }

        private List<Expr> parseInValues() {
            expectSymbol("(");
            List<Expr> values = parseExpressionList();
            expectSymbol(")");
            return values;
        }

        private Expr parseAdditive() {
            Expr left = parseMultiplicative();
            while (peek().type == TokenType.SYMBOL && Set.of("+", "-").contains(peek().value)) {
                String operator = advance().value;
                left = new BinaryExpr(operator, left, parseMultiplicative());
            }
            return left;
        }

        private Expr parseMultiplicative() {
            Expr left = parseUnary();
            while (peek().type == TokenType.SYMBOL && Set.of("*", "/", "%").contains(peek().value)) {
                String operator = advance().value;
                left = new BinaryExpr(operator, left, parseUnary());
            }
            return left;
        }

        private Expr parseUnary() {
            if (matchSymbol("-")) {
                return new UnaryExpr("-", parseUnary());
            }
            return parsePrimary();
        }

        private Expr parsePrimary() {
            Token token = peek();
            if (matchSymbol("(")) {
                Expr expression = parseExpression();
                expectSymbol(")");
                return expression;
            }
            if (token.type == TokenType.NUMBER) {
                advance();
                return new LiteralExpr(token.value.contains(".") ? Double.parseDouble(token.value) : Long.parseLong(token.value));
            }
            if (token.type == TokenType.STRING) {
                advance();
                return new LiteralExpr(token.value);
            }
            if (matchKeyword("NULL")) {
                return new NullExpr();
            }
            if (matchKeyword("TRUE")) {
                return new LiteralExpr(true);
            }
            if (matchKeyword("FALSE")) {
                return new LiteralExpr(false);
            }
            if (matchSymbol("*")) {
                return new StarExpr();
            }
            if (token.type == TokenType.IDENT || token.type == TokenType.KEYWORD) {
                String name = advance().value;
                if (matchSymbol("(")) {
                    List<Expr> args = new ArrayList<>();
                    if (!matchSymbol(")")) {
                        if (matchSymbol("*")) {
                            args.add(new StarExpr());
                        } else {
                            args.add(parseExpression());
                            while (matchSymbol(",")) {
                                args.add(parseExpression());
                            }
                        }
                        expectSymbol(")");
                    }
                    return new FunctionExpr(name, args);
                }
                if (matchSymbol(".")) {
                    return new ColumnExpr(name, expectIdentifier());
                }
                return new ColumnExpr(null, name);
            }
            throw error("unexpected token: " + token.value);
        }

        private String expectIdentifier() {
            Token token = advance();
            if (token.type != TokenType.IDENT && token.type != TokenType.KEYWORD) {
                throw error("expected identifier");
            }
            return token.value;
        }

        private Integer numberAsInt(Token token) {
            if (token.type != TokenType.NUMBER) {
                throw error("expected number");
            }
            return Integer.parseInt(token.value);
        }

        private Token peek() {
            return tokens.get(position);
        }

        private Token advance() {
            Token token = tokens.get(position);
            if (token.type != TokenType.EOF) {
                position++;
            }
            return token;
        }

        private void expect(TokenType type) {
            Token token = advance();
            if (token.type != type) {
                throw error("expected " + type + ", got " + token.type);
            }
        }

        private void expectKeyword(String value) {
            if (!matchKeyword(value)) {
                throw error("expected " + value);
            }
        }

        private boolean matchKeyword(String value) {
            if (peek().type == TokenType.KEYWORD && peek().value.equalsIgnoreCase(value)) {
                advance();
                return true;
            }
            return false;
        }

        private void expectSymbol(String value) {
            if (!matchSymbol(value)) {
                throw error("expected " + value);
            }
        }

        private boolean matchSymbol(String value) {
            if (peek().type == TokenType.SYMBOL && peek().value.equals(value)) {
                advance();
                return true;
            }
            return false;
        }

        private SqlExecutionException error(String message) {
            return new SqlExecutionException(message + " near token " + position);
        }
    }

    private enum TokenType {
        IDENT, KEYWORD, NUMBER, STRING, SYMBOL, EOF
    }

    private record Token(TokenType type, String value) {
    }

    private record SelectStatement(
        boolean distinct,
        List<SelectItem> selectItems,
        TableRef from,
        List<Join> joins,
        Expr where,
        List<Expr> groupBy,
        Expr having,
        List<OrderItem> orderBy,
        Integer limit,
        Integer offset
    ) {
    }

    private record SelectItem(Expr expression, String alias) {
    }

    private record TableRef(String name, String alias) {
    }

    private record Join(String type, TableRef table, Expr on) {
    }

    private record OrderItem(Expr expression, boolean descending) {
    }

    private interface Expr {
    }

    private record LiteralExpr(Object value) implements Expr {
    }

    private record NullExpr() implements Expr {
    }

    private record ColumnExpr(String table, String name) implements Expr {
    }

    private record StarExpr() implements Expr {
    }

    private record UnaryExpr(String operator, Expr expression) implements Expr {
    }

    private record BinaryExpr(String operator, Expr left, Expr right) implements Expr {
    }

    private record IsNullExpr(Expr expression, boolean negated) implements Expr {
    }

    private record BetweenExpr(Expr expression, Expr lower, Expr upper, boolean negated) implements Expr {
    }

    private record InExpr(Expr expression, List<Expr> values, boolean negated) implements Expr {
    }

    private record LikeExpr(Expr expression, Expr pattern, boolean negated) implements Expr {
    }

    private record FunctionExpr(String name, List<Expr> args) implements Expr {
    }

    private record RowContext(LinkedHashMap<String, Object> values) {
        RowContext merge(RowContext other) {
            LinkedHashMap<String, Object> merged = new LinkedHashMap<>(values);
            merged.putAll(other.values);
            return new RowContext(merged);
        }
    }

    private record RowFrame(RowContext row, List<RowContext> groupRows) {
    }

    private record Projection(List<String> columns, List<List<Object>> rows) {
    }
}

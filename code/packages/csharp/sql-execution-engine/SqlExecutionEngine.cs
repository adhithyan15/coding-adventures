using System.Globalization;
using System.Text.RegularExpressions;

namespace CodingAdventures.SqlExecutionEngine;

public interface IDataSource
{
    IReadOnlyList<string> Schema(string tableName);

    IReadOnlyList<IReadOnlyDictionary<string, object?>> Scan(string tableName);
}

public sealed record QueryResult(IReadOnlyList<string> Columns, IReadOnlyList<IReadOnlyList<object?>> Rows);

public sealed record ExecutionResult(bool Ok, QueryResult? Result, string? Error)
{
    public static ExecutionResult Success(QueryResult result) => new(true, result, null);

    public static ExecutionResult Failure(string error) => new(false, null, error);
}

public sealed class SqlExecutionException : Exception
{
    public SqlExecutionException(string message) : base(message)
    {
    }

    public SqlExecutionException(string message, Exception innerException) : base(message, innerException)
    {
    }
}

public sealed class InMemoryDataSource : IDataSource
{
    private readonly Dictionary<string, IReadOnlyList<string>> schemas = new(StringComparer.Ordinal);
    private readonly Dictionary<string, IReadOnlyList<IReadOnlyDictionary<string, object?>>> tables = new(StringComparer.Ordinal);

    public InMemoryDataSource AddTable(string name, IReadOnlyList<string> schema, IReadOnlyList<IReadOnlyDictionary<string, object?>> rows)
    {
        schemas[name] = schema.ToArray();
        tables[name] = rows
            .Select(row => new Dictionary<string, object?>(row, StringComparer.Ordinal))
            .Cast<IReadOnlyDictionary<string, object?>>()
            .ToArray();
        return this;
    }

    public IReadOnlyList<string> Schema(string tableName)
    {
        if (!schemas.TryGetValue(tableName, out var schema))
        {
            throw new SqlExecutionException($"table not found: {tableName}");
        }
        return schema;
    }

    public IReadOnlyList<IReadOnlyDictionary<string, object?>> Scan(string tableName)
    {
        if (!tables.TryGetValue(tableName, out var rows))
        {
            throw new SqlExecutionException($"table not found: {tableName}");
        }
        return rows
            .Select(row => new Dictionary<string, object?>(row, StringComparer.Ordinal))
            .Cast<IReadOnlyDictionary<string, object?>>()
            .ToArray();
    }
}

public static class SqlExecutionEngine
{
    private static readonly HashSet<string> Keywords = new(StringComparer.OrdinalIgnoreCase)
    {
        "SELECT", "FROM", "WHERE", "GROUP", "BY", "HAVING", "ORDER", "LIMIT", "OFFSET",
        "DISTINCT", "ALL", "JOIN", "INNER", "LEFT", "RIGHT", "FULL", "OUTER", "CROSS",
        "ON", "AS", "AND", "OR", "NOT", "IS", "NULL", "IN", "BETWEEN", "LIKE", "TRUE",
        "FALSE", "ASC", "DESC", "COUNT", "SUM", "AVG", "MIN", "MAX", "UPPER", "LOWER", "LENGTH"
    };

    public static QueryResult Execute(string sql, IDataSource dataSource)
    {
        try
        {
            var parser = new Parser(Tokenize(sql));
            return ExecuteSelect(parser.ParseStatement(), dataSource);
        }
        catch (SqlExecutionException)
        {
            throw;
        }
        catch (Exception ex)
        {
            throw new SqlExecutionException(ex.Message, ex);
        }
    }

    public static ExecutionResult TryExecute(string sql, IDataSource dataSource)
    {
        try
        {
            return ExecutionResult.Success(Execute(sql, dataSource));
        }
        catch (Exception ex)
        {
            return ExecutionResult.Failure(ex.Message);
        }
    }

    private static QueryResult ExecuteSelect(SelectStatement statement, IDataSource dataSource)
    {
        var rows = ScanTable(dataSource, statement.From.Name, statement.From.Alias);
        foreach (var join in statement.Joins)
        {
            rows = ApplyJoin(rows, ScanTable(dataSource, join.Table.Name, join.Table.Alias), join);
        }

        if (statement.Where is not null)
        {
            rows = rows.Where(row => Truthy(Eval(statement.Where, row.Values, null))).ToList();
        }

        var frames = MakeFrames(rows, statement);
        if (statement.Having is not null)
        {
            frames = frames.Where(frame => Truthy(Eval(statement.Having, frame.Row.Values, frame.GroupRows))).ToList();
        }

        if (statement.OrderBy.Count > 0)
        {
            frames.Sort(new RowFrameComparer(statement.OrderBy));
        }

        var projection = Project(frames, statement);
        var projectedRows = projection.Rows;

        if (statement.Distinct)
        {
            var seen = new HashSet<string>(StringComparer.Ordinal);
            projectedRows = projectedRows.Where(row => seen.Add(RowKey(row))).ToList();
        }

        var from = Math.Max(0, statement.Offset ?? 0);
        var to = projectedRows.Count;
        if (statement.Limit is not null)
        {
            to = Math.Min(to, from + Math.Max(0, statement.Limit.Value));
        }

        var sliced = from > projectedRows.Count
            ? new List<IReadOnlyList<object?>>()
            : projectedRows.Skip(from).Take(to - from).ToList();

        return new QueryResult(projection.Columns, sliced);
    }

    private static List<RowContext> ScanTable(IDataSource dataSource, string tableName, string alias)
    {
        var schema = dataSource.Schema(tableName);
        var rawRows = dataSource.Scan(tableName);
        var rows = new List<RowContext>();

        foreach (var raw in rawRows)
        {
            var values = new Dictionary<string, object?>(StringComparer.Ordinal);
            foreach (var column in schema)
            {
                raw.TryGetValue(column, out var value);
                values[column] = value;
                values[$"{alias}.{column}"] = value;
                values[$"{tableName}.{column}"] = value;
            }
            rows.Add(new RowContext(values));
        }

        return rows;
    }

    private static List<RowContext> ApplyJoin(List<RowContext> leftRows, List<RowContext> rightRows, Join join)
    {
        var result = new List<RowContext>();
        if (join.Type == "CROSS")
        {
            foreach (var left in leftRows)
            foreach (var right in rightRows)
            {
                result.Add(left.Merge(right));
            }
            return result;
        }

        foreach (var left in leftRows)
        {
            var matched = false;
            foreach (var right in rightRows)
            {
                var merged = left.Merge(right);
                if (join.On is null || Truthy(Eval(join.On, merged.Values, null)))
                {
                    result.Add(merged);
                    matched = true;
                }
            }
            if (!matched && join.Type == "LEFT")
            {
                result.Add(left);
            }
        }

        return result;
    }

    private static List<RowFrame> MakeFrames(List<RowContext> rows, SelectStatement statement)
    {
        var grouped = statement.GroupBy.Count > 0;
        var aggregated = HasAggregate(statement.SelectItems) || HasAggregate(statement.Having);
        if (!grouped && !aggregated)
        {
            return rows.Select(row => new RowFrame(row, null)).ToList();
        }

        if (!grouped)
        {
            return new List<RowFrame>
            {
                new(rows.FirstOrDefault() ?? new RowContext(new Dictionary<string, object?>(StringComparer.Ordinal)), rows)
            };
        }

        var groups = new Dictionary<string, List<RowContext>>(StringComparer.Ordinal);
        var order = new List<string>();
        foreach (var row in rows)
        {
            var key = RowKey(statement.GroupBy.Select(expr => Eval(expr, row.Values, null)));
            if (!groups.TryGetValue(key, out var groupRows))
            {
                groupRows = new List<RowContext>();
                groups[key] = groupRows;
                order.Add(key);
            }
            groupRows.Add(row);
        }

        return order.Select(key => new RowFrame(groups[key][0], groups[key])).ToList();
    }

    private static Projection Project(IReadOnlyList<RowFrame> frames, SelectStatement statement)
    {
        if (statement.SelectItems.Count == 1 && statement.SelectItems[0].Expression is StarExpr)
        {
            var columns = frames.Count == 0
                ? new List<string>()
                : frames[0].Row.Values.Keys.Where(key => !key.Contains('.', StringComparison.Ordinal)).Order(StringComparer.Ordinal).ToList();
            var rows = frames
                .Select(frame => (IReadOnlyList<object?>)columns.Select(column => frame.Row.Values.GetValueOrDefault(column)).ToList())
                .ToList();
            return new Projection(columns, rows);
        }

        var projectedColumns = statement.SelectItems
            .Select(item => item.Alias ?? ExpressionLabel(item.Expression))
            .ToList();
        var projectedRows = frames
            .Select(frame => (IReadOnlyList<object?>)statement.SelectItems
                .Select(item => Eval(item.Expression, frame.Row.Values, frame.GroupRows))
                .ToList())
            .ToList();
        return new Projection(projectedColumns, projectedRows);
    }

    private static object? Eval(Expr expression, IReadOnlyDictionary<string, object?> row, IReadOnlyList<RowContext>? groupRows)
    {
        return expression switch
        {
            LiteralExpr literal => literal.Value,
            NullExpr => null,
            ColumnExpr column => EvalColumn(column, row),
            UnaryExpr unary => EvalUnary(unary, row, groupRows),
            BinaryExpr binary => EvalBinary(binary, row, groupRows),
            IsNullExpr isNull => isNull.Negated
                ? Eval(isNull.Expression, row, groupRows) is not null
                : Eval(isNull.Expression, row, groupRows) is null,
            BetweenExpr between => EvalBetween(between, row, groupRows),
            InExpr inExpr => EvalIn(inExpr, row, groupRows),
            LikeExpr like => EvalLike(like, row, groupRows),
            FunctionExpr function => EvalFunction(function, row, groupRows),
            StarExpr => row,
            _ => throw new SqlExecutionException("unknown expression")
        };
    }

    private static object? EvalColumn(ColumnExpr column, IReadOnlyDictionary<string, object?> row)
    {
        if (column.Table is not null)
        {
            return row.GetValueOrDefault($"{column.Table}.{column.Name}");
        }
        if (row.TryGetValue(column.Name, out var value))
        {
            return value;
        }
        foreach (var (key, candidate) in row)
        {
            if (key.EndsWith($".{column.Name}", StringComparison.Ordinal))
            {
                return candidate;
            }
        }
        return null;
    }

    private static object? EvalUnary(UnaryExpr unary, IReadOnlyDictionary<string, object?> row, IReadOnlyList<RowContext>? groupRows)
    {
        var value = Eval(unary.Expression, row, groupRows);
        return unary.Operator switch
        {
            "NOT" => value is null ? null : !Truthy(value),
            "-" => value is null ? null : -AsDouble(value),
            _ => throw new SqlExecutionException($"unknown unary operator: {unary.Operator}")
        };
    }

    private static object? EvalBinary(BinaryExpr binary, IReadOnlyDictionary<string, object?> row, IReadOnlyList<RowContext>? groupRows)
    {
        if (binary.Operator == "AND")
        {
            var leftBool = Eval(binary.Left, row, groupRows);
            if (leftBool is not null && !Truthy(leftBool)) return false;
            var rightBool = Eval(binary.Right, row, groupRows);
            if (rightBool is not null && !Truthy(rightBool)) return false;
            return leftBool is null || rightBool is null ? null : true;
        }
        if (binary.Operator == "OR")
        {
            var leftBool = Eval(binary.Left, row, groupRows);
            if (leftBool is not null && Truthy(leftBool)) return true;
            var rightBool = Eval(binary.Right, row, groupRows);
            if (rightBool is not null && Truthy(rightBool)) return true;
            return leftBool is null || rightBool is null ? null : false;
        }

        var left = Eval(binary.Left, row, groupRows);
        var right = Eval(binary.Right, row, groupRows);
        if (left is null || right is null) return null;

        return binary.Operator switch
        {
            "+" => AsDouble(left) + AsDouble(right),
            "-" => AsDouble(left) - AsDouble(right),
            "*" => AsDouble(left) * AsDouble(right),
            "/" => AsDouble(left) / AsDouble(right),
            "%" => AsDouble(left) % AsDouble(right),
            "=" => SqlEquals(left, right),
            "!=" or "<>" => !SqlEquals(left, right),
            "<" => CompareSql(left, right) < 0,
            ">" => CompareSql(left, right) > 0,
            "<=" => CompareSql(left, right) <= 0,
            ">=" => CompareSql(left, right) >= 0,
            _ => throw new SqlExecutionException($"unknown operator: {binary.Operator}")
        };
    }

    private static object? EvalBetween(BetweenExpr between, IReadOnlyDictionary<string, object?> row, IReadOnlyList<RowContext>? groupRows)
    {
        var value = Eval(between.Expression, row, groupRows);
        var lower = Eval(between.Lower, row, groupRows);
        var upper = Eval(between.Upper, row, groupRows);
        if (value is null || lower is null || upper is null) return null;
        var result = CompareSql(value, lower) >= 0 && CompareSql(value, upper) <= 0;
        return between.Negated ? !result : result;
    }

    private static object? EvalIn(InExpr inExpr, IReadOnlyDictionary<string, object?> row, IReadOnlyList<RowContext>? groupRows)
    {
        var value = Eval(inExpr.Expression, row, groupRows);
        if (value is null) return null;
        var found = inExpr.Values.Any(candidate =>
        {
            var candidateValue = Eval(candidate, row, groupRows);
            return candidateValue is not null && SqlEquals(value, candidateValue);
        });
        return inExpr.Negated ? !found : found;
    }

    private static object? EvalLike(LikeExpr like, IReadOnlyDictionary<string, object?> row, IReadOnlyList<RowContext>? groupRows)
    {
        var value = Eval(like.Expression, row, groupRows);
        var pattern = Eval(like.Pattern, row, groupRows);
        if (value is null || pattern is null) return null;
        var regex = "^" + Regex.Escape(Convert.ToString(pattern, CultureInfo.InvariantCulture)!)
            .Replace("%", ".*", StringComparison.Ordinal)
            .Replace("_", ".", StringComparison.Ordinal) + "$";
        var result = Regex.IsMatch(Convert.ToString(value, CultureInfo.InvariantCulture)!, regex);
        return like.Negated ? !result : result;
    }

    private static object? EvalFunction(FunctionExpr function, IReadOnlyDictionary<string, object?> row, IReadOnlyList<RowContext>? groupRows)
    {
        var name = function.Name.ToUpperInvariant();
        if (new[] { "COUNT", "SUM", "AVG", "MIN", "MAX" }.Contains(name, StringComparer.Ordinal))
        {
            if (groupRows is null) throw new SqlExecutionException($"aggregate used outside grouped context: {name}");
            if (name == "COUNT")
            {
                if (function.Args.Count == 1 && function.Args[0] is StarExpr) return groupRows.Count;
                return groupRows.Count(groupRow => Eval(function.Args[0], groupRow.Values, null) is not null);
            }

            var values = groupRows
                .Select(groupRow => Eval(function.Args[0], groupRow.Values, null))
                .Where(value => value is not null)
                .ToList();
            if (values.Count == 0) return null;
            return name switch
            {
                "SUM" => values.Sum(value => AsDouble(value!)),
                "AVG" => values.Average(value => AsDouble(value!)),
                "MIN" => values.Aggregate((best, next) => CompareSql(next, best) < 0 ? next : best),
                "MAX" => values.Aggregate((best, next) => CompareSql(next, best) > 0 ? next : best),
                _ => throw new SqlExecutionException($"unknown aggregate: {name}")
            };
        }

        var argument = function.Args.Count == 0 ? null : Eval(function.Args[0], row, groupRows);
        if (argument is null) return null;
        return name switch
        {
            "UPPER" => Convert.ToString(argument, CultureInfo.InvariantCulture)!.ToUpperInvariant(),
            "LOWER" => Convert.ToString(argument, CultureInfo.InvariantCulture)!.ToLowerInvariant(),
            "LENGTH" => Convert.ToString(argument, CultureInfo.InvariantCulture)!.Length,
            _ => throw new SqlExecutionException($"unknown function: {name}")
        };
    }

    private static bool HasAggregate(IEnumerable<SelectItem> items) => items.Any(item => HasAggregate(item.Expression));

    private static bool HasAggregate(Expr? expression)
    {
        return expression switch
        {
            null => false,
            FunctionExpr function when new[] { "COUNT", "SUM", "AVG", "MIN", "MAX" }.Contains(function.Name.ToUpperInvariant()) => true,
            FunctionExpr function => function.Args.Any(HasAggregate),
            BinaryExpr binary => HasAggregate(binary.Left) || HasAggregate(binary.Right),
            UnaryExpr unary => HasAggregate(unary.Expression),
            IsNullExpr isNull => HasAggregate(isNull.Expression),
            BetweenExpr between => HasAggregate(between.Expression) || HasAggregate(between.Lower) || HasAggregate(between.Upper),
            InExpr inExpr => HasAggregate(inExpr.Expression) || inExpr.Values.Any(HasAggregate),
            LikeExpr like => HasAggregate(like.Expression) || HasAggregate(like.Pattern),
            _ => false
        };
    }

    private static bool Truthy(object? value) => value switch
    {
        null => false,
        bool boolean => boolean,
        int integer => integer != 0,
        long integer => integer != 0,
        double number => number != 0,
        decimal number => number != 0,
        _ => !string.IsNullOrEmpty(Convert.ToString(value, CultureInfo.InvariantCulture))
    };

    private static bool SqlEquals(object left, object right)
    {
        return left is IConvertible && right is IConvertible && IsNumeric(left) && IsNumeric(right)
            ? Math.Abs(AsDouble(left) - AsDouble(right)) < double.Epsilon
            : Equals(left, right);
    }

    private static int CompareSql(object? left, object? right)
    {
        if (left is null && right is null) return 0;
        if (left is null) return 1;
        if (right is null) return -1;
        if (IsNumeric(left) && IsNumeric(right)) return AsDouble(left).CompareTo(AsDouble(right));
        return string.Compare(Convert.ToString(left, CultureInfo.InvariantCulture), Convert.ToString(right, CultureInfo.InvariantCulture), StringComparison.Ordinal);
    }

    private static bool IsNumeric(object value) => value is byte or sbyte or short or ushort or int or uint or long or ulong or float or double or decimal;

    private static double AsDouble(object value) => Convert.ToDouble(value, CultureInfo.InvariantCulture);

    private static string ExpressionLabel(Expr expression) => expression switch
    {
        ColumnExpr column => column.Name,
        FunctionExpr function when function.Args.Count == 1 && function.Args[0] is StarExpr => $"{function.Name.ToUpperInvariant()}(*)",
        FunctionExpr function => $"{function.Name.ToUpperInvariant()}(...)",
        LiteralExpr literal => Convert.ToString(literal.Value, CultureInfo.InvariantCulture) ?? "NULL",
        _ => "?"
    };

    private static string RowKey(IEnumerable<object?> values)
    {
        return string.Join("\u0000", values.Select(value => value is null ? "<NULL>" : $"{value.GetType().FullName}:{value}"));
    }

    private static IReadOnlyList<Token> Tokenize(string sql)
    {
        var tokens = new List<Token>();
        var index = 0;
        while (index < sql.Length)
        {
            var ch = sql[index];
            if (char.IsWhiteSpace(ch))
            {
                index++;
                continue;
            }
            if (ch == '-' && index + 1 < sql.Length && sql[index + 1] == '-')
            {
                index += 2;
                while (index < sql.Length && sql[index] != '\n') index++;
                continue;
            }
            if (ch == '\'')
            {
                index++;
                var value = "";
                while (index < sql.Length)
                {
                    if (sql[index] == '\'' && index + 1 < sql.Length && sql[index + 1] == '\'')
                    {
                        value += "'";
                        index += 2;
                    }
                    else if (sql[index] == '\'')
                    {
                        index++;
                        break;
                    }
                    else
                    {
                        value += sql[index++];
                    }
                }
                tokens.Add(new Token(TokenType.String, value));
                continue;
            }
            if (char.IsDigit(ch) || (ch == '.' && index + 1 < sql.Length && char.IsDigit(sql[index + 1])))
            {
                var start = index++;
                while (index < sql.Length && (char.IsDigit(sql[index]) || sql[index] == '.')) index++;
                tokens.Add(new Token(TokenType.Number, sql[start..index]));
                continue;
            }
            if (char.IsLetter(ch) || ch == '_')
            {
                var start = index++;
                while (index < sql.Length && (char.IsLetterOrDigit(sql[index]) || sql[index] == '_')) index++;
                var value = sql[start..index];
                tokens.Add(new Token(Keywords.Contains(value) ? TokenType.Keyword : TokenType.Identifier, value));
                continue;
            }
            if (ch is '"' or '`')
            {
                var quote = ch;
                var start = ++index;
                while (index < sql.Length && sql[index] != quote) index++;
                tokens.Add(new Token(TokenType.Identifier, sql[start..index]));
                index++;
                continue;
            }
            if (index + 1 < sql.Length && new[] { "!=", "<>", "<=", ">=" }.Contains(sql.Substring(index, 2)))
            {
                tokens.Add(new Token(TokenType.Symbol, sql.Substring(index, 2)));
                index += 2;
                continue;
            }
            if ("=<>+-*/%(),.;".Contains(ch, StringComparison.Ordinal))
            {
                tokens.Add(new Token(TokenType.Symbol, ch.ToString()));
            }
            index++;
        }
        tokens.Add(new Token(TokenType.End, ""));
        return tokens;
    }

    private sealed class RowFrameComparer(IReadOnlyList<OrderItem> orderBy) : IComparer<RowFrame>
    {
        public int Compare(RowFrame? left, RowFrame? right)
        {
            if (left is null || right is null) return 0;
            foreach (var item in orderBy)
            {
                var cmp = CompareSql(Eval(item.Expression, left.Row.Values, left.GroupRows), Eval(item.Expression, right.Row.Values, right.GroupRows));
                if (cmp != 0) return item.Descending ? -cmp : cmp;
            }
            return 0;
        }
    }

    private sealed class Parser(IReadOnlyList<Token> tokens)
    {
        private int position;

        public SelectStatement ParseStatement()
        {
            ExpectKeyword("SELECT");
            var distinct = MatchKeyword("DISTINCT");
            MatchKeyword("ALL");
            var selectItems = ParseSelectList();
            ExpectKeyword("FROM");
            var from = ParseTableRef();
            var joins = ParseJoins();
            var where = MatchKeyword("WHERE") ? ParseExpression() : null;
            var groupBy = new List<Expr>();
            if (MatchKeyword("GROUP"))
            {
                ExpectKeyword("BY");
                groupBy = ParseExpressionList();
            }
            var having = MatchKeyword("HAVING") ? ParseExpression() : null;
            var orderBy = new List<OrderItem>();
            if (MatchKeyword("ORDER"))
            {
                ExpectKeyword("BY");
                orderBy = ParseOrderList();
            }
            var limit = MatchKeyword("LIMIT") ? NumberAsInt(Advance()) : (int?)null;
            var offset = MatchKeyword("OFFSET") ? NumberAsInt(Advance()) : (int?)null;
            MatchSymbol(";");
            Expect(TokenType.End);
            return new SelectStatement(distinct, selectItems, from, joins, where, groupBy, having, orderBy, limit, offset);
        }

        private List<SelectItem> ParseSelectList()
        {
            var items = new List<SelectItem>();
            do
            {
                if (MatchSymbol("*"))
                {
                    items.Add(new SelectItem(new StarExpr(), null));
                }
                else
                {
                    var expression = ParseExpression();
                    string? alias = null;
                    if (MatchKeyword("AS")) alias = ExpectIdentifier();
                    else if (Peek().Type == TokenType.Identifier) alias = Advance().Value;
                    items.Add(new SelectItem(expression, alias));
                }
            } while (MatchSymbol(","));
            return items;
        }

        private TableRef ParseTableRef()
        {
            var name = ExpectIdentifier();
            var alias = name;
            if (MatchKeyword("AS")) alias = ExpectIdentifier();
            else if (Peek().Type == TokenType.Identifier) alias = Advance().Value;
            return new TableRef(name, alias);
        }

        private List<Join> ParseJoins()
        {
            var joins = new List<Join>();
            while (true)
            {
                string? type = null;
                if (MatchKeyword("INNER"))
                {
                    type = "INNER";
                    ExpectKeyword("JOIN");
                }
                else if (MatchKeyword("LEFT"))
                {
                    type = "LEFT";
                    MatchKeyword("OUTER");
                    ExpectKeyword("JOIN");
                }
                else if (MatchKeyword("CROSS"))
                {
                    type = "CROSS";
                    ExpectKeyword("JOIN");
                }
                else if (MatchKeyword("JOIN"))
                {
                    type = "INNER";
                }
                if (type is null) break;
                var table = ParseTableRef();
                Expr? on = null;
                if (type != "CROSS")
                {
                    ExpectKeyword("ON");
                    on = ParseExpression();
                }
                joins.Add(new Join(type, table, on));
            }
            return joins;
        }

        private List<Expr> ParseExpressionList()
        {
            var expressions = new List<Expr>();
            do expressions.Add(ParseExpression());
            while (MatchSymbol(","));
            return expressions;
        }

        private List<OrderItem> ParseOrderList()
        {
            var items = new List<OrderItem>();
            do
            {
                var expr = ParseExpression();
                var desc = MatchKeyword("DESC");
                if (!desc) MatchKeyword("ASC");
                items.Add(new OrderItem(expr, desc));
            } while (MatchSymbol(","));
            return items;
        }

        private Expr ParseExpression() => ParseOr();

        private Expr ParseOr()
        {
            var left = ParseAnd();
            while (MatchKeyword("OR")) left = new BinaryExpr("OR", left, ParseAnd());
            return left;
        }

        private Expr ParseAnd()
        {
            var left = ParseNot();
            while (MatchKeyword("AND")) left = new BinaryExpr("AND", left, ParseNot());
            return left;
        }

        private Expr ParseNot() => MatchKeyword("NOT") ? new UnaryExpr("NOT", ParseNot()) : ParseComparison();

        private Expr ParseComparison()
        {
            var left = ParseAdditive();
            if (MatchKeyword("IS"))
            {
                var negated = MatchKeyword("NOT");
                ExpectKeyword("NULL");
                return new IsNullExpr(left, negated);
            }
            if (MatchKeyword("NOT"))
            {
                if (MatchKeyword("BETWEEN"))
                {
                    var lower = ParseAdditive();
                    ExpectKeyword("AND");
                    return new BetweenExpr(left, lower, ParseAdditive(), true);
                }
                if (MatchKeyword("IN")) return new InExpr(left, ParseInValues(), true);
                if (MatchKeyword("LIKE")) return new LikeExpr(left, ParseAdditive(), true);
                throw Error("expected BETWEEN, IN, or LIKE after NOT");
            }
            if (MatchKeyword("BETWEEN"))
            {
                var lower = ParseAdditive();
                ExpectKeyword("AND");
                return new BetweenExpr(left, lower, ParseAdditive(), false);
            }
            if (MatchKeyword("IN")) return new InExpr(left, ParseInValues(), false);
            if (MatchKeyword("LIKE")) return new LikeExpr(left, ParseAdditive(), false);
            if (Peek().Type == TokenType.Symbol && new[] { "=", "!=", "<>", "<", ">", "<=", ">=" }.Contains(Peek().Value))
            {
                var op = Advance().Value;
                return new BinaryExpr(op, left, ParseAdditive());
            }
            return left;
        }

        private List<Expr> ParseInValues()
        {
            ExpectSymbol("(");
            var values = ParseExpressionList();
            ExpectSymbol(")");
            return values;
        }

        private Expr ParseAdditive()
        {
            var left = ParseMultiplicative();
            while (Peek().Type == TokenType.Symbol && new[] { "+", "-" }.Contains(Peek().Value))
            {
                var op = Advance().Value;
                left = new BinaryExpr(op, left, ParseMultiplicative());
            }
            return left;
        }

        private Expr ParseMultiplicative()
        {
            var left = ParseUnary();
            while (Peek().Type == TokenType.Symbol && new[] { "*", "/", "%" }.Contains(Peek().Value))
            {
                var op = Advance().Value;
                left = new BinaryExpr(op, left, ParseUnary());
            }
            return left;
        }

        private Expr ParseUnary() => MatchSymbol("-") ? new UnaryExpr("-", ParseUnary()) : ParsePrimary();

        private Expr ParsePrimary()
        {
            var token = Peek();
            if (MatchSymbol("("))
            {
                var expr = ParseExpression();
                ExpectSymbol(")");
                return expr;
            }
            if (token.Type == TokenType.Number)
            {
                Advance();
                return token.Value.Contains('.', StringComparison.Ordinal)
                    ? new LiteralExpr(double.Parse(token.Value, CultureInfo.InvariantCulture))
                    : new LiteralExpr(long.Parse(token.Value, CultureInfo.InvariantCulture));
            }
            if (token.Type == TokenType.String)
            {
                Advance();
                return new LiteralExpr(token.Value);
            }
            if (MatchKeyword("NULL")) return new NullExpr();
            if (MatchKeyword("TRUE")) return new LiteralExpr(true);
            if (MatchKeyword("FALSE")) return new LiteralExpr(false);
            if (MatchSymbol("*")) return new StarExpr();
            if (token.Type is TokenType.Identifier or TokenType.Keyword)
            {
                var name = Advance().Value;
                if (MatchSymbol("("))
                {
                    var args = new List<Expr>();
                    if (!MatchSymbol(")"))
                    {
                        if (MatchSymbol("*")) args.Add(new StarExpr());
                        else
                        {
                            args.Add(ParseExpression());
                            while (MatchSymbol(",")) args.Add(ParseExpression());
                        }
                        ExpectSymbol(")");
                    }
                    return new FunctionExpr(name, args);
                }
                if (MatchSymbol(".")) return new ColumnExpr(name, ExpectIdentifier());
                return new ColumnExpr(null, name);
            }
            throw Error($"unexpected token: {token.Value}");
        }

        private string ExpectIdentifier()
        {
            var token = Advance();
            if (token.Type is not (TokenType.Identifier or TokenType.Keyword)) throw Error("expected identifier");
            return token.Value;
        }

        private int NumberAsInt(Token token)
        {
            if (token.Type != TokenType.Number) throw Error("expected number");
            return int.Parse(token.Value, CultureInfo.InvariantCulture);
        }

        private Token Peek() => tokens[position];

        private Token Advance()
        {
            var token = tokens[position];
            if (token.Type != TokenType.End) position++;
            return token;
        }

        private void Expect(TokenType type)
        {
            if (Advance().Type != type) throw Error($"expected {type}");
        }

        private void ExpectKeyword(string value)
        {
            if (!MatchKeyword(value)) throw Error($"expected {value}");
        }

        private bool MatchKeyword(string value)
        {
            if (Peek().Type == TokenType.Keyword && string.Equals(Peek().Value, value, StringComparison.OrdinalIgnoreCase))
            {
                Advance();
                return true;
            }
            return false;
        }

        private void ExpectSymbol(string value)
        {
            if (!MatchSymbol(value)) throw Error($"expected {value}");
        }

        private bool MatchSymbol(string value)
        {
            if (Peek().Type == TokenType.Symbol && Peek().Value == value)
            {
                Advance();
                return true;
            }
            return false;
        }

        private SqlExecutionException Error(string message) => new($"{message} near token {position}");
    }

    private enum TokenType { Identifier, Keyword, Number, String, Symbol, End }

    private sealed record Token(TokenType Type, string Value);

    private sealed record SelectStatement(
        bool Distinct,
        IReadOnlyList<SelectItem> SelectItems,
        TableRef From,
        IReadOnlyList<Join> Joins,
        Expr? Where,
        IReadOnlyList<Expr> GroupBy,
        Expr? Having,
        IReadOnlyList<OrderItem> OrderBy,
        int? Limit,
        int? Offset);

    private sealed record SelectItem(Expr Expression, string? Alias);

    private sealed record TableRef(string Name, string Alias);

    private sealed record Join(string Type, TableRef Table, Expr? On);

    private sealed record OrderItem(Expr Expression, bool Descending);

    private abstract record Expr;
    private sealed record LiteralExpr(object? Value) : Expr;
    private sealed record NullExpr : Expr;
    private sealed record ColumnExpr(string? Table, string Name) : Expr;
    private sealed record StarExpr : Expr;
    private sealed record UnaryExpr(string Operator, Expr Expression) : Expr;
    private sealed record BinaryExpr(string Operator, Expr Left, Expr Right) : Expr;
    private sealed record IsNullExpr(Expr Expression, bool Negated) : Expr;
    private sealed record BetweenExpr(Expr Expression, Expr Lower, Expr Upper, bool Negated) : Expr;
    private sealed record InExpr(Expr Expression, IReadOnlyList<Expr> Values, bool Negated) : Expr;
    private sealed record LikeExpr(Expr Expression, Expr Pattern, bool Negated) : Expr;
    private sealed record FunctionExpr(string Name, IReadOnlyList<Expr> Args) : Expr;

    private sealed record RowContext(Dictionary<string, object?> Values)
    {
        public RowContext Merge(RowContext other)
        {
            var merged = new Dictionary<string, object?>(Values, StringComparer.Ordinal);
            foreach (var (key, value) in other.Values) merged[key] = value;
            return new RowContext(merged);
        }
    }

    private sealed record RowFrame(RowContext Row, IReadOnlyList<RowContext>? GroupRows);

    private sealed record Projection(IReadOnlyList<string> Columns, List<IReadOnlyList<object?>> Rows);
}

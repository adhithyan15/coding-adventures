using System.Globalization;
using System.Text;
using System.Text.RegularExpressions;

namespace CodingAdventures.MiniSqlite;

/// <summary>Entry point for the Level 0 in-memory mini-sqlite facade.</summary>
public static class MiniSqlite
{
    public const string ApiLevel = "2.0";
    public const int ThreadSafety = 1;
    public const string ParamStyle = "qmark";

    public static Connection Connect(string database, ConnectionOptions? options = null)
    {
        if (database != ":memory:")
        {
            throw new MiniSqliteException("NotSupportedError", "C# mini-sqlite supports only :memory: in Level 0");
        }

        return new Connection(options ?? ConnectionOptions.Default);
    }
}

public sealed record ConnectionOptions(bool Autocommit = false)
{
    public static ConnectionOptions Default { get; } = new();
}

public sealed record Column(string Name);

public sealed class MiniSqliteException(string kind, string message) : Exception(message)
{
    public string Kind { get; } = kind;
}

public sealed class Connection(ConnectionOptions options) : IDisposable
{
    private Database _db = new();
    private readonly bool _autocommit = options.Autocommit;
    private Database? _snapshot;
    private bool _closed;

    public Cursor Cursor()
    {
        AssertOpen();
        return new Cursor(this);
    }

    public Cursor Execute(string sql, params object?[] parameters) => Cursor().Execute(sql, parameters);

    public Cursor Execute(string sql, IReadOnlyList<object?> parameters) => Cursor().Execute(sql, parameters);

    public Cursor ExecuteMany(string sql, IEnumerable<IReadOnlyList<object?>> parameterSets)
        => Cursor().ExecuteMany(sql, parameterSets);

    public void Commit()
    {
        AssertOpen();
        _snapshot = null;
    }

    public void Rollback()
    {
        AssertOpen();
        if (_snapshot is not null)
        {
            _db = _snapshot.Copy();
            _snapshot = null;
        }
    }

    public void Dispose()
    {
        if (_closed)
        {
            return;
        }

        if (_snapshot is not null)
        {
            _db = _snapshot.Copy();
        }

        _snapshot = null;
        _closed = true;
    }

    internal Result ExecuteBound(string sql, IReadOnlyList<object?> parameters)
    {
        AssertOpen();
        var bound = SqlText.BindParameters(sql, parameters);
        try
        {
            return SqlText.FirstKeyword(bound) switch
            {
                "BEGIN" => Begin(),
                "COMMIT" => CommitResult(),
                "ROLLBACK" => RollbackResult(),
                "SELECT" => _db.Select(bound),
                "CREATE" => WithSnapshot(() => _db.Create(SqlText.ParseCreate(bound))),
                "DROP" => WithSnapshot(() => _db.Drop(SqlText.ParseDrop(bound))),
                "INSERT" => WithSnapshot(() => _db.Insert(SqlText.ParseInsert(bound))),
                "UPDATE" => WithSnapshot(() => _db.Update(SqlText.ParseUpdate(bound))),
                "DELETE" => WithSnapshot(() => _db.Delete(SqlText.ParseDelete(bound))),
                _ => throw new ArgumentException("unsupported SQL statement"),
            };
        }
        catch (MiniSqliteException)
        {
            throw;
        }
        catch (Exception ex)
        {
            throw new MiniSqliteException("OperationalError", ex.Message);
        }
    }

    private Result Begin()
    {
        EnsureSnapshot();
        return Result.Empty(0);
    }

    private Result CommitResult()
    {
        _snapshot = null;
        return Result.Empty(0);
    }

    private Result RollbackResult()
    {
        if (_snapshot is not null)
        {
            _db = _snapshot.Copy();
            _snapshot = null;
        }
        return Result.Empty(0);
    }

    private Result WithSnapshot(Func<Result> action)
    {
        EnsureSnapshot();
        return action();
    }

    private void EnsureSnapshot()
    {
        if (!_autocommit && _snapshot is null)
        {
            _snapshot = _db.Copy();
        }
    }

    private void AssertOpen()
    {
        if (_closed)
        {
            throw new MiniSqliteException("ProgrammingError", "connection is closed");
        }
    }
}

public sealed class Cursor(Connection connection) : IDisposable
{
    private IReadOnlyList<IReadOnlyList<object?>> _rows = Array.Empty<IReadOnlyList<object?>>();
    private int _offset;
    private bool _closed;

    public IReadOnlyList<Column> Description { get; private set; } = Array.Empty<Column>();
    public int RowCount { get; private set; } = -1;
    public object? LastRowId { get; private set; }
    public int ArraySize { get; set; } = 1;

    public Cursor Execute(string sql, params object?[] parameters) => Execute(sql, (IReadOnlyList<object?>)parameters);

    public Cursor Execute(string sql, IReadOnlyList<object?> parameters)
    {
        if (_closed)
        {
            throw new MiniSqliteException("ProgrammingError", "cursor is closed");
        }

        var result = connection.ExecuteBound(sql, parameters);
        _rows = result.Rows;
        _offset = 0;
        RowCount = result.RowsAffected;
        Description = result.Columns.Select(column => new Column(column)).ToArray();
        return this;
    }

    public Cursor ExecuteMany(string sql, IEnumerable<IReadOnlyList<object?>> parameterSets)
    {
        var total = 0;
        var any = false;
        foreach (var parameters in parameterSets)
        {
            any = true;
            Execute(sql, parameters);
            if (RowCount > 0)
            {
                total += RowCount;
            }
        }

        if (any)
        {
            RowCount = total;
        }

        return this;
    }

    public IReadOnlyList<object?>? FetchOne()
    {
        if (_closed || _offset >= _rows.Count)
        {
            return null;
        }

        return _rows[_offset++];
    }

    public IReadOnlyList<IReadOnlyList<object?>> FetchMany() => FetchMany(ArraySize);

    public IReadOnlyList<IReadOnlyList<object?>> FetchMany(int size)
    {
        if (_closed)
        {
            return Array.Empty<IReadOnlyList<object?>>();
        }

        var rows = new List<IReadOnlyList<object?>>();
        for (var i = 0; i < size; i++)
        {
            var row = FetchOne();
            if (row is null)
            {
                break;
            }

            rows.Add(row);
        }

        return rows;
    }

    public IReadOnlyList<IReadOnlyList<object?>> FetchAll()
    {
        if (_closed)
        {
            return Array.Empty<IReadOnlyList<object?>>();
        }

        var rows = new List<IReadOnlyList<object?>>();
        while (FetchOne() is { } row)
        {
            rows.Add(row);
        }

        return rows;
    }

    public void Dispose()
    {
        _closed = true;
        _rows = Array.Empty<IReadOnlyList<object?>>();
        Description = Array.Empty<Column>();
    }
}

internal sealed record Result(IReadOnlyList<string> Columns, IReadOnlyList<IReadOnlyList<object?>> Rows, int RowsAffected)
{
    public static Result Empty(int rowsAffected) => new(Array.Empty<string>(), Array.Empty<IReadOnlyList<object?>>(), rowsAffected);
}

internal sealed record CreateStmt(string Table, IReadOnlyList<string> Columns, bool IfNotExists);
internal sealed record DropStmt(string Table, bool IfExists);
internal sealed record InsertStmt(string Table, IReadOnlyList<string> Columns, IReadOnlyList<IReadOnlyList<object?>> Rows);
internal sealed record Assignment(string Column, object? Value);
internal sealed record UpdateStmt(string Table, IReadOnlyList<Assignment> Assignments, string Where);
internal sealed record DeleteStmt(string Table, string Where);
internal sealed record Split(string Left, string Right);
internal sealed record OperatorAt(int Index, string Operator);

internal sealed class Table(IReadOnlyList<string> columns)
{
    public List<string> Columns { get; } = columns.ToList();
    public List<Dictionary<string, object?>> Rows { get; } = [];

    public Table Copy()
    {
        var copy = new Table(Columns);
        foreach (var row in Rows)
        {
            copy.Rows.Add(new Dictionary<string, object?>(row));
        }

        return copy;
    }
}

internal sealed class Database
{
    private readonly Dictionary<string, Table> _tables = new(StringComparer.OrdinalIgnoreCase);

    public Database Copy()
    {
        var copy = new Database();
        foreach (var (name, table) in _tables)
        {
            copy._tables[name] = table.Copy();
        }

        return copy;
    }

    public Result Create(CreateStmt stmt)
    {
        if (_tables.ContainsKey(stmt.Table))
        {
            if (stmt.IfNotExists)
            {
                return Result.Empty(0);
            }

            throw new ArgumentException($"table already exists: {stmt.Table}");
        }

        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var column in stmt.Columns)
        {
            if (!seen.Add(column))
            {
                throw new ArgumentException($"duplicate column: {column}");
            }
        }

        _tables[stmt.Table] = new Table(stmt.Columns);
        return Result.Empty(0);
    }

    public Result Drop(DropStmt stmt)
    {
        if (!_tables.ContainsKey(stmt.Table))
        {
            if (stmt.IfExists)
            {
                return Result.Empty(0);
            }

            throw new ArgumentException($"no such table: {stmt.Table}");
        }

        _tables.Remove(stmt.Table);
        return Result.Empty(0);
    }

    public Result Insert(InsertStmt stmt)
    {
        var table = GetTable(stmt.Table);
        var columns = stmt.Columns.Count == 0
            ? table.Columns.ToArray()
            : stmt.Columns.Select(column => SqlText.CanonicalColumn(table, column)).ToArray();

        foreach (var values in stmt.Rows)
        {
            if (values.Count != columns.Length)
            {
                throw new ArgumentException($"INSERT expected {columns.Length} values, got {values.Count}");
            }

            var row = table.Columns.ToDictionary(column => column, _ => (object?)null, StringComparer.OrdinalIgnoreCase);
            for (var i = 0; i < columns.Length; i++)
            {
                row[columns[i]] = values[i];
            }

            table.Rows.Add(row);
        }

        return Result.Empty(stmt.Rows.Count);
    }

    public Result Update(UpdateStmt stmt)
    {
        var table = GetTable(stmt.Table);
        var matches = MatchingRows(table, stmt.Where);
        var assignments = stmt.Assignments
            .Select(assignment => new Assignment(SqlText.CanonicalColumn(table, assignment.Column), assignment.Value))
            .ToArray();

        foreach (var row in matches)
        {
            foreach (var assignment in assignments)
            {
                row[assignment.Column] = assignment.Value;
            }
        }

        return Result.Empty(matches.Count);
    }

    public Result Delete(DeleteStmt stmt)
    {
        var table = GetTable(stmt.Table);
        var matches = MatchingRows(table, stmt.Where);
        foreach (var row in matches)
        {
            table.Rows.Remove(row);
        }

        return Result.Empty(matches.Count);
    }

    public Result Select(string sql)
    {
        var body = Regex.Replace(SqlText.StripTrailingSemicolon(sql), @"^\s*SELECT\s+", "", RegexOptions.IgnoreCase);
        var fromSplit = SqlText.SplitTopLevelKeyword(body, "FROM");
        if (fromSplit.Right.Length == 0)
        {
            throw new ArgumentException("invalid SELECT statement");
        }

        var columnSql = fromSplit.Left;
        var rest = fromSplit.Right;
        var tableName = SqlText.IdentifierAtStart(rest) ?? throw new ArgumentException("invalid SELECT statement");
        rest = SqlText.Trim(rest[tableName.Length..]);
        var orderSplit = SqlText.SplitTopLevelKeyword(rest, "ORDER BY");
        var whereSplit = SqlText.SplitTopLevelKeyword(orderSplit.Left, "WHERE");

        var table = GetTable(tableName);
        var ordered = MatchingRows(table, whereSplit.Right).ToList();
        SqlText.ApplyOrder(table, ordered, orderSplit.Right);

        var selectedColumns = SqlText.ParseSelectedColumns(table, columnSql);
        var rows = ordered
            .Select(row => (IReadOnlyList<object?>)selectedColumns.Select(column => SqlText.ValueOfColumn(table, row, column)).ToArray())
            .ToArray();

        return new Result(selectedColumns, rows, -1);
    }

    private Table GetTable(string tableName)
    {
        if (!_tables.TryGetValue(tableName, out var table))
        {
            throw new ArgumentException($"no such table: {tableName}");
        }

        return table;
    }

    private List<Dictionary<string, object?>> MatchingRows(Table table, string whereSql)
    {
        return string.IsNullOrWhiteSpace(whereSql)
            ? table.Rows.ToList()
            : table.Rows.Where(row => SqlText.MatchesWhere(table, row, whereSql)).ToList();
    }
}

internal static class SqlText
{
    public static string Trim(string? value) => value?.Trim() ?? "";

    public static string StripTrailingSemicolon(string sql) => Regex.Replace(Trim(sql), @";\s*$", "");

    public static string FirstKeyword(string sql)
    {
        var value = Trim(sql);
        var end = 0;
        while (end < value.Length && (char.IsLetter(value[end]) || value[end] == '_'))
        {
            end++;
        }

        return value[..end].ToUpperInvariant();
    }

    public static string BindParameters(string sql, IReadOnlyList<object?> parameters)
    {
        var outSql = new StringBuilder();
        var index = 0;
        var i = 0;
        while (i < sql.Length)
        {
            var ch = sql[i];
            if (ch is '\'' or '"')
            {
                var next = ReadQuoted(sql, i, ch);
                outSql.Append(sql, i, next - i);
                i = next;
            }
            else if (ch == '-' && i + 1 < sql.Length && sql[i + 1] == '-')
            {
                var next = i + 2;
                while (next < sql.Length && sql[next] != '\n')
                {
                    next++;
                }

                outSql.Append(sql, i, next - i);
                i = next;
            }
            else if (ch == '/' && i + 1 < sql.Length && sql[i + 1] == '*')
            {
                var next = i + 2;
                while (next + 1 < sql.Length && sql.Substring(next, 2) != "*/")
                {
                    next++;
                }

                next = Math.Min(next + 2, sql.Length);
                outSql.Append(sql, i, next - i);
                i = next;
            }
            else if (ch == '?')
            {
                if (index >= parameters.Count)
                {
                    throw new MiniSqliteException("ProgrammingError", "not enough parameters for SQL statement");
                }

                outSql.Append(ToSqlLiteral(parameters[index++]));
                i++;
            }
            else
            {
                outSql.Append(ch);
                i++;
            }
        }

        if (index < parameters.Count)
        {
            throw new MiniSqliteException("ProgrammingError", "too many parameters for SQL statement");
        }

        return outSql.ToString();
    }

    public static CreateStmt ParseCreate(string sql)
    {
        var stripped = StripTrailingSemicolon(sql);
        var ifNotExists = Regex.IsMatch(stripped, @"^\s*CREATE\s+TABLE\s+IF\s+NOT\s+EXISTS\s+", RegexOptions.IgnoreCase);
        var prefix = ifNotExists
            ? @"^\s*CREATE\s+TABLE\s+IF\s+NOT\s+EXISTS\s+"
            : @"^\s*CREATE\s+TABLE\s+";
        var restStart = Regex.Replace(stripped, prefix, "", RegexOptions.IgnoreCase);
        var table = IdentifierAtStart(restStart) ?? throw new ArgumentException("invalid CREATE TABLE statement");
        var rest = Trim(restStart[table.Length..]);
        if (!rest.StartsWith('(') || !rest.EndsWith(')'))
        {
            throw new ArgumentException("invalid CREATE TABLE statement");
        }

        var columns = SplitTopLevel(rest[1..^1], ',')
            .Select(IdentifierAtStart)
            .Where(name => name is not null)
            .Cast<string>()
            .ToArray();
        if (columns.Length == 0)
        {
            throw new ArgumentException("CREATE TABLE requires at least one column");
        }

        return new CreateStmt(table, columns, ifNotExists);
    }

    public static DropStmt ParseDrop(string sql)
    {
        var stripped = StripTrailingSemicolon(sql);
        var ifExists = Regex.IsMatch(stripped, @"^\s*DROP\s+TABLE\s+IF\s+EXISTS\s+", RegexOptions.IgnoreCase);
        var prefix = ifExists ? @"^\s*DROP\s+TABLE\s+IF\s+EXISTS\s+" : @"^\s*DROP\s+TABLE\s+";
        var rest = Regex.Replace(stripped, prefix, "", RegexOptions.IgnoreCase);
        var table = IdentifierAtStart(rest);
        if (table is null || Trim(rest[table.Length..]).Length != 0)
        {
            throw new ArgumentException("invalid DROP TABLE statement");
        }

        return new DropStmt(table, ifExists);
    }

    public static InsertStmt ParseInsert(string sql)
    {
        var stripped = StripTrailingSemicolon(sql);
        var start = Regex.Replace(stripped, @"^\s*INSERT\s+INTO\s+", "", RegexOptions.IgnoreCase);
        if (start == stripped)
        {
            throw new ArgumentException("invalid INSERT statement");
        }

        var table = IdentifierAtStart(start) ?? throw new ArgumentException("invalid INSERT statement");
        var rest = Trim(start[table.Length..]);
        var columns = new List<string>();
        if (rest.StartsWith('('))
        {
            var close = FindMatchingParen(rest, 0);
            if (close < 0)
            {
                throw new ArgumentException("invalid INSERT statement");
            }

            columns.AddRange(SplitTopLevel(rest[1..close], ',').Select(Trim));
            rest = Trim(rest[(close + 1)..]);
        }

        if (!rest.StartsWith("VALUES", StringComparison.OrdinalIgnoreCase))
        {
            throw new ArgumentException("invalid INSERT statement");
        }

        return new InsertStmt(table, columns, ParseValueRows(rest["VALUES".Length..]));
    }

    public static UpdateStmt ParseUpdate(string sql)
    {
        var stripped = StripTrailingSemicolon(sql);
        var start = Regex.Replace(stripped, @"^\s*UPDATE\s+", "", RegexOptions.IgnoreCase);
        if (start == stripped)
        {
            throw new ArgumentException("invalid UPDATE statement");
        }

        var table = IdentifierAtStart(start) ?? throw new ArgumentException("invalid UPDATE statement");
        var rest = Trim(start[table.Length..]);
        if (!rest.StartsWith("SET", StringComparison.OrdinalIgnoreCase))
        {
            throw new ArgumentException("invalid UPDATE statement");
        }

        rest = Trim(rest["SET".Length..]);
        var whereSplit = SplitTopLevelKeyword(rest, "WHERE");
        var assignments = SplitTopLevel(whereSplit.Left, ',')
            .Select(assignment =>
            {
                var op = FindTopLevelOperator(assignment, ["="]) ?? throw new ArgumentException($"invalid assignment: {assignment}");
                var column = Trim(assignment[..op.Index]);
                if (!Regex.IsMatch(column, @"^[A-Za-z_][A-Za-z0-9_]*$"))
                {
                    throw new ArgumentException($"invalid identifier: {column}");
                }

                return new Assignment(column, ParseLiteral(assignment[(op.Index + op.Operator.Length)..]));
            })
            .ToArray();

        if (assignments.Length == 0)
        {
            throw new ArgumentException("UPDATE requires at least one assignment");
        }

        return new UpdateStmt(table, assignments, whereSplit.Right);
    }

    public static DeleteStmt ParseDelete(string sql)
    {
        var stripped = StripTrailingSemicolon(sql);
        var start = Regex.Replace(stripped, @"^\s*DELETE\s+FROM\s+", "", RegexOptions.IgnoreCase);
        if (start == stripped)
        {
            throw new ArgumentException("invalid DELETE statement");
        }

        var table = IdentifierAtStart(start) ?? throw new ArgumentException("invalid DELETE statement");
        var rest = Trim(start[table.Length..]);
        var where = "";
        if (rest.Length != 0)
        {
            var whereSplit = SplitTopLevelKeyword(rest, "WHERE");
            if (whereSplit.Left.Length != 0 || whereSplit.Right.Length == 0)
            {
                throw new ArgumentException("invalid DELETE statement");
            }

            where = whereSplit.Right;
        }

        return new DeleteStmt(table, where);
    }

    public static Split SplitTopLevelKeyword(string text, string keyword)
    {
        var upper = text.ToUpperInvariant();
        var depth = 0;
        char? quote = null;
        for (var i = 0; i < text.Length; i++)
        {
            var ch = text[i];
            if (quote is not null)
            {
                if (ch == quote)
                {
                    if (i + 1 < text.Length && text[i + 1] == quote)
                    {
                        i++;
                    }
                    else
                    {
                        quote = null;
                    }
                }
            }
            else if (ch is '\'' or '"')
            {
                quote = ch;
            }
            else if (ch == '(')
            {
                depth++;
            }
            else if (ch == ')')
            {
                depth = Math.Max(0, depth - 1);
            }
            else if (depth == 0
                     && i + keyword.Length <= text.Length
                     && upper.Substring(i, keyword.Length) == keyword
                     && (i == 0 || IsBoundaryChar(text[i - 1]))
                     && (i + keyword.Length == text.Length || IsBoundaryChar(text[i + keyword.Length])))
            {
                return new Split(Trim(text[..i]), Trim(text[(i + keyword.Length)..]));
            }
        }

        return new Split(Trim(text), "");
    }

    public static string? IdentifierAtStart(string text)
    {
        var value = Trim(text);
        if (value.Length == 0 || (!char.IsLetter(value[0]) && value[0] != '_'))
        {
            return null;
        }

        var end = 1;
        while (end < value.Length && (char.IsLetterOrDigit(value[end]) || value[end] == '_'))
        {
            end++;
        }

        return value[..end];
    }

    public static IReadOnlyList<string> ParseSelectedColumns(Table table, string columnSql)
    {
        return Trim(columnSql) == "*"
            ? table.Columns.ToArray()
            : SplitTopLevel(columnSql, ',').Select(column => CanonicalColumn(table, column)).ToArray();
    }

    public static string CanonicalColumn(Table table, string column)
    {
        var wanted = Trim(column);
        foreach (var candidate in table.Columns)
        {
            if (string.Equals(candidate, wanted, StringComparison.OrdinalIgnoreCase))
            {
                return candidate;
            }
        }

        throw new ArgumentException($"no such column: {wanted}");
    }

    public static object? ValueOfColumn(Table table, IReadOnlyDictionary<string, object?> row, string column)
        => row[CanonicalColumn(table, column)];

    public static bool MatchesWhere(Table table, IReadOnlyDictionary<string, object?> row, string whereSql)
    {
        var where = Trim(whereSql);
        if (where.Length == 0)
        {
            return true;
        }

        var orSplit = SplitTopLevelKeyword(where, "OR");
        if (orSplit.Right.Length != 0)
        {
            return MatchesWhere(table, row, orSplit.Left) || MatchesWhere(table, row, orSplit.Right);
        }

        var andSplit = SplitTopLevelKeyword(where, "AND");
        if (andSplit.Right.Length != 0)
        {
            return MatchesWhere(table, row, andSplit.Left) && MatchesWhere(table, row, andSplit.Right);
        }

        if (where.EndsWith(" IS NOT NULL", StringComparison.OrdinalIgnoreCase))
        {
            var column = where[..^" IS NOT NULL".Length];
            return ValueOfColumn(table, row, column) is not null;
        }

        if (where.EndsWith(" IS NULL", StringComparison.OrdinalIgnoreCase))
        {
            var column = where[..^" IS NULL".Length];
            return ValueOfColumn(table, row, column) is null;
        }

        var inIndex = CultureInfo.InvariantCulture.CompareInfo.IndexOf(where, " IN ", CompareOptions.IgnoreCase);
        if (inIndex > 0)
        {
            var column = where[..inIndex];
            var valuesSql = Trim(where[(inIndex + 4)..]);
            if (!valuesSql.StartsWith('(') || !valuesSql.EndsWith(')'))
            {
                throw new ArgumentException("invalid IN predicate");
            }

            var actual = ValueOfColumn(table, row, column);
            return SplitTopLevel(valuesSql[1..^1], ',').Any(part => ValuesEqual(actual, ParseLiteral(part)));
        }

        var op = FindTopLevelOperator(where, ["<=", ">=", "!=", "<>", "=", "<", ">"])
            ?? throw new ArgumentException($"unsupported WHERE predicate: {where}");
        var left = ValueOfColumn(table, row, where[..op.Index]);
        var right = ParseLiteral(where[(op.Index + op.Operator.Length)..]);
        return ComparePredicate(left, right, op.Operator);
    }

    public static void ApplyOrder(Table table, List<Dictionary<string, object?>> rows, string orderSql)
    {
        if (string.IsNullOrWhiteSpace(orderSql))
        {
            return;
        }

        var parts = Trim(orderSql).Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries);
        var column = CanonicalColumn(table, parts[0]);
        var desc = parts.Length > 1 && string.Equals(parts[1], "DESC", StringComparison.OrdinalIgnoreCase);
        rows.Sort((left, right) =>
        {
            var cmp = CompareNullable(left[column], right[column]);
            return desc ? -cmp : cmp;
        });
    }

    private static int ReadQuoted(string sql, int index, char quote)
    {
        var i = index + 1;
        while (i < sql.Length)
        {
            var ch = sql[i];
            if (ch == quote)
            {
                if (i + 1 < sql.Length && sql[i + 1] == quote)
                {
                    i += 2;
                }
                else
                {
                    return i + 1;
                }
            }
            else
            {
                i++;
            }
        }

        return sql.Length;
    }

    private static string ToSqlLiteral(object? value)
    {
        return value switch
        {
            null => "NULL",
            bool boolValue => boolValue ? "TRUE" : "FALSE",
            byte or sbyte or short or ushort or int or uint or long or ulong or float or double or decimal
                => Convert.ToString(value, CultureInfo.InvariantCulture) ?? "NULL",
            string stringValue => QuoteSqlString(stringValue),
            char charValue => QuoteSqlString(charValue.ToString()),
            _ => throw new MiniSqliteException("ProgrammingError", $"unsupported parameter type: {value.GetType().Name}"),
        };
    }

    private static string QuoteSqlString(string value) => $"'{value.Replace("'", "''")}'";

    private static IReadOnlyList<IReadOnlyList<object?>> ParseValueRows(string sql)
    {
        var rest = Trim(sql);
        var rows = new List<IReadOnlyList<object?>>();
        while (rest.Length != 0)
        {
            if (!rest.StartsWith('('))
            {
                throw new ArgumentException("INSERT VALUES rows must be parenthesized");
            }

            var close = FindMatchingParen(rest, 0);
            if (close < 0)
            {
                throw new ArgumentException("unterminated INSERT VALUES row");
            }

            var row = SplitTopLevel(rest[1..close], ',').Select(ParseLiteral).ToArray();
            if (row.Length == 0)
            {
                throw new ArgumentException("INSERT row requires at least one value");
            }

            rows.Add(row);
            rest = Trim(rest[(close + 1)..]);
            if (rest.StartsWith(','))
            {
                rest = Trim(rest[1..]);
            }
            else if (rest.Length != 0)
            {
                throw new ArgumentException("invalid text after INSERT row");
            }
        }

        if (rows.Count == 0)
        {
            throw new ArgumentException("INSERT requires at least one row");
        }

        return rows;
    }

    private static object? ParseLiteral(string text)
    {
        var value = Trim(text);
        if (string.Equals(value, "NULL", StringComparison.OrdinalIgnoreCase))
        {
            return null;
        }

        if (string.Equals(value, "TRUE", StringComparison.OrdinalIgnoreCase))
        {
            return true;
        }

        if (string.Equals(value, "FALSE", StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        if (value.Length >= 2 && value[0] == '\'' && value[^1] == '\'')
        {
            return value[1..^1].Replace("''", "'");
        }

        if (Regex.IsMatch(value, @"^[-+]?(?:\d+(?:\.\d*)?|\.\d+)$"))
        {
            return value.Contains('.')
                ? double.Parse(value, CultureInfo.InvariantCulture)
                : long.Parse(value, CultureInfo.InvariantCulture);
        }

        throw new ArgumentException($"expected literal value, got: {text}");
    }

    private static IReadOnlyList<string> SplitTopLevel(string text, char delimiter)
    {
        var parts = new List<string>();
        var current = new StringBuilder();
        var depth = 0;
        char? quote = null;
        for (var i = 0; i < text.Length; i++)
        {
            var ch = text[i];
            if (quote is not null)
            {
                current.Append(ch);
                if (ch == quote)
                {
                    if (i + 1 < text.Length && text[i + 1] == quote)
                    {
                        current.Append(text[++i]);
                    }
                    else
                    {
                        quote = null;
                    }
                }
            }
            else if (ch is '\'' or '"')
            {
                quote = ch;
                current.Append(ch);
            }
            else if (ch == '(')
            {
                depth++;
                current.Append(ch);
            }
            else if (ch == ')')
            {
                depth = Math.Max(0, depth - 1);
                current.Append(ch);
            }
            else if (depth == 0 && ch == delimiter)
            {
                var part = Trim(current.ToString());
                if (part.Length != 0)
                {
                    parts.Add(part);
                }

                current.Clear();
            }
            else
            {
                current.Append(ch);
            }
        }

        var finalPart = Trim(current.ToString());
        if (finalPart.Length != 0)
        {
            parts.Add(finalPart);
        }

        return parts;
    }

    private static int FindMatchingParen(string text, int openIndex)
    {
        var depth = 0;
        char? quote = null;
        for (var i = openIndex; i < text.Length; i++)
        {
            var ch = text[i];
            if (quote is not null)
            {
                if (ch == quote)
                {
                    if (i + 1 < text.Length && text[i + 1] == quote)
                    {
                        i++;
                    }
                    else
                    {
                        quote = null;
                    }
                }
            }
            else if (ch is '\'' or '"')
            {
                quote = ch;
            }
            else if (ch == '(')
            {
                depth++;
            }
            else if (ch == ')')
            {
                depth--;
                if (depth == 0)
                {
                    return i;
                }
            }
        }

        return -1;
    }

    private static OperatorAt? FindTopLevelOperator(string text, IReadOnlyList<string> operators)
    {
        var depth = 0;
        char? quote = null;
        for (var i = 0; i < text.Length; i++)
        {
            var ch = text[i];
            if (quote is not null)
            {
                if (ch == quote)
                {
                    if (i + 1 < text.Length && text[i + 1] == quote)
                    {
                        i++;
                    }
                    else
                    {
                        quote = null;
                    }
                }
            }
            else if (ch is '\'' or '"')
            {
                quote = ch;
            }
            else if (ch == '(')
            {
                depth++;
            }
            else if (ch == ')')
            {
                depth = Math.Max(0, depth - 1);
            }
            else if (depth == 0)
            {
                foreach (var op in operators)
                {
                    if (i + op.Length <= text.Length && text.Substring(i, op.Length) == op)
                    {
                        return new OperatorAt(i, op);
                    }
                }
            }
        }

        return null;
    }

    private static bool ComparePredicate(object? left, object? right, string op)
    {
        if (op == "=")
        {
            return ValuesEqual(left, right);
        }

        if (op is "!=" or "<>")
        {
            return !ValuesEqual(left, right);
        }

        if (left is null || right is null)
        {
            return false;
        }

        var cmp = CompareValues(left, right);
        return op switch
        {
            "<" => cmp < 0,
            ">" => cmp > 0,
            "<=" => cmp <= 0,
            ">=" => cmp >= 0,
            _ => false,
        };
    }

    private static bool ValuesEqual(object? left, object? right)
    {
        if (left is null || right is null)
        {
            return left is null && right is null;
        }

        if (left is IConvertible && right is IConvertible && IsNumber(left) && IsNumber(right))
        {
            return Convert.ToDouble(left, CultureInfo.InvariantCulture)
                .Equals(Convert.ToDouble(right, CultureInfo.InvariantCulture));
        }

        return Equals(left, right);
    }

    private static int CompareNullable(object? left, object? right)
    {
        if (left is null && right is null)
        {
            return 0;
        }

        if (left is null)
        {
            return 1;
        }

        if (right is null)
        {
            return -1;
        }

        return CompareValues(left, right);
    }

    private static int CompareValues(object left, object right)
    {
        if (IsNumber(left) && IsNumber(right))
        {
            return Convert.ToDouble(left, CultureInfo.InvariantCulture)
                .CompareTo(Convert.ToDouble(right, CultureInfo.InvariantCulture));
        }

        return string.CompareOrdinal(left.ToString(), right.ToString());
    }

    private static bool IsNumber(object value)
    {
        return value is byte or sbyte or short or ushort or int or uint or long or ulong or float or double or decimal;
    }

    private static bool IsBoundaryChar(char ch) => !char.IsLetterOrDigit(ch) && ch != '_';
}

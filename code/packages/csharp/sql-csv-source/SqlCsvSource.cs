using System.Globalization;
using CodingAdventures.SqlExecutionEngine;
using CsvParserType = CodingAdventures.CsvParser.CsvParser;
using SqlEngine = CodingAdventures.SqlExecutionEngine.SqlExecutionEngine;

namespace CodingAdventures.SqlCsvSource;

public sealed class CsvDataSource(string directory) : IDataSource
{
    public string Directory { get; } = directory;

    public IReadOnlyList<string> Schema(string tableName)
    {
        var source = ReadTable(tableName);
        var firstLine = source.Split('\n', 2)[0].Trim();
        if (firstLine.Length == 0)
        {
            return [];
        }

        return firstLine
            .Split(',')
            .Select(column => column.Trim())
            .Where(column => column.Length > 0)
            .ToArray();
    }

    public IReadOnlyList<IReadOnlyDictionary<string, object?>> Scan(string tableName)
    {
        var rows = CsvParserType.ParseCsv(ReadTable(tableName));
        return rows
            .Select(row => row.ToDictionary(
                entry => entry.Key,
                entry => Coerce(entry.Value),
                StringComparer.Ordinal))
            .Cast<IReadOnlyDictionary<string, object?>>()
            .ToArray();
    }

    private string ReadTable(string tableName)
    {
        var path = Path.Combine(Directory, tableName + ".csv");
        try
        {
            return File.ReadAllText(path);
        }
        catch (FileNotFoundException ex)
        {
            throw new SqlExecutionException($"table not found: {tableName}", ex);
        }
        catch (DirectoryNotFoundException ex)
        {
            throw new SqlExecutionException($"table not found: {tableName}", ex);
        }
        catch (IOException ex)
        {
            throw new SqlExecutionException($"reading CSV table: {tableName}", ex);
        }
    }

    public static object? Coerce(string value)
    {
        if (value.Length == 0)
        {
            return null;
        }

        if (value.Equals("true", StringComparison.OrdinalIgnoreCase))
        {
            return true;
        }

        if (value.Equals("false", StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        if (long.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out var integer))
        {
            return integer;
        }

        if (double.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, out var real))
        {
            return real;
        }

        return value;
    }
}

public static class SqlCsvSource
{
    public static QueryResult ExecuteCsv(string sql, string directory) =>
        SqlEngine.Execute(sql, new CsvDataSource(directory));

    public static ExecutionResult TryExecuteCsv(string sql, string directory) =>
        SqlEngine.TryExecute(sql, new CsvDataSource(directory));
}

using CodingAdventures.SqlExecutionEngine;

namespace CodingAdventures.SqlCsvSource.Tests;

public sealed class SqlCsvSourceTests
{
    private static string Fixtures => Path.Combine(AppContext.BaseDirectory, "fixtures");

    [Fact]
    public void ExposesSchemaInHeaderOrder()
    {
        var source = new CsvDataSource(Fixtures);

        Assert.Equal(["id", "name", "dept_id", "salary", "active"], source.Schema("employees"));
        Assert.Equal(["id", "name", "budget"], source.Schema("departments"));
    }

    [Fact]
    public void ScansRowsWithCoercedValues()
    {
        var rows = new CsvDataSource(Fixtures).Scan("employees");

        Assert.Equal(4, rows.Count);
        Assert.Equal(1L, rows[0]["id"]);
        Assert.Equal("Alice", rows[0]["name"]);
        Assert.Equal(90000L, rows[0]["salary"]);
        Assert.Equal(true, rows[0]["active"]);
        Assert.Null(rows[3]["dept_id"]);
    }

    [Fact]
    public void ExecutesSelectsAgainstCsvFiles()
    {
        var result = SqlCsvSource.ExecuteCsv(
            "SELECT name, salary FROM employees WHERE active = true AND salary > 70000 ORDER BY salary DESC",
            Fixtures);

        Assert.Equal(["name", "salary"], result.Columns);
        Assert.Equal(["Alice", 90000L], result.Rows[0]);
        Assert.Equal(["Bob", 75000L], result.Rows[1]);
    }

    [Fact]
    public void SupportsNullPredicates()
    {
        var result = SqlCsvSource.ExecuteCsv("SELECT name FROM employees WHERE dept_id IS NULL", Fixtures);

        Assert.Equal([["Dave"]], result.Rows);
    }

    [Fact]
    public void SupportsJoinsAcrossCsvFiles()
    {
        var result = SqlCsvSource.ExecuteCsv(
            """
            SELECT e.name AS emp_name, d.name AS dept_name
            FROM employees AS e
            INNER JOIN departments AS d ON e.dept_id = d.id
            ORDER BY e.id
            """,
            Fixtures);

        Assert.Equal(["emp_name", "dept_name"], result.Columns);
        Assert.Equal(["Alice", "Engineering"], result.Rows[0]);
        Assert.Equal(["Bob", "Marketing"], result.Rows[1]);
        Assert.Equal(["Carol", "Engineering"], result.Rows[2]);
    }

    [Fact]
    public void SupportsGroupingAggregatesLimitAndOffset()
    {
        var result = SqlCsvSource.ExecuteCsv(
            "SELECT dept_id, COUNT(*) AS cnt FROM employees WHERE dept_id IS NOT NULL GROUP BY dept_id ORDER BY dept_id LIMIT 2",
            Fixtures);

        Assert.Equal(["dept_id", "cnt"], result.Columns);
        Assert.Equal([1L, 2], result.Rows[0]);
        Assert.Equal([2L, 1], result.Rows[1]);
    }

    [Fact]
    public void ReportsMissingTablesThroughEngineErrors()
    {
        var ex = Assert.Throws<SqlExecutionException>(
            () => SqlCsvSource.ExecuteCsv("SELECT * FROM no_such_table", Fixtures));

        Assert.Contains("table not found: no_such_table", ex.Message);

        var result = SqlCsvSource.TryExecuteCsv("SELECT * FROM no_such_table", Fixtures);
        Assert.False(result.Ok);
        Assert.NotNull(result.Error);
    }

    [Fact]
    public void CoercesScalarValues()
    {
        Assert.Null(CsvDataSource.Coerce(""));
        Assert.Equal(true, CsvDataSource.Coerce("TRUE"));
        Assert.Equal(false, CsvDataSource.Coerce("false"));
        Assert.Equal(42L, CsvDataSource.Coerce("42"));
        Assert.Equal(-5L, CsvDataSource.Coerce("-5"));
        Assert.Equal(3.14, CsvDataSource.Coerce("3.14"));
        Assert.Equal("123abc", CsvDataSource.Coerce("123abc"));
        Assert.IsType<string>(CsvDataSource.Coerce("hello"));
    }
}

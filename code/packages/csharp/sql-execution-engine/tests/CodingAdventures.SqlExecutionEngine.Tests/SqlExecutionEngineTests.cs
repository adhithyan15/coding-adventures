namespace CodingAdventures.SqlExecutionEngine.Tests;

public class SqlExecutionEngineTests
{
    [Fact]
    public void ScansInMemoryTables()
    {
        var source = DataSource();

        Assert.Equal(new[] { "id", "name", "dept", "salary", "active" }, source.Schema("employees"));
        Assert.Equal(5, source.Scan("employees").Count);
        Assert.Throws<SqlExecutionException>(() => source.Schema("missing"));
    }

    [Fact]
    public void SelectsAndFiltersRows()
    {
        var result = SqlExecutionEngine.Execute(
            "SELECT name, salary FROM employees WHERE active = true AND salary >= 70000 ORDER BY salary DESC",
            DataSource());

        Assert.Equal(new[] { "name", "salary" }, result.Columns);
        Assert.Equal(new object?[] { "Alice", 95_000 }, result.Rows[0]);
        Assert.Equal(new object?[] { "Bob", 72_000 }, result.Rows[1]);
    }

    [Fact]
    public void SupportsNullPredicatesAndLike()
    {
        var nullResult = SqlExecutionEngine.Execute("SELECT name FROM employees WHERE dept IS NULL", DataSource());
        Assert.Equal(new object?[] { "Dave" }, nullResult.Rows[0]);

        var likeResult = SqlExecutionEngine.Execute("SELECT name FROM employees WHERE name LIKE 'A%'", DataSource());
        Assert.Equal(new object?[] { "Alice" }, likeResult.Rows[0]);
    }

    [Fact]
    public void SupportsJoins()
    {
        var result = SqlExecutionEngine.Execute(
            "SELECT e.name, d.budget FROM employees AS e INNER JOIN departments AS d ON e.dept = d.dept ORDER BY e.id",
            DataSource());

        Assert.Equal(new[] { "name", "budget" }, result.Columns);
        Assert.Equal(4, result.Rows.Count);
        Assert.Equal(new object?[] { "Alice", 500_000 }, result.Rows[0]);
        Assert.Equal(new object?[] { "Eve", 150_000 }, result.Rows[3]);
    }

    [Fact]
    public void SupportsGroupingAndAggregates()
    {
        var result = SqlExecutionEngine.Execute(
            "SELECT dept, COUNT(*) AS cnt, SUM(salary) AS total FROM employees WHERE dept IS NOT NULL GROUP BY dept HAVING COUNT(*) >= 1 ORDER BY dept",
            DataSource());

        Assert.Equal(new[] { "dept", "cnt", "total" }, result.Columns);
        Assert.Equal(new object?[] { "Engineering", 2, 183_000.0 }, result.Rows[0]);
        Assert.Equal(new object?[] { "HR", 1, 70_000.0 }, result.Rows[1]);
        Assert.Equal(new object?[] { "Marketing", 1, 72_000.0 }, result.Rows[2]);
    }

    [Fact]
    public void SupportsDistinctLimitAndOffset()
    {
        var result = SqlExecutionEngine.Execute(
            "SELECT DISTINCT dept FROM employees WHERE dept IS NOT NULL ORDER BY dept LIMIT 2 OFFSET 1",
            DataSource());

        Assert.Equal(new[] { "dept" }, result.Columns);
        Assert.Equal(new object?[] { "HR" }, result.Rows[0]);
        Assert.Equal(new object?[] { "Marketing" }, result.Rows[1]);
    }

    [Fact]
    public void ReportsErrorsThroughTryExecute()
    {
        var result = SqlExecutionEngine.TryExecute("SELECT * FROM ghosts", DataSource());

        Assert.False(result.Ok);
        Assert.NotNull(result.Error);
        Assert.Contains("table not found: ghosts", result.Error);
    }

    [Fact]
    public void SelectStarUsesBareColumns()
    {
        var result = SqlExecutionEngine.Execute("SELECT * FROM employees WHERE id = 1", DataSource());

        Assert.Equal(new[] { "active", "dept", "id", "name", "salary" }, result.Columns);
        Assert.Equal(new object?[] { true, "Engineering", 1, "Alice", 95_000 }, result.Rows[0]);
    }

    private static InMemoryDataSource DataSource()
    {
        return new InMemoryDataSource()
            .AddTable(
                "employees",
                new[] { "id", "name", "dept", "salary", "active" },
                new[]
                {
                    Row("id", 1, "name", "Alice", "dept", "Engineering", "salary", 95_000, "active", true),
                    Row("id", 2, "name", "Bob", "dept", "Marketing", "salary", 72_000, "active", true),
                    Row("id", 3, "name", "Carol", "dept", "Engineering", "salary", 88_000, "active", false),
                    Row("id", 4, "name", "Dave", "dept", null, "salary", 60_000, "active", true),
                    Row("id", 5, "name", "Eve", "dept", "HR", "salary", 70_000, "active", false),
                })
            .AddTable(
                "departments",
                new[] { "dept", "budget" },
                new[]
                {
                    Row("dept", "Engineering", "budget", 500_000),
                    Row("dept", "Marketing", "budget", 200_000),
                    Row("dept", "HR", "budget", 150_000),
                });
    }

    private static IReadOnlyDictionary<string, object?> Row(params object?[] pairs)
    {
        var row = new Dictionary<string, object?>(StringComparer.Ordinal);
        for (var index = 0; index < pairs.Length; index += 2)
        {
            row[(string)pairs[index]!] = pairs[index + 1];
        }
        return row;
    }
}

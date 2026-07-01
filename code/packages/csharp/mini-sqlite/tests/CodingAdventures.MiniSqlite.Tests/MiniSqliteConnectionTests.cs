// MiniSqliteConnectionTests.cs — Level 1 pipeline integration tests.
//
// These tests exercise the MiniSqliteConnection class through its public API:
//   var conn = new MiniSqliteConnection();
//   conn.Execute("CREATE TABLE ...");
//   conn.Execute("INSERT INTO ...");
//   QueryResult r = conn.Execute("SELECT ...");
//
// Test categories:
//   1. Basic DDL (CREATE TABLE, DROP TABLE)
//   2. DML (INSERT, UPDATE, DELETE)
//   3. SELECT * and projection
//   4. SELECT with WHERE
//   5. SELECT with ORDER BY
//   6. SELECT with LIMIT / OFFSET
//   7. SELECT with aggregates (COUNT, SUM, AVG, MIN, MAX)
//   8. SELECT DISTINCT
//   9. GROUP BY + HAVING
//  10. Error handling (unknown table, unknown column, param count)
//  11. NULL handling
//  12. Scalar functions (LENGTH, UPPER, LOWER, SUBSTR, TRIM, REPLACE, ABS, ROUND)
//  13. Parameter binding
//  14. Transaction commit/rollback
//  15. Conformance fixture harness (all 24 fixtures from mini-sqlite-conformance)

using System.Globalization;
using System.Text.Json;
using CodingAdventures.MiniSqlite;

namespace CodingAdventures.MiniSqlite.Tests;

// ── Helper ─────────────────────────────────────────────────────────────────────

file static class Helpers
{
    /// <summary>
    /// Compares two object? values for conformance-test equality.
    ///
    /// Rules:
    ///  • null == null
    ///  • Numbers are compared as doubles to handle int vs long vs double.
    ///  • Strings are compared ordinally.
    ///  • JsonElement values (from the fixture JSON) are unwrapped first.
    /// </summary>
    public static bool RowValuesEqual(object? expected, object? actual)
    {
        // Unwrap JsonElement from fixture JSON
        if (expected is JsonElement je)
            expected = UnwrapJson(je);

        if (expected is null && actual is null) return true;
        if (expected is null || actual is null)  return false;

        // Numeric comparison via double
        if (IsNumeric(expected) && IsNumeric(actual))
            return Convert.ToDouble(expected, CultureInfo.InvariantCulture)
                .Equals(Convert.ToDouble(actual, CultureInfo.InvariantCulture));

        return Equals(expected.ToString(), actual?.ToString());
    }

    private static object? UnwrapJson(JsonElement je) => je.ValueKind switch
    {
        JsonValueKind.Null    => null,
        JsonValueKind.True    => true,
        JsonValueKind.False   => false,
        JsonValueKind.Number  => je.TryGetInt64(out var i) ? (object)i : je.GetDouble(),
        JsonValueKind.String  => je.GetString(),
        _                     => je.ToString(),
    };

    private static bool IsNumeric(object v) =>
        v is byte or sbyte or short or ushort or int or uint or long or ulong or float or double or decimal;
}

// ── 1. Basic DDL ───────────────────────────────────────────────────────────────

public sealed class CreateDropTableTests
{
    [Fact]
    public void CreateTable_Then_Select_Returns_Empty()
    {
        var conn   = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE users (id INT, name TEXT)");
        var result = conn.Execute("SELECT * FROM users");
        Assert.Equal(new[] { "id", "name" }, result.Columns);
        Assert.Empty(result.Rows);
    }

    [Fact]
    public void DropTable_Removes_Table()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE t (x INT)");
        conn.Execute("DROP TABLE t");
        var ex = Assert.Throws<MiniSqliteException>(() => conn.Execute("SELECT * FROM t"));
        Assert.Equal("OperationalError", ex.Kind);
    }

    [Fact]
    public void CreateTable_IfNotExists_IsIdempotent()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE IF NOT EXISTS t (x INT)");
        conn.Execute("CREATE TABLE IF NOT EXISTS t (x INT)"); // should not throw
        conn.Execute("INSERT INTO t VALUES (1)");
        var r = conn.Execute("SELECT x FROM t");
        Assert.Single(r.Rows);
    }

    [Fact]
    public void DropTable_IfExists_IsIdempotent()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("DROP TABLE IF EXISTS nonexistent");   // should not throw
    }
}

// ── 2. DML ─────────────────────────────────────────────────────────────────────

public sealed class DmlTests
{
    [Fact]
    public void Insert_Single_Row()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE users (id INT, name TEXT)");
        var r = conn.Execute("INSERT INTO users VALUES (1, 'Alice')");
        Assert.Equal(1, r.RowsAffected);
    }

    [Fact]
    public void Insert_Multiple_Rows()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE users (id INT, name TEXT)");
        conn.Execute("INSERT INTO users VALUES (1, 'Alice')");
        conn.Execute("INSERT INTO users VALUES (2, 'Bob')");
        var r = conn.Execute("SELECT * FROM users");
        Assert.Equal(2, r.Rows.Count);
    }

    [Fact]
    public void Update_Modifies_Matching_Rows()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE users (id INT, name TEXT)");
        conn.Execute("INSERT INTO users VALUES (1, 'Alice')");
        conn.Execute("INSERT INTO users VALUES (2, 'Bob')");
        var r = conn.Execute("UPDATE users SET name = 'Bobby' WHERE id = 2");
        Assert.Equal(1, r.RowsAffected);

        var rows = conn.Execute("SELECT name FROM users WHERE id = 2").Rows;
        Assert.Equal("Bobby", rows[0][0]?.ToString());
    }

    [Fact]
    public void Delete_Removes_Matching_Rows()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE users (id INT, name TEXT)");
        conn.Execute("INSERT INTO users VALUES (1, 'Alice')");
        conn.Execute("INSERT INTO users VALUES (2, 'Bob')");
        conn.Execute("INSERT INTO users VALUES (3, 'Carol')");
        var r = conn.Execute("DELETE FROM users WHERE id > 1");
        Assert.Equal(2, r.RowsAffected);
        Assert.Single(conn.Execute("SELECT * FROM users").Rows);
    }

    [Fact]
    public void Delete_Without_Where_Clears_Table()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE t (x INT)");
        conn.Execute("INSERT INTO t VALUES (1)");
        conn.Execute("INSERT INTO t VALUES (2)");
        conn.Execute("DELETE FROM t");
        Assert.Empty(conn.Execute("SELECT * FROM t").Rows);
    }
}

// ── 3. SELECT ─────────────────────────────────────────────────────────────────

public sealed class SelectTests
{
    [Fact]
    public void Select_Star_Returns_All_Columns()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE users (id INT, name TEXT)");
        conn.Execute("INSERT INTO users VALUES (1, 'Alice')");
        conn.Execute("INSERT INTO users VALUES (2, 'Bob')");
        var r = conn.Execute("SELECT * FROM users WHERE id > 1");
        Assert.Equal(new[] { "id", "name" }, r.Columns);
        Assert.Single(r.Rows);
        Assert.Equal("Bob", r.Rows[0][1]?.ToString());
    }

    [Fact]
    public void Select_Projection()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE employees (id INT, first_name TEXT, last_name TEXT, salary REAL)");
        conn.Execute("INSERT INTO employees VALUES (1, 'Alice', 'Smith', 75000)");
        conn.Execute("INSERT INTO employees VALUES (2, 'Bob', 'Jones', 82000)");
        var r = conn.Execute("SELECT first_name AS name, salary AS pay FROM employees ORDER BY id");
        Assert.Equal(new[] { "name", "pay" }, r.Columns);
        Assert.Equal(2, r.Rows.Count);
        Assert.Equal("Alice", r.Rows[0][0]?.ToString());
    }

    [Fact]
    public void Select_Where_Filters_Rows()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE users (id INT, name TEXT)");
        conn.Execute("INSERT INTO users VALUES (1, 'Alice')");
        conn.Execute("INSERT INTO users VALUES (2, 'Bob')");
        conn.Execute("INSERT INTO users VALUES (3, 'Carol')");
        var r = conn.Execute("SELECT * FROM users WHERE id > 1");
        Assert.Equal(2, r.Rows.Count);
    }

    [Fact]
    public void Select_OrderBy_Sorts_Results()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE users (id INT, name TEXT)");
        conn.Execute("INSERT INTO users VALUES (3, 'Carol')");
        conn.Execute("INSERT INTO users VALUES (1, 'Alice')");
        conn.Execute("INSERT INTO users VALUES (2, 'Bob')");
        var r = conn.Execute("SELECT name FROM users ORDER BY id ASC");
        Assert.Equal("Alice", r.Rows[0][0]?.ToString());
        Assert.Equal("Bob",   r.Rows[1][0]?.ToString());
        Assert.Equal("Carol", r.Rows[2][0]?.ToString());
    }

    [Fact]
    public void Select_OrderBy_Desc()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE t (x INT)");
        conn.Execute("INSERT INTO t VALUES (1)");
        conn.Execute("INSERT INTO t VALUES (3)");
        conn.Execute("INSERT INTO t VALUES (2)");
        var r = conn.Execute("SELECT x FROM t ORDER BY x DESC");
        Assert.Equal(3L, Convert.ToInt64(r.Rows[0][0]));
        Assert.Equal(2L, Convert.ToInt64(r.Rows[1][0]));
        Assert.Equal(1L, Convert.ToInt64(r.Rows[2][0]));
    }

    [Fact]
    public void Select_Limit()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE t (x INT)");
        for (var i = 1; i <= 10; i++)
            conn.Execute($"INSERT INTO t VALUES ({i})");
        var r = conn.Execute("SELECT x FROM t ORDER BY x LIMIT 3");
        Assert.Equal(3, r.Rows.Count);
        Assert.Equal(1L, Convert.ToInt64(r.Rows[0][0]));
    }

    [Fact]
    public void Select_Limit_With_Offset()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE t (x INT)");
        for (var i = 1; i <= 5; i++)
            conn.Execute($"INSERT INTO t VALUES ({i})");
        var r = conn.Execute("SELECT x FROM t ORDER BY x LIMIT 2 OFFSET 2");
        Assert.Equal(2, r.Rows.Count);
        Assert.Equal(3L, Convert.ToInt64(r.Rows[0][0]));
        Assert.Equal(4L, Convert.ToInt64(r.Rows[1][0]));
    }
}

// ── 4. Aggregates ─────────────────────────────────────────────────────────────

public sealed class AggregateTests
{
    private static MiniSqliteConnection BuildSalesTable()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE sales (region TEXT, amount INTEGER)");
        conn.Execute("INSERT INTO sales VALUES ('east', 100)");
        conn.Execute("INSERT INTO sales VALUES ('east', 200)");
        conn.Execute("INSERT INTO sales VALUES ('west', 150)");
        conn.Execute("INSERT INTO sales VALUES ('west', 50)");
        conn.Execute("INSERT INTO sales VALUES ('west', 300)");
        return conn;
    }

    [Fact]
    public void Count_Star()
    {
        var conn = BuildSalesTable();
        var r = conn.Execute("SELECT COUNT(*) AS n FROM sales");
        Assert.Equal(new[] { "n" }, r.Columns);
        Assert.Equal(5L, Convert.ToInt64(r.Rows[0][0]));
    }

    [Fact]
    public void Sum()
    {
        var conn = BuildSalesTable();
        var r = conn.Execute("SELECT SUM(amount) AS total FROM sales");
        Assert.Equal(800L, Convert.ToInt64(r.Rows[0][0]));
    }

    [Fact]
    public void Avg()
    {
        var conn = BuildSalesTable();
        var r = conn.Execute("SELECT AVG(amount) AS avg FROM sales");
        Assert.Equal(160.0, Convert.ToDouble(r.Rows[0][0]), precision: 5);
    }

    [Fact]
    public void Min_Max()
    {
        var conn = BuildSalesTable();
        var r = conn.Execute("SELECT MIN(amount) AS lo, MAX(amount) AS hi FROM sales");
        Assert.Equal(2, r.Columns.Count);
        Assert.Equal(50L,  Convert.ToInt64(r.Rows[0][0]));
        Assert.Equal(300L, Convert.ToInt64(r.Rows[0][1]));
    }

    [Fact]
    public void GroupBy_Sum()
    {
        var conn = BuildSalesTable();
        var r = conn.Execute("SELECT region, SUM(amount) AS total FROM sales GROUP BY region ORDER BY region");
        Assert.Equal(2, r.Rows.Count);
        Assert.Equal("east", r.Rows[0][0]?.ToString());
        Assert.Equal(300L,   Convert.ToInt64(r.Rows[0][1]));
        Assert.Equal("west", r.Rows[1][0]?.ToString());
        Assert.Equal(500L,   Convert.ToInt64(r.Rows[1][1]));
    }

    [Fact]
    public void Having_Filters_Groups()
    {
        var conn = BuildSalesTable();
        var r = conn.Execute(
            "SELECT region, COUNT(*) AS n FROM sales GROUP BY region HAVING COUNT(*) > 1 ORDER BY region");
        Assert.Equal(2, r.Rows.Count);
        Assert.Equal("east", r.Rows[0][0]?.ToString());
        Assert.Equal(2L, Convert.ToInt64(r.Rows[0][1]));
    }

    [Fact]
    public void Count_Star_On_Empty_Table()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE empty_t (val INTEGER)");
        var r = conn.Execute("SELECT COUNT(*) AS cnt FROM empty_t");
        Assert.Equal(0L, Convert.ToInt64(r.Rows[0][0]));
    }

    [Fact]
    public void Sum_On_Empty_Table_Is_Null()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE empty_t (val INTEGER)");
        var r = conn.Execute("SELECT SUM(val) AS s FROM empty_t");
        Assert.Null(r.Rows[0][0]);
    }
}

// ── 5. DISTINCT ───────────────────────────────────────────────────────────────

public sealed class DistinctTests
{
    [Fact]
    public void Select_Distinct_Deduplicates_Rows()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE colors (name TEXT)");
        conn.Execute("INSERT INTO colors VALUES ('red')");
        conn.Execute("INSERT INTO colors VALUES ('blue')");
        conn.Execute("INSERT INTO colors VALUES ('red')");
        conn.Execute("INSERT INTO colors VALUES ('green')");
        conn.Execute("INSERT INTO colors VALUES ('blue')");
        var r = conn.Execute("SELECT DISTINCT name FROM colors ORDER BY name");
        Assert.Equal(3, r.Rows.Count);
        Assert.Equal("blue",  r.Rows[0][0]?.ToString());
        Assert.Equal("green", r.Rows[1][0]?.ToString());
        Assert.Equal("red",   r.Rows[2][0]?.ToString());
    }
}

// ── 6. NULL handling ──────────────────────────────────────────────────────────

public sealed class NullTests
{
    [Fact]
    public void Insert_And_Select_Null()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE t (id INT, value TEXT)");
        conn.Execute("INSERT INTO t VALUES (1, 'present')");
        conn.Execute("INSERT INTO t VALUES (2, NULL)");
        var r = conn.Execute("SELECT id, value FROM t ORDER BY id");
        Assert.Equal(2, r.Rows.Count);
        Assert.Equal("present", r.Rows[0][1]?.ToString());
        Assert.Null(r.Rows[1][1]);
    }

    [Fact]
    public void Is_Null_Predicate()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE t (id INT, value TEXT)");
        conn.Execute("INSERT INTO t VALUES (1, 'x')");
        conn.Execute("INSERT INTO t VALUES (2, NULL)");
        conn.Execute("INSERT INTO t VALUES (3, NULL)");
        var r = conn.Execute("SELECT id FROM t WHERE value IS NULL ORDER BY id");
        Assert.Equal(2, r.Rows.Count);
        Assert.Equal(2L, Convert.ToInt64(r.Rows[0][0]));
    }

    [Fact]
    public void Is_Not_Null_Predicate()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE t (id INT, value TEXT)");
        conn.Execute("INSERT INTO t VALUES (1, 'x')");
        conn.Execute("INSERT INTO t VALUES (2, NULL)");
        var r = conn.Execute("SELECT id FROM t WHERE value IS NOT NULL");
        Assert.Single(r.Rows);
        Assert.Equal(1L, Convert.ToInt64(r.Rows[0][0]));
    }
}

// ── 7. Parameter binding ──────────────────────────────────────────────────────

public sealed class ParameterBindingTests
{
    [Fact]
    public void Qmark_String_Parameter()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE t (name TEXT)");
        conn.Execute("INSERT INTO t VALUES (?)", new object?[] { "Alice" });
        var r = conn.Execute("SELECT name FROM t");
        Assert.Equal("Alice", r.Rows[0][0]?.ToString());
    }

    [Fact]
    public void Qmark_Int_Parameter()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE t (x INT)");
        conn.Execute("INSERT INTO t VALUES (?)", new object?[] { 42 });
        var r = conn.Execute("SELECT x FROM t WHERE x = ?", new object?[] { 42 });
        Assert.Single(r.Rows);
    }

    [Fact]
    public void Qmark_Null_Parameter()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE t (x INT)");
        conn.Execute("INSERT INTO t VALUES (?)", new object?[] { null });
        var r = conn.Execute("SELECT x FROM t WHERE x IS NULL");
        Assert.Single(r.Rows);
    }

    [Fact]
    public void Too_Few_Parameters_Throws_ProgrammingError()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE t (a INT, b INT)");
        var ex = Assert.Throws<MiniSqliteException>(
            () => conn.Execute("INSERT INTO t VALUES (?, ?)", new object?[] { 1 }));
        Assert.Equal("ProgrammingError", ex.Kind);
    }

    [Fact]
    public void Too_Many_Parameters_Throws_ProgrammingError()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE t (a INT, b INT)");
        var ex = Assert.Throws<MiniSqliteException>(
            () => conn.Execute("INSERT INTO t VALUES (?, ?)", new object?[] { 1, 2, 3 }));
        Assert.Equal("ProgrammingError", ex.Kind);
    }
}

// ── 8. Error handling ─────────────────────────────────────────────────────────

public sealed class ErrorHandlingTests
{
    [Fact]
    public void Unknown_Table_Throws_OperationalError()
    {
        var conn = new MiniSqliteConnection();
        var ex = Assert.Throws<MiniSqliteException>(
            () => conn.Execute("SELECT * FROM nonexistent_table"));
        Assert.Equal("OperationalError", ex.Kind);
    }

    [Fact]
    public void Insert_Unknown_Table_Throws_OperationalError()
    {
        var conn = new MiniSqliteConnection();
        var ex = Assert.Throws<MiniSqliteException>(
            () => conn.Execute("INSERT INTO missing_table VALUES (1, 2)"));
        Assert.Equal("OperationalError", ex.Kind);
    }

    [Fact]
    public void Connection_Still_Usable_After_Error()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE t (a INT, b INT)");
        Assert.Throws<MiniSqliteException>(
            () => conn.Execute("INSERT INTO t VALUES (?, ?)", new object?[] { 1 }));
        // Connection should still work after the error
        conn.Execute("INSERT INTO t VALUES (10, 20)");
        var r = conn.Execute("SELECT a, b FROM t");
        Assert.Single(r.Rows);
    }
}

// ── 9. Scalar functions ───────────────────────────────────────────────────────

public sealed class ScalarFunctionTests
{
    [Fact]
    public void Length_String()
    {
        var conn = new MiniSqliteConnection();
        var r = conn.Execute("SELECT LENGTH('hello') AS n");
        Assert.Equal(5L, Convert.ToInt64(r.Rows[0][0]));
    }

    [Fact]
    public void Upper_Lower()
    {
        var conn = new MiniSqliteConnection();
        var r = conn.Execute("SELECT UPPER('hello') AS u, LOWER('WORLD') AS l");
        Assert.Equal("HELLO", r.Rows[0][0]?.ToString());
        Assert.Equal("world", r.Rows[0][1]?.ToString());
    }

    [Fact]
    public void Substr()
    {
        var conn = new MiniSqliteConnection();
        var r = conn.Execute("SELECT SUBSTR('hello', 2, 3) AS s");
        Assert.Equal("ell", r.Rows[0][0]?.ToString());
    }

    [Fact]
    public void Trim()
    {
        var conn = new MiniSqliteConnection();
        var r = conn.Execute("SELECT TRIM('  hi  ') AS t");
        Assert.Equal("hi", r.Rows[0][0]?.ToString());
    }

    [Fact]
    public void Replace()
    {
        var conn = new MiniSqliteConnection();
        var r = conn.Execute("SELECT REPLACE('hello world', 'world', 'SQL') AS r");
        Assert.Equal("hello SQL", r.Rows[0][0]?.ToString());
    }

    [Fact]
    public void Abs_Integer()
    {
        var conn = new MiniSqliteConnection();
        var r = conn.Execute("SELECT ABS(-5) AS a");
        Assert.Equal(5L, Convert.ToInt64(r.Rows[0][0]));
    }

    [Fact]
    public void Abs_Null_Is_Null()
    {
        var conn = new MiniSqliteConnection();
        var r = conn.Execute("SELECT ABS(NULL) AS a");
        Assert.Null(r.Rows[0][0]);
    }

    [Fact]
    public void Round_To_Precision()
    {
        var conn = new MiniSqliteConnection();
        var r = conn.Execute("SELECT ROUND(3.14159, 2) AS r");
        Assert.Equal(3.14, Convert.ToDouble(r.Rows[0][0]), precision: 5);
    }
}

// ── 10. Transactions ─────────────────────────────────────────────────────────

public sealed class TransactionTests
{
    [Fact]
    public void Commit_Persists_Changes()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE accounts (owner TEXT, balance INTEGER)");
        conn.Execute("INSERT INTO accounts VALUES ('alice', 1000)");
        conn.Commit();
        conn.Execute("UPDATE accounts SET balance = 900 WHERE owner = 'alice'");
        conn.Commit();
        var r = conn.Execute("SELECT balance FROM accounts WHERE owner = 'alice'");
        Assert.Equal(900L, Convert.ToInt64(r.Rows[0][0]));
    }

    [Fact]
    public void Rollback_Restores_Previous_State()
    {
        var conn = new MiniSqliteConnection();
        conn.Execute("CREATE TABLE wallet (user TEXT, coins INTEGER)");
        conn.Execute("INSERT INTO wallet VALUES ('alice', 100)");
        conn.Commit();
        conn.Execute("UPDATE wallet SET coins = 50 WHERE user = 'alice'");
        conn.Rollback();
        var r = conn.Execute("SELECT coins FROM wallet WHERE user = 'alice'");
        Assert.Equal(100L, Convert.ToInt64(r.Rows[0][0]));
    }
}

// ── 11. Conformance fixture harness ───────────────────────────────────────────
//
// Loads each of the 24 JSON fixture files from the mini-sqlite-conformance
// specification and runs all steps against a fresh MiniSqliteConnection.
//
// The fixture files are copied to the output directory under
// "conformance-fixtures/" by the .csproj <Content> items.

public sealed class ConformanceFixtureTests
{
    // The conformance fixtures are copied to the build output next to the
    // assembly.  We locate them relative to the assembly path at runtime.
    private static string FixturesDir()
    {
        var asm = typeof(ConformanceFixtureTests).Assembly.Location;
        return Path.Combine(Path.GetDirectoryName(asm)!, "conformance-fixtures");
    }

    /// <summary>
    /// Returns one test-data object per fixture file so xUnit can name each
    /// test after its fixture id.
    /// </summary>
    public static IEnumerable<object[]> AllFixtures()
    {
        var dir = FixturesDir();
        if (!Directory.Exists(dir))
            yield break;

        foreach (var file in Directory.GetFiles(dir, "*.json").OrderBy(f => f))
        {
            if (Path.GetFileName(file) == "manifest.json") continue;
            var doc = JsonDocument.Parse(File.ReadAllText(file));
            var id  = doc.RootElement.GetProperty("id").GetString() ?? Path.GetFileNameWithoutExtension(file);
            yield return new object[] { id, file };
        }
    }

    [Theory]
    [MemberData(nameof(AllFixtures))]
    public void RunFixture(string id, string filePath)
    {
        var doc  = JsonDocument.Parse(File.ReadAllText(filePath));
        var root = doc.RootElement;

        // Fixture 12 tests file-path-level0 rejection — it calls connect_expect_error
        // with database paths other than ":memory:". MiniSqliteConnection is always
        // in-memory, so we verify that non-:memory: paths are not accepted at Level 0.
        // We skip this fixture for the Level 1 connection (which has no connect step).
        if (id == "12-error-file-path-level0")
        {
            // Level 1 connection is always in-memory — there's no Connect API to test.
            // The Level 0 MiniSqlite.Connect() already handles this case in MiniSqliteTests.
            return;
        }

        var conn = new MiniSqliteConnection();

        // Some fixtures have a top-level "connect_steps" array that runs before "steps".
        if (root.TryGetProperty("connect_steps", out var connectSteps))
        {
            RunSteps(conn, id, connectSteps);
        }

        if (root.TryGetProperty("steps", out var steps))
        {
            RunSteps(conn, id, steps);
        }
    }

    private static void RunSteps(MiniSqliteConnection conn, string fixtureId, JsonElement steps)
    {
        foreach (var step in steps.EnumerateArray())
        {
            var op = step.GetProperty("op").GetString() ?? "";

            switch (op)
            {
                case "execute":
                    ExecuteStep(conn, step);
                    break;

                case "executemany":
                    ExecuteManyStep(conn, step);
                    break;

                case "query":
                    QueryStep(conn, step, fixtureId);
                    break;

                case "fetchone_test":
                    FetchOneTestStep(conn, step, fixtureId);
                    break;

                case "fetchmany_test":
                    FetchManyTestStep(conn, step, fixtureId);
                    break;

                case "fetchall_test":
                case "fetchall_empty_test":
                    FetchAllTestStep(conn, step, fixtureId);
                    break;

                case "commit":
                    conn.Commit();
                    break;

                case "rollback":
                    conn.Rollback();
                    break;

                case "expect_error":
                    ExpectErrorStep(conn, step, fixtureId);
                    break;

                case "connect_expect_error":
                    // Level 1 has no Connect API — skip.
                    break;

                default:
                    // Unknown op type — warn but don't fail.
                    break;
            }
        }
    }

    // Execute a SQL statement (no result check).
    private static void ExecuteStep(MiniSqliteConnection conn, JsonElement step)
    {
        var sql    = step.GetProperty("sql").GetString() ?? "";
        var @params = ReadParams(step);
        conn.Execute(sql, @params);
    }

    // Execute the same SQL with each row of params.
    private static void ExecuteManyStep(MiniSqliteConnection conn, JsonElement step)
    {
        var sql = step.GetProperty("sql").GetString() ?? "";
        foreach (var paramRow in step.GetProperty("param_seq").EnumerateArray())
        {
            var ps = paramRow.EnumerateArray().Select(UnwrapJsonParam).ToArray();
            conn.Execute(sql, ps);
        }
    }

    // Execute SQL and compare the result against expected columns + rows.
    private static void QueryStep(MiniSqliteConnection conn, JsonElement step, string fixtureId)
    {
        var sql    = step.GetProperty("sql").GetString() ?? "";
        var @params = ReadParams(step);
        var result = conn.Execute(sql, @params);

        // Compare column names (case-insensitive per spec).
        if (step.TryGetProperty("expected_columns", out var expectedCols))
        {
            var expCols = expectedCols.EnumerateArray().Select(e => e.GetString() ?? "").ToArray();
            var actCols = result.Columns.ToArray();
            Assert.Equal(expCols.Length, actCols.Length);
            for (var i = 0; i < expCols.Length; i++)
                Assert.True(string.Equals(expCols[i], actCols[i], StringComparison.OrdinalIgnoreCase),
                    $"[{fixtureId}] Column {i}: expected '{expCols[i]}', got '{actCols[i]}'");
        }

        // Compare rows.
        if (step.TryGetProperty("expected_rows", out var expectedRows))
        {
            var expRows = expectedRows.EnumerateArray().ToArray();
            Assert.True(expRows.Length == result.Rows.Count,
                $"[{fixtureId}] Row count: expected {expRows.Length}, got {result.Rows.Count}");

            for (var ri = 0; ri < expRows.Length; ri++)
            {
                var expRow = expRows[ri].EnumerateArray().ToArray();
                var actRow = result.Rows[ri];
                Assert.True(expRow.Length == actRow.Count,
                    $"[{fixtureId}] Row {ri} column count: expected {expRow.Length}, got {actRow.Count}");

                for (var ci = 0; ci < expRow.Length; ci++)
                {
                    var expVal = UnwrapJsonParam(expRow[ci]);
                    var actVal = actRow[ci];
                    Assert.True(Helpers.RowValuesEqual(expVal, actVal),
                        $"[{fixtureId}] Row {ri}, col {ci}: expected {expVal ?? "NULL"}, got {actVal ?? "NULL"}");
                }
            }
        }
    }

    // fetchone_test: execute sql, call fetchone() twice, compare.
    private static void FetchOneTestStep(MiniSqliteConnection conn, JsonElement step, string fixtureId)
    {
        var sql    = step.GetProperty("sql").GetString() ?? "";
        var result = conn.Execute(sql);

        var expFirst  = step.GetProperty("expected_first").EnumerateArray().Select(UnwrapJsonParam).ToArray();
        var expSecond = step.GetProperty("expected_second").EnumerateArray().Select(UnwrapJsonParam).ToArray();

        // Simulate fetchone() by reading first and second rows.
        Assert.True(result.Rows.Count >= 2, $"[{fixtureId}] Not enough rows for fetchone_test");
        for (var ci = 0; ci < expFirst.Length; ci++)
            Assert.True(Helpers.RowValuesEqual(expFirst[ci], result.Rows[0][ci]),
                $"[{fixtureId}] fetchone row 0, col {ci}");
        for (var ci = 0; ci < expSecond.Length; ci++)
            Assert.True(Helpers.RowValuesEqual(expSecond[ci], result.Rows[1][ci]),
                $"[{fixtureId}] fetchone row 1, col {ci}");
    }

    // fetchmany_test: execute sql, compare first and second batches.
    private static void FetchManyTestStep(MiniSqliteConnection conn, JsonElement step, string fixtureId)
    {
        var sql    = step.GetProperty("sql").GetString() ?? "";
        var result = conn.Execute(sql);
        var size   = step.TryGetProperty("size", out var sz) ? sz.GetInt32() : 1;

        var expFirstBatch  = step.GetProperty("expected_first_batch").EnumerateArray()
            .Select(r => r.EnumerateArray().Select(UnwrapJsonParam).ToArray()).ToArray();
        var expSecondBatch = step.GetProperty("expected_second_batch").EnumerateArray()
            .Select(r => r.EnumerateArray().Select(UnwrapJsonParam).ToArray()).ToArray();

        // First batch = rows[0..size-1]
        var firstBatch = result.Rows.Take(size).ToArray();
        Assert.True(expFirstBatch.Length == firstBatch.Length,
            $"[{fixtureId}] fetchmany batch1 row count: expected {expFirstBatch.Length}, got {firstBatch.Length}");
        for (var ri = 0; ri < expFirstBatch.Length; ri++)
            for (var ci = 0; ci < expFirstBatch[ri].Length; ci++)
                Assert.True(Helpers.RowValuesEqual(expFirstBatch[ri][ci], firstBatch[ri][ci]),
                    $"[{fixtureId}] fetchmany batch1 row {ri}, col {ci}");

        // Second batch = rows[size..]
        var secondBatch = result.Rows.Skip(size).ToArray();
        Assert.True(expSecondBatch.Length == secondBatch.Length,
            $"[{fixtureId}] fetchmany batch2 row count: expected {expSecondBatch.Length}, got {secondBatch.Length}");
        for (var ri = 0; ri < expSecondBatch.Length; ri++)
            for (var ci = 0; ci < expSecondBatch[ri].Length; ci++)
                Assert.True(Helpers.RowValuesEqual(expSecondBatch[ri][ci], secondBatch[ri][ci]),
                    $"[{fixtureId}] fetchmany batch2 row {ri}, col {ci}");
    }

    // fetchall_test / fetchall_empty_test: execute sql and compare all rows.
    private static void FetchAllTestStep(MiniSqliteConnection conn, JsonElement step, string fixtureId)
    {
        var sql    = step.GetProperty("sql").GetString() ?? "";
        var result = conn.Execute(sql);

        if (step.TryGetProperty("expected_rows", out var expRowsEl))
        {
            var expRows = expRowsEl.EnumerateArray()
                .Select(r => r.EnumerateArray().Select(UnwrapJsonParam).ToArray()).ToArray();
            Assert.True(expRows.Length == result.Rows.Count,
                $"[{fixtureId}] fetchall row count: expected {expRows.Length}, got {result.Rows.Count}");
            for (var ri = 0; ri < expRows.Length; ri++)
                for (var ci = 0; ci < expRows[ri].Length; ci++)
                    Assert.True(Helpers.RowValuesEqual(expRows[ri][ci], result.Rows[ri][ci]),
                        $"[{fixtureId}] fetchall row {ri}, col {ci}");
        }
    }

    // expect_error: execute the SQL and verify a MiniSqliteException is thrown.
    private static void ExpectErrorStep(MiniSqliteConnection conn, JsonElement step, string fixtureId)
    {
        var sql       = step.GetProperty("sql").GetString() ?? "";
        var @params    = ReadParams(step);
        var errorType = step.TryGetProperty("error_type", out var et) ? et.GetString() : null;

        var ex = Assert.Throws<MiniSqliteException>(() => conn.Execute(sql, @params));
        if (errorType is not null)
            Assert.True(errorType == ex.Kind,
                $"[{fixtureId}] Expected error_type '{errorType}', got '{ex.Kind}' (message: {ex.Message})");
    }

    // Extract positional parameters from a step's "params" field.
    private static object?[] ReadParams(JsonElement step)
    {
        if (!step.TryGetProperty("params", out var ps))
            return Array.Empty<object?>();
        return ps.EnumerateArray().Select(UnwrapJsonParam).ToArray();
    }

    // Convert a JsonElement into a C# value suitable as a SQL parameter.
    private static object? UnwrapJsonParam(JsonElement je) => je.ValueKind switch
    {
        JsonValueKind.Null   => null,
        JsonValueKind.True   => true,
        JsonValueKind.False  => false,
        JsonValueKind.Number => je.TryGetInt64(out var i) ? (object)i : je.GetDouble(),
        JsonValueKind.String => je.GetString(),
        _                    => je.ToString(),
    };
}

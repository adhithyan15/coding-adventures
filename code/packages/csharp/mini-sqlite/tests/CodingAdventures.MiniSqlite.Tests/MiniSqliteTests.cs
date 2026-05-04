namespace CodingAdventures.MiniSqlite.Tests;

public sealed class MiniSqliteTests
{
    [Fact]
    public void ExposesDbApiStyleConstants()
    {
        Assert.Equal("2.0", MiniSqlite.ApiLevel);
        Assert.Equal(1, MiniSqlite.ThreadSafety);
        Assert.Equal("qmark", MiniSqlite.ParamStyle);
    }

    [Fact]
    public void CreatesInsertsAndSelectsRows()
    {
        using var conn = MiniSqlite.Connect(":memory:");
        conn.Execute("CREATE TABLE users (id INTEGER, name TEXT, active BOOLEAN)");
        conn.ExecuteMany("INSERT INTO users VALUES (?, ?, ?)", new[]
        {
            new object?[] { 1, "Alice", true },
            new object?[] { 2, "Bob", false },
            new object?[] { 3, "Carol", true },
        });

        var cursor = conn.Execute("SELECT name FROM users WHERE active = ? ORDER BY id ASC", true);
        Assert.Equal("name", cursor.Description[0].Name);
        var rows = cursor.FetchAll();
        Assert.Equal("Alice", rows[0][0]);
        Assert.Equal("Carol", rows[1][0]);
    }

    [Fact]
    public void FetchesIncrementally()
    {
        using var conn = MiniSqlite.Connect(":memory:");
        conn.Execute("CREATE TABLE nums (n INTEGER)");
        conn.ExecuteMany("INSERT INTO nums VALUES (?)", new[]
        {
            new object?[] { 1 },
            new object?[] { 2 },
            new object?[] { 3 },
        });
        var cursor = conn.Execute("SELECT n FROM nums ORDER BY n ASC");

        Assert.Equal(1L, Convert.ToInt64(cursor.FetchOne()![0]));
        Assert.Equal(2L, Convert.ToInt64(cursor.FetchMany(1)[0][0]));
        Assert.Equal(3L, Convert.ToInt64(cursor.FetchAll()[0][0]));
        Assert.Null(cursor.FetchOne());
    }

    [Fact]
    public void UpdatesAndDeletesRows()
    {
        using var conn = MiniSqlite.Connect(":memory:");
        conn.Execute("CREATE TABLE users (id INTEGER, name TEXT)");
        conn.ExecuteMany("INSERT INTO users VALUES (?, ?)", new[]
        {
            new object?[] { 1, "Alice" },
            new object?[] { 2, "Bob" },
            new object?[] { 3, "Carol" },
        });

        var updated = conn.Execute("UPDATE users SET name = ? WHERE id = ?", "Bobby", 2);
        Assert.Equal(1, updated.RowCount);

        var deleted = conn.Execute("DELETE FROM users WHERE id IN (?, ?)", 1, 3);
        Assert.Equal(2, deleted.RowCount);

        var rows = conn.Execute("SELECT id, name FROM users").FetchAll();
        Assert.Equal(2L, Convert.ToInt64(rows[0][0]));
        Assert.Equal("Bobby", rows[0][1]);
    }

    [Fact]
    public void RollsBackAndCommitsSnapshots()
    {
        using var conn = MiniSqlite.Connect(":memory:");
        conn.Execute("CREATE TABLE users (id INTEGER, name TEXT)");
        conn.Commit();
        conn.Execute("INSERT INTO users VALUES (?, ?)", 1, "Alice");
        conn.Rollback();
        Assert.Empty(conn.Execute("SELECT * FROM users").FetchAll());

        conn.Execute("INSERT INTO users VALUES (?, ?)", 1, "Alice");
        conn.Commit();
        conn.Rollback();
        Assert.Single(conn.Execute("SELECT * FROM users").FetchAll());
    }

    [Fact]
    public void RejectsFileBackedConnections()
    {
        var error = Assert.Throws<MiniSqliteException>(() => MiniSqlite.Connect("app.db"));
        Assert.Equal("NotSupportedError", error.Kind);
    }
}

namespace CodingAdventures.SqlBackend.Tests;

internal static class TestBackends
{
    public static InMemoryBackend MakeUsers()
    {
        var backend = new InMemoryBackend();
        backend.CreateTable(
            "users",
            new[]
            {
                new ColumnDef("id", "INTEGER", PrimaryKey: true),
                new ColumnDef("name", "TEXT", NotNull: true),
                new ColumnDef("age", "INTEGER"),
                new ColumnDef("email", "TEXT", Unique: true),
            },
            ifNotExists: false);
        backend.Insert("users", new Row { ["id"] = 1, ["name"] = "Alice", ["age"] = 30, ["email"] = "alice@example.com" });
        backend.Insert("users", new Row { ["id"] = 2, ["name"] = "Bob", ["age"] = 25, ["email"] = "bob@example.com" });
        backend.Insert("users", new Row { ["id"] = 3, ["name"] = "Carol", ["age"] = null, ["email"] = null });
        return backend;
    }
}

public sealed class SqlValueTests
{
    [Fact]
    public void TypeNamesMatchPortableValueSet()
    {
        Assert.Equal("NULL", SqlValues.TypeName(null));
        Assert.Equal("BOOLEAN", SqlValues.TypeName(true));
        Assert.Equal("INTEGER", SqlValues.TypeName(42));
        Assert.Equal("REAL", SqlValues.TypeName(1.5));
        Assert.Equal("TEXT", SqlValues.TypeName("hello"));
        Assert.Equal("BLOB", SqlValues.TypeName(new byte[] { 1, 2 }));
    }

    [Fact]
    public void RejectsNonSqlValues()
    {
        Assert.False(SqlValues.IsSqlValue(new object()));
        Assert.Throws<ArgumentException>(() => SqlValues.TypeName(new object()));
    }
}

public sealed class RowIteratorTests
{
    [Fact]
    public void ListRowIteratorReturnsCopies()
    {
        var rows = new[] { new Row { ["id"] = 1, ["name"] = "alice" } };
        var iterator = new ListRowIterator(rows);
        var first = iterator.Next();

        Assert.NotNull(first);
        first["name"] = "mutated";
        Assert.Equal("alice", rows[0]["name"]);
        Assert.Null(iterator.Next());
    }

    [Fact]
    public void ListCursorTracksCurrentRow()
    {
        var rows = new List<Row> { new() { ["id"] = 1 }, new() { ["id"] = 2 } };
        var cursor = new ListCursor(rows);

        Assert.Null(cursor.CurrentRow);
        Assert.Equal(1, cursor.Next()?["id"]);
        Assert.Equal(1, cursor.CurrentRow?["id"]);
        Assert.Equal(2, cursor.Next()?["id"]);
        cursor.Close();
        Assert.Null(cursor.Next());
    }
}

public sealed class SchemaTests
{
    [Fact]
    public void ColumnFlagsExposeEffectiveConstraints()
    {
        var pk = new ColumnDef("id", "INTEGER", PrimaryKey: true);
        var unique = new ColumnDef("email", "TEXT", Unique: true);
        var withNullDefault = ColumnDef.WithDefault("middle", "TEXT", null);

        Assert.True(pk.EffectiveNotNull);
        Assert.True(pk.EffectiveUnique);
        Assert.False(unique.EffectiveNotNull);
        Assert.True(unique.EffectiveUnique);
        Assert.True(withNullDefault.HasDefault);
        Assert.Null(withNullDefault.DefaultValue);
    }

    [Fact]
    public void SchemaProviderAdapterReturnsColumnNames()
    {
        var backend = TestBackends.MakeUsers();
        var schema = BackendAdapters.AsSchemaProvider(backend);

        Assert.Equal(new[] { "id", "name", "age", "email" }, schema.Columns("users"));
        Assert.Throws<TableNotFound>(() => schema.Columns("missing"));
    }
}

public sealed class InMemoryBackendTests
{
    [Fact]
    public void TablesAndColumnsExposeSchema()
    {
        var backend = TestBackends.MakeUsers();

        Assert.Contains("users", backend.Tables());
        Assert.Equal("name", backend.Columns("users")[1].Name);
        Assert.Throws<TableNotFound>(() => backend.Columns("missing"));
    }

    [Fact]
    public void ScanYieldsRowsInInsertionOrder()
    {
        var backend = TestBackends.MakeUsers();
        usingIterator(backend.Scan("users"), rows =>
        {
            Assert.Equal(new object?[] { 1, 2, 3 }, rows.Select(row => row["id"]).ToArray());
        });
    }

    [Fact]
    public void InsertAppliesDefaultsAndRejectsUnknownColumns()
    {
        var backend = new InMemoryBackend();
        backend.CreateTable(
            "items",
            new[]
            {
                new ColumnDef("id", "INTEGER", PrimaryKey: true),
                ColumnDef.WithDefault("status", "TEXT", "active"),
            },
            ifNotExists: false);

        backend.Insert("items", new Row { ["id"] = 1 });
        Assert.Throws<ColumnNotFound>(() => backend.Insert("items", new Row { ["id"] = 2, ["ghost"] = "x" }));

        usingIterator(backend.Scan("items"), rows =>
        {
            Assert.Equal("active", rows.Single()["status"]);
        });
    }

    [Fact]
    public void InsertEnforcesPrimaryKeyAndUniqueConstraints()
    {
        var backend = TestBackends.MakeUsers();

        Assert.Throws<ConstraintViolation>(() => backend.Insert("users", new Row
        {
            ["id"] = 1,
            ["name"] = "duplicate",
            ["age"] = 99,
            ["email"] = "dup@example.com",
        }));

        Assert.Throws<ConstraintViolation>(() => backend.Insert("users", new Row
        {
            ["id"] = 4,
            ["name"] = "duplicate",
            ["age"] = 99,
            ["email"] = "alice@example.com",
        }));
    }

    [Fact]
    public void UniqueColumnsAllowMultipleNulls()
    {
        var backend = new InMemoryBackend();
        backend.CreateTable(
            "users",
            new[] { new ColumnDef("id", "INTEGER", PrimaryKey: true), new ColumnDef("email", "TEXT", Unique: true) },
            ifNotExists: false);

        backend.Insert("users", new Row { ["id"] = 1, ["email"] = null });
        backend.Insert("users", new Row { ["id"] = 2, ["email"] = null });

        usingIterator(backend.Scan("users"), rows => Assert.Equal(2, rows.Count));
    }

    [Fact]
    public void PositionedUpdateAndDeleteUseCursor()
    {
        var backend = TestBackends.MakeUsers();
        var cursor = backend.OpenCursor("users");
        Assert.Equal(1, cursor.Next()?["id"]);

        backend.Update("users", cursor, new Dictionary<string, object?> { ["name"] = "ALICE" });
        Assert.Equal("ALICE", backend.OpenCursor("users").Next()?["name"]);

        backend.Delete("users", cursor);
        Assert.Equal(2, backend.OpenCursor("users").Next()?["id"]);
    }

    [Fact]
    public void UpdateRejectsInvalidCursorAndUnknownColumns()
    {
        var backend = TestBackends.MakeUsers();
        var cursor = backend.OpenCursor("users");

        Assert.Throws<Unsupported>(() => backend.Update("users", cursor, new Dictionary<string, object?> { ["name"] = "x" }));

        cursor.Next();
        Assert.Throws<ColumnNotFound>(() => backend.Update("users", cursor, new Dictionary<string, object?> { ["ghost"] = "x" }));
        Assert.Throws<Unsupported>(() => backend.Update("users", new FakeCursor(), new Dictionary<string, object?>()));
    }

    [Fact]
    public void CreateDropAndAddColumnImplementDdl()
    {
        var backend = new InMemoryBackend();
        backend.CreateTable("t", new[] { new ColumnDef("id", "INTEGER") }, ifNotExists: false);
        backend.CreateTable("t", Array.Empty<ColumnDef>(), ifNotExists: true);
        Assert.Throws<TableAlreadyExists>(() => backend.CreateTable("t", Array.Empty<ColumnDef>(), ifNotExists: false));

        backend.AddColumn("t", ColumnDef.WithDefault("status", "TEXT", "new"));
        backend.Insert("t", new Row { ["id"] = 1 });
        usingIterator(backend.Scan("t"), rows => Assert.Equal("new", rows.Single()["status"]));

        Assert.Throws<ColumnAlreadyExists>(() => backend.AddColumn("t", new ColumnDef("status", "TEXT")));
        Assert.Throws<ConstraintViolation>(() => backend.AddColumn("t", new ColumnDef("must_have", "TEXT", NotNull: true)));

        backend.DropTable("t", ifExists: false);
        backend.DropTable("t", ifExists: true);
        Assert.Throws<TableNotFound>(() => backend.DropTable("t", ifExists: false));
    }

    [Fact]
    public void TransactionsCommitAndRollback()
    {
        var backend = TestBackends.MakeUsers();
        var handle = backend.BeginTransaction();
        backend.Insert("users", new Row { ["id"] = 4, ["name"] = "Dave", ["age"] = 41, ["email"] = "dave@example.com" });
        backend.Rollback(handle);
        usingIterator(backend.Scan("users"), rows => Assert.DoesNotContain(rows, row => Equals(row["id"], 4)));

        var committed = backend.BeginTransaction();
        backend.Insert("users", new Row { ["id"] = 4, ["name"] = "Dave", ["age"] = 41, ["email"] = "dave@example.com" });
        backend.Commit(committed);
        usingIterator(backend.Scan("users"), rows => Assert.Contains(rows, row => Equals(row["id"], 4)));
    }

    [Fact]
    public void TransactionsRejectNestedAndStaleHandles()
    {
        var backend = TestBackends.MakeUsers();
        var first = backend.BeginTransaction();

        Assert.Equal(first, backend.CurrentTransaction());
        Assert.Throws<Unsupported>(() => backend.BeginTransaction());
        backend.Commit(first);
        Assert.Throws<Unsupported>(() => backend.Commit(first));
    }

    [Fact]
    public void IndexesCanBeListedScannedAndDropped()
    {
        var backend = TestBackends.MakeUsers();
        backend.CreateIndex(new IndexDef("idx_age", "users", new[] { "age" }));

        Assert.Equal("idx_age", backend.ListIndexes("users").Single().Name);
        var rowids = backend.ScanIndex("idx_age", new object?[] { 25 }, new object?[] { 30 }).ToArray();
        Assert.Equal(new[] { 1, 0 }, rowids);

        usingIterator(backend.ScanByRowIds("users", rowids), rows =>
        {
            Assert.Equal(new object?[] { 2, 1 }, rows.Select(row => row["id"]).ToArray());
        });

        backend.DropIndex("idx_age");
        Assert.Empty(backend.ListIndexes());
        Assert.Throws<IndexNotFound>(() => backend.DropIndex("idx_age"));
        backend.DropIndex("idx_age", ifExists: true);
    }

    [Fact]
    public void IndexCreationValidatesInputs()
    {
        var backend = TestBackends.MakeUsers();
        backend.CreateIndex(new IndexDef("idx_email", "users", new[] { "email" }, unique: true));

        Assert.Throws<IndexAlreadyExists>(() => backend.CreateIndex(new IndexDef("idx_email", "users", new[] { "email" })));
        Assert.Throws<TableNotFound>(() => backend.CreateIndex(new IndexDef("bad_table", "missing", new[] { "id" })));
        Assert.Throws<ColumnNotFound>(() => backend.CreateIndex(new IndexDef("bad_column", "users", new[] { "missing" })));
        Assert.Throws<IndexNotFound>(() => backend.ScanIndex("missing", null, null).ToArray());
    }

    [Fact]
    public void DefaultSavepointsAndTriggersExposeOptionalOperations()
    {
        var backend = TestBackends.MakeUsers();

        Assert.Throws<Unsupported>(() => backend.CreateSavepoint("s1"));
        Assert.Throws<Unsupported>(() => backend.CreateTrigger(new TriggerDef("tr", "users", "AFTER", "INSERT", "SELECT 1")));
        Assert.Empty(backend.ListTriggers("users"));
    }

    private static void usingIterator(IRowIterator iterator, Action<IReadOnlyList<Row>> assertions)
    {
        var rows = new List<Row>();
        try
        {
            while (iterator.Next() is { } row)
            {
                rows.Add(row);
            }

            assertions(rows);
        }
        finally
        {
            iterator.Close();
        }
    }

    private sealed class FakeCursor : ICursor
    {
        public Row? CurrentRow => null;
        public Row? Next() => null;
        public void Close()
        {
        }
    }
}

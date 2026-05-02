namespace CodingAdventures.SqlBackend.Tests

open System
open System.Collections.Generic
open Xunit
open CodingAdventures.SqlBackend.FSharp

module private Fixtures =
    let row (pairs: (string * obj) list) =
        let r = Row()
        for key, value in pairs do
            r[key] <- value
        r

    let assignments (pairs: (string * obj) list) =
        let d = Dictionary<string, obj>(StringComparer.OrdinalIgnoreCase)
        for key, value in pairs do
            d[key] <- value
        d :> IReadOnlyDictionary<string, obj>

    let users () =
        let backend = InMemoryBackend()
        backend.CreateTable(
            "users",
            [| ColumnDef("id", "INTEGER", primaryKey = true)
               ColumnDef("name", "TEXT", notNull = true)
               ColumnDef("age", "INTEGER")
               ColumnDef("email", "TEXT", unique = true) |],
            false)

        backend.Insert("users", row [ "id", box 1; "name", box "Alice"; "age", box 30; "email", box "alice@example.com" ])
        backend.Insert("users", row [ "id", box 2; "name", box "Bob"; "age", box 25; "email", box "bob@example.com" ])
        backend.Insert("users", row [ "id", box 3; "name", box "Carol"; "age", null; "email", null ])
        backend

    let collect (iterator: IRowIterator) =
        let rows = ResizeArray<Row>()
        try
            let mutable next = iterator.Next()
            while not (isNull next) do
                rows.Add(next)
                next <- iterator.Next()
            rows |> Seq.toArray
        finally
            iterator.Close()

module SqlValueTests =
    [<Fact>]
    let ``classifies SQL values`` () =
        Assert.Equal("NULL", SqlValues.typeName null)
        Assert.Equal("BOOLEAN", SqlValues.typeName (box true))
        Assert.Equal("INTEGER", SqlValues.typeName (box 42))
        Assert.Equal("REAL", SqlValues.typeName (box 1.5))
        Assert.Equal("TEXT", SqlValues.typeName (box "hello"))
        Assert.Equal("BLOB", SqlValues.typeName (box [| byte 1 |]))
        Assert.False(SqlValues.isSqlValue (box (obj())))
        Assert.Throws<ArgumentException>(fun () -> SqlValues.typeName (box (obj())) |> ignore) |> ignore

module IteratorTests =
    [<Fact>]
    let ``list iterator returns row copies`` () =
        let source = [| Fixtures.row [ "id", box 1; "name", box "Alice" ] |]
        let iterator = ListRowIterator(source) :> IRowIterator
        let row = iterator.Next()
        row["name"] <- box "mutated"
        Assert.Equal("Alice", source[0]["name"] :?> string)
        Assert.Null(iterator.Next())

    [<Fact>]
    let ``cursor tracks current row`` () =
        let rows = ResizeArray([ Fixtures.row [ "id", box 1 ]; Fixtures.row [ "id", box 2 ] ])
        let cursor = ListCursor(rows) :> ICursor
        Assert.Null(cursor.CurrentRow)
        Assert.Equal(1, cursor.Next()["id"] :?> int)
        Assert.Equal(1, cursor.CurrentRow["id"] :?> int)
        Assert.Equal(2, cursor.Next()["id"] :?> int)
        cursor.Close()
        Assert.Null(cursor.Next())

module SchemaTests =
    [<Fact>]
    let ``column metadata exposes effective constraints`` () =
        let pk = ColumnDef("id", "INTEGER", primaryKey = true)
        let unique = ColumnDef("email", "TEXT", unique = true)
        let withNullDefault = ColumnDef.WithDefault("middle", "TEXT", null)

        Assert.True(pk.EffectiveNotNull)
        Assert.True(pk.EffectiveUnique)
        Assert.False(unique.EffectiveNotNull)
        Assert.True(unique.EffectiveUnique)
        Assert.True(withNullDefault.HasDefault)
        Assert.Null(withNullDefault.DefaultValue)

    [<Fact>]
    let ``schema provider adapter returns column names`` () =
        let backend = Fixtures.users ()
        let schema = BackendAdapters.asSchemaProvider backend

        Assert.Equal<string array>([| "id"; "name"; "age"; "email" |], schema.Columns("users") |> Seq.toArray)
        Assert.Throws<TableNotFound>(fun () -> schema.Columns("missing") |> ignore) |> ignore

module InMemoryBackendTests =
    [<Fact>]
    let ``tables columns and scan expose data`` () =
        let backend = Fixtures.users ()
        Assert.Contains("users", backend.Tables())
        Assert.Equal("name", (backend.Columns("users")).[1].Name)

        let ids = backend.Scan("users") |> Fixtures.collect |> Array.map (fun row -> row["id"])
        Assert.Equal<obj array>([| box 1; box 2; box 3 |], ids)
        Assert.Throws<TableNotFound>(fun () -> backend.Columns("missing") |> ignore) |> ignore

    [<Fact>]
    let ``insert applies defaults and rejects bad rows`` () =
        let backend = InMemoryBackend()
        backend.CreateTable(
            "items",
            [| ColumnDef("id", "INTEGER", primaryKey = true)
               ColumnDef.WithDefault("status", "TEXT", box "active") |],
            false)

        backend.Insert("items", Fixtures.row [ "id", box 1 ])
        Assert.Throws<ColumnNotFound>(fun () -> backend.Insert("items", Fixtures.row [ "id", box 2; "ghost", box "x" ])) |> ignore

        let rows = backend.Scan("items") |> Fixtures.collect
        Assert.Equal("active", rows[0]["status"] :?> string)

    [<Fact>]
    let ``constraints enforce primary key unique and not null`` () =
        let backend = Fixtures.users ()

        Assert.Throws<ConstraintViolation>(fun () ->
            backend.Insert("users", Fixtures.row [ "id", box 1; "name", box "Duplicate"; "age", box 9; "email", box "dup@example.com" ]))
        |> ignore

        Assert.Throws<ConstraintViolation>(fun () ->
            backend.Insert("users", Fixtures.row [ "id", box 4; "name", null; "age", box 9; "email", box "dup@example.com" ]))
        |> ignore

        Assert.Throws<ConstraintViolation>(fun () ->
            backend.Insert("users", Fixtures.row [ "id", box 4; "name", box "Duplicate"; "age", box 9; "email", box "alice@example.com" ]))
        |> ignore

    [<Fact>]
    let ``unique columns allow multiple null values`` () =
        let backend = InMemoryBackend()
        backend.CreateTable("users", [| ColumnDef("id", "INTEGER", primaryKey = true); ColumnDef("email", "TEXT", unique = true) |], false)
        backend.Insert("users", Fixtures.row [ "id", box 1; "email", null ])
        backend.Insert("users", Fixtures.row [ "id", box 2; "email", null ])
        Assert.Equal(2, backend.Scan("users") |> Fixtures.collect |> Array.length)

    [<Fact>]
    let ``positioned update and delete use cursors`` () =
        let backend = Fixtures.users ()
        let cursor = backend.OpenCursor("users") :> ICursor
        Assert.Equal(1, (cursor.Next()).["id"] :?> int)

        backend.Update("users", cursor, Fixtures.assignments [ "name", box "ALICE" ])
        Assert.Equal("ALICE", ((backend.OpenCursor("users") :> ICursor).Next()).["name"] :?> string)

        backend.Delete("users", cursor)
        Assert.Equal(2, ((backend.OpenCursor("users") :> ICursor).Next()).["id"] :?> int)

    [<Fact>]
    let ``update validates cursor and columns`` () =
        let backend = Fixtures.users ()
        let cursor = backend.OpenCursor("users") :> ICursor

        Assert.Throws<Unsupported>(fun () -> backend.Update("users", cursor, Fixtures.assignments [ "name", box "x" ])) |> ignore
        cursor.Next() |> ignore
        Assert.Throws<ColumnNotFound>(fun () -> backend.Update("users", cursor, Fixtures.assignments [ "ghost", box "x" ])) |> ignore

    [<Fact>]
    let ``ddl creates drops and alters tables`` () =
        let backend = InMemoryBackend()
        backend.CreateTable("t", [| ColumnDef("id", "INTEGER") |], false)
        backend.CreateTable("t", Array.empty, true)
        Assert.Throws<TableAlreadyExists>(fun () -> backend.CreateTable("t", Array.empty, false)) |> ignore

        backend.AddColumn("t", ColumnDef.WithDefault("status", "TEXT", box "new"))
        backend.Insert("t", Fixtures.row [ "id", box 1 ])
        Assert.Equal("new", ((backend.Scan("t") |> Fixtures.collect).[0]).["status"] :?> string)
        Assert.Throws<ColumnAlreadyExists>(fun () -> backend.AddColumn("t", ColumnDef("status", "TEXT"))) |> ignore
        Assert.Throws<ConstraintViolation>(fun () -> backend.AddColumn("t", ColumnDef("required", "TEXT", notNull = true))) |> ignore

        backend.DropTable("t", false)
        backend.DropTable("t", true)
        Assert.Throws<TableNotFound>(fun () -> backend.DropTable("t", false)) |> ignore

    [<Fact>]
    let ``transactions commit rollback and reject stale handles`` () =
        let backend = Fixtures.users ()
        let handle = backend.BeginTransaction()
        backend.Insert("users", Fixtures.row [ "id", box 4; "name", box "Dave"; "age", box 41; "email", box "dave@example.com" ])
        backend.Rollback(handle)
        Assert.DoesNotContain(backend.Scan("users") |> Fixtures.collect, fun row -> Object.Equals(row["id"], box 4))

        let committed = backend.BeginTransaction()
        backend.Insert("users", Fixtures.row [ "id", box 4; "name", box "Dave"; "age", box 41; "email", box "dave@example.com" ])
        backend.Commit(committed)
        Assert.Contains(backend.Scan("users") |> Fixtures.collect, fun row -> Object.Equals(row["id"], box 4))

        let active = backend.BeginTransaction()
        Assert.Equal(Some active, backend.CurrentTransaction())
        Assert.Throws<Unsupported>(fun () -> backend.BeginTransaction() |> ignore) |> ignore
        backend.Commit(active)
        Assert.Throws<Unsupported>(fun () -> backend.Commit(active)) |> ignore

    [<Fact>]
    let ``indexes list scan and drop`` () =
        let backend = Fixtures.users ()
        backend.CreateIndex(IndexDef("idx_age", "users", columns = [ "age" ]))

        Assert.Equal("idx_age", (backend.ListIndexes(Some "users")).[0].Name)
        let rowids = backend.ScanIndex("idx_age", Some([| box 25 |]), Some([| box 30 |]), true, true) |> Seq.toArray
        Assert.Equal<int array>([| 1; 0 |], rowids)

        let ids = backend.ScanByRowIds("users", rowids) |> Fixtures.collect |> Array.map (fun row -> row["id"])
        Assert.Equal<obj array>([| box 2; box 1 |], ids)

        backend.DropIndex("idx_age", false)
        Assert.Empty(backend.ListIndexes(None))
        Assert.Throws<IndexNotFound>(fun () -> backend.DropIndex("idx_age", false)) |> ignore
        backend.DropIndex("idx_age", true)

    [<Fact>]
    let ``index creation validates inputs`` () =
        let backend = Fixtures.users ()
        backend.CreateIndex(IndexDef("idx_email", "users", columns = [ "email" ], unique = true))

        Assert.Throws<IndexAlreadyExists>(fun () -> backend.CreateIndex(IndexDef("idx_email", "users", columns = [ "email" ]))) |> ignore
        Assert.Throws<TableNotFound>(fun () -> backend.CreateIndex(IndexDef("idx_bad_table", "missing", columns = [ "id" ]))) |> ignore
        Assert.Throws<ColumnNotFound>(fun () -> backend.CreateIndex(IndexDef("idx_bad_column", "users", columns = [ "missing" ]))) |> ignore
        Assert.Throws<IndexNotFound>(fun () -> backend.ScanIndex("missing", None, None, true, true) |> Seq.toArray |> ignore) |> ignore

    [<Fact>]
    let ``optional savepoints and triggers default cleanly`` () =
        let backend = Fixtures.users ()
        Assert.Throws<Unsupported>(fun () -> backend.CreateSavepoint("s1")) |> ignore
        Assert.Throws<Unsupported>(fun () -> backend.CreateTrigger(TriggerDef("tr", "users", "AFTER", "INSERT", "SELECT 1"))) |> ignore
        Assert.Empty(backend.ListTriggers("users"))

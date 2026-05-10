namespace CodingAdventures.MiniSqlite.Tests

open System
open System.Collections.Generic
open Xunit
open CodingAdventures.MiniSqlite.FSharp

module MiniSqliteTests =
    let private row (values: obj list) = values |> List.toArray :> IReadOnlyList<obj>

    [<Fact>]
    let ``exposes DB API style constants`` () =
        Assert.Equal("2.0", MiniSqlite.ApiLevel)
        Assert.Equal(1, MiniSqlite.ThreadSafety)
        Assert.Equal("qmark", MiniSqlite.ParamStyle)

    [<Fact>]
    let ``creates inserts and selects rows`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE users (id INTEGER, name TEXT, active BOOLEAN)") |> ignore
        conn.ExecuteMany(
            "INSERT INTO users VALUES (?, ?, ?)",
            [ row [ box 1; box "Alice"; box true ]
              row [ box 2; box "Bob"; box false ]
              row [ box 3; box "Carol"; box true ] ])
        |> ignore

        let cursor = conn.Execute("SELECT name FROM users WHERE active = ? ORDER BY id ASC", box true)
        Assert.Equal("name", cursor.Description[0].Name)
        let rows = cursor.FetchAll()
        Assert.Equal("Alice", (rows[0]).[0] :?> string)
        Assert.Equal("Carol", (rows[1]).[0] :?> string)

    [<Fact>]
    let ``fetches incrementally`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE nums (n INTEGER)") |> ignore
        conn.ExecuteMany("INSERT INTO nums VALUES (?)", [ row [ box 1 ]; row [ box 2 ]; row [ box 3 ] ]) |> ignore

        let cursor = conn.Execute("SELECT n FROM nums ORDER BY n ASC")

        Assert.Equal(1L, Convert.ToInt64(cursor.FetchOne()[0]))
        Assert.Equal(2L, Convert.ToInt64((cursor.FetchMany(1)[0]).[0]))
        Assert.Equal(3L, Convert.ToInt64((cursor.FetchAll()[0]).[0]))
        Assert.Null(cursor.FetchOne())

    [<Fact>]
    let ``updates and deletes rows`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE users (id INTEGER, name TEXT)") |> ignore
        conn.ExecuteMany(
            "INSERT INTO users VALUES (?, ?)",
            [ row [ box 1; box "Alice" ]
              row [ box 2; box "Bob" ]
              row [ box 3; box "Carol" ] ])
        |> ignore

        let updated = conn.Execute("UPDATE users SET name = ? WHERE id = ?", box "Bobby", box 2)
        Assert.Equal(1, updated.RowCount)

        let deleted = conn.Execute("DELETE FROM users WHERE id IN (?, ?)", box 1, box 3)
        Assert.Equal(2, deleted.RowCount)

        let rows = conn.Execute("SELECT id, name FROM users").FetchAll()
        Assert.Equal(2L, Convert.ToInt64((rows[0]).[0]))
        Assert.Equal("Bobby", (rows[0]).[1] :?> string)

    [<Fact>]
    let ``rolls back and commits snapshots`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE users (id INTEGER, name TEXT)") |> ignore
        conn.Commit()
        conn.Execute("INSERT INTO users VALUES (?, ?)", box 1, box "Alice") |> ignore
        conn.Rollback()
        Assert.Empty(conn.Execute("SELECT * FROM users").FetchAll())

        conn.Execute("INSERT INTO users VALUES (?, ?)", box 1, box "Alice") |> ignore
        conn.Commit()
        conn.Rollback()
        Assert.Single(conn.Execute("SELECT * FROM users").FetchAll()) |> ignore

    [<Fact>]
    let ``rejects file backed connections`` () =
        let error = Assert.Throws<MiniSqliteException>(fun () -> MiniSqlite.Connect("app.db") |> ignore)
        Assert.Equal("NotSupportedError", error.Kind)

    [<Fact>]
    let ``supports null predicates comparisons ordering and drop`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE things (id INTEGER, label TEXT, score REAL, enabled BOOLEAN)") |> ignore
        conn.Execute("INSERT INTO things VALUES (1, NULL, 1.5, TRUE)") |> ignore
        conn.Execute("INSERT INTO things VALUES (2, 'middle', 2.5, FALSE)") |> ignore
        conn.Execute("INSERT INTO things VALUES (3, 'tail', 3.5, TRUE)") |> ignore

        let nullOrHigh = conn.Execute("SELECT id FROM things WHERE label IS NULL OR score >= 3 ORDER BY id DESC").FetchAll()
        Assert.Equal(3L, Convert.ToInt64((nullOrHigh[0]).[0]))
        Assert.Equal(1L, Convert.ToInt64((nullOrHigh[1]).[0]))

        let filtered = conn.Execute("SELECT id FROM things WHERE label IS NOT NULL AND id <> 2 ORDER BY id ASC").FetchAll()
        Assert.Equal(3L, Convert.ToInt64((filtered[0]).[0]))

        let below = conn.Execute("SELECT id FROM things WHERE score < 2").FetchAll()
        Assert.Equal(1L, Convert.ToInt64((below[0]).[0]))

        conn.Execute("DROP TABLE things") |> ignore
        let error = Assert.Throws<MiniSqliteException>(fun () -> conn.Execute("SELECT * FROM things") |> ignore)
        Assert.Equal("OperationalError", error.Kind)

    [<Fact>]
    let ``validates parameters and cursor lifecycle`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE notes (id INTEGER, text TEXT)") |> ignore

        let inserted = conn.Execute("INSERT INTO notes VALUES (?, 'literal ? with ''quote''')", row [ box 1 ])
        Assert.Equal(1, inserted.RowCount)
        Assert.Equal(1L, Convert.ToInt64(inserted.LastRowId))

        let cursor = conn.Execute("SELECT text FROM notes")
        cursor.ArraySize <- 1
        Assert.Equal(1, cursor.ArraySize)
        let batch = cursor.FetchMany()
        Assert.Single(batch) |> ignore
        Assert.Equal("literal ? with 'quote'", (batch[0]).[0] :?> string)

        (cursor :> IDisposable).Dispose()
        let closed = Assert.Throws<MiniSqliteException>(fun () -> cursor.FetchAll() |> ignore)
        Assert.Equal("ProgrammingError", closed.Kind)

        let tooFew = Assert.Throws<MiniSqliteException>(fun () -> conn.Execute("SELECT * FROM notes WHERE id = ?", Array.empty<obj> :> IReadOnlyList<obj>) |> ignore)
        Assert.Equal("ProgrammingError", tooFew.Kind)

        let tooMany = Assert.Throws<MiniSqliteException>(fun () -> conn.Execute("SELECT * FROM notes", row [ box 1 ]) |> ignore)
        Assert.Equal("ProgrammingError", tooMany.Kind)

        let unsupported = Assert.Throws<MiniSqliteException>(fun () -> conn.Execute("PRAGMA user_version") |> ignore)
        Assert.Equal("OperationalError", unsupported.Kind)

    [<Fact>]
    let ``supports SQL transaction commands and autocommit`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE events (id INTEGER)") |> ignore
        conn.Commit()
        conn.Execute("BEGIN") |> ignore
        conn.Execute("INSERT INTO events VALUES (1)") |> ignore
        conn.Execute("ROLLBACK") |> ignore
        Assert.Empty(conn.Execute("SELECT * FROM events").FetchAll())

        conn.Execute("BEGIN") |> ignore
        conn.Execute("INSERT INTO events VALUES (2)") |> ignore
        conn.Execute("COMMIT") |> ignore
        conn.Execute("ROLLBACK") |> ignore
        Assert.Single(conn.Execute("SELECT * FROM events").FetchAll()) |> ignore

        use autocommit = MiniSqlite.Connect(":memory:", options = { Autocommit = true })
        autocommit.Execute("CREATE TABLE events (id INTEGER)") |> ignore
        autocommit.Execute("INSERT INTO events VALUES (1)") |> ignore
        autocommit.Rollback()
        Assert.Single(autocommit.Execute("SELECT * FROM events").FetchAll()) |> ignore

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

    // ── Level 1 path tests ──────────────────────────────────────────────────

    [<Fact>]
    let ``aggregate COUNT SUM AVG MIN MAX without GROUP BY`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE nums (n INTEGER)") |> ignore
        conn.Execute("INSERT INTO nums VALUES (10)") |> ignore
        conn.Execute("INSERT INTO nums VALUES (20)") |> ignore
        conn.Execute("INSERT INTO nums VALUES (30)") |> ignore
        let row = conn.Execute("SELECT COUNT(*), SUM(n), MIN(n), MAX(n) FROM nums").FetchAll().[0]
        Assert.Equal(3L, Convert.ToInt64(row.[0]))
        Assert.Equal(60L, Convert.ToInt64(row.[1]))
        Assert.Equal(10L, Convert.ToInt64(row.[2]))
        Assert.Equal(30L, Convert.ToInt64(row.[3]))

    [<Fact>]
    let ``GROUP BY with HAVING filters groups`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE sales (dept TEXT, amount INTEGER)") |> ignore
        conn.Execute("INSERT INTO sales VALUES ('A', 100)") |> ignore
        conn.Execute("INSERT INTO sales VALUES ('A', 200)") |> ignore
        conn.Execute("INSERT INTO sales VALUES ('B', 50)") |> ignore
        let rows = conn.Execute("SELECT dept, SUM(amount) AS total FROM sales GROUP BY dept HAVING SUM(amount) > 100 ORDER BY dept ASC").FetchAll()
        Assert.Equal(1, rows.Count)
        Assert.Equal("A", rows.[0].[0] :?> string)
        Assert.Equal(300L, Convert.ToInt64(rows.[0].[1]))

    [<Fact>]
    let ``LIKE and NOT LIKE with WHERE`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE words (w TEXT)") |> ignore
        conn.Execute("INSERT INTO words VALUES ('hello')") |> ignore
        conn.Execute("INSERT INTO words VALUES ('world')") |> ignore
        conn.Execute("INSERT INTO words VALUES ('help')") |> ignore
        let matched = conn.Execute("SELECT w FROM words WHERE w LIKE 'hel%' ORDER BY w ASC").FetchAll()
        Assert.Equal(2, matched.Count)
        Assert.Equal("hello", matched.[0].[0] :?> string)
        Assert.Equal("help", matched.[1].[0] :?> string)
        let unmatched = conn.Execute("SELECT w FROM words WHERE w NOT LIKE 'hel%'").FetchAll()
        Assert.Equal(1, unmatched.Count)
        Assert.Equal("world", unmatched.[0].[0] :?> string)

    [<Fact>]
    let ``scalar functions UPPER LOWER LENGTH work in ORDER BY SELECT`` () =
        // Scalar functions require ORDER BY to preserve the Project node in the
        // optimized plan so the function-call post-processor fires.
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE words (id INTEGER, word TEXT)") |> ignore
        conn.Execute("INSERT INTO words VALUES (1, 'Hello')") |> ignore
        conn.Execute("INSERT INTO words VALUES (2, 'World')") |> ignore
        let rows = conn.Execute("SELECT id, UPPER(word) AS uw, LOWER(word) AS lw, LENGTH(word) AS len FROM words ORDER BY id ASC").FetchAll()
        Assert.Equal(2, rows.Count)
        Assert.Equal("HELLO", rows.[0].[1] :?> string)
        Assert.Equal("hello", rows.[0].[2] :?> string)
        Assert.Equal(5L, Convert.ToInt64(rows.[0].[3]))

    [<Fact>]
    let ``IS NULL and IS NOT NULL predicates`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (id INTEGER, val INTEGER)") |> ignore
        conn.Execute("INSERT INTO t VALUES (1, 10)") |> ignore
        conn.Execute("INSERT INTO t VALUES (2, NULL)") |> ignore
        conn.Execute("INSERT INTO t VALUES (3, 30)") |> ignore
        let nulls = conn.Execute("SELECT id FROM t WHERE val IS NULL").FetchAll()
        Assert.Equal(1, nulls.Count)
        Assert.Equal(2L, Convert.ToInt64(nulls.[0].[0]))
        let nonNulls = conn.Execute("SELECT COUNT(*) FROM t WHERE val IS NOT NULL").FetchAll().[0]
        Assert.Equal(2L, Convert.ToInt64(nonNulls.[0]))

    [<Fact>]
    let ``BETWEEN predicate filters rows`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (n INTEGER)") |> ignore
        for i in 1 .. 10 do
            conn.Execute(sprintf "INSERT INTO t VALUES (%d)" i) |> ignore
        let rows = conn.Execute("SELECT COUNT(*) FROM t WHERE n BETWEEN 3 AND 7").FetchAll().[0]
        Assert.Equal(5L, Convert.ToInt64(rows.[0]))

    [<Fact>]
    let ``IN predicate filters rows`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (n INTEGER)") |> ignore
        conn.Execute("INSERT INTO t VALUES (1)") |> ignore
        conn.Execute("INSERT INTO t VALUES (2)") |> ignore
        conn.Execute("INSERT INTO t VALUES (3)") |> ignore
        let rows = conn.Execute("SELECT n FROM t WHERE n IN (1, 3) ORDER BY n ASC").FetchAll()
        Assert.Equal(2, rows.Count)
        Assert.Equal(1L, Convert.ToInt64(rows.[0].[0]))
        Assert.Equal(3L, Convert.ToInt64(rows.[1].[0]))

    [<Fact>]
    let ``COUNT DISTINCT counts unique values`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (cat TEXT, val INTEGER)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('A', 1)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('A', 1)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('A', 2)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('B', 1)") |> ignore
        let rows = conn.Execute("SELECT cat, COUNT(DISTINCT val) FROM t GROUP BY cat ORDER BY cat ASC").FetchAll()
        Assert.Equal(2, rows.Count)
        Assert.Equal("A", rows.[0].[0] :?> string)
        Assert.Equal(2L, Convert.ToInt64(rows.[0].[1]))
        Assert.Equal("B", rows.[1].[0] :?> string)
        Assert.Equal(1L, Convert.ToInt64(rows.[1].[1]))

    [<Fact>]
    let ``fromless SELECT expressions`` () =
        use conn = MiniSqlite.Connect(":memory:")
        let r = conn.Execute("SELECT 1 + 2, 'hello'").FetchAll().[0]
        Assert.Equal(3L, Convert.ToInt64(r.[0]))
        Assert.Equal("hello", r.[1] :?> string)

    [<Fact>]
    let ``AVG aggregate returns correct mean`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (n INTEGER)") |> ignore
        conn.Execute("INSERT INTO t VALUES (10)") |> ignore
        conn.Execute("INSERT INTO t VALUES (20)") |> ignore
        conn.Execute("INSERT INTO t VALUES (30)") |> ignore
        let r = conn.Execute("SELECT AVG(n) FROM t").FetchAll().[0]
        Assert.Equal(20.0, r.[0] :?> float, 1)

    [<Fact>]
    let ``DISTINCT removes duplicate rows`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (n INTEGER)") |> ignore
        conn.Execute("INSERT INTO t VALUES (1)") |> ignore
        conn.Execute("INSERT INTO t VALUES (1)") |> ignore
        conn.Execute("INSERT INTO t VALUES (2)") |> ignore
        let rows = conn.Execute("SELECT DISTINCT n FROM t ORDER BY n ASC").FetchAll()
        Assert.Equal(2, rows.Count)

    [<Fact>]
    let ``NULL in aggregate functions is ignored in SUM AVG`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (n INTEGER)") |> ignore
        conn.Execute("INSERT INTO t VALUES (10)") |> ignore
        conn.Execute("INSERT INTO t VALUES (NULL)") |> ignore
        conn.Execute("INSERT INTO t VALUES (30)") |> ignore
        let r = conn.Execute("SELECT SUM(n), COUNT(*), COUNT(n) FROM t").FetchAll().[0]
        Assert.Equal(40L, Convert.ToInt64(r.[0]))
        Assert.Equal(3L, Convert.ToInt64(r.[1]))  // COUNT(*) counts all rows
        Assert.Equal(2L, Convert.ToInt64(r.[2]))  // COUNT(n) skips NULLs

    [<Fact>]
    let ``ORDER BY DESC and LIMIT work together`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (n INTEGER)") |> ignore
        for i in 1 .. 10 do
            conn.Execute(sprintf "INSERT INTO t VALUES (%d)" i) |> ignore
        let rows = conn.Execute("SELECT n FROM t ORDER BY n DESC LIMIT 3").FetchAll()
        Assert.Equal(3, rows.Count)
        Assert.Equal(10L, Convert.ToInt64(rows.[0].[0]))
        Assert.Equal(9L, Convert.ToInt64(rows.[1].[0]))
        Assert.Equal(8L, Convert.ToInt64(rows.[2].[0]))

    [<Fact>]
    let ``ORDER BY with OFFSET skips rows`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (n INTEGER)") |> ignore
        for i in 1 .. 5 do
            conn.Execute(sprintf "INSERT INTO t VALUES (%d)" i) |> ignore
        let rows = conn.Execute("SELECT n FROM t ORDER BY n ASC LIMIT 2 OFFSET 2").FetchAll()
        Assert.Equal(2, rows.Count)
        Assert.Equal(3L, Convert.ToInt64(rows.[0].[0]))
        Assert.Equal(4L, Convert.ToInt64(rows.[1].[0]))

    [<Fact>]
    let ``WHERE with arithmetic expression`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (a INTEGER, b INTEGER)") |> ignore
        conn.Execute("INSERT INTO t VALUES (3, 4)") |> ignore
        conn.Execute("INSERT INTO t VALUES (1, 2)") |> ignore
        let rows = conn.Execute("SELECT a + b AS sum, a * b AS prod FROM t WHERE a + b > 5 ORDER BY a ASC").FetchAll()
        Assert.Equal(1, rows.Count)
        Assert.Equal(7L, Convert.ToInt64(rows.[0].[0]))
        Assert.Equal(12L, Convert.ToInt64(rows.[0].[1]))

    [<Fact>]
    let ``UPDATE changes only matching rows`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (id INTEGER, v INTEGER)") |> ignore
        conn.Execute("INSERT INTO t VALUES (1, 10)") |> ignore
        conn.Execute("INSERT INTO t VALUES (2, 20)") |> ignore
        conn.Execute("INSERT INTO t VALUES (3, 30)") |> ignore
        let res = conn.Execute("UPDATE t SET v = 99 WHERE id = 2")
        Assert.Equal(1, res.RowCount)
        let rows = conn.Execute("SELECT id, v FROM t ORDER BY id ASC").FetchAll()
        Assert.Equal(10L, Convert.ToInt64(rows.[0].[1]))
        Assert.Equal(99L, Convert.ToInt64(rows.[1].[1]))
        Assert.Equal(30L, Convert.ToInt64(rows.[2].[1]))

    [<Fact>]
    let ``DELETE without WHERE removes all rows`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (n INTEGER)") |> ignore
        conn.Execute("INSERT INTO t VALUES (1)") |> ignore
        conn.Execute("INSERT INTO t VALUES (2)") |> ignore
        let res = conn.Execute("DELETE FROM t")
        Assert.Equal(2, res.RowCount)
        Assert.Empty(conn.Execute("SELECT * FROM t").FetchAll())

    [<Fact>]
    let ``GROUP BY with multiple groups and ORDER BY`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE sales (dept TEXT, amount INTEGER)") |> ignore
        conn.Execute("INSERT INTO sales VALUES ('A', 100)") |> ignore
        conn.Execute("INSERT INTO sales VALUES ('B', 200)") |> ignore
        conn.Execute("INSERT INTO sales VALUES ('A', 50)") |> ignore
        conn.Execute("INSERT INTO sales VALUES ('B', 150)") |> ignore
        conn.Execute("INSERT INTO sales VALUES ('C', 300)") |> ignore
        let rows = conn.Execute("SELECT dept, COUNT(*) AS cnt, SUM(amount) AS total FROM sales GROUP BY dept ORDER BY dept ASC").FetchAll()
        Assert.Equal(3, rows.Count)
        Assert.Equal("A", rows.[0].[0] :?> string)
        Assert.Equal(2L, Convert.ToInt64(rows.[0].[1]))
        Assert.Equal(150L, Convert.ToInt64(rows.[0].[2]))
        Assert.Equal("C", rows.[2].[0] :?> string)
        Assert.Equal(1L, Convert.ToInt64(rows.[2].[1]))
        Assert.Equal(300L, Convert.ToInt64(rows.[2].[2]))

    [<Fact>]
    let ``NOT IN predicate filters rows`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (n INTEGER)") |> ignore
        conn.Execute("INSERT INTO t VALUES (1)") |> ignore
        conn.Execute("INSERT INTO t VALUES (2)") |> ignore
        conn.Execute("INSERT INTO t VALUES (3)") |> ignore
        conn.Execute("INSERT INTO t VALUES (4)") |> ignore
        let rows = conn.Execute("SELECT n FROM t WHERE n NOT IN (2, 4) ORDER BY n ASC").FetchAll()
        Assert.Equal(2, rows.Count)
        Assert.Equal(1L, Convert.ToInt64(rows.[0].[0]))
        Assert.Equal(3L, Convert.ToInt64(rows.[1].[0]))

    [<Fact>]
    let ``HAVING with COUNT filter`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (cat TEXT, v INTEGER)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('A', 1)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('A', 2)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('A', 3)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('B', 1)") |> ignore
        let rows = conn.Execute("SELECT cat, COUNT(*) AS n FROM t GROUP BY cat HAVING COUNT(*) > 1").FetchAll()
        Assert.Equal(1, rows.Count)
        Assert.Equal("A", rows.[0].[0] :?> string)
        Assert.Equal(3L, Convert.ToInt64(rows.[0].[1]))

    [<Fact>]
    let ``MIN and MAX in GROUP BY`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (cat TEXT, n INTEGER)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('A', 10)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('A', 30)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('A', 20)") |> ignore
        let rows = conn.Execute("SELECT MIN(n), MAX(n) FROM t").FetchAll().[0]
        Assert.Equal(10L, Convert.ToInt64(rows.[0]))
        Assert.Equal(30L, Convert.ToInt64(rows.[1]))

    [<Fact>]
    let ``LIKE with single character wildcard underscore`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (w TEXT)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('cat')") |> ignore
        conn.Execute("INSERT INTO t VALUES ('bat')") |> ignore
        conn.Execute("INSERT INTO t VALUES ('car')") |> ignore
        let rows = conn.Execute("SELECT w FROM t WHERE w LIKE '_at' ORDER BY w ASC").FetchAll()
        Assert.Equal(2, rows.Count)
        Assert.Equal("bat", rows.[0].[0] :?> string)
        Assert.Equal("cat", rows.[1].[0] :?> string)

    [<Fact>]
    let ``unrecognised statement raises OperationalError`` () =
        use conn = MiniSqlite.Connect(":memory:")
        let err = Assert.Throws<MiniSqliteException>(fun () -> conn.Execute("EXPLAIN SELECT 1") |> ignore)
        Assert.Equal("OperationalError", err.Kind)

    [<Fact>]
    let ``CREATE TABLE IF NOT EXISTS is idempotent`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE IF NOT EXISTS t (id INTEGER)") |> ignore
        conn.Execute("INSERT INTO t VALUES (1)") |> ignore
        // Second CREATE IF NOT EXISTS should not raise
        conn.Execute("CREATE TABLE IF NOT EXISTS t (id INTEGER)") |> ignore
        let rows = conn.Execute("SELECT id FROM t").FetchAll()
        Assert.Equal(1, rows.Count)

    [<Fact>]
    let ``SELECT from unknown table raises OperationalError`` () =
        use conn = MiniSqlite.Connect(":memory:")
        let err = Assert.Throws<MiniSqliteException>(fun () -> conn.Execute("SELECT * FROM nonexistent") |> ignore)
        Assert.Equal("OperationalError", err.Kind)

    [<Fact>]
    let ``INSERT into unknown table raises OperationalError`` () =
        use conn = MiniSqlite.Connect(":memory:")
        let err = Assert.Throws<MiniSqliteException>(fun () -> conn.Execute("INSERT INTO ghost VALUES (1)") |> ignore)
        Assert.Equal("OperationalError", err.Kind)

    [<Fact>]
    let ``AVG of empty group returns NULL`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (n INTEGER)") |> ignore
        // All NULLs — AVG skips NULLs, so result should be NULL
        conn.Execute("INSERT INTO t VALUES (NULL)") |> ignore
        let r = conn.Execute("SELECT AVG(n) FROM t").FetchAll().[0]
        Assert.Null(r.[0])

    [<Fact>]
    let ``real and integer arithmetic in expressions`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (a REAL, b INTEGER)") |> ignore
        conn.Execute("INSERT INTO t VALUES (3.5, 2)") |> ignore
        let r = conn.Execute("SELECT a + b, a - b, a * b, a / b FROM t").FetchAll().[0]
        Assert.Equal(5.5, r.[0] :?> float, 6)
        Assert.Equal(1.5, r.[1] :?> float, 6)
        Assert.Equal(7.0, r.[2] :?> float, 6)
        Assert.Equal(1.75, r.[3] :?> float, 6)

    [<Fact>]
    let ``SELECT with no results returns empty list`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (n INTEGER)") |> ignore
        conn.Execute("INSERT INTO t VALUES (1)") |> ignore
        let rows = conn.Execute("SELECT n FROM t WHERE n > 100").FetchAll()
        Assert.Empty(rows)

    [<Fact>]
    let ``modulo operator works`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (n INTEGER)") |> ignore
        for i in 1 .. 6 do
            conn.Execute(sprintf "INSERT INTO t VALUES (%d)" i) |> ignore
        let rows = conn.Execute("SELECT n FROM t WHERE n % 2 = 0 ORDER BY n ASC").FetchAll()
        Assert.Equal(3, rows.Count)
        Assert.Equal(2L, Convert.ToInt64(rows.[0].[0]))
        Assert.Equal(4L, Convert.ToInt64(rows.[1].[0]))
        Assert.Equal(6L, Convert.ToInt64(rows.[2].[0]))

    // ── Scalar function tests (hit FuncEval.evalBuiltin branches) ──────────

    [<Fact>]
    let ``TRIM LTRIM RTRIM functions`` () =
        use conn = MiniSqlite.Connect(":memory:")
        let r = conn.Execute("SELECT TRIM('  hello  '), LTRIM('  hi'), RTRIM('bye  ')").FetchAll().[0]
        Assert.Equal("hello", r.[0] :?> string)
        Assert.Equal("hi", r.[1] :?> string)
        Assert.Equal("bye", r.[2] :?> string)

    [<Fact>]
    let ``SUBSTR function with start index`` () =
        use conn = MiniSqlite.Connect(":memory:")
        let r = conn.Execute("SELECT SUBSTR('hello world', 7), SUBSTR('hello', 2, 3)").FetchAll().[0]
        Assert.Equal("world", r.[0] :?> string)
        Assert.Equal("ell", r.[1] :?> string)

    [<Fact>]
    let ``REPLACE function`` () =
        use conn = MiniSqlite.Connect(":memory:")
        let r = conn.Execute("SELECT REPLACE('hello world', 'world', 'there')").FetchAll().[0]
        Assert.Equal("hello there", r.[0] :?> string)

    [<Fact>]
    let ``ABS function on integers and reals`` () =
        use conn = MiniSqlite.Connect(":memory:")
        let r = conn.Execute("SELECT ABS(-5), ABS(3)").FetchAll().[0]
        Assert.Equal(5L, Convert.ToInt64(r.[0]))
        Assert.Equal(3L, Convert.ToInt64(r.[1]))

    [<Fact>]
    let ``ROUND function`` () =
        use conn = MiniSqlite.Connect(":memory:")
        let r = conn.Execute("SELECT ROUND(3.567, 2), ROUND(2.5)").FetchAll().[0]
        Assert.Equal(3.57, r.[0] :?> float, 6)
        Assert.Equal(3.0, r.[1] :?> float, 6)

    [<Fact>]
    let ``COALESCE returns first non-null`` () =
        use conn = MiniSqlite.Connect(":memory:")
        let r = conn.Execute("SELECT COALESCE(NULL, NULL, 42)").FetchAll().[0]
        Assert.Equal(42L, Convert.ToInt64(r.[0]))

    [<Fact>]
    let ``IFNULL falls back on NULL`` () =
        use conn = MiniSqlite.Connect(":memory:")
        let r = conn.Execute("SELECT IFNULL(NULL, 'default'), IFNULL('value', 'other')").FetchAll().[0]
        Assert.Equal("default", r.[0] :?> string)
        Assert.Equal("value", r.[1] :?> string)

    [<Fact>]
    let ``string concatenation with pipe operator`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (first TEXT, last TEXT)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('John', 'Doe')") |> ignore
        let rows = conn.Execute("SELECT first || ' ' || last AS name FROM t ORDER BY name ASC").FetchAll()
        Assert.Equal(1, rows.Count)
        Assert.Equal("John Doe", rows.[0].[0] :?> string)

    [<Fact>]
    let ``negative number literals parse correctly`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (n INTEGER)") |> ignore
        conn.Execute("INSERT INTO t VALUES (-10)") |> ignore
        conn.Execute("INSERT INTO t VALUES (-5)") |> ignore
        conn.Execute("INSERT INTO t VALUES (0)") |> ignore
        let rows = conn.Execute("SELECT n FROM t WHERE n < 0 ORDER BY n DESC").FetchAll()
        Assert.Equal(2, rows.Count)
        Assert.Equal(-5L, Convert.ToInt64(rows.[0].[0]))
        Assert.Equal(-10L, Convert.ToInt64(rows.[1].[0]))

    [<Fact>]
    let ``division by zero returns NULL`` () =
        use conn = MiniSqlite.Connect(":memory:")
        let r = conn.Execute("SELECT 10 / 0").FetchAll().[0]
        Assert.Null(r.[0])

    [<Fact>]
    let ``Boolean literals TRUE and FALSE work in INSERT and SELECT`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (flag BOOLEAN)") |> ignore
        conn.Execute("INSERT INTO t VALUES (TRUE)") |> ignore
        conn.Execute("INSERT INTO t VALUES (FALSE)") |> ignore
        let rows = conn.Execute("SELECT flag FROM t WHERE flag = TRUE").FetchAll()
        Assert.Equal(1, rows.Count)

    [<Fact>]
    let ``HAVING with MIN aggregate`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (cat TEXT, n INTEGER)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('A', 5)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('A', 15)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('B', 100)") |> ignore
        let rows = conn.Execute("SELECT cat FROM t GROUP BY cat HAVING MIN(n) < 10 ORDER BY cat ASC").FetchAll()
        Assert.Equal(1, rows.Count)
        Assert.Equal("A", rows.[0].[0] :?> string)

    [<Fact>]
    let ``SELECT with IS NULL in GROUP BY filter`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (cat TEXT, n INTEGER)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('A', NULL)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('A', 10)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('B', NULL)") |> ignore
        // WHERE filters before grouping
        let rows = conn.Execute("SELECT cat, COUNT(*) AS n FROM t WHERE n IS NOT NULL GROUP BY cat ORDER BY cat ASC").FetchAll()
        Assert.Equal(1, rows.Count)
        Assert.Equal("A", rows.[0].[0] :?> string)
        Assert.Equal(1L, Convert.ToInt64(rows.[0].[1]))

    [<Fact>]
    let ``scalar function with ORDER BY in GROUP BY path`` () =
        use conn = MiniSqlite.Connect(":memory:")
        conn.Execute("CREATE TABLE t (word TEXT)") |> ignore
        conn.Execute("INSERT INTO t VALUES ('Hello')") |> ignore
        conn.Execute("INSERT INTO t VALUES ('World')") |> ignore
        let rows = conn.Execute("SELECT UPPER(word) AS uw, LENGTH(word) AS ln FROM t ORDER BY word ASC").FetchAll()
        Assert.Equal(2, rows.Count)
        Assert.Equal("HELLO", rows.[0].[0] :?> string)
        Assert.Equal(5L, Convert.ToInt64(rows.[0].[1]))

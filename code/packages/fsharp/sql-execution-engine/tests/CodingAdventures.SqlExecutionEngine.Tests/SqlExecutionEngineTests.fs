namespace CodingAdventures.SqlExecutionEngine.Tests

open System
open System.Collections.Generic
open Xunit
open CodingAdventures.SqlExecutionEngine.FSharp

module SqlExecutionEngineTests =
    let private row (pairs: (string * obj) list) =
        let values = Dictionary<string, obj>(StringComparer.Ordinal)
        for key, value in pairs do
            values[key] <- value
        values :> IReadOnlyDictionary<string, obj>

    let private dataSource () =
        InMemoryDataSource()
            .AddTable(
                "employees",
                [ "id"; "name"; "dept"; "salary"; "active" ],
                [ row [ "id", box 1L; "name", box "Alice"; "dept", box "Engineering"; "salary", box 95000L; "active", box true ]
                  row [ "id", box 2L; "name", box "Bob"; "dept", box "Marketing"; "salary", box 72000L; "active", box true ]
                  row [ "id", box 3L; "name", box "Carol"; "dept", box "Engineering"; "salary", box 88000L; "active", box false ]
                  row [ "id", box 4L; "name", box "Dave"; "dept", null; "salary", box 60000L; "active", box true ]
                  row [ "id", box 5L; "name", box "Eve"; "dept", box "HR"; "salary", box 70000L; "active", box false ] ])
            .AddTable(
                "departments",
                [ "dept"; "budget" ],
                [ row [ "dept", box "Engineering"; "budget", box 500000L ]
                  row [ "dept", box "Marketing"; "budget", box 200000L ]
                  row [ "dept", box "HR"; "budget", box 150000L ] ])
        :> IDataSource

    [<Fact>]
    let ``scans in-memory tables`` () =
        let source = dataSource ()
        Assert.Equal<string array>([| "id"; "name"; "dept"; "salary"; "active" |], source.Schema "employees")
        Assert.Equal(5, (source.Scan "employees").Length)
        Assert.Throws<SqlExecutionException>(fun () -> source.Schema "missing" |> ignore) |> ignore

    [<Fact>]
    let ``selects and filters rows`` () =
        let result =
            SqlExecutionEngine.execute
                "SELECT name, salary FROM employees WHERE active = true AND salary >= 70000 ORDER BY salary DESC"
                (dataSource ())

        Assert.Equal<string array>([| "name"; "salary" |], result.Columns)
        Assert.Equal<obj array>([| box "Alice"; box 95000L |], result.Rows[0])
        Assert.Equal<obj array>([| box "Bob"; box 72000L |], result.Rows[1])

    [<Fact>]
    let ``supports null predicates and like`` () =
        let nullResult = SqlExecutionEngine.execute "SELECT name FROM employees WHERE dept IS NULL" (dataSource ())
        Assert.Equal<obj array>([| box "Dave" |], nullResult.Rows[0])

        let likeResult = SqlExecutionEngine.execute "SELECT name FROM employees WHERE name LIKE 'A%'" (dataSource ())
        Assert.Equal<obj array>([| box "Alice" |], likeResult.Rows[0])

    [<Fact>]
    let ``supports joins`` () =
        let result =
            SqlExecutionEngine.execute
                "SELECT e.name, d.budget FROM employees AS e INNER JOIN departments AS d ON e.dept = d.dept ORDER BY e.id"
                (dataSource ())

        Assert.Equal<string array>([| "name"; "budget" |], result.Columns)
        Assert.Equal(4, result.Rows.Length)
        Assert.Equal<obj array>([| box "Alice"; box 500000L |], result.Rows[0])
        Assert.Equal<obj array>([| box "Eve"; box 150000L |], result.Rows[3])

    [<Fact>]
    let ``supports grouping aggregates and having`` () =
        let result =
            SqlExecutionEngine.execute
                "SELECT dept, COUNT(*) AS cnt, SUM(salary) AS total FROM employees WHERE dept IS NOT NULL GROUP BY dept HAVING COUNT(*) >= 1 ORDER BY dept"
                (dataSource ())

        Assert.Equal<string array>([| "dept"; "cnt"; "total" |], result.Columns)
        Assert.Equal<obj array>([| box "Engineering"; box 2; box 183000.0 |], result.Rows[0])
        Assert.Equal<obj array>([| box "HR"; box 1; box 70000.0 |], result.Rows[1])
        Assert.Equal<obj array>([| box "Marketing"; box 1; box 72000.0 |], result.Rows[2])

    [<Fact>]
    let ``supports distinct limit and offset`` () =
        let result =
            SqlExecutionEngine.execute
                "SELECT DISTINCT dept FROM employees WHERE dept IS NOT NULL ORDER BY dept LIMIT 2 OFFSET 1"
                (dataSource ())

        Assert.Equal<string array>([| "dept" |], result.Columns)
        Assert.Equal<obj array>([| box "HR" |], result.Rows[0])
        Assert.Equal<obj array>([| box "Marketing" |], result.Rows[1])

    [<Fact>]
    let ``reports errors through tryExecute`` () =
        let result = SqlExecutionEngine.tryExecute "SELECT * FROM ghosts" (dataSource ())

        Assert.False(result.Ok)
        Assert.True(result.Error.Value.Contains("table not found: ghosts"))

    [<Fact>]
    let ``select star uses bare columns`` () =
        let result = SqlExecutionEngine.execute "SELECT * FROM employees WHERE id = 1" (dataSource ())

        Assert.Equal<string array>([| "active"; "dept"; "id"; "name"; "salary" |], result.Columns)
        Assert.Equal<obj array>([| box true; box "Engineering"; box 1L; box "Alice"; box 95000L |], result.Rows[0])

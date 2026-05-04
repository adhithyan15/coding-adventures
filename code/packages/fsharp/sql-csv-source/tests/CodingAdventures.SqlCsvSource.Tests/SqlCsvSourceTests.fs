namespace CodingAdventures.SqlCsvSource.Tests

open System
open System.IO
open Xunit
open CodingAdventures.SqlCsvSource.FSharp
open CodingAdventures.SqlExecutionEngine.FSharp

module SqlCsvSourceTests =
    let private fixtures =
        Path.Combine(AppContext.BaseDirectory, "fixtures")

    [<Fact>]
    let ``exposes schema in header order`` () =
        let source = CsvDataSource(fixtures)

        Assert.Equal<string array>([| "id"; "name"; "dept_id"; "salary"; "active" |], source.Schema("employees"))
        Assert.Equal<string array>([| "id"; "name"; "budget" |], source.Schema("departments"))

        let moduleSource = SqlCsvSource.csvDataSource fixtures
        Assert.Equal<string array>([| "id"; "name"; "budget" |], moduleSource.Schema("departments"))

        let directorySource = CsvDataSource(DirectoryInfo(fixtures))
        Assert.Equal<string array>([| "id"; "name"; "dept_id"; "salary"; "active" |], directorySource.Schema("employees"))

    [<Fact>]
    let ``scans rows with coerced values`` () =
        let rows = CsvDataSource(fixtures).Scan("employees")

        Assert.Equal(4, rows.Length)
        Assert.Equal<obj>(box 1L, rows[0].["id"])
        Assert.Equal<obj>(box "Alice", rows[0].["name"])
        Assert.Equal<obj>(box 90000L, rows[0].["salary"])
        Assert.Equal<obj>(box true, rows[0].["active"])
        Assert.Null(rows[3].["dept_id"])

    [<Fact>]
    let ``executes selects against csv files`` () =
        let result =
            SqlCsvSource.executeCsv
                "SELECT name, salary FROM employees WHERE active = true AND salary > 70000 ORDER BY salary DESC"
                fixtures

        Assert.Equal<string array>([| "name"; "salary" |], result.Columns)
        Assert.Equal<obj array>([| box "Alice"; box 90000L |], result.Rows[0])
        Assert.Equal<obj array>([| box "Bob"; box 75000L |], result.Rows[1])

    [<Fact>]
    let ``supports null predicates`` () =
        let result = SqlCsvSource.executeCsv "SELECT name FROM employees WHERE dept_id IS NULL" fixtures

        Assert.Equal<obj array>([| box "Dave" |], result.Rows[0])

    [<Fact>]
    let ``supports joins across csv files`` () =
        let result =
            SqlCsvSource.executeCsv
                """
                SELECT e.name AS emp_name, d.name AS dept_name
                FROM employees AS e
                INNER JOIN departments AS d ON e.dept_id = d.id
                ORDER BY e.id
                """
                fixtures

        Assert.Equal<string array>([| "emp_name"; "dept_name" |], result.Columns)
        Assert.Equal<obj array>([| box "Alice"; box "Engineering" |], result.Rows[0])
        Assert.Equal<obj array>([| box "Bob"; box "Marketing" |], result.Rows[1])
        Assert.Equal<obj array>([| box "Carol"; box "Engineering" |], result.Rows[2])

    [<Fact>]
    let ``supports grouping aggregates limit and offset`` () =
        let result =
            SqlCsvSource.executeCsv
                "SELECT dept_id, COUNT(*) AS cnt FROM employees WHERE dept_id IS NOT NULL GROUP BY dept_id ORDER BY dept_id LIMIT 2"
                fixtures

        Assert.Equal<string array>([| "dept_id"; "cnt" |], result.Columns)
        Assert.Equal<obj array>([| box 1L; box 2 |], result.Rows[0])
        Assert.Equal<obj array>([| box 2L; box 1 |], result.Rows[1])

    [<Fact>]
    let ``reports missing tables through engine errors`` () =
        let ex =
            Assert.Throws<SqlExecutionException>(fun () ->
                SqlCsvSource.executeCsv "SELECT * FROM no_such_table" fixtures |> ignore)

        Assert.Contains("table not found: no_such_table", ex.Message)

        let result = SqlCsvSource.tryExecuteCsv "SELECT * FROM no_such_table" fixtures
        Assert.False(result.Ok)
        Assert.True(result.Error.IsSome)

        let missingDirectory = Path.Combine(fixtures, "missing")
        let directoryEx =
            Assert.Throws<SqlExecutionException>(fun () ->
                CsvDataSource(missingDirectory).Schema("employees") |> ignore)

        Assert.Contains("table not found: employees", directoryEx.Message)

    [<Fact>]
    let ``handles quoted headers and header parse errors`` () =
        let directory =
            Path.Combine(Path.GetTempPath(), "sql-csv-source-fsharp-" + Guid.NewGuid().ToString("N"))

        Directory.CreateDirectory(directory) |> ignore

        try
            File.WriteAllText(
                Path.Combine(directory, "people.csv"),
                "\"display,name\",\"said \"\"hi\"\"\"\nAlice,yes\n")

            let source = CsvDataSource(directory)
            Assert.Equal<string array>([| "display,name"; "said \"hi\"" |], source.Schema("people"))

            let rows = source.Scan("people")
            Assert.Equal<obj>(box "Alice", rows[0].["display,name"])
            Assert.Equal<obj>(box "yes", rows[0].["said \"hi\""])

            File.WriteAllText(Path.Combine(directory, "broken.csv"), "\"unterminated,name\nAlice,yes\n")
            let ex =
                Assert.Throws<SqlExecutionException>(fun () ->
                    source.Schema("broken") |> ignore)

            Assert.Contains("parsing CSV header for table broken", ex.Message)
        finally
            Directory.Delete(directory, true)

    [<Fact>]
    let ``coerces scalar values`` () =
        Assert.Null(CsvDataSource.Coerce(""))
        Assert.Equal<obj>(box true, CsvDataSource.Coerce("TRUE"))
        Assert.Equal<obj>(box false, CsvDataSource.Coerce("false"))
        Assert.Equal<obj>(box 42L, CsvDataSource.Coerce("42"))
        Assert.Equal<obj>(box -5L, CsvDataSource.Coerce("-5"))
        Assert.Equal<obj>(box 3.14, CsvDataSource.Coerce("3.14"))
        Assert.Equal<obj>(box "123abc", CsvDataSource.Coerce("123abc"))
        Assert.IsType<string>(CsvDataSource.Coerce("hello")) |> ignore

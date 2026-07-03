#nowarn "3261"
#nowarn "3264"
/// Conformance test suite for CodingAdventures.MiniSqlite.FSharp.
///
/// Each test reads a JSON fixture from
///   code/specs/mini-sqlite-conformance/fixtures/<id>.json
/// and executes every step through the public Connection/Cursor API,
/// asserting that the observed output matches the fixture expectations.
///
/// Supported fixture operations:
///   execute        — conn.Execute(sql, params?) — expects no error
///   executemany    — conn.ExecuteMany(sql, param_seq)
///   query          — conn.Execute then FetchAll(), compare columns + rows
///   expect_error   — conn.Execute must raise MiniSqliteException(error_type)
///   fetchone_test  — two consecutive fetchone() calls
///   fetchmany_test — two consecutive fetchmany(size) calls
///   fetchall_test  — full fetchall() comparison
///   fetchall_empty_test — fetchall() on empty result
///   connect_steps / connect_expect_error
///              — verify that Connect(non-memory path) raises NotSupportedError
///
/// Value comparison follows JSON semantics:
///   JSON null     → DB null (null obj)
///   JSON number   → integer or real via Convert.ToInt64 / ToDouble
///   JSON string   → string comparison
///   JSON bool     → treated as integer (true→1, false→0) for SQLite compat
///
namespace CodingAdventures.MiniSqlite.Tests

open System
open System.Collections.Generic
open System.IO
open System.Text.Json
open Xunit
open CodingAdventures.MiniSqlite.FSharp

// ─── Fixture JSON helpers ─────────────────────────────────────────────────────

module private Fixture =

    /// Locate the fixtures directory.  During test execution the fixtures are
    /// copied into the output directory alongside the DLL via the .fsproj
    /// LinkBase="fixtures" glob, so we look for them relative to the
    /// executing assembly.
    let private fixturesDir =
        let asm = Reflection.Assembly.GetExecutingAssembly().Location
        let dir = Path.GetDirectoryName(asm)
        if dir = null then failwith "Could not determine assembly directory"
        Path.Combine(dir, "fixtures")

    let load (id: string) : JsonElement =
        let path = Path.Combine(fixturesDir, $"{id}.json")
        if not (File.Exists path) then
            failwith $"Fixture not found: {path}"
        JsonDocument.Parse(File.ReadAllText path).RootElement

    /// Convert a JsonElement value to the .NET object we store / compare.
    let private jsonToObj (el: JsonElement) : obj =
        match el.ValueKind with
        | JsonValueKind.Null     -> box (null: string)
        | JsonValueKind.True     -> box 1L  // SQLite: booleans are integers
        | JsonValueKind.False    -> box 0L
        | JsonValueKind.String   -> box (el.GetString())
        | JsonValueKind.Number   ->
            // Prefer integer if value has no fractional part.
            if el.TryGetInt64(ref 0L) then box (el.GetInt64())
            else box (el.GetDouble())
        | _ -> failwith $"Unsupported JSON value kind: {el.ValueKind}"

    /// Convert a JsonElement array (one fixture row) to a plain obj list.
    let private rowOfJson (el: JsonElement) : obj list =
        [ for v in el.EnumerateArray() -> jsonToObj v ]

    /// Parse params from a step element (may be absent → empty list).
    let paramsOfStep (step: JsonElement) : obj list =
        match step.TryGetProperty "params" with
        | true, arr -> [ for v in arr.EnumerateArray() -> jsonToObj v ]
        | _ -> []

    /// Parse param_seq from an executemany step.
    let paramSeqOfStep (step: JsonElement) : IReadOnlyList<obj> list =
        match step.TryGetProperty "param_seq" with
        | true, arr ->
            [ for tupleEl in arr.EnumerateArray() ->
                ([ for v in tupleEl.EnumerateArray() -> jsonToObj v ]
                 |> List.toArray :> IReadOnlyList<obj>) ]
        | _ -> []

    let expectedColumns (step: JsonElement) : string list =
        match step.TryGetProperty "expected_columns" with
        | true, arr -> [ for v in arr.EnumerateArray() -> v.GetString() ]
        | _ -> []

    let expectedRows (step: JsonElement) : obj list list =
        match step.TryGetProperty "expected_rows" with
        | true, arr -> [ for row in arr.EnumerateArray() -> rowOfJson row ]
        | _ -> []

// ─── Assertion helpers ────────────────────────────────────────────────────────

module private Assert2 =

    /// Compare a single DB cell value against its JSON expectation.
    /// Both integer and real cells from the DB are normalised before comparison
    /// so that e.g. SQL INTEGER 5 matches JSON number 5 even when the runtime
    /// object type is int32, int64, or double.
    let private cellsEqual (expected: obj) (actual: obj) : bool =
        match expected, actual with
        | null, null -> true
        | null, _    -> false
        | _,    null -> false
        | (:? string as s1), (:? string as s2) -> s1 = s2
        | _ ->
            // Numeric comparison: try to unify as (integer, real) pair.
            let tryInt (o: obj) =
                try Some (Convert.ToInt64 o) with _ -> None
            let tryDbl (o: obj) =
                try Some (Convert.ToDouble o) with _ -> None
            match tryInt expected, tryInt actual with
            | Some ei, Some ai -> ei = ai
            | _ ->
                match tryDbl expected, tryDbl actual with
                | Some ed, Some ad -> ed = ad
                | _ -> obj.Equals(expected, actual)

    let rowsEqual (exp: obj list) (act: IReadOnlyList<obj>) : bool =
        exp.Length = act.Count &&
        List.forall2 cellsEqual exp (act |> Seq.toList)

    let assertRowsMatch
        (fixId: string)
        (stepIdx: int)
        (expCols: string list)
        (expRows: obj list list)
        (cursor: Cursor) =
        let actCols = cursor.Description |> Seq.map (fun d -> d.Name) |> Seq.toList
        Assert.Equal<string list>(expCols, actCols) |> ignore
        let actRows = cursor.FetchAll()
        Assert.Equal(expRows.Length, actRows.Count)
        for i in 0 .. expRows.Length - 1 do
            let exp = expRows.[i]
            let act = actRows.[i]
            if not (rowsEqual exp act) then
                let expStr = exp |> List.map (sprintf "%A") |> String.concat ", "
                let actStr = act |> Seq.map (sprintf "%A") |> String.concat ", "
                failwith $"[{fixId}] step {stepIdx}: row {i} mismatch.\n  expected: [{expStr}]\n  actual:   [{actStr}]"

// ─── Fixture runner ───────────────────────────────────────────────────────────

module private Runner =

    let run (fixId: string) =
        let fixture = Fixture.load fixId

        // Some fixtures use connect_steps (connection-level tests) instead of
        // the normal steps array.
        let hasConnectSteps =
            match fixture.TryGetProperty "connect_steps" with
            | true, _ -> true
            | _ -> false

        if hasConnectSteps then
            // Each step inside connect_steps describes one connection attempt.
            let steps = fixture.GetProperty("connect_steps").EnumerateArray() |> Seq.toArray
            for step in steps do
                let op = step.GetProperty("op").GetString()
                match op with
                | "connect_expect_error" ->
                    let db = step.GetProperty("database").GetString()
                    let errorType = step.GetProperty("error_type").GetString()
                    let ex = Assert.Throws<MiniSqliteException>(fun () ->
                        MiniSqlite.Connect(db) |> ignore)
                    Assert.Equal(errorType, ex.Kind)
                | other ->
                    failwith $"[{fixId}] Unknown connect_step op: {other}"
        else
            use conn = MiniSqlite.Connect(":memory:")
            let steps = fixture.GetProperty("steps").EnumerateArray() |> Seq.toArray
            for idx, step in steps |> Array.indexed do
                let op = step.GetProperty("op").GetString()
                match op with
                | "commit" ->
                    conn.Commit()

                | "rollback" ->
                    conn.Rollback()

                | "execute" ->
                    let sql    = step.GetProperty("sql").GetString()
                    let parms  = Fixture.paramsOfStep step
                    let paramArr = parms |> List.toArray :> IReadOnlyList<obj>
                    conn.Execute(sql, paramArr) |> ignore

                | "executemany" ->
                    let sql      = step.GetProperty("sql").GetString()
                    let paramSeq = Fixture.paramSeqOfStep step
                    conn.ExecuteMany(sql, paramSeq) |> ignore

                | "query" ->
                    let sql      = step.GetProperty("sql").GetString()
                    let parms    = Fixture.paramsOfStep step
                    let paramArr = parms |> List.toArray :> IReadOnlyList<obj>
                    let expCols  = Fixture.expectedColumns step
                    let expRows  = Fixture.expectedRows step
                    let cursor   = conn.Execute(sql, paramArr)
                    Assert2.assertRowsMatch fixId idx expCols expRows cursor

                | "expect_error" ->
                    let sql       = step.GetProperty("sql").GetString()
                    let parms     = Fixture.paramsOfStep step
                    let paramArr  = parms |> List.toArray :> IReadOnlyList<obj>
                    let errorType = step.GetProperty("error_type").GetString()
                    let ex = Assert.Throws<MiniSqliteException>(fun () ->
                        conn.Execute(sql, paramArr) |> ignore)
                    Assert.Equal(errorType, ex.Kind)

                | "fetchone_test" ->
                    let sql    = step.GetProperty("sql").GetString()
                    let cursor = conn.Execute(sql)
                    let first  = cursor.FetchOne()
                    let second = cursor.FetchOne()
                    // The fixture uses expected_first / expected_second keys directly.
                    let parseRow (key: string) : obj list option =
                        match step.TryGetProperty key with
                        | true, arr ->
                            Some [ for v in arr.EnumerateArray() ->
                                       match v.ValueKind with
                                       | JsonValueKind.Null   -> box (null: string)
                                       | JsonValueKind.True   -> box 1L
                                       | JsonValueKind.False  -> box 0L
                                       | JsonValueKind.String -> box (v.GetString())
                                       | JsonValueKind.Number ->
                                           if v.TryGetInt64(ref 0L) then box (v.GetInt64())
                                           else box (v.GetDouble())
                                       | _ -> box (null: string) ]
                        | _ -> None
                    match parseRow "expected_first" with
                    | Some exp ->
                        Assert.NotNull(first)
                        if not (Assert2.rowsEqual exp first) then
                            failwith $"[{fixId}] fetchone_test step {idx}: first row mismatch"
                    | None -> ()
                    match parseRow "expected_second" with
                    | Some exp ->
                        Assert.NotNull(second)
                        if not (Assert2.rowsEqual exp second) then
                            failwith $"[{fixId}] fetchone_test step {idx}: second row mismatch"
                    | None -> ()

                | "fetchmany_test" ->
                    let sql    = step.GetProperty("sql").GetString()
                    let size   = step.GetProperty("size").GetInt32()
                    let cursor = conn.Execute(sql)
                    let batch1 = cursor.FetchMany(size)
                    let batch2 = cursor.FetchMany(size)
                    let parseRows (key: string) : obj list list =
                        match step.TryGetProperty key with
                        | true, arr ->
                            [ for rowEl in arr.EnumerateArray() ->
                                [ for v in rowEl.EnumerateArray() ->
                                    match v.ValueKind with
                                    | JsonValueKind.Null   -> box (null: string)
                                    | JsonValueKind.True   -> box 1L
                                    | JsonValueKind.False  -> box 0L
                                    | JsonValueKind.String -> box (v.GetString())
                                    | JsonValueKind.Number ->
                                        if v.TryGetInt64(ref 0L) then box (v.GetInt64())
                                        else box (v.GetDouble())
                                    | _ -> box (null: string) ] ]
                        | _ -> []
                    let expBatch1 = parseRows "expected_first_batch"
                    let expBatch2 = parseRows "expected_second_batch"
                    Assert.Equal(expBatch1.Length, batch1.Count)
                    for i in 0 .. expBatch1.Length - 1 do
                        if not (Assert2.rowsEqual expBatch1.[i] batch1.[i]) then
                            failwith $"[{fixId}] fetchmany_test step {idx}: first batch row {i} mismatch"
                    Assert.Equal(expBatch2.Length, batch2.Count)
                    for i in 0 .. expBatch2.Length - 1 do
                        if not (Assert2.rowsEqual expBatch2.[i] batch2.[i]) then
                            failwith $"[{fixId}] fetchmany_test step {idx}: second batch row {i} mismatch"

                | "fetchall_test" | "fetchall_empty_test" ->
                    let sql    = step.GetProperty("sql").GetString()
                    let cursor = conn.Execute(sql)
                    let rows   = cursor.FetchAll()
                    let expRows = Fixture.expectedRows step
                    Assert.Equal(expRows.Length, rows.Count)
                    for i in 0 .. expRows.Length - 1 do
                        if not (Assert2.rowsEqual expRows.[i] rows.[i]) then
                            failwith $"[{fixId}] fetchall_test step {idx}: row {i} mismatch"

                | other ->
                    failwith $"[{fixId}] Unknown step op: {other}"

// ─── Individual test facts (one per fixture) ──────────────────────────────────

module ConformanceTests =

    [<Fact>]
    let ``fixture 01 create-select`` () =
        Runner.run "01-create-select"

    [<Fact>]
    let ``fixture 02 qmark-binding-insert`` () =
        Runner.run "02-qmark-binding-insert"

    [<Fact>]
    let ``fixture 03 projection-aliases`` () =
        Runner.run "03-projection-aliases"

    [<Fact>]
    let ``fixture 04 where-filtering`` () =
        Runner.run "04-where-filtering"

    [<Fact>]
    let ``fixture 05 order-by-limit-offset`` () =
        Runner.run "05-order-by-limit-offset"

    [<Fact>]
    let ``fixture 06 aggregates`` () =
        Runner.run "06-aggregates"

    [<Fact>]
    let ``fixture 07 update-delete`` () =
        Runner.run "07-update-delete"

    [<Fact>]
    let ``fixture 08 transaction-commit`` () =
        Runner.run "08-transaction-commit"

    [<Fact>]
    let ``fixture 09 transaction-rollback`` () =
        Runner.run "09-transaction-rollback"

    [<Fact>]
    let ``fixture 10 error-wrong-param-count`` () =
        Runner.run "10-error-wrong-param-count"

    [<Fact>]
    let ``fixture 11 error-unknown-table`` () =
        Runner.run "11-error-unknown-table"

    [<Fact>]
    let ``fixture 12 error-file-path-level0`` () =
        Runner.run "12-error-file-path-level0"

    [<Fact>]
    let ``fixture 13 drop-table`` () =
        Runner.run "13-drop-table"

    [<Fact>]
    let ``fixture 14 executemany`` () =
        Runner.run "14-executemany"

    [<Fact>]
    let ``fixture 15 fetchone-fetchmany`` () =
        Runner.run "15-fetchone-fetchmany"

    [<Fact>]
    let ``fixture 16 null-handling`` () =
        Runner.run "16-null-handling"

    [<Fact>]
    let ``fixture 17 null-aggregate-semantics`` () =
        Runner.run "17-null-aggregate-semantics"

    [<Fact>]
    let ``fixture 18 string-functions`` () =
        Runner.run "18-string-functions"

    [<Fact>]
    let ``fixture 19 math-functions`` () =
        Runner.run "19-math-functions"

    [<Fact>]
    let ``fixture 20 limit-edge-cases`` () =
        Runner.run "20-limit-edge-cases"

    [<Fact>]
    let ``fixture 21 distinct-aggregate`` () =
        Runner.run "21-distinct-aggregate"

    [<Fact>]
    let ``fixture 22 string-concat-null`` () =
        Runner.run "22-string-concat-null"

    [<Fact>]
    let ``fixture 23 null-in-order-by`` () =
        Runner.run "23-null-in-order-by"

    [<Fact>]
    let ``fixture 24 having-aggregate`` () =
        Runner.run "24-having-aggregate"

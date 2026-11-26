/// Replays the whole committed pbt-v1 corpus through the F# API, with its negative controls and the
/// F#-layer sabotage seams (VER05l sections 4 and 6).
module CodingAdventures.TestBootstrap.FSharp.Tests.CorpusTests

open System
open System.Collections.Generic
open CodingAdventures.TestBootstrap.CorpusJson
open CodingAdventures.TestBootstrap.Pbt
open CodingAdventures.TestBootstrap.FSharp
open CodingAdventures.TestBootstrap.FSharp.Tests.Support
open CodingAdventures.TestBootstrap.FSharp.Tests.FsCorpusRunner
open Xunit

let private run (cases: obj list) = bounded 120 (fun () -> evaluate cases)

// ---- corruption: same shape, different value ---------------------------------------------------

let private corruptValue (node: obj) : obj =
    match node with
    | :? bool as b -> box (not b)
    | _ ->
        let m = obj' node
        if (m.["kind"] :?> string) = "int" then
            m.["value"] <- box (string (bigint.Parse(m.["value"] :?> string) + 1I))
        else
            let hex = m.["hex"] :?> string
            m.["hex"] <- box (if hex.Length = 0 then "ff" else hex.Substring 2)
        box m

let private corruptFirst (c: Dictionary<string, obj>) (field: string) =
    let l = c.[field] :?> List<obj>
    l.[0] <- corruptValue l.[0]

let private corruptions (op: string) : (Dictionary<string, obj> -> unit) list =
    match op with
    | "gen_primitive" | "gen_bounded" | "gen_bytes" -> [ fun c -> corruptFirst c "outputs" ]
    | "shrink_int" | "shrink_refinement" | "shrink_bytes" ->
        [ (fun c -> c.["expected_minimal"] <- corruptValue c.["expected_minimal"])
          (fun c -> c.["expected_steps"] <- corruptValue c.["expected_steps"]) ]
    | "shrink_list" ->
        [ (fun c -> corruptFirst c "expected_minimal")
          (fun c -> c.["expected_steps"] <- corruptValue c.["expected_steps"]) ]
    | "replay_consistency" ->
        [ (fun c -> c.["initial_fail"] <- corruptValue c.["initial_fail"])
          (fun c -> c.["expected_minimal"] <- corruptValue c.["expected_minimal"]) ]
    | "filter_exhausted" -> [ (fun c -> c.["failure_id"] <- box "property-panic"); (fun c -> c.["verdict"] <- box "pass") ]
    | "failure_id_valid" -> [ fun c -> c.["valid"] <- corruptValue c.["valid"] ]
    | "corpus_reject" -> [ fun c -> c.["text"] <- box "tfv 1\nr R1 Kop S706274\n" ]
    | _ -> failwithf "no corruption defined for op %s" op

let private copy (cases: obj list) = cases |> List.map deepCopy

let private problemFor (cases: obj list) (id: string) (fragment: string) =
    (run cases).Problems |> List.exists (fun p -> p.StartsWith(id + ":", StringComparison.Ordinal) && p.Contains fragment)

// ---- the positive run ---------------------------------------------------------------------------

type Replay() =
    inherit SeamTest()

    [<Fact>]
    member _.``the whole corpus conforms: 25 of 25 evaluated through the F# API``() =
        let report = run (loadCases ())
        Assert.Empty report.Problems
        Assert.Equal(25, report.Evaluated)
        Assert.Equal(25, CaseCount)

    [<Fact>]
    member _.``every op of the fixture is present``() =
        let ops = loadCases () |> List.map (fun c -> (obj' c).["op"] :?> string) |> Set.ofList
        Assert.Equal(11, ops.Count)

    // ---- negative controls ---------------------------------------------------------------------

    [<Fact>]
    member _.``corrupting any case fails the run and names exactly that case``() =
        let pristine = loadCases ()
        let mutable corrupted = 0
        pristine
        |> List.iteri (fun i original ->
            let o = obj' original
            let id = o.["id"] :?> string
            let op = o.["op"] :?> string
            for corruption in corruptions op do
                let cases = copy pristine
                corruption (obj' cases.[i])
                Assert.NotEqual<string>(Canon.Render(deepCopy original), Canon.Render(cases.[i]))
                let report = run cases
                Assert.True(not report.Problems.IsEmpty, id + " (" + op + "): corruption went unnoticed")
                for p in report.Problems do
                    Assert.True(p.StartsWith(id + ":", StringComparison.Ordinal), id + ": problem names another case or none: " + p)
                corrupted <- corrupted + 1)
        Assert.True(corrupted >= 25, sprintf "every case must be corrupted at least once, got %d" corrupted)

    [<Fact>]
    member _.``dropping any case is a problem``() =
        let pristine = loadCases ()
        for i in 0 .. pristine.Length - 1 do
            let cases = pristine |> List.indexed |> List.filter (fun (j, _) -> j <> i) |> List.map snd
            let report = run cases
            Assert.NotEmpty report.Problems
            Assert.Contains("expected 25 cases, got 24", report.Problems.Head)
        Assert.NotEmpty (run []).Problems
        Assert.NotEmpty (run (List.truncate 8 pristine)).Problems

    [<Fact>]
    member _.``a truncated document fails to parse``() =
        let data = IO.File.ReadAllBytes(fixturePath ())
        Assert.Throws<JsonException>(fun () -> Corpus.Parse(Array.sub data 0 (data.Length / 2)) |> ignore) |> ignore
        Assert.Throws<JsonException>(fun () -> Corpus.Parse(Array.empty) |> ignore) |> ignore

    [<Fact>]
    member _.``duplicated case ids are a problem``() =
        let cases = copy (loadCases ())
        let cases2 = cases |> List.mapi (fun i c -> if i = 1 then deepCopy cases.[0] else c)
        Assert.Contains("duplicate or missing case id", (run cases2).Problems)

    [<Fact>]
    member _.``an unknown op and a bad field set are problems``() =
        let cases = copy (loadCases ())
        let first = obj' cases.[0]
        let id = first.["id"] :?> string
        first.["op"] <- box "gen_everything"
        Assert.True(problemFor cases id "unknown op")
        first.["op"] <- box "gen_primitive"
        first.["surprise"] <- box "field"
        Assert.True(problemFor cases id "fields")
        first.Remove "surprise" |> ignore
        first.Remove "count" |> ignore
        Assert.True(problemFor cases id "fields")

    [<Fact>]
    member _.``a non-object case and a non-string id are problems, not crashes``() =
        let cases = loadCases ()
        let bad = box "not an object" :: cases.Tail
        Assert.Contains((run bad).Problems, fun p -> p.Contains "case is not an object")
        let numId = Dictionary<string, obj>()
        numId.["id"] <- box (JsonNumber "7")
        numId.["op"] <- box "x"
        Assert.Contains((run (box numId :: cases.Tail)).Problems, fun p -> p.Contains "missing a string id")

    [<Fact>]
    member _.``the coverage gates need every op and every failure id``() =
        let only =
            loadCases ()
            |> List.filter (fun c ->
                let m = obj' c
                (m.["op"] :?> string) = "failure_id_valid"
                && (m.["input"] :?> string) <> "unregistered-id"
                && (m.["input"] :?> string) <> "property-panic")
        let problems = (evaluateWith only -1).Problems
        Assert.Contains(problems, fun p -> p.Contains "operations never exercised")
        Assert.Contains(problems, fun p -> p.Contains "registered failure ids missing")
        Assert.Contains(problems, fun p -> p.Contains "unregistered-id rejection")

    [<Fact>]
    member _.``an undecodable case is a problem naming the case``() =
        let cases = copy (loadCases ())
        let first = obj' cases.[0]
        let id = first.["id"] :?> string
        let intNode (text: string) =
            let d = Dictionary<string, obj>()
            d.["kind"] <- box "int"
            d.["value"] <- box text
            box d
        first.["count"] <- intNode "007"
        Assert.True(problemFor cases id "non-canonical int")
        first.["count"] <- intNode "99999"
        Assert.True(problemFor cases id "outside")
        first.["count"] <- intNode "3"
        Assert.True(problemFor cases id "exactly count")

    [<Fact>]
    member _.``a shrinker handed a non-falsifying start is a problem, not a pass``() =
        // shrink_int whose initial satisfies the property: the F# API says NotFalsifying.
        let cases = copy (loadCases ())
        let i = cases |> List.findIndex (fun c -> (obj' c).["op"] :?> string = "shrink_int")
        let c = obj' cases.[i]
        let id = c.["id"] :?> string
        let intNode (text: string) =
            let d = Dictionary<string, obj>()
            d.["kind"] <- box "int"
            d.["value"] <- box text
            box d
        c.["initial"] <- intNode "0"
        c.["target"] <- intNode "0"
        Assert.True(problemFor cases id "does not falsify")

    // ---- F#-layer sabotage: the corpus must notice a wrong F# wrapper -------------------------------

    [<Fact>]
    member _.``a shifted seed in the F# layer is caught``() =
        Seams.seedOffset <- 1UL
        let problems = (run (loadCases ())).Problems
        Assert.Contains(problems, fun p -> p.StartsWith "pbt/gen_")
        Assert.Contains(problems, fun p -> p.StartsWith "pbt/replay/")

    [<Fact>]
    member _.``a dropped shrink step in the F# layer is caught``() =
        Seams.shrinkStepDelta <- -1
        let problems = (run (loadCases ())).Problems
        Assert.Contains(problems, fun p -> p.StartsWith "pbt/shrink_" && p.Contains "step")

    [<Fact>]
    member _.``an extra shrink step in the F# layer is caught``() =
        Seams.shrinkStepDelta <- 1
        Assert.NotEmpty (run (loadCases ())).Problems

    [<Fact>]
    member _.``swapped bounds in the F# layer are caught``() =
        Seams.swapBounds <- true
        let problems = (run (loadCases ())).Problems
        Assert.NotEmpty problems
        Assert.Contains(problems, fun p -> p.StartsWith "pbt/gen_bounded")

    [<Fact>]
    member _.``resetting the seams restores a green corpus``() =
        Seams.seedOffset <- 5UL
        Seams.shrinkStepDelta <- 2
        Seams.swapBounds <- true
        Seams.reset ()
        Assert.Empty (run (loadCases ())).Problems

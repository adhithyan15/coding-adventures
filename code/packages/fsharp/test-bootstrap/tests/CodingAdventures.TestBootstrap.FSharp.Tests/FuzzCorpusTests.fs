/// Replays the whole committed fuzz-v1 corpus through the F# API, with its negative controls and the
/// F#-layer sabotage seams (VER06m sections 3 and 4).
module CodingAdventures.TestBootstrap.FSharp.Tests.FuzzCorpusTests

open System
open System.Collections.Generic
open CodingAdventures.TestBootstrap.CorpusJson
open CodingAdventures.TestBootstrap.Fuzz
open CodingAdventures.TestBootstrap.FSharp
open CodingAdventures.TestBootstrap.FSharp.Fuzzing
open CodingAdventures.TestBootstrap.FSharp.Tests.Support
open CodingAdventures.TestBootstrap.FSharp.Tests.FuzzSupport
open CodingAdventures.TestBootstrap.FSharp.Tests.FsFuzzCorpusRunner
open Xunit

let private run (cases: obj list) = bounded 120 (fun () -> evaluate cases)

let private intNode (text: string) : obj =
    let d = Dictionary<string, obj>()
    d.["kind"] <- box "int"
    d.["value"] <- box text
    box d

let private bytesNode (hexText: string) : obj =
    let d = Dictionary<string, obj>()
    d.["kind"] <- box "bytes"
    d.["hex"] <- box hexText
    box d

// ---- corruption: same shape, different value ---------------------------------------------------

let private corruptValue (node: obj) : obj =
    match node with
    | :? bool as b -> box (not b)
    | :? string as s -> box (s + "x")
    | _ ->
        let m = obj' node
        if (m.["kind"] :?> string) = "int" then
            m.["value"] <- box (string (bigint.Parse(m.["value"] :?> string) + 1I))
        else
            let h = m.["hex"] :?> string
            m.["hex"] <- box (if h.Length = 0 then "ff" else h.Substring 2)
        box m

let private corruptField (field: string) : Dictionary<string, obj> -> unit =
    fun c -> c.[field] <- corruptValue c.[field]

let private corruptions (op: string) : (Dictionary<string, obj> -> unit) list =
    match op with
    | "mutate_bit_flip" | "mutate_byte_flip" | "mutate_arithmetic" | "mutate_boundary" | "mutate_block_delete"
    | "mutate_block_insert" | "mutate_block_duplicate" | "mutate_splice" | "mutate_sequence" ->
        [ corruptField "output" ]
    | "corpus_admit" -> [ corruptField "admitted"; corruptField "new_edges" ]
    | "crash_dedup" -> [ corruptField "is_duplicate" ]
    | "failure_id_valid" -> [ corruptField "valid" ]
    | "private_reporting" -> [ corruptField "tap_line"; corruptField "contains_raw_payload" ]
    | "corpus_reject" -> [ fun c -> c.["text"] <- box "tfv 1\nr R1 Kop S706274\n" ]
    | _ -> failwithf "no corruption defined for op %s" op

let private copy (cases: obj list) = cases |> List.map deepCopy

let private problemFor (cases: obj list) (id: string) (fragment: string) =
    (run cases).Problems |> List.exists (fun p -> p.StartsWith(id + ":", StringComparison.Ordinal) && p.Contains fragment)

let private firstOf (cases: obj list) (op: string) : Dictionary<string, obj> =
    cases |> List.find (fun c -> (obj' c).["op"] :?> string = op) |> obj'

// ---- the positive run ---------------------------------------------------------------------------

type Replay() =
    inherit FuzzSeamTest()

    [<Fact>]
    member _.``the whole corpus conforms: 25 of 25 evaluated through the F# API``() =
        let report = run (loadFuzzCases ())
        Assert.Empty report.Problems
        Assert.Equal(25, report.Evaluated)
        Assert.Equal(25, CaseCount)

    [<Fact>]
    member _.``every op of the fixture is present``() =
        let ops = loadFuzzCases () |> List.map (fun c -> (obj' c).["op"] :?> string) |> Set.ofList
        Assert.Equal(14, ops.Count)

    // ---- negative controls ---------------------------------------------------------------------

    [<Fact>]
    member _.``corrupting any case fails the run and names exactly that case``() =
        let pristine = loadFuzzCases ()
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
        let pristine = loadFuzzCases ()
        for i in 0 .. pristine.Length - 1 do
            let cases = pristine |> List.indexed |> List.filter (fun (j, _) -> j <> i) |> List.map snd
            let report = run cases
            Assert.NotEmpty report.Problems
            Assert.Contains("expected 25 cases, got 24", report.Problems.Head)
        Assert.NotEmpty (run []).Problems
        Assert.NotEmpty (run (List.truncate 8 pristine)).Problems

    [<Fact>]
    member _.``a truncated document fails to parse``() =
        let data = IO.File.ReadAllBytes(fuzzFixturePath ())
        Assert.Throws<JsonException>(fun () -> FuzzCorpus.Parse(Array.sub data 0 (data.Length / 2)) |> ignore) |> ignore
        Assert.Throws<JsonException>(fun () -> FuzzCorpus.Parse(Array.empty) |> ignore) |> ignore

    [<Fact>]
    member _.``duplicated case ids are a problem``() =
        let cases = copy (loadFuzzCases ())
        let cases2 = cases |> List.mapi (fun i c -> if i = 1 then deepCopy cases.[0] else c)
        Assert.Contains("duplicate or missing case id", (run cases2).Problems)

    [<Fact>]
    member _.``an unknown op and a bad field set are problems``() =
        let cases = copy (loadFuzzCases ())
        let first = obj' cases.[0]
        let id = first.["id"] :?> string
        first.["op"] <- box "mutate_everything"
        Assert.True(problemFor cases id "unknown op")
        first.["op"] <- box "mutate_bit_flip"
        first.["surprise"] <- box "field"
        Assert.True(problemFor cases id "fields")
        first.Remove "surprise" |> ignore
        first.Remove "count" |> ignore
        Assert.True(problemFor cases id "fields")

    [<Fact>]
    member _.``a non-object case and a non-string id are problems, not crashes``() =
        let cases = loadFuzzCases ()
        let bad = box "not an object" :: cases.Tail
        Assert.Contains((run bad).Problems, fun p -> p.Contains "case is not an object")
        let numId = Dictionary<string, obj>()
        numId.["id"] <- box (JsonNumber "7")
        numId.["op"] <- box "x"
        Assert.Contains((run (box numId :: cases.Tail)).Problems, fun p -> p.Contains "missing a string id")

    [<Fact>]
    member _.``the coverage gates need every op and an accepted and a rejected failure id``() =
        let only = loadFuzzCases () |> List.filter (fun c -> (obj' c).["op"] :?> string = "mutate_bit_flip")
        let problems = (evaluateWith only -1).Problems
        Assert.Contains(problems, fun p -> p.Contains "operations never exercised")
        Assert.Contains(problems, fun p -> p.Contains "registered-id acceptance")
        Assert.Contains(problems, fun p -> p.Contains "unregistered-id rejection")

    [<Fact>]
    member _.``an undecodable case is a problem naming the case``() =
        let cases = copy (loadFuzzCases ())
        let first = obj' cases.[0]
        let id = first.["id"] :?> string
        first.["count"] <- intNode "007"
        Assert.True(problemFor cases id "non-canonical int")
        first.["count"] <- box "not an int"
        Assert.True(problemFor cases id "malformed" || problemFor cases id "could not be evaluated")
        first.["count"] <- intNode "1"
        first.["input"] <- bytesNode "0"
        Assert.True(problemFor cases id "could not be evaluated")

    [<Fact>]
    member _.``a wrongly typed field is a problem naming the case``() =
        // Decodes fine but is the wrong type for the op: each typed accessor reports it.
        let wrong (op: string) (field: string) (value: obj) (fragment: string) =
            let cases = copy (loadFuzzCases ())
            let c = firstOf cases op
            c.[field] <- value
            Assert.True(problemFor cases (c.["id"] :?> string) fragment, op + "." + field + " bad type went unnoticed")
        wrong "mutate_bit_flip" "input" (box "text") "must be bytes"
        wrong "mutate_bit_flip" "count" (box "text") "must be an int"
        wrong "mutate_arithmetic" "endian" (box "middle") "endian must be"
        wrong "mutate_arithmetic" "endian" (intNode "1") "must be a string"
        wrong "corpus_admit" "admitted" (intNode "1") "must be a bool"
        wrong "corpus_admit" "existing_edges" (box "x") "must be a list of ints"
        wrong "corpus_admit" "existing_edges" (box (List<obj>([ box "x" ]))) "must be a list of ints"
        wrong "corpus_admit" "existing_edges" (box (List<obj>([ intNode "-1" ]))) "outside [0, 2^64)"
        wrong "crash_dedup" "crash_a_frames" (box "x") "must be a list of strings"
        wrong "crash_dedup" "crash_a_frames" (box (List<obj>([ intNode "1" ]))) "must be a list of strings"
        wrong "private_reporting" "failure_id" (box "fuzz-arbitrary-error") "refused"
        wrong "private_reporting" "sig_hash" (box "XYZ") "refused"
        wrong "corpus_reject" "text" (intNode "1") "must be a string"
        wrong "mutate_sequence" "steps" (intNode "99999") "outside"
        wrong "failure_id_valid" "input" (intNode "1") "disagrees"

    [<Fact>]
    member _.``the aggregate mutate_sequence budget bounds a hostile corpus``() =
        let cases = copy (loadFuzzCases ())
        let seqCase = firstOf cases "mutate_sequence"
        let id = seqCase.["id"] :?> string
        // 10,000 steps of a 10,000-byte-capped buffer: charge is far past 2^24.
        seqCase.["steps"] <- intNode "10000"
        seqCase.["input"] <- bytesNode (String('0', 20_000))
        Assert.True(problemFor cases id "work budget")
        // A second sequence case after a breach fails without running.
        let clone = deepCopy (box seqCase) |> obj'
        clone.["id"] <- box (id + "/second")
        clone.["steps"] <- intNode "1"
        clone.["input"] <- bytesNode "00"
        let report = run (cases @ [ box clone ])
        Assert.Contains(report.Problems, fun p -> p.StartsWith(id + "/second:") && p.Contains "work budget")

    [<Fact>]
    member _.``too many cases are capped, not run``() =
        let one = loadFuzzCases () |> List.head
        let many = [ for i in 0 .. 10_000 -> (let c = obj' (deepCopy one) in c.["id"] <- box (sprintf "case/%d" i); box c) ]
        let report = bounded 120 (fun () -> evaluateWith many -1)
        Assert.Contains(report.Problems, fun p -> p.Contains "exceeds the cap")

    // ---- F#-layer sabotage: the corpus must notice a wrong F# wrapper -------------------------------

    [<Fact>]
    member _.``an offset shifted in the F# mutators is caught``() =
        FuzzSeams.offsetDelta <- 1I
        let problems = (run (loadFuzzCases ())).Problems
        Assert.Contains(problems, fun p -> p.StartsWith "fuzz/mutate_bit_flip/bit0:")
        Assert.Contains(problems, fun p -> p.StartsWith "fuzz/mutate_splice/")

    [<Fact>]
    member _.``a flipped byte order in the F# mutators is caught``() =
        FuzzSeams.swapEndian <- true
        let problems = (run (loadFuzzCases ())).Problems
        Assert.Contains(problems, fun p -> p.StartsWith "fuzz/mutate_arithmetic/u16_le_add35:")
        Assert.Contains(problems, fun p -> p.StartsWith "fuzz/mutate_boundary/u32_max_le:" || p.StartsWith "fuzz/mutate_boundary/u32_signed_min_le:")

    [<Fact>]
    member _.``an inverted dedup in the F# layer is caught``() =
        FuzzSeams.invertDedup <- true
        let problems = (run (loadFuzzCases ())).Problems
        Assert.Contains(problems, fun p -> p.StartsWith "fuzz/crash_dedup/same_signature:")
        Assert.Contains(problems, fun p -> p.StartsWith "fuzz/crash_dedup/divergent_signature:")

    [<Fact>]
    member _.``a leaked tap target in the F# layer is caught``() =
        FuzzSeams.leakTapTarget <- true
        Assert.Contains((run (loadFuzzCases ())).Problems, fun p -> p.StartsWith "fuzz/private_reporting/opaque_token:")

    [<Fact>]
    member _.``an admit-everything F# layer is caught``() =
        FuzzSeams.admitEverything <- true
        Assert.Contains((run (loadFuzzCases ())).Problems, fun p -> p.StartsWith "fuzz/corpus_admit/redundant_edge:")

    [<Fact>]
    member _.``an accept-any failure id F# layer is caught``() =
        FuzzSeams.acceptAnyFailureId <- true
        let problems = (run (loadFuzzCases ())).Problems
        Assert.Contains(problems, fun p -> p.StartsWith "fuzz/failure_id/unregistered:")

    [<Fact>]
    member _.``a shifted seed in the F# sequence wrapper is caught``() =
        FuzzSeams.seedOffset <- 1UL
        Assert.Contains((run (loadFuzzCases ())).Problems, fun p -> p.StartsWith "fuzz/mutate_sequence/seed42_steps4:")

    [<Fact>]
    member _.``resetting the seams restores a green corpus``() =
        FuzzSeams.offsetDelta <- 5I
        FuzzSeams.swapEndian <- true
        FuzzSeams.invertDedup <- true
        FuzzSeams.leakTapTarget <- true
        FuzzSeams.admitEverything <- true
        FuzzSeams.acceptAnyFailureId <- true
        FuzzSeams.seedOffset <- 9UL
        FuzzSeams.energyDelta <- 3
        FuzzSeams.dropFrame <- true
        FuzzSeams.reset ()
        let report = run (loadFuzzCases ())
        Assert.Empty report.Problems
        Assert.Equal(25, report.Evaluated)

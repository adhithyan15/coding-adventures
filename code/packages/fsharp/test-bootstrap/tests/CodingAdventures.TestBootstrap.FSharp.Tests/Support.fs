/// Shared helpers: fixture lookup, deep copies, seam hygiene and a hang guard.
module CodingAdventures.TestBootstrap.FSharp.Tests.Support

open System
open System.IO
open System.Threading.Tasks
open CodingAdventures.TestBootstrap.CorpusJson
open CodingAdventures.TestBootstrap.Pbt
open CodingAdventures.TestBootstrap.FSharp
open Xunit

// The seams are process-global, so no test may overlap another.
[<assembly: CollectionBehavior(DisableTestParallelization = true)>]
do ()

/// The committed fixture, found by walking up from the test binary. Never a copy.
let fixturePath () : string =
    let rec up (dir: DirectoryInfo) =
        if isNull dir then
            failwith "could not find code/specs/fixtures/pbt-v1/cases.json above the test binary"
        else
            let candidate = Path.Combine(dir.FullName, "code", "specs", "fixtures", "pbt-v1", "cases.json")
            if File.Exists candidate then candidate else up dir.Parent
    up (DirectoryInfo AppContext.BaseDirectory)

let loadCases () : obj list =
    // The envelope is decoded by the C# strict reader; the cases are evaluated by F# only.
    Corpus.Parse(File.ReadAllBytes(fixturePath ())) |> Seq.cast<obj> |> List.ofSeq

let rec deepCopy (node: obj) : obj =
    match node with
    | :? Collections.Generic.Dictionary<string, obj> as m ->
        let d = Collections.Generic.Dictionary<string, obj>()
        for kv in m do
            d.[kv.Key] <- deepCopy kv.Value
        box d
    | :? Collections.Generic.List<obj> as l ->
        let r = Collections.Generic.List<obj>()
        for x in l do
            r.Add(deepCopy x)
        box r
    | other -> other

let obj' (o: obj) : Collections.Generic.Dictionary<string, obj> =
    o :?> Collections.Generic.Dictionary<string, obj>

let utf8 (s: string) : byte[] = Text.Encoding.UTF8.GetBytes s

/// Runs `f` on a worker and fails (instead of hanging the run) if it takes longer than `seconds`.
let bounded (seconds: int) (f: unit -> 'a) : 'a =
    let task = Task.Run(fun () -> f ())
    if task.Wait(TimeSpan.FromSeconds(float seconds)) then
        task.Result
    else
        failwithf "test body still running after %d seconds: treated as a hang" seconds

/// Base class: every test starts and ends with production seams.
type SeamTest() =
    do Seams.reset ()
    interface IDisposable with
        member _.Dispose() = Seams.reset ()

let big (n: int) : bigint = bigint n

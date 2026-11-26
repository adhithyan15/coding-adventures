/// Shared helpers of the F# fuzz tests: the committed fuzz-v1 fixture, hex, seam hygiene.
module CodingAdventures.TestBootstrap.FSharp.Tests.FuzzSupport

open System
open System.IO
open CodingAdventures.TestBootstrap.Fuzz
open CodingAdventures.TestBootstrap.FSharp
open CodingAdventures.TestBootstrap.FSharp.Fuzzing

/// The committed fuzz-v1 fixture, found by walking up from the test binary. Never a copy.
let fuzzFixturePath () : string =
    let rec up (dir: DirectoryInfo) =
        if isNull dir then
            failwith "could not find code/specs/fixtures/fuzz-v1/cases.json above the test binary"
        else
            let candidate = Path.Combine(dir.FullName, "code", "specs", "fixtures", "fuzz-v1", "cases.json")
            if File.Exists candidate then candidate else up dir.Parent
    up (DirectoryInfo AppContext.BaseDirectory)

/// The fuzz-v1 cases. The envelope is decoded by the C# strict reader; the cases are evaluated by F# only.
let loadFuzzCases () : obj list =
    FuzzCorpus.Parse(File.ReadAllBytes(fuzzFixturePath ())) |> Seq.cast<obj> |> List.ofSeq

let hx (s: string) : byte[] = Convert.FromHexString s

let hex (b: byte[]) : string = Convert.ToHexString(b).ToLowerInvariant()

let bg (s: string) : bigint = bigint.Parse s

/// Base class: every test starts and ends with production seams (PBT's and the fuzz layer's).
type FuzzSeamTest() =
    do
        Seams.reset ()
        FuzzSeams.reset ()
    interface IDisposable with
        member _.Dispose() =
            Seams.reset ()
            FuzzSeams.reset ()

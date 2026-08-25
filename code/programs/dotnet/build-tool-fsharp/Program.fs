module BuildToolFSharp.Program

open System.Collections.Generic
open System.Runtime.CompilerServices
open CodingAdventures.BuildTool.CSharp

// build-tool -- F# front door over the shared .NET build engine
// ==============================================================
//
// The repo now has both a C# and an F# entry point for the build tool. The
// heavy lifting lives in the shared .NET engine exposed by the C# project so
// the language-paired programs stay behaviorally identical instead of drifting.

[<MethodImpl(MethodImplOptions.NoInlining)>]
let validateTrackedArtifactSnapshot
    (unicodeVersion: string)
    (entries: IReadOnlyList<TrackedArtifactEntry>) =
    Validator.ValidateTrackedArtifactSnapshot(unicodeVersion, entries)

[<EntryPoint>]
let main argv =
    BuildToolApp.RunAsync(argv) |> Async.AwaitTask |> Async.RunSynchronously

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
let validateTrackedArtifactSnapshot (unicodeVersion: string) (entries: IReadOnlyList<TrackedArtifactEntry>) =
    Validator.ValidateTrackedArtifactSnapshot(unicodeVersion, entries)

// An F# front door is evidence only when callers can enter the reviewed
// process-free contract through an F# symbol. Keeping this facade deliberately
// thin preserves the shared .NET implementation while making the language
// boundary visible to tests, documentation, and coverage tools.
[<MethodImpl(MethodImplOptions.NoInlining)>]
let validateOrphanCrateSnapshot (snapshot: OrphanCrateSnapshot) =
    Validator.ValidateOrphanCrateSnapshot(snapshot)

// Toolchain declarations are BUILD metadata, not executable commands. This
// native F# symbol accepts only the already-bounded, caller-supplied snapshot
// records defined by the shared .NET engine. It deliberately does not discover
// a checkout, inspect Git, read the environment, or launch the CLI; the F#
// fixture surface can therefore prove the portable decision contract directly.
[<MethodImpl(MethodImplOptions.NoInlining)>]
let evaluateToolchainSnapshot
    (platform: string)
    (forceFull: bool)
    (packages: IReadOnlyList<ToolchainPackageSnapshot>)
    (scheduledPackages: IReadOnlyList<string>)
    (forcedToolchains: IReadOnlyList<string>)
    =
    ToolchainDetection.EvaluateSnapshot(platform, forceFull, packages, scheduledPackages, forcedToolchains)

[<EntryPoint>]
let main argv =
    BuildToolApp.RunAsync(argv) |> Async.AwaitTask |> Async.RunSynchronously

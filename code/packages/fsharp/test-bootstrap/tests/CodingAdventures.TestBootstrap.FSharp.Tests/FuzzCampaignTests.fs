/// Campaign-level tests of the F# fuzz binding (VER06m sections 5, 6 and 8): the executor's semantics
/// (fresh handle per run, sticky exhaustion, stale handles, OOM mapping) reach F# targets exactly as
/// they reach C# lambdas; exceptions from F# code, `failwith`, `async { }` and `task { }` are crashes;
/// printing a report, a crash record or the options never shows input bytes or the seed.
///
/// Every campaign runs through `bounded`, so a regression fails the test instead of hanging the run.
module CodingAdventures.TestBootstrap.FSharp.Tests.FuzzCampaignTests

#nowarn "25"

open System
open System.Runtime.CompilerServices
open System.Threading
open System.Threading.Tasks
open CodingAdventures.TestBootstrap.FSharp
open CodingAdventures.TestBootstrap.FSharp.Fuzzing
open CodingAdventures.TestBootstrap.FSharp.Tests.Support
open CodingAdventures.TestBootstrap.FSharp.Tests.FuzzSupport
open Xunit

let private opts (iterations: int) : FuzzOptions =
    { FuzzOptions.defaults with MaxIterations = iterations; Seed = 1UL }

/// Runs a campaign on a worker with a deadline.
let private campaign (seeds: byte[] list) (o: FuzzOptions) (target: FuzzTarget) : FuzzReport =
    bounded 60 (fun () -> Campaign.run seeds o target)

/// One triage execution of `target` under the given budgets (no mutation).
let private once (stepBudget: int) (memBudget: int) (target: FuzzTarget) : FuzzReport =
    campaign [ [| 1uy |] ] { opts 0 with StepBudget = stepBudget; MemBudget = memBudget } target

let private swallow (f: unit -> unit) : unit =
    try
        f ()
    with _ ->
        ()

// Named, non-inlined crash sites: their frames are stable (a lambda's closure is named after a source line).
[<MethodImpl(MethodImplOptions.NoInlining)>]
let private siteA (_d: byte[]) : unit = failwith "boom from site A"

[<MethodImpl(MethodImplOptions.NoInlining)>]
let private siteB (_d: byte[]) : unit = invalidOp "boom from site B"

let private lateFailure : Async<unit> =
    async {
        do! Async.Sleep 1
        invalidOp "late"
    }

type Semantics() =
    inherit FuzzSeamTest()

    [<Fact>]
    member _.``a clean campaign reports edges, iterations and the caller-reported tier``() =
        let r = campaign [ [| 1uy; 2uy |] ] (opts 50) (fun d h -> h.Edge 1UL; if d.Length > 1 then h.Edge "long")
        Assert.True r.IsClean
        Assert.Equal(50, r.Iterations)
        Assert.Equal(2, r.TotalEdges)
        Assert.Equal(CallerReported, r.CoverageTier)
        Assert.True(r.AdmittedSeeds >= 1)
        Assert.Empty r.Crashes
        Assert.Equal(0, r.UniqueCrashes)
        Assert.Equal(0, r.Timeouts + r.Ooms + r.CrashesDropped + r.AdmissionsDropped)

    [<Fact>]
    member _.``a target that reports no edge is black-box and still clean``() =
        let r = campaign [ [| 1uy |] ] (opts 10) (fun _ _ -> ())
        Assert.Equal(BlackBox, r.CoverageTier)
        Assert.Equal(0, r.TotalEdges)
        Assert.True r.IsClean
        Assert.Equal(1, r.AdmittedSeeds)

    [<Fact>]
    member _.``zero iterations is not clean``() =
        let r = campaign [ [| 1uy |] ] (opts 0) (fun _ _ -> ())
        Assert.False r.IsClean
        Assert.Equal(0, r.Iterations)

    [<Fact>]
    member _.``an edge by id and an edge by label are different edges``() =
        let ids = campaign [ [| 1uy |] ] (opts 0) (fun _ h -> h.Edge 7UL; h.Edge 8UL)
        Assert.Equal(2, ids.TotalEdges)
        let same = campaign [ [| 1uy |] ] (opts 0) (fun _ h -> h.Edge 7UL; h.Edge 7UL)
        Assert.Equal(1, same.TotalEdges)
        let labels = campaign [ [| 1uy |] ] (opts 0) (fun _ h -> h.Edge "a"; h.Edge "b"; h.Edge "a")
        Assert.Equal(2, labels.TotalEdges)
        // ids are reduced mod 65,536 by the bitmap: 1 and 65537 are one edge
        let reduced = campaign [ [| 1uy |] ] (opts 0) (fun _ h -> h.Edge 1UL; h.Edge 65537UL)
        Assert.Equal(1, reduced.TotalEdges)
        // a label edge is the FNV-1a id edge
        let viaId = campaign [ [| 1uy |] ] (opts 0) (fun _ h -> h.Edge "a"; h.Edge(Handle.EdgeId "a"))
        Assert.Equal(1, viaId.TotalEdges)
        let curried = campaign [ [| 1uy |] ] (opts 0) (fun _ h -> Handle.edge h 7UL; Handle.edgeLabel h "a"; Handle.edge h (Handle.edgeId "b"))
        Assert.Equal(3, curried.TotalEdges)

    [<Fact>]
    member _.``tick forwards: the budget-th tick passes and the next one is a timeout``() =
        let pass = once 3 100 (fun _ h -> h.Tick(); h.Tick(); h.Tick())
        Assert.Equal(0, pass.Timeouts)
        let over = once 3 100 (fun _ h -> h.Tick(); h.Tick(); h.Tick(); h.Tick())
        Assert.Equal(1, over.Timeouts)
        let curried = once 1 100 (fun _ h -> Handle.tick h; Handle.tick h)
        Assert.Equal(1, curried.Timeouts)
        let loop = once 10 100 (fun _ h -> while true do h.Tick())
        Assert.Equal(1, loop.Timeouts)

    [<Fact>]
    member _.``reserve forwards: the budget fits, one byte more is an OOM, non-positive reserves nothing``() =
        Assert.Equal(0, (once 100 100 (fun _ h -> h.Reserve 100L)).Ooms)
        Assert.Equal(0, (once 100 100 (fun _ h -> h.Reserve 60L; h.Reserve 40L)).Ooms)
        Assert.Equal(1, (once 100 100 (fun _ h -> h.Reserve 60L; h.Reserve 41L)).Ooms)
        Assert.Equal(1, (once 100 100 (fun _ h -> h.Reserve 101L)).Ooms)
        Assert.Equal(1, (once 100 100 (fun _ h -> Handle.reserve h 101L)).Ooms)
        Assert.Equal(0, (once 100 100 (fun _ h -> h.Reserve 0L; h.Reserve -5L; h.Reserve Int64.MinValue)).Ooms)

    [<Fact>]
    member _.``a real OutOfMemoryException is an OOM, not a crash``() =
        let r = once 100 100 (fun _ _ -> raise (OutOfMemoryException()))
        Assert.Equal(1, r.Ooms)
        Assert.Empty r.Crashes

    // ---- sticky exhaustion: every swallow shape, both budgets ----------------------------------------

    [<Fact>]
    member _.``swallowing a breach and calling the OTHER hook is still stopped``() =
        // swallow a steps breach, then reserve: a timeout
        let a = once 1 100 (fun _ h -> h.Tick(); swallow h.Tick; h.Reserve 1L)
        Assert.Equal((1, 0, 0), (a.Timeouts, a.Ooms, a.Crashes.Length))
        // swallow a memory breach, then tick: an OOM (the breach that ran out first)
        let b = once 1000 100 (fun _ h -> swallow (fun () -> h.Reserve 1000L); h.Tick())
        Assert.Equal((0, 1, 0), (b.Timeouts, b.Ooms, b.Crashes.Length))
        // swallow a memory breach, then reserve again
        let c = once 1000 100 (fun _ h -> swallow (fun () -> h.Reserve 1000L); h.Reserve 1L)
        Assert.Equal((0, 1, 0), (c.Timeouts, c.Ooms, c.Crashes.Length))
        // swallow a steps breach, then tick again
        let d = once 0 100 (fun _ h -> swallow h.Tick; h.Tick())
        Assert.Equal((1, 0, 0), (d.Timeouts, d.Ooms, d.Crashes.Length))

    [<Fact>]
    member _.``swallowing a breach and returning is a timeout or an OOM, never a pass``() =
        let steps = once 0 100 (fun _ h -> swallow h.Tick)
        Assert.Equal((1, 0, 0), (steps.Timeouts, steps.Ooms, steps.AdmittedSeeds))
        let mem = once 1000 100 (fun _ h -> swallow (fun () -> h.Reserve 1000L))
        Assert.Equal((0, 1, 0), (mem.Timeouts, mem.Ooms, mem.AdmittedSeeds))

    [<Fact>]
    member _.``swallowing a breach and then throwing something else is still the breach``() =
        let steps = once 0 100 (fun _ h -> swallow h.Tick; failwith "ordinary")
        Assert.Equal((1, 0, 0), (steps.Timeouts, steps.Ooms, steps.Crashes.Length))
        let mem = once 1000 100 (fun _ h -> swallow (fun () -> h.Reserve 1000L); failwith "ordinary")
        Assert.Equal((0, 1, 0), (mem.Timeouts, mem.Ooms, mem.Crashes.Length))

    [<Fact>]
    member _.``a swallowing loop only ever gets sentinels back``() =
        let mutable swallowed = 0
        let r = once 1000 100 (fun _ h -> for _ in 1 .. 5000 do (try h.Tick() with _ -> swallowed <- swallowed + 1))
        Assert.Equal(1, r.Timeouts)
        Assert.Equal(4000, swallowed)

    // ---- a fresh handle per run; a stale handle is inert -----------------------------------------------

    [<Fact>]
    member _.``budgets restart for every execution``() =
        // 5 ticks per run with a budget of 5: a handle shared between runs would time out on the second.
        let r = campaign [ [| 1uy |] ] ({ opts 20 with StepBudget = 5 }) (fun _ h -> for _ in 1 .. 5 do h.Tick())
        Assert.Equal(0, r.Timeouts)
        Assert.True r.IsClean
        let m = campaign [ [| 1uy |] ] ({ opts 20 with MemBudget = 10 }) (fun _ h -> h.Reserve 10L)
        Assert.Equal(0, m.Ooms)

    [<Fact>]
    member _.``a stale handle from a spawned Task is inert``() =
        use release = new ManualResetEventSlim(false)
        let mutable spawned : Task = null
        let r =
            campaign [ [| 1uy |] ] (opts 0) (fun _ h ->
                spawned <- Task.Run(fun () ->
                    release.Wait()
                    h.Edge 5UL
                    h.Tick()))
        // a plain Task is not async void: the run itself is fine
        Assert.Equal(0, r.Crashes.Length)
        Assert.Equal(1, r.AdmittedSeeds)
        release.Set()
        // the retired Tick is refused (the Task faults) ...
        Assert.Throws<AggregateException>(fun () -> spawned.Wait 10_000 |> ignore) |> ignore
        // ... and the stale Edge recorded nothing
        Assert.Equal(0, r.TotalEdges)
        Assert.Equal(BlackBox, r.CoverageTier)

    [<Fact>]
    member _.``a stale handle used by an un-awaited async workflow records nothing``() =
        // Documented hazard (VER06m 6.3): Edge is safe after the run, Tick is not, so only Edge is used.
        use release = new ManualResetEventSlim(false)
        use finished = new ManualResetEventSlim(false)
        let r =
            campaign [ [| 1uy |] ] (opts 0) (fun _ h ->
                Async.Start(async {
                    release.Wait()
                    h.Edge 9UL
                    finished.Set() }))
        Assert.Equal(1, r.AdmittedSeeds)
        release.Set()
        Assert.True(finished.Wait 10_000)
        Assert.Equal(0, r.TotalEdges)

    // ---- exceptions: F# code reaches the C# executor untouched --------------------------------------

    [<Fact>]
    member _.``an exception raised by an F# target is a crash with its own type as the kind``() =
        let r = campaign [ [| 0xffuy; 1uy; 2uy |] ] (opts 0) (fun _ _ -> raise (InvalidOperationException "boom"))
        Assert.Equal(1, r.Crashes.Length)
        let c = r.Crashes.Head
        Assert.Equal("System.InvalidOperationException", c.Kind)
        Assert.Matches("^[0-9a-f]{8}$", c.Signature)
        Assert.Equal(1, c.Hits)
        Assert.False r.IsClean
        Assert.Equal(1, r.UniqueCrashes)

    [<Fact>]
    member _.``failwith is a crash of kind System.Exception``() =
        let r = campaign [ [| 1uy |] ] (opts 0) (fun _ _ -> failwith "boom")
        Assert.Equal(1, r.Crashes.Length)
        Assert.Equal("System.Exception", r.Crashes.Head.Kind)

    [<Fact>]
    member _.``other F# exceptions are crashes of their own kinds``() =
        let kind (f: unit -> unit) = (campaign [ [| 1uy |] ] (opts 0) (fun _ _ -> f ())).Crashes.Head.Kind
        Assert.Equal("System.ArgumentException", kind (fun () -> invalidArg "x" "bad"))
        Assert.Equal("System.InvalidOperationException", kind (fun () -> invalidOp "bad"))
        Assert.Equal("System.NullReferenceException", kind (fun () -> (null: string).Length |> ignore))
        Assert.Equal("System.DivideByZeroException", kind (fun () -> (1 / (int (Environment.TickCount64 &&& 0L))) |> ignore))
        Assert.Equal("Microsoft.FSharp.Core.MatchFailureException", kind (fun () -> let (Some _) = (if Environment.TickCount64 > 0L then None else Some 1) in ()))

    [<Fact>]
    member _.``an exception inside async and task workflows is a crash``() =
        let kind (f: unit -> unit) = (campaign [ [| 1uy |] ] (opts 0) (fun _ _ -> f ())).Crashes.Head.Kind
        // Async.RunSynchronously rethrows the original exception
        Assert.Equal("System.Exception", kind (fun () -> Async.RunSynchronously(async { failwith "in async" })))
        Assert.Equal("System.InvalidOperationException", kind (fun () -> Async.RunSynchronously lateFailure))
        // task { } read with GetAwaiter().GetResult() unwraps too
        Assert.Equal("System.Exception", kind (fun () -> (task { failwith "in task" } : Task<unit>).GetAwaiter().GetResult()))
        Assert.Equal("System.Exception", kind (fun () -> Task.Run(fun () -> failwith "spawned").GetAwaiter().GetResult()))
        // .Wait() wraps in AggregateException, and the wrapper is then the kind
        Assert.Equal("System.AggregateException", kind (fun () -> Task.Run(fun () -> failwith "spawned").Wait()))
        Assert.Equal("System.AggregateException", kind (fun () -> (Async.StartAsTask(async { failwith "x" })).Wait()))

    [<Fact>]
    member _.``budgets cross async workflows driven synchronously``() =
        let t = once 2 100 (fun _ h -> Async.RunSynchronously(async { h.Tick(); h.Tick(); h.Tick() }))
        Assert.Equal((1, 0, 0), (t.Timeouts, t.Ooms, t.Crashes.Length))
        let m = once 100 10 (fun _ h -> Async.RunSynchronously(async { h.Reserve 11L }))
        Assert.Equal((0, 1, 0), (m.Timeouts, m.Ooms, m.Crashes.Length))
        let task = once 2 100 (fun _ h -> (task { h.Tick(); h.Tick(); h.Tick() } : Task<unit>).GetAwaiter().GetResult())
        Assert.Equal((1, 0, 0), (task.Timeouts, task.Ooms, task.Crashes.Length))

    [<Fact>]
    member _.``a workflow that swallows the sentinel is still stopped``() =
        let r = once 0 100 (fun _ h -> Async.RunSynchronously(async { try h.Tick() with _ -> () }))
        Assert.Equal(1, r.Timeouts)

    [<Fact>]
    member _.``crash frames name the site and end at the stable adapter``() =
        let r = campaign [ [| 1uy |] ] (opts 0) (fun d _ -> siteA d)
        let frames = r.Crashes.Head.Frames
        Assert.Equal("FuzzCampaignTests.siteA", frames.Head)
        Assert.Equal("TargetAdapter.Invoke", List.last frames)
        for f in frames do
            Assert.DoesNotContain(":", f)
        Assert.DoesNotContain("boom", string r)
        Assert.DoesNotContain("boom", string r.Crashes.Head)

    // ---- dedup -------------------------------------------------------------------------------------

    [<Fact>]
    member _.``one crash site is one record however many inputs hit it``() =
        let seeds = [ for i in 1 .. 6 -> [| byte i; 7uy |] ]
        let r = campaign seeds (opts 0) (fun d _ -> siteA d)
        Assert.Equal(1, r.Crashes.Length)
        Assert.True(r.Crashes.Head.Hits >= 6, sprintf "hits %d" r.Crashes.Head.Hits)
        Assert.Equal(1, r.UniqueCrashes)

    [<Fact>]
    member _.``two crash sites are two records``() =
        let target : FuzzTarget = fun d _ -> if d.[0] % 2uy = 0uy then siteA d else siteB d
        let r = campaign [ [| 1uy |]; [| 2uy |] ] (opts 0) target
        Assert.Equal(2, r.Crashes.Length)
        Assert.Equal(2, r.UniqueCrashes)
        let kinds = r.Crashes |> List.map (fun c -> c.Kind) |> List.sort
        Assert.Equal<string list>([ "System.Exception"; "System.InvalidOperationException" ], kinds)
        Assert.NotEqual<string>(r.Crashes.[0].Signature, r.Crashes.[1].Signature)

    [<Fact>]
    member _.``the minimal input still reproduces the signature and is private``() =
        let target : FuzzTarget = fun d _ -> if d.Length >= 4 && d.[0] = 0xdeuy && d.[1] = 0xaduy then siteA d
        let r = campaign [ [| 0xdeuy; 0xaduy; 0xbeuy; 0xefuy; 1uy; 2uy; 3uy; 4uy |] ] (opts 0) target
        let c = r.Crashes.Head
        let minimal = c.MinimalInput()
        Assert.True(minimal.Length >= 4 && minimal.Length <= 8)
        Assert.Equal(0xdeuy, minimal.[0])
        // a copy: scribbling on it does not change the record
        minimal.[0] <- 0uy
        Assert.Equal(0xdeuy, c.MinimalInput().[0])
        Assert.False c.InputDropped

    [<Fact>]
    member _.``a campaign is deterministic``() =
        let target : FuzzTarget = fun d h -> h.Edge 1UL; (if d.Length > 2 && d.[0] > 200uy then h.Edge 2UL); (if d.Length > 5 then h.Edge "x")
        let a = campaign [ [| 1uy; 2uy; 3uy |] ] (opts 200) target
        let b = campaign [ [| 1uy; 2uy; 3uy |] ] (opts 200) target
        Assert.Equal(string a, string b)
        Assert.Equal<byte[] list>(a.Corpus(), b.Corpus())

    // ---- the builder -----------------------------------------------------------------------------------

    [<Fact>]
    member _.``the builder runs a campaign and reaches every option``() =
        let maxSeen = ref 0
        let ticks = ref 0
        let r =
            bounded 60 (fun () ->
                fuzz {
                    seed 99UL
                    iterations 30
                    maxInputLen 6
                    baseEnergy 3
                    minimizeBudget 5
                    stepBudget 4
                    memBudget 50
                    seeds [ [| 1uy; 2uy |] ]
                    target (fun d h ->
                        maxSeen.Value <- max maxSeen.Value d.Length
                        h.Edge 1UL
                        h.Reserve 50L
                        for _ in 1 .. 4 do h.Tick()
                        ticks.Value <- ticks.Value + 1)
                })
        Assert.Equal(30, r.Iterations)
        Assert.True(maxSeen.Value <= 6, sprintf "longest input %d" maxSeen.Value)
        Assert.Equal(31, ticks.Value)
        Assert.Equal(0, r.Timeouts + r.Ooms)
        Assert.Equal(99UL, r.Seed)

    [<Fact>]
    member _.``the builder equals the explicit form``() =
        let tgt : FuzzTarget = fun d h -> h.Edge 1UL; (if d.Length > 1 && d.[0] > 100uy then h.Edge 2UL)
        let o =
            { MaxIterations = 40; Seed = 5UL; MaxInputLen = 12; BaseEnergy = 4; MinimizeBudget = 9; StepBudget = 77; MemBudget = 88 }
        let explicit = campaign [ [| 1uy; 200uy |] ] o tgt
        let viaOptions = bounded 60 (fun () -> fuzz { options o; seeds [ [| 1uy; 200uy |] ]; target tgt })
        let viaFields =
            bounded 60 (fun () ->
                fuzz {
                    seed 5UL
                    iterations 40
                    maxInputLen 12
                    baseEnergy 4
                    minimizeBudget 9
                    stepBudget 77
                    memBudget 88
                    seeds [ [| 1uy; 200uy |] ]
                    target tgt
                })
        Assert.Equal(string explicit, string viaOptions)
        Assert.Equal(string explicit, string viaFields)
        Assert.Equal<byte[] list>(explicit.Corpus(), viaFields.Corpus())
        Assert.Equal(5UL, viaFields.Seed)

    [<Fact>]
    member _.``the builder without a target is an error``() =
        let e = Assert.Throws<InvalidOperationException>(fun () -> fuzz { iterations 3 } |> ignore)
        Assert.Contains("no target", e.Message)

    [<Fact>]
    member _.``invalid options and oversized corpora are refused by the engine``() =
        Assert.Throws<ArgumentException>(fun () -> Campaign.run [] { opts 1 with MaxIterations = 1_000_001 } (fun _ _ -> ()) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Campaign.run [] { opts 1 with MemBudget = -1 } (fun _ _ -> ()) |> ignore) |> ignore
        let tooMany = [ for _ in 0 .. 10_000 -> [| 0uy |] ]
        Assert.Throws<ArgumentException>(fun () -> Campaign.run tooMany (opts 1) (fun _ _ -> ()) |> ignore) |> ignore
        let tooLong = [ Array.zeroCreate<byte> ((1 <<< 20) + 1) ]
        Assert.Throws<ArgumentException>(fun () -> Campaign.run tooLong (opts 1) (fun _ _ -> ()) |> ignore) |> ignore

    // ---- differential against the engine, with pinned numbers --------------------------------------

    [<Fact>]
    member _.``the report views are the engine's and match pinned numbers``() =
        let csTarget =
            CsTarget(fun d h ->
                h.Edge 1UL
                if d.Length > 0 && d.[0] > 127uy then h.Edge 2UL
                if d.Length > 1 && d.[1] = 0uy then h.Edge 3UL
                if d.Length > 3 && d.[0] = 0x41uy && d.[1] = 0x42uy then h.Edge(CsHandle.EdgeId "ab"))
        let fsTarget : FuzzTarget =
            fun d h ->
                h.Edge 1UL
                if d.Length > 0 && d.[0] > 127uy then h.Edge 2UL
                if d.Length > 1 && d.[1] = 0uy then h.Edge 3UL
                if d.Length > 3 && d.[0] = 0x41uy && d.[1] = 0x42uy then h.Edge "ab"
        let o = { opts 300 with Seed = 7UL }
        let seeds = [ [| 1uy; 2uy |]; [| 0x41uy; 0x42uy; 0uy; 0uy |] ]
        let engine =
            CsFuzzEngine.Run(System.Collections.Generic.List<byte[]>(seeds), o.ToEngine(), csTarget)
        let r = campaign seeds o fsTarget
        Assert.Equal(engine.Iterations, r.Iterations)
        Assert.Equal(engine.AdmittedSeeds, r.AdmittedSeeds)
        Assert.Equal(engine.TotalEdges, r.TotalEdges)
        Assert.Equal(engine.Timeouts, r.Timeouts)
        Assert.Equal(engine.Ooms, r.Ooms)
        Assert.Equal(engine.CrashesDropped, r.CrashesDropped)
        Assert.Equal(engine.AdmissionsDropped, r.AdmissionsDropped)
        Assert.Equal(engine.IsClean, r.IsClean)
        Assert.Equal(engine.UniqueCrashes, r.UniqueCrashes)
        Assert.Equal(engine.Seed, r.Seed)
        Assert.Equal(engine.Crashes.Count, r.Crashes.Length)
        Assert.Equal<byte[] list>(List.ofSeq (engine.GetCorpus()), r.Corpus())
        Assert.Equal(engine.ToString(), r.ToString())
        // pinned numbers (computed once from the C# engine; the same campaign must always give them)
        Assert.Equal(300, r.Iterations)
        Assert.Equal(3, r.AdmittedSeeds)
        Assert.Equal(4, r.TotalEdges)
        Assert.True r.IsClean

    [<Fact>]
    member _.``crash record views are the engine's``() =
        let csTarget = CsTarget(fun d _ -> if d.Length > 0 then raise (InvalidOperationException "x"))
        let engine = CsFuzzEngine.Run(System.Collections.Generic.List<byte[]>([ [| 1uy; 2uy; 3uy |] ]), (opts 0).ToEngine(), csTarget)
        let r = campaign [ [| 1uy; 2uy; 3uy |] ] (opts 0) (fun d _ -> if d.Length > 0 then raise (InvalidOperationException "x"))
        let e = engine.Crashes.[0]
        let c = r.Crashes.Head
        Assert.Equal(e.Kind, c.Kind)
        Assert.Equal(e.Hits, c.Hits)
        Assert.Equal(e.Minimized, c.Minimized)
        Assert.Equal(e.InputDropped, c.InputDropped)
        Assert.Equal<byte[]>(e.MinimalInput(), c.MinimalInput())
        Assert.Equal<string list>([ "TargetAdapter.Invoke" ], c.Frames |> List.skip (c.Frames.Length - 1))

// ---- TAP over a real report -------------------------------------------------------------------------

type TapReports() =
    inherit FuzzSeamTest()

    [<Fact>]
    member _.``tap failures render every bucket without a payload``() =
        let target : FuzzTarget =
            fun d h ->
                match d.[0] with
                | 1uy -> failwith "crash"
                | 2uy -> while true do h.Tick()
                | _ -> h.Reserve 1_000_000L
        let r = campaign [ [| 1uy; 0xdeuy; 0xaduy |]; [| 2uy |]; [| 3uy |] ] ({ opts 0 with StepBudget = 10; MemBudget = 100 }) target
        Assert.Equal((1, 1, 1), (r.Crashes.Length, r.Timeouts, r.Ooms))
        let lines = Tap.failures r "fz" 5
        Assert.Equal(3, lines.Length)
        Assert.Equal("not ok 5 - fz [fuzz-crash:" + r.Crashes.Head.Signature + "]", lines.[0])
        Assert.Equal("not ok 6 - fz [fuzz-timeout:4e61ea76]", lines.[1])
        Assert.Equal("not ok 7 - fz [fuzz-oom:7146a410]", lines.[2])
        for l in lines do
            Assert.DoesNotContain("dead", l)
            Assert.DoesNotContain("boom", l)
        // the first test number is honoured and the target is sanitised
        Assert.Equal("not ok 40 - a_b [fuzz-crash:" + r.Crashes.Head.Signature + "]", (Tap.failures r "a b" 40).[0])
        // a clean report has none
        Assert.Empty(Tap.failures (campaign [ [| 1uy |] ] (opts 5) (fun _ _ -> ())) "fz" 1)

// ---- privacy: no bytes, no seed ---------------------------------------------------------------------

type Privacy() =
    inherit FuzzSeamTest()

    let secretSeed = 987654321987UL

    let renderings (o: obj) : string list =
        [ sprintf "%A" o; sprintf "%O" o; sprintf "%s" (o.ToString()); sprintf "%A" [ o ]; sprintf "%A" (Some o); $"{o}"; String.Format("{0}", o) ]

    let assertNoSecrets (what: string) (texts: string list) =
        for t in texts do
            for needle in [ "987654321987"; "deadbeef"; "DEADBEEF"; "de-ad-be-ef"; "222uy"; "173uy"; "190uy"; "239uy"; "0xde" ] do
                Assert.False(t.Contains(needle, StringComparison.OrdinalIgnoreCase), what + " printed " + needle + ": " + t)

    [<Fact>]
    member _.``options print their limits and never the seed``() =
        let o = { FuzzOptions.defaults with Seed = secretSeed; MaxIterations = 123 }
        let texts = renderings (box o)
        assertNoSecrets "FuzzOptions" texts
        for t in texts do
            Assert.Contains("maxIterations=123", t)
        Assert.Equal("FuzzOptions{maxIterations=123 maxInputLen=4096 baseEnergy=10 minimizeBudget=2000 stepBudget=200000 memBudget=67108864}", string o)
        // the seed is still the caller's own field
        Assert.Equal(secretSeed, o.Seed)
        // a campaign spec prints nothing private either
        let spec = fuzz.Yield(())
        assertNoSecrets "CampaignSpec" (renderings (box spec))

    [<Fact>]
    member _.``report and crash record print counts, never bytes or the seed``() =
        let target : FuzzTarget = fun d h -> h.Edge 1UL; (if d.Length >= 4 && d.[0] = 0xdeuy && d.[1] = 0xaduy then siteA d)
        let r = campaign [ [| 0xdeuy; 0xaduy; 0xbeuy; 0xefuy |] ] ({ opts 0 with Seed = secretSeed }) target
        let c = r.Crashes.Head
        // the secrets exist: this is not vacuous
        Assert.Equal(secretSeed, r.Seed)
        Assert.Equal<byte[]>([| 0xdeuy; 0xaduy; 0xbeuy; 0xefuy |], c.MinimalInput() |> Array.truncate 4)
        assertNoSecrets "FuzzReport" (renderings (box r))
        assertNoSecrets "CrashRecord" (renderings (box c))
        assertNoSecrets "crash list" [ sprintf "%A" r.Crashes ]
        assertNoSecrets "tap" (Tap.failures r "t" 1)
        // the printed form is the engine's: counts and the signature
        Assert.Contains("crashes=1", string r)
        Assert.Contains("sig=" + c.Signature, string c)
        Assert.Contains("kind=System.Exception", string c)

    [<Fact>]
    member _.``a report with a corpus never prints the corpus``() =
        let r = campaign [ [| 0xdeuy; 0xaduy; 0xbeuy; 0xefuy |] ] ({ opts 20 with Seed = secretSeed }) (fun _ h -> h.Edge 1UL)
        Assert.Equal<byte[]>([| 0xdeuy; 0xaduy; 0xbeuy; 0xefuy |], r.Corpus().Head)
        assertNoSecrets "FuzzReport" (renderings (box r))

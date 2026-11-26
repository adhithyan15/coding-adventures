// ===========================================================================
// CodingAdventures.TestBootstrap.FSharp.Fuzzing: an F# face for the fuzz-v1 engine
// ===========================================================================
//
// This file is a BINDING, not an engine (VER06m).  The argument is the one of the
// PBT binding next to it (VER05l) and of Kotlin over Java (VER06g): F# compiles to
// the same CIL and runs on the same runtime as C#, so a second fuzz engine written
// in F# would share the runtime, `BigInteger`, the wrap-around `ulong` arithmetic
// and the CLR stack walk of the C# engine.  It would be a duplicate, not an
// independent witness, and two copies can only drift apart.  So:
//
//     F# code ──►  Campaign.run / fuzz { }  /  Mutate.* / Signature.* / Tap.*
//                          │ delegates
//                          ▼
//                     C# engine (CodingAdventures.TestBootstrap.Fuzz)
//                        owns every mutator, the coverage bitmap, admission,
//                        the energy schedule, signatures, minimisation, the
//                        budgets, async-void detection, TAP lines and every cap
//
// What this file may do:   rename, reshape and convert types (a `uint64` for a
//                          `ulong`, an F# `list` for a `List<string>`, a union for
//                          a C# enum) and choose idiomatic argument order.
// What it must NOT do:     decide a mutation, an admission, an energy, a signature,
//                          a dedup answer, a budget, a cap or a failure id.  A
//                          conversion has to be a bijection that forwards the
//                          engine's answer untouched.
//
// Exceptions.  Nothing here catches an exception thrown by user code.  An F#
// target is invoked from a small named class (`TargetAdapter`), reached from the
// engine through a delegate, so `failwith`, a `raise`, an exception inside a
// `task { }` or an `async { }` driven synchronously all arrive at the C#
// executor exactly as a C# lambda's throw would.
//
// Privacy.  A fuzz report may not print input bytes or the campaign seed
// (VER06 section 6).  F# records and unions generate structural `ToString`/`%A`,
// so `FuzzReport` and `CrashRecord` are CLASSES whose `ToString` is the engine's,
// and `FuzzOptions` (a record, so `with` works) carries a redacting
// `StructuredFormatDisplay` and `ToString`.

namespace CodingAdventures.TestBootstrap.FSharp.Fuzzing

open System
open System.Collections.Generic
open System.Runtime.CompilerServices

// ---------------------------------------------------------------------------
// Short names for the C# engine types
// ---------------------------------------------------------------------------
//
// The F# surface re-uses the C# names (`Handle`, `FuzzOptions`, ...), so the engine's
// types are only ever reached through these abbreviations; the C# namespace is never
// opened wholesale.

[<AutoOpen>]
module internal FuzzEngineTypes =
    type CsHandle = CodingAdventures.TestBootstrap.Fuzz.Handle
    type CsTarget = CodingAdventures.TestBootstrap.Fuzz.Target
    type CsFuzzOptions = CodingAdventures.TestBootstrap.Fuzz.FuzzOptions
    type CsFuzzReport = CodingAdventures.TestBootstrap.Fuzz.FuzzReport
    type CsCrashRecord = CodingAdventures.TestBootstrap.Fuzz.CrashRecord
    type CsFuzzEngine = CodingAdventures.TestBootstrap.Fuzz.FuzzEngine
    type CsMutate = CodingAdventures.TestBootstrap.Fuzz.Mutate
    type CsEndian = CodingAdventures.TestBootstrap.Fuzz.Endian
    type CsBitmap = CodingAdventures.TestBootstrap.Fuzz.Bitmap
    type CsSignature = CodingAdventures.TestBootstrap.Fuzz.FuzzSignature
    type CsFailureIds = CodingAdventures.TestBootstrap.Fuzz.FuzzFailureIds

// ---------------------------------------------------------------------------
// Sabotage seams (VER06m section 4)
// ---------------------------------------------------------------------------
//
// A conformance check that cannot fail is worse than none.  Each seam makes THIS
// layer wrong on purpose, so a test can require the corpus (or the pinned-value
// tests) to notice, which proves the run really goes through F#.  The module is
// internal and visible only to the test assembly (the `InternalsVisibleTo` of
// TestBootstrap.fs); production code never writes a seam, it only reads them.

/// <summary>Test-only sabotage switches for the F# fuzz layer. Production never writes them.</summary>
module internal FuzzSeams =
    /// <summary>Added to every offset a mutator wrapper hands to the engine.</summary>
    let mutable offsetDelta = 0I
    /// <summary>When true, multi-byte mutator wrappers flip the byte order.</summary>
    let mutable swapEndian = false
    /// <summary>When true, `Signature.isDuplicate` answers the opposite.</summary>
    let mutable invertDedup = false
    /// <summary>When true, `Tap.format` appends the target again.</summary>
    let mutable leakTapTarget = false
    /// <summary>When true, `Admission.admit` always reports a novelty.</summary>
    let mutable admitEverything = false
    /// <summary>When true, `FuzzFailureId.isValid` accepts everything.</summary>
    let mutable acceptAnyFailureId = false
    /// <summary>Added to the seed `Mutate.sequence` hands to the engine.</summary>
    let mutable seedOffset = 0UL
    /// <summary>Added to every `Energy.calculate` result.</summary>
    let mutable energyDelta = 0
    /// <summary>When true, `Signature.crash` drops its last frame.</summary>
    let mutable dropFrame = false

    /// <summary>Restores production behaviour.</summary>
    let reset () =
        offsetDelta <- 0I
        swapEndian <- false
        invertDedup <- false
        leakTapTarget <- false
        admitEverything <- false
        acceptAnyFailureId <- false
        seedOffset <- 0UL
        energyDelta <- 0
        dropFrame <- false

// ---------------------------------------------------------------------------
// Handle
// ---------------------------------------------------------------------------

/// <summary>
/// What a target sees: coverage reporting and two deterministic budgets. The campaign makes one
/// wrapper per execution and the engine retires its handle afterwards, so a task or workflow that
/// outlives the run can only touch a retired handle: <c>Edge</c> does nothing and <c>Tick</c> and
/// <c>Reserve</c> throw. A target must not retain it.
/// </summary>
[<Sealed>]
type Handle internal (engine: CsHandle) =
    /// <summary>Reports that the target executed the program point with this id (reduced mod 65,536).</summary>
    member _.Edge(id: uint64) : unit = engine.Edge id

    /// <summary>Reports a program point by name; the id is a stable FNV-1a hash of the label.</summary>
    member _.Edge(label: string) : unit = engine.Edge(CsHandle.EdgeId label)

    /// <summary>
    /// Counts one unit of work. After <c>StepBudget</c> ticks the execution is a timeout: this throws
    /// the engine's private sentinel and keeps throwing. A target that swallows it is still stopped
    /// by the engine's sticky exhaustion check (VER06h section 4.2).
    /// </summary>
    member _.Tick() : unit = engine.Tick()

    /// <summary>
    /// Declares an allocation of <c>n</c> bytes about to be made on behalf of the input; past
    /// <c>MemBudget</c> the execution is an out-of-memory outcome. A non-positive <c>n</c> reserves nothing.
    /// </summary>
    member _.Reserve(n: int64) : unit = engine.Reserve n

    /// <summary>A stable edge id derived from a name.</summary>
    static member EdgeId(label: string) : uint64 = CsHandle.EdgeId label

/// <summary>Curried forms of the <c>Handle</c> members, for pipelines.</summary>
[<RequireQualifiedAccess>]
module Handle =
    /// <summary>Reports an edge by id.</summary>
    let edge (h: Handle) (id: uint64) : unit = h.Edge id
    /// <summary>Reports an edge by name.</summary>
    let edgeLabel (h: Handle) (label: string) : unit = h.Edge label
    /// <summary>Counts one unit of work.</summary>
    let tick (h: Handle) : unit = h.Tick()
    /// <summary>Reserves memory.</summary>
    let reserve (h: Handle) (n: int64) : unit = h.Reserve n
    /// <summary>The stable edge id of a name.</summary>
    let edgeId (label: string) : uint64 = Handle.EdgeId label

/// <summary>The code under test: it gets its own copy of the input and a per-execution <c>Handle</c>.</summary>
type FuzzTarget = byte[] -> Handle -> unit

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// <summary>
/// Bounds a campaign; the engine validates every field against its ceiling (CWE-400). Derive variants
/// with <c>with</c>. Printing (<c>ToString</c>, <c>%A</c>, <c>%O</c>, interpolation) shows the limits
/// only, never the seed; structural equality and comparison are switched off so they cannot be used
/// as a seed oracle.
/// </summary>
[<NoEquality; NoComparison; StructuredFormatDisplay("{Redacted}")>]
type FuzzOptions =
    { /// <summary>Mutation executions, in [0, 1,000,000].</summary>
      MaxIterations: int
      /// <summary>The campaign seed (never printed).</summary>
      Seed: uint64
      /// <summary>The longest mutated candidate, in [0, 1 MiB].</summary>
      MaxInputLen: int
      /// <summary>The energy base, in [0, 1,000,000].</summary>
      BaseEnergy: int
      /// <summary>Executions spent minimising one crash, in [0, 100,000].</summary>
      MinimizeBudget: int
      /// <summary><c>Tick()</c> calls per execution, in [0, 100,000,000].</summary>
      StepBudget: int
      /// <summary><c>Reserve()</c> bytes per execution, in [0, 1 GiB].</summary>
      MemBudget: int }

    /// <summary>The options as the engine sees them.</summary>
    member internal o.ToEngine() : CsFuzzOptions =
        CsFuzzOptions(o.MaxIterations, o.Seed, o.MaxInputLen, o.BaseEnergy, o.MinimizeBudget, o.StepBudget, o.MemBudget)

    /// <summary>The limits only (the engine's own text): no seed.</summary>
    member o.Redacted : string = o.ToEngine().ToString()

    override o.ToString() = o.Redacted

[<RequireQualifiedAccess>]
module FuzzOptions =
    /// <summary>An engine options value as the F# record.</summary>
    let internal ofEngine (e: CsFuzzOptions) : FuzzOptions =
        { MaxIterations = e.MaxIterations
          Seed = e.Seed
          MaxInputLen = e.MaxInputLen
          BaseEnergy = e.BaseEnergy
          MinimizeBudget = e.MinimizeBudget
          StepBudget = e.StepBudget
          MemBudget = e.MemBudget }

    /// <summary>The engine's standard campaign: 1000 iterations, seed 42, inputs up to 4096 bytes.</summary>
    let defaults : FuzzOptions = ofEngine (CsFuzzOptions.Defaults())

    /// <summary>Ceiling of the step budget a target can observe.</summary>
    let maxStepBudget : int = CsFuzzOptions.MaxStepBudget

    /// <summary>Ceiling of the memory budget a target can observe.</summary>
    let maxMemBudget : int = CsFuzzOptions.MaxMemBudget

    /// <summary>Ceiling of the executions spent minimising one crash.</summary>
    let maxMinimizeBudget : int = CsFuzzOptions.MaxMinimizeBudget

    /// <summary>Checks every field against its range; raises <c>ArgumentException</c>.</summary>
    let validate (o: FuzzOptions) : unit = o.ToEngine().Validate()

// ---------------------------------------------------------------------------
// Report views
// ---------------------------------------------------------------------------

/// <summary>What coverage was actually collected; the binding never upgrades the label.</summary>
type CoverageTier =
    /// <summary>The target reported edges (<c>caller-reported</c>).</summary>
    | CallerReported
    /// <summary>No edge was collected, so there was no feedback (<c>black-box-uninstrumented</c>).</summary>
    | BlackBox

/// <summary>
/// One deduplicated crash. A class on purpose (see the file header): the minimal input is reachable
/// only through <c>MinimalInput ()</c>, never through <c>ToString</c> or <c>%A</c>.
/// </summary>
[<Sealed>]
type CrashRecord internal (engine: CsCrashRecord) =
    /// <summary>The 8-hex-digit signature.</summary>
    member _.Signature : string = engine.Signature
    /// <summary>The normalised crash kind (a type name).</summary>
    member _.Kind : string = engine.Kind
    /// <summary>The normalised frames, innermost first.</summary>
    member _.Frames : string list = List.ofSeq engine.Frames
    /// <summary>How many executions hit this signature.</summary>
    member _.Hits : int = engine.Hits
    /// <summary>False when minimisation was skipped or cut short by a budget.</summary>
    member _.Minimized : bool = engine.Minimized
    /// <summary>True when the retained-bytes cap forced the input to be dropped.</summary>
    member _.InputDropped : bool = engine.InputDropped
    /// <summary>A copy of the shortest input known to reproduce the signature. PRIVATE: never log it.</summary>
    member _.MinimalInput() : byte[] = engine.MinimalInput()
    /// <summary>Signature, kind, frames and counts; never input bytes.</summary>
    override _.ToString() = engine.ToString()

/// <summary>
/// What a campaign found. The seed and the corpus are PRIVATE: reachable only through the explicit
/// <c>Seed</c> and <c>Corpus ()</c> members, never through <c>ToString</c> or <c>%A</c>.
/// </summary>
[<Sealed>]
type FuzzReport internal (engine: CsFuzzReport) =
    let crashes = [ for c in engine.Crashes -> CrashRecord c ]

    /// <summary>The engine report behind this view.</summary>
    member internal _.Engine : CsFuzzReport = engine
    /// <summary>Mutation executions run.</summary>
    member _.Iterations : int = engine.Iterations
    /// <summary>Seeds in the final corpus.</summary>
    member _.AdmittedSeeds : int = engine.AdmittedSeeds
    /// <summary>Distinct covered edges.</summary>
    member _.TotalEdges : int = engine.TotalEdges
    /// <summary>One record per distinct signature.</summary>
    member _.Crashes : CrashRecord list = crashes
    /// <summary>Executions that exhausted their step budget.</summary>
    member _.Timeouts : int = engine.Timeouts
    /// <summary>Executions that exhausted their memory budget.</summary>
    member _.Ooms : int = engine.Ooms
    /// <summary>Novel signatures beyond the report's cap.</summary>
    member _.CrashesDropped : int = engine.CrashesDropped
    /// <summary>Corpus admissions refused by the retained-bytes cap.</summary>
    member _.AdmissionsDropped : int = engine.AdmissionsDropped
    /// <summary>What was actually collected.</summary>
    member _.CoverageTier : CoverageTier =
        if engine.CoverageTier = CsFuzzReport.TierReported then CallerReported else BlackBox
    /// <summary>Whether the campaign can be read as "found nothing" (the engine's definition).</summary>
    member _.IsClean : bool = engine.IsClean
    /// <summary>The number of distinct crash signatures seen.</summary>
    member _.UniqueCrashes : int = engine.UniqueCrashes
    /// <summary>The campaign seed. PRIVATE.</summary>
    member _.Seed : uint64 = engine.Seed
    /// <summary>Copies of the admitted seeds. PRIVATE, like <c>MinimalInput</c>.</summary>
    member _.Corpus() : byte[] list = List.ofSeq (engine.GetCorpus())
    /// <summary>Counts, never seeds or bytes.</summary>
    override _.ToString() = engine.ToString()

// ---------------------------------------------------------------------------
// Running a campaign
// ---------------------------------------------------------------------------

/// The one place an F# target meets the engine.  A NAMED class, reached through a
/// `Delegate.CreateDelegate`, so the stack frame of every crash is the stable
/// `TargetAdapter.Invoke` and not an F# closure named after a source line of this file.
/// It has no try/with: whatever the target throws reaches the C# executor.
[<Sealed; AllowNullLiteral>]
type internal TargetAdapter(target: FuzzTarget) =
    member _.Invoke(data: byte[], h: CsHandle) : unit = target data (Handle h)

[<RequireQualifiedAccess>]
module Campaign =
    let private toDelegate (target: FuzzTarget) : CsTarget =
        Delegate.CreateDelegate(typeof<CsTarget>, TargetAdapter target, "Invoke") :?> CsTarget

    /// <summary>
    /// Runs a bounded, deterministic coverage-guided campaign: triage every seed, then mutate. The same
    /// (corpus, options, target) gives the same report. Invalid options raise <c>ArgumentException</c>.
    /// </summary>
    let run (initialCorpus: byte[] list) (options: FuzzOptions) (target: FuzzTarget) : FuzzReport =
        let corpus = List<byte[]>(initialCorpus)
        CsFuzzEngine.Run(corpus, options.ToEngine(), toDelegate target) |> FuzzReport

/// <summary>The state a <c>fuzz { }</c> expression accumulates. Printing shows nothing private.</summary>
[<Sealed>]
type CampaignSpec internal (options: FuzzOptions, seeds: byte[] list, target: FuzzTarget option) =
    member internal _.Options = options
    member internal _.Seeds = seeds
    member internal _.Target = target
    override _.ToString() = "CampaignSpec{" + options.ToString() + " seeds=" + string seeds.Length + "}"

/// <summary>
/// The <c>fuzz { }</c> builder. Every operation takes the previous state and returns the next; the
/// expression evaluates to the <c>FuzzReport</c>.
/// </summary>
[<Sealed>]
type FuzzBuilder() =
    member _.Yield(_: unit) : CampaignSpec = CampaignSpec(FuzzOptions.defaults, [], None)

    member _.Run(spec: CampaignSpec) : FuzzReport =
        match spec.Target with
        | None -> invalidOp "fuzz { }: no target was given (add `target (fun data h -> ...)`)"
        | Some t -> Campaign.run spec.Seeds spec.Options t

    [<CustomOperation("seed")>]
    member _.Seed(s: CampaignSpec, seed: uint64) = CampaignSpec({ s.Options with Seed = seed }, s.Seeds, s.Target)

    [<CustomOperation("iterations")>]
    member _.Iterations(s: CampaignSpec, n: int) = CampaignSpec({ s.Options with MaxIterations = n }, s.Seeds, s.Target)

    [<CustomOperation("maxInputLen")>]
    member _.MaxInputLen(s: CampaignSpec, n: int) = CampaignSpec({ s.Options with MaxInputLen = n }, s.Seeds, s.Target)

    [<CustomOperation("baseEnergy")>]
    member _.BaseEnergy(s: CampaignSpec, n: int) = CampaignSpec({ s.Options with BaseEnergy = n }, s.Seeds, s.Target)

    [<CustomOperation("minimizeBudget")>]
    member _.MinimizeBudget(s: CampaignSpec, n: int) = CampaignSpec({ s.Options with MinimizeBudget = n }, s.Seeds, s.Target)

    [<CustomOperation("stepBudget")>]
    member _.StepBudget(s: CampaignSpec, n: int) = CampaignSpec({ s.Options with StepBudget = n }, s.Seeds, s.Target)

    [<CustomOperation("memBudget")>]
    member _.MemBudget(s: CampaignSpec, n: int) = CampaignSpec({ s.Options with MemBudget = n }, s.Seeds, s.Target)

    [<CustomOperation("options")>]
    member _.Options(s: CampaignSpec, o: FuzzOptions) = CampaignSpec(o, s.Seeds, s.Target)

    [<CustomOperation("seeds")>]
    member _.Seeds(s: CampaignSpec, corpus: byte[] list) = CampaignSpec(s.Options, corpus, s.Target)

    [<CustomOperation("target")>]
    member _.Target(s: CampaignSpec, t: FuzzTarget) = CampaignSpec(s.Options, s.Seeds, Some t)

[<AutoOpen>]
module FuzzExpression =
    /// <summary>
    /// <c>fuzz { seed 7UL; iterations 500; seeds [ [| 1uy |] ]; target (fun data h -> ...) }</c>
    /// builds and runs a campaign.
    /// </summary>
    let fuzz = FuzzBuilder()

// ---------------------------------------------------------------------------
// Mutators
// ---------------------------------------------------------------------------

/// <summary>Byte order of a multi-byte mutation.</summary>
type Endian =
    | Little
    | Big

/// <summary>
/// The deterministic mutators of VER06 section 3. Offsets, counts, deltas and values are
/// <c>bigint</c>: the shared corpus carries integers of up to 1,000 digits, and the engine's
/// big-integer entry points saturate and wrap exactly as the corpus pins. Every function returns a
/// fresh array.
/// </summary>
[<RequireQualifiedAccess>]
module Mutate =
    let private off (o: bigint) : bigint = o + FuzzSeams.offsetDelta

    let private endian (e: Endian) : CsEndian =
        let e = if FuzzSeams.swapEndian then (match e with Little -> Big | Big -> Little) else e
        match e with
        | Little -> CsEndian.Little
        | Big -> CsEndian.Big

    /// <summary>Flips <c>count</c> bits starting at <c>bitOffset</c> (LSB-first).</summary>
    let bitFlip (data: byte[]) (bitOffset: bigint) (count: bigint) : byte[] =
        CsMutate.BitFlipBig(data, off bitOffset, count)

    /// <summary>Complements <c>count</c> bytes starting at <c>byteOffset</c>.</summary>
    let byteFlip (data: byte[]) (byteOffset: bigint) (count: bigint) : byte[] =
        CsMutate.ByteFlipBig(data, off byteOffset, count)

    /// <summary>Adds <c>delta</c> (wrapping) to the <c>width</c>-byte integer at <c>byteOffset</c>.</summary>
    let byteArithmetic (data: byte[]) (byteOffset: bigint) (delta: bigint) (width: int) (e: Endian) : byte[] =
        CsMutate.ByteArithmeticBig(data, off byteOffset, delta, width, endian e)

    /// <summary>Overwrites the <c>width</c>-byte integer at <c>byteOffset</c> with <c>value</c>.</summary>
    let boundaryInsert (data: byte[]) (byteOffset: bigint) (width: int) (value: bigint) (e: Endian) : byte[] =
        CsMutate.BoundaryInsertBig(data, off byteOffset, width, value, endian e)

    /// <summary>Removes <c>length</c> bytes at <c>offset</c>.</summary>
    let blockDelete (data: byte[]) (offset: bigint) (length: bigint) : byte[] =
        CsMutate.BlockDeleteBig(data, off offset, length)

    /// <summary>Inserts <c>insert</c> at <c>offset</c>.</summary>
    let blockInsert (data: byte[]) (offset: bigint) (insert: byte[]) : byte[] =
        CsMutate.BlockInsertBig(data, off offset, insert)

    /// <summary>Copies <c>length</c> bytes from <c>srcOffset</c> and inserts them at <c>dstOffset</c>.</summary>
    let blockDuplicate (data: byte[]) (srcOffset: bigint) (length: bigint) (dstOffset: bigint) : byte[] =
        CsMutate.BlockDuplicateBig(data, off srcOffset, length, dstOffset)

    /// <summary>The first <c>targetSplit</c> bytes of <c>target</c> followed by <c>donor</c> from <c>donorSplit</c> on.</summary>
    let splice (target: byte[]) (donor: byte[]) (targetSplit: bigint) (donorSplit: bigint) : byte[] =
        CsMutate.SpliceBig(target, donor, off targetSplit, donorSplit)

    /// <summary>
    /// <c>steps</c> seeded mutation steps; the draw order is the cross-lane contract. Arguments over the
    /// engine's caps raise <c>ArgumentException</c>.
    /// </summary>
    let sequence (data: byte[]) (seed: uint64) (steps: int) (maxLen: int) : byte[] =
        CsMutate.MutateSequence(data, seed + FuzzSeams.seedOffset, steps, maxLen)

// ---------------------------------------------------------------------------
// Admission, energy, signatures, TAP, failure ids
// ---------------------------------------------------------------------------

/// <summary>Whether a trace holds an edge absent from the corpus, and how many unique novel edges.</summary>
type Admission = { Admitted: bool; NewEdges: int }

[<RequireQualifiedAccess>]
module Admission =
    /// <summary>The pure novelty check; ids are compared as full unsigned 64-bit words.</summary>
    let admit (existing: uint64 list) (trace: uint64 list) : Admission =
        let n = CsBitmap.CorpusAdmit(Array.ofList existing, Array.ofList trace)
        if FuzzSeams.admitEverything then
            { Admitted = true; NewEdges = max 1 n.NewEdges }
        else
            { Admitted = n.Admitted; NewEdges = n.NewEdges }

/// <summary>The mutation budget of a seed.</summary>
[<RequireQualifiedAccess>]
module Energy =
    /// <summary><c>max(1, round(base * (1 + new / total) / sqrt(len)))</c>, half away from zero.</summary>
    let calculate (baseEnergy: int) (seedLen: int) (newEdges: int) (totalEdges: int) : int =
        CsBitmap.CalculateEnergy(baseEnergy, seedLen, newEdges, totalEdges) + FuzzSeams.energyDelta

/// <summary>Crash signatures and deduplication.</summary>
[<RequireQualifiedAccess>]
module Signature =
    /// <summary>The low 32 bits of <c>fnv1a64(kind ":" frames joined by ":")</c> as 8 lowercase hex digits.</summary>
    let crash (kind: string) (frames: string list) : string =
        let frames = if FuzzSeams.dropFrame && not frames.IsEmpty then List.take (frames.Length - 1) frames else frames
        CsSignature.CrashSignature(kind, frames)

    /// <summary>Whether two crash reports share one signature.</summary>
    let isDuplicate (kindA: string) (framesA: string list) (kindB: string) (framesB: string list) : bool =
        let same = CsSignature.IsDuplicateCrash(kindA, framesA, kindB, framesB)
        if FuzzSeams.invertDedup then not same else same

/// <summary>The only public failure line of the fuzz lane.</summary>
[<RequireQualifiedAccess>]
module Tap =
    /// <summary>
    /// <c>not ok N - target [failure-id:hex8]</c>: no input bytes, seed, message or stack. Raises
    /// <c>ArgumentException</c> for an unregistered failure id or a sig hash that is not 8 lowercase hex digits.
    /// </summary>
    let format (testNum: int) (target: string) (failureId: string) (sigHash: string) : string =
        let line = CsSignature.FormatTapFailure(testNum, target, failureId, sigHash)
        if FuzzSeams.leakTapTarget then line + target else line

    /// <summary>One opaque line per distinct crash signature, then one for timeouts and one for OOMs.</summary>
    let failures (report: FuzzReport) (target: string) (firstTest: int) : string list =
        List.ofSeq (CsSignature.TapFailures(report.Engine, target, firstTest))

/// <summary>The closed vocabulary of fuzz failure ids.</summary>
[<RequireQualifiedAccess>]
module FuzzFailureId =
    /// <summary>A target crashed.</summary>
    [<Literal>]
    let crash = "fuzz-crash"
    /// <summary>A target exhausted its step budget.</summary>
    [<Literal>]
    let timeout = "fuzz-timeout"
    /// <summary>A target exhausted its memory budget.</summary>
    [<Literal>]
    let oom = "fuzz-oom"
    /// <summary>Registered; this engine cannot produce it.</summary>
    [<Literal>]
    let leak = "fuzz-leak"
    /// <summary>Registered; this engine cannot produce it.</summary>
    [<Literal>]
    let panic = "fuzz-panic"

    /// <summary>Every registered id, in registration order.</summary>
    let all : string list = List.ofSeq CsFailureIds.All

    /// <summary>Whether the candidate is one of the five registered ids.</summary>
    let isValid (candidate: string) : bool =
        FuzzSeams.acceptAnyFailureId || CsFailureIds.IsFuzzFailureId candidate

/// <summary>The strict TFV validator the <c>corpus_reject</c> op calls.</summary>
[<RequireQualifiedAccess>]
module FuzzTfv =
    /// <summary>Whether the text is a valid TFV document.</summary>
    let isValid (text: string) : bool = CodingAdventures.TestBootstrap.Tfv.TfvValidator.IsValid text

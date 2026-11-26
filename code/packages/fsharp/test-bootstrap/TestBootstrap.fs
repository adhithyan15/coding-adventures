// ===========================================================================
// CodingAdventures.TestBootstrap.FSharp: an F# face for the pbt-v1 engine
// ===========================================================================
//
// This file is a BINDING, not an engine (VER05l).  F# compiles to the same CIL
// as C# and runs on the same runtime, so a second engine written in F# would
// share the runtime, System.Numerics.BigInteger and the wrap-around `ulong`
// arithmetic of the C# engine.  It would be a duplicate, not an independent
// witness: two copies that can only drift apart.  The repository has already
// paid for that once (the TypeScript lane's two engines, WD-901).  So:
//
//        F# code ──►  Gen<'T> / Prop.check   (this file: types and sugar)
//                          │ delegates
//                          ▼
//                     C# engine               (owns every draw, shrink move,
//                                              step count, budget and cap)
//
// What this file may do:   rename, reshape and convert types (an F# tuple for a
//                          C# `Pair`, an F# list for a `List<T>`), and choose
//                          idiomatic argument order.
// What it must NOT do:     decide a draw, a shrink move, a step count, a failure
//                          id, a budget or a cap.  A conversion has to be a
//                          bijection that forwards the engine's answer untouched.
//
// Exceptions.  Nothing here catches an exception thrown by user code.  An F#
// function handed to the engine is wrapped in a `System.Func`, and the engine
// invokes it inside its own try/catch, so a throw from a sampler, a property, a
// filter predicate or the body of a `gen { }` becomes exactly the verdict a C#
// lambda's throw would (VER05l section 3).

namespace CodingAdventures.TestBootstrap.FSharp

open System
open System.Numerics
open System.Runtime.CompilerServices
open CodingAdventures.TestBootstrap.Prng

// Only the test assembly may reach the internal sabotage seams (see below).
[<assembly: InternalsVisibleTo("CodingAdventures.TestBootstrap.FSharp.Tests")>]
do ()

// ---------------------------------------------------------------------------
// Short names for the C# engine types
// ---------------------------------------------------------------------------
//
// The F# surface re-uses the C# names (`Gen`, `Shrink`, `Options`, ...), so the
// engine's types are referred to through abbreviations and the C# namespace is
// never opened wholesale.

[<AutoOpen>]
module internal EngineTypes =
    type CsGen<'T> = CodingAdventures.TestBootstrap.Pbt.Gen<'T>
    type CsGens = CodingAdventures.TestBootstrap.Pbt.Gens
    type CsShrink = CodingAdventures.TestBootstrap.Pbt.Shrink
    type CsRunner = CodingAdventures.TestBootstrap.Pbt.Runner
    type CsShrunk<'T> = CodingAdventures.TestBootstrap.Pbt.Shrunk<'T>
    type CsShrinker<'T> = CodingAdventures.TestBootstrap.Pbt.Shrinker<'T>
    type CsOptions = CodingAdventures.TestBootstrap.Pbt.Options
    type CsCiMode = CodingAdventures.TestBootstrap.Pbt.CiMode
    type CsVerdict = CodingAdventures.TestBootstrap.Pbt.Verdict
    type CsFailureIds = CodingAdventures.TestBootstrap.Pbt.FailureIds
    type CsArm<'T> = CodingAdventures.TestBootstrap.Pbt.Arm<'T>
    type CsPair<'A, 'B> = CodingAdventures.TestBootstrap.Pbt.Pair<'A, 'B>
    type CsTriple<'A, 'B, 'C> = CodingAdventures.TestBootstrap.Pbt.Triple<'A, 'B, 'C>

// ---------------------------------------------------------------------------
// Sabotage seams (VER05l section 6)
// ---------------------------------------------------------------------------
//
// A conformance check that cannot fail is worse than none.  The seams let a test
// make THIS layer wrong on purpose (shift the seed, drop a shrink step, swap
// bounds) and require the corpus run to notice, which proves the corpus really
// goes through F# and not around it.  The module is internal and visible only to
// the test assembly (InternalsVisibleTo in the fsproj); production code never
// writes a seam, it only reads them.

/// <summary>Test-only sabotage switches for the F# layer. Production never writes them.</summary>
module internal Seams =
    /// <summary>Added to every seed the F# layer hands to the engine.</summary>
    let mutable seedOffset = 0UL

    /// <summary>Added to every shrink step count the F# layer reports.</summary>
    let mutable shrinkStepDelta = 0

    /// <summary>When true, `Generate.bounded` and `Shrink.integer` swap min and max.</summary>
    let mutable swapBounds = false

    /// <summary>Restores production behaviour.</summary>
    let reset () =
        seedOffset <- 0UL
        shrinkStepDelta <- 0
        swapBounds <- false

    /// <summary>The seed the engine actually receives (wraps modulo 2^64 like the engine's own arithmetic).</summary>
    let seed (s: uint64) : uint64 = s + seedOffset

    /// <summary>The (lo, hi) the engine actually receives.</summary>
    let bounds (lo: 'a) (hi: 'a) : 'a * 'a = if swapBounds then (hi, lo) else (lo, hi)

// ---------------------------------------------------------------------------
// Results of shrinking
// ---------------------------------------------------------------------------

/// <summary>The smallest value found that still falsifies, and how many accepted moves it took.</summary>
type Shrunk<'T> = { Value: 'T; Steps: int }

/// <summary>Why a shrinker refused its start value.</summary>
type ShrinkError =
    /// <summary>The start value satisfies the property, so there is nothing to shrink.</summary>
    | NotFalsifying

/// <summary>The first failing draw of a replay and the value it shrinks to.</summary>
type Replay = { InitialFail: bigint; Minimal: bigint }

/// <summary>
/// A custom shrinker: <c>shrinker value falsifies maxSteps</c>. <c>falsifies v</c> is true
/// while <c>v</c> still makes the property fail.
/// </summary>
type ShrinkFn<'T> = 'T -> ('T -> bool) -> int -> Shrunk<'T>

module internal Convert =
    /// <summary>An engine shrink result as an F# record, with the sabotage seam applied.</summary>
    let shrunk (r: CsShrunk<'T>) : Shrunk<'T> =
        { Value = r.Value; Steps = r.Steps + Seams.shrinkStepDelta }

    /// <summary>An F# shrinker as an engine shrinker (a plain forwarding wrapper).</summary>
    let shrinker (f: ShrinkFn<'T>) : CsShrinker<'T> =
        CsShrinker<'T>(fun value falsifies maxSteps ->
            let r = f value (fun v -> falsifies.Invoke v) maxSteps
            CsShrunk<'T>(r.Value, r.Steps))

// ---------------------------------------------------------------------------
// Generators
// ---------------------------------------------------------------------------

/// <summary>
/// A generator of <c>'T</c>: it draws from a PRNG stream and knows how to shrink what it
/// drew. An opaque wrapper over the C# engine's generator; build one with the
/// <c>Gen</c> module or the <c>gen { }</c> expression.
/// </summary>
[<Sealed>]
type Gen<'T> internal (engine: CsGen<'T>) =
    /// <summary>The wrapped engine generator.</summary>
    member internal _.Engine = engine

[<RequireQualifiedAccess>]
module Gen =

    let internal ofEngine (g: CsGen<'T>) : Gen<'T> = Gen<'T>(g)

    /// <summary>
    /// Re-expresses an engine generator over another type through a bijection. Sampling maps the
    /// drawn value; shrinking maps the F# value back, runs the engine's own shrinker, and maps the
    /// result forward. The bijection must not change what the shrinker does.
    /// </summary>
    let internal biject (toFs: 'C -> 'F) (toCs: 'F -> 'C) (inner: CsGen<'C>) : CsGen<'F> =
        CsGen<'F>.Of(
            Func<SplitMix64, 'F>(fun rng -> toFs (inner.Sample rng)),
            CsShrinker<'F>(fun value falsifies maxSteps ->
                let r = inner.Shrink(toCs value, Func<'C, bool>(fun c -> falsifies.Invoke(toFs c)), maxSteps)
                CsShrunk<'F>(toFs r.Value, r.Steps)))

    /// <summary>Builds a generator from an F# sampler over the engine PRNG and an optional shrinker.</summary>
    let custom (sampler: SplitMix64 -> 'T) (shrinker: ShrinkFn<'T> option) : Gen<'T> =
        let cs =
            match shrinker with
            | Some f -> Convert.shrinker f
            | None -> Unchecked.defaultof<CsShrinker<'T>>
        ofEngine (CsGen<'T>.Of(Func<SplitMix64, 'T>(sampler), cs))

    /// <summary>Draws one value.</summary>
    let sample (rng: SplitMix64) (g: Gen<'T>) : 'T = g.Engine.Sample rng

    /// <summary>
    /// Shrinks <c>value</c> while <c>falsifies</c> holds. A public entry point of the engine: it opens
    /// a child of the ambient shrink budget with its own cap.
    /// </summary>
    let shrink (value: 'T) (falsifies: 'T -> bool) (maxSteps: int) (g: Gen<'T>) : Shrunk<'T> =
        Convert.shrunk (g.Engine.Shrink(value, Func<'T, bool>(falsifies), maxSteps))

    /// <summary>Always generates <c>v</c> and draws nothing.</summary>
    let constant (v: 'T) : Gen<'T> = ofEngine (CsGens.Constant v)

    /// <summary>Transforms generated values. Drops shrinking: <c>f</c> has no inverse.</summary>
    let map (f: 'T -> 'U) (g: Gen<'T>) : Gen<'U> = ofEngine (CsGens.Map(g.Engine, Func<'T, 'U>(f)))

    /// <summary>Keeps values satisfying <c>pred</c>, giving up after 100 rejected samples.</summary>
    let filter (pred: 'T -> bool) (g: Gen<'T>) : Gen<'T> =
        ofEngine (CsGens.Filter(g.Engine, Func<'T, bool>(pred)))

    /// <summary>
    /// Keeps values satisfying <c>pred</c>, giving up after exactly <c>maxRetries</c> rejected
    /// samples (the sample then fails with <c>property-exhausted</c>). Shrinking is preserved.
    /// </summary>
    let filterWith (maxRetries: int) (pred: 'T -> bool) (g: Gen<'T>) : Gen<'T> =
        ofEngine (CsGens.Filter(g.Engine, Func<'T, bool>(pred), maxRetries))

    /// <summary>Dependent generation: sample, then sample from <c>f value</c>. No shrinking.</summary>
    let bind (f: 'T -> Gen<'U>) (g: Gen<'T>) : Gen<'U> =
        ofEngine (CsGens.FlatMap(g.Engine, Func<'T, CsGen<'U>>(fun v -> (f v).Engine)))

    /// <summary>Defers building a generator until it is sampled, so a throw while building it is a generator error.</summary>
    let delay (f: unit -> Gen<'T>) : Gen<'T> =
        ofEngine (CsGen<'T>.Of(Func<SplitMix64, 'T>(fun rng -> (f ()).Engine.Sample rng), Unchecked.defaultof<CsShrinker<'T>>))

    /// <summary>Picks one generator uniformly. The list must not be empty.</summary>
    let oneOf (gens: Gen<'T> list) : Gen<'T> =
        ofEngine (CsGens.OneOf(gens |> List.map (fun g -> g.Engine) |> Array.ofList))

    /// <summary>
    /// A weighted choice from <c>(weight, generator)</c> pairs. A zero weight is never picked; an
    /// empty list, a negative weight, a zero total or a total above 2^32 is rejected.
    /// </summary>
    let frequency (arms: (int64 * Gen<'T>) list) : Gen<'T> =
        ofEngine (CsGens.Frequency(arms |> List.map (fun (w, g) -> CsArm<'T>(w, g.Engine)) |> Array.ofList))

    /// <summary>A list of between <c>minLen</c> and <c>maxLen</c> elements; shrinks with the element shrinker.</summary>
    let list (minLen: int) (maxLen: int) (elem: Gen<'T>) : Gen<'T list> =
        ofEngine (biject List.ofSeq (fun (l: 'T list) -> System.Collections.Generic.List<'T>(l)) (CsGens.ListOf(elem.Engine, minLen, maxLen)))

    /// <summary>A pair; shrinks one coordinate at a time through the engine's tuple shrinker.</summary>
    let tuple (a: Gen<'A>) (b: Gen<'B>) : Gen<'A * 'B> =
        ofEngine (
            biject
                (fun (p: CsPair<'A, 'B>) -> (p.First, p.Second))
                (fun (x, y) -> CsPair<'A, 'B>(x, y))
                (CsGens.Tuple2(a.Engine, b.Engine))
        )

    /// <summary>A triple; shrinks one coordinate at a time through the engine's tuple shrinker.</summary>
    let tuple3 (a: Gen<'A>) (b: Gen<'B>) (c: Gen<'C>) : Gen<'A * 'B * 'C> =
        ofEngine (
            biject
                (fun (t: CsTriple<'A, 'B, 'C>) -> (t.First, t.Second, t.Third))
                (fun (x, y, z) -> CsTriple<'A, 'B, 'C>(x, y, z))
                (CsGens.Tuple3(a.Engine, b.Engine, c.Engine))
        )

    /// <summary>A boolean: <c>Bounded(2)</c>.</summary>
    let bool : Gen<bool> = ofEngine (CsGens.Bool())

    /// <summary>[0, 255].</summary>
    let u8 : Gen<byte> = ofEngine (CsGens.U8())

    /// <summary>A full 64-bit word.</summary>
    let u64 : Gen<uint64> = ofEngine (CsGens.U64())

    /// <summary>A signed 128-bit integer from two stream words.</summary>
    let i128 : Gen<bigint> = ofEngine (CsGens.I128())

    /// <summary>A uniform integer over the inclusive range [lo, hi], any width.</summary>
    let integer (lo: bigint) (hi: bigint) : Gen<bigint> = ofEngine (CsGens.Integer(lo, hi))

    /// <summary>A uniform 64-bit integer over the inclusive range [lo, hi].</summary>
    let int64 (lo: int64) (hi: int64) : Gen<int64> = ofEngine (CsGens.Int64(lo, hi))

    /// <summary>A byte string of between <c>minLen</c> and <c>maxLen</c> bytes.</summary>
    let bytes (minLen: int) (maxLen: int) : Gen<byte[]> = ofEngine (CsGens.Bytes(minLen, maxLen))

// ---------------------------------------------------------------------------
// The gen { } computation expression
// ---------------------------------------------------------------------------
//
//     gen {
//         let! n = Gen.integer 1I 10I
//         let! xs = Gen.list 0 5 Gen.u8
//         return (n, xs)
//     }
//
// desugars to nested `Gen.bind` calls, so it inherits bind's semantics: no
// shrinking (a bind has no inverse) and one draw per `let!`, in order.  The whole
// body is wrapped in `Delay`, so even code before the first `let!` runs at SAMPLE
// time, inside the engine's try/catch: a throw anywhere in a `gen { }` is a
// `property-exhausted` verdict, never an exception escaping `Prop.check`.

/// <summary>The builder behind <c>gen { }</c>.</summary>
[<Sealed>]
type GenBuilder() =
    member _.Bind(g: Gen<'T>, f: 'T -> Gen<'U>) : Gen<'U> = Gen.bind f g
    member _.Return(x: 'T) : Gen<'T> = Gen.constant x
    member _.ReturnFrom(g: Gen<'T>) : Gen<'T> = g
    member _.Zero() : Gen<unit> = Gen.constant ()
    member _.Delay(f: unit -> Gen<'T>) : Gen<'T> = Gen.delay f
    member _.Combine(first: Gen<unit>, second: Gen<'T>) : Gen<'T> = Gen.bind (fun () -> second) first

[<AutoOpen>]
module GenExpression =
    /// <summary>The generator computation expression.</summary>
    let gen = GenBuilder()

// ---------------------------------------------------------------------------
// Options and verdicts
// ---------------------------------------------------------------------------

/// <summary>
/// Whether output is public CI output. <c>Unset</c>, the default, redacts: the engine fails
/// closed, because a seed plus a property is a working exploit reproducer.
/// </summary>
type CiMode =
    /// <summary>Treated as CI: witnesses and seeds are redacted.</summary>
    | Unset
    /// <summary>Redacts.</summary>
    | On
    /// <summary>Local development: messages carry the seed and the witness.</summary>
    | Off

/// <summary>Configures <c>Prop.check</c>.</summary>
type Options =
    { /// <summary>How many cases to run. 0 is legal and is a failure: a property that never ran proves nothing.</summary>
      NumTests: int
      /// <summary>The shrink budget, 0 to 10,000.</summary>
      MaxShrinkSteps: int
      /// <summary>Redaction mode.</summary>
      Ci: CiMode }

[<RequireQualifiedAccess>]
module Options =
    /// <summary>100 cases, 1000 shrink steps and CI redaction.</summary>
    let defaults = { NumTests = 100; MaxShrinkSteps = 1000; Ci = Unset }

    /// <summary>The same options with another CI mode.</summary>
    let withCi (mode: CiMode) (o: Options) : Options = { o with Ci = mode }

    /// <summary>
    /// Pure CI detection: true when <c>CI</c> is set to anything but empty, "0" or "false". Pass a
    /// lookup over the environment; this package reads no environment itself.
    /// </summary>
    let ciFromLookup (lookup: string -> string option) : bool =
        CsOptions.CiFromLookup(Func<string, string>(fun name -> lookup name |> Option.toObj))

    let internal toEngine (o: Options) : CsOptions =
        let mode =
            match o.Ci with
            | Unset -> CsCiMode.Unset
            | On -> CsCiMode.On
            | Off -> CsCiMode.Off
        CsOptions(o.NumTests, o.MaxShrinkSteps, mode)

/// <summary>
/// The closed failure vocabulary (VER05 section 5.1): the four ids a failing verdict may carry.
/// </summary>
[<RequireQualifiedAccess>]
module FailureId =
    /// <summary>The property was falsified; a minimal witness was found.</summary>
    let counterexample = CsFailureIds.Counterexample

    /// <summary>Nothing (or no valid input) was evaluated.</summary>
    let exhausted = CsFailureIds.Exhausted

    /// <summary>Registered but never produced by this lane.</summary>
    let timeout = CsFailureIds.Timeout

    /// <summary>The property itself threw.</summary>
    let panic = CsFailureIds.Panic

    /// <summary>The closed set, in registration order.</summary>
    let all : string list = CsFailureIds.All |> List.ofSeq

    /// <summary>True only for one of the four registered ids.</summary>
    let isValid (candidate: string) : bool = CsFailureIds.IsPbtFailureId candidate

    /// <summary>The id itself when registered, else None.</summary>
    let tryParse (candidate: string) : string option =
        if isValid candidate then Some candidate else None

/// <summary>
/// The details of a failing verdict. In a redacted verdict (<c>Ci</c> is not <c>Off</c>)
/// <c>Seed</c>, <c>Counterexample</c> and <c>Minimal</c> are <c>None</c>: only the message,
/// iteration, step count and digest remain.
/// </summary>
type Failure<'T> =
    { FailureId: string
      Message: string
      CasesRun: int
      Iteration: int
      ShrinkSteps: int
      Seed: uint64 option
      Counterexample: 'T option
      Minimal: 'T option
      Digest: string option }

/// <summary>The outcome of <c>Prop.check</c>.</summary>
type Verdict<'T> =
    /// <summary>Every case held; carries the number of cases run.</summary>
    | Passed of casesRun: int
    /// <summary>The property was falsified (<c>property-counterexample</c>) or threw (<c>property-panic</c>).</summary>
    | Failed of Failure<'T>
    /// <summary>Nothing valid was evaluated (<c>property-exhausted</c>): never a pass.</summary>
    | Exhausted of Failure<'T>

[<RequireQualifiedAccess>]
module Verdict =
    let private unboxWitness (o: obj) : 'T option =
        match o with
        | null -> None
        | v -> Some(unbox<'T> v)

    /// <summary>An engine verdict as the F# union.</summary>
    let internal ofEngine<'T> (v: CsVerdict) : Verdict<'T> =
        if v.Passed then
            Passed v.CasesRun
        else
            let failure : Failure<'T> =
                { FailureId = v.FailureId
                  Message = v.Message
                  CasesRun = v.CasesRun
                  Iteration = v.Iteration
                  ShrinkSteps = v.ShrinkSteps
                  Seed = Option.ofNullable v.Seed
                  Counterexample = unboxWitness v.Counterexample
                  Minimal = unboxWitness v.Minimal
                  Digest = Option.ofObj v.Digest }
            if v.FailureId = FailureId.exhausted then Exhausted failure else Failed failure

    /// <summary>True for <c>Passed</c>.</summary>
    let isPassed (v: Verdict<'T>) : bool =
        match v with
        | Passed _ -> true
        | _ -> false

    /// <summary>The failure of a non-passing verdict.</summary>
    let tryFailure (v: Verdict<'T>) : Failure<'T> option =
        match v with
        | Passed _ -> None
        | Failed f
        | Exhausted f -> Some f

// ---------------------------------------------------------------------------
// Properties
// ---------------------------------------------------------------------------

[<RequireQualifiedAccess>]
module Prop =

    /// <summary>The seed <c>forAll</c> uses.</summary>
    let defaultSeed : uint64 = 0x5EEDUL

    /// <summary>
    /// Runs <c>prop</c> over <c>options.NumTests</c> candidates drawn from <c>seed</c>.
    /// <c>prop</c> returns <c>true</c> when the property HOLDS. A throw from <c>prop</c> is a
    /// <c>property-panic</c> verdict; a throw from the generator is <c>property-exhausted</c>.
    /// Invalid options raise <c>ArgumentException</c> (no verdict is produced).
    /// </summary>
    /// <remarks>
    /// A direct call to the engine's runner: a <c>Prop.check</c> made inside a property joins the
    /// ambient shrink budget of the outermost check and is charged to it (VER05l section 5).
    /// </remarks>
    let check (seed: uint64) (options: Options) (gen: Gen<'T>) (prop: 'T -> bool) : Verdict<'T> =
        CsRunner.Check<'T>(Seams.seed seed, gen.Engine, Func<'T, bool>(prop), Options.toEngine options)
        |> Verdict.ofEngine

    /// <summary><c>check</c> with an explicit shrinker that overrides the generator's own.</summary>
    let checkWithShrinker (seed: uint64) (options: Options) (shrinker: ShrinkFn<'T>) (gen: Gen<'T>) (prop: 'T -> bool) : Verdict<'T> =
        CsRunner.Check<'T>(Seams.seed seed, gen.Engine, Func<'T, bool>(prop), Options.toEngine options, Convert.shrinker shrinker)
        |> Verdict.ofEngine

    /// <summary><c>check</c> with <c>defaultSeed</c> and <c>Options.defaults</c>.</summary>
    let forAll (gen: Gen<'T>) (prop: 'T -> bool) : Verdict<'T> =
        check defaultSeed Options.defaults gen prop

    /// <summary>The verdict as a <c>Result</c>: the case count when it passed, the failure otherwise.</summary>
    let toResult (v: Verdict<'T>) : Result<int, Failure<'T>> =
        match v with
        | Passed n -> Ok n
        | Failed f
        | Exhausted f -> Error f

    /// <summary>
    /// Raises <c>PropertyFailedException</c> carrying the failure id unless the verdict passed, so a
    /// property can be used as an ordinary test assertion.
    /// </summary>
    let assertPassed (v: Verdict<'T>) : unit =
        match v with
        | Passed _ -> ()
        | Failed f
        | Exhausted f -> raise (CodingAdventures.TestBootstrap.Pbt.PropertyFailedException(f.FailureId, f.Message))

// ---------------------------------------------------------------------------
// Standalone shrinkers (the corpus's shrink_* operations)
// ---------------------------------------------------------------------------

/// <summary>
/// Shrinkers over concrete values. <c>holds</c> returns <c>true</c> when the property holds, so a
/// value falsifies when <c>holds</c> is false. A start value that satisfies the property is
/// <c>Error NotFalsifying</c>; every other misuse raises <c>ArgumentException</c> as in the engine.
/// </summary>
[<RequireQualifiedAccess>]
module Shrink =

    let private guard (f: unit -> CsShrunk<'T>) : Result<Shrunk<'T>, ShrinkError> =
        try
            Ok(Convert.shrunk (f ()))
        with :? CodingAdventures.TestBootstrap.Pbt.NotFalsifyingException ->
            Error NotFalsifying

    /// <summary>Shrinks an integer toward <c>target</c> inside [lo, hi].</summary>
    let integer (initial: bigint) (target: bigint) (lo: bigint) (hi: bigint) (holds: bigint -> bool) (maxSteps: int) : Result<Shrunk<bigint>, ShrinkError> =
        let (lo, hi) = Seams.bounds lo hi
        guard (fun () -> CsShrink.ShrinkInt(initial, target, lo, hi, Func<bigint, bool>(holds), maxSteps))

    /// <summary>Shrinks a list of integers: empty probe, halves, single deletion, then each element toward <c>elemTarget</c>.</summary>
    let list (initial: bigint list) (elemTarget: bigint) (holds: bigint list -> bool) (maxSteps: int) : Result<Shrunk<bigint list>, ShrinkError> =
        let cs = System.Collections.Generic.List<bigint>(initial)
        let run () =
            CsShrink.ShrinkList(cs, elemTarget, Func<System.Collections.Generic.List<bigint>, bool>(fun l -> holds (List.ofSeq l)), maxSteps)
        match guard run with
        | Ok r -> Ok { Value = List.ofSeq r.Value; Steps = r.Steps }
        | Error e -> Error e

    /// <summary>Shrinks a byte string.</summary>
    let bytes (initial: byte[]) (holds: byte[] -> bool) (maxSteps: int) : Result<Shrunk<byte[]>, ShrinkError> =
        guard (fun () -> CsShrink.ShrinkBytes(initial, Func<byte[], bool>(holds), maxSteps))

    /// <summary>
    /// Replays a failure from nothing but <c>seed</c>: draws up to 1000 values <c>Bounded(1000)</c>,
    /// takes the first that falsifies <c>holds</c> and shrinks it toward 0 over [0, 1000]. Raises
    /// <c>ArgumentException</c> if no draw falsified.
    /// </summary>
    let simulateReplay (seed: uint64) (holds: bigint -> bool) : Replay =
        let r = CsShrink.SimulateReplay(Seams.seed seed, Func<bigint, bool>(holds))
        { InitialFail = r.InitialFail; Minimal = r.Minimal }

// ---------------------------------------------------------------------------
// Standalone generation (the corpus's gen_* operations)
// ---------------------------------------------------------------------------

/// <summary><c>count</c> values drawn straight from a seed, for conformance and reproduction.</summary>
[<RequireQualifiedAccess>]
module Generate =

    let private primitive (seed: uint64) (name: string) (count: int) : obj list =
        CsGens.GenPrimitive(Seams.seed seed, name, count) |> List.ofSeq

    /// <summary><c>count</c> booleans.</summary>
    let bools (seed: uint64) (count: int) : bool list = primitive seed "bool" count |> List.map unbox<bool>

    /// <summary><c>count</c> bytes (u8).</summary>
    let u8s (seed: uint64) (count: int) : byte list = primitive seed "u8" count |> List.map (unbox<bigint> >> byte)

    /// <summary><c>count</c> 64-bit words (u64).</summary>
    let u64s (seed: uint64) (count: int) : uint64 list = primitive seed "u64" count |> List.map (unbox<bigint> >> uint64)

    /// <summary><c>count</c> signed 128-bit integers.</summary>
    let i128s (seed: uint64) (count: int) : bigint list = primitive seed "i128" count |> List.map unbox<bigint>

    /// <summary><c>count</c> uniform integers over [lo, hi].</summary>
    let bounded (seed: uint64) (lo: bigint) (hi: bigint) (count: int) : bigint list =
        let (lo, hi) = Seams.bounds lo hi
        CsGens.GenBounded(Seams.seed seed, lo, hi, count) |> List.ofSeq

    /// <summary><c>count</c> byte strings of length in [minLen, maxLen].</summary>
    let bytes (seed: uint64) (minLen: int) (maxLen: int) (count: int) : byte[] list =
        CsGens.GenBytes(Seams.seed seed, minLen, maxLen, count) |> List.ofSeq

// ---------------------------------------------------------------------------
// Seeds
// ---------------------------------------------------------------------------

/// <summary>Seed derivation.</summary>
[<RequireQualifiedAccess>]
module Seed =
    /// <summary>
    /// A sub-seed for the named property: <c>SplitMix64(runSeed XOR fnv1a64(utf8 id)).NextU64()</c>,
    /// so every property in one run gets an independent stream.
    /// </summary>
    let sub (runSeed: uint64) (id: string) : uint64 = SplitMix64.SubSeed(runSeed, id)

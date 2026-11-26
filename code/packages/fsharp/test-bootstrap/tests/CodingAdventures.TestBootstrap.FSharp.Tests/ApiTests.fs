/// Unit tests of the F# surface: every combinator, option and verdict shape. Where the C# engine has
/// an equivalent, the F# value is compared with the engine's own (the binding must forward, not
/// reinterpret).
module CodingAdventures.TestBootstrap.FSharp.Tests.ApiTests

open System
open CodingAdventures.TestBootstrap.Prng
open CodingAdventures.TestBootstrap.FSharp
open CodingAdventures.TestBootstrap.FSharp.Tests.Support
open Xunit

type private Cs = CodingAdventures.TestBootstrap.Pbt.Gens
type private CsShrink = CodingAdventures.TestBootstrap.Pbt.Shrink

let private off = { Options.defaults with Ci = Off }
let private draw (g: Gen<'T>) (seed: uint64) (n: int) : 'T list =
    let rng = SplitMix64 seed
    [ for _ in 1..n -> Gen.sample rng g ]

type Generators() =
    inherit SeamTest()

    [<Fact>]
    member _.``primitives draw exactly what the engine draws``() =
        let engine (g: CodingAdventures.TestBootstrap.Pbt.Gen<'T>) =
            let rng = SplitMix64 7UL
            [ for _ in 1..20 -> g.Sample rng ]
        Assert.Equal<bool list>(engine (Cs.Bool()), draw Gen.bool 7UL 20)
        Assert.Equal<byte list>(engine (Cs.U8()), draw Gen.u8 7UL 20)
        Assert.Equal<uint64 list>(engine (Cs.U64()), draw Gen.u64 7UL 20)
        Assert.Equal<bigint list>(engine (Cs.I128()), draw Gen.i128 7UL 20)
        Assert.Equal<bigint list>(engine (Cs.Integer(-5I, 5I)), draw (Gen.integer -5I 5I) 7UL 20)
        Assert.Equal<int64 list>(engine (Cs.Int64(-9L, 9L)), draw (Gen.int64 -9L 9L) 7UL 20)
        Assert.Equal<byte[] list>(engine (Cs.Bytes(0, 6)), draw (Gen.bytes 0 6) 7UL 20)

    [<Fact>]
    member _.``constant map and bind``() =
        Assert.Equal<int list>([ 4; 4; 4 ], draw (Gen.constant 4) 1UL 3)
        let doubled = Gen.integer 0I 9I |> Gen.map (fun x -> x * 2I)
        Assert.All(draw doubled 1UL 50, fun x -> Assert.True(x.IsEven))
        let dependent = Gen.integer 1I 5I |> Gen.bind (fun n -> Gen.list (int n) (int n) Gen.u8)
        Assert.All(draw dependent 1UL 20, fun l -> Assert.InRange(l.Length, 1, 5))

    [<Fact>]
    member _.``filter keeps matching values and filterWith gives up after maxRetries``() =
        let evens = Gen.integer 0I 100I |> Gen.filter (fun x -> x.IsEven)
        Assert.All(draw evens 3UL 50, fun x -> Assert.True(x.IsEven))
        let calls = ref 0
        let never = Gen.integer 0I 10I |> Gen.filterWith 7 (fun _ -> calls.Value <- calls.Value + 1; false)
        Assert.Throws<CodingAdventures.TestBootstrap.Pbt.ExhaustedException>(fun () -> Gen.sample (SplitMix64 1UL) never |> ignore) |> ignore
        Assert.Equal(7, calls.Value)
        Assert.Throws<ArgumentException>(fun () -> Gen.filterWith 0 (fun _ -> true) Gen.bool |> ignore) |> ignore

    [<Fact>]
    member _.``filter preserves shrinking``() =
        let g = Gen.integer 0I 1000I |> Gen.filter (fun x -> x.IsEven)
        let r = Gen.shrink 900I (fun x -> x >= 10I) 1000 g
        // Odd candidates are not valid inputs, so every accepted move stays even and still falsifies.
        Assert.True(r.Value.IsEven && r.Value >= 10I && r.Value < 900I && r.Steps > 0)

    [<Fact>]
    member _.``oneOf picks among its arms and rejects an empty list``() =
        let g = Gen.oneOf [ Gen.constant 1; Gen.constant 2; Gen.constant 3 ]
        Assert.Equal<int list>([ 1; 2; 3 ], draw g 5UL 200 |> List.distinct |> List.sort)
        Assert.Throws<ArgumentException>(fun () -> Gen.oneOf ([]: Gen<int> list) |> ignore) |> ignore

    [<Fact>]
    member _.``frequency honours weights and rejects bad ones``() =
        let g = Gen.frequency [ (0L, Gen.constant 'z'); (1L, Gen.constant 'a'); (3L, Gen.constant 'b') ]
        let xs = draw g 9UL 400
        Assert.DoesNotContain('z', xs)
        let count c = xs |> List.filter ((=) c) |> List.length
        Assert.True(count 'b' > count 'a')
        Assert.Throws<ArgumentException>(fun () -> Gen.frequency ([]: (int64 * Gen<int>) list) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Gen.frequency [ (-1L, Gen.constant 1) ] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Gen.frequency [ (0L, Gen.constant 1) ] |> ignore) |> ignore

    [<Fact>]
    member _.``list matches the engine's list in draws and in shrinking``() =
        let fs = Gen.list 0 8 Gen.u8
        let cs = Cs.ListOf(Cs.U8(), 0, 8)
        let rng = SplitMix64 11UL
        let expected = [ for _ in 1..20 -> List.ofSeq (cs.Sample rng) ]
        Assert.Equal<byte list list>(expected, draw fs 11UL 20)
        let start = [ 9uy; 3uy; 7uy; 7uy; 1uy ]
        let falsifies (l: byte list) = List.sum (List.map int l) >= 10
        let fsr = Gen.shrink start falsifies 1000 fs
        let csr = cs.Shrink(Collections.Generic.List<byte>(start), (fun l -> falsifies (List.ofSeq l)), 1000)
        Assert.Equal<byte list>(List.ofSeq csr.Value, fsr.Value)
        Assert.Equal(csr.Steps, fsr.Steps)

    [<Fact>]
    member _.``tuple and tuple3 match the engine's tuples in draws and in shrinking``() =
        let a = Gen.integer 0I 100I
        let b = Gen.integer 0I 100I
        let c = Gen.u8
        let rngFs = SplitMix64 21UL
        let rngCs = SplitMix64 21UL
        let t2 = Gen.tuple a b
        let c2 = Cs.Tuple2(Cs.Integer(0I, 100I), Cs.Integer(0I, 100I))
        let p = c2.Sample rngCs
        Assert.Equal((p.First, p.Second), Gen.sample rngFs t2)
        let r = Gen.shrink (80I, 90I) (fun (x, y) -> x + y >= 50I) 1000 t2
        let cr = c2.Shrink(CodingAdventures.TestBootstrap.Pbt.Pair<bigint, bigint>(80I, 90I), (fun q -> q.First + q.Second >= 50I), 1000)
        Assert.Equal((cr.Value.First, cr.Value.Second), r.Value)
        Assert.Equal(cr.Steps, r.Steps)
        let t3 = Gen.tuple3 a b c
        let c3 = Cs.Tuple3(Cs.Integer(0I, 100I), Cs.Integer(0I, 100I), Cs.U8())
        let q = c3.Sample(SplitMix64 5UL)
        Assert.Equal((q.First, q.Second, q.Third), Gen.sample (SplitMix64 5UL) t3)
        let r3 = Gen.shrink (50I, 60I, 70uy) (fun (x, y, z) -> x + y + bigint (int z) >= 30I) 1000 t3
        let cr3 = c3.Shrink(CodingAdventures.TestBootstrap.Pbt.Triple<bigint, bigint, byte>(50I, 60I, 70uy), (fun t -> t.First + t.Second + bigint (int t.Third) >= 30I), 1000)
        Assert.Equal((cr3.Value.First, cr3.Value.Second, cr3.Value.Third), r3.Value)
        Assert.Equal(cr3.Steps, r3.Steps)

    [<Fact>]
    member _.``custom builds a generator with and without a shrinker``() =
        let plain = Gen.custom (fun rng -> int (rng.Bounded 10UL)) None
        Assert.All(draw plain 1UL 30, fun x -> Assert.InRange(x, 0, 9))
        Assert.Equal(77, (Gen.shrink 77 (fun _ -> true) 10 plain).Value)
        Assert.Equal(0, (Gen.shrink 77 (fun _ -> true) 10 plain).Steps)
        let halving =
            Gen.custom (fun rng -> int (rng.Bounded 1000UL)) (Some(fun v falsifies _ -> if falsifies (v / 2) then { Value = v / 2; Steps = 1 } else { Value = v; Steps = 0 }))
        let r = Gen.shrink 800 (fun x -> x > 100) 10 halving
        Assert.Equal(400, r.Value)
        Assert.Equal(1, r.Steps)

    [<Fact>]
    member _.``shrink validates its budget``() =
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> Gen.shrink 1I (fun _ -> true) -1 (Gen.integer 0I 5I) |> ignore) |> ignore

type Expression() =
    inherit SeamTest()

    [<Fact>]
    member _.``gen binds draws in order and returns``() =
        let g =
            gen {
                let! n = Gen.integer 1I 10I
                let! xs = Gen.list 0 5 Gen.u8
                return (n, xs)
            }
        let rng = SplitMix64 13UL
        let (n, xs) = Gen.sample rng g
        // The same two draws, made by hand, in the same order.
        let rng2 = SplitMix64 13UL
        Assert.Equal(Gen.sample rng2 (Gen.integer 1I 10I), n)
        Assert.Equal<byte list>(Gen.sample rng2 (Gen.list 0 5 Gen.u8), xs)

    [<Fact>]
    member _.``return from zero and combine``() =
        let rf = gen { return! Gen.constant 9 }
        Assert.Equal(9, Gen.sample (SplitMix64 0UL) rf)
        let z = gen { if true then () }
        Assert.Equal((), Gen.sample (SplitMix64 0UL) z)
        let c =
            gen {
                if true then ()
                return 5
            }
        Assert.Equal(5, Gen.sample (SplitMix64 0UL) c)

    [<Fact>]
    member _.``a gen expression works inside Prop.check``() =
        let g =
            gen {
                let! a = Gen.integer 0I 50I
                let! b = Gen.integer 0I 50I
                return a + b
            }
        Assert.Equal(Passed 100, Prop.check 1UL off g (fun s -> s <= 100I))

type Runner() =
    inherit SeamTest()

    [<Fact>]
    member _.``a passing property is Passed with its case count``() =
        Assert.Equal(Passed 100, Prop.forAll (Gen.integer 0I 10I) (fun x -> x <= 10I))
        Assert.Equal(Passed 7, Prop.check 1UL { off with NumTests = 7 } Gen.bool (fun _ -> true))

    [<Fact>]
    member _.``a local failure carries seed, witness and the shrunk minimal``() =
        match Prop.check 42UL off (Gen.integer 0I 1000I) (fun x -> x < 10I) with
        | Failed f ->
            Assert.Equal(FailureId.counterexample, f.FailureId)
            Assert.Equal(Some 42UL, f.Seed)
            Assert.Equal(Some 10I, f.Minimal)
            Assert.True(f.Counterexample.Value >= 10I)
            Assert.True(f.ShrinkSteps > 0)
            Assert.Equal(None, f.Digest)
            Assert.Contains("seed=42", f.Message)
        | other -> failwithf "expected Failed, got %A" other

    [<Fact>]
    member _.``a redacted failure carries no seed and no witness``() =
        for ci in [ Unset; On ] do
            match Prop.check 42UL { off with Ci = ci } (Gen.integer 0I 1000I) (fun x -> x < 10I) with
            | Failed f ->
                Assert.Equal(None, f.Seed)
                Assert.Equal(None, f.Counterexample)
                Assert.Equal(None, f.Minimal)
                Assert.True(f.Digest.IsSome)
                Assert.DoesNotContain("seed=", f.Message)
            | other -> failwithf "expected Failed, got %A" other

    [<Fact>]
    member _.``the default options redact``() =
        Assert.Equal(Unset, Options.defaults.Ci)
        match Prop.forAll (Gen.integer 0I 1000I) (fun x -> x < 10I) with
        | Failed f -> Assert.Equal(None, f.Seed)
        | other -> failwithf "expected Failed, got %A" other

    [<Fact>]
    member _.``zero cases is Exhausted, never a pass``() =
        match Prop.check 1UL { off with NumTests = 0 } Gen.bool (fun _ -> true) with
        | Exhausted f ->
            Assert.Equal(FailureId.exhausted, f.FailureId)
            Assert.Equal(0, f.CasesRun)
        | other -> failwithf "expected Exhausted, got %A" other

    [<Fact>]
    member _.``invalid options raise instead of producing a verdict``() =
        Assert.Throws<ArgumentException>(fun () -> Prop.check 1UL { off with NumTests = -1 } Gen.bool (fun _ -> true) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Prop.check 1UL { off with MaxShrinkSteps = 10001 } Gen.bool (fun _ -> true) |> ignore) |> ignore

    [<Fact>]
    member _.``the same seed gives the same verdict``() =
        let run () = Prop.check 99UL off (Gen.list 0 10 (Gen.integer 0I 50I)) (fun l -> List.sum l < 60I)
        Assert.Equal(run (), run ())

    [<Fact>]
    member _.``checkWithShrinker overrides the generator's shrinker``() =
        let shrinker (v: bigint) (falsifies: bigint -> bool) (_: int) : Shrunk<bigint> =
            if falsifies 10I then { Value = 10I; Steps = 1 } else { Value = v; Steps = 0 }
        match Prop.checkWithShrinker 42UL off shrinker (Gen.integer 0I 1000I) (fun x -> x < 10I) with
        | Failed f ->
            Assert.Equal(Some 10I, f.Minimal)
            Assert.Equal(1, f.ShrinkSteps)
        | other -> failwithf "expected Failed, got %A" other

    [<Fact>]
    member _.``a shrinker that lies or throws is not believed``() =
        let liar (_: bigint) (_: bigint -> bool) (_: int) : Shrunk<bigint> = { Value = 0I; Steps = 1 } // 0 does not falsify
        let thrower (_: bigint) (_: bigint -> bool) (_: int) : Shrunk<bigint> = failwith "shrinker blew up"
        for s in [ liar; thrower ] do
            match Prop.checkWithShrinker 42UL off s (Gen.integer 0I 1000I) (fun x -> x < 10I) with
            | Failed f ->
                Assert.Equal(0, f.ShrinkSteps)
                Assert.Equal(f.Counterexample, f.Minimal)
            | other -> failwithf "expected Failed, got %A" other

    [<Fact>]
    member _.``toResult isPassed tryFailure and assertPassed``() =
        let ok = Prop.check 1UL off Gen.bool (fun _ -> true)
        let bad = Prop.check 1UL off (Gen.integer 0I 100I) (fun x -> x < 1I)
        let none = Prop.check 1UL { off with NumTests = 0 } Gen.bool (fun _ -> true)
        Assert.Equal(Ok 100, Prop.toResult ok)
        Assert.True(Result.isError (Prop.toResult bad))
        Assert.True(Result.isError (Prop.toResult none))
        Assert.True(Verdict.isPassed ok)
        Assert.False(Verdict.isPassed bad)
        Assert.Equal(None, Verdict.tryFailure ok)
        Assert.True((Verdict.tryFailure bad).IsSome)
        Assert.True((Verdict.tryFailure none).IsSome)
        Prop.assertPassed ok
        let e = Assert.Throws<CodingAdventures.TestBootstrap.Pbt.PropertyFailedException>(fun () -> Prop.assertPassed bad)
        Assert.Equal(FailureId.counterexample, e.FailureId)
        let e2 = Assert.Throws<CodingAdventures.TestBootstrap.Pbt.PropertyFailedException>(fun () -> Prop.assertPassed none)
        Assert.Equal(FailureId.exhausted, e2.FailureId)

type Vocabulary() =
    inherit SeamTest()

    [<Fact>]
    member _.``options helpers``() =
        Assert.Equal({ NumTests = 100; MaxShrinkSteps = 1000; Ci = Unset }, Options.defaults)
        Assert.Equal(On, (Options.defaults |> Options.withCi On).Ci)
        let env (kv: (string * string) list) name = kv |> List.tryFind (fun (k, _) -> k = name) |> Option.map snd
        Assert.False(Options.ciFromLookup (env []))
        Assert.False(Options.ciFromLookup (env [ "CI", "" ]))
        Assert.False(Options.ciFromLookup (env [ "CI", "0" ]))
        Assert.False(Options.ciFromLookup (env [ "CI", " False " ]))
        Assert.True(Options.ciFromLookup (env [ "CI", "true" ]))
        Assert.True(Options.ciFromLookup (env [ "CI", "1" ]))

    [<Fact>]
    member _.``the failure id vocabulary is closed``() =
        Assert.Equal<string list>([ "property-counterexample"; "property-exhausted"; "property-timeout"; "property-panic" ], FailureId.all)
        Assert.Equal("property-timeout", FailureId.timeout)
        Assert.Equal("property-panic", FailureId.panic)
        for id in FailureId.all do
            Assert.True(FailureId.isValid id)
            Assert.Equal(Some id, FailureId.tryParse id)
        Assert.False(FailureId.isValid "unregistered-id")
        Assert.False(FailureId.isValid null)
        Assert.Equal(None, FailureId.tryParse "nope")

    [<Fact>]
    member _.``seed derivation matches the engine``() =
        Assert.Equal(SplitMix64.SubSeed(5UL, "a"), Seed.sub 5UL "a")
        Assert.NotEqual(Seed.sub 5UL "a", Seed.sub 5UL "b")

type Standalone() =
    inherit SeamTest()

    [<Fact>]
    member _.``Shrink.integer shrinks and refuses a non-falsifying start``() =
        match Shrink.integer 900I 0I 0I 1000I (fun x -> x <= 100I) 1000 with
        | Ok r -> Assert.Equal(101I, r.Value)
        | Error e -> failwithf "%A" e
        Assert.Equal(Error NotFalsifying, Shrink.integer 5I 0I 0I 1000I (fun x -> x <= 100I) 1000)
        Assert.Throws<ArgumentException>(fun () -> Shrink.integer 5I 0I 10I 0I (fun _ -> false) 1000 |> ignore) |> ignore

    [<Fact>]
    member _.``Shrink.list and Shrink.bytes``() =
        match Shrink.list [ 5I; 9I; 9I; 2I ] 0I (fun l -> l |> List.pairwise |> List.forall (fun (a, b) -> a <> b)) 1000 with
        | Ok r -> Assert.Equal<bigint list>([ 9I; 9I ], r.Value)
        | Error e -> failwithf "%A" e
        Assert.Equal(Error NotFalsifying, Shrink.list [ 1I ] 0I (fun _ -> true) 1000)
        match Shrink.bytes [| 1uy; 2uy; 3uy; 4uy; 5uy; 6uy |] (fun b -> b.Length < 5) 1000 with
        | Ok r -> Assert.Equal(5, r.Value.Length)
        | Error e -> failwithf "%A" e
        Assert.Equal(Error NotFalsifying, Shrink.bytes [||] (fun _ -> true) 1000)

    [<Fact>]
    member _.``simulateReplay is deterministic and refuses an unfalsifiable property``() =
        let a = Shrink.simulateReplay 3UL (fun x -> x <= 100I)
        Assert.Equal(a, Shrink.simulateReplay 3UL (fun x -> x <= 100I))
        Assert.Equal(101I, a.Minimal)
        Assert.Throws<ArgumentException>(fun () -> Shrink.simulateReplay 3UL (fun _ -> true) |> ignore) |> ignore

    [<Fact>]
    member _.``Generate returns typed lists that match the engine``() =
        let prim name = Cs.GenPrimitive(8UL, name, 6) |> Seq.map (unbox<bigint>) |> List.ofSeq
        Assert.Equal(6, (Generate.bools 8UL 6).Length)
        Assert.Equal<bigint list>(prim "u8", Generate.u8s 8UL 6 |> List.map (int >> bigint))
        Assert.Equal<bigint list>(prim "u64", Generate.u64s 8UL 6 |> List.map bigint)
        Assert.Equal<bigint list>(prim "i128", Generate.i128s 8UL 6)
        Assert.Equal<bigint list>(List.ofSeq (Cs.GenBounded(8UL, -3I, 3I, 6)), Generate.bounded 8UL -3I 3I 6)
        Assert.Equal<byte[] list>(List.ofSeq (Cs.GenBytes(8UL, 1, 4, 6)), Generate.bytes 8UL 1 4 6)
        Assert.Throws<ArgumentException>(fun () -> Cs.GenPrimitive(1UL, "f32", 1) |> ignore) |> ignore

    [<Fact>]
    member _.``the seams change what the F# layer forwards``() =
        let baseline = Generate.u64s 1UL 3
        Seams.seedOffset <- 1UL
        let shifted = Generate.u64s 1UL 3
        Seams.reset ()
        Assert.Equal<uint64 list>(Generate.u64s 2UL 3, shifted)
        Assert.NotEqual<uint64 list>(baseline, shifted)
        Seams.swapBounds <- true
        let swapped = Generate.bounded 1UL 5I 0I 3
        Seams.reset ()
        Assert.Equal<bigint list>(Generate.bounded 1UL 0I 5I 3, swapped)

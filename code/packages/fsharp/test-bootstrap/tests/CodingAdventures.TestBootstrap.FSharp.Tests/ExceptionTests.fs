/// An exception thrown from an F# function must reach the C# engine's own handling and become the
/// verdict a C# lambda's throw would (VER05l section 3). The binding adds no catch of its own.
module CodingAdventures.TestBootstrap.FSharp.Tests.ExceptionTests

open System
open CodingAdventures.TestBootstrap.FSharp
open CodingAdventures.TestBootstrap.FSharp.Tests.Support
open Xunit

/// An F# exception type: it is a System.Exception subclass at runtime.
exception Boom of string

let private off = { Options.defaults with Ci = Off }
let private redacted = Options.defaults

let private expectExhausted (v: Verdict<'T>) : Failure<'T> =
    match v with
    | Exhausted f ->
        Assert.Equal(FailureId.exhausted, f.FailureId)
        f
    | other -> failwithf "expected Exhausted, got %A" other

type Throws() =
    inherit SeamTest()

    [<Fact>]
    member _.``a throw in a generator is property-exhausted``() =
        let g = Gen.custom (fun _ -> raise (InvalidOperationException "sampler blew up")) None
        let f = expectExhausted (Prop.check 1UL off g (fun _ -> true))
        Assert.Contains("sampler blew up", f.Message)
        // Redacted: the type name only, never the exception text.
        let r = expectExhausted (Prop.check 1UL redacted g (fun _ -> true))
        Assert.Contains("System.InvalidOperationException", r.Message)
        Assert.DoesNotContain("sampler blew up", r.Message)
        // A throw in map is a generator throw too.
        let m = Gen.integer 0I 5I |> Gen.map (fun _ -> failwith "map blew up" : int)
        Assert.Contains("map blew up", (expectExhausted (Prop.check 1UL off m (fun _ -> true))).Message)

    [<Fact>]
    member _.``a throw in the property is property-panic``() =
        for exn in [ (fun () -> failwith "prop blew up" : bool); (fun () -> raise (Boom "custom")); (fun () -> raise (InvalidCastException "cast")) ] do
            match Prop.check 5UL off Gen.bool (fun _ -> exn ()) with
            | Failed f ->
                Assert.Equal(FailureId.panic, f.FailureId)
                Assert.Equal(1, f.Iteration)
                Assert.Equal(Some 5UL, f.Seed)
            | other -> failwithf "expected a panic, got %A" other
        match Prop.check 5UL redacted Gen.bool (fun _ -> failwith "secret") with
        | Failed f ->
            Assert.Equal(FailureId.panic, f.FailureId)
            Assert.Equal(None, f.Seed)
            Assert.DoesNotContain("secret", f.Message)
        | other -> failwithf "expected a panic, got %A" other

    [<Fact>]
    member _.``a throw in a filter predicate is property-exhausted``() =
        let g = Gen.integer 0I 10I |> Gen.filter (fun _ -> raise (Boom "predicate"))
        let f = expectExhausted (Prop.check 1UL off g (fun _ -> true))
        Assert.Contains("predicate", f.Message)

    [<Fact>]
    member _.``a throw inside a gen expression is property-exhausted``() =
        // Before the first let!: Delay defers it to sample time, so it is still inside the engine.
        let early : Gen<int> =
            gen {
                failwith "before any bind"
                return 1
            }
        Assert.Contains("before any bind", (expectExhausted (Prop.check 1UL off early (fun _ -> true))).Message)
        // In a let! continuation.
        let mid =
            gen {
                let! n = Gen.integer 0I 9I
                if n >= 0I then failwith "after a bind"
                return n
            }
        Assert.Contains("after a bind", (expectExhausted (Prop.check 1UL off mid (fun _ -> true))).Message)
        // In the returned expression.
        let ret =
            gen {
                let! n = Gen.integer 0I 9I
                return (if n >= 0I then raise (Boom "in return") else n)
            }
        Assert.Contains("in return", (expectExhausted (Prop.check 1UL off ret (fun _ -> true))).Message)
        // Constructing the generator is safe; only sampling throws.
        let built = gen { return! (failwith "return!" : Gen<int>) }
        Assert.Contains("return!", (expectExhausted (Prop.check 1UL off built (fun _ -> true))).Message)

    [<Fact>]
    member _.``a throw in Gen.bind and Gen.delay continuations is property-exhausted``() =
        let b = Gen.integer 0I 3I |> Gen.bind (fun _ -> failwith "bind k" : Gen<int>)
        Assert.Contains("bind k", (expectExhausted (Prop.check 1UL off b (fun _ -> true))).Message)
        let d = Gen.delay (fun () -> failwith "delay k" : Gen<int>)
        Assert.Contains("delay k", (expectExhausted (Prop.check 1UL off d (fun _ -> true))).Message)

    [<Fact>]
    member _.``a throw in the property while shrinking is not a smaller counterexample``() =
        let falsified = ref false
        let prop (x: bigint) =
            if x < 10I then true
            elif not falsified.Value then
                falsified.Value <- true
                false
            else failwith "shrink-time blew up"
        match Prop.check 3UL off (Gen.integer 0I 1000I) prop with
        | Failed f ->
            Assert.Equal(FailureId.counterexample, f.FailureId)
            Assert.Equal(0, f.ShrinkSteps)
            Assert.Equal(f.Counterexample, f.Minimal)
        | other -> failwithf "expected a counterexample, got %A" other

    [<Fact>]
    member _.``a throw in a filter predicate while shrinking is not a smaller counterexample``() =
        let sampling = ref true
        let g = Gen.integer 0I 1000I |> Gen.filter (fun _ -> if sampling.Value then true else failwith "late predicate")
        let prop (x: bigint) =
            if x < 10I then true
            else
                sampling.Value <- false
                false
        match Prop.check 3UL off g prop with
        | Failed f ->
            Assert.Equal(FailureId.counterexample, f.FailureId)
            Assert.Equal(0, f.ShrinkSteps)
        | other -> failwithf "expected a counterexample, got %A" other

    [<Fact>]
    member _.``a throw in a custom shrinker's falsifies callback is contained``() =
        let shrinker (v: bigint) (falsifies: bigint -> bool) (_: int) : Shrunk<bigint> =
            falsifies 10I |> ignore
            { Value = v; Steps = 0 }
        let calls = ref 0
        let prop (x: bigint) =
            calls.Value <- calls.Value + 1
            if x < 10I then true
            elif calls.Value > 1 then failwith "second call"
            else false
        match Prop.checkWithShrinker 3UL off shrinker (Gen.integer 0I 1000I) prop with
        | Failed f -> Assert.Equal(FailureId.counterexample, f.FailureId)
        | other -> failwithf "expected a counterexample, got %A" other

    [<Fact>]
    member _.``the property and generator exceptions do not escape Prop.forAll``() =
        let v = bounded 30 (fun () -> Prop.forAll (Gen.custom (fun _ -> failwith "x") None) (fun () -> true))
        Assert.False(Verdict.isPassed v)

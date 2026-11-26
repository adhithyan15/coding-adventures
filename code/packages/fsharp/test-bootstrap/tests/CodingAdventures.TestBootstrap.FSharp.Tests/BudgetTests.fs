/// The binding opens no fresh budget root: a `Prop.check` made inside an F# property is charged to
/// the outermost C# check scope through the engine's AsyncLocal ambient (VER05l section 5).
module CodingAdventures.TestBootstrap.FSharp.Tests.BudgetTests

open System
open System.Threading.Tasks
open CodingAdventures.TestBootstrap.FSharp
open CodingAdventures.TestBootstrap.FSharp.Tests.Support
open Xunit

let private off = { Options.defaults with Ci = Off }

type Nested() =
    inherit SeamTest()

    /// An outer check of N cases whose property runs an inner check of 100,000 passing cases each
    /// time. Unbounded that is N x 100,000 evaluations; the root cap is numTests + 16*maxSteps + 64
    /// attempts, so the whole tree can evaluate only a few hundred. A runaway guard throws (a
    /// failing verdict) rather than letting a regression spin for minutes.
    [<Fact>]
    member _.``a nested F# check is bounded by the outer C# root``() =
        let maxSteps = 5
        let numTests = 20
        let rootCap = numTests + (16 * maxSteps + 64)
        let innerCases = ref 0L
        let guard = 5_000_000L
        let inner = Gen.integer 0I 9I
        let outerProp (x: bigint) =
            let v =
                Prop.check 1UL { off with NumTests = 100_000; MaxShrinkSteps = 10 } inner (fun _ ->
                    innerCases.Value <- innerCases.Value + 1L
                    if innerCases.Value > guard then failwith "runaway inner check: the budget did not bound it"
                    true)
            ignore v
            x < 10I
        let verdict = bounded 120 (fun () -> Prop.check 1UL { off with NumTests = numTests; MaxShrinkSteps = maxSteps } (Gen.integer 0I 1000I) outerProp)
        match verdict with
        | Failed f -> Assert.Equal(FailureId.counterexample, f.FailureId)
        | other -> failwithf "the outer property must still be falsified, got %A" other
        Assert.True(innerCases.Value > 0L, "the inner checks never ran")
        Assert.True(innerCases.Value <= int64 (2 * rootCap), sprintf "inner cases %d exceed twice the root cap %d" innerCases.Value rootCap)
        Assert.True(innerCases.Value < 100_000L, "a single uncapped inner check would already be 100,000")

    /// A starved nested check is Exhausted: it must never report a pass for work it did not do.
    [<Fact>]
    member _.``a starved nested F# check is Exhausted and never a pass``() =
        let verdicts = ResizeArray<Verdict<bigint>>()
        let outerProp (x: bigint) =
            verdicts.Add(Prop.check 1UL { off with NumTests = 100_000; MaxShrinkSteps = 10 } (Gen.integer 0I 9I) (fun _ -> true))
            x < 10I
        bounded 120 (fun () -> Prop.check 1UL { off with NumTests = 20; MaxShrinkSteps = 5 } (Gen.integer 0I 1000I) outerProp) |> ignore
        Assert.NotEmpty verdicts
        // The inner checks cannot all have passed 100,000 cases each.
        Assert.Contains(verdicts, fun v -> match v with Exhausted _ -> true | _ -> false)
        Assert.DoesNotContain(verdicts, fun v -> match v with Passed n -> n > 1000 | _ -> false)

    /// Outside any check the same inner call is a root of its own and really runs all its cases:
    /// the bound above comes from the ambient scope, not from a cap the binding applies.
    [<Fact>]
    member _.``the same inner check run at top level is not capped``() =
        let n = ref 0
        let v = Prop.check 1UL { off with NumTests = 5000 } (Gen.integer 0I 9I) (fun _ -> n.Value <- n.Value + 1; true)
        Assert.Equal(Passed 5000, v)
        Assert.Equal(5000, n.Value)

    /// The ambient scope flows into tasks started inside the property (AsyncLocal), so a nested check
    /// run on another task is charged to the same root.
    [<Fact>]
    member _.``a nested check on another task is charged to the same root``() =
        let innerCases = ref 0L
        let outerProp (x: bigint) =
            let t =
                Task.Run(fun () ->
                    Prop.check 1UL { off with NumTests = 100_000; MaxShrinkSteps = 10 } (Gen.integer 0I 9I) (fun _ ->
                        System.Threading.Interlocked.Increment(&innerCases.contents) |> ignore
                        true)
                    |> ignore)
            t.Wait()
            x < 10I
        bounded 120 (fun () -> Prop.check 1UL { off with NumTests = 20; MaxShrinkSteps = 5 } (Gen.integer 0I 1000I) outerProp) |> ignore
        Assert.InRange(innerCases.Value, 1L, int64 (2 * (20 + 16 * 5 + 64)))

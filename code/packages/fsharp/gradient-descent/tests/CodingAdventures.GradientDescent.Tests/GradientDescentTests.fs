namespace CodingAdventures.GradientDescent.Tests

open System
open CodingAdventures.GradientDescent
open Xunit

type GradientDescentTests() =
    [<Fact>]
    member _.SgdAppliesWeightUpdates() =
        let updated = GradientDescent.Sgd([ 1.0; -0.5; 2.0 ], [ 0.1; -0.2; 0.0 ], 0.1)

        Assert.Equal(0.99, updated[0], 9)
        Assert.Equal(-0.48, updated[1], 9)
        Assert.Equal(2.0, updated[2], 9)

    [<Fact>]
    member _.SgdRejectsInvalidInputs() =
        Assert.Throws<ArgumentNullException>(fun () -> GradientDescent.Sgd(Unchecked.defaultof<float list>, [ 1.0 ], 0.1) |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> GradientDescent.Sgd([ 1.0 ], Unchecked.defaultof<float list>, 0.1) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> GradientDescent.Sgd([], [], 0.1) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> GradientDescent.Sgd([ 1.0 ], [], 0.1) |> ignore) |> ignore

namespace CodingAdventures.Perceptron.Tests

open System
open Xunit
open CodingAdventures.Perceptron.FSharp

module PerceptronTests =
    let private andFeatures =
        [|
            [| 0.0; 0.0 |]
            [| 0.0; 1.0 |]
            [| 1.0; 0.0 |]
            [| 1.0; 1.0 |]
        |]

    [<Fact>]
    let ``fit learns AND gate`` () =
        let model = Perceptron(0.8, 5000)

        model.Fit(andFeatures, [| 0.0; 0.0; 0.0; 1.0 |])

        let predictions = model.Predict andFeatures
        predictions.[0..2]
        |> Array.iter (fun value -> Assert.True(value < 0.2, $"Expected negative class below 0.2, got {value}."))
        Assert.True(predictions.[3] > 0.7, $"Expected positive class above 0.7, got {predictions.[3]}.")
        Assert.Equal(2, model.Weights.Value.Length)

    [<Fact>]
    let ``fit accepts column labels`` () =
        let model = Perceptron(0.8, 5000)

        model.Fit(andFeatures, [| [| 0.0 |]; [| 0.0 |]; [| 0.0 |]; [| 1.0 |] |])

        Assert.True((model.Predict [| [| 1.0; 1.0 |] |]).[0] > 0.7)

    [<Fact>]
    let ``predict requires fit`` () =
        let model = Perceptron()

        Assert.Throws<InvalidOperationException>(fun () -> model.Predict(andFeatures) |> ignore)
        |> ignore

    [<Fact>]
    let ``fit validates training data`` () =
        let model = Perceptron()
        let emptyFeatures: float array array = [||]

        Assert.Throws<ArgumentException>(fun () -> model.Fit(emptyFeatures, [| 0.0 |])) |> ignore
        Assert.Throws<ArgumentException>(fun () -> model.Fit([| [| 0.0 |]; [| 1.0; 2.0 |] |], [| 0.0; 1.0 |])) |> ignore
        Assert.Throws<ArgumentException>(fun () -> model.Fit([| [| 0.0 |] |], [| 0.0; 1.0 |])) |> ignore
        Assert.Throws<ArgumentException>(fun () -> model.Fit([| [| 0.0 |] |], [| [| 0.0; 1.0 |] |])) |> ignore

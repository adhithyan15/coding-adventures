namespace CodingAdventures.LossFunctions.Tests

open System
open Xunit
open CodingAdventures.LossFunctions

type LossFunctionsTests() =
    let assertClose expected actual tolerance =
        Assert.True(abs (expected - actual) <= tolerance, $"Expected {expected} but got {actual}.")

    let assertArrayClose (expected: float array) (actual: float array) tolerance =
        Assert.Equal(expected.Length, actual.Length)
        Array.iter2 (fun left right -> assertClose left right tolerance) expected actual

    [<Fact>]
    member _.``Core loss functions match the spec vectors``() =
        assertClose 0.02 (LossFunctions.mse [| 1.0; 0.0; 0.0 |] [| 0.9; 0.1; 0.2 |]) 1e-9
        assertClose 0.13333333333333333 (LossFunctions.mae [| 1.0; 0.0; 0.0 |] [| 0.9; 0.1; 0.2 |]) 1e-9
        assertClose 0.14462152754328741 (LossFunctions.bce [| 1.0; 0.0; 1.0 |] [| 0.9; 0.1; 0.8 |]) 1e-9
        assertClose 0.07438118377140324 (LossFunctions.cce [| 1.0; 0.0; 0.0 |] [| 0.8; 0.1; 0.1 |]) 1e-9

    [<Fact>]
    member _.``Identical vectors have zero regression loss``() =
        let values = [| 1.0; 0.0; 0.5 |]
        assertClose 0.0 (LossFunctions.mse values values) 1e-12
        assertClose 0.0 (LossFunctions.mae values values) 1e-12

    [<Fact>]
    member _.``Derivatives match the reference calculations``() =
        assertArrayClose [| -0.2; 0.2 |] (LossFunctions.mseDerivative [| 1.0; 0.0 |] [| 0.8; 0.2 |]) 1e-9
        assertArrayClose [| -1.0 / 3.0; 1.0 / 3.0; 0.0 |] (LossFunctions.maeDerivative [| 1.0; 0.0; 0.5 |] [| 0.8; 0.2; 0.5 |]) 1e-9
        assertArrayClose [| -0.625; 0.625 |] (LossFunctions.bceDerivative [| 1.0; 0.0 |] [| 0.8; 0.2 |]) 1e-9
        assertArrayClose [| -0.625; 0.0 |] (LossFunctions.cceDerivative [| 1.0; 0.0 |] [| 0.8; 0.2 |]) 1e-9

    [<Fact>]
    member _.``Invalid shapes throw for every loss``() =
        Assert.Throws<ArgumentException>(fun () -> LossFunctions.mse [| 1.0 |] [| 0.9; 0.1 |] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> LossFunctions.mae [| 1.0 |] [| 0.9; 0.1 |] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> LossFunctions.bce [| 1.0 |] [| 0.9; 0.1 |] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> LossFunctions.cce [| 1.0 |] [| 0.9; 0.1 |] |> ignore) |> ignore

    [<Fact>]
    member _.``Empty vectors throw for every loss``() =
        Assert.Throws<ArgumentException>(fun () -> LossFunctions.mse [||] [||] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> LossFunctions.mae [||] [||] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> LossFunctions.bce [||] [||] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> LossFunctions.cce [||] [||] |> ignore) |> ignore

    [<Fact>]
    member _.``Cross entropy clamps boundary probabilities to stay finite``() =
        let bce = LossFunctions.bce [| 1.0; 0.0 |] [| 1.0; 0.0 |]
        let cce = LossFunctions.cce [| 1.0; 0.0; 0.0 |] [| 1.0; 0.0; 0.0 |]

        Assert.True(Double.IsFinite(bce))
        Assert.True(Double.IsFinite(cce))

namespace CodingAdventures.SingleLayerNetwork.Tests

open Xunit
open CodingAdventures.SingleLayerNetwork

type SingleLayerNetworkTests() =
    let near expected actual = Assert.InRange(actual, expected - 1.0e-6, expected + 1.0e-6)

    [<Fact>]
    member _.``one epoch exposes matrix gradients``() =
        let step =
            SingleLayerNetwork.trainOneEpochWithMatrices
                [| [| 1.0; 2.0 |] |]
                [| [| 3.0; 5.0 |] |]
                [| [| 0.0; 0.0 |]; [| 0.0; 0.0 |] |]
                [| 0.0; 0.0 |]
                0.1
                Linear
        near -3.0 step.WeightGradients.[0].[0]
        near -10.0 step.WeightGradients.[1].[1]
        near 0.3 step.NextWeights.[0].[0]
        near 1.0 step.NextWeights.[1].[1]

    [<Fact>]
    member _.``fit learns m inputs to n outputs``() =
        let model = Model(3, 2)
        let history =
            model.Fit(
                [| [| 0.0; 0.0; 1.0 |]; [| 1.0; 2.0; 1.0 |]; [| 2.0; 1.0; 1.0 |] |],
                [| [| 1.0; -1.0 |]; [| 3.0; 2.0 |]; [| 4.0; 1.0 |] |],
                learningRate = 0.05,
                epochs = 500
            )
        Assert.True(history.[history.Length - 1].Loss < history.[0].Loss)
        Assert.Equal(2, model.Predict([| [| 1.0; 1.0; 1.0 |] |]).[0].Length)

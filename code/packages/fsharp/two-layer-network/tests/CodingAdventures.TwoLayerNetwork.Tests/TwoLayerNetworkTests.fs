namespace CodingAdventures.TwoLayerNetwork.Tests

open Xunit
open CodingAdventures.TwoLayerNetwork

module TwoLayerNetworkTests =
    let inputs = [| [| 0.0; 0.0 |]; [| 0.0; 1.0 |]; [| 1.0; 0.0 |]; [| 1.0; 1.0 |] |]
    let targets = [| [| 0.0 |]; [| 1.0 |]; [| 1.0 |]; [| 0.0 |] |]

    [<Fact>]
    let ``forward pass exposes hidden activations`` () =
        let passed = TwoLayerNetwork.forward inputs (TwoLayerNetwork.xorWarmStartParameters()) Sigmoid Sigmoid
        Assert.Equal(4, passed.HiddenActivations.Length)
        Assert.Equal(2, passed.HiddenActivations[0].Length)
        Assert.True(passed.Predictions[1][0] > 0.7)
        Assert.True(passed.Predictions[0][0] < 0.3)

    [<Fact>]
    let ``training step exposes both layer gradients`` () =
        let step = TwoLayerNetwork.trainOneEpoch inputs targets (TwoLayerNetwork.xorWarmStartParameters()) 0.5 Sigmoid Sigmoid
        Assert.Equal(2, step.InputToHiddenWeightGradients.Length)
        Assert.Equal(2, step.InputToHiddenWeightGradients[0].Length)
        Assert.Equal(2, step.HiddenToOutputWeightGradients.Length)
        Assert.Equal(1, step.HiddenToOutputWeightGradients[0].Length)

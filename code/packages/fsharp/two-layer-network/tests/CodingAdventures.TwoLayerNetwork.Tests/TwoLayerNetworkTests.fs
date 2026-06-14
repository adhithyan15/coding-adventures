namespace CodingAdventures.TwoLayerNetwork.Tests

open Xunit
open CodingAdventures.TwoLayerNetwork

module TwoLayerNetworkTests =
    let inputs = [| [| 0.0; 0.0 |]; [| 0.0; 1.0 |]; [| 1.0; 0.0 |]; [| 1.0; 1.0 |] |]
    let targets = [| [| 0.0 |]; [| 1.0 |]; [| 1.0 |]; [| 0.0 |] |]

    let sampleParameters inputCount hiddenCount =
        { InputToHiddenWeights =
            Array.init inputCount (fun feature ->
                Array.init hiddenCount (fun hidden -> 0.17 * float (feature + 1) - 0.11 * float (hidden + 1)))
          HiddenBiases = Array.init hiddenCount (fun hidden -> 0.05 * float (hidden - 1))
          HiddenToOutputWeights = Array.init hiddenCount (fun hidden -> [| 0.13 * float (hidden + 1) - 0.25 |])
          OutputBiases = [| 0.02 |] }

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

    [<Fact>]
    let ``hidden layer teaching examples run one training step`` () =
        let cases =
            [| ("XNOR", inputs, [| [| 1.0 |]; [| 0.0 |]; [| 0.0 |]; [| 1.0 |] |], 3)
               ("absolute value", [| [| -1.0 |]; [| -0.5 |]; [| 0.0 |]; [| 0.5 |]; [| 1.0 |] |], [| [| 1.0 |]; [| 0.5 |]; [| 0.0 |]; [| 0.5 |]; [| 1.0 |] |], 4)
               ("piecewise pricing", [| [| 0.1 |]; [| 0.3 |]; [| 0.5 |]; [| 0.7 |]; [| 0.9 |] |], [| [| 0.12 |]; [| 0.25 |]; [| 0.55 |]; [| 0.88 |]; [| 0.88 |] |], 4)
               ("circle classifier", [| [| 0.0; 0.0 |]; [| 0.5; 0.0 |]; [| 1.0; 1.0 |]; [| -0.5; 0.5 |]; [| -1.0; 0.0 |] |], [| [| 1.0 |]; [| 1.0 |]; [| 0.0 |]; [| 1.0 |]; [| 0.0 |] |], 5)
               ("two moons", [| [| 1.0; 0.0 |]; [| 0.0; 0.5 |]; [| 0.5; 0.85 |]; [| 0.5; -0.35 |]; [| -1.0; 0.0 |]; [| 2.0; 0.5 |] |], [| [| 0.0 |]; [| 1.0 |]; [| 0.0 |]; [| 1.0 |]; [| 0.0 |]; [| 1.0 |] |], 5)
               ("interaction features", [| [| 0.2; 0.25; 0.0 |]; [| 0.6; 0.5; 1.0 |]; [| 1.0; 0.75; 1.0 |]; [| 1.0; 1.0; 0.0 |] |], [| [| 0.08 |]; [| 0.72 |]; [| 0.96 |]; [| 0.76 |] |], 5) |]

        for name, exampleInputs, exampleTargets, hiddenCount in cases do
            let step = TwoLayerNetwork.trainOneEpoch exampleInputs exampleTargets (sampleParameters exampleInputs[0].Length hiddenCount) 0.4 Sigmoid Sigmoid
            Assert.True(step.Loss >= 0.0, name)
            Assert.Equal(exampleInputs[0].Length, step.InputToHiddenWeightGradients.Length)
            Assert.Equal(hiddenCount, step.HiddenToOutputWeightGradients.Length)

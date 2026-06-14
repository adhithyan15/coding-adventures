namespace CodingAdventures.TwoLayerNetwork.Tests;

using Xunit;

public sealed class TwoLayerNetworkTests
{
    private static readonly double[][] Inputs =
    [
        [0.0, 0.0],
        [0.0, 1.0],
        [1.0, 0.0],
        [1.0, 1.0]
    ];

    private static readonly double[][] Targets = [[0.0], [1.0], [1.0], [0.0]];

    [Fact]
    public void ForwardPassExposesHiddenActivations()
    {
        var pass = TwoLayerNetwork.Forward(Inputs, TwoLayerNetwork.XorWarmStartParameters());

        Assert.Equal(4, pass.HiddenActivations.Length);
        Assert.Equal(2, pass.HiddenActivations[0].Length);
        Assert.True(pass.Predictions[1][0] > 0.7);
        Assert.True(pass.Predictions[0][0] < 0.3);
    }

    [Fact]
    public void TrainingStepExposesBothLayerGradients()
    {
        var step = TwoLayerNetwork.TrainOneEpoch(Inputs, Targets, TwoLayerNetwork.XorWarmStartParameters(), 0.5);

        Assert.Equal(2, step.InputToHiddenWeightGradients.Length);
        Assert.Equal(2, step.InputToHiddenWeightGradients[0].Length);
        Assert.Equal(2, step.HiddenToOutputWeightGradients.Length);
        Assert.Single(step.HiddenToOutputWeightGradients[0]);
    }

    [Fact]
    public void HiddenLayerTeachingExamplesRunOneTrainingStep()
    {
        var cases = new (string Name, double[][] Inputs, double[][] Targets, int HiddenCount)[]
        {
            ("XNOR", Inputs, [[1.0], [0.0], [0.0], [1.0]], 3),
            ("absolute value", [[-1.0], [-0.5], [0.0], [0.5], [1.0]], [[1.0], [0.5], [0.0], [0.5], [1.0]], 4),
            ("piecewise pricing", [[0.1], [0.3], [0.5], [0.7], [0.9]], [[0.12], [0.25], [0.55], [0.88], [0.88]], 4),
            ("circle classifier", [[0.0, 0.0], [0.5, 0.0], [1.0, 1.0], [-0.5, 0.5], [-1.0, 0.0]], [[1.0], [1.0], [0.0], [1.0], [0.0]], 5),
            ("two moons", [[1.0, 0.0], [0.0, 0.5], [0.5, 0.85], [0.5, -0.35], [-1.0, 0.0], [2.0, 0.5]], [[0.0], [1.0], [0.0], [1.0], [0.0], [1.0]], 5),
            ("interaction features", [[0.2, 0.25, 0.0], [0.6, 0.5, 1.0], [1.0, 0.75, 1.0], [1.0, 1.0, 0.0]], [[0.08], [0.72], [0.96], [0.76]], 5),
        };

        foreach (var item in cases)
        {
            var step = TwoLayerNetwork.TrainOneEpoch(item.Inputs, item.Targets, SampleParameters(item.Inputs[0].Length, item.HiddenCount), 0.4);

            Assert.True(step.Loss >= 0.0, item.Name);
            Assert.Equal(item.Inputs[0].Length, step.InputToHiddenWeightGradients.Length);
            Assert.Equal(item.HiddenCount, step.HiddenToOutputWeightGradients.Length);
        }
    }

    private static Parameters SampleParameters(int inputCount, int hiddenCount)
    {
        var inputToHidden = new double[inputCount][];
        for (var feature = 0; feature < inputCount; feature++)
        {
            inputToHidden[feature] = new double[hiddenCount];
            for (var hidden = 0; hidden < hiddenCount; hidden++)
            {
                inputToHidden[feature][hidden] = 0.17 * (feature + 1) - 0.11 * (hidden + 1);
            }
        }

        var hiddenBiases = new double[hiddenCount];
        var hiddenToOutput = new double[hiddenCount][];
        for (var hidden = 0; hidden < hiddenCount; hidden++)
        {
            hiddenBiases[hidden] = 0.05 * (hidden - 1);
            hiddenToOutput[hidden] = [0.13 * (hidden + 1) - 0.25];
        }

        return new Parameters(inputToHidden, hiddenBiases, hiddenToOutput, [0.02]);
    }
}

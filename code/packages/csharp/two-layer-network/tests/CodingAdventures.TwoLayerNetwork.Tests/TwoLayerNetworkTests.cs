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
}

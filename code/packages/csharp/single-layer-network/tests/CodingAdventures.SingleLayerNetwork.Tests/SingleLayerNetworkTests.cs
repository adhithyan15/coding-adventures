namespace CodingAdventures.SingleLayerNetwork.Tests;

using Xunit;

public sealed class SingleLayerNetworkTests
{
    [Fact]
    public void OneEpochExposesMatrixGradients()
    {
        var step = SingleLayerNetwork.TrainOneEpochWithMatrices(
            [[1.0, 2.0]],
            [[3.0, 5.0]],
            [[0.0, 0.0], [0.0, 0.0]],
            [0.0, 0.0],
            0.1
        );

        Assert.Equal([[0.0, 0.0]], step.Predictions);
        Assert.Equal([[-3.0, -5.0]], step.Errors);
        Assert.Equal([[-3.0, -5.0], [-6.0, -10.0]], step.WeightGradients);
        Assert.Equal(0.3, step.NextWeights[0][0], 6);
        Assert.Equal(1.0, step.NextWeights[1][1], 6);
    }

    [Fact]
    public void FitLearnsMInputsToNOutputs()
    {
        var model = new SingleLayerNetwork(3, 2);
        var history = model.Fit(
            [[0.0, 0.0, 1.0], [1.0, 2.0, 1.0], [2.0, 1.0, 1.0]],
            [[1.0, -1.0], [3.0, 2.0], [4.0, 1.0]],
            0.05,
            500
        );
        Assert.True(history[^1].Loss < history[0].Loss);
        Assert.Equal(2, model.Predict([[1.0, 1.0, 1.0]])[0].Length);
    }
}

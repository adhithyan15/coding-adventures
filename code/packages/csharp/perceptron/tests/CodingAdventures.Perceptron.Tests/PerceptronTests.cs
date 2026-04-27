namespace CodingAdventures.Perceptron.Tests;

public sealed class PerceptronTests
{
    private static readonly double[][] AndFeatures =
    [
        [0.0, 0.0],
        [0.0, 1.0],
        [1.0, 0.0],
        [1.0, 1.0],
    ];

    [Fact]
    public void FitLearnsAndGate()
    {
        var model = new Perceptron(learningRate: 0.8, epochs: 5_000);

        model.Fit(AndFeatures, [0.0, 0.0, 0.0, 1.0]);

        var predictions = model.Predict(AndFeatures);
        Assert.All(predictions.Take(3), value => Assert.True(value < 0.2, $"Expected negative class below 0.2, got {value}."));
        Assert.True(predictions[3] > 0.7, $"Expected positive class above 0.7, got {predictions[3]}.");
        Assert.Equal(2, model.Weights!.Count);
    }

    [Fact]
    public void FitAcceptsColumnLabels()
    {
        var model = new Perceptron(learningRate: 0.8, epochs: 5_000);

        model.Fit(AndFeatures, [[0.0], [0.0], [0.0], [1.0]]);

        Assert.True(model.Predict([[1.0, 1.0]])[0] > 0.7);
    }

    [Fact]
    public void PredictRequiresFit()
    {
        var model = new Perceptron();

        Assert.Throws<InvalidOperationException>(() => model.Predict(AndFeatures));
    }

    [Fact]
    public void FitValidatesTrainingData()
    {
        var model = new Perceptron();

        Assert.Throws<ArgumentException>(() => model.Fit(Array.Empty<IReadOnlyList<double>>(), [0.0]));
        Assert.Throws<ArgumentException>(() => model.Fit([[0.0], [1.0, 2.0]], [0.0, 1.0]));
        Assert.Throws<ArgumentException>(() => model.Fit([[0.0]], [0.0, 1.0]));
        Assert.Throws<ArgumentException>(() => model.Fit([[0.0]], [[0.0, 1.0]]));
    }
}

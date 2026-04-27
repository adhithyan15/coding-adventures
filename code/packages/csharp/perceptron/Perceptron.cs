using Activations = CodingAdventures.ActivationFunctions.ActivationFunctions;
using Losses = CodingAdventures.LossFunctions.LossFunctions;

namespace CodingAdventures.Perceptron;

/// <summary>
/// Single-neuron binary classifier trained with sigmoid activation and binary cross-entropy.
/// </summary>
public sealed class Perceptron
{
    private double[]? _weights;

    /// <summary>Create a perceptron trainer.</summary>
    public Perceptron(double learningRate = 0.1, int epochs = 2000)
    {
        if (!double.IsFinite(learningRate))
        {
            throw new ArgumentOutOfRangeException(nameof(learningRate), "Learning rate must be finite.");
        }

        if (epochs < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(epochs), "Epochs must be non-negative.");
        }

        LearningRate = learningRate;
        Epochs = epochs;
    }

    /// <summary>Step size applied to each gradient update.</summary>
    public double LearningRate { get; }

    /// <summary>Number of training epochs to run.</summary>
    public int Epochs { get; }

    /// <summary>Current bias term.</summary>
    public double Bias { get; private set; }

    /// <summary>Current trained weights, or null before fitting.</summary>
    public IReadOnlyList<double>? Weights => _weights?.ToArray();

    /// <summary>Fit against labels supplied as a single binary value per sample.</summary>
    public void Fit(
        IReadOnlyList<IReadOnlyList<double>> features,
        IReadOnlyList<double> labels,
        int logSteps = 0)
    {
        var x = ValidateFeatures(features);
        var y = ValidateLabels(labels, x.Length);
        var featureCount = x[0].Length;

        _weights = new double[featureCount];
        Bias = 0.0;

        for (var epoch = 0; epoch <= Epochs; epoch++)
        {
            var rawScores = new double[x.Length];
            var predictions = new double[x.Length];

            for (var row = 0; row < x.Length; row++)
            {
                rawScores[row] = Score(x[row], _weights, Bias);
                predictions[row] = Activations.Sigmoid(rawScores[row]);
            }

            var lossGradient = Losses.BceDerivative(y, predictions);
            var weightGradient = new double[featureCount];
            var biasGradient = 0.0;

            for (var row = 0; row < x.Length; row++)
            {
                var combined = lossGradient[row] * Activations.SigmoidDerivative(rawScores[row]);
                for (var col = 0; col < featureCount; col++)
                {
                    weightGradient[col] += x[row][col] * combined;
                }

                biasGradient += combined;
            }

            for (var col = 0; col < featureCount; col++)
            {
                _weights[col] -= LearningRate * weightGradient[col];
            }

            Bias -= LearningRate * biasGradient;

            if (logSteps > 0 && epoch % logSteps == 0)
            {
                var loss = Losses.Bce(y, predictions);
                Console.WriteLine($"Epoch {epoch,4} | BCE Loss: {loss:F4} | Bias: {Bias:F2}");
            }
        }
    }

    /// <summary>Fit against labels supplied as one-column rows.</summary>
    public void Fit(
        IReadOnlyList<IReadOnlyList<double>> features,
        IReadOnlyList<IReadOnlyList<double>> labels,
        int logSteps = 0) =>
        Fit(features, FlattenLabels(labels), logSteps);

    /// <summary>Predict probabilities for each sample.</summary>
    public double[] Predict(IReadOnlyList<IReadOnlyList<double>> features)
    {
        if (_weights is null)
        {
            throw new InvalidOperationException("Perceptron has not been trained yet. Call Fit first.");
        }

        var x = ValidateFeatures(features);
        if (x[0].Length != _weights.Length)
        {
            throw new ArgumentException(
                $"Feature width {x[0].Length} does not match trained width {_weights.Length}.",
                nameof(features));
        }

        var predictions = new double[x.Length];
        for (var row = 0; row < x.Length; row++)
        {
            predictions[row] = Activations.Sigmoid(Score(x[row], _weights, Bias));
        }

        return predictions;
    }

    private static double Score(IReadOnlyList<double> row, IReadOnlyList<double> weights, double bias)
    {
        var sum = bias;
        for (var i = 0; i < weights.Count; i++)
        {
            sum += row[i] * weights[i];
        }

        return sum;
    }

    private static double[][] ValidateFeatures(IReadOnlyList<IReadOnlyList<double>> features)
    {
        ArgumentNullException.ThrowIfNull(features);
        if (features.Count == 0)
        {
            throw new ArgumentException("Training data must contain at least one sample.", nameof(features));
        }

        var expectedColumns = -1;
        var copy = new double[features.Count][];
        for (var row = 0; row < features.Count; row++)
        {
            var values = features[row];
            ArgumentNullException.ThrowIfNull(values);

            if (row == 0)
            {
                if (values.Count == 0)
                {
                    throw new ArgumentException("Samples must contain at least one feature.", nameof(features));
                }

                expectedColumns = values.Count;
            }
            else if (values.Count != expectedColumns)
            {
                throw new ArgumentException(
                    $"Sample {row} has {values.Count} features, expected {expectedColumns}.",
                    nameof(features));
            }

            copy[row] = new double[expectedColumns];
            for (var col = 0; col < expectedColumns; col++)
            {
                var value = values[col];
                if (!double.IsFinite(value))
                {
                    throw new ArgumentException("Feature values must be finite.", nameof(features));
                }

                copy[row][col] = value;
            }
        }

        return copy;
    }

    private static double[] ValidateLabels(IReadOnlyList<double> labels, int expectedRows)
    {
        ArgumentNullException.ThrowIfNull(labels);
        if (labels.Count != expectedRows || labels.Count == 0)
        {
            throw new ArgumentException("Labels must match the non-zero sample count.", nameof(labels));
        }

        var copy = new double[labels.Count];
        for (var i = 0; i < labels.Count; i++)
        {
            var value = labels[i];
            if (!double.IsFinite(value))
            {
                throw new ArgumentException("Labels must be finite.", nameof(labels));
            }

            copy[i] = value;
        }

        return copy;
    }

    private static double[] FlattenLabels(IReadOnlyList<IReadOnlyList<double>> labels)
    {
        ArgumentNullException.ThrowIfNull(labels);
        var copy = new double[labels.Count];
        for (var i = 0; i < labels.Count; i++)
        {
            var row = labels[i];
            ArgumentNullException.ThrowIfNull(row);
            if (row.Count != 1)
            {
                throw new ArgumentException("Column labels must have exactly one value per row.", nameof(labels));
            }

            copy[i] = row[0];
        }

        return copy;
    }
}

namespace CodingAdventures.SingleLayerNetwork;

public enum ActivationName
{
    Linear,
    Sigmoid
}

public sealed record TrainingStep(
    double[][] Predictions,
    double[][] Errors,
    double[][] WeightGradients,
    double[] BiasGradients,
    double[][] NextWeights,
    double[] NextBiases,
    double Loss
);

public sealed class SingleLayerNetwork
{
    public const string Version = "0.1.0";

    public double[][] Weights { get; private set; }
    public double[] Biases { get; private set; }
    public ActivationName Activation { get; }

    public SingleLayerNetwork(int inputCount, int outputCount, ActivationName activation = ActivationName.Linear)
    {
        Weights = Enumerable.Range(0, inputCount).Select(_ => new double[outputCount]).ToArray();
        Biases = new double[outputCount];
        Activation = activation;
    }

    public double[][] Predict(double[][] inputs) => PredictWithParameters(inputs, Weights, Biases, Activation);

    public IReadOnlyList<TrainingStep> Fit(double[][] inputs, double[][] targets, double learningRate = 0.05, int epochs = 100)
    {
        var history = new List<TrainingStep>(epochs);
        for (var epoch = 0; epoch < epochs; epoch++)
        {
            var step = TrainOneEpochWithMatrices(inputs, targets, Weights, Biases, learningRate, Activation);
            Weights = step.NextWeights;
            Biases = step.NextBiases;
            history.Add(step);
        }
        return history;
    }

    public static double[][] PredictWithParameters(double[][] inputs, double[][] weights, double[] biases, ActivationName activation = ActivationName.Linear)
    {
        var (sampleCount, inputCount) = ValidateMatrix("inputs", inputs);
        var (weightRows, outputCount) = ValidateMatrix("weights", weights);
        if (inputCount != weightRows)
            throw new ArgumentException("input column count must match weight row count");
        if (biases.Length != outputCount)
            throw new ArgumentException("bias count must match output count");

        var predictions = NewMatrix(sampleCount, outputCount);
        for (var row = 0; row < sampleCount; row++)
        {
            for (var output = 0; output < outputCount; output++)
            {
                var total = biases[output];
                for (var input = 0; input < inputCount; input++)
                    total += inputs[row][input] * weights[input][output];
                predictions[row][output] = Activate(total, activation);
            }
        }
        return predictions;
    }

    public static TrainingStep TrainOneEpochWithMatrices(
        double[][] inputs,
        double[][] targets,
        double[][] weights,
        double[] biases,
        double learningRate,
        ActivationName activation = ActivationName.Linear)
    {
        var (sampleCount, inputCount) = ValidateMatrix("inputs", inputs);
        var (targetRows, outputCount) = ValidateMatrix("targets", targets);
        var (weightRows, weightCols) = ValidateMatrix("weights", weights);
        if (targetRows != sampleCount)
            throw new ArgumentException("inputs and targets must have the same row count");
        if (weightRows != inputCount || weightCols != outputCount)
            throw new ArgumentException("weights must be shaped input_count x output_count");
        if (biases.Length != outputCount)
            throw new ArgumentException("bias count must match output count");

        var predictions = PredictWithParameters(inputs, weights, biases, activation);
        var scale = 2.0 / (sampleCount * outputCount);
        var errors = NewMatrix(sampleCount, outputCount);
        var deltas = NewMatrix(sampleCount, outputCount);
        var lossTotal = 0.0;
        for (var row = 0; row < sampleCount; row++)
        {
            for (var output = 0; output < outputCount; output++)
            {
                var error = predictions[row][output] - targets[row][output];
                errors[row][output] = error;
                deltas[row][output] = scale * error * DerivativeFromOutput(predictions[row][output], activation);
                lossTotal += error * error;
            }
        }

        var weightGradients = NewMatrix(inputCount, outputCount);
        var nextWeights = NewMatrix(inputCount, outputCount);
        for (var input = 0; input < inputCount; input++)
        {
            for (var output = 0; output < outputCount; output++)
            {
                for (var row = 0; row < sampleCount; row++)
                    weightGradients[input][output] += inputs[row][input] * deltas[row][output];
                nextWeights[input][output] = weights[input][output] - learningRate * weightGradients[input][output];
            }
        }

        var biasGradients = new double[outputCount];
        var nextBiases = new double[outputCount];
        for (var output = 0; output < outputCount; output++)
        {
            for (var row = 0; row < sampleCount; row++)
                biasGradients[output] += deltas[row][output];
            nextBiases[output] = biases[output] - learningRate * biasGradients[output];
        }

        return new TrainingStep(
            predictions,
            errors,
            weightGradients,
            biasGradients,
            nextWeights,
            nextBiases,
            lossTotal / (sampleCount * outputCount)
        );
    }

    private static double Activate(double value, ActivationName activation) =>
        activation switch
        {
            ActivationName.Linear => value,
            ActivationName.Sigmoid when value >= 0.0 => 1.0 / (1.0 + Math.Exp(-value)),
            ActivationName.Sigmoid => Math.Exp(value) / (1.0 + Math.Exp(value)),
            _ => throw new ArgumentOutOfRangeException(nameof(activation))
        };

    private static double DerivativeFromOutput(double output, ActivationName activation) =>
        activation switch
        {
            ActivationName.Linear => 1.0,
            ActivationName.Sigmoid => output * (1.0 - output),
            _ => throw new ArgumentOutOfRangeException(nameof(activation))
        };

    private static (int Rows, int Columns) ValidateMatrix(string name, double[][] matrix)
    {
        if (matrix.Length == 0)
            throw new ArgumentException($"{name} must contain at least one row");
        var width = matrix[0].Length;
        if (width == 0)
            throw new ArgumentException($"{name} must contain at least one column");
        if (matrix.Any(row => row.Length != width))
            throw new ArgumentException($"{name} must be rectangular");
        return (matrix.Length, width);
    }

    private static double[][] NewMatrix(int rows, int columns) =>
        Enumerable.Range(0, rows).Select(_ => new double[columns]).ToArray();
}

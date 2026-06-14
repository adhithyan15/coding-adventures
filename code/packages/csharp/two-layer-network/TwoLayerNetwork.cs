namespace CodingAdventures.TwoLayerNetwork;

public enum ActivationName
{
    Linear,
    Sigmoid
}

public sealed record Parameters(
    double[][] InputToHiddenWeights,
    double[] HiddenBiases,
    double[][] HiddenToOutputWeights,
    double[] OutputBiases
);

public sealed record ForwardPass(
    double[][] HiddenRaw,
    double[][] HiddenActivations,
    double[][] OutputRaw,
    double[][] Predictions
);

public sealed record TrainingStep(
    double[][] Predictions,
    double[][] Errors,
    double[][] OutputDeltas,
    double[][] HiddenDeltas,
    double[][] HiddenToOutputWeightGradients,
    double[] OutputBiasGradients,
    double[][] InputToHiddenWeightGradients,
    double[] HiddenBiasGradients,
    Parameters NextParameters,
    double Loss
);

public sealed class TwoLayerNetwork
{
    public const string Version = "0.1.0";

    public Parameters Parameters { get; private set; }
    public double LearningRate { get; }
    public ActivationName HiddenActivation { get; }
    public ActivationName OutputActivation { get; }

    public TwoLayerNetwork(
        Parameters parameters,
        double learningRate = 0.5,
        ActivationName hiddenActivation = ActivationName.Sigmoid,
        ActivationName outputActivation = ActivationName.Sigmoid)
    {
        Parameters = parameters;
        LearningRate = learningRate;
        HiddenActivation = hiddenActivation;
        OutputActivation = outputActivation;
    }

    public static Parameters XorWarmStartParameters() =>
        new(
            new[] { new[] { 4.0, -4.0 }, new[] { 4.0, -4.0 } },
            new[] { -2.0, 6.0 },
            new[] { new[] { 4.0 }, new[] { 4.0 } },
            new[] { -6.0 }
        );

    public double[][] Predict(double[][] inputs) => Forward(inputs, Parameters, HiddenActivation, OutputActivation).Predictions;

    public ForwardPass Inspect(double[][] inputs) => Forward(inputs, Parameters, HiddenActivation, OutputActivation);

    public IReadOnlyList<TrainingStep> Fit(double[][] inputs, double[][] targets, int epochs = 100)
    {
        var history = new List<TrainingStep>(epochs);
        for (var epoch = 0; epoch < epochs; epoch++)
        {
            var step = TrainOneEpoch(inputs, targets, Parameters, LearningRate, HiddenActivation, OutputActivation);
            Parameters = step.NextParameters;
            history.Add(step);
        }
        return history;
    }

    public static ForwardPass Forward(
        double[][] inputs,
        Parameters parameters,
        ActivationName hiddenActivation = ActivationName.Sigmoid,
        ActivationName outputActivation = ActivationName.Sigmoid)
    {
        var hiddenRaw = AddBiases(Dot(inputs, parameters.InputToHiddenWeights), parameters.HiddenBiases);
        var hiddenActivations = ApplyActivation(hiddenRaw, hiddenActivation);
        var outputRaw = AddBiases(Dot(hiddenActivations, parameters.HiddenToOutputWeights), parameters.OutputBiases);
        var predictions = ApplyActivation(outputRaw, outputActivation);
        return new ForwardPass(hiddenRaw, hiddenActivations, outputRaw, predictions);
    }

    public static TrainingStep TrainOneEpoch(
        double[][] inputs,
        double[][] targets,
        Parameters parameters,
        double learningRate,
        ActivationName hiddenActivation = ActivationName.Sigmoid,
        ActivationName outputActivation = ActivationName.Sigmoid)
    {
        var (sampleCount, _) = ValidateMatrix("inputs", inputs);
        var (_, outputCount) = ValidateMatrix("targets", targets);
        var pass = Forward(inputs, parameters, hiddenActivation, outputActivation);
        var scale = 2.0 / (sampleCount * outputCount);
        var errors = NewMatrix(sampleCount, outputCount);
        var outputDeltas = NewMatrix(sampleCount, outputCount);
        for (var row = 0; row < sampleCount; row++)
        {
            for (var output = 0; output < outputCount; output++)
            {
                var error = pass.Predictions[row][output] - targets[row][output];
                errors[row][output] = error;
                outputDeltas[row][output] = scale * error * Derivative(pass.OutputRaw[row][output], pass.Predictions[row][output], outputActivation);
            }
        }

        var h2oGradients = Dot(Transpose(pass.HiddenActivations), outputDeltas);
        var outputBiasGradients = ColumnSums(outputDeltas);
        var hiddenErrors = Dot(outputDeltas, Transpose(parameters.HiddenToOutputWeights));
        var hiddenWidth = parameters.HiddenBiases.Length;
        var hiddenDeltas = NewMatrix(sampleCount, hiddenWidth);
        for (var row = 0; row < sampleCount; row++)
        {
            for (var hidden = 0; hidden < hiddenWidth; hidden++)
            {
                hiddenDeltas[row][hidden] = hiddenErrors[row][hidden] *
                    Derivative(pass.HiddenRaw[row][hidden], pass.HiddenActivations[row][hidden], hiddenActivation);
            }
        }

        var i2hGradients = Dot(Transpose(inputs), hiddenDeltas);
        var hiddenBiasGradients = ColumnSums(hiddenDeltas);
        return new TrainingStep(
            pass.Predictions,
            errors,
            outputDeltas,
            hiddenDeltas,
            h2oGradients,
            outputBiasGradients,
            i2hGradients,
            hiddenBiasGradients,
            new Parameters(
                SubtractScaled(parameters.InputToHiddenWeights, i2hGradients, learningRate),
                SubtractScaled(parameters.HiddenBiases, hiddenBiasGradients, learningRate),
                SubtractScaled(parameters.HiddenToOutputWeights, h2oGradients, learningRate),
                SubtractScaled(parameters.OutputBiases, outputBiasGradients, learningRate)
            ),
            MeanSquaredError(errors)
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

    private static double Derivative(double raw, double activated, ActivationName activation) =>
        activation switch
        {
            ActivationName.Linear => 1.0,
            ActivationName.Sigmoid => activated * (1.0 - activated),
            _ => throw new ArgumentOutOfRangeException(nameof(activation))
        };

    private static double[][] Dot(double[][] left, double[][] right)
    {
        var (rows, width) = ValidateMatrix("left", left);
        var (rightRows, cols) = ValidateMatrix("right", right);
        if (width != rightRows)
            throw new ArgumentException("matrix shapes do not align");
        var result = NewMatrix(rows, cols);
        for (var row = 0; row < rows; row++)
            for (var col = 0; col < cols; col++)
                for (var k = 0; k < width; k++)
                    result[row][col] += left[row][k] * right[k][col];
        return result;
    }

    private static double[][] Transpose(double[][] matrix)
    {
        var (rows, cols) = ValidateMatrix("matrix", matrix);
        var result = NewMatrix(cols, rows);
        for (var row = 0; row < rows; row++)
            for (var col = 0; col < cols; col++)
                result[col][row] = matrix[row][col];
        return result;
    }

    private static double[][] AddBiases(double[][] matrix, double[] biases) =>
        matrix.Select(row => row.Select((value, col) => value + biases[col]).ToArray()).ToArray();

    private static double[][] ApplyActivation(double[][] matrix, ActivationName activation) =>
        matrix.Select(row => row.Select(value => Activate(value, activation)).ToArray()).ToArray();

    private static double[] ColumnSums(double[][] matrix)
    {
        var (_, cols) = ValidateMatrix("matrix", matrix);
        var sums = new double[cols];
        foreach (var row in matrix)
            for (var col = 0; col < cols; col++)
                sums[col] += row[col];
        return sums;
    }

    private static double MeanSquaredError(double[][] errors)
    {
        var values = errors.SelectMany(row => row).ToArray();
        return values.Sum(value => value * value) / values.Length;
    }

    private static double[][] SubtractScaled(double[][] matrix, double[][] gradients, double learningRate) =>
        matrix.Select((row, rowIndex) => row.Select((value, col) => value - learningRate * gradients[rowIndex][col]).ToArray()).ToArray();

    private static double[] SubtractScaled(double[] values, double[] gradients, double learningRate) =>
        values.Select((value, index) => value - learningRate * gradients[index]).ToArray();

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

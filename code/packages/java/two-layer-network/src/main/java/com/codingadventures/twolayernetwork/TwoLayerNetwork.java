package com.codingadventures.twolayernetwork;

import java.util.Arrays;

public final class TwoLayerNetwork {
    public static final String VERSION = "0.1.0";

    public enum ActivationName { LINEAR, SIGMOID }

    public record Parameters(
        double[][] inputToHiddenWeights,
        double[] hiddenBiases,
        double[][] hiddenToOutputWeights,
        double[] outputBiases
    ) {}

    public record ForwardPass(
        double[][] hiddenRaw,
        double[][] hiddenActivations,
        double[][] outputRaw,
        double[][] predictions
    ) {}

    public record TrainingStep(
        double[][] predictions,
        double[][] errors,
        double[][] outputDeltas,
        double[][] hiddenDeltas,
        double[][] hiddenToOutputWeightGradients,
        double[] outputBiasGradients,
        double[][] inputToHiddenWeightGradients,
        double[] hiddenBiasGradients,
        Parameters nextParameters,
        double loss
    ) {}

    private TwoLayerNetwork() {}

    public static Parameters xorWarmStartParameters() {
        return new Parameters(
            new double[][]{{4.0, -4.0}, {4.0, -4.0}},
            new double[]{-2.0, 6.0},
            new double[][]{{4.0}, {4.0}},
            new double[]{-6.0}
        );
    }

    public static ForwardPass forward(double[][] inputs, Parameters parameters) {
        return forward(inputs, parameters, ActivationName.SIGMOID, ActivationName.SIGMOID);
    }

    public static ForwardPass forward(
        double[][] inputs,
        Parameters parameters,
        ActivationName hiddenActivation,
        ActivationName outputActivation
    ) {
        double[][] hiddenRaw = addBiases(dot(inputs, parameters.inputToHiddenWeights()), parameters.hiddenBiases());
        double[][] hiddenActivations = applyActivation(hiddenRaw, hiddenActivation);
        double[][] outputRaw = addBiases(dot(hiddenActivations, parameters.hiddenToOutputWeights()), parameters.outputBiases());
        double[][] predictions = applyActivation(outputRaw, outputActivation);
        return new ForwardPass(hiddenRaw, hiddenActivations, outputRaw, predictions);
    }

    public static TrainingStep trainOneEpoch(double[][] inputs, double[][] targets, Parameters parameters, double learningRate) {
        ForwardPass pass = forward(inputs, parameters);
        int sampleCount = inputs.length;
        int outputCount = targets[0].length;
        double scale = 2.0 / (sampleCount * outputCount);
        double[][] errors = newMatrix(sampleCount, outputCount);
        double[][] outputDeltas = newMatrix(sampleCount, outputCount);
        for (int row = 0; row < sampleCount; row++) {
            for (int output = 0; output < outputCount; output++) {
                double error = pass.predictions()[row][output] - targets[row][output];
                errors[row][output] = error;
                outputDeltas[row][output] = scale * error * derivative(pass.outputRaw()[row][output], pass.predictions()[row][output], ActivationName.SIGMOID);
            }
        }
        double[][] h2oGradients = dot(transpose(pass.hiddenActivations()), outputDeltas);
        double[] outputBiasGradients = columnSums(outputDeltas);
        double[][] hiddenErrors = dot(outputDeltas, transpose(parameters.hiddenToOutputWeights()));
        int hiddenWidth = parameters.hiddenBiases().length;
        double[][] hiddenDeltas = newMatrix(sampleCount, hiddenWidth);
        for (int row = 0; row < sampleCount; row++) {
            for (int hidden = 0; hidden < hiddenWidth; hidden++) {
                hiddenDeltas[row][hidden] = hiddenErrors[row][hidden] *
                    derivative(pass.hiddenRaw()[row][hidden], pass.hiddenActivations()[row][hidden], ActivationName.SIGMOID);
            }
        }
        double[][] i2hGradients = dot(transpose(inputs), hiddenDeltas);
        double[] hiddenBiasGradients = columnSums(hiddenDeltas);
        return new TrainingStep(
            pass.predictions(),
            errors,
            outputDeltas,
            hiddenDeltas,
            h2oGradients,
            outputBiasGradients,
            i2hGradients,
            hiddenBiasGradients,
            new Parameters(
                subtractScaled(parameters.inputToHiddenWeights(), i2hGradients, learningRate),
                subtractScaled(parameters.hiddenBiases(), hiddenBiasGradients, learningRate),
                subtractScaled(parameters.hiddenToOutputWeights(), h2oGradients, learningRate),
                subtractScaled(parameters.outputBiases(), outputBiasGradients, learningRate)
            ),
            meanSquaredError(errors)
        );
    }

    private static double activate(double value, ActivationName activation) {
        if (activation == ActivationName.LINEAR) return value;
        if (value >= 0.0) return 1.0 / (1.0 + Math.exp(-value));
        double z = Math.exp(value);
        return z / (1.0 + z);
    }

    private static double derivative(double raw, double activated, ActivationName activation) {
        return activation == ActivationName.LINEAR ? 1.0 : activated * (1.0 - activated);
    }

    private static double[][] dot(double[][] left, double[][] right) {
        int rows = left.length;
        int width = left[0].length;
        int cols = right[0].length;
        double[][] result = newMatrix(rows, cols);
        for (int row = 0; row < rows; row++) {
            for (int col = 0; col < cols; col++) {
                for (int k = 0; k < width; k++) {
                    result[row][col] += left[row][k] * right[k][col];
                }
            }
        }
        return result;
    }

    private static double[][] transpose(double[][] matrix) {
        double[][] result = newMatrix(matrix[0].length, matrix.length);
        for (int row = 0; row < matrix.length; row++) {
            for (int col = 0; col < matrix[0].length; col++) {
                result[col][row] = matrix[row][col];
            }
        }
        return result;
    }

    private static double[][] addBiases(double[][] matrix, double[] biases) {
        double[][] result = newMatrix(matrix.length, matrix[0].length);
        for (int row = 0; row < matrix.length; row++) {
            for (int col = 0; col < matrix[row].length; col++) {
                result[row][col] = matrix[row][col] + biases[col];
            }
        }
        return result;
    }

    private static double[][] applyActivation(double[][] matrix, ActivationName activation) {
        double[][] result = newMatrix(matrix.length, matrix[0].length);
        for (int row = 0; row < matrix.length; row++) {
            for (int col = 0; col < matrix[row].length; col++) {
                result[row][col] = activate(matrix[row][col], activation);
            }
        }
        return result;
    }

    private static double[] columnSums(double[][] matrix) {
        double[] sums = new double[matrix[0].length];
        for (double[] row : matrix) {
            for (int col = 0; col < row.length; col++) {
                sums[col] += row[col];
            }
        }
        return sums;
    }

    private static double meanSquaredError(double[][] errors) {
        double total = 0.0;
        int count = 0;
        for (double[] row : errors) {
            for (double value : row) {
                total += value * value;
                count++;
            }
        }
        return total / count;
    }

    private static double[][] subtractScaled(double[][] matrix, double[][] gradients, double learningRate) {
        double[][] result = newMatrix(matrix.length, matrix[0].length);
        for (int row = 0; row < matrix.length; row++) {
            for (int col = 0; col < matrix[row].length; col++) {
                result[row][col] = matrix[row][col] - learningRate * gradients[row][col];
            }
        }
        return result;
    }

    private static double[] subtractScaled(double[] values, double[] gradients, double learningRate) {
        double[] result = Arrays.copyOf(values, values.length);
        for (int index = 0; index < values.length; index++) {
            result[index] = values[index] - learningRate * gradients[index];
        }
        return result;
    }

    private static double[][] newMatrix(int rows, int cols) {
        double[][] result = new double[rows][cols];
        return result;
    }
}

package com.codingadventures.singlelayernetwork;

public final class SingleLayerNetwork {
    public static final String VERSION = "0.1.0";

    private double[][] weights;
    private double[] biases;
    private final ActivationName activation;

    public SingleLayerNetwork(int inputCount, int outputCount) {
        this(inputCount, outputCount, ActivationName.LINEAR);
    }

    public SingleLayerNetwork(int inputCount, int outputCount, ActivationName activation) {
        this.weights = new double[inputCount][outputCount];
        this.biases = new double[outputCount];
        this.activation = activation;
    }

    public double[][] weights() {
        return weights;
    }

    public double[] biases() {
        return biases;
    }

    public double[][] predict(double[][] inputs) {
        return predictWithParameters(inputs, weights, biases, activation);
    }

    public TrainingStep[] fit(double[][] inputs, double[][] targets, double learningRate, int epochs) {
        TrainingStep[] history = new TrainingStep[epochs];
        for (int epoch = 0; epoch < epochs; epoch++) {
            TrainingStep step = trainOneEpochWithMatrices(inputs, targets, weights, biases, learningRate, activation);
            weights = step.nextWeights();
            biases = step.nextBiases();
            history[epoch] = step;
        }
        return history;
    }

    public static double[][] predictWithParameters(double[][] inputs, double[][] weights, double[] biases, ActivationName activation) {
        int[] inputShape = validateMatrix("inputs", inputs);
        int[] weightShape = validateMatrix("weights", weights);
        int sampleCount = inputShape[0];
        int inputCount = inputShape[1];
        int outputCount = weightShape[1];
        if (inputCount != weightShape[0]) {
            throw new IllegalArgumentException("input column count must match weight row count");
        }
        if (biases.length != outputCount) {
            throw new IllegalArgumentException("bias count must match output count");
        }

        double[][] predictions = new double[sampleCount][outputCount];
        for (int row = 0; row < sampleCount; row++) {
            for (int output = 0; output < outputCount; output++) {
                double total = biases[output];
                for (int input = 0; input < inputCount; input++) {
                    total += inputs[row][input] * weights[input][output];
                }
                predictions[row][output] = activate(total, activation);
            }
        }
        return predictions;
    }

    public static TrainingStep trainOneEpochWithMatrices(
        double[][] inputs,
        double[][] targets,
        double[][] weights,
        double[] biases,
        double learningRate,
        ActivationName activation
    ) {
        int[] inputShape = validateMatrix("inputs", inputs);
        int[] targetShape = validateMatrix("targets", targets);
        int[] weightShape = validateMatrix("weights", weights);
        int sampleCount = inputShape[0];
        int inputCount = inputShape[1];
        int outputCount = targetShape[1];
        if (targetShape[0] != sampleCount) {
            throw new IllegalArgumentException("inputs and targets must have the same row count");
        }
        if (weightShape[0] != inputCount || weightShape[1] != outputCount) {
            throw new IllegalArgumentException("weights must be shaped input_count x output_count");
        }
        if (biases.length != outputCount) {
            throw new IllegalArgumentException("bias count must match output count");
        }

        double[][] predictions = predictWithParameters(inputs, weights, biases, activation);
        double scale = 2.0 / (sampleCount * outputCount);
        double[][] errors = new double[sampleCount][outputCount];
        double[][] deltas = new double[sampleCount][outputCount];
        double lossTotal = 0.0;
        for (int row = 0; row < sampleCount; row++) {
            for (int output = 0; output < outputCount; output++) {
                double error = predictions[row][output] - targets[row][output];
                errors[row][output] = error;
                deltas[row][output] = scale * error * derivativeFromOutput(predictions[row][output], activation);
                lossTotal += error * error;
            }
        }

        double[][] weightGradients = new double[inputCount][outputCount];
        double[][] nextWeights = new double[inputCount][outputCount];
        for (int input = 0; input < inputCount; input++) {
            for (int output = 0; output < outputCount; output++) {
                for (int row = 0; row < sampleCount; row++) {
                    weightGradients[input][output] += inputs[row][input] * deltas[row][output];
                }
                nextWeights[input][output] = weights[input][output] - learningRate * weightGradients[input][output];
            }
        }

        double[] biasGradients = new double[outputCount];
        double[] nextBiases = new double[outputCount];
        for (int output = 0; output < outputCount; output++) {
            for (int row = 0; row < sampleCount; row++) {
                biasGradients[output] += deltas[row][output];
            }
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

    private static double activate(double value, ActivationName activation) {
        return switch (activation) {
            case LINEAR -> value;
            case SIGMOID -> value >= 0.0
                ? 1.0 / (1.0 + Math.exp(-value))
                : Math.exp(value) / (1.0 + Math.exp(value));
        };
    }

    private static double derivativeFromOutput(double output, ActivationName activation) {
        return switch (activation) {
            case LINEAR -> 1.0;
            case SIGMOID -> output * (1.0 - output);
        };
    }

    private static int[] validateMatrix(String name, double[][] matrix) {
        if (matrix.length == 0) {
            throw new IllegalArgumentException(name + " must contain at least one row");
        }
        int width = matrix[0].length;
        if (width == 0) {
            throw new IllegalArgumentException(name + " must contain at least one column");
        }
        for (double[] row : matrix) {
            if (row.length != width) {
                throw new IllegalArgumentException(name + " must be rectangular");
            }
        }
        return new int[] {matrix.length, width};
    }
}

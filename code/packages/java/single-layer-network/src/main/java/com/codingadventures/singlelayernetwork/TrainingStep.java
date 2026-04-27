package com.codingadventures.singlelayernetwork;

public record TrainingStep(
    double[][] predictions,
    double[][] errors,
    double[][] weightGradients,
    double[] biasGradients,
    double[][] nextWeights,
    double[] nextBiases,
    double loss
) {}

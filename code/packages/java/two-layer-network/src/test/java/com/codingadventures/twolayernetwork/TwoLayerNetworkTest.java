package com.codingadventures.twolayernetwork;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class TwoLayerNetworkTest {
    private static final double[][] INPUTS = {
        {0.0, 0.0},
        {0.0, 1.0},
        {1.0, 0.0},
        {1.0, 1.0}
    };
    private static final double[][] TARGETS = {{0.0}, {1.0}, {1.0}, {0.0}};

    @Test
    void forwardPassExposesHiddenActivations() {
        var pass = TwoLayerNetwork.forward(INPUTS, TwoLayerNetwork.xorWarmStartParameters());

        assertEquals(4, pass.hiddenActivations().length);
        assertEquals(2, pass.hiddenActivations()[0].length);
        assertTrue(pass.predictions()[1][0] > 0.7);
        assertTrue(pass.predictions()[0][0] < 0.3);
    }

    @Test
    void trainingStepExposesBothLayerGradients() {
        var step = TwoLayerNetwork.trainOneEpoch(INPUTS, TARGETS, TwoLayerNetwork.xorWarmStartParameters(), 0.5);

        assertEquals(2, step.inputToHiddenWeightGradients().length);
        assertEquals(2, step.inputToHiddenWeightGradients()[0].length);
        assertEquals(2, step.hiddenToOutputWeightGradients().length);
        assertEquals(1, step.hiddenToOutputWeightGradients()[0].length);
    }
}

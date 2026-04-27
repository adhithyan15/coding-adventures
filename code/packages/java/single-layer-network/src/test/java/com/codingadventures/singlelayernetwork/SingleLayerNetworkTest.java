package com.codingadventures.singlelayernetwork;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class SingleLayerNetworkTest {
    @Test
    void oneEpochExposesMatrixGradients() {
        TrainingStep step = SingleLayerNetwork.trainOneEpochWithMatrices(
            new double[][] {{1.0, 2.0}},
            new double[][] {{3.0, 5.0}},
            new double[][] {{0.0, 0.0}, {0.0, 0.0}},
            new double[] {0.0, 0.0},
            0.1,
            ActivationName.LINEAR
        );

        assertEquals(-3.0, step.weightGradients()[0][0], 1.0e-6);
        assertEquals(-10.0, step.weightGradients()[1][1], 1.0e-6);
        assertEquals(0.3, step.nextWeights()[0][0], 1.0e-6);
        assertEquals(1.0, step.nextWeights()[1][1], 1.0e-6);
    }

    @Test
    void fitLearnsMInputsToNOutputs() {
        SingleLayerNetwork model = new SingleLayerNetwork(3, 2);
        TrainingStep[] history = model.fit(
            new double[][] {{0.0, 0.0, 1.0}, {1.0, 2.0, 1.0}, {2.0, 1.0, 1.0}},
            new double[][] {{1.0, -1.0}, {3.0, 2.0}, {4.0, 1.0}},
            0.05,
            500
        );
        assertTrue(history[history.length - 1].loss() < history[0].loss());
        assertEquals(2, model.predict(new double[][] {{1.0, 1.0, 1.0}})[0].length);
    }
}

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

    @Test
    void hiddenLayerTeachingExamplesRunOneTrainingStep() {
        var cases = new ExampleCase[] {
            new ExampleCase("XNOR", INPUTS, new double[][]{{1.0}, {0.0}, {0.0}, {1.0}}, 3),
            new ExampleCase("absolute value", new double[][]{{-1.0}, {-0.5}, {0.0}, {0.5}, {1.0}}, new double[][]{{1.0}, {0.5}, {0.0}, {0.5}, {1.0}}, 4),
            new ExampleCase("piecewise pricing", new double[][]{{0.1}, {0.3}, {0.5}, {0.7}, {0.9}}, new double[][]{{0.12}, {0.25}, {0.55}, {0.88}, {0.88}}, 4),
            new ExampleCase("circle classifier", new double[][]{{0.0, 0.0}, {0.5, 0.0}, {1.0, 1.0}, {-0.5, 0.5}, {-1.0, 0.0}}, new double[][]{{1.0}, {1.0}, {0.0}, {1.0}, {0.0}}, 5),
            new ExampleCase("two moons", new double[][]{{1.0, 0.0}, {0.0, 0.5}, {0.5, 0.85}, {0.5, -0.35}, {-1.0, 0.0}, {2.0, 0.5}}, new double[][]{{0.0}, {1.0}, {0.0}, {1.0}, {0.0}, {1.0}}, 5),
            new ExampleCase("interaction features", new double[][]{{0.2, 0.25, 0.0}, {0.6, 0.5, 1.0}, {1.0, 0.75, 1.0}, {1.0, 1.0, 0.0}}, new double[][]{{0.08}, {0.72}, {0.96}, {0.76}}, 5),
        };

        for (var example : cases) {
            var step = TwoLayerNetwork.trainOneEpoch(
                example.inputs,
                example.targets,
                sampleParameters(example.inputs[0].length, example.hiddenCount),
                0.4
            );

            assertTrue(step.loss() >= 0.0, example.name);
            assertEquals(example.inputs[0].length, step.inputToHiddenWeightGradients().length, example.name);
            assertEquals(example.hiddenCount, step.hiddenToOutputWeightGradients().length, example.name);
        }
    }

    private static TwoLayerNetwork.Parameters sampleParameters(int inputCount, int hiddenCount) {
        var inputToHidden = new double[inputCount][hiddenCount];
        for (var feature = 0; feature < inputCount; feature++) {
            for (var hidden = 0; hidden < hiddenCount; hidden++) {
                inputToHidden[feature][hidden] = 0.17 * (feature + 1) - 0.11 * (hidden + 1);
            }
        }

        var hiddenBiases = new double[hiddenCount];
        var hiddenToOutput = new double[hiddenCount][1];
        for (var hidden = 0; hidden < hiddenCount; hidden++) {
            hiddenBiases[hidden] = 0.05 * (hidden - 1);
            hiddenToOutput[hidden][0] = 0.13 * (hidden + 1) - 0.25;
        }

        return new TwoLayerNetwork.Parameters(inputToHidden, hiddenBiases, hiddenToOutput, new double[]{0.02});
    }

    private record ExampleCase(String name, double[][] inputs, double[][] targets, int hiddenCount) {}
}

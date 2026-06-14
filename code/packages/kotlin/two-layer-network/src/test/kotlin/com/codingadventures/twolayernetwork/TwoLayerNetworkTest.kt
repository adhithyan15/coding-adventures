package com.codingadventures.twolayernetwork

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class TwoLayerNetworkTest {
    private val inputs = arrayOf(
        doubleArrayOf(0.0, 0.0),
        doubleArrayOf(0.0, 1.0),
        doubleArrayOf(1.0, 0.0),
        doubleArrayOf(1.0, 1.0),
    )
    private val targets = arrayOf(doubleArrayOf(0.0), doubleArrayOf(1.0), doubleArrayOf(1.0), doubleArrayOf(0.0))

    @Test fun `forward pass exposes hidden activations`() {
        val pass = forward(inputs, xorWarmStartParameters())

        assertEquals(4, pass.hiddenActivations.size)
        assertEquals(2, pass.hiddenActivations[0].size)
        assertTrue(pass.predictions[1][0] > 0.7)
        assertTrue(pass.predictions[0][0] < 0.3)
    }

    @Test fun `training step exposes both layer gradients`() {
        val step = trainOneEpoch(inputs, targets, xorWarmStartParameters(), 0.5)

        assertEquals(2, step.inputToHiddenWeightGradients.size)
        assertEquals(2, step.inputToHiddenWeightGradients[0].size)
        assertEquals(2, step.hiddenToOutputWeightGradients.size)
        assertEquals(1, step.hiddenToOutputWeightGradients[0].size)
    }

    @Test fun `hidden layer teaching examples run one training step`() {
        val cases = listOf(
            ExampleCase("XNOR", inputs, arrayOf(doubleArrayOf(1.0), doubleArrayOf(0.0), doubleArrayOf(0.0), doubleArrayOf(1.0)), 3),
            ExampleCase("absolute value", arrayOf(doubleArrayOf(-1.0), doubleArrayOf(-0.5), doubleArrayOf(0.0), doubleArrayOf(0.5), doubleArrayOf(1.0)), arrayOf(doubleArrayOf(1.0), doubleArrayOf(0.5), doubleArrayOf(0.0), doubleArrayOf(0.5), doubleArrayOf(1.0)), 4),
            ExampleCase("piecewise pricing", arrayOf(doubleArrayOf(0.1), doubleArrayOf(0.3), doubleArrayOf(0.5), doubleArrayOf(0.7), doubleArrayOf(0.9)), arrayOf(doubleArrayOf(0.12), doubleArrayOf(0.25), doubleArrayOf(0.55), doubleArrayOf(0.88), doubleArrayOf(0.88)), 4),
            ExampleCase("circle classifier", arrayOf(doubleArrayOf(0.0, 0.0), doubleArrayOf(0.5, 0.0), doubleArrayOf(1.0, 1.0), doubleArrayOf(-0.5, 0.5), doubleArrayOf(-1.0, 0.0)), arrayOf(doubleArrayOf(1.0), doubleArrayOf(1.0), doubleArrayOf(0.0), doubleArrayOf(1.0), doubleArrayOf(0.0)), 5),
            ExampleCase("two moons", arrayOf(doubleArrayOf(1.0, 0.0), doubleArrayOf(0.0, 0.5), doubleArrayOf(0.5, 0.85), doubleArrayOf(0.5, -0.35), doubleArrayOf(-1.0, 0.0), doubleArrayOf(2.0, 0.5)), arrayOf(doubleArrayOf(0.0), doubleArrayOf(1.0), doubleArrayOf(0.0), doubleArrayOf(1.0), doubleArrayOf(0.0), doubleArrayOf(1.0)), 5),
            ExampleCase("interaction features", arrayOf(doubleArrayOf(0.2, 0.25, 0.0), doubleArrayOf(0.6, 0.5, 1.0), doubleArrayOf(1.0, 0.75, 1.0), doubleArrayOf(1.0, 1.0, 0.0)), arrayOf(doubleArrayOf(0.08), doubleArrayOf(0.72), doubleArrayOf(0.96), doubleArrayOf(0.76)), 5),
        )

        for (example in cases) {
            val step = trainOneEpoch(example.inputs, example.targets, sampleParameters(example.inputs[0].size, example.hiddenCount), 0.4)

            assertTrue(step.loss >= 0.0, example.name)
            assertEquals(example.inputs[0].size, step.inputToHiddenWeightGradients.size, example.name)
            assertEquals(example.hiddenCount, step.hiddenToOutputWeightGradients.size, example.name)
        }
    }

    private fun sampleParameters(inputCount: Int, hiddenCount: Int): Parameters =
        Parameters(
            inputToHiddenWeights = Array(inputCount) { feature ->
                DoubleArray(hiddenCount) { hidden -> 0.17 * (feature + 1) - 0.11 * (hidden + 1) }
            },
            hiddenBiases = DoubleArray(hiddenCount) { hidden -> 0.05 * (hidden - 1) },
            hiddenToOutputWeights = Array(hiddenCount) { hidden -> doubleArrayOf(0.13 * (hidden + 1) - 0.25) },
            outputBiases = doubleArrayOf(0.02),
        )

    private data class ExampleCase(
        val name: String,
        val inputs: Array<DoubleArray>,
        val targets: Array<DoubleArray>,
        val hiddenCount: Int,
    )
}

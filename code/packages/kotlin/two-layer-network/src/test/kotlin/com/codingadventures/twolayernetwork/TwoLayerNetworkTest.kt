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
}

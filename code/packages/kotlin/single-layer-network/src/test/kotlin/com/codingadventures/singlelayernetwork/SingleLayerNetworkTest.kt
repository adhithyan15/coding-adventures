package com.codingadventures.singlelayernetwork

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class SingleLayerNetworkTest {
    @Test
    fun oneEpochExposesMatrixGradients() {
        val step = trainOneEpochWithMatrices(
            inputs = arrayOf(doubleArrayOf(1.0, 2.0)),
            targets = arrayOf(doubleArrayOf(3.0, 5.0)),
            weights = arrayOf(doubleArrayOf(0.0, 0.0), doubleArrayOf(0.0, 0.0)),
            biases = doubleArrayOf(0.0, 0.0),
            learningRate = 0.1,
        )

        assertEquals(-3.0, step.weightGradients[0][0], 1.0e-6)
        assertEquals(-10.0, step.weightGradients[1][1], 1.0e-6)
        assertEquals(0.3, step.nextWeights[0][0], 1.0e-6)
        assertEquals(1.0, step.nextWeights[1][1], 1.0e-6)
    }

    @Test
    fun fitLearnsMInputsToNOutputs() {
        val model = SingleLayerNetwork(3, 2)
        val history = model.fit(
            inputs = arrayOf(doubleArrayOf(0.0, 0.0, 1.0), doubleArrayOf(1.0, 2.0, 1.0), doubleArrayOf(2.0, 1.0, 1.0)),
            targets = arrayOf(doubleArrayOf(1.0, -1.0), doubleArrayOf(3.0, 2.0), doubleArrayOf(4.0, 1.0)),
            learningRate = 0.05,
            epochs = 500,
        )
        assertTrue(history.last().loss < history.first().loss)
        assertEquals(2, model.predict(arrayOf(doubleArrayOf(1.0, 1.0, 1.0)))[0].size)
    }
}

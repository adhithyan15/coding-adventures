package com.codingadventures.neuralgraphvm

import com.codingadventures.neuralnetwork.*
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class NeuralGraphVMTest {
    private fun tinyGraph(): NeuralGraph {
        val graph = createNeuralGraph("tiny")
        addInput(graph, "x0"); addInput(graph, "x1"); addConstant(graph, "bias", 1.0)
        addWeightedSum(graph, "sum", listOf(wi("x0", 0.25, "x0_to_sum"), wi("x1", 0.75, "x1_to_sum"), wi("bias", -1.0, "bias_to_sum")))
        addActivation(graph, "relu", "sum", "relu", edgeId = "sum_to_relu")
        addOutput(graph, "out", "relu", "prediction", edgeId = "relu_to_out")
        return graph
    }
    @Test fun runsTinyWeightedSum() { assertEquals(6.0, runNeuralBytecodeForward(compileNeuralGraphToBytecode(tinyGraph()), mapOf("x0" to 4.0, "x1" to 8.0))["prediction"]!!, 1e-9) }
    @Test fun runsXor() {
        val bytecode = compileNeuralNetworkToBytecode(createXorNetwork())
        for ((x0, x1, expected) in listOf(Triple(0.0,0.0,0.0), Triple(0.0,1.0,1.0), Triple(1.0,0.0,1.0), Triple(1.0,1.0,0.0))) {
            val prediction = runNeuralBytecodeForward(bytecode, mapOf("x0" to x0, "x1" to x1))["prediction"]!!
            assertTrue(if (expected == 1.0) prediction > 0.99 else prediction < 0.01)
        }
    }
}

package com.codingadventures.neuralnetwork

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class NeuralNetworkTest {
    @Test fun buildsTinyWeightedGraph() {
        val graph = createNeuralGraph("tiny")
        addInput(graph, "x0"); addInput(graph, "x1"); addConstant(graph, "bias", 1.0)
        addWeightedSum(graph, "sum", listOf(wi("x0", 0.25, "x0_to_sum"), wi("x1", 0.75, "x1_to_sum"), wi("bias", -1.0, "bias_to_sum")))
        addActivation(graph, "relu", "sum", "relu", edgeId = "sum_to_relu")
        addOutput(graph, "out", "relu", "prediction", edgeId = "relu_to_out")
        assertEquals(3, graph.incomingEdges("sum").size)
        assertEquals("out", graph.topologicalSort().last())
    }
    @Test fun xorNetworkHasHiddenOutputEdge() { assertTrue(createXorNetwork().graph.edges.any { it.id == "h_or_to_out" }) }
}

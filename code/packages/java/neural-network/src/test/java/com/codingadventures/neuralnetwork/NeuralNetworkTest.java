package com.codingadventures.neuralnetwork;

import org.junit.jupiter.api.Test;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class NeuralNetworkTest {
    @Test void buildsTinyWeightedGraph() {
        var graph = NeuralNetwork.createNeuralGraph("tiny");
        NeuralNetwork.addInput(graph, "x0");
        NeuralNetwork.addInput(graph, "x1");
        NeuralNetwork.addConstant(graph, "bias", 1.0);
        NeuralNetwork.addWeightedSum(graph, "sum", List.of(NeuralNetwork.wi("x0", 0.25, "x0_to_sum"), NeuralNetwork.wi("x1", 0.75, "x1_to_sum"), NeuralNetwork.wi("bias", -1.0, "bias_to_sum")));
        NeuralNetwork.addActivation(graph, "relu", "sum", "relu", java.util.Map.of(), "sum_to_relu");
        NeuralNetwork.addOutput(graph, "out", "relu", "prediction", java.util.Map.of(), "relu_to_out");
        assertEquals(3, graph.incomingEdges("sum").size());
        assertEquals("out", graph.topologicalSort().getLast());
    }

    @Test void xorNetworkHasHiddenOutputEdge() {
        assertTrue(NeuralNetwork.createXorNetwork().graph().edges().stream().anyMatch(edge -> edge.id().equals("h_or_to_out")));
    }
}

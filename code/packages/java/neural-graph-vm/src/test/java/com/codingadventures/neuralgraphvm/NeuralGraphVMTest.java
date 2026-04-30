package com.codingadventures.neuralgraphvm;

import com.codingadventures.neuralnetwork.NeuralNetwork;
import org.junit.jupiter.api.Test;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

class NeuralGraphVMTest {
    private static NeuralNetwork.Graph tinyGraph() {
        var graph = NeuralNetwork.createNeuralGraph("tiny");
        NeuralNetwork.addInput(graph, "x0");
        NeuralNetwork.addInput(graph, "x1");
        NeuralNetwork.addConstant(graph, "bias", 1.0);
        NeuralNetwork.addWeightedSum(graph, "sum", List.of(NeuralNetwork.wi("x0", 0.25, "x0_to_sum"), NeuralNetwork.wi("x1", 0.75, "x1_to_sum"), NeuralNetwork.wi("bias", -1.0, "bias_to_sum")));
        NeuralNetwork.addActivation(graph, "relu", "sum", "relu", Map.of(), "sum_to_relu");
        NeuralNetwork.addOutput(graph, "out", "relu", "prediction", Map.of(), "relu_to_out");
        return graph;
    }

    @Test void runsTinyWeightedSum() {
        var outputs = NeuralGraphVM.runNeuralBytecodeForward(NeuralGraphVM.compileNeuralGraphToBytecode(tinyGraph()), Map.of("x0", 4.0, "x1", 8.0));
        assertEquals(6.0, outputs.get("prediction"), 1e-9);
    }

    @Test void runsXor() {
        var bytecode = NeuralGraphVM.compileNeuralNetworkToBytecode(NeuralNetwork.createXorNetwork());
        for (double[] c : new double[][]{{0,0,0},{0,1,1},{1,0,1},{1,1,0}}) {
            double prediction = NeuralGraphVM.runNeuralBytecodeForward(bytecode, Map.of("x0", c[0], "x1", c[1])).get("prediction");
            assertTrue(c[2] == 1.0 ? prediction > 0.99 : prediction < 0.01);
        }
    }
}

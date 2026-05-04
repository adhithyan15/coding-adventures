package com.codingadventures.neuralgraphvm;

import com.codingadventures.neuralnetwork.NeuralNetwork;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public final class NeuralGraphVM {
    private NeuralGraphVM() {}

    public record Instruction(String op, String dst, String inputName, String outputName, String edgeId, Double value, String left, String right, List<String> inputs, String input, String activation, String sourceNode, String sourceEdge) {}
    public record BytecodeFunction(String id, String kind, List<Instruction> instructions) {}
    public record GraphEdge(String id, String from, String to, double weight) {}
    public record BytecodeModule(String magic, int version, List<String> nodes, List<GraphEdge> edges, List<BytecodeFunction> functions) {}

    public static BytecodeModule compileNeuralNetworkToBytecode(NeuralNetwork.Model network) { return compileNeuralGraphToBytecode(network.graph()); }
    public static BytecodeModule compileNeuralGraphToBytecode(NeuralNetwork.Graph graph) {
        Map<String, String> values = new HashMap<>();
        int[] nextValueId = {0};
        List<Instruction> instructions = new ArrayList<>();
        for (String node : graph.topologicalSort()) {
            Map<String, Object> props = graph.nodeProperties(node);
            String op = (String) props.getOrDefault("nn.op", "weighted_sum");
            switch (op) {
                case "input" -> {
                    String dst = alloc(nextValueId); values.put(node, dst);
                    instructions.add(new Instruction("LOAD_INPUT", dst, (String) props.getOrDefault("nn.input", node), null, null, null, null, null, List.of(), null, null, node, null));
                }
                case "constant" -> {
                    String dst = alloc(nextValueId); values.put(node, dst);
                    instructions.add(new Instruction("LOAD_CONST", dst, null, null, null, ((Number) props.get("nn.value")).doubleValue(), null, null, List.of(), null, null, node, null));
                }
                case "weighted_sum" -> {
                    List<String> terms = new ArrayList<>();
                    var incoming = new ArrayList<>(graph.incomingEdges(node));
                    incoming.sort(Comparator.comparing(NeuralNetwork.Edge::id));
                    for (NeuralNetwork.Edge edge : incoming) {
                        String weightValue = alloc(nextValueId);
                        String termValue = alloc(nextValueId);
                        instructions.add(new Instruction("LOAD_EDGE_WEIGHT", weightValue, null, null, edge.id(), null, null, null, List.of(), null, null, null, edge.id()));
                        instructions.add(new Instruction("MUL", termValue, null, null, null, null, values.get(edge.from()), weightValue, List.of(), null, null, null, edge.id()));
                        terms.add(termValue);
                    }
                    String dst = alloc(nextValueId); values.put(node, dst);
                    instructions.add(terms.isEmpty()
                        ? new Instruction("LOAD_CONST", dst, null, null, null, 0.0, null, null, List.of(), null, null, node, null)
                        : new Instruction("ADD", dst, null, null, null, null, null, null, terms, null, null, node, null));
                }
                case "activation" -> {
                    String dst = alloc(nextValueId); values.put(node, dst);
                    instructions.add(new Instruction("ACTIVATE", dst, null, null, null, null, null, null, List.of(), singleInputValue(graph, values, node), (String) props.getOrDefault("nn.activation", "relu"), node, null));
                }
                case "output" -> {
                    String input = singleInputValue(graph, values, node); values.put(node, input);
                    instructions.add(new Instruction("STORE_OUTPUT", null, null, (String) props.getOrDefault("nn.output", node), null, null, null, null, List.of(), input, null, node, null));
                }
                default -> throw new IllegalArgumentException("unsupported neural graph op: " + op);
            }
        }
        return new BytecodeModule("CANN", 0, graph.nodes(), graph.edges().stream().map(edge -> new GraphEdge(edge.id(), edge.from(), edge.to(), edge.weight())).toList(), List.of(new BytecodeFunction("forward", "forward", instructions)));
    }

    public static Map<String, Double> runNeuralBytecodeForward(BytecodeModule module, Map<String, Double> inputs) {
        Map<String, Double> values = new HashMap<>();
        Map<String, Double> edgeWeights = new HashMap<>();
        for (GraphEdge edge : module.edges()) edgeWeights.put(edge.id(), edge.weight());
        Map<String, Double> outputs = new HashMap<>();
        BytecodeFunction forward = module.functions().stream().filter(fn -> fn.kind().equals("forward")).findFirst().orElseThrow();
        for (Instruction instruction : forward.instructions()) {
            switch (instruction.op()) {
                case "LOAD_INPUT" -> values.put(instruction.dst(), inputs.get(instruction.inputName()));
                case "LOAD_CONST" -> values.put(instruction.dst(), instruction.value() == null ? 0.0 : instruction.value());
                case "LOAD_EDGE_WEIGHT" -> values.put(instruction.dst(), edgeWeights.getOrDefault(instruction.edgeId(), 1.0));
                case "MUL" -> values.put(instruction.dst(), values.get(instruction.left()) * values.get(instruction.right()));
                case "ADD" -> values.put(instruction.dst(), instruction.inputs().stream().mapToDouble(values::get).sum());
                case "ACTIVATE" -> values.put(instruction.dst(), applyNeuralActivation(values.get(instruction.input()), instruction.activation()));
                case "STORE_OUTPUT" -> outputs.put(instruction.outputName(), values.get(instruction.input()));
                default -> throw new IllegalArgumentException("unsupported opcode: " + instruction.op());
            }
        }
        return outputs;
    }

    public static double applyNeuralActivation(double value, String activation) {
        return switch (activation == null ? "relu" : activation) {
            case "relu" -> value > 0 ? value : 0.0;
            case "sigmoid" -> 1.0 / (1.0 + Math.exp(-value));
            case "tanh" -> Math.tanh(value);
            default -> value;
        };
    }
    private static String alloc(int[] nextValueId) { return "v" + nextValueId[0]++; }
    private static String singleInputValue(NeuralNetwork.Graph graph, Map<String, String> values, String node) {
        var incoming = graph.incomingEdges(node);
        if (incoming.size() != 1) throw new IllegalArgumentException("node " + node + " expects exactly one input");
        return values.get(incoming.getFirst().from());
    }
}

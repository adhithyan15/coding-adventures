package com.codingadventures.neuralnetwork;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public final class NeuralNetwork {
    private NeuralNetwork() {}

    public record Edge(String id, String from, String to, double weight, Map<String, Object> properties) {}
    public record WeightedInput(String from, double weight, String edgeId, Map<String, Object> properties) {
        public WeightedInput(String from, double weight, String edgeId) {
            this(from, weight, edgeId, Map.of());
        }
    }

    public static final class Graph {
        private final Map<String, Object> graphProperties = new HashMap<>();
        private final List<String> nodes = new ArrayList<>();
        private final Map<String, Map<String, Object>> nodeProperties = new HashMap<>();
        private final List<Edge> edges = new ArrayList<>();
        private int nextEdgeId = 0;

        public Graph(String name) {
            graphProperties.put("nn.version", "0");
            if (name != null && !name.isEmpty()) graphProperties.put("nn.name", name);
        }

        public Map<String, Object> graphProperties() { return Map.copyOf(graphProperties); }
        public List<String> nodes() { return List.copyOf(nodes); }
        public List<Edge> edges() { return List.copyOf(edges); }

        public void addNode(String node, Map<String, Object> properties) {
            if (!nodeProperties.containsKey(node)) {
                nodes.add(node);
                nodeProperties.put(node, new HashMap<>());
            }
            if (properties != null) nodeProperties.get(node).putAll(properties);
        }

        public Map<String, Object> nodeProperties(String node) {
            return Map.copyOf(nodeProperties.getOrDefault(node, Map.of()));
        }

        public String addEdge(String from, String to, double weight, Map<String, Object> properties, String edgeId) {
            addNode(from, Map.of());
            addNode(to, Map.of());
            String id = edgeId != null ? edgeId : "e" + nextEdgeId++;
            Map<String, Object> props = new HashMap<>(properties == null ? Map.of() : properties);
            props.put("weight", weight);
            edges.add(new Edge(id, from, to, weight, Map.copyOf(props)));
            return id;
        }

        public List<Edge> incomingEdges(String node) {
            List<Edge> incoming = new ArrayList<>();
            for (Edge edge : edges) if (edge.to().equals(node)) incoming.add(edge);
            return incoming;
        }

        public List<String> topologicalSort() {
            Map<String, Integer> indegree = new HashMap<>();
            for (String node : nodes) indegree.put(node, 0);
            for (Edge edge : edges) {
                indegree.putIfAbsent(edge.from(), 0);
                indegree.put(edge.to(), indegree.getOrDefault(edge.to(), 0) + 1);
            }
            List<String> readyList = new ArrayList<>();
            for (Map.Entry<String, Integer> entry : indegree.entrySet()) if (entry.getValue() == 0) readyList.add(entry.getKey());
            Collections.sort(readyList);
            ArrayDeque<String> ready = new ArrayDeque<>(readyList);
            List<String> order = new ArrayList<>();
            while (!ready.isEmpty()) {
                String node = ready.removeFirst();
                order.add(node);
                List<String> released = new ArrayList<>();
                for (Edge edge : edges) {
                    if (edge.from().equals(node)) {
                        indegree.put(edge.to(), indegree.get(edge.to()) - 1);
                        if (indegree.get(edge.to()) == 0) released.add(edge.to());
                    }
                }
                Collections.sort(released);
                ready.addAll(released);
            }
            if (order.size() != indegree.size()) throw new IllegalStateException("neural graph contains a cycle");
            return order;
        }
    }

    public static final class Model {
        private final Graph graph;
        public Model(String name) { this.graph = createNeuralGraph(name); }
        public Graph graph() { return graph; }
        public Model input(String node) { addInput(graph, node, node, Map.of()); return this; }
        public Model constant(String node, double value, Map<String, Object> properties) { addConstant(graph, node, value, properties); return this; }
        public Model weightedSum(String node, List<WeightedInput> inputs, Map<String, Object> properties) { addWeightedSum(graph, node, inputs, properties); return this; }
        public Model activation(String node, String input, String activation, Map<String, Object> properties, String edgeId) { addActivation(graph, node, input, activation, properties, edgeId); return this; }
        public Model output(String node, String input, String outputName, Map<String, Object> properties, String edgeId) { addOutput(graph, node, input, outputName, properties, edgeId); return this; }
    }

    public static Graph createNeuralGraph(String name) { return new Graph(name); }
    public static Model createNeuralNetwork(String name) { return new Model(name); }

    public static void addInput(Graph graph, String node) { addInput(graph, node, node, Map.of()); }
    public static void addInput(Graph graph, String node, String inputName, Map<String, Object> properties) {
        Map<String, Object> props = merge(properties, Map.of("nn.op", "input", "nn.input", inputName));
        graph.addNode(node, props);
    }
    public static void addConstant(Graph graph, String node, double value) { addConstant(graph, node, value, Map.of()); }
    public static void addConstant(Graph graph, String node, double value, Map<String, Object> properties) {
        if (!Double.isFinite(value)) throw new IllegalArgumentException("constant value must be finite");
        graph.addNode(node, merge(properties, Map.of("nn.op", "constant", "nn.value", value)));
    }
    public static void addWeightedSum(Graph graph, String node, List<WeightedInput> inputs) { addWeightedSum(graph, node, inputs, Map.of()); }
    public static void addWeightedSum(Graph graph, String node, List<WeightedInput> inputs, Map<String, Object> properties) {
        graph.addNode(node, merge(properties, Map.of("nn.op", "weighted_sum")));
        for (WeightedInput input : inputs) graph.addEdge(input.from(), node, input.weight(), input.properties(), input.edgeId());
    }
    public static String addActivation(Graph graph, String node, String input, String activation, Map<String, Object> properties, String edgeId) {
        graph.addNode(node, merge(properties, Map.of("nn.op", "activation", "nn.activation", activation)));
        return graph.addEdge(input, node, 1.0, Map.of(), edgeId);
    }
    public static String addOutput(Graph graph, String node, String input, String outputName, Map<String, Object> properties, String edgeId) {
        graph.addNode(node, merge(properties, Map.of("nn.op", "output", "nn.output", outputName)));
        return graph.addEdge(input, node, 1.0, Map.of(), edgeId);
    }
    public static Model createXorNetwork() { return createXorNetwork("xor"); }
    public static Model createXorNetwork(String name) {
        return createNeuralNetwork(name)
            .input("x0").input("x1").constant("bias", 1.0, Map.of("nn.role", "bias"))
            .weightedSum("h_or_sum", List.of(wi("x0", 20, "x0_to_h_or"), wi("x1", 20, "x1_to_h_or"), wi("bias", -10, "bias_to_h_or")), Map.of("nn.layer", "hidden"))
            .activation("h_or", "h_or_sum", "sigmoid", Map.of("nn.layer", "hidden"), "h_or_sum_to_h_or")
            .weightedSum("h_nand_sum", List.of(wi("x0", -20, "x0_to_h_nand"), wi("x1", -20, "x1_to_h_nand"), wi("bias", 30, "bias_to_h_nand")), Map.of("nn.layer", "hidden"))
            .activation("h_nand", "h_nand_sum", "sigmoid", Map.of("nn.layer", "hidden"), "h_nand_sum_to_h_nand")
            .weightedSum("out_sum", List.of(wi("h_or", 20, "h_or_to_out"), wi("h_nand", 20, "h_nand_to_out"), wi("bias", -30, "bias_to_out")), Map.of("nn.layer", "output"))
            .activation("out_activation", "out_sum", "sigmoid", Map.of("nn.layer", "output"), "out_sum_to_activation")
            .output("out", "out_activation", "prediction", Map.of("nn.layer", "output"), "activation_to_out");
    }
    public static WeightedInput wi(String from, double weight, String edgeId) { return new WeightedInput(from, weight, edgeId); }
    private static Map<String, Object> merge(Map<String, Object> first, Map<String, Object> second) {
        Map<String, Object> merged = new HashMap<>(first == null ? Map.of() : first);
        merged.putAll(second);
        return merged;
    }
}

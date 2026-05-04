package com.codingadventures.graph;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

public final class Graph implements TraversalGraph {
    public record WeightedEdge(String leftNode, String rightNode, double weight) {}

    private final Map<String, Map<String, Double>> adjacency = new LinkedHashMap<>();
    private final Map<String, Object> graphProperties = new LinkedHashMap<>();
    private final Map<String, Map<String, Object>> nodeProperties = new LinkedHashMap<>();
    private final Map<String, Map<String, Object>> edgeProperties = new LinkedHashMap<>();

    @Override
    public int size() {
        return adjacency.size();
    }

    public void addNode(String node) {
        addNode(node, Map.of());
    }

    public void addNode(String node, Map<String, Object> properties) {
        adjacency.computeIfAbsent(node, ignored -> new LinkedHashMap<>());
        mergeNodeProperties(node, properties);
    }

    public void removeNode(String node) {
        Map<String, Double> neighbors = adjacency.get(node);
        if (neighbors == null) {
            throw new NodeNotFoundError(node);
        }

        for (String neighbor : List.copyOf(neighbors.keySet())) {
            adjacency.get(neighbor).remove(node);
            edgeProperties.remove(canonicalEdgeKey(node, neighbor));
        }
        adjacency.remove(node);
        nodeProperties.remove(node);
    }

    @Override
    public boolean hasNode(String node) {
        return adjacency.containsKey(node);
    }

    @Override
    public List<String> nodes() {
        return List.copyOf(adjacency.keySet());
    }

    public void addEdge(String leftNode, String rightNode) {
        addEdge(leftNode, rightNode, 1.0);
    }

    public void addEdge(String leftNode, String rightNode, double weight) {
        addNode(leftNode);
        addNode(rightNode);
        adjacency.get(leftNode).put(rightNode, weight);
        adjacency.get(rightNode).put(leftNode, weight);
        mergeEdgeProperties(leftNode, rightNode, weight, Map.of());
    }

    public void addEdge(String leftNode, String rightNode, double weight, Map<String, Object> properties) {
        addNode(leftNode);
        addNode(rightNode);
        adjacency.get(leftNode).put(rightNode, weight);
        adjacency.get(rightNode).put(leftNode, weight);
        mergeEdgeProperties(leftNode, rightNode, weight, properties);
    }

    public void removeEdge(String leftNode, String rightNode) {
        if (!hasEdge(leftNode, rightNode)) {
            throw new EdgeNotFoundError(leftNode, rightNode);
        }
        adjacency.get(leftNode).remove(rightNode);
        adjacency.get(rightNode).remove(leftNode);
        edgeProperties.remove(canonicalEdgeKey(leftNode, rightNode));
    }

    public boolean hasEdge(String leftNode, String rightNode) {
        return adjacency.containsKey(leftNode) && adjacency.get(leftNode).containsKey(rightNode);
    }

    public double edgeWeight(String leftNode, String rightNode) {
        Map<String, Double> neighbors = adjacency.get(leftNode);
        if (neighbors == null || !neighbors.containsKey(rightNode)) {
            throw new EdgeNotFoundError(leftNode, rightNode);
        }
        return neighbors.get(rightNode);
    }

    public Map<String, Object> graphProperties() {
        return immutablePropertyCopy(graphProperties);
    }

    public void setGraphProperty(String key, Object value) {
        graphProperties.put(key, value);
    }

    public void removeGraphProperty(String key) {
        graphProperties.remove(key);
    }

    public Map<String, Object> nodeProperties(String node) {
        if (!hasNode(node)) {
            throw new NodeNotFoundError(node);
        }
        return immutablePropertyCopy(nodeProperties.getOrDefault(node, Map.of()));
    }

    public void setNodeProperty(String node, String key, Object value) {
        if (!hasNode(node)) {
            throw new NodeNotFoundError(node);
        }
        nodeProperties.computeIfAbsent(node, ignored -> new LinkedHashMap<>()).put(key, value);
    }

    public void removeNodeProperty(String node, String key) {
        if (!hasNode(node)) {
            throw new NodeNotFoundError(node);
        }
        Map<String, Object> properties = nodeProperties.get(node);
        if (properties != null) {
            properties.remove(key);
        }
    }

    public Map<String, Object> edgeProperties(String leftNode, String rightNode) {
        if (!hasEdge(leftNode, rightNode)) {
            throw new EdgeNotFoundError(leftNode, rightNode);
        }
        Map<String, Object> copy = new LinkedHashMap<>(edgeProperties.getOrDefault(
                canonicalEdgeKey(leftNode, rightNode), Map.of()));
        copy.put("weight", edgeWeight(leftNode, rightNode));
        return Collections.unmodifiableMap(copy);
    }

    public void setEdgeProperty(String leftNode, String rightNode, String key, Object value) {
        if (!hasEdge(leftNode, rightNode)) {
            throw new EdgeNotFoundError(leftNode, rightNode);
        }
        if ("weight".equals(key)) {
            if (!(value instanceof Number number)) {
                throw new IllegalArgumentException("Edge property 'weight' must be numeric.");
            }
            setEdgeWeight(leftNode, rightNode, number.doubleValue());
        }
        edgeProperties.computeIfAbsent(canonicalEdgeKey(leftNode, rightNode), ignored -> new LinkedHashMap<>())
                .put(key, value);
    }

    public void removeEdgeProperty(String leftNode, String rightNode, String key) {
        if (!hasEdge(leftNode, rightNode)) {
            throw new EdgeNotFoundError(leftNode, rightNode);
        }
        if ("weight".equals(key)) {
            setEdgeWeight(leftNode, rightNode, 1.0);
            edgeProperties.computeIfAbsent(canonicalEdgeKey(leftNode, rightNode), ignored -> new LinkedHashMap<>())
                    .put("weight", 1.0);
            return;
        }
        Map<String, Object> properties = edgeProperties.get(canonicalEdgeKey(leftNode, rightNode));
        if (properties != null) {
            properties.remove(key);
        }
    }

    public List<WeightedEdge> edges() {
        Set<String> seen = new LinkedHashSet<>();
        List<WeightedEdge> result = new ArrayList<>();
        for (Map.Entry<String, Map<String, Double>> entry : adjacency.entrySet()) {
            for (Map.Entry<String, Double> neighbor : entry.getValue().entrySet()) {
                String leftNode = entry.getKey();
                String rightNode = neighbor.getKey();
                String edgeKey = canonicalEdgeKey(leftNode, rightNode);
                if (seen.add(edgeKey)) {
                    if (leftNode.compareTo(rightNode) <= 0) {
                        result.add(new WeightedEdge(leftNode, rightNode, neighbor.getValue()));
                    } else {
                        result.add(new WeightedEdge(rightNode, leftNode, neighbor.getValue()));
                    }
                }
            }
        }
        return result;
    }

    @Override
    public List<String> neighbors(String node) {
        Map<String, Double> neighbors = adjacency.get(node);
        if (neighbors == null) {
            throw new NodeNotFoundError(node);
        }
        return List.copyOf(neighbors.keySet());
    }

    public List<String> breadthFirst(String startNode) {
        return Traversals.breadthFirst(this, startNode);
    }

    public List<String> depthFirst(String startNode) {
        return Traversals.depthFirst(this, startNode);
    }

    public List<Set<String>> connectedComponents() {
        return Traversals.connectedComponents(this);
    }

    public boolean isConnected() {
        return Traversals.isConnected(this);
    }

    private static String canonicalEdgeKey(String leftNode, String rightNode) {
        return leftNode.compareTo(rightNode) <= 0
                ? leftNode + "\0" + rightNode
                : rightNode + "\0" + leftNode;
    }

    private static Map<String, Object> immutablePropertyCopy(Map<String, Object> properties) {
        return Collections.unmodifiableMap(new LinkedHashMap<>(properties));
    }

    private void mergeNodeProperties(String node, Map<String, Object> properties) {
        if (properties == null) {
            return;
        }
        nodeProperties.computeIfAbsent(node, ignored -> new LinkedHashMap<>()).putAll(properties);
    }

    private void mergeEdgeProperties(String leftNode, String rightNode, double weight, Map<String, Object> properties) {
        Map<String, Object> target = edgeProperties.computeIfAbsent(canonicalEdgeKey(leftNode, rightNode),
                ignored -> new LinkedHashMap<>());
        if (properties != null) {
            target.putAll(properties);
        }
        target.put("weight", weight);
    }

    private void setEdgeWeight(String leftNode, String rightNode, double weight) {
        adjacency.get(leftNode).put(rightNode, weight);
        adjacency.get(rightNode).put(leftNode, weight);
    }
}

package com.codingadventures.graph;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

public final class Graph implements TraversalGraph {
    public record WeightedEdge(String leftNode, String rightNode, double weight) {}

    private final Map<String, Map<String, Double>> adjacency = new LinkedHashMap<>();

    @Override
    public int size() {
        return adjacency.size();
    }

    public void addNode(String node) {
        adjacency.computeIfAbsent(node, ignored -> new LinkedHashMap<>());
    }

    public void removeNode(String node) {
        Map<String, Double> neighbors = adjacency.get(node);
        if (neighbors == null) {
            throw new NodeNotFoundError(node);
        }

        for (String neighbor : List.copyOf(neighbors.keySet())) {
            adjacency.get(neighbor).remove(node);
        }
        adjacency.remove(node);
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
    }

    public void removeEdge(String leftNode, String rightNode) {
        if (!hasEdge(leftNode, rightNode)) {
            throw new EdgeNotFoundError(leftNode, rightNode);
        }
        adjacency.get(leftNode).remove(rightNode);
        adjacency.get(rightNode).remove(leftNode);
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
}

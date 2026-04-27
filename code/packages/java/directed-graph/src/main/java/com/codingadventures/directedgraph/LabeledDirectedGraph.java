package com.codingadventures.directedgraph;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

public final class LabeledDirectedGraph {
    private final Graph graph = new Graph(true);
    private final Map<String, Set<String>> labels = new LinkedHashMap<>();

    public void addNode(String node) {
        graph.addNode(node);
    }

    public void removeNode(String node) {
        if (!graph.hasNode(node)) {
            throw new NodeNotFoundError(node);
        }
        for (String successor : graph.successors(node)) {
            labels.remove(edgeKey(node, successor));
        }
        for (String predecessor : graph.predecessors(node)) {
            labels.remove(edgeKey(predecessor, node));
        }
        labels.remove(edgeKey(node, node));
        graph.removeNode(node);
    }

    public boolean hasNode(String node) {
        return graph.hasNode(node);
    }

    public List<String> nodes() {
        return graph.nodes();
    }

    public int size() {
        return graph.size();
    }

    public void addEdge(String fromNode, String toNode, String label) {
        if (!graph.hasEdge(fromNode, toNode)) {
            graph.addEdge(fromNode, toNode);
        }
        labels.computeIfAbsent(edgeKey(fromNode, toNode), ignored -> new LinkedHashSet<>()).add(label);
    }

    public void removeEdge(String fromNode, String toNode, String label) {
        if (!graph.hasNode(fromNode)) {
            throw new NodeNotFoundError(fromNode);
        }
        if (!graph.hasNode(toNode)) {
            throw new NodeNotFoundError(toNode);
        }
        Set<String> labelSet = labels.get(edgeKey(fromNode, toNode));
        if (labelSet == null || !labelSet.remove(label)) {
            throw new EdgeNotFoundError(fromNode, toNode);
        }
        if (labelSet.isEmpty()) {
            labels.remove(edgeKey(fromNode, toNode));
            graph.removeEdge(fromNode, toNode);
        }
    }

    public boolean hasEdge(String fromNode, String toNode) {
        return graph.hasEdge(fromNode, toNode);
    }

    public boolean hasEdge(String fromNode, String toNode, String label) {
        return labels.getOrDefault(edgeKey(fromNode, toNode), Set.of()).contains(label);
    }

    public Set<String> labels(String fromNode, String toNode) {
        return Set.copyOf(labels.getOrDefault(edgeKey(fromNode, toNode), Set.of()));
    }

    public List<String> successors(String node) {
        return graph.successors(node);
    }

    public List<String> successors(String node, String label) {
        List<String> matches = new ArrayList<>();
        for (String successor : successors(node)) {
            if (hasEdge(node, successor, label)) {
                matches.add(successor);
            }
        }
        return matches;
    }

    public Set<String> transitiveClosure(String startNode) {
        return graph.transitiveClosure(startNode);
    }

    public List<String> topologicalSort() {
        return graph.topologicalSort();
    }

    private static String edgeKey(String fromNode, String toNode) {
        return fromNode + "\0" + toNode;
    }
}

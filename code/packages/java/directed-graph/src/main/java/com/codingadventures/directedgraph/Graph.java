package com.codingadventures.directedgraph;

import com.codingadventures.graph.TraversalGraph;
import com.codingadventures.graph.Traversals;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

public final class Graph implements TraversalGraph {
    private final Map<String, Set<String>> forward = new LinkedHashMap<>();
    private final Map<String, Set<String>> reverse = new LinkedHashMap<>();
    private final boolean allowSelfLoops;

    public Graph() {
        this(false);
    }

    public Graph(boolean allowSelfLoops) {
        this.allowSelfLoops = allowSelfLoops;
    }

    public boolean allowSelfLoops() {
        return allowSelfLoops;
    }

    public int size() {
        return forward.size();
    }

    public void addNode(String node) {
        forward.computeIfAbsent(node, ignored -> new LinkedHashSet<>());
        reverse.computeIfAbsent(node, ignored -> new LinkedHashSet<>());
    }

    public void removeNode(String node) {
        if (!forward.containsKey(node)) {
            throw new NodeNotFoundError(node);
        }
        for (String successor : List.copyOf(forward.get(node))) {
            reverse.get(successor).remove(node);
        }
        for (String predecessor : List.copyOf(reverse.get(node))) {
            forward.get(predecessor).remove(node);
        }
        forward.remove(node);
        reverse.remove(node);
    }

    public boolean hasNode(String node) {
        return forward.containsKey(node);
    }

    public List<String> nodes() {
        return List.copyOf(forward.keySet());
    }

    public void addEdge(String fromNode, String toNode) {
        if (fromNode.equals(toNode) && !allowSelfLoops) {
            throw new IllegalArgumentException("Self-loops are not allowed: \"" + fromNode + "\" -> \"" + toNode + "\"");
        }
        addNode(fromNode);
        addNode(toNode);
        forward.get(fromNode).add(toNode);
        reverse.get(toNode).add(fromNode);
    }

    public void removeEdge(String fromNode, String toNode) {
        if (!hasNode(fromNode)) {
            throw new NodeNotFoundError(fromNode);
        }
        if (!hasNode(toNode) || !forward.get(fromNode).contains(toNode)) {
            throw new EdgeNotFoundError(fromNode, toNode);
        }
        forward.get(fromNode).remove(toNode);
        reverse.get(toNode).remove(fromNode);
    }

    public boolean hasEdge(String fromNode, String toNode) {
        return hasNode(fromNode) && forward.get(fromNode).contains(toNode);
    }

    public List<String> successors(String node) {
        if (!hasNode(node)) {
            throw new NodeNotFoundError(node);
        }
        return List.copyOf(forward.get(node));
    }

    public List<String> predecessors(String node) {
        if (!hasNode(node)) {
            throw new NodeNotFoundError(node);
        }
        return List.copyOf(reverse.get(node));
    }

    @Override
    public List<String> neighbors(String node) {
        return successors(node);
    }

    public Set<String> transitiveClosure(String startNode) {
        return new LinkedHashSet<>(Traversals.reachable(this, startNode));
    }

    public Set<String> transitiveDependents(String startNode) {
        return new LinkedHashSet<>(Traversals.reachable(new ReverseTraversalGraph(), startNode));
    }

    public List<String> topologicalSort() {
        Map<String, Integer> indegree = new LinkedHashMap<>();
        for (String node : forward.keySet()) {
            indegree.put(node, reverse.get(node).size());
        }

        ArrayDeque<String> queue = new ArrayDeque<>();
        indegree.forEach((node, degree) -> {
            if (degree == 0) {
                queue.addLast(node);
            }
        });

        List<String> order = new ArrayList<>();
        while (!queue.isEmpty()) {
            String node = queue.removeFirst();
            order.add(node);
            for (String successor : forward.get(node)) {
                int nextDegree = indegree.get(successor) - 1;
                indegree.put(successor, nextDegree);
                if (nextDegree == 0) {
                    queue.addLast(successor);
                }
            }
        }

        if (order.size() != forward.size()) {
            throw new CycleError("Graph contains a cycle", findCycle());
        }
        return order;
    }

    private List<String> findCycle() {
        Set<String> visited = new LinkedHashSet<>();
        Set<String> onStack = new LinkedHashSet<>();
        Map<String, String> parent = new LinkedHashMap<>();

        for (String node : forward.keySet()) {
            List<String> cycle = findCycleFrom(node, visited, onStack, parent);
            if (cycle != null) {
                return cycle;
            }
        }
        return List.of();
    }

    private List<String> findCycleFrom(String node, Set<String> visited, Set<String> onStack, Map<String, String> parent) {
        if (onStack.contains(node)) {
            List<String> cycle = new ArrayList<>();
            cycle.add(node);
            String current = parent.get(node);
            while (current != null && !current.equals(node)) {
                cycle.add(0, current);
                current = parent.get(current);
            }
            cycle.add(node);
            return cycle;
        }
        if (!visited.add(node)) {
            return null;
        }

        onStack.add(node);
        for (String successor : forward.get(node)) {
            parent.put(successor, node);
            List<String> cycle = findCycleFrom(successor, visited, onStack, parent);
            if (cycle != null) {
                return cycle;
            }
        }
        onStack.remove(node);
        return null;
    }

    private final class ReverseTraversalGraph implements TraversalGraph {
        @Override
        public boolean hasNode(String node) {
            return Graph.this.hasNode(node);
        }

        @Override
        public List<String> nodes() {
            return Graph.this.nodes();
        }

        @Override
        public List<String> neighbors(String node) {
            return Graph.this.predecessors(node);
        }

        @Override
        public int size() {
            return Graph.this.size();
        }
    }
}

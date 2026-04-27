package com.codingadventures.graph;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Set;

public final class Traversals {
    private Traversals() {}

    public static List<String> breadthFirst(TraversalGraph graph, String startNode) {
        ensureNode(graph, startNode);

        Set<String> visited = new LinkedHashSet<>();
        ArrayDeque<String> queue = new ArrayDeque<>();
        List<String> order = new ArrayList<>();
        visited.add(startNode);
        queue.addLast(startNode);

        while (!queue.isEmpty()) {
            String node = queue.removeFirst();
            order.add(node);
            for (String neighbor : graph.neighbors(node)) {
                if (visited.add(neighbor)) {
                    queue.addLast(neighbor);
                }
            }
        }

        return order;
    }

    public static List<String> depthFirst(TraversalGraph graph, String startNode) {
        ensureNode(graph, startNode);

        Set<String> visited = new LinkedHashSet<>();
        ArrayDeque<String> stack = new ArrayDeque<>();
        List<String> order = new ArrayList<>();
        stack.addLast(startNode);

        while (!stack.isEmpty()) {
            String node = stack.removeLast();
            if (!visited.add(node)) {
                continue;
            }
            order.add(node);

            List<String> neighbors = graph.neighbors(node);
            for (int index = neighbors.size() - 1; index >= 0; index--) {
                String neighbor = neighbors.get(index);
                if (!visited.contains(neighbor)) {
                    stack.addLast(neighbor);
                }
            }
        }

        return order;
    }

    public static Set<String> reachable(TraversalGraph graph, String startNode) {
        LinkedHashSet<String> reachable = new LinkedHashSet<>(breadthFirst(graph, startNode));
        reachable.remove(startNode);
        return reachable;
    }

    public static boolean isConnected(TraversalGraph graph) {
        if (graph.size() == 0) {
            return true;
        }
        return breadthFirst(graph, graph.nodes().get(0)).size() == graph.size();
    }

    public static List<Set<String>> connectedComponents(TraversalGraph graph) {
        LinkedHashSet<String> remaining = new LinkedHashSet<>(graph.nodes());
        List<Set<String>> components = new ArrayList<>();

        while (!remaining.isEmpty()) {
            String startNode = remaining.iterator().next();
            LinkedHashSet<String> component = new LinkedHashSet<>(breadthFirst(graph, startNode));
            components.add(component);
            remaining.removeAll(component);
        }

        return components;
    }

    private static void ensureNode(TraversalGraph graph, String node) {
        if (!graph.hasNode(node)) {
            throw new NodeNotFoundError(node);
        }
    }
}

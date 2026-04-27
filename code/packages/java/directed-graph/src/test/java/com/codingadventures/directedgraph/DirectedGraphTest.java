package com.codingadventures.directedgraph;

import org.junit.jupiter.api.Test;

import java.util.List;
import java.util.Set;

import static org.junit.jupiter.api.Assertions.*;

class DirectedGraphTest {
    @Test
    void topologicalSortOrdersDag() {
        Graph graph = new Graph();
        graph.addEdge("compile", "link");
        graph.addEdge("link", "package");

        assertEquals(List.of("compile", "link", "package"), graph.topologicalSort());
    }

    @Test
    void detectsCycles() {
        Graph graph = new Graph();
        graph.addEdge("A", "B");
        graph.addEdge("B", "C");
        graph.addEdge("C", "A");

        assertThrows(CycleError.class, graph::topologicalSort);
    }

    @Test
    void computesTransitiveClosure() {
        Graph graph = new Graph();
        graph.addEdge("A", "B");
        graph.addEdge("B", "C");
        graph.addEdge("A", "D");

        assertEquals(Set.of("B", "C", "D"), graph.transitiveClosure("A"));
    }

    @Test
    void labeledGraphFiltersByLabel() {
        LabeledDirectedGraph graph = new LabeledDirectedGraph();
        graph.addEdge("locked", "unlocked", "coin");
        graph.addEdge("locked", "locked", "push");

        assertTrue(graph.hasEdge("locked", "unlocked", "coin"));
        assertEquals(List.of("unlocked"), graph.successors("locked", "coin"));
        assertEquals(Set.of("push"), graph.labels("locked", "locked"));
    }
}

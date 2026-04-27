package com.codingadventures.graph;

import org.junit.jupiter.api.Test;

import java.util.List;
import java.util.Set;

import static org.junit.jupiter.api.Assertions.*;

class GraphTest {

    @Test
    void undirectedEdgesAreSymmetric() {
        Graph graph = new Graph();
        graph.addEdge("A", "B", 2.5);

        assertTrue(graph.hasEdge("A", "B"));
        assertTrue(graph.hasEdge("B", "A"));
        assertEquals(2.5, graph.edgeWeight("A", "B"));
        assertEquals(2.5, graph.edgeWeight("B", "A"));
    }

    @Test
    void traversalsUseSharedHelpers() {
        Graph graph = new Graph();
        graph.addEdge("A", "B");
        graph.addEdge("B", "C");
        graph.addEdge("A", "D");

        assertEquals(List.of("A", "B", "D", "C"), graph.breadthFirst("A"));
        assertEquals(List.of("A", "B", "C", "D"), graph.depthFirst("A"));
    }

    @Test
    void connectedComponentsSplitDisconnectedSubgraphs() {
        Graph graph = new Graph();
        graph.addEdge("A", "B");
        graph.addEdge("C", "D");
        graph.addNode("E");

        List<Set<String>> components = graph.connectedComponents();
        assertEquals(3, components.size());
        assertEquals(Set.of("A", "B"), components.get(0));
        assertEquals(Set.of("C", "D"), components.get(1));
        assertEquals(Set.of("E"), components.get(2));
        assertFalse(graph.isConnected());
    }

    @Test
    void removingMissingNodeThrows() {
        Graph graph = new Graph();
        assertThrows(NodeNotFoundError.class, () -> graph.removeNode("missing"));
    }
}

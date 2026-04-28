package com.codingadventures.graph;

import org.junit.jupiter.api.Test;

import java.util.List;
import java.util.Map;
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

    @Test
    void propertyBagsTrackGraphNodeAndEdgeMetadata() {
        Graph graph = new Graph();

        graph.setGraphProperty("name", "city-map");
        graph.setGraphProperty("version", 1);
        assertEquals("city-map", graph.graphProperties().get("name"));
        assertEquals(1, graph.graphProperties().get("version"));
        graph.removeGraphProperty("version");
        assertFalse(graph.graphProperties().containsKey("version"));

        graph.addNode("A", Map.of("kind", "input"));
        graph.addNode("A", Map.of("trainable", false));
        graph.setNodeProperty("A", "slot", 0);
        assertEquals("input", graph.nodeProperties("A").get("kind"));
        assertEquals(false, graph.nodeProperties("A").get("trainable"));
        assertEquals(0, graph.nodeProperties("A").get("slot"));
        graph.removeNodeProperty("A", "slot");
        assertFalse(graph.nodeProperties("A").containsKey("slot"));

        graph.addEdge("A", "B", 2.5, Map.of("role", "distance"));
        assertEquals("distance", graph.edgeProperties("B", "A").get("role"));
        assertEquals(2.5, graph.edgeProperties("B", "A").get("weight"));
        graph.setEdgeProperty("B", "A", "weight", 7.0);
        assertEquals(7.0, graph.edgeWeight("A", "B"));
        graph.setEdgeProperty("A", "B", "trainable", true);
        graph.removeEdgeProperty("A", "B", "role");
        assertEquals(true, graph.edgeProperties("A", "B").get("trainable"));
        assertFalse(graph.edgeProperties("A", "B").containsKey("role"));

        graph.removeEdge("A", "B");
        assertThrows(EdgeNotFoundError.class, () -> graph.edgeProperties("A", "B"));
    }
}

package com.codingadventures.graph

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlin.test.assertFailsWith

class GraphTest {

    @Test
    fun undirectedEdgesAreSymmetric() {
        val graph = Graph()
        graph.addEdge("A", "B", 2.5)

        assertTrue(graph.hasEdge("A", "B"))
        assertTrue(graph.hasEdge("B", "A"))
        assertEquals(2.5, graph.edgeWeight("A", "B"))
        assertEquals(2.5, graph.edgeWeight("B", "A"))
    }

    @Test
    fun traversalsUseSharedHelpers() {
        val graph = Graph()
        graph.addEdge("A", "B")
        graph.addEdge("B", "C")
        graph.addEdge("A", "D")

        assertEquals(listOf("A", "B", "D", "C"), graph.breadthFirst("A"))
        assertEquals(listOf("A", "B", "C", "D"), graph.depthFirst("A"))
    }

    @Test
    fun connectedComponentsSplitDisconnectedSubgraphs() {
        val graph = Graph()
        graph.addEdge("A", "B")
        graph.addEdge("C", "D")
        graph.addNode("E")

        val components = graph.connectedComponents()
        assertEquals(3, components.size)
        assertEquals(setOf("A", "B"), components[0])
        assertEquals(setOf("C", "D"), components[1])
        assertEquals(setOf("E"), components[2])
        assertFalse(graph.isConnected())
    }

    @Test
    fun removingMissingNodeThrows() {
        val graph = Graph()
        assertFailsWith<NodeNotFoundError> { graph.removeNode("missing") }
    }
}

package com.codingadventures.directedgraph

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class DirectedGraphTest {
    @Test
    fun topologicalSortOrdersDag() {
        val graph = Graph()
        graph.addEdge("compile", "link")
        graph.addEdge("link", "package")

        assertEquals(listOf("compile", "link", "package"), graph.topologicalSort())
    }

    @Test
    fun detectsCycles() {
        val graph = Graph()
        graph.addEdge("A", "B")
        graph.addEdge("B", "C")
        graph.addEdge("C", "A")

        assertFailsWith<CycleError> { graph.topologicalSort() }
    }

    @Test
    fun computesTransitiveClosure() {
        val graph = Graph()
        graph.addEdge("A", "B")
        graph.addEdge("B", "C")
        graph.addEdge("A", "D")

        assertEquals(setOf("B", "C", "D"), graph.transitiveClosure("A"))
    }

    @Test
    fun labeledGraphFiltersByLabel() {
        val graph = LabeledDirectedGraph()
        graph.addEdge("locked", "unlocked", "coin")
        graph.addEdge("locked", "locked", "push")

        assertTrue(graph.hasEdge("locked", "unlocked", "coin"))
        assertEquals(listOf("unlocked"), graph.successors("locked", "coin"))
        assertEquals(setOf("push"), graph.labels("locked", "locked"))
    }
}

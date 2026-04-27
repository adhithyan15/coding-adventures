package com.codingadventures.directedgraph

import com.codingadventures.graph.TraversalGraph
import com.codingadventures.graph.Traversals

class CycleError(message: String, val cycle: List<String>) : RuntimeException(message)
class NodeNotFoundError(val node: String) : RuntimeException("Node not found: \"$node\"")
class EdgeNotFoundError(val fromNode: String, val toNode: String) : RuntimeException("Edge not found: \"$fromNode\" -> \"$toNode\"")

class Graph(private val allowSelfLoops: Boolean = false) : TraversalGraph {
    private val forward = linkedMapOf<String, MutableSet<String>>()
    private val reverse = linkedMapOf<String, MutableSet<String>>()

    override val size: Int get() = forward.size

    fun addNode(node: String) {
        forward.getOrPut(node) { linkedSetOf() }
        reverse.getOrPut(node) { linkedSetOf() }
    }

    fun removeNode(node: String) {
        if (!forward.containsKey(node)) throw NodeNotFoundError(node)
        forward.getValue(node).toList().forEach { successor -> reverse.getValue(successor).remove(node) }
        reverse.getValue(node).toList().forEach { predecessor -> forward.getValue(predecessor).remove(node) }
        forward.remove(node)
        reverse.remove(node)
    }

    override fun hasNode(node: String): Boolean = forward.containsKey(node)
    override fun nodes(): List<String> = forward.keys.toList()

    fun addEdge(fromNode: String, toNode: String) {
        if (fromNode == toNode && !allowSelfLoops) {
            throw IllegalArgumentException("Self-loops are not allowed: \"$fromNode\" -> \"$toNode\"")
        }
        addNode(fromNode)
        addNode(toNode)
        forward.getValue(fromNode).add(toNode)
        reverse.getValue(toNode).add(fromNode)
    }

    fun removeEdge(fromNode: String, toNode: String) {
        if (!hasNode(fromNode)) throw NodeNotFoundError(fromNode)
        if (!hasNode(toNode) || toNode !in forward.getValue(fromNode)) throw EdgeNotFoundError(fromNode, toNode)
        forward.getValue(fromNode).remove(toNode)
        reverse.getValue(toNode).remove(fromNode)
    }

    fun hasEdge(fromNode: String, toNode: String): Boolean = hasNode(fromNode) && toNode in forward.getValue(fromNode)

    fun successors(node: String): List<String> {
        if (!hasNode(node)) throw NodeNotFoundError(node)
        return forward.getValue(node).toList()
    }

    fun predecessors(node: String): List<String> {
        if (!hasNode(node)) throw NodeNotFoundError(node)
        return reverse.getValue(node).toList()
    }

    override fun neighbors(node: String): List<String> = successors(node)

    fun transitiveClosure(startNode: String): Set<String> = Traversals.reachable(this, startNode)

    fun transitiveDependents(startNode: String): Set<String> =
        Traversals.reachable(object : TraversalGraph {
            override fun hasNode(node: String): Boolean = this@Graph.hasNode(node)
            override fun nodes(): List<String> = this@Graph.nodes()
            override fun neighbors(node: String): List<String> = this@Graph.predecessors(node)
            override val size: Int get() = this@Graph.size
        }, startNode)

    fun topologicalSort(): List<String> {
        val indegree = forward.keys.associateWithTo(linkedMapOf()) { reverse.getValue(it).size }
        val queue = ArrayDeque(indegree.filterValues { it == 0 }.keys)
        val order = mutableListOf<String>()

        while (queue.isNotEmpty()) {
            val node = queue.removeFirst()
            order += node
            forward.getValue(node).forEach { successor ->
                val nextDegree = indegree.getValue(successor) - 1
                indegree[successor] = nextDegree
                if (nextDegree == 0) queue += successor
            }
        }

        if (order.size != forward.size) {
            throw CycleError("Graph contains a cycle", findCycle())
        }
        return order
    }

    private fun findCycle(): List<String> {
        val visited = linkedSetOf<String>()
        val onStack = linkedSetOf<String>()
        val parent = linkedMapOf<String, String>()
        forward.keys.forEach { node ->
            findCycleFrom(node, visited, onStack, parent)?.let { return it }
        }
        return emptyList()
    }

    private fun findCycleFrom(
        node: String,
        visited: MutableSet<String>,
        onStack: MutableSet<String>,
        parent: MutableMap<String, String>,
    ): List<String>? {
        if (node in onStack) {
            val cycle = mutableListOf(node)
            var current = parent[node]
            while (current != null && current != node) {
                cycle.add(0, current)
                current = parent[current]
            }
            cycle += node
            return cycle
        }
        if (!visited.add(node)) return null

        onStack += node
        forward.getValue(node).forEach { successor ->
            parent[successor] = node
            findCycleFrom(successor, visited, onStack, parent)?.let { return it }
        }
        onStack.remove(node)
        return null
    }
}

class LabeledDirectedGraph {
    private val graph = Graph(allowSelfLoops = true)
    private val labels = linkedMapOf<String, MutableSet<String>>()

    val size: Int get() = graph.size

    fun addNode(node: String) = graph.addNode(node)
    fun hasNode(node: String): Boolean = graph.hasNode(node)
    fun nodes(): List<String> = graph.nodes()

    fun removeNode(node: String) {
        if (!graph.hasNode(node)) throw NodeNotFoundError(node)
        graph.successors(node).forEach { labels.remove(edgeKey(node, it)) }
        graph.predecessors(node).forEach { labels.remove(edgeKey(it, node)) }
        labels.remove(edgeKey(node, node))
        graph.removeNode(node)
    }

    fun addEdge(fromNode: String, toNode: String, label: String) {
        if (!graph.hasEdge(fromNode, toNode)) {
            graph.addEdge(fromNode, toNode)
        }
        labels.getOrPut(edgeKey(fromNode, toNode)) { linkedSetOf() }.add(label)
    }

    fun removeEdge(fromNode: String, toNode: String, label: String) {
        if (!graph.hasNode(fromNode)) throw NodeNotFoundError(fromNode)
        if (!graph.hasNode(toNode)) throw NodeNotFoundError(toNode)
        val labelSet = labels[edgeKey(fromNode, toNode)] ?: throw EdgeNotFoundError(fromNode, toNode)
        if (!labelSet.remove(label)) throw EdgeNotFoundError(fromNode, toNode)
        if (labelSet.isEmpty()) {
            labels.remove(edgeKey(fromNode, toNode))
            graph.removeEdge(fromNode, toNode)
        }
    }

    fun hasEdge(fromNode: String, toNode: String): Boolean = graph.hasEdge(fromNode, toNode)
    fun hasEdge(fromNode: String, toNode: String, label: String): Boolean = labels[edgeKey(fromNode, toNode)]?.contains(label) == true
    fun labels(fromNode: String, toNode: String): Set<String> = labels[edgeKey(fromNode, toNode)]?.toSet() ?: emptySet()
    fun successors(node: String): List<String> = graph.successors(node)
    fun successors(node: String, label: String): List<String> = graph.successors(node).filter { hasEdge(node, it, label) }
    fun transitiveClosure(startNode: String): Set<String> = graph.transitiveClosure(startNode)
    fun topologicalSort(): List<String> = graph.topologicalSort()

    private fun edgeKey(fromNode: String, toNode: String): String = "$fromNode\u0000$toNode"
}

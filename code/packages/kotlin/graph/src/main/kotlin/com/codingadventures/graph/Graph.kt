package com.codingadventures.graph

class NodeNotFoundError(node: String) : RuntimeException("Node not found: \"$node\"")
class EdgeNotFoundError(leftNode: String, rightNode: String) :
    RuntimeException("Edge not found: \"$leftNode\" -- \"$rightNode\"")

interface TraversalGraph {
    fun hasNode(node: String): Boolean
    fun nodes(): List<String>
    fun neighbors(node: String): List<String>
    val size: Int
}

object Traversals {
    fun breadthFirst(graph: TraversalGraph, startNode: String): List<String> {
        ensureNode(graph, startNode)

        val visited = linkedSetOf(startNode)
        val queue = ArrayDeque(listOf(startNode))
        val order = mutableListOf<String>()

        while (queue.isNotEmpty()) {
            val node = queue.removeFirst()
            order += node
            graph.neighbors(node).forEach { neighbor ->
                if (visited.add(neighbor)) {
                    queue += neighbor
                }
            }
        }

        return order
    }

    fun depthFirst(graph: TraversalGraph, startNode: String): List<String> {
        ensureNode(graph, startNode)

        val visited = linkedSetOf<String>()
        val stack = ArrayDeque(listOf(startNode))
        val order = mutableListOf<String>()

        while (stack.isNotEmpty()) {
            val node = stack.removeLast()
            if (!visited.add(node)) continue
            order += node

            graph.neighbors(node).asReversed().forEach { neighbor ->
                if (neighbor !in visited) {
                    stack += neighbor
                }
            }
        }

        return order
    }

    fun reachable(graph: TraversalGraph, startNode: String): Set<String> =
        breadthFirst(graph, startNode).drop(1).toCollection(linkedSetOf())

    fun isConnected(graph: TraversalGraph): Boolean {
        if (graph.size == 0) return true
        return breadthFirst(graph, graph.nodes().first()).size == graph.size
    }

    fun connectedComponents(graph: TraversalGraph): List<Set<String>> {
        val remaining = graph.nodes().toCollection(linkedSetOf())
        val components = mutableListOf<Set<String>>()

        while (remaining.isNotEmpty()) {
            val startNode = remaining.first()
            val component = breadthFirst(graph, startNode).toCollection(linkedSetOf())
            components += component
            remaining.removeAll(component)
        }

        return components
    }

    private fun ensureNode(graph: TraversalGraph, node: String) {
        if (!graph.hasNode(node)) throw NodeNotFoundError(node)
    }
}

data class WeightedEdge(val leftNode: String, val rightNode: String, val weight: Double)

class Graph : TraversalGraph {
    private val adjacency = linkedMapOf<String, LinkedHashMap<String, Double>>()

    override val size: Int get() = adjacency.size

    fun addNode(node: String) {
        adjacency.getOrPut(node) { linkedMapOf() }
    }

    fun removeNode(node: String) {
        val neighbors = adjacency[node] ?: throw NodeNotFoundError(node)
        neighbors.keys.toList().forEach { neighbor ->
            adjacency.getValue(neighbor).remove(node)
        }
        adjacency.remove(node)
    }

    override fun hasNode(node: String): Boolean = adjacency.containsKey(node)
    override fun nodes(): List<String> = adjacency.keys.toList()

    fun addEdge(leftNode: String, rightNode: String, weight: Double = 1.0) {
        addNode(leftNode)
        addNode(rightNode)
        adjacency.getValue(leftNode)[rightNode] = weight
        adjacency.getValue(rightNode)[leftNode] = weight
    }

    fun removeEdge(leftNode: String, rightNode: String) {
        if (!hasEdge(leftNode, rightNode)) throw EdgeNotFoundError(leftNode, rightNode)
        adjacency.getValue(leftNode).remove(rightNode)
        adjacency.getValue(rightNode).remove(leftNode)
    }

    fun hasEdge(leftNode: String, rightNode: String): Boolean =
        adjacency[leftNode]?.containsKey(rightNode) == true

    fun edgeWeight(leftNode: String, rightNode: String): Double =
        adjacency[leftNode]?.get(rightNode) ?: throw EdgeNotFoundError(leftNode, rightNode)

    fun edges(): List<WeightedEdge> {
        val seen = linkedSetOf<String>()
        val result = mutableListOf<WeightedEdge>()
        adjacency.forEach { (leftNode, neighbors) ->
            neighbors.forEach { (rightNode, weight) ->
                val edgeKey = canonicalEdgeKey(leftNode, rightNode)
                if (seen.add(edgeKey)) {
                    result += if (leftNode <= rightNode) {
                        WeightedEdge(leftNode, rightNode, weight)
                    } else {
                        WeightedEdge(rightNode, leftNode, weight)
                    }
                }
            }
        }
        return result
    }

    override fun neighbors(node: String): List<String> =
        adjacency[node]?.keys?.toList() ?: throw NodeNotFoundError(node)

    fun breadthFirst(startNode: String): List<String> = Traversals.breadthFirst(this, startNode)
    fun depthFirst(startNode: String): List<String> = Traversals.depthFirst(this, startNode)
    fun connectedComponents(): List<Set<String>> = Traversals.connectedComponents(this)
    fun isConnected(): Boolean = Traversals.isConnected(this)

    private fun canonicalEdgeKey(leftNode: String, rightNode: String): String =
        if (leftNode <= rightNode) "$leftNode\u0000$rightNode" else "$rightNode\u0000$leftNode"
}

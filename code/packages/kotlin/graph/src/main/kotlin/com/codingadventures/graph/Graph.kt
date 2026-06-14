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
    private val graphProperties = linkedMapOf<String, Any?>()
    private val nodeProperties = linkedMapOf<String, LinkedHashMap<String, Any?>>()
    private val edgeProperties = linkedMapOf<String, LinkedHashMap<String, Any?>>()

    override val size: Int get() = adjacency.size

    fun addNode(node: String, properties: Map<String, Any?> = emptyMap()) {
        adjacency.getOrPut(node) { linkedMapOf() }
        nodeProperties.getOrPut(node) { linkedMapOf() }.putAll(properties)
    }

    fun removeNode(node: String) {
        val neighbors = adjacency[node] ?: throw NodeNotFoundError(node)
        neighbors.keys.toList().forEach { neighbor ->
            adjacency.getValue(neighbor).remove(node)
            edgeProperties.remove(canonicalEdgeKey(node, neighbor))
        }
        adjacency.remove(node)
        nodeProperties.remove(node)
    }

    override fun hasNode(node: String): Boolean = adjacency.containsKey(node)
    override fun nodes(): List<String> = adjacency.keys.toList()

    fun addEdge(
        leftNode: String,
        rightNode: String,
        weight: Double = 1.0,
        properties: Map<String, Any?> = emptyMap(),
    ) {
        addNode(leftNode)
        addNode(rightNode)
        adjacency.getValue(leftNode)[rightNode] = weight
        adjacency.getValue(rightNode)[leftNode] = weight
        edgeProperties.getOrPut(canonicalEdgeKey(leftNode, rightNode)) { linkedMapOf() }.apply {
            putAll(properties)
            put("weight", weight)
        }
    }

    fun removeEdge(leftNode: String, rightNode: String) {
        if (!hasEdge(leftNode, rightNode)) throw EdgeNotFoundError(leftNode, rightNode)
        adjacency.getValue(leftNode).remove(rightNode)
        adjacency.getValue(rightNode).remove(leftNode)
        edgeProperties.remove(canonicalEdgeKey(leftNode, rightNode))
    }

    fun hasEdge(leftNode: String, rightNode: String): Boolean =
        adjacency[leftNode]?.containsKey(rightNode) == true

    fun edgeWeight(leftNode: String, rightNode: String): Double =
        adjacency[leftNode]?.get(rightNode) ?: throw EdgeNotFoundError(leftNode, rightNode)

    fun graphProperties(): Map<String, Any?> = graphProperties.toMap()

    fun setGraphProperty(key: String, value: Any?) {
        graphProperties[key] = value
    }

    fun removeGraphProperty(key: String) {
        graphProperties.remove(key)
    }

    fun nodeProperties(node: String): Map<String, Any?> {
        if (!hasNode(node)) throw NodeNotFoundError(node)
        return nodeProperties[node]?.toMap() ?: emptyMap()
    }

    fun setNodeProperty(node: String, key: String, value: Any?) {
        if (!hasNode(node)) throw NodeNotFoundError(node)
        nodeProperties.getOrPut(node) { linkedMapOf() }[key] = value
    }

    fun removeNodeProperty(node: String, key: String) {
        if (!hasNode(node)) throw NodeNotFoundError(node)
        nodeProperties[node]?.remove(key)
    }

    fun edgeProperties(leftNode: String, rightNode: String): Map<String, Any?> {
        if (!hasEdge(leftNode, rightNode)) throw EdgeNotFoundError(leftNode, rightNode)
        val properties = edgeProperties[canonicalEdgeKey(leftNode, rightNode)]?.toMutableMap() ?: linkedMapOf()
        properties["weight"] = edgeWeight(leftNode, rightNode)
        return properties.toMap()
    }

    fun setEdgeProperty(leftNode: String, rightNode: String, key: String, value: Any?) {
        if (!hasEdge(leftNode, rightNode)) throw EdgeNotFoundError(leftNode, rightNode)
        if (key == "weight") {
            require(value is Number) { "Edge property 'weight' must be numeric." }
            setEdgeWeight(leftNode, rightNode, value.toDouble())
        }
        edgeProperties.getOrPut(canonicalEdgeKey(leftNode, rightNode)) { linkedMapOf() }[key] = value
    }

    fun removeEdgeProperty(leftNode: String, rightNode: String, key: String) {
        if (!hasEdge(leftNode, rightNode)) throw EdgeNotFoundError(leftNode, rightNode)
        if (key == "weight") {
            setEdgeWeight(leftNode, rightNode, 1.0)
            edgeProperties.getOrPut(canonicalEdgeKey(leftNode, rightNode)) { linkedMapOf() }["weight"] = 1.0
            return
        }
        edgeProperties[canonicalEdgeKey(leftNode, rightNode)]?.remove(key)
    }

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

    private fun setEdgeWeight(leftNode: String, rightNode: String, weight: Double) {
        adjacency.getValue(leftNode)[rightNode] = weight
        adjacency.getValue(rightNode)[leftNode] = weight
    }
}

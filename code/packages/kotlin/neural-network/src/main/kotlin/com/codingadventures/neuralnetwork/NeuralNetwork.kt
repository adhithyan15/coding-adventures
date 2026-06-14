package com.codingadventures.neuralnetwork

import java.util.ArrayDeque

data class NeuralEdge(val id: String, val from: String, val to: String, val weight: Double, val properties: Map<String, Any?> = emptyMap())
data class WeightedInput(val from: String, val weight: Double, val edgeId: String? = null, val properties: Map<String, Any?> = emptyMap())

class NeuralGraph(name: String? = null) {
    val graphProperties: MutableMap<String, Any?> = mutableMapOf("nn.version" to "0")
    private val orderedNodes = mutableListOf<String>()
    private val nodePropertyTable = mutableMapOf<String, MutableMap<String, Any?>>()
    private val edgeList = mutableListOf<NeuralEdge>()
    private var nextEdgeId = 0

    init { if (name != null) graphProperties["nn.name"] = name }
    val nodes: List<String> get() = orderedNodes.toList()
    val edges: List<NeuralEdge> get() = edgeList.toList()

    fun addNode(node: String, properties: Map<String, Any?> = emptyMap()) {
        val target = nodePropertyTable.getOrPut(node) { orderedNodes.add(node); mutableMapOf() }
        target.putAll(properties)
    }
    fun nodeProperties(node: String): Map<String, Any?> = nodePropertyTable[node]?.toMap() ?: emptyMap()
    fun addEdge(from: String, to: String, weight: Double = 1.0, properties: Map<String, Any?> = emptyMap(), edgeId: String? = null): String {
        addNode(from); addNode(to)
        val id = edgeId ?: "e${nextEdgeId++}"
        edgeList.add(NeuralEdge(id, from, to, weight, properties + ("weight" to weight)))
        return id
    }
    fun incomingEdges(node: String): List<NeuralEdge> = edgeList.filter { it.to == node }
    fun topologicalSort(): List<String> {
        val indegree = orderedNodes.associateWith { 0 }.toMutableMap()
        for (edge in edgeList) {
            indegree.putIfAbsent(edge.from, 0)
            indegree[edge.to] = (indegree[edge.to] ?: 0) + 1
        }
        val ready = ArrayDeque(indegree.filterValues { it == 0 }.keys.sorted())
        val order = mutableListOf<String>()
        while (!ready.isEmpty()) {
            val node = ready.removeFirst()
            order.add(node)
            val released = mutableListOf<String>()
            for (edge in edgeList.filter { it.from == node }) {
                indegree[edge.to] = indegree[edge.to]!! - 1
                if (indegree[edge.to] == 0) released.add(edge.to)
            }
            released.sorted().forEach { ready.add(it) }
        }
        require(order.size == indegree.size) { "neural graph contains a cycle" }
        return order
    }
}

class NeuralNetworkModel(name: String? = null) {
    val graph: NeuralGraph = createNeuralGraph(name)
    fun input(node: String): NeuralNetworkModel { addInput(graph, node); return this }
    fun constant(node: String, value: Double, properties: Map<String, Any?> = emptyMap()): NeuralNetworkModel { addConstant(graph, node, value, properties); return this }
    fun weightedSum(node: String, inputs: List<WeightedInput>, properties: Map<String, Any?> = emptyMap()): NeuralNetworkModel { addWeightedSum(graph, node, inputs, properties); return this }
    fun activation(node: String, input: String, activation: String, properties: Map<String, Any?> = emptyMap(), edgeId: String? = null): NeuralNetworkModel { addActivation(graph, node, input, activation, properties, edgeId); return this }
    fun output(node: String, input: String, outputName: String = node, properties: Map<String, Any?> = emptyMap(), edgeId: String? = null): NeuralNetworkModel { addOutput(graph, node, input, outputName, properties, edgeId); return this }
}

fun createNeuralGraph(name: String? = null) = NeuralGraph(name)
fun createNeuralNetwork(name: String? = null) = NeuralNetworkModel(name)
fun wi(from: String, weight: Double, edgeId: String) = WeightedInput(from, weight, edgeId)
fun addInput(graph: NeuralGraph, node: String, inputName: String = node, properties: Map<String, Any?> = emptyMap()) = graph.addNode(node, properties + mapOf("nn.op" to "input", "nn.input" to inputName))
fun addConstant(graph: NeuralGraph, node: String, value: Double, properties: Map<String, Any?> = emptyMap()) { require(value.isFinite()); graph.addNode(node, properties + mapOf("nn.op" to "constant", "nn.value" to value)) }
fun addWeightedSum(graph: NeuralGraph, node: String, inputs: List<WeightedInput>, properties: Map<String, Any?> = emptyMap()) { graph.addNode(node, properties + ("nn.op" to "weighted_sum")); inputs.forEach { graph.addEdge(it.from, node, it.weight, it.properties, it.edgeId) } }
fun addActivation(graph: NeuralGraph, node: String, input: String, activation: String, properties: Map<String, Any?> = emptyMap(), edgeId: String? = null): String { graph.addNode(node, properties + mapOf("nn.op" to "activation", "nn.activation" to activation)); return graph.addEdge(input, node, 1.0, edgeId = edgeId) }
fun addOutput(graph: NeuralGraph, node: String, input: String, outputName: String = node, properties: Map<String, Any?> = emptyMap(), edgeId: String? = null): String { graph.addNode(node, properties + mapOf("nn.op" to "output", "nn.output" to outputName)); return graph.addEdge(input, node, 1.0, edgeId = edgeId) }

fun createXorNetwork(name: String = "xor") = createNeuralNetwork(name)
    .input("x0").input("x1").constant("bias", 1.0, mapOf("nn.role" to "bias"))
    .weightedSum("h_or_sum", listOf(wi("x0", 20.0, "x0_to_h_or"), wi("x1", 20.0, "x1_to_h_or"), wi("bias", -10.0, "bias_to_h_or")), mapOf("nn.layer" to "hidden"))
    .activation("h_or", "h_or_sum", "sigmoid", mapOf("nn.layer" to "hidden"), "h_or_sum_to_h_or")
    .weightedSum("h_nand_sum", listOf(wi("x0", -20.0, "x0_to_h_nand"), wi("x1", -20.0, "x1_to_h_nand"), wi("bias", 30.0, "bias_to_h_nand")), mapOf("nn.layer" to "hidden"))
    .activation("h_nand", "h_nand_sum", "sigmoid", mapOf("nn.layer" to "hidden"), "h_nand_sum_to_h_nand")
    .weightedSum("out_sum", listOf(wi("h_or", 20.0, "h_or_to_out"), wi("h_nand", 20.0, "h_nand_to_out"), wi("bias", -30.0, "bias_to_out")), mapOf("nn.layer" to "output"))
    .activation("out_activation", "out_sum", "sigmoid", mapOf("nn.layer" to "output"), "out_sum_to_activation")
    .output("out", "out_activation", "prediction", mapOf("nn.layer" to "output"), "activation_to_out")

package com.codingadventures.neuralgraphvm

import com.codingadventures.neuralnetwork.*
import kotlin.math.exp
import kotlin.math.tanh

data class Instruction(val op: String, val dst: String? = null, val inputName: String? = null, val outputName: String? = null, val edgeId: String? = null, val value: Double? = null, val left: String? = null, val right: String? = null, val inputs: List<String> = emptyList(), val input: String? = null, val activation: String? = null, val sourceNode: String? = null, val sourceEdge: String? = null)
data class BytecodeFunction(val id: String, val kind: String, val instructions: List<Instruction>)
data class BytecodeGraphEdge(val id: String, val from: String, val to: String, val weight: Double)
data class BytecodeModule(val magic: String = "CANN", val version: Int = 0, val nodes: List<String>, val edges: List<BytecodeGraphEdge>, val functions: List<BytecodeFunction>)

fun compileNeuralNetworkToBytecode(network: NeuralNetworkModel) = compileNeuralGraphToBytecode(network.graph)
fun compileNeuralGraphToBytecode(graph: NeuralGraph): BytecodeModule {
    val values = mutableMapOf<String, String>()
    var nextValueId = 0
    fun alloc() = "v${nextValueId++}"
    val instructions = mutableListOf<Instruction>()
    for (node in graph.topologicalSort()) {
        val props = graph.nodeProperties(node)
        when (props["nn.op"] as? String ?: "weighted_sum") {
            "input" -> { val dst = alloc(); values[node] = dst; instructions.add(Instruction("LOAD_INPUT", dst = dst, inputName = props["nn.input"] as? String ?: node, sourceNode = node)) }
            "constant" -> { val dst = alloc(); values[node] = dst; instructions.add(Instruction("LOAD_CONST", dst = dst, value = (props["nn.value"] as Number).toDouble(), sourceNode = node)) }
            "weighted_sum" -> {
                val terms = mutableListOf<String>()
                for (edge in graph.incomingEdges(node).sortedBy { it.id }) {
                    val weightValue = alloc(); val termValue = alloc()
                    instructions.add(Instruction("LOAD_EDGE_WEIGHT", dst = weightValue, edgeId = edge.id, sourceEdge = edge.id))
                    instructions.add(Instruction("MUL", dst = termValue, left = values[edge.from], right = weightValue, sourceEdge = edge.id))
                    terms.add(termValue)
                }
                val dst = alloc(); values[node] = dst
                instructions.add(if (terms.isEmpty()) Instruction("LOAD_CONST", dst = dst, value = 0.0, sourceNode = node) else Instruction("ADD", dst = dst, inputs = terms, sourceNode = node))
            }
            "activation" -> { val dst = alloc(); values[node] = dst; instructions.add(Instruction("ACTIVATE", dst = dst, input = singleInputValue(graph, values, node), activation = props["nn.activation"] as? String ?: "relu", sourceNode = node)) }
            "output" -> { val input = singleInputValue(graph, values, node); values[node] = input; instructions.add(Instruction("STORE_OUTPUT", outputName = props["nn.output"] as? String ?: node, input = input, sourceNode = node)) }
            else -> error("unsupported neural graph op: ${props["nn.op"]}")
        }
    }
    return BytecodeModule(nodes = graph.nodes, edges = graph.edges.map { BytecodeGraphEdge(it.id, it.from, it.to, it.weight) }, functions = listOf(BytecodeFunction("forward", "forward", instructions)))
}

fun runNeuralBytecodeForward(module: BytecodeModule, inputs: Map<String, Double>): Map<String, Double> {
    val values = mutableMapOf<String, Double>()
    val edgeWeights = module.edges.associate { it.id to it.weight }
    val outputs = mutableMapOf<String, Double>()
    val forward = module.functions.first { it.kind == "forward" }
    for (inst in forward.instructions) {
        when (inst.op) {
            "LOAD_INPUT" -> values[inst.dst!!] = inputs[inst.inputName]!!
            "LOAD_CONST" -> values[inst.dst!!] = inst.value ?: 0.0
            "LOAD_EDGE_WEIGHT" -> values[inst.dst!!] = edgeWeights[inst.edgeId] ?: 1.0
            "MUL" -> values[inst.dst!!] = values[inst.left]!! * values[inst.right]!!
            "ADD" -> values[inst.dst!!] = inst.inputs.sumOf { values[it]!! }
            "ACTIVATE" -> values[inst.dst!!] = applyNeuralActivation(values[inst.input]!!, inst.activation ?: "relu")
            "STORE_OUTPUT" -> outputs[inst.outputName ?: "output"] = values[inst.input]!!
            else -> error("unsupported opcode: ${inst.op}")
        }
    }
    return outputs
}

fun applyNeuralActivation(value: Double, activation: String) = when (activation) {
    "relu" -> if (value > 0) value else 0.0
    "sigmoid" -> 1.0 / (1.0 + exp(-value))
    "tanh" -> tanh(value)
    else -> value
}

private fun singleInputValue(graph: NeuralGraph, values: Map<String, String>, node: String): String {
    val incoming = graph.incomingEdges(node)
    require(incoming.size == 1) { "node $node expects exactly one input" }
    return values[incoming.first().from]!!
}

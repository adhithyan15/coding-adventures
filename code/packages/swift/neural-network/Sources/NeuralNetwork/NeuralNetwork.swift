import Foundation

public enum NeuralPropertyValue: Equatable, Sendable {
    case string(String)
    case number(Double)
    case boolean(Bool)
    case null
}

public typealias NeuralPropertyBag = [String: NeuralPropertyValue]

public struct NeuralEdge: Equatable, Sendable {
    public let id: String
    public let from: String
    public let to: String
    public let weight: Double
    public let properties: NeuralPropertyBag
}

public struct WeightedInput: Equatable, Sendable {
    public let from: String
    public let weight: Double
    public let edgeId: String?
    public let properties: NeuralPropertyBag

    public init(from: String, weight: Double, edgeId: String? = nil, properties: NeuralPropertyBag = [:]) {
        self.from = from
        self.weight = weight
        self.edgeId = edgeId
        self.properties = properties
    }
}

public final class NeuralGraph: @unchecked Sendable {
    public private(set) var graphProperties: NeuralPropertyBag = ["nn.version": .string("0")]
    private var orderedNodes: [String] = []
    private var nodePropertyTable: [String: NeuralPropertyBag] = [:]
    private var edgeList: [NeuralEdge] = []
    private var nextEdgeId = 0

    public init(name: String? = nil) {
        if let name { graphProperties["nn.name"] = .string(name) }
    }

    public var nodes: [String] { orderedNodes }
    public var edges: [NeuralEdge] { edgeList }

    public func addNode(_ node: String, properties: NeuralPropertyBag = [:]) {
        if nodePropertyTable[node] == nil {
            orderedNodes.append(node)
            nodePropertyTable[node] = [:]
        }
        nodePropertyTable[node]!.merge(properties) { _, new in new }
    }

    public func nodeProperties(_ node: String) -> NeuralPropertyBag {
        nodePropertyTable[node] ?? [:]
    }

    @discardableResult
    public func addEdge(from: String, to: String, weight: Double = 1.0, properties: NeuralPropertyBag = [:], edgeId: String? = nil) -> String {
        addNode(from)
        addNode(to)
        let id = edgeId ?? "e\(nextEdgeId)"
        if edgeId == nil { nextEdgeId += 1 }
        var props = properties
        props["weight"] = .number(weight)
        edgeList.append(NeuralEdge(id: id, from: from, to: to, weight: weight, properties: props))
        return id
    }

    public func incomingEdges(_ node: String) -> [NeuralEdge] {
        edgeList.filter { $0.to == node }
    }

    public func topologicalSort() throws -> [String] {
        var indegree: [String: Int] = Dictionary(uniqueKeysWithValues: orderedNodes.map { ($0, 0) })
        for edge in edgeList {
            indegree[edge.from, default: 0] += 0
            indegree[edge.to, default: 0] += 1
        }
        var ready = indegree.filter { $0.value == 0 }.map(\.key).sorted()
        var order: [String] = []
        while !ready.isEmpty {
            let node = ready.removeFirst()
            order.append(node)
            var released: [String] = []
            for edge in edgeList where edge.from == node {
                indegree[edge.to]! -= 1
                if indegree[edge.to] == 0 { released.append(edge.to) }
            }
            ready.append(contentsOf: released.sorted())
        }
        if order.count != indegree.count { throw NeuralGraphError.cycle }
        return order
    }
}

public enum NeuralGraphError: Error { case cycle }

public final class NeuralNetworkModel: @unchecked Sendable {
    public let graph: NeuralGraph
    public init(name: String? = nil) { graph = createNeuralGraph(name: name) }

    @discardableResult public func input(_ node: String) -> Self { addInput(graph, node: node); return self }
    @discardableResult public func constant(_ node: String, value: Double, properties: NeuralPropertyBag = [:]) -> Self { addConstant(graph, node: node, value: value, properties: properties); return self }
    @discardableResult public func weightedSum(_ node: String, inputs: [WeightedInput], properties: NeuralPropertyBag = [:]) -> Self { addWeightedSum(graph, node: node, inputs: inputs, properties: properties); return self }
    @discardableResult public func activation(_ node: String, input: String, activation: String, properties: NeuralPropertyBag = [:], edgeId: String? = nil) -> Self { addActivation(graph, node: node, input: input, activation: activation, properties: properties, edgeId: edgeId); return self }
    @discardableResult public func output(_ node: String, input: String, outputName: String? = nil, properties: NeuralPropertyBag = [:], edgeId: String? = nil) -> Self { addOutput(graph, node: node, input: input, outputName: outputName, properties: properties, edgeId: edgeId); return self }
}

public func createNeuralGraph(name: String? = nil) -> NeuralGraph { NeuralGraph(name: name) }
public func createNeuralNetwork(name: String? = nil) -> NeuralNetworkModel { NeuralNetworkModel(name: name) }

public func addInput(_ graph: NeuralGraph, node: String, inputName: String? = nil, properties: NeuralPropertyBag = [:]) {
    var props = properties
    props["nn.op"] = .string("input")
    props["nn.input"] = .string(inputName ?? node)
    graph.addNode(node, properties: props)
}

public func addConstant(_ graph: NeuralGraph, node: String, value: Double, properties: NeuralPropertyBag = [:]) {
    precondition(value.isFinite, "constant value must be finite")
    var props = properties
    props["nn.op"] = .string("constant")
    props["nn.value"] = .number(value)
    graph.addNode(node, properties: props)
}

public func addWeightedSum(_ graph: NeuralGraph, node: String, inputs: [WeightedInput], properties: NeuralPropertyBag = [:]) {
    var props = properties
    props["nn.op"] = .string("weighted_sum")
    graph.addNode(node, properties: props)
    for input in inputs { graph.addEdge(from: input.from, to: node, weight: input.weight, properties: input.properties, edgeId: input.edgeId) }
}

@discardableResult public func addActivation(_ graph: NeuralGraph, node: String, input: String, activation: String, properties: NeuralPropertyBag = [:], edgeId: String? = nil) -> String {
    var props = properties
    props["nn.op"] = .string("activation")
    props["nn.activation"] = .string(activation)
    graph.addNode(node, properties: props)
    return graph.addEdge(from: input, to: node, weight: 1.0, edgeId: edgeId)
}

@discardableResult public func addOutput(_ graph: NeuralGraph, node: String, input: String, outputName: String? = nil, properties: NeuralPropertyBag = [:], edgeId: String? = nil) -> String {
    var props = properties
    props["nn.op"] = .string("output")
    props["nn.output"] = .string(outputName ?? node)
    graph.addNode(node, properties: props)
    return graph.addEdge(from: input, to: node, weight: 1.0, edgeId: edgeId)
}

public func createXorNetwork(name: String = "xor") -> NeuralNetworkModel {
    createNeuralNetwork(name: name)
        .input("x0")
        .input("x1")
        .constant("bias", value: 1.0, properties: ["nn.role": .string("bias")])
        .weightedSum("h_or_sum", inputs: [WeightedInput(from: "x0", weight: 20, edgeId: "x0_to_h_or"), WeightedInput(from: "x1", weight: 20, edgeId: "x1_to_h_or"), WeightedInput(from: "bias", weight: -10, edgeId: "bias_to_h_or")], properties: ["nn.layer": .string("hidden")])
        .activation("h_or", input: "h_or_sum", activation: "sigmoid", properties: ["nn.layer": .string("hidden")], edgeId: "h_or_sum_to_h_or")
        .weightedSum("h_nand_sum", inputs: [WeightedInput(from: "x0", weight: -20, edgeId: "x0_to_h_nand"), WeightedInput(from: "x1", weight: -20, edgeId: "x1_to_h_nand"), WeightedInput(from: "bias", weight: 30, edgeId: "bias_to_h_nand")], properties: ["nn.layer": .string("hidden")])
        .activation("h_nand", input: "h_nand_sum", activation: "sigmoid", properties: ["nn.layer": .string("hidden")], edgeId: "h_nand_sum_to_h_nand")
        .weightedSum("out_sum", inputs: [WeightedInput(from: "h_or", weight: 20, edgeId: "h_or_to_out"), WeightedInput(from: "h_nand", weight: 20, edgeId: "h_nand_to_out"), WeightedInput(from: "bias", weight: -30, edgeId: "bias_to_out")], properties: ["nn.layer": .string("output")])
        .activation("out_activation", input: "out_sum", activation: "sigmoid", properties: ["nn.layer": .string("output")], edgeId: "out_sum_to_activation")
        .output("out", input: "out_activation", outputName: "prediction", properties: ["nn.layer": .string("output")], edgeId: "activation_to_out")
}

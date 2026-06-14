import Foundation
import NeuralNetwork

public struct NeuralBytecodeInstruction: Equatable, Sendable {
    public let op: String
    public let dst: String?
    public let inputName: String?
    public let outputName: String?
    public let edgeId: String?
    public let value: Double?
    public let left: String?
    public let right: String?
    public let inputs: [String]
    public let input: String?
    public let activation: String?
    public let sourceNode: String?
    public let sourceEdge: String?

    public init(op: String, dst: String? = nil, inputName: String? = nil, outputName: String? = nil, edgeId: String? = nil, value: Double? = nil, left: String? = nil, right: String? = nil, inputs: [String] = [], input: String? = nil, activation: String? = nil, sourceNode: String? = nil, sourceEdge: String? = nil) {
        self.op = op; self.dst = dst; self.inputName = inputName; self.outputName = outputName; self.edgeId = edgeId; self.value = value; self.left = left; self.right = right; self.inputs = inputs; self.input = input; self.activation = activation; self.sourceNode = sourceNode; self.sourceEdge = sourceEdge
    }
}

public struct NeuralBytecodeFunction: Equatable, Sendable { public let id: String; public let kind: String; public let instructions: [NeuralBytecodeInstruction] }
public struct NeuralBytecodeGraphEdge: Equatable, Sendable { public let id: String; public let from: String; public let to: String; public let weight: Double }
public struct NeuralBytecodeModule: Equatable, Sendable { public let magic = "CANN"; public let version = 0; public let nodes: [String]; public let edges: [NeuralBytecodeGraphEdge]; public let functions: [NeuralBytecodeFunction] }

public func compileNeuralNetworkToBytecode(_ network: NeuralNetworkModel) throws -> NeuralBytecodeModule { try compileNeuralGraphToBytecode(network.graph) }

public func compileNeuralGraphToBytecode(_ graph: NeuralGraph) throws -> NeuralBytecodeModule {
    let order = try graph.topologicalSort()
    var instructions: [NeuralBytecodeInstruction] = []
    var values: [String: String] = [:]
    var nextValueId = 0
    func alloc() -> String { defer { nextValueId += 1 }; return "v\(nextValueId)" }

    for node in order {
        let props = graph.nodeProperties(node)
        let op = stringProp(props["nn.op"]) ?? "weighted_sum"
        switch op {
        case "input":
            let dst = alloc(); values[node] = dst
            instructions.append(.init(op: "LOAD_INPUT", dst: dst, inputName: stringProp(props["nn.input"]) ?? node, sourceNode: node))
        case "constant":
            let dst = alloc(); values[node] = dst
            instructions.append(.init(op: "LOAD_CONST", dst: dst, value: numberProp(props["nn.value"]), sourceNode: node))
        case "weighted_sum":
            var terms: [String] = []
            for edge in graph.incomingEdges(node).sorted(by: { $0.id < $1.id }) {
                let weightValue = alloc(); let termValue = alloc()
                instructions.append(.init(op: "LOAD_EDGE_WEIGHT", dst: weightValue, edgeId: edge.id, sourceEdge: edge.id))
                instructions.append(.init(op: "MUL", dst: termValue, left: values[edge.from], right: weightValue, sourceEdge: edge.id))
                terms.append(termValue)
            }
            let dst = alloc(); values[node] = dst
            instructions.append(terms.isEmpty ? .init(op: "LOAD_CONST", dst: dst, value: 0.0, sourceNode: node) : .init(op: "ADD", dst: dst, inputs: terms, sourceNode: node))
        case "activation":
            let dst = alloc(); values[node] = dst
            instructions.append(.init(op: "ACTIVATE", dst: dst, input: try singleInputValue(graph, values, node), activation: stringProp(props["nn.activation"]) ?? "relu", sourceNode: node))
        case "output":
            let input = try singleInputValue(graph, values, node); values[node] = input
            instructions.append(.init(op: "STORE_OUTPUT", outputName: stringProp(props["nn.output"]) ?? node, input: input, sourceNode: node))
        default:
            throw NeuralGraphVMError.unsupportedOperation(op)
        }
    }

    return NeuralBytecodeModule(nodes: graph.nodes, edges: graph.edges.map { .init(id: $0.id, from: $0.from, to: $0.to, weight: $0.weight) }, functions: [.init(id: "forward", kind: "forward", instructions: instructions)])
}

public func runNeuralBytecodeForward(_ module: NeuralBytecodeModule, inputs: [String: Double]) throws -> [String: Double] {
    var values: [String: Double] = [:]
    let edgeWeights = Dictionary(uniqueKeysWithValues: module.edges.map { ($0.id, $0.weight) })
    var outputs: [String: Double] = [:]
    guard let forward = module.functions.first(where: { $0.kind == "forward" }) else { throw NeuralGraphVMError.missingForward }
    for inst in forward.instructions {
        switch inst.op {
        case "LOAD_INPUT": values[try required(inst.dst)] = inputs[try required(inst.inputName)]!
        case "LOAD_CONST": values[try required(inst.dst)] = inst.value ?? 0.0
        case "LOAD_EDGE_WEIGHT": values[try required(inst.dst)] = edgeWeights[try required(inst.edgeId)] ?? 1.0
        case "MUL": values[try required(inst.dst)] = values[try required(inst.left)]! * values[try required(inst.right)]!
        case "ADD": values[try required(inst.dst)] = inst.inputs.reduce(0.0) { $0 + values[$1]! }
        case "ACTIVATE": values[try required(inst.dst)] = applyNeuralActivation(values[try required(inst.input)]!, activation: inst.activation ?? "relu")
        case "STORE_OUTPUT": outputs[inst.outputName ?? "output"] = values[try required(inst.input)]!
        default: throw NeuralGraphVMError.unsupportedOperation(inst.op)
        }
    }
    return outputs
}

public func applyNeuralActivation(_ value: Double, activation: String) -> Double {
    switch activation {
    case "relu": return value > 0 ? value : 0.0
    case "sigmoid": return 1.0 / (1.0 + Foundation.exp(-value))
    case "tanh": return Foundation.tanh(value)
    default: return value
    }
}

public enum NeuralGraphVMError: Error { case unsupportedOperation(String); case missingForward; case missingValue; case badInputCount }

private func stringProp(_ value: NeuralPropertyValue?) -> String? { if case .string(let value) = value { return value }; return nil }
private func numberProp(_ value: NeuralPropertyValue?) -> Double? { if case .number(let value) = value { return value }; return nil }
private func required(_ value: String?) throws -> String { guard let value else { throw NeuralGraphVMError.missingValue }; return value }
private func singleInputValue(_ graph: NeuralGraph, _ values: [String: String], _ node: String) throws -> String {
    let incoming = graph.incomingEdges(node)
    guard incoming.count == 1 else { throw NeuralGraphVMError.badInputCount }
    return values[incoming[0].from]!
}

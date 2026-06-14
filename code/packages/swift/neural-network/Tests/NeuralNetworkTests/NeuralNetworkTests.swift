import Testing
@testable import NeuralNetwork

@Suite("NeuralNetwork")
struct NeuralNetworkTests {
    @Test func buildsTinyWeightedGraph() throws {
        let graph = createNeuralGraph(name: "tiny")
        addInput(graph, node: "x0")
        addInput(graph, node: "x1")
        addConstant(graph, node: "bias", value: 1.0)
        addWeightedSum(graph, node: "sum", inputs: [WeightedInput(from: "x0", weight: 0.25, edgeId: "x0_to_sum"), WeightedInput(from: "x1", weight: 0.75, edgeId: "x1_to_sum"), WeightedInput(from: "bias", weight: -1.0, edgeId: "bias_to_sum")])
        addActivation(graph, node: "relu", input: "sum", activation: "relu", edgeId: "sum_to_relu")
        addOutput(graph, node: "out", input: "relu", outputName: "prediction", edgeId: "relu_to_out")
        #expect(graph.incomingEdges("sum").count == 3)
        #expect(try graph.topologicalSort().last == "out")
    }

    @Test func xorNetworkHasHiddenOutputEdge() {
        let network = createXorNetwork()
        #expect(network.graph.edges.contains { $0.id == "h_or_to_out" })
    }
}

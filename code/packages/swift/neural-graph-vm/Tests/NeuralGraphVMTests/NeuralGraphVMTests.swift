import Testing
@testable import NeuralGraphVM
import NeuralNetwork

func tinyGraph() -> NeuralGraph {
    let graph = createNeuralGraph(name: "tiny")
    addInput(graph, node: "x0")
    addInput(graph, node: "x1")
    addConstant(graph, node: "bias", value: 1.0)
    addWeightedSum(graph, node: "sum", inputs: [WeightedInput(from: "x0", weight: 0.25, edgeId: "x0_to_sum"), WeightedInput(from: "x1", weight: 0.75, edgeId: "x1_to_sum"), WeightedInput(from: "bias", weight: -1.0, edgeId: "bias_to_sum")])
    addActivation(graph, node: "relu", input: "sum", activation: "relu", edgeId: "sum_to_relu")
    addOutput(graph, node: "out", input: "relu", outputName: "prediction", edgeId: "relu_to_out")
    return graph
}

@Suite("NeuralGraphVM")
struct NeuralGraphVMTests {
    @Test func runsTinyWeightedSum() throws {
        let outputs = try runNeuralBytecodeForward(try compileNeuralGraphToBytecode(tinyGraph()), inputs: ["x0": 4.0, "x1": 8.0])
        #expect(abs(outputs["prediction"]! - 6.0) < 1.0e-9)
    }

    @Test func runsXor() throws {
        let bytecode = try compileNeuralNetworkToBytecode(createXorNetwork())
        for (x0, x1, expected) in [(0.0, 0.0, 0.0), (0.0, 1.0, 1.0), (1.0, 0.0, 1.0), (1.0, 1.0, 0.0)] {
            let prediction = try runNeuralBytecodeForward(bytecode, inputs: ["x0": x0, "x1": x1])["prediction"]!
            #expect(expected == 1.0 ? prediction > 0.99 : prediction < 0.01)
        }
    }
}

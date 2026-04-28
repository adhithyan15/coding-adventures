import XCTest
@testable import TwoLayerNetwork

final class TwoLayerNetworkTests: XCTestCase {
    private let inputs = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]
    private let targets = [[0.0], [1.0], [1.0], [0.0]]

    func testForwardPassExposesHiddenActivations() throws {
        let pass = try forward(inputs: inputs, parameters: xorWarmStartParameters())

        XCTAssertEqual(pass.hiddenActivations.count, 4)
        XCTAssertEqual(pass.hiddenActivations[0].count, 2)
        XCTAssertGreaterThan(pass.predictions[1][0], 0.7)
        XCTAssertLessThan(pass.predictions[0][0], 0.3)
    }

    func testTrainingStepExposesBothLayerGradients() throws {
        let step = try trainOneEpoch(inputs: inputs, targets: targets, parameters: xorWarmStartParameters(), learningRate: 0.5)

        XCTAssertEqual(step.inputToHiddenWeightGradients.count, 2)
        XCTAssertEqual(step.inputToHiddenWeightGradients[0].count, 2)
        XCTAssertEqual(step.hiddenToOutputWeightGradients.count, 2)
        XCTAssertEqual(step.hiddenToOutputWeightGradients[0].count, 1)
    }

    func testHiddenLayerTeachingExamplesRunOneTrainingStep() throws {
        let cases = [
            ExampleCase(name: "XNOR", inputs: inputs, targets: [[1.0], [0.0], [0.0], [1.0]], hiddenCount: 3),
            ExampleCase(name: "absolute value", inputs: [[-1.0], [-0.5], [0.0], [0.5], [1.0]], targets: [[1.0], [0.5], [0.0], [0.5], [1.0]], hiddenCount: 4),
            ExampleCase(name: "piecewise pricing", inputs: [[0.1], [0.3], [0.5], [0.7], [0.9]], targets: [[0.12], [0.25], [0.55], [0.88], [0.88]], hiddenCount: 4),
            ExampleCase(name: "circle classifier", inputs: [[0.0, 0.0], [0.5, 0.0], [1.0, 1.0], [-0.5, 0.5], [-1.0, 0.0]], targets: [[1.0], [1.0], [0.0], [1.0], [0.0]], hiddenCount: 5),
            ExampleCase(name: "two moons", inputs: [[1.0, 0.0], [0.0, 0.5], [0.5, 0.85], [0.5, -0.35], [-1.0, 0.0], [2.0, 0.5]], targets: [[0.0], [1.0], [0.0], [1.0], [0.0], [1.0]], hiddenCount: 5),
            ExampleCase(name: "interaction features", inputs: [[0.2, 0.25, 0.0], [0.6, 0.5, 1.0], [1.0, 0.75, 1.0], [1.0, 1.0, 0.0]], targets: [[0.08], [0.72], [0.96], [0.76]], hiddenCount: 5),
        ]

        for example in cases {
            let step = try trainOneEpoch(
                inputs: example.inputs,
                targets: example.targets,
                parameters: sampleParameters(inputCount: example.inputs[0].count, hiddenCount: example.hiddenCount),
                learningRate: 0.4
            )

            XCTAssertGreaterThanOrEqual(step.loss, 0.0, example.name)
            XCTAssertEqual(step.inputToHiddenWeightGradients.count, example.inputs[0].count, example.name)
            XCTAssertEqual(step.hiddenToOutputWeightGradients.count, example.hiddenCount, example.name)
        }
    }

    private func sampleParameters(inputCount: Int, hiddenCount: Int) -> Parameters {
        var inputToHiddenWeights = Array(repeating: Array(repeating: 0.0, count: hiddenCount), count: inputCount)
        for feature in 0..<inputCount {
            for hidden in 0..<hiddenCount {
                inputToHiddenWeights[feature][hidden] = 0.17 * Double(feature + 1) - 0.11 * Double(hidden + 1)
            }
        }

        var hiddenBiases = Array(repeating: 0.0, count: hiddenCount)
        var hiddenToOutputWeights = Array(repeating: [0.0], count: hiddenCount)
        for hidden in 0..<hiddenCount {
            hiddenBiases[hidden] = 0.05 * Double(hidden - 1)
            hiddenToOutputWeights[hidden][0] = 0.13 * Double(hidden + 1) - 0.25
        }

        return Parameters(
            inputToHiddenWeights: inputToHiddenWeights,
            hiddenBiases: hiddenBiases,
            hiddenToOutputWeights: hiddenToOutputWeights,
            outputBiases: [0.02]
        )
    }

    private struct ExampleCase {
        let name: String
        let inputs: [[Double]]
        let targets: [[Double]]
        let hiddenCount: Int
    }
}

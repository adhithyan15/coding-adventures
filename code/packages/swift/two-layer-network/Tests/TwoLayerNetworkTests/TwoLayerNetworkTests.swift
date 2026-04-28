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
}

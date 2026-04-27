import XCTest
@testable import SingleLayerNetwork

final class SingleLayerNetworkTests: XCTestCase {
    func testOneEpochExposesMatrixGradients() throws {
        let step = try trainOneEpochWithMatrices(
            inputs: [[1.0, 2.0]],
            targets: [[3.0, 5.0]],
            weights: [[0.0, 0.0], [0.0, 0.0]],
            biases: [0.0, 0.0],
            learningRate: 0.1
        )

        XCTAssertEqual(step.weightGradients[0][0], -3.0, accuracy: 1.0e-6)
        XCTAssertEqual(step.weightGradients[1][1], -10.0, accuracy: 1.0e-6)
        XCTAssertEqual(step.nextWeights[0][0], 0.3, accuracy: 1.0e-6)
        XCTAssertEqual(step.nextWeights[1][1], 1.0, accuracy: 1.0e-6)
    }

    func testFitLearnsMInputsToNOutputs() throws {
        var model = SingleLayerNetwork(inputCount: 3, outputCount: 2)
        let history = try model.fit(
            [[0.0, 0.0, 1.0], [1.0, 2.0, 1.0], [2.0, 1.0, 1.0]],
            [[1.0, -1.0], [3.0, 2.0], [4.0, 1.0]],
            learningRate: 0.05,
            epochs: 500
        )
        XCTAssertLessThan(history.last!.loss, history.first!.loss)
        XCTAssertEqual(try model.predict([[1.0, 1.0, 1.0]])[0].count, 2)
    }
}

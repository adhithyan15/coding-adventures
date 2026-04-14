import XCTest
@testable import GradientDescent

final class GradientDescentTests: XCTestCase {
    func testSgd() throws {
        let result = try GradientDescent.sgd(weights: [1.0, 2.0], gradients: [0.1, 0.2], learningRate: 0.5)
        XCTAssertEqual(result[0], 0.95, accuracy: 0.0001)
        XCTAssertEqual(result[1], 1.9, accuracy: 0.0001)
    }
}

import XCTest
@testable import ActivationFunctions

final class ActivationFunctionsTests: XCTestCase {

    private let eps: Double = 1e-10

    // ========================================================================
    // MARK: - Linear Tests
    // ========================================================================

    func testLinearNegative() {
        XCTAssertEqual(ActivationFunctions.linear(-3.0), -3.0, accuracy: eps)
    }

    func testLinearZero() {
        XCTAssertEqual(ActivationFunctions.linear(0.0), 0.0, accuracy: eps)
    }

    func testLinearPositive() {
        XCTAssertEqual(ActivationFunctions.linear(5.0), 5.0, accuracy: eps)
    }

    func testLinearDerivativeEverywhere() {
        XCTAssertEqual(ActivationFunctions.linearDerivative(-3.0), 1.0, accuracy: eps)
        XCTAssertEqual(ActivationFunctions.linearDerivative(0.0), 1.0, accuracy: eps)
        XCTAssertEqual(ActivationFunctions.linearDerivative(5.0), 1.0, accuracy: eps)
    }

    // ========================================================================
    // MARK: - Sigmoid Tests
    // ========================================================================

    func testSigmoidAtZero() {
        XCTAssertEqual(ActivationFunctions.sigmoid(0.0), 0.5, accuracy: eps)
    }

    func testSigmoidPositive() {
        XCTAssertEqual(ActivationFunctions.sigmoid(1.0), 0.7310585786300049, accuracy: eps)
    }

    func testSigmoidNegative() {
        XCTAssertEqual(ActivationFunctions.sigmoid(-1.0), 0.2689414213699951, accuracy: eps)
    }

    func testSigmoidLargePositive() {
        XCTAssertEqual(ActivationFunctions.sigmoid(10.0), 0.9999546021312976, accuracy: 1e-8)
    }

    func testSigmoidOverflowNegative() {
        XCTAssertEqual(ActivationFunctions.sigmoid(-710.0), 0.0)
    }

    func testSigmoidOverflowPositive() {
        XCTAssertEqual(ActivationFunctions.sigmoid(710.0), 1.0)
    }

    func testSigmoidSymmetry() {
        // σ(-x) = 1 - σ(x)
        let values = [0.5, 1.0, 2.0, 5.0, 10.0]
        for x in values {
            XCTAssertEqual(
                ActivationFunctions.sigmoid(-x),
                1.0 - ActivationFunctions.sigmoid(x),
                accuracy: eps,
                "Symmetry failed at x = \(x)")
        }
    }

    func testSigmoidRange() {
        let values = [-100.0, -10.0, -1.0, 0.0, 1.0, 10.0, 100.0]
        for x in values {
            let s = ActivationFunctions.sigmoid(x)
            XCTAssertGreaterThanOrEqual(s, 0.0, "Sigmoid < 0 at x = \(x)")
            XCTAssertLessThanOrEqual(s, 1.0, "Sigmoid > 1 at x = \(x)")
        }
    }

    // ========================================================================
    // MARK: - Sigmoid Derivative Tests
    // ========================================================================

    func testSigmoidDerivativeAtZero() {
        XCTAssertEqual(ActivationFunctions.sigmoidDerivative(0.0), 0.25, accuracy: eps)
    }

    func testSigmoidDerivativeAtOne() {
        XCTAssertEqual(ActivationFunctions.sigmoidDerivative(1.0), 0.19661193324148185, accuracy: eps)
    }

    func testSigmoidDerivativeSaturated() {
        let d = ActivationFunctions.sigmoidDerivative(10.0)
        XCTAssertEqual(d, 0.0000453978, accuracy: 1e-8)
    }

    func testSigmoidDerivativeNonNegative() {
        let values = [-10.0, -1.0, 0.0, 1.0, 10.0]
        for x in values {
            XCTAssertGreaterThanOrEqual(ActivationFunctions.sigmoidDerivative(x), 0.0)
        }
    }

    // ========================================================================
    // MARK: - ReLU Tests
    // ========================================================================

    func testReluPositive() {
        XCTAssertEqual(ActivationFunctions.relu(5.0), 5.0)
    }

    func testReluNegative() {
        XCTAssertEqual(ActivationFunctions.relu(-3.0), 0.0)
    }

    func testReluZero() {
        XCTAssertEqual(ActivationFunctions.relu(0.0), 0.0)
    }

    func testReluIdempotence() {
        // ReLU(ReLU(x)) = ReLU(x)
        let values = [-5.0, -1.0, 0.0, 1.0, 5.0]
        for x in values {
            let r = ActivationFunctions.relu(x)
            XCTAssertEqual(ActivationFunctions.relu(r), r, accuracy: eps)
        }
    }

    // ========================================================================
    // MARK: - ReLU Derivative Tests
    // ========================================================================

    func testReluDerivativePositive() {
        XCTAssertEqual(ActivationFunctions.reluDerivative(5.0), 1.0)
    }

    func testReluDerivativeNegative() {
        XCTAssertEqual(ActivationFunctions.reluDerivative(-3.0), 0.0)
    }

    func testReluDerivativeZero() {
        XCTAssertEqual(ActivationFunctions.reluDerivative(0.0), 0.0)
    }

    // ========================================================================
    // MARK: - Leaky ReLU Tests
    // ========================================================================

    func testLeakyReluPositive() {
        XCTAssertEqual(ActivationFunctions.leakyRelu(5.0), 5.0, accuracy: eps)
    }

    func testLeakyReluNegative() {
        XCTAssertEqual(ActivationFunctions.leakyRelu(-3.0), -0.03, accuracy: eps)
    }

    func testLeakyReluZero() {
        XCTAssertEqual(ActivationFunctions.leakyRelu(0.0), 0.0, accuracy: eps)
    }

    func testLeakyReluDerivativePositive() {
        XCTAssertEqual(ActivationFunctions.leakyReluDerivative(5.0), 1.0, accuracy: eps)
    }

    func testLeakyReluDerivativeNegative() {
        XCTAssertEqual(ActivationFunctions.leakyReluDerivative(-3.0), 0.01, accuracy: eps)
    }

    func testLeakyReluDerivativeZero() {
        XCTAssertEqual(ActivationFunctions.leakyReluDerivative(0.0), 0.01, accuracy: eps)
    }

    // ========================================================================
    // MARK: - Tanh Tests
    // ========================================================================

    func testTanhAtZero() {
        XCTAssertEqual(ActivationFunctions.tanh(0.0), 0.0, accuracy: eps)
    }

    func testTanhPositive() {
        XCTAssertEqual(ActivationFunctions.tanh(1.0), 0.7615941559557649, accuracy: eps)
    }

    func testTanhNegative() {
        XCTAssertEqual(ActivationFunctions.tanh(-1.0), -0.7615941559557649, accuracy: eps)
    }

    func testTanhOddSymmetry() {
        // tanh(-x) = -tanh(x)
        let values = [0.5, 1.0, 2.0, 5.0]
        for x in values {
            XCTAssertEqual(
                ActivationFunctions.tanh(-x),
                -ActivationFunctions.tanh(x),
                accuracy: eps,
                "Odd symmetry failed at x = \(x)")
        }
    }

    func testTanhRange() {
        let values = [-100.0, -10.0, -1.0, 0.0, 1.0, 10.0, 100.0]
        for x in values {
            let t = ActivationFunctions.tanh(x)
            XCTAssertGreaterThanOrEqual(t, -1.0)
            XCTAssertLessThanOrEqual(t, 1.0)
        }
    }

    // ========================================================================
    // MARK: - Tanh Derivative Tests
    // ========================================================================

    func testTanhDerivativeAtZero() {
        XCTAssertEqual(ActivationFunctions.tanhDerivative(0.0), 1.0, accuracy: eps)
    }

    func testTanhDerivativeAtOne() {
        XCTAssertEqual(ActivationFunctions.tanhDerivative(1.0), 0.4199743416140261, accuracy: eps)
    }

    func testTanhDerivativeNonNegative() {
        let values = [-10.0, -1.0, 0.0, 1.0, 10.0]
        for x in values {
            XCTAssertGreaterThanOrEqual(ActivationFunctions.tanhDerivative(x), 0.0)
        }
    }

    // ========================================================================
    // MARK: - Softplus Tests
    // ========================================================================

    func testSoftplusAtZero() {
        XCTAssertEqual(ActivationFunctions.softplus(0.0), 0.6931471805599453, accuracy: eps)
    }

    func testSoftplusPositive() {
        XCTAssertEqual(ActivationFunctions.softplus(1.0), 1.3132616875182228, accuracy: eps)
    }

    func testSoftplusNegative() {
        XCTAssertEqual(ActivationFunctions.softplus(-1.0), 0.31326168751822286, accuracy: eps)
    }

    func testSoftplusLargePositiveStable() {
        XCTAssertGreaterThan(ActivationFunctions.softplus(1000.0), 999.0)
    }

    func testSoftplusDerivativeEqualsSigmoid() {
        XCTAssertEqual(ActivationFunctions.softplusDerivative(0.0), 0.5, accuracy: eps)
        XCTAssertEqual(ActivationFunctions.softplusDerivative(1.0), ActivationFunctions.sigmoid(1.0), accuracy: eps)
        XCTAssertEqual(ActivationFunctions.softplusDerivative(-1.0), ActivationFunctions.sigmoid(-1.0), accuracy: eps)
    }
}

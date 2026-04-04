import XCTest
@testable import LossFunctions

final class LossFunctionsTests: XCTestCase {

    // ========================================================================
    // MARK: - Helpers
    // ========================================================================

    /// Compare two doubles with a tolerance.
    private func approxEqual(_ a: Double, _ b: Double,
                             tol: Double = 1e-7,
                             file: StaticString = #file,
                             line: UInt = #line) {
        XCTAssertEqual(a, b, accuracy: tol, file: file, line: line)
    }

    /// Compare two arrays element-wise.
    private func approxEqualArray(_ a: [Double], _ b: [Double],
                                  tol: Double = 1e-7,
                                  file: StaticString = #file,
                                  line: UInt = #line) {
        XCTAssertEqual(a.count, b.count, file: file, line: line)
        for i in 0..<a.count {
            XCTAssertEqual(a[i], b[i], accuracy: tol,
                           "Mismatch at index \(i)", file: file, line: line)
        }
    }

    // ========================================================================
    // MARK: - MSE Tests
    // ========================================================================

    func testMSEParityVector() {
        // From spec: y_true=[1,0,0], y_pred=[0.9,0.1,0.2] → 0.02
        let result = LossFunctions.mse([1.0, 0.0, 0.0], [0.9, 0.1, 0.2])
        approxEqual(result, 0.02)
    }

    func testMSEPerfectPrediction() {
        let result = LossFunctions.mse([1.0, 2.0, 3.0], [1.0, 2.0, 3.0])
        approxEqual(result, 0.0)
    }

    func testMSESymmetry() {
        // MSE(a,b) == MSE(b,a) because (a-b)^2 == (b-a)^2
        let a = [1.0, 2.0, 3.0]
        let b = [1.5, 2.5, 3.5]
        approxEqual(LossFunctions.mse(a, b), LossFunctions.mse(b, a))
    }

    func testMSEDerivative() {
        let yTrue = [1.0, 0.0, 0.0]
        let yPred = [0.9, 0.1, 0.2]
        let grad = LossFunctions.mseDerivative(yTrue, yPred)
        // d/dŷ = 2/n * (ŷ - y) = 2/3 * [-0.1, 0.1, 0.2]
        let expected = [-2.0/30.0, 2.0/30.0, 4.0/30.0]
        approxEqualArray(grad, expected)
    }

    func testMSEDerivativeSignConvention() {
        // When prediction > truth, derivative should be positive
        let grad = LossFunctions.mseDerivative([0.0], [1.0])
        XCTAssertGreaterThan(grad[0], 0.0)
    }

    // ========================================================================
    // MARK: - MAE Tests
    // ========================================================================

    func testMAEParityVector() {
        // From spec: y_true=[1,0,0], y_pred=[0.9,0.1,0.2] → 0.1333...
        let result = LossFunctions.mae([1.0, 0.0, 0.0], [0.9, 0.1, 0.2])
        approxEqual(result, 0.1333333333)
    }

    func testMAEPerfectPrediction() {
        let result = LossFunctions.mae([1.0, 2.0, 3.0], [1.0, 2.0, 3.0])
        approxEqual(result, 0.0)
    }

    func testMAEDerivative() {
        let grad = LossFunctions.maeDerivative([1.0, 0.0, 0.0], [0.9, 0.1, 0.2])
        // signs: [neg, pos, pos], each ±1/3
        approxEqualArray(grad, [-1.0/3.0, 1.0/3.0, 1.0/3.0])
    }

    func testMAEDerivativeAtZero() {
        // When prediction == truth, derivative is 0 by convention
        let grad = LossFunctions.maeDerivative([1.0], [1.0])
        approxEqual(grad[0], 0.0)
    }

    // ========================================================================
    // MARK: - BCE Tests
    // ========================================================================

    func testBCEParityVector() {
        // From spec: y_true=[1,0,1], y_pred=[0.9,0.1,0.8] → 0.1446215275
        let result = LossFunctions.bce([1.0, 0.0, 1.0], [0.9, 0.1, 0.8])
        approxEqual(result, 0.1446215275, tol: 1e-6)
    }

    func testBCEPerfectPrediction() {
        // Near-perfect predictions should give near-zero loss
        let result = LossFunctions.bce([1.0, 0.0], [0.9999999, 0.0000001])
        XCTAssertLessThan(result, 0.001)
    }

    func testBCENonNegative() {
        // BCE should always be non-negative
        let result = LossFunctions.bce([1.0, 0.0, 1.0], [0.5, 0.5, 0.5])
        XCTAssertGreaterThanOrEqual(result, 0.0)
    }

    func testBCEDerivative() {
        let grad = LossFunctions.bceDerivative([1.0, 0.0], [0.9, 0.1])
        // Each element should be finite
        for g in grad {
            XCTAssertFalse(g.isNaN)
            XCTAssertFalse(g.isInfinite)
        }
    }

    func testBCEHandlesEdgePredictions() {
        // Should not crash with predictions at 0.0 or 1.0 (clamped)
        let result = LossFunctions.bce([1.0, 0.0], [0.0, 1.0])
        XCTAssertFalse(result.isNaN)
        XCTAssertFalse(result.isInfinite)
    }

    // ========================================================================
    // MARK: - CCE Tests
    // ========================================================================

    func testCCEParityVector() {
        // From spec: y_true=[1,0,0], y_pred=[0.8,0.1,0.1] → 0.07438118
        let result = LossFunctions.cce([1.0, 0.0, 0.0], [0.8, 0.1, 0.1])
        approxEqual(result, 0.07438118, tol: 1e-5)
    }

    func testCCEOnlyCorrectClassContributes() {
        // CCE: only the term where y=1 matters, so changing other preds
        // shouldn't affect the loss (as long as y=0 for those).
        let loss1 = LossFunctions.cce([1.0, 0.0, 0.0], [0.8, 0.1, 0.1])
        let loss2 = LossFunctions.cce([1.0, 0.0, 0.0], [0.8, 0.05, 0.15])
        approxEqual(loss1, loss2)
    }

    func testCCENonNegative() {
        let result = LossFunctions.cce([0.0, 1.0, 0.0], [0.1, 0.7, 0.2])
        XCTAssertGreaterThanOrEqual(result, 0.0)
    }

    func testCCEDerivative() {
        let grad = LossFunctions.cceDerivative([1.0, 0.0, 0.0], [0.8, 0.1, 0.1])
        // Gradient for correct class (y=1) should be negative (encouraging higher prob)
        XCTAssertLessThan(grad[0], 0.0)
        // Gradient for incorrect classes (y=0) should be ~0
        approxEqual(grad[1], 0.0, tol: 1e-5)
        approxEqual(grad[2], 0.0, tol: 1e-5)
    }

    func testCCEHandlesZeroPredictions() {
        // Should not crash with zero predictions (clamped)
        let result = LossFunctions.cce([1.0, 0.0], [0.0, 1.0])
        XCTAssertFalse(result.isNaN)
        XCTAssertFalse(result.isInfinite)
    }

    // ========================================================================
    // MARK: - Single Element Tests
    // ========================================================================

    func testSingleElementMSE() {
        approxEqual(LossFunctions.mse([1.0], [0.5]), 0.25)
    }

    func testSingleElementMAE() {
        approxEqual(LossFunctions.mae([1.0], [0.5]), 0.5)
    }

    // ========================================================================
    // MARK: - Large Array Test
    // ========================================================================

    func testLargeArrayMSE() {
        let n = 1000
        let yTrue = (0..<n).map { Double($0) / Double(n) }
        let yPred = yTrue  // perfect prediction
        approxEqual(LossFunctions.mse(yTrue, yPred), 0.0)
    }
}

import XCTest
@testable import Wave

final class WaveTests: XCTestCase {

    private let eps = 1e-10

    // ========================================================================
    // MARK: - Construction
    // ========================================================================

    func testBasicConstruction() {
        let w = Wave(amplitude: 1.0, frequency: 440.0)
        XCTAssertEqual(w.amplitude, 1.0)
        XCTAssertEqual(w.frequency, 440.0)
        XCTAssertEqual(w.phase, 0.0)
    }

    func testConstructionWithPhase() {
        let w = Wave(amplitude: 2.0, frequency: 100.0, phase: Double.pi / 2)
        XCTAssertEqual(w.phase, Double.pi / 2, accuracy: eps)
    }

    // ========================================================================
    // MARK: - Derived Properties
    // ========================================================================

    func testPeriod() {
        let w = Wave(amplitude: 1.0, frequency: 4.0)
        XCTAssertEqual(w.period, 0.25, accuracy: eps)
    }

    func testAngularFrequency() {
        let w = Wave(amplitude: 1.0, frequency: 1.0)
        XCTAssertEqual(w.angularFrequency, 2.0 * Double.pi, accuracy: eps)
    }

    // ========================================================================
    // MARK: - Evaluation: Parity Tests from Spec
    // ========================================================================

    func testZeroCrossing() {
        // Phase 0 wave evaluates to 0 at t=0
        let w = Wave(amplitude: 1.0, frequency: 1.0)
        XCTAssertEqual(w.evaluate(at: 0.0), 0.0, accuracy: eps)
    }

    func testPeak() {
        // 1 Hz wave reaches amplitude A at t=0.25 (quarter period)
        let w = Wave(amplitude: 3.0, frequency: 1.0)
        XCTAssertEqual(w.evaluate(at: 0.25), 3.0, accuracy: 1e-9)
    }

    func testPeriodicity() {
        // Value at t equals value at t+T
        let w = Wave(amplitude: 2.0, frequency: 5.0)
        let t = 0.123
        let period = w.period
        XCTAssertEqual(
            w.evaluate(at: t),
            w.evaluate(at: t + period),
            accuracy: 1e-9,
            "Wave should be periodic with period T")
    }

    func testPhaseShift() {
        // Phase π/2 starts at peak
        let w = Wave(amplitude: 1.0, frequency: 1.0, phase: Double.pi / 2)
        XCTAssertEqual(w.evaluate(at: 0.0), 1.0, accuracy: 1e-9)
    }

    func testTrough() {
        // 1 Hz wave reaches -A at t=0.75
        let w = Wave(amplitude: 2.0, frequency: 1.0)
        XCTAssertEqual(w.evaluate(at: 0.75), -2.0, accuracy: 1e-9)
    }

    func testZeroAmplitude() {
        let w = Wave(amplitude: 0.0, frequency: 1.0)
        XCTAssertEqual(w.evaluate(at: 0.5), 0.0, accuracy: eps)
    }

    // ========================================================================
    // MARK: - Multiple Time Points
    // ========================================================================

    func testMultipleTimePoints() {
        let w = Wave(amplitude: 1.0, frequency: 1.0)
        // sin(0)=0, sin(π/2)=1, sin(π)=0, sin(3π/2)=-1
        XCTAssertEqual(w.evaluate(at: 0.0), 0.0, accuracy: 1e-9)
        XCTAssertEqual(w.evaluate(at: 0.25), 1.0, accuracy: 1e-9)
        XCTAssertEqual(w.evaluate(at: 0.5), 0.0, accuracy: 1e-9)
        XCTAssertEqual(w.evaluate(at: 0.75), -1.0, accuracy: 1e-9)
    }

    // ========================================================================
    // MARK: - High Frequency
    // ========================================================================

    func testHighFrequency() {
        let w = Wave(amplitude: 1.0, frequency: 1000.0)
        XCTAssertEqual(w.period, 0.001, accuracy: eps)
        // Quarter-period peak
        XCTAssertEqual(w.evaluate(at: 0.00025), 1.0, accuracy: 1e-8)
    }

    // ========================================================================
    // MARK: - Opposite Phase Cancellation
    // ========================================================================

    func testOppositePhase() {
        let w1 = Wave(amplitude: 1.0, frequency: 1.0, phase: 0.0)
        let w2 = Wave(amplitude: 1.0, frequency: 1.0, phase: Double.pi)
        let t = 0.3
        // Two waves with opposite phase should cancel
        XCTAssertEqual(w1.evaluate(at: t) + w2.evaluate(at: t), 0.0, accuracy: 1e-9)
    }
}

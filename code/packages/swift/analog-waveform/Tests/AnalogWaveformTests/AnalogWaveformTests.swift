// ============================================================================
// AnalogWaveformTests.swift
// ============================================================================

import XCTest
@testable import AnalogWaveform

final class AnalogWaveformTests: XCTestCase {
    func testConstantWaveform() {
        let dc = ConstantWaveform(amplitude: 5.0)
        XCTAssertEqual(dc.sampleAt(0.0), 5.0)
        XCTAssertEqual(dc.sampleAt(1.5), 5.0)
        XCTAssertEqual(dc.sampleAt(999.9), 5.0)
    }

    func testSineWaveform() {
        let ac = SineWaveform(amplitude: 10.0, frequency: 1.0) // 1 Hz
        
        // At t=0, sin(0) = 0
        XCTAssertEqual(ac.sampleAt(0.0), 0.0, accuracy: 1e-9)
        
        // At t=0.25 (quarter cycle), sin(pi/2) = 1 => 10.0
        XCTAssertEqual(ac.sampleAt(0.25), 10.0, accuracy: 1e-9)
        
        // At t=0.5 (half cycle), sin(pi) = 0
        XCTAssertEqual(ac.sampleAt(0.5), 0.0, accuracy: 1e-9)
        
        // At t=0.75 (three quarter cycle), sin(3pi/2) = -1 => -10.0
        XCTAssertEqual(ac.sampleAt(0.75), -10.0, accuracy: 1e-9)
    }
}

// ============================================================================
// DiscreteWaveformTests.swift
// ============================================================================

import XCTest
import AnalogWaveform
@testable import DiscreteWaveform

final class DiscreteWaveformTests: XCTestCase {
    func testProperties() {
        let dw = DiscreteWaveform(samples: [1.0, 2.0, 3.0, 4.0], sampleRate: 2.0)
        XCTAssertEqual(dw.samplePeriod, 0.5)
        XCTAssertEqual(dw.duration, 2.0)
    }

    func testFromAnalog() {
        let cw = ConstantWaveform(amplitude: 5.0)
        let dw = DiscreteWaveform(from: cw, sampleRate: 10.0, duration: 1.0)
        XCTAssertEqual(dw.samples.count, 10)
        for s in dw.samples {
            XCTAssertEqual(s, 5.0)
        }
    }

    func testZeroOrderHold() {
        let dw = DiscreteWaveform(samples: [10.0, 20.0, 30.0], sampleRate: 1.0)
        
        // Before first sample
        XCTAssertEqual(dw.zeroOrderHold(at: -1.0), 10.0) 
        
        // At exact sample boundaries
        XCTAssertEqual(dw.zeroOrderHold(at: 0.0), 10.0)
        XCTAssertEqual(dw.zeroOrderHold(at: 1.0), 20.0)
        XCTAssertEqual(dw.zeroOrderHold(at: 2.0), 30.0)
        
        // Between samples (Zero Order Hold)
        XCTAssertEqual(dw.zeroOrderHold(at: 0.5), 10.0)
        XCTAssertEqual(dw.zeroOrderHold(at: 1.9), 20.0)
        XCTAssertEqual(dw.zeroOrderHold(at: 2.1), 30.0)
        
        // After last sample
        XCTAssertEqual(dw.zeroOrderHold(at: 5.0), 30.0)
    }
}

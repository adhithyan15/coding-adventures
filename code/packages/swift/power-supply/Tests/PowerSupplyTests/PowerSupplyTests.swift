// ============================================================================
// PowerSupplyTests.swift
// ============================================================================

import XCTest
@testable import PowerSupply

final class PowerSupplyTests: XCTestCase {
    func testIdealDCSupply() {
        let dc = IdealDCSupply(voltage: 9.0) // 9V battery
        XCTAssertEqual(dc.voltage(at: 0.0), 9.0)
        XCTAssertEqual(dc.voltage(at: 10.0), 9.0)
    }

    func testIdealSinusoidalSource() {
        let ac = IdealSinusoidalSource(peakVoltage: 170.0, frequency: 60.0) // 120V mains (approx 170V peak)
        
        XCTAssertEqual(ac.voltage(at: 0.0), 0.0, accuracy: 1e-6)
        
        // 1/4 cycle (1/240 seconds implies 90 deg phase) => max voltage
        XCTAssertEqual(ac.voltage(at: 1.0 / 240.0), 170.0, accuracy: 1e-6)
        
        // 1/2 cycle
        XCTAssertEqual(ac.voltage(at: 1.0 / 120.0), 0.0, accuracy: 1e-6)
    }
}

// ============================================================================
// ElectronicsTests.swift
// ============================================================================

import XCTest
import PowerSupply
import AnalogWaveform
@testable import Electronics

final class ElectronicsTests: XCTestCase {
    func testIdealResistor() {
        let res = IdealResistor(resistance: 100.0) // 100 Ohms
        XCTAssertEqual(res.current(voltage: 5.0), 0.05) // 5V / 100 Ohms = 50mA
        XCTAssertEqual(res.voltage(current: 0.02), 2.0) // 20mA * 100 Ohms = 2V
        XCTAssertEqual(res.power(voltage: 10.0), 1.0) // (10^2) / 100 = 1W
    }

    func testVoltageDivider() {
        // 5V input across 10k and 10k = 2.5V out
        let r1 = IdealResistor(resistance: 10_000.0)
        let r2 = IdealResistor(resistance: 10_000.0)
        let div = VoltageDivider(r1: r1, r2: r2)
        XCTAssertEqual(div.vOut(vIn: 5.0), 2.5)

        // 9V input across 2k and 1k = 3V across the 1k piece
        let div2 = VoltageDivider(r1: IdealResistor(resistance: 2000.0), r2: IdealResistor(resistance: 1000.0))
        XCTAssertEqual(div2.vOut(vIn: 9.0), 3.0)
    }

    func testDCAnalysis() {
        let dc = IdealDCSupply(voltage: 12.0)
        let r = IdealResistor(resistance: 24.0)
        
        XCTAssertEqual(DCAnalysis.current(supply: dc, resistor: r), 0.5)
        XCTAssertEqual(DCAnalysis.powerDissipation(supply: dc, resistor: r), 6.0)
    }

    func testSinusoidalResponse() {
        let ac = IdealSinusoidalSource(peakVoltage: 10.0, frequency: 1.0)
        let r = IdealResistor(resistance: 5.0)
        let response = SinusoidalResistorResponse(source: ac, resistor: r)
        
        // At t=0, V=0
        XCTAssertEqual(response.current(at: 0.0), 0.0, accuracy: 1e-6)
        XCTAssertEqual(response.instantaneousPower(at: 0.0), 0.0, accuracy: 1e-6)
        
        // At t=0.25, V=10.0
        XCTAssertEqual(response.current(at: 0.25), 2.0, accuracy: 1e-6) // 10V / 5 Ohms
        XCTAssertEqual(response.instantaneousPower(at: 0.25), 20.0, accuracy: 1e-6) // (10*10) / 5
    }
}

import XCTest
@testable import Transistors

// ============================================================================
// TransistorsTests — Comprehensive tests for the Transistors module
// ============================================================================
//
// Test strategy:
//
//   1. MOSFET tests — region classification and current equations
//   2. BJT tests — region classification and Ebers-Moll current
//   3. CMOS gate tests — truth tables and digital evaluation
//   4. TTL gate tests — truth tables
//   5. Amplifier tests — gain and bandwidth calculations
//   6. Analysis tests — noise margins, power, timing
//
// All truth table tests use the digital evaluation API (0/1 inputs/outputs)
// since that is the interface consumed by the logic-gates package.
// ============================================================================

// MARK: - MOSFET Tests

final class NMOSTests: XCTestCase {

    let nmos = NMOS()

    func testCutoffWhenVgsBelowVth() {
        XCTAssertEqual(nmos.region(vgs: 0.0, vds: 1.0), .cutoff)
    }

    func testCutoffAtExactlyVth() {
        XCTAssertEqual(nmos.region(vgs: 0.4, vds: 1.0), .cutoff)
    }

    func testLinearRegion() {
        // Vgs = 1.0, Vds = 0.2, overdrive = 0.6 → Vds < Vov → linear
        XCTAssertEqual(nmos.region(vgs: 1.0, vds: 0.2), .linear)
    }

    func testSaturationRegion() {
        // Vds = 1.0 > overdrive = 0.6 → saturation
        XCTAssertEqual(nmos.region(vgs: 1.0, vds: 1.0), .saturation)
    }

    func testNotConductingBelowThreshold() {
        XCTAssertFalse(nmos.isConducting(vgs: 0.3))
    }

    func testConductingAboveThreshold() {
        XCTAssertTrue(nmos.isConducting(vgs: 0.5))
    }

    func testZeroCurrentInCutoff() {
        XCTAssertEqual(nmos.drainCurrent(vgs: 0.0, vds: 1.0), 0.0)
    }

    func testPositiveCurrentInLinear() {
        XCTAssertGreaterThan(nmos.drainCurrent(vgs: 1.0, vds: 0.2), 0.0)
    }

    func testPositiveCurrentInSaturation() {
        XCTAssertGreaterThan(nmos.drainCurrent(vgs: 1.0, vds: 1.5), 0.0)
    }

    func testSaturationCurrentGreaterThanDeepLinear() {
        let idLinear = nmos.drainCurrent(vgs: 1.0, vds: 0.1)
        let idSat    = nmos.drainCurrent(vgs: 1.0, vds: 1.5)
        XCTAssertGreaterThan(idSat, idLinear)
    }

    func testZeroTransconductanceInCutoff() {
        XCTAssertEqual(nmos.transconductance(vgs: 0.0, vds: 1.0), 0.0)
    }

    func testPositiveTransconductanceInSaturation() {
        XCTAssertGreaterThan(nmos.transconductance(vgs: 1.0, vds: 1.5), 0.0)
    }
}

final class PMOSTests: XCTestCase {

    let pmos = PMOS()

    func testCutoffWhenVgsPositive() {
        XCTAssertEqual(pmos.region(vgs: 0.5, vds: -1.0), .cutoff)
    }

    func testConductingWhenVgsNegativeEnough() {
        XCTAssertTrue(pmos.isConducting(vgs: -1.0))
    }

    func testNotConductingWhenVgsNearZero() {
        XCTAssertFalse(pmos.isConducting(vgs: -0.1))
    }

    func testPositiveCurrentWhenConducting() {
        XCTAssertGreaterThan(pmos.drainCurrent(vgs: -1.0, vds: -1.5), 0.0)
    }

    func testZeroCurrentInCutoff() {
        XCTAssertEqual(pmos.drainCurrent(vgs: 0.0, vds: -1.5), 0.0)
    }

    func testComplementaryToNMOS() {
        // In a 1.8 V CMOS inverter: when NMOS is ON at Vin=1.8, PMOS is OFF
        let nmos = NMOS()
        let vin: Double = 1.8
        XCTAssertTrue(nmos.isConducting(vgs: vin))
        XCTAssertFalse(pmos.isConducting(vgs: vin - 1.8))
    }
}

// MARK: - BJT Tests

final class NPNTests: XCTestCase {

    let npn = NPN()

    func testCutoffWhenVbeLow() {
        XCTAssertEqual(npn.region(vbe: 0.5, vce: 1.0), .cutoff)
    }

    func testActiveRegion() {
        XCTAssertEqual(npn.region(vbe: 0.7, vce: 1.0), .active)
    }

    func testSaturationRegion() {
        // Vce = 0.1 < VceSat = 0.2
        XCTAssertEqual(npn.region(vbe: 0.7, vce: 0.1), .saturation)
    }

    func testNotConductingBelowVbeOn() {
        XCTAssertFalse(npn.isConducting(vbe: 0.6))
    }

    func testConductingAtVbeOn() {
        XCTAssertTrue(npn.isConducting(vbe: 0.7))
    }

    func testZeroCollectorCurrentInCutoff() {
        XCTAssertEqual(npn.collectorCurrent(vbe: 0.5, vce: 1.0), 0.0)
    }

    func testPositiveCollectorCurrentInActive() {
        XCTAssertGreaterThan(npn.collectorCurrent(vbe: 0.7, vce: 1.0), 0.0)
    }

    func testCurrentGainRelationship() {
        // Ic ≈ β × Ib in active region
        let ic = npn.collectorCurrent(vbe: 0.7, vce: 1.0)
        let ib = npn.baseCurrent(vbe: 0.7, vce: 1.0)
        XCTAssertEqual(ic / ib, npn.params.beta, accuracy: npn.params.beta * 0.01)
    }

    func testTransconductancePositiveInActive() {
        XCTAssertGreaterThan(npn.transconductance(vbe: 0.7, vce: 1.0), 0.0)
    }

    func testExponentClampingPreventsInfinity() {
        // Very high Vbe must not produce NaN or Inf
        let ic = npn.collectorCurrent(vbe: 5.0, vce: 1.0)
        XCTAssertFalse(ic.isNaN)
        XCTAssertFalse(ic.isInfinite)
    }
}

final class PNPTests: XCTestCase {

    let pnp = PNP()

    func testCutoffWhenVbePositive() {
        XCTAssertEqual(pnp.region(vbe: 0.5, vce: -1.0), .cutoff)
    }

    func testActiveRegion() {
        XCTAssertEqual(pnp.region(vbe: -0.7, vce: -1.0), .active)
    }

    func testConductingNegativeVbe() {
        XCTAssertTrue(pnp.isConducting(vbe: -0.7))
    }

    func testNotConductingPositiveVbe() {
        XCTAssertFalse(pnp.isConducting(vbe: 0.7))
    }
}

// MARK: - CMOS Gate Truth Table Tests

final class CMOSInverterTests: XCTestCase {

    let inv = CMOSInverter()

    func testNOT0() { XCTAssertEqual(inv.evaluateDigital(0), 1) }
    func testNOT1() { XCTAssertEqual(inv.evaluateDigital(1), 0) }

    func testAnalogLowGivesHighOutput() {
        XCTAssertGreaterThan(inv.evaluate(inputVoltage: 0.0).voltage, inv.circuit.vdd * 0.5)
    }

    func testAnalogHighGivesLowOutput() {
        XCTAssertLessThan(inv.evaluate(inputVoltage: inv.circuit.vdd).voltage, inv.circuit.vdd * 0.5)
    }

    func testTransistorCount() {
        XCTAssertEqual(inv.evaluate(inputVoltage: 0.0).transistorCount, 2)
    }

    func testVTCHas100Points() {
        XCTAssertEqual(inv.voltageTransferCharacteristic(steps: 100).count, 100)
    }

    func testVTCStartsNearZero() {
        let vtc = inv.voltageTransferCharacteristic(steps: 100)
        XCTAssertLessThan(vtc.first!.vin, 0.01)
    }

    func testVTCEndsNearVdd() {
        let vtc = inv.voltageTransferCharacteristic(steps: 100)
        XCTAssertGreaterThan(vtc.last!.vin, inv.circuit.vdd * 0.99)
    }
}

final class CMOSNandTests: XCTestCase {

    let nand = CMOSNand()

    func testNAND00() { XCTAssertEqual(nand.evaluateDigital(0, 0), 1) }
    func testNAND01() { XCTAssertEqual(nand.evaluateDigital(0, 1), 1) }
    func testNAND10() { XCTAssertEqual(nand.evaluateDigital(1, 0), 1) }
    func testNAND11() { XCTAssertEqual(nand.evaluateDigital(1, 1), 0) }

    func testTransistorCount() {
        XCTAssertEqual(nand.evaluate(va: 0.0, vb: 0.0).transistorCount, 4)
    }
}

final class CMOSNorTests: XCTestCase {

    let nor = CMOSNor()

    func testNOR00() { XCTAssertEqual(nor.evaluateDigital(0, 0), 1) }
    func testNOR01() { XCTAssertEqual(nor.evaluateDigital(0, 1), 0) }
    func testNOR10() { XCTAssertEqual(nor.evaluateDigital(1, 0), 0) }
    func testNOR11() { XCTAssertEqual(nor.evaluateDigital(1, 1), 0) }

    func testTransistorCount() {
        XCTAssertEqual(nor.evaluate(va: 0.0, vb: 0.0).transistorCount, 4)
    }
}

final class CMOSAndTests: XCTestCase {

    let and = CMOSAnd()

    func testAND00() { XCTAssertEqual(and.evaluateDigital(0, 0), 0) }
    func testAND01() { XCTAssertEqual(and.evaluateDigital(0, 1), 0) }
    func testAND10() { XCTAssertEqual(and.evaluateDigital(1, 0), 0) }
    func testAND11() { XCTAssertEqual(and.evaluateDigital(1, 1), 1) }

    func testTransistorCount() {
        XCTAssertEqual(and.evaluate(va: 0.0, vb: 0.0).transistorCount, 6)
    }
}

final class CMOSOrTests: XCTestCase {

    let or = CMOSOr()

    func testOR00() { XCTAssertEqual(or.evaluateDigital(0, 0), 0) }
    func testOR01() { XCTAssertEqual(or.evaluateDigital(0, 1), 1) }
    func testOR10() { XCTAssertEqual(or.evaluateDigital(1, 0), 1) }
    func testOR11() { XCTAssertEqual(or.evaluateDigital(1, 1), 1) }

    func testTransistorCount() {
        XCTAssertEqual(or.evaluate(va: 0.0, vb: 0.0).transistorCount, 6)
    }
}

final class CMOSXorTests: XCTestCase {

    let xor = CMOSXor()

    func testXOR00() { XCTAssertEqual(xor.evaluateDigital(0, 0), 0) }
    func testXOR01() { XCTAssertEqual(xor.evaluateDigital(0, 1), 1) }
    func testXOR10() { XCTAssertEqual(xor.evaluateDigital(1, 0), 1) }
    func testXOR11() { XCTAssertEqual(xor.evaluateDigital(1, 1), 0) }

    func testTransistorCount() {
        XCTAssertEqual(xor.evaluate(va: 0.0, vb: 0.0).transistorCount, 12)
    }
}

// MARK: - TTL Gate Tests

final class TTLNandTests: XCTestCase {

    let gate = TTLNand()

    func testTTLNAND00() { XCTAssertEqual(gate.evaluateDigital(0, 0), 1) }
    func testTTLNAND01() { XCTAssertEqual(gate.evaluateDigital(0, 1), 1) }
    func testTTLNAND10() { XCTAssertEqual(gate.evaluateDigital(1, 0), 1) }
    func testTTLNAND11() { XCTAssertEqual(gate.evaluateDigital(1, 1), 0) }

    func testStaticPowerIsPositive() {
        XCTAssertGreaterThan(gate.staticPower(), 0.0)
    }

    func testOutputLowMeetsSpec() {
        // Both HIGH → output ≤ 0.4 V (TTL VOL spec)
        let out = gate.evaluate(va: gate.vcc * 0.7, vb: gate.vcc * 0.7)
        XCTAssertLessThanOrEqual(out.voltage, 0.4)
    }

    func testOutputHighMeetsSpec() {
        // One LOW → output ≥ 2.4 V (TTL VOH spec)
        let out = gate.evaluate(va: 0.2, vb: gate.vcc * 0.7)
        XCTAssertGreaterThanOrEqual(out.voltage, 2.4)
    }
}

final class RTLInverterTests: XCTestCase {

    let inv = RTLInverter()

    func testRTLNOT0() { XCTAssertEqual(inv.evaluateDigital(0), 1) }
    func testRTLNOT1() { XCTAssertEqual(inv.evaluateDigital(1), 0) }

    func testTransistorCount() {
        XCTAssertEqual(inv.evaluate(inputVoltage: 0.0).transistorCount, 1)
    }
}

// MARK: - Amplifier Tests

final class AmplifierTests: XCTestCase {

    func testCommonSourceGainIsNegative() {
        let amp = analyzeCommonSource(
            transistor: NMOS(), vgs: 0.8, vdd: 1.8,
            rDrain: 10_000, cLoad: 10e-15
        )
        XCTAssertLessThan(amp.voltageGain, 0.0)
    }

    func testCommonSourceTransconductancePositive() {
        let amp = analyzeCommonSource(
            transistor: NMOS(), vgs: 0.8, vdd: 1.8,
            rDrain: 10_000, cLoad: 10e-15
        )
        XCTAssertGreaterThan(amp.transconductance, 0.0)
    }

    func testCommonSourceBandwidthFiniteAndPositive() {
        let amp = analyzeCommonSource(
            transistor: NMOS(), vgs: 0.8, vdd: 1.8,
            rDrain: 10_000, cLoad: 10e-15
        )
        XCTAssertGreaterThan(amp.bandwidth, 0.0)
        XCTAssertFalse(amp.bandwidth.isInfinite)
    }

    func testCommonEmitterGainIsNegative() {
        let amp = analyzeCommonEmitter(
            transistor: NPN(), vbe: 0.7, vcc: 5.0,
            rCollector: 1_000, cLoad: 10e-12
        )
        XCTAssertLessThan(amp.voltageGain, 0.0)
    }

    func testMOSFETInputImpedanceVeryHigh() {
        let amp = analyzeCommonSource(
            transistor: NMOS(), vgs: 0.8, vdd: 1.8,
            rDrain: 10_000, cLoad: 10e-15
        )
        XCTAssertGreaterThan(amp.inputImpedance, 1e9)
    }
}

// MARK: - Analysis Tests

final class NoiseMarginTests: XCTestCase {

    func testCMOSNoiseMarginsPositive() {
        let nm = computeNoiseMargins(inverter: CMOSInverter())
        XCTAssertGreaterThan(nm.nml, 0.0)
        XCTAssertGreaterThan(nm.nmh, 0.0)
    }

    func testCMOSVOLLessThanVIL() {
        let nm = computeNoiseMargins(inverter: CMOSInverter())
        XCTAssertLessThan(nm.vol, nm.vil)
    }

    func testCMOSVIHLessThanVOH() {
        let nm = computeNoiseMargins(inverter: CMOSInverter())
        XCTAssertLessThan(nm.vih, nm.voh)
    }

    func testTTLNoiseMarginsPositive() {
        let nm = computeTTLNoiseMargins()
        XCTAssertGreaterThan(nm.nml, 0.0)
        XCTAssertGreaterThan(nm.nmh, 0.0)
    }

    func testTTLVolLow() {
        XCTAssertLessThan(computeTTLNoiseMargins().vol, 0.5)
    }

    func testTTLVohHigh() {
        XCTAssertGreaterThan(computeTTLNoiseMargins().voh, 2.4)
    }
}

final class PowerAnalysisTests: XCTestCase {

    let inv = CMOSInverter()

    func testDynamicPowerIncreasesWithFrequency() {
        let lo = analyzePower(inverter: inv, frequency: 1e6,  cLoad: 10e-15)
        let hi = analyzePower(inverter: inv, frequency: 1e9,  cLoad: 10e-15)
        XCTAssertGreaterThan(hi.dynamicPower, lo.dynamicPower)
    }

    func testTotalPowerEqualsStaticPlusDynamic() {
        let a = analyzePower(inverter: inv, frequency: 1e9, cLoad: 10e-15)
        XCTAssertEqual(a.totalPower, a.staticPower + a.dynamicPower, accuracy: 1e-20)
    }

    func testEnergyPerSwitchPositive() {
        XCTAssertGreaterThan(
            analyzePower(inverter: inv, frequency: 1e9, cLoad: 10e-15).energyPerSwitch, 0.0
        )
    }

    func testCMOSStaticPowerNearlyZero() {
        let a = analyzePower(inverter: inv, frequency: 0.0, cLoad: 10e-15)
        XCTAssertLessThan(a.staticPower, 1e-8)
    }

    func testTTLStaticPowerSignificant() {
        let a = analyzeTTLPower(gate: TTLNand(), frequency: 0.0, cLoad: 10e-15)
        XCTAssertGreaterThan(a.staticPower, 1e-4)
    }
}

final class TimingAnalysisTests: XCTestCase {

    func testPropagationDelaysPositive() {
        let t = analyzeTiming(inverter: CMOSInverter(), cLoad: 10e-15)
        XCTAssertGreaterThan(t.tphl, 0.0)
        XCTAssertGreaterThan(t.tplh, 0.0)
    }

    func testAverageDelayIsArithmetic() {
        let t = analyzeTiming(inverter: CMOSInverter(), cLoad: 10e-15)
        XCTAssertEqual(t.tpd, (t.tphl + t.tplh) / 2.0, accuracy: 1e-30)
    }

    func testMaxFrequencyInverseOfDelay() {
        let t = analyzeTiming(inverter: CMOSInverter(), cLoad: 10e-15)
        XCTAssertEqual(t.maxFrequency, 1.0 / (2.0 * t.tpd), accuracy: 1.0)
    }
}

final class CMOSvsTTLTests: XCTestCase {

    func testComparisonReturnsSixProperties() {
        XCTAssertEqual(compareCMOSvsTTL().count, 6)
    }

    func testCMOSStaticPowerLowerThanTTL() {
        let row = compareCMOSvsTTL().first { $0.property.contains("Static") }!
        XCTAssertLessThan(row.cmos, row.ttl)
    }

    func testCMOSSupplyLowerThanTTL() {
        let row = compareCMOSvsTTL().first { $0.property.contains("Supply") }!
        XCTAssertLessThan(row.cmos, row.ttl)
    }
}

final class MooresLawTests: XCTestCase {

    func testResultCountMatchesNodeCount() {
        let nodes = [180.0, 130.0, 90.0, 65.0]
        XCTAssertEqual(demonstrateCMOSScaling(nodes: nodes).count, nodes.count)
    }

    func testSmallerNodeHigherFrequency() {
        let r = demonstrateCMOSScaling(nodes: [180.0, 90.0])
        XCTAssertGreaterThan(r[1].frequency, r[0].frequency)
    }

    func testSmallerNodeLowerVdd() {
        let r = demonstrateCMOSScaling(nodes: [180.0, 90.0])
        XCTAssertLessThan(r[1].vdd, r[0].vdd)
    }

    func testSmallerNodeHigherDensity() {
        let r = demonstrateCMOSScaling(nodes: [180.0, 90.0])
        XCTAssertGreaterThan(r[1].transistorsPerMM2, r[0].transistorsPerMM2)
    }
}

// MARK: - Types Tests

final class TypesTests: XCTestCase {

    func testDefaultNMOSParams() {
        let p = MOSFETParams.defaultNMOS
        XCTAssertEqual(p.vth, 0.4)
        XCTAssertEqual(p.k, 200e-6)
    }

    func testDefaultPMOSHalfNMOSMobility() {
        XCTAssertEqual(MOSFETParams.defaultPMOS.k, 100e-6)
    }

    func testDefaultNPNBeta() {
        XCTAssertEqual(BJTParams.defaultNPN.beta, 200.0)
    }

    func testThermalVoltageAt300K() {
        let vt = CircuitParams(vdd: 1.8, temperature: 300.0).thermalVoltage
        XCTAssertEqual(vt, 0.02585, accuracy: 0.0001)
    }

    func testNoiseMarginFormulas() {
        let nm = NoiseMargins(vol: 0.1, voh: 1.7, vil: 0.5, vih: 1.3)
        XCTAssertEqual(nm.nml, 0.4, accuracy: 1e-10)
        XCTAssertEqual(nm.nmh, 0.4, accuracy: 1e-10)
    }

    func testPowerAnalysisTotalIsSum() {
        let pa = PowerAnalysis(staticPower: 1e-9, dynamicPower: 1e-6, energyPerSwitch: 1e-15)
        XCTAssertEqual(pa.totalPower, pa.staticPower + pa.dynamicPower, accuracy: 1e-30)
    }

    func testTimingMaxFrequencyFormula() {
        let t = TimingAnalysis(tphl: 1e-10, tplh: 1e-10, riseTime: 2e-10, fallTime: 2e-10)
        XCTAssertEqual(t.maxFrequency, 5e9, accuracy: 1.0)
    }

    func testVersionDefined() {
        XCTAssertFalse(version.isEmpty)
    }
}

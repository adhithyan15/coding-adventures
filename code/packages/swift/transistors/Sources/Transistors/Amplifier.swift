// Amplifier.swift
// Part of coding-adventures — an educational computing stack.
//
// ============================================================================
// Amplifier — Single-Stage Transistor Amplifier Analysis
// ============================================================================
//
// Transistors are not only switches — they are also amplifiers. The same
// device that implements a NOT gate in digital logic can amplify an audio
// signal in an analog circuit.
//
// # Common-Source Amplifier (MOSFET)
//
//   Vdd
//    │
//   [Rd] drain resistor
//    │
//    ├───── Vout
//    │
//   [NMOS drain]
//   [NMOS gate] ← Vin (small AC signal)
//   [NMOS source]
//    │
//   GND
//
// The gate controls the drain current: Id = gm × Vgs.
// With drain resistor Rd: Vout = −gm × Rd × Vin.
//
// Key result: Voltage gain = −gm × Rd (negative = inverting).
//
// # Common-Emitter Amplifier (BJT)
//
//   Vcc
//    │
//   [Rc] collector resistor
//    │
//    ├───── Vout
//    │
//   [NPN collector]
//   [NPN base] ← Vin
//   [NPN emitter]
//    │
//   GND
//
// Voltage gain = −gm × Rc (same form as common-source).
//
// # Why Negative Gain?
//
// Both configurations are inverting. When Vin increases, the transistor
// conducts more current, pulling the output down (toward GND).
// The sign inversion is why a MOSFET inverter works — it's an amplifier
// with gain so high that the output saturates at either Vdd or GND.
//
// ============================================================================

/// Analysis of a common-source MOSFET amplifier.
///
/// - Parameters:
///   - transistor: The NMOS device.
///   - vgs: DC bias gate-source voltage (V).
///   - vdd: Supply voltage (V).
///   - rDrain: Drain load resistor (Ω).
///   - cLoad: Load capacitance (F). Limits bandwidth.
/// - Returns: Small-signal analysis result.
public func analyzeCommonSource(
    transistor: NMOS,
    vgs: Double,
    vdd: Double,
    rDrain: Double,
    cLoad: Double
) -> AmplifierAnalysis {
    // DC operating point: Vds = Vdd − Id × Rd
    let vds = vdd / 2.0  // assume biased at midpoint for maximum swing
    let gm = transistor.transconductance(vgs: vgs, vds: vds)

    // Voltage gain: Av = −gm × Rd (inverting)
    let voltageGain = -gm * rDrain

    // Input impedance: essentially infinite (oxide gate)
    let inputImpedance = 1e12

    // Output impedance: Rd in parallel with transistor ro (≈ Rd for long channel)
    let outputImpedance = rDrain

    // −3 dB bandwidth: f = 1 / (2π × Rd × Cload)
    let bandwidth = cLoad > 0 ? 1.0 / (2.0 * Double.pi * rDrain * cLoad) : 1e9

    // DC operating point voltage at drain
    let id = transistor.drainCurrent(vgs: vgs, vds: vds)
    let operatingPoint = vdd - id * rDrain

    return AmplifierAnalysis(
        voltageGain: voltageGain,
        transconductance: gm,
        inputImpedance: inputImpedance,
        outputImpedance: outputImpedance,
        bandwidth: bandwidth,
        operatingPoint: operatingPoint
    )
}

/// Analysis of a common-emitter BJT amplifier.
///
/// - Parameters:
///   - transistor: The NPN device.
///   - vbe: DC bias base-emitter voltage (V). Typically 0.65–0.75 V.
///   - vcc: Supply voltage (V).
///   - rCollector: Collector load resistor (Ω).
///   - cLoad: Load capacitance (F).
/// - Returns: Small-signal analysis result.
public func analyzeCommonEmitter(
    transistor: NPN,
    vbe: Double,
    vcc: Double,
    rCollector: Double,
    cLoad: Double
) -> AmplifierAnalysis {
    // DC operating point
    let vce = vcc / 2.0  // assume midpoint bias
    let gm = transistor.transconductance(vbe: vbe, vce: vce)
    let ic = transistor.collectorCurrent(vbe: vbe, vce: vce)

    // Voltage gain: Av = −gm × Rc
    let voltageGain = -gm * rCollector

    // Input impedance: rπ = β / gm (base looking in)
    let rPi = gm > 0 ? transistor.params.beta / gm : 1e6
    let inputImpedance = rPi

    // Output impedance: ≈ Rc (early voltage ignored in simplified model)
    let outputImpedance = rCollector

    // Bandwidth: f = 1 / (2π × Rc × Cload)
    let bandwidth = cLoad > 0 ? 1.0 / (2.0 * Double.pi * rCollector * cLoad) : 1e9

    // DC operating point voltage at collector
    let operatingPoint = vcc - ic * rCollector

    return AmplifierAnalysis(
        voltageGain: voltageGain,
        transconductance: gm,
        inputImpedance: inputImpedance,
        outputImpedance: outputImpedance,
        bandwidth: bandwidth,
        operatingPoint: operatingPoint
    )
}

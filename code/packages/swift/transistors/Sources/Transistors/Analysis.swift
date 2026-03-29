// Analysis.swift
// Part of coding-adventures — an educational computing stack.
//
// ============================================================================
// Analysis — Electrical Characterization of Logic Gates
// ============================================================================
//
// Beyond evaluating whether a gate outputs 0 or 1, circuit designers need
// to know how robust, fast, and power-hungry their logic is. This module
// provides three analyses:
//
//   1. Noise Margins — how much electrical noise can the gate tolerate?
//   2. Power Analysis — how much energy does each switching event cost?
//   3. Timing Analysis — how fast can the gate switch?
//
// # Noise Margins (the "eye opening" of digital logic)
//
//   Vdd ─────────────────── VOH (output HIGH level)
//         ┌─────────────── VIH (input HIGH threshold)
//         │   ← NMH (high noise margin)
//         │
//         │
//         └─────────────── VIL (input LOW threshold)
//         ┌─────────────── VOL (output LOW level)
//   GND ─────────────────── 0 V
//
//   NMH = VOH − VIH   (a noise spike of NMH V is tolerated on a HIGH signal)
//   NML = VIL − VOL   (a noise spike of NML V is tolerated on a LOW signal)
//
// For 180 nm CMOS at 1.8 V: NMH ≈ NML ≈ 0.7 V — very comfortable margins.
// For TTL at 5 V: NMH ≈ 0.4 V, NML ≈ 0.4 V — tighter, but adequate.
//
// # CMOS vs TTL Comparison
//
//   | Property      | CMOS (180 nm, 1.8 V) | TTL (5 V) |
//   |---------------|----------------------|-----------|
//   | Static power  | ≈ 0                  | ≈ 10 mW   |
//   | Dynamic power | C × Vdd² × f        | moderate  |
//   | Speed         | GHz capable          | ≈ 50 MHz  |
//   | Noise margins | ≈ 0.7 V (38% of Vdd) | ≈ 0.4 V  |
//   | Fan-out       | very high            | limited   |
//
// ============================================================================

// MARK: - Noise Margin Analysis

/// Computes noise margins for a CMOS inverter.
///
/// The noise margins are derived from the inverter's voltage transfer
/// characteristic (VTC). VIL and VIH are the input voltages where the
/// VTC slope = −1 (unity gain crossing points).
///
/// - Parameters:
///   - inverter: The CMOS inverter to analyze.
/// - Returns: Noise margin measurements.
public func computeNoiseMargins(inverter: CMOSInverter) -> NoiseMargins {
    let vdd = inverter.circuit.vdd

    // For an ideal symmetric CMOS inverter:
    //   VIL = Vdd/2 − Vdd/(2×(n+1))  where n = kn/kp ratio
    //   VIH = Vdd/2 + Vdd/(2×(n+1))
    // For n = 1 (symmetric): VIL = Vdd/3, VIH = 2×Vdd/3
    //
    // Simplified: sample the VTC and find the unity-gain crossings.
    let vtc = inverter.voltageTransferCharacteristic(steps: 200)

    var vil = vdd / 3.0   // fallback approximation
    var vih = 2.0 * vdd / 3.0

    // Find where |slope| = 1 using finite differences
    for i in 1..<vtc.count {
        let (vin0, vout0) = (vtc[i-1].vin, vtc[i-1].vout)
        let (vin1, vout1) = (vtc[i].vin, vtc[i].vout)
        let dvin = vin1 - vin0
        guard dvin > 0 else { continue }
        let slope = (vout1 - vout0) / dvin
        if slope < -1.0 && vin0 < vdd / 2.0 {
            vil = vin0   // last point with slope > −1 on the low side
        }
        if slope < -1.0 && vin0 > vdd / 2.0 {
            vih = vin1   // first point past the midpoint where slope < −1
            break
        }
    }

    let vol = vdd * 0.05  // CMOS output LOW ≈ 5% of Vdd
    let voh = vdd * 0.95  // CMOS output HIGH ≈ 95% of Vdd

    return NoiseMargins(vol: vol, voh: voh, vil: vil, vih: vih)
}

/// Computes noise margins for a TTL NAND gate (approximate model).
///
/// TTL uses defined logic levels from the 7400-series datasheet:
///   VOL ≤ 0.4 V, VOH ≥ 2.4 V, VIL ≤ 0.8 V, VIH ≥ 2.0 V
public func computeTTLNoiseMargins() -> NoiseMargins {
    return NoiseMargins(
        vol: 0.2,   // typical VOL
        voh: 3.4,   // typical VOH at 5 V
        vil: 0.8,   // max input LOW (datasheet)
        vih: 2.0    // min input HIGH (datasheet)
    )
}

// MARK: - Power Analysis

/// Analyzes the power consumption of a CMOS inverter.
///
/// CMOS power has two components:
///
/// **Static (leakage) power**: Subthreshold current flows even when the
///   transistors are "off". In modern processes this is a real concern, but
///   in our 180 nm model it is negligibly small.
///
/// **Dynamic (switching) power**: Each time the output switches, the load
///   capacitance charges or discharges through the supply:
///   P_dyn = α × C × Vdd² × f
///   where α (activity factor) ≈ 0.5 for a random data stream.
///
/// - Parameters:
///   - inverter: The CMOS inverter.
///   - frequency: Switching frequency (Hz).
///   - cLoad: Load capacitance (F).
///   - activityFactor: Fraction of cycles that involve a 0→1 transition (0–1).
/// - Returns: Power breakdown.
public func analyzePower(
    inverter: CMOSInverter,
    frequency: Double,
    cLoad: Double,
    activityFactor: Double = 0.5
) -> PowerAnalysis {
    let vdd = inverter.circuit.vdd
    let staticP = inverter.staticPower()
    let dynamicP = activityFactor * cLoad * vdd * vdd * frequency
    let energyPerSwitch = cLoad * vdd * vdd  // E = C×V²

    return PowerAnalysis(
        staticPower: staticP,
        dynamicPower: dynamicP,
        energyPerSwitch: energyPerSwitch
    )
}

/// Analyzes power of a TTL NAND gate.
///
/// TTL draws significant static current regardless of switching activity —
/// this is why TTL-heavy designs run hot.
///
/// - Parameters:
///   - gate: The TTL NAND gate.
///   - frequency: Switching frequency (Hz).
///   - cLoad: Load capacitance (F).
public func analyzeTTLPower(
    gate: TTLNand,
    frequency: Double,
    cLoad: Double
) -> PowerAnalysis {
    let staticP = gate.staticPower()
    let dynamicP = 0.5 * cLoad * gate.vcc * gate.vcc * frequency
    let energyPerSwitch = cLoad * gate.vcc * gate.vcc

    return PowerAnalysis(
        staticPower: staticP,
        dynamicPower: dynamicP,
        energyPerSwitch: energyPerSwitch
    )
}

// MARK: - Timing Analysis

/// Analyzes switching timing of a CMOS inverter.
///
/// Propagation delays are dominated by the RC time constant of the output
/// node: τ = R_eq × C_load.
///
/// - tpHL: propagation delay HIGH-to-LOW (output falls)
///         τ = (Cload × Vdd) / Id_nmos  (NMOS pulls output down)
/// - tpLH: propagation delay LOW-to-HIGH (output rises)
///         τ = (Cload × Vdd) / Id_pmos  (PMOS pulls output up)
///
/// - Parameters:
///   - inverter: The CMOS inverter.
///   - cLoad: Total load capacitance (F).
public func analyzeTiming(
    inverter: CMOSInverter,
    cLoad: Double
) -> TimingAnalysis {
    let vdd = inverter.circuit.vdd

    // Evaluate both transistors at Vgs = Vdd (fully ON)
    let idNmos = inverter.nmos.drainCurrent(vgs: vdd, vds: vdd / 2.0)
    let idPmos = inverter.pmos.drainCurrent(vgs: -vdd, vds: -vdd / 2.0)

    // tpHL: NMOS charging Cload to Vdd/2 (falling transition)
    let tphl = idNmos > 0 ? (cLoad * vdd / 2.0) / idNmos : 1e-9
    // tpLH: PMOS charging Cload to Vdd/2 (rising transition)
    let tplh = idPmos > 0 ? (cLoad * vdd / 2.0) / idPmos : 1.5e-9

    // Rise/fall times: 10%→90% ≈ 2.2 × RC
    let rEqNmos = vdd / (2.0 * max(idNmos, 1e-12))
    let rEqPmos = vdd / (2.0 * max(idPmos, 1e-12))
    let fallTime = 2.2 * rEqNmos * cLoad
    let riseTime = 2.2 * rEqPmos * cLoad

    return TimingAnalysis(tphl: tphl, tplh: tplh, riseTime: riseTime, fallTime: fallTime)
}

// MARK: - CMOS vs TTL Comparison

/// Compares CMOS and TTL gate characteristics at a given operating frequency.
///
/// Returns a dictionary mapping property names to (CMOS value, TTL value) tuples.
/// Useful for understanding the trade-offs between the two logic families.
public func compareCMOSvsTTL(
    frequency: Double = 1e6,
    cLoad: Double = 10e-15
) -> [(property: String, cmos: Double, ttl: Double)] {
    let cmosInverter = CMOSInverter(circuit: .cmos18)
    let ttlGate = TTLNand(vcc: 5.0)

    let cmosPower = analyzePower(inverter: cmosInverter, frequency: frequency, cLoad: cLoad)
    let ttlPower = analyzeTTLPower(gate: ttlGate, frequency: frequency, cLoad: cLoad)

    let cmosTiming = analyzeTiming(inverter: cmosInverter, cLoad: cLoad)
    let ttlDelay = 10e-9  // TTL 7400: typical 10 ns propagation delay

    let cmosNM = computeNoiseMargins(inverter: cmosInverter)
    let ttlNM = computeTTLNoiseMargins()

    return [
        (property: "Static power (W)",   cmos: cmosPower.staticPower,  ttl: ttlPower.staticPower),
        (property: "Dynamic power (W)",  cmos: cmosPower.dynamicPower, ttl: ttlPower.dynamicPower),
        (property: "Propagation delay (s)", cmos: cmosTiming.tpd,     ttl: ttlDelay),
        (property: "High noise margin (V)", cmos: cmosNM.nmh,          ttl: ttlNM.nmh),
        (property: "Low noise margin (V)",  cmos: cmosNM.nml,          ttl: ttlNM.nml),
        (property: "Supply voltage (V)",    cmos: cmosInverter.circuit.vdd, ttl: ttlGate.vcc),
    ]
}

// MARK: - Moore's Law Illustration

/// Demonstrates the effect of technology scaling on CMOS performance.
///
/// As the transistor gate length shrinks (Moore's Law), power density and
/// speed both improve — until leakage current becomes the limiting factor
/// at sub-22 nm nodes.
///
/// - Parameter nodes: Technology node lengths in nm (e.g., [180, 130, 90, 65]).
/// - Returns: Array of per-node performance metrics.
public func demonstrateCMOSScaling(nodes: [Double]) -> [(
    node: Double,
    vdd: Double,
    frequency: Double,
    powerPerGate: Double,
    transistorsPerMM2: Double
)] {
    // Dennard scaling rules (approximate):
    //   Vdd scales ∝ 1/S  (S = scaling factor = ratio of node sizes)
    //   frequency ∝ S
    //   power ∝ 1/S²
    // Reference: 180 nm node
    let refNode = 180.0
    let refVdd = 1.8
    let refFreq = 1e9      // 1 GHz
    let refPower = 10e-6   // 10 µW/gate
    let refDensity = 10e6  // transistors/mm²

    return nodes.map { node in
        let s = refNode / node
        let vdd = refVdd / s
        let freq = refFreq * s
        let power = refPower / (s * s)
        let density = refDensity * s * s  // more transistors fit in same area

        return (
            node: node,
            vdd: vdd,
            frequency: freq,
            powerPerGate: power,
            transistorsPerMM2: density
        )
    }
}

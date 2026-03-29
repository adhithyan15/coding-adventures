// BJT.swift
// Part of coding-adventures — an educational computing stack.

import Foundation
//
// ============================================================================
// BJT — Bipolar Junction Transistor
// ============================================================================
//
// A BJT controls collector current with a small base current. Unlike the
// MOSFET's voltage-controlled gate, the BJT is current-controlled — a small
// base current (µA) enables a large collector current (mA).
//
// # BJT Structure (NPN)
//
//        Collector (C)
//             │
//    Base (B) ──► [transistor]
//             │
//        Emitter (E)
//
// # How It Works
//
// Forward-biasing the B-E junction (Vbe ≈ 0.7 V) injects minority carriers
// into the base region. These carriers diffuse across to the collector, where
// the reverse-biased B-C junction sweeps them out. The result:
//
//   Ic = β × Ib   (in the active region)
//
// β (beta, or hFE) is the current gain. Typical values: 100–400 for small
// signal transistors.
//
// # The Ebers-Moll Equation
//
// The collector current in the active region is modelled by the Shockley
// diode equation applied to the B-E junction:
//
//   Ic = Is × (exp(Vbe / Vt) − 1)
//
// where Vt = kT/q ≈ 26 mV at room temperature (the thermal voltage).
//
// We clamp the exponent to prevent overflow:
//   exp(min(Vbe/Vt, 40))
//
// # NPN vs PNP
//
// NPN: Electrons are majority carriers. Conducts when Vbe > 0 and Vce > 0.
//   Used in pull-down switching in TTL logic.
//
// PNP: Holes are majority carriers. Conducts when Vbe < 0 and Vce < 0
//   (i.e., emitter more positive than base/collector).
//   Used in pull-up switching and complementary circuits.
//
// ============================================================================

/// NPN bipolar junction transistor.
///
/// The classic switching transistor. Used in TTL logic gates, signal
/// amplifiers, and motor drivers. The 2N2222 is the archetypal NPN.
public struct NPN {

    public let params: BJTParams

    /// Thermal voltage at the operating temperature (≈ 26 mV at 300 K).
    private let vt: Double

    public init(params: BJTParams = .defaultNPN, temperature: Double = 300.0) {
        self.params = params
        // Vt = kT/q: Boltzmann constant × temperature / electron charge
        self.vt = 1.38e-23 * temperature / 1.6e-19
    }

    // MARK: - Operating Region

    /// Determines the NPN operating region.
    ///
    /// | Region | Vbe | Vce | State |
    /// |--------|-----|-----|-------|
    /// | Cutoff | < VbeOn | any | OFF — no current |
    /// | Active | ≥ VbeOn | > VceSat | ON amplifying |
    /// | Saturation | ≥ VbeOn | ≤ VceSat | ON fully conducting |
    public func region(vbe: Double, vce: Double) -> BJTRegion {
        guard vbe >= params.vbeOn else { return .cutoff }
        return vce > params.vceSat ? .active : .saturation
    }

    // MARK: - Currents

    /// Collector current (A) using the Ebers-Moll model.
    ///
    /// In the active region this is the exponential diode equation.
    /// In saturation, Vce is clamped to VceSat, limiting current.
    /// In cutoff, no current flows.
    public func collectorCurrent(vbe: Double, vce: Double) -> Double {
        switch region(vbe: vbe, vce: vce) {
        case .cutoff:
            return 0.0
        case .active:
            // Ic = Is × (exp(Vbe/Vt) − 1), clamped to prevent overflow
            let exponent = min(vbe / vt, 40.0)
            return params.is_ * (exp(exponent) - 1.0)
        case .saturation:
            // Fully ON: Vce = VceSat, limit current to a reasonable saturation value
            let exponent = min(params.vbeOn / vt, 40.0)
            return params.is_ * (exp(exponent) - 1.0)
        }
    }

    /// Base current (A). In the active region: Ib = Ic / β.
    public func baseCurrent(vbe: Double, vce: Double) -> Double {
        let ic = collectorCurrent(vbe: vbe, vce: vce)
        return ic > 0 ? ic / params.beta : 0.0
    }

    /// Returns true if the transistor is conducting (active or saturation).
    public func isConducting(vbe: Double) -> Bool {
        return vbe >= params.vbeOn
    }

    // MARK: - Small-Signal Parameters

    /// Transconductance gm (A/V) at the given operating point.
    ///
    /// gm = ∂Ic/∂Vbe = Ic / Vt
    ///
    /// This is used in amplifier analysis to compute voltage gain.
    public func transconductance(vbe: Double, vce: Double) -> Double {
        let ic = collectorCurrent(vbe: vbe, vce: vce)
        return ic > 0 ? ic / vt : 0.0
    }
}

// MARK: -

/// PNP bipolar junction transistor.
///
/// The complement of the NPN. Conducts when the emitter-base junction
/// is forward biased (Vbe < 0 in standard polarity, i.e., base more
/// negative than emitter).
///
/// # Sign Convention
///
/// For PNP, the conventional references are inverted:
///   - Veb = emitter − base voltage (positive for conduction)
///   - Vec = emitter − collector voltage (positive in active region)
///
/// This implementation accepts Vbe and Vce with the same sign convention
/// as NPN but interprets them with flipped polarity internally.
public struct PNP {

    public let params: BJTParams

    private let vt: Double

    public init(params: BJTParams = .defaultPNP, temperature: Double = 300.0) {
        self.params = params
        self.vt = 1.38e-23 * temperature / 1.6e-19
    }

    // MARK: - Operating Region

    /// Determines the PNP operating region.
    ///
    /// PNP conducts when Vbe ≤ −VbeOn (base more negative than emitter).
    public func region(vbe: Double, vce: Double) -> BJTRegion {
        // PNP conducts when the magnitude of Vbe exceeds VbeOn
        guard vbe <= -params.vbeOn else { return .cutoff }
        // Saturation when |Vce| ≤ VceSat
        return abs(vce) > params.vceSat ? .active : .saturation
    }

    // MARK: - Currents

    /// Collector current magnitude (A). Returns a positive value.
    public func collectorCurrent(vbe: Double, vce: Double) -> Double {
        switch region(vbe: vbe, vce: vce) {
        case .cutoff:
            return 0.0
        case .active:
            let vbeAbs = abs(vbe)
            let exponent = min(vbeAbs / vt, 40.0)
            return params.is_ * (exp(exponent) - 1.0)
        case .saturation:
            let exponent = min(params.vbeOn / vt, 40.0)
            return params.is_ * (exp(exponent) - 1.0)
        }
    }

    /// Base current magnitude (A).
    public func baseCurrent(vbe: Double, vce: Double) -> Double {
        let ic = collectorCurrent(vbe: vbe, vce: vce)
        return ic > 0 ? ic / params.beta : 0.0
    }

    /// Returns true if the PNP is conducting.
    public func isConducting(vbe: Double) -> Bool {
        return vbe <= -params.vbeOn
    }

    /// Transconductance gm (A/V).
    public func transconductance(vbe: Double, vce: Double) -> Double {
        let ic = collectorCurrent(vbe: vbe, vce: vce)
        return ic > 0 ? ic / vt : 0.0
    }
}

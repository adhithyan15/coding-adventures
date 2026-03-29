// MOSFET.swift
// Part of coding-adventures — an educational computing stack.
//
// ============================================================================
// MOSFET — Metal-Oxide-Semiconductor Field-Effect Transistor
// ============================================================================
//
// A MOSFET controls current between Drain and Source using voltage on the Gate.
// No current flows into the gate (the oxide layer acts as an insulator), which
// is why CMOS circuits consume almost zero static power.
//
// # MOSFET Structure
//
//         Gate (G)
//           |
//    ───────┴──────   ← oxide insulator (SiO₂)
//   Source (S)  Drain (D)
//    │                   │
//    └───── Channel ──────┘
//              ↑
//       Controlled by Vgs
//
// # NMOS vs PMOS
//
// NMOS (N-channel): Conducts when Vgs > Vth. Gate voltage "pulls" electrons
//   into the channel. Fast — electrons are the majority carrier.
//
// PMOS (P-channel): Conducts when Vgs < −|Vth|. Gate voltage "pulls" holes
//   into the channel. Slower than NMOS (~2× lower mobility).
//
// CMOS logic uses BOTH types. The complementary arrangement means exactly
// one of the two transistors is ON at any moment, giving near-zero static
// power dissipation.
//
// # The Shockley Model
//
// NMOS drain current (simplified square-law model):
//
//   Cutoff (Vgs ≤ Vth):
//     Id = 0
//
//   Linear (Vgs > Vth, Vds < Vgs − Vth):
//     Id = K·(W/L)·[(Vgs − Vth)·Vds − Vds²/2]
//
//   Saturation (Vgs > Vth, Vds ≥ Vgs − Vth):
//     Id = K·(W/L)/2·(Vgs − Vth)²
//
// ============================================================================

/// NMOS transistor — N-channel MOSFET.
///
/// Conducts when the gate-source voltage exceeds the threshold voltage (Vth).
/// Used as the pull-down network in CMOS gates (connects output to GND).
public struct NMOS {

    public let params: MOSFETParams

    public init(params: MOSFETParams = .defaultNMOS) {
        self.params = params
    }

    // MARK: - Operating Region

    /// Determines which of the three operating regions this NMOS is in.
    ///
    /// - Parameters:
    ///   - vgs: Gate-source voltage (V).
    ///   - vds: Drain-source voltage (V).
    /// - Returns: The operating region.
    public func region(vgs: Double, vds: Double) -> MOSFETRegion {
        guard vgs > params.vth else { return .cutoff }
        let vdsSat = vgs - params.vth  // voltage at onset of saturation
        return vds < vdsSat ? .linear : .saturation
    }

    // MARK: - Current

    /// Calculates the drain current (A) using the Shockley square-law model.
    ///
    /// - Parameters:
    ///   - vgs: Gate-source voltage (V).
    ///   - vds: Drain-source voltage (V).
    /// - Returns: Drain current in amperes. Always ≥ 0.
    public func drainCurrent(vgs: Double, vds: Double) -> Double {
        let kWL = params.k * (params.w / params.l)
        switch region(vgs: vgs, vds: vds) {
        case .cutoff:
            return 0.0
        case .linear:
            return kWL * ((vgs - params.vth) * vds - 0.5 * vds * vds)
        case .saturation:
            let vov = vgs - params.vth   // overdrive voltage
            return 0.5 * kWL * vov * vov
        }
    }

    /// Returns true if this NMOS is in the conducting state (linear or saturation).
    public func isConducting(vgs: Double) -> Bool {
        return vgs > params.vth
    }

    // MARK: - Derived Quantities

    /// Output voltage at the drain, assuming a resistive load to Vdd.
    ///
    /// Models a simple NMOS inverter: when the NMOS is ON, it pulls the
    /// output toward GND; when OFF, the output floats to Vdd.
    ///
    /// - Parameters:
    ///   - vgs: Gate-source voltage (V).
    ///   - vdd: Supply voltage (V).
    /// - Returns: Drain voltage (V).
    public func outputVoltage(vgs: Double, vdd: Double) -> Double {
        return isConducting(vgs: vgs) ? 0.05 * vdd : vdd
    }

    /// Small-signal transconductance gm (A/V) at the given operating point.
    ///
    /// gm = ∂Id/∂Vgs = K·(W/L)·(Vgs − Vth) in saturation.
    ///
    /// Transconductance quantifies how strongly the gate voltage controls
    /// the drain current — a key figure of merit for amplifiers.
    public func transconductance(vgs: Double, vds: Double) -> Double {
        guard region(vgs: vgs, vds: vds) == .saturation else { return 0.0 }
        return params.k * (params.w / params.l) * (vgs - params.vth)
    }
}

// MARK: -

/// PMOS transistor — P-channel MOSFET.
///
/// Conducts when the gate-source voltage falls below −|Vth|.
/// In CMOS, PMOS provides the pull-up network (connects output to Vdd).
///
/// # Sign Convention
///
/// PMOS voltages are referenced with source at Vdd:
///   - Vgs is negative when the gate is pulled LOW (conducting).
///   - Vds is negative when drain < source (normal operating direction).
///
/// This implementation uses absolute magnitudes internally and applies
/// signs consistently at the boundary — the same convention used in
/// most introductory textbooks.
public struct PMOS {

    public let params: MOSFETParams

    public init(params: MOSFETParams = .defaultPMOS) {
        self.params = params
    }

    // MARK: - Operating Region

    /// Determines the operating region of this PMOS.
    ///
    /// - Parameters:
    ///   - vgs: Gate-source voltage (V). Negative for conduction (source at Vdd).
    ///   - vds: Drain-source voltage (V). Negative in normal operation.
    public func region(vgs: Double, vds: Double) -> MOSFETRegion {
        // PMOS conducts when Vgs < −Vth (i.e., |Vgs| > Vth)
        guard vgs < -params.vth else { return .cutoff }
        let vdsSat = vgs + params.vth   // negative saturation threshold
        return vds > vdsSat ? .linear : .saturation
    }

    // MARK: - Current

    /// Drain current magnitude (A). Returns a positive value regardless of sign.
    public func drainCurrent(vgs: Double, vds: Double) -> Double {
        let kWL = params.k * (params.w / params.l)
        switch region(vgs: vgs, vds: vds) {
        case .cutoff:
            return 0.0
        case .linear:
            // Use absolute values; current flows from source to drain (Vdd → output)
            let vov = abs(vgs) - params.vth
            let vdsAbs = abs(vds)
            return kWL * (vov * vdsAbs - 0.5 * vdsAbs * vdsAbs)
        case .saturation:
            let vov = abs(vgs) - params.vth
            return 0.5 * kWL * vov * vov
        }
    }

    /// Returns true if this PMOS is conducting.
    public func isConducting(vgs: Double) -> Bool {
        return vgs < -params.vth
    }

    /// Output voltage at the drain, assuming a resistive load to GND.
    public func outputVoltage(vgs: Double, vdd: Double) -> Double {
        return isConducting(vgs: vgs) ? 0.95 * vdd : 0.05 * vdd
    }

    /// Small-signal transconductance gm (A/V) at the given operating point.
    public func transconductance(vgs: Double, vds: Double) -> Double {
        guard region(vgs: vgs, vds: vds) == .saturation else { return 0.0 }
        return params.k * (params.w / params.l) * (abs(vgs) - params.vth)
    }
}

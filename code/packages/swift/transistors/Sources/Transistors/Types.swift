// Types.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// Transistors — Types, Parameters, and Output Structures
// ============================================================================
//
// This file defines the data types shared across the entire transistors
// package: device parameters, operating regions, and output records.
//
// # A Brief Transistor Primer
//
// A transistor is a three-terminal semiconductor device that acts as an
// electrically-controlled switch or amplifier. Two families matter most
// in digital electronics:
//
//   MOSFET (Metal-Oxide-Semiconductor Field-Effect Transistor)
//     - Controls current with voltage on a "gate" terminal.
//     - Draws almost zero current from the controlling circuit.
//     - Used in CMOS logic — the technology inside every modern CPU.
//
//   BJT (Bipolar Junction Transistor)
//     - Controls current with a small "base" current.
//     - Faster in some analog circuits; more complex power characteristics.
//     - Used in TTL (Transistor-Transistor Logic) — the technology that
//       powered computers from the 1970s through the early 1990s.
//
// ============================================================================

import Foundation

// MARK: - Operating Regions

/// The three operating regions of a MOSFET.
///
/// Think of a MOSFET as a water valve:
///
///   - Cutoff:     Valve is closed — no water (current) flows.
///   - Linear:     Valve is partially open — flow is proportional to opening.
///   - Saturation: Valve is wide open — flow is limited by the pipe (channel),
///                 not the valve position.
///
/// In digital logic we care most about cutoff (= logic 0) and saturation
/// (= logic 1). Linear is the transition region that determines switching speed.
public enum MOSFETRegion: String, Equatable {
    case cutoff
    case linear
    case saturation
}

/// The three operating regions of a BJT.
///
/// The analogy maps cleanly onto MOSFETs:
///
///   - Cutoff:     No base current → transistor is OFF.
///   - Active:     Transistor amplifies — collector current = β × base current.
///   - Saturation: Transistor is fully ON — used as a switch in TTL gates.
public enum BJTRegion: String, Equatable {
    case cutoff
    case active
    case saturation
}

// MARK: - Device Parameters

/// Parameters for a MOSFET device.
///
/// These values correspond to a typical 180 nm CMOS process node —
/// the technology generation that dominated in the early 2000s.
///
/// | Parameter | Symbol | Typical 180 nm value | What it means |
/// |-----------|--------|----------------------|---------------|
/// | Threshold voltage | Vth | 0.4 V | Gate voltage where channel starts forming |
/// | Transconductance | K | 200 µA/V² | How strongly the gate controls the channel |
/// | Width | W | 1.0 µm | Physical gate width — wider = more current |
/// | Length | L | 0.18 µm | Physical gate length — shorter = faster |
/// | Gate capacitance | CGate | 10 fF | Slows switching; limits max frequency |
/// | Drain capacitance | CDrain | 5 fF | Load capacitance at drain output |
public struct MOSFETParams {
    /// Threshold voltage (V). The gate-source voltage at which the channel
    /// begins to form. Below Vth: no current flows (cutoff).
    public let vth: Double

    /// Process transconductance parameter (A/V²). Encodes carrier mobility
    /// and oxide thickness: K = μ × Cox. Larger K = more current for same Vgs.
    public let k: Double

    /// Gate width (µm). Wider gates conduct more current but occupy more die area.
    public let w: Double

    /// Gate length (µm). The key scaling parameter — shrinking L is Moore's Law.
    public let l: Double

    /// Gate capacitance (F). Energy cost of switching the gate state.
    public let cGate: Double

    /// Drain capacitance (F). Limits the speed of voltage transitions at the output.
    public let cDrain: Double

    public init(
        vth: Double = 0.4,
        k: Double = 200e-6,
        w: Double = 1.0,
        l: Double = 0.18,
        cGate: Double = 10e-15,
        cDrain: Double = 5e-15
    ) {
        self.vth = vth
        self.k = k
        self.w = w
        self.l = l
        self.cGate = cGate
        self.cDrain = cDrain
    }

    /// Default 180 nm CMOS NMOS parameters.
    public static let defaultNMOS = MOSFETParams()

    /// Default 180 nm CMOS PMOS parameters.
    /// PMOS typically has ~2× lower mobility than NMOS, so K is halved.
    /// Threshold voltage is negative (conducts when Vgs < −Vth).
    public static let defaultPMOS = MOSFETParams(
        vth: 0.4,
        k: 100e-6,
        w: 2.0,   // wider to compensate for lower mobility
        l: 0.18,
        cGate: 15e-15,
        cDrain: 7e-15
    )
}

/// Parameters for a BJT device.
///
/// Modelled after the 2N2222 NPN transistor — a classic general-purpose
/// device used in TTL circuits, signal amplifiers, and switching applications.
public struct BJTParams {
    /// Current gain (β, dimensionless). Collector current = β × base current.
    /// Typical small-signal NPN values range from 100 to 300.
    public let beta: Double

    /// Base-emitter on-voltage (V). The diode junction voltage at which the
    /// transistor begins to conduct. ≈ 0.7 V for silicon.
    public let vbeOn: Double

    /// Collector-emitter saturation voltage (V). The residual voltage across
    /// a fully-saturated transistor. ≈ 0.2 V for the 2N2222.
    public let vceSat: Double

    /// Reverse saturation current (A). Appears in the Shockley diode equation.
    /// Typically in the picoamp range for silicon.
    public let is_: Double

    /// Base capacitance (F). Limits switching speed; smaller = faster.
    public let cBase: Double

    public init(
        beta: Double = 200.0,
        vbeOn: Double = 0.7,
        vceSat: Double = 0.2,
        is_: Double = 1e-14,
        cBase: Double = 8e-12
    ) {
        self.beta = beta
        self.vbeOn = vbeOn
        self.vceSat = vceSat
        self.is_ = is_
        self.cBase = cBase
    }

    /// Default NPN (2N2222-style) parameters.
    public static let defaultNPN = BJTParams()

    /// Default PNP parameters. Complementary to the NPN; slightly lower β.
    public static let defaultPNP = BJTParams(
        beta: 150.0,
        vbeOn: 0.7,
        vceSat: 0.25,
        is_: 1e-14,
        cBase: 10e-12
    )
}

/// Parameters shared by a gate or circuit.
public struct CircuitParams {
    /// Supply voltage (V). 1.8 V is typical for 180 nm CMOS; 5 V for TTL.
    public let vdd: Double

    /// Operating temperature (K). Room temperature = 300 K = 27 °C.
    /// Affects BJT exponential current equations via thermal voltage Vt = kT/q.
    public let temperature: Double

    public init(vdd: Double = 1.8, temperature: Double = 300.0) {
        self.vdd = vdd
        self.temperature = temperature
    }

    /// Thermal voltage: Vt = kT/q ≈ 26 mV at room temperature.
    ///
    /// This appears in the Shockley equation for BJT current:
    ///   Ic = Is × (exp(Vbe / Vt) − 1)
    public var thermalVoltage: Double {
        // Boltzmann constant k = 1.38e-23 J/K; electron charge q = 1.6e-19 C.
        return 1.38e-23 * temperature / 1.6e-19
    }

    /// Default 1.8 V CMOS circuit at room temperature.
    public static let cmos18 = CircuitParams(vdd: 1.8)

    /// Standard 5 V TTL supply at room temperature.
    public static let ttl5v = CircuitParams(vdd: 5.0)
}

// MARK: - Output Records

/// The complete output of a gate evaluation.
///
/// Beyond a simple 0/1 result, this record captures the physical quantities
/// that matter for circuit design: voltage levels, power consumption, and timing.
public struct GateOutput {
    /// Digital logic value: 0 (LOW) or 1 (HIGH).
    public let logicValue: Int

    /// Output voltage (V). For CMOS: HIGH ≈ Vdd, LOW ≈ 0 V.
    public let voltage: Double

    /// Current drawn from the supply (A).
    public let currentDraw: Double

    /// Instantaneous power dissipation (W) = Vdd × currentDraw.
    public let powerDissipation: Double

    /// Propagation delay (s) — time for output to respond to input change.
    public let propagationDelay: Double

    /// Number of transistors used to implement this gate.
    public let transistorCount: Int

    public init(
        logicValue: Int,
        voltage: Double,
        currentDraw: Double,
        powerDissipation: Double,
        propagationDelay: Double,
        transistorCount: Int
    ) {
        self.logicValue = logicValue
        self.voltage = voltage
        self.currentDraw = currentDraw
        self.powerDissipation = powerDissipation
        self.propagationDelay = propagationDelay
        self.transistorCount = transistorCount
    }
}

/// Analysis result for a single-transistor amplifier stage.
public struct AmplifierAnalysis {
    /// Small-signal voltage gain (V/V). Negative for inverting amplifiers.
    public let voltageGain: Double

    /// Transconductance gm (A/V). How much drain current changes per volt at gate.
    public let transconductance: Double

    /// Input impedance (Ω). How much the amplifier loads the driving source.
    public let inputImpedance: Double

    /// Output impedance (Ω). How much the amplifier can drive a load.
    public let outputImpedance: Double

    /// −3 dB bandwidth (Hz). Frequency where gain drops to 1/√2 of mid-band.
    public let bandwidth: Double

    /// DC operating point voltage at the drain/collector (V).
    public let operatingPoint: Double

    public init(
        voltageGain: Double,
        transconductance: Double,
        inputImpedance: Double,
        outputImpedance: Double,
        bandwidth: Double,
        operatingPoint: Double
    ) {
        self.voltageGain = voltageGain
        self.transconductance = transconductance
        self.inputImpedance = inputImpedance
        self.outputImpedance = outputImpedance
        self.bandwidth = bandwidth
        self.operatingPoint = operatingPoint
    }
}

/// Noise margin measurements for a logic gate.
///
/// Noise margins define how much electrical noise a gate can tolerate before
/// misinterpreting a logic level. Larger margins = more robust circuit.
///
///   NML (low noise margin)  = VIL − VOL
///   NMH (high noise margin) = VOH − VIH
///
///   VOL: output voltage when outputting LOW
///   VOH: output voltage when outputting HIGH
///   VIL: maximum input voltage still interpreted as LOW
///   VIH: minimum input voltage still interpreted as HIGH
public struct NoiseMargins {
    /// Output LOW voltage (V). Should be close to 0 V.
    public let vol: Double
    /// Output HIGH voltage (V). Should be close to Vdd.
    public let voh: Double
    /// Maximum input voltage still read as LOW (V).
    public let vil: Double
    /// Minimum input voltage still read as HIGH (V).
    public let vih: Double
    /// Low noise margin (V) = VIL − VOL. Must be > 0.
    public let nml: Double
    /// High noise margin (V) = VOH − VIH. Must be > 0.
    public let nmh: Double

    public init(vol: Double, voh: Double, vil: Double, vih: Double) {
        self.vol = vol
        self.voh = voh
        self.vil = vil
        self.vih = vih
        self.nml = vil - vol
        self.nmh = voh - vih
    }
}

/// Power consumption breakdown for a gate.
public struct PowerAnalysis {
    /// Static (leakage) power (W). Consumed even when inputs are not switching.
    public let staticPower: Double
    /// Dynamic (switching) power (W). Consumed charging/discharging gate capacitances.
    public let dynamicPower: Double
    /// Total power (W) = static + dynamic.
    public let totalPower: Double
    /// Energy per switching event (J).
    public let energyPerSwitch: Double

    public init(staticPower: Double, dynamicPower: Double, energyPerSwitch: Double) {
        self.staticPower = staticPower
        self.dynamicPower = dynamicPower
        self.totalPower = staticPower + dynamicPower
        self.energyPerSwitch = energyPerSwitch
    }
}

/// Timing characteristics for a logic gate.
public struct TimingAnalysis {
    /// Propagation delay HIGH-to-LOW (s). Time for output to fall after input rises.
    public let tphl: Double
    /// Propagation delay LOW-to-HIGH (s). Time for output to rise after input falls.
    public let tplh: Double
    /// Average propagation delay (s) = (tphl + tplh) / 2.
    public let tpd: Double
    /// Output rise time (s): 10% → 90% of Vdd.
    public let riseTime: Double
    /// Output fall time (s): 90% → 10% of Vdd.
    public let fallTime: Double
    /// Maximum operating frequency (Hz) = 1 / (2 × tpd).
    public let maxFrequency: Double

    public init(tphl: Double, tplh: Double, riseTime: Double, fallTime: Double) {
        self.tphl = tphl
        self.tplh = tplh
        self.tpd = (tphl + tplh) / 2.0
        self.riseTime = riseTime
        self.fallTime = fallTime
        self.maxFrequency = 1.0 / (2.0 * self.tpd)
    }
}

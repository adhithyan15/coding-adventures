//! Shared types for the transistors crate.
//!
//! # Enums and Parameter Structs
//!
//! These types define the vocabulary of transistor simulation. Every transistor
//! has an operating region (cutoff, linear, saturation), and every circuit has
//! electrical parameters (voltage, capacitance, etc.).
//!
//! We derive `Clone` and `Copy` for parameter structs because transistor
//! characteristics are fixed once manufactured — you can't change a transistor's
//! threshold voltage after fabrication.

use std::collections::HashMap;

// ===========================================================================
// OPERATING REGION ENUMS
// ===========================================================================
// A transistor is an analog device that operates differently depending on
// the voltages applied to its terminals. The three "regions" describe these
// different operating modes.

/// Operating region of a MOSFET transistor.
///
/// Think of it like a water faucet with three positions:
///
/// - **Cutoff**: Faucet is fully closed. No water flows.
///   (Vgs < Vth — gate voltage too low to turn on)
///
/// - **Linear**: Faucet is open, and water flow increases as you
///   turn the handle more. Flow is proportional to both handle position
///   AND water pressure.
///   (Vgs > Vth, Vds < Vgs - Vth — acts like a resistor)
///
/// - **Saturation**: Faucet is wide open, but the pipe is the bottleneck.
///   Adding more pressure doesn't increase flow much.
///   (Vgs > Vth, Vds >= Vgs - Vth — current is roughly constant)
///
/// For digital circuits, we only use Cutoff (OFF) and deep Linear (ON).
/// For analog amplifiers, we operate in Saturation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MOSFETRegion {
    Cutoff,
    Linear,
    Saturation,
}

/// Operating region of a BJT transistor.
///
/// Similar to MOSFET regions but with different names and physics:
///
/// - **Cutoff**: No base current -> no collector current. Switch OFF.
///   (Vbe < ~0.7V)
///
/// - **Active**: Small base current, large collector current.
///   Ic = beta * Ib. This is the AMPLIFIER region.
///   (Vbe >= ~0.7V, Vce > ~0.2V)
///
/// - **Saturation**: Both junctions forward-biased. Collector current
///   is maximum — transistor is fully ON as a switch.
///   (Vbe >= ~0.7V, Vce <= ~0.2V)
///
/// Confusing naming alert: MOSFET "saturation" = constant current (amplifier).
/// BJT "saturation" = fully ON (switch). These are DIFFERENT behaviors despite
/// sharing a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BJTRegion {
    Cutoff,
    Active,
    Saturation,
}

/// Transistor polarity/type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransistorType {
    NMOS,
    PMOS,
    NPN,
    PNP,
}

// ===========================================================================
// ELECTRICAL PARAMETERS
// ===========================================================================
// These structs hold the physical characteristics of transistors.
// Default values represent common, well-documented transistor types
// so that users can start experimenting immediately without needing
// to look up datasheets.

/// Electrical parameters for a MOSFET transistor.
///
/// Default values represent a typical 180nm CMOS process — the last
/// "large" process node that is still widely used in education and
/// analog/mixed-signal chips.
///
/// Key parameters:
///
/// - `vth`: Threshold voltage — the minimum Vgs to turn the transistor ON.
///   Lower Vth = faster switching but more leakage current.
///   Modern CPUs use Vth around 0.2-0.4V.
///
/// - `k`: Transconductance parameter — controls how much current flows
///   for a given Vgs. Higher k = more current = faster but more power.
///   k = mu * Cox * (W/L) where mu is carrier mobility and Cox is
///   oxide capacitance per unit area.
///
/// - `w`, `l`: Channel width and length. The W/L ratio is the main knob
///   chip designers use to tune transistor strength.
///
/// - `c_gate`: Gate capacitance — determines switching speed.
///
/// - `c_drain`: Drain junction capacitance — contributes to output load.
#[derive(Debug, Clone, Copy)]
pub struct MOSFETParams {
    pub vth: f64,
    pub k: f64,
    pub w: f64,
    pub l: f64,
    pub c_gate: f64,
    pub c_drain: f64,
}

impl Default for MOSFETParams {
    fn default() -> Self {
        Self {
            vth: 0.4,
            k: 0.001,
            w: 1e-6,
            l: 180e-9,
            c_gate: 1e-15,
            c_drain: 0.5e-15,
        }
    }
}

/// Electrical parameters for a BJT transistor.
///
/// Default values represent a typical small-signal NPN transistor
/// like the 2N2222 — one of the most common transistors ever made.
///
/// Key parameters:
///
/// - `beta`: Current gain (hfe) — the ratio Ic/Ib. A beta of 100
///   means 1mA of base current controls 100mA of collector current.
///
/// - `vbe_on`: Base-emitter voltage when conducting. For silicon BJTs,
///   this is always around 0.6-0.7V.
///
/// - `vce_sat`: Collector-emitter voltage when fully saturated.
///   Ideally 0V, practically about 0.1-0.3V.
///
/// - `is_`: Reverse saturation current — the tiny leakage current
///   that flows even when the transistor is OFF. Named `is_` with
///   trailing underscore because `is` is a keyword in many languages.
///
/// - `c_base`: Base capacitance — limits switching speed.
#[derive(Debug, Clone, Copy)]
pub struct BJTParams {
    pub beta: f64,
    pub vbe_on: f64,
    pub vce_sat: f64,
    pub is_: f64,
    pub c_base: f64,
}

impl Default for BJTParams {
    fn default() -> Self {
        Self {
            beta: 100.0,
            vbe_on: 0.7,
            vce_sat: 0.2,
            is_: 1e-14,
            c_base: 5e-12,
        }
    }
}

/// Parameters for a complete logic gate circuit.
///
/// - `vdd`: Supply voltage. Modern CMOS uses 0.7-1.2V, older CMOS
///   used 3.3V or 5V, TTL always uses 5V.
///
/// - `temperature`: Junction temperature in Kelvin. Room temperature is
///   ~300K (27C).
#[derive(Debug, Clone, Copy)]
pub struct CircuitParams {
    pub vdd: f64,
    pub temperature: f64,
}

impl Default for CircuitParams {
    fn default() -> Self {
        Self {
            vdd: 3.3,
            temperature: 300.0,
        }
    }
}

// ===========================================================================
// RESULT TYPES
// ===========================================================================
// These structs hold the results of transistor and circuit analysis.

/// Result of evaluating a logic gate with voltage-level detail.
///
/// Unlike the logic_gates crate which only returns 0 or 1, this gives
/// you the full electrical picture: what voltage does the output actually
/// sit at? How much power is being consumed? How long did the signal
/// take to propagate?
#[derive(Debug, Clone)]
pub struct GateOutput {
    pub logic_value: u8,
    pub voltage: f64,
    pub current_draw: f64,
    pub power_dissipation: f64,
    pub propagation_delay: f64,
    pub transistor_count: usize,
}

/// Results of analyzing a transistor as an amplifier.
///
/// When a transistor operates in its linear/active region (not as a
/// digital switch), it can amplify signals. These parameters describe
/// the quality of that amplification.
///
/// - `voltage_gain`: How much the output voltage changes per unit change
///   in input voltage. Negative for inverting amplifiers.
///
/// - `transconductance`: gm — the ratio of output current change to input
///   voltage change. Units: Siemens (A/V).
///
/// - `input_impedance`: How much the amplifier "loads" the signal source.
///   MOSFET: very high (>1 GOhm). BJT: moderate (~1-10 kOhm).
///
/// - `output_impedance`: How "stiff" the output is.
///
/// - `bandwidth`: Frequency at which gain drops to 70.7% (-3dB).
#[derive(Debug, Clone)]
pub struct AmplifierAnalysis {
    pub voltage_gain: f64,
    pub transconductance: f64,
    pub input_impedance: f64,
    pub output_impedance: f64,
    pub bandwidth: f64,
    pub operating_point: HashMap<String, f64>,
}

/// Noise margin analysis for a logic family.
///
/// Noise margins tell you how much electrical noise a digital signal
/// can tolerate before being misinterpreted.
///
/// - `vol`: Output LOW voltage
/// - `voh`: Output HIGH voltage
/// - `vil`: Input LOW threshold
/// - `vih`: Input HIGH threshold
/// - `nml`: Noise Margin LOW = vil - vol
/// - `nmh`: Noise Margin HIGH = voh - vih
#[derive(Debug, Clone, Copy)]
pub struct NoiseMargins {
    pub vol: f64,
    pub voh: f64,
    pub vil: f64,
    pub vih: f64,
    pub nml: f64,
    pub nmh: f64,
}

/// Power consumption breakdown for a gate or circuit.
///
/// - `static_power`: Power consumed even when the gate is not switching.
///   CMOS: ~nW (transistor leakage). TTL: ~mW (resistor bias current).
///
/// - `dynamic_power`: Power consumed during switching transitions.
///   P_dyn = C_load * Vdd^2 * f * alpha.
///
/// - `total_power`: static + dynamic.
///
/// - `energy_per_switch`: Energy for one complete 0->1->0 transition.
#[derive(Debug, Clone, Copy)]
pub struct PowerAnalysis {
    pub static_power: f64,
    pub dynamic_power: f64,
    pub total_power: f64,
    pub energy_per_switch: f64,
}

/// Timing characteristics for a gate.
///
/// - `tphl`: Propagation delay from HIGH to LOW output.
/// - `tplh`: Propagation delay from LOW to HIGH output.
/// - `tpd`: Average propagation delay = (tphl + tplh) / 2.
/// - `rise_time`: Time for output to go from 10% to 90% of Vdd.
/// - `fall_time`: Time for output to go from 90% to 10% of Vdd.
/// - `max_frequency`: Maximum clock frequency = 1 / (2 * tpd).
#[derive(Debug, Clone, Copy)]
pub struct TimingAnalysis {
    pub tphl: f64,
    pub tplh: f64,
    pub tpd: f64,
    pub rise_time: f64,
    pub fall_time: f64,
    pub max_frequency: f64,
}

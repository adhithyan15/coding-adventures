//! Analog Amplifier Analysis — transistors as signal amplifiers.
//!
//! # Beyond Digital: Transistors as Amplifiers
//!
//! A transistor used as a digital switch operates in only two states: ON and OFF.
//! But transistors are fundamentally ANALOG devices. When biased in the right
//! operating region (saturation for MOSFET, active for BJT), they can amplify
//! small signals into larger ones.
//!
//! # Common-Source Amplifier (MOSFET)
//!
//! ```text
//!     Vdd
//!      |
//!     [Rd]  <- voltage drop = Ids x Rd
//!      |
//! -----| Drain (output)
//! Gate -||
//! -----| Source
//!      |
//!     GND
//! ```
//!
//! Voltage gain: Av = -gm x Rd (inverting amplifier)
//!
//! # Common-Emitter Amplifier (BJT)
//!
//! Voltage gain: Av = -gm x Rc = -(Ic/Vt) x Rc

use std::collections::HashMap;

use crate::bjt::NPN;
use crate::mosfet::NMOS;
use crate::types::AmplifierAnalysis;

/// Analyze an NMOS common-source amplifier configuration.
///
/// The common-source amplifier is the most basic MOSFET amplifier topology.
/// The input signal is applied to the gate, and the output is taken from
/// the drain. A drain resistor (Rd) converts the drain current variation
/// into a voltage swing.
///
/// For the amplifier to work, the MOSFET must be biased in SATURATION:
/// Vgs > Vth AND Vds >= Vgs - Vth.
pub fn analyze_common_source_amp(
    transistor: &NMOS,
    vgs: f64,
    vdd: f64,
    r_drain: f64,
    c_load: Option<f64>,
) -> AmplifierAnalysis {
    let c_load = c_load.unwrap_or(1e-12);

    // Calculate DC operating point
    let ids = transistor.drain_current(vgs, vdd); // Approximate: Vds ~ Vdd initially
    let vds = vdd - ids * r_drain; // Actual drain voltage

    // Recalculate with correct Vds
    let ids = transistor.drain_current(vgs, vds.max(0.0));
    let vds = vdd - ids * r_drain;

    // Transconductance
    let gm = transistor.transconductance(vgs, vds.max(0.0));

    // Voltage gain: Av = -gm x Rd (inverting)
    let voltage_gain = -gm * r_drain;

    // Input impedance: essentially infinite for MOSFET (gate is insulated)
    let input_impedance = 1e12; // 1 TOhm

    // Output impedance: approximately Rd
    let output_impedance = r_drain;

    // Bandwidth: f_3dB = 1 / (2*pi * Rd * C_load)
    let bandwidth = 1.0 / (2.0 * std::f64::consts::PI * r_drain * c_load);

    let mut operating_point = HashMap::new();
    operating_point.insert("vgs".to_string(), vgs);
    operating_point.insert("vds".to_string(), vds);
    operating_point.insert("ids".to_string(), ids);
    operating_point.insert("gm".to_string(), gm);

    AmplifierAnalysis {
        voltage_gain,
        transconductance: gm,
        input_impedance,
        output_impedance,
        bandwidth,
        operating_point,
    }
}

/// Analyze an NPN common-emitter amplifier configuration.
///
/// The BJT equivalent of the common-source amplifier. Input is applied
/// to the base, output taken from the collector.
///
/// BJT amplifiers typically have higher voltage gain than MOSFET amplifiers
/// at the same current, because BJT transconductance (gm = Ic/Vt) is
/// higher than MOSFET transconductance for the same bias current.
///
/// However, BJT amplifiers have lower input impedance because base current
/// flows continuously.
pub fn analyze_common_emitter_amp(
    transistor: &NPN,
    vbe: f64,
    vcc: f64,
    r_collector: f64,
    c_load: Option<f64>,
) -> AmplifierAnalysis {
    let c_load = c_load.unwrap_or(1e-12);

    // Calculate DC operating point
    let vce = vcc; // Initial approximation
    let ic = transistor.collector_current(vbe, vce);
    let vce = (vcc - ic * r_collector).max(0.0);

    // Recalculate with correct Vce
    let ic = transistor.collector_current(vbe, vce);

    // Transconductance
    let gm = transistor.transconductance(vbe, vce);

    // Voltage gain: Av = -gm x Rc
    let voltage_gain = -gm * r_collector;

    // Input impedance: r_pi = beta / gm = beta * Vt / Ic
    let beta = transistor.params.beta;
    let vt = 0.026;
    let r_pi = if ic > 0.0 {
        beta * vt / ic
    } else {
        1e12 // Very high when no current flows
    };

    let input_impedance = r_pi;
    let output_impedance = r_collector;

    // Bandwidth
    let bandwidth = 1.0 / (2.0 * std::f64::consts::PI * r_collector * c_load);

    let mut operating_point = HashMap::new();
    operating_point.insert("vbe".to_string(), vbe);
    operating_point.insert("vce".to_string(), vce);
    operating_point.insert("ic".to_string(), ic);
    operating_point.insert("ib".to_string(), transistor.base_current(vbe, vce));
    operating_point.insert("gm".to_string(), gm);

    AmplifierAnalysis {
        voltage_gain,
        transconductance: gm,
        input_impedance,
        output_impedance,
        bandwidth,
        operating_point,
    }
}

//! Electrical Analysis — noise margins, power, timing, and technology comparison.
//!
//! # Why Electrical Analysis Matters
//!
//! Digital logic designers don't just care about truth tables — they care about:
//!
//! 1. **Noise Margins**: Can the circuit tolerate voltage fluctuations?
//!    A chip has billions of wires running millimeters apart, each creating
//!    electromagnetic interference on its neighbors.
//!
//! 2. **Power**: How much energy does the chip consume? A modern CPU runs at
//!    ~100 watts. Power = the #1 constraint in modern chip design.
//!
//! 3. **Timing**: How fast can the circuit switch? The propagation delay
//!    determines the maximum clock frequency.
//!
//! 4. **Scaling**: How do these properties change as we shrink transistors?
//!    Moore's Law predicts transistor count doubles every ~2 years, but the
//!    "power wall" and "leakage wall" constrain how far we can push.

use std::collections::HashMap;

use crate::cmos_gates::{CMOSInverter, CMOSNand, CMOSNor};
use crate::ttl_gates::TTLNand;
use crate::types::{CircuitParams, MOSFETParams, NoiseMargins, PowerAnalysis, TimingAnalysis};

/// Which type of gate we are analyzing.
///
/// We use an enum to dispatch analysis functions to the correct gate type.
/// This avoids dynamic dispatch overhead and makes the type system do the work.
pub enum GateType<'a> {
    CMOSInverter(&'a CMOSInverter),
    CMOSNand(&'a CMOSNand),
    CMOSNor(&'a CMOSNor),
    TTLNand(&'a TTLNand),
}

/// Analyze noise margins for a gate.
///
/// Noise margins tell you how much electrical noise a digital signal
/// can tolerate before being misinterpreted by the next gate in the chain.
///
/// For CMOS:
///   VOL ~ 0V, VOH ~ Vdd -> large noise margins
///   NML ~ NMH ~ 0.4 * Vdd (symmetric)
///
/// For TTL:
///   VOL ~ 0.2V, VOH ~ 3.5V -> smaller margins
///   VIL = 0.8V, VIH = 2.0V (defined by spec)
pub fn compute_noise_margins(gate: GateType) -> NoiseMargins {
    let (vol, voh, vil, vih) = match gate {
        GateType::CMOSInverter(inv) => {
            let vdd = inv.circuit.vdd;
            (0.0, vdd, 0.4 * vdd, 0.6 * vdd)
        }
        GateType::TTLNand(nand) => {
            let vol = 0.2;
            let voh = nand.vcc - 0.7;
            let vil = 0.8;
            let vih = 2.0;
            (vol, voh, vil, vih)
        }
        // CMOS NAND/NOR use same margins as inverter
        GateType::CMOSNand(nand) => {
            let vdd = nand.circuit.vdd;
            (0.0, vdd, 0.4 * vdd, 0.6 * vdd)
        }
        GateType::CMOSNor(nor) => {
            let vdd = nor.circuit.vdd;
            (0.0, vdd, 0.4 * vdd, 0.6 * vdd)
        }
    };

    let nml = vil - vol;
    let nmh = voh - vih;

    NoiseMargins {
        vol,
        voh,
        vil,
        vih,
        nml,
        nmh,
    }
}

/// Compute power consumption for a gate at a given operating frequency.
///
/// # Power in CMOS
///
/// P_total = P_static + P_dynamic
///
/// P_static ~ negligible (nanowatts)
/// P_dynamic = C_load * Vdd^2 * f * alpha
///
/// # Power in TTL
///
/// P_static = V_cc * I_cc ~ milliwatts (DOMINATES!)
pub fn analyze_power(
    gate: GateType,
    frequency: Option<f64>,
    c_load: Option<f64>,
    activity_factor: Option<f64>,
) -> PowerAnalysis {
    let frequency = frequency.unwrap_or(1e9);
    let c_load = c_load.unwrap_or(1e-12);
    let activity_factor = activity_factor.unwrap_or(0.5);

    let (static_power, vdd) = match &gate {
        GateType::TTLNand(nand) => (nand.static_power(), nand.vcc),
        GateType::CMOSInverter(inv) => (0.0, inv.circuit.vdd),
        GateType::CMOSNand(nand) => (0.0, nand.circuit.vdd),
        GateType::CMOSNor(nor) => (0.0, nor.circuit.vdd),
    };

    // Dynamic power: P = C * V^2 * f * alpha
    let dynamic = c_load * vdd * vdd * frequency * activity_factor;
    let total = static_power + dynamic;

    // Energy per switching event: E = C * V^2
    let energy_per_switch = c_load * vdd * vdd;

    PowerAnalysis {
        static_power,
        dynamic_power: dynamic,
        total_power: total,
        energy_per_switch,
    }
}

/// Compute timing characteristics for a gate.
///
/// For CMOS: t_pd ~ (C_load * Vdd) / (2 * I_sat)
/// For TTL: t_pd ~ 5-15 ns (fixed)
pub fn analyze_timing(gate: GateType, c_load: Option<f64>) -> TimingAnalysis {
    let c_load = c_load.unwrap_or(1e-12);

    let (tphl, tplh, rise_time, fall_time) = match &gate {
        GateType::TTLNand(_) => {
            (7e-9, 11e-9, 15e-9, 10e-9)
        }
        GateType::CMOSInverter(inv) => {
            compute_cmos_timing(&inv.nmos.params, &inv.pmos.params, inv.circuit.vdd, c_load)
        }
        GateType::CMOSNand(nand) => {
            compute_cmos_timing(&nand.nmos1.params, &nand.pmos1.params, nand.circuit.vdd, c_load)
        }
        GateType::CMOSNor(nor) => {
            compute_cmos_timing(&nor.nmos1.params, &nor.pmos1.params, nor.circuit.vdd, c_load)
        }
    };

    let tpd = (tphl + tplh) / 2.0;
    let max_frequency = if tpd > 0.0 { 1.0 / (2.0 * tpd) } else { f64::INFINITY };

    TimingAnalysis {
        tphl,
        tplh,
        tpd,
        rise_time,
        fall_time,
        max_frequency,
    }
}

/// Helper to compute CMOS timing from NMOS/PMOS parameters.
fn compute_cmos_timing(
    nmos_params: &MOSFETParams,
    pmos_params: &MOSFETParams,
    vdd: f64,
    c_load: f64,
) -> (f64, f64, f64, f64) {
    let k_n = nmos_params.k;
    let vth_n = nmos_params.vth;
    let k_p = pmos_params.k;
    let vth_p = pmos_params.vth;

    let ids_sat_n = if vdd > vth_n {
        0.5 * k_n * (vdd - vth_n).powi(2)
    } else {
        1e-12
    };

    let ids_sat_p = if vdd > vth_p {
        0.5 * k_p * (vdd - vth_p).powi(2)
    } else {
        1e-12
    };

    // Propagation delays
    let tphl = c_load * vdd / (2.0 * ids_sat_n); // Pull-down (NMOS)
    let tplh = c_load * vdd / (2.0 * ids_sat_p); // Pull-up (PMOS)

    // Rise and fall times (2.2 RC time constants)
    let r_on_n = if ids_sat_n > 0.0 { vdd / (2.0 * ids_sat_n) } else { 1e6 };
    let r_on_p = if ids_sat_p > 0.0 { vdd / (2.0 * ids_sat_p) } else { 1e6 };
    let rise_time = 2.2 * r_on_p * c_load;
    let fall_time = 2.2 * r_on_n * c_load;

    (tphl, tplh, rise_time, fall_time)
}

/// Compare CMOS and TTL NAND gates across all metrics.
///
/// This function demonstrates WHY CMOS replaced TTL:
/// - CMOS has ~1000x less static power
/// - CMOS has better noise margins (relative to Vdd)
/// - CMOS can operate at lower voltages
pub fn compare_cmos_vs_ttl(
    frequency: Option<f64>,
    c_load: Option<f64>,
) -> HashMap<String, HashMap<String, f64>> {
    let frequency = frequency.unwrap_or(1e6);
    let c_load = c_load.unwrap_or(1e-12);

    let cmos_nand = CMOSNand::new(None, None, None);
    let ttl_nand = TTLNand::new(None, None);

    let cmos_power = analyze_power(
        GateType::CMOSNand(&cmos_nand),
        Some(frequency),
        Some(c_load),
        None,
    );
    let ttl_power = analyze_power(
        GateType::TTLNand(&ttl_nand),
        Some(frequency),
        Some(c_load),
        None,
    );

    let cmos_timing = analyze_timing(GateType::CMOSNand(&cmos_nand), Some(c_load));
    let ttl_timing = analyze_timing(GateType::TTLNand(&ttl_nand), Some(c_load));

    let cmos_inv = CMOSInverter::new(None, None, None);
    let cmos_nm = compute_noise_margins(GateType::CMOSInverter(&cmos_inv));
    let ttl_nm = compute_noise_margins(GateType::TTLNand(&ttl_nand));

    let mut cmos = HashMap::new();
    cmos.insert("transistor_count".to_string(), 4.0);
    cmos.insert("supply_voltage".to_string(), cmos_nand.circuit.vdd);
    cmos.insert("static_power_w".to_string(), cmos_power.static_power);
    cmos.insert("dynamic_power_w".to_string(), cmos_power.dynamic_power);
    cmos.insert("total_power_w".to_string(), cmos_power.total_power);
    cmos.insert("propagation_delay_s".to_string(), cmos_timing.tpd);
    cmos.insert("max_frequency_hz".to_string(), cmos_timing.max_frequency);
    cmos.insert("noise_margin_low_v".to_string(), cmos_nm.nml);
    cmos.insert("noise_margin_high_v".to_string(), cmos_nm.nmh);

    let mut ttl = HashMap::new();
    ttl.insert("transistor_count".to_string(), 3.0);
    ttl.insert("supply_voltage".to_string(), ttl_nand.vcc);
    ttl.insert("static_power_w".to_string(), ttl_power.static_power);
    ttl.insert("dynamic_power_w".to_string(), ttl_power.dynamic_power);
    ttl.insert("total_power_w".to_string(), ttl_power.total_power);
    ttl.insert("propagation_delay_s".to_string(), ttl_timing.tpd);
    ttl.insert("max_frequency_hz".to_string(), ttl_timing.max_frequency);
    ttl.insert("noise_margin_low_v".to_string(), ttl_nm.nml);
    ttl.insert("noise_margin_high_v".to_string(), ttl_nm.nmh);

    let mut result = HashMap::new();
    result.insert("cmos".to_string(), cmos);
    result.insert("ttl".to_string(), ttl);
    result
}

/// Show how CMOS performance changes with technology scaling.
///
/// As transistors shrink (Moore's Law), several properties change:
/// - Gate length decreases -> faster switching
/// - Supply voltage decreases -> less power per switch
/// - Gate capacitance decreases -> less energy per transition
/// - BUT leakage current INCREASES -> more static power (the "leakage wall")
pub fn demonstrate_cmos_scaling(
    technology_nodes: Option<&[f64]>,
) -> Vec<HashMap<String, f64>> {
    let default_nodes = [180e-9, 90e-9, 45e-9, 22e-9, 7e-9, 3e-9];
    let nodes = technology_nodes.unwrap_or(&default_nodes);

    let mut results = Vec::new();

    for &node in nodes {
        // Empirical scaling relationships (simplified)
        let scale = node / 180e-9;

        let vdd = (3.3 * scale.powf(0.5)).max(0.7);
        let vth = (0.4 * scale.powf(0.3)).max(0.15);
        let c_gate = 1e-15 * scale;
        let k = 0.001 / scale.powf(0.5);

        // Create transistor and circuit with scaled parameters
        let params = MOSFETParams {
            vth,
            k,
            l: node,
            c_gate,
            ..MOSFETParams::default()
        };
        let circuit = CircuitParams {
            vdd,
            ..CircuitParams::default()
        };
        let inv = CMOSInverter::new(Some(circuit), Some(params), Some(params));

        let c_load_scaled = c_gate * 10.0;
        let timing = analyze_timing(
            GateType::CMOSInverter(&inv),
            Some(c_load_scaled),
        );
        let power = analyze_power(
            GateType::CMOSInverter(&inv),
            Some(1e9),
            Some(c_load_scaled),
            None,
        );

        // Leakage current increases exponentially as Vth decreases
        let leakage = 1e-12 * ((0.4 - vth) / 0.052).exp();

        let mut entry = HashMap::new();
        entry.insert("node_nm".to_string(), node * 1e9);
        entry.insert("vdd_v".to_string(), vdd);
        entry.insert("vth_v".to_string(), vth);
        entry.insert("c_gate_f".to_string(), c_gate);
        entry.insert("propagation_delay_s".to_string(), timing.tpd);
        entry.insert("dynamic_power_w".to_string(), power.dynamic_power);
        entry.insert("leakage_current_a".to_string(), leakage);
        entry.insert("max_frequency_hz".to_string(), timing.max_frequency);

        results.push(entry);
    }

    results
}

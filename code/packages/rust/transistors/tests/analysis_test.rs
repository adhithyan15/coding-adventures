//! Tests for electrical analysis functions.

use transistors::analysis::{
    analyze_power, analyze_timing, compare_cmos_vs_ttl, compute_noise_margins,
    demonstrate_cmos_scaling, GateType,
};
use transistors::cmos_gates::{CMOSInverter, CMOSNand, CMOSNor};
use transistors::ttl_gates::TTLNand;

// =========================================================================
// Noise Margin Tests
// =========================================================================

#[test]
fn cmos_positive_noise_margins() {
    // CMOS noise margins should be positive
    let inv = CMOSInverter::new(None, None, None);
    let nm = compute_noise_margins(GateType::CMOSInverter(&inv));
    assert!(nm.nml > 0.0);
    assert!(nm.nmh > 0.0);
}

#[test]
fn cmos_symmetric_noise_margins() {
    // CMOS noise margins should be roughly symmetric
    let inv = CMOSInverter::new(None, None, None);
    let nm = compute_noise_margins(GateType::CMOSInverter(&inv));
    assert!((nm.nml - nm.nmh).abs() < nm.nml * 0.5);
}

#[test]
fn ttl_positive_noise_margins() {
    // TTL noise margins should be positive
    let nand = TTLNand::new(None, None);
    let nm = compute_noise_margins(GateType::TTLNand(&nand));
    assert!(nm.nml > 0.0);
    assert!(nm.nmh > 0.0);
}

#[test]
fn cmos_vol_near_zero() {
    // CMOS output LOW should be near 0V
    let inv = CMOSInverter::new(None, None, None);
    let nm = compute_noise_margins(GateType::CMOSInverter(&inv));
    assert!(nm.vol < 0.1);
}

#[test]
fn ttl_vol_vce_sat() {
    // TTL output LOW should be near Vce_sat
    let nand = TTLNand::new(None, None);
    let nm = compute_noise_margins(GateType::TTLNand(&nand));
    assert!(nm.vol < 0.5);
}

// =========================================================================
// Power Analysis Tests
// =========================================================================

#[test]
fn cmos_zero_static_power() {
    // CMOS gates should have near-zero static power
    let inv = CMOSInverter::new(None, None, None);
    let power = analyze_power(GateType::CMOSInverter(&inv), None, None, None);
    assert!(power.static_power < 1e-9);
}

#[test]
fn ttl_significant_static_power() {
    // TTL gates should have milliwatt-level static power
    let nand = TTLNand::new(None, None);
    let power = analyze_power(GateType::TTLNand(&nand), None, None, None);
    assert!(power.static_power > 1e-3);
}

#[test]
fn positive_dynamic_power() {
    // Dynamic power should be positive at non-zero frequency
    let inv = CMOSInverter::new(None, None, None);
    let power = analyze_power(GateType::CMOSInverter(&inv), Some(1e9), None, None);
    assert!(power.dynamic_power > 0.0);
}

#[test]
fn total_power_sum() {
    // Total power should be static + dynamic
    let inv = CMOSInverter::new(None, None, None);
    let power = analyze_power(GateType::CMOSInverter(&inv), Some(1e9), None, None);
    assert!((power.total_power - (power.static_power + power.dynamic_power)).abs() < 1e-15);
}

#[test]
fn energy_per_switch_positive() {
    // Energy per switch should be positive
    let inv = CMOSInverter::new(None, None, None);
    let power = analyze_power(GateType::CMOSInverter(&inv), None, None, None);
    assert!(power.energy_per_switch > 0.0);
}

#[test]
fn cmos_nand_power() {
    // CMOSNand should also work with analyze_power
    let nand = CMOSNand::new(None, None, None);
    let power = analyze_power(GateType::CMOSNand(&nand), None, None, None);
    assert_eq!(power.static_power, 0.0);
}

#[test]
fn cmos_nor_power() {
    // CMOSNor should also work with analyze_power
    let nor = CMOSNor::new(None, None, None);
    let power = analyze_power(GateType::CMOSNor(&nor), None, None, None);
    assert_eq!(power.static_power, 0.0);
}

// =========================================================================
// Timing Analysis Tests
// =========================================================================

#[test]
fn cmos_positive_delays() {
    // CMOS propagation delays should be positive
    let inv = CMOSInverter::new(None, None, None);
    let timing = analyze_timing(GateType::CMOSInverter(&inv), None);
    assert!(timing.tphl > 0.0);
    assert!(timing.tplh > 0.0);
    assert!(timing.tpd > 0.0);
}

#[test]
fn tpd_is_average() {
    // tpd should be the average of tphl and tplh
    let inv = CMOSInverter::new(None, None, None);
    let timing = analyze_timing(GateType::CMOSInverter(&inv), None);
    let expected = (timing.tphl + timing.tplh) / 2.0;
    assert!((timing.tpd - expected).abs() < 1e-20);
}

#[test]
fn cmos_faster_than_ttl() {
    // CMOS delay should be faster than TTL delay
    let inv = CMOSInverter::new(None, None, None);
    let nand = TTLNand::new(None, None);
    let cmos_timing = analyze_timing(GateType::CMOSInverter(&inv), None);
    let ttl_timing = analyze_timing(GateType::TTLNand(&nand), None);
    assert!(cmos_timing.tpd < ttl_timing.tpd);
}

#[test]
fn positive_rise_fall() {
    // Rise and fall times should be positive
    let inv = CMOSInverter::new(None, None, None);
    let timing = analyze_timing(GateType::CMOSInverter(&inv), None);
    assert!(timing.rise_time > 0.0);
    assert!(timing.fall_time > 0.0);
}

#[test]
fn max_frequency_positive() {
    // Maximum frequency should be positive
    let inv = CMOSInverter::new(None, None, None);
    let timing = analyze_timing(GateType::CMOSInverter(&inv), None);
    assert!(timing.max_frequency > 0.0);
}

#[test]
fn cmos_nand_timing() {
    // CMOSNand should also work with analyze_timing
    let nand = CMOSNand::new(None, None, None);
    let timing = analyze_timing(GateType::CMOSNand(&nand), None);
    assert!(timing.tpd > 0.0);
}

#[test]
fn cmos_nor_timing() {
    // CMOSNor should also work with analyze_timing
    let nor = CMOSNor::new(None, None, None);
    let timing = analyze_timing(GateType::CMOSNor(&nor), None);
    assert!(timing.tpd > 0.0);
}

// =========================================================================
// Comparison Utility Tests
// =========================================================================

#[test]
fn compare_returns_both() {
    // compare_cmos_vs_ttl should return both CMOS and TTL data
    let result = compare_cmos_vs_ttl(None, None);
    assert!(result.contains_key("cmos"));
    assert!(result.contains_key("ttl"));
}

#[test]
fn cmos_less_static_power() {
    // CMOS should have much less static power than TTL
    let result = compare_cmos_vs_ttl(None, None);
    assert!(result["cmos"]["static_power_w"] < result["ttl"]["static_power_w"]);
}

#[test]
fn scaling_returns_list() {
    // demonstrate_cmos_scaling should return a list of entries
    let result = demonstrate_cmos_scaling(None);
    assert!(!result.is_empty());
}

#[test]
fn scaling_default_nodes() {
    // Default should produce 6 technology nodes
    let result = demonstrate_cmos_scaling(None);
    assert_eq!(result.len(), 6);
}

#[test]
fn scaling_custom_nodes() {
    // Custom technology nodes should be respected
    let nodes = [180e-9, 45e-9];
    let result = demonstrate_cmos_scaling(Some(&nodes));
    assert_eq!(result.len(), 2);
}

#[test]
fn scaling_vdd_decreases() {
    // Supply voltage should generally decrease with scaling
    let result = demonstrate_cmos_scaling(None);
    assert!(result[0]["vdd_v"] > result[result.len() - 1]["vdd_v"]);
}

#[test]
fn scaling_has_expected_keys() {
    // Each scaling result should have expected keys
    let nodes = [180e-9];
    let result = demonstrate_cmos_scaling(Some(&nodes));
    let entry = &result[0];
    assert!(entry.contains_key("node_nm"));
    assert!(entry.contains_key("vdd_v"));
    assert!(entry.contains_key("vth_v"));
    assert!(entry.contains_key("propagation_delay_s"));
    assert!(entry.contains_key("dynamic_power_w"));
    assert!(entry.contains_key("leakage_current_a"));
}

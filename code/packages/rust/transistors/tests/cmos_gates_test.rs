//! Tests for CMOS logic gates built from transistors.

use transistors::cmos_gates::{CMOSAnd, CMOSInverter, CMOSNand, CMOSNor, CMOSOr, CMOSXor};
use transistors::types::CircuitParams;

// =========================================================================
// CMOS Inverter Tests
// =========================================================================

#[test]
fn inverter_truth_table() {
    // NOT gate: 0->1, 1->0
    let inv = CMOSInverter::new(None, None, None);
    assert_eq!(inv.evaluate_digital(0).unwrap(), 1);
    assert_eq!(inv.evaluate_digital(1).unwrap(), 0);
}

#[test]
fn inverter_voltage_swing_high_input() {
    // Input HIGH -> output near GND
    let inv = CMOSInverter::new(Some(CircuitParams { vdd: 3.3, ..Default::default() }), None, None);
    let result = inv.evaluate(3.3);
    assert!(result.voltage < 0.1);
}

#[test]
fn inverter_voltage_swing_low_input() {
    // Input LOW -> output near Vdd
    let inv = CMOSInverter::new(Some(CircuitParams { vdd: 3.3, ..Default::default() }), None, None);
    let result = inv.evaluate(0.0);
    assert!(result.voltage > 3.2);
}

#[test]
fn inverter_static_power_zero() {
    // CMOS should have near-zero static power
    let inv = CMOSInverter::new(None, None, None);
    assert!(inv.static_power() < 1e-9);
}

#[test]
fn inverter_dynamic_power() {
    // Dynamic power should be positive and scale with V^2
    let inv = CMOSInverter::new(Some(CircuitParams { vdd: 3.3, ..Default::default() }), None, None);
    let p = inv.dynamic_power(1e9, 1e-12);
    assert!(p > 0.0);
}

#[test]
fn inverter_dynamic_power_scales_with_v_squared() {
    // Halving Vdd should reduce dynamic power by ~4x
    let inv_high = CMOSInverter::new(Some(CircuitParams { vdd: 3.3, ..Default::default() }), None, None);
    let inv_low = CMOSInverter::new(Some(CircuitParams { vdd: 1.65, ..Default::default() }), None, None);
    let p_high = inv_high.dynamic_power(1e9, 1e-12);
    let p_low = inv_low.dynamic_power(1e9, 1e-12);
    let ratio = p_high / p_low;
    assert!(ratio > 3.5 && ratio < 4.5);
}

#[test]
fn inverter_vtc_has_sharp_transition() {
    // VTC should show output snap from HIGH to LOW
    let inv = CMOSInverter::new(Some(CircuitParams { vdd: 3.3, ..Default::default() }), None, None);
    let vtc = inv.voltage_transfer_characteristic(10);
    assert_eq!(vtc.len(), 11);
    // First point: input=0, output should be HIGH
    assert!(vtc[0].1 > 3.0);
    // Last point: input=Vdd, output should be LOW
    assert!(vtc[vtc.len() - 1].1 < 0.5);
}

#[test]
fn inverter_rejects_invalid_input() {
    // evaluate_digital should reject non-binary inputs
    let inv = CMOSInverter::new(None, None, None);
    assert!(inv.evaluate_digital(2).is_err());
}

#[test]
fn inverter_transistor_count() {
    // Inverter uses 2 transistors
    let inv = CMOSInverter::new(None, None, None);
    let result = inv.evaluate(0.0);
    assert_eq!(result.transistor_count, 2);
}

// =========================================================================
// CMOS NAND Tests
// =========================================================================

#[test]
fn nand_truth_table() {
    let nand = CMOSNand::new(None, None, None);
    assert_eq!(nand.evaluate_digital(0, 0).unwrap(), 1);
    assert_eq!(nand.evaluate_digital(0, 1).unwrap(), 1);
    assert_eq!(nand.evaluate_digital(1, 0).unwrap(), 1);
    assert_eq!(nand.evaluate_digital(1, 1).unwrap(), 0);
}

#[test]
fn nand_transistor_count() {
    let nand = CMOSNand::new(None, None, None);
    assert_eq!(nand.transistor_count(), 4);
}

#[test]
fn nand_voltage_output_high() {
    let nand = CMOSNand::new(Some(CircuitParams { vdd: 3.3, ..Default::default() }), None, None);
    let result = nand.evaluate(0.0, 0.0);
    assert!(result.voltage > 3.0);
}

#[test]
fn nand_voltage_output_low() {
    let nand = CMOSNand::new(Some(CircuitParams { vdd: 3.3, ..Default::default() }), None, None);
    let result = nand.evaluate(3.3, 3.3);
    assert!(result.voltage < 0.5);
}

#[test]
fn nand_rejects_invalid_input() {
    let nand = CMOSNand::new(None, None, None);
    assert!(nand.evaluate_digital(2, 0).is_err());
}

// =========================================================================
// CMOS NOR Tests
// =========================================================================

#[test]
fn nor_truth_table() {
    let nor = CMOSNor::new(None, None, None);
    assert_eq!(nor.evaluate_digital(0, 0).unwrap(), 1);
    assert_eq!(nor.evaluate_digital(0, 1).unwrap(), 0);
    assert_eq!(nor.evaluate_digital(1, 0).unwrap(), 0);
    assert_eq!(nor.evaluate_digital(1, 1).unwrap(), 0);
}

#[test]
fn nor_rejects_invalid_input() {
    let nor = CMOSNor::new(None, None, None);
    assert!(nor.evaluate_digital(0, 2).is_err());
}

// =========================================================================
// CMOS AND Tests
// =========================================================================

#[test]
fn and_truth_table() {
    let and_gate = CMOSAnd::new(None);
    assert_eq!(and_gate.evaluate_digital(0, 0).unwrap(), 0);
    assert_eq!(and_gate.evaluate_digital(0, 1).unwrap(), 0);
    assert_eq!(and_gate.evaluate_digital(1, 0).unwrap(), 0);
    assert_eq!(and_gate.evaluate_digital(1, 1).unwrap(), 1);
}

#[test]
fn and_rejects_invalid_input() {
    let and_gate = CMOSAnd::new(None);
    assert!(and_gate.evaluate_digital(2, 0).is_err());
}

// =========================================================================
// CMOS OR Tests
// =========================================================================

#[test]
fn or_truth_table() {
    let or_gate = CMOSOr::new(None);
    assert_eq!(or_gate.evaluate_digital(0, 0).unwrap(), 0);
    assert_eq!(or_gate.evaluate_digital(0, 1).unwrap(), 1);
    assert_eq!(or_gate.evaluate_digital(1, 0).unwrap(), 1);
    assert_eq!(or_gate.evaluate_digital(1, 1).unwrap(), 1);
}

#[test]
fn or_rejects_invalid_input() {
    let or_gate = CMOSOr::new(None);
    assert!(or_gate.evaluate_digital(0, 2).is_err());
}

// =========================================================================
// CMOS XOR Tests
// =========================================================================

#[test]
fn xor_truth_table() {
    let xor_gate = CMOSXor::new(None);
    assert_eq!(xor_gate.evaluate_digital(0, 0).unwrap(), 0);
    assert_eq!(xor_gate.evaluate_digital(0, 1).unwrap(), 1);
    assert_eq!(xor_gate.evaluate_digital(1, 0).unwrap(), 1);
    assert_eq!(xor_gate.evaluate_digital(1, 1).unwrap(), 0);
}

#[test]
fn xor_evaluate_from_nands() {
    // NAND-based XOR should match direct XOR
    let xor_gate = CMOSXor::new(None);
    for a in 0..=1u8 {
        for b in 0..=1u8 {
            assert_eq!(
                xor_gate.evaluate_from_nands(a, b).unwrap(),
                xor_gate.evaluate_digital(a, b).unwrap()
            );
        }
    }
}

#[test]
fn xor_rejects_invalid_input() {
    let xor_gate = CMOSXor::new(None);
    assert!(xor_gate.evaluate_digital(0, 2).is_err());
}

//! Tests for TTL logic gates (historical BJT-based).

use transistors::ttl_gates::{RTLInverter, TTLNand};

// =========================================================================
// TTL NAND Tests
// =========================================================================

#[test]
fn ttl_nand_truth_table() {
    let nand = TTLNand::new(None, None);
    assert_eq!(nand.evaluate_digital(0, 0).unwrap(), 1);
    assert_eq!(nand.evaluate_digital(0, 1).unwrap(), 1);
    assert_eq!(nand.evaluate_digital(1, 0).unwrap(), 1);
    assert_eq!(nand.evaluate_digital(1, 1).unwrap(), 0);
}

#[test]
fn ttl_nand_static_power_milliwatts() {
    // TTL gates dissipate milliwatts even when idle
    let nand = TTLNand::new(None, None);
    assert!(nand.static_power() > 1e-3);
}

#[test]
fn ttl_nand_output_voltage_low() {
    // Output LOW should be near Vce_sat (~0.2V)
    let nand = TTLNand::new(None, None);
    let result = nand.evaluate(5.0, 5.0);
    assert!(result.voltage < 0.5);
    assert_eq!(result.logic_value, 0);
}

#[test]
fn ttl_nand_output_voltage_high() {
    // Output HIGH should be near Vcc - 0.7V
    let nand = TTLNand::new(None, None);
    let result = nand.evaluate(0.0, 0.0);
    assert!(result.voltage > 3.0);
    assert_eq!(result.logic_value, 1);
}

#[test]
fn ttl_nand_propagation_delay() {
    // TTL should have propagation delay in nanosecond range
    let nand = TTLNand::new(None, None);
    let result = nand.evaluate(5.0, 5.0);
    assert!(result.propagation_delay > 1e-9);
    assert!(result.propagation_delay < 100e-9);
}

#[test]
fn ttl_nand_rejects_invalid_input() {
    let nand = TTLNand::new(None, None);
    assert!(nand.evaluate_digital(2, 0).is_err());
}

#[test]
fn ttl_nand_custom_vcc() {
    // Custom Vcc should be respected
    let nand = TTLNand::new(Some(3.3), None);
    assert_eq!(nand.vcc, 3.3);
}

// =========================================================================
// RTL Inverter Tests
// =========================================================================

#[test]
fn rtl_inverter_truth_table() {
    let inv = RTLInverter::new(None, None, None, None);
    assert_eq!(inv.evaluate_digital(0).unwrap(), 1);
    assert_eq!(inv.evaluate_digital(1).unwrap(), 0);
}

#[test]
fn rtl_inverter_output_voltage_high() {
    // Input LOW -> output near Vcc
    let inv = RTLInverter::new(None, None, None, None);
    let result = inv.evaluate(0.0);
    assert!(result.voltage > 4.0);
    assert_eq!(result.logic_value, 1);
}

#[test]
fn rtl_inverter_output_voltage_low() {
    // Input HIGH -> output near GND
    let inv = RTLInverter::new(None, None, None, None);
    let result = inv.evaluate(5.0);
    assert!(result.voltage < 1.0);
    assert_eq!(result.logic_value, 0);
}

#[test]
fn rtl_inverter_propagation_delay() {
    // RTL should be slower than TTL
    let inv = RTLInverter::new(None, None, None, None);
    let result = inv.evaluate(5.0);
    assert!(result.propagation_delay > 10e-9);
}

#[test]
fn rtl_inverter_rejects_invalid_input() {
    let inv = RTLInverter::new(None, None, None, None);
    assert!(inv.evaluate_digital(2).is_err());
}

#[test]
fn rtl_inverter_custom_resistors() {
    // Custom resistor values should be respected
    let inv = RTLInverter::new(None, Some(5000.0), Some(2000.0), None);
    assert_eq!(inv.r_base, 5000.0);
    assert_eq!(inv.r_collector, 2000.0);
}

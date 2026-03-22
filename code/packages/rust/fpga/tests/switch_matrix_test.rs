//! Integration tests for the SwitchMatrix module.

use fpga::switch_matrix::SwitchMatrix;
use std::collections::HashMap;

fn make_ports(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| s.to_string()).collect()
}

#[test]
fn test_basic_routing() {
    let mut sm = SwitchMatrix::new(make_ports(&["north", "south", "east", "west", "clb_out"]));
    sm.connect("clb_out", "east").unwrap();
    sm.connect("north", "south").unwrap();

    let mut inputs = HashMap::new();
    inputs.insert("clb_out".to_string(), 1u8);
    inputs.insert("north".to_string(), 0u8);
    let outputs = sm.route(&inputs);

    assert_eq!(outputs.get("east"), Some(&1));
    assert_eq!(outputs.get("south"), Some(&0));
}

#[test]
fn test_fan_out() {
    let mut sm = SwitchMatrix::new(make_ports(&["a", "b", "c"]));
    sm.connect("a", "b").unwrap();
    sm.connect("a", "c").unwrap();

    let mut inputs = HashMap::new();
    inputs.insert("a".to_string(), 1u8);
    let outputs = sm.route(&inputs);

    assert_eq!(outputs.get("b"), Some(&1));
    assert_eq!(outputs.get("c"), Some(&1));
}

#[test]
fn test_disconnect() {
    let mut sm = SwitchMatrix::new(make_ports(&["a", "b"]));
    sm.connect("a", "b").unwrap();
    assert_eq!(sm.connection_count(), 1);

    sm.disconnect("b").unwrap();
    assert_eq!(sm.connection_count(), 0);
}

#[test]
fn test_clear_all_connections() {
    let mut sm = SwitchMatrix::new(make_ports(&["a", "b", "c", "d"]));
    sm.connect("a", "b").unwrap();
    sm.connect("c", "d").unwrap();
    assert_eq!(sm.connection_count(), 2);

    sm.clear();
    assert_eq!(sm.connection_count(), 0);
}

#[test]
fn test_unknown_source_error() {
    let mut sm = SwitchMatrix::new(make_ports(&["a", "b"]));
    let result = sm.connect("nonexistent", "b");
    assert!(result.is_err());
}

#[test]
fn test_unknown_destination_error() {
    let mut sm = SwitchMatrix::new(make_ports(&["a", "b"]));
    let result = sm.connect("a", "nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_self_connection_error() {
    let mut sm = SwitchMatrix::new(make_ports(&["a", "b"]));
    let result = sm.connect("a", "a");
    assert!(result.is_err());
}

#[test]
fn test_duplicate_destination_error() {
    let mut sm = SwitchMatrix::new(make_ports(&["a", "b", "c"]));
    sm.connect("a", "b").unwrap();
    let result = sm.connect("c", "b"); // b already has a source
    assert!(result.is_err());
}

#[test]
fn test_unconnected_input_not_routed() {
    let mut sm = SwitchMatrix::new(make_ports(&["a", "b", "c"]));
    sm.connect("a", "b").unwrap();

    let mut inputs = HashMap::new();
    inputs.insert("c".to_string(), 1u8); // c is not connected to anything
    let outputs = sm.route(&inputs);
    assert!(outputs.is_empty());
}

#[test]
fn test_ports_accessor() {
    let sm = SwitchMatrix::new(make_ports(&["x", "y", "z"]));
    assert_eq!(sm.ports().len(), 3);
    assert!(sm.ports().contains("x"));
    assert!(sm.ports().contains("y"));
    assert!(sm.ports().contains("z"));
}

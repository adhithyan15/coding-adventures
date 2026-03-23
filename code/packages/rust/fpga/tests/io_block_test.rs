//! Integration tests for the IOBlock module.

use fpga::io_block::{IOBlock, IOMode};

#[test]
fn test_input_mode_pad_to_fabric() {
    let mut io = IOBlock::new("in0".to_string(), IOMode::Input);
    io.drive_pad(1);
    assert_eq!(io.read_internal(), Some(1));
    assert_eq!(io.read_pad(), Some(1));
}

#[test]
fn test_output_mode_fabric_to_pad() {
    let mut io = IOBlock::new("out0".to_string(), IOMode::Output);
    io.drive_internal(1);
    assert_eq!(io.read_pad(), Some(1));

    io.drive_internal(0);
    assert_eq!(io.read_pad(), Some(0));
}

#[test]
fn test_tristate_mode_high_impedance() {
    let mut io = IOBlock::new("bus0".to_string(), IOMode::Tristate);
    io.drive_internal(1);
    assert_eq!(io.read_pad(), None); // high-Z
}

#[test]
fn test_mode_change() {
    let mut io = IOBlock::new("pin0".to_string(), IOMode::Input);
    assert_eq!(io.mode(), IOMode::Input);

    io.configure(IOMode::Output);
    assert_eq!(io.mode(), IOMode::Output);

    io.configure(IOMode::Tristate);
    assert_eq!(io.mode(), IOMode::Tristate);
}

#[test]
fn test_name_accessor() {
    let io = IOBlock::new("my_pin".to_string(), IOMode::Input);
    assert_eq!(io.name(), "my_pin");
}

#[test]
fn test_input_default_value_zero() {
    let io = IOBlock::new("pin".to_string(), IOMode::Input);
    assert_eq!(io.read_internal(), Some(0));
    assert_eq!(io.read_pad(), Some(0));
}

#[test]
fn test_output_default_value_zero() {
    let io = IOBlock::new("pin".to_string(), IOMode::Output);
    assert_eq!(io.read_pad(), Some(0));
}

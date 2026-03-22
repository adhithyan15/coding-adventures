//! Integration tests for the FPGA fabric.

use fpga::bitstream::Bitstream;
use fpga::fabric::FPGA;
use std::collections::HashMap;

#[test]
fn test_fpga_creates_clbs_from_bitstream() {
    let json = r#"{
        "clbs": {
            "clb_0": { "slice0": { "lut_a": [0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0] } },
            "clb_1": { "slice0": {} }
        }
    }"#;
    let bs = Bitstream::from_json_str(json).unwrap();
    let fpga = FPGA::new(bs);
    assert_eq!(fpga.clbs().len(), 2);
}

#[test]
fn test_fpga_evaluate_and_gate() {
    let json = r#"{
        "clbs": {
            "clb_0": {
                "slice0": {
                    "lut_a": [0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0]
                }
            }
        }
    }"#;
    let bs = Bitstream::from_json_str(json).unwrap();
    let mut fpga = FPGA::new(bs);

    // AND(1,1) = 1
    let out = fpga.evaluate_clb(
        "clb_0",
        &[1, 1, 0, 0],
        &[0, 0, 0, 0],
        &[0, 0, 0, 0],
        &[0, 0, 0, 0],
        0,
        0,
    );
    assert_eq!(out.slice0.output_a, 1);

    // AND(1,0) = 0
    let out = fpga.evaluate_clb(
        "clb_0",
        &[1, 0, 0, 0],
        &[0, 0, 0, 0],
        &[0, 0, 0, 0],
        &[0, 0, 0, 0],
        0,
        0,
    );
    assert_eq!(out.slice0.output_a, 0);
}

#[test]
fn test_fpga_io_pins() {
    let json = r#"{
        "io": {
            "btn_0": { "mode": "input" },
            "led_0": { "mode": "output" },
            "bus_0": { "mode": "tristate" }
        }
    }"#;
    let bs = Bitstream::from_json_str(json).unwrap();
    let mut fpga = FPGA::new(bs);

    // Input pin
    fpga.set_input("btn_0", 1);
    assert_eq!(fpga.read_output("btn_0"), Some(1));

    // Output pin
    fpga.drive_output("led_0", 1);
    assert_eq!(fpga.read_output("led_0"), Some(1));

    // Tristate pin
    fpga.drive_output("bus_0", 1);
    assert_eq!(fpga.read_output("bus_0"), None); // high-Z
}

#[test]
fn test_fpga_routing() {
    let json = r#"{
        "routing": {
            "sw_0": [
                { "src": "clb_out", "dst": "east" },
                { "src": "north", "dst": "south" }
            ]
        }
    }"#;
    let bs = Bitstream::from_json_str(json).unwrap();
    let fpga = FPGA::new(bs);

    let mut signals = HashMap::new();
    signals.insert("clb_out".to_string(), 1u8);
    signals.insert("north".to_string(), 0u8);

    let out = fpga.route("sw_0", &signals);
    assert_eq!(out.get("east"), Some(&1));
    assert_eq!(out.get("south"), Some(&0));
}

#[test]
fn test_fpga_full_pipeline() {
    // A complete FPGA with CLB, routing, and I/O
    let json = r#"{
        "clbs": {
            "clb_0": {
                "slice0": {
                    "lut_a": [0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0]
                }
            }
        },
        "routing": {
            "sw_0": [
                { "src": "a", "dst": "b" }
            ]
        },
        "io": {
            "in_a": { "mode": "input" },
            "in_b": { "mode": "input" },
            "out":  { "mode": "output" }
        }
    }"#;
    let bs = Bitstream::from_json_str(json).unwrap();
    let mut fpga = FPGA::new(bs);

    // Set input pins
    fpga.set_input("in_a", 1);
    fpga.set_input("in_b", 1);

    // Evaluate CLB
    let clb_out = fpga.evaluate_clb(
        "clb_0",
        &[1, 1, 0, 0],
        &[0, 0, 0, 0],
        &[0, 0, 0, 0],
        &[0, 0, 0, 0],
        0,
        0,
    );
    assert_eq!(clb_out.slice0.output_a, 1); // AND(1,1)

    // Drive output pin with CLB result
    fpga.drive_output("out", clb_out.slice0.output_a);
    assert_eq!(fpga.read_output("out"), Some(1));
}

#[test]
fn test_fpga_bitstream_accessor() {
    let json = r#"{ "lut_k": 4 }"#;
    let bs = Bitstream::from_json_str(json).unwrap();
    let fpga = FPGA::new(bs);
    assert_eq!(fpga.bitstream().lut_k, 4);
}

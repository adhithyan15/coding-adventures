//! Integration tests for the Bitstream module.

use fpga::bitstream::Bitstream;

#[test]
fn test_parse_empty_config() {
    let json = r#"{ "clbs": {}, "routing": {}, "io": {} }"#;
    let bs = Bitstream::from_json_str(json).unwrap();
    assert!(bs.clbs.is_empty());
    assert!(bs.routing.is_empty());
    assert!(bs.io.is_empty());
    assert_eq!(bs.lut_k, 4);
}

#[test]
fn test_parse_custom_lut_k() {
    let json = r#"{ "lut_k": 6 }"#;
    let bs = Bitstream::from_json_str(json).unwrap();
    assert_eq!(bs.lut_k, 6);
}

#[test]
fn test_parse_clb_full() {
    let json = r#"{
        "clbs": {
            "clb_0_0": {
                "slice0": {
                    "lut_a": [0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0],
                    "lut_b": [0,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0],
                    "ff_a": true,
                    "ff_b": false,
                    "carry": true
                },
                "slice1": {
                    "lut_a": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1],
                    "ff_a": false
                }
            }
        }
    }"#;
    let bs = Bitstream::from_json_str(json).unwrap();
    let clb = bs.clbs.get("clb_0_0").unwrap();

    // Slice 0
    assert_eq!(clb.slice0.lut_a[3], 1);
    assert_eq!(clb.slice0.lut_b[1], 1);
    assert!(clb.slice0.ff_a_enabled);
    assert!(!clb.slice0.ff_b_enabled);
    assert!(clb.slice0.carry_enabled);

    // Slice 1
    assert_eq!(clb.slice1.lut_a[15], 1);
    assert!(!clb.slice1.ff_a_enabled);
    // LUT B defaults to all zeros
    assert_eq!(clb.slice1.lut_b, vec![0u8; 16]);
}

#[test]
fn test_parse_routing() {
    let json = r#"{
        "routing": {
            "sw_0_0": [
                { "src": "clb_out_a", "dst": "east" },
                { "src": "north", "dst": "south" }
            ]
        }
    }"#;
    let bs = Bitstream::from_json_str(json).unwrap();
    let routes = bs.routing.get("sw_0_0").unwrap();
    assert_eq!(routes.len(), 2);
    assert_eq!(routes[0].source, "clb_out_a");
    assert_eq!(routes[0].destination, "east");
    assert_eq!(routes[1].source, "north");
    assert_eq!(routes[1].destination, "south");
}

#[test]
fn test_parse_io() {
    let json = r#"{
        "io": {
            "pin_A0": { "mode": "input" },
            "pin_B0": { "mode": "output" },
            "pin_C0": { "mode": "tristate" }
        }
    }"#;
    let bs = Bitstream::from_json_str(json).unwrap();
    assert_eq!(bs.io.get("pin_A0").unwrap().mode, "input");
    assert_eq!(bs.io.get("pin_B0").unwrap().mode, "output");
    assert_eq!(bs.io.get("pin_C0").unwrap().mode, "tristate");
}

#[test]
fn test_parse_minimal_config() {
    // Only io, no clbs or routing
    let json = r#"{ "io": { "led": { "mode": "output" } } }"#;
    let bs = Bitstream::from_json_str(json).unwrap();
    assert!(bs.clbs.is_empty());
    assert!(bs.routing.is_empty());
    assert_eq!(bs.io.len(), 1);
}

#[test]
fn test_invalid_json_error() {
    let result = Bitstream::from_json_str("not valid json");
    assert!(result.is_err());
}

#[test]
fn test_clb_defaults_fill_missing_fields() {
    let json = r#"{
        "clbs": {
            "clb_0": {
                "slice0": {}
            }
        }
    }"#;
    let bs = Bitstream::from_json_str(json).unwrap();
    let clb = bs.clbs.get("clb_0").unwrap();
    // All should be defaults
    assert_eq!(clb.slice0.lut_a, vec![0u8; 16]);
    assert_eq!(clb.slice0.lut_b, vec![0u8; 16]);
    assert!(!clb.slice0.ff_a_enabled);
    assert!(!clb.slice0.ff_b_enabled);
    assert!(!clb.slice0.carry_enabled);
}

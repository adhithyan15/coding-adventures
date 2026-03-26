//! FPGA Fabric — the top-level FPGA model.
//!
//! # What is an FPGA?
//!
//! An FPGA (Field-Programmable Gate Array) is a chip containing:
//! - A grid of CLBs (Configurable Logic Blocks) for computation
//! - A routing fabric (switch matrices) for interconnection
//! - I/O blocks at the perimeter for external connections
//! - Block RAM tiles for on-chip memory
//!
//! The key property: **all of this is programmable**. By loading a
//! bitstream (configuration data), the same physical chip can become
//! any digital circuit.
//!
//! # Our FPGA Model
//!
//! ```text
//! +----------------------------------------------------+
//! |                    FPGA Fabric                       |
//! |                                                     |
//! |  [IO] [IO] [IO] [IO] [IO] [IO] [IO] [IO]          |
//! |                                                     |
//! |  [IO] [CLB]--[SW]--[CLB]--[SW]--[CLB] [IO]        |
//! |         |            |            |                  |
//! |        [SW]         [SW]         [SW]               |
//! |         |            |            |                  |
//! |  [IO] [CLB]--[SW]--[CLB]--[SW]--[CLB] [IO]        |
//! |                                                     |
//! |  [IO] [IO] [IO] [IO] [IO] [IO] [IO] [IO]          |
//! |                                                     |
//! |            [BRAM]        [BRAM]                     |
//! +----------------------------------------------------+
//! ```

use std::collections::HashMap;

use crate::bitstream::Bitstream;
use crate::clb::{CLBOutput, CLB};
use crate::io_block::{IOBlock, IOMode};
use crate::switch_matrix::SwitchMatrix;

/// Top-level FPGA fabric model.
///
/// Creates and configures CLBs, switch matrices, and I/O blocks
/// from a [`Bitstream`], then provides methods to evaluate the
/// configured circuit.
///
/// # Example
///
/// ```
/// use fpga::bitstream::Bitstream;
/// use fpga::fabric::FPGA;
///
/// let json = r#"{
///     "clbs": {
///         "clb_0": {
///             "slice0": {
///                 "lut_a": [0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0]
///             }
///         }
///     },
///     "io": {
///         "in_a": { "mode": "input" },
///         "out":  { "mode": "output" }
///     }
/// }"#;
/// let bs = Bitstream::from_json_str(json).unwrap();
/// let fpga = FPGA::new(bs);
/// assert!(fpga.clbs().contains_key("clb_0"));
/// ```
#[derive(Debug)]
pub struct FPGA {
    bitstream: Bitstream,
    clbs: HashMap<String, CLB>,
    switches: HashMap<String, SwitchMatrix>,
    ios: HashMap<String, IOBlock>,
}

impl FPGA {
    /// Create and configure an FPGA from a bitstream.
    pub fn new(bitstream: Bitstream) -> Self {
        let mut fpga = Self {
            clbs: HashMap::new(),
            switches: HashMap::new(),
            ios: HashMap::new(),
            bitstream: bitstream.clone(),
        };
        fpga.configure(&bitstream);
        fpga
    }

    /// Apply bitstream configuration to create and program all elements.
    fn configure(&mut self, bs: &Bitstream) {
        // Create and configure CLBs
        for (name, clb_cfg) in &bs.clbs {
            let mut clb = CLB::new(bs.lut_k);

            clb.slice0_mut().configure(
                &clb_cfg.slice0.lut_a,
                &clb_cfg.slice0.lut_b,
                clb_cfg.slice0.ff_a_enabled,
                clb_cfg.slice0.ff_b_enabled,
                clb_cfg.slice0.carry_enabled,
            );
            clb.slice1_mut().configure(
                &clb_cfg.slice1.lut_a,
                &clb_cfg.slice1.lut_b,
                clb_cfg.slice1.ff_a_enabled,
                clb_cfg.slice1.ff_b_enabled,
                clb_cfg.slice1.carry_enabled,
            );

            self.clbs.insert(name.clone(), clb);
        }

        // Create and configure switch matrices
        for (sw_name, routes) in &bs.routing {
            // Collect all port names referenced in routes
            let mut port_set = std::collections::HashSet::new();
            for route in routes {
                port_set.insert(route.source.clone());
                port_set.insert(route.destination.clone());
            }

            if !port_set.is_empty() {
                let ports: Vec<String> = port_set.into_iter().collect();
                let mut sm = SwitchMatrix::new(ports);
                for route in routes {
                    sm.connect(&route.source, &route.destination)
                        .expect("routing configuration error");
                }
                self.switches.insert(sw_name.clone(), sm);
            }
        }

        // Create I/O blocks
        for (pin_name, io_cfg) in &bs.io {
            let mode = match io_cfg.mode.as_str() {
                "output" => IOMode::Output,
                "tristate" => IOMode::Tristate,
                _ => IOMode::Input,
            };
            self.ios
                .insert(pin_name.clone(), IOBlock::new(pin_name.clone(), mode));
        }
    }

    /// Evaluate a specific CLB.
    ///
    /// # Panics
    ///
    /// Panics if `clb_name` is not found.
    pub fn evaluate_clb(
        &mut self,
        clb_name: &str,
        slice0_inputs_a: &[u8],
        slice0_inputs_b: &[u8],
        slice1_inputs_a: &[u8],
        slice1_inputs_b: &[u8],
        clock: u8,
        carry_in: u8,
    ) -> CLBOutput {
        let clb = self
            .clbs
            .get_mut(clb_name)
            .unwrap_or_else(|| panic!("CLB {clb_name:?} not found"));

        clb.evaluate(
            slice0_inputs_a,
            slice0_inputs_b,
            slice1_inputs_a,
            slice1_inputs_b,
            clock,
            carry_in,
        )
    }

    /// Route signals through a switch matrix.
    ///
    /// # Panics
    ///
    /// Panics if `switch_name` is not found.
    pub fn route(
        &self,
        switch_name: &str,
        signals: &HashMap<String, u8>,
    ) -> HashMap<String, u8> {
        let sm = self
            .switches
            .get(switch_name)
            .unwrap_or_else(|| panic!("Switch matrix {switch_name:?} not found"));

        sm.route(signals)
    }

    /// Drive an input pin.
    ///
    /// # Panics
    ///
    /// Panics if `pin_name` is not found.
    pub fn set_input(&mut self, pin_name: &str, value: u8) {
        let io = self
            .ios
            .get_mut(pin_name)
            .unwrap_or_else(|| panic!("I/O pin {pin_name:?} not found"));
        io.drive_pad(value);
    }

    /// Read an output pin.
    ///
    /// # Panics
    ///
    /// Panics if `pin_name` is not found.
    pub fn read_output(&self, pin_name: &str) -> Option<u8> {
        let io = self
            .ios
            .get(pin_name)
            .unwrap_or_else(|| panic!("I/O pin {pin_name:?} not found"));
        io.read_pad()
    }

    /// Drive the internal side of an output pin (fabric -> external).
    ///
    /// # Panics
    ///
    /// Panics if `pin_name` is not found.
    pub fn drive_output(&mut self, pin_name: &str, value: u8) {
        let io = self
            .ios
            .get_mut(pin_name)
            .unwrap_or_else(|| panic!("I/O pin {pin_name:?} not found"));
        io.drive_internal(value);
    }

    /// All CLBs in the fabric.
    pub fn clbs(&self) -> &HashMap<String, CLB> {
        &self.clbs
    }

    /// All switch matrices in the fabric.
    pub fn switches(&self) -> &HashMap<String, SwitchMatrix> {
        &self.switches
    }

    /// All I/O blocks.
    pub fn ios(&self) -> &HashMap<String, IOBlock> {
        &self.ios
    }

    /// The loaded bitstream configuration.
    pub fn bitstream(&self) -> &Bitstream {
        &self.bitstream
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fpga_from_bitstream() {
        let json = r#"{
            "clbs": {
                "clb_0": {
                    "slice0": {
                        "lut_a": [0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0]
                    }
                }
            },
            "io": {
                "in_a": { "mode": "input" },
                "out":  { "mode": "output" }
            }
        }"#;
        let bs = Bitstream::from_json_str(json).unwrap();
        let fpga = FPGA::new(bs);

        assert!(fpga.clbs().contains_key("clb_0"));
        assert!(fpga.ios().contains_key("in_a"));
        assert!(fpga.ios().contains_key("out"));
    }

    #[test]
    fn test_fpga_evaluate_clb() {
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

        let out = fpga.evaluate_clb(
            "clb_0",
            &[1, 1, 0, 0], // LUT A inputs: AND(1,1) = 1
            &[0, 0, 0, 0],
            &[0, 0, 0, 0],
            &[0, 0, 0, 0],
            0,
            0,
        );
        assert_eq!(out.slice0.output_a, 1);
    }

    #[test]
    fn test_fpga_io() {
        let json = r#"{
            "io": {
                "in_a": { "mode": "input" },
                "out_b": { "mode": "output" }
            }
        }"#;
        let bs = Bitstream::from_json_str(json).unwrap();
        let mut fpga = FPGA::new(bs);

        fpga.set_input("in_a", 1);
        fpga.drive_output("out_b", 1);
        assert_eq!(fpga.read_output("out_b"), Some(1));
    }

    #[test]
    fn test_fpga_routing() {
        let json = r#"{
            "routing": {
                "sw_0": [
                    { "src": "a", "dst": "b" }
                ]
            }
        }"#;
        let bs = Bitstream::from_json_str(json).unwrap();
        let fpga = FPGA::new(bs);

        let mut signals = HashMap::new();
        signals.insert("a".to_string(), 1u8);
        let out = fpga.route("sw_0", &signals);
        assert_eq!(out.get("b"), Some(&1));
    }
}

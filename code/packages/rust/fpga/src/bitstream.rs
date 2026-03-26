//! Bitstream — FPGA configuration data.
//!
//! # What is a Bitstream?
//!
//! In a real FPGA, a bitstream is a binary blob that programs every
//! configurable element: LUT truth tables, flip-flop enables, carry chain
//! enables, routing switch states, I/O pad modes, and Block RAM contents.
//!
//! The bitstream is loaded at power-up (or during runtime for partial
//! reconfiguration) and writes to the SRAM cells that control the fabric.
//!
//! # Our JSON Configuration
//!
//! Instead of a binary format, we use JSON for readability and education.
//! The JSON configuration specifies:
//!
//! 1. **CLBs**: Which LUTs get which truth tables, FF enables, carry enables
//! 2. **Routing**: Which switch matrix ports are connected
//! 3. **I/O**: Pin names, modes, and mappings
//!
//! # Example JSON
//!
//! ```json
//! {
//!     "clbs": {
//!         "clb_0_0": {
//!             "slice0": {
//!                 "lut_a": [0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0],
//!                 "lut_b": [0,1,1,0,1,0,0,1,1,0,0,1,0,1,1,0],
//!                 "ff_a": true,
//!                 "ff_b": false,
//!                 "carry": false
//!             },
//!             "slice1": { ... }
//!         }
//!     },
//!     "routing": {
//!         "sw_0_0": [
//!             {"src": "clb_out_a", "dst": "east"}
//!         ]
//!     },
//!     "io": {
//!         "pin_A0": {"mode": "input"},
//!         "pin_B0": {"mode": "output"}
//!     }
//! }
//! ```

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Configuration for one slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceConfig {
    /// Truth table for LUT A (2^k entries).
    pub lut_a: Vec<u8>,
    /// Truth table for LUT B (2^k entries).
    pub lut_b: Vec<u8>,
    /// Route LUT A through flip-flop.
    pub ff_a_enabled: bool,
    /// Route LUT B through flip-flop.
    pub ff_b_enabled: bool,
    /// Enable carry chain.
    pub carry_enabled: bool,
}

/// Configuration for one CLB (2 slices).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CLBConfig {
    /// Slice 0 configuration.
    pub slice0: SliceConfig,
    /// Slice 1 configuration.
    pub slice1: SliceConfig,
}

/// A single routing connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteConfig {
    /// Source port name.
    pub source: String,
    /// Destination port name.
    pub destination: String,
}

/// Configuration for one I/O block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IOConfig {
    /// Mode string: "input", "output", or "tristate".
    pub mode: String,
}

/// FPGA configuration data — the "program" for the fabric.
///
/// # Example
///
/// ```
/// use fpga::bitstream::Bitstream;
/// let json = r#"{
///     "clbs": {},
///     "routing": {},
///     "io": { "pin_A0": { "mode": "input" } }
/// }"#;
/// let bs = Bitstream::from_json_str(json).unwrap();
/// assert_eq!(bs.io.get("pin_A0").unwrap().mode, "input");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bitstream {
    /// CLB configurations keyed by name (e.g., "clb_0_0").
    pub clbs: HashMap<String, CLBConfig>,
    /// Switch matrix connections keyed by matrix name.
    pub routing: HashMap<String, Vec<RouteConfig>>,
    /// I/O block configurations keyed by pin name.
    pub io: HashMap<String, IOConfig>,
    /// Number of LUT inputs (default 4).
    pub lut_k: usize,
}

// ===========================================================================
// JSON deserialization types (internal)
// ===========================================================================

#[derive(Deserialize)]
struct JsonSliceConfig {
    lut_a: Option<Vec<u8>>,
    lut_b: Option<Vec<u8>>,
    ff_a: Option<bool>,
    ff_b: Option<bool>,
    carry: Option<bool>,
}

#[derive(Deserialize)]
struct JsonCLBConfig {
    slice0: Option<JsonSliceConfig>,
    slice1: Option<JsonSliceConfig>,
}

#[derive(Deserialize)]
struct JsonRouteEntry {
    src: String,
    dst: String,
}

#[derive(Deserialize)]
struct JsonIOConfig {
    mode: Option<String>,
}

#[derive(Deserialize)]
struct JsonBitstream {
    lut_k: Option<usize>,
    clbs: Option<HashMap<String, JsonCLBConfig>>,
    routing: Option<HashMap<String, Vec<JsonRouteEntry>>>,
    io: Option<HashMap<String, JsonIOConfig>>,
}

impl Bitstream {
    /// Load a bitstream from a JSON file.
    pub fn from_json_file(path: &Path) -> Result<Self, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("failed to read file: {e}"))?;
        Self::from_json_str(&content)
    }

    /// Parse a bitstream from a JSON string.
    pub fn from_json_str(json: &str) -> Result<Self, String> {
        let data: JsonBitstream =
            serde_json::from_str(json).map_err(|e| format!("JSON parse error: {e}"))?;

        let lut_k = data.lut_k.unwrap_or(4);
        let lut_size = 1 << lut_k;

        // Parse CLBs
        let mut clbs = HashMap::new();
        if let Some(clb_map) = data.clbs {
            for (name, clb_data) in clb_map {
                let s0 = clb_data.slice0.unwrap_or(JsonSliceConfig {
                    lut_a: None,
                    lut_b: None,
                    ff_a: None,
                    ff_b: None,
                    carry: None,
                });
                let s1 = clb_data.slice1.unwrap_or(JsonSliceConfig {
                    lut_a: None,
                    lut_b: None,
                    ff_a: None,
                    ff_b: None,
                    carry: None,
                });

                clbs.insert(
                    name,
                    CLBConfig {
                        slice0: SliceConfig {
                            lut_a: s0.lut_a.unwrap_or_else(|| vec![0; lut_size]),
                            lut_b: s0.lut_b.unwrap_or_else(|| vec![0; lut_size]),
                            ff_a_enabled: s0.ff_a.unwrap_or(false),
                            ff_b_enabled: s0.ff_b.unwrap_or(false),
                            carry_enabled: s0.carry.unwrap_or(false),
                        },
                        slice1: SliceConfig {
                            lut_a: s1.lut_a.unwrap_or_else(|| vec![0; lut_size]),
                            lut_b: s1.lut_b.unwrap_or_else(|| vec![0; lut_size]),
                            ff_a_enabled: s1.ff_a.unwrap_or(false),
                            ff_b_enabled: s1.ff_b.unwrap_or(false),
                            carry_enabled: s1.carry.unwrap_or(false),
                        },
                    },
                );
            }
        }

        // Parse routing
        let mut routing = HashMap::new();
        if let Some(route_map) = data.routing {
            for (sw_name, routes) in route_map {
                routing.insert(
                    sw_name,
                    routes
                        .into_iter()
                        .map(|r| RouteConfig {
                            source: r.src,
                            destination: r.dst,
                        })
                        .collect(),
                );
            }
        }

        // Parse I/O
        let mut io = HashMap::new();
        if let Some(io_map) = data.io {
            for (pin_name, io_data) in io_map {
                io.insert(
                    pin_name,
                    IOConfig {
                        mode: io_data.mode.unwrap_or_else(|| "input".to_string()),
                    },
                );
            }
        }

        Ok(Self {
            clbs,
            routing,
            io,
            lut_k,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_bitstream() {
        let json = r#"{ "clbs": {}, "routing": {}, "io": {} }"#;
        let bs = Bitstream::from_json_str(json).unwrap();
        assert!(bs.clbs.is_empty());
        assert!(bs.routing.is_empty());
        assert!(bs.io.is_empty());
        assert_eq!(bs.lut_k, 4);
    }

    #[test]
    fn test_parse_io_config() {
        let json = r#"{
            "io": {
                "pin_A": { "mode": "input" },
                "pin_B": { "mode": "output" }
            }
        }"#;
        let bs = Bitstream::from_json_str(json).unwrap();
        assert_eq!(bs.io.get("pin_A").unwrap().mode, "input");
        assert_eq!(bs.io.get("pin_B").unwrap().mode, "output");
    }

    #[test]
    fn test_parse_clb_config() {
        let json = r#"{
            "clbs": {
                "clb_0": {
                    "slice0": {
                        "lut_a": [0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0],
                        "ff_a": true,
                        "carry": true
                    }
                }
            }
        }"#;
        let bs = Bitstream::from_json_str(json).unwrap();
        let clb = bs.clbs.get("clb_0").unwrap();
        assert_eq!(clb.slice0.lut_a[3], 1);
        assert!(clb.slice0.ff_a_enabled);
        assert!(clb.slice0.carry_enabled);
        assert!(!clb.slice0.ff_b_enabled);
    }

    #[test]
    fn test_parse_routing() {
        let json = r#"{
            "routing": {
                "sw_0": [
                    { "src": "clb_out", "dst": "east" },
                    { "src": "north", "dst": "south" }
                ]
            }
        }"#;
        let bs = Bitstream::from_json_str(json).unwrap();
        let routes = bs.routing.get("sw_0").unwrap();
        assert_eq!(routes.len(), 2);
        assert_eq!(routes[0].source, "clb_out");
        assert_eq!(routes[0].destination, "east");
    }

    #[test]
    fn test_invalid_json_returns_error() {
        let result = Bitstream::from_json_str("not json");
        assert!(result.is_err());
    }
}

//! Switch Matrix — programmable routing crossbar for the FPGA fabric.
//!
//! # What is a Switch Matrix?
//!
//! The routing fabric is what makes an FPGA truly programmable. LUTs and
//! CLBs compute boolean functions, but the switch matrix determines how
//! those functions connect to each other.
//!
//! A switch matrix sits at each intersection of the routing grid. It is a
//! crossbar that can connect any of its input wires to any of its output
//! wires, based on configuration bits stored in SRAM.
//!
//! # Grid Layout
//!
//! ```text
//! +-----+     +-----+     +-----+
//! | CLB |--SW--| CLB |--SW--| CLB |
//! +--+--+     +--+--+     +--+--+
//!    |SW          |SW          |SW
//! +--+--+     +--+--+     +--+--+
//! | CLB |--SW--| CLB |--SW--| CLB |
//! +-----+     +-----+     +-----+
//!
//! SW = Switch Matrix
//! ```
//!
//! # Connection Model
//!
//! We model the switch matrix as a set of named ports and a configurable
//! connection map. Each connection maps a source port to a destination port.
//! When a signal arrives at a source port, the switch matrix routes it to
//! the connected destination port.

use std::collections::{HashMap, HashSet};

/// Programmable routing crossbar.
///
/// Connects named signal ports via configurable routes. Each route
/// maps a source port to a destination port. Multiple routes can
/// share the same source (fan-out) but each destination can only
/// have one source (no bus contention).
///
/// # Example
///
/// ```
/// use fpga::switch_matrix::SwitchMatrix;
/// let ports: Vec<String> = vec!["north", "south", "east", "west", "clb_out"]
///     .into_iter().map(String::from).collect();
/// let mut sm = SwitchMatrix::new(ports);
/// sm.connect("clb_out", "east").unwrap();
/// sm.connect("north", "south").unwrap();
///
/// let mut inputs = std::collections::HashMap::new();
/// inputs.insert("clb_out".to_string(), 1u8);
/// inputs.insert("north".to_string(), 0u8);
/// let outputs = sm.route(&inputs);
/// assert_eq!(outputs.get("east"), Some(&1));
/// assert_eq!(outputs.get("south"), Some(&0));
/// ```
#[derive(Debug, Clone)]
pub struct SwitchMatrix {
    /// Set of all port names.
    ports: HashSet<String>,
    /// Connection map: destination -> source.
    connections: HashMap<String, String>,
}

impl SwitchMatrix {
    /// Create a new switch matrix with the given port names.
    ///
    /// # Panics
    ///
    /// Panics if ports is empty or contains empty strings.
    pub fn new(ports: Vec<String>) -> Self {
        assert!(!ports.is_empty(), "ports must be non-empty");
        for p in &ports {
            assert!(!p.is_empty(), "port names must be non-empty strings");
        }

        Self {
            ports: ports.into_iter().collect(),
            connections: HashMap::new(),
        }
    }

    /// Create a route from source to destination.
    ///
    /// # Errors
    ///
    /// Returns an error string if:
    /// - Source or destination port is unknown
    /// - Source equals destination
    /// - Destination is already connected
    pub fn connect(&mut self, source: &str, destination: &str) -> Result<(), String> {
        if !self.ports.contains(source) {
            return Err(format!("unknown source port: {source:?}"));
        }
        if !self.ports.contains(destination) {
            return Err(format!("unknown destination port: {destination:?}"));
        }
        if source == destination {
            return Err(format!("cannot connect port {source:?} to itself"));
        }
        if self.connections.contains_key(destination) {
            return Err(format!(
                "destination {destination:?} already connected to {:?}",
                self.connections[destination]
            ));
        }

        self.connections
            .insert(destination.to_string(), source.to_string());
        Ok(())
    }

    /// Remove the route to a destination port.
    ///
    /// # Errors
    ///
    /// Returns an error string if the port is unknown or not connected.
    pub fn disconnect(&mut self, destination: &str) -> Result<(), String> {
        if !self.ports.contains(destination) {
            return Err(format!("unknown port: {destination:?}"));
        }
        if !self.connections.contains_key(destination) {
            return Err(format!("port {destination:?} is not connected"));
        }

        self.connections.remove(destination);
        Ok(())
    }

    /// Remove all connections (reset the switch matrix).
    pub fn clear(&mut self) {
        self.connections.clear();
    }

    /// Propagate signals through the switch matrix.
    ///
    /// For each connected destination whose source appears in the input map,
    /// the output map will contain that destination with the source's value.
    pub fn route(&self, inputs: &HashMap<String, u8>) -> HashMap<String, u8> {
        let mut outputs = HashMap::new();
        for (dest, src) in &self.connections {
            if let Some(&val) = inputs.get(src) {
                outputs.insert(dest.clone(), val);
            }
        }
        outputs
    }

    /// Set of all port names.
    pub fn ports(&self) -> &HashSet<String> {
        &self.ports
    }

    /// Current connection map (destination -> source). Returns a clone.
    pub fn connections(&self) -> HashMap<String, String> {
        self.connections.clone()
    }

    /// Number of active connections.
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ports(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_connect_and_route() {
        let mut sm = SwitchMatrix::new(make_ports(&["a", "b", "c"]));
        sm.connect("a", "b").unwrap();

        let mut inputs = HashMap::new();
        inputs.insert("a".to_string(), 1);
        let out = sm.route(&inputs);
        assert_eq!(out.get("b"), Some(&1));
    }

    #[test]
    fn test_disconnect() {
        let mut sm = SwitchMatrix::new(make_ports(&["a", "b"]));
        sm.connect("a", "b").unwrap();
        sm.disconnect("b").unwrap();
        assert_eq!(sm.connection_count(), 0);
    }

    #[test]
    fn test_clear() {
        let mut sm = SwitchMatrix::new(make_ports(&["a", "b", "c"]));
        sm.connect("a", "b").unwrap();
        sm.connect("a", "c").unwrap();
        sm.clear();
        assert_eq!(sm.connection_count(), 0);
    }

    #[test]
    fn test_duplicate_destination_error() {
        let mut sm = SwitchMatrix::new(make_ports(&["a", "b", "c"]));
        sm.connect("a", "b").unwrap();
        let result = sm.connect("c", "b");
        assert!(result.is_err());
    }

    #[test]
    fn test_self_connection_error() {
        let mut sm = SwitchMatrix::new(make_ports(&["a", "b"]));
        let result = sm.connect("a", "a");
        assert!(result.is_err());
    }
}

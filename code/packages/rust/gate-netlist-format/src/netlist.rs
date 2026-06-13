//! HNL data model: Port, Net, NetSlice, Instance, Module, Netlist.
//!
//! Includes JSON serde (stable schema) and structural validation rules R1-R11.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::cells::BUILTIN_CELL_TYPES;

pub const SCHEMA_VERSION: &str = "0.1.0";

// ---------------------------------------------------------------------------
// Direction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Input,
    Output,
    Inout,
}

// ---------------------------------------------------------------------------
// Level
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    #[default]
    Generic,
    Stdcell,
    Mixed,
}

// ---------------------------------------------------------------------------
// Port
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Port {
    pub name: String,
    #[serde(rename = "dir")]
    pub direction: Direction,
    pub width: u32,
}

impl Port {
    pub fn new(name: impl Into<String>, direction: Direction, width: u32) -> Self {
        assert!(width >= 1, "port width must be >= 1");
        Self { name: name.into(), direction, width }
    }
}

// ---------------------------------------------------------------------------
// Net
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Net {
    pub name: String,
    pub width: u32,
}

impl Net {
    pub fn new(name: impl Into<String>, width: u32) -> Self {
        assert!(width >= 1, "net width must be >= 1");
        Self { name: name.into(), width }
    }
}

// ---------------------------------------------------------------------------
// NetSlice
// ---------------------------------------------------------------------------

/// A connection to specific bits of a named net or port.
///
/// A 1-bit connection to net `a` bit 0: `NetSlice { net: "a", bits: [0] }`.
/// A 4-bit connection to net `sum` bits 3:0: `bits: [3,2,1,0]` (MSB-first).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetSlice {
    pub net: String,
    pub bits: Vec<u32>,
}

impl NetSlice {
    pub fn single(net: impl Into<String>, bit: u32) -> Self {
        Self { net: net.into(), bits: vec![bit] }
    }

    pub fn range(net: impl Into<String>, msb: u32, lsb: u32) -> Self {
        let bits = (lsb..=msb).rev().collect();
        Self { net: net.into(), bits }
    }

    pub fn width(&self) -> u32 {
        self.bits.len() as u32
    }
}

// ---------------------------------------------------------------------------
// Instance
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instance {
    pub name: String,
    #[serde(rename = "type")]
    pub cell_type: String,
    #[serde(default)]
    pub connections: HashMap<String, NetSlice>,
    #[serde(default)]
    pub parameters: HashMap<String, serde_json::Value>,
}

impl Instance {
    pub fn new(name: impl Into<String>, cell_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cell_type: cell_type.into(),
            connections: HashMap::new(),
            parameters: HashMap::new(),
        }
    }

    pub fn connect(mut self, pin: impl Into<String>, slice: NetSlice) -> Self {
        self.connections.insert(pin.into(), slice);
        self
    }
}

// ---------------------------------------------------------------------------
// Module
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Module {
    #[serde(skip)]
    pub name: String,
    #[serde(default)]
    pub ports: Vec<Port>,
    #[serde(default)]
    pub nets: Vec<Net>,
    #[serde(default)]
    pub instances: Vec<Instance>,
}

impl Module {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), ..Default::default() }
    }

    pub fn port(&self, name: &str) -> Option<&Port> {
        self.ports.iter().find(|p| p.name == name)
    }

    pub fn net(&self, name: &str) -> Option<&Net> {
        self.nets.iter().find(|n| n.name == name)
    }
}

// ---------------------------------------------------------------------------
// Netlist
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Netlist {
    #[serde(rename = "format", default = "default_format")]
    pub format: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub level: Level,
    pub top: String,
    /// Module bodies keyed by name. During serde the name comes from the
    /// map key, so each `Module`'s `name` field is skipped in JSON.
    pub modules: HashMap<String, Module>,
}

fn default_format() -> String { "HNL".to_string() }
fn default_version() -> String { SCHEMA_VERSION.to_string() }

impl Netlist {
    pub fn new(top: impl Into<String>) -> Self {
        Self {
            format: "HNL".to_string(),
            version: SCHEMA_VERSION.to_string(),
            level: Level::Generic,
            top: top.into(),
            modules: HashMap::new(),
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(s: &str) -> Result<Self, NetlistError> {
        let mut nl: Self = serde_json::from_str(s).map_err(NetlistError::Json)?;
        if nl.format != "HNL" {
            return Err(NetlistError::BadFormat(nl.format));
        }
        let file_major = nl.version.split('.').next().unwrap_or("0");
        let lib_major = SCHEMA_VERSION.split('.').next().unwrap_or("0");
        if file_major != lib_major {
            return Err(NetlistError::VersionMismatch {
                file: nl.version.clone(),
                lib: SCHEMA_VERSION.to_string(),
            });
        }
        // Restore module names from map keys.
        for (name, module) in &mut nl.modules {
            module.name = name.clone();
        }
        Ok(nl)
    }

    pub fn stats(&self) -> NetlistStats {
        let mut cell_counts: HashMap<String, usize> = HashMap::new();
        let mut total_cells = 0;
        let mut total_nets = 0;
        for module in self.modules.values() {
            total_nets += module.nets.len();
            for inst in &module.instances {
                *cell_counts.entry(inst.cell_type.clone()).or_insert(0) += 1;
                total_cells += 1;
            }
        }
        NetlistStats { cell_counts, total_cells, total_nets }
    }

    pub fn validate(&self) -> ValidationReport {
        validate_netlist(self)
    }
}

// ---------------------------------------------------------------------------
// Stats + Error
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct NetlistStats {
    pub cell_counts: HashMap<String, usize>,
    pub total_cells: usize,
    pub total_nets: usize,
}

#[derive(Debug)]
pub enum NetlistError {
    Json(serde_json::Error),
    BadFormat(String),
    VersionMismatch { file: String, lib: String },
}

impl std::fmt::Display for NetlistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetlistError::Json(e) => write!(f, "JSON: {e}"),
            NetlistError::BadFormat(fmt) => write!(f, "not an HNL document (format={fmt:?})"),
            NetlistError::VersionMismatch { file, lib } => {
                write!(f, "HNL version mismatch: file={file}, lib={lib}")
            }
        }
    }
}

impl std::error::Error for NetlistError {}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct ValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationReport {
    pub fn ok(&self) -> bool { self.errors.is_empty() }
}

fn validate_netlist(nl: &Netlist) -> ValidationReport {
    let mut report = ValidationReport::default();

    // R1 — top exists.
    if !nl.modules.contains_key(&nl.top) {
        report.errors.push(format!("R1: top module {:?} not in modules", nl.top));
        return report;
    }

    for (mod_name, module) in &nl.modules {
        validate_module(mod_name, module, nl, &mut report);
    }

    // R11 — no transitive self-instantiation.
    for mod_name in nl.modules.keys() {
        check_no_self_inst(mod_name, nl, &mut report);
    }

    report
}

fn validate_module(
    mod_name: &str,
    module: &Module,
    nl: &Netlist,
    report: &mut ValidationReport,
) {
    use std::collections::HashSet;

    let port_names: HashSet<&str> = module.ports.iter().map(|p| p.name.as_str()).collect();
    let net_names: HashSet<&str> = module.nets.iter().map(|n| n.name.as_str()).collect();

    if port_names.len() != module.ports.len() {
        report.errors.push(format!("module {mod_name:?}: duplicate port names"));
    }
    if net_names.len() != module.nets.len() {
        report.errors.push(format!("module {mod_name:?}: duplicate net names"));
    }

    for inst in &module.instances {
        // Resolve: builtin or user module.
        let (inst_inputs, inst_pin_set, inst_widths): (Vec<&str>, HashSet<&str>, HashMap<&str, u32>) =
            if let Some(sig) = BUILTIN_CELL_TYPES.get(inst.cell_type.as_str()) {
                let pins: HashSet<&str> = sig
                    .inputs
                    .iter()
                    .chain(sig.outputs.iter())
                    .copied()
                    .collect();
                let widths: HashMap<&str, u32> = pins
                    .iter()
                    .map(|&p| (p, sig.width(p)))
                    .collect();
                (sig.inputs.to_vec(), pins, widths)
            } else if let Some(user_mod) = nl.modules.get(&inst.cell_type) {
                let ports: HashSet<&str> =
                    user_mod.ports.iter().map(|p| p.name.as_str()).collect();
                let inputs: Vec<&str> = user_mod
                    .ports
                    .iter()
                    .filter(|p| p.direction == Direction::Input)
                    .map(|p| p.name.as_str())
                    .collect();
                let widths: HashMap<&str, u32> =
                    user_mod.ports.iter().map(|p| (p.name.as_str(), p.width)).collect();
                (inputs, ports, widths)
            } else {
                // R2 — unknown cell type.
                report.errors.push(format!(
                    "R2: instance {mod_name}.{}: unknown cell type {:?}",
                    inst.name, inst.cell_type
                ));
                continue;
            };

        // R3 — every input pin connected.
        for in_pin in &inst_inputs {
            if !inst.connections.contains_key(*in_pin) {
                report.errors.push(format!(
                    "R3: instance {mod_name}.{}: input pin {:?} not connected",
                    inst.name, in_pin
                ));
            }
        }

        for (conn_pin, conn_slice) in &inst.connections {
            // R4 — connection key is a real pin.
            if !inst_pin_set.contains(conn_pin.as_str()) {
                report.errors.push(format!(
                    "R4: instance {mod_name}.{}: pin {:?} not declared on {:?}",
                    inst.name, conn_pin, inst.cell_type
                ));
                continue;
            }

            // R5 — width compatibility.
            let expected = inst_widths.get(conn_pin.as_str()).copied().unwrap_or(1);
            if conn_slice.width() != expected {
                report.errors.push(format!(
                    "R5: instance {mod_name}.{}.{conn_pin}: \
                     width {} != expected {expected}",
                    inst.name, conn_slice.width()
                ));
            }

            // R6 — net exists.
            let net_ok = net_names.contains(conn_slice.net.as_str())
                || port_names.contains(conn_slice.net.as_str());
            if !net_ok {
                report.errors.push(format!(
                    "R6: instance {mod_name}.{}.{conn_pin}: net {:?} not declared",
                    inst.name, conn_slice.net
                ));
                continue;
            }

            // R7 — bits in range.
            let target_width = module
                .net(&conn_slice.net)
                .map(|n| n.width)
                .or_else(|| module.port(&conn_slice.net).map(|p| p.width))
                .unwrap_or(0);
            for &bit in &conn_slice.bits {
                if bit >= target_width {
                    report.errors.push(format!(
                        "R7: instance {mod_name}.{}.{conn_pin}: \
                         bit {bit} out of range for {:?} (width {target_width})",
                        inst.name, conn_slice.net
                    ));
                }
            }
        }
    }
}

fn check_no_self_inst(start: &str, nl: &Netlist, report: &mut ValidationReport) {
    use std::collections::HashSet;
    let mut seen: HashSet<&str> = HashSet::new();
    let mut stack: Vec<&str> = vec![start];
    let mut first = true;

    while let Some(cur) = stack.pop() {
        if !first && cur == start {
            report.errors.push(format!(
                "R11: module {start:?} transitively instantiates itself"
            ));
            return;
        }
        first = false;
        if !seen.insert(cur) { continue; }
        if let Some(m) = nl.modules.get(cur) {
            for inst in &m.instances {
                stack.push(inst.cell_type.as_str());
            }
        }
    }
}

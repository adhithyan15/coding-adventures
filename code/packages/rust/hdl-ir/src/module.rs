//! Module-level HIR constructs.
//!
//! A `Module` is the fundamental unit of hardware description — analogous to
//! a class in software. It has a boundary (ports), internal state (nets,
//! variables), sub-circuits (instances), concurrent assignments, and
//! sequential processes.
//!
//! ## Key types
//!
//! ```text
//! Module
//!   ├── ports: Vec<Port>            — I/O pins (named, typed, directional)
//!   ├── nets: Vec<Net>              — internal wires/registers
//!   ├── parameters: Vec<Parameter> — compile-time generics
//!   ├── instances: Vec<Instance>   — sub-module instantiations
//!   ├── cont_assigns: Vec<ContAssign> — Verilog `assign` / VHDL signal<=
//!   └── processes: Vec<Process>    — `always`/`process` blocks
//!
//! Library
//!   └── modules: {name → Module}   — named collection for VHDL multi-library
//! ```
//!
//! ## Level
//!
//! Each Module carries a `Level` tag indicating how much structure has been
//! resolved: `Behavioral` (full HDL body), `Structural` (instances only,
//! no processes), or `Unknown`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::expr::Expr;
use crate::provenance::Provenance;
use crate::stmt::Stmt;
use crate::types::Ty;

// ---------------------------------------------------------------------------
// Direction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    In,
    Out,
    Inout,
}

// ---------------------------------------------------------------------------
// Port
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Port {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: Ty,
    pub direction: Direction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

// ---------------------------------------------------------------------------
// Net + NetKind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NetKind {
    #[default]
    Signal,
    Wire,
    Reg,
    Tri,
    Wand,
    Wor,
    Supply0,
    Supply1,
    ResolvedSignal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Net {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: Ty,
    #[serde(default)]
    pub kind: NetKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial: Option<Expr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

// ---------------------------------------------------------------------------
// Variable (process-local)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: Ty,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial: Option<Expr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

// ---------------------------------------------------------------------------
// Parameter (compile-time generic)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: Ty,
    pub default: Expr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

// ---------------------------------------------------------------------------
// Instance
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instance {
    pub name: String,
    pub module: String,
    #[serde(default)]
    pub connections: HashMap<String, Expr>,
    #[serde(default)]
    pub parameters: HashMap<String, Expr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

// ---------------------------------------------------------------------------
// ContAssign (concurrent / continuous assignment)
// ---------------------------------------------------------------------------

/// A concurrent assignment: `assign lhs = rhs;` in Verilog,
/// `lhs <= rhs;` at the architecture body level in VHDL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContAssign {
    pub target: Expr,
    pub rhs: Expr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

// ---------------------------------------------------------------------------
// Sensitivity list
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitivityKind {
    Posedge,
    Negedge,
    Change,
    All, // `always @(*)` / `always_comb`
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensitivityItem {
    pub kind: SensitivityKind,
    pub expr: Expr,
}

// ---------------------------------------------------------------------------
// Process
// ---------------------------------------------------------------------------

/// A sequential block: `always`/`initial` in Verilog, `process` in VHDL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Process {
    #[serde(default)]
    pub sensitivity: Vec<SensitivityItem>,
    #[serde(default)]
    pub variables: Vec<Variable>,
    pub body: Vec<Stmt>,
    pub is_initial: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

// ---------------------------------------------------------------------------
// Level
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    #[default]
    Behavioral,
    Structural,
    Unknown,
}

// ---------------------------------------------------------------------------
// Module
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Module {
    pub name: String,
    #[serde(default)]
    pub ports: Vec<Port>,
    #[serde(default)]
    pub nets: Vec<Net>,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
    #[serde(default)]
    pub instances: Vec<Instance>,
    #[serde(default)]
    pub cont_assigns: Vec<ContAssign>,
    #[serde(default)]
    pub processes: Vec<Process>,
    #[serde(default)]
    pub level: Level,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

impl Module {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), ..Default::default() }
    }
}

// ---------------------------------------------------------------------------
// Library
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    pub modules: HashMap<String, Module>,
}

impl Library {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), modules: HashMap::new() }
    }
}

//! # HDL-IR — Hardware Intermediate Representation
//!
//! The HIR is the single canonical form that every front-end (Verilog, VHDL,
//! Ruby DSL) elaborates into, and that every back-end (simulation, synthesis,
//! FPGA, ASIC) consumes. Think of it as the "assembly language of hardware":
//! below the syntax wars of HDL dialects, above the raw gates of a netlist.
//!
//! ## Why a unified IR?
//!
//! Verilog and VHDL evolved independently over four decades. Their surface
//! syntax, type systems, and concurrency models differ substantially. But at
//! the level of *what a synthesiser actually needs to do*, the concepts are
//! nearly identical: ports, wires, concurrent assignments, sequential
//! processes, module instances. HIR captures that common semantic core.
//!
//! ## Structure at a glance
//!
//! ```text
//! HIR                          ← top-level document
//!   top: String                ← name of the root module
//!   modules: {name → Module}  ← all module definitions
//!   libraries: {name → Library}
//!
//! Module
//!   ports: Vec<Port>           ← I/O boundary
//!   nets: Vec<Net>             ← internal signals
//!   parameters: Vec<Parameter>
//!   instances: Vec<Instance>   ← sub-module instantiations
//!   cont_assigns: Vec<ContAssign>  ← concurrent assignments (Verilog `assign`)
//!   processes: Vec<Process>    ← sequential blocks (always/process)
//!
//! Expr                         ← expression tree (Lit, BinaryOp, NetRef, …)
//! Stmt                         ← statement tree (Assign, If, Case, Loop, …)
//! ```
//!
//! ## JSON round-trip
//!
//! Every HIR node implements `serde::Serialize` / `serde::Deserialize` with
//! a stable JSON schema (version `"0.1.0"`). The schema uses a `"kind"`
//! discriminator on expression and statement nodes, matching the Python
//! reference implementation.
//!
//! ## The 4-bit adder in HIR
//!
//! ```text
//! module adder4 {
//!   ports: [a:4 IN, b:4 IN, sum:5 OUT]
//!   cont_assigns: [
//!     ContAssign { target: PortRef("sum"), rhs: BinaryOp("+", PortRef("a"), PortRef("b")) }
//!   ]
//! }
//! ```

pub mod expr;
pub mod hir;
pub mod module;
pub mod provenance;
pub mod stmt;
pub mod types;
pub mod validate;

pub use expr::{Expr, BINARY_OPS, UNARY_OPS};
pub use hir::{Hir, HirStats};
pub use module::{
    ContAssign, Direction, Instance, Level, Library, Module, Net, NetKind, Parameter, Port,
    Process, SensitivityItem, SensitivityKind, Variable,
};
pub use provenance::{Provenance, SourceLang, SourceLocation};
pub use stmt::Stmt;
pub use types::Ty;
pub use validate::{validate, ValidationReport};

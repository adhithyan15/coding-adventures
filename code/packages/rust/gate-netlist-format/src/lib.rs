//! # Gate Netlist Format (HNL)
//!
//! The HNL (Hardware NetList) is the canonical netlist format that sits
//! *below* the HIR and *above* the physical layout. It is the language of
//! the synthesis pipeline:
//!
//! ```text
//! HIR  →  (synthesis)  →  HNL[GENERIC]  →  (tech-mapping)  →  HNL[STDCELL]
//!                                                                 │
//!                                         asic-floorplan  ◄───────┘
//!                                         asic-placement
//!                                         asic-routing
//!                                         gdsii-writer
//! ```
//!
//! ## Two levels
//!
//! A netlist can be at one of two levels:
//!
//! - **GENERIC** — instances name built-in cells (`AND2`, `OR2`, `DFF`, …).
//!   These are technology-independent primitives, the output of `synthesis`.
//! - **STDCELL** — instances name real library cells (`sky130_fd_sc_hd__and2_1`).
//!   The output of `tech-mapping`. These correspond 1-to-1 with physical
//!   standard cells in a silicon process.
//!
//! ## JSON schema (`format: "HNL"`, `version: "0.1.0"`)
//!
//! ```json
//! {
//!   "format": "HNL", "version": "0.1.0", "level": "generic", "top": "adder4",
//!   "modules": {
//!     "adder4": {
//!       "ports": [{"name":"a","dir":"input","width":4}, ...],
//!       "nets":  [{"name":"_n0","width":1}, ...],
//!       "instances": [{"name":"xor_0","type":"XOR2",
//!                      "connections":{"A":{"net":"a","bits":[0]},
//!                                     "B":{"net":"b","bits":[0]},
//!                                     "Y":{"net":"_n0","bits":[0]}}}]
//!     }
//!   }
//! }
//! ```

pub mod cells;
pub mod netlist;

pub use cells::{CellTypeSig, BUILTIN_CELL_TYPES};
pub use netlist::{
    Direction, Instance, Level, Module, Net, Netlist, NetlistError, NetlistStats, NetSlice, Port,
    ValidationReport,
};

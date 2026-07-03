//! # real-fpga-export
//!
//! Converts an HIR document to structural Verilog and drives the open-tool
//! iCE40 FPGA flow: yosys → nextpnr-ice40 → icepack → (optionally) iceprog.
//!
//! ## The pipeline at a glance
//!
//! ```text
//! HIR ──write_verilog_str()──► .v file
//!      │
//!      └──to_ice40()──────────► yosys (synthesise)
//!                                │
//!                                ▼
//!                              .json (technology-mapped netlist)
//!                                │
//!                         nextpnr-ice40 (place & route)
//!                                │
//!                                ▼
//!                              .asc (ASCII bitstream)
//!                                │
//!                            icepack (pack)
//!                                │
//!                                ▼
//!                              .bin (binary bitstream)
//!                                │
//!                         iceprog (flash to board)
//! ```
//!
//! ## Quick start
//!
//! ```rust
//! use real_fpga_export::{write_verilog_str, ToolchainOptions, to_ice40};
//!
//! // Given an `Hir` value:
//! // let verilog = write_verilog_str(&hir);
//! // println!("{}", verilog);
//! //
//! // Or run the full toolchain (requires yosys / nextpnr / icepack on PATH):
//! // let result = to_ice40(&hir, "top", None, std::path::Path::new("build"),
//! //                        "hx1k", "tq144", None, true).unwrap();
//! ```

pub mod toolchain;
pub mod verilog_writer;

pub use toolchain::{program_ice40, to_ice40, ToolchainOptions, ToolchainResult};
pub use verilog_writer::{write_verilog, write_verilog_str};

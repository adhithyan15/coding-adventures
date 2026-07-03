//! # Standard Cell Library
//!
//! Liberty-style NLDM (Non-Linear Delay Model) timing library for the
//! Sky130 HD teaching subset.
//!
//! ## Concepts
//!
//! A **lookup table** (LUT) represents cell delay or output transition time
//! as a 2-D grid indexed by *(input slew, output load)*:
//!
//! ```text
//! slew (ns)  0.01  0.05  0.10  0.20  0.50
//!              ┌─────────────────────────┐
//! load 0.50 fF │ 0.04  0.05  0.06  0.08 …│
//! load 1.00 fF │ 0.06  0.07  0.08  0.10 …│
//! load 2.00 fF │ …                        │
//! load 5.00 fF │ …                        │
//! load 10.0 fF │ …               0.20 …  │
//!              └─────────────────────────┘
//! ```
//!
//! Given actual (slew, load) values, bilinear interpolation produces a
//! realistic delay estimate. Values are in nanoseconds.
//!
//! ## Data provenance
//!
//! v0.1.0 ships **hand-curated** values tuned to within ~10% of Sky130
//! reference characterization. v0.2.0 will replace these with SPICE-driven
//! characterization runs.
//!
//! ## Usage
//!
//! ```rust
//! use standard_cell_library::{build_default_library, select_drive};
//!
//! let lib = build_default_library();
//! let cell = lib.get("sky130_fd_sc_hd__inv_1").unwrap();
//! let arc = &cell.timing_arcs[0];
//! let delay_ns = arc.cell_rise.lookup(0.05, 1.0);
//! assert!(delay_ns > 0.0);
//!
//! let best = select_drive(&lib, "sky130_fd_sc_hd__inv", 2.0, Some(0.10));
//! assert!(best.starts_with("sky130_fd_sc_hd__inv_"));
//! ```

pub mod data;
pub mod drive;
pub mod library;
pub mod lut;

pub use data::build_default_library;
pub use drive::select_drive;
pub use library::{CellTiming, Library, TimingArc};
pub use lut::LookupTable;

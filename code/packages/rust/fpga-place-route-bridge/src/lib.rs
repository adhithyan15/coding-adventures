//! # fpga-place-route-bridge
//!
//! Maps an HNL (generic-level gate netlist) to a FPGA JSON configuration
//! understood by downstream tools.
//!
//! ## What this crate does
//!
//! ```text
//! HNL[GENERIC]  ──hnl_to_fpga_json()──►  JSON config dict
//!                                         (clbs, routing, io, device)
//! ```
//!
//! Three things happen inside:
//!
//! 1. **LUT packing** — Each primitive cell (`AND2`, `OR2`, `NOT`, …) maps to
//!    a 4-input LUT.  The cell's truth table is expanded to 16 entries by
//!    ignoring extra high-order inputs.
//!
//! 2. **Placement** — Cells are placed in row-major order on a fabric grid.
//!    The grid size is controlled by `FpgaBridgeOptions`.
//!
//! 3. **Routing** — Each connection from a net source to a LUT input becomes
//!    one route entry in the JSON.
//!
//! ## Truth tables
//!
//! The `TRUTH_TABLE_TYPES` slice lists all built-in cell types for which a
//! truth table is defined.  Call `truth_table(cell_type)` to look up a
//! specific entry at zero cost (match on a `&str`).
//!
//! ```text
//! AND2 truth table (2-input AND):
//!
//!  A  B | Y        (bit 0 = A, bit 1 = B, combo = B<<1 | A)
//! ------+--
//!  0  0 | 0   combo 0 → table[0] = 0
//!  1  0 | 0   combo 1 → table[1] = 0
//!  0  1 | 0   combo 2 → table[2] = 0
//!  1  1 | 1   combo 3 → table[3] = 1
//! ```
//!
//! Expanded to 4-input LUT (16 entries): the upper two input bits are ignored,
//! so each 2-input entry repeats 4 times: `[0,0,0,1, 0,0,0,1, 0,0,0,1, 0,0,0,1]`.

pub mod bridge;

pub use bridge::{
    hnl_to_fpga_json, truth_table, truth_table_types,
    FpgaBridgeOptions, FpgaBridgeReport,
};

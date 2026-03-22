//! # FPGA — Field-Programmable Gate Array simulation.
//!
//! An FPGA (Field-Programmable Gate Array) is a chip containing:
//! - A grid of CLBs (Configurable Logic Blocks) for computation
//! - A routing fabric (switch matrices) for interconnection
//! - I/O blocks at the perimeter for external connections
//! - Block RAM tiles for on-chip memory
//!
//! The key property: **all of this is programmable**. By loading a
//! bitstream (configuration data), the same physical chip can become
//! any digital circuit — a CPU, a signal processor, a network switch,
//! or anything else that fits within its resources.
//!
//! ## Module hierarchy
//!
//! ```text
//! LUT            -- K-input look-up table (the atom of programmable logic)
//!     |
//! Slice          -- 2 LUTs + 2 flip-flops + carry chain
//!     |
//! CLB            -- 2 slices (the core compute tile)
//!     |
//! SwitchMatrix   -- programmable routing crossbar
//! IOBlock        -- bidirectional I/O pad
//!     |
//! Bitstream      -- configuration data (JSON-based)
//!     |
//! FPGA           -- top-level fabric combining all elements
//! ```

pub mod bitstream;
pub mod clb;
pub mod fabric;
pub mod io_block;
pub mod lut;
pub mod slice;
pub mod switch_matrix;

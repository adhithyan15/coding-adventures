//! # Block RAM — SRAM cells, arrays, and configurable RAM modules.
//!
//! This crate models memory at the gate level, building up from individual
//! SRAM cells to complete RAM modules suitable for use in FPGA simulation.
//!
//! The hierarchy follows real hardware:
//!
//! ```text
//! SRAMCell      — 1-bit storage (cross-coupled inverters + access transistors)
//!     |
//! SRAMArray     — 2D grid of cells with row/column addressing
//!     |
//! SinglePortRAM — synchronous RAM with one read/write port
//! DualPortRAM   — synchronous RAM with two independent ports
//!     |
//! ConfigurableBRAM — FPGA Block RAM with reconfigurable aspect ratio
//! ```
//!
//! ## What is SRAM?
//!
//! SRAM (Static Random-Access Memory) is the fastest type of memory in a
//! computer. It is used for CPU caches (L1/L2/L3), register files, and FPGA
//! Block RAM. "Static" means the memory holds its value as long as power is
//! supplied — unlike DRAM, which must be periodically refreshed.

pub mod bram;
pub mod ram;
pub mod sram;

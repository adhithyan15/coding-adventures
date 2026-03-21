//! # RISC-V RV32I Simulator with M-mode privileged extensions
//!
//! Implements all 37 RV32I instructions plus M-mode CSR access, trap handling,
//! and mret. Built on the cpu-simulator crate for RegisterFile and Memory.

pub mod opcodes;
pub mod csr;
pub mod decode;
pub mod execute;
pub mod encoding;
pub mod simulator;
pub mod core_adapter;

pub use csr::CSRFile;
pub use core_adapter::RiscVISADecoder;
pub use simulator::RiscVSimulator;

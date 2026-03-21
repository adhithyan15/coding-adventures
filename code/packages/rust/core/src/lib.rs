//! # Core -- Configurable processor core integrating all D-series components
//!
//! This crate integrates all D-series micro-architectural components into a
//! complete processor core:
//!
//!   - Pipeline (D04): moves instructions through stages (IF, ID, EX, MEM, WB)
//!   - Branch Predictor (D02): guesses which way branches will go
//!   - Hazard Detection (D03): detects data, control, and structural hazards
//!   - Cache Hierarchy (D01): L1I, L1D, optional L2 for fast memory access
//!   - Register File: fast storage for operands and results
//!   - Clock: drives everything in lockstep
//!
//! ## Modules
//! - `config` - `CoreConfig`, presets, `MultiCoreConfig`
//! - `register_file` - `RegisterFile` with configurable width and zero register
//! - `decoder` - `ISADecoder` trait, `MockDecoder` for testing
//! - `memory_controller` - `MemoryController` for shared main memory
//! - `interrupt_controller` - `InterruptController` for interrupt routing
//! - `stats` - `CoreStats` for aggregate performance metrics
//! - `core` - `Core` struct with Step/Run execution
//! - `multi_core` - `MultiCoreCPU` for multi-core simulation
//!
//! ## Quick start
//! ```
//! use core::{Core, simple_config, MockDecoder, encode_addi, encode_halt, encode_program};
//!
//! let config = simple_config();
//! let decoder = Box::new(MockDecoder::new());
//! let mut c = Core::new(config, decoder).unwrap();
//! let program = encode_program(&[encode_addi(1, 0, 42), encode_halt()]);
//! c.load_program(&program, 0);
//! let stats = c.run(100);
//! assert!(c.is_halted());
//! assert!(stats.ipc() > 0.0);
//! ```

pub mod config;
pub mod core;
pub mod decoder;
pub mod interrupt_controller;
pub mod memory_controller;
pub mod multi_core;
pub mod register_file;
pub mod stats;

// Re-export the main types at the crate root for convenient access.
pub use config::{
    CoreConfig, FPUnitConfig, MultiCoreConfig, RegisterFileConfig,
    cortex_a78_like_config, create_branch_predictor, simple_config,
};
pub use core::Core;
pub use decoder::{
    ISADecoder, MockDecoder, encode_add, encode_addi, encode_branch, encode_halt, encode_load,
    encode_nop, encode_program, encode_store, encode_sub,
};
pub use interrupt_controller::{AcknowledgedInterrupt, InterruptController, PendingInterrupt};
pub use memory_controller::{MemoryController, MemoryReadResult};
pub use multi_core::MultiCoreCPU;
pub use register_file::RegisterFile;
pub use stats::CoreStats;

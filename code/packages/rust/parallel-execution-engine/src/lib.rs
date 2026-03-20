//! # Parallel Execution Engine -- orchestrating thousands of cores in parallel.
//!
//! This crate implements **Layer 8 of the accelerator computing stack** -- the
//! parallel execution engine that sits between individual processing elements
//! (Layer 9, `gpu-core`) and the compute unit (Layer 7, future `sm-simulator`).
//!
//! This is where parallelism happens. Layer 9 gave us a single core that executes
//! one instruction at a time. Layer 8 takes many of those cores and orchestrates
//! them to execute in parallel -- but the *way* they're orchestrated differs
//! fundamentally across architectures.
//!
//! ## Layer Position
//!
//! ```text
//! Layer 11: Logic Gates (AND, OR, XOR, NAND)
//!     |
//! Layer 10: FP Arithmetic (IEEE 754 add/mul/fma)
//!     |
//! Layer 9:  Accelerator Core (gpu-core) -- one core, one instruction at a time
//!     |
//! Layer 8:  Parallel Execution Engine  <-- YOU ARE HERE
//!     |
//!     +-->  GPU (NVIDIA): WarpEngine      -- 32 threads, SIMT, divergence masks
//!     +-->  GPU (AMD):    WavefrontEngine  -- 32/64 lanes, SIMD, lane masking
//!     +-->  GPU (Intel):  SubsliceEngine   -- SIMD8 x EU threads, thread arbitration
//!     +-->  TPU (Google): SystolicArray    -- NxN PE grid, dataflow, no instruction fetch
//!     +-->  NPU (Apple):  MACArrayEngine   -- scheduled MAC array, compiler-driven
//! ```
//!
//! ## Execution Models
//!
//! Despite radical hardware differences, all engines share a common interface
//! defined by the [`ParallelExecutionEngine`] trait. Each engine implements a
//! different parallel execution model:
//!
//! | Engine            | Model         | Architecture   | Key Feature              |
//! |-------------------|---------------|----------------|--------------------------|
//! | `WarpEngine`      | SIMT          | NVIDIA/ARM     | Hardware divergence mgmt |
//! | `WavefrontEngine` | SIMD          | AMD GCN/RDNA   | Explicit EXEC mask       |
//! | `SubsliceEngine`  | SIMD+MT       | Intel Xe       | Thread arbitration       |
//! | `SystolicArray`   | Dataflow      | Google TPU     | No instructions at all   |
//! | `MACArrayEngine`  | Scheduled MAC | Apple ANE      | Compiler-driven schedule |
//!
//! ## Quick Start
//!
//! ```
//! use gpu_core::opcodes::{limm, fmul, halt};
//! use parallel_execution_engine::warp_engine::{WarpEngine, WarpConfig};
//! use parallel_execution_engine::protocols::ParallelExecutionEngine;
//!
//! // Create a 4-thread SIMT warp
//! let mut config = WarpConfig::default();
//! config.warp_width = 4;
//! let mut engine = WarpEngine::new(config);
//!
//! // Load a program: R2 = 2.0 * 3.0
//! engine.load_program(vec![
//!     limm(0, 2.0),
//!     limm(1, 3.0),
//!     fmul(2, 0, 1),
//!     halt(),
//! ]);
//!
//! // Run all threads
//! let traces = engine.run(1000).unwrap();
//!
//! // All 4 threads computed the same result
//! assert_eq!(engine.threads()[0].core.registers.read_float(2), 6.0);
//! assert!(engine.halted());
//! ```

pub mod protocols;
pub mod warp_engine;
pub mod wavefront_engine;
pub mod systolic_array;
pub mod mac_array_engine;
pub mod subslice_engine;

// Re-export the most commonly used types for convenience.
pub use protocols::{
    ParallelExecutionEngine, ExecutionModel, EngineTrace,
    DivergenceInfo, DataflowInfo,
};
pub use warp_engine::{WarpEngine, WarpConfig};
pub use wavefront_engine::{WavefrontEngine, WavefrontConfig};
pub use systolic_array::{SystolicArray, SystolicConfig};
pub use mac_array_engine::{MACArrayEngine, MACArrayConfig};
pub use subslice_engine::{SubsliceEngine, SubsliceConfig};

//! # Compute Unit -- the factory floor of accelerator computing.
//!
//! This crate implements **Layer 7 of the accelerator computing stack** -- the
//! compute unit that manages multiple parallel execution engines, schedules work
//! across them, and provides shared resources (memory, caches, register files).
//!
//! Just as the CPU Core composes a pipeline, branch predictor, caches, and
//! register file into a working processor, the Compute Unit composes execution
//! engines, schedulers, shared memory, and caches into a working accelerator
//! compute unit. **It's composition, not new logic** -- the intelligence is in
//! how the existing pieces are wired together.
//!
//! ## Layer Position
//!
//! ```text
//! Layer 11: Logic Gates
//!     |
//! Layer 10: FP Arithmetic (shared)
//!     |
//! Layer 9:  Accelerator Core (gpu-core)
//!     |
//! Layer 8:  Parallel Execution Engine (parallel-execution-engine)
//!     |     +-- WarpEngine, WavefrontEngine, SystolicArray,
//!     |     |   MACArrayEngine, SubsliceEngine
//!     |
//! Layer 7:  Compute Unit  <-- YOU ARE HERE
//!     |     +-- StreamingMultiprocessor (NVIDIA SM)
//!     |     +-- AMDComputeUnit (AMD CU)
//!     |     +-- MatrixMultiplyUnit (Google TPU MXU)
//!     |     +-- XeCore (Intel)
//!     |     +-- NeuralEngineCore (Apple ANE)
//!     |
//! Layer 6:  Device Simulator -- future (full GPU/TPU/NPU)
//! ```
//!
//! ## The Assembly Line Analogy
//!
//! If a single GPU core (Layer 9) is one worker at a desk, and a warp/wavefront
//! (Layer 8) is a team of 32 workers doing the same task on different data, then
//! the compute unit (Layer 7) is **the factory floor**:
//!
//! - **Workers** = execution engines (warps, wavefronts, systolic arrays)
//! - **Floor manager** = warp/wavefront scheduler
//! - **Shared toolbox** = shared memory / LDS (data accessible to all teams)
//! - **Supply closet** = L1 cache (recent data kept nearby)
//! - **Filing cabinets** = register file (massive, partitioned among teams)
//! - **Work orders** = thread blocks / work groups queued for execution
//!
//! ## Quick Start
//!
//! ```
//! use gpu_core::opcodes::{limm, fmul, halt};
//! use compute_unit::protocols::{WorkItem, ComputeUnit as ComputeUnitTrait};
//! use compute_unit::streaming_multiprocessor::{StreamingMultiprocessor, SMConfig};
//!
//! // Create an SM with a small config for testing
//! let mut config = SMConfig::default();
//! config.max_warps = 8;
//! let mut sm = StreamingMultiprocessor::new(config);
//!
//! // Dispatch a thread block: R2 = 2.0 * 3.0
//! let work = WorkItem {
//!     work_id: 0,
//!     program: Some(vec![limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()]),
//!     thread_count: 64,
//!     ..WorkItem::default()
//! };
//! sm.dispatch(work).unwrap();
//!
//! // Run until complete
//! let traces = sm.run(10000);
//! assert!(!traces.is_empty());
//! ```

pub mod protocols;
pub mod streaming_multiprocessor;
pub mod amd_compute_unit;
pub mod matrix_multiply_unit;
pub mod xe_core;
pub mod neural_engine_core;

// Re-export the most commonly used types for convenience.
pub use protocols::{
    Architecture, ComputeUnit, ComputeUnitTrace, ResourceError,
    SchedulingPolicy, SharedMemory, WarpState, WorkItem,
};
pub use streaming_multiprocessor::{StreamingMultiprocessor, SMConfig};
pub use amd_compute_unit::{AMDComputeUnit, AMDCUConfig};
pub use matrix_multiply_unit::{MatrixMultiplyUnit, MXUConfig};
pub use xe_core::{XeCore, XeCoreConfig};
pub use neural_engine_core::{NeuralEngineCore, ANECoreConfig};

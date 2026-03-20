//! # Device Simulator -- complete accelerator device simulators.
//!
//! This crate implements **Layer 6 of the accelerator computing stack** -- full
//! device simulators that combine multiple compute units (Layer 7), global memory,
//! caches, and work distributors into complete accelerator models.
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
//! Layer 8:  Parallel Execution Engine
//!     |
//! Layer 7:  Compute Unit
//!     |
//! Layer 6:  Device Simulator  <-- YOU ARE HERE
//!     |     +-- NvidiaGPU (NVIDIA GPU with GigaThread Engine)
//!     |     +-- AmdGPU (AMD GPU with Shader Engines)
//!     |     +-- GoogleTPU (Google TPU with Scalar/MXU/Vector pipeline)
//!     |     +-- IntelGPU (Intel GPU with Xe-Slices)
//!     |     +-- AppleANE (Apple Neural Engine with unified memory)
//!     |
//! Layer 5:  ISA Simulator -- future
//! ```
//!
//! ## The Factory Complex Analogy
//!
//! If a single compute unit (Layer 7) is one factory floor, then the device
//! simulator (Layer 6) is **the entire factory complex**:
//!
//! - **Factory floors** = compute units (SMs, CUs, MXUs, Xe-Cores, ANE Cores)
//! - **Warehouse** = global memory (VRAM/HBM, shared by all floors)
//! - **Loading dock** = host interface (PCIe/NVLink to CPU)
//! - **Floor manager's office** = work distributor (assigns work to floors)
//! - **Express lane between floors** = L2 cache (shared, faster than warehouse)
//!
//! ## Five Device Types
//!
//! Each device type models a real accelerator architecture:
//!
//! | Device   | Distributor       | Memory     | Execution Model    |
//! |----------|-------------------|------------|--------------------|
//! | NVIDIA   | GigaThread Engine | HBM3       | SIMT thread blocks |
//! | AMD      | Command Processor | GDDR6      | SIMD wavefronts    |
//! | Google   | TPU Sequencer     | HBM2e      | Systolic pipeline  |
//! | Intel    | Command Streamer  | GDDR6      | SIMD + threads     |
//! | Apple    | Schedule Replayer | Unified    | Compiler-driven    |
//!
//! ## Quick Start
//!
//! ```
//! use device_simulator::nvidia_gpu::NvidiaGPU;
//! use device_simulator::protocols::{AcceleratorDevice, KernelDescriptor};
//! use gpu_core::opcodes::{limm, halt};
//!
//! // Create a small NVIDIA GPU for testing
//! let mut gpu = NvidiaGPU::new(None, 4);
//!
//! // Launch a simple kernel
//! let mut kernel = KernelDescriptor::default();
//! kernel.name = "test".to_string();
//! kernel.program = Some(vec![limm(0, 42.0), halt()]);
//! kernel.grid_dim = (2, 1, 1);
//! kernel.block_dim = (32, 1, 1);
//! gpu.launch_kernel(kernel);
//!
//! // Run to completion
//! let traces = gpu.run(2000);
//! assert!(!traces.is_empty());
//! assert!(gpu.idle());
//! ```

pub mod protocols;
pub mod global_memory;
pub mod work_distributor;
pub mod nvidia_gpu;
pub mod amd_gpu;
pub mod google_tpu;
pub mod intel_gpu;
pub mod apple_ane;

// Re-export the most commonly used types for convenience.
pub use protocols::{
    AcceleratorDevice, DeviceConfig, DeviceStats, DeviceTrace,
    KernelDescriptor, MemoryTransaction,
};
pub use global_memory::{GlobalMemoryStats, SimpleGlobalMemory};
pub use nvidia_gpu::NvidiaGPU;
pub use amd_gpu::AmdGPU;
pub use google_tpu::GoogleTPU;
pub use intel_gpu::IntelGPU;
pub use apple_ane::AppleANE;

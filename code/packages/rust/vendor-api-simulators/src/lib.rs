//! # Vendor API Simulators -- Layer 3 of the accelerator computing stack.
//!
//! This crate implements six vendor GPU API simulators, each a thin wrapper
//! over the Vulkan-inspired compute runtime (Layer 5). Think of Layer 5 as
//! the **GPU driver internals** and Layer 3 as the **API the programmer sees**.
//!
//! ## The Six Simulators
//!
//! | Simulator  | Paradigm                | Module    |
//! |------------|-------------------------|-----------|
//! | **CUDA**   | Implicit, NVIDIA-only   | `cuda`    |
//! | **OpenCL** | Portable, event-driven  | `opencl`  |
//! | **Metal**  | Apple, encoder model    | `metal`   |
//! | **Vulkan** | Ultra-explicit          | `vulkan`  |
//! | **WebGPU** | Safe, browser-first     | `webgpu`  |
//! | **OpenGL** | Legacy state machine    | `opengl`  |
//!
//! ## Architecture
//!
//! All six simulators share a common `BaseSimulator` that performs device
//! discovery and setup via the compute-runtime:
//!
//! ```text
//!  CUDARuntime / CLContext / MTLDevice / VkInstance / GPU / GLContext
//!            \       |        |        |       /       /
//!             +------+--------+--------+------+------+
//!                            |
//!                     BaseSimulator
//!                            |
//!                    RuntimeInstance  (Layer 5)
//!                            |
//!                  DeviceSimulator    (Layer 6)
//! ```
//!
//! Each simulator translates vendor-specific API calls into the common
//! Layer 5 operations (memory allocation, command recording, queue
//! submission, synchronization).

pub mod base;
pub mod cuda;
pub mod opencl;
pub mod metal;
pub mod vulkan;
pub mod webgpu;
pub mod opengl;

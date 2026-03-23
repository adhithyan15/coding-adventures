//! Backend module declarations.
//!
//! This module re-exports all seven BLAS backends:
//!
//! - [`cpu::CpuBlas`]       -- pure Rust reference implementation
//! - [`cuda::CudaBlas`]     -- NVIDIA CUDA GPU backend
//! - [`metal::MetalBlas`]   -- Apple Metal GPU backend
//! - [`opencl::OpenClBlas`] -- portable OpenCL GPU backend
//! - [`vulkan::VulkanBlas`] -- explicit Vulkan GPU backend
//! - [`webgpu::WebGpuBlas`] -- browser-friendly WebGPU backend
//! - [`opengl::OpenGlBlas`] -- legacy OpenGL compute backend

pub mod cpu;
pub mod gpu_base;

pub mod cuda;
pub mod metal;
pub mod opencl;
pub mod opengl;
pub mod vulkan;
pub mod webgpu;

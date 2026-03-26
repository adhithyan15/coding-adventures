//! # BLAS Library -- Pluggable Linear Algebra with 7 Swappable Backends
//!
//! This crate implements a BLAS (Basic Linear Algebra Subprograms) library
//! with seven interchangeable backends:
//!
//! | Backend    | Hardware          | Module                      |
//! |------------|-------------------|-----------------------------|
//! | **CPU**    | Any CPU           | [`backends::cpu::CpuBlas`]  |
//! | **CUDA**   | NVIDIA GPUs       | [`backends::cuda`]          |
//! | **Metal**  | Apple Silicon     | [`backends::metal`]         |
//! | **OpenCL** | Any GPU/CPU/FPGA  | [`backends::opencl`]        |
//! | **Vulkan** | Cross-platform    | [`backends::vulkan`]        |
//! | **WebGPU** | Browsers          | [`backends::webgpu`]        |
//! | **OpenGL** | Legacy GPUs       | [`backends::opengl`]        |
//!
//! # Architecture
//!
//! ```text
//!                          BlasBackend (trait)
//!                         /       |        \
//!                    CpuBlas  GpuBlasWrapper<T>  (your custom backend)
//!                             /   |   |   \
//!                         CUDA Metal OpenCL ...
//! ```
//!
//! Every backend implements the [`BlasBackend`](traits::BlasBackend) trait,
//! which covers BLAS Levels 1-3. The CPU backend additionally implements
//! [`MlBlasBackend`](traits::MlBlasBackend) for ML extensions (ReLU, GELU,
//! softmax, attention, etc.).
//!
//! # Quick Start
//!
//! ```
//! use blas_library::{CpuBlas, Vector, Matrix, Transpose};
//! use blas_library::traits::BlasBackend;
//!
//! let blas = CpuBlas;
//!
//! // Level 1: SAXPY
//! let x = Vector::new(vec![1.0, 2.0, 3.0]);
//! let y = Vector::new(vec![4.0, 5.0, 6.0]);
//! let result = blas.saxpy(2.0, &x, &y).unwrap();
//! assert_eq!(result.data(), &[6.0, 9.0, 12.0]);
//!
//! // Level 3: GEMM (matrix multiply)
//! let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
//! let b = Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2);
//! let c = Matrix::zeros(2, 2);
//! let result = blas.sgemm(
//!     Transpose::NoTrans, Transpose::NoTrans,
//!     1.0, &a, &b, 0.0, &c,
//! ).unwrap();
//! assert_eq!(result.data(), &[19.0, 22.0, 43.0, 50.0]);
//! ```
//!
//! # Backend Selection with the Registry
//!
//! ```
//! use blas_library::BackendRegistry;
//!
//! let registry = BackendRegistry::with_defaults();
//!
//! // Auto-detect the best available backend
//! let best = registry.get_best().unwrap();
//! println!("Using backend: {} ({})", best.name(), best.device_name());
//!
//! // Or request a specific one
//! let cpu = registry.get("cpu").unwrap();
//! ```

pub mod backends;
pub mod registry;
pub mod traits;
pub mod types;

// Re-export the most commonly used types at the crate root.
pub use backends::cpu::CpuBlas;
pub use backends::cuda::CudaBlas;
pub use backends::gpu_base::{GpuBlasBackend, GpuBlasWrapper};
pub use backends::metal::MetalBlas;
pub use backends::opencl::OpenClBlas;
pub use backends::opengl::OpenGlBlas;
pub use backends::vulkan::VulkanBlas;
pub use backends::webgpu::WebGpuBlas;
pub use registry::BackendRegistry;
pub use types::{Matrix, Side, StorageOrder, Transpose, Vector};

# frozen_string_literal: true

# ---------------------------------------------------------------------------
# BLAS Library -- seven BLAS backends over simulated GPU hardware.
# ---------------------------------------------------------------------------
#
# === What is This Package? ===
#
# This is a complete BLAS (Basic Linear Algebra Subprograms) library with
# seven interchangeable backends:
#
#     1. CPU     -- pure Ruby reference implementation (always available)
#     2. CUDA    -- NVIDIA GPU via CUDARuntime simulator
#     3. Metal   -- Apple Silicon via MTLDevice simulator
#     4. OpenCL  -- cross-platform via CLContext simulator
#     5. Vulkan  -- explicit control via VkDevice simulator
#     6. WebGPU  -- browser-safe via GPUDevice simulator
#     7. OpenGL  -- legacy state machine via GLContext simulator
#
# All seven backends produce IDENTICAL results because the actual arithmetic
# is performed by the CPU reference. The GPU backends exercise the full GPU
# memory pipeline (allocate, upload, compute, download, free) using the
# vendor API simulators from Layer 4.
#
# === Quick Start ===
#
#     require "coding_adventures_blas_library"
#     include CodingAdventures::BlasLibrary
#
#     blas = create_blas("cpu")     # Or "cuda", "metal", "opencl", etc.
#     x = Vector.new(data: [1.0, 2.0, 3.0], size: 3)
#     y = Vector.new(data: [4.0, 5.0, 6.0], size: 3)
#     result = blas.saxpy(2.0, x, y)   # => [6.0, 9.0, 12.0]
#
# === Backend Selection ===
#
#     blas = create_blas("auto")   # Best available (cuda > metal > ... > cpu)
#     blas = create_blas("cuda")   # Specific backend
#
# === Architecture ===
#
#     Layer 6 (this package):  BLAS operations (saxpy, sgemm, softmax, ...)
#     Layer 5:                 Vendor API simulators (CUDA, Metal, etc.)
#     Layer 4:                 Compute runtime (Vulkan-inspired)
#     Layers 1-3:              Hardware simulation (logic gates -> ALU -> GPU)

require "coding_adventures_vendor_api_simulators"

require_relative "coding_adventures/blas_library/version"
require_relative "coding_adventures/blas_library/types"
require_relative "coding_adventures/blas_library/backends/cpu"
require_relative "coding_adventures/blas_library/backends/gpu_base"
require_relative "coding_adventures/blas_library/backends/cuda"
require_relative "coding_adventures/blas_library/backends/metal"
require_relative "coding_adventures/blas_library/backends/opencl"
require_relative "coding_adventures/blas_library/backends/opengl"
require_relative "coding_adventures/blas_library/backends/vulkan"
require_relative "coding_adventures/blas_library/backends/webgpu"
require_relative "coding_adventures/blas_library/registry"
require_relative "coding_adventures/blas_library/convenience"

# Register all backends in the global registry.
#
# This happens at require time so that create_blas("auto") can find them.
# Each backend is registered as a CLASS (not an instance) -- instantiation
# is deferred until get() or get_best() is called.
CodingAdventures::BlasLibrary::GLOBAL_REGISTRY.register(
  "cpu", CodingAdventures::BlasLibrary::Backends::CpuBlas
)
CodingAdventures::BlasLibrary::GLOBAL_REGISTRY.register(
  "cuda", CodingAdventures::BlasLibrary::Backends::CudaBlas
)
CodingAdventures::BlasLibrary::GLOBAL_REGISTRY.register(
  "metal", CodingAdventures::BlasLibrary::Backends::MetalBlas
)
CodingAdventures::BlasLibrary::GLOBAL_REGISTRY.register(
  "opencl", CodingAdventures::BlasLibrary::Backends::OpenClBlas
)
CodingAdventures::BlasLibrary::GLOBAL_REGISTRY.register(
  "opengl", CodingAdventures::BlasLibrary::Backends::OpenGlBlas
)
CodingAdventures::BlasLibrary::GLOBAL_REGISTRY.register(
  "vulkan", CodingAdventures::BlasLibrary::Backends::VulkanBlas
)
CodingAdventures::BlasLibrary::GLOBAL_REGISTRY.register(
  "webgpu", CodingAdventures::BlasLibrary::Backends::WebGpuBlas
)

# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Vendor API Simulators -- six GPU programming APIs over one runtime.
# ---------------------------------------------------------------------------
#
# This package provides six vendor API simulators, each wrapping the same
# Vulkan-inspired compute runtime (Layer 5) with different programming models:
#
#     CUDA    -- NVIDIA's implicit, "just launch it" model
#     OpenCL  -- Khronos cross-platform, event-based dependencies
#     Metal   -- Apple's unified memory, command encoder model
#     Vulkan  -- Ultra-explicit, maximum control
#     WebGPU  -- Safe, browser-first, single queue
#     OpenGL  -- Legacy global state machine
#
# === Quick Start ===
#
#     require "coding_adventures_vendor_api_simulators"
#     include CodingAdventures::VendorApiSimulators
#
#     # CUDA style (simplest)
#     cuda = CUDARuntime.new
#     d_x = cuda.malloc(256)
#     cuda.launch_kernel(kernel, grid: Dim3.new(x: 1), block: Dim3.new(x: 32), args: [d_x])
#     cuda.device_synchronize
#     cuda.free(d_x)
#
#     # Metal style (unified memory)
#     device = MTLDevice.new
#     buf = device.make_buffer(256)
#     buf.write_bytes(data)
#     result = buf.contents
#
#     # OpenGL style (state machine)
#     gl = GLContext.new
#     shader = gl.create_shader(GL_COMPUTE_SHADER)

require "set"
require "coding_adventures_compute_runtime"

require_relative "vendor_api_simulators/version"
require_relative "vendor_api_simulators/base"
require_relative "vendor_api_simulators/cuda"
require_relative "vendor_api_simulators/opencl"
require_relative "vendor_api_simulators/metal"
require_relative "vendor_api_simulators/vulkan_sim"
require_relative "vendor_api_simulators/webgpu"
require_relative "vendor_api_simulators/opengl"

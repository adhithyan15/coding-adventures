# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Device Simulator -- Layer 6 of the accelerator computing stack.
# ---------------------------------------------------------------------------
#
# This gem simulates complete accelerator devices, assembling multiple
# compute units (Layer 7) with global memory, L2 cache, and work distribution
# into full devices that can launch and execute kernels.
#
#     Layer 9:  gpu-core (one core, one instruction at a time)
#         |
#     Layer 8:  parallel-execution-engine (warps, wavefronts, systolic arrays)
#         |
#     Layer 7:  compute-unit (SM, CU, MXU, XeCore, ANECore)
#         |
#     Layer 6:  device-simulator (THIS PACKAGE)
#         |
#         +-- NvidiaGPU       -- many SMs + HBM + L2 + GigaThread
#         +-- AmdGPU          -- CUs in Shader Engines + Infinity Cache
#         +-- GoogleTPU       -- Scalar/Vector/MXU pipeline + HBM
#         +-- IntelGPU        -- Xe-Cores in Xe-Slices + L2
#         +-- AppleANE        -- NE cores + SRAM + DMA + unified memory
#
# Usage:
#   require "coding_adventures_device_simulator"
#   include CodingAdventures
#
#   gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 4)
#   gpu.launch_kernel(DeviceSimulator::KernelDescriptor.new(
#     name: "test",
#     program: [GpuCore.limm(0, 42.0), GpuCore.halt],
#     grid_dim: [2, 1, 1],
#     block_dim: [32, 1, 1],
#   ))
#   traces = gpu.run(1000)
#   puts "Completed in #{traces.length} cycles"

require "coding_adventures_gpu_core"
require "coding_adventures_fp_arithmetic"
require "coding_adventures_parallel_execution_engine"
require "coding_adventures_clock"
require "coding_adventures_compute_unit"
require "coding_adventures_cache"

require_relative "coding_adventures/device_simulator/version"
require_relative "coding_adventures/device_simulator/protocols"
require_relative "coding_adventures/device_simulator/global_memory"
require_relative "coding_adventures/device_simulator/work_distributor"
require_relative "coding_adventures/device_simulator/nvidia_gpu"
require_relative "coding_adventures/device_simulator/amd_gpu"
require_relative "coding_adventures/device_simulator/google_tpu"
require_relative "coding_adventures/device_simulator/intel_gpu"
require_relative "coding_adventures/device_simulator/apple_ane"

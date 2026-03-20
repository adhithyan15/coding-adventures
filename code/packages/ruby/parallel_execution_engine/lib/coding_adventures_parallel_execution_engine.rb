# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Parallel Execution Engine -- Layer 8 of the accelerator computing stack.
# ---------------------------------------------------------------------------
#
# This gem implements the parallel execution engine that sits between
# individual processing elements (Layer 9, gpu_core) and the compute unit
# (Layer 7, future sm_simulator).
#
# This is where parallelism happens. Layer 9 gave us a single core that
# executes one instruction at a time. Layer 8 takes many of those cores
# and orchestrates them to execute in parallel -- but the WAY they're
# orchestrated differs fundamentally across architectures.
#
# The dependency chain:
#   Logic Gates (AND, OR, NOT, XOR)
#     +-- Arithmetic (half_adder, full_adder, ripple_carry_adder)
#         +-- FP Arithmetic (IEEE 754 encoding, add, mul, fma)
#             +-- GPU Core (one core, one instruction at a time)
#                 +-- Parallel Execution Engine (this package)
#
# Five engines are provided:
#
#   WarpEngine       -- SIMT (NVIDIA CUDA / ARM Mali style)
#                       32 threads, hardware-managed divergence
#
#   WavefrontEngine  -- SIMD (AMD GCN/RDNA style)
#                       32/64 lanes, explicit EXEC mask
#
#   SystolicArray    -- Dataflow (Google TPU style)
#                       NxN PE grid, no instructions, just data flow
#
#   MACArrayEngine   -- Scheduled MAC (Apple NPU / Qualcomm style)
#                       Compiler-driven schedule, no runtime scheduler
#
#   SubsliceEngine   -- Hybrid SIMD (Intel Xe style)
#                       SIMD8 x EU threads, thread arbitration
#
# Usage:
#   require "coding_adventures_parallel_execution_engine"
#   include CodingAdventures
#
#   clock = Clock::ClockGenerator.new
#   engine = ParallelExecutionEngine::WarpEngine.new(
#     ParallelExecutionEngine::WarpConfig.new(warp_width: 4),
#     clock
#   )
#   engine.load_program([GpuCore.limm(0, 42.0), GpuCore.halt])
#   traces = engine.run
#   engine.threads[0].core.registers.read_float(0)  # => 42.0

require "coding_adventures_gpu_core"
require "coding_adventures_fp_arithmetic"

require_relative "coding_adventures/parallel_execution_engine/version"
require_relative "coding_adventures/parallel_execution_engine/protocols"
require_relative "coding_adventures/parallel_execution_engine/warp_engine"
require_relative "coding_adventures/parallel_execution_engine/wavefront_engine"
require_relative "coding_adventures/parallel_execution_engine/systolic_array"
require_relative "coding_adventures/parallel_execution_engine/mac_array_engine"
require_relative "coding_adventures/parallel_execution_engine/subslice_engine"

# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Compute Unit -- Layer 7 of the accelerator computing stack.
# ---------------------------------------------------------------------------
#
# This gem implements the compute unit that sits above execution engines
# (Layer 8, parallel-execution-engine) and below the device simulator
# (Layer 6, future).
#
# This is where the real architectural diversity shows up. At Layer 8, the
# execution engines were already different (SIMT vs SIMD vs systolic). At
# Layer 7, the differences multiply -- each architecture wraps those engines
# in very different organizational structures:
#
# Five compute units are provided:
#
#   StreamingMultiprocessor  -- NVIDIA SM
#                               4 warp schedulers, 48-64 warps, shared memory
#
#   AMDComputeUnit           -- AMD CU (GCN/RDNA)
#                               4 SIMD units, scalar unit, LDS
#
#   MatrixMultiplyUnit       -- Google TPU MXU
#                               Systolic array + vector unit, no threads
#
#   XeCore                   -- Intel Xe Core
#                               8-16 EUs with hardware threads, SLM
#
#   NeuralEngineCore         -- Apple ANE Core
#                               MAC array + DMA, compiler-scheduled
#
# The dependency chain:
#   Logic Gates (AND, OR, NOT, XOR)
#     +-- Arithmetic (half_adder, full_adder, ripple_carry_adder)
#         +-- FP Arithmetic (IEEE 754 encoding, add, mul, fma)
#             +-- GPU Core (one core, one instruction at a time)
#                 +-- Parallel Execution Engine (warps, wavefronts, etc.)
#                     +-- Compute Unit (this package)
#
# Usage:
#   require "coding_adventures_compute_unit"
#   include CodingAdventures
#
#   clock = Clock::ClockGenerator.new
#   sm = ComputeUnit::StreamingMultiprocessor.new(
#     ComputeUnit::SMConfig.new(max_warps: 8),
#     clock
#   )
#   sm.dispatch(ComputeUnit::WorkItem.new(
#     work_id: 0,
#     program: [GpuCore.limm(0, 2.0), GpuCore.limm(1, 3.0), GpuCore.fmul(2, 0, 1), GpuCore.halt],
#     thread_count: 64
#   ))
#   traces = sm.run
#   puts "Completed in #{traces.length} cycles, occupancy: #{sm.occupancy}"

require "coding_adventures_gpu_core"
require "coding_adventures_fp_arithmetic"
require "coding_adventures_parallel_execution_engine"
require "coding_adventures_clock"

require_relative "coding_adventures/compute_unit/version"
require_relative "coding_adventures/compute_unit/protocols"
require_relative "coding_adventures/compute_unit/streaming_multiprocessor"
require_relative "coding_adventures/compute_unit/amd_compute_unit"
require_relative "coding_adventures/compute_unit/matrix_multiply_unit"
require_relative "coding_adventures/compute_unit/xe_core"
require_relative "coding_adventures/compute_unit/neural_engine_core"

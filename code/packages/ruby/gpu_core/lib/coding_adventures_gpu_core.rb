# frozen_string_literal: true

# ---------------------------------------------------------------------------
# GPU Core -- a generic, pluggable GPU processing element simulator.
# ---------------------------------------------------------------------------
#
# This gem implements a single GPU core (processing element) that can simulate
# any vendor's GPU core by swapping out the instruction set (ISA). It's built
# on top of the fp_arithmetic gem which provides IEEE 754 floating-point
# operations at the bit level.
#
# The dependency chain:
#   Logic Gates (AND, OR, NOT, XOR)
#     +-- Arithmetic (half_adder, full_adder, ripple_carry_adder)
#         +-- FP Arithmetic (IEEE 754 encoding, add, mul, fma)
#             +-- GPU Core (this package)
#
# Architecture:
#   - FPRegisterFile: stores FloatBits values in configurable register count
#   - LocalMemory: byte-addressable scratchpad with FP load/store
#   - Instruction/Opcode: structured instruction representation (16 opcodes)
#   - GenericISA: default educational instruction set
#   - GPUCore: the fetch-execute loop that ties it all together
#   - GPUCoreTrace: execution trace records for debugging and education
#
# Usage:
#   require "coding_adventures_gpu_core"
#   include CodingAdventures
#
#   core = GpuCore::GPUCore.new
#   core.load_program([
#     GpuCore.limm(0, 3.0),
#     GpuCore.limm(1, 4.0),
#     GpuCore.fmul(2, 0, 1),
#     GpuCore.halt,
#   ])
#   traces = core.run
#   core.registers.read_float(2)  # => 12.0

require "coding_adventures_fp_arithmetic"

require_relative "coding_adventures/gpu_core/version"
require_relative "coding_adventures/gpu_core/opcodes"
require_relative "coding_adventures/gpu_core/execute_result"
require_relative "coding_adventures/gpu_core/registers"
require_relative "coding_adventures/gpu_core/memory"
require_relative "coding_adventures/gpu_core/trace"
require_relative "coding_adventures/gpu_core/generic_isa"
require_relative "coding_adventures/gpu_core/core"

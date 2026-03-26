# frozen_string_literal: true

# CPU Simulator — Layer 8 of the computing stack.
#
# Simulates the core of a processor: registers, memory, program counter,
# and the fetch-decode-execute cycle that drives all computation.
#
# This is a generic CPU model — not tied to any specific architecture.
# The ISA simulators (RISC-V, ARM, WASM, Intel 4004) build on top of this
# by providing their own instruction decoders and executors.

require_relative "coding_adventures/cpu_simulator/version"
require_relative "coding_adventures/cpu_simulator/simulator"
require_relative "coding_adventures/cpu_simulator/sparse_memory"

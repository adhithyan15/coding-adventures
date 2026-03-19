# frozen_string_literal: true

# Entry point for the coding_adventures_cpu_pipeline gem.
#
# This gem implements a configurable N-stage CPU instruction pipeline simulator.
# The pipeline moves "tokens" (representing instructions) through stages,
# handling normal advancement, stalls, flushes, and forwarding.
#
# The classic 5-stage pipeline:
#
#   IF (Instruction Fetch) -> ID (Instruction Decode) -> EX (Execute)
#     -> MEM (Memory Access) -> WB (Write Back)
#
# The pipeline is ISA-independent -- actual instruction semantics are provided
# via callback functions (fetch, decode, execute, memory, writeback).
#
# Usage:
#   require "coding_adventures_cpu_pipeline"
#
#   config = CodingAdventures::CpuPipeline.classic_5_stage
#   pipeline = CodingAdventures::CpuPipeline::Pipeline.new(
#     config: config,
#     fetch_fn: ->(pc) { instruction_memory[pc / 4] || 0 },
#     decode_fn: ->(raw, tok) { ... },
#     execute_fn: ->(tok) { ... },
#     memory_fn: ->(tok) { ... },
#     writeback_fn: ->(tok) { ... }
#   )
#   stats = pipeline.run(100)

require_relative "coding_adventures/cpu_pipeline/version"
require_relative "coding_adventures/cpu_pipeline/token"
require_relative "coding_adventures/cpu_pipeline/snapshot"
require_relative "coding_adventures/cpu_pipeline/pipeline"

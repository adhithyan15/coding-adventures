# frozen_string_literal: true

# Entry point for the coding_adventures_core gem.
#
# This gem integrates all D-series micro-architectural components into a
# complete processor core. It composes:
#
#   - Pipeline (D04): moves instructions through stages
#   - Branch Predictor (D02): guesses branch directions
#   - Hazard Detection (D03): detects data, control, and structural hazards
#   - Cache Hierarchy (D01): L1I/L1D/L2 for fast memory access
#   - Register File: fast storage for operands and results
#   - Clock: drives everything in lockstep
#
# The Core itself defines no new micro-architectural behavior. It wires the
# parts together, like a motherboard connects CPU, RAM, and peripherals.
#
# Modules:
#   RegisterFileConfig - Configuration for the register file
#   FPUnitConfig       - Configuration for the optional FP unit
#   CoreConfig         - Complete core configuration
#   MultiCoreConfig    - Multi-core processor configuration
#   RegisterFile       - General-purpose register file
#   MockDecoder        - Simple ISA decoder for testing
#   MemoryController   - Memory access serialization
#   InterruptController - Interrupt routing
#   CoreStats          - Aggregate performance statistics
#   Core               - Complete processor core
#   MultiCoreCPU       - Multi-core processor
#
# Usage:
#   require "coding_adventures_core"
#
#   config = CodingAdventures::Core.simple_config
#   decoder = CodingAdventures::Core::MockDecoder.new
#   core = CodingAdventures::Core::Core.new(config, decoder)

require "coding_adventures_cache"
require "coding_adventures_branch_predictor"
require "coding_adventures_cpu_pipeline"
require "coding_adventures_hazard_detection"
require "coding_adventures_clock"

require_relative "coding_adventures/core/version"
require_relative "coding_adventures/core/config"
require_relative "coding_adventures/core/register_file"
require_relative "coding_adventures/core/decoder"
require_relative "coding_adventures/core/memory_controller"
require_relative "coding_adventures/core/interrupt_controller"
require_relative "coding_adventures/core/stats"
require_relative "coding_adventures/core/core"
require_relative "coding_adventures/core/multi_core"

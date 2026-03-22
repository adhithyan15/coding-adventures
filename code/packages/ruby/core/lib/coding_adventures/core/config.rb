# frozen_string_literal: true

# CoreConfig and preset configurations for a processor core.
#
# = The Core: a Motherboard for Micro-Architecture
#
# A processor core is not a single piece of hardware. It is a composition of
# many sub-components, each independently designed and tested:
#
#   - Pipeline (D04): moves instructions through stages (IF, ID, EX, MEM, WB)
#   - Branch Predictor (D02): guesses which way branches will go
#   - Hazard Detection (D03): detects data, control, and structural hazards
#   - Cache Hierarchy (D01): L1I, L1D, optional L2 for fast memory access
#   - Register File: fast storage for operands and results
#   - Clock: drives everything in lockstep
#
# The Core itself defines no new micro-architectural behavior. It wires the
# parts together, like a motherboard connects CPU, RAM, and peripherals.
# The same Core can run ARM, RISC-V, or any custom ISA -- the ISA decoder
# is injected from outside.
#
# = Configuration
#
# Every parameter that a real CPU architect would tune is exposed in
# CoreConfig. Change the branch predictor and you get different accuracy.
# Double the L1 cache and you get fewer misses. Deepen the pipeline and
# you get higher clock speeds but worse misprediction penalties.

module CodingAdventures
  module Core
    # =========================================================================
    # RegisterFileConfig -- configuration for the register file
    # =========================================================================

    # RegisterFileConfig holds the parameters for the general-purpose register
    # file.
    #
    # Real-world register file sizes:
    #
    #   MIPS:     32 registers, 32-bit  (R0 hardwired to zero)
    #   ARMv8:    31 registers, 64-bit  (X0-X30, no zero register)
    #   RISC-V:   32 registers, 32/64-bit (x0 hardwired to zero)
    #   x86-64:   16 registers, 64-bit  (RAX, RBX, ..., R15)
    #
    # The zero_register convention (RISC-V, MIPS) simplifies instruction
    # encoding: any instruction can discard its result by writing to R0, and
    # any instruction can use "zero" as an operand without a special immediate
    # encoding.
    class RegisterFileConfig
      # @return [Integer] Number of general-purpose registers (typical: 16 or 32).
      attr_reader :count

      # @return [Integer] Bit width of each register (32 or 64).
      attr_reader :width

      # @return [Boolean] Whether register 0 is hardwired to zero (RISC-V/MIPS).
      attr_reader :zero_register

      def initialize(count: 16, width: 32, zero_register: true)
        @count = count
        @width = width
        @zero_register = zero_register
      end
    end

    # Returns a sensible default: 16 registers, 32-bit, with R0 hardwired to
    # zero (RISC-V convention).
    def self.default_register_file_config
      RegisterFileConfig.new(count: 16, width: 32, zero_register: true)
    end

    # =========================================================================
    # FPUnitConfig -- configuration for the floating-point unit
    # =========================================================================

    # FPUnitConfig configures the optional floating-point unit.
    #
    # Not all cores have an FP unit. Microcontrollers (ARM Cortex-M0) and
    # efficiency cores often omit it to save area and power. When fp_unit is
    # nil in CoreConfig, the core has no floating-point support.
    class FPUnitConfig
      # @return [Array<String>] Supported FP formats: "fp16", "fp32", "fp64".
      attr_reader :formats

      # @return [Integer] How many cycles an FP operation takes (typical: 3-5).
      attr_reader :pipeline_depth

      def initialize(formats: [], pipeline_depth: 4)
        @formats = formats
        @pipeline_depth = pipeline_depth
      end
    end

    # =========================================================================
    # CoreConfig -- complete configuration for a processor core
    # =========================================================================

    # CoreConfig holds every tunable parameter for a processor core.
    #
    # This is the "spec sheet" for the core. A CPU architect decides these
    # values based on the target workload, power budget, and die area.
    #
    # Changing any parameter affects measurable performance:
    #
    #   Deeper pipeline         -> higher clock speed, worse misprediction penalty
    #   Better branch predictor -> fewer pipeline flushes
    #   Larger L1 cache         -> fewer cache misses
    #   More registers          -> fewer spills to memory
    #   Forwarding enabled      -> fewer stall cycles
    class CoreConfig
      # @return [String] Human-readable identifier for this configuration.
      attr_accessor :name

      # @return [CodingAdventures::CpuPipeline::PipelineConfig] Pipeline stage config.
      attr_accessor :pipeline

      # @return [String] Branch predictor algorithm name.
      attr_accessor :branch_predictor_type

      # @return [Integer] Number of entries in the prediction table.
      attr_accessor :branch_predictor_size

      # @return [Integer] Number of entries in the Branch Target Buffer.
      attr_accessor :btb_size

      # @return [Boolean] Whether hazard detection is enabled.
      attr_accessor :hazard_detection

      # @return [Boolean] Whether data forwarding (bypassing) is enabled.
      attr_accessor :forwarding

      # @return [RegisterFileConfig, nil] Register file configuration.
      attr_accessor :register_file

      # @return [FPUnitConfig, nil] Floating-point unit configuration (nil = no FP).
      attr_accessor :fp_unit

      # @return [CodingAdventures::Cache::CacheConfig, nil] L1 instruction cache config.
      attr_accessor :l1i_cache

      # @return [CodingAdventures::Cache::CacheConfig, nil] L1 data cache config.
      attr_accessor :l1d_cache

      # @return [CodingAdventures::Cache::CacheConfig, nil] Unified L2 cache config (nil = no L2).
      attr_accessor :l2_cache

      # @return [Integer] Size of main memory in bytes (default: 65536).
      attr_accessor :memory_size

      # @return [Integer] Access latency for main memory in cycles (default: 100).
      attr_accessor :memory_latency

      def initialize(
        name: "Default",
        pipeline: nil,
        branch_predictor_type: "static_always_not_taken",
        branch_predictor_size: 256,
        btb_size: 64,
        hazard_detection: true,
        forwarding: true,
        register_file: nil,
        fp_unit: nil,
        l1i_cache: nil,
        l1d_cache: nil,
        l2_cache: nil,
        memory_size: 65536,
        memory_latency: 100
      )
        @name = name
        @pipeline = pipeline
        @branch_predictor_type = branch_predictor_type
        @branch_predictor_size = branch_predictor_size
        @btb_size = btb_size
        @hazard_detection = hazard_detection
        @forwarding = forwarding
        @register_file = register_file
        @fp_unit = fp_unit
        @l1i_cache = l1i_cache
        @l1d_cache = l1d_cache
        @l2_cache = l2_cache
        @memory_size = memory_size
        @memory_latency = memory_latency
      end
    end

    # Returns the default core config: a minimal, sensible configuration for
    # testing. This is the "teaching core" -- a 5-stage pipeline with static
    # prediction, small caches, and 16 registers.
    def self.default_core_config
      CoreConfig.new(
        name: "Default",
        pipeline: CodingAdventures::CpuPipeline.classic_5_stage,
        branch_predictor_type: "static_always_not_taken",
        branch_predictor_size: 256,
        btb_size: 64,
        hazard_detection: true,
        forwarding: true,
        register_file: nil,
        fp_unit: nil,
        l1i_cache: nil,
        l1d_cache: nil,
        l2_cache: nil,
        memory_size: 65536,
        memory_latency: 100
      )
    end

    # =========================================================================
    # Preset Configurations -- famous real-world cores approximated
    # =========================================================================

    # Returns a minimal teaching core inspired by the MIPS R2000 (1985):
    #   - 5-stage pipeline (IF, ID, EX, MEM, WB)
    #   - Static predictor (always not taken)
    #   - 4KB direct-mapped L1I and L1D caches
    #   - No L2 cache
    #   - 16 registers, 32-bit
    #   - No floating point
    #
    # Expected IPC: ~0.7-0.9 on simple programs.
    def self.simple_config
      l1i = CodingAdventures::Cache::CacheConfig.new(
        name: "L1I", total_size: 4096, line_size: 64,
        associativity: 1, access_latency: 1, write_policy: "write-back"
      )
      l1d = CodingAdventures::Cache::CacheConfig.new(
        name: "L1D", total_size: 4096, line_size: 64,
        associativity: 1, access_latency: 1, write_policy: "write-back"
      )
      reg_cfg = RegisterFileConfig.new(count: 16, width: 32, zero_register: true)

      CoreConfig.new(
        name: "Simple",
        pipeline: CodingAdventures::CpuPipeline.classic_5_stage,
        branch_predictor_type: "static_always_not_taken",
        branch_predictor_size: 256,
        btb_size: 64,
        hazard_detection: true,
        forwarding: true,
        register_file: reg_cfg,
        fp_unit: nil,
        l1i_cache: l1i,
        l1d_cache: l1d,
        l2_cache: nil,
        memory_size: 65536,
        memory_latency: 100
      )
    end

    # Approximates the ARM Cortex-A78 performance core (2020).
    #
    # The Cortex-A78 is used in Snapdragon 888 and Dimensity 9000:
    #   - 13-stage pipeline (deep for high frequency)
    #   - 2-bit predictor with 4096 entries (simplified vs real TAGE)
    #   - 64KB 4-way L1I and L1D
    #   - 256KB 8-way L2
    #   - 31 registers, 64-bit (ARMv8)
    #   - FP32 and FP64 support
    #
    # Expected IPC: ~0.85-0.95 (our model is in-order; real A78 is out-of-order).
    def self.cortex_a78_like_config
      l1i = CodingAdventures::Cache::CacheConfig.new(
        name: "L1I", total_size: 65536, line_size: 64,
        associativity: 4, access_latency: 1, write_policy: "write-back"
      )
      l1d = CodingAdventures::Cache::CacheConfig.new(
        name: "L1D", total_size: 65536, line_size: 64,
        associativity: 4, access_latency: 1, write_policy: "write-back"
      )
      l2 = CodingAdventures::Cache::CacheConfig.new(
        name: "L2", total_size: 262144, line_size: 64,
        associativity: 8, access_latency: 12, write_policy: "write-back"
      )
      reg_cfg = RegisterFileConfig.new(count: 31, width: 64, zero_register: false)
      fp_cfg = FPUnitConfig.new(formats: ["fp32", "fp64"], pipeline_depth: 4)

      CoreConfig.new(
        name: "CortexA78Like",
        pipeline: CodingAdventures::CpuPipeline.deep_13_stage,
        branch_predictor_type: "two_bit",
        branch_predictor_size: 4096,
        btb_size: 1024,
        hazard_detection: true,
        forwarding: true,
        register_file: reg_cfg,
        fp_unit: fp_cfg,
        l1i_cache: l1i,
        l1d_cache: l1d,
        l2_cache: l2,
        memory_size: 1048576,
        memory_latency: 100
      )
    end

    # =========================================================================
    # MultiCoreConfig -- configuration for a multi-core processor
    # =========================================================================

    # MultiCoreConfig holds the configuration for a multi-core CPU.
    #
    # In a multi-core system, each core has its own L1 and L2 caches but
    # shares an L3 cache and main memory. The memory controller serializes
    # requests from multiple cores.
    class MultiCoreConfig
      # @return [Integer] Number of processor cores.
      attr_accessor :num_cores

      # @return [CoreConfig] Configuration shared by all cores.
      attr_accessor :core_config

      # @return [CodingAdventures::Cache::CacheConfig, nil] Shared L3 cache config.
      attr_accessor :l3_cache

      # @return [Integer] Total shared memory in bytes.
      attr_accessor :memory_size

      # @return [Integer] DRAM access latency in cycles.
      attr_accessor :memory_latency

      def initialize(num_cores: 2, core_config: nil, l3_cache: nil, memory_size: 1048576, memory_latency: 100)
        @num_cores = num_cores
        @core_config = core_config
        @l3_cache = l3_cache
        @memory_size = memory_size
        @memory_latency = memory_latency
      end
    end

    # Returns a 2-core configuration for testing.
    def self.default_multi_core_config
      MultiCoreConfig.new(
        num_cores: 2,
        core_config: simple_config,
        l3_cache: nil,
        memory_size: 1048576,
        memory_latency: 100
      )
    end

    # =========================================================================
    # Helper: create branch predictor from config
    # =========================================================================

    # Builds a branch predictor from config strings.
    #
    # This factory function decouples the config (which uses strings) from the
    # concrete predictor types. The Core calls this once during construction.
    def self.create_branch_predictor(type_name, size)
      case type_name
      when "static_always_taken"
        CodingAdventures::BranchPredictor::AlwaysTakenPredictor.new
      when "static_always_not_taken"
        CodingAdventures::BranchPredictor::AlwaysNotTakenPredictor.new
      when "static_btfnt"
        CodingAdventures::BranchPredictor::BackwardTakenForwardNotTaken.new
      when "one_bit"
        CodingAdventures::BranchPredictor::OneBitPredictor.new(table_size: size)
      when "two_bit"
        CodingAdventures::BranchPredictor::TwoBitPredictor.new(
          table_size: size,
          initial_state: CodingAdventures::BranchPredictor::TwoBitState::WEAKLY_NOT_TAKEN
        )
      else
        # Fall back to always-not-taken for unknown types.
        CodingAdventures::BranchPredictor::AlwaysNotTakenPredictor.new
      end
    end
  end
end

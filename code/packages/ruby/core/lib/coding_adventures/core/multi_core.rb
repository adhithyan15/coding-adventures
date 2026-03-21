# frozen_string_literal: true

# MultiCoreCPU -- multiple cores sharing L3 cache and memory.
#
# = Architecture Diagram
#
#   Core 0: L1I + L1D + L2 (private)
#   Core 1: L1I + L1D + L2 (private)
#   Core 2: L1I + L1D + L2 (private)
#   Core 3: L1I + L1D + L2 (private)
#           |    |    |    |
#      ==============================
#      Shared L3 Cache (optional)
#      ==============================
#                 |
#      Memory Controller (serializes requests)
#                 |
#      Shared Main Memory (DRAM)
#
# = Execution Model
#
# All cores run on the same clock. Each call to step advances every core
# by one cycle. Cores are independent -- they do not share register files
# or pipeline state. They only interact through shared memory.
#
# = Cache Coherence
#
# This implementation does NOT model cache coherence (MESI protocol, etc.).
# Writes by one core become visible to other cores only when they reach
# main memory. Cache coherence is a future extension.

module CodingAdventures
  module Core
    class MultiCoreCPU
      # @return [MultiCoreConfig] the multi-core configuration.
      attr_reader :config

      # @return [Array<Core>] the processor cores.
      attr_reader :cores

      # @return [MemoryController] the shared memory controller.
      attr_reader :shared_memory_controller

      # @return [InterruptController] the interrupt controller.
      attr_reader :interrupt_controller

      # @return [Integer] the global cycle count.
      attr_reader :cycle

      # Creates a multi-core processor.
      #
      # All cores share the same main memory. Each core gets its own ISA
      # decoder (from the decoders array). If decoders.length < num_cores,
      # the last decoder is reused for remaining cores.
      #
      # @param config [MultiCoreConfig] multi-core configuration.
      # @param decoders [Array<Object>] ISA decoders for each core.
      def initialize(config, decoders)
        @config = config

        # Allocate shared memory.
        mem_size = config.memory_size
        mem_size = 1048576 if mem_size <= 0
        shared_memory = Array.new(mem_size, 0)

        mem_latency = config.memory_latency
        mem_latency = 100 if mem_latency <= 0
        @shared_memory_controller = MemoryController.new(shared_memory, mem_latency)

        # Optional shared L3 cache.
        @l3_cache = config.l3_cache ? CodingAdventures::Cache::CacheSimulator.new(config.l3_cache) : nil

        # Create cores.
        num_cores = config.num_cores
        num_cores = 1 if num_cores <= 0

        @cores = Array.new(num_cores) do |i|
          # Select decoder for this core.
          decoder = (i < decoders.length) ? decoders[i] : decoders[0]

          # Override the core config to use shared memory size.
          core_cfg = config.core_config.dup
          core_cfg.memory_size = mem_size
          core_cfg.memory_latency = mem_latency

          c = Core.new(core_cfg, decoder)

          # Replace the core's memory controller with the shared one,
          # so all cores read/write the same memory.
          c.instance_variable_set(:@mem_ctrl, @shared_memory_controller)

          c
        end

        @interrupt_controller = InterruptController.new(num_cores)
        @cycle = 0
      end

      # Loads a program into memory for a specific core.
      #
      # Since all cores share memory, the program is written to the shared
      # memory at the given address. The specified core's PC is set to
      # start_address.
      #
      # @param core_id [Integer] which core to load for.
      # @param program [Array<Integer>] byte array of program data.
      # @param start_address [Integer] memory address to load at.
      def load_program(core_id, program, start_address)
        return if core_id < 0 || core_id >= @cores.length

        # Write program to shared memory.
        @shared_memory_controller.load_program(program, start_address)

        # Set the core's PC.
        @cores[core_id].pipeline.set_pc(start_address)
      end

      # Advances all cores by one clock cycle.
      #
      # @return [Array<CodingAdventures::CpuPipeline::PipelineSnapshot>] snapshots.
      def step
        @cycle += 1
        snapshots = @cores.map(&:step)

        # Tick the shared memory controller.
        @shared_memory_controller.tick

        snapshots
      end

      # Runs all cores until all have halted or max_cycles is reached.
      #
      # @param max_cycles [Integer] maximum number of cycles.
      # @return [Array<CoreStats>] per-core statistics.
      def run(max_cycles)
        while @cycle < max_cycles
          break if all_halted?
          step
        end
        stats
      end

      # Returns per-core statistics.
      #
      # @return [Array<CoreStats>] statistics for each core.
      def stats
        @cores.map(&:stats)
      end

      # Returns true if every core has halted.
      #
      # @return [Boolean] whether all cores are halted.
      def all_halted?
        @cores.all?(&:halted?)
      end
    end
  end
end

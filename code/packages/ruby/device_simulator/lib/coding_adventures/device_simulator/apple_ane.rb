# frozen_string_literal: true

# ---------------------------------------------------------------------------
# AppleANE -- device simulator with unified memory.
# ---------------------------------------------------------------------------
#
# === Apple ANE Architecture ===
#
# The Apple Neural Engine is radically different from GPUs and TPUs.
# It's a fixed-function accelerator designed for neural network inference,
# optimized for power efficiency over flexibility.
#
#     +----------------------------------------------------+
#     |           Apple Neural Engine                       |
#     |                                                     |
#     |  +------------------------------------------------+|
#     |  |       DMA Controller (schedule replayer)        ||
#     |  +----+-----+-----+------+------------------------+|
#     |       |     |     |      |                          |
#     |  +------+ +------+ +------+ +------+               |
#     |  |Core 0| |Core 1| |Core 2| |Core N|               |
#     |  | MAC  | | MAC  | | MAC  | | MAC  |               |
#     |  | Array| | Array| | Array| | Array|               |
#     |  +--+---+ +--+---+ +--+---+ +--+---+               |
#     |     +--------+--------+--------+                    |
#     |                |                                    |
#     |  +-------------+----------------------------------+|
#     |  |         Shared SRAM (32 MB)                     ||
#     |  +-------------+----------------------------------+|
#     |                |                                    |
#     |  +-------------+----------------------------------+|
#     |  |   Unified Memory (shared with CPU & GPU)        ||
#     |  |   No copy needed -- just remap page tables      ||
#     |  +------------------------------------------------+|
#     +----------------------------------------------------+
#
# === Unified Memory: The Game Changer ===
#
# Apple's unified memory architecture means the ANE, CPU, and GPU all
# share the same physical memory. When you "copy" data to the ANE, there's
# no actual data movement -- the system just updates page table mappings.
#
#     Discrete GPU: Copy 8 MB over PCIe -> 125 us overhead
#     Apple ANE:    Remap page tables -> ~0 us overhead
#
# === Compiler-Driven Scheduling ===
#
# Unlike GPUs (hardware warp schedulers) and TPUs (sequencer), the ANE
# relies entirely on the CoreML compiler to generate a fixed execution
# schedule. The hardware simply replays this schedule.

module CodingAdventures
  module DeviceSimulator
    class AppleANE
      # Create a new Apple ANE device simulator.
      #
      # @param config [DeviceConfig, nil] Full config. Uses defaults if nil.
      # @param num_cores [Integer] Shorthand -- number of NE cores.
      def initialize(config: nil, num_cores: 4)
        if config.nil?
          config = ANEConfig.new(
            name: "Apple ANE (#{num_cores} cores)",
            architecture: "apple_ane_core",
            num_compute_units: num_cores,
            l2_cache_size: 0,
            l2_cache_latency: 0,
            l2_cache_associativity: 0,
            global_memory_size: 16 * 1024 * 1024,
            global_memory_bandwidth: 200.0,
            global_memory_latency: 100,
            memory_channels: 8,
            host_bandwidth: 200.0,
            host_latency: 0,
            unified_memory: true,
            max_concurrent_kernels: 1,
            work_distribution_policy: "scheduled",
            shared_sram_size: 4 * 1024 * 1024,
            sram_bandwidth: 1000.0,
            sram_latency: 5,
            dma_channels: 4,
            dma_bandwidth: 100.0
          )
        end

        @config = config
        @clock = Clock::ClockGenerator.new(frequency_hz: 1_000_000_000)

        # Create NE cores
        core_config = config.cu_config || ComputeUnit::ANECoreConfig.new
        @cores = Array.new(config.num_compute_units) do
          ComputeUnit::NeuralEngineCore.new(core_config, @clock)
        end

        # Global memory (unified -- zero-copy)
        @global_memory = SimpleGlobalMemory.new(
          capacity: config.global_memory_size,
          bandwidth: config.global_memory_bandwidth,
          latency: config.global_memory_latency,
          channels: config.memory_channels,
          host_bandwidth: config.host_bandwidth,
          host_latency: config.host_latency,
          unified: config.unified_memory
        )

        # Schedule replayer (compiler-driven)
        dma_latency = 10
        compute_latency = 20
        if config.is_a?(ANEConfig)
          dma_latency = [1, (1024 / config.dma_bandwidth).to_i].max
          compute_latency = 20
        end

        @replayer = ANEScheduleReplayer.new(
          @cores,
          dma_latency: dma_latency,
          compute_latency: compute_latency,
          activate_latency: 5
        )

        @cycle = 0
        @kernels_launched = 0
      end

      # --- Identity ---

      def name = @config.name
      def config = @config

      # --- Memory management ---

      def malloc(size) = @global_memory.allocate(size)
      def free(address) = @global_memory.free(address)

      # Copy from host -- zero-cost on unified memory!
      #
      # On Apple's unified memory, this doesn't actually copy data.
      # The CPU and ANE share the same physical memory. The 'copy'
      # just updates page table mappings.
      def memcpy_host_to_device(dst, data)
        @global_memory.copy_from_host(dst, data)
      end

      # Copy to host -- zero-cost on unified memory!
      def memcpy_device_to_host(src, size)
        @global_memory.copy_to_host(src, size)
      end

      # --- Operation launch ---

      # Submit an operation to the schedule replayer.
      #
      # The compiler (us) generates a complete execution schedule
      # including DMA loads, compute, activation, and DMA stores.
      def launch_kernel(kernel)
        @replayer.submit_operation(kernel)
        @kernels_launched += 1
      end

      # --- Simulation ---

      def step(clock_edge = nil)
        @cycle += 1
        edge = clock_edge || @clock.tick

        # Replay the next step in the compiler-generated schedule
        schedule_actions = @replayer.step

        # Step all cores
        cu_traces = @cores.map { |core| core.step(edge) }

        active_cores = @cores.count { |core| !core.idle? }

        DeviceTrace.new(
          cycle: @cycle,
          device_name: @config.name,
          distributor_actions: schedule_actions,
          pending_blocks: @replayer.pending_count,
          active_blocks: active_cores,
          cu_traces: cu_traces,
          device_occupancy: @cores.empty? ? 0.0 : active_cores.to_f / @cores.length
        )
      end

      def run(max_cycles = 10_000)
        traces = []
        max_cycles.times do
          trace = step
          traces << trace
          break if idle?
        end
        traces
      end

      def idle?
        @replayer.idle?
      end

      def reset
        @cores.each(&:reset)
        @global_memory.reset
        @replayer.reset
        @cycle = 0
        @kernels_launched = 0
      end

      # --- Observability ---

      def stats
        DeviceStats.new(
          total_cycles: @cycle,
          total_kernels_launched: @kernels_launched,
          total_blocks_dispatched: @replayer.total_dispatched,
          global_memory_stats: @global_memory.stats
        )
      end

      def compute_units = @cores.dup
      def global_memory = @global_memory

      # True -- Apple ANE always uses unified memory.
      def unified_memory? = true
    end
  end
end

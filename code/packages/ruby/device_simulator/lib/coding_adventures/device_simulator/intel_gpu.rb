# frozen_string_literal: true

# ---------------------------------------------------------------------------
# IntelGPU -- device simulator with Xe-Slices.
# ---------------------------------------------------------------------------
#
# === Intel GPU Architecture (Xe-HPG / Arc) ===
#
# Intel organizes Xe-Cores into **Xe-Slices**, with each slice sharing
# a large L1 cache. This is similar to AMD's Shader Engines but at a
# different granularity.
#
#     +------------------------------------------------------+
#     |                Intel GPU                              |
#     |  +--------------------------------------------------+|
#     |  |     Command Streamer (distributor)                ||
#     |  +--------------------+-----------------------------+|
#     |                       |                              ||
#     |  +--------------------+--------------------+         ||
#     |  |         Xe-Slice 0                      |         ||
#     |  |  +--------+ +--------+ +--------+ +--+ |         ||
#     |  |  |XeCore 0| |XeCore 1| |XeCore 2| |3 | |         ||
#     |  |  +--------+ +--------+ +--------+ +--+ |         ||
#     |  |  L1 Cache (192 KB shared)               |         ||
#     |  +-------------------------------------------+       ||
#     |  ... (4-8 Xe-Slices)                                 ||
#     |                                                      ||
#     |  +--------------------------------------------------+|
#     |  |         L2 Cache (16 MB shared)                   ||
#     |  +--------------------+-----------------------------+|
#     |                       |                              ||
#     |  +--------------------+-----------------------------+|
#     |  |        GDDR6 (16 GB, 512 GB/s)                   ||
#     |  +--------------------------------------------------+|
#     +------------------------------------------------------+

module CodingAdventures
  module DeviceSimulator
    # A group of Xe-Cores sharing an L1 cache.
    class XeSlice
      attr_reader :slice_id, :xe_cores

      def initialize(slice_id, xe_cores)
        @slice_id = slice_id
        @xe_cores = xe_cores
      end

      def idle?
        @xe_cores.all?(&:idle?)
      end
    end

    class IntelGPU
      # Create a new Intel GPU device simulator.
      #
      # @param config [DeviceConfig, nil] Full config. Uses defaults if nil.
      # @param num_cores [Integer] Shorthand -- total Xe-Cores.
      def initialize(config: nil, num_cores: 4)
        if config.nil?
          config = DeviceConfig.new(
            name: "Intel GPU (#{num_cores} Xe-Cores)",
            architecture: "intel_xe_core",
            num_compute_units: num_cores,
            l2_cache_size: 4096,
            l2_cache_latency: 180,
            l2_cache_associativity: 4,
            l2_cache_line_size: 64,
            global_memory_size: 16 * 1024 * 1024,
            global_memory_bandwidth: 512.0,
            global_memory_latency: 350,
            memory_channels: 4,
            host_bandwidth: 32.0,
            host_latency: 100,
            unified_memory: false,
            max_concurrent_kernels: 16,
            work_distribution_policy: "round_robin"
          )
        end

        @config = config
        @clock = Clock::ClockGenerator.new(frequency_hz: 2_100_000_000)

        # Create Xe-Cores
        core_config = config.cu_config || ComputeUnit::XeCoreConfig.new
        @all_cores = Array.new(config.num_compute_units) do
          ComputeUnit::XeCore.new(core_config, @clock)
        end

        # Group into Xe-Slices
        cores_per_slice = if config.is_a?(IntelGPUConfig)
          config.slice_config.xe_cores_per_slice
        else
          [1, config.num_compute_units / 2].max
        end

        @xe_slices = []
        @all_cores.each_slice(cores_per_slice) do |slice_cores|
          @xe_slices << XeSlice.new(@xe_slices.length, slice_cores)
        end

        # L2 cache
        @l2 = if config.l2_cache_size > 0
          Cache::CacheSimulator.new(Cache::CacheConfig.new(
            name: "L2",
            total_size: config.l2_cache_size,
            line_size: config.l2_cache_line_size,
            associativity: config.l2_cache_associativity,
            access_latency: config.l2_cache_latency
          ))
        end

        # Global memory
        @global_memory = SimpleGlobalMemory.new(
          capacity: config.global_memory_size,
          bandwidth: config.global_memory_bandwidth,
          latency: config.global_memory_latency,
          channels: config.memory_channels,
          host_bandwidth: config.host_bandwidth,
          host_latency: config.host_latency,
          unified: config.unified_memory
        )

        # Work distributor (Command Streamer)
        @distributor = GPUWorkDistributor.new(
          @all_cores,
          policy: config.work_distribution_policy
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

      def memcpy_host_to_device(dst, data)
        @global_memory.copy_from_host(dst, data)
      end

      def memcpy_device_to_host(src, size)
        @global_memory.copy_to_host(src, size)
      end

      # --- Kernel launch ---

      def launch_kernel(kernel)
        @distributor.submit_kernel(kernel)
        @kernels_launched += 1
      end

      # --- Simulation ---

      def step(clock_edge = nil)
        @cycle += 1
        edge = clock_edge || @clock.tick

        dist_actions = @distributor.step

        cu_traces = []
        total_active_warps = 0
        total_max_warps = 0

        @all_cores.each do |core|
          trace = core.step(edge)
          cu_traces << trace
          total_active_warps += trace.active_warps
          total_max_warps += trace.total_warps
        end

        device_occupancy = if total_max_warps > 0
          total_active_warps.to_f / total_max_warps
        else
          0.0
        end

        active_blocks = @all_cores.count { |core| !core.idle? }

        DeviceTrace.new(
          cycle: @cycle,
          device_name: @config.name,
          distributor_actions: dist_actions,
          pending_blocks: @distributor.pending_count,
          active_blocks: active_blocks,
          cu_traces: cu_traces,
          total_active_warps: total_active_warps,
          device_occupancy: device_occupancy
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
        @distributor.pending_count == 0 && @all_cores.all?(&:idle?)
      end

      def reset
        @all_cores.each(&:reset)
        @global_memory.reset
        @distributor.reset
        @cycle = 0
        @kernels_launched = 0
      end

      # --- Observability ---

      def stats
        DeviceStats.new(
          total_cycles: @cycle,
          total_kernels_launched: @kernels_launched,
          total_blocks_dispatched: @distributor.total_dispatched,
          global_memory_stats: @global_memory.stats
        )
      end

      def compute_units = @all_cores.dup

      # Access to Xe-Slices (Intel-specific).
      def xe_slices = @xe_slices

      def global_memory = @global_memory
    end
  end
end

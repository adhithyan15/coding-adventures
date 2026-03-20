# frozen_string_literal: true

# ---------------------------------------------------------------------------
# AmdGPU -- device simulator with Shader Engines and Infinity Cache.
# ---------------------------------------------------------------------------
#
# === AMD GPU Architecture ===
#
# AMD organizes compute units (CUs) into **Shader Engines** (SEs). This is
# a mid-level hierarchy that NVIDIA doesn't have -- CUs within the same SE
# share a geometry processor and rasterizer (for graphics), and for compute
# workloads, the Command Processor assigns entire work-groups to SEs first.
#
#     +------------------------------------------------------+
#     |                    AMD GPU                            |
#     |  +--------------------------------------------------+|
#     |  |       Command Processor (distributor)             ||
#     |  +------------------+-------------------------------+|
#     |                     |                                ||
#     |  +------------------+-------------------+            ||
#     |  |      Shader Engine 0                 |            ||
#     |  |  +------+ +------+ ... +------+      |            ||
#     |  |  |CU 0  | |CU 1  |     |CU N  |      |            ||
#     |  |  +------+ +------+     +------+      |            ||
#     |  +--------------------------------------+            ||
#     |  ... more Shader Engines                             ||
#     |                                                      ||
#     |  +--------------------------------------------------+|
#     |  |     Infinity Cache (96 MB, ~50 cycle lat.)        ||
#     |  +------------------+-------------------------------+|
#     |                     |                                ||
#     |  +------------------+-------------------------------+|
#     |  |           GDDR6 (24 GB, 960 GB/s)                ||
#     |  +--------------------------------------------------+|
#     +------------------------------------------------------+

module CodingAdventures
  module DeviceSimulator
    # A group of CUs that share resources.
    #
    # In a real AMD GPU, a Shader Engine shares a geometry processor,
    # rasterizer, and some L1 cache. For compute workloads, it mainly
    # affects how the Command Processor assigns work.
    class ShaderEngine
      attr_reader :engine_id, :cus

      def initialize(engine_id, cus)
        @engine_id = engine_id
        @cus = cus
      end

      # True when all CUs in this SE are idle.
      def idle?
        @cus.all?(&:idle?)
      end
    end

    class AmdGPU
      # Create a new AMD GPU device simulator.
      #
      # @param config [DeviceConfig, nil] Full config. Uses defaults if nil.
      # @param num_cus [Integer] Shorthand -- total CUs.
      def initialize(config: nil, num_cus: 4)
        if config.nil?
          config = DeviceConfig.new(
            name: "AMD GPU (#{num_cus} CUs)",
            architecture: "amd_cu",
            num_compute_units: num_cus,
            l2_cache_size: 4096,
            l2_cache_latency: 150,
            l2_cache_associativity: 4,
            l2_cache_line_size: 64,
            global_memory_size: 16 * 1024 * 1024,
            global_memory_bandwidth: 960.0,
            global_memory_latency: 350,
            memory_channels: 4,
            host_bandwidth: 32.0,
            host_latency: 100,
            unified_memory: false,
            max_concurrent_kernels: 8,
            work_distribution_policy: "round_robin"
          )
        end

        @config = config
        @clock = Clock::ClockGenerator.new(frequency_hz: 1_800_000_000)

        # Create CUs
        cu_config = config.cu_config || ComputeUnit::AMDCUConfig.new
        @all_cus = Array.new(config.num_compute_units) do
          ComputeUnit::AMDComputeUnit.new(cu_config, @clock)
        end

        # Group into Shader Engines
        se_size = if config.is_a?(AmdGPUConfig)
          config.se_config.cus_per_engine
        else
          [1, config.num_compute_units / 2].max
        end

        @shader_engines = []
        @all_cus.each_slice(se_size) do |se_cus|
          @shader_engines << ShaderEngine.new(@shader_engines.length, se_cus)
        end

        # Infinity Cache (if AMD-specific config)
        @infinity_cache = if config.is_a?(AmdGPUConfig) && config.infinity_cache_size > 0
          ic_size = config.infinity_cache_size
          ic_size_pow2 = 1 << (ic_size.bit_length - 1)
          Cache::CacheSimulator.new(Cache::CacheConfig.new(
            name: "InfinityCache",
            total_size: [ic_size_pow2, 4096].min,
            line_size: 64,
            associativity: 16,
            access_latency: config.infinity_cache_latency
          ))
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

        # Work distributor (Command Processor)
        @distributor = GPUWorkDistributor.new(
          @all_cus,
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

        @all_cus.each do |cu|
          trace = cu.step(edge)
          cu_traces << trace
          total_active_warps += trace.active_warps
          total_max_warps += trace.total_warps
        end

        device_occupancy = if total_max_warps > 0
          total_active_warps.to_f / total_max_warps
        else
          0.0
        end

        active_blocks = @all_cus.count { |cu| !cu.idle? }

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
        @distributor.pending_count == 0 && @all_cus.all?(&:idle?)
      end

      def reset
        @all_cus.each(&:reset)
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

      def compute_units = @all_cus.dup

      # Access to Shader Engines (AMD-specific).
      def shader_engines = @shader_engines

      def global_memory = @global_memory
    end
  end
end

# frozen_string_literal: true

# ---------------------------------------------------------------------------
# NvidiaGPU -- device simulator with GigaThread Engine.
# ---------------------------------------------------------------------------
#
# === NVIDIA GPU Architecture ===
#
# The NVIDIA GPU is the most widely-used accelerator for machine learning.
# Its architecture is built around Streaming Multiprocessors (SMs), each
# of which can independently schedule and execute thousands of threads.
#
#     +---------------------------------------------------+
#     |                  NVIDIA GPU                        |
#     |                                                    |
#     |  +----------------------------------------------+  |
#     |  |        GigaThread Engine (distributor)        |  |
#     |  +--------------------+-------------------------+  |
#     |                       |                            |
#     |  +------+ +------+ +------+ ... +------+          |
#     |  |SM 0  | |SM 1  | |SM 2  |     |SM N  |          |
#     |  +--+---+ +--+---+ +--+---+     +--+---+          |
#     |     +--------+--------+-----------+                |
#     |                       |                            |
#     |  +--------------------+-------------------------+  |
#     |  |            L2 Cache (shared)                 |  |
#     |  +--------------------+-------------------------+  |
#     |                       |                            |
#     |  +--------------------+-------------------------+  |
#     |  |          HBM3 (80 GB, 3.35 TB/s)             |  |
#     |  +----------------------------------------------+  |
#     +---------------------------------------------------+
#
# === GigaThread Engine ===
#
# The GigaThread Engine is the top-level work distributor. When a kernel
# is launched, it:
#
# 1. Creates thread blocks from the grid dimensions
# 2. Assigns blocks to SMs with available resources
# 3. As SMs complete blocks, assigns new ones
# 4. Continues until all blocks are dispatched
#
# This creates **waves** of execution:
# - Wave 1: Fill all SMs to capacity
# - Wave 2: As SMs finish, refill them
# - ...until all blocks are done

module CodingAdventures
  module DeviceSimulator
    class NvidiaGPU
      # Create a new NVIDIA GPU device simulator.
      #
      # @param config [DeviceConfig, nil] Full device config. Uses defaults if nil.
      # @param num_sms [Integer] Shorthand -- creates a config with this many SMs.
      #   Ignored if config is provided.
      def initialize(config: nil, num_sms: 4)
        if config.nil?
          config = DeviceConfig.new(
            name: "NVIDIA GPU (#{num_sms} SMs)",
            architecture: "nvidia_sm",
            num_compute_units: num_sms,
            l2_cache_size: 4096,
            l2_cache_latency: 200,
            l2_cache_associativity: 4,
            l2_cache_line_size: 64,
            global_memory_size: 16 * 1024 * 1024,
            global_memory_bandwidth: 1000.0,
            global_memory_latency: 400,
            memory_channels: 4,
            host_bandwidth: 64.0,
            host_latency: 100,
            unified_memory: false,
            max_concurrent_kernels: 128,
            work_distribution_policy: "round_robin"
          )
        end

        @config = config
        @clock = Clock::ClockGenerator.new(frequency_hz: 1_500_000_000)

        # Create SMs
        sm_config = config.cu_config || ComputeUnit::SMConfig.new(
          max_warps: 8,
          num_schedulers: 2,
          shared_memory_size: 4096,
          register_file_size: 8192
        )
        @sms = Array.new(config.num_compute_units) do
          ComputeUnit::StreamingMultiprocessor.new(sm_config, @clock)
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

        # Work distributor (GigaThread Engine)
        @distributor = GPUWorkDistributor.new(
          @sms,
          policy: config.work_distribution_policy
        )

        # Stats
        @cycle = 0
        @total_l2_hits = 0
        @total_l2_misses = 0
        @kernels_launched = 0
      end

      # --- Identity ---

      # Device name.
      def name
        @config.name
      end

      # Full device configuration.
      def config
        @config
      end

      # --- Memory management ---

      # Allocate device memory.
      # @param size [Integer] Bytes to allocate.
      # @return [Integer] Device pointer (address).
      def malloc(size)
        @global_memory.allocate(size)
      end

      # Free device memory.
      # @param address [Integer] Previously allocated address.
      def free(address)
        @global_memory.free(address)
      end

      # Copy from host to device. Returns cycles consumed.
      # @param dst [Integer] Destination address.
      # @param data [String] Binary data.
      # @return [Integer] Cycles consumed.
      def memcpy_host_to_device(dst, data)
        @global_memory.copy_from_host(dst, data)
      end

      # Copy from device to host.
      # @param src [Integer] Source address.
      # @param size [Integer] Bytes to copy.
      # @return [Array(String, Integer)] [data, cycles].
      def memcpy_device_to_host(src, size)
        @global_memory.copy_to_host(src, size)
      end

      # --- Kernel launch ---

      # Submit a kernel for execution via the GigaThread Engine.
      # @param kernel [KernelDescriptor] The kernel to launch.
      def launch_kernel(kernel)
        @distributor.submit_kernel(kernel)
        @kernels_launched += 1
      end

      # --- Simulation ---

      # Advance the entire device by one clock cycle.
      #
      # 1. GigaThread assigns pending blocks to SMs with free resources
      # 2. Each SM steps (scheduler picks warps, engines execute)
      # 3. Collect traces from all SMs
      # 4. Build device-wide trace
      #
      # @param clock_edge [Object, nil] Optional clock edge.
      # @return [DeviceTrace]
      def step(clock_edge = nil)
        @cycle += 1

        edge = clock_edge || @clock.tick

        # 1. Distribute pending blocks to SMs
        dist_actions = @distributor.step

        # 2. Step all SMs
        cu_traces = []
        total_active_warps = 0
        total_max_warps = 0

        @sms.each do |sm|
          trace = sm.step(edge)
          cu_traces << trace
          total_active_warps += trace.active_warps
          total_max_warps += trace.total_warps
        end

        # 3. Compute device-level metrics
        device_occupancy = if total_max_warps > 0
          total_active_warps.to_f / total_max_warps
        else
          0.0
        end

        active_blocks = @sms.count { |sm| !sm.idle? }

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

      # Run until all work is done or max_cycles reached.
      # @param max_cycles [Integer] Safety limit.
      # @return [Array<DeviceTrace>]
      def run(max_cycles = 10_000)
        traces = []
        max_cycles.times do
          trace = step
          traces << trace
          break if idle?
        end
        traces
      end

      # True when all SMs are idle and no pending blocks remain.
      def idle?
        @distributor.pending_count == 0 && @sms.all?(&:idle?)
      end

      # Reset everything.
      def reset
        @sms.each(&:reset)
        @global_memory.reset
        @distributor.reset
        if @l2 && @config.l2_cache_size > 0
          @l2 = Cache::CacheSimulator.new(Cache::CacheConfig.new(
            name: "L2",
            total_size: @config.l2_cache_size,
            line_size: @config.l2_cache_line_size,
            associativity: @config.l2_cache_associativity,
            access_latency: @config.l2_cache_latency
          ))
        end
        @cycle = 0
        @total_l2_hits = 0
        @total_l2_misses = 0
        @kernels_launched = 0
      end

      # --- Observability ---

      # Aggregate statistics.
      # @return [DeviceStats]
      def stats
        DeviceStats.new(
          total_cycles: @cycle,
          active_cycles: @cycle > 0 ? @cycle : 0,
          total_kernels_launched: @kernels_launched,
          total_blocks_dispatched: @distributor.total_dispatched,
          global_memory_stats: @global_memory.stats
        )
      end

      # Direct access to SMs.
      # @return [Array<StreamingMultiprocessor>]
      def compute_units
        @sms.dup
      end

      # Access to device memory.
      # @return [SimpleGlobalMemory]
      def global_memory
        @global_memory
      end
    end
  end
end

# frozen_string_literal: true

# ---------------------------------------------------------------------------
# GoogleTPU -- device simulator with Scalar/Vector/MXU pipeline.
# ---------------------------------------------------------------------------
#
# === TPU Architecture ===
#
# The TPU is fundamentally different from GPUs. Instead of thousands of
# small cores executing thread programs, the TPU has:
#
# 1. **One large MXU** (Matrix Multiply Unit) -- a 128x128 systolic array
#    that multiplies entire matrices in hardware.
# 2. **A vector unit** -- handles element-wise operations (activation
#    functions, normalization, softmax).
# 3. **A scalar unit** -- handles control flow, address calculation,
#    and loop counters.
#
# These three units form a **pipeline**: while the MXU processes one
# matrix tile, the vector unit post-processes the previous tile, and
# the scalar unit prepares the next tile.
#
# === No Thread Blocks ===
#
# TPUs don't have threads, warps, or thread blocks:
#
#     GPU: "Run this program on 65,536 threads"
#     TPU: "Multiply this 1024x512 matrix by this 512x768 matrix"

module CodingAdventures
  module DeviceSimulator
    class GoogleTPU
      # Create a new Google TPU device simulator.
      #
      # @param config [DeviceConfig, nil] Full config. Uses defaults if nil.
      # @param mxu_size [Integer] Systolic array dimension (e.g., 128).
      def initialize(config: nil, mxu_size: 4)
        if config.nil?
          config = TPUConfig.new(
            name: "Google TPU (MXU #{mxu_size}x#{mxu_size})",
            architecture: "google_mxu",
            num_compute_units: 1,
            l2_cache_size: 0,
            l2_cache_latency: 0,
            l2_cache_associativity: 0,
            global_memory_size: 16 * 1024 * 1024,
            global_memory_bandwidth: 1200.0,
            global_memory_latency: 300,
            memory_channels: 4,
            host_bandwidth: 500.0,
            host_latency: 100,
            unified_memory: false,
            max_concurrent_kernels: 1,
            work_distribution_policy: "sequential",
            vector_unit_width: mxu_size,
            scalar_registers: 32,
            transpose_unit: true
          )
        end

        @config = config
        @clock = Clock::ClockGenerator.new(frequency_hz: 1_000_000_000)

        # Create MXU
        mxu_config = config.cu_config || ComputeUnit::MXUConfig.new
        @mxu = ComputeUnit::MatrixMultiplyUnit.new(mxu_config, @clock)

        # The sequencer orchestrates Scalar -> MXU -> Vector pipeline
        vec_width = if config.is_a?(TPUConfig)
          config.vector_unit_width
        else
          mxu_size
        end

        @sequencer = TPUSequencer.new(
          @mxu,
          mxu_size: mxu_size,
          vector_width: vec_width,
          scalar_latency: 5,
          mxu_latency: 20,
          vector_latency: 10
        )

        # Global memory (HBM)
        @global_memory = SimpleGlobalMemory.new(
          capacity: config.global_memory_size,
          bandwidth: config.global_memory_bandwidth,
          latency: config.global_memory_latency,
          channels: config.memory_channels,
          host_bandwidth: config.host_bandwidth,
          host_latency: config.host_latency,
          unified: config.unified_memory
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

      # --- Operation launch ---

      # Submit an operation (matmul, etc.) to the sequencer.
      def launch_kernel(kernel)
        @sequencer.submit_operation(kernel)
        @kernels_launched += 1
      end

      # --- Simulation ---

      def step(clock_edge = nil)
        @cycle += 1
        edge = clock_edge || @clock.tick

        # Advance the Scalar -> MXU -> Vector pipeline
        seq_actions = @sequencer.step

        # Also step the MXU compute unit
        cu_trace = @mxu.step(edge)

        DeviceTrace.new(
          cycle: @cycle,
          device_name: @config.name,
          distributor_actions: seq_actions,
          pending_blocks: @sequencer.pending_count,
          active_blocks: @sequencer.idle? ? 0 : 1,
          cu_traces: [cu_trace],
          device_occupancy: @sequencer.idle? ? 0.0 : 1.0
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
        @sequencer.idle?
      end

      def reset
        @mxu.reset
        @sequencer.reset
        @global_memory.reset
        @cycle = 0
        @kernels_launched = 0
      end

      # --- Observability ---

      def stats
        DeviceStats.new(
          total_cycles: @cycle,
          total_kernels_launched: @kernels_launched,
          total_blocks_dispatched: @sequencer.total_dispatched,
          global_memory_stats: @global_memory.stats
        )
      end

      def compute_units = [@mxu]
      def global_memory = @global_memory
    end
  end
end

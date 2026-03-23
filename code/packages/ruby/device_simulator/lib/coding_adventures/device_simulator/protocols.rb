# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Protocols -- shared types for all device simulators.
# ---------------------------------------------------------------------------
#
# === What is a Device Simulator? ===
#
# A device simulator models a **complete accelerator** -- not just one compute
# unit, but the entire chip with all its compute units, global memory, caches,
# and the work distributor that ties them together.
#
# Think of it as the difference between simulating one factory floor (Layer 7)
# versus simulating the entire factory complex:
#
#     Layer 7 (Compute Unit):    One SM / CU / MXU -- a single factory floor
#     Layer 6 (Device):          The whole factory -- all floors + warehouse +
#                                shipping dock + floor manager's office
#
# The device layer adds four new concepts:
#
# 1. **Global Memory (VRAM)** -- the large device-wide memory (the warehouse).
#    All compute units share it. High bandwidth but high latency (~400 cycles).
#
# 2. **L2 Cache** -- sits between compute units and global memory. Reduces the
#    average latency for frequently-accessed data.
#
# 3. **Work Distributor** -- takes kernel launches (work orders) and assigns
#    thread blocks to compute units that have available resources.
#
# 4. **Host Interface** -- the connection to the CPU. Data must be copied from
#    CPU memory to device memory before the GPU can use it (except on Apple's
#    unified memory, where it's zero-copy).
#
# === Duck Type Design ===
#
# Like Layer 7, we use Ruby duck typing to define a common interface:
#
#     device.name                          -> String
#     device.config                        -> DeviceConfig
#     device.malloc(size)                  -> Integer (address)
#     device.free(address)                 -> nil
#     device.memcpy_host_to_device(dst, data) -> Integer (cycles)
#     device.memcpy_device_to_host(src, size) -> [bytes, cycles]
#     device.launch_kernel(kernel)         -> nil
#     device.step(clock_edge)              -> DeviceTrace
#     device.run(max_cycles)               -> Array<DeviceTrace>
#     device.idle?                         -> Boolean
#     device.reset                         -> nil
#     device.stats                         -> DeviceStats
#     device.compute_units                 -> Array
#
# === Memory Hierarchy at the Device Level ===
#
#                 +----------------+
#     CPU RAM --> | Host Interface | --> PCIe / NVLink / unified
#                 +-------+--------+
#                         |
#                 +-------+--------+
#                 | Global Memory  |  24-80 GB, ~400 cycle latency
#                 |  (HBM/GDDR)   |  1-3 TB/s bandwidth
#                 +-------+--------+
#                         |
#                 +-------+--------+
#                 |   L2 Cache     |  4-96 MB, ~200 cycle latency
#                 |  (shared)      |
#                 +--+----+----+---+
#                    |    |    |
#                  CU 0 CU 1 ... CU N   (each with local shared memory)

module CodingAdventures
  module DeviceSimulator
    # -----------------------------------------------------------------------
    # MemoryTransaction -- a single wide memory access after coalescing
    # -----------------------------------------------------------------------
    #
    # When 32 threads in a warp each request 4 bytes, those 128 bytes of
    # requests might coalesce into a single 128-byte transaction (best case)
    # or 32 separate transactions (worst case -- scattered access).
    #
    # === Coalescing Visual ===
    #
    # Best case (1 transaction):
    #     Thread  0  1  2  3  4  ...  31
    #     Addr   [0][4][8][12][16]...[124]
    #            +------------------------+
    #              One 128B transaction
    #
    # Worst case (32 transactions):
    #     Thread  0     1      2      3
    #     Addr   [0]  [512]  [1024]  [1536]  ...
    #             |      |      |      |
    #             v      v      v      v
    #          Trans 1 Trans 2 Trans 3 Trans 4
    #
    # Fields:
    #   address:     Aligned start address of the transaction.
    #   size:        Transaction size in bytes (32, 64, or 128).
    #   thread_mask: Bitmask of which threads are served by this transaction.
    MemoryTransaction = Data.define(:address, :size, :thread_mask)

    # -----------------------------------------------------------------------
    # GlobalMemoryStats -- tracks memory access patterns and efficiency
    # -----------------------------------------------------------------------
    #
    # === Why Track These? ===
    #
    # Memory access patterns are the #1 performance bottleneck on GPUs.
    # A kernel that achieves perfect coalescing uses 32x less bandwidth than
    # one with fully scattered access. These stats tell you whether your
    # memory accesses are efficient.
    #
    # Key metric: **coalescing_efficiency**
    #     = total_requests / total_transactions
    #     Ideal = 1.0 (every request coalesces into existing transactions)
    #     Worst = 32.0 for 32-wide warps (nothing coalesces)
    class GlobalMemoryStats
      attr_accessor :total_reads, :total_writes, :total_transactions,
        :total_requests, :bytes_transferred, :coalescing_efficiency,
        :partition_conflicts, :host_to_device_bytes,
        :device_to_host_bytes, :host_transfer_cycles

      def initialize
        @total_reads = 0
        @total_writes = 0
        @total_transactions = 0
        @total_requests = 0
        @bytes_transferred = 0
        @coalescing_efficiency = 0.0
        @partition_conflicts = 0
        @host_to_device_bytes = 0
        @device_to_host_bytes = 0
        @host_transfer_cycles = 0
      end

      # Recalculate coalescing efficiency from current counts.
      def update_efficiency
        if @total_transactions > 0
          @coalescing_efficiency = @total_requests.to_f / @total_transactions
        end
      end
    end

    # -----------------------------------------------------------------------
    # KernelDescriptor -- what gets launched on the device
    # -----------------------------------------------------------------------
    #
    # === Two Worlds ===
    #
    # GPU-style devices (NVIDIA, AMD, Intel) receive a **program** with grid
    # and block dimensions -- "run this code on this many threads."
    #
    # Dataflow-style devices (TPU, NPU) receive an **operation** with input
    # and weight data -- "multiply these matrices" or "apply this activation."
    #
    # The same KernelDescriptor handles both by having fields for each style.
    # GPU devices use the program/grid/block fields. Dataflow devices use the
    # operation/input/weight fields.
    #
    # === GPU Example ===
    #
    #     # SAXPY: Y = alpha * X + Y
    #     kernel = KernelDescriptor.new(
    #       name: "saxpy",
    #       program: [limm(0, alpha), load(1, ...), fmul(2, 0, 1), ...],
    #       grid_dim: [256, 1, 1],     # 256 blocks
    #       block_dim: [256, 1, 1],    # 256 threads per block
    #     )
    #     # Total: 256 x 256 = 65,536 threads
    #
    # === Dataflow Example ===
    #
    #     # Matrix multiply: C = A x B
    #     kernel = KernelDescriptor.new(
    #       name: "matmul",
    #       operation: "matmul",
    #       input_data: a_matrix,
    #       weight_data: b_matrix,
    #     )
    KernelDescriptor = Data.define(
      :name,
      :kernel_id,
      :program,
      :grid_dim,
      :block_dim,
      :shared_mem_bytes,
      :registers_per_thread,
      :operation,
      :input_data,
      :weight_data,
      :output_address
    ) do
      def initialize(
        name: "unnamed",
        kernel_id: 0,
        program: nil,
        grid_dim: [1, 1, 1],
        block_dim: [32, 1, 1],
        shared_mem_bytes: 0,
        registers_per_thread: 32,
        operation: "",
        input_data: nil,
        weight_data: nil,
        output_address: 0
      )
        super
      end

      # Total number of threads across all blocks.
      def total_threads
        gx, gy, gz = grid_dim
        bx, by, bz = block_dim
        gx * gy * gz * bx * by * bz
      end

      # Total number of thread blocks in the grid.
      def total_blocks
        gx, gy, gz = grid_dim
        gx * gy * gz
      end

      # Number of threads in each block.
      def threads_per_block
        bx, by, bz = block_dim
        bx * by * bz
      end
    end

    # -----------------------------------------------------------------------
    # DeviceConfig -- full device specification
    # -----------------------------------------------------------------------
    #
    # === The Knobs That Define a Device ===
    #
    # Every accelerator is characterized by:
    # - How many compute units it has
    # - How much and how fast its memory is
    # - How it connects to the CPU
    # - How it distributes work
    #
    # === Memory Hierarchy Parameters ===
    #
    #     Host RAM --[host_bandwidth]--> Global Memory (VRAM)
    #                                         |
    #                                 [global_memory_bandwidth]
    #                                         |
    #                                    L2 Cache
    #                                         |
    #                                 Compute Units (shared memory)
    #                                         |
    #                                    Registers
    DeviceConfig = Data.define(
      :name,
      :architecture,
      :num_compute_units,
      :cu_config,
      :l2_cache_size,
      :l2_cache_latency,
      :l2_cache_associativity,
      :l2_cache_line_size,
      :global_memory_size,
      :global_memory_bandwidth,
      :global_memory_latency,
      :memory_channels,
      :host_bandwidth,
      :host_latency,
      :unified_memory,
      :max_concurrent_kernels,
      :work_distribution_policy
    ) do
      def initialize(
        name: "Generic Accelerator",
        architecture: "generic",
        num_compute_units: 4,
        cu_config: nil,
        l2_cache_size: 4 * 1024 * 1024,
        l2_cache_latency: 200,
        l2_cache_associativity: 16,
        l2_cache_line_size: 128,
        global_memory_size: 16 * 1024 * 1024 * 1024,
        global_memory_bandwidth: 1000.0,
        global_memory_latency: 400,
        memory_channels: 8,
        host_bandwidth: 64.0,
        host_latency: 1000,
        unified_memory: false,
        max_concurrent_kernels: 1,
        work_distribution_policy: "round_robin"
      )
        super
      end
    end

    # -----------------------------------------------------------------------
    # Vendor-specific configs
    # -----------------------------------------------------------------------

    # AMD Shader Engine -- mid-level grouping of CUs.
    #
    # AMD organizes CUs into Shader Engines, each sharing a geometry
    # processor and rasterizer. For compute workloads, the main effect
    # is that the Command Processor assigns work at the SE level first.
    ShaderEngineConfig = Data.define(:cus_per_engine, :shared_l1_size) do
      def initialize(cus_per_engine: 16, shared_l1_size: 32 * 1024)
        super
      end
    end

    # AMD-specific config with Shader Engine hierarchy.
    AmdGPUConfig = Data.define(
      :name, :architecture, :num_compute_units, :cu_config,
      :l2_cache_size, :l2_cache_latency, :l2_cache_associativity,
      :l2_cache_line_size, :global_memory_size, :global_memory_bandwidth,
      :global_memory_latency, :memory_channels, :host_bandwidth,
      :host_latency, :unified_memory, :max_concurrent_kernels,
      :work_distribution_policy, :num_shader_engines, :se_config,
      :infinity_cache_size, :infinity_cache_latency, :num_aces
    ) do
      def initialize(
        name: "AMD RX 7900 XTX",
        architecture: "amd_cu",
        num_compute_units: 96,
        cu_config: nil,
        l2_cache_size: 6 * 1024 * 1024,
        l2_cache_latency: 150,
        l2_cache_associativity: 16,
        l2_cache_line_size: 128,
        global_memory_size: 24 * 1024 * 1024 * 1024,
        global_memory_bandwidth: 960.0,
        global_memory_latency: 350,
        memory_channels: 6,
        host_bandwidth: 32.0,
        host_latency: 1000,
        unified_memory: false,
        max_concurrent_kernels: 8,
        work_distribution_policy: "round_robin",
        num_shader_engines: 6,
        se_config: nil,
        infinity_cache_size: 96 * 1024 * 1024,
        infinity_cache_latency: 50,
        num_aces: 4
      )
        se_config ||= ShaderEngineConfig.new
        super
      end
    end

    # Intel Xe-Slice -- mid-level grouping of Xe-Cores.
    XeSliceConfig = Data.define(:xe_cores_per_slice, :l1_cache_per_slice) do
      def initialize(xe_cores_per_slice: 4, l1_cache_per_slice: 192 * 1024)
        super
      end
    end

    # Intel-specific config with Xe-Slice hierarchy.
    IntelGPUConfig = Data.define(
      :name, :architecture, :num_compute_units, :cu_config,
      :l2_cache_size, :l2_cache_latency, :l2_cache_associativity,
      :l2_cache_line_size, :global_memory_size, :global_memory_bandwidth,
      :global_memory_latency, :memory_channels, :host_bandwidth,
      :host_latency, :unified_memory, :max_concurrent_kernels,
      :work_distribution_policy, :num_xe_slices, :slice_config
    ) do
      def initialize(
        name: "Intel Arc A770",
        architecture: "intel_xe_core",
        num_compute_units: 32,
        cu_config: nil,
        l2_cache_size: 16 * 1024 * 1024,
        l2_cache_latency: 180,
        l2_cache_associativity: 16,
        l2_cache_line_size: 128,
        global_memory_size: 16 * 1024 * 1024 * 1024,
        global_memory_bandwidth: 512.0,
        global_memory_latency: 350,
        memory_channels: 4,
        host_bandwidth: 32.0,
        host_latency: 1000,
        unified_memory: false,
        max_concurrent_kernels: 16,
        work_distribution_policy: "round_robin",
        num_xe_slices: 8,
        slice_config: nil
      )
        slice_config ||= XeSliceConfig.new
        super
      end
    end

    # One ICI link to another TPU chip.
    ICILink = Data.define(:target_chip_id, :bandwidth, :latency) do
      def initialize(target_chip_id: 0, bandwidth: 500.0, latency: 500)
        super
      end
    end

    # TPU-specific config with Vector/Scalar units and ICI.
    TPUConfig = Data.define(
      :name, :architecture, :num_compute_units, :cu_config,
      :l2_cache_size, :l2_cache_latency, :l2_cache_associativity,
      :l2_cache_line_size, :global_memory_size, :global_memory_bandwidth,
      :global_memory_latency, :memory_channels, :host_bandwidth,
      :host_latency, :unified_memory, :max_concurrent_kernels,
      :work_distribution_policy, :vector_unit_width, :scalar_registers,
      :transpose_unit, :ici_links
    ) do
      def initialize(
        name: "Google TPU v4",
        architecture: "google_mxu",
        num_compute_units: 1,
        cu_config: nil,
        l2_cache_size: 0,
        l2_cache_latency: 0,
        l2_cache_associativity: 0,
        l2_cache_line_size: 128,
        global_memory_size: 32 * 1024 * 1024 * 1024,
        global_memory_bandwidth: 1200.0,
        global_memory_latency: 300,
        memory_channels: 4,
        host_bandwidth: 500.0,
        host_latency: 500,
        unified_memory: false,
        max_concurrent_kernels: 1,
        work_distribution_policy: "sequential",
        vector_unit_width: 128,
        scalar_registers: 32,
        transpose_unit: true,
        ici_links: []
      )
        super
      end
    end

    # Apple ANE-specific config with DMA and SRAM.
    #
    # The ANE is unique: it shares unified memory with CPU and GPU,
    # eliminating the PCIe transfer bottleneck entirely. The 'copy'
    # operation just remaps page tables -- zero cycles, zero bytes moved.
    ANEConfig = Data.define(
      :name, :architecture, :num_compute_units, :cu_config,
      :l2_cache_size, :l2_cache_latency, :l2_cache_associativity,
      :l2_cache_line_size, :global_memory_size, :global_memory_bandwidth,
      :global_memory_latency, :memory_channels, :host_bandwidth,
      :host_latency, :unified_memory, :max_concurrent_kernels,
      :work_distribution_policy, :shared_sram_size, :sram_bandwidth,
      :sram_latency, :dma_channels, :dma_bandwidth
    ) do
      def initialize(
        name: "Apple M3 Max ANE",
        architecture: "apple_ane_core",
        num_compute_units: 16,
        cu_config: nil,
        l2_cache_size: 0,
        l2_cache_latency: 0,
        l2_cache_associativity: 0,
        l2_cache_line_size: 128,
        global_memory_size: 128 * 1024 * 1024 * 1024,
        global_memory_bandwidth: 200.0,
        global_memory_latency: 100,
        memory_channels: 8,
        host_bandwidth: 200.0,
        host_latency: 0,
        unified_memory: true,
        max_concurrent_kernels: 1,
        work_distribution_policy: "scheduled",
        shared_sram_size: 32 * 1024 * 1024,
        sram_bandwidth: 1000.0,
        sram_latency: 5,
        dma_channels: 4,
        dma_bandwidth: 100.0
      )
        super
      end
    end

    # -----------------------------------------------------------------------
    # DeviceTrace -- cycle-by-cycle visibility into the whole device
    # -----------------------------------------------------------------------
    #
    # === Why Trace the Whole Device? ===
    #
    # At the compute unit level (Layer 7), traces show what one SM/CU is doing.
    # At the device level, we need to see all compute units simultaneously, plus
    # the memory system and work distributor. This is the information that tools
    # like NVIDIA Nsight Systems show -- the big picture of device utilization.
    #
    # Key questions a DeviceTrace answers:
    # - How many compute units are busy vs idle?
    # - Is the memory system a bottleneck (high bandwidth utilization)?
    # - Is the work distributor keeping up (many pending blocks)?
    # - What's the overall device occupancy?
    DeviceTrace = Data.define(
      :cycle,
      :device_name,
      :distributor_actions,
      :pending_blocks,
      :active_blocks,
      :cu_traces,
      :l2_hits,
      :l2_misses,
      :memory_transactions,
      :memory_bandwidth_used,
      :total_active_warps,
      :device_occupancy,
      :flops_this_cycle
    ) do
      def initialize(
        cycle:,
        device_name:,
        distributor_actions: [],
        pending_blocks: 0,
        active_blocks: 0,
        cu_traces: [],
        l2_hits: 0,
        l2_misses: 0,
        memory_transactions: 0,
        memory_bandwidth_used: 0.0,
        total_active_warps: 0,
        device_occupancy: 0.0,
        flops_this_cycle: 0
      )
        super
      end

      # Human-readable summary of this cycle.
      #
      # Example output:
      #
      #     [Cycle 10] NVIDIA H100 -- 45.2% occupancy
      #       Distributor: Block 42 -> SM 7, Block 43 -> SM 12
      #       Pending: 890 blocks, Active: 1056 blocks
      #       L2: 342 hits, 12 misses (96.6% hit rate)
      #       Memory: 8 transactions, 45.2% bandwidth
      #       Active warps: 4234
      def format
        lines = [
          "[Cycle #{cycle}] #{device_name} " \
          "-- #{(device_occupancy * 100).round(1)}% occupancy"
        ]

        unless distributor_actions.empty?
          actions_str = distributor_actions.join(", ")
          lines << "  Distributor: #{actions_str}"
        end

        lines << "  Pending: #{pending_blocks} blocks, " \
                 "Active: #{active_blocks} blocks"

        total_l2 = l2_hits + l2_misses
        if total_l2 > 0
          hit_rate = (l2_hits.to_f / total_l2 * 100).round(1)
          lines << "  L2: #{l2_hits} hits, #{l2_misses} misses " \
                   "(#{hit_rate}% hit rate)"
        end

        lines << "  Memory: #{memory_transactions} transactions, " \
                 "#{(memory_bandwidth_used * 100).round(1)}% bandwidth"

        lines << "  Active warps: #{total_active_warps}"

        lines.join("\n")
      end
    end

    # -----------------------------------------------------------------------
    # DeviceStats -- aggregate metrics across the entire simulation
    # -----------------------------------------------------------------------
    #
    # === Performance Analysis ===
    #
    # These stats answer the key performance questions:
    #
    # 1. **Compute utilization**: Are the compute units busy or sitting idle?
    # 2. **Memory bandwidth utilization**: Is the memory system saturated?
    # 3. **Load imbalance**: Are some CUs doing more work than others?
    # 4. **L2 effectiveness**: Is the cache helping?
    class DeviceStats
      attr_accessor :total_cycles, :active_cycles, :idle_cycles,
        :total_flops, :achieved_tflops, :peak_tflops,
        :compute_utilization, :global_memory_stats,
        :l2_hit_rate, :memory_bandwidth_utilization,
        :total_kernels_launched, :total_blocks_dispatched,
        :avg_blocks_per_cu, :load_imbalance,
        :per_cu_active_cycles, :per_cu_occupancy

      def initialize(
        total_cycles: 0,
        active_cycles: 0,
        idle_cycles: 0,
        total_flops: 0,
        achieved_tflops: 0.0,
        peak_tflops: 0.0,
        compute_utilization: 0.0,
        global_memory_stats: nil,
        l2_hit_rate: 0.0,
        memory_bandwidth_utilization: 0.0,
        total_kernels_launched: 0,
        total_blocks_dispatched: 0,
        avg_blocks_per_cu: 0.0,
        load_imbalance: 0.0,
        per_cu_active_cycles: [],
        per_cu_occupancy: []
      )
        @total_cycles = total_cycles
        @active_cycles = active_cycles
        @idle_cycles = idle_cycles
        @total_flops = total_flops
        @achieved_tflops = achieved_tflops
        @peak_tflops = peak_tflops
        @compute_utilization = compute_utilization
        @global_memory_stats = global_memory_stats || GlobalMemoryStats.new
        @l2_hit_rate = l2_hit_rate
        @memory_bandwidth_utilization = memory_bandwidth_utilization
        @total_kernels_launched = total_kernels_launched
        @total_blocks_dispatched = total_blocks_dispatched
        @avg_blocks_per_cu = avg_blocks_per_cu
        @load_imbalance = load_imbalance
        @per_cu_active_cycles = per_cu_active_cycles
        @per_cu_occupancy = per_cu_occupancy
      end
    end
  end
end

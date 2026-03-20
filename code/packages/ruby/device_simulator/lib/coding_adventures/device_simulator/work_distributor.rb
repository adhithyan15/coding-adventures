# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Work Distributor -- assigns work to compute units.
# ---------------------------------------------------------------------------
#
# === Three Distribution Strategies ===
#
# Different accelerator architectures distribute work in fundamentally
# different ways. This module implements all three:
#
# 1. **GPU Block Distributor** (NVIDIA, AMD, Intel)
#    - Takes a kernel launch with grid/block dimensions
#    - Decomposes into thread blocks
#    - Assigns blocks to compute units that have free resources
#    - Continues assigning as CUs complete blocks (multi-wave)
#
# 2. **TPU Sequencer** (Google TPU)
#    - Takes HLO operations (matmul, add, relu, etc.)
#    - Tiles large operations to fit the MXU
#    - Pipelines through Scalar -> MXU -> Vector units
#    - One operation at a time (no thread blocks)
#
# 3. **ANE Schedule Replayer** (Apple Neural Engine)
#    - Compiler generates a complete execution schedule at compile time
#    - The "distributor" simply replays the schedule
#    - No dynamic scheduling decisions -- everything is predetermined
#    - DMA loads data to SRAM, cores process it, DMA stores results

module CodingAdventures
  module DeviceSimulator
    # =====================================================================
    # GPU Block Distributor
    # =====================================================================
    #
    # Distributes thread blocks to compute units.
    #
    # Used by NVIDIA (GigaThread Engine), AMD (Command Processor),
    # and Intel (Command Streamer). The same algorithm works for all
    # three -- they differ only in CU-level resource limits.
    #
    # === Distribution Policies ===
    #
    #     round_robin:  Cycle through CUs evenly. Fair, simple.
    #     fill_first:   Fill one CU before moving to next. Max occupancy per CU.
    #     least_loaded: Assign to CU with fewest active warps. Best balance.
    class GPUWorkDistributor
      attr_reader :total_dispatched

      # @param compute_units [Array] The CUs to distribute work to.
      # @param policy [String] Distribution policy name.
      def initialize(compute_units, policy: "round_robin")
        @cus = compute_units
        @policy = policy
        @pending = []
        @rr_index = 0 # For round-robin policy
        @total_dispatched = 0
      end

      # Number of blocks waiting to be assigned.
      def pending_count
        @pending.length
      end

      # Decompose a kernel into thread blocks and queue them.
      #
      # Each thread block becomes a WorkItem. The block's position in
      # the grid is encoded in the work_id (we use a linear index).
      #
      # === Grid Linearization ===
      #
      # A 3D grid (gx, gy, gz) is linearized:
      #     block_id = bz * gx * gy + by * gx + bx
      #
      # This is the same order CUDA uses for blockIdx.
      #
      # @param kernel [KernelDescriptor] The kernel to decompose.
      def submit_kernel(kernel)
        kernel.total_blocks.times do |block_id|
          work = ComputeUnit::WorkItem.new(
            work_id: block_id,
            program: kernel.program,
            thread_count: kernel.threads_per_block,
            registers_per_thread: kernel.registers_per_thread,
            shared_mem_bytes: kernel.shared_mem_bytes
          )
          @pending << work
        end
      end

      # Try to assign pending blocks to available CUs.
      #
      # Returns a list of human-readable assignment descriptions.
      # Each entry looks like: "Block 42 -> SM 7"
      #
      # === Algorithm ===
      #
      # For each CU (in policy order):
      #     While there are pending blocks:
      #         Try to dispatch the next block to this CU
      #         If CU rejects it (ResourceError), move to next CU
      #         If CU accepts it, log the assignment
      #
      # @return [Array<String>] Assignment descriptions.
      def step
        return [] if @pending.empty?

        assignments = []
        order = cu_order

        order.each do |cu|
          while @pending.any?
            block = @pending.first
            begin
              cu.dispatch(block)
              @pending.shift
              @total_dispatched += 1
              assignments << "Block #{block.work_id} -> #{cu.name}"
            rescue ComputeUnit::ResourceError, StandardError
              # CU can't accept this block (full) -- try next CU
              break
            end
          end
        end

        assignments
      end

      # Clear all pending work and reset counters.
      def reset
        @pending.clear
        @rr_index = 0
        @total_dispatched = 0
      end

      private

      # Return CUs in the order dictated by the policy.
      #
      # round_robin:  Start from rr_index, wrap around.
      # fill_first:   Just return in order (fill CU 0 first, then CU 1, ...).
      # least_loaded: Sort by idle status (idle CUs first).
      def cu_order
        n = @cus.length
        return [] if n == 0

        if @policy == "fill_first"
          return @cus.dup
        end

        if @policy == "least_loaded"
          return @cus.sort_by { |cu| cu.idle? ? 0 : 1 }
        end

        # Default: round_robin
        ordered = n.times.map { |i| @cus[(@rr_index + i) % n] }
        @rr_index = (@rr_index + 1) % n
        ordered
      end
    end

    # =====================================================================
    # TileOperation -- a single tile operation in the TPU pipeline
    # =====================================================================
    class TileOperation
      attr_accessor :tile_id, :operation, :input_data, :weight_data,
        :status, :cycles_remaining

      def initialize(tile_id:, operation:, input_data: nil, weight_data: nil,
        status: "pending", cycles_remaining: 0)
        @tile_id = tile_id
        @operation = operation
        @input_data = input_data
        @weight_data = weight_data
        @status = status
        @cycles_remaining = cycles_remaining
      end
    end

    # =====================================================================
    # TPU Sequencer
    # =====================================================================
    #
    # Orchestrates operations through Scalar + Vector + MXU units.
    #
    # === TPU Execution Pipeline ===
    #
    # The TPU processes operations through a three-stage pipeline:
    #
    #     Scalar Unit -> MXU -> Vector Unit
    #
    # Stage 1 (Scalar): Prepare addresses, loop counters, control flow.
    # Stage 2 (MXU):    The heavy lifting -- matrix multiply on the systolic array.
    # Stage 3 (Vector): Post-processing -- activation functions, normalization.
    #
    # These three stages overlap: while the MXU crunches tile N, the Vector
    # unit processes tile N-1, and the Scalar unit prepares tile N+1.
    #
    #     Time ->
    #     Scalar: [tile 0] [tile 1] [tile 2] [tile 3] ...
    #     MXU:           [tile 0] [tile 1] [tile 2] ...
    #     Vector:               [tile 0] [tile 1] ...
    class TPUSequencer
      attr_reader :total_dispatched

      # @param mxu [Object] The MXU compute unit.
      # @param mxu_size [Integer] The systolic array dimension (e.g., 128).
      # @param vector_width [Integer] Width of the vector unit.
      # @param scalar_latency [Integer] Cycles for scalar setup per tile.
      # @param mxu_latency [Integer] Cycles for MXU processing per tile.
      # @param vector_latency [Integer] Cycles for vector post-processing per tile.
      def initialize(mxu, mxu_size: 128, vector_width: 128,
        scalar_latency: 5, mxu_latency: 20, vector_latency: 10)
        @mxu = mxu
        @mxu_size = mxu_size
        @vector_width = vector_width
        @scalar_latency = scalar_latency
        @mxu_latency = mxu_latency
        @vector_latency = vector_latency

        @pending = []
        @scalar_tile = nil
        @mxu_tile = nil
        @vector_tile = nil
        @completed = []
        @total_dispatched = 0
      end

      # Number of tiles waiting to be processed.
      def pending_count
        @pending.length
      end

      # Tile a large operation and queue the tiles.
      #
      # === Tiling ===
      #
      # If the input matrix is 256x256 but the MXU is 128x128, we need
      # to split it into 4 tiles:
      #
      #     Tile 0: rows 0-127,   cols 0-127
      #     Tile 1: rows 0-127,   cols 128-255
      #     Tile 2: rows 128-255, cols 0-127
      #     Tile 3: rows 128-255, cols 128-255
      #
      # @param kernel [KernelDescriptor] The operation to tile.
      def submit_operation(kernel)
        input_data = kernel.input_data || [[0.0]]
        weight_data = kernel.weight_data || [[0.0]]

        rows = input_data.length
        cols = weight_data[0] ? weight_data[0].length : 1
        mxu = @mxu_size

        num_row_tiles = [1, (rows + mxu - 1) / mxu].max
        num_col_tiles = [1, (cols + mxu - 1) / mxu].max

        tile_id = 0
        num_row_tiles.times do
          num_col_tiles.times do
            tile = TileOperation.new(
              tile_id: tile_id,
              operation: kernel.operation.empty? ? "matmul" : kernel.operation,
              input_data: input_data,
              weight_data: weight_data,
              cycles_remaining: @scalar_latency
            )
            @pending << tile
            tile_id += 1
          end
        end
      end

      # Advance the pipeline by one cycle.
      #
      # @return [Array<String>] Descriptions of what happened this cycle.
      def step
        actions = []

        # Vector stage: finish processing
        if @vector_tile
          @vector_tile.cycles_remaining -= 1
          if @vector_tile.cycles_remaining <= 0
            @vector_tile.status = "done"
            @completed << @vector_tile
            actions << "Vector: completed tile #{@vector_tile.tile_id}"
            @vector_tile = nil
          end
        end

        # MXU stage: process matrix multiply
        if @mxu_tile
          @mxu_tile.cycles_remaining -= 1
          if @mxu_tile.cycles_remaining <= 0
            @mxu_tile.status = "vector"
            @mxu_tile.cycles_remaining = @vector_latency
            # Move to vector stage (if free)
            if @vector_tile.nil?
              @vector_tile = @mxu_tile
              @mxu_tile = nil
              actions << "MXU -> Vector: tile #{@vector_tile.tile_id}"
            end
          end
        end

        # Scalar stage: prepare next tile
        if @scalar_tile
          @scalar_tile.cycles_remaining -= 1
          if @scalar_tile.cycles_remaining <= 0
            @scalar_tile.status = "mxu"
            @scalar_tile.cycles_remaining = @mxu_latency
            # Move to MXU stage (if free)
            if @mxu_tile.nil?
              @mxu_tile = @scalar_tile
              @scalar_tile = nil
              @total_dispatched += 1
              actions << "Scalar -> MXU: tile #{@mxu_tile.tile_id}"
            end
          end
        end

        # Feed from pending queue to scalar stage
        if @scalar_tile.nil? && @pending.any?
          @scalar_tile = @pending.shift
          @scalar_tile.status = "scalar"
          @scalar_tile.cycles_remaining = @scalar_latency
          actions << "Scalar: started tile #{@scalar_tile.tile_id}"
        end

        actions
      end

      # True when all tiles are processed.
      def idle?
        @pending.empty? && @scalar_tile.nil? && @mxu_tile.nil? && @vector_tile.nil?
      end

      # Clear all state.
      def reset
        @pending.clear
        @scalar_tile = nil
        @mxu_tile = nil
        @vector_tile = nil
        @completed.clear
        @total_dispatched = 0
      end
    end

    # =====================================================================
    # ScheduleEntry -- one step in a compiler-generated ANE schedule
    # =====================================================================
    #
    # The CoreML compiler pre-determines everything:
    # - Which core processes which tile
    # - When DMA loads happen
    # - When DMA stores happen
    # - The exact order of operations
    ScheduleEntry = Data.define(:cycle, :action, :core_id, :description, :data, :weights) do
      def initialize(cycle:, action:, core_id: -1, description: "", data: nil, weights: nil)
        super
      end
    end

    # =====================================================================
    # ANE Schedule Replayer
    # =====================================================================
    #
    # Replays a compiler-generated execution schedule.
    #
    # === Why No Dynamic Scheduling? ===
    #
    # Unlike GPUs (which have hardware schedulers that decide at runtime
    # which warp to execute), the Apple Neural Engine relies entirely on
    # the compiler. The CoreML compiler analyzes the neural network graph,
    # determines the optimal tiling strategy, generates DMA transfer
    # schedules, and produces a fixed execution plan.
    #
    # This makes the hardware simpler (no complex scheduler) and more
    # power-efficient (no scheduling overhead), but less flexible --
    # the ANE can only run workloads the compiler knows how to schedule.
    #
    # === Schedule Structure ===
    #
    #     Step 0: DMA load input tile 0 -> Core 0 SRAM
    #     Step 1: DMA load weights -> Core 0 SRAM
    #     Step 2: Core 0 compute (MAC array)
    #     Step 3: Core 0 activate (ReLU)
    #     Step 4: DMA store result -> output buffer
    #     Step 5: DMA load input tile 1 -> Core 1 SRAM (overlaps!)
    #     ...
    class ANEScheduleReplayer
      attr_reader :total_dispatched

      # @param compute_units [Array] The ANE cores to schedule onto.
      # @param dma_latency [Integer] Cycles per DMA transfer.
      # @param compute_latency [Integer] Cycles per MAC array computation.
      # @param activate_latency [Integer] Cycles per activation function.
      def initialize(compute_units, dma_latency: 10, compute_latency: 20,
        activate_latency: 5)
        @cus = compute_units
        @dma_latency = dma_latency
        @compute_latency = compute_latency
        @activate_latency = activate_latency

        @schedule = []
        @current_step = 0
        @total_dispatched = 0
      end

      # Number of schedule steps remaining.
      def pending_count
        [0, @schedule.length - @current_step].max
      end

      # Generate a schedule from a kernel descriptor.
      #
      # The compiler (us, acting as the compiler) determines:
      # 1. How to tile the input across available cores
      # 2. When to load data via DMA
      # 3. When each core computes
      # 4. When to apply activation functions
      # 5. When to store results via DMA
      #
      # @param kernel [KernelDescriptor] The operation to schedule.
      def submit_operation(kernel)
        input_data = kernel.input_data || [[0.0]]
        weight_data = kernel.weight_data || [[0.0]]

        num_cores = @cus.length
        rows = input_data.length

        cycle = 0
        [num_cores, rows].min.times do |core_id|
          # DMA load input
          @schedule << ScheduleEntry.new(
            cycle: cycle,
            action: "dma_load",
            core_id: core_id,
            description: "DMA load input tile -> Core #{core_id}",
            data: input_data
          )
          cycle += @dma_latency

          # DMA load weights
          @schedule << ScheduleEntry.new(
            cycle: cycle,
            action: "dma_load",
            core_id: core_id,
            description: "DMA load weights -> Core #{core_id}",
            weights: weight_data
          )
          cycle += @dma_latency

          # Compute
          @schedule << ScheduleEntry.new(
            cycle: cycle,
            action: "compute",
            core_id: core_id,
            description: "Core #{core_id}: MAC array compute"
          )
          cycle += @compute_latency

          # Activate
          @schedule << ScheduleEntry.new(
            cycle: cycle,
            action: "activate",
            core_id: core_id,
            description: "Core #{core_id}: activation (ReLU)"
          )
          cycle += @activate_latency

          # DMA store
          @schedule << ScheduleEntry.new(
            cycle: cycle,
            action: "dma_store",
            core_id: core_id,
            description: "DMA store result from Core #{core_id}"
          )
          cycle += @dma_latency
        end
      end

      # Execute the next step in the pre-computed schedule.
      #
      # @return [Array<String>] Descriptions of what happened this cycle.
      def step
        return [] if @current_step >= @schedule.length

        entry = @schedule[@current_step]
        @current_step += 1
        @total_dispatched += 1

        [entry.description]
      end

      # True when the entire schedule has been replayed.
      def idle?
        @current_step >= @schedule.length
      end

      # Clear the schedule and reset.
      def reset
        @schedule.clear
        @current_step = 0
        @total_dispatched = 0
      end
    end
  end
end

# frozen_string_literal: true

# ---------------------------------------------------------------------------
# StreamingMultiprocessor -- NVIDIA SM simulator.
# ---------------------------------------------------------------------------
#
# === What is a Streaming Multiprocessor? ===
#
# The SM is the heart of NVIDIA's GPU architecture. Every NVIDIA GPU -- from
# the GeForce in your laptop to the H100 in a data center -- is built from
# SMs. Each SM is a self-contained compute unit that can independently
# execute work without coordination with other SMs.
#
# An SM contains:
# - **Warp schedulers** (4 on modern GPUs) that pick ready warps to execute
# - **WarpEngines** (one per scheduler) that execute 32-thread warps
# - **Register file** (256 KB, 65536 registers) partitioned among warps
# - **Shared memory** (up to 228 KB) for inter-thread communication
# - **L1 cache** (often shares capacity with shared memory)
#
# === The Key Innovation: Latency Hiding ===
#
# CPUs hide latency with deep pipelines, out-of-order execution, and branch
# prediction -- complex hardware that's expensive in transistors and power.
#
# GPUs take the opposite approach: have MANY warps, and when one stalls,
# switch to another. A single SM can have 48-64 warps resident. When warp 0
# stalls on a memory access (~400 cycles), the scheduler instantly switches
# to warp 1. By the time it has cycled through enough warps, warp 0's data
# has arrived.
#
#     CPU strategy:  Make one thread FAST (deep pipeline, speculation, OoO)
#     GPU strategy:  Have MANY threads, switch instantly to hide latency
#
# === Architecture Diagram ===
#
#     StreamingMultiprocessor
#     +---------------------------------------------------------------+
#     |                                                               |
#     |  Warp Scheduler 0        Warp Scheduler 1                     |
#     |  +------------------+   +------------------+                  |
#     |  | w0: READY        |   | w1: STALLED      |                  |
#     |  | w4: READY        |   | w5: READY        |                  |
#     |  | w8: COMPLETED    |   | w9: RUNNING      |                  |
#     |  +--------+---------+   +--------+---------+                  |
#     |           |                      |                            |
#     |           v                      v                            |
#     |  +------------------+   +------------------+                  |
#     |  | WarpEngine 0     |   | WarpEngine 1     |                  |
#     |  | (32 threads)     |   | (32 threads)     |                  |
#     |  +------------------+   +------------------+                  |
#     |                                                               |
#     |  Shared Resources:                                            |
#     |  +-----------------------------------------------------------+|
#     |  | Register File: 256 KB (65,536 x 32-bit registers)         ||
#     |  | Shared Memory: 96 KB (configurable split with L1 cache)   ||
#     |  +-----------------------------------------------------------+|
#     +---------------------------------------------------------------+

module CodingAdventures
  module ComputeUnit
    # -----------------------------------------------------------------------
    # SMConfig -- all tunable parameters for an NVIDIA-style SM
    # -----------------------------------------------------------------------
    #
    # Real-world SM configurations (for reference):
    #
    #     Parameter             | Volta (V100) | Ampere (A100) | Hopper (H100)
    #     ----------------------+--------------+---------------+--------------
    #     Warp schedulers       | 4            | 4             | 4
    #     Max warps per SM      | 64           | 64            | 64
    #     Max threads per SM    | 2048         | 2048          | 2048
    #     CUDA cores (FP32)     | 64           | 64            | 128
    #     Register file         | 256 KB       | 256 KB        | 256 KB
    #     Shared memory         | 96 KB        | 164 KB        | 228 KB
    #     L1 cache              | combined w/ shared mem
    #
    # Our default configuration models a Volta-class SM with reduced sizes
    # for faster simulation.
    SMConfig = Data.define(
      :num_schedulers,
      :warp_width,
      :max_warps,
      :max_threads,
      :max_blocks,
      :scheduling_policy,
      :register_file_size,
      :max_registers_per_thread,
      :shared_memory_size,
      :l1_cache_size,
      :instruction_cache_size,
      :float_format,
      :isa,
      :memory_latency_cycles,
      :barrier_enabled
    ) do
      def initialize(
        num_schedulers: 4,
        warp_width: 32,
        max_warps: 48,
        max_threads: 1536,
        max_blocks: 16,
        scheduling_policy: :gto,
        register_file_size: 65_536,
        max_registers_per_thread: 255,
        shared_memory_size: 98_304,
        l1_cache_size: 32_768,
        instruction_cache_size: 131_072,
        float_format: FpArithmetic::FP32,
        isa: nil,
        memory_latency_cycles: 200,
        barrier_enabled: true
      )
        super(
          num_schedulers: num_schedulers,
          warp_width: warp_width,
          max_warps: max_warps,
          max_threads: max_threads,
          max_blocks: max_blocks,
          scheduling_policy: scheduling_policy,
          register_file_size: register_file_size,
          max_registers_per_thread: max_registers_per_thread,
          shared_memory_size: shared_memory_size,
          l1_cache_size: l1_cache_size,
          instruction_cache_size: instruction_cache_size,
          float_format: float_format,
          isa: isa || GpuCore::GenericISA.new,
          memory_latency_cycles: memory_latency_cycles,
          barrier_enabled: barrier_enabled
        )
      end
    end

    # -----------------------------------------------------------------------
    # WarpSlot -- tracks one warp's state in the scheduler
    # -----------------------------------------------------------------------
    #
    # Each WarpSlot tracks the state of one warp -- whether it's ready to
    # execute, stalled waiting for memory, completed, etc. The scheduler
    # scans these slots to find ready warps.
    #
    # === Warp Lifecycle ===
    #
    #     1. dispatch() creates a WarpSlot in :ready state
    #     2. Scheduler picks it -> :running
    #     3. After execution:
    #        - If LOAD/STORE: transition to :stalled_memory for N cycles
    #        - If HALT: transition to :completed
    #        - Otherwise: back to :ready
    #     4. After stall countdown expires: back to :ready
    class WarpSlot
      attr_accessor :state, :stall_counter, :age
      attr_reader :warp_id, :work_id, :engine, :registers_used

      def initialize(warp_id:, work_id:, state:, engine:, stall_counter: 0, age: 0, registers_used: 0)
        @warp_id = warp_id
        @work_id = work_id
        @state = state
        @engine = engine
        @stall_counter = stall_counter
        @age = age
        @registers_used = registers_used
      end
    end

    # -----------------------------------------------------------------------
    # WarpScheduler -- picks which warp to issue each cycle
    # -----------------------------------------------------------------------
    #
    # === How Warp Scheduling Works ===
    #
    # On each clock cycle, the scheduler:
    # 1. Scans all warp slots assigned to it
    # 2. Decrements stall counters for stalled warps
    # 3. Transitions warps whose stalls have resolved to :ready
    # 4. Picks one :ready warp according to the scheduling policy
    # 5. Returns that warp for execution
    #
    # === Scheduling Policies ===
    #
    # ROUND_ROBIN:
    #     Simply rotates through warps: 0, 1, 2, ..., wrap around.
    #     Skips non-ready warps. Fair but doesn't optimize for locality.
    #
    # GTO (Greedy-Then-Oldest):
    #     Keeps issuing from the same warp until it stalls, then picks
    #     the oldest ready warp. This improves cache locality because
    #     the same warp's instructions tend to access nearby memory.
    class WarpScheduler
      attr_reader :scheduler_id, :policy, :warps

      def initialize(scheduler_id:, policy:)
        @scheduler_id = scheduler_id
        @policy = policy
        @warps = []
        @rr_index = 0
        @last_issued = nil
      end

      # Add a warp to this scheduler's management.
      def add_warp(slot)
        @warps << slot
      end

      # Decrement stall counters and transition stalled warps to :ready.
      # Called once per cycle before scheduling.
      def tick_stalls
        @warps.each do |warp|
          if warp.stall_counter > 0
            warp.stall_counter -= 1
            if warp.stall_counter == 0 &&
               (warp.state == :stalled_memory || warp.state == :stalled_dependency)
              warp.state = :ready
            end
          end

          # Age all non-completed warps (for :oldest_first / :gto)
          if warp.state != :completed && warp.state != :running
            warp.age += 1
          end
        end
      end

      # Select a ready warp according to the scheduling policy.
      #
      # @return [WarpSlot, nil] The selected WarpSlot, or nil if no warps are ready.
      def pick_warp
        ready = @warps.select { |w| w.state == :ready }
        return nil if ready.empty?

        case @policy
        when :round_robin then pick_round_robin(ready)
        when :gto then pick_gto(ready)
        when :lrr then pick_round_robin(ready)
        when :oldest_first then pick_oldest_first(ready)
        when :greedy then pick_oldest_first(ready)
        else ready[0]
        end
      end

      # Record that a warp was just issued (for GTO policy).
      def mark_issued(warp_id)
        @last_issued = warp_id
        @warps.each do |w|
          if w.warp_id == warp_id
            w.age = 0
            break
          end
        end
      end

      # Clear all warps from this scheduler.
      def reset
        @warps.clear
        @rr_index = 0
        @last_issued = nil
      end

      private

      # Round-robin: rotate through warps in order.
      def pick_round_robin(ready)
        all_ids = @warps.map(&:warp_id)
        all_ids.length.times do |i|
          idx = (@rr_index + i) % all_ids.length
          target_id = all_ids[idx]
          ready.each do |w|
            if w.warp_id == target_id
              @rr_index = (idx + 1) % all_ids.length
              return w
            end
          end
        end
        ready[0]
      end

      # GTO: keep issuing same warp until it stalls, then oldest.
      def pick_gto(ready)
        if @last_issued
          ready.each do |w|
            return w if w.warp_id == @last_issued
          end
        end
        pick_oldest_first(ready)
      end

      # Oldest first: pick the warp that has been waiting longest.
      def pick_oldest_first(ready)
        ready.max_by(&:age)
      end
    end

    # -----------------------------------------------------------------------
    # StreamingMultiprocessor -- the main SM simulator
    # -----------------------------------------------------------------------
    #
    # Manages multiple warps executing thread blocks, with a configurable
    # warp scheduler, shared memory, and register file partitioning.
    #
    # === Usage Pattern ===
    #
    #     1. Create SM with config and clock
    #     2. Dispatch one or more WorkItems (thread blocks)
    #     3. Call step() or run() to simulate execution
    #     4. Read traces to understand what happened
    #
    # === How dispatch() Works ===
    #
    # When a thread block is dispatched to the SM:
    #
    #     1. Check resources: enough registers? shared memory? warp slots?
    #     2. Decompose the block into warps (every 32 threads = 1 warp)
    #     3. Allocate registers for each warp
    #     4. Reserve shared memory for the block
    #     5. Create WarpEngine instances for each warp
    #     6. Add warp slots to the schedulers (round-robin distribution)
    class StreamingMultiprocessor
      attr_reader :config, :shared_memory, :warp_slots

      def initialize(config, clock)
        @config = config
        @clock = clock
        @cycle = 0

        # Shared memory for the SM
        @shared_memory = SharedMemory.new(size: config.shared_memory_size)
        @shared_memory_used = 0

        # Register file tracking
        @registers_allocated = 0

        # Warp schedulers -- one per scheduler slot
        @schedulers = Array.new(config.num_schedulers) do |i|
          WarpScheduler.new(scheduler_id: i, policy: config.scheduling_policy)
        end

        # Track all active warp slots
        @warp_slots = []
        @next_warp_id = 0
        @active_blocks = []
      end

      # --- Properties ---

      def name
        "SM"
      end

      def architecture
        :nvidia_sm
      end

      # True if no active warps remain.
      def idle?
        @warp_slots.empty? || @warp_slots.all? { |w| w.state == :completed }
      end

      # Current occupancy: active (non-completed) warps / max warps.
      #
      # Occupancy is the key performance metric for GPU kernels. Low
      # occupancy means the SM can't hide memory latency because there
      # aren't enough warps to switch between when one stalls.
      def occupancy
        return 0.0 if @config.max_warps == 0

        active = @warp_slots.count { |w| w.state != :completed }
        active.to_f / @config.max_warps
      end

      # --- Occupancy calculation ---

      # Calculate theoretical occupancy for a kernel launch configuration.
      #
      # This is the STATIC occupancy calculation -- how full the SM could
      # theoretically be, given the resource requirements of a kernel.
      #
      # === How Occupancy is Limited ===
      #
      # Occupancy is limited by the tightest constraint among:
      #
      # 1. Register pressure:
      #    Each warp needs registers_per_thread * 32 registers.
      #    Total warps = register_file_size / regs_per_warp.
      #
      # 2. Shared memory:
      #    Each block needs shared_mem_per_block bytes.
      #    Max blocks = shared_memory_size / shared_mem_per_block.
      #    Max warps = max_blocks * warps_per_block.
      #
      # 3. Hardware limit:
      #    The SM simply can't hold more than max_warps warps.
      #
      # @param registers_per_thread [Integer] Registers per thread.
      # @param shared_mem_per_block [Integer] Shared memory bytes per block.
      # @param threads_per_block [Integer] Threads per block.
      # @return [Float] Theoretical occupancy (0.0 to 1.0).
      def compute_occupancy(registers_per_thread:, shared_mem_per_block:, threads_per_block:)
        warp_w = @config.warp_width
        warps_per_block = (threads_per_block + warp_w - 1) / warp_w

        # Limit 1: register file
        regs_per_warp = registers_per_thread * @config.warp_width
        max_warps_by_regs = if regs_per_warp > 0
          @config.register_file_size / regs_per_warp
        else
          @config.max_warps
        end

        # Limit 2: shared memory
        max_warps_by_smem = if shared_mem_per_block > 0
          max_blocks_by_smem = @config.shared_memory_size / shared_mem_per_block
          max_blocks_by_smem * warps_per_block
        else
          @config.max_warps
        end

        # Limit 3: hardware limit
        max_warps_by_hw = @config.max_warps

        # Actual occupancy is limited by the tightest constraint
        active_warps = [max_warps_by_regs, max_warps_by_smem, max_warps_by_hw].min
        [active_warps.to_f / @config.max_warps, 1.0].min
      end

      # --- Dispatch ---

      # Dispatch a thread block to this SM.
      #
      # Decomposes the thread block into warps, allocates registers and
      # shared memory, creates WarpEngine instances, and adds warp slots
      # to the schedulers.
      #
      # @param work [WorkItem] The WorkItem to dispatch.
      # @raise [ResourceError] If not enough resources for this work item.
      def dispatch(work)
        num_warps = (work.thread_count + @config.warp_width - 1) / @config.warp_width
        regs_needed = work.registers_per_thread * @config.warp_width * num_warps
        smem_needed = work.shared_mem_bytes

        # Check resource availability
        current_active = @warp_slots.count { |w| w.state != :completed }

        if current_active + num_warps > @config.max_warps
          raise ResourceError,
            "Not enough warp slots: need #{num_warps}, " \
            "available #{@config.max_warps - current_active}"
        end

        if @registers_allocated + regs_needed > @config.register_file_size
          avail_regs = @config.register_file_size - @registers_allocated
          raise ResourceError,
            "Not enough registers: need #{regs_needed}, " \
            "available #{avail_regs}"
        end

        if @shared_memory_used + smem_needed > @config.shared_memory_size
          avail_smem = @config.shared_memory_size - @shared_memory_used
          raise ResourceError,
            "Not enough shared memory: need #{smem_needed}, " \
            "available #{avail_smem}"
        end

        # Allocate resources
        @registers_allocated += regs_needed
        @shared_memory_used += smem_needed
        @active_blocks << work.work_id

        # Create warps and distribute across schedulers
        num_warps.times do |warp_idx|
          warp_id = @next_warp_id
          @next_warp_id += 1

          # Determine thread range for this warp
          thread_start = warp_idx * @config.warp_width
          thread_end = [thread_start + @config.warp_width, work.thread_count].min
          actual_threads = thread_end - thread_start

          # Create a WarpEngine for this warp
          engine = ParallelExecutionEngine::WarpEngine.new(
            ParallelExecutionEngine::WarpConfig.new(
              warp_width: actual_threads,
              num_registers: work.registers_per_thread,
              float_format: @config.float_format,
              isa: @config.isa
            ),
            @clock
          )

          # Load program if provided
          engine.load_program(work.program) if work.program

          # Set per-thread data if provided
          actual_threads.times do |t_offset|
            global_tid = thread_start + t_offset
            if work.per_thread_data.key?(global_tid)
              work.per_thread_data[global_tid].each do |reg, val|
                engine.set_thread_register(t_offset, reg, val)
              end
            end
          end

          # Create the warp slot
          slot = WarpSlot.new(
            warp_id: warp_id,
            work_id: work.work_id,
            state: :ready,
            engine: engine,
            registers_used: work.registers_per_thread * actual_threads
          )
          @warp_slots << slot

          # Distribute to schedulers round-robin
          sched_idx = warp_idx % @config.num_schedulers
          @schedulers[sched_idx].add_warp(slot)
        end
      end

      # --- Execution ---

      # One cycle: schedulers pick warps, engines execute, stalls update.
      #
      # === Step-by-Step ===
      #
      # 1. Tick stall counters on all schedulers.
      # 2. Each scheduler picks one ready warp.
      # 3. Execute picked warps on their WarpEngines.
      # 4. Check for memory instructions -> stall the warp.
      # 5. Check for HALT -> mark warp as completed.
      # 6. Build and return a ComputeUnitTrace.
      #
      # @param clock_edge [ClockEdge] The clock edge that triggered this step.
      # @return [ComputeUnitTrace] Trace for this cycle.
      def step(clock_edge)
        @cycle += 1

        # Phase 1: Tick stall counters
        @schedulers.each(&:tick_stalls)

        # Phase 2: Each scheduler picks a warp and executes it
        engine_traces = {}
        scheduler_actions = []

        @schedulers.each do |sched|
          picked = sched.pick_warp
          if picked.nil?
            scheduler_actions << "S#{sched.scheduler_id}: no ready warp"
            next
          end

          # Mark as running
          picked.state = :running

          # Execute one cycle on the warp's engine
          trace = picked.engine.step(clock_edge)
          engine_traces[picked.warp_id] = trace

          # Record the scheduling decision
          sched.mark_issued(picked.warp_id)
          scheduler_actions << "S#{sched.scheduler_id}: issued warp #{picked.warp_id}"

          # Phase 3: Check results and update warp state
          if picked.engine.halted?
            picked.state = :completed
          elsif memory_instruction?(trace)
            picked.state = :stalled_memory
            picked.stall_counter = @config.memory_latency_cycles
          else
            picked.state = :ready
          end
        end

        # Build the trace
        active_warps = @warp_slots.count { |w| w.state != :completed }
        total_warps = @config.max_warps

        ComputeUnitTrace.new(
          cycle: @cycle,
          unit_name: name,
          architecture: architecture,
          scheduler_action: scheduler_actions.join("; "),
          active_warps: active_warps,
          total_warps: total_warps,
          engine_traces: engine_traces,
          shared_memory_used: @shared_memory_used,
          shared_memory_total: @config.shared_memory_size,
          register_file_used: @registers_allocated,
          register_file_total: @config.register_file_size,
          occupancy: total_warps > 0 ? active_warps.to_f / total_warps : 0.0
        )
      end

      # Run until all work completes or max_cycles.
      #
      # Creates clock edges internally to drive execution.
      #
      # @param max_cycles [Integer] Safety limit to prevent infinite loops.
      # @return [Array<ComputeUnitTrace>] One trace per cycle.
      def run(max_cycles: 100_000)
        traces = []
        (1..max_cycles).each do |cycle_num|
          edge = Clock::ClockEdge.new(
            cycle: cycle_num, value: 1,
            "rising?": true, "falling?": false
          )
          trace = step(edge)
          traces << trace
          break if idle?
        end
        traces
      end

      # Reset all state: engines, schedulers, shared memory.
      def reset
        @schedulers.each(&:reset)
        @warp_slots.clear
        @shared_memory.reset
        @shared_memory_used = 0
        @registers_allocated = 0
        @active_blocks.clear
        @next_warp_id = 0
        @cycle = 0
      end

      def to_s
        active = @warp_slots.count { |w| w.state != :completed }
        "StreamingMultiprocessor(warps=#{active}/#{@config.max_warps}, " \
          "occupancy=#{"%.1f%%" % (occupancy * 100)}, " \
          "policy=#{@config.scheduling_policy})"
      end

      def inspect
        to_s
      end

      private

      # Check if the executed instruction was a memory operation.
      # Memory operations (LOAD/STORE) stall the warp for
      # memory_latency_cycles to simulate global memory latency.
      def memory_instruction?(trace)
        desc = trace.description.upcase
        desc.include?("LOAD") || desc.include?("STORE")
      end
    end
  end
end

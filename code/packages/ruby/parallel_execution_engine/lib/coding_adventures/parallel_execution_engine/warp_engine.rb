# frozen_string_literal: true

# ---------------------------------------------------------------------------
# WarpEngine -- SIMT parallel execution (NVIDIA CUDA / ARM Mali style).
# ---------------------------------------------------------------------------
#
# === What is SIMT? ===
#
# SIMT stands for "Single Instruction, Multiple Threads." NVIDIA invented this
# term to describe how their GPU cores work. It's a hybrid between two older
# concepts:
#
#     SISD (one instruction, one datum):
#         Like a single CPU core. Our gpu-core package at Layer 9.
#
#     SIMD (one instruction, multiple data):
#         Like AMD wavefronts. One instruction operates on a wide vector.
#         There are no "threads" -- just lanes in a vector ALU.
#
#     SIMT (one instruction, multiple threads):
#         Like NVIDIA warps. Multiple threads, each with its own registers
#         and (logically) its own program counter. They USUALLY execute
#         the same instruction, but CAN diverge.
#
# The key difference between SIMD and SIMT:
#
#     SIMD: "I have one wide ALU that processes 32 numbers at once."
#     SIMT: "I have 32 tiny threads that happen to execute in lockstep."
#
# This distinction matters when threads need to take different paths (branches).
# In SIMD, you just mask off lanes. In SIMT, the hardware manages a divergence
# stack to serialize the paths and then reconverge.
#
# === How a Warp Works ===
#
# A warp is a group of threads (32 for NVIDIA, 16 for ARM Mali) that the
# hardware schedules together. On each clock cycle:
#
#     1. The warp scheduler picks one instruction (at the warp's PC).
#     2. That instruction is issued to ALL active threads simultaneously.
#     3. Each thread executes the instruction on its OWN registers.
#     4. If the instruction is a branch, threads may diverge.
#
#     +-----------------------------------------------------+
#     |  Warp (32 threads)                                   |
#     |                                                      |
#     |  Active Mask: [T,T,T,T,T,T,T,T,...,T,T,T,T]         |
#     |  PC: 0x004                                           |
#     |                                                      |
#     |  +------+ +------+ +------+       +------+           |
#     |  | T0   | | T1   | | T2   |  ...  | T31  |           |
#     |  |R0=1.0| |R0=2.0| |R0=3.0|       |R0=32.|           |
#     |  |R1=0.5| |R1=0.5| |R1=0.5|       |R1=0.5|           |
#     |  +------+ +------+ +------+       +------+           |
#     |                                                      |
#     |  Instruction: FMUL R2, R0, R1                        |
#     |  Result: T0.R2=0.5, T1.R2=1.0, T2.R2=1.5, ...       |
#     +-----------------------------------------------------+
#
# === Divergence: The Price of Flexibility ===
#
# When threads in a warp encounter a branch and disagree on which way to go,
# the warp "diverges." The hardware serializes the paths:
#
#     Step 1: Evaluate the branch condition for ALL threads.
#     Step 2: Threads that go "true" -> execute first (others masked off).
#     Step 3: Push (reconvergence_pc, other_mask) onto the divergence stack.
#     Step 4: When "true" path finishes, pop the stack.
#     Step 5: Execute the "false" path (first group masked off).
#     Step 6: At the reconvergence point, all threads are active again.
#
#     Example with 4 threads:
#
#     if (thread_id < 2):    Mask: [T,T,F,F]  <- threads 0,1 take true path
#         path_A()           Only threads 0,1 execute
#     else:                  Mask: [F,F,T,T]  <- threads 2,3 take false path
#         path_B()           Only threads 2,3 execute
#     // reconverge          Mask: [T,T,T,T]  <- all 4 threads active again

module CodingAdventures
  module ParallelExecutionEngine
    # -----------------------------------------------------------------------
    # WarpConfig -- configuration for a SIMT warp engine
    # -----------------------------------------------------------------------
    #
    # Real-world reference values:
    #
    #     Vendor      | Warp Width | Registers | Memory     | Max Divergence
    #     ------------+------------+-----------+------------+---------------
    #     NVIDIA      | 32         | 255       | 512 KB     | 32+ levels
    #     ARM Mali    | 16         | 64        | varies     | 16+ levels
    #     Our default | 32         | 32        | 1024 B     | 32 levels
    WarpConfig = Data.define(
      :warp_width,
      :num_registers,
      :memory_per_thread,
      :float_format,
      :max_divergence_depth,
      :isa,
      :independent_thread_scheduling
    ) do
      def initialize(
        warp_width: 32,
        num_registers: 32,
        memory_per_thread: 1024,
        float_format: FpArithmetic::FP32,
        max_divergence_depth: 32,
        isa: nil,
        independent_thread_scheduling: false
      )
        super(
          warp_width: warp_width,
          num_registers: num_registers,
          memory_per_thread: memory_per_thread,
          float_format: float_format,
          max_divergence_depth: max_divergence_depth,
          isa: isa || GpuCore::GenericISA.new,
          independent_thread_scheduling: independent_thread_scheduling
        )
      end
    end

    # -----------------------------------------------------------------------
    # ThreadContext -- per-thread execution context in a SIMT warp
    # -----------------------------------------------------------------------
    #
    # Each thread in the warp has:
    # - thread_id: its position in the warp (0 to warp_width-1)
    # - core: a full GPUCore instance with its own registers and memory
    # - active: whether this thread is currently executing (false = masked off)
    # - pc: per-thread program counter (used in independent scheduling mode)
    #
    # In NVIDIA hardware, each CUDA thread has 255 registers. In our simulator,
    # each thread gets a full GPUCore instance, which is heavier but lets us
    # reuse all the existing instruction execution infrastructure.
    class ThreadContext
      attr_reader :thread_id, :core
      attr_accessor :active, :pc

      def initialize(thread_id:, core:, active: true, pc: 0)
        @thread_id = thread_id
        @core = core
        @active = active
        @pc = pc
      end
    end

    # -----------------------------------------------------------------------
    # DivergenceStackEntry -- one entry on the divergence stack
    # -----------------------------------------------------------------------
    #
    # When threads diverge at a branch, we push an entry recording:
    # - reconvergence_pc: where threads should rejoin
    # - saved_mask: which threads took the OTHER path (will run later)
    #
    # This is the pre-Volta divergence handling mechanism.
    DivergenceStackEntry = Data.define(:reconvergence_pc, :saved_mask)

    # -----------------------------------------------------------------------
    # WarpEngine -- the SIMT parallel execution engine
    # -----------------------------------------------------------------------
    #
    # Manages N threads executing in lockstep with hardware divergence support.
    # Each thread is backed by a real GPUCore instance from the gpu-core package.
    #
    # === Usage Pattern ===
    #
    #     1. Create engine with config and clock
    #     2. Load program (same program goes to all threads)
    #     3. Set per-thread register values (give each thread different data)
    #     4. Step or run (engine issues instructions to all active threads)
    #     5. Read results from per-thread registers
    #
    # Example:
    #     clock = CodingAdventures::Clock::ClockGenerator.new
    #     engine = WarpEngine.new(WarpConfig.new(warp_width: 4), clock)
    #     engine.load_program([GpuCore.limm(0, 2.0), GpuCore.fmul(2, 0, 1), GpuCore.halt])
    #     traces = engine.run
    #     engine.threads[0].core.registers.read_float(2)
    class WarpEngine
      attr_reader :threads, :config

      def initialize(config, clock)
        @config = config
        @clock = clock
        @cycle = 0
        @program = []

        # Create one GPUCore per thread. Each thread is an independent
        # processing element with its own registers and local memory.
        @threads = Array.new(config.warp_width) do |i|
          ThreadContext.new(
            thread_id: i,
            core: GpuCore::GPUCore.new(
              isa: config.isa,
              fmt: config.float_format,
              num_registers: config.num_registers,
              memory_size: config.memory_per_thread
            )
          )
        end

        # The divergence stack for pre-Volta branch handling.
        @divergence_stack = []
        @all_halted = false
      end

      # --- Properties (duck type interface) ---

      def name
        "WarpEngine"
      end

      def width
        @config.warp_width
      end

      def execution_model
        :simt
      end

      def active_mask
        @threads.map(&:active)
      end

      def halted?
        @all_halted
      end

      # --- Program loading ---

      # Load the same program into all threads.
      #
      # In real NVIDIA hardware, all threads in a warp share the same
      # instruction memory. We simulate this by loading the same program
      # into each thread's GPUCore.
      def load_program(program)
        @program = program.dup
        @threads.each do |thread|
          thread.core.load_program(@program)
          thread.active = true
          thread.pc = 0
        end
        @all_halted = false
        @cycle = 0
        @divergence_stack.clear
      end

      # --- Per-thread register setup ---

      # Set a register value for a specific thread.
      #
      # This is how you give each thread different data to work on.
      # In a real GPU kernel, each thread would compute its global index
      # and use it to load different data from memory. In our simulator,
      # we pre-load the data into registers.
      #
      # @param thread_id [Integer] Which thread (0 to warp_width - 1).
      # @param reg [Integer] Which register (0 to num_registers - 1).
      # @param value [Float] The float value to write.
      # @raise [IndexError] If thread_id is out of range.
      def set_thread_register(thread_id, reg, value)
        if thread_id < 0 || thread_id >= @config.warp_width
          raise IndexError,
            "Thread ID #{thread_id} out of range [0, #{@config.warp_width})"
        end
        @threads[thread_id].core.registers.write_float(reg, value)
      end

      # --- Execution ---

      # Execute one cycle: issue one instruction to all active threads.
      #
      # On each clock edge:
      # 1. Find the instruction at the current warp PC.
      # 2. Issue it to all active (non-masked) threads.
      # 3. Detect divergence on branch instructions.
      # 4. Handle reconvergence when appropriate.
      # 5. Build and return an EngineTrace.
      #
      # @param clock_edge [ClockEdge] The clock edge that triggered this step.
      # @return [EngineTrace] Describing what all threads did this cycle.
      def step(clock_edge)
        @cycle += 1

        # If all halted, produce a no-op trace
        return make_halted_trace if @all_halted

        # Check for reconvergence
        check_reconvergence

        # Find active, non-halted threads
        active_threads = @threads.select { |t| t.active && !t.core.halted? }

        if active_threads.empty?
          if @divergence_stack.any?
            return pop_divergence_and_trace
          end
          @all_halted = true
          return make_halted_trace
        end

        # Save pre-step mask for divergence tracking
        mask_before = @threads.map(&:active)

        # Execute the instruction on all active, non-halted threads
        unit_traces = {}
        branch_taken_threads = []
        branch_not_taken_threads = []

        @threads.each do |thread|
          if thread.active && !thread.core.halted?
            begin
              trace = thread.core.step
              unit_traces[thread.thread_id] = trace.description

              # Detect branch divergence
              if trace.next_pc != trace.pc + 1 && !trace.halted
                branch_taken_threads << thread.thread_id
              elsif !trace.halted
                branch_not_taken_threads << thread.thread_id
              end

              unit_traces[thread.thread_id] = "HALTED" if trace.halted
            rescue RuntimeError
              thread.active = false
              unit_traces[thread.thread_id] = "(error -- deactivated)"
            end
          elsif thread.core.halted?
            unit_traces[thread.thread_id] = "(halted)"
          else
            unit_traces[thread.thread_id] = "(masked off)"
          end
        end

        # Handle divergence
        divergence_info = nil
        if branch_taken_threads.any? && branch_not_taken_threads.any?
          divergence_info = handle_divergence(
            branch_taken_threads,
            branch_not_taken_threads,
            mask_before
          )
        end

        # Check if all threads are now halted
        @all_halted = true if @threads.all? { |t| t.core.halted? }

        # Build the trace
        current_mask = @threads.map { |t| t.active && !t.core.halted? }
        active_count = current_mask.count(true)
        total = @config.warp_width

        # Get a description from the first active instruction
        skip_descriptions = ["(masked off)", "(halted)", "(error -- deactivated)"]
        first_active = @threads.each_with_object(nil) do |t, _|
          desc = unit_traces[t.thread_id]
          break desc if desc && !skip_descriptions.include?(desc)
        end || "no active threads"

        EngineTrace.new(
          cycle: @cycle,
          engine_name: name,
          execution_model: execution_model,
          description: "#{first_active} -- #{active_count}/#{total} threads active",
          unit_traces: unit_traces,
          active_mask: current_mask,
          active_count: active_count,
          total_count: total,
          utilization: total > 0 ? active_count.to_f / total : 0.0,
          divergence_info: divergence_info
        )
      end

      # Run until all threads halt or max_cycles reached.
      #
      # @param max_cycles [Integer] Safety limit to prevent infinite loops.
      # @return [Array<EngineTrace>] One trace per cycle.
      # @raise [RuntimeError] If max_cycles exceeded.
      def run(max_cycles: 10_000)
        traces = []
        (1..max_cycles).each do |cycle_num|
          edge = Clock::ClockEdge.new(
            cycle: cycle_num,
            value: 1,
            "rising?": true,
            "falling?": false
          )
          trace = step(edge)
          traces << trace
          break if @all_halted
        end

        if !@all_halted && traces.length >= max_cycles
          raise RuntimeError, "WarpEngine: max_cycles (#{max_cycles}) reached"
        end

        traces
      end

      # Reset the engine to its initial state.
      def reset
        @threads.each do |thread|
          thread.core.reset
          thread.active = true
          thread.pc = 0
          thread.core.load_program(@program) if @program.any?
        end
        @divergence_stack.clear
        @all_halted = false
        @cycle = 0
      end

      def to_s
        active = @threads.count(&:active)
        halted_count = @threads.count { |t| t.core.halted? }
        "WarpEngine(width=#{@config.warp_width}, " \
          "active=#{active}, halted_threads=#{halted_count}, " \
          "divergence_depth=#{@divergence_stack.length})"
      end

      def inspect
        to_s
      end

      private

      # --- Divergence handling ---

      # Handle a divergent branch by pushing onto the divergence stack.
      def handle_divergence(taken_threads, not_taken_threads, mask_before)
        # The reconvergence PC is the maximum PC among all active threads
        # after the branch.
        all_pcs = (taken_threads + not_taken_threads).map { |tid| @threads[tid].core.pc }
        reconvergence_pc = all_pcs.max

        # Build the saved mask: threads that took the "not taken" path
        saved_mask = Array.new(@config.warp_width, false)
        not_taken_threads.each do |tid|
          saved_mask[tid] = true
          @threads[tid].active = false
        end

        # Push onto the divergence stack
        if @divergence_stack.length < @config.max_divergence_depth
          @divergence_stack.push(
            DivergenceStackEntry.new(
              reconvergence_pc: reconvergence_pc,
              saved_mask: saved_mask
            )
          )
        end

        mask_after = @threads.map(&:active)

        DivergenceInfo.new(
          active_mask_before: mask_before,
          active_mask_after: mask_after,
          reconvergence_pc: reconvergence_pc,
          divergence_depth: @divergence_stack.length
        )
      end

      # Check if active threads have reached a reconvergence point.
      def check_reconvergence
        return if @divergence_stack.empty?

        entry = @divergence_stack.last
        active_threads = @threads.select { |t| t.active && !t.core.halted? }
        return if active_threads.empty?

        all_at_reconvergence = active_threads.all? do |t|
          t.core.pc >= entry.reconvergence_pc
        end

        if all_at_reconvergence
          @divergence_stack.pop
          entry.saved_mask.each_with_index do |should_activate, tid|
            if should_activate && !@threads[tid].core.halted?
              @threads[tid].active = true
            end
          end
        end
      end

      # Pop the divergence stack when all active threads are done.
      def pop_divergence_and_trace
        entry = @divergence_stack.pop

        entry.saved_mask.each_with_index do |should_activate, tid|
          if should_activate && !@threads[tid].core.halted?
            @threads[tid].active = true
          end
        end

        current_mask = @threads.map { |t| t.active && !t.core.halted? }
        active_count = current_mask.count(true)

        EngineTrace.new(
          cycle: @cycle,
          engine_name: name,
          execution_model: execution_model,
          description: "Divergence stack pop -- reactivated #{active_count} threads",
          unit_traces: @threads.to_h { |t|
            [t.thread_id, entry.saved_mask[t.thread_id] ? "reactivated" : "(waiting)"]
          },
          active_mask: current_mask,
          active_count: active_count,
          total_count: @config.warp_width,
          utilization: @config.warp_width > 0 ? active_count.to_f / @config.warp_width : 0.0
        )
      end

      # Produce a trace for when all threads are halted.
      def make_halted_trace
        EngineTrace.new(
          cycle: @cycle,
          engine_name: name,
          execution_model: execution_model,
          description: "All threads halted",
          unit_traces: @threads.to_h { |t| [t.thread_id, "(halted)"] },
          active_mask: Array.new(@config.warp_width, false),
          active_count: 0,
          total_count: @config.warp_width,
          utilization: 0.0
        )
      end
    end
  end
end

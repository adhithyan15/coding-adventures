# frozen_string_literal: true

# ---------------------------------------------------------------------------
# AMDComputeUnit -- AMD Compute Unit (GCN/RDNA) simulator.
# ---------------------------------------------------------------------------
#
# === How AMD CUs Differ from NVIDIA SMs ===
#
# While NVIDIA and AMD GPUs look similar from the outside, their internal
# organization is quite different:
#
#     NVIDIA SM:                          AMD CU (GCN):
#     ---------                           --------
#     4 warp schedulers                   4 SIMD units (16-wide each)
#     Each issues 1 warp (32 threads)     Each runs 1 wavefront (64 lanes)
#     Total: 128 threads/cycle            Total: 64 lanes x 4 = 256 lanes/cycle
#
#     Register file: unified              Register file: per-SIMD VGPR
#     Shared memory: explicit             LDS: explicit (similar to shared mem)
#     Warp scheduling: hardware           Wavefront scheduling: hardware
#     Scalar unit: per-thread             Scalar unit: SHARED by wavefront
#
# === The Scalar Unit -- AMD's Key Innovation ===
#
# The scalar unit executes operations that are the SAME across all lanes:
# - Address computation (base_addr + offset)
# - Loop counters (i++)
# - Branch conditions (if i < N)
# - Constants (pi, epsilon, etc.)
#
# Instead of doing this 64 times (once per lane), AMD does it ONCE in the
# scalar unit and broadcasts the result. This saves power and register space.
#
# === Architecture Diagram ===
#
#     AMDComputeUnit (GCN-style)
#     +---------------------------------------------------------------+
#     |                                                               |
#     |  Wavefront Scheduler                                          |
#     |  +----------------------------------------------------------+ |
#     |  | wf0: READY  wf1: STALLED  wf2: READY  wf3: READY ...    | |
#     |  +----------------------------------------------------------+ |
#     |                                                               |
#     |  +------------------+ +------------------+                    |
#     |  | SIMD Unit 0      | | SIMD Unit 1      |                    |
#     |  | 16-wide ALU      | | 16-wide ALU      |                    |
#     |  | VGPR: 256        | | VGPR: 256        |                    |
#     |  +------------------+ +------------------+                    |
#     |  +------------------+ +------------------+                    |
#     |  | SIMD Unit 2      | | SIMD Unit 3      |                    |
#     |  | 16-wide ALU      | | 16-wide ALU      |                    |
#     |  +------------------+ +------------------+                    |
#     |                                                               |
#     |  +------------------+                                         |
#     |  | Scalar Unit      |  <- executes once for all lanes         |
#     |  | SGPR: 104        |  (address computation, flow control)    |
#     |  +------------------+                                         |
#     |                                                               |
#     |  Shared Resources:                                            |
#     |  +-----------------------------------------------------------+|
#     |  | LDS (Local Data Share): 64 KB                              ||
#     |  | L1 Vector Cache: 16 KB                                     ||
#     |  | L1 Scalar Cache: 16 KB                                     ||
#     |  +-----------------------------------------------------------+|
#     +---------------------------------------------------------------+

module CodingAdventures
  module ComputeUnit
    # -----------------------------------------------------------------------
    # AMDCUConfig -- configuration for an AMD-style Compute Unit
    # -----------------------------------------------------------------------
    #
    # Real-world CU configurations:
    #
    #     Parameter            | GCN (Vega)   | RDNA2 (RX 6000) | RDNA3
    #     ---------------------+--------------+------------------+------
    #     SIMD units           | 4            | 2 (per CU)       | 2
    #     Wave width           | 64           | 32 (native)      | 32
    #     Max wavefronts       | 40           | 32               | 32
    #     VGPRs per SIMD       | 256          | 256              | 256
    #     SGPRs                | 104          | 104              | 104
    #     LDS size             | 64 KB        | 128 KB           | 128 KB
    #     L1 vector cache      | 16 KB        | 128 KB           | 128 KB
    AMDCUConfig = Data.define(
      :num_simd_units,
      :wave_width,
      :max_wavefronts,
      :max_work_groups,
      :scheduling_policy,
      :vgpr_per_simd,
      :sgpr_count,
      :lds_size,
      :l1_vector_cache,
      :l1_scalar_cache,
      :l1_instruction_cache,
      :float_format,
      :isa,
      :memory_latency_cycles
    ) do
      def initialize(
        num_simd_units: 4,
        wave_width: 64,
        max_wavefronts: 40,
        max_work_groups: 16,
        scheduling_policy: :lrr,
        vgpr_per_simd: 256,
        sgpr_count: 104,
        lds_size: 65_536,
        l1_vector_cache: 16_384,
        l1_scalar_cache: 16_384,
        l1_instruction_cache: 32_768,
        float_format: FpArithmetic::FP32,
        isa: nil,
        memory_latency_cycles: 200
      )
        super(
          num_simd_units: num_simd_units,
          wave_width: wave_width,
          max_wavefronts: max_wavefronts,
          max_work_groups: max_work_groups,
          scheduling_policy: scheduling_policy,
          vgpr_per_simd: vgpr_per_simd,
          sgpr_count: sgpr_count,
          lds_size: lds_size,
          l1_vector_cache: l1_vector_cache,
          l1_scalar_cache: l1_scalar_cache,
          l1_instruction_cache: l1_instruction_cache,
          float_format: float_format,
          isa: isa || GpuCore::GenericISA.new,
          memory_latency_cycles: memory_latency_cycles
        )
      end
    end

    # -----------------------------------------------------------------------
    # WavefrontSlot -- tracks one wavefront's state
    # -----------------------------------------------------------------------
    #
    # Similar to WarpSlot in the NVIDIA SM, but for AMD wavefronts.
    # Each slot tracks the wavefront's state and which SIMD unit
    # it's assigned to.
    class WavefrontSlot
      attr_accessor :state, :stall_counter, :age
      attr_reader :wave_id, :work_id, :simd_unit, :engine, :vgprs_used

      def initialize(wave_id:, work_id:, state:, simd_unit:, engine:,
        stall_counter: 0, age: 0, vgprs_used: 0)
        @wave_id = wave_id
        @work_id = work_id
        @state = state
        @simd_unit = simd_unit
        @engine = engine
        @stall_counter = stall_counter
        @age = age
        @vgprs_used = vgprs_used
      end
    end

    # -----------------------------------------------------------------------
    # AMDComputeUnit -- the main CU simulator
    # -----------------------------------------------------------------------
    #
    # Manages wavefronts across SIMD units, with scalar unit support,
    # LDS (Local Data Share), and wavefront scheduling.
    #
    # === Key Differences from StreamingMultiprocessor ===
    #
    # 1. **SIMD units instead of warp schedulers**: Each SIMD unit is a
    #    16-wide vector ALU. A 64-wide wavefront takes 4 cycles to execute.
    #
    # 2. **Scalar unit**: Operations common to all lanes execute once on
    #    the scalar unit instead of per-lane.
    #
    # 3. **LDS instead of shared memory**: Functionally similar but with
    #    different banking.
    #
    # 4. **LRR scheduling**: AMD typically uses Loose Round Robin instead
    #    of NVIDIA's GTO.
    class AMDComputeUnit
      attr_reader :config, :lds, :wavefront_slots

      def initialize(config, clock)
        @config = config
        @clock = clock
        @cycle = 0

        # LDS (Local Data Share) -- AMD's shared memory
        @lds = SharedMemory.new(size: config.lds_size)
        @lds_used = 0

        # Wavefront tracking
        @wavefront_slots = []
        @next_wave_id = 0

        # VGPR tracking per SIMD unit
        @vgpr_allocated = Array.new(config.num_simd_units, 0)

        # Simple round-robin index for scheduling
        @rr_index = 0
      end

      # --- Properties ---

      def name
        "CU"
      end

      def architecture
        :amd_cu
      end

      # True if no active wavefronts remain.
      def idle?
        @wavefront_slots.empty? ||
          @wavefront_slots.all? { |w| w.state == :completed }
      end

      # Current occupancy: active wavefronts / max wavefronts.
      def occupancy
        return 0.0 if @config.max_wavefronts == 0

        active = @wavefront_slots.count { |w| w.state != :completed }
        active.to_f / @config.max_wavefronts
      end

      # --- Dispatch ---

      # Dispatch a work group to this CU.
      #
      # Decomposes the work group into wavefronts and assigns them to
      # SIMD units round-robin.
      #
      # @param work [WorkItem] The WorkItem to dispatch.
      # @raise [ResourceError] If not enough resources.
      def dispatch(work)
        num_waves = (work.thread_count + @config.wave_width - 1) / @config.wave_width

        current_active = @wavefront_slots.count { |w| w.state != :completed }

        if current_active + num_waves > @config.max_wavefronts
          raise ResourceError,
            "Not enough wavefront slots: need #{num_waves}, " \
            "available #{@config.max_wavefronts - current_active}"
        end

        smem_needed = work.shared_mem_bytes
        if @lds_used + smem_needed > @config.lds_size
          raise ResourceError,
            "Not enough LDS: need #{smem_needed}, " \
            "available #{@config.lds_size - @lds_used}"
        end

        @lds_used += smem_needed

        num_waves.times do |wave_idx|
          wave_id = @next_wave_id
          @next_wave_id += 1

          thread_start = wave_idx * @config.wave_width
          thread_end = [thread_start + @config.wave_width, work.thread_count].min
          actual_lanes = thread_end - thread_start

          # Assign to a SIMD unit round-robin
          simd_unit = wave_idx % @config.num_simd_units

          # Create WavefrontEngine
          engine = ParallelExecutionEngine::WavefrontEngine.new(
            ParallelExecutionEngine::WavefrontConfig.new(
              wave_width: actual_lanes,
              num_vgprs: [@config.vgpr_per_simd, 256].min,
              num_sgprs: @config.sgpr_count,
              float_format: @config.float_format,
              isa: @config.isa
            ),
            @clock
          )

          engine.load_program(work.program) if work.program

          # Set per-lane data
          actual_lanes.times do |lane_offset|
            global_tid = thread_start + lane_offset
            if work.per_thread_data.key?(global_tid)
              work.per_thread_data[global_tid].each do |reg, val|
                engine.set_lane_register(lane_offset, reg, val)
              end
            end
          end

          slot = WavefrontSlot.new(
            wave_id: wave_id,
            work_id: work.work_id,
            state: :ready,
            simd_unit: simd_unit,
            engine: engine,
            vgprs_used: [@config.vgpr_per_simd, 256].min
          )
          @wavefront_slots << slot
        end
      end

      # --- Execution ---

      # One cycle: schedule wavefronts, execute on SIMD units.
      #
      # @param clock_edge [ClockEdge] The clock edge that triggered this step.
      # @return [ComputeUnitTrace] A trace for this cycle.
      def step(clock_edge)
        @cycle += 1

        # Tick stall counters
        @wavefront_slots.each do |slot|
          if slot.stall_counter > 0
            slot.stall_counter -= 1
            if slot.stall_counter == 0 && slot.state == :stalled_memory
              slot.state = :ready
            end
          end
          if slot.state != :completed && slot.state != :running
            slot.age += 1
          end
        end

        # Schedule: pick up to num_simd_units wavefronts (one per SIMD unit)
        engine_traces = {}
        scheduler_actions = []

        @config.num_simd_units.times do |simd_id|
          ready = @wavefront_slots.select do |w|
            w.state == :ready && w.simd_unit == simd_id
          end
          next if ready.empty?

          # LRR: pick oldest ready wavefront (approximation of LRR)
          picked = ready.max_by(&:age)
          picked.state = :running

          trace = picked.engine.step(clock_edge)
          engine_traces[picked.wave_id] = trace

          scheduler_actions << "SIMD#{simd_id}: issued wave #{picked.wave_id}"
          picked.age = 0

          # Update state after execution
          if picked.engine.halted?
            picked.state = :completed
          elsif memory_instruction?(trace)
            picked.state = :stalled_memory
            picked.stall_counter = @config.memory_latency_cycles
          else
            picked.state = :ready
          end
        end

        if scheduler_actions.empty?
          scheduler_actions << "all wavefronts stalled or completed"
        end

        active_waves = @wavefront_slots.count { |w| w.state != :completed }
        total_vgprs = @config.vgpr_per_simd * @config.num_simd_units

        ComputeUnitTrace.new(
          cycle: @cycle,
          unit_name: name,
          architecture: architecture,
          scheduler_action: scheduler_actions.join("; "),
          active_warps: active_waves,
          total_warps: @config.max_wavefronts,
          engine_traces: engine_traces,
          shared_memory_used: @lds_used,
          shared_memory_total: @config.lds_size,
          register_file_used: @vgpr_allocated.sum,
          register_file_total: total_vgprs,
          occupancy: @config.max_wavefronts > 0 ? active_waves.to_f / @config.max_wavefronts : 0.0
        )
      end

      # Run until all work completes or max_cycles.
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

      # Reset all state.
      def reset
        @wavefront_slots.clear
        @lds.reset
        @lds_used = 0
        @vgpr_allocated = Array.new(@config.num_simd_units, 0)
        @next_wave_id = 0
        @rr_index = 0
        @cycle = 0
      end

      def to_s
        active = @wavefront_slots.count { |w| w.state != :completed }
        "AMDComputeUnit(waves=#{active}/#{@config.max_wavefronts}, " \
          "occupancy=#{"%.1f%%" % (occupancy * 100)})"
      end

      def inspect
        to_s
      end

      private

      # Check if the executed instruction was a memory operation.
      def memory_instruction?(trace)
        desc = trace.description.upcase
        desc.include?("LOAD") || desc.include?("STORE")
      end
    end
  end
end

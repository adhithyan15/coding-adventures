# frozen_string_literal: true

# ---------------------------------------------------------------------------
# WavefrontEngine -- SIMD parallel execution (AMD GCN/RDNA style).
# ---------------------------------------------------------------------------
#
# === What is a Wavefront? ===
#
# AMD calls their parallel execution unit a "wavefront." It's 64 lanes on GCN
# (Graphics Core Next) or 32 lanes on RDNA (Radeon DNA). A wavefront is
# fundamentally different from an NVIDIA warp:
#
#     NVIDIA Warp (SIMT):                AMD Wavefront (SIMD):
#     +--------------------------+       +--------------------------+
#     | 32 threads               |       | 32 lanes                 |
#     | Each has its own regs    |       | ONE vector register file  |
#     | Logically own PC         |       | ONE program counter       |
#     | HW manages divergence    |       | Explicit EXEC mask        |
#     +--------------------------+       +--------------------------+
#
# The critical architectural difference:
#
#     SIMT (NVIDIA): "32 independent threads that HAPPEN to run together"
#     SIMD (AMD):    "1 instruction that operates on a 32-wide vector"
#
# === AMD's Two Register Files ===
#
# AMD wavefronts have TWO types of registers, which is architecturally unique:
#
#     Vector GPRs (VGPRs):              Scalar GPRs (SGPRs):
#     +------------------------+        +------------------------+
#     | v0: [l0][l1]...[l31]  |        | s0:  42.0              |
#     | v1: [l0][l1]...[l31]  |        | s1:  3.14              |
#     | ...                    |        | ...                    |
#     | v255:[l0][l1]...[l31]  |        | s103: 0.0              |
#     +------------------------+        +------------------------+
#     One value PER LANE                One value for ALL LANES
#
# SGPRs are used for values that are the SAME across all lanes: constants,
# loop counters, memory base addresses.
#
# === The EXEC Mask ===
#
# AMD uses a register called EXEC to control which lanes execute each
# instruction. Unlike NVIDIA's hardware-managed divergence, the EXEC mask
# is explicitly set by instructions.

module CodingAdventures
  module ParallelExecutionEngine
    # -----------------------------------------------------------------------
    # WavefrontConfig -- configuration for an AMD-style SIMD wavefront
    # -----------------------------------------------------------------------
    #
    # Real-world reference values:
    #
    #     Architecture | Wave Width | VGPRs | SGPRs | LDS
    #     -------------+------------+-------+-------+---------
    #     AMD GCN      | 64         | 256   | 104   | 64 KB
    #     AMD RDNA     | 32         | 256   | 104   | 64 KB
    #     Our default  | 32         | 256   | 104   | 64 KB
    WavefrontConfig = Data.define(
      :wave_width,
      :num_vgprs,
      :num_sgprs,
      :lds_size,
      :float_format,
      :isa
    ) do
      def initialize(
        wave_width: 32,
        num_vgprs: 256,
        num_sgprs: 104,
        lds_size: 65_536,
        float_format: FpArithmetic::FP32,
        isa: nil
      )
        super(
          wave_width: wave_width,
          num_vgprs: num_vgprs,
          num_sgprs: num_sgprs,
          lds_size: lds_size,
          float_format: float_format,
          isa: isa || GpuCore::GenericISA.new
        )
      end
    end

    # -----------------------------------------------------------------------
    # VectorRegisterFile -- one value per lane per register
    # -----------------------------------------------------------------------
    #
    # AMD-style vector register file: num_vgprs registers x wave_width lanes.
    # Each "register" is actually a vector of wave_width values:
    #
    #     +--------------------------------------------+
    #     |         Lane 0   Lane 1   Lane 2  ...      |
    #     | v0:    [ 1.0  ] [ 2.0  ] [ 3.0  ]  ...    |
    #     | v1:    [ 0.5  ] [ 0.5  ] [ 0.5  ]  ...    |
    #     | v2:    [ 0.0  ] [ 0.0  ] [ 0.0  ]  ...    |
    #     | ...                                        |
    #     +--------------------------------------------+
    class VectorRegisterFile
      attr_reader :num_vgprs, :wave_width, :fmt

      def initialize(num_vgprs:, wave_width:, fmt: FpArithmetic::FP32)
        @num_vgprs = num_vgprs
        @wave_width = wave_width
        @fmt = fmt
        @zero = FpArithmetic.float_to_bits(0.0, fmt)
        # 2D storage: _data[reg_index][lane_index] = FloatBits
        @data = Array.new(num_vgprs) do
          Array.new(wave_width) { FpArithmetic.float_to_bits(0.0, fmt) }
        end
      end

      # Read one lane of a vector register as a Ruby float.
      def read(vreg, lane)
        FpArithmetic.bits_to_float(@data[vreg][lane])
      end

      # Write a Ruby float to one lane of a vector register.
      def write(vreg, lane, value)
        @data[vreg][lane] = FpArithmetic.float_to_bits(value, @fmt)
      end

      # Read all lanes of a vector register.
      def read_all_lanes(vreg)
        (0...@wave_width).map { |lane| FpArithmetic.bits_to_float(@data[vreg][lane]) }
      end
    end

    # -----------------------------------------------------------------------
    # ScalarRegisterFile -- one value shared across all lanes
    # -----------------------------------------------------------------------
    #
    # AMD-style scalar register file: num_sgprs single-value registers.
    # Scalar registers hold values that are the SAME for all lanes:
    # constants, loop counters, memory base addresses.
    class ScalarRegisterFile
      attr_reader :num_sgprs, :fmt

      def initialize(num_sgprs:, fmt: FpArithmetic::FP32)
        @num_sgprs = num_sgprs
        @fmt = fmt
        @data = Array.new(num_sgprs) { FpArithmetic.float_to_bits(0.0, fmt) }
      end

      # Read a scalar register as a Ruby float.
      def read(sreg)
        FpArithmetic.bits_to_float(@data[sreg])
      end

      # Write a Ruby float to a scalar register.
      def write(sreg, value)
        @data[sreg] = FpArithmetic.float_to_bits(value, @fmt)
      end
    end

    # -----------------------------------------------------------------------
    # WavefrontEngine -- the SIMD parallel execution engine
    # -----------------------------------------------------------------------
    #
    # One instruction stream, one wide vector ALU, explicit EXEC mask.
    # Internally uses GPUCore per lane for instruction execution, but
    # exposes the AMD-style vector/scalar register interface.
    #
    # === Key Differences from WarpEngine ===
    #
    # 1. ONE program counter (not per-thread PCs).
    # 2. Vector registers are a 2D array (vreg x lane), not per-thread.
    # 3. Scalar registers are shared across all lanes.
    # 4. EXEC mask is explicitly controlled, not hardware-managed.
    # 5. No divergence stack -- mask management is programmer/compiler's job.
    class WavefrontEngine
      attr_reader :config, :vrf, :srf

      def initialize(config, clock)
        @config = config
        @clock = clock
        @cycle = 0
        @program = []

        # The EXEC mask: true = lane is active, false = lane is masked off.
        @exec_mask = Array.new(config.wave_width, true)

        # Vector and scalar register files (AMD-style)
        @vrf = VectorRegisterFile.new(
          num_vgprs: config.num_vgprs,
          wave_width: config.wave_width,
          fmt: config.float_format
        )
        @srf = ScalarRegisterFile.new(
          num_sgprs: config.num_sgprs,
          fmt: config.float_format
        )

        # Internal: one GPUCore per lane for instruction execution.
        @lanes = Array.new(config.wave_width) do
          GpuCore::GPUCore.new(
            isa: config.isa,
            fmt: config.float_format,
            num_registers: config.num_vgprs,
            memory_size: config.lds_size / [config.wave_width, 1].max
          )
        end

        @all_halted = false
      end

      # --- Properties (duck type interface) ---

      def name
        "WavefrontEngine"
      end

      def width
        @config.wave_width
      end

      def execution_model
        :simd
      end

      def exec_mask
        @exec_mask.dup
      end

      def halted?
        @all_halted
      end

      # --- Program loading ---

      def load_program(program)
        @program = program.dup
        @lanes.each { |lane| lane.load_program(@program) }
        @exec_mask = Array.new(@config.wave_width, true)
        @all_halted = false
        @cycle = 0
      end

      # --- Register setup ---

      # Set a per-lane vector register value.
      # @raise [IndexError] If lane is out of range.
      def set_lane_register(lane, vreg, value)
        if lane < 0 || lane >= @config.wave_width
          raise IndexError, "Lane #{lane} out of range [0, #{@config.wave_width})"
        end
        @vrf.write(vreg, lane, value)
        @lanes[lane].registers.write_float(vreg, value)
      end

      # Set a scalar register value (shared across all lanes).
      # @raise [IndexError] If sreg is out of range.
      def set_scalar_register(sreg, value)
        if sreg < 0 || sreg >= @config.num_sgprs
          raise IndexError,
            "Scalar register #{sreg} out of range [0, #{@config.num_sgprs})"
        end
        @srf.write(sreg, value)
      end

      # Explicitly set the EXEC mask.
      # @raise [ValueError] If mask length doesn't match wave_width.
      def set_exec_mask(mask)
        if mask.length != @config.wave_width
          raise ArgumentError,
            "Mask length #{mask.length} != wave_width #{@config.wave_width}"
        end
        @exec_mask = mask.dup
      end

      # --- Execution ---

      # Execute one cycle: issue one instruction to all active lanes.
      def step(clock_edge)
        @cycle += 1

        return make_halted_trace if @all_halted

        mask_before = @exec_mask.dup

        # Execute on active lanes only
        unit_traces = {}

        @config.wave_width.times do |lane_id|
          lane_core = @lanes[lane_id]
          if @exec_mask[lane_id] && !lane_core.halted?
            begin
              trace = lane_core.step
              unit_traces[lane_id] = trace.description
              unit_traces[lane_id] = "HALTED" if trace.halted
            rescue RuntimeError
              unit_traces[lane_id] = "(error)"
            end
          elsif lane_core.halted?
            unit_traces[lane_id] = "(halted)"
          else
            # Lane is masked off -- still advance its PC to stay in sync.
            if !lane_core.halted?
              begin
                lane_core.step
                unit_traces[lane_id] = "(masked -- result discarded)"
              rescue RuntimeError
                unit_traces[lane_id] = "(masked -- error)"
              end
            else
              unit_traces[lane_id] = "(halted)"
            end
          end
        end

        # Sync VRF with internal core registers for active lanes
        @config.wave_width.times do |lane_id|
          if @exec_mask[lane_id]
            [self.config.num_vgprs, 32].min.times do |vreg|
              val = @lanes[lane_id].registers.read_float(vreg)
              @vrf.write(vreg, lane_id, val)
            end
          end
        end

        # Check if all lanes halted
        @all_halted = true if @lanes.all?(&:halted?)

        active_count = @config.wave_width.times.count do |i|
          @exec_mask[i] && !@lanes[i].halted?
        end
        total = @config.wave_width

        # Build description
        skip_set = [
          "(masked -- result discarded)", "(halted)", "(error)",
          "(masked -- error)", "HALTED"
        ]
        first_desc = (0...@config.wave_width).each do |i|
          desc = unit_traces[i]
          break desc if desc && !skip_set.include?(desc)
        end
        first_desc = "no active lanes" if first_desc.is_a?(Integer)

        current_mask = (0...@config.wave_width).map do |i|
          @exec_mask[i] && !@lanes[i].halted?
        end

        EngineTrace.new(
          cycle: @cycle,
          engine_name: name,
          execution_model: execution_model,
          description: "#{first_desc} -- #{active_count}/#{total} lanes active",
          unit_traces: unit_traces,
          active_mask: current_mask,
          active_count: active_count,
          total_count: total,
          utilization: total > 0 ? active_count.to_f / total : 0.0,
          divergence_info: DivergenceInfo.new(
            active_mask_before: mask_before,
            active_mask_after: @exec_mask.dup,
            reconvergence_pc: -1,
            divergence_depth: 0
          )
        )
      end

      # Run until all lanes halt or max_cycles reached.
      def run(max_cycles: 10_000)
        traces = []
        (1..max_cycles).each do |cycle_num|
          edge = Clock::ClockEdge.new(
            cycle: cycle_num, value: 1,
            "rising?": true, "falling?": false
          )
          trace = step(edge)
          traces << trace
          break if @all_halted
        end

        if !@all_halted && traces.length >= max_cycles
          raise RuntimeError, "WavefrontEngine: max_cycles (#{max_cycles}) reached"
        end

        traces
      end

      # Reset to initial state.
      def reset
        @lanes.each do |lane|
          lane.reset
          lane.load_program(@program) if @program.any?
        end
        @exec_mask = Array.new(@config.wave_width, true)
        @all_halted = false
        @cycle = 0
        @vrf = VectorRegisterFile.new(
          num_vgprs: @config.num_vgprs,
          wave_width: @config.wave_width,
          fmt: @config.float_format
        )
        @srf = ScalarRegisterFile.new(
          num_sgprs: @config.num_sgprs,
          fmt: @config.float_format
        )
      end

      def to_s
        active = @exec_mask.count(true)
        "WavefrontEngine(width=#{@config.wave_width}, " \
          "active_lanes=#{active}, halted=#{@all_halted})"
      end

      def inspect
        to_s
      end

      private

      def make_halted_trace
        EngineTrace.new(
          cycle: @cycle,
          engine_name: name,
          execution_model: execution_model,
          description: "All lanes halted",
          unit_traces: (0...@config.wave_width).to_h { |i| [i, "(halted)"] },
          active_mask: Array.new(@config.wave_width, false),
          active_count: 0,
          total_count: @config.wave_width,
          utilization: 0.0
        )
      end
    end
  end
end

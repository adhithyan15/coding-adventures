# frozen_string_literal: true

# ---------------------------------------------------------------------------
# MACArrayEngine -- compiler-scheduled MAC array execution (NPU style).
# ---------------------------------------------------------------------------
#
# === What is a MAC Array? ===
#
# A MAC (Multiply-Accumulate) array is a bank of multiply-accumulate units
# driven entirely by a schedule that the compiler generates at compile time.
# There is NO hardware scheduler -- the compiler decides exactly which MAC
# unit processes which data on which cycle.
#
# This is the execution model used by:
# - Apple Neural Engine (ANE)
# - Qualcomm Hexagon NPU
# - Many custom AI accelerator ASICs
#
# === How It Differs from Other Models ===
#
#     GPU (SIMT/SIMD):                   NPU (Scheduled MAC):
#     +--------------------------+       +--------------------------+
#     | Hardware scheduler       |       | NO hardware scheduler    |
#     | Runtime decisions        |       | All decisions at compile  |
#     | Branch prediction        |       | NO branches              |
#     | Dynamic resource alloc   |       | Static resource plan     |
#     | Flexible but complex     |       | Simple but rigid         |
#     +--------------------------+       +--------------------------+
#
# === The Execution Pipeline ===
#
#     1. LOAD_INPUT:    Move data from external memory to input buffer
#     2. LOAD_WEIGHTS:  Move weights from external memory to weight buffer
#     3. MAC:           Multiply input[i] * weight[i] for all MACs in parallel
#     4. REDUCE:        Sum the MAC results (adder tree)
#     5. ACTIVATE:      Apply activation function (ReLU, sigmoid, tanh)
#     6. STORE_OUTPUT:  Write result to output buffer
#
# === Why NPUs Are Power-Efficient ===
#
# By moving all scheduling to compile time, NPUs eliminate:
# - Branch prediction hardware (saves transistors and power)
# - Instruction cache (the "program" is a simple schedule table)
# - Warp/wavefront scheduler (no runtime thread management)
# - Speculation hardware (nothing is speculative)

module CodingAdventures
  module ParallelExecutionEngine
    # -----------------------------------------------------------------------
    # MACOperation -- operations in a MAC array schedule
    # -----------------------------------------------------------------------
    #
    # We use Ruby symbols as our enum:
    #
    #     :load_input    Fill the input buffer with activation data.
    #     :load_weights  Fill the weight buffer with weight data.
    #     :mac           Parallel multiply-accumulate across all MAC units.
    #     :reduce        Sum results from multiple MACs (adder tree).
    #     :activate      Apply a non-linear activation function.
    #     :store_output  Write results to the output buffer.
    MAC_OPERATIONS = %i[load_input load_weights mac reduce activate store_output].freeze

    # -----------------------------------------------------------------------
    # ActivationFunction -- hardware-supported activation functions
    # -----------------------------------------------------------------------
    #
    # Neural networks use non-linear "activation functions" after each layer.
    # NPUs typically implement a few common ones in hardware:
    #
    #     :none    f(x) = x              (identity / linear)
    #     :relu    f(x) = max(0, x)      (most popular; simple, fast)
    #     :sigmoid f(x) = 1/(1+e^-x)    (classic; squashes to [0,1])
    #     :tanh    f(x) = tanh(x)        (squashes to [-1,1])
    ACTIVATION_FUNCTIONS = %i[none relu sigmoid tanh].freeze

    # -----------------------------------------------------------------------
    # MACScheduleEntry -- one entry in the MAC array schedule
    # -----------------------------------------------------------------------
    #
    # The compiler generates these at compile time. Each entry describes
    # exactly what happens on one cycle.
    MACScheduleEntry = Data.define(
      :cycle,
      :operation,
      :input_indices,
      :weight_indices,
      :output_index,
      :activation
    ) do
      def initialize(
        cycle:,
        operation:,
        input_indices: [],
        weight_indices: [],
        output_index: 0,
        activation: :none
      )
        super
      end
    end

    # -----------------------------------------------------------------------
    # MACArrayConfig -- configuration for a scheduled MAC array engine
    # -----------------------------------------------------------------------
    #
    # Real-world reference values:
    #
    #     Hardware          | MACs | Input Buf | Weight Buf | Format
    #     ------------------+------+-----------+------------+-------
    #     Apple ANE (M1)    | 16K  | varies    | varies     | FP16/INT8
    #     Qualcomm Hexagon  | 2K   | varies    | varies     | INT8
    #     Our default       | 8    | 1024      | 4096       | FP16
    MACArrayConfig = Data.define(
      :num_macs,
      :input_buffer_size,
      :weight_buffer_size,
      :output_buffer_size,
      :float_format,
      :accumulator_format,
      :has_activation_unit
    ) do
      def initialize(
        num_macs: 8,
        input_buffer_size: 1024,
        weight_buffer_size: 4096,
        output_buffer_size: 1024,
        float_format: FpArithmetic::FP16,
        accumulator_format: FpArithmetic::FP32,
        has_activation_unit: true
      )
        super
      end
    end

    # -----------------------------------------------------------------------
    # MACArrayEngine -- the scheduled execution engine
    # -----------------------------------------------------------------------
    #
    # No hardware scheduler. The compiler generates a static schedule that
    # says exactly what each MAC does on each cycle.
    class MACArrayEngine
      attr_reader :config

      def initialize(config, clock)
        @config = config
        @clock = clock
        @cycle = 0

        # Buffers: simple arrays of float values.
        @input_buffer = Array.new(config.input_buffer_size, 0.0)
        @weight_buffer = Array.new(config.weight_buffer_size, 0.0)
        @output_buffer = Array.new(config.output_buffer_size, 0.0)

        # MAC accumulators: one per MAC unit.
        @mac_accumulators = Array.new(config.num_macs, 0.0)

        # The compiler-generated schedule.
        @schedule = []
        @schedule_pc = 0
        @halted = false
      end

      # --- Properties (duck type interface) ---

      def name
        "MACArrayEngine"
      end

      def width
        @config.num_macs
      end

      def execution_model
        :scheduled_mac
      end

      def halted?
        @halted
      end

      # --- Data loading ---

      def load_inputs(data)
        data.each_with_index do |val, i|
          @input_buffer[i] = val if i < @config.input_buffer_size
        end
      end

      def load_weights(data)
        data.each_with_index do |val, i|
          @weight_buffer[i] = val if i < @config.weight_buffer_size
        end
      end

      def load_schedule(schedule)
        @schedule = schedule.dup
        @schedule_pc = 0
        @halted = false
      end

      # --- Execution ---

      # Execute one scheduled cycle.
      def step(clock_edge)
        @cycle += 1

        return make_idle_trace("Schedule complete") if @halted

        # Find schedule entries for this cycle
        entries = @schedule.select { |e| e.cycle == @cycle }

        if entries.empty?
          # Check if we've passed all schedule entries
          max_cycle = @schedule.map(&:cycle).max || 0
          if @cycle > max_cycle
            @halted = true
            return make_idle_trace("Schedule complete")
          end
          return make_idle_trace("No operation this cycle")
        end

        # Execute all entries for this cycle
        unit_traces = {}
        active_count = 0
        descriptions = []

        entries.each do |entry|
          case entry.operation
          when :load_input
            desc = exec_load_input(entry)
            descriptions << desc
            active_count = entry.input_indices.length
          when :load_weights
            desc = exec_load_weights(entry)
            descriptions << desc
            active_count = entry.weight_indices.length
          when :mac
            desc, traces = exec_mac(entry)
            descriptions << desc
            unit_traces.merge!(traces)
            active_count = traces.length
          when :reduce
            desc = exec_reduce(entry)
            descriptions << desc
            active_count = 1
          when :activate
            desc = exec_activate(entry)
            descriptions << desc
            active_count = 1
          when :store_output
            desc = exec_store(entry)
            descriptions << desc
            active_count = 1
          end
        end

        total = @config.num_macs
        description = descriptions.join("; ")

        EngineTrace.new(
          cycle: @cycle,
          engine_name: name,
          execution_model: execution_model,
          description: "#{description} -- #{active_count}/#{total} MACs active",
          unit_traces: unit_traces,
          active_mask: Array.new(total) { |i| i < active_count },
          active_count: active_count,
          total_count: total,
          utilization: total > 0 ? active_count.to_f / total : 0.0
        )
      end

      # Run the full schedule.
      def run(max_cycles: 10_000)
        traces = []
        (1..max_cycles).each do |cycle_num|
          edge = Clock::ClockEdge.new(
            cycle: cycle_num, value: 1,
            "rising?": true, "falling?": false
          )
          trace = step(edge)
          traces << trace
          break if @halted
        end

        if !@halted && traces.length >= max_cycles
          raise RuntimeError, "MACArrayEngine: max_cycles (#{max_cycles}) reached"
        end

        traces
      end

      # Read results from the output buffer.
      def read_outputs
        @output_buffer.dup
      end

      # Reset to initial state.
      def reset
        @input_buffer = Array.new(@config.input_buffer_size, 0.0)
        @weight_buffer = Array.new(@config.weight_buffer_size, 0.0)
        @output_buffer = Array.new(@config.output_buffer_size, 0.0)
        @mac_accumulators = Array.new(@config.num_macs, 0.0)
        @schedule_pc = 0
        @halted = false
        @cycle = 0
      end

      def to_s
        "MACArrayEngine(num_macs=#{@config.num_macs}, " \
          "cycle=#{@cycle}, halted=#{@halted})"
      end

      def inspect
        to_s
      end

      private

      def exec_load_input(entry)
        "LOAD_INPUT indices=#{entry.input_indices}"
      end

      def exec_load_weights(entry)
        "LOAD_WEIGHTS indices=#{entry.weight_indices}"
      end

      # Execute a MAC operation: multiply input[i] * weight[i] for each MAC.
      def exec_mac(entry)
        unit_traces = {}
        num_ops = [
          entry.input_indices.length,
          entry.weight_indices.length,
          @config.num_macs
        ].min

        num_ops.times do |mac_id|
          in_idx = entry.input_indices[mac_id]
          wt_idx = entry.weight_indices[mac_id]

          in_val = @input_buffer[in_idx]
          wt_val = @weight_buffer[wt_idx]

          result = in_val * wt_val
          @mac_accumulators[mac_id] = result

          unit_traces[mac_id] = format("MAC: %.4g * %.4g = %.4g", in_val, wt_val, result)
        end

        ["MAC #{num_ops} operations", unit_traces]
      end

      # Execute a REDUCE operation: sum all MAC accumulators.
      def exec_reduce(entry)
        total = @mac_accumulators.sum
        out_idx = entry.output_index
        @output_buffer[out_idx] = total if out_idx < @config.output_buffer_size
        format("REDUCE sum=%.4g -> output[#{out_idx}]", total)
      end

      # Execute an ACTIVATE operation: apply activation function.
      def exec_activate(entry)
        unless @config.has_activation_unit
          return "ACTIVATE skipped (no hardware activation unit)"
        end

        out_idx = entry.output_index
        if out_idx >= @config.output_buffer_size
          return "ACTIVATE error: index #{out_idx} out of range"
        end

        val = @output_buffer[out_idx]

        result = case entry.activation.to_s
        when "none"
          val
        when "relu"
          [0.0, val].max
        when "sigmoid"
          clamped = [[-500.0, val].max, 500.0].min
          1.0 / (1.0 + Math.exp(-clamped))
        when "tanh"
          Math.tanh(val)
        else
          val
        end

        @output_buffer[out_idx] = result
        format("ACTIVATE %s(%.4g) = %.4g", entry.activation, val, result)
      end

      def exec_store(entry)
        out_idx = entry.output_index
        val = out_idx < @config.output_buffer_size ? @output_buffer[out_idx] : 0.0
        format("STORE_OUTPUT output[#{out_idx}] = %.4g", val)
      end

      def make_idle_trace(description)
        EngineTrace.new(
          cycle: @cycle,
          engine_name: name,
          execution_model: execution_model,
          description: description,
          unit_traces: {},
          active_mask: Array.new(@config.num_macs, false),
          active_count: 0,
          total_count: @config.num_macs,
          utilization: 0.0
        )
      end
    end
  end
end

# frozen_string_literal: true

# ---------------------------------------------------------------------------
# SystolicArray -- dataflow execution for matrix multiplication (Google TPU style).
# ---------------------------------------------------------------------------
#
# === What is a Systolic Array? ===
#
# The word "systolic" comes from the Greek "systole" (contraction), like a
# heartbeat. In a systolic array, data pulses through a grid of processing
# elements on each clock cycle, just like blood pulses through the body with
# each heartbeat.
#
# A systolic array is radically different from GPU execution:
#
#     GPU (SIMT/SIMD):                   TPU (Systolic):
#     +--------------------------+       +--------------------------+
#     | Has instructions         |       | NO instructions           |
#     | Has program counter      |       | NO program counter        |
#     | Has branches             |       | NO branches               |
#     | Complex control logic    |       | Dead-simple PEs           |
#     | General-purpose          |       | Matrix multiply ONLY      |
#     +--------------------------+       +--------------------------+
#
# Each PE in the array does exactly ONE thing on each clock cycle:
#
#     accumulator += input_from_left * local_weight
#
# Then it passes the input to the right neighbor and the accumulator down.
# That's it. No instruction fetch, no decode, no branch prediction. Just
# multiply, accumulate, and pass.
#
# === Why TPUs Use Systolic Arrays ===
#
# Neural network inference and training are dominated by matrix multiplication
# (the GEMM operation). A systolic array is the most efficient hardware for
# matrix multiply because:
#
#     1. No instruction overhead (no fetch, decode, branch)
#     2. Maximum data reuse (each value is used N times as it flows through)
#     3. Nearest-neighbor communication only (each PE talks to adjacent PEs)
#     4. Regular, predictable data movement (no cache misses)
#     5. Simple PE design -> high clock frequency, low power

module CodingAdventures
  module ParallelExecutionEngine
    # -----------------------------------------------------------------------
    # SystolicConfig -- configuration for a systolic array engine
    # -----------------------------------------------------------------------
    #
    # Real-world reference values:
    #
    #     Hardware    | Rows | Cols | Format | Accumulator
    #     ------------+------+------+--------+------------
    #     TPU v1      | 256  | 256  | INT8   | INT32
    #     TPU v2/v3   | 128  | 128  | BF16   | FP32
    #     Our default | 4    | 4    | FP32   | FP32
    SystolicConfig = Data.define(
      :rows,
      :cols,
      :float_format,
      :accumulator_format
    ) do
      def initialize(
        rows: 4,
        cols: 4,
        float_format: FpArithmetic::FP32,
        accumulator_format: FpArithmetic::FP32
      )
        super
      end
    end

    # -----------------------------------------------------------------------
    # SystolicPE -- one processing element in the grid
    # -----------------------------------------------------------------------
    #
    # Each PE is extremely simple -- it's just a multiply-accumulate unit
    # with two data ports:
    #
    #     Input from left --> [  weight  ] --> Output to right
    #                         [  x + acc ]
    #                              |
    #                       Partial sum flows down
    #
    # On each clock cycle, a PE does:
    #     1. If there's an input: accumulator += input * weight
    #     2. Pass the input to the right neighbor
    class SystolicPE
      attr_reader :row, :col
      attr_accessor :weight, :accumulator, :input_buffer

      def initialize(row:, col:, weight:, accumulator:, input_buffer: nil)
        @row = row
        @col = col
        @weight = weight
        @accumulator = accumulator
        @input_buffer = input_buffer
      end

      # Perform one MAC cycle.
      #
      # If there's an input waiting in the buffer:
      #     accumulator += input_buffer * weight
      # Returns the input (to be passed to the right neighbor), or nil.
      def compute
        return nil if @input_buffer.nil?

        input_val = @input_buffer
        @input_buffer = nil

        # MAC: accumulator = input * weight + accumulator
        @accumulator = FpArithmetic.fp_fma(input_val, @weight, @accumulator)

        input_val # Pass to right neighbor
      end
    end

    # -----------------------------------------------------------------------
    # SystolicArray -- the dataflow execution engine
    # -----------------------------------------------------------------------
    #
    # An NxN grid of processing elements. Data flows through the array --
    # activations left-to-right, partial sums accumulate in each PE.
    # No instruction stream. Just data in, results out.
    #
    # === Data Flow Pattern ===
    #
    #     Inputs feed from the left edge:
    #
    #     a[0] --> PE(0,0) --> PE(0,1) --> PE(0,2) --> PE(0,3)
    #     a[1] --> PE(1,0) --> PE(1,1) --> PE(1,2) --> PE(1,3)
    #     a[2] --> PE(2,0) --> PE(2,1) --> PE(2,2) --> PE(2,3)
    #     a[3] --> PE(3,0) --> PE(3,1) --> PE(3,2) --> PE(3,3)
    class SystolicArray
      attr_reader :config, :grid

      def initialize(config, clock)
        @config = config
        @clock = clock
        @cycle = 0
        @halted = false

        # Create the NxN grid of PEs
        @grid = Array.new(config.rows) do |r|
          Array.new(config.cols) do |c|
            SystolicPE.new(
              row: r, col: c,
              weight: FpArithmetic.float_to_bits(0.0, config.float_format),
              accumulator: FpArithmetic.float_to_bits(0.0, config.accumulator_format)
            )
          end
        end

        # Input queues: one per row, feeding from the left edge.
        @input_queues = Array.new(config.rows) { [] }
        @total_inputs_fed = 0
        @total_inputs_expected = 0
      end

      # --- Properties (duck type interface) ---

      def name
        "SystolicArray"
      end

      def width
        @config.rows * @config.cols
      end

      def execution_model
        :systolic
      end

      def halted?
        @halted
      end

      # --- Weight loading ---

      # Pre-load the weight matrix into the PE array.
      # weights[row][col] goes to PE(row, col).
      def load_weights(weights)
        weights.each_with_index do |row_weights, r|
          break if r >= @config.rows
          row_weights.each_with_index do |w, c|
            break if c >= @config.cols
            @grid[r][c].weight = FpArithmetic.float_to_bits(w, @config.float_format)
          end
        end
      end

      # --- Input feeding ---

      # Feed one activation value into the left edge of the specified row.
      # @raise [IndexError] If row is out of range.
      def feed_input(row, value)
        if row < 0 || row >= @config.rows
          raise IndexError, "Row #{row} out of range [0, #{@config.rows})"
        end
        @input_queues[row] << FpArithmetic.float_to_bits(value, @config.float_format)
        @total_inputs_fed += 1
      end

      # Feed a full column vector to all rows.
      def feed_input_vector(values)
        values.each_with_index do |val, row_idx|
          fb = FpArithmetic.float_to_bits(val, @config.float_format)
          @input_queues[row_idx] << fb
          @total_inputs_fed += 1
        end
      end

      # --- Execution ---

      # Advance one cycle: data moves one PE to the right.
      def step(clock_edge)
        @cycle += 1

        active_count = 0
        pe_states = []

        # Phase 1: Move data rightward through the array.
        # Process from right to left to avoid data collision.
        @config.rows.times do |r|
          (@config.cols - 1).downto(0) do |c|
            pe = @grid[r][c]
            output = pe.compute

            if output
              active_count += 1
              # Pass input to right neighbor (if exists)
              @grid[r][c + 1].input_buffer = output if c + 1 < @config.cols
            end
          end

          # Build state strings (left to right for display)
          row_states = @config.cols.times.map do |c|
            pe = @grid[r][c]
            acc_val = FpArithmetic.bits_to_float(pe.accumulator)
            has_input = !pe.input_buffer.nil?
            state = format("acc=%.4g", acc_val)
            if has_input
              in_val = FpArithmetic.bits_to_float(pe.input_buffer)
              state += format(", in=%.4g", in_val)
            end
            state
          end
          pe_states << row_states
        end

        # Phase 2: Feed new inputs from queues into column 0
        @config.rows.times do |r|
          if @input_queues[r].any?
            val = @input_queues[r].shift
            @grid[r][0].input_buffer = val
          end
        end

        # Check if computation is complete
        total = @config.rows * @config.cols
        any_input_remaining = @input_queues.any? { |q| q.any? }
        any_input_in_flight = @grid.any? do |row|
          row.any? { |pe| !pe.input_buffer.nil? }
        end

        @halted = true if !any_input_remaining && !any_input_in_flight

        utilization = total > 0 ? active_count.to_f / total : 0.0

        EngineTrace.new(
          cycle: @cycle,
          engine_name: name,
          execution_model: execution_model,
          description: "Systolic step -- #{active_count}/#{total} PEs active",
          unit_traces: build_unit_traces(pe_states),
          active_mask: build_active_mask(active_count),
          active_count: active_count,
          total_count: total,
          utilization: utilization,
          dataflow_info: DataflowInfo.new(pe_states: pe_states)
        )
      end

      # Convenience: run a complete matrix multiplication C = A x W.
      def run_matmul(activations:, weights:)
        num_output_rows = activations.length
        inner_dim = activations[0]&.length || 0
        num_output_cols = weights[0]&.length || 0

        # Load weights: PE(k, j) gets W[k][j]
        reset
        load_weights(weights)

        result = []

        # Compute one output row at a time
        num_output_rows.times do |i|
          # Reset accumulators (but keep weights)
          zero_acc = FpArithmetic.float_to_bits(0.0, @config.accumulator_format)
          @config.rows.times do |r|
            @config.cols.times do |c|
              @grid[r][c].accumulator = zero_acc
              @grid[r][c].input_buffer = nil
            end
          end
          @input_queues = Array.new(@config.rows) { [] }
          @halted = false

          # Feed A[i][k] into row k with staggered timing
          feed_schedule = {}
          inner_dim.times do |k|
            cycle = k
            feed_schedule[cycle] ||= []
            feed_schedule[cycle] << [k, activations[i][k]]
          end

          # Run until all data has flowed through
          total_steps = inner_dim + @config.cols + 1
          total_steps.times do |step_num|
            if feed_schedule[step_num]
              feed_schedule[step_num].each do |row, val|
                feed_input(row, val)
              end
            end

            edge = Clock::ClockEdge.new(
              cycle: step_num + 1, value: 1,
              "rising?": true, "falling?": false
            )
            step(edge)
          end

          # Drain: sum accumulators vertically for each column j
          row_result = num_output_cols.times.map do |j|
            col_sum = 0.0
            [inner_dim, @config.rows].min.times do |k|
              col_sum += FpArithmetic.bits_to_float(@grid[k][j].accumulator)
            end
            col_sum
          end
          result << row_result
        end

        result
      end

      # Read the accumulated results from all PEs.
      def drain_outputs
        @config.rows.times.map do |r|
          @config.cols.times.map do |c|
            FpArithmetic.bits_to_float(@grid[r][c].accumulator)
          end
        end
      end

      # Reset the array to its initial state.
      def reset
        zero_acc = FpArithmetic.float_to_bits(0.0, @config.accumulator_format)
        @config.rows.times do |r|
          @config.cols.times do |c|
            @grid[r][c].accumulator = zero_acc
            @grid[r][c].input_buffer = nil
          end
        end
        @input_queues = Array.new(@config.rows) { [] }
        @cycle = 0
        @halted = false
        @total_inputs_fed = 0
      end

      def to_s
        "SystolicArray(#{@config.rows}x#{@config.cols}, " \
          "cycle=#{@cycle}, halted=#{@halted})"
      end

      def inspect
        to_s
      end

      private

      def build_unit_traces(pe_states)
        result = {}
        @config.rows.times do |r|
          @config.cols.times do |c|
            result[r * @config.cols + c] = pe_states[r][c]
          end
        end
        result
      end

      def build_active_mask(active_count)
        total = @config.rows * @config.cols
        Array.new(total) do |i|
          @grid.flatten.any? { |pe| !pe.input_buffer.nil? } || i < active_count
        end
      end
    end
  end
end

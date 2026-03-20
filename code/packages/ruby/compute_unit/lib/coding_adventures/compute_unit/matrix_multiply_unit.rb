# frozen_string_literal: true

# ---------------------------------------------------------------------------
# MatrixMultiplyUnit -- Google TPU MXU simulator.
# ---------------------------------------------------------------------------
#
# === What is an MXU? ===
#
# The Matrix Multiply Unit is the heart of Google's TPU (Tensor Processing
# Unit). It's fundamentally different from GPU compute units -- there are NO
# threads, NO warps, NO schedulers. Instead, it has:
#
# 1. **Systolic arrays** -- the main compute engine (from Layer 8)
# 2. **Vector unit** -- for element-wise operations (activation functions)
# 3. **Accumulators** -- for storing partial matrix results
# 4. **Control sequencer** -- manages the tiling schedule
#
# === Why No Threads? ===
#
# Matrix multiplication is perfectly predictable. You know exactly which
# values need to be multiplied together and in what order. There's no
# branching, no data-dependent control flow, no need for a runtime scheduler.
#
#     GPU:  Complex hardware scheduler decides at runtime
#     TPU:  Simple hardware follows compile-time plan
#
# === Tiling: How Large Matmuls Fit Small Arrays ===
#
# A TPU v2 has a 128x128 systolic array, but neural networks often need
# matmuls like 1024x1024 or even 4096x4096. The solution is **tiling**:
#
#     Large matmul: C[1024x1024] = A[1024x1024] x B[1024x1024]
#
#     The MXU can only do 128x128 at a time, so:
#
#     for i in (0...1024).step(128)        # 8 row tiles
#       for j in (0...1024).step(128)      # 8 column tiles
#         acc = 0
#         for k in (0...1024).step(128)    # 8 reduction tiles
#           load A[i:i+128, k:k+128] into activation buffer
#           load B[k:k+128, j:j+128] into weight buffer
#           acc += systolic_matmul(A_tile, B_tile)
#         C[i:i+128, j:j+128] = apply_vector_ops(acc)

module CodingAdventures
  module ComputeUnit
    # -----------------------------------------------------------------------
    # MXUConfig -- configuration for a TPU-style Matrix Multiply Unit
    # -----------------------------------------------------------------------
    #
    # Real-world MXU configurations:
    #
    #     Parameter           | TPU v1       | TPU v2/v3    | TPU v4
    #     --------------------+--------------+--------------+---------
    #     Array size          | 256x256      | 128x128      | 128x128
    #     Input format        | INT8         | BF16         | BF16
    #     Accumulator format  | INT32        | FP32         | FP32
    #     Vector width        | 256          | 128          | 128
    #     HBM bandwidth       | 30 GB/s      | 900 GB/s     | 1200 GB/s
    MXUConfig = Data.define(
      :array_rows,
      :array_cols,
      :systolic_format,
      :accumulator_format,
      :vector_width,
      :vector_format,
      :accumulator_count,
      :weight_buffer_size,
      :activation_buffer_size
    ) do
      def initialize(
        array_rows: 128,
        array_cols: 128,
        systolic_format: FpArithmetic::BF16,
        accumulator_format: FpArithmetic::FP32,
        vector_width: 128,
        vector_format: FpArithmetic::FP32,
        accumulator_count: 128,
        weight_buffer_size: 4_194_304,
        activation_buffer_size: 2_097_152
      )
        super
      end
    end

    # -----------------------------------------------------------------------
    # MatrixMultiplyUnit -- the main MXU simulator
    # -----------------------------------------------------------------------
    #
    # Uses a systolic array from Layer 8 to perform matrix multiplication,
    # with tiling logic for matrices larger than the array, and a vector
    # unit for post-processing (activation functions, bias add).
    #
    # === Execution Model ===
    #
    # The MXU has no threads or schedulers. Instead, it processes **tiles**
    # of a larger matrix operation. The control sequencer manages:
    #
    # 1. Loading weight tiles into the systolic array
    # 2. Streaming activation tiles through the array
    # 3. Accumulating partial results
    # 4. Applying vector operations (activation functions)
    # 5. Storing output tiles
    class MatrixMultiplyUnit
      attr_reader :config, :systolic_array

      def initialize(config, clock)
        @config = config
        @clock = clock
        @cycle = 0

        # Create the systolic array engine
        @systolic_array = ParallelExecutionEngine::SystolicArray.new(
          ParallelExecutionEngine::SystolicConfig.new(
            rows: config.array_rows,
            cols: config.array_cols,
            float_format: FpArithmetic::FP32,
            accumulator_format: FpArithmetic::FP32
          ),
          clock
        )

        # Accumulators for storing partial tile results
        @accumulators = []

        # Result storage
        @current_result = []

        # Work queue
        @work_items = []
        @idle_flag = true
      end

      # --- Properties ---

      def name
        "MXU"
      end

      def architecture
        :google_mxu
      end

      # True if no work remains.
      def idle?
        @idle_flag
      end

      # The result matrix from the last matmul.
      def result
        @current_result
      end

      # --- Dispatch ---

      # Dispatch a matrix multiply operation.
      #
      # The WorkItem must provide input_data (activation matrix) and
      # weight_data (weight matrix). The MXU will perform:
      #
      #     result = input_data x weight_data
      #
      # @param work [WorkItem] WorkItem with input_data and weight_data set.
      def dispatch(work)
        @work_items << work
        @idle_flag = false
      end

      # --- Execution ---

      # Advance one cycle of the MXU.
      #
      # If work is pending, performs the matmul using the systolic array.
      #
      # @param clock_edge [ClockEdge] The clock edge that triggered this step.
      # @return [ComputeUnitTrace] A trace for this cycle.
      def step(clock_edge)
        @cycle += 1

        if @idle_flag || @work_items.empty?
          return make_idle_trace
        end

        # Process the first pending work item
        work = @work_items[0]

        if work.input_data && work.weight_data
          @current_result = @systolic_array.run_matmul(
            activations: work.input_data,
            weights: work.weight_data
          )
        else
          @current_result = []
        end

        # Mark work as done
        @work_items.shift
        @idle_flag = true if @work_items.empty?

        # Build trace
        rows = @current_result.length
        cols = @current_result.empty? ? 0 : @current_result[0].length

        ComputeUnitTrace.new(
          cycle: @cycle,
          unit_name: name,
          architecture: architecture,
          scheduler_action: "matmul complete: #{rows}x#{cols} result",
          active_warps: @idle_flag ? 0 : 1,
          total_warps: 1,
          engine_traces: {},
          shared_memory_used: 0,
          shared_memory_total: @config.weight_buffer_size,
          register_file_used: @config.accumulator_count,
          register_file_total: @config.accumulator_count,
          occupancy: @idle_flag ? 0.0 : 1.0
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

      # Convenience: run a complete matmul with optional activation.
      #
      # === Supported Activation Functions ===
      #
      #     none:    f(x) = x              (identity)
      #     relu:    f(x) = max(0, x)      (most popular)
      #     sigmoid: f(x) = 1/(1+e^-x)    (squashes to [0,1])
      #     tanh:    f(x) = tanh(x)        (squashes to [-1,1])
      #
      # @param activations [Array<Array<Float>>] Input matrix A (M x K).
      # @param weights [Array<Array<Float>>] Weight matrix W (K x N).
      # @param activation_fn [String] Activation function name.
      # @return [Array<Array<Float>>] Result matrix C = activation_fn(A x W).
      def run_matmul(activations:, weights:, activation_fn: "none")
        result = @systolic_array.run_matmul(activations: activations, weights: weights)

        if activation_fn != "none"
          result = apply_activation(result, activation_fn)
        end

        @current_result = result
        result
      end

      # Reset all state.
      def reset
        @systolic_array.reset
        @accumulators.clear
        @current_result.clear
        @work_items.clear
        @idle_flag = true
        @cycle = 0
      end

      def to_s
        "MatrixMultiplyUnit(#{@config.array_rows}x#{@config.array_cols}, " \
          "idle=#{@idle_flag})"
      end

      def inspect
        to_s
      end

      private

      # Apply an activation function element-wise to a matrix.
      # Simulates the MXU's vector unit.
      def apply_activation(matrix, fn_name)
        matrix.map do |row|
          row.map do |val|
            case fn_name
            when "relu" then [0.0, val].max
            when "sigmoid"
              clamped = [[-500.0, val].max, 500.0].min
              1.0 / (1.0 + Math.exp(-clamped))
            when "tanh" then Math.tanh(val)
            else val
            end
          end
        end
      end

      # Produce a trace for when the MXU is idle.
      def make_idle_trace
        ComputeUnitTrace.new(
          cycle: @cycle,
          unit_name: name,
          architecture: architecture,
          scheduler_action: "idle",
          active_warps: 0,
          total_warps: 1,
          engine_traces: {},
          shared_memory_used: 0,
          shared_memory_total: @config.weight_buffer_size,
          register_file_used: 0,
          register_file_total: @config.accumulator_count,
          occupancy: 0.0
        )
      end
    end
  end
end

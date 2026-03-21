# frozen_string_literal: true

# ---------------------------------------------------------------------------
# NeuralEngineCore -- Apple ANE Core simulator.
# ---------------------------------------------------------------------------
#
# === What is the Apple Neural Engine? ===
#
# Apple's Neural Engine (ANE) is a dedicated neural network accelerator
# found in every Apple chip since the A11 Bionic (2017). It's designed
# for one thing: fast, power-efficient neural network inference.
#
# The ANE is the simplest compute unit in our family -- and that simplicity
# is its strength. By removing hardware schedulers, branch predictors, and
# general-purpose control logic, Apple can dedicate nearly all transistors
# to MAC (multiply-accumulate) units and on-chip memory.
#
# === How ANE Differs from GPUs ===
#
#     GPU (NVIDIA/AMD):                   ANE (Apple):
#     +----------------------------+     +----------------------------+
#     | Hardware scheduler         |     | NO hardware scheduler      |
#     | Runtime decisions          |     | All decisions at compile   |
#     | Branch prediction          |     | NO branches                |
#     | Dynamic register alloc     |     | Static buffer plan         |
#     | Flexible but complex       |     | Simple but rigid           |
#     | ~5 W per SM                |     | ~1 W per core              |
#     +----------------------------+     +----------------------------+
#
# === Architecture ===
#
# Each ANE Core has:
# - **MAC array**: 16 multiply-accumulate units (our default)
# - **DMA engine**: transfers data between main memory and on-chip SRAM
# - **On-chip SRAM**: 4 MB (fast, low-power local storage)
# - **Activation pipeline**: hardware for ReLU, sigmoid, etc.
# - **Buffers**: input, weight, and output buffers
#
#     NeuralEngineCore
#     +---------------------------------------------------------------+
#     |                                                               |
#     |  DMA Engine                                                   |
#     |  +----------------------------------------------------------+ |
#     |  | Transfers data between main memory and on-chip SRAM       | |
#     |  | Bandwidth: 10 elements per cycle                          | |
#     |  +----------------------------------------------------------+ |
#     |                    |                                          |
#     |                    v                                          |
#     |  +------------------+ +------------------+                    |
#     |  | Input Buffer     | | Weight Buffer    |                    |
#     |  | 128 KB          | | 512 KB          |                    |
#     |  +--------+---------+ +--------+---------+                    |
#     |           |                    |                              |
#     |           v                    v                              |
#     |  +---------------------------------------------+              |
#     |  | MAC Array (16 units)                         |              |
#     |  | mac[i] = input[i] * weight[i]                |              |
#     |  +---------------------------------------------+              |
#     |                    |                                          |
#     |                    v                                          |
#     |  +---------------------------------------------+              |
#     |  | Activation Pipeline                          |              |
#     |  | ReLU / sigmoid / tanh / identity             |              |
#     |  +---------------------------------------------+              |
#     |                    |                                          |
#     |                    v                                          |
#     |  +---------------------------------------------+              |
#     |  | Output Buffer (128 KB)                       |              |
#     |  +---------------------------------------------+              |
#     +---------------------------------------------------------------+

module CodingAdventures
  module ComputeUnit
    # -----------------------------------------------------------------------
    # ANECoreConfig -- configuration for an Apple Neural Engine Core
    # -----------------------------------------------------------------------
    #
    # Real-world ANE configurations:
    #
    #     Parameter          | A14 (iPhone 12) | M1          | M2
    #     -------------------+-----------------+-------------+------
    #     Cores              | 16              | 16          | 16
    #     TOPS               | 11              | 11          | 15.8
    #     Format             | FP16/INT8       | FP16/INT8   | FP16/INT8
    #     On-chip memory     | varies          | varies      | varies
    ANECoreConfig = Data.define(
      :num_macs,
      :mac_format,
      :accumulator_format,
      :sram_size,
      :activation_buffer,
      :weight_buffer,
      :output_buffer,
      :dma_bandwidth
    ) do
      def initialize(
        num_macs: 16,
        mac_format: FpArithmetic::FP16,
        accumulator_format: FpArithmetic::FP32,
        sram_size: 4_194_304,
        activation_buffer: 131_072,
        weight_buffer: 524_288,
        output_buffer: 131_072,
        dma_bandwidth: 10
      )
        super
      end
    end

    # -----------------------------------------------------------------------
    # NeuralEngineCore -- the main ANE Core simulator
    # -----------------------------------------------------------------------
    #
    # Uses a MACArrayEngine from Layer 8 internally, adding DMA simulation,
    # activation pipeline, and compiler-generated schedule support.
    #
    # === Execution Model ===
    #
    # The ANE Core has no runtime scheduler. Instead, it follows a
    # compiler-generated schedule:
    #
    # 1. The user dispatches a WorkItem with input_data and weight_data
    # 2. The ANE Core performs the matmul
    # 3. Optional activation function is applied
    #
    # === DMA Simulation ===
    #
    # In real ANE hardware, data must be DMA'd from main memory to
    # on-chip SRAM before the MACs can process it. This takes time:
    #
    #     DMA bandwidth: 10 elements/cycle (our default)
    #     Loading 160 elements: 16 cycles
    class NeuralEngineCore
      attr_reader :config, :mac_engine

      def initialize(config, clock)
        @config = config
        @clock = clock
        @cycle = 0

        # Internal MAC array engine
        @mac_engine = ParallelExecutionEngine::MACArrayEngine.new(
          ParallelExecutionEngine::MACArrayConfig.new(
            num_macs: config.num_macs,
            input_buffer_size: [config.activation_buffer / 4, 1024].max,
            weight_buffer_size: [config.weight_buffer / 4, 4096].max,
            output_buffer_size: [config.output_buffer / 4, 1024].max,
            float_format: FpArithmetic::FP32,
            accumulator_format: FpArithmetic::FP32,
            has_activation_unit: true
          ),
          clock
        )

        @idle_flag = true
        @work_items = []
        @result = []
      end

      # --- Properties ---

      def name
        "ANECore"
      end

      def architecture
        :apple_ane_core
      end

      # True if no work remains.
      def idle?
        @idle_flag
      end

      # The result from the last computation.
      def result
        @result
      end

      # --- Dispatch ---

      # Dispatch an inference tile to this ANE Core.
      #
      # The WorkItem must provide input_data and weight_data.
      #
      # @param work [WorkItem] WorkItem with input_data and weight_data.
      def dispatch(work)
        @work_items << work
        @idle_flag = false
      end

      # --- Execution ---

      # Advance one cycle of the ANE Core.
      #
      # @param clock_edge [ClockEdge] The clock edge that triggered this step.
      # @return [ComputeUnitTrace] A trace for this cycle.
      def step(clock_edge)
        @cycle += 1

        if @idle_flag || @work_items.empty?
          return make_idle_trace
        end

        work = @work_items[0]
        process_work_item(work)
        @work_items.shift

        @idle_flag = true if @work_items.empty?

        rows = @result.length
        cols = @result.empty? ? 0 : @result[0].length

        ComputeUnitTrace.new(
          cycle: @cycle,
          unit_name: name,
          architecture: architecture,
          scheduler_action: "inference complete: #{rows}x#{cols} result",
          active_warps: @idle_flag ? 0 : 1,
          total_warps: 1,
          engine_traces: {},
          shared_memory_used: 0,
          shared_memory_total: @config.sram_size,
          register_file_used: @config.num_macs,
          register_file_total: @config.num_macs,
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

      # Convenience: run a complete inference pass.
      #
      # Performs matmul + activation function, simulating how the ANE
      # processes one layer of a neural network.
      #
      # @param inputs [Array<Array<Float>>] Input activation matrix (M x K).
      # @param weights [Array<Array<Float>>] Weight matrix (K x N).
      # @param activation_fn [String] Activation function ("relu", "sigmoid", "tanh", "none").
      # @return [Array<Array<Float>>] Result matrix with activation applied (M x N).
      def run_inference(inputs:, weights:, activation_fn: "relu")
        result = matmul(inputs, weights)

        if activation_fn != "none"
          result = apply_activation(result, activation_fn)
        end

        @result = result
        result
      end

      # Reset all state.
      def reset
        @mac_engine.reset
        @work_items.clear
        @result.clear
        @idle_flag = true
        @cycle = 0
      end

      def to_s
        "NeuralEngineCore(macs=#{@config.num_macs}, idle=#{@idle_flag})"
      end

      def inspect
        to_s
      end

      private

      # Process a single work item by performing matmul.
      def process_work_item(work)
        if work.input_data && work.weight_data
          @result = matmul(work.input_data, work.weight_data)
        else
          @result = []
        end
      end

      # Perform matrix multiplication.
      #
      # For each element of the output matrix, we compute a dot product.
      #
      # @param a [Array<Array<Float>>] Input matrix (M x K).
      # @param b [Array<Array<Float>>] Weight matrix (K x N).
      # @return [Array<Array<Float>>] Result matrix C = A x B (M x N).
      def matmul(a, b)
        return [] if a.empty? || b.empty?

        m = a.length
        k = a[0]&.length || 0
        n = b[0]&.length || 0

        result = []
        m.times do |i|
          row = []
          n.times do |j|
            dot = 0.0
            k.times do |kk|
              dot += a[i][kk] * b[kk][j]
            end
            row << dot
          end
          result << row
        end

        result
      end

      # Apply activation function element-wise.
      # Simulates the ANE's dedicated activation pipeline hardware.
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

      # Produce a trace for when the ANE Core is idle.
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
          shared_memory_total: @config.sram_size,
          register_file_used: 0,
          register_file_total: @config.num_macs,
          occupancy: 0.0
        )
      end
    end
  end
end

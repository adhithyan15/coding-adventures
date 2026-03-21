# frozen_string_literal: true

require_relative "test_helper"

# ---------------------------------------------------------------------------
# Tests for the Apple NeuralEngineCore simulator.
# ---------------------------------------------------------------------------

class TestNeuralEngineCore < Minitest::Test
  include CodingAdventures

  def setup
    @clock = Clock::ClockGenerator.new
    @config = ComputeUnit::ANECoreConfig.new(num_macs: 4)
    @ane = ComputeUnit::NeuralEngineCore.new(@config, @clock)
  end

  # --- Properties ---

  def test_name
    assert_equal "ANECore", @ane.name
  end

  def test_architecture
    assert_equal :apple_ane_core, @ane.architecture
  end

  def test_idle_when_empty
    assert @ane.idle?
  end

  # --- Dispatch and run ---

  def test_dispatch_and_run
    work = ComputeUnit::WorkItem.new(
      work_id: 0,
      input_data: [[1.0, 2.0, 3.0, 4.0]],
      weight_data: [[0.5], [0.5], [0.5], [0.5]]
    )

    @ane.dispatch(work)
    refute @ane.idle?

    traces = @ane.run
    assert @ane.idle?

    # dot product: 1*0.5 + 2*0.5 + 3*0.5 + 4*0.5 = 5.0
    assert_in_delta 5.0, @ane.result[0][0], 0.01
  end

  # --- run_inference convenience method ---

  def test_run_inference_relu
    inputs = [[1.0, -2.0]]
    weights = [[1.0], [1.0]]

    # result = [[1*1 + (-2)*1]] = [[-1.0]]
    # relu(-1.0) = 0.0
    result = @ane.run_inference(inputs: inputs, weights: weights, activation_fn: "relu")
    assert_in_delta 0.0, result[0][0], 0.01
  end

  def test_run_inference_sigmoid
    inputs = [[1.0]]
    weights = [[0.0]]

    # result = [[0.0]], sigmoid(0.0) = 0.5
    result = @ane.run_inference(inputs: inputs, weights: weights, activation_fn: "sigmoid")
    assert_in_delta 0.5, result[0][0], 0.01
  end

  def test_run_inference_tanh
    inputs = [[1.0]]
    weights = [[0.0]]

    result = @ane.run_inference(inputs: inputs, weights: weights, activation_fn: "tanh")
    assert_in_delta 0.0, result[0][0], 0.01
  end

  def test_run_inference_none
    inputs = [[2.0]]
    weights = [[3.0]]

    result = @ane.run_inference(inputs: inputs, weights: weights, activation_fn: "none")
    assert_in_delta 6.0, result[0][0], 0.01
  end

  # --- 2x2 matmul ---

  def test_2x2_matmul
    work = ComputeUnit::WorkItem.new(
      work_id: 0,
      input_data: [[1.0, 2.0], [3.0, 4.0]],
      weight_data: [[5.0, 6.0], [7.0, 8.0]]
    )

    @ane.dispatch(work)
    @ane.run

    result = @ane.result
    assert_in_delta 19.0, result[0][0], 0.01
    assert_in_delta 22.0, result[0][1], 0.01
    assert_in_delta 43.0, result[1][0], 0.01
    assert_in_delta 50.0, result[1][1], 0.01
  end

  # --- Trace contents ---

  def test_trace_fields
    @ane.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0,
      input_data: [[1.0]],
      weight_data: [[2.0]]
    ))
    traces = @ane.run

    refute_empty traces
    assert_equal "ANECore", traces[0].unit_name
    assert_equal :apple_ane_core, traces[0].architecture
    assert_includes traces.last.scheduler_action, "inference complete"
  end

  # --- Idle trace ---

  def test_idle_trace
    edge = Clock::ClockEdge.new(cycle: 1, value: 1, "rising?": true, "falling?": false)
    trace = @ane.step(edge)
    assert_equal "idle", trace.scheduler_action
    assert_in_delta 0.0, trace.occupancy
  end

  # --- Empty work item ---

  def test_dispatch_without_data
    @ane.dispatch(ComputeUnit::WorkItem.new(work_id: 0))
    @ane.run
    assert_empty @ane.result
  end

  # --- Reset ---

  def test_reset
    @ane.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0,
      input_data: [[1.0]],
      weight_data: [[2.0]]
    ))
    @ane.run
    @ane.reset

    assert @ane.idle?
    assert_empty @ane.result
  end

  # --- Multiple dispatches ---

  def test_multiple_dispatches
    @ane.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0,
      input_data: [[1.0]],
      weight_data: [[2.0]]
    ))
    @ane.dispatch(ComputeUnit::WorkItem.new(
      work_id: 1,
      input_data: [[3.0]],
      weight_data: [[4.0]]
    ))

    traces = @ane.run
    assert @ane.idle?
    assert_in_delta 12.0, @ane.result[0][0], 0.01
  end

  # --- to_s ---

  def test_to_s
    str = @ane.to_s
    assert_includes str, "NeuralEngineCore"
    assert_includes str, "macs="
  end
end

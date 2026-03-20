# frozen_string_literal: true

require_relative "test_helper"

# ---------------------------------------------------------------------------
# Tests for the Google TPU MatrixMultiplyUnit (MXU) simulator.
# ---------------------------------------------------------------------------

class TestMatrixMultiplyUnit < Minitest::Test
  include CodingAdventures

  def setup
    @clock = Clock::ClockGenerator.new
    @config = ComputeUnit::MXUConfig.new(
      array_rows: 4,
      array_cols: 4,
      accumulator_count: 4
    )
    @mxu = ComputeUnit::MatrixMultiplyUnit.new(@config, @clock)
  end

  # --- Properties ---

  def test_name
    assert_equal "MXU", @mxu.name
  end

  def test_architecture
    assert_equal :google_mxu, @mxu.architecture
  end

  def test_idle_when_empty
    assert @mxu.idle?
  end

  # --- Simple matmul via dispatch ---

  def test_dispatch_and_run_2x2
    work = ComputeUnit::WorkItem.new(
      work_id: 0,
      input_data: [[1.0, 2.0], [3.0, 4.0]],
      weight_data: [[5.0, 6.0], [7.0, 8.0]]
    )

    @mxu.dispatch(work)
    refute @mxu.idle?

    traces = @mxu.run
    assert @mxu.idle?

    # C = A x B
    # [[1*5+2*7, 1*6+2*8], [3*5+4*7, 3*6+4*8]] = [[19, 22], [43, 50]]
    result = @mxu.result
    assert_equal 2, result.length
    assert_in_delta 19.0, result[0][0], 0.1
    assert_in_delta 22.0, result[0][1], 0.1
    assert_in_delta 43.0, result[1][0], 0.1
    assert_in_delta 50.0, result[1][1], 0.1
  end

  # --- run_matmul convenience method ---

  def test_run_matmul_identity
    # Multiply by identity matrix
    a = [[1.0, 0.0], [0.0, 1.0]]
    b = [[3.0, 4.0], [5.0, 6.0]]

    result = @mxu.run_matmul(activations: a, weights: b)
    assert_in_delta 3.0, result[0][0], 0.1
    assert_in_delta 4.0, result[0][1], 0.1
    assert_in_delta 5.0, result[1][0], 0.1
    assert_in_delta 6.0, result[1][1], 0.1
  end

  # --- Activation functions ---

  def test_run_matmul_with_relu
    # Result will have negative values that ReLU zeroes out
    a = [[1.0, 0.0]]
    b = [[-2.0], [3.0]]

    result = @mxu.run_matmul(activations: a, weights: b, activation_fn: "relu")
    assert_in_delta 0.0, result[0][0], 0.01  # relu(-2.0) = 0.0
  end

  def test_run_matmul_with_sigmoid
    a = [[1.0]]
    b = [[0.0]]

    result = @mxu.run_matmul(activations: a, weights: b, activation_fn: "sigmoid")
    assert_in_delta 0.5, result[0][0], 0.01  # sigmoid(0) = 0.5
  end

  def test_run_matmul_with_tanh
    a = [[1.0]]
    b = [[0.0]]

    result = @mxu.run_matmul(activations: a, weights: b, activation_fn: "tanh")
    assert_in_delta 0.0, result[0][0], 0.01  # tanh(0) = 0.0
  end

  # --- Trace contents ---

  def test_trace_on_dispatch
    work = ComputeUnit::WorkItem.new(
      work_id: 0,
      input_data: [[1.0]],
      weight_data: [[2.0]]
    )
    @mxu.dispatch(work)
    traces = @mxu.run

    refute_empty traces
    assert_equal "MXU", traces[0].unit_name
    assert_equal :google_mxu, traces[0].architecture
    assert_includes traces.last.scheduler_action, "matmul complete"
  end

  # --- Idle trace ---

  def test_idle_trace
    edge = Clock::ClockEdge.new(cycle: 1, value: 1, "rising?": true, "falling?": false)
    trace = @mxu.step(edge)
    assert_equal "idle", trace.scheduler_action
    assert_in_delta 0.0, trace.occupancy
  end

  # --- Reset ---

  def test_reset
    @mxu.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0,
      input_data: [[1.0]],
      weight_data: [[2.0]]
    ))
    @mxu.run
    @mxu.reset

    assert @mxu.idle?
    assert_empty @mxu.result
  end

  # --- Multiple dispatches ---

  def test_multiple_dispatches_processed_sequentially
    @mxu.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0,
      input_data: [[1.0]],
      weight_data: [[2.0]]
    ))
    @mxu.dispatch(ComputeUnit::WorkItem.new(
      work_id: 1,
      input_data: [[3.0]],
      weight_data: [[4.0]]
    ))

    traces = @mxu.run
    assert @mxu.idle?
    # Last result should be from second work item
    assert_in_delta 12.0, @mxu.result[0][0], 0.1
  end

  # --- to_s ---

  def test_to_s
    str = @mxu.to_s
    assert_includes str, "MatrixMultiplyUnit"
    assert_includes str, "4x4"
  end
end

# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_device_simulator"

# Tests for work distributors -- GPU, TPU, and ANE strategies.
class TestGPUWorkDistributor < Minitest::Test
  include CodingAdventures

  def make_sms(n = 4)
    clk = Clock::ClockGenerator.new(frequency_hz: 1_000_000)
    config = ComputeUnit::SMConfig.new(
      max_warps: 4, num_schedulers: 1,
      shared_memory_size: 1024, register_file_size: 2048
    )
    sms = Array.new(n) { ComputeUnit::StreamingMultiprocessor.new(config, clk) }
    [sms, clk]
  end

  def test_submit_kernel_creates_blocks
    sms, _ = make_sms(2)
    dist = DeviceSimulator::GPUWorkDistributor.new(sms)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "test", grid_dim: [4, 1, 1], block_dim: [32, 1, 1]
    )
    dist.submit_kernel(kernel)
    assert_equal 4, dist.pending_count
  end

  def test_step_dispatches_blocks
    sms, _ = make_sms(2)
    dist = DeviceSimulator::GPUWorkDistributor.new(sms)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "test",
      program: [GpuCore.limm(0, 1.0), GpuCore.halt],
      grid_dim: [2, 1, 1], block_dim: [32, 1, 1]
    )
    dist.submit_kernel(kernel)
    actions = dist.step
    assert actions.length >= 1
    assert dist.pending_count < 2
  end

  def test_round_robin_distributes_evenly
    sms, _ = make_sms(4)
    dist = DeviceSimulator::GPUWorkDistributor.new(sms, policy: "round_robin")
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "test",
      program: [GpuCore.limm(0, 1.0), GpuCore.halt],
      grid_dim: [4, 1, 1], block_dim: [32, 1, 1]
    )
    dist.submit_kernel(kernel)
    dist.step
    assert dist.total_dispatched > 0
  end

  def test_total_dispatched_tracks
    sms, _ = make_sms(2)
    dist = DeviceSimulator::GPUWorkDistributor.new(sms)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "test",
      program: [GpuCore.limm(0, 1.0), GpuCore.halt],
      grid_dim: [2, 1, 1], block_dim: [32, 1, 1]
    )
    dist.submit_kernel(kernel)
    dist.step
    assert dist.total_dispatched >= 1
  end

  def test_empty_step_returns_no_actions
    sms, _ = make_sms(2)
    dist = DeviceSimulator::GPUWorkDistributor.new(sms)
    actions = dist.step
    assert_equal [], actions
  end

  def test_reset_clears_pending
    sms, _ = make_sms(2)
    dist = DeviceSimulator::GPUWorkDistributor.new(sms)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "test", grid_dim: [4, 1, 1], block_dim: [32, 1, 1]
    )
    dist.submit_kernel(kernel)
    dist.reset
    assert_equal 0, dist.pending_count
    assert_equal 0, dist.total_dispatched
  end

  def test_fill_first_policy
    sms, _ = make_sms(2)
    dist = DeviceSimulator::GPUWorkDistributor.new(sms, policy: "fill_first")
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "test",
      program: [GpuCore.limm(0, 1.0), GpuCore.halt],
      grid_dim: [2, 1, 1], block_dim: [32, 1, 1]
    )
    dist.submit_kernel(kernel)
    dist.step
    assert dist.total_dispatched >= 1
  end

  def test_least_loaded_policy
    sms, _ = make_sms(2)
    dist = DeviceSimulator::GPUWorkDistributor.new(sms, policy: "least_loaded")
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "test",
      program: [GpuCore.limm(0, 1.0), GpuCore.halt],
      grid_dim: [2, 1, 1], block_dim: [32, 1, 1]
    )
    dist.submit_kernel(kernel)
    dist.step
    assert dist.total_dispatched >= 1
  end

  def test_kernel_descriptor_properties
    k = DeviceSimulator::KernelDescriptor.new(
      grid_dim: [4, 2, 1], block_dim: [16, 16, 1]
    )
    assert_equal 8, k.total_blocks
    assert_equal 256, k.threads_per_block
    assert_equal 2048, k.total_threads
  end
end

class TestTPUSequencer < Minitest::Test
  include CodingAdventures

  def make_mxu
    clk = Clock::ClockGenerator.new(frequency_hz: 1_000_000)
    [ComputeUnit::MatrixMultiplyUnit.new(ComputeUnit::MXUConfig.new, clk), clk]
  end

  def test_submit_operation_creates_tiles
    mxu, _ = make_mxu
    seq = DeviceSimulator::TPUSequencer.new(
      mxu, mxu_size: 2, scalar_latency: 2, mxu_latency: 5, vector_latency: 3
    )
    kernel = DeviceSimulator::KernelDescriptor.new(
      operation: "matmul",
      input_data: [[1.0, 2.0], [3.0, 4.0]],
      weight_data: [[5.0, 6.0], [7.0, 8.0]]
    )
    seq.submit_operation(kernel)
    assert seq.pending_count >= 1
  end

  def test_step_advances_pipeline
    mxu, _ = make_mxu
    seq = DeviceSimulator::TPUSequencer.new(
      mxu, mxu_size: 2, scalar_latency: 1, mxu_latency: 2, vector_latency: 1
    )
    kernel = DeviceSimulator::KernelDescriptor.new(
      operation: "matmul",
      input_data: [[1.0, 2.0], [3.0, 4.0]],
      weight_data: [[5.0, 6.0], [7.0, 8.0]]
    )
    seq.submit_operation(kernel)
    actions = seq.step
    assert actions.length >= 1
  end

  def test_runs_to_completion
    mxu, _ = make_mxu
    seq = DeviceSimulator::TPUSequencer.new(
      mxu, mxu_size: 2, scalar_latency: 1, mxu_latency: 2, vector_latency: 1
    )
    kernel = DeviceSimulator::KernelDescriptor.new(
      operation: "matmul",
      input_data: [[1.0, 2.0], [3.0, 4.0]],
      weight_data: [[5.0, 6.0], [7.0, 8.0]]
    )
    seq.submit_operation(kernel)
    100.times do
      seq.step
      break if seq.idle?
    end
    assert seq.idle?
  end

  def test_idle_initially
    mxu, _ = make_mxu
    seq = DeviceSimulator::TPUSequencer.new(mxu, mxu_size: 2)
    assert seq.idle?
  end

  def test_reset
    mxu, _ = make_mxu
    seq = DeviceSimulator::TPUSequencer.new(
      mxu, mxu_size: 2, scalar_latency: 1, mxu_latency: 2, vector_latency: 1
    )
    kernel = DeviceSimulator::KernelDescriptor.new(
      operation: "matmul",
      input_data: [[1.0]], weight_data: [[1.0]]
    )
    seq.submit_operation(kernel)
    seq.step
    seq.reset
    assert seq.idle?
    assert_equal 0, seq.pending_count
  end
end

class TestANEScheduleReplayer < Minitest::Test
  include CodingAdventures

  def make_ane_cores(n = 4)
    clk = Clock::ClockGenerator.new(frequency_hz: 1_000_000)
    cores = Array.new(n) { ComputeUnit::NeuralEngineCore.new(ComputeUnit::ANECoreConfig.new, clk) }
    [cores, clk]
  end

  def test_submit_generates_schedule
    cores, _ = make_ane_cores(2)
    replayer = DeviceSimulator::ANEScheduleReplayer.new(
      cores, dma_latency: 1, compute_latency: 2, activate_latency: 1
    )
    kernel = DeviceSimulator::KernelDescriptor.new(
      operation: "conv2d",
      input_data: [[1.0, 2.0], [3.0, 4.0]],
      weight_data: [[0.5, 0.5], [0.5, 0.5]]
    )
    replayer.submit_operation(kernel)
    assert replayer.pending_count > 0
  end

  def test_step_replays_schedule
    cores, _ = make_ane_cores(2)
    replayer = DeviceSimulator::ANEScheduleReplayer.new(
      cores, dma_latency: 1, compute_latency: 2, activate_latency: 1
    )
    kernel = DeviceSimulator::KernelDescriptor.new(
      operation: "conv2d",
      input_data: [[1.0, 2.0]], weight_data: [[0.5, 0.5]]
    )
    replayer.submit_operation(kernel)
    actions = replayer.step
    assert actions.length >= 1
  end

  def test_runs_to_completion
    cores, _ = make_ane_cores(2)
    replayer = DeviceSimulator::ANEScheduleReplayer.new(
      cores, dma_latency: 1, compute_latency: 2, activate_latency: 1
    )
    kernel = DeviceSimulator::KernelDescriptor.new(
      operation: "inference",
      input_data: [[1.0]], weight_data: [[1.0]]
    )
    replayer.submit_operation(kernel)
    100.times do
      replayer.step
      break if replayer.idle?
    end
    assert replayer.idle?
  end

  def test_idle_initially
    cores, _ = make_ane_cores(2)
    replayer = DeviceSimulator::ANEScheduleReplayer.new(cores)
    assert replayer.idle?
  end

  def test_reset
    cores, _ = make_ane_cores(2)
    replayer = DeviceSimulator::ANEScheduleReplayer.new(
      cores, dma_latency: 1, compute_latency: 1, activate_latency: 1
    )
    kernel = DeviceSimulator::KernelDescriptor.new(
      operation: "test",
      input_data: [[1.0]], weight_data: [[1.0]]
    )
    replayer.submit_operation(kernel)
    replayer.step
    replayer.reset
    assert replayer.idle?
    assert_equal 0, replayer.pending_count
  end

  def test_total_dispatched
    cores, _ = make_ane_cores(2)
    replayer = DeviceSimulator::ANEScheduleReplayer.new(
      cores, dma_latency: 1, compute_latency: 1, activate_latency: 1
    )
    kernel = DeviceSimulator::KernelDescriptor.new(
      operation: "test",
      input_data: [[1.0]], weight_data: [[1.0]]
    )
    replayer.submit_operation(kernel)
    100.times do
      replayer.step
      break if replayer.idle?
    end
    assert replayer.total_dispatched > 0
  end
end

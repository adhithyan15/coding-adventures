# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_device_simulator"

# Tests for Google TPU device simulator.
class TestGoogleTPUConstruction < Minitest::Test
  include CodingAdventures

  def test_default_construction
    tpu = DeviceSimulator::GoogleTPU.new(mxu_size: 4)
    assert_includes tpu.name, "TPU"
    assert_equal 1, tpu.compute_units.length
  end

  def test_with_config
    config = DeviceSimulator::TPUConfig.new(
      name: "Test TPU",
      num_compute_units: 1,
      global_memory_size: 1024 * 1024,
      vector_unit_width: 4
    )
    tpu = DeviceSimulator::GoogleTPU.new(config: config)
    assert_equal "Test TPU", tpu.name
  end

  def test_starts_idle
    tpu = DeviceSimulator::GoogleTPU.new(mxu_size: 4)
    assert tpu.idle?
  end
end

class TestGoogleTPUMatmulExecution < Minitest::Test
  include CodingAdventures

  def test_launch_matmul
    tpu = DeviceSimulator::GoogleTPU.new(mxu_size: 2)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "matmul",
      operation: "matmul",
      input_data: [[1.0, 2.0], [3.0, 4.0]],
      weight_data: [[5.0, 6.0], [7.0, 8.0]]
    )
    tpu.launch_kernel(kernel)
    refute tpu.idle?
  end

  def test_run_matmul_to_completion
    tpu = DeviceSimulator::GoogleTPU.new(mxu_size: 2)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "matmul",
      operation: "matmul",
      input_data: [[1.0, 2.0], [3.0, 4.0]],
      weight_data: [[5.0, 6.0], [7.0, 8.0]]
    )
    tpu.launch_kernel(kernel)
    traces = tpu.run(500)
    assert traces.length > 0
    assert tpu.idle?
  end

  def test_large_matmul_tiles
    tpu = DeviceSimulator::GoogleTPU.new(mxu_size: 2)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "big_matmul",
      operation: "matmul",
      input_data: Array.new(4) { [1.0] * 4 },
      weight_data: Array.new(4) { [1.0] * 4 }
    )
    tpu.launch_kernel(kernel)
    traces = tpu.run(1000)
    assert tpu.idle?
  end
end

class TestGoogleTPUMemory < Minitest::Test
  include CodingAdventures

  def test_malloc_and_transfer
    tpu = DeviceSimulator::GoogleTPU.new(mxu_size: 4)
    addr = tpu.malloc(256)
    cycles = tpu.memcpy_host_to_device(addr, "\x00".b * 256)
    assert cycles > 0
  end
end

class TestGoogleTPUTraces < Minitest::Test
  include CodingAdventures

  def test_trace_has_pipeline_actions
    tpu = DeviceSimulator::GoogleTPU.new(mxu_size: 2)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "matmul",
      operation: "matmul",
      input_data: [[1.0, 2.0], [3.0, 4.0]],
      weight_data: [[5.0, 6.0], [7.0, 8.0]]
    )
    tpu.launch_kernel(kernel)
    trace = tpu.step
    refute_empty trace.distributor_actions
  end

  def test_trace_format
    tpu = DeviceSimulator::GoogleTPU.new(mxu_size: 2)
    trace = tpu.step
    formatted = trace.format
    assert_includes formatted, "TPU"
  end
end

class TestGoogleTPUReset < Minitest::Test
  include CodingAdventures

  def test_reset
    tpu = DeviceSimulator::GoogleTPU.new(mxu_size: 2)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "matmul",
      operation: "matmul",
      input_data: [[1.0]], weight_data: [[1.0]]
    )
    tpu.launch_kernel(kernel)
    tpu.run(500)
    tpu.reset
    assert tpu.idle?
  end

  def test_stats
    tpu = DeviceSimulator::GoogleTPU.new(mxu_size: 2)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "matmul",
      operation: "matmul",
      input_data: [[1.0]], weight_data: [[1.0]]
    )
    tpu.launch_kernel(kernel)
    tpu.run(500)
    stats = tpu.stats
    assert_equal 1, stats.total_kernels_launched
  end
end

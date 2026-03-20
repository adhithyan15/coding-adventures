# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_device_simulator"

# Cross-device tests -- same workloads on all architectures.
class TestCrossDeviceAllStartIdle < Minitest::Test
  include CodingAdventures

  def all_gpu_devices
    {
      "NVIDIA" => DeviceSimulator::NvidiaGPU.new(num_sms: 2),
      "AMD" => DeviceSimulator::AmdGPU.new(num_cus: 2),
      "Intel" => DeviceSimulator::IntelGPU.new(num_cores: 2)
    }
  end

  def all_dataflow_devices
    {
      "TPU" => DeviceSimulator::GoogleTPU.new(mxu_size: 2),
      "ANE" => DeviceSimulator::AppleANE.new(num_cores: 2)
    }
  end

  def all_devices
    all_gpu_devices.merge(all_dataflow_devices)
  end

  def test_all_devices_start_idle
    all_devices.each do |name, device|
      assert device.idle?, "#{name} should start idle"
    end
  end

  def test_all_have_non_empty_names
    all_devices.each do |name, device|
      refute_empty device.name, "#{name} should have a name"
    end
  end

  def test_all_have_compute_units
    all_devices.each do |name, device|
      assert device.compute_units.length > 0, "#{name} should have compute units"
    end
  end

  def test_all_can_step_when_idle
    all_devices.each do |name, device|
      trace = device.step
      assert trace.cycle > 0, "#{name} step should produce a trace"
    end
  end

  def test_all_reset_to_idle
    all_devices.each do |name, device|
      device.step
      device.step
      device.reset
      assert device.idle?, "#{name} should be idle after reset"
    end
  end
end

class TestCrossDeviceGPUKernelExecution < Minitest::Test
  include CodingAdventures

  def all_gpu_devices
    {
      "NVIDIA" => DeviceSimulator::NvidiaGPU.new(num_sms: 2),
      "AMD" => DeviceSimulator::AmdGPU.new(num_cus: 2),
      "Intel" => DeviceSimulator::IntelGPU.new(num_cores: 2)
    }
  end

  def test_all_gpus_run_simple_kernel
    all_gpu_devices.each do |name, device|
      kernel = DeviceSimulator::KernelDescriptor.new(
        name: "test_simple",
        program: [GpuCore.limm(0, 42.0), GpuCore.halt],
        grid_dim: [2, 1, 1], block_dim: [32, 1, 1]
      )
      device.launch_kernel(kernel)
      traces = device.run(2000)
      assert traces.length > 0, "#{name}: should produce traces"
      assert device.idle?, "#{name}: should be idle after completion"
    end
  end
end

class TestCrossDeviceDataflowExecution < Minitest::Test
  include CodingAdventures

  def all_dataflow_devices
    {
      "TPU" => DeviceSimulator::GoogleTPU.new(mxu_size: 2),
      "ANE" => DeviceSimulator::AppleANE.new(num_cores: 2)
    }
  end

  def test_all_dataflow_run_matmul
    all_dataflow_devices.each do |name, device|
      kernel = DeviceSimulator::KernelDescriptor.new(
        name: "matmul",
        operation: "matmul",
        input_data: [[1.0, 2.0], [3.0, 4.0]],
        weight_data: [[5.0, 6.0], [7.0, 8.0]]
      )
      device.launch_kernel(kernel)
      traces = device.run(1000)
      assert traces.length > 0, "#{name}: should produce traces"
      assert device.idle?, "#{name}: should be idle after matmul"
    end
  end
end

class TestCrossDeviceMemoryOps < Minitest::Test
  include CodingAdventures

  def all_devices
    {
      "NVIDIA" => DeviceSimulator::NvidiaGPU.new(num_sms: 2),
      "AMD" => DeviceSimulator::AmdGPU.new(num_cus: 2),
      "Intel" => DeviceSimulator::IntelGPU.new(num_cores: 2),
      "TPU" => DeviceSimulator::GoogleTPU.new(mxu_size: 2),
      "ANE" => DeviceSimulator::AppleANE.new(num_cores: 2)
    }
  end

  def test_all_can_malloc_and_free
    all_devices.each do |name, device|
      addr = device.malloc(256)
      assert addr >= 0, "#{name}: malloc should return valid address"
      device.free(addr)
    end
  end

  def test_all_can_transfer_data
    all_devices.each do |name, device|
      addr = device.malloc(64)
      device.memcpy_host_to_device(addr, "\x42".b * 64)
      data, _ = device.memcpy_device_to_host(addr, 64)
      assert_equal "\x42".b * 64, data, "#{name}: data should round-trip"
    end
  end

  def test_unified_vs_discrete_transfer_cost
    ane = DeviceSimulator::AppleANE.new(num_cores: 2)
    nvidia = DeviceSimulator::NvidiaGPU.new(num_sms: 2)

    ane_addr = ane.malloc(256)
    nvidia_addr = nvidia.malloc(256)

    ane_cycles = ane.memcpy_host_to_device(ane_addr, "\x00".b * 256)
    nvidia_cycles = nvidia.memcpy_host_to_device(nvidia_addr, "\x00".b * 256)

    assert_equal 0, ane_cycles, "ANE unified memory should be zero-cost"
    assert nvidia_cycles > 0, "NVIDIA discrete should have transfer cost"
  end
end

class TestCrossDeviceStats < Minitest::Test
  include CodingAdventures

  def test_all_track_kernels
    devices = {
      "NVIDIA" => DeviceSimulator::NvidiaGPU.new(num_sms: 2),
      "AMD" => DeviceSimulator::AmdGPU.new(num_cus: 2),
      "Intel" => DeviceSimulator::IntelGPU.new(num_cores: 2),
      "TPU" => DeviceSimulator::GoogleTPU.new(mxu_size: 2),
      "ANE" => DeviceSimulator::AppleANE.new(num_cores: 2)
    }

    devices.each do |name, device|
      kernel = if %w[NVIDIA AMD Intel].include?(name)
        DeviceSimulator::KernelDescriptor.new(
          name: "test",
          program: [GpuCore.limm(0, 1.0), GpuCore.halt],
          grid_dim: [1, 1, 1], block_dim: [32, 1, 1]
        )
      else
        DeviceSimulator::KernelDescriptor.new(
          name: "test",
          operation: "matmul",
          input_data: [[1.0]], weight_data: [[1.0]]
        )
      end
      device.launch_kernel(kernel)
      device.run(1000)
      stats = device.stats
      assert_equal 1, stats.total_kernels_launched,
        "#{name}: should track kernel launches"
    end
  end
end

class TestCrossDeviceTraceFormat < Minitest::Test
  include CodingAdventures

  def test_all_produce_readable_traces
    devices = {
      "NVIDIA" => DeviceSimulator::NvidiaGPU.new(num_sms: 2),
      "AMD" => DeviceSimulator::AmdGPU.new(num_cus: 2),
      "Intel" => DeviceSimulator::IntelGPU.new(num_cores: 2),
      "TPU" => DeviceSimulator::GoogleTPU.new(mxu_size: 2),
      "ANE" => DeviceSimulator::AppleANE.new(num_cores: 2)
    }

    devices.each do |name, device|
      trace = device.step
      formatted = trace.format
      assert_kind_of String, formatted, "#{name}: format should return String"
      refute_empty formatted, "#{name}: format should be non-empty"
    end
  end
end

# frozen_string_literal: true

require_relative "test_helper"

# Tests for RuntimeInstance and device discovery.
class TestRuntimeInstance < Minitest::Test
  include CodingAdventures

  def test_default_construction
    instance = ComputeRuntime::RuntimeInstance.new
    assert_equal "0.1.0", instance.version
  end

  def test_enumerate_default_devices
    instance = ComputeRuntime::RuntimeInstance.new
    devices = instance.enumerate_physical_devices
    assert_equal 5, devices.length # NVIDIA, AMD, Google, Intel, Apple
  end

  def test_device_names
    instance = ComputeRuntime::RuntimeInstance.new
    devices = instance.enumerate_physical_devices
    names = devices.map(&:name)
    assert names.any? { |n| n.include?("NVIDIA") }
    assert names.any? { |n| n.include?("AMD") }
    assert names.any? { |n| n.include?("TPU") || n.include?("Google") }
    assert names.any? { |n| n.include?("Intel") }
    assert names.any? { |n| n.include?("Apple") || n.include?("ANE") }
  end

  def test_device_types
    instance = ComputeRuntime::RuntimeInstance.new
    devices = instance.enumerate_physical_devices
    types = devices.map(&:device_type).uniq
    assert_includes types, :gpu
    assert_includes types, :tpu
    assert_includes types, :npu
  end

  def test_device_vendors
    instance = ComputeRuntime::RuntimeInstance.new
    devices = instance.enumerate_physical_devices
    vendors = devices.map(&:vendor).uniq
    assert_includes vendors, "nvidia"
    assert_includes vendors, "amd"
    assert_includes vendors, "google"
    assert_includes vendors, "intel"
    assert_includes vendors, "apple"
  end

  def test_custom_devices
    nvidia = DeviceSimulator::NvidiaGPU.new(num_sms: 4)
    instance = ComputeRuntime::RuntimeInstance.new(
      devices: [[nvidia, :gpu, "nvidia"]]
    )
    devices = instance.enumerate_physical_devices
    assert_equal 1, devices.length
    assert_equal "nvidia", devices[0].vendor
  end

  def test_device_ids_are_unique
    instance = ComputeRuntime::RuntimeInstance.new
    devices = instance.enumerate_physical_devices
    ids = devices.map(&:device_id)
    assert_equal ids.length, ids.uniq.length
  end
end

class TestPhysicalDevice < Minitest::Test
  include CodingAdventures

  def test_memory_properties_discrete
    instance = ComputeRuntime::RuntimeInstance.new
    nvidia = instance.enumerate_physical_devices.find { |d| d.vendor == "nvidia" }
    mem = nvidia.memory_properties
    refute mem.is_unified
    assert mem.heaps.length >= 2 # VRAM + staging
  end

  def test_memory_properties_unified
    instance = ComputeRuntime::RuntimeInstance.new
    apple = instance.enumerate_physical_devices.find { |d| d.vendor == "apple" }
    mem = apple.memory_properties
    assert mem.is_unified
    assert mem.heaps.length >= 1
  end

  def test_queue_families
    instance = ComputeRuntime::RuntimeInstance.new
    nvidia = instance.enumerate_physical_devices.find { |d| d.vendor == "nvidia" }
    families = nvidia.queue_families
    assert families.length >= 1
    assert families.any? { |f| f.queue_type == :compute }
  end

  def test_discrete_has_transfer_queue
    instance = ComputeRuntime::RuntimeInstance.new
    nvidia = instance.enumerate_physical_devices.find { |d| d.vendor == "nvidia" }
    assert nvidia.queue_families.any? { |f| f.queue_type == :transfer }
  end

  def test_unified_no_separate_transfer
    instance = ComputeRuntime::RuntimeInstance.new
    apple = instance.enumerate_physical_devices.find { |d| d.vendor == "apple" }
    refute apple.queue_families.any? { |f| f.queue_type == :transfer }
  end

  def test_supports_feature
    instance = ComputeRuntime::RuntimeInstance.new
    nvidia = instance.enumerate_physical_devices.find { |d| d.vendor == "nvidia" }
    assert nvidia.supports_feature("fp32")
    refute nvidia.supports_feature("unified_memory")
  end

  def test_apple_supports_unified
    instance = ComputeRuntime::RuntimeInstance.new
    apple = instance.enumerate_physical_devices.find { |d| d.vendor == "apple" }
    assert apple.supports_feature("unified_memory")
  end

  def test_limits
    instance = ComputeRuntime::RuntimeInstance.new
    nvidia = instance.enumerate_physical_devices.find { |d| d.vendor == "nvidia" }
    limits = nvidia.limits
    assert limits.max_workgroup_size[0] > 0
    assert limits.max_buffer_size > 0
    assert limits.max_push_constant_size > 0
  end
end

class TestLogicalDevice < Minitest::Test
  include CodingAdventures

  def make_device
    instance = ComputeRuntime::RuntimeInstance.new
    physical = instance.enumerate_physical_devices[0]
    [instance, physical]
  end

  def test_create_logical_device
    instance, physical = make_device
    device = instance.create_logical_device(physical)
    assert_equal physical, device.physical_device
    assert device.queues.key?("compute")
  end

  def test_default_queue
    instance, physical = make_device
    device = instance.create_logical_device(physical)
    assert_equal 1, device.queues["compute"].length
  end

  def test_multiple_queues
    instance, physical = make_device
    device = instance.create_logical_device(physical,
      queue_requests: [{"type" => "compute", "count" => 3}])
    assert_equal 3, device.queues["compute"].length
  end

  def test_memory_manager
    instance, physical = make_device
    device = instance.create_logical_device(physical)
    refute_nil device.memory_manager
  end

  def test_factory_methods
    instance, physical = make_device
    device = instance.create_logical_device(physical)

    cb = device.create_command_buffer
    refute_nil cb

    fence = device.create_fence
    refute fence.signaled

    sem = device.create_semaphore
    refute sem.signaled

    event = device.create_event
    refute event.signaled
  end

  def test_create_fence_signaled
    instance, physical = make_device
    device = instance.create_logical_device(physical)
    fence = device.create_fence(signaled: true)
    assert fence.signaled
  end

  def test_wait_idle
    instance, physical = make_device
    device = instance.create_logical_device(physical)
    device.wait_idle # Should not raise
  end

  def test_reset
    instance, physical = make_device
    device = instance.create_logical_device(physical)
    device.reset # Should not raise
  end

  def test_all_device_types
    instance = ComputeRuntime::RuntimeInstance.new
    instance.enumerate_physical_devices.each do |physical|
      device = instance.create_logical_device(physical)
      assert_equal physical.name, device.physical_device.name
      assert device.queues.key?("compute")
    end
  end
end

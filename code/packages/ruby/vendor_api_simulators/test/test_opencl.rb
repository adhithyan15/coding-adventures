# frozen_string_literal: true

require_relative "test_helper"

# ---------------------------------------------------------------------------
# OpenCL Runtime Simulator Tests
# ---------------------------------------------------------------------------
class TestOpenCL < Minitest::Test
  include CodingAdventures::VendorApiSimulators

  def setup
    @ctx = CLContext.new
  end

  # =================================================================
  # Platform discovery
  # =================================================================

  def test_get_platforms
    platforms = CLPlatform.get_platforms
    assert_kind_of Array, platforms
    assert_equal 1, platforms.length
    assert_instance_of CLPlatform, platforms[0]
  end

  def test_platform_properties
    platform = CLPlatform.get_platforms[0]
    refute_empty platform.name
    refute_empty platform.vendor
    refute_empty platform.version
  end

  def test_platform_get_devices_all
    platform = CLPlatform.get_platforms[0]
    devices = platform.get_devices(CLDeviceType::ALL)
    assert_kind_of Array, devices
    refute_empty devices
  end

  def test_platform_get_devices_gpu
    platform = CLPlatform.get_platforms[0]
    devices = platform.get_devices(CLDeviceType::GPU)
    assert_kind_of Array, devices
  end

  # =================================================================
  # Context creation
  # =================================================================

  def test_context_creates_successfully
    assert_instance_of CLContext, @ctx
  end

  def test_context_has_devices
    refute_empty @ctx._devices
  end

  def test_context_with_specific_device
    platform = CLPlatform.get_platforms[0]
    devices = platform.get_devices
    ctx = CLContext.new(devices: [devices[0]])
    assert_instance_of CLContext, ctx
  end

  # =================================================================
  # Device info
  # =================================================================

  def test_device_name
    device = @ctx._devices[0]
    refute_empty device.name
  end

  def test_device_type
    device = @ctx._devices[0]
    dt = device.device_type
    assert_includes [CLDeviceType::GPU, CLDeviceType::CPU, CLDeviceType::ACCELERATOR], dt
  end

  def test_device_max_compute_units
    device = @ctx._devices[0]
    assert_equal 4, device.max_compute_units
  end

  def test_device_max_work_group_size
    device = @ctx._devices[0]
    assert_kind_of Integer, device.max_work_group_size
    assert device.max_work_group_size > 0
  end

  def test_device_global_mem_size
    device = @ctx._devices[0]
    assert device.global_mem_size > 0
  end

  def test_device_get_info
    device = @ctx._devices[0]
    assert_equal device.name, device.get_info(CLDeviceInfo::NAME)
    assert_equal device.device_type, device.get_info(CLDeviceInfo::TYPE)
    assert_equal device.max_compute_units, device.get_info(CLDeviceInfo::MAX_COMPUTE_UNITS)
    assert_equal device.max_work_group_size, device.get_info(CLDeviceInfo::MAX_WORK_GROUP_SIZE)
    assert_equal device.global_mem_size, device.get_info(CLDeviceInfo::GLOBAL_MEM_SIZE)
  end

  # =================================================================
  # Buffer creation
  # =================================================================

  def test_create_buffer_read_write
    buf = @ctx.create_buffer(CLMemFlags::READ_WRITE, 256)
    assert_instance_of CLBuffer, buf
    assert_equal 256, buf.size
    assert_equal CLMemFlags::READ_WRITE, buf.flags
  end

  def test_create_buffer_read_only
    buf = @ctx.create_buffer(CLMemFlags::READ_ONLY, 128)
    assert_equal 128, buf.size
    assert_equal CLMemFlags::READ_ONLY, buf.flags
  end

  def test_create_buffer_with_copy_host_ptr
    data = "\x01\x02\x03\x04".b
    buf = @ctx.create_buffer(
      CLMemFlags::READ_WRITE | CLMemFlags::COPY_HOST_PTR,
      4,
      host_ptr: data
    )
    assert_equal 4, buf.size
  end

  # =================================================================
  # Program and kernel
  # =================================================================

  def test_create_program
    prog = @ctx.create_program_with_source("test_source")
    assert_instance_of CLProgram, prog
    assert_equal CLBuildStatus::NONE, prog.build_status
  end

  def test_build_program
    prog = @ctx.create_program_with_source("test")
    prog.build
    assert_equal CLBuildStatus::SUCCESS, prog.build_status
  end

  def test_create_kernel_after_build
    prog = @ctx.create_program_with_source("test")
    prog.build
    kernel = prog.create_kernel("my_kernel")
    assert_instance_of CLKernel, kernel
    assert_equal "my_kernel", kernel.name
  end

  def test_create_kernel_before_build_raises
    prog = @ctx.create_program_with_source("test")
    assert_raises(RuntimeError) { prog.create_kernel("my_kernel") }
  end

  def test_kernel_set_arg
    prog = @ctx.create_program_with_source("test")
    prog.build
    kernel = prog.create_kernel("k")
    buf = @ctx.create_buffer(CLMemFlags::READ_WRITE, 64)
    kernel.set_arg(0, buf)
    assert_equal buf, kernel._args[0]
  end

  def test_kernel_set_multiple_args
    prog = @ctx.create_program_with_source("test")
    prog.build
    kernel = prog.create_kernel("k")
    buf1 = @ctx.create_buffer(CLMemFlags::READ_WRITE, 64)
    buf2 = @ctx.create_buffer(CLMemFlags::READ_WRITE, 64)
    kernel.set_arg(0, buf1)
    kernel.set_arg(1, buf2)
    assert_equal buf1, kernel._args[0]
    assert_equal buf2, kernel._args[1]
  end

  # =================================================================
  # Command queue operations
  # =================================================================

  def test_create_command_queue
    queue = @ctx.create_command_queue
    assert_instance_of CLCommandQueue, queue
  end

  def test_create_command_queue_with_device
    device = @ctx._devices[0]
    queue = @ctx.create_command_queue(device: device)
    assert_instance_of CLCommandQueue, queue
  end

  def test_enqueue_write_buffer
    queue = @ctx.create_command_queue
    buf = @ctx.create_buffer(CLMemFlags::READ_WRITE, 4)
    data = "\xAA\xBB\xCC\xDD".b
    event = queue.enqueue_write_buffer(buf, 0, 4, data)
    assert_instance_of CLEvent, event
  end

  def test_enqueue_read_buffer
    queue = @ctx.create_command_queue
    buf = @ctx.create_buffer(CLMemFlags::READ_WRITE, 4)
    data = "\x01\x02\x03\x04".b
    queue.enqueue_write_buffer(buf, 0, 4, data)

    output = (+"\x00\x00\x00\x00").force_encoding(Encoding::BINARY)
    event = queue.enqueue_read_buffer(buf, 0, 4, output)
    assert_instance_of CLEvent, event
    assert_equal data, output
  end

  def test_enqueue_nd_range_kernel
    queue = @ctx.create_command_queue
    prog = @ctx.create_program_with_source("test")
    prog.build
    kernel = prog.create_kernel("compute")
    buf = @ctx.create_buffer(CLMemFlags::READ_WRITE, 256)
    kernel.set_arg(0, buf)

    event = queue.enqueue_nd_range_kernel(kernel, [128])
    assert_instance_of CLEvent, event
    assert_equal CLEventStatus::COMPLETE, event.status
  end

  def test_enqueue_nd_range_kernel_with_local_size
    queue = @ctx.create_command_queue
    prog = @ctx.create_program_with_source("test")
    prog.build
    kernel = prog.create_kernel("compute")
    buf = @ctx.create_buffer(CLMemFlags::READ_WRITE, 256)
    kernel.set_arg(0, buf)

    event = queue.enqueue_nd_range_kernel(kernel, [128], local_size: [64])
    assert_instance_of CLEvent, event
  end

  def test_enqueue_nd_range_kernel_2d
    queue = @ctx.create_command_queue
    prog = @ctx.create_program_with_source("test")
    prog.build
    kernel = prog.create_kernel("compute")

    event = queue.enqueue_nd_range_kernel(kernel, [64, 64], local_size: [8, 8])
    assert_instance_of CLEvent, event
  end

  def test_enqueue_nd_range_kernel_3d
    queue = @ctx.create_command_queue
    prog = @ctx.create_program_with_source("test")
    prog.build
    kernel = prog.create_kernel("compute")

    event = queue.enqueue_nd_range_kernel(kernel, [32, 32, 4], local_size: [8, 8, 2])
    assert_instance_of CLEvent, event
  end

  def test_enqueue_copy_buffer
    queue = @ctx.create_command_queue
    src = @ctx.create_buffer(CLMemFlags::READ_WRITE, 4)
    dst = @ctx.create_buffer(CLMemFlags::READ_WRITE, 4)
    data = "\x11\x22\x33\x44".b
    queue.enqueue_write_buffer(src, 0, 4, data)

    event = queue.enqueue_copy_buffer(src, dst, 4)
    assert_instance_of CLEvent, event
  end

  def test_enqueue_fill_buffer
    queue = @ctx.create_command_queue
    buf = @ctx.create_buffer(CLMemFlags::READ_WRITE, 16)
    event = queue.enqueue_fill_buffer(buf, "\xFF".b, 0, 16)
    assert_instance_of CLEvent, event
  end

  def test_enqueue_fill_buffer_empty_pattern
    queue = @ctx.create_command_queue
    buf = @ctx.create_buffer(CLMemFlags::READ_WRITE, 16)
    event = queue.enqueue_fill_buffer(buf, "".b, 0, 16)
    assert_instance_of CLEvent, event
  end

  # =================================================================
  # Event model
  # =================================================================

  def test_event_wait
    queue = @ctx.create_command_queue
    buf = @ctx.create_buffer(CLMemFlags::READ_WRITE, 4)
    event = queue.enqueue_write_buffer(buf, 0, 4, "\x00\x00\x00\x00".b)
    event.wait
    assert_equal CLEventStatus::COMPLETE, event.status
  end

  def test_event_dependency_chain
    queue = @ctx.create_command_queue
    buf = @ctx.create_buffer(CLMemFlags::READ_WRITE, 4)
    ev1 = queue.enqueue_write_buffer(buf, 0, 4, "\x01\x02\x03\x04".b)

    prog = @ctx.create_program_with_source("test")
    prog.build
    kernel = prog.create_kernel("compute")
    kernel.set_arg(0, buf)
    ev2 = queue.enqueue_nd_range_kernel(kernel, [32], wait_list: [ev1])
    assert_instance_of CLEvent, ev2
  end

  def test_queue_finish
    queue = @ctx.create_command_queue
    queue.finish
  end

  def test_queue_flush
    queue = @ctx.create_command_queue
    queue.flush
  end

  # =================================================================
  # Event and wait list
  # =================================================================

  def test_enqueue_with_wait_list
    queue = @ctx.create_command_queue
    buf1 = @ctx.create_buffer(CLMemFlags::READ_WRITE, 4)
    buf2 = @ctx.create_buffer(CLMemFlags::READ_WRITE, 4)
    ev1 = queue.enqueue_write_buffer(buf1, 0, 4, "\x01\x02\x03\x04".b)
    ev2 = queue.enqueue_write_buffer(buf2, 0, 4, "\x05\x06\x07\x08".b)

    output = (+"\x00\x00\x00\x00").force_encoding(Encoding::BINARY)
    ev3 = queue.enqueue_read_buffer(buf1, 0, 4, output, wait_list: [ev1, ev2])
    assert_equal CLEventStatus::COMPLETE, ev3.status
  end

  def test_enqueue_copy_buffer_with_wait_list
    queue = @ctx.create_command_queue
    src = @ctx.create_buffer(CLMemFlags::READ_WRITE, 4)
    dst = @ctx.create_buffer(CLMemFlags::READ_WRITE, 4)
    ev1 = queue.enqueue_write_buffer(src, 0, 4, "\x01\x02\x03\x04".b)
    ev2 = queue.enqueue_copy_buffer(src, dst, 4, wait_list: [ev1])
    assert_equal CLEventStatus::COMPLETE, ev2.status
  end
end

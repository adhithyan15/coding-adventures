# frozen_string_literal: true

require_relative "test_helper"

# ---------------------------------------------------------------------------
# CUDA Runtime Simulator Tests
# ---------------------------------------------------------------------------
#
# These tests verify that the CUDA API simulator correctly wraps the Layer 5
# compute runtime with CUDA semantics: implicit command buffers, automatic
# synchronization, and the malloc/memcpy/launch/free workflow.
class TestCUDA < Minitest::Test
  include CodingAdventures::VendorApiSimulators

  def setup
    @cuda = CUDARuntime.new
  end

  # =================================================================
  # Construction and device selection
  # =================================================================

  def test_cuda_runtime_creates_successfully
    assert_instance_of CUDARuntime, @cuda
  end

  def test_cuda_selects_nvidia_device
    # The runtime should prefer an NVIDIA device
    assert_equal "nvidia", @cuda._physical_device.vendor
  end

  def test_cuda_has_compute_queue
    refute_nil @cuda._compute_queue
  end

  def test_cuda_has_memory_manager
    refute_nil @cuda._memory_manager
  end

  # =================================================================
  # Device management
  # =================================================================

  def test_get_device_returns_zero_initially
    assert_equal 0, @cuda.get_device
  end

  def test_set_device_valid_id
    @cuda.set_device(0)
    assert_equal 0, @cuda.get_device
  end

  def test_set_device_invalid_negative_raises
    assert_raises(ArgumentError) { @cuda.set_device(-1) }
  end

  def test_set_device_invalid_too_large_raises
    assert_raises(ArgumentError) { @cuda.set_device(999) }
  end

  def test_get_device_properties
    props = @cuda.get_device_properties
    assert_instance_of CUDADeviceProperties, props
    refute_empty props.name
    assert_kind_of Integer, props.total_global_mem
    assert props.total_global_mem > 0
    assert_kind_of Integer, props.max_threads_per_block
  end

  def test_device_properties_defaults
    props = CUDADeviceProperties.new
    assert_equal "", props.name
    assert_equal 0, props.total_global_mem
    assert_equal 49152, props.shared_mem_per_block
    assert_equal 1024, props.max_threads_per_block
    assert_equal 32, props.warp_size
    assert_equal [8, 0], props.compute_capability
  end

  def test_device_synchronize
    # Should not raise -- just waits for idle
    @cuda.device_synchronize
  end

  def test_device_reset
    ptr = @cuda.malloc(64)
    @cuda.device_reset
    # After reset, streams and events should be cleared
  end

  # =================================================================
  # Memory management
  # =================================================================

  def test_malloc_returns_device_ptr
    ptr = @cuda.malloc(1024)
    assert_instance_of CUDADevicePtr, ptr
    assert_equal 1024, ptr.size
    assert_kind_of Integer, ptr.device_address
    @cuda.free(ptr)
  end

  def test_malloc_managed_returns_device_ptr
    ptr = @cuda.malloc_managed(512)
    assert_instance_of CUDADevicePtr, ptr
    assert_equal 512, ptr.size
    @cuda.free(ptr)
  end

  def test_free_releases_memory
    ptr = @cuda.malloc(256)
    @cuda.free(ptr)
    assert ptr._buffer.freed
  end

  def test_memcpy_host_to_device
    ptr = @cuda.malloc(4)
    data = "\x01\x02\x03\x04".b
    @cuda.memcpy(ptr, data, 4, :host_to_device)
    # Verify by reading back
    output = (+"\x00\x00\x00\x00").force_encoding(Encoding::BINARY)
    @cuda.memcpy(output, ptr, 4, :device_to_host)
    assert_equal data, output
    @cuda.free(ptr)
  end

  def test_memcpy_device_to_host
    ptr = @cuda.malloc(4)
    data = "\xAA\xBB\xCC\xDD".b
    @cuda.memcpy(ptr, data, 4, :host_to_device)

    result = (+"\x00\x00\x00\x00").force_encoding(Encoding::BINARY)
    @cuda.memcpy(result, ptr, 4, :device_to_host)
    assert_equal data, result
    @cuda.free(ptr)
  end

  def test_memcpy_device_to_device
    src = @cuda.malloc(4)
    dst = @cuda.malloc(4)
    data = "\x11\x22\x33\x44".b
    @cuda.memcpy(src, data, 4, :host_to_device)
    @cuda.memcpy(dst, src, 4, :device_to_device)

    result = (+"\x00\x00\x00\x00").force_encoding(Encoding::BINARY)
    @cuda.memcpy(result, dst, 4, :device_to_host)
    assert_equal data, result
    @cuda.free(src)
    @cuda.free(dst)
  end

  def test_memcpy_host_to_host
    src = "hello".b
    dst = (+"\x00" * 5).force_encoding(Encoding::BINARY)
    @cuda.memcpy(dst, src, 5, :host_to_host)
    assert_equal src, dst
  end

  def test_memcpy_host_to_device_wrong_dst_type_raises
    assert_raises(TypeError) { @cuda.memcpy("not_a_ptr", "\x00".b, 1, :host_to_device) }
  end

  def test_memcpy_host_to_device_wrong_src_type_raises
    ptr = @cuda.malloc(4)
    assert_raises(TypeError) { @cuda.memcpy(ptr, ptr, 4, :host_to_device) }
    @cuda.free(ptr)
  end

  def test_memcpy_device_to_host_wrong_src_type_raises
    dst = (+"\x00").force_encoding(Encoding::BINARY)
    assert_raises(TypeError) { @cuda.memcpy(dst, "not_a_ptr", 1, :device_to_host) }
  end

  def test_memcpy_device_to_host_wrong_dst_type_raises
    ptr = @cuda.malloc(4)
    assert_raises(TypeError) { @cuda.memcpy(ptr, ptr, 4, :device_to_host) }
    @cuda.free(ptr)
  end

  def test_memcpy_device_to_device_wrong_types_raises
    ptr = @cuda.malloc(4)
    assert_raises(TypeError) { @cuda.memcpy("x", ptr, 4, :device_to_device) }
    assert_raises(TypeError) { @cuda.memcpy(ptr, "x", 4, :device_to_device) }
    @cuda.free(ptr)
  end

  def test_memcpy_host_to_host_wrong_types_raises
    assert_raises(TypeError) { @cuda.memcpy(CUDADevicePtr.new(buffer: nil), "x", 1, :host_to_host) }
  end

  def test_memset
    ptr = @cuda.malloc(16)
    @cuda.memset(ptr, 0xFF, 16)
    result = (+"\x00" * 16).force_encoding(Encoding::BINARY)
    @cuda.memcpy(result, ptr, 16, :device_to_host)
    # After memset, all bytes should be 0xFF
    assert_equal "\xFF".b * 16, result
    @cuda.free(ptr)
  end

  # =================================================================
  # Kernel launch
  # =================================================================

  def test_launch_kernel_no_args
    kernel = CUDAKernel.new(code: nil, name: "test_kernel")
    grid = Dim3.new(x: 1)
    block = Dim3.new(x: 32)
    # Should not raise
    @cuda.launch_kernel(kernel, grid: grid, block: block)
  end

  def test_launch_kernel_with_args
    ptr = @cuda.malloc(256)
    kernel = CUDAKernel.new(code: nil, name: "test_with_args")
    grid = Dim3.new(x: 4)
    block = Dim3.new(x: 64)
    @cuda.launch_kernel(kernel, grid: grid, block: block, args: [ptr])
    @cuda.free(ptr)
  end

  def test_launch_kernel_with_stream
    stream = @cuda.create_stream
    kernel = CUDAKernel.new(code: nil)
    @cuda.launch_kernel(kernel, grid: Dim3.new(x: 1), block: Dim3.new(x: 32), stream: stream)
    @cuda.destroy_stream(stream)
  end

  def test_dim3_defaults
    d = Dim3.new(x: 4)
    assert_equal 4, d.x
    assert_equal 1, d.y
    assert_equal 1, d.z
  end

  def test_dim3_all_values
    d = Dim3.new(x: 4, y: 2, z: 3)
    assert_equal 4, d.x
    assert_equal 2, d.y
    assert_equal 3, d.z
  end

  def test_cuda_kernel_defaults
    k = CUDAKernel.new(code: nil)
    assert_equal "unnamed_kernel", k.name
    assert_nil k.code
  end

  def test_cuda_kernel_custom_name
    k = CUDAKernel.new(code: [1, 2, 3], name: "my_kernel")
    assert_equal "my_kernel", k.name
    assert_equal [1, 2, 3], k.code
  end

  # =================================================================
  # Streams
  # =================================================================

  def test_create_stream
    stream = @cuda.create_stream
    assert_instance_of CUDAStream, stream
    @cuda.destroy_stream(stream)
  end

  def test_destroy_stream
    stream = @cuda.create_stream
    @cuda.destroy_stream(stream)
    assert_raises(ArgumentError) { @cuda.destroy_stream(stream) }
  end

  def test_stream_synchronize
    stream = @cuda.create_stream
    @cuda.stream_synchronize(stream)
    @cuda.destroy_stream(stream)
  end

  def test_stream_synchronize_with_pending_fence
    stream = @cuda.create_stream
    fence = @cuda._logical_device.create_fence(signaled: true)
    stream._pending_fence = fence
    @cuda.stream_synchronize(stream)
    @cuda.destroy_stream(stream)
  end

  # =================================================================
  # Events
  # =================================================================

  def test_create_event
    event = @cuda.create_event
    assert_instance_of CUDAEvent, event
    assert_equal false, event._recorded
  end

  def test_record_event
    event = @cuda.create_event
    @cuda.record_event(event)
    assert_equal true, event._recorded
  end

  def test_record_event_on_stream
    stream = @cuda.create_stream
    event = @cuda.create_event
    @cuda.record_event(event, stream: stream)
    assert_equal true, event._recorded
    @cuda.destroy_stream(stream)
  end

  def test_synchronize_event
    event = @cuda.create_event
    @cuda.record_event(event)
    @cuda.synchronize_event(event)
  end

  def test_synchronize_unrecorded_event_raises
    event = @cuda.create_event
    assert_raises(RuntimeError) { @cuda.synchronize_event(event) }
  end

  def test_elapsed_time
    e1 = @cuda.create_event
    e2 = @cuda.create_event
    @cuda.record_event(e1)
    @cuda.record_event(e2)
    time = @cuda.elapsed_time(e1, e2)
    assert_kind_of Float, time
  end

  def test_elapsed_time_unrecorded_start_raises
    e1 = @cuda.create_event
    e2 = @cuda.create_event
    @cuda.record_event(e2)
    assert_raises(RuntimeError) { @cuda.elapsed_time(e1, e2) }
  end

  def test_elapsed_time_unrecorded_end_raises
    e1 = @cuda.create_event
    e2 = @cuda.create_event
    @cuda.record_event(e1)
    assert_raises(RuntimeError) { @cuda.elapsed_time(e1, e2) }
  end
end

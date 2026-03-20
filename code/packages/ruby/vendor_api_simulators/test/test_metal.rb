# frozen_string_literal: true

require_relative "test_helper"

# ---------------------------------------------------------------------------
# Metal Runtime Simulator Tests
# ---------------------------------------------------------------------------
class TestMetal < Minitest::Test
  include CodingAdventures::VendorApiSimulators

  def setup
    @device = MTLDevice.new
  end

  # =================================================================
  # Device creation
  # =================================================================

  def test_device_creates_successfully
    assert_instance_of MTLDevice, @device
  end

  def test_device_prefers_apple
    # The device should prefer Apple hardware
    assert_equal "apple", @device._physical_device.vendor
  end

  def test_device_name
    refute_empty @device.name
  end

  # =================================================================
  # Buffer management (unified memory)
  # =================================================================

  def test_make_buffer
    buf = @device.make_buffer(1024)
    assert_instance_of MTLBuffer, buf
    assert_equal 1024, buf.length
  end

  def test_make_buffer_with_options
    buf = @device.make_buffer(512, options: MTLResourceOptions::STORAGE_MODE_PRIVATE)
    assert_equal 512, buf.length
  end

  def test_buffer_write_and_read
    buf = @device.make_buffer(4)
    data = "\x01\x02\x03\x04".b
    buf.write_bytes(data)
    result = buf.contents
    assert_equal data, result.byteslice(0, 4)
  end

  def test_buffer_write_with_offset
    buf = @device.make_buffer(8)
    data = "\xAA\xBB".b
    buf.write_bytes(data, offset: 4)
    result = buf.contents
    assert_equal "\xAA".b, result.byteslice(4, 1)
    assert_equal "\xBB".b, result.byteslice(5, 1)
  end

  def test_buffer_contents_returns_string
    buf = @device.make_buffer(16)
    result = buf.contents
    assert_kind_of String, result
    assert_equal 16, result.bytesize
  end

  # =================================================================
  # Library and function
  # =================================================================

  def test_make_library
    library = @device.make_library(source: "my_shader")
    assert_instance_of MTLLibrary, library
  end

  def test_library_make_function
    library = @device.make_library(source: "my_shader")
    func = library.make_function("compute_fn")
    assert_instance_of MTLFunction, func
    assert_equal "compute_fn", func.name
  end

  def test_function_with_code
    library = MTLLibrary.new("src", functions: {"kern" => [1, 2, 3]})
    func = library.make_function("kern")
    assert_equal [1, 2, 3], func._code
  end

  # =================================================================
  # Pipeline state
  # =================================================================

  def test_make_compute_pipeline_state
    func = MTLFunction.new("test_func")
    pso = @device.make_compute_pipeline_state(func)
    assert_instance_of MTLComputePipelineState, pso
    assert_equal 1024, pso.max_total_threads_per_threadgroup
  end

  # =================================================================
  # Command queue and command buffer
  # =================================================================

  def test_make_command_queue
    queue = @device.make_command_queue
    assert_instance_of MTLCommandQueue, queue
  end

  def test_make_command_buffer
    queue = @device.make_command_queue
    cb = queue.make_command_buffer
    assert_instance_of MTLCommandBuffer, cb
    assert_equal MTLCommandBufferStatus::NOT_ENQUEUED, cb.status
  end

  def test_command_buffer_commit_and_wait
    queue = @device.make_command_queue
    cb = queue.make_command_buffer
    encoder = cb.make_compute_command_encoder
    encoder.end_encoding
    cb.commit
    assert_equal MTLCommandBufferStatus::COMPLETED, cb.status
    cb.wait_until_completed
  end

  def test_command_buffer_completed_handler
    queue = @device.make_command_queue
    cb = queue.make_command_buffer
    called = false
    cb.add_completed_handler(-> { called = true })
    encoder = cb.make_compute_command_encoder
    encoder.end_encoding
    cb.commit
    assert called
  end

  # =================================================================
  # Compute command encoder
  # =================================================================

  def test_compute_encoder_dispatch
    queue = @device.make_command_queue
    cb = queue.make_command_buffer

    func = MTLFunction.new("test_func")
    pso = @device.make_compute_pipeline_state(func)

    encoder = cb.make_compute_command_encoder
    encoder.set_compute_pipeline_state(pso)

    buf = @device.make_buffer(256)
    encoder.set_buffer(buf, offset: 0, index: 0)

    encoder.dispatch_threadgroups(
      MTLSize.new(width: 4),
      MTLSize.new(width: 64)
    )
    encoder.end_encoding
    assert encoder.ended?

    cb.commit
    cb.wait_until_completed
  end

  def test_compute_encoder_dispatch_threads
    queue = @device.make_command_queue
    cb = queue.make_command_buffer

    func = MTLFunction.new("test_func")
    pso = @device.make_compute_pipeline_state(func)

    encoder = cb.make_compute_command_encoder
    encoder.set_compute_pipeline_state(pso)
    encoder.dispatch_threads(
      MTLSize.new(width: 256),
      MTLSize.new(width: 64)
    )
    encoder.end_encoding
    cb.commit
  end

  def test_compute_encoder_set_bytes
    queue = @device.make_command_queue
    cb = queue.make_command_buffer
    encoder = cb.make_compute_command_encoder
    encoder.set_bytes("\x00\x00\x80\x3F".b, index: 2)
    encoder.end_encoding
    cb.commit
  end

  def test_dispatch_without_pipeline_raises
    queue = @device.make_command_queue
    cb = queue.make_command_buffer
    encoder = cb.make_compute_command_encoder
    assert_raises(RuntimeError) do
      encoder.dispatch_threadgroups(MTLSize.new(width: 1), MTLSize.new(width: 32))
    end
  end

  def test_multiple_buffers_bound
    queue = @device.make_command_queue
    cb = queue.make_command_buffer

    func = MTLFunction.new("multi_buf")
    pso = @device.make_compute_pipeline_state(func)

    encoder = cb.make_compute_command_encoder
    encoder.set_compute_pipeline_state(pso)

    buf0 = @device.make_buffer(128)
    buf1 = @device.make_buffer(128)
    encoder.set_buffer(buf0, offset: 0, index: 0)
    encoder.set_buffer(buf1, offset: 0, index: 1)

    encoder.dispatch_threadgroups(MTLSize.new(width: 2), MTLSize.new(width: 32))
    encoder.end_encoding
    cb.commit
  end

  # =================================================================
  # Blit command encoder
  # =================================================================

  def test_blit_encoder_copy
    queue = @device.make_command_queue
    cb = queue.make_command_buffer

    src = @device.make_buffer(64)
    dst = @device.make_buffer(64)
    src.write_bytes("\x42".b * 64)

    encoder = cb.make_blit_command_encoder
    encoder.copy_from_buffer(src, 0, to_buffer: dst, dst_offset: 0, size: 64)
    encoder.end_encoding
    assert encoder.ended?
    cb.commit
  end

  def test_blit_encoder_fill
    queue = @device.make_command_queue
    cb = queue.make_command_buffer

    buf = @device.make_buffer(32)
    encoder = cb.make_blit_command_encoder
    encoder.fill_buffer(buf, 0...32, 0xFF)
    encoder.end_encoding
    cb.commit
  end

  # =================================================================
  # MTLSize
  # =================================================================

  def test_mtl_size_defaults
    s = MTLSize.new(width: 64)
    assert_equal 64, s.width
    assert_equal 1, s.height
    assert_equal 1, s.depth
  end

  def test_mtl_size_3d
    s = MTLSize.new(width: 8, height: 8, depth: 4)
    assert_equal 8, s.width
    assert_equal 8, s.height
    assert_equal 4, s.depth
  end

  # =================================================================
  # Resource options and status
  # =================================================================

  def test_resource_options_constants
    assert_equal "shared", MTLResourceOptions::STORAGE_MODE_SHARED
    assert_equal "private", MTLResourceOptions::STORAGE_MODE_PRIVATE
    assert_equal "managed", MTLResourceOptions::STORAGE_MODE_MANAGED
  end

  def test_command_buffer_status_constants
    assert_equal "not_enqueued", MTLCommandBufferStatus::NOT_ENQUEUED
    assert_equal "completed", MTLCommandBufferStatus::COMPLETED
  end
end

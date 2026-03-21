# frozen_string_literal: true

require_relative "test_helper"

# ---------------------------------------------------------------------------
# OpenGL Compute Simulator Tests
# ---------------------------------------------------------------------------
class TestOpenGL < Minitest::Test
  include CodingAdventures::VendorApiSimulators

  def setup
    @gl = GLContext.new
  end

  # =================================================================
  # Shader management
  # =================================================================

  def test_create_shader
    shader = @gl.create_shader(GL_COMPUTE_SHADER)
    assert_kind_of Integer, shader
    assert shader > 0
  end

  def test_create_shader_invalid_type_raises
    assert_raises(ArgumentError) { @gl.create_shader(0xDEAD) }
  end

  def test_shader_source
    shader = @gl.create_shader(GL_COMPUTE_SHADER)
    @gl.shader_source(shader, "my_shader_src")
  end

  def test_shader_source_invalid_handle_raises
    assert_raises(ArgumentError) { @gl.shader_source(9999, "src") }
  end

  def test_compile_shader
    shader = @gl.create_shader(GL_COMPUTE_SHADER)
    @gl.shader_source(shader, "src")
    @gl.compile_shader(shader)
  end

  def test_compile_shader_invalid_handle_raises
    assert_raises(ArgumentError) { @gl.compile_shader(9999) }
  end

  def test_delete_shader
    shader = @gl.create_shader(GL_COMPUTE_SHADER)
    @gl.delete_shader(shader)
  end

  # =================================================================
  # Program management
  # =================================================================

  def test_create_program
    prog = @gl.create_program
    assert_kind_of Integer, prog
    assert prog > 0
  end

  def test_attach_shader
    shader = @gl.create_shader(GL_COMPUTE_SHADER)
    prog = @gl.create_program
    @gl.attach_shader(prog, shader)
  end

  def test_attach_shader_invalid_program_raises
    shader = @gl.create_shader(GL_COMPUTE_SHADER)
    assert_raises(ArgumentError) { @gl.attach_shader(9999, shader) }
  end

  def test_attach_shader_invalid_shader_raises
    prog = @gl.create_program
    assert_raises(ArgumentError) { @gl.attach_shader(prog, 9999) }
  end

  def test_link_program
    shader = @gl.create_shader(GL_COMPUTE_SHADER)
    @gl.shader_source(shader, "test")
    @gl.compile_shader(shader)
    prog = @gl.create_program
    @gl.attach_shader(prog, shader)
    @gl.link_program(prog)
  end

  def test_link_program_invalid_handle_raises
    assert_raises(ArgumentError) { @gl.link_program(9999) }
  end

  def test_link_program_no_shaders_raises
    prog = @gl.create_program
    assert_raises(RuntimeError) { @gl.link_program(prog) }
  end

  def test_use_program
    shader = @gl.create_shader(GL_COMPUTE_SHADER)
    @gl.shader_source(shader, "test")
    @gl.compile_shader(shader)
    prog = @gl.create_program
    @gl.attach_shader(prog, shader)
    @gl.link_program(prog)
    @gl.use_program(prog)
  end

  def test_use_program_zero_unbinds
    shader = @gl.create_shader(GL_COMPUTE_SHADER)
    @gl.shader_source(shader, "test")
    @gl.compile_shader(shader)
    prog = @gl.create_program
    @gl.attach_shader(prog, shader)
    @gl.link_program(prog)
    @gl.use_program(prog)
    @gl.use_program(0)
  end

  def test_use_program_invalid_handle_raises
    assert_raises(ArgumentError) { @gl.use_program(9999) }
  end

  def test_use_program_not_linked_raises
    prog = @gl.create_program
    assert_raises(RuntimeError) { @gl.use_program(prog) }
  end

  def test_delete_program
    shader = @gl.create_shader(GL_COMPUTE_SHADER)
    @gl.shader_source(shader, "test")
    @gl.compile_shader(shader)
    prog = @gl.create_program
    @gl.attach_shader(prog, shader)
    @gl.link_program(prog)
    @gl.use_program(prog)
    @gl.delete_program(prog)
  end

  # =================================================================
  # Buffer management
  # =================================================================

  def test_gen_buffers
    bufs = @gl.gen_buffers(3)
    assert_equal 3, bufs.length
    assert_kind_of Integer, bufs[0]
  end

  def test_bind_buffer
    bufs = @gl.gen_buffers(1)
    @gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
  end

  def test_bind_buffer_zero_unbinds
    bufs = @gl.gen_buffers(1)
    @gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
    @gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, 0)
  end

  def test_bind_buffer_invalid_handle_raises
    assert_raises(ArgumentError) { @gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, 9999) }
  end

  def test_buffer_data
    bufs = @gl.gen_buffers(1)
    @gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
    @gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 256, nil, GL_STATIC_DRAW)
  end

  def test_buffer_data_with_initial_data
    bufs = @gl.gen_buffers(1)
    @gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
    data = "\x01\x02\x03\x04".b
    @gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 4, data, GL_DYNAMIC_DRAW)
  end

  def test_buffer_data_no_bound_buffer_raises
    assert_raises(RuntimeError) { @gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 64, nil, GL_STATIC_DRAW) }
  end

  def test_buffer_sub_data
    bufs = @gl.gen_buffers(1)
    @gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
    @gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 64, nil, GL_DYNAMIC_DRAW)
    @gl.buffer_sub_data(GL_SHADER_STORAGE_BUFFER, 0, "\xAA\xBB".b)
  end

  def test_buffer_sub_data_no_bound_buffer_raises
    assert_raises(RuntimeError) { @gl.buffer_sub_data(GL_SHADER_STORAGE_BUFFER, 0, "\x00".b) }
  end

  def test_bind_buffer_base
    bufs = @gl.gen_buffers(1)
    @gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
    @gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 64, nil, GL_STATIC_DRAW)
    @gl.bind_buffer_base(GL_SHADER_STORAGE_BUFFER, 0, bufs[0])
  end

  def test_bind_buffer_base_invalid_handle_raises
    assert_raises(ArgumentError) { @gl.bind_buffer_base(GL_SHADER_STORAGE_BUFFER, 0, 9999) }
  end

  def test_delete_buffers
    bufs = @gl.gen_buffers(2)
    bufs.each do |b|
      @gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, b)
      @gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 64, nil, GL_STATIC_DRAW)
    end
    @gl.delete_buffers(bufs)
  end

  def test_delete_buffers_unallocated
    bufs = @gl.gen_buffers(2)
    @gl.delete_buffers(bufs)
  end

  # =================================================================
  # Map buffer range
  # =================================================================

  def test_map_buffer_range
    bufs = @gl.gen_buffers(1)
    data = "\x01\x02\x03\x04".b * 4
    @gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
    @gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 16, data, GL_DYNAMIC_DRAW)
    result = @gl.map_buffer_range(GL_SHADER_STORAGE_BUFFER, 0, 16, GL_MAP_READ_BIT)
    assert_kind_of String, result
    assert_equal 16, result.bytesize
  end

  def test_map_buffer_range_no_bound_buffer_raises
    assert_raises(RuntimeError) { @gl.map_buffer_range(GL_SHADER_STORAGE_BUFFER, 0, 16, GL_MAP_READ_BIT) }
  end

  def test_unmap_buffer
    result = @gl.unmap_buffer(GL_SHADER_STORAGE_BUFFER)
    assert_equal true, result
  end

  # =================================================================
  # Compute dispatch
  # =================================================================

  def test_dispatch_compute
    shader = @gl.create_shader(GL_COMPUTE_SHADER)
    @gl.shader_source(shader, "test")
    @gl.compile_shader(shader)
    prog = @gl.create_program
    @gl.attach_shader(prog, shader)
    @gl.link_program(prog)
    @gl.use_program(prog)

    bufs = @gl.gen_buffers(1)
    @gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
    @gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 256, nil, GL_DYNAMIC_DRAW)
    @gl.bind_buffer_base(GL_SHADER_STORAGE_BUFFER, 0, bufs[0])

    @gl.dispatch_compute(4, 1, 1)
  end

  def test_dispatch_compute_no_program_raises
    assert_raises(RuntimeError) { @gl.dispatch_compute(1) }
  end

  def test_dispatch_compute_3d
    shader = @gl.create_shader(GL_COMPUTE_SHADER)
    @gl.shader_source(shader, "test")
    @gl.compile_shader(shader)
    prog = @gl.create_program
    @gl.attach_shader(prog, shader)
    @gl.link_program(prog)
    @gl.use_program(prog)
    @gl.dispatch_compute(2, 2, 2)
  end

  # =================================================================
  # Synchronization
  # =================================================================

  def test_memory_barrier
    @gl.memory_barrier(GL_SHADER_STORAGE_BARRIER_BIT)
  end

  def test_memory_barrier_all
    @gl.memory_barrier(GL_ALL_BARRIER_BITS)
  end

  def test_fence_sync
    sync = @gl.fence_sync
    assert_kind_of Integer, sync
    assert sync > 0
  end

  def test_client_wait_sync_already_signaled
    sync = @gl.fence_sync
    result = @gl.client_wait_sync(sync, GL_SYNC_FLUSH_COMMANDS_BIT, 1_000_000)
    assert_equal GL_ALREADY_SIGNALED, result
  end

  def test_client_wait_sync_invalid_handle
    result = @gl.client_wait_sync(9999, 0, 1_000_000)
    assert_equal GL_WAIT_FAILED, result
  end

  def test_delete_sync
    sync = @gl.fence_sync
    @gl.delete_sync(sync)
  end

  def test_finish
    @gl.finish
  end

  # =================================================================
  # Uniforms
  # =================================================================

  def test_get_uniform_location
    shader = @gl.create_shader(GL_COMPUTE_SHADER)
    @gl.shader_source(shader, "test")
    @gl.compile_shader(shader)
    prog = @gl.create_program
    @gl.attach_shader(prog, shader)
    @gl.link_program(prog)
    loc = @gl.get_uniform_location(prog, "my_uniform")
    assert_kind_of Integer, loc
  end

  def test_get_uniform_location_invalid_program_raises
    assert_raises(ArgumentError) { @gl.get_uniform_location(9999, "u") }
  end

  def test_uniform_1f
    shader = @gl.create_shader(GL_COMPUTE_SHADER)
    @gl.shader_source(shader, "test")
    @gl.compile_shader(shader)
    prog = @gl.create_program
    @gl.attach_shader(prog, shader)
    @gl.link_program(prog)
    @gl.use_program(prog)
    loc = @gl.get_uniform_location(prog, "scale")
    @gl.uniform_1f(loc, 3.14)
  end

  def test_uniform_1i
    shader = @gl.create_shader(GL_COMPUTE_SHADER)
    @gl.shader_source(shader, "test")
    @gl.compile_shader(shader)
    prog = @gl.create_program
    @gl.attach_shader(prog, shader)
    @gl.link_program(prog)
    @gl.use_program(prog)
    loc = @gl.get_uniform_location(prog, "count")
    @gl.uniform_1i(loc, 42)
  end

  # =================================================================
  # GL constants
  # =================================================================

  def test_gl_constants_exist
    assert_equal 0x91B9, GL_COMPUTE_SHADER
    assert_equal 0x90D2, GL_SHADER_STORAGE_BUFFER
    assert_equal 0x8892, GL_ARRAY_BUFFER
    assert_equal 0x8A11, GL_UNIFORM_BUFFER
  end

  def test_gl_usage_constants
    assert_equal 0x88E4, GL_STATIC_DRAW
    assert_equal 0x88E8, GL_DYNAMIC_DRAW
    assert_equal 0x88E0, GL_STREAM_DRAW
  end

  def test_gl_map_bits
    assert_equal 0x0001, GL_MAP_READ_BIT
    assert_equal 0x0002, GL_MAP_WRITE_BIT
  end

  def test_gl_barrier_bits
    assert_equal 0x00002000, GL_SHADER_STORAGE_BARRIER_BIT
    assert_equal 0x00000200, GL_BUFFER_UPDATE_BARRIER_BIT
    assert_equal 0xFFFFFFFF, GL_ALL_BARRIER_BITS
  end

  def test_gl_sync_constants
    assert_equal 0x911A, GL_ALREADY_SIGNALED
    assert_equal 0x911C, GL_CONDITION_SATISFIED
    assert_equal 0x911B, GL_TIMEOUT_EXPIRED
    assert_equal 0x911D, GL_WAIT_FAILED
  end

  # =================================================================
  # Handle generation -- unique IDs
  # =================================================================

  def test_handles_are_unique
    s1 = @gl.create_shader(GL_COMPUTE_SHADER)
    s2 = @gl.create_shader(GL_COMPUTE_SHADER)
    p1 = @gl.create_program
    bufs = @gl.gen_buffers(2)
    all_ids = [s1, s2, p1] + bufs
    assert_equal all_ids.length, all_ids.uniq.length
  end

  # =================================================================
  # Buffer reallocation
  # =================================================================

  def test_buffer_data_replaces_old_allocation
    bufs = @gl.gen_buffers(1)
    @gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
    @gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 64, nil, GL_STATIC_DRAW)
    # Reallocate same handle with different size
    @gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 128, nil, GL_DYNAMIC_DRAW)
  end
end

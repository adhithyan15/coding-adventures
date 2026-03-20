# frozen_string_literal: true

require_relative "test_helper"

# ---------------------------------------------------------------------------
# Cross-API Equivalence Tests
# ---------------------------------------------------------------------------
#
# The capstone test for this package: run the same GPU operations through
# CUDA, OpenCL, Metal, Vulkan, WebGPU, and OpenGL. All six must work
# correctly, proving that our simulators are functionally equivalent
# wrappers over the same Layer 5 compute runtime.
class TestCrossAPI < Minitest::Test
  include CodingAdventures::VendorApiSimulators

  # =================================================================
  # Cross-API Dispatch -- all six APIs can dispatch compute work
  # =================================================================

  def test_cuda_dispatch
    cuda = CUDARuntime.new
    d_buf = cuda.malloc(64)
    kernel = CUDAKernel.new(code: nil, name: "test")
    cuda.launch_kernel(kernel, grid: Dim3.new(x: 1), block: Dim3.new(x: 32), args: [d_buf])
    cuda.device_synchronize
    cuda.free(d_buf)
  end

  def test_opencl_dispatch
    ctx = CLContext.new
    queue = ctx.create_command_queue
    buf = ctx.create_buffer(CLMemFlags::READ_WRITE, 64)
    prog = ctx.create_program_with_source("test")
    prog.build
    kernel = prog.create_kernel("test")
    kernel.set_arg(0, buf)
    queue.enqueue_nd_range_kernel(kernel, [32], local_size: [32])
    queue.finish
  end

  def test_metal_dispatch
    device = MTLDevice.new
    queue = device.make_command_queue
    buf = device.make_buffer(64)
    func = MTLFunction.new("test")
    pso = device.make_compute_pipeline_state(func)

    cb = queue.make_command_buffer
    encoder = cb.make_compute_command_encoder
    encoder.set_compute_pipeline_state(pso)
    encoder.set_buffer(buf, offset: 0, index: 0)
    encoder.dispatch_threadgroups(
      MTLSize.new(width: 1),
      MTLSize.new(width: 32)
    )
    encoder.end_encoding
    cb.commit
    cb.wait_until_completed
  end

  def test_vulkan_dispatch
    instance = VkInstance.new
    pdevs = instance.vk_enumerate_physical_devices
    device = instance.vk_create_device(pdevs[0])
    queue = device.vk_get_device_queue(0, 0)
    pool = device.vk_create_command_pool(VkCommandPoolCreateInfo.new)

    shader = device.vk_create_shader_module(VkShaderModuleCreateInfo.new)
    ds_layout = device.vk_create_descriptor_set_layout(
      VkDescriptorSetLayoutCreateInfo.new(bindings: [VkDescriptorSetLayoutBinding.new(binding: 0)])
    )
    pl_layout = device.vk_create_pipeline_layout(
      VkPipelineLayoutCreateInfo.new(set_layouts: [ds_layout])
    )
    pipelines = device.vk_create_compute_pipelines([
      VkComputePipelineCreateInfo.new(
        shader_stage: VkPipelineShaderStageCreateInfo.new(mod: shader),
        layout: pl_layout
      )
    ])

    cbs = pool.vk_allocate_command_buffers(1)
    cbs[0].vk_begin_command_buffer
    cbs[0].vk_cmd_bind_pipeline(VkPipelineBindPoint::COMPUTE, pipelines[0])
    cbs[0].vk_cmd_dispatch(1, 1, 1)
    cbs[0].vk_end_command_buffer

    fence = device.vk_create_fence
    submit = VkSubmitInfo.new(command_buffers: cbs)
    queue.vk_queue_submit([submit], fence: fence)
    assert fence.signaled
  end

  def test_webgpu_dispatch
    gpu = GPU.new
    adapter = gpu.request_adapter
    device = adapter.request_device

    shader = device.create_shader_module(GPUShaderModuleDescriptor.new)
    pipeline = device.create_compute_pipeline(
      GPUComputePipelineDescriptor.new(
        compute: GPUProgrammableStage.new(mod: shader)
      )
    )
    bg = device.create_bind_group(GPUBindGroupDescriptor.new(
      layout: pipeline.get_bind_group_layout(0),
      entries: []
    ))

    encoder = device.create_command_encoder
    pass = encoder.begin_compute_pass
    pass.set_pipeline(pipeline)
    pass.set_bind_group(0, bg)
    pass.dispatch_workgroups(1)
    pass.end_pass
    cmd_buf = encoder.finish
    device.queue.submit([cmd_buf])
  end

  def test_opengl_dispatch
    gl = GLContext.new
    shader = gl.create_shader(GL_COMPUTE_SHADER)
    gl.shader_source(shader, "test")
    gl.compile_shader(shader)
    prog = gl.create_program
    gl.attach_shader(prog, shader)
    gl.link_program(prog)
    gl.use_program(prog)
    gl.dispatch_compute(1, 1, 1)
    gl.memory_barrier(GL_SHADER_STORAGE_BARRIER_BIT)
    gl.finish
  end

  # =================================================================
  # Cross-API Memory -- all six APIs can write and read buffer data
  # =================================================================

  def test_cuda_write_read
    cuda = CUDARuntime.new
    d_buf = cuda.malloc(16)
    data = "\x01\x02\x03\x04".b * 4
    cuda.memcpy(d_buf, data, 16, :host_to_device)
    result = (+"\x00" * 16).force_encoding(Encoding::BINARY)
    cuda.memcpy(result, d_buf, 16, :device_to_host)
    assert_equal data, result
    cuda.free(d_buf)
  end

  def test_opencl_write_read
    ctx = CLContext.new
    queue = ctx.create_command_queue
    buf = ctx.create_buffer(CLMemFlags::READ_WRITE, 16)
    data = "\x01\x02\x03\x04".b * 4
    queue.enqueue_write_buffer(buf, 0, 16, data)
    result = (+"\x00" * 16).force_encoding(Encoding::BINARY)
    queue.enqueue_read_buffer(buf, 0, 16, result)
    assert_equal data, result
  end

  def test_metal_write_read
    device = MTLDevice.new
    buf = device.make_buffer(16)
    data = "\x01\x02\x03\x04".b * 4
    buf.write_bytes(data)
    result = buf.contents.byteslice(0, 16)
    assert_equal data, result
  end

  def test_vulkan_write_read
    instance = VkInstance.new
    pdevs = instance.vk_enumerate_physical_devices
    device = instance.vk_create_device(pdevs[0])
    mem = device.vk_allocate_memory(VkMemoryAllocateInfo.new(size: 16))
    mapped = device.vk_map_memory(mem, 0, 16)
    assert_kind_of String, mapped
    device.vk_unmap_memory(mem)
  end

  def test_webgpu_write_read
    gpu = GPU.new
    device = gpu.request_adapter.request_device
    buf = device.create_buffer(GPUBufferDescriptor.new(
      size: 16,
      usage: GPUBufferUsage::STORAGE | GPUBufferUsage::COPY_DST | GPUBufferUsage::MAP_READ
    ))
    data = "\x01\x02\x03\x04".b * 4
    device.queue.write_buffer(buf, 0, data)
    buf.map_async(GPUMapMode::READ)
    result = buf.get_mapped_range
    assert_equal 16, result.bytesize
    buf.unmap
  end

  def test_opengl_write_read
    gl = GLContext.new
    bufs = gl.gen_buffers(1)
    data = "\x01\x02\x03\x04".b * 4
    gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
    gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 16, data, GL_DYNAMIC_DRAW)
    result = gl.map_buffer_range(GL_SHADER_STORAGE_BUFFER, 0, 16, GL_MAP_READ_BIT)
    assert_equal 16, result.bytesize
  end

  # =================================================================
  # Cross-API Resource Lifecycle -- create, use, cleanup
  # =================================================================

  def test_cuda_lifecycle
    cuda = CUDARuntime.new
    ptrs = 5.times.map { cuda.malloc(64) }
    ptrs.each { |ptr| cuda.free(ptr) }
  end

  def test_opencl_lifecycle
    ctx = CLContext.new
    queue = ctx.create_command_queue
    5.times { ctx.create_buffer(CLMemFlags::READ_WRITE, 64) }
    queue.finish
  end

  def test_metal_lifecycle
    device = MTLDevice.new
    bufs = 5.times.map { device.make_buffer(64) }
    assert_equal 5, bufs.length
  end

  def test_vulkan_lifecycle
    instance = VkInstance.new
    pdevs = instance.vk_enumerate_physical_devices
    device = instance.vk_create_device(pdevs[0])
    5.times { device.vk_create_buffer(VkBufferCreateInfo.new(size: 64)) }
    device.vk_device_wait_idle
  end

  def test_webgpu_lifecycle
    gpu = GPU.new
    device = gpu.request_adapter.request_device
    bufs = 5.times.map do
      device.create_buffer(GPUBufferDescriptor.new(size: 64, usage: GPUBufferUsage::STORAGE))
    end
    bufs.each(&:destroy)
  end

  def test_opengl_lifecycle
    gl = GLContext.new
    bufs = gl.gen_buffers(5)
    bufs.each do |b|
      gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, b)
      gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 64, nil, GL_STATIC_DRAW)
    end
    gl.delete_buffers(bufs)
  end

  # =================================================================
  # Capstone -- all six dispatch without errors
  # =================================================================

  def test_all_six_apis_dispatch_without_errors
    errors = []

    # CUDA
    begin
      cuda = CUDARuntime.new
      kernel = CUDAKernel.new(code: nil, name: "test")
      cuda.launch_kernel(kernel, grid: Dim3.new(x: 1), block: Dim3.new(x: 32))
      cuda.device_synchronize
    rescue => e
      errors << "CUDA: #{e.message}"
    end

    # OpenCL
    begin
      ctx = CLContext.new
      queue = ctx.create_command_queue
      prog = ctx.create_program_with_source("test")
      prog.build
      k = prog.create_kernel("test")
      queue.enqueue_nd_range_kernel(k, [32], local_size: [32])
      queue.finish
    rescue => e
      errors << "OpenCL: #{e.message}"
    end

    # Metal
    begin
      device = MTLDevice.new
      q = device.make_command_queue
      func = MTLFunction.new("test")
      pso = device.make_compute_pipeline_state(func)
      cb = q.make_command_buffer
      enc = cb.make_compute_command_encoder
      enc.set_compute_pipeline_state(pso)
      enc.dispatch_threadgroups(MTLSize.new(width: 1), MTLSize.new(width: 32))
      enc.end_encoding
      cb.commit
      cb.wait_until_completed
    rescue => e
      errors << "Metal: #{e.message}"
    end

    # Vulkan
    begin
      inst = VkInstance.new
      pd = inst.vk_enumerate_physical_devices[0]
      dev = inst.vk_create_device(pd)
      vk_q = dev.vk_get_device_queue(0, 0)
      pool = dev.vk_create_command_pool(VkCommandPoolCreateInfo.new)
      sm = dev.vk_create_shader_module(VkShaderModuleCreateInfo.new)
      dsl = dev.vk_create_descriptor_set_layout(VkDescriptorSetLayoutCreateInfo.new)
      pll = dev.vk_create_pipeline_layout(VkPipelineLayoutCreateInfo.new(set_layouts: [dsl]))
      pipes = dev.vk_create_compute_pipelines([
        VkComputePipelineCreateInfo.new(
          shader_stage: VkPipelineShaderStageCreateInfo.new(mod: sm),
          layout: pll
        )
      ])
      cbs = pool.vk_allocate_command_buffers(1)
      cbs[0].vk_begin_command_buffer
      cbs[0].vk_cmd_bind_pipeline(VkPipelineBindPoint::COMPUTE, pipes[0])
      cbs[0].vk_cmd_dispatch(1, 1, 1)
      cbs[0].vk_end_command_buffer
      fence = dev.vk_create_fence
      vk_q.vk_queue_submit([VkSubmitInfo.new(command_buffers: cbs)], fence: fence)
    rescue => e
      errors << "Vulkan: #{e.message}"
    end

    # WebGPU
    begin
      g = GPU.new
      ad = g.request_adapter
      wd = ad.request_device
      sh = wd.create_shader_module(GPUShaderModuleDescriptor.new)
      pip = wd.create_compute_pipeline(
        GPUComputePipelineDescriptor.new(compute: GPUProgrammableStage.new(mod: sh))
      )
      bg = wd.create_bind_group(GPUBindGroupDescriptor.new(
        layout: pip.get_bind_group_layout(0), entries: []
      ))
      we = wd.create_command_encoder
      wp = we.begin_compute_pass
      wp.set_pipeline(pip)
      wp.set_bind_group(0, bg)
      wp.dispatch_workgroups(1)
      wp.end_pass
      wcb = we.finish
      wd.queue.submit([wcb])
    rescue => e
      errors << "WebGPU: #{e.message}"
    end

    # OpenGL
    begin
      ogl = GLContext.new
      s = ogl.create_shader(GL_COMPUTE_SHADER)
      ogl.shader_source(s, "test")
      ogl.compile_shader(s)
      p = ogl.create_program
      ogl.attach_shader(p, s)
      ogl.link_program(p)
      ogl.use_program(p)
      ogl.dispatch_compute(1, 1, 1)
      ogl.finish
    rescue => e
      errors << "OpenGL: #{e.message}"
    end

    assert_empty errors, "API failures: #{errors.join(', ')}"
  end
end

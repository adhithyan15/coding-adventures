# frozen_string_literal: true

require_relative "test_helper"

# ---------------------------------------------------------------------------
# Vulkan Runtime Simulator Tests
# ---------------------------------------------------------------------------
class TestVulkan < Minitest::Test
  include CodingAdventures::VendorApiSimulators

  def setup
    @instance = VkInstance.new
    @physical_devices = @instance.vk_enumerate_physical_devices
    @physical = @physical_devices[0]
    @device = @instance.vk_create_device(@physical)
  end

  # =================================================================
  # Instance and device discovery
  # =================================================================

  def test_instance_creates_successfully
    assert_instance_of VkInstance, @instance
  end

  def test_enumerate_physical_devices
    assert_kind_of Array, @physical_devices
    refute_empty @physical_devices
    assert_instance_of VkPhysicalDevice, @physical_devices[0]
  end

  def test_physical_device_properties
    props = @physical.vk_get_physical_device_properties
    assert_kind_of Hash, props
    refute_empty props["device_name"]
    refute_empty props["vendor"]
  end

  def test_physical_device_memory_properties
    props = @physical.vk_get_physical_device_memory_properties
    assert_kind_of Hash, props
    assert props["heap_count"] > 0
    assert_kind_of Array, props["heaps"]
  end

  def test_physical_device_queue_family_properties
    qf_props = @physical.vk_get_physical_device_queue_family_properties
    assert_kind_of Array, qf_props
    refute_empty qf_props
    assert_kind_of Hash, qf_props[0]
  end

  # =================================================================
  # Logical device creation
  # =================================================================

  def test_create_device
    assert_instance_of VkDevice, @device
  end

  def test_get_device_queue
    queue = @device.vk_get_device_queue(0, 0)
    assert_instance_of VkQueue, queue
  end

  def test_get_device_queue_transfer
    queue = @device.vk_get_device_queue(1, 0)
    assert_instance_of VkQueue, queue
  end

  # =================================================================
  # Memory allocation
  # =================================================================

  def test_allocate_memory_device_local
    alloc_info = VkMemoryAllocateInfo.new(size: 256, memory_type_index: 0)
    mem = @device.vk_allocate_memory(alloc_info)
    assert_instance_of VkDeviceMemory, mem
  end

  def test_allocate_memory_host_visible
    alloc_info = VkMemoryAllocateInfo.new(size: 128, memory_type_index: 1)
    mem = @device.vk_allocate_memory(alloc_info)
    assert_instance_of VkDeviceMemory, mem
  end

  def test_map_and_unmap_memory
    alloc_info = VkMemoryAllocateInfo.new(size: 64, memory_type_index: 0)
    mem = @device.vk_allocate_memory(alloc_info)
    data = @device.vk_map_memory(mem, 0, 64)
    assert_kind_of String, data
    @device.vk_unmap_memory(mem)
  end

  # =================================================================
  # Buffer creation
  # =================================================================

  def test_create_buffer
    ci = VkBufferCreateInfo.new(size: 512)
    buf = @device.vk_create_buffer(ci)
    assert_instance_of VkBuffer, buf
    assert_equal 512, buf.size
  end

  def test_bind_buffer_memory
    ci = VkBufferCreateInfo.new(size: 256)
    buf = @device.vk_create_buffer(ci)
    alloc_info = VkMemoryAllocateInfo.new(size: 256)
    mem = @device.vk_allocate_memory(alloc_info)
    # Should not raise
    @device.vk_bind_buffer_memory(buf, mem, 0)
  end

  # =================================================================
  # Shader and pipeline
  # =================================================================

  def test_create_shader_module
    ci = VkShaderModuleCreateInfo.new(code: nil)
    shader = @device.vk_create_shader_module(ci)
    assert_instance_of VkShaderModule, shader
  end

  def test_create_descriptor_set_layout
    binding = VkDescriptorSetLayoutBinding.new(binding: 0, descriptor_type: "storage")
    ci = VkDescriptorSetLayoutCreateInfo.new(bindings: [binding])
    layout = @device.vk_create_descriptor_set_layout(ci)
    assert_instance_of VkDescriptorSetLayout, layout
  end

  def test_create_pipeline_layout
    binding = VkDescriptorSetLayoutBinding.new(binding: 0)
    ds_ci = VkDescriptorSetLayoutCreateInfo.new(bindings: [binding])
    ds_layout = @device.vk_create_descriptor_set_layout(ds_ci)

    pl_ci = VkPipelineLayoutCreateInfo.new(set_layouts: [ds_layout])
    pl_layout = @device.vk_create_pipeline_layout(pl_ci)
    assert_instance_of VkPipelineLayout, pl_layout
  end

  def test_create_compute_pipeline
    shader_ci = VkShaderModuleCreateInfo.new
    shader = @device.vk_create_shader_module(shader_ci)

    ds_ci = VkDescriptorSetLayoutCreateInfo.new(bindings: [])
    ds_layout = @device.vk_create_descriptor_set_layout(ds_ci)
    pl_ci = VkPipelineLayoutCreateInfo.new(set_layouts: [ds_layout])
    pl_layout = @device.vk_create_pipeline_layout(pl_ci)

    stage = VkPipelineShaderStageCreateInfo.new(mod: shader)
    pipe_ci = VkComputePipelineCreateInfo.new(shader_stage: stage, layout: pl_layout)

    pipelines = @device.vk_create_compute_pipelines([pipe_ci])
    assert_equal 1, pipelines.length
    assert_instance_of VkPipeline, pipelines[0]
  end

  # =================================================================
  # Descriptor sets
  # =================================================================

  def test_allocate_descriptor_sets
    binding = VkDescriptorSetLayoutBinding.new(binding: 0)
    ds_ci = VkDescriptorSetLayoutCreateInfo.new(bindings: [binding])
    ds_layout = @device.vk_create_descriptor_set_layout(ds_ci)

    alloc_info = VkDescriptorSetAllocateInfo.new(set_layouts: [ds_layout])
    sets = @device.vk_allocate_descriptor_sets(alloc_info)
    assert_equal 1, sets.length
    assert_instance_of VkDescriptorSet, sets[0]
  end

  def test_update_descriptor_sets
    binding = VkDescriptorSetLayoutBinding.new(binding: 0)
    ds_ci = VkDescriptorSetLayoutCreateInfo.new(bindings: [binding])
    ds_layout = @device.vk_create_descriptor_set_layout(ds_ci)

    alloc_info = VkDescriptorSetAllocateInfo.new(set_layouts: [ds_layout])
    sets = @device.vk_allocate_descriptor_sets(alloc_info)

    buf_ci = VkBufferCreateInfo.new(size: 64)
    buf = @device.vk_create_buffer(buf_ci)

    write = VkWriteDescriptorSet.new(
      dst_set: sets[0],
      dst_binding: 0,
      buffer_info: VkDescriptorBufferInfo.new(buffer: buf)
    )
    @device.vk_update_descriptor_sets([write])
  end

  # =================================================================
  # Command pool and command buffer
  # =================================================================

  def test_create_command_pool
    ci = VkCommandPoolCreateInfo.new(queue_family_index: 0)
    pool = @device.vk_create_command_pool(ci)
    assert_instance_of VkCommandPool, pool
  end

  def test_allocate_command_buffers
    ci = VkCommandPoolCreateInfo.new
    pool = @device.vk_create_command_pool(ci)
    cbs = pool.vk_allocate_command_buffers(2)
    assert_equal 2, cbs.length
    assert_instance_of VkCommandBuffer, cbs[0]
  end

  def test_reset_command_pool
    ci = VkCommandPoolCreateInfo.new
    pool = @device.vk_create_command_pool(ci)
    cbs = pool.vk_allocate_command_buffers(1)
    pool.vk_reset_command_pool
  end

  def test_free_command_buffers
    ci = VkCommandPoolCreateInfo.new
    pool = @device.vk_create_command_pool(ci)
    cbs = pool.vk_allocate_command_buffers(2)
    pool.vk_free_command_buffers([cbs[0]])
  end

  def test_command_buffer_record_and_dispatch
    ci = VkCommandPoolCreateInfo.new
    pool = @device.vk_create_command_pool(ci)
    cbs = pool.vk_allocate_command_buffers(1)
    cb = cbs[0]

    # Create pipeline
    shader = @device.vk_create_shader_module(VkShaderModuleCreateInfo.new)
    ds_layout = @device.vk_create_descriptor_set_layout(VkDescriptorSetLayoutCreateInfo.new)
    pl_layout = @device.vk_create_pipeline_layout(VkPipelineLayoutCreateInfo.new(set_layouts: [ds_layout]))
    stage = VkPipelineShaderStageCreateInfo.new(mod: shader)
    pipelines = @device.vk_create_compute_pipelines([
      VkComputePipelineCreateInfo.new(shader_stage: stage, layout: pl_layout)
    ])

    cb.vk_begin_command_buffer
    cb.vk_cmd_bind_pipeline(VkPipelineBindPoint::COMPUTE, pipelines[0])
    cb.vk_cmd_dispatch(4, 1, 1)
    cb.vk_end_command_buffer

    queue = @device.vk_get_device_queue(0, 0)
    fence = @device.vk_create_fence
    submit = VkSubmitInfo.new(command_buffers: [cb])
    result = queue.vk_queue_submit([submit], fence: fence)
    assert_equal VkResult::SUCCESS, result
  end

  # =================================================================
  # Fence and semaphore
  # =================================================================

  def test_create_fence
    fence = @device.vk_create_fence
    assert_instance_of VkFence, fence
    refute fence.signaled
  end

  def test_create_fence_signaled
    fence = @device.vk_create_fence(flags: 1)
    assert fence.signaled
  end

  def test_wait_for_fences
    fence = @device.vk_create_fence(flags: 1)
    result = @device.vk_wait_for_fences([fence], true, 1000)
    assert_equal VkResult::SUCCESS, result
  end

  def test_wait_for_fences_not_ready
    fence = @device.vk_create_fence
    result = @device.vk_wait_for_fences([fence], true, 1000)
    assert_equal VkResult::NOT_READY, result
  end

  def test_reset_fences
    fence = @device.vk_create_fence(flags: 1)
    assert fence.signaled
    @device.vk_reset_fences([fence])
    refute fence.signaled
  end

  def test_create_semaphore
    sem = @device.vk_create_semaphore
    assert_instance_of VkSemaphore, sem
  end

  # =================================================================
  # Command buffer commands
  # =================================================================

  def test_cmd_copy_buffer
    ci = VkCommandPoolCreateInfo.new
    pool = @device.vk_create_command_pool(ci)
    cb = pool.vk_allocate_command_buffers(1)[0]

    src = @device.vk_create_buffer(VkBufferCreateInfo.new(size: 64))
    dst = @device.vk_create_buffer(VkBufferCreateInfo.new(size: 64))

    cb.vk_begin_command_buffer
    cb.vk_cmd_copy_buffer(src, dst, [VkBufferCopy.new(size: 64)])
    cb.vk_end_command_buffer
  end

  def test_cmd_fill_buffer
    ci = VkCommandPoolCreateInfo.new
    pool = @device.vk_create_command_pool(ci)
    cb = pool.vk_allocate_command_buffers(1)[0]
    buf = @device.vk_create_buffer(VkBufferCreateInfo.new(size: 32))

    cb.vk_begin_command_buffer
    cb.vk_cmd_fill_buffer(buf, 0, 32, 0xFF)
    cb.vk_end_command_buffer
  end

  def test_cmd_push_constants
    ci = VkCommandPoolCreateInfo.new
    pool = @device.vk_create_command_pool(ci)
    cb = pool.vk_allocate_command_buffers(1)[0]

    ds_layout = @device.vk_create_descriptor_set_layout(VkDescriptorSetLayoutCreateInfo.new)
    pl_layout = @device.vk_create_pipeline_layout(VkPipelineLayoutCreateInfo.new(set_layouts: [ds_layout]))

    cb.vk_begin_command_buffer
    cb.vk_cmd_push_constants(pl_layout, 0, "\x00\x00\x80\x3F".b)
    cb.vk_end_command_buffer
  end

  def test_cmd_pipeline_barrier
    ci = VkCommandPoolCreateInfo.new
    pool = @device.vk_create_command_pool(ci)
    cb = pool.vk_allocate_command_buffers(1)[0]

    cb.vk_begin_command_buffer
    cb.vk_cmd_pipeline_barrier("compute", "compute")
    cb.vk_end_command_buffer
  end

  def test_device_wait_idle
    @device.vk_device_wait_idle
  end

  def test_queue_wait_idle
    queue = @device.vk_get_device_queue(0, 0)
    queue.vk_queue_wait_idle
  end

  # =================================================================
  # VkResult constants
  # =================================================================

  def test_vk_result_constants
    assert_equal 0, VkResult::SUCCESS
    assert_equal 1, VkResult::NOT_READY
    assert_equal 2, VkResult::TIMEOUT
    assert_equal(-3, VkResult::ERROR_OUT_OF_DEVICE_MEMORY)
  end

  # =================================================================
  # Create info defaults
  # =================================================================

  def test_buffer_create_info_defaults
    ci = VkBufferCreateInfo.new
    assert_equal 0, ci.size
    assert_equal VkBufferUsageFlagBits::STORAGE_BUFFER, ci.usage
    assert_equal VkSharingMode::EXCLUSIVE, ci.sharing_mode
  end

  def test_submit_info_defaults
    si = VkSubmitInfo.new
    assert_equal [], si.command_buffers
    assert_equal [], si.wait_semaphores
    assert_equal [], si.signal_semaphores
  end
end

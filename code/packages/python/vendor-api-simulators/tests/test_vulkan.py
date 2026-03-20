"""Tests for the Vulkan runtime simulator."""

import pytest
from gpu_core import limm, halt

from vendor_api_simulators.vulkan import (
    VkInstance,
    VkPhysicalDevice,
    VkDevice,
    VkQueue,
    VkCommandPool,
    VkCommandBuffer,
    VkBuffer,
    VkDeviceMemory,
    VkShaderModule,
    VkPipeline,
    VkDescriptorSetLayout,
    VkPipelineLayout,
    VkDescriptorSet,
    VkFence,
    VkSemaphore,
    VkResult,
    VkPipelineBindPoint,
    VkBufferUsageFlagBits,
    VkBufferCreateInfo,
    VkMemoryAllocateInfo,
    VkShaderModuleCreateInfo,
    VkComputePipelineCreateInfo,
    VkPipelineShaderStageCreateInfo,
    VkSubmitInfo,
    VkBufferCopy,
    VkWriteDescriptorSet,
    VkDescriptorBufferInfo,
    VkCommandPoolCreateInfo,
    VkDescriptorSetLayoutCreateInfo,
    VkDescriptorSetLayoutBinding,
    VkPipelineLayoutCreateInfo,
    VkDescriptorSetAllocateInfo,
)


class TestVkInstance:
    """Test instance creation and device enumeration."""

    def test_create_instance(self):
        """VkInstance initializes successfully."""
        instance = VkInstance()
        assert instance._instance is not None

    def test_enumerate_physical_devices(self):
        """vk_enumerate_physical_devices() returns devices."""
        instance = VkInstance()
        devices = instance.vk_enumerate_physical_devices()
        assert len(devices) >= 1
        assert all(isinstance(d, VkPhysicalDevice) for d in devices)

    def test_create_device(self):
        """vk_create_device() creates a logical device."""
        instance = VkInstance()
        pdevs = instance.vk_enumerate_physical_devices()
        device = instance.vk_create_device(pdevs[0])
        assert isinstance(device, VkDevice)


class TestVkPhysicalDevice:
    """Test physical device queries."""

    def test_device_properties(self):
        """vk_get_physical_device_properties() returns a dict."""
        instance = VkInstance()
        pdev = instance.vk_enumerate_physical_devices()[0]
        props = pdev.vk_get_physical_device_properties()
        assert "device_name" in props
        assert "device_type" in props
        assert "vendor" in props

    def test_memory_properties(self):
        """vk_get_physical_device_memory_properties() returns heap info."""
        instance = VkInstance()
        pdev = instance.vk_enumerate_physical_devices()[0]
        mem = pdev.vk_get_physical_device_memory_properties()
        assert "heap_count" in mem
        assert "heaps" in mem
        assert mem["heap_count"] >= 1

    def test_queue_family_properties(self):
        """vk_get_physical_device_queue_family_properties() returns families."""
        instance = VkInstance()
        pdev = instance.vk_enumerate_physical_devices()[0]
        families = pdev.vk_get_physical_device_queue_family_properties()
        assert len(families) >= 1
        assert "queue_type" in families[0]
        assert "queue_count" in families[0]


class TestVkDevice:
    """Test logical device operations."""

    def _make_device(self):
        instance = VkInstance()
        pdevs = instance.vk_enumerate_physical_devices()
        return instance.vk_create_device(pdevs[0])

    def test_get_device_queue(self):
        """vk_get_device_queue() returns a VkQueue."""
        device = self._make_device()
        queue = device.vk_get_device_queue(0, 0)
        assert isinstance(queue, VkQueue)

    def test_create_command_pool(self):
        """vk_create_command_pool() returns a VkCommandPool."""
        device = self._make_device()
        pool = device.vk_create_command_pool(VkCommandPoolCreateInfo(0))
        assert isinstance(pool, VkCommandPool)

    def test_allocate_memory(self):
        """vk_allocate_memory() returns VkDeviceMemory."""
        device = self._make_device()
        mem = device.vk_allocate_memory(VkMemoryAllocateInfo(size=256))
        assert isinstance(mem, VkDeviceMemory)

    def test_allocate_memory_host_visible(self):
        """vk_allocate_memory() with memory_type_index=1 (host-visible)."""
        device = self._make_device()
        mem = device.vk_allocate_memory(
            VkMemoryAllocateInfo(size=128, memory_type_index=1)
        )
        assert isinstance(mem, VkDeviceMemory)

    def test_create_buffer(self):
        """vk_create_buffer() returns VkBuffer."""
        device = self._make_device()
        buf = device.vk_create_buffer(VkBufferCreateInfo(size=64))
        assert isinstance(buf, VkBuffer)
        assert buf.size == 64

    def test_bind_buffer_memory(self):
        """vk_bind_buffer_memory() succeeds (no-op in simulator)."""
        device = self._make_device()
        buf = device.vk_create_buffer(VkBufferCreateInfo(size=64))
        mem = device.vk_allocate_memory(VkMemoryAllocateInfo(size=64))
        device.vk_bind_buffer_memory(buf, mem, 0)

    def test_map_unmap_memory(self):
        """vk_map_memory() returns data, vk_unmap_memory() succeeds."""
        device = self._make_device()
        mem = device.vk_allocate_memory(VkMemoryAllocateInfo(size=32))
        data = device.vk_map_memory(mem, 0, 32)
        assert isinstance(data, bytearray)
        assert len(data) == 32
        device.vk_unmap_memory(mem)

    def test_create_shader_module(self, simple_instructions):
        """vk_create_shader_module() returns VkShaderModule."""
        device = self._make_device()
        sm = device.vk_create_shader_module(
            VkShaderModuleCreateInfo(code=simple_instructions)
        )
        assert isinstance(sm, VkShaderModule)

    def test_create_descriptor_set_layout(self):
        """vk_create_descriptor_set_layout() returns a layout."""
        device = self._make_device()
        layout = device.vk_create_descriptor_set_layout(
            VkDescriptorSetLayoutCreateInfo(
                bindings=[VkDescriptorSetLayoutBinding(binding=0)]
            )
        )
        assert isinstance(layout, VkDescriptorSetLayout)

    def test_create_pipeline_layout(self):
        """vk_create_pipeline_layout() returns a layout."""
        device = self._make_device()
        ds_layout = device.vk_create_descriptor_set_layout(
            VkDescriptorSetLayoutCreateInfo()
        )
        pl_layout = device.vk_create_pipeline_layout(
            VkPipelineLayoutCreateInfo(set_layouts=[ds_layout])
        )
        assert isinstance(pl_layout, VkPipelineLayout)

    def test_create_compute_pipelines(self, simple_instructions):
        """vk_create_compute_pipelines() returns pipelines."""
        device = self._make_device()
        sm = device.vk_create_shader_module(
            VkShaderModuleCreateInfo(code=simple_instructions)
        )
        ds_layout = device.vk_create_descriptor_set_layout(
            VkDescriptorSetLayoutCreateInfo()
        )
        pl_layout = device.vk_create_pipeline_layout(
            VkPipelineLayoutCreateInfo(set_layouts=[ds_layout])
        )
        pipelines = device.vk_create_compute_pipelines([
            VkComputePipelineCreateInfo(
                shader_stage=VkPipelineShaderStageCreateInfo(module=sm),
                layout=pl_layout,
            )
        ])
        assert len(pipelines) == 1
        assert isinstance(pipelines[0], VkPipeline)

    def test_allocate_descriptor_sets(self):
        """vk_allocate_descriptor_sets() returns sets."""
        device = self._make_device()
        ds_layout = device.vk_create_descriptor_set_layout(
            VkDescriptorSetLayoutCreateInfo(
                bindings=[VkDescriptorSetLayoutBinding(binding=0)]
            )
        )
        sets = device.vk_allocate_descriptor_sets(
            VkDescriptorSetAllocateInfo(set_layouts=[ds_layout])
        )
        assert len(sets) == 1
        assert isinstance(sets[0], VkDescriptorSet)

    def test_update_descriptor_sets(self):
        """vk_update_descriptor_sets() writes buffer bindings."""
        device = self._make_device()
        ds_layout = device.vk_create_descriptor_set_layout(
            VkDescriptorSetLayoutCreateInfo(
                bindings=[VkDescriptorSetLayoutBinding(binding=0)]
            )
        )
        sets = device.vk_allocate_descriptor_sets(
            VkDescriptorSetAllocateInfo(set_layouts=[ds_layout])
        )
        buf = device.vk_create_buffer(VkBufferCreateInfo(size=64))
        device.vk_update_descriptor_sets([
            VkWriteDescriptorSet(
                dst_set=sets[0],
                dst_binding=0,
                buffer_info=VkDescriptorBufferInfo(buffer=buf),
            )
        ])

    def test_create_fence(self):
        """vk_create_fence() returns VkFence."""
        device = self._make_device()
        fence = device.vk_create_fence()
        assert isinstance(fence, VkFence)
        assert not fence.signaled

    def test_create_fence_signaled(self):
        """vk_create_fence(flags=1) returns signaled fence."""
        device = self._make_device()
        fence = device.vk_create_fence(flags=1)
        assert fence.signaled

    def test_create_semaphore(self):
        """vk_create_semaphore() returns VkSemaphore."""
        device = self._make_device()
        sem = device.vk_create_semaphore()
        assert isinstance(sem, VkSemaphore)

    def test_wait_for_fences(self):
        """vk_wait_for_fences() with signaled fences returns SUCCESS."""
        device = self._make_device()
        fence = device.vk_create_fence(flags=1)
        result = device.vk_wait_for_fences([fence], wait_all=True, timeout=1000)
        assert result == VkResult.SUCCESS

    def test_wait_for_fences_not_ready(self):
        """vk_wait_for_fences() with unsignaled returns NOT_READY."""
        device = self._make_device()
        fence = device.vk_create_fence()
        result = device.vk_wait_for_fences([fence], wait_all=True, timeout=0)
        assert result == VkResult.NOT_READY

    def test_wait_for_fences_any(self):
        """vk_wait_for_fences() with wait_all=False, one signaled."""
        device = self._make_device()
        f1 = device.vk_create_fence()
        f2 = device.vk_create_fence(flags=1)
        result = device.vk_wait_for_fences([f1, f2], wait_all=False, timeout=0)
        assert result == VkResult.SUCCESS

    def test_reset_fences(self):
        """vk_reset_fences() clears signaled state."""
        device = self._make_device()
        fence = device.vk_create_fence(flags=1)
        assert fence.signaled
        device.vk_reset_fences([fence])
        assert not fence.signaled

    def test_device_wait_idle(self):
        """vk_device_wait_idle() completes without error."""
        device = self._make_device()
        device.vk_device_wait_idle()


class TestVkCommandPoolAndBuffer:
    """Test command pool and buffer operations."""

    def _make_device_and_pool(self):
        instance = VkInstance()
        pdev = instance.vk_enumerate_physical_devices()[0]
        device = instance.vk_create_device(pdev)
        pool = device.vk_create_command_pool(VkCommandPoolCreateInfo(0))
        return device, pool

    def test_allocate_command_buffers(self):
        """vk_allocate_command_buffers() returns CBs."""
        device, pool = self._make_device_and_pool()
        cbs = pool.vk_allocate_command_buffers(2)
        assert len(cbs) == 2
        assert all(isinstance(cb, VkCommandBuffer) for cb in cbs)

    def test_reset_command_pool(self):
        """vk_reset_command_pool() resets all CBs."""
        device, pool = self._make_device_and_pool()
        cbs = pool.vk_allocate_command_buffers(1)
        cbs[0].vk_begin_command_buffer()
        pool.vk_reset_command_pool()

    def test_free_command_buffers(self):
        """vk_free_command_buffers() removes CBs from pool."""
        device, pool = self._make_device_and_pool()
        cbs = pool.vk_allocate_command_buffers(2)
        pool.vk_free_command_buffers([cbs[0]])
        assert len(pool._command_buffers) == 1

    def test_command_buffer_begin_end(self):
        """Begin and end recording a command buffer."""
        device, pool = self._make_device_and_pool()
        cbs = pool.vk_allocate_command_buffers(1)
        cbs[0].vk_begin_command_buffer()
        cbs[0].vk_end_command_buffer()


class TestVkQueueSubmit:
    """Test queue submission."""

    def _setup(self, instructions):
        instance = VkInstance()
        pdev = instance.vk_enumerate_physical_devices()[0]
        device = instance.vk_create_device(pdev)
        queue = device.vk_get_device_queue(0, 0)
        pool = device.vk_create_command_pool(VkCommandPoolCreateInfo(0))
        return device, queue, pool

    def test_submit_empty(self, simple_instructions):
        """Submit a minimal command buffer."""
        device, queue, pool = self._setup(simple_instructions)
        sm = device.vk_create_shader_module(
            VkShaderModuleCreateInfo(code=simple_instructions)
        )
        ds_layout = device.vk_create_descriptor_set_layout(
            VkDescriptorSetLayoutCreateInfo()
        )
        pl_layout = device.vk_create_pipeline_layout(
            VkPipelineLayoutCreateInfo(set_layouts=[ds_layout])
        )
        pipelines = device.vk_create_compute_pipelines([
            VkComputePipelineCreateInfo(
                shader_stage=VkPipelineShaderStageCreateInfo(module=sm),
                layout=pl_layout,
            )
        ])
        cbs = pool.vk_allocate_command_buffers(1)
        cbs[0].vk_begin_command_buffer()
        cbs[0].vk_cmd_bind_pipeline(VkPipelineBindPoint.COMPUTE, pipelines[0])
        cbs[0].vk_cmd_dispatch(1, 1, 1)
        cbs[0].vk_end_command_buffer()

        fence = device.vk_create_fence()
        result = queue.vk_queue_submit(
            [VkSubmitInfo(command_buffers=cbs)], fence=fence
        )
        assert result == VkResult.SUCCESS
        assert fence.signaled

    def test_cmd_copy_buffer(self):
        """vk_cmd_copy_buffer records a copy command."""
        instance = VkInstance()
        pdev = instance.vk_enumerate_physical_devices()[0]
        device = instance.vk_create_device(pdev)
        queue = device.vk_get_device_queue(0, 0)
        pool = device.vk_create_command_pool(VkCommandPoolCreateInfo(0))

        src = device.vk_create_buffer(VkBufferCreateInfo(size=16))
        dst = device.vk_create_buffer(VkBufferCreateInfo(size=16))

        cbs = pool.vk_allocate_command_buffers(1)
        cbs[0].vk_begin_command_buffer()
        cbs[0].vk_cmd_copy_buffer(src, dst, [VkBufferCopy(size=16)])
        cbs[0].vk_end_command_buffer()

        fence = device.vk_create_fence()
        queue.vk_queue_submit(
            [VkSubmitInfo(command_buffers=cbs)], fence=fence
        )
        assert fence.signaled

    def test_cmd_fill_buffer(self):
        """vk_cmd_fill_buffer records a fill command."""
        instance = VkInstance()
        pdev = instance.vk_enumerate_physical_devices()[0]
        device = instance.vk_create_device(pdev)
        pool = device.vk_create_command_pool(VkCommandPoolCreateInfo(0))
        queue = device.vk_get_device_queue(0, 0)

        buf = device.vk_create_buffer(VkBufferCreateInfo(size=16))
        cbs = pool.vk_allocate_command_buffers(1)
        cbs[0].vk_begin_command_buffer()
        cbs[0].vk_cmd_fill_buffer(buf, 0, 16, 0xFF)
        cbs[0].vk_end_command_buffer()

        fence = device.vk_create_fence()
        queue.vk_queue_submit(
            [VkSubmitInfo(command_buffers=cbs)], fence=fence
        )

    def test_cmd_push_constants(self, simple_instructions):
        """vk_cmd_push_constants records push constant data."""
        instance = VkInstance()
        pdev = instance.vk_enumerate_physical_devices()[0]
        device = instance.vk_create_device(pdev)
        pool = device.vk_create_command_pool(VkCommandPoolCreateInfo(0))

        ds_layout = device.vk_create_descriptor_set_layout(
            VkDescriptorSetLayoutCreateInfo()
        )
        pl_layout = device.vk_create_pipeline_layout(
            VkPipelineLayoutCreateInfo(set_layouts=[ds_layout])
        )

        cbs = pool.vk_allocate_command_buffers(1)
        cbs[0].vk_begin_command_buffer()
        cbs[0].vk_cmd_push_constants(pl_layout, 0, b"\x00\x00\x80\x3f")
        cbs[0].vk_end_command_buffer()

    def test_cmd_pipeline_barrier(self, simple_instructions):
        """vk_cmd_pipeline_barrier inserts a barrier."""
        instance = VkInstance()
        pdev = instance.vk_enumerate_physical_devices()[0]
        device = instance.vk_create_device(pdev)
        pool = device.vk_create_command_pool(VkCommandPoolCreateInfo(0))

        cbs = pool.vk_allocate_command_buffers(1)
        cbs[0].vk_begin_command_buffer()
        cbs[0].vk_cmd_pipeline_barrier("compute", "compute")
        cbs[0].vk_end_command_buffer()

    def test_queue_wait_idle(self):
        """vk_queue_wait_idle() completes without error."""
        instance = VkInstance()
        pdev = instance.vk_enumerate_physical_devices()[0]
        device = instance.vk_create_device(pdev)
        queue = device.vk_get_device_queue(0, 0)
        queue.vk_queue_wait_idle()

    def test_cmd_bind_descriptor_sets(self, simple_instructions):
        """vk_cmd_bind_descriptor_sets records descriptor binding."""
        instance = VkInstance()
        pdev = instance.vk_enumerate_physical_devices()[0]
        device = instance.vk_create_device(pdev)
        pool = device.vk_create_command_pool(VkCommandPoolCreateInfo(0))

        ds_layout = device.vk_create_descriptor_set_layout(
            VkDescriptorSetLayoutCreateInfo(
                bindings=[VkDescriptorSetLayoutBinding(binding=0)]
            )
        )
        pl_layout = device.vk_create_pipeline_layout(
            VkPipelineLayoutCreateInfo(set_layouts=[ds_layout])
        )
        sets = device.vk_allocate_descriptor_sets(
            VkDescriptorSetAllocateInfo(set_layouts=[ds_layout])
        )

        cbs = pool.vk_allocate_command_buffers(1)
        cbs[0].vk_begin_command_buffer()
        cbs[0].vk_cmd_bind_descriptor_sets(
            VkPipelineBindPoint.COMPUTE, pl_layout, sets
        )
        cbs[0].vk_end_command_buffer()

    def test_unmap_already_unmapped(self):
        """vk_unmap_memory on already unmapped memory is safe."""
        instance = VkInstance()
        pdev = instance.vk_enumerate_physical_devices()[0]
        device = instance.vk_create_device(pdev)
        mem = device.vk_allocate_memory(VkMemoryAllocateInfo(size=32))
        # Don't map it, just try to unmap (should be a no-op)
        device.vk_unmap_memory(mem)

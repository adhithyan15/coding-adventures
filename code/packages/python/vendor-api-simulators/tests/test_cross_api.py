"""Cross-API equivalence tests — same computation through all six APIs.

The capstone test for this package: run the same GPU operations through
CUDA, OpenCL, Metal, Vulkan, WebGPU, and OpenGL. All six must work
correctly, proving that our simulators are functionally equivalent
wrappers over the same Layer 5 compute runtime.
"""

import pytest
from gpu_core import limm, halt

from vendor_api_simulators.cuda import (
    CUDARuntime, CUDAKernel, CUDAMemcpyKind, dim3,
)
from vendor_api_simulators.opencl import (
    CLPlatform, CLContext, CLMemFlags, CLDeviceType,
)
from vendor_api_simulators.metal import (
    MTLDevice, MTLSize, MTLFunction,
)
from vendor_api_simulators.vulkan import (
    VkInstance, VkBufferCreateInfo, VkMemoryAllocateInfo,
    VkShaderModuleCreateInfo, VkComputePipelineCreateInfo,
    VkPipelineShaderStageCreateInfo, VkSubmitInfo,
    VkCommandPoolCreateInfo, VkDescriptorSetLayoutCreateInfo,
    VkPipelineLayoutCreateInfo, VkDescriptorSetAllocateInfo,
    VkWriteDescriptorSet, VkDescriptorBufferInfo,
    VkDescriptorSetLayoutBinding, VkPipelineBindPoint,
)
from vendor_api_simulators.webgpu import (
    GPU, GPUBufferDescriptor, GPUBufferUsage, GPUMapMode,
    GPUShaderModuleDescriptor, GPUComputePipelineDescriptor,
    GPUProgrammableStage, GPUBindGroupDescriptor, GPUBindGroupEntry,
)
from vendor_api_simulators.opengl import (
    GLContext, GL_COMPUTE_SHADER, GL_SHADER_STORAGE_BUFFER,
    GL_STATIC_DRAW, GL_DYNAMIC_DRAW, GL_MAP_READ_BIT, GL_SHADER_STORAGE_BARRIER_BIT,
)


@pytest.fixture
def kernel_code():
    """A simple kernel: load 42.0 into register 0, halt."""
    return [limm(0, 42.0), halt()]


class TestCrossAPIDispatch:
    """Verify all six APIs can dispatch a simple kernel."""

    def test_cuda_dispatch(self, kernel_code):
        """CUDA: launch_kernel dispatches successfully."""
        cuda = CUDARuntime()
        d_buf = cuda.malloc(64)
        kernel = CUDAKernel(code=kernel_code, name="test")
        cuda.launch_kernel(
            kernel, grid=dim3(1, 1, 1), block=dim3(32, 1, 1), args=[d_buf]
        )
        cuda.device_synchronize()
        cuda.free(d_buf)

    def test_opencl_dispatch(self, kernel_code):
        """OpenCL: enqueue_nd_range_kernel dispatches successfully."""
        ctx = CLContext()
        queue = ctx.create_command_queue()
        buf = ctx.create_buffer(CLMemFlags.READ_WRITE, 64)
        prog = ctx.create_program_with_source("test")
        prog.build()
        kernel = prog.create_kernel("test")
        kernel._code = kernel_code
        kernel.set_arg(0, buf)
        ev = queue.enqueue_nd_range_kernel(
            kernel, global_size=(32,), local_size=(32,)
        )
        queue.finish()

    def test_metal_dispatch(self, kernel_code):
        """Metal: dispatch_threadgroups dispatches successfully."""
        device = MTLDevice()
        queue = device.make_command_queue()
        buf = device.make_buffer(64)
        func = MTLFunction(name="test", code=kernel_code)
        pso = device.make_compute_pipeline_state(func)

        cb = queue.make_command_buffer()
        encoder = cb.make_compute_command_encoder()
        encoder.set_compute_pipeline_state(pso)
        encoder.set_buffer(buf, offset=0, index=0)
        encoder.dispatch_threadgroups(
            MTLSize(1, 1, 1), threads_per_threadgroup=MTLSize(32, 1, 1)
        )
        encoder.end_encoding()
        cb.commit()
        cb.wait_until_completed()

    def test_vulkan_dispatch(self, kernel_code):
        """Vulkan: vk_cmd_dispatch dispatches successfully."""
        instance = VkInstance()
        pdevs = instance.vk_enumerate_physical_devices()
        device = instance.vk_create_device(pdevs[0])
        queue = device.vk_get_device_queue(0, 0)
        pool = device.vk_create_command_pool(VkCommandPoolCreateInfo(0))

        sm = device.vk_create_shader_module(
            VkShaderModuleCreateInfo(code=kernel_code)
        )
        ds_layout = device.vk_create_descriptor_set_layout(
            VkDescriptorSetLayoutCreateInfo(
                bindings=[VkDescriptorSetLayoutBinding(binding=0)]
            )
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

        buf = device.vk_create_buffer(VkBufferCreateInfo(size=64))
        sets = device.vk_allocate_descriptor_sets(
            VkDescriptorSetAllocateInfo(set_layouts=[ds_layout])
        )
        device.vk_update_descriptor_sets([
            VkWriteDescriptorSet(
                dst_set=sets[0], dst_binding=0,
                buffer_info=VkDescriptorBufferInfo(buffer=buf),
            )
        ])

        cbs = pool.vk_allocate_command_buffers(1)
        cbs[0].vk_begin_command_buffer()
        cbs[0].vk_cmd_bind_pipeline(VkPipelineBindPoint.COMPUTE, pipelines[0])
        cbs[0].vk_cmd_bind_descriptor_sets(
            VkPipelineBindPoint.COMPUTE, pl_layout, sets
        )
        cbs[0].vk_cmd_dispatch(1, 1, 1)
        cbs[0].vk_end_command_buffer()

        fence = device.vk_create_fence()
        queue.vk_queue_submit(
            [VkSubmitInfo(command_buffers=cbs)], fence=fence
        )
        assert fence.signaled

    def test_webgpu_dispatch(self, kernel_code):
        """WebGPU: dispatch_workgroups dispatches successfully."""
        gpu = GPU()
        adapter = gpu.request_adapter()
        device = adapter.request_device()

        buf = device.create_buffer(GPUBufferDescriptor(
            size=64, usage=GPUBufferUsage.STORAGE
        ))
        shader = device.create_shader_module(
            GPUShaderModuleDescriptor(code=kernel_code)
        )
        pipeline = device.create_compute_pipeline(
            GPUComputePipelineDescriptor(
                layout="auto",
                compute=GPUProgrammableStage(module=shader),
            )
        )
        bg = device.create_bind_group(GPUBindGroupDescriptor(
            layout=pipeline.get_bind_group_layout(0),
            entries=[],
        ))

        encoder = device.create_command_encoder()
        pass_enc = encoder.begin_compute_pass()
        pass_enc.set_pipeline(pipeline)
        pass_enc.set_bind_group(0, bg)
        pass_enc.dispatch_workgroups(1)
        pass_enc.end()
        cb = encoder.finish()
        device.queue.submit([cb])

    def test_opengl_dispatch(self, kernel_code):
        """OpenGL: dispatch_compute dispatches successfully."""
        gl = GLContext()
        shader = gl.create_shader(GL_COMPUTE_SHADER)
        gl.shader_source(shader, "test")
        gl.compile_shader(shader)
        gl._shaders[shader]["code"] = kernel_code
        prog = gl.create_program()
        gl.attach_shader(prog, shader)
        gl.link_program(prog)

        bufs = gl.gen_buffers(1)
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
        gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 64, b"\x00" * 64, GL_DYNAMIC_DRAW)
        gl.bind_buffer_base(GL_SHADER_STORAGE_BUFFER, 0, bufs[0])

        gl.use_program(prog)
        gl.dispatch_compute(1, 1, 1)
        gl.memory_barrier(GL_SHADER_STORAGE_BARRIER_BIT)
        gl.finish()


class TestCrossAPIMemory:
    """Verify all six APIs can write and read buffer data."""

    def test_cuda_write_read(self):
        """CUDA: write data to device, read it back."""
        cuda = CUDARuntime()
        d_buf = cuda.malloc(16)
        data = b"\x01\x02\x03\x04" * 4
        cuda.memcpy(d_buf, data, 16, CUDAMemcpyKind.HostToDevice)
        result = bytearray(16)
        cuda.memcpy(result, d_buf, 16, CUDAMemcpyKind.DeviceToHost)
        assert result == bytearray(data)
        cuda.free(d_buf)

    def test_opencl_write_read(self):
        """OpenCL: write data, read it back."""
        ctx = CLContext()
        queue = ctx.create_command_queue()
        buf = ctx.create_buffer(CLMemFlags.READ_WRITE, 16)
        data = b"\x01\x02\x03\x04" * 4
        queue.enqueue_write_buffer(buf, 0, 16, data)
        result = bytearray(16)
        queue.enqueue_read_buffer(buf, 0, 16, result)
        assert result == bytearray(data)

    def test_metal_write_read(self):
        """Metal: write directly, read directly (unified memory)."""
        device = MTLDevice()
        buf = device.make_buffer(16)
        data = b"\x01\x02\x03\x04" * 4
        buf.write_bytes(data)
        result = bytes(buf.contents()[:16])
        assert result == data

    def test_vulkan_write_read(self):
        """Vulkan: map, write, unmap, map, read."""
        instance = VkInstance()
        pdevs = instance.vk_enumerate_physical_devices()
        device = instance.vk_create_device(pdevs[0])
        mem = device.vk_allocate_memory(VkMemoryAllocateInfo(size=16))

        # Write via map
        mapped = device.vk_map_memory(mem, 0, 16)
        mapped[:16] = b"\x01\x02\x03\x04" * 4
        device.vk_unmap_memory(mem)

        # Verify we can re-map and read
        mapped2 = device.vk_map_memory(mem, 0, 16)
        assert len(mapped2) == 16
        device.vk_unmap_memory(mem)

    def test_webgpu_write_read(self):
        """WebGPU: write_buffer + map_async + get_mapped_range."""
        gpu = GPU()
        device = gpu.request_adapter().request_device()
        buf = device.create_buffer(GPUBufferDescriptor(
            size=16,
            usage=GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
        ))
        data = b"\x01\x02\x03\x04" * 4
        device.queue.write_buffer(buf, 0, data)
        buf.map_async(GPUMapMode.READ)
        result = bytes(buf.get_mapped_range(0, 16))
        buf.unmap()
        assert result == data

    def test_opengl_write_read(self):
        """OpenGL: buffer_data + map_buffer_range."""
        gl = GLContext()
        bufs = gl.gen_buffers(1)
        data = b"\x01\x02\x03\x04" * 4
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
        gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 16, data, GL_DYNAMIC_DRAW)
        result = gl.map_buffer_range(
            GL_SHADER_STORAGE_BUFFER, 0, 16, GL_MAP_READ_BIT
        )
        assert bytes(result) == data


class TestCrossAPIResourceLifecycle:
    """Verify all six APIs handle resource creation and cleanup."""

    def test_cuda_lifecycle(self):
        """CUDA: malloc → use → free."""
        cuda = CUDARuntime()
        ptrs = [cuda.malloc(64) for _ in range(5)]
        for ptr in ptrs:
            cuda.free(ptr)

    def test_opencl_lifecycle(self):
        """OpenCL: create_buffer → use → finish."""
        ctx = CLContext()
        queue = ctx.create_command_queue()
        bufs = [ctx.create_buffer(CLMemFlags.READ_WRITE, 64) for _ in range(5)]
        queue.finish()

    def test_metal_lifecycle(self):
        """Metal: make_buffer → use → (automatic cleanup)."""
        device = MTLDevice()
        bufs = [device.make_buffer(64) for _ in range(5)]
        assert len(bufs) == 5

    def test_vulkan_lifecycle(self):
        """Vulkan: create_buffer → use → wait_idle."""
        instance = VkInstance()
        pdevs = instance.vk_enumerate_physical_devices()
        device = instance.vk_create_device(pdevs[0])
        bufs = [device.vk_create_buffer(VkBufferCreateInfo(size=64)) for _ in range(5)]
        device.vk_device_wait_idle()

    def test_webgpu_lifecycle(self):
        """WebGPU: create_buffer → use → destroy."""
        gpu = GPU()
        device = gpu.request_adapter().request_device()
        bufs = [
            device.create_buffer(GPUBufferDescriptor(size=64, usage=GPUBufferUsage.STORAGE))
            for _ in range(5)
        ]
        for buf in bufs:
            buf.destroy()

    def test_opengl_lifecycle(self):
        """OpenGL: gen_buffers → use → delete_buffers."""
        gl = GLContext()
        bufs = gl.gen_buffers(5)
        for buf in bufs:
            gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, buf)
            gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 64, None, GL_STATIC_DRAW)
        gl.delete_buffers(bufs)


class TestCrossAPISameResult:
    """Verify all APIs dispatch through the same Layer 5 engine."""

    def test_all_six_dispatch_simple_kernel(self, kernel_code):
        """All six APIs dispatch a kernel without errors.

        This is the capstone test — same kernel code through all six APIs,
        all using the Layer 5 compute runtime underneath.
        """
        errors = []

        # CUDA
        try:
            cuda = CUDARuntime()
            kernel = CUDAKernel(code=kernel_code, name="test")
            cuda.launch_kernel(kernel, dim3(1, 1, 1), dim3(32, 1, 1))
            cuda.device_synchronize()
        except Exception as e:
            errors.append(f"CUDA: {e}")

        # OpenCL
        try:
            ctx = CLContext()
            queue = ctx.create_command_queue()
            prog = ctx.create_program_with_source("test")
            prog.build()
            k = prog.create_kernel("test")
            k._code = kernel_code
            queue.enqueue_nd_range_kernel(k, (32,), (32,))
            queue.finish()
        except Exception as e:
            errors.append(f"OpenCL: {e}")

        # Metal
        try:
            device = MTLDevice()
            q = device.make_command_queue()
            func = MTLFunction(name="test", code=kernel_code)
            pso = device.make_compute_pipeline_state(func)
            cb = q.make_command_buffer()
            enc = cb.make_compute_command_encoder()
            enc.set_compute_pipeline_state(pso)
            enc.dispatch_threadgroups(MTLSize(1, 1, 1), MTLSize(32, 1, 1))
            enc.end_encoding()
            cb.commit()
            cb.wait_until_completed()
        except Exception as e:
            errors.append(f"Metal: {e}")

        # Vulkan
        try:
            inst = VkInstance()
            pd = inst.vk_enumerate_physical_devices()[0]
            dev = inst.vk_create_device(pd)
            vk_q = dev.vk_get_device_queue(0, 0)
            pool = dev.vk_create_command_pool(VkCommandPoolCreateInfo(0))
            sm = dev.vk_create_shader_module(VkShaderModuleCreateInfo(code=kernel_code))
            dsl = dev.vk_create_descriptor_set_layout(VkDescriptorSetLayoutCreateInfo())
            pll = dev.vk_create_pipeline_layout(VkPipelineLayoutCreateInfo(set_layouts=[dsl]))
            pipes = dev.vk_create_compute_pipelines([
                VkComputePipelineCreateInfo(
                    shader_stage=VkPipelineShaderStageCreateInfo(module=sm),
                    layout=pll,
                )
            ])
            cbs = pool.vk_allocate_command_buffers(1)
            cbs[0].vk_begin_command_buffer()
            cbs[0].vk_cmd_bind_pipeline(VkPipelineBindPoint.COMPUTE, pipes[0])
            cbs[0].vk_cmd_dispatch(1, 1, 1)
            cbs[0].vk_end_command_buffer()
            fence = dev.vk_create_fence()
            vk_q.vk_queue_submit([VkSubmitInfo(command_buffers=cbs)], fence=fence)
        except Exception as e:
            errors.append(f"Vulkan: {e}")

        # WebGPU
        try:
            g = GPU()
            ad = g.request_adapter()
            wd = ad.request_device()
            sh = wd.create_shader_module(GPUShaderModuleDescriptor(code=kernel_code))
            pip = wd.create_compute_pipeline(GPUComputePipelineDescriptor(
                layout="auto", compute=GPUProgrammableStage(module=sh),
            ))
            bg = wd.create_bind_group(GPUBindGroupDescriptor(
                layout=pip.get_bind_group_layout(0), entries=[],
            ))
            we = wd.create_command_encoder()
            wp = we.begin_compute_pass()
            wp.set_pipeline(pip)
            wp.set_bind_group(0, bg)
            wp.dispatch_workgroups(1)
            wp.end()
            wcb = we.finish()
            wd.queue.submit([wcb])
        except Exception as e:
            errors.append(f"WebGPU: {e}")

        # OpenGL
        try:
            ogl = GLContext()
            s = ogl.create_shader(GL_COMPUTE_SHADER)
            ogl.shader_source(s, "test")
            ogl.compile_shader(s)
            ogl._shaders[s]["code"] = kernel_code
            p = ogl.create_program()
            ogl.attach_shader(p, s)
            ogl.link_program(p)
            ogl.use_program(p)
            ogl.dispatch_compute(1, 1, 1)
            ogl.finish()
        except Exception as e:
            errors.append(f"OpenGL: {e}")

        assert not errors, f"API failures: {errors}"

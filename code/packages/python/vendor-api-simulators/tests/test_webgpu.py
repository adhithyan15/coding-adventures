"""Tests for the WebGPU runtime simulator."""

import pytest
from gpu_core import limm, halt

from vendor_api_simulators.webgpu import (
    GPU,
    GPUAdapter,
    GPUDevice,
    GPUQueue,
    GPUCommandEncoder,
    GPUComputePassEncoder,
    GPUCommandBuffer,
    GPUBuffer,
    GPUShaderModule,
    GPUComputePipeline,
    GPUBindGroup,
    GPUBindGroupLayout,
    GPUBufferUsage,
    GPUMapMode,
    GPUBufferDescriptor,
    GPUShaderModuleDescriptor,
    GPUComputePipelineDescriptor,
    GPUProgrammableStage,
    GPUBindGroupDescriptor,
    GPUBindGroupEntry,
    GPUBindGroupLayoutDescriptor,
    GPUBindGroupLayoutEntry,
    GPUPipelineLayoutDescriptor,
    GPURequestAdapterOptions,
    GPUDeviceDescriptor,
    GPUComputePassDescriptor,
    GPUCommandEncoderDescriptor,
)


class TestGPUAndAdapter:
    """Test GPU entry point and adapter selection."""

    def test_create_gpu(self):
        """GPU() initializes without error."""
        gpu = GPU()
        assert len(gpu._physical_devices) > 0

    def test_request_adapter(self):
        """request_adapter() returns a GPUAdapter."""
        gpu = GPU()
        adapter = gpu.request_adapter()
        assert isinstance(adapter, GPUAdapter)
        assert len(adapter.name) > 0

    def test_request_adapter_with_options(self):
        """request_adapter() with options."""
        gpu = GPU()
        adapter = gpu.request_adapter(
            GPURequestAdapterOptions(power_preference="high-performance")
        )
        assert isinstance(adapter, GPUAdapter)

    def test_request_adapter_low_power(self):
        """request_adapter() with low-power preference."""
        gpu = GPU()
        adapter = gpu.request_adapter(
            GPURequestAdapterOptions(power_preference="low-power")
        )
        assert isinstance(adapter, GPUAdapter)

    def test_adapter_features(self):
        """Adapter has features set."""
        gpu = GPU()
        adapter = gpu.request_adapter()
        assert "compute" in adapter.features

    def test_adapter_limits(self):
        """Adapter has limits."""
        gpu = GPU()
        adapter = gpu.request_adapter()
        assert adapter.limits.max_buffer_size > 0


class TestGPUDevice:
    """Test device creation and resource factories."""

    def _make_device(self):
        gpu = GPU()
        adapter = gpu.request_adapter()
        return adapter.request_device()

    def test_request_device(self):
        """request_device() returns a GPUDevice."""
        device = self._make_device()
        assert isinstance(device, GPUDevice)

    def test_request_device_with_descriptor(self):
        """request_device() with descriptor."""
        gpu = GPU()
        adapter = gpu.request_adapter()
        device = adapter.request_device(GPUDeviceDescriptor())
        assert isinstance(device, GPUDevice)

    def test_device_has_queue(self):
        """Device has a queue property."""
        device = self._make_device()
        assert isinstance(device.queue, GPUQueue)

    def test_device_features(self):
        """Device has features."""
        device = self._make_device()
        assert "compute" in device.features

    def test_create_buffer(self):
        """create_buffer() returns a GPUBuffer."""
        device = self._make_device()
        buf = device.create_buffer(GPUBufferDescriptor(
            size=256, usage=GPUBufferUsage.STORAGE
        ))
        assert isinstance(buf, GPUBuffer)
        assert buf.size == 256

    def test_create_buffer_mapped(self):
        """create_buffer() with mapped_at_creation=True."""
        device = self._make_device()
        buf = device.create_buffer(GPUBufferDescriptor(
            size=64,
            usage=GPUBufferUsage.STORAGE | GPUBufferUsage.MAP_WRITE,
            mapped_at_creation=True,
        ))
        assert isinstance(buf, GPUBuffer)

    def test_create_shader_module(self, simple_instructions):
        """create_shader_module() returns a GPUShaderModule."""
        device = self._make_device()
        shader = device.create_shader_module(
            GPUShaderModuleDescriptor(code=simple_instructions)
        )
        assert isinstance(shader, GPUShaderModule)

    def test_create_compute_pipeline(self, simple_instructions):
        """create_compute_pipeline() returns a GPUComputePipeline."""
        device = self._make_device()
        shader = device.create_shader_module(
            GPUShaderModuleDescriptor(code=simple_instructions)
        )
        pipeline = device.create_compute_pipeline(
            GPUComputePipelineDescriptor(
                layout="auto",
                compute=GPUProgrammableStage(module=shader, entry_point="main"),
            )
        )
        assert isinstance(pipeline, GPUComputePipeline)

    def test_pipeline_get_bind_group_layout(self, simple_instructions):
        """get_bind_group_layout() returns a layout."""
        device = self._make_device()
        shader = device.create_shader_module(
            GPUShaderModuleDescriptor(code=simple_instructions)
        )
        pipeline = device.create_compute_pipeline(
            GPUComputePipelineDescriptor(
                layout="auto",
                compute=GPUProgrammableStage(module=shader),
            )
        )
        layout = pipeline.get_bind_group_layout(0)
        assert isinstance(layout, GPUBindGroupLayout)

    def test_pipeline_get_bind_group_layout_invalid(self, simple_instructions):
        """get_bind_group_layout() with invalid index raises."""
        device = self._make_device()
        shader = device.create_shader_module(
            GPUShaderModuleDescriptor(code=simple_instructions)
        )
        pipeline = device.create_compute_pipeline(
            GPUComputePipelineDescriptor(
                layout="auto",
                compute=GPUProgrammableStage(module=shader),
            )
        )
        with pytest.raises(IndexError):
            pipeline.get_bind_group_layout(99)

    def test_create_bind_group(self):
        """create_bind_group() returns a GPUBindGroup."""
        device = self._make_device()
        buf = device.create_buffer(GPUBufferDescriptor(
            size=64, usage=GPUBufferUsage.STORAGE
        ))
        bg_layout = device.create_bind_group_layout(
            GPUBindGroupLayoutDescriptor(
                entries=[GPUBindGroupLayoutEntry(binding=0)]
            )
        )
        bg = device.create_bind_group(GPUBindGroupDescriptor(
            layout=bg_layout,
            entries=[GPUBindGroupEntry(binding=0, resource=buf)],
        ))
        assert isinstance(bg, GPUBindGroup)

    def test_create_bind_group_layout(self):
        """create_bind_group_layout() returns a GPUBindGroupLayout."""
        device = self._make_device()
        layout = device.create_bind_group_layout(
            GPUBindGroupLayoutDescriptor(
                entries=[GPUBindGroupLayoutEntry(binding=0)]
            )
        )
        assert isinstance(layout, GPUBindGroupLayout)

    def test_create_pipeline_layout(self):
        """create_pipeline_layout() returns a GPUPipelineLayout."""
        device = self._make_device()
        bg_layout = device.create_bind_group_layout(
            GPUBindGroupLayoutDescriptor()
        )
        pl_layout = device.create_pipeline_layout(
            GPUPipelineLayoutDescriptor(bind_group_layouts=[bg_layout])
        )
        assert pl_layout is not None

    def test_create_command_encoder(self):
        """create_command_encoder() returns a GPUCommandEncoder."""
        device = self._make_device()
        encoder = device.create_command_encoder()
        assert isinstance(encoder, GPUCommandEncoder)

    def test_create_command_encoder_with_descriptor(self):
        """create_command_encoder() with descriptor."""
        device = self._make_device()
        encoder = device.create_command_encoder(
            GPUCommandEncoderDescriptor(label="test")
        )
        assert isinstance(encoder, GPUCommandEncoder)

    def test_destroy(self):
        """destroy() completes without error."""
        device = self._make_device()
        device.destroy()


class TestGPUBuffer:
    """Test buffer mapping and access."""

    def _make_device(self):
        gpu = GPU()
        return gpu.request_adapter().request_device()

    def test_buffer_size(self):
        """Buffer has correct size."""
        device = self._make_device()
        buf = device.create_buffer(GPUBufferDescriptor(size=128))
        assert buf.size == 128

    def test_buffer_usage(self):
        """Buffer has correct usage flags."""
        device = self._make_device()
        buf = device.create_buffer(GPUBufferDescriptor(
            size=64, usage=GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST
        ))
        assert GPUBufferUsage.STORAGE in buf.usage

    def test_map_async_and_get_mapped_range(self):
        """map_async() + get_mapped_range() returns data."""
        device = self._make_device()
        buf = device.create_buffer(GPUBufferDescriptor(
            size=16, usage=GPUBufferUsage.MAP_READ | GPUBufferUsage.STORAGE
        ))
        buf.map_async(GPUMapMode.READ)
        data = buf.get_mapped_range()
        assert isinstance(data, bytearray)
        assert len(data) == 16

    def test_get_mapped_range_without_map(self):
        """get_mapped_range() without map_async raises."""
        device = self._make_device()
        buf = device.create_buffer(GPUBufferDescriptor(size=16))
        with pytest.raises(RuntimeError, match="not mapped"):
            buf.get_mapped_range()

    def test_unmap(self):
        """unmap() succeeds after mapping."""
        device = self._make_device()
        buf = device.create_buffer(GPUBufferDescriptor(size=16))
        buf.map_async(GPUMapMode.WRITE)
        buf.unmap()

    def test_unmap_without_map(self):
        """unmap() without mapping raises."""
        device = self._make_device()
        buf = device.create_buffer(GPUBufferDescriptor(size=16))
        with pytest.raises(RuntimeError, match="not mapped"):
            buf.unmap()

    def test_destroy_buffer(self):
        """destroy() frees the buffer."""
        device = self._make_device()
        buf = device.create_buffer(GPUBufferDescriptor(size=16))
        buf.destroy()
        assert buf._destroyed

    def test_map_destroyed_buffer(self):
        """map_async() on destroyed buffer raises."""
        device = self._make_device()
        buf = device.create_buffer(GPUBufferDescriptor(size=16))
        buf.destroy()
        with pytest.raises(RuntimeError, match="destroyed"):
            buf.map_async(GPUMapMode.READ)


class TestGPUQueue:
    """Test queue operations."""

    def _make_device(self):
        gpu = GPU()
        return gpu.request_adapter().request_device()

    def test_write_buffer(self):
        """queue.write_buffer() writes data."""
        device = self._make_device()
        buf = device.create_buffer(GPUBufferDescriptor(
            size=16, usage=GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST
        ))
        device.queue.write_buffer(buf, 0, b"\xAA\xBB\xCC\xDD" * 4)

    def test_submit_command_buffer(self, simple_instructions):
        """queue.submit() executes command buffers."""
        device = self._make_device()
        shader = device.create_shader_module(
            GPUShaderModuleDescriptor(code=simple_instructions)
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


class TestGPUCommandEncoder:
    """Test command encoding."""

    def _make_device(self):
        gpu = GPU()
        return gpu.request_adapter().request_device()

    def test_begin_compute_pass(self):
        """begin_compute_pass() returns a GPUComputePassEncoder."""
        device = self._make_device()
        encoder = device.create_command_encoder()
        pass_enc = encoder.begin_compute_pass()
        assert isinstance(pass_enc, GPUComputePassEncoder)

    def test_begin_compute_pass_with_descriptor(self):
        """begin_compute_pass() with descriptor."""
        device = self._make_device()
        encoder = device.create_command_encoder()
        pass_enc = encoder.begin_compute_pass(
            GPUComputePassDescriptor(label="test_pass")
        )
        assert isinstance(pass_enc, GPUComputePassEncoder)

    def test_copy_buffer_to_buffer(self):
        """copy_buffer_to_buffer() records a copy command."""
        device = self._make_device()
        src = device.create_buffer(GPUBufferDescriptor(
            size=16, usage=GPUBufferUsage.COPY_SRC | GPUBufferUsage.STORAGE
        ))
        dst = device.create_buffer(GPUBufferDescriptor(
            size=16, usage=GPUBufferUsage.COPY_DST | GPUBufferUsage.STORAGE
        ))
        encoder = device.create_command_encoder()
        encoder.copy_buffer_to_buffer(src, 0, dst, 0, 16)
        cb = encoder.finish()
        device.queue.submit([cb])

    def test_finish_returns_command_buffer(self):
        """finish() returns a GPUCommandBuffer."""
        device = self._make_device()
        encoder = device.create_command_encoder()
        cb = encoder.finish()
        assert isinstance(cb, GPUCommandBuffer)


class TestGPUComputePassEncoder:
    """Test compute pass encoding."""

    def _make_device(self):
        gpu = GPU()
        return gpu.request_adapter().request_device()

    def test_dispatch_without_pipeline(self):
        """dispatch without setting pipeline raises."""
        device = self._make_device()
        encoder = device.create_command_encoder()
        pass_enc = encoder.begin_compute_pass()
        with pytest.raises(RuntimeError, match="No pipeline"):
            pass_enc.dispatch_workgroups(1)

    def test_dispatch_workgroups(self, simple_instructions):
        """dispatch_workgroups() records a dispatch."""
        device = self._make_device()
        shader = device.create_shader_module(
            GPUShaderModuleDescriptor(code=simple_instructions)
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
        pass_enc.dispatch_workgroups(4, 2, 1)
        pass_enc.end()
        cb = encoder.finish()
        device.queue.submit([cb])

    def test_end_compute_pass(self):
        """end() completes without error."""
        device = self._make_device()
        encoder = device.create_command_encoder()
        pass_enc = encoder.begin_compute_pass()
        pass_enc.end()


class TestGPUFullPipeline:
    """End-to-end WebGPU workflow tests."""

    def test_full_workflow(self, simple_instructions):
        """Full pipeline: GPU → adapter → device → encoder → submit."""
        gpu = GPU()
        adapter = gpu.request_adapter()
        device = adapter.request_device()

        buf = device.create_buffer(GPUBufferDescriptor(
            size=64,
            usage=GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
        ))
        device.queue.write_buffer(buf, 0, b"\x00" * 64)

        shader = device.create_shader_module(
            GPUShaderModuleDescriptor(code=simple_instructions)
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
        compute_pass = encoder.begin_compute_pass()
        compute_pass.set_pipeline(pipeline)
        compute_pass.set_bind_group(0, bg)
        compute_pass.dispatch_workgroups(1)
        compute_pass.end()
        cb = encoder.finish()

        device.queue.submit([cb])

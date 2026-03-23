"""Tests for ValidationLayer — error detection."""

import pytest

from compute_runtime import (
    RuntimeInstance,
    CommandBuffer,
    CommandBufferState,
    MemoryType,
    BufferUsage,
    DescriptorBinding,
    ValidationError,
    ValidationLayer,
)
from compute_runtime.memory import Buffer
from compute_runtime.pipeline import (
    DescriptorSet,
    DescriptorSetLayout,
    Pipeline,
    PipelineLayout,
    ShaderModule,
)
from gpu_core import limm, halt


def make_device():
    instance = RuntimeInstance()
    physical = instance.enumerate_physical_devices()[0]
    return instance.create_logical_device(physical)


class TestCommandBufferValidation:
    def test_validate_begin_initial(self) -> None:
        vl = ValidationLayer()
        cb = CommandBuffer()
        vl.validate_begin(cb)  # Should not raise

    def test_validate_begin_recording_fails(self) -> None:
        vl = ValidationLayer()
        cb = CommandBuffer()
        cb.begin()
        with pytest.raises(ValidationError, match="recording"):
            vl.validate_begin(cb)

    def test_validate_end_recording(self) -> None:
        vl = ValidationLayer()
        cb = CommandBuffer()
        cb.begin()
        vl.validate_end(cb)  # Should not raise

    def test_validate_end_initial_fails(self) -> None:
        vl = ValidationLayer()
        cb = CommandBuffer()
        with pytest.raises(ValidationError, match="initial"):
            vl.validate_end(cb)

    def test_validate_submit_recorded(self) -> None:
        vl = ValidationLayer()
        cb = CommandBuffer()
        cb.begin()
        cb.end()
        vl.validate_submit(cb)  # Should not raise

    def test_validate_submit_initial_fails(self) -> None:
        vl = ValidationLayer()
        cb = CommandBuffer()
        with pytest.raises(ValidationError, match="initial"):
            vl.validate_submit(cb)


class TestDispatchValidation:
    def test_dispatch_without_pipeline(self) -> None:
        vl = ValidationLayer()
        cb = CommandBuffer()
        cb.begin()
        with pytest.raises(ValidationError, match="no pipeline"):
            vl.validate_dispatch(cb, 1, 1, 1)

    def test_dispatch_negative_dims(self) -> None:
        vl = ValidationLayer()
        device = make_device()
        shader = device.create_shader_module(code=[halt()])
        layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        cb = CommandBuffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)

        with pytest.raises(ValidationError, match="positive"):
            vl.validate_dispatch(cb, -1, 1, 1)

    def test_dispatch_zero_dims(self) -> None:
        vl = ValidationLayer()
        device = make_device()
        shader = device.create_shader_module(code=[halt()])
        layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        cb = CommandBuffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)

        with pytest.raises(ValidationError, match="positive"):
            vl.validate_dispatch(cb, 0, 1, 1)

    def test_dispatch_valid(self) -> None:
        vl = ValidationLayer()
        device = make_device()
        shader = device.create_shader_module(code=[halt()])
        layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        cb = CommandBuffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        vl.validate_dispatch(cb, 4, 2, 1)  # Should not raise


class TestMemoryValidation:
    def test_map_host_visible(self) -> None:
        vl = ValidationLayer()
        buf = Buffer(
            buffer_id=0, size=64,
            memory_type=MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
            usage=BufferUsage.STORAGE,
        )
        vl.validate_map(buf)  # Should not raise

    def test_map_device_local_fails(self) -> None:
        vl = ValidationLayer()
        buf = Buffer(
            buffer_id=0, size=64,
            memory_type=MemoryType.DEVICE_LOCAL,
            usage=BufferUsage.STORAGE,
        )
        with pytest.raises(ValidationError, match="HOST_VISIBLE"):
            vl.validate_map(buf)

    def test_map_freed_fails(self) -> None:
        vl = ValidationLayer()
        buf = Buffer(
            buffer_id=0, size=64,
            memory_type=MemoryType.HOST_VISIBLE,
            usage=BufferUsage.STORAGE,
            freed=True,
        )
        with pytest.raises(ValidationError, match="freed"):
            vl.validate_map(buf)

    def test_map_already_mapped_fails(self) -> None:
        vl = ValidationLayer()
        buf = Buffer(
            buffer_id=0, size=64,
            memory_type=MemoryType.HOST_VISIBLE,
            usage=BufferUsage.STORAGE,
            mapped=True,
        )
        with pytest.raises(ValidationError, match="already mapped"):
            vl.validate_map(buf)

    def test_buffer_usage_validation(self) -> None:
        vl = ValidationLayer()
        buf = Buffer(
            buffer_id=0, size=64,
            memory_type=MemoryType.DEVICE_LOCAL,
            usage=BufferUsage.STORAGE,
        )
        vl.validate_buffer_usage(buf, BufferUsage.STORAGE)  # OK

    def test_buffer_usage_missing(self) -> None:
        vl = ValidationLayer()
        buf = Buffer(
            buffer_id=0, size=64,
            memory_type=MemoryType.DEVICE_LOCAL,
            usage=BufferUsage.STORAGE,
        )
        with pytest.raises(ValidationError, match="lacks required"):
            vl.validate_buffer_usage(buf, BufferUsage.TRANSFER_SRC)

    def test_buffer_not_freed(self) -> None:
        vl = ValidationLayer()
        buf = Buffer(
            buffer_id=0, size=64,
            memory_type=MemoryType.DEVICE_LOCAL,
            usage=BufferUsage.STORAGE,
            freed=True,
        )
        with pytest.raises(ValidationError, match="freed"):
            vl.validate_buffer_not_freed(buf)


class TestBarrierValidation:
    def test_write_without_barrier_warns(self) -> None:
        vl = ValidationLayer()
        vl.record_write(42)
        vl.validate_read_after_write(42)
        assert len(vl.warnings) == 1
        assert "barrier" in vl.warnings[0].lower()

    def test_write_with_barrier_ok(self) -> None:
        vl = ValidationLayer()
        vl.record_write(42)
        vl.record_barrier()  # Global barrier
        vl.validate_read_after_write(42)
        assert len(vl.warnings) == 0

    def test_unwritten_buffer_no_warning(self) -> None:
        vl = ValidationLayer()
        vl.validate_read_after_write(99)
        assert len(vl.warnings) == 0

    def test_barrier_specific_buffer(self) -> None:
        vl = ValidationLayer()
        vl.record_write(10)
        vl.record_write(20)
        vl.record_barrier(buffer_ids={10})

        vl.validate_read_after_write(10)  # OK, barriered
        assert len(vl.warnings) == 0

        vl.validate_read_after_write(20)  # Not barriered
        assert len(vl.warnings) == 1

    def test_clear(self) -> None:
        vl = ValidationLayer()
        vl.record_write(1)
        vl.validate_read_after_write(1)
        assert len(vl.warnings) == 1
        vl.clear()
        assert len(vl.warnings) == 0
        assert len(vl.errors) == 0


class TestDescriptorSetValidation:
    def test_valid_descriptor_set(self) -> None:
        vl = ValidationLayer()
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(64, MemoryType.DEVICE_LOCAL, usage=BufferUsage.STORAGE)

        ds_layout = device.create_descriptor_set_layout([
            DescriptorBinding(binding=0, type="storage"),
        ])
        desc_set = device.create_descriptor_set(ds_layout)
        desc_set.write(0, buf)

        shader = device.create_shader_module(code=[halt()])
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        vl.validate_descriptor_set(desc_set, pipeline)
        assert len(vl.warnings) == 0

    def test_missing_binding_warns(self) -> None:
        vl = ValidationLayer()
        device = make_device()

        ds_layout = device.create_descriptor_set_layout([
            DescriptorBinding(binding=0, type="storage"),
        ])
        desc_set = device.create_descriptor_set(ds_layout)
        # Don't write binding 0

        shader = device.create_shader_module(code=[halt()])
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        vl.validate_descriptor_set(desc_set, pipeline)
        assert len(vl.warnings) == 1
        assert "not set" in vl.warnings[0]

    def test_freed_buffer_in_descriptor(self) -> None:
        vl = ValidationLayer()
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(64, MemoryType.DEVICE_LOCAL, usage=BufferUsage.STORAGE)

        ds_layout = device.create_descriptor_set_layout([
            DescriptorBinding(binding=0, type="storage"),
        ])
        desc_set = device.create_descriptor_set(ds_layout)
        desc_set.write(0, buf)

        # Free the buffer after binding
        mm.free(buf)

        shader = device.create_shader_module(code=[halt()])
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        with pytest.raises(ValidationError, match="freed"):
            vl.validate_descriptor_set(desc_set, pipeline)

"""End-to-end execution tests — full pipeline from allocation to results."""

import struct
import pytest

from gpu_core import limm, halt

from compute_runtime import (
    RuntimeInstance,
    MemoryType,
    BufferUsage,
    PipelineBarrier,
    PipelineStage,
    MemoryBarrier,
    AccessFlags,
    DescriptorBinding,
    RuntimeEventType,
    DeviceType,
)


def make_device(vendor: str = "nvidia"):
    instance = RuntimeInstance()
    physical = next(
        d for d in instance.enumerate_physical_devices()
        if d.vendor == vendor
    )
    return instance.create_logical_device(physical)


class TestGPUExecution:
    """End-to-end tests on GPU-style devices."""

    def test_simple_dispatch(self) -> None:
        """Dispatch a minimal kernel and verify it completes."""
        device = make_device("nvidia")
        queue = device.queues["compute"][0]
        mm = device.memory_manager

        shader = device.create_shader_module(
            code=[limm(0, 42.0), halt()],
            local_size=(32, 1, 1),
        )
        ds_layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch(1, 1, 1)
        cb.end()

        fence = device.create_fence()
        queue.submit([cb], fence=fence)
        assert fence.signaled
        assert fence.wait()

    def test_dispatch_with_barrier(self) -> None:
        """Dispatch → barrier → dispatch."""
        device = make_device("nvidia")
        queue = device.queues["compute"][0]

        shader = device.create_shader_module(
            code=[limm(0, 1.0), halt()],
            local_size=(32, 1, 1),
        )
        ds_layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch(1, 1, 1)
        cb.cmd_pipeline_barrier(PipelineBarrier(
            src_stage=PipelineStage.COMPUTE,
            dst_stage=PipelineStage.COMPUTE,
            memory_barriers=(
                MemoryBarrier(AccessFlags.SHADER_WRITE, AccessFlags.SHADER_READ),
            ),
        ))
        cb.cmd_dispatch(1, 1, 1)
        cb.end()

        fence = device.create_fence()
        queue.submit([cb], fence=fence)
        assert fence.signaled
        assert device.stats.total_dispatches == 2
        assert device.stats.total_barriers == 1

    def test_upload_and_dispatch(self) -> None:
        """Upload data via staging buffer, then dispatch."""
        device = make_device("nvidia")
        queue = device.queues["compute"][0]
        mm = device.memory_manager

        # Allocate staging and device buffers
        staging = mm.allocate(
            64,
            MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
            usage=BufferUsage.TRANSFER_SRC,
        )
        device_buf = mm.allocate(
            64,
            MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
            usage=BufferUsage.STORAGE | BufferUsage.TRANSFER_DST,
        )

        # Write to staging
        mapped = mm.map(staging)
        mapped.write(0, b"\x42" * 64)
        mm.unmap(staging)

        # Upload + dispatch
        shader = device.create_shader_module(
            code=[limm(0, 1.0), halt()],
            local_size=(32, 1, 1),
        )
        ds_layout = device.create_descriptor_set_layout([
            DescriptorBinding(binding=0, type="storage"),
        ])
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        desc_set = device.create_descriptor_set(ds_layout)
        desc_set.write(0, device_buf)

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_copy_buffer(staging, device_buf, 64)
        cb.cmd_pipeline_barrier(PipelineBarrier(
            src_stage=PipelineStage.TRANSFER,
            dst_stage=PipelineStage.COMPUTE,
            memory_barriers=(
                MemoryBarrier(AccessFlags.TRANSFER_WRITE, AccessFlags.SHADER_READ),
            ),
        ))
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_bind_descriptor_set(desc_set)
        cb.cmd_dispatch(1, 1, 1)
        cb.end()

        fence = device.create_fence()
        queue.submit([cb], fence=fence)
        assert fence.signaled

    def test_all_gpu_devices(self) -> None:
        """Every GPU device type should complete a basic dispatch."""
        for vendor in ["nvidia", "amd", "intel"]:
            device = make_device(vendor)
            queue = device.queues["compute"][0]

            shader = device.create_shader_module(
                code=[limm(0, 42.0), halt()],
                local_size=(32, 1, 1),
            )
            ds_layout = device.create_descriptor_set_layout([])
            pl_layout = device.create_pipeline_layout([ds_layout])
            pipeline = device.create_compute_pipeline(shader, pl_layout)

            cb = device.create_command_buffer()
            cb.begin()
            cb.cmd_bind_pipeline(pipeline)
            cb.cmd_dispatch(1, 1, 1)
            cb.end()

            fence = device.create_fence()
            queue.submit([cb], fence=fence)
            assert fence.signaled, f"{vendor} dispatch should complete"


class TestDataflowExecution:
    """End-to-end tests on dataflow-style devices (TPU, ANE)."""

    def test_tpu_dispatch(self) -> None:
        device = make_device("google")
        queue = device.queues["compute"][0]

        shader = device.create_shader_module(operation="matmul")
        ds_layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch(1, 1, 1)
        cb.end()

        fence = device.create_fence()
        queue.submit([cb], fence=fence)
        assert fence.signaled

    def test_ane_dispatch(self) -> None:
        device = make_device("apple")
        queue = device.queues["compute"][0]

        shader = device.create_shader_module(operation="matmul")
        ds_layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch(1, 1, 1)
        cb.end()

        fence = device.create_fence()
        queue.submit([cb], fence=fence)
        assert fence.signaled


class TestUnifiedMemory:
    """Tests specific to Apple's unified memory architecture."""

    def test_zero_copy_pattern(self) -> None:
        """Apple: no staging buffer needed — write directly."""
        device = make_device("apple")
        queue = device.queues["compute"][0]
        mm = device.memory_manager

        # On unified memory, DEVICE_LOCAL + HOST_VISIBLE works
        buf = mm.allocate(
            64,
            MemoryType.DEVICE_LOCAL | MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
            usage=BufferUsage.STORAGE,
        )

        # Write directly — no staging buffer!
        mapped = mm.map(buf)
        mapped.write(0, b"\x42" * 64)
        mm.unmap(buf)

        # Dispatch
        shader = device.create_shader_module(operation="matmul")
        ds_layout = device.create_descriptor_set_layout([
            DescriptorBinding(binding=0, type="storage"),
        ])
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)
        desc_set = device.create_descriptor_set(ds_layout)
        desc_set.write(0, buf)

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_bind_descriptor_set(desc_set)
        cb.cmd_dispatch(1, 1, 1)
        cb.end()

        fence = device.create_fence()
        queue.submit([cb], fence=fence)
        assert fence.signaled


class TestCommandBufferReuse:
    """Test recording once, submitting twice."""

    def test_reuse_after_completion(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]

        shader = device.create_shader_module(
            code=[limm(0, 1.0), halt()],
            local_size=(32, 1, 1),
        )
        ds_layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        cb = device.create_command_buffer()

        # First use
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch(1, 1, 1)
        cb.end()
        fence1 = device.create_fence()
        queue.submit([cb], fence=fence1)
        assert fence1.signaled

        # Reset and reuse
        cb.reset()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch(2, 1, 1)
        cb.end()
        fence2 = device.create_fence()
        queue.submit([cb], fence=fence2)
        assert fence2.signaled

        assert device.stats.total_dispatches == 2


class TestMultiSubmit:
    """Test submitting multiple command buffers together."""

    def test_sequential_command_buffers(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]

        shader = device.create_shader_module(
            code=[limm(0, 1.0), halt()],
            local_size=(32, 1, 1),
        )
        ds_layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        # Create 3 CBs
        cbs = []
        for _ in range(3):
            cb = device.create_command_buffer()
            cb.begin()
            cb.cmd_bind_pipeline(pipeline)
            cb.cmd_dispatch(1, 1, 1)
            cb.end()
            cbs.append(cb)

        fence = device.create_fence()
        queue.submit(cbs, fence=fence)
        assert fence.signaled
        assert device.stats.total_dispatches == 3
        assert device.stats.total_command_buffers == 3


class TestRuntimeStats:
    def test_stats_accumulate(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]

        shader = device.create_shader_module(
            code=[limm(0, 1.0), halt()],
            local_size=(32, 1, 1),
        )
        ds_layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        for _ in range(5):
            cb = device.create_command_buffer()
            cb.begin()
            cb.cmd_bind_pipeline(pipeline)
            cb.cmd_dispatch(1, 1, 1)
            cb.end()
            queue.submit([cb])

        stats = device.stats
        assert stats.total_submissions == 5
        assert stats.total_dispatches == 5
        assert stats.total_device_cycles > 0

    def test_traces_collected(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]

        shader = device.create_shader_module(
            code=[limm(0, 1.0), halt()],
            local_size=(32, 1, 1),
        )
        ds_layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch(1, 1, 1)
        cb.end()

        queue.submit([cb])

        assert len(device.stats.traces) > 0

    def test_utilization_calculated(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]

        shader = device.create_shader_module(
            code=[limm(0, 1.0), halt()],
            local_size=(32, 1, 1),
        )
        ds_layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch(1, 1, 1)
        cb.end()

        queue.submit([cb])
        # After at least one dispatch, utilization should be calculated
        assert device.stats.total_device_cycles > 0

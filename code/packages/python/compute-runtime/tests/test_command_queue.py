"""Tests for CommandQueue — submission and execution."""

import pytest
from gpu_core import limm, halt

from compute_runtime import (
    RuntimeInstance,
    CommandBuffer,
    CommandBufferState,
    MemoryType,
    BufferUsage,
    RuntimeEventType,
    PipelineBarrier,
    PipelineStage,
    DescriptorBinding,
)


def make_device(vendor: str = "nvidia"):
    instance = RuntimeInstance()
    physical = next(
        d for d in instance.enumerate_physical_devices()
        if d.vendor == vendor
    )
    return instance.create_logical_device(physical)


def make_pipeline(device):
    shader = device.create_shader_module(code=[limm(0, 42.0), halt()])
    ds_layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([ds_layout])
    return device.create_compute_pipeline(shader, pl_layout)


class TestSubmit:
    def test_basic_submit(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]
        pipeline = make_pipeline(device)

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch(1, 1, 1)
        cb.end()

        fence = device.create_fence()
        traces = queue.submit([cb], fence=fence)

        assert fence.signaled
        assert cb.state == CommandBufferState.COMPLETE
        assert len(traces) > 0

    def test_submit_not_recorded_fails(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]

        cb = device.create_command_buffer()
        cb.begin()  # Still RECORDING, not RECORDED

        with pytest.raises(RuntimeError, match="recording"):
            queue.submit([cb])

    def test_fence_signaled_after_submit(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]
        pipeline = make_pipeline(device)

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch(1, 1, 1)
        cb.end()

        fence = device.create_fence()
        queue.submit([cb], fence=fence)
        assert fence.wait()

    def test_submit_without_fence(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]
        pipeline = make_pipeline(device)

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch(1, 1, 1)
        cb.end()

        traces = queue.submit([cb])  # No fence
        assert len(traces) > 0

    def test_multiple_command_buffers(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]
        pipeline = make_pipeline(device)

        cb1 = device.create_command_buffer()
        cb1.begin()
        cb1.cmd_bind_pipeline(pipeline)
        cb1.cmd_dispatch(1, 1, 1)
        cb1.end()

        cb2 = device.create_command_buffer()
        cb2.begin()
        cb2.cmd_bind_pipeline(pipeline)
        cb2.cmd_dispatch(2, 1, 1)
        cb2.end()

        fence = device.create_fence()
        traces = queue.submit([cb1, cb2], fence=fence)

        assert fence.signaled
        assert cb1.state == CommandBufferState.COMPLETE
        assert cb2.state == CommandBufferState.COMPLETE

    def test_stats_updated(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]
        pipeline = make_pipeline(device)

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch(1, 1, 1)
        cb.end()

        queue.submit([cb])

        assert device.stats.total_submissions == 1
        assert device.stats.total_command_buffers == 1
        assert device.stats.total_dispatches == 1


class TestSemaphores:
    def test_signal_semaphore(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]
        pipeline = make_pipeline(device)
        sem = device.create_semaphore()

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch(1, 1, 1)
        cb.end()

        queue.submit([cb], signal_semaphores=[sem])
        assert sem.signaled

    def test_wait_semaphore(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]
        pipeline = make_pipeline(device)
        sem = device.create_semaphore()

        # First submission signals semaphore
        cb1 = device.create_command_buffer()
        cb1.begin()
        cb1.cmd_bind_pipeline(pipeline)
        cb1.cmd_dispatch(1, 1, 1)
        cb1.end()
        queue.submit([cb1], signal_semaphores=[sem])

        # Second submission waits on semaphore
        cb2 = device.create_command_buffer()
        cb2.begin()
        cb2.cmd_bind_pipeline(pipeline)
        cb2.cmd_dispatch(1, 1, 1)
        cb2.end()
        traces = queue.submit([cb2], wait_semaphores=[sem])

        # Semaphore consumed (reset after wait)
        assert not sem.signaled

    def test_wait_unsignaled_fails(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]
        pipeline = make_pipeline(device)
        sem = device.create_semaphore()

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch(1, 1, 1)
        cb.end()

        with pytest.raises(RuntimeError, match="not signaled"):
            queue.submit([cb], wait_semaphores=[sem])


class TestTransferCommands:
    def test_copy_buffer(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]
        mm = device.memory_manager

        src = mm.allocate(
            64,
            MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
            usage=BufferUsage.TRANSFER_SRC,
        )
        dst = mm.allocate(
            64,
            MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
            usage=BufferUsage.TRANSFER_DST,
        )

        # Write data to src
        mapped = mm.map(src)
        mapped.write(0, b"\x42" * 64)
        mm.unmap(src)

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_copy_buffer(src, dst, 64)
        cb.end()

        queue.submit([cb])
        assert device.stats.total_transfers == 1

    def test_fill_buffer(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]
        mm = device.memory_manager

        buf = mm.allocate(
            64,
            MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
            usage=BufferUsage.TRANSFER_DST,
        )

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_fill_buffer(buf, 0xFF)
        cb.end()

        queue.submit([cb])
        assert device.stats.total_transfers == 1

    def test_update_buffer(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]
        mm = device.memory_manager

        buf = mm.allocate(
            64,
            MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
            usage=BufferUsage.TRANSFER_DST,
        )

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_update_buffer(buf, 0, b"\xAA" * 16)
        cb.end()

        queue.submit([cb])
        assert device.stats.total_transfers == 1


class TestBarriers:
    def test_barrier_recorded_in_stats(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_pipeline_barrier(PipelineBarrier(
            src_stage=PipelineStage.COMPUTE,
            dst_stage=PipelineStage.TRANSFER,
        ))
        cb.end()

        queue.submit([cb])
        assert device.stats.total_barriers == 1

    def test_barrier_produces_trace(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_pipeline_barrier(PipelineBarrier(
            src_stage=PipelineStage.COMPUTE,
            dst_stage=PipelineStage.TRANSFER,
        ))
        cb.end()

        traces = queue.submit([cb])
        barrier_traces = [
            t for t in traces
            if t.event_type == RuntimeEventType.BARRIER
        ]
        assert len(barrier_traces) == 1


class TestQueueProperties:
    def test_queue_type(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]
        from compute_runtime import QueueType
        assert queue.queue_type == QueueType.COMPUTE

    def test_wait_idle(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]
        queue.wait_idle()  # Should not raise


class TestTraces:
    def test_submit_produces_traces(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]
        pipeline = make_pipeline(device)

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch(1, 1, 1)
        cb.end()

        traces = queue.submit([cb])
        assert len(traces) > 0

        event_types = {t.event_type for t in traces}
        assert RuntimeEventType.SUBMIT in event_types
        assert RuntimeEventType.BEGIN_EXECUTION in event_types
        assert RuntimeEventType.END_EXECUTION in event_types

    def test_trace_format(self) -> None:
        device = make_device()
        queue = device.queues["compute"][0]
        pipeline = make_pipeline(device)

        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch(1, 1, 1)
        cb.end()

        traces = queue.submit([cb])
        for trace in traces:
            formatted = trace.format()
            assert isinstance(formatted, str)
            assert len(formatted) > 0

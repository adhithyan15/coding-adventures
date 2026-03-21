"""Tests for CommandBuffer — recording and state transitions."""

import pytest
from gpu_core import limm, halt

from compute_runtime import (
    RuntimeInstance,
    CommandBuffer,
    CommandBufferState,
    MemoryType,
    BufferUsage,
    PipelineBarrier,
    PipelineStage,
    MemoryBarrier,
    AccessFlags,
    DescriptorBinding,
)


def make_device():
    instance = RuntimeInstance()
    physical = instance.enumerate_physical_devices()[0]
    return instance.create_logical_device(physical)


class TestLifecycle:
    def test_initial_state(self) -> None:
        cb = CommandBuffer()
        assert cb.state == CommandBufferState.INITIAL

    def test_begin(self) -> None:
        cb = CommandBuffer()
        cb.begin()
        assert cb.state == CommandBufferState.RECORDING

    def test_end(self) -> None:
        cb = CommandBuffer()
        cb.begin()
        cb.end()
        assert cb.state == CommandBufferState.RECORDED

    def test_reset(self) -> None:
        cb = CommandBuffer()
        cb.begin()
        cb.end()
        cb.reset()
        assert cb.state == CommandBufferState.INITIAL

    def test_begin_from_wrong_state(self) -> None:
        cb = CommandBuffer()
        cb.begin()
        with pytest.raises(RuntimeError, match="recording"):
            cb.begin()

    def test_end_from_wrong_state(self) -> None:
        cb = CommandBuffer()
        with pytest.raises(RuntimeError, match="initial"):
            cb.end()

    def test_record_without_begin(self) -> None:
        device = make_device()
        shader = device.create_shader_module(code=[limm(0, 1.0), halt()])
        layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        cb = CommandBuffer()
        with pytest.raises(RuntimeError, match="initial"):
            cb.cmd_bind_pipeline(pipeline)

    def test_unique_ids(self) -> None:
        cb1 = CommandBuffer()
        cb2 = CommandBuffer()
        assert cb1.command_buffer_id != cb2.command_buffer_id

    def test_reuse_after_reset(self) -> None:
        device = make_device()
        shader = device.create_shader_module(code=[limm(0, 1.0), halt()])
        layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        cb = CommandBuffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.end()
        assert len(cb.commands) == 1

        cb.reset()
        assert len(cb.commands) == 0
        assert cb.state == CommandBufferState.INITIAL

        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_bind_pipeline(pipeline)
        cb.end()
        assert len(cb.commands) == 2


class TestComputeCommands:
    def test_bind_pipeline(self) -> None:
        device = make_device()
        shader = device.create_shader_module(code=[limm(0, 1.0), halt()])
        layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        cb = CommandBuffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.end()

        assert len(cb.commands) == 1
        assert cb.commands[0].command == "bind_pipeline"

    def test_bind_descriptor_set(self) -> None:
        device = make_device()
        layout = device.create_descriptor_set_layout([
            DescriptorBinding(binding=0, type="storage"),
        ])
        desc_set = device.create_descriptor_set(layout)

        cb = CommandBuffer()
        cb.begin()
        cb.cmd_bind_descriptor_set(desc_set)
        cb.end()

        assert len(cb.commands) == 1
        assert cb.commands[0].command == "bind_descriptor_set"

    def test_dispatch(self) -> None:
        device = make_device()
        shader = device.create_shader_module(code=[limm(0, 1.0), halt()])
        layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        cb = CommandBuffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch(4, 1, 1)
        cb.end()

        assert len(cb.commands) == 2
        dispatch_cmd = cb.commands[1]
        assert dispatch_cmd.command == "dispatch"
        assert dispatch_cmd.args["group_x"] == 4

    def test_dispatch_without_pipeline(self) -> None:
        cb = CommandBuffer()
        cb.begin()
        with pytest.raises(RuntimeError, match="no pipeline"):
            cb.cmd_dispatch(1, 1, 1)

    def test_push_constants(self) -> None:
        cb = CommandBuffer()
        cb.begin()
        cb.cmd_push_constants(0, b"\x00\x00\x80\x3f")  # 1.0f
        cb.end()

        assert cb.commands[0].command == "push_constants"
        assert cb.commands[0].args["size"] == 4

    def test_dispatch_indirect(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            12, MemoryType.DEVICE_LOCAL, usage=BufferUsage.INDIRECT
        )
        shader = device.create_shader_module(code=[limm(0, 1.0), halt()])
        layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        cb = CommandBuffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch_indirect(buf)
        cb.end()

        assert cb.commands[1].command == "dispatch_indirect"


class TestTransferCommands:
    def test_copy_buffer(self) -> None:
        device = make_device()
        mm = device.memory_manager
        src = mm.allocate(
            64, MemoryType.DEVICE_LOCAL, usage=BufferUsage.TRANSFER_SRC
        )
        dst = mm.allocate(
            64, MemoryType.DEVICE_LOCAL, usage=BufferUsage.TRANSFER_DST
        )

        cb = CommandBuffer()
        cb.begin()
        cb.cmd_copy_buffer(src, dst, 64)
        cb.end()

        assert cb.commands[0].command == "copy_buffer"
        assert cb.commands[0].args["size"] == 64

    def test_fill_buffer(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            64, MemoryType.DEVICE_LOCAL, usage=BufferUsage.TRANSFER_DST
        )

        cb = CommandBuffer()
        cb.begin()
        cb.cmd_fill_buffer(buf, 0)
        cb.end()

        assert cb.commands[0].command == "fill_buffer"
        assert cb.commands[0].args["value"] == 0

    def test_update_buffer(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            64, MemoryType.DEVICE_LOCAL, usage=BufferUsage.TRANSFER_DST
        )

        cb = CommandBuffer()
        cb.begin()
        cb.cmd_update_buffer(buf, 0, b"\x42" * 16)
        cb.end()

        assert cb.commands[0].command == "update_buffer"


class TestSyncCommands:
    def test_pipeline_barrier(self) -> None:
        cb = CommandBuffer()
        cb.begin()
        cb.cmd_pipeline_barrier(PipelineBarrier(
            src_stage=PipelineStage.COMPUTE,
            dst_stage=PipelineStage.TRANSFER,
            memory_barriers=(
                MemoryBarrier(
                    src_access=AccessFlags.SHADER_WRITE,
                    dst_access=AccessFlags.TRANSFER_READ,
                ),
            ),
        ))
        cb.end()

        assert cb.commands[0].command == "pipeline_barrier"
        assert cb.commands[0].args["memory_barrier_count"] == 1

    def test_set_event(self) -> None:
        device = make_device()
        event = device.create_event()

        cb = CommandBuffer()
        cb.begin()
        cb.cmd_set_event(event, PipelineStage.COMPUTE)
        cb.end()

        assert cb.commands[0].command == "set_event"

    def test_wait_event(self) -> None:
        device = make_device()
        event = device.create_event()

        cb = CommandBuffer()
        cb.begin()
        cb.cmd_wait_event(
            event, PipelineStage.COMPUTE, PipelineStage.COMPUTE
        )
        cb.end()

        assert cb.commands[0].command == "wait_event"

    def test_reset_event(self) -> None:
        device = make_device()
        event = device.create_event()

        cb = CommandBuffer()
        cb.begin()
        cb.cmd_reset_event(event, PipelineStage.COMPUTE)
        cb.end()

        assert cb.commands[0].command == "reset_event"


class TestCommandList:
    def test_multiple_commands(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            64,
            MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
            usage=BufferUsage.STORAGE | BufferUsage.TRANSFER_DST,
        )
        shader = device.create_shader_module(code=[limm(0, 1.0), halt()])
        layout = device.create_descriptor_set_layout([
            DescriptorBinding(binding=0, type="storage"),
        ])
        pl_layout = device.create_pipeline_layout([layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)
        desc_set = device.create_descriptor_set(layout)
        desc_set.write(0, buf)

        cb = CommandBuffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_bind_descriptor_set(desc_set)
        cb.cmd_dispatch(1, 1, 1)
        cb.cmd_pipeline_barrier(PipelineBarrier(
            src_stage=PipelineStage.COMPUTE,
            dst_stage=PipelineStage.TRANSFER,
        ))
        cb.cmd_fill_buffer(buf, 0)
        cb.end()

        commands = cb.commands
        assert len(commands) == 5
        assert commands[0].command == "bind_pipeline"
        assert commands[1].command == "bind_descriptor_set"
        assert commands[2].command == "dispatch"
        assert commands[3].command == "pipeline_barrier"
        assert commands[4].command == "fill_buffer"

"""CommandBuffer — recorded sequence of GPU commands.

=== The Record-Then-Submit Model ===

Instead of calling GPU operations one at a time (like CUDA), Vulkan records
commands into a buffer and submits the whole buffer at once. This is the
single most important concept in Vulkan:

    # CUDA style (implicit, one at a time):
    cudaMemcpy(dst, src, size)     # executes immediately
    kernel<<<grid, block>>>(args)  # executes immediately
    cudaMemcpy(host, dst, size)    # executes immediately

    # Vulkan style (explicit, batched):
    cb.begin()                     # start recording
    cb.cmd_copy_buffer(...)        # just records — doesn't execute
    cb.cmd_dispatch(...)           # just records — doesn't execute
    cb.cmd_copy_buffer(...)        # just records — doesn't execute
    cb.end()                       # stop recording

    queue.submit([cb])             # NOW everything executes

=== Why Batch? ===

1. **Driver optimization** — the driver sees all commands at once and can
   reorder, merge, or eliminate redundancies.

2. **Reuse** — submit the same CB multiple times without re-recording.
   Perfect for inference: same operations, different input data.

3. **Multi-threaded recording** — different CPU threads record different
   CBs in parallel, then submit them together. The recording is CPU-only
   (no GPU involvement), so it scales perfectly.

4. **Validation** — check the entire sequence for errors before any GPU
   work starts. Much easier to debug than mid-execution failures.

=== State Machine ===

A command buffer has strict states:

    INITIAL ──begin()──► RECORDING ──end()──► RECORDED ──submit()──► PENDING
       ▲                                                                │
       └──────────────────── reset() ◄── COMPLETE ◄─────────────────────┘
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from .protocols import (
    AccessFlags,
    CommandBufferState,
    PipelineBarrier,
    PipelineStage,
    RecordedCommand,
)

if TYPE_CHECKING:
    from .memory import Buffer
    from .pipeline import DescriptorSet, Pipeline
    from .sync import Event


class CommandBuffer:
    """A recorded sequence of GPU commands.

    === Command Types ===

    Compute commands:
        cmd_bind_pipeline     — select which kernel to run
        cmd_bind_descriptor_set — bind memory to kernel parameters
        cmd_push_constants    — small inline data (≤128 bytes)
        cmd_dispatch          — launch kernel with grid dimensions
        cmd_dispatch_indirect — read grid dimensions from a GPU buffer

    Transfer commands:
        cmd_copy_buffer       — device-to-device memory copy
        cmd_fill_buffer       — fill buffer with a constant value
        cmd_update_buffer     — write small data inline (CPU → GPU)

    Synchronization commands:
        cmd_pipeline_barrier  — execution + memory ordering
        cmd_set_event         — signal an event from GPU
        cmd_wait_event        — wait for event before proceeding
        cmd_reset_event       — reset event from GPU
    """

    _next_id: int = 0

    def __init__(self) -> None:
        self._id = CommandBuffer._next_id
        CommandBuffer._next_id += 1
        self._state = CommandBufferState.INITIAL
        self._commands: list[RecordedCommand] = []

        # Currently bound state (for validation)
        self._bound_pipeline: Pipeline | None = None
        self._bound_descriptor_set: DescriptorSet | None = None
        self._push_constants: bytes = b""

    @property
    def command_buffer_id(self) -> int:
        """Unique identifier."""
        return self._id

    @property
    def state(self) -> CommandBufferState:
        """Current lifecycle state."""
        return self._state

    @property
    def commands(self) -> list[RecordedCommand]:
        """All recorded commands."""
        return list(self._commands)

    @property
    def bound_pipeline(self) -> Pipeline | None:
        """Currently bound pipeline (for validation)."""
        return self._bound_pipeline

    @property
    def bound_descriptor_set(self) -> DescriptorSet | None:
        """Currently bound descriptor set (for validation)."""
        return self._bound_descriptor_set

    # =================================================================
    # Lifecycle
    # =================================================================

    def begin(self) -> None:
        """Start recording commands.

        Transitions: INITIAL → RECORDING, or COMPLETE → RECORDING (reuse).

        Raises:
            RuntimeError: If not in INITIAL or COMPLETE state.
        """
        if self._state not in (
            CommandBufferState.INITIAL,
            CommandBufferState.COMPLETE,
        ):
            raise RuntimeError(
                f"Cannot begin recording: state is {self._state.value} "
                f"(expected INITIAL or COMPLETE)"
            )
        self._state = CommandBufferState.RECORDING
        self._commands.clear()
        self._bound_pipeline = None
        self._bound_descriptor_set = None
        self._push_constants = b""

    def end(self) -> None:
        """Finish recording commands.

        Transitions: RECORDING → RECORDED.

        Raises:
            RuntimeError: If not in RECORDING state.
        """
        if self._state != CommandBufferState.RECORDING:
            raise RuntimeError(
                f"Cannot end recording: state is {self._state.value} "
                f"(expected RECORDING)"
            )
        self._state = CommandBufferState.RECORDED

    def reset(self) -> None:
        """Reset to INITIAL state for reuse.

        Clears all recorded commands and bound state.
        """
        self._state = CommandBufferState.INITIAL
        self._commands.clear()
        self._bound_pipeline = None
        self._bound_descriptor_set = None
        self._push_constants = b""

    def _mark_pending(self) -> None:
        """Internal: mark as submitted (called by CommandQueue)."""
        self._state = CommandBufferState.PENDING

    def _mark_complete(self) -> None:
        """Internal: mark as finished (called by CommandQueue)."""
        self._state = CommandBufferState.COMPLETE

    def _require_recording(self) -> None:
        """Internal: ensure we're in RECORDING state."""
        if self._state != CommandBufferState.RECORDING:
            raise RuntimeError(
                f"Cannot record command: state is {self._state.value} "
                f"(expected RECORDING)"
            )

    # =================================================================
    # Compute commands
    # =================================================================

    def cmd_bind_pipeline(self, pipeline: Pipeline) -> None:
        """Bind a compute pipeline for subsequent dispatches.

        Must be called before cmd_dispatch().

        Args:
            pipeline: The compiled pipeline to bind.
        """
        self._require_recording()
        self._bound_pipeline = pipeline
        self._commands.append(
            RecordedCommand("bind_pipeline", {"pipeline_id": pipeline.pipeline_id})
        )

    def cmd_bind_descriptor_set(self, descriptor_set: DescriptorSet) -> None:
        """Bind a descriptor set (buffer assignments) for subsequent dispatches.

        Must be called after cmd_bind_pipeline().

        Args:
            descriptor_set: The descriptor set with buffer bindings.
        """
        self._require_recording()
        self._bound_descriptor_set = descriptor_set
        self._commands.append(
            RecordedCommand(
                "bind_descriptor_set", {"set_id": descriptor_set.set_id}
            )
        )

    def cmd_push_constants(self, offset: int, data: bytes) -> None:
        """Set push constant data for the next dispatch.

        Push constants are small pieces of data (≤128 bytes) sent inline
        with the dispatch command. They're faster than buffer reads for
        small values like alpha in SAXPY.

        Args:
            offset: Byte offset into the push constant range.
            data:   The bytes to set.
        """
        self._require_recording()
        self._push_constants = data
        self._commands.append(
            RecordedCommand(
                "push_constants",
                {"offset": offset, "size": len(data)},
            )
        )

    def cmd_dispatch(
        self, group_x: int, group_y: int = 1, group_z: int = 1
    ) -> None:
        """Launch a compute kernel.

        === Dispatch Dimensions ===

        The dispatch creates a 3D grid of workgroups:

            Total threads = (group_x × group_y × group_z) ×
                           (local_x × local_y × local_z)

        Where local dimensions come from the shader module.

        For SAXPY with N=1024 elements, local_size=(256,1,1):
            cmd_dispatch(1024 // 256, 1, 1)  → 4 workgroups × 256 threads

        Args:
            group_x: Workgroups in X dimension.
            group_y: Workgroups in Y dimension.
            group_z: Workgroups in Z dimension.

        Raises:
            RuntimeError: If no pipeline is bound.
        """
        self._require_recording()
        if self._bound_pipeline is None:
            raise RuntimeError("Cannot dispatch: no pipeline bound")
        self._commands.append(
            RecordedCommand(
                "dispatch",
                {"group_x": group_x, "group_y": group_y, "group_z": group_z},
            )
        )

    def cmd_dispatch_indirect(self, buffer: Buffer, offset: int = 0) -> None:
        """Launch a compute kernel with grid dimensions from a GPU buffer.

        The buffer contains three uint32 values: (group_x, group_y, group_z).
        This is useful when the GPU itself determines how much work to do
        (e.g., stream compaction → dispatch only non-zero elements).

        Args:
            buffer: Buffer containing dispatch dimensions.
            offset: Byte offset into the buffer.
        """
        self._require_recording()
        if self._bound_pipeline is None:
            raise RuntimeError("Cannot dispatch: no pipeline bound")
        self._commands.append(
            RecordedCommand(
                "dispatch_indirect",
                {"buffer_id": buffer.buffer_id, "offset": offset},
            )
        )

    # =================================================================
    # Transfer commands
    # =================================================================

    def cmd_copy_buffer(
        self,
        src: Buffer,
        dst: Buffer,
        size: int,
        src_offset: int = 0,
        dst_offset: int = 0,
    ) -> None:
        """Copy data between device buffers.

        This is how you move data from a staging buffer to a device-local
        buffer, or between two device-local buffers.

        Args:
            src:        Source buffer.
            dst:        Destination buffer.
            size:       Bytes to copy.
            src_offset: Byte offset in source.
            dst_offset: Byte offset in destination.
        """
        self._require_recording()
        self._commands.append(
            RecordedCommand(
                "copy_buffer",
                {
                    "src_id": src.buffer_id,
                    "dst_id": dst.buffer_id,
                    "size": size,
                    "src_offset": src_offset,
                    "dst_offset": dst_offset,
                },
            )
        )

    def cmd_fill_buffer(
        self,
        buffer: Buffer,
        value: int,
        offset: int = 0,
        size: int = 0,
    ) -> None:
        """Fill a buffer with a constant byte value.

        Useful for zeroing buffers before use.

        Args:
            buffer: The buffer to fill.
            value:  Byte value to fill with (0-255).
            offset: Byte offset to start filling.
            size:   Bytes to fill (0 = whole buffer).
        """
        self._require_recording()
        self._commands.append(
            RecordedCommand(
                "fill_buffer",
                {
                    "buffer_id": buffer.buffer_id,
                    "value": value,
                    "offset": offset,
                    "size": size if size > 0 else buffer.size,
                },
            )
        )

    def cmd_update_buffer(
        self, buffer: Buffer, offset: int, data: bytes
    ) -> None:
        """Write small data inline from CPU to device buffer.

        Limited to small updates (≤ 65536 bytes). For large transfers,
        use a staging buffer + cmd_copy_buffer.

        Args:
            buffer: Destination buffer.
            offset: Byte offset in the buffer.
            data:   Bytes to write.
        """
        self._require_recording()
        self._commands.append(
            RecordedCommand(
                "update_buffer",
                {
                    "buffer_id": buffer.buffer_id,
                    "offset": offset,
                    "data": data,
                },
            )
        )

    # =================================================================
    # Synchronization commands
    # =================================================================

    def cmd_pipeline_barrier(self, barrier: PipelineBarrier) -> None:
        """Insert an execution + memory barrier.

        === When to Use ===

        After a kernel writes to a buffer and before another kernel reads it:

            cb.cmd_dispatch(...)  # writes to buffer X
            cb.cmd_pipeline_barrier(PipelineBarrier(
                src_stage=PipelineStage.COMPUTE,
                dst_stage=PipelineStage.COMPUTE,
                memory_barriers=(MemoryBarrier(SHADER_WRITE, SHADER_READ),),
            ))
            cb.cmd_dispatch(...)  # reads from buffer X

        Without the barrier, the second kernel might see stale data.

        Args:
            barrier: The barrier specification.
        """
        self._require_recording()
        self._commands.append(
            RecordedCommand(
                "pipeline_barrier",
                {
                    "src_stage": barrier.src_stage.value,
                    "dst_stage": barrier.dst_stage.value,
                    "memory_barrier_count": len(barrier.memory_barriers),
                    "buffer_barrier_count": len(barrier.buffer_barriers),
                },
            )
        )

    def cmd_set_event(self, event: Event, stage: PipelineStage) -> None:
        """Signal an event from the GPU.

        The event is signaled after all commands of the given stage
        complete (for commands recorded before this one).

        Args:
            event: The event to signal.
            stage: Wait for this stage before signaling.
        """
        self._require_recording()
        self._commands.append(
            RecordedCommand(
                "set_event",
                {"event_id": event.event_id, "stage": stage.value},
            )
        )

    def cmd_wait_event(
        self,
        event: Event,
        src_stage: PipelineStage,
        dst_stage: PipelineStage,
    ) -> None:
        """Wait for an event before proceeding.

        Args:
            event:     The event to wait on.
            src_stage: The stage that set the event.
            dst_stage: The stage that should wait.
        """
        self._require_recording()
        self._commands.append(
            RecordedCommand(
                "wait_event",
                {
                    "event_id": event.event_id,
                    "src_stage": src_stage.value,
                    "dst_stage": dst_stage.value,
                },
            )
        )

    def cmd_reset_event(self, event: Event, stage: PipelineStage) -> None:
        """Reset an event from the GPU side.

        Args:
            event: The event to reset.
            stage: Wait for this stage before resetting.
        """
        self._require_recording()
        self._commands.append(
            RecordedCommand(
                "reset_event",
                {"event_id": event.event_id, "stage": stage.value},
            )
        )

"""CommandQueue — FIFO submission of command buffers to a device.

=== How Submission Works ===

When you submit command buffers to a queue, the runtime processes them
sequentially, executing each recorded command against the Layer 6 device:

    queue.submit([cb1, cb2], fence=fence)
        │
        ├── Execute cb1's commands:
        │   ├── bind_pipeline → set current pipeline
        │   ├── bind_descriptor_set → set current descriptors
        │   ├── dispatch(4, 1, 1) → device.launch_kernel() + device.run()
        │   └── pipeline_barrier → (ensure completion, log trace)
        │
        ├── Execute cb2's commands:
        │   ├── copy_buffer → device.memcpy
        │   └── ...
        │
        ├── Signal semaphores (if any)
        └── Signal fence (if any)

=== Multiple Queues ===

A device can have multiple queues. Queues of different types (compute,
transfer) can execute in parallel — while the compute queue runs a kernel,
the transfer queue can copy data.

Queues of the same type execute sequentially within that queue but may
run in parallel with other queues.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from device_simulator import KernelDescriptor

from .command_buffer import CommandBuffer
from .protocols import (
    CommandBufferState,
    QueueType,
    RuntimeEventType,
    RuntimeStats,
    RuntimeTrace,
)

if TYPE_CHECKING:
    from device_simulator import AcceleratorDevice

    from .memory import MemoryManager
    from .pipeline import DescriptorSet, Pipeline
    from .sync import Event, Fence, Semaphore


class CommandQueue:
    """A FIFO queue that submits command buffers to a device.

    === Queue Properties ===

    - Commands within a CB execute sequentially
    - CBs within a submission execute sequentially
    - Multiple submissions execute sequentially (FIFO)
    - Multiple QUEUES can execute in parallel

    === Typical Usage ===

        compute_queue = device.queues["compute"][0]
        compute_queue.submit(
            command_buffers=[cb],
            fence=fence,
        )
        fence.wait()
    """

    def __init__(
        self,
        queue_type: QueueType,
        queue_index: int,
        device: AcceleratorDevice,
        memory_manager: MemoryManager,
        stats: RuntimeStats,
    ) -> None:
        self._queue_type = queue_type
        self._queue_index = queue_index
        self._device = device
        self._memory_manager = memory_manager
        self._stats = stats
        self._total_cycles: int = 0

        # Execution state
        self._current_pipeline: Pipeline | None = None
        self._current_descriptor_set: DescriptorSet | None = None
        self._current_push_constants: bytes = b""

    @property
    def queue_type(self) -> QueueType:
        """What kind of work this queue handles."""
        return self._queue_type

    @property
    def queue_index(self) -> int:
        """Index within queues of the same type."""
        return self._queue_index

    @property
    def total_cycles(self) -> int:
        """Total device cycles consumed by this queue."""
        return self._total_cycles

    def submit(
        self,
        command_buffers: list[CommandBuffer],
        *,
        wait_semaphores: list[Semaphore] | None = None,
        signal_semaphores: list[Semaphore] | None = None,
        fence: Fence | None = None,
    ) -> list[RuntimeTrace]:
        """Submit command buffers for execution.

        === Submission Flow ===

        1. Wait for all wait_semaphores to be signaled
        2. Execute each command buffer sequentially
        3. Signal all signal_semaphores
        4. Signal the fence (if provided)

        Args:
            command_buffers:    CBs to execute (in order).
            wait_semaphores:    Wait for these before starting.
            signal_semaphores:  Signal these when all CBs complete.
            fence:              Signal this fence when done (for CPU waiting).

        Returns:
            List of RuntimeTrace events generated during execution.

        Raises:
            RuntimeError: If any CB is not in RECORDED state.
            RuntimeError: If a wait_semaphore is not signaled.
        """
        traces: list[RuntimeTrace] = []
        wait_sems = wait_semaphores or []
        signal_sems = signal_semaphores or []

        # Validate CB states
        for cb in command_buffers:
            if cb.state != CommandBufferState.RECORDED:
                raise RuntimeError(
                    f"CB#{cb.command_buffer_id} is in state {cb.state.value}, "
                    f"expected RECORDED"
                )

        # Wait on semaphores
        for sem in wait_sems:
            if not sem.signaled:
                raise RuntimeError(
                    f"Semaphore {sem.semaphore_id} is not signaled — "
                    f"cannot proceed (possible deadlock)"
                )
            traces.append(
                RuntimeTrace(
                    timestamp_cycles=self._total_cycles,
                    event_type=RuntimeEventType.SEMAPHORE_WAIT,
                    description=f"Wait on semaphore S{sem.semaphore_id}",
                    queue_type=self._queue_type,
                    semaphore_id=sem.semaphore_id,
                )
            )
            sem.reset()  # Consume the semaphore

        # Log submission
        self._stats.total_submissions += 1
        self._stats.total_command_buffers += len(command_buffers)

        cb_ids = [cb.command_buffer_id for cb in command_buffers]
        traces.append(
            RuntimeTrace(
                timestamp_cycles=self._total_cycles,
                event_type=RuntimeEventType.SUBMIT,
                description=(
                    f"Submit CB {cb_ids} to {self._queue_type.value} queue"
                ),
                queue_type=self._queue_type,
            )
        )

        # Execute each command buffer
        for cb in command_buffers:
            cb._mark_pending()
            cb_traces = self._execute_command_buffer(cb)
            traces.extend(cb_traces)
            cb._mark_complete()

        # Signal semaphores
        for sem in signal_sems:
            sem.signal()
            self._stats.total_semaphore_signals += 1
            traces.append(
                RuntimeTrace(
                    timestamp_cycles=self._total_cycles,
                    event_type=RuntimeEventType.SEMAPHORE_SIGNAL,
                    description=f"Signal semaphore S{sem.semaphore_id}",
                    queue_type=self._queue_type,
                    semaphore_id=sem.semaphore_id,
                )
            )

        # Signal fence
        if fence is not None:
            fence.signal()
            traces.append(
                RuntimeTrace(
                    timestamp_cycles=self._total_cycles,
                    event_type=RuntimeEventType.FENCE_SIGNAL,
                    description=f"Signal fence F{fence.fence_id}",
                    queue_type=self._queue_type,
                    fence_id=fence.fence_id,
                )
            )

        # Update stats
        self._stats.total_device_cycles = self._total_cycles
        self._stats.update_utilization()
        self._stats.traces.extend(traces)

        return traces

    def wait_idle(self) -> None:
        """Block until this queue has no pending work.

        In our synchronous simulation, submit() always runs to completion,
        so this is a no-op. In a real async system, this would block.
        """
        pass

    def _execute_command_buffer(
        self, cb: CommandBuffer
    ) -> list[RuntimeTrace]:
        """Execute all commands in a command buffer.

        We replay the CB's recorded bind state: as we encounter bind_pipeline
        and bind_descriptor_set commands, we update _current_pipeline and
        _current_descriptor_set to mirror the state the CB recorded.
        """
        traces: list[RuntimeTrace] = []

        # Replay the CB's bind state by re-executing binds.
        # The CB stores references to the pipeline/descriptor it bound.
        self._current_pipeline = cb.bound_pipeline
        self._current_descriptor_set = cb.bound_descriptor_set

        traces.append(
            RuntimeTrace(
                timestamp_cycles=self._total_cycles,
                event_type=RuntimeEventType.BEGIN_EXECUTION,
                description=f"Begin CB#{cb.command_buffer_id}",
                queue_type=self._queue_type,
                command_buffer_id=cb.command_buffer_id,
            )
        )

        for cmd in cb.commands:
            cmd_traces = self._execute_command(cmd)
            traces.extend(cmd_traces)

        traces.append(
            RuntimeTrace(
                timestamp_cycles=self._total_cycles,
                event_type=RuntimeEventType.END_EXECUTION,
                description=f"End CB#{cb.command_buffer_id}",
                queue_type=self._queue_type,
                command_buffer_id=cb.command_buffer_id,
            )
        )

        return traces

    def _execute_command(self, cmd: RecordedCommand) -> list[RuntimeTrace]:
        """Execute a single recorded command against the device."""
        handler = {
            "bind_pipeline": self._exec_bind_pipeline,
            "bind_descriptor_set": self._exec_bind_descriptor_set,
            "push_constants": self._exec_push_constants,
            "dispatch": self._exec_dispatch,
            "dispatch_indirect": self._exec_dispatch_indirect,
            "copy_buffer": self._exec_copy_buffer,
            "fill_buffer": self._exec_fill_buffer,
            "update_buffer": self._exec_update_buffer,
            "pipeline_barrier": self._exec_pipeline_barrier,
            "set_event": self._exec_set_event,
            "wait_event": self._exec_wait_event,
            "reset_event": self._exec_reset_event,
        }.get(cmd.command)

        if handler is None:
            raise RuntimeError(f"Unknown command: {cmd.command}")

        return handler(cmd.args)

    # =================================================================
    # Command executors
    # =================================================================

    def _exec_bind_pipeline(self, args: dict[str, Any]) -> list[RuntimeTrace]:
        """Bind pipeline — just update state, no device interaction."""
        # The pipeline object is stored on the CB, we look it up by ID
        # In our implementation, the CB already stored the reference
        # We'll access it through the CB's bound state when dispatching
        return []

    def _exec_bind_descriptor_set(
        self, args: dict[str, Any]
    ) -> list[RuntimeTrace]:
        """Bind descriptor set — just update state."""
        return []

    def _exec_push_constants(
        self, args: dict[str, Any]
    ) -> list[RuntimeTrace]:
        """Push constants — just update state."""
        return []

    def _exec_dispatch(self, args: dict[str, Any]) -> list[RuntimeTrace]:
        """Dispatch — translate to KernelDescriptor and execute on device."""
        group_x = args["group_x"]
        group_y = args["group_y"]
        group_z = args["group_z"]

        # Build KernelDescriptor from current pipeline + descriptor set
        pipeline = self._current_pipeline
        if pipeline is None:
            raise RuntimeError("No pipeline bound for dispatch")

        shader = pipeline.shader

        if shader.is_gpu_style:
            kernel = KernelDescriptor(
                name=f"dispatch_{group_x}x{group_y}x{group_z}",
                program=shader.code,
                grid_dim=(group_x, group_y, group_z),
                block_dim=shader.local_size,
            )
        else:
            # Dataflow-style dispatch
            kernel = KernelDescriptor(
                name=f"op_{shader.operation}",
                operation=shader.operation,
                # Dataflow input_data/weight_data would come from descriptor set
                # For now, use placeholder data
                input_data=[[1.0]],
                weight_data=[[1.0]],
            )

        self._device.launch_kernel(kernel)
        device_traces = self._device.run(10000)
        cycles = len(device_traces)
        self._total_cycles += cycles

        self._stats.total_dispatches += 1

        return [
            RuntimeTrace(
                timestamp_cycles=self._total_cycles,
                event_type=RuntimeEventType.END_EXECUTION,
                description=(
                    f"Dispatch ({group_x},{group_y},{group_z}) "
                    f"completed in {cycles} cycles"
                ),
                queue_type=self._queue_type,
                device_traces=tuple(device_traces),
            )
        ]

    def _exec_dispatch_indirect(
        self, args: dict[str, Any]
    ) -> list[RuntimeTrace]:
        """Dispatch indirect — read grid dims from buffer, then dispatch."""
        import struct

        buffer_id = args["buffer_id"]
        offset = args["offset"]

        buf = self._memory_manager.get_buffer(buffer_id)
        data = self._memory_manager._get_buffer_data(buffer_id)
        group_x, group_y, group_z = struct.unpack_from("<III", data, offset)

        return self._exec_dispatch(
            {"group_x": group_x, "group_y": group_y, "group_z": group_z}
        )

    def _exec_copy_buffer(self, args: dict[str, Any]) -> list[RuntimeTrace]:
        """Copy buffer — transfer data between device buffers."""
        src_id = args["src_id"]
        dst_id = args["dst_id"]
        size = args["size"]
        src_offset = args.get("src_offset", 0)
        dst_offset = args.get("dst_offset", 0)

        src_data = self._memory_manager._get_buffer_data(src_id)
        dst_data = self._memory_manager._get_buffer_data(dst_id)

        # Copy the bytes
        dst_data[dst_offset : dst_offset + size] = src_data[
            src_offset : src_offset + size
        ]

        # Also sync to device memory
        src_buf = self._memory_manager.get_buffer(src_id)
        dst_buf = self._memory_manager.get_buffer(dst_id)

        # Read from source device address, write to destination
        data_bytes, read_cycles = self._device.memcpy_device_to_host(
            src_buf.device_address + src_offset, size
        )
        write_cycles = self._device.memcpy_host_to_device(
            dst_buf.device_address + dst_offset, data_bytes
        )

        cycles = read_cycles + write_cycles
        self._total_cycles += cycles
        self._stats.total_transfers += 1

        return [
            RuntimeTrace(
                timestamp_cycles=self._total_cycles,
                event_type=RuntimeEventType.MEMORY_TRANSFER,
                description=(
                    f"Copy {size} bytes: buf#{src_id} → buf#{dst_id} "
                    f"({cycles} cycles)"
                ),
                queue_type=self._queue_type,
            )
        ]

    def _exec_fill_buffer(self, args: dict[str, Any]) -> list[RuntimeTrace]:
        """Fill buffer with a constant value."""
        buffer_id = args["buffer_id"]
        value = args["value"]
        offset = args["offset"]
        size = args["size"]

        buf_data = self._memory_manager._get_buffer_data(buffer_id)
        buf_data[offset : offset + size] = bytes([value & 0xFF]) * size

        # Sync to device
        buf = self._memory_manager.get_buffer(buffer_id)
        fill_bytes = bytes([value & 0xFF]) * size
        self._device.memcpy_host_to_device(
            buf.device_address + offset, fill_bytes
        )

        self._stats.total_transfers += 1
        return []

    def _exec_update_buffer(
        self, args: dict[str, Any]
    ) -> list[RuntimeTrace]:
        """Write small data inline."""
        buffer_id = args["buffer_id"]
        offset = args["offset"]
        data = args["data"]

        buf_data = self._memory_manager._get_buffer_data(buffer_id)
        buf_data[offset : offset + len(data)] = data

        # Sync to device
        buf = self._memory_manager.get_buffer(buffer_id)
        self._device.memcpy_host_to_device(
            buf.device_address + offset, data
        )

        self._stats.total_transfers += 1
        return []

    def _exec_pipeline_barrier(
        self, args: dict[str, Any]
    ) -> list[RuntimeTrace]:
        """Pipeline barrier — in synchronous mode, this is mostly a no-op."""
        self._stats.total_barriers += 1
        return [
            RuntimeTrace(
                timestamp_cycles=self._total_cycles,
                event_type=RuntimeEventType.BARRIER,
                description=(
                    f"Barrier: {args['src_stage']} → {args['dst_stage']}"
                ),
                queue_type=self._queue_type,
            )
        ]

    def _exec_set_event(self, args: dict[str, Any]) -> list[RuntimeTrace]:
        """Set event from GPU side."""
        # In our synchronous sim, events are set immediately
        return []

    def _exec_wait_event(self, args: dict[str, Any]) -> list[RuntimeTrace]:
        """Wait for event."""
        return []

    def _exec_reset_event(self, args: dict[str, Any]) -> list[RuntimeTrace]:
        """Reset event from GPU side."""
        return []

    def set_pipeline(self, pipeline: Pipeline) -> None:
        """Internal: set the current pipeline state from CB execution."""
        self._current_pipeline = pipeline

    def set_descriptor_set(self, descriptor_set: DescriptorSet) -> None:
        """Internal: set the current descriptor set from CB execution."""
        self._current_descriptor_set = descriptor_set

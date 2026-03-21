"""ValidationLayer — catches GPU programming errors early.

=== What is a Validation Layer? ===

In Vulkan, validation layers are optional middleware that check every API
call for errors. They're enabled during development and disabled in
production (for performance). Common errors they catch:

    - Dispatching without binding a pipeline
    - Using a freed buffer in a descriptor set
    - Missing a barrier between write and read
    - Mapping a DEVICE_LOCAL-only buffer
    - Exceeding device limits

Our validation layer wraps a LogicalDevice and checks every operation.
It's always enabled (since we're a simulator, not a production runtime).

=== Usage ===

    device = instance.create_logical_device(physical)
    validated = ValidationLayer(device)

    # Now use validated.* instead of device.*
    # It will raise ValidationError on any misuse
"""

from __future__ import annotations

from typing import Any

from .command_buffer import CommandBuffer
from .memory import Buffer
from .pipeline import DescriptorSet, Pipeline
from .protocols import (
    BufferUsage,
    CommandBufferState,
    MemoryType,
)


class ValidationError(Exception):
    """Raised when a validation check fails.

    These errors represent GPU programming mistakes — things that would
    cause undefined behavior or crashes on real hardware.
    """

    pass


class ValidationLayer:
    """Validates runtime operations and raises clear error messages.

    === What It Checks ===

    1. Command buffer state transitions (can't record without begin())
    2. Pipeline/descriptor binding (can't dispatch without binding both)
    3. Memory type compatibility (can't map DEVICE_LOCAL)
    4. Buffer usage flags (can't use STORAGE buffer as TRANSFER_SRC)
    5. Freed resource detection (can't use freed buffers)
    6. Barrier correctness (warn on write→read without barrier)
    """

    def __init__(self) -> None:
        self._warnings: list[str] = []
        self._errors: list[str] = []
        # Track which buffers have been written to (for barrier checking)
        self._written_buffers: set[int] = set()
        # Track which buffers have barriers protecting reads
        self._barriered_buffers: set[int] = set()

    @property
    def warnings(self) -> list[str]:
        """All validation warnings issued so far."""
        return list(self._warnings)

    @property
    def errors(self) -> list[str]:
        """All validation errors issued so far."""
        return list(self._errors)

    def clear(self) -> None:
        """Clear all warnings and errors."""
        self._warnings.clear()
        self._errors.clear()
        self._written_buffers.clear()
        self._barriered_buffers.clear()

    # --- Command buffer validation ---

    def validate_begin(self, cb: CommandBuffer) -> None:
        """Validate that begin() is allowed."""
        if cb.state not in (
            CommandBufferState.INITIAL,
            CommandBufferState.COMPLETE,
        ):
            raise ValidationError(
                f"Cannot begin CB#{cb.command_buffer_id}: "
                f"state is {cb.state.value} (expected INITIAL or COMPLETE)"
            )

    def validate_end(self, cb: CommandBuffer) -> None:
        """Validate that end() is allowed."""
        if cb.state != CommandBufferState.RECORDING:
            raise ValidationError(
                f"Cannot end CB#{cb.command_buffer_id}: "
                f"state is {cb.state.value} (expected RECORDING)"
            )

    def validate_submit(self, cb: CommandBuffer) -> None:
        """Validate that a CB can be submitted."""
        if cb.state != CommandBufferState.RECORDED:
            raise ValidationError(
                f"Cannot submit CB#{cb.command_buffer_id}: "
                f"state is {cb.state.value} (expected RECORDED)"
            )

    # --- Dispatch validation ---

    def validate_dispatch(
        self,
        cb: CommandBuffer,
        group_x: int,
        group_y: int,
        group_z: int,
    ) -> None:
        """Validate a dispatch command."""
        if cb.bound_pipeline is None:
            raise ValidationError(
                f"Cannot dispatch in CB#{cb.command_buffer_id}: "
                f"no pipeline bound (call cmd_bind_pipeline first)"
            )
        if group_x <= 0 or group_y <= 0 or group_z <= 0:
            raise ValidationError(
                f"Dispatch dimensions must be positive: "
                f"({group_x}, {group_y}, {group_z})"
            )

    # --- Memory validation ---

    def validate_map(self, buffer: Buffer) -> None:
        """Validate that a buffer can be mapped."""
        if buffer.freed:
            raise ValidationError(
                f"Cannot map freed buffer {buffer.buffer_id}"
            )
        if buffer.mapped:
            raise ValidationError(
                f"Buffer {buffer.buffer_id} is already mapped"
            )
        if not (MemoryType.HOST_VISIBLE in buffer.memory_type):
            raise ValidationError(
                f"Cannot map buffer {buffer.buffer_id}: "
                f"not HOST_VISIBLE (type={buffer.memory_type}). "
                f"Use a staging buffer for DEVICE_LOCAL memory."
            )

    def validate_buffer_usage(
        self, buffer: Buffer, required_usage: BufferUsage
    ) -> None:
        """Validate that a buffer has the required usage flags."""
        if not (required_usage in buffer.usage):
            raise ValidationError(
                f"Buffer {buffer.buffer_id} lacks required usage "
                f"{required_usage} (has {buffer.usage})"
            )

    def validate_buffer_not_freed(self, buffer: Buffer) -> None:
        """Validate that a buffer is not freed."""
        if buffer.freed:
            raise ValidationError(
                f"Buffer {buffer.buffer_id} has been freed"
            )

    # --- Barrier validation ---

    def record_write(self, buffer_id: int) -> None:
        """Record that a buffer was written to (for barrier checking)."""
        self._written_buffers.add(buffer_id)
        self._barriered_buffers.discard(buffer_id)

    def record_barrier(self, buffer_ids: set[int] | None = None) -> None:
        """Record that a barrier was placed (covers some/all buffers)."""
        if buffer_ids is None:
            # Global barrier — covers all written buffers
            self._barriered_buffers.update(self._written_buffers)
        else:
            self._barriered_buffers.update(buffer_ids)

    def validate_read_after_write(self, buffer_id: int) -> None:
        """Warn if reading a buffer that was written without a barrier."""
        if (
            buffer_id in self._written_buffers
            and buffer_id not in self._barriered_buffers
        ):
            self._warnings.append(
                f"Reading buffer {buffer_id} after write without barrier. "
                f"Insert cmd_pipeline_barrier() between write and read."
            )

    # --- Descriptor set validation ---

    def validate_descriptor_set(
        self, descriptor_set: DescriptorSet, pipeline: Pipeline
    ) -> None:
        """Validate that a descriptor set is compatible with a pipeline."""
        layout = pipeline.layout
        if not layout.set_layouts:
            return  # No descriptors needed

        expected_layout = layout.set_layouts[0]
        for binding_def in expected_layout.bindings:
            buf = descriptor_set.get_buffer(binding_def.binding)
            if buf is None:
                self._warnings.append(
                    f"Binding {binding_def.binding} not set in "
                    f"descriptor set {descriptor_set.set_id}"
                )
            elif buf.freed:
                raise ValidationError(
                    f"Binding {binding_def.binding} uses freed buffer "
                    f"{buf.buffer_id}"
                )

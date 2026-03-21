"""Pipeline — compiled kernels, descriptor sets, shader modules.

=== What is a Pipeline? ===

A pipeline is a **compiled kernel ready to execute**. In Vulkan terms, it
packages three things together:

    1. ShaderModule — the compiled program (instructions)
    2. PipelineLayout — what data the kernel expects (descriptor set layout)
    3. Pipeline — the combined, ready-to-dispatch object

Think of it like a function call:
    - ShaderModule = the function body (code)
    - DescriptorSetLayout = the function signature (parameter types)
    - DescriptorSet = the actual arguments (concrete buffers)
    - Pipeline = the compiled function ready to call

=== Why Separate Shader from Pipeline? ===

The same shader code can be used in multiple pipelines with different
layouts. And the same pipeline can be used with different descriptor sets
(different data). This separation enables reuse:

    shader = compile("saxpy.glsl")

    pipeline_1d = create_pipeline(shader, layout_1d)   # 1D data
    pipeline_2d = create_pipeline(shader, layout_2d)   # 2D data

    desc_set_A = create_descriptor_set(layout_1d)
    desc_set_A.write(0, buffer_A)

    desc_set_B = create_descriptor_set(layout_1d)
    desc_set_B.write(0, buffer_B)

    # Same pipeline, different data:
    cb.cmd_bind_pipeline(pipeline_1d)
    cb.cmd_bind_descriptor_set(desc_set_A)
    cb.cmd_dispatch(100, 1, 1)
    cb.cmd_bind_descriptor_set(desc_set_B)
    cb.cmd_dispatch(100, 1, 1)
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from .memory import Buffer
from .protocols import DescriptorBinding


# =========================================================================
# ShaderModule — compiled program
# =========================================================================


class ShaderModule:
    """A compiled program ready to be used in a pipeline.

    === GPU vs Dataflow ===

    For GPU-style devices (NVIDIA, AMD, Intel), the code is a list of
    instructions from our GenericISA (gpu-core package).

    For dataflow-style devices (TPU, ANE), the code is an operation
    descriptor — just the operation name and parameters.

    The shader module doesn't care which — it stores whatever code was
    given. The pipeline compilation step adapts it to the target device.
    """

    _next_id: int = 0

    def __init__(
        self,
        code: list[Any] | None = None,
        *,
        operation: str = "",
        entry_point: str = "main",
        local_size: tuple[int, int, int] = (32, 1, 1),
    ) -> None:
        self._id = ShaderModule._next_id
        ShaderModule._next_id += 1
        self._code = code
        self._operation = operation
        self._entry_point = entry_point
        self._local_size = local_size

    @property
    def module_id(self) -> int:
        """Unique identifier."""
        return self._id

    @property
    def code(self) -> list[Any] | None:
        """GPU-style: list of instructions. None for dataflow."""
        return self._code

    @property
    def operation(self) -> str:
        """Dataflow-style: operation name (e.g., 'matmul'). Empty for GPU."""
        return self._operation

    @property
    def entry_point(self) -> str:
        """Entry point name (typically 'main')."""
        return self._entry_point

    @property
    def local_size(self) -> tuple[int, int, int]:
        """Workgroup dimensions declared in the shader."""
        return self._local_size

    @property
    def is_gpu_style(self) -> bool:
        """True if this is a GPU-style shader (has instruction code)."""
        return self._code is not None

    @property
    def is_dataflow_style(self) -> bool:
        """True if this is a dataflow-style shader (has operation name)."""
        return bool(self._operation)


# =========================================================================
# DescriptorSetLayout — describes the shape of data bindings
# =========================================================================


class DescriptorSetLayout:
    """Describes what data a kernel expects.

    === What is a Layout? ===

    A layout is like a function signature — it says "this kernel takes
    3 storage buffers." It doesn't say WHICH buffers, just how many
    and what type.

    The actual buffer assignments happen when you create a DescriptorSet
    from this layout and call write() on it.

    Example:
        layout = DescriptorSetLayout([
            DescriptorBinding(binding=0, type="storage"),  # input X
            DescriptorBinding(binding=1, type="storage"),  # input Y
            DescriptorBinding(binding=2, type="storage"),  # output Z
        ])
    """

    _next_id: int = 0

    def __init__(self, bindings: list[DescriptorBinding]) -> None:
        self._id = DescriptorSetLayout._next_id
        DescriptorSetLayout._next_id += 1
        self._bindings = tuple(bindings)

    @property
    def layout_id(self) -> int:
        """Unique identifier."""
        return self._id

    @property
    def bindings(self) -> tuple[DescriptorBinding, ...]:
        """The binding slots in this layout."""
        return self._bindings


# =========================================================================
# PipelineLayout — shader + descriptor layout + push constants
# =========================================================================


class PipelineLayout:
    """Describes the complete interface of a pipeline.

    Combines:
    - Descriptor set layouts (what buffers the kernel reads/writes)
    - Push constant size (small inline data like alpha in SAXPY)
    """

    _next_id: int = 0

    def __init__(
        self,
        set_layouts: list[DescriptorSetLayout],
        push_constant_size: int = 0,
    ) -> None:
        self._id = PipelineLayout._next_id
        PipelineLayout._next_id += 1
        self._set_layouts = list(set_layouts)
        self._push_constant_size = push_constant_size

    @property
    def layout_id(self) -> int:
        """Unique identifier."""
        return self._id

    @property
    def set_layouts(self) -> list[DescriptorSetLayout]:
        """Descriptor set layouts used by this pipeline."""
        return self._set_layouts

    @property
    def push_constant_size(self) -> int:
        """Maximum bytes for push constants."""
        return self._push_constant_size


# =========================================================================
# Pipeline — compiled, ready to dispatch
# =========================================================================


class Pipeline:
    """A compiled kernel bound to a pipeline layout.

    === Creating a Pipeline ===

        shader = device.create_shader_module(code=[...], local_size=(256,1,1))
        layout = device.create_pipeline_layout(set_layouts=[ds_layout])
        pipeline = device.create_compute_pipeline(shader, layout)

    Once created, bind it in a command buffer:
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_dispatch(grid_x, grid_y, grid_z)
    """

    _next_id: int = 0

    def __init__(
        self,
        shader: ShaderModule,
        layout: PipelineLayout,
    ) -> None:
        self._id = Pipeline._next_id
        Pipeline._next_id += 1
        self._shader = shader
        self._layout = layout

    @property
    def pipeline_id(self) -> int:
        """Unique identifier."""
        return self._id

    @property
    def shader(self) -> ShaderModule:
        """The compiled shader module."""
        return self._shader

    @property
    def layout(self) -> PipelineLayout:
        """The pipeline layout (descriptor sets + push constants)."""
        return self._layout

    @property
    def workgroup_size(self) -> tuple[int, int, int]:
        """Local workgroup dimensions from the shader."""
        return self._shader.local_size


# =========================================================================
# DescriptorSet — concrete buffer bindings
# =========================================================================


class DescriptorSet:
    """Concrete buffer assignments for a descriptor set layout.

    === Layout vs Set ===

    Layout says: "binding 0 is a storage buffer"
    Set says:    "binding 0 is buf_x (address 0x1000, 4096 bytes)"

    You create a set from a layout, then write() buffers into it.
    Multiple sets can share the same layout with different buffers.
    """

    _next_id: int = 0

    def __init__(self, layout: DescriptorSetLayout) -> None:
        self._id = DescriptorSet._next_id
        DescriptorSet._next_id += 1
        self._layout = layout
        self._bindings: dict[int, Buffer] = {}

    @property
    def set_id(self) -> int:
        """Unique identifier."""
        return self._id

    @property
    def layout(self) -> DescriptorSetLayout:
        """The layout this set was created from."""
        return self._layout

    @property
    def bindings(self) -> dict[int, Buffer]:
        """Current buffer bindings (binding number → Buffer)."""
        return dict(self._bindings)

    def write(self, binding: int, buffer: Buffer) -> None:
        """Bind a buffer to a slot.

        Args:
            binding: Slot number (must exist in layout).
            buffer:  The buffer to bind.

        Raises:
            ValueError: If binding doesn't exist in layout or buffer is freed.
        """
        # Validate binding exists in layout
        valid_bindings = {b.binding for b in self._layout.bindings}
        if binding not in valid_bindings:
            raise ValueError(
                f"Binding {binding} not in layout (valid: {valid_bindings})"
            )
        if buffer.freed:
            raise ValueError(
                f"Cannot bind freed buffer {buffer.buffer_id} to binding {binding}"
            )
        self._bindings[binding] = buffer

    def get_buffer(self, binding: int) -> Buffer | None:
        """Get the buffer at a binding slot, or None if not bound."""
        return self._bindings.get(binding)

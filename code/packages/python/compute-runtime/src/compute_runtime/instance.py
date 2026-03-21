"""Instance — device discovery, physical/logical device management.

=== The Entry Point ===

The RuntimeInstance is how everything starts. It's the first object you
create, and it gives you access to all available hardware:

    instance = RuntimeInstance()
    devices = instance.enumerate_physical_devices()
    # → [PhysicalDevice("NVIDIA H100"), PhysicalDevice("Apple M3 Max ANE"), ...]

=== Physical vs Logical Device ===

A PhysicalDevice is a read-only description of hardware. You can query
its name, type, memory, and capabilities, but you can't use it directly.

A LogicalDevice is a usable handle. It wraps a PhysicalDevice and provides:
- Command queues for submitting work
- Memory manager for allocating buffers
- Factory methods for pipelines, sync objects, etc.

Why the separation?
- A system may have multiple GPUs. You query all of them, compare, and pick.
- Multiple logical devices can share one physical device.
- The physical device never changes. The logical device owns mutable state.

This pattern comes directly from Vulkan (VkPhysicalDevice vs VkDevice).
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from device_simulator import (
    AcceleratorDevice,
    AmdGPU,
    AppleANE,
    DeviceConfig,
    GoogleTPU,
    IntelGPU,
    NvidiaGPU,
)

from .command_buffer import CommandBuffer
from .command_queue import CommandQueue
from .memory import MemoryManager
from .pipeline import (
    DescriptorSet,
    DescriptorSetLayout,
    Pipeline,
    PipelineLayout,
    ShaderModule,
)
from .protocols import (
    DescriptorBinding,
    DeviceLimits,
    DeviceType,
    MemoryHeap,
    MemoryProperties,
    MemoryType,
    QueueFamily,
    QueueType,
    RuntimeStats,
)
from .sync import Event, Fence, Semaphore


# =========================================================================
# PhysicalDevice — read-only hardware description
# =========================================================================


class PhysicalDevice:
    """Read-only description of a physical accelerator.

    === What You Can Learn ===

    - name: "NVIDIA H100", "Apple M3 Max ANE", etc.
    - device_type: GPU, TPU, or NPU
    - vendor: "nvidia", "amd", "google", "intel", "apple"
    - memory_properties: what memory types are available, how much
    - queue_families: what kinds of queues the device supports
    - limits: hardware constraints (max workgroup size, buffer size, etc.)

    You can't execute anything on a PhysicalDevice. Create a LogicalDevice
    for that.
    """

    def __init__(
        self,
        device_id: int,
        name: str,
        device_type: DeviceType,
        vendor: str,
        accelerator: AcceleratorDevice,
        memory_properties: MemoryProperties,
        queue_families: list[QueueFamily],
        limits: DeviceLimits,
    ) -> None:
        self._device_id = device_id
        self._name = name
        self._device_type = device_type
        self._vendor = vendor
        self._accelerator = accelerator
        self._memory_properties = memory_properties
        self._queue_families = list(queue_families)
        self._limits = limits

    @property
    def device_id(self) -> int:
        """Unique device identifier."""
        return self._device_id

    @property
    def name(self) -> str:
        """Human-readable name."""
        return self._name

    @property
    def device_type(self) -> DeviceType:
        """GPU, TPU, or NPU."""
        return self._device_type

    @property
    def vendor(self) -> str:
        """Vendor identifier."""
        return self._vendor

    @property
    def memory_properties(self) -> MemoryProperties:
        """Available memory types and heaps."""
        return self._memory_properties

    @property
    def queue_families(self) -> list[QueueFamily]:
        """Available queue families."""
        return list(self._queue_families)

    @property
    def limits(self) -> DeviceLimits:
        """Hardware limits."""
        return self._limits

    def supports_feature(self, feature: str) -> bool:
        """Check if a feature is supported.

        Currently supported features:
        - "fp32": 32-bit float (always true)
        - "fp16": 16-bit float
        - "unified_memory": CPU/GPU shared memory
        - "transfer_queue": dedicated DMA engine

        Args:
            feature: Feature name to query.

        Returns:
            True if supported.
        """
        features: dict[str, bool] = {
            "fp32": True,
            "fp16": True,
            "unified_memory": self._memory_properties.is_unified,
            "transfer_queue": any(
                qf.queue_type == QueueType.TRANSFER
                for qf in self._queue_families
            ),
        }
        return features.get(feature, False)


# =========================================================================
# LogicalDevice — usable handle with queues and factories
# =========================================================================


class LogicalDevice:
    """A usable device handle with command queues and resource factories.

    === What You Can Do ===

    - Submit work via command queues
    - Allocate memory via memory_manager
    - Create command buffers, pipelines, sync objects
    - Wait for all work to complete

    === Created From ===

    LogicalDevice is created by RuntimeInstance.create_logical_device(),
    not directly. You specify which queue types you want, and the logical
    device creates them.
    """

    def __init__(
        self,
        physical_device: PhysicalDevice,
        accelerator: AcceleratorDevice,
        queues: dict[str, list[CommandQueue]],
        memory_manager: MemoryManager,
        stats: RuntimeStats,
    ) -> None:
        self._physical = physical_device
        self._accelerator = accelerator
        self._queues = queues
        self._memory_manager = memory_manager
        self._stats = stats

    @property
    def physical_device(self) -> PhysicalDevice:
        """The underlying physical device."""
        return self._physical

    @property
    def queues(self) -> dict[str, list[CommandQueue]]:
        """Command queues by type name ('compute', 'transfer')."""
        return self._queues

    @property
    def memory_manager(self) -> MemoryManager:
        """Memory allocation manager."""
        return self._memory_manager

    @property
    def stats(self) -> RuntimeStats:
        """Runtime statistics."""
        return self._stats

    # --- Factory methods ---

    def create_command_buffer(self) -> CommandBuffer:
        """Create a new command buffer."""
        return CommandBuffer()

    def create_shader_module(
        self,
        code: list[Any] | None = None,
        *,
        operation: str = "",
        entry_point: str = "main",
        local_size: tuple[int, int, int] = (32, 1, 1),
    ) -> ShaderModule:
        """Create a shader module from code or operation descriptor.

        For GPU-style devices, pass code (list of Instructions).
        For dataflow devices, pass operation name.

        Args:
            code:        GPU-style instruction list.
            operation:   Dataflow-style operation name.
            entry_point: Entry point name (default "main").
            local_size:  Workgroup dimensions.

        Returns:
            A new ShaderModule.
        """
        return ShaderModule(
            code=code,
            operation=operation,
            entry_point=entry_point,
            local_size=local_size,
        )

    def create_descriptor_set_layout(
        self, bindings: list[DescriptorBinding]
    ) -> DescriptorSetLayout:
        """Create a descriptor set layout.

        Args:
            bindings: List of binding slots.

        Returns:
            A new DescriptorSetLayout.
        """
        return DescriptorSetLayout(bindings)

    def create_pipeline_layout(
        self,
        set_layouts: list[DescriptorSetLayout],
        push_constant_size: int = 0,
    ) -> PipelineLayout:
        """Create a pipeline layout.

        Args:
            set_layouts:          Descriptor set layouts used by the pipeline.
            push_constant_size:   Max push constant bytes.

        Returns:
            A new PipelineLayout.
        """
        return PipelineLayout(set_layouts, push_constant_size)

    def create_compute_pipeline(
        self, shader: ShaderModule, layout: PipelineLayout
    ) -> Pipeline:
        """Create a compute pipeline.

        Args:
            shader: Compiled shader module.
            layout: Pipeline layout.

        Returns:
            A new Pipeline, ready to bind in a command buffer.
        """
        return Pipeline(shader, layout)

    def create_descriptor_set(
        self, layout: DescriptorSetLayout
    ) -> DescriptorSet:
        """Create a descriptor set from a layout.

        Args:
            layout: The layout to create from.

        Returns:
            A new DescriptorSet (bindings not yet assigned).
        """
        return DescriptorSet(layout)

    def create_fence(self, signaled: bool = False) -> Fence:
        """Create a fence for CPU↔GPU synchronization.

        Args:
            signaled: If True, fence starts already signaled.

        Returns:
            A new Fence.
        """
        return Fence(signaled=signaled)

    def create_semaphore(self) -> Semaphore:
        """Create a semaphore for GPU queue↔queue synchronization."""
        return Semaphore()

    def create_event(self) -> Event:
        """Create an event for fine-grained GPU-side signaling."""
        return Event()

    def wait_idle(self) -> None:
        """Block until all queues finish all pending work."""
        for queue_list in self._queues.values():
            for queue in queue_list:
                queue.wait_idle()

    def reset(self) -> None:
        """Reset all device state."""
        self._accelerator.reset()


# =========================================================================
# RuntimeInstance — the entry point
# =========================================================================


def _make_physical_device(
    device_id: int,
    accelerator: AcceleratorDevice,
    device_type: DeviceType,
    vendor: str,
) -> PhysicalDevice:
    """Create a PhysicalDevice from an AcceleratorDevice.

    Maps Layer 6 device configuration to Layer 5 physical device properties.
    """
    config = accelerator.config
    is_unified = config.unified_memory

    # Build memory heaps based on device type
    if is_unified:
        heaps = (
            MemoryHeap(
                size=config.global_memory_size,
                flags=(
                    MemoryType.DEVICE_LOCAL
                    | MemoryType.HOST_VISIBLE
                    | MemoryType.HOST_COHERENT
                ),
            ),
        )
    else:
        heaps = (
            # VRAM heap (GPU-only, fast)
            MemoryHeap(
                size=config.global_memory_size,
                flags=MemoryType.DEVICE_LOCAL,
            ),
            # Staging heap (CPU-visible, slower)
            MemoryHeap(
                size=min(
                    config.global_memory_size // 4,
                    256 * 1024 * 1024,
                ),
                flags=MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
            ),
        )

    memory_properties = MemoryProperties(heaps=heaps, is_unified=is_unified)

    # Build queue families
    queue_families = [
        QueueFamily(queue_type=QueueType.COMPUTE, count=4),
    ]
    # Discrete GPUs have a separate transfer queue (DMA engine)
    if not is_unified:
        queue_families.append(
            QueueFamily(queue_type=QueueType.TRANSFER, count=2),
        )

    limits = DeviceLimits()

    return PhysicalDevice(
        device_id=device_id,
        name=accelerator.name,
        device_type=device_type,
        vendor=vendor,
        accelerator=accelerator,
        memory_properties=memory_properties,
        queue_families=queue_families,
        limits=limits,
    )


class RuntimeInstance:
    """The runtime entry point — discovers devices and creates handles.

    === Usage ===

        instance = RuntimeInstance()

        # Enumerate all available devices
        devices = instance.enumerate_physical_devices()

        # Pick one and create a logical device
        device = instance.create_logical_device(
            physical_device=devices[0],
            queue_requests=[{"type": "compute", "count": 1}],
        )

    === Default Devices ===

    By default, the instance creates one of each device type with small
    configurations for testing. Pass custom AcceleratorDevice instances
    via the `devices` parameter for specific hardware models.
    """

    def __init__(
        self,
        devices: list[tuple[AcceleratorDevice, DeviceType, str]] | None = None,
    ) -> None:
        """Create a runtime instance.

        Args:
            devices: Optional list of (AcceleratorDevice, DeviceType, vendor) tuples.
                     If None, creates default test devices (small configs).
        """
        self._version = "0.1.0"

        if devices is not None:
            self._physical_devices = [
                _make_physical_device(i, dev, dtype, vendor)
                for i, (dev, dtype, vendor) in enumerate(devices)
            ]
        else:
            # Create small default devices for testing
            self._physical_devices = self._create_default_devices()

    @property
    def version(self) -> str:
        """Runtime version string."""
        return self._version

    def enumerate_physical_devices(self) -> list[PhysicalDevice]:
        """Return all available physical devices.

        Returns a list of PhysicalDevice objects that you can inspect
        and choose from. Each one represents a distinct hardware device.
        """
        return list(self._physical_devices)

    def create_logical_device(
        self,
        physical_device: PhysicalDevice,
        queue_requests: list[dict[str, Any]] | None = None,
    ) -> LogicalDevice:
        """Create a logical device from a physical device.

        Args:
            physical_device: The hardware to use.
            queue_requests:  Optional queue configuration.
                            Each dict has "type" (str) and "count" (int).
                            Default: one compute queue.

        Returns:
            A LogicalDevice ready for use.
        """
        if queue_requests is None:
            queue_requests = [{"type": "compute", "count": 1}]

        stats = RuntimeStats()
        accelerator = physical_device._accelerator

        memory_manager = MemoryManager(
            device=accelerator,
            memory_properties=physical_device.memory_properties,
            stats=stats,
        )

        # Create requested queues
        queues: dict[str, list[CommandQueue]] = {}
        for req in queue_requests:
            qt_str = req["type"]
            count = req.get("count", 1)
            qt = QueueType(qt_str) if qt_str in ("compute", "transfer") else QueueType.COMPUTE_TRANSFER
            queue_list = [
                CommandQueue(
                    queue_type=qt,
                    queue_index=i,
                    device=accelerator,
                    memory_manager=memory_manager,
                    stats=stats,
                )
                for i in range(count)
            ]
            queues[qt_str] = queue_list

        return LogicalDevice(
            physical_device=physical_device,
            accelerator=accelerator,
            queues=queues,
            memory_manager=memory_manager,
            stats=stats,
        )

    def _create_default_devices(self) -> list[PhysicalDevice]:
        """Create small default devices for testing."""
        defaults: list[tuple[AcceleratorDevice, DeviceType, str]] = [
            (NvidiaGPU(num_sms=2), DeviceType.GPU, "nvidia"),
            (AmdGPU(num_cus=2), DeviceType.GPU, "amd"),
            (GoogleTPU(mxu_size=2), DeviceType.TPU, "google"),
            (IntelGPU(num_cores=2), DeviceType.GPU, "intel"),
            (AppleANE(num_cores=2), DeviceType.NPU, "apple"),
        ]
        return [
            _make_physical_device(i, dev, dtype, vendor)
            for i, (dev, dtype, vendor) in enumerate(defaults)
        ]

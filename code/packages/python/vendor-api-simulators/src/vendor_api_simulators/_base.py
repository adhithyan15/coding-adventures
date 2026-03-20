"""BaseVendorSimulator — the shared foundation for all six vendor API simulators.

=== Why a Base Class? ===

Every GPU API, no matter how different its surface looks, needs to do the same
things underneath:

    1. Find a GPU                  --> RuntimeInstance
    2. Create a usable handle      --> LogicalDevice
    3. Get a queue for submission   --> CommandQueue
    4. Manage memory                --> MemoryManager

This base class sets all that up. Each simulator subclass then adds its
vendor-specific vocabulary on top.

Think of it like building six different restaurant fronts (CUDA Grill, Metal
Bistro, Vulkan Steakhouse...) that all share the same kitchen in the back.
The kitchen is our compute runtime (Layer 5). The restaurant menus look
completely different, but the same chefs cook the same food.

=== Device Selection ===

Different APIs have different preferences for which device to use:

    - CUDA always wants an NVIDIA GPU (vendor_hint="nvidia")
    - Metal always wants an Apple device (vendor_hint="apple")
    - OpenCL, Vulkan, WebGPU, OpenGL are cross-vendor

The _select_device() method handles this: it picks the best matching device
from the runtime's enumerated physical devices, preferring the vendor hint
if given, then falling back to any GPU.

=== The _create_and_submit_cb() Helper ===

CUDA and OpenGL hide command buffers from the user. When you call
cudaMemcpy() or glDispatchCompute(), those APIs internally:

    1. Create a command buffer
    2. Begin recording
    3. Record the command(s) via a callback
    4. End recording
    5. Submit to the compute queue with a fence
    6. Wait for the fence

This helper encapsulates that pattern. Pass a callback that records
commands into a CB, and this method handles the rest.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Callable

from compute_runtime import (
    CommandBuffer,
    LogicalDevice,
    MemoryManager,
    PhysicalDevice,
    RuntimeInstance,
)

if TYPE_CHECKING:
    from compute_runtime import CommandQueue, DeviceType


class BaseVendorSimulator:
    """Common foundation for all vendor API simulators.

    === What This Provides ===

    Every subclass gets:
    - self._instance:         RuntimeInstance (Layer 5 entry point)
    - self._physical_devices: All available physical devices
    - self._physical_device:  The selected physical device
    - self._logical_device:   The usable device handle
    - self._compute_queue:    A compute queue for submitting work
    - self._memory_manager:   For allocating and managing memory

    === Usage ===

    Subclasses call super().__init__() with optional device_type and
    vendor_hint to control which device is selected:

        class CUDARuntime(BaseVendorSimulator):
            def __init__(self):
                super().__init__(vendor_hint="nvidia")

        class MTLDevice(BaseVendorSimulator):
            def __init__(self):
                super().__init__(vendor_hint="apple")
    """

    def __init__(
        self,
        device_type: DeviceType | None = None,
        vendor_hint: str | None = None,
    ) -> None:
        """Initialize the simulator with device discovery and setup.

        Args:
            device_type: Preferred device type (GPU, TPU, NPU). If None,
                        any type is acceptable.
            vendor_hint: Preferred vendor string (e.g., "nvidia", "apple").
                        If the preferred vendor isn't found, falls back to
                        any available device.
        """
        # Step 1: Create the runtime instance (discovers all hardware)
        self._instance = RuntimeInstance()

        # Step 2: Enumerate all physical devices
        self._physical_devices = self._instance.enumerate_physical_devices()

        # Step 3: Select the best matching device
        self._physical_device = self._select_device(device_type, vendor_hint)

        # Step 4: Create a logical device (the usable handle)
        self._logical_device = self._instance.create_logical_device(
            self._physical_device
        )

        # Step 5: Get a compute queue for submitting work
        self._compute_queue = self._logical_device.queues["compute"][0]

        # Step 6: Get the memory manager for allocations
        self._memory_manager = self._logical_device.memory_manager

    def _select_device(
        self,
        device_type: DeviceType | None,
        vendor_hint: str | None,
    ) -> PhysicalDevice:
        """Pick the best matching device from enumerated physical devices.

        === Selection Strategy ===

        The strategy is a two-pass filter:

        Pass 1: Try to match both vendor_hint AND device_type (if given).
        Pass 2: Try vendor_hint only.
        Pass 3: Try device_type only.
        Pass 4: Take the first device (any will do).

        This ensures that:
        - CUDARuntime(vendor_hint="nvidia") gets an NVIDIA GPU
        - MTLDevice(vendor_hint="apple") gets an Apple device
        - VulkanRuntime() gets whatever is available

        Args:
            device_type: Preferred device type, or None for any.
            vendor_hint: Preferred vendor string, or None for any.

        Returns:
            The best matching PhysicalDevice.

        Raises:
            RuntimeError: If no devices are available at all.
        """
        if not self._physical_devices:
            raise RuntimeError("No physical devices available")

        # Pass 1: Match both vendor and type
        if vendor_hint and device_type:
            for dev in self._physical_devices:
                if dev.vendor == vendor_hint and dev.device_type == device_type:
                    return dev

        # Pass 2: Match vendor only
        if vendor_hint:
            for dev in self._physical_devices:
                if dev.vendor == vendor_hint:
                    return dev

        # Pass 3: Match device type only
        if device_type:
            for dev in self._physical_devices:
                if dev.device_type == device_type:
                    return dev

        # Pass 4: Take whatever is available
        return self._physical_devices[0]

    def _create_and_submit_cb(
        self,
        record_fn: Callable[[CommandBuffer], None],
        queue: CommandQueue | None = None,
    ) -> CommandBuffer:
        """Create a command buffer, record commands, submit, and wait.

        === The "Immediate Execution" Pattern ===

        APIs like CUDA and OpenGL present an "immediate" execution model
        where each API call appears to execute right away. Under the hood,
        they still use command buffers — they just hide them from you.

        This method implements that pattern:

            1. Create a new command buffer
            2. Begin recording
            3. Call record_fn(cb) to record whatever commands the caller wants
            4. End recording
            5. Submit to the queue with a fence
            6. Wait for the fence to signal (synchronous completion)
            7. Return the command buffer (for inspection/debugging)

        Args:
            record_fn: A callback that receives a CommandBuffer in RECORDING
                      state and records commands into it.
            queue:    Which queue to submit to. Defaults to self._compute_queue.

        Returns:
            The completed CommandBuffer.

        Example:
            def record_dispatch(cb):
                cb.cmd_bind_pipeline(pipeline)
                cb.cmd_dispatch(4, 1, 1)

            self._create_and_submit_cb(record_dispatch)
        """
        target_queue = queue or self._compute_queue

        # Create and begin recording
        cb = self._logical_device.create_command_buffer()
        cb.begin()

        # Let the caller record whatever commands they need
        record_fn(cb)

        # End recording and submit
        cb.end()
        fence = self._logical_device.create_fence()
        target_queue.submit([cb], fence=fence)
        fence.wait()

        return cb

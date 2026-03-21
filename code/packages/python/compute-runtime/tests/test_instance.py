"""Tests for RuntimeInstance and device discovery."""

import pytest
from device_simulator import NvidiaGPU, AppleANE, AmdGPU, GoogleTPU, IntelGPU

from compute_runtime import (
    RuntimeInstance,
    PhysicalDevice,
    LogicalDevice,
    DeviceType,
    QueueType,
    MemoryType,
)


class TestRuntimeInstance:
    def test_default_construction(self) -> None:
        instance = RuntimeInstance()
        assert instance.version == "0.1.0"

    def test_enumerate_default_devices(self) -> None:
        instance = RuntimeInstance()
        devices = instance.enumerate_physical_devices()
        assert len(devices) == 5  # NVIDIA, AMD, Google, Intel, Apple

    def test_device_names(self) -> None:
        instance = RuntimeInstance()
        devices = instance.enumerate_physical_devices()
        names = [d.name for d in devices]
        assert any("NVIDIA" in n for n in names)
        assert any("AMD" in n for n in names)
        assert any("TPU" in n or "Google" in n for n in names)
        assert any("Intel" in n for n in names)
        assert any("Apple" in n or "ANE" in n for n in names)

    def test_device_types(self) -> None:
        instance = RuntimeInstance()
        devices = instance.enumerate_physical_devices()
        types = {d.device_type for d in devices}
        assert DeviceType.GPU in types
        assert DeviceType.TPU in types
        assert DeviceType.NPU in types

    def test_device_vendors(self) -> None:
        instance = RuntimeInstance()
        devices = instance.enumerate_physical_devices()
        vendors = {d.vendor for d in devices}
        assert "nvidia" in vendors
        assert "amd" in vendors
        assert "google" in vendors
        assert "intel" in vendors
        assert "apple" in vendors

    def test_custom_devices(self) -> None:
        nvidia = NvidiaGPU(num_sms=4)
        instance = RuntimeInstance(
            devices=[(nvidia, DeviceType.GPU, "nvidia")]
        )
        devices = instance.enumerate_physical_devices()
        assert len(devices) == 1
        assert devices[0].vendor == "nvidia"

    def test_device_ids_are_unique(self) -> None:
        instance = RuntimeInstance()
        devices = instance.enumerate_physical_devices()
        ids = [d.device_id for d in devices]
        assert len(ids) == len(set(ids))


class TestPhysicalDevice:
    def test_memory_properties_discrete(self) -> None:
        """Discrete GPUs have separate VRAM and staging heaps."""
        instance = RuntimeInstance()
        nvidia = next(
            d for d in instance.enumerate_physical_devices()
            if d.vendor == "nvidia"
        )
        mem = nvidia.memory_properties
        assert not mem.is_unified
        assert len(mem.heaps) >= 2  # VRAM + staging

    def test_memory_properties_unified(self) -> None:
        """Apple has unified memory."""
        instance = RuntimeInstance()
        apple = next(
            d for d in instance.enumerate_physical_devices()
            if d.vendor == "apple"
        )
        mem = apple.memory_properties
        assert mem.is_unified
        assert len(mem.heaps) >= 1

    def test_queue_families(self) -> None:
        instance = RuntimeInstance()
        nvidia = next(
            d for d in instance.enumerate_physical_devices()
            if d.vendor == "nvidia"
        )
        families = nvidia.queue_families
        assert len(families) >= 1
        assert any(
            f.queue_type == QueueType.COMPUTE for f in families
        )

    def test_discrete_has_transfer_queue(self) -> None:
        instance = RuntimeInstance()
        nvidia = next(
            d for d in instance.enumerate_physical_devices()
            if d.vendor == "nvidia"
        )
        assert any(
            f.queue_type == QueueType.TRANSFER
            for f in nvidia.queue_families
        )

    def test_unified_no_separate_transfer(self) -> None:
        """Apple unified memory doesn't need separate transfer queue."""
        instance = RuntimeInstance()
        apple = next(
            d for d in instance.enumerate_physical_devices()
            if d.vendor == "apple"
        )
        has_transfer = any(
            f.queue_type == QueueType.TRANSFER
            for f in apple.queue_families
        )
        assert not has_transfer

    def test_supports_feature(self) -> None:
        instance = RuntimeInstance()
        nvidia = next(
            d for d in instance.enumerate_physical_devices()
            if d.vendor == "nvidia"
        )
        assert nvidia.supports_feature("fp32")
        assert not nvidia.supports_feature("unified_memory")

    def test_apple_supports_unified(self) -> None:
        instance = RuntimeInstance()
        apple = next(
            d for d in instance.enumerate_physical_devices()
            if d.vendor == "apple"
        )
        assert apple.supports_feature("unified_memory")

    def test_limits(self) -> None:
        instance = RuntimeInstance()
        nvidia = next(
            d for d in instance.enumerate_physical_devices()
            if d.vendor == "nvidia"
        )
        limits = nvidia.limits
        assert limits.max_workgroup_size[0] > 0
        assert limits.max_buffer_size > 0
        assert limits.max_push_constant_size > 0


class TestLogicalDevice:
    def test_create_logical_device(self) -> None:
        instance = RuntimeInstance()
        physical = instance.enumerate_physical_devices()[0]
        device = instance.create_logical_device(physical)
        assert device.physical_device is physical
        assert "compute" in device.queues

    def test_default_queue(self) -> None:
        instance = RuntimeInstance()
        physical = instance.enumerate_physical_devices()[0]
        device = instance.create_logical_device(physical)
        assert len(device.queues["compute"]) == 1

    def test_multiple_queues(self) -> None:
        instance = RuntimeInstance()
        physical = instance.enumerate_physical_devices()[0]
        device = instance.create_logical_device(
            physical,
            queue_requests=[{"type": "compute", "count": 3}],
        )
        assert len(device.queues["compute"]) == 3

    def test_memory_manager(self) -> None:
        instance = RuntimeInstance()
        physical = instance.enumerate_physical_devices()[0]
        device = instance.create_logical_device(physical)
        assert device.memory_manager is not None

    def test_factory_methods(self) -> None:
        instance = RuntimeInstance()
        physical = instance.enumerate_physical_devices()[0]
        device = instance.create_logical_device(physical)

        cb = device.create_command_buffer()
        assert cb is not None

        fence = device.create_fence()
        assert not fence.signaled

        sem = device.create_semaphore()
        assert not sem.signaled

        event = device.create_event()
        assert not event.signaled

    def test_create_fence_signaled(self) -> None:
        instance = RuntimeInstance()
        physical = instance.enumerate_physical_devices()[0]
        device = instance.create_logical_device(physical)
        fence = device.create_fence(signaled=True)
        assert fence.signaled

    def test_wait_idle(self) -> None:
        instance = RuntimeInstance()
        physical = instance.enumerate_physical_devices()[0]
        device = instance.create_logical_device(physical)
        device.wait_idle()  # Should not raise

    def test_reset(self) -> None:
        instance = RuntimeInstance()
        physical = instance.enumerate_physical_devices()[0]
        device = instance.create_logical_device(physical)
        device.reset()  # Should not raise

    def test_all_device_types(self) -> None:
        """Every device type should produce a valid logical device."""
        instance = RuntimeInstance()
        for physical in instance.enumerate_physical_devices():
            device = instance.create_logical_device(physical)
            assert device.physical_device.name == physical.name
            assert "compute" in device.queues

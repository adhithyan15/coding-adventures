"""Cross-device tests — same workloads on all architectures."""

import pytest

from gpu_core import limm, halt

from device_simulator import (
    NvidiaGPU,
    AmdGPU,
    GoogleTPU,
    IntelGPU,
    AppleANE,
    KernelDescriptor,
    DeviceConfig,
    DeviceTrace,
    AcceleratorDevice,
)


# =========================================================================
# Helpers
# =========================================================================


def all_gpu_devices() -> dict[str, object]:
    """Create one of each GPU-style device (small configs for testing)."""
    return {
        "NVIDIA": NvidiaGPU(num_sms=2),
        "AMD": AmdGPU(num_cus=2),
        "Intel": IntelGPU(num_cores=2),
    }


def all_dataflow_devices() -> dict[str, object]:
    """Create one of each dataflow device (small configs for testing)."""
    return {
        "TPU": GoogleTPU(mxu_size=2),
        "ANE": AppleANE(num_cores=2),
    }


def all_devices() -> dict[str, object]:
    """Create one of every device type."""
    devices = all_gpu_devices()
    devices.update(all_dataflow_devices())
    return devices


# =========================================================================
# Basic lifecycle tests
# =========================================================================


class TestAllStartIdle:
    def test_all_devices_start_idle(self) -> None:
        for name, device in all_devices().items():
            assert device.idle, f"{name} should start idle"


class TestAllHaveNames:
    def test_all_have_non_empty_names(self) -> None:
        for name, device in all_devices().items():
            assert device.name, f"{name} should have a name"


class TestAllHaveComputeUnits:
    def test_all_have_compute_units(self) -> None:
        for name, device in all_devices().items():
            cus = device.compute_units
            assert len(cus) > 0, f"{name} should have compute units"


class TestAllCanStep:
    def test_all_can_step_when_idle(self) -> None:
        """Every device should be able to step even when idle."""
        for name, device in all_devices().items():
            trace = device.step()
            assert trace.cycle > 0, f"{name} step should produce a trace"


class TestAllCanReset:
    def test_all_reset_to_idle(self) -> None:
        for name, device in all_devices().items():
            device.step()
            device.step()
            device.reset()
            assert device.idle, f"{name} should be idle after reset"


# =========================================================================
# GPU-style kernel execution
# =========================================================================


class TestGPUKernelExecution:
    def test_all_gpus_run_simple_kernel(self) -> None:
        """All GPU-style devices should run a simple program to completion."""
        for name, device in all_gpu_devices().items():
            kernel = KernelDescriptor(
                name="test_simple",
                program=[limm(0, 42.0), halt()],
                grid_dim=(2, 1, 1),
                block_dim=(32, 1, 1),
            )
            device.launch_kernel(kernel)
            traces = device.run(2000)
            assert len(traces) > 0, f"{name}: should produce traces"
            assert device.idle, f"{name}: should be idle after completion"


# =========================================================================
# Dataflow-style execution
# =========================================================================


class TestDataflowExecution:
    def test_all_dataflow_run_matmul(self) -> None:
        """TPU and ANE should process a matrix operation to completion."""
        for name, device in all_dataflow_devices().items():
            kernel = KernelDescriptor(
                name="matmul",
                operation="matmul",
                input_data=[[1.0, 2.0], [3.0, 4.0]],
                weight_data=[[5.0, 6.0], [7.0, 8.0]],
            )
            device.launch_kernel(kernel)
            traces = device.run(1000)
            assert len(traces) > 0, f"{name}: should produce traces"
            assert device.idle, f"{name}: should be idle after matmul"


# =========================================================================
# Memory management
# =========================================================================


class TestAllMemoryOps:
    def test_all_can_malloc_and_free(self) -> None:
        for name, device in all_devices().items():
            addr = device.malloc(256)
            assert addr >= 0, f"{name}: malloc should return valid address"
            device.free(addr)

    def test_all_can_transfer_data(self) -> None:
        for name, device in all_devices().items():
            addr = device.malloc(64)
            cycles_h2d = device.memcpy_host_to_device(addr, b"\x42" * 64)
            data, cycles_d2h = device.memcpy_device_to_host(addr, 64)
            assert data == b"\x42" * 64, f"{name}: data should round-trip"

    def test_unified_vs_discrete_transfer_cost(self) -> None:
        """Apple ANE should have zero-cost transfers. Others should not."""
        ane = AppleANE(num_cores=2)
        nvidia = NvidiaGPU(num_sms=2)

        ane_addr = ane.malloc(256)
        nvidia_addr = nvidia.malloc(256)

        ane_cycles = ane.memcpy_host_to_device(ane_addr, b"\x00" * 256)
        nvidia_cycles = nvidia.memcpy_host_to_device(nvidia_addr, b"\x00" * 256)

        assert ane_cycles == 0, "ANE unified memory should be zero-cost"
        assert nvidia_cycles > 0, "NVIDIA discrete should have transfer cost"


# =========================================================================
# Stats
# =========================================================================


class TestAllStats:
    def test_all_track_kernels(self) -> None:
        for name, device in all_devices().items():
            if name in ("NVIDIA", "AMD", "Intel"):
                kernel = KernelDescriptor(
                    name="test",
                    program=[limm(0, 1.0), halt()],
                    grid_dim=(1, 1, 1),
                    block_dim=(32, 1, 1),
                )
            else:
                kernel = KernelDescriptor(
                    name="test",
                    operation="matmul",
                    input_data=[[1.0]],
                    weight_data=[[1.0]],
                )
            device.launch_kernel(kernel)
            device.run(1000)
            stats = device.stats
            assert stats.total_kernels_launched == 1, (
                f"{name}: should track kernel launches"
            )


# =========================================================================
# Trace format
# =========================================================================


class TestAllTraceFormat:
    def test_all_produce_readable_traces(self) -> None:
        for name, device in all_devices().items():
            trace = device.step()
            formatted = trace.format()
            assert isinstance(formatted, str), f"{name}: format() should return str"
            assert len(formatted) > 0, f"{name}: format() should be non-empty"

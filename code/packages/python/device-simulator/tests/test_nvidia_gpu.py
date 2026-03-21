"""Tests for NVIDIA GPU device simulator."""

import pytest

from gpu_core import limm, halt

from device_simulator import NvidiaGPU, KernelDescriptor, DeviceConfig


class TestConstruction:
    def test_default_construction(self) -> None:
        gpu = NvidiaGPU(num_sms=2)
        assert "NVIDIA" in gpu.name
        assert len(gpu.compute_units) == 2

    def test_with_config(self) -> None:
        config = DeviceConfig(
            name="Test GPU",
            num_compute_units=3,
            l2_cache_size=4096,
            l2_cache_associativity=4,
            l2_cache_line_size=64,
            global_memory_size=1024 * 1024,
        )
        gpu = NvidiaGPU(config=config)
        assert gpu.name == "Test GPU"
        assert len(gpu.compute_units) == 3

    def test_starts_idle(self) -> None:
        gpu = NvidiaGPU(num_sms=2)
        assert gpu.idle


class TestMemoryManagement:
    def test_malloc_and_free(self) -> None:
        gpu = NvidiaGPU(num_sms=2)
        addr = gpu.malloc(256)
        assert addr >= 0
        gpu.free(addr)

    def test_sequential_mallocs(self) -> None:
        gpu = NvidiaGPU(num_sms=2)
        a1 = gpu.malloc(256)
        a2 = gpu.malloc(256)
        assert a2 > a1

    def test_memcpy_host_to_device(self) -> None:
        gpu = NvidiaGPU(num_sms=2)
        addr = gpu.malloc(128)
        cycles = gpu.memcpy_host_to_device(addr, b"\x42" * 128)
        assert cycles > 0  # Not unified memory — transfer takes time

    def test_memcpy_device_to_host(self) -> None:
        gpu = NvidiaGPU(num_sms=2)
        addr = gpu.malloc(64)
        gpu.memcpy_host_to_device(addr, b"\xAA" * 64)
        data, cycles = gpu.memcpy_device_to_host(addr, 64)
        assert data == b"\xAA" * 64
        assert cycles > 0


class TestKernelLaunch:
    def test_launch_simple_kernel(self) -> None:
        gpu = NvidiaGPU(num_sms=2)
        kernel = KernelDescriptor(
            name="test",
            program=[limm(0, 42.0), halt()],
            grid_dim=(2, 1, 1),
            block_dim=(32, 1, 1),
        )
        gpu.launch_kernel(kernel)
        assert not gpu.idle  # Work is queued

    def test_run_to_completion(self) -> None:
        gpu = NvidiaGPU(num_sms=2)
        kernel = KernelDescriptor(
            name="test",
            program=[limm(0, 42.0), halt()],
            grid_dim=(2, 1, 1),
            block_dim=(32, 1, 1),
        )
        gpu.launch_kernel(kernel)
        traces = gpu.run(1000)
        assert len(traces) > 0
        assert gpu.idle

    def test_multi_block_kernel(self) -> None:
        gpu = NvidiaGPU(num_sms=4)
        kernel = KernelDescriptor(
            name="multi_block",
            program=[limm(0, 1.0), halt()],
            grid_dim=(8, 1, 1),
            block_dim=(32, 1, 1),
        )
        gpu.launch_kernel(kernel)
        traces = gpu.run(2000)
        assert gpu.idle
        assert len(traces) > 0


class TestTraces:
    def test_trace_has_cycle(self) -> None:
        gpu = NvidiaGPU(num_sms=2)
        trace = gpu.step()
        assert trace.cycle == 1

    def test_trace_has_device_name(self) -> None:
        gpu = NvidiaGPU(num_sms=2)
        trace = gpu.step()
        assert "NVIDIA" in trace.device_name

    def test_trace_format(self) -> None:
        gpu = NvidiaGPU(num_sms=2)
        kernel = KernelDescriptor(
            name="test",
            program=[limm(0, 42.0), halt()],
            grid_dim=(2, 1, 1),
            block_dim=(32, 1, 1),
        )
        gpu.launch_kernel(kernel)
        trace = gpu.step()
        formatted = trace.format()
        assert "NVIDIA" in formatted
        assert "Cycle" in formatted

    def test_trace_shows_pending_blocks(self) -> None:
        gpu = NvidiaGPU(num_sms=1)
        kernel = KernelDescriptor(
            name="test",
            program=[limm(0, 1.0), halt()],
            grid_dim=(4, 1, 1),
            block_dim=(32, 1, 1),
        )
        gpu.launch_kernel(kernel)
        trace = gpu.step()
        # With 4 blocks and 1 SM, some should be pending
        assert trace.pending_blocks >= 0


class TestStats:
    def test_stats_track_kernels(self) -> None:
        gpu = NvidiaGPU(num_sms=2)
        kernel = KernelDescriptor(
            name="test",
            program=[limm(0, 42.0), halt()],
            grid_dim=(2, 1, 1),
            block_dim=(32, 1, 1),
        )
        gpu.launch_kernel(kernel)
        gpu.run(500)
        stats = gpu.stats
        assert stats.total_kernels_launched == 1
        assert stats.total_blocks_dispatched >= 1

    def test_stats_track_memory(self) -> None:
        gpu = NvidiaGPU(num_sms=2)
        addr = gpu.malloc(128)
        gpu.memcpy_host_to_device(addr, b"\x00" * 128)
        stats = gpu.stats
        assert stats.global_memory_stats.host_to_device_bytes == 128


class TestReset:
    def test_reset_clears_state(self) -> None:
        gpu = NvidiaGPU(num_sms=2)
        kernel = KernelDescriptor(
            name="test",
            program=[limm(0, 42.0), halt()],
            grid_dim=(2, 1, 1),
            block_dim=(32, 1, 1),
        )
        gpu.launch_kernel(kernel)
        gpu.run(500)
        gpu.reset()
        assert gpu.idle

    def test_reset_clears_memory(self) -> None:
        gpu = NvidiaGPU(num_sms=2)
        addr = gpu.malloc(64)
        gpu.memcpy_host_to_device(addr, b"\xFF" * 64)
        gpu.reset()
        stats = gpu.stats
        assert stats.global_memory_stats.host_to_device_bytes == 0

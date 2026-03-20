"""Tests for Intel GPU device simulator."""

from gpu_core import limm, halt

from device_simulator import IntelGPU, KernelDescriptor, IntelGPUConfig, XeSliceConfig


class TestConstruction:
    def test_default_construction(self) -> None:
        gpu = IntelGPU(num_cores=4)
        assert "Intel" in gpu.name
        assert len(gpu.compute_units) == 4

    def test_with_config(self) -> None:
        config = IntelGPUConfig(
            name="Test Intel",
            num_compute_units=4,
            l2_cache_size=4096,
            l2_cache_associativity=4,
            l2_cache_line_size=64,
            global_memory_size=1024 * 1024,
            num_xe_slices=2,
            slice_config=XeSliceConfig(xe_cores_per_slice=2),
        )
        gpu = IntelGPU(config=config)
        assert gpu.name == "Test Intel"
        assert len(gpu.xe_slices) == 2

    def test_starts_idle(self) -> None:
        gpu = IntelGPU(num_cores=2)
        assert gpu.idle

    def test_xe_slice_grouping(self) -> None:
        config = IntelGPUConfig(
            name="Test Intel",
            num_compute_units=8,
            l2_cache_size=4096,
            l2_cache_associativity=4,
            l2_cache_line_size=64,
            global_memory_size=1024 * 1024,
            num_xe_slices=4,
            slice_config=XeSliceConfig(xe_cores_per_slice=2),
        )
        gpu = IntelGPU(config=config)
        assert len(gpu.xe_slices) == 4
        for s in gpu.xe_slices:
            assert len(s.xe_cores) == 2


class TestKernelExecution:
    def test_launch_and_run(self) -> None:
        gpu = IntelGPU(num_cores=2)
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

    def test_multi_block(self) -> None:
        gpu = IntelGPU(num_cores=4)
        kernel = KernelDescriptor(
            name="multi",
            program=[limm(0, 1.0), halt()],
            grid_dim=(4, 1, 1),
            block_dim=(32, 1, 1),
        )
        gpu.launch_kernel(kernel)
        traces = gpu.run(2000)
        assert gpu.idle


class TestMemory:
    def test_malloc_and_transfer(self) -> None:
        gpu = IntelGPU(num_cores=2)
        addr = gpu.malloc(256)
        cycles = gpu.memcpy_host_to_device(addr, b"\x42" * 256)
        assert cycles > 0
        data, _ = gpu.memcpy_device_to_host(addr, 256)
        assert data == b"\x42" * 256


class TestTraces:
    def test_trace_format(self) -> None:
        gpu = IntelGPU(num_cores=2)
        trace = gpu.step()
        formatted = trace.format()
        assert "Intel" in formatted

    def test_xe_slice_idle(self) -> None:
        config = IntelGPUConfig(
            name="Test Intel",
            num_compute_units=4,
            l2_cache_size=4096,
            l2_cache_associativity=4,
            l2_cache_line_size=64,
            global_memory_size=1024 * 1024,
            num_xe_slices=2,
            slice_config=XeSliceConfig(xe_cores_per_slice=2),
        )
        gpu = IntelGPU(config=config)
        for s in gpu.xe_slices:
            assert s.idle


class TestReset:
    def test_reset(self) -> None:
        gpu = IntelGPU(num_cores=2)
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

    def test_stats(self) -> None:
        gpu = IntelGPU(num_cores=2)
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

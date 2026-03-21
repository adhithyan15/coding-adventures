"""Tests for AMD GPU device simulator."""

from gpu_core import limm, halt

from device_simulator import AmdGPU, KernelDescriptor, AmdGPUConfig, ShaderEngineConfig


class TestConstruction:
    def test_default_construction(self) -> None:
        gpu = AmdGPU(num_cus=4)
        assert "AMD" in gpu.name
        assert len(gpu.compute_units) == 4

    def test_with_amd_config(self) -> None:
        config = AmdGPUConfig(
            name="Test AMD",
            num_compute_units=4,
            l2_cache_size=4096,
            l2_cache_associativity=4,
            l2_cache_line_size=64,
            global_memory_size=1024 * 1024,
            num_shader_engines=2,
            se_config=ShaderEngineConfig(cus_per_engine=2),
        )
        gpu = AmdGPU(config=config)
        assert gpu.name == "Test AMD"
        assert len(gpu.shader_engines) == 2
        assert len(gpu.compute_units) == 4

    def test_starts_idle(self) -> None:
        gpu = AmdGPU(num_cus=2)
        assert gpu.idle

    def test_shader_engine_grouping(self) -> None:
        config = AmdGPUConfig(
            name="Test AMD",
            num_compute_units=6,
            l2_cache_size=4096,
            l2_cache_associativity=4,
            l2_cache_line_size=64,
            global_memory_size=1024 * 1024,
            num_shader_engines=3,
            se_config=ShaderEngineConfig(cus_per_engine=2),
        )
        gpu = AmdGPU(config=config)
        assert len(gpu.shader_engines) == 3
        for se in gpu.shader_engines:
            assert len(se.cus) == 2


class TestKernelExecution:
    def test_launch_and_run(self) -> None:
        gpu = AmdGPU(num_cus=2)
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
        gpu = AmdGPU(num_cus=4)
        kernel = KernelDescriptor(
            name="multi_block",
            program=[limm(0, 1.0), halt()],
            grid_dim=(4, 1, 1),
            block_dim=(32, 1, 1),
        )
        gpu.launch_kernel(kernel)
        traces = gpu.run(2000)
        assert gpu.idle


class TestMemory:
    def test_malloc_and_transfer(self) -> None:
        gpu = AmdGPU(num_cus=2)
        addr = gpu.malloc(256)
        cycles = gpu.memcpy_host_to_device(addr, b"\x42" * 256)
        assert cycles > 0
        data, _ = gpu.memcpy_device_to_host(addr, 256)
        assert data == b"\x42" * 256


class TestTraces:
    def test_trace_format(self) -> None:
        gpu = AmdGPU(num_cus=2)
        trace = gpu.step()
        formatted = trace.format()
        assert "AMD" in formatted

    def test_shader_engine_idle(self) -> None:
        config = AmdGPUConfig(
            name="Test AMD",
            num_compute_units=4,
            l2_cache_size=4096,
            l2_cache_associativity=4,
            l2_cache_line_size=64,
            global_memory_size=1024 * 1024,
            num_shader_engines=2,
            se_config=ShaderEngineConfig(cus_per_engine=2),
        )
        gpu = AmdGPU(config=config)
        for se in gpu.shader_engines:
            assert se.idle


class TestReset:
    def test_reset(self) -> None:
        gpu = AmdGPU(num_cus=2)
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
        gpu = AmdGPU(num_cus=2)
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

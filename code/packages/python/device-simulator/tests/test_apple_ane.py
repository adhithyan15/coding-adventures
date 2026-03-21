"""Tests for Apple ANE device simulator."""

from device_simulator import AppleANE, KernelDescriptor, ANEConfig


class TestConstruction:
    def test_default_construction(self) -> None:
        ane = AppleANE(num_cores=4)
        assert "Apple" in ane.name
        assert len(ane.compute_units) == 4

    def test_with_config(self) -> None:
        config = ANEConfig(
            name="Test ANE",
            num_compute_units=8,
            global_memory_size=1024 * 1024,
            unified_memory=True,
            host_latency=0,
        )
        ane = AppleANE(config=config)
        assert ane.name == "Test ANE"
        assert len(ane.compute_units) == 8

    def test_starts_idle(self) -> None:
        ane = AppleANE(num_cores=4)
        assert ane.idle

    def test_unified_memory(self) -> None:
        ane = AppleANE(num_cores=4)
        assert ane.is_unified_memory


class TestUnifiedMemory:
    def test_zero_copy_host_to_device(self) -> None:
        """Apple's unified memory: transfers cost zero cycles."""
        ane = AppleANE(num_cores=4)
        addr = ane.malloc(256)
        cycles = ane.memcpy_host_to_device(addr, b"\x42" * 256)
        assert cycles == 0  # Zero-copy!

    def test_zero_copy_device_to_host(self) -> None:
        ane = AppleANE(num_cores=4)
        addr = ane.malloc(64)
        ane.memcpy_host_to_device(addr, b"\xAA" * 64)
        data, cycles = ane.memcpy_device_to_host(addr, 64)
        assert data == b"\xAA" * 64
        assert cycles == 0  # Zero-copy!

    def test_data_persists_after_zero_copy(self) -> None:
        ane = AppleANE(num_cores=4)
        addr = ane.malloc(128)
        ane.memcpy_host_to_device(addr, b"\xFF" * 128)
        data, _ = ane.memcpy_device_to_host(addr, 128)
        assert data == b"\xFF" * 128


class TestInferenceExecution:
    def test_launch_inference(self) -> None:
        ane = AppleANE(num_cores=2)
        kernel = KernelDescriptor(
            name="conv2d",
            operation="conv2d",
            input_data=[[1.0, 2.0], [3.0, 4.0]],
            weight_data=[[0.5, 0.5], [0.5, 0.5]],
        )
        ane.launch_kernel(kernel)
        assert not ane.idle

    def test_run_to_completion(self) -> None:
        ane = AppleANE(num_cores=2)
        kernel = KernelDescriptor(
            name="inference",
            operation="matmul",
            input_data=[[1.0, 2.0], [3.0, 4.0]],
            weight_data=[[5.0, 6.0], [7.0, 8.0]],
        )
        ane.launch_kernel(kernel)
        traces = ane.run(500)
        assert len(traces) > 0
        assert ane.idle

    def test_schedule_replay(self) -> None:
        """ANE uses compiler-generated schedule, not dynamic scheduling."""
        ane = AppleANE(num_cores=4)
        kernel = KernelDescriptor(
            name="inference",
            operation="matmul",
            input_data=[[1.0]],
            weight_data=[[1.0]],
        )
        ane.launch_kernel(kernel)
        trace = ane.step()
        # Schedule replayer should produce actions
        assert trace.distributor_actions


class TestTraces:
    def test_trace_format(self) -> None:
        ane = AppleANE(num_cores=2)
        trace = ane.step()
        formatted = trace.format()
        assert "Apple" in formatted

    def test_trace_active_blocks(self) -> None:
        ane = AppleANE(num_cores=4)
        trace = ane.step()
        # Idle — no active blocks
        assert trace.active_blocks >= 0


class TestReset:
    def test_reset(self) -> None:
        ane = AppleANE(num_cores=2)
        kernel = KernelDescriptor(
            name="test",
            operation="matmul",
            input_data=[[1.0]],
            weight_data=[[1.0]],
        )
        ane.launch_kernel(kernel)
        ane.run(500)
        ane.reset()
        assert ane.idle

    def test_stats(self) -> None:
        ane = AppleANE(num_cores=2)
        kernel = KernelDescriptor(
            name="test",
            operation="matmul",
            input_data=[[1.0]],
            weight_data=[[1.0]],
        )
        ane.launch_kernel(kernel)
        ane.run(500)
        stats = ane.stats
        assert stats.total_kernels_launched == 1

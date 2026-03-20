"""Tests for Google TPU device simulator."""

from device_simulator import GoogleTPU, KernelDescriptor, TPUConfig


class TestConstruction:
    def test_default_construction(self) -> None:
        tpu = GoogleTPU(mxu_size=4)
        assert "TPU" in tpu.name
        assert len(tpu.compute_units) == 1  # One MXU

    def test_with_config(self) -> None:
        config = TPUConfig(
            name="Test TPU",
            num_compute_units=1,
            global_memory_size=1024 * 1024,
            vector_unit_width=4,
        )
        tpu = GoogleTPU(config=config)
        assert tpu.name == "Test TPU"

    def test_starts_idle(self) -> None:
        tpu = GoogleTPU(mxu_size=4)
        assert tpu.idle


class TestMatmulExecution:
    def test_launch_matmul(self) -> None:
        tpu = GoogleTPU(mxu_size=2)
        kernel = KernelDescriptor(
            name="matmul",
            operation="matmul",
            input_data=[[1.0, 2.0], [3.0, 4.0]],
            weight_data=[[5.0, 6.0], [7.0, 8.0]],
        )
        tpu.launch_kernel(kernel)
        assert not tpu.idle

    def test_run_matmul_to_completion(self) -> None:
        tpu = GoogleTPU(mxu_size=2)
        kernel = KernelDescriptor(
            name="matmul",
            operation="matmul",
            input_data=[[1.0, 2.0], [3.0, 4.0]],
            weight_data=[[5.0, 6.0], [7.0, 8.0]],
        )
        tpu.launch_kernel(kernel)
        traces = tpu.run(500)
        assert len(traces) > 0
        assert tpu.idle

    def test_large_matmul_tiles(self) -> None:
        """Input larger than MXU should be tiled."""
        tpu = GoogleTPU(mxu_size=2)
        # 4x4 input with MXU size 2 → 4 tiles
        kernel = KernelDescriptor(
            name="big_matmul",
            operation="matmul",
            input_data=[[1.0] * 4 for _ in range(4)],
            weight_data=[[1.0] * 4 for _ in range(4)],
        )
        tpu.launch_kernel(kernel)
        traces = tpu.run(1000)
        assert tpu.idle


class TestMemory:
    def test_malloc_and_transfer(self) -> None:
        tpu = GoogleTPU(mxu_size=4)
        addr = tpu.malloc(256)
        cycles = tpu.memcpy_host_to_device(addr, b"\x00" * 256)
        assert cycles > 0  # Not unified memory


class TestTraces:
    def test_trace_has_pipeline_actions(self) -> None:
        tpu = GoogleTPU(mxu_size=2)
        kernel = KernelDescriptor(
            name="matmul",
            operation="matmul",
            input_data=[[1.0, 2.0], [3.0, 4.0]],
            weight_data=[[5.0, 6.0], [7.0, 8.0]],
        )
        tpu.launch_kernel(kernel)
        trace = tpu.step()
        # Should show pipeline activity
        assert trace.distributor_actions  # Scalar/MXU/Vector actions

    def test_trace_format(self) -> None:
        tpu = GoogleTPU(mxu_size=2)
        trace = tpu.step()
        formatted = trace.format()
        assert "TPU" in formatted


class TestReset:
    def test_reset(self) -> None:
        tpu = GoogleTPU(mxu_size=2)
        kernel = KernelDescriptor(
            name="matmul",
            operation="matmul",
            input_data=[[1.0]],
            weight_data=[[1.0]],
        )
        tpu.launch_kernel(kernel)
        tpu.run(500)
        tpu.reset()
        assert tpu.idle

    def test_stats(self) -> None:
        tpu = GoogleTPU(mxu_size=2)
        kernel = KernelDescriptor(
            name="matmul",
            operation="matmul",
            input_data=[[1.0]],
            weight_data=[[1.0]],
        )
        tpu.launch_kernel(kernel)
        tpu.run(500)
        stats = tpu.stats
        assert stats.total_kernels_launched == 1

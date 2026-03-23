"""Tests for NeuralEngineCore — Apple ANE Core simulator."""

from __future__ import annotations

from clock import Clock

from compute_unit import Architecture, WorkItem
from compute_unit.neural_engine_core import ANECoreConfig, NeuralEngineCore

# ---------------------------------------------------------------------------
# ANECoreConfig tests
# ---------------------------------------------------------------------------


class TestANECoreConfig:
    """Test ANECoreConfig defaults and customization."""

    def test_defaults(self) -> None:
        config = ANECoreConfig()
        assert config.num_macs == 16
        assert config.sram_size == 4194304
        assert config.activation_buffer == 131072
        assert config.weight_buffer == 524288
        assert config.output_buffer == 131072
        assert config.dma_bandwidth == 10

    def test_custom_config(self) -> None:
        config = ANECoreConfig(num_macs=8, dma_bandwidth=20)
        assert config.num_macs == 8
        assert config.dma_bandwidth == 20


# ---------------------------------------------------------------------------
# NeuralEngineCore tests
# ---------------------------------------------------------------------------


class TestNeuralEngineCore:
    """Test the Apple ANE Core simulator."""

    def test_creation(self) -> None:
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(num_macs=4), clock)
        assert ane.name == "ANECore"
        assert ane.architecture == Architecture.APPLE_ANE_CORE
        assert ane.idle

    def test_simple_inference_dot_product(self) -> None:
        """Simple dot product: [1, 2, 3, 4] . [0.5, 0.5, 0.5, 0.5] = 5.0"""
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(num_macs=4), clock)
        result = ane.run_inference(
            inputs=[[1.0, 2.0, 3.0, 4.0]],
            weights=[[0.5], [0.5], [0.5], [0.5]],
            activation_fn="none",
        )
        assert len(result) == 1
        assert abs(result[0][0] - 5.0) < 0.01

    def test_matmul_2x2(self) -> None:
        """2x2 matrix multiply."""
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(), clock)
        result = ane.run_inference(
            inputs=[[1.0, 2.0], [3.0, 4.0]],
            weights=[[5.0, 6.0], [7.0, 8.0]],
            activation_fn="none",
        )
        assert abs(result[0][0] - 19.0) < 0.01
        assert abs(result[0][1] - 22.0) < 0.01
        assert abs(result[1][0] - 43.0) < 0.01
        assert abs(result[1][1] - 50.0) < 0.01

    def test_relu_activation(self) -> None:
        """ReLU: max(0, x)."""
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(), clock)
        result = ane.run_inference(
            inputs=[[1.0, -2.0]],
            weights=[[1.0], [1.0]],
            activation_fn="relu",
        )
        # 1*1 + (-2)*1 = -1, ReLU(-1) = 0
        assert abs(result[0][0]) < 0.01

    def test_relu_passes_positive(self) -> None:
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(), clock)
        result = ane.run_inference(
            inputs=[[3.0, 2.0]],
            weights=[[1.0], [1.0]],
            activation_fn="relu",
        )
        # 3+2=5, ReLU(5)=5
        assert abs(result[0][0] - 5.0) < 0.01

    def test_sigmoid_activation(self) -> None:
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(), clock)
        result = ane.run_inference(
            inputs=[[0.0]],
            weights=[[1.0]],
            activation_fn="sigmoid",
        )
        assert abs(result[0][0] - 0.5) < 0.01

    def test_tanh_activation(self) -> None:
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(), clock)
        result = ane.run_inference(
            inputs=[[0.0]],
            weights=[[1.0]],
            activation_fn="tanh",
        )
        assert abs(result[0][0]) < 0.01

    def test_sigmoid_large_positive(self) -> None:
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(), clock)
        result = ane.run_inference(
            inputs=[[100.0]],
            weights=[[1.0]],
            activation_fn="sigmoid",
        )
        assert result[0][0] > 0.99

    def test_sigmoid_large_negative(self) -> None:
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(), clock)
        result = ane.run_inference(
            inputs=[[-100.0]],
            weights=[[1.0]],
            activation_fn="sigmoid",
        )
        assert result[0][0] < 0.01

    def test_dispatch_and_run(self) -> None:
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(), clock)
        ane.dispatch(WorkItem(
            work_id=0,
            input_data=[[1.0, 2.0]],
            weight_data=[[3.0], [4.0]],
        ))
        traces = ane.run()
        assert len(traces) > 0
        assert ane.idle
        # 1*3 + 2*4 = 11
        assert abs(ane.result[0][0] - 11.0) < 0.01

    def test_dispatch_without_data(self) -> None:
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(), clock)
        ane.dispatch(WorkItem(work_id=0))
        ane.run()
        assert ane.idle
        assert ane.result == []

    def test_traces_have_correct_architecture(self) -> None:
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(), clock)
        ane.dispatch(WorkItem(
            work_id=0,
            input_data=[[1.0]],
            weight_data=[[2.0]],
        ))
        traces = ane.run()
        for trace in traces:
            assert trace.architecture == Architecture.APPLE_ANE_CORE
            assert trace.unit_name == "ANECore"

    def test_idle_trace(self) -> None:
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(), clock)
        from clock import ClockEdge

        trace = ane.step(
            ClockEdge(cycle=1, value=1, is_rising=True, is_falling=False)
        )
        assert trace.scheduler_action == "idle"
        assert trace.occupancy == 0.0

    def test_multiple_dispatches(self) -> None:
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(), clock)
        ane.dispatch(WorkItem(
            work_id=0,
            input_data=[[1.0]],
            weight_data=[[2.0]],
        ))
        ane.dispatch(WorkItem(
            work_id=1,
            input_data=[[3.0]],
            weight_data=[[4.0]],
        ))
        ane.run()
        assert ane.idle

    def test_mac_engine_accessible(self) -> None:
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(), clock)
        assert ane.mac_engine is not None

    def test_reset(self) -> None:
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(), clock)
        ane.run_inference([[1.0]], [[2.0]], activation_fn="relu")
        ane.reset()
        assert ane.idle
        assert ane.result == []

    def test_repr(self) -> None:
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(num_macs=8), clock)
        r = repr(ane)
        assert "NeuralEngineCore" in r
        assert "macs=8" in r

    def test_unknown_activation(self) -> None:
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(), clock)
        result = ane.run_inference(
            inputs=[[5.0]],
            weights=[[1.0]],
            activation_fn="unknown",
        )
        assert abs(result[0][0] - 5.0) < 0.01

    def test_3x2_matmul(self) -> None:
        """Non-square matmul."""
        clock = Clock()
        ane = NeuralEngineCore(ANECoreConfig(), clock)
        result = ane.run_inference(
            inputs=[[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]],
            weights=[[7.0], [8.0]],
            activation_fn="none",
        )
        assert len(result) == 3
        assert abs(result[0][0] - 23.0) < 0.01  # 1*7+2*8
        assert abs(result[1][0] - 53.0) < 0.01  # 3*7+4*8
        assert abs(result[2][0] - 83.0) < 0.01  # 5*7+6*8

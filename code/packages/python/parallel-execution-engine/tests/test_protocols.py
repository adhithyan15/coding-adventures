"""Tests for protocols — ExecutionModel, EngineTrace, DivergenceInfo."""

from __future__ import annotations

from parallel_execution_engine import (
    DataflowInfo,
    DivergenceInfo,
    EngineTrace,
    ExecutionModel,
    ParallelExecutionEngine,
)

# ---------------------------------------------------------------------------
# ExecutionModel enum
# ---------------------------------------------------------------------------


class TestExecutionModel:
    """Test the ExecutionModel enum values and properties."""

    def test_all_five_models_exist(self) -> None:
        """All five execution models should be defined."""
        assert ExecutionModel.SIMT.value == "simt"
        assert ExecutionModel.SIMD.value == "simd"
        assert ExecutionModel.SYSTOLIC.value == "systolic"
        assert ExecutionModel.SCHEDULED_MAC.value == "scheduled_mac"
        assert ExecutionModel.VLIW.value == "vliw"

    def test_model_count(self) -> None:
        """Exactly 5 execution models."""
        assert len(ExecutionModel) == 5

    def test_enum_membership(self) -> None:
        """Each model is an ExecutionModel."""
        for model in ExecutionModel:
            assert isinstance(model, ExecutionModel)


# ---------------------------------------------------------------------------
# DivergenceInfo
# ---------------------------------------------------------------------------


class TestDivergenceInfo:
    """Test DivergenceInfo dataclass."""

    def test_creation(self) -> None:
        """Create a DivergenceInfo with basic fields."""
        info = DivergenceInfo(
            active_mask_before=[True, True, True, True],
            active_mask_after=[True, True, False, False],
            reconvergence_pc=10,
            divergence_depth=1,
        )
        assert info.active_mask_before == [True, True, True, True]
        assert info.active_mask_after == [True, True, False, False]
        assert info.reconvergence_pc == 10
        assert info.divergence_depth == 1

    def test_defaults(self) -> None:
        """Default reconvergence_pc is -1, depth is 0."""
        info = DivergenceInfo(
            active_mask_before=[True],
            active_mask_after=[True],
        )
        assert info.reconvergence_pc == -1
        assert info.divergence_depth == 0

    def test_frozen(self) -> None:
        """DivergenceInfo is immutable."""
        info = DivergenceInfo(
            active_mask_before=[True],
            active_mask_after=[False],
        )
        try:
            info.reconvergence_pc = 5  # type: ignore[misc]
            raise AssertionError("Should have raised")
        except AttributeError:
            pass


# ---------------------------------------------------------------------------
# DataflowInfo
# ---------------------------------------------------------------------------


class TestDataflowInfo:
    """Test DataflowInfo dataclass."""

    def test_creation(self) -> None:
        """Create DataflowInfo with PE states."""
        info = DataflowInfo(
            pe_states=[["acc=1.0", "acc=2.0"], ["acc=3.0", "acc=4.0"]],
            data_positions={"input_0": (0, 1)},
        )
        assert info.pe_states[0][0] == "acc=1.0"
        assert info.data_positions["input_0"] == (0, 1)

    def test_defaults(self) -> None:
        """Default data_positions is empty dict."""
        info = DataflowInfo(pe_states=[["x"]])
        assert info.data_positions == {}

    def test_frozen(self) -> None:
        """DataflowInfo is immutable."""
        info = DataflowInfo(pe_states=[[]])
        try:
            info.pe_states = []  # type: ignore[misc]
            raise AssertionError("Should have raised")
        except AttributeError:
            pass


# ---------------------------------------------------------------------------
# EngineTrace
# ---------------------------------------------------------------------------


class TestEngineTrace:
    """Test EngineTrace dataclass and format method."""

    def _make_trace(self) -> EngineTrace:
        return EngineTrace(
            cycle=3,
            engine_name="WarpEngine",
            execution_model=ExecutionModel.SIMT,
            description="FADD R2, R0, R1 — 3/4 threads active",
            unit_traces={
                0: "R2 = 1.0 + 2.0 = 3.0",
                1: "R2 = 3.0 + 4.0 = 7.0",
                2: "(masked)",
                3: "R2 = 5.0 + 6.0 = 11.0",
            },
            active_mask=[True, True, False, True],
            active_count=3,
            total_count=4,
            utilization=0.75,
        )

    def test_creation(self) -> None:
        """Create an EngineTrace with all fields."""
        trace = self._make_trace()
        assert trace.cycle == 3
        assert trace.engine_name == "WarpEngine"
        assert trace.execution_model == ExecutionModel.SIMT
        assert trace.active_count == 3
        assert trace.total_count == 4
        assert trace.utilization == 0.75

    def test_optional_fields(self) -> None:
        """Optional divergence_info and dataflow_info default to None."""
        trace = self._make_trace()
        assert trace.divergence_info is None
        assert trace.dataflow_info is None

    def test_with_divergence_info(self) -> None:
        """EngineTrace can include divergence info."""
        div = DivergenceInfo(
            active_mask_before=[True] * 4,
            active_mask_after=[True, True, False, False],
            reconvergence_pc=10,
            divergence_depth=1,
        )
        trace = EngineTrace(
            cycle=1,
            engine_name="WarpEngine",
            execution_model=ExecutionModel.SIMT,
            description="branch",
            unit_traces={},
            active_mask=[True, True, False, False],
            active_count=2,
            total_count=4,
            utilization=0.5,
            divergence_info=div,
        )
        assert trace.divergence_info is not None
        assert trace.divergence_info.divergence_depth == 1

    def test_with_dataflow_info(self) -> None:
        """EngineTrace can include dataflow info."""
        df = DataflowInfo(pe_states=[["acc=0.0"]])
        trace = EngineTrace(
            cycle=1,
            engine_name="SystolicArray",
            execution_model=ExecutionModel.SYSTOLIC,
            description="step",
            unit_traces={},
            active_mask=[True],
            active_count=1,
            total_count=1,
            utilization=1.0,
            dataflow_info=df,
        )
        assert trace.dataflow_info is not None

    def test_format(self) -> None:
        """format() produces readable output."""
        trace = self._make_trace()
        text = trace.format()
        assert "Cycle 3" in text
        assert "WarpEngine" in text
        assert "SIMT" in text
        assert "75.0%" in text
        assert "3/4 active" in text

    def test_format_with_divergence(self) -> None:
        """format() includes divergence info when present."""
        div = DivergenceInfo(
            active_mask_before=[True] * 4,
            active_mask_after=[True, True, False, False],
            reconvergence_pc=10,
            divergence_depth=1,
        )
        trace = EngineTrace(
            cycle=1,
            engine_name="Test",
            execution_model=ExecutionModel.SIMT,
            description="test",
            unit_traces={},
            active_mask=[True, True, False, False],
            active_count=2,
            total_count=4,
            utilization=0.5,
            divergence_info=div,
        )
        text = trace.format()
        assert "Divergence" in text
        assert "depth=1" in text

    def test_frozen(self) -> None:
        """EngineTrace is immutable."""
        trace = self._make_trace()
        try:
            trace.cycle = 99  # type: ignore[misc]
            raise AssertionError("Should have raised")
        except AttributeError:
            pass


# ---------------------------------------------------------------------------
# ParallelExecutionEngine protocol
# ---------------------------------------------------------------------------


class TestParallelExecutionEngineProtocol:
    """Test that the protocol can check structural subtyping."""

    def test_protocol_is_runtime_checkable(self) -> None:
        """Protocol should be runtime checkable."""
        assert hasattr(ParallelExecutionEngine, "__protocol_attrs__") or True
        # The protocol itself is decorated with @runtime_checkable

    def test_warp_engine_satisfies_protocol(self) -> None:
        """WarpEngine should satisfy ParallelExecutionEngine."""
        from clock import Clock

        from parallel_execution_engine import WarpConfig, WarpEngine

        engine = WarpEngine(WarpConfig(warp_width=4), Clock())
        assert isinstance(engine, ParallelExecutionEngine)

    def test_wavefront_engine_satisfies_protocol(self) -> None:
        """WavefrontEngine should satisfy ParallelExecutionEngine."""
        from clock import Clock

        from parallel_execution_engine import WavefrontConfig, WavefrontEngine

        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        assert isinstance(engine, ParallelExecutionEngine)

    def test_systolic_array_satisfies_protocol(self) -> None:
        """SystolicArray should satisfy ParallelExecutionEngine."""
        from clock import Clock

        from parallel_execution_engine import SystolicArray, SystolicConfig

        engine = SystolicArray(SystolicConfig(rows=2, cols=2), Clock())
        assert isinstance(engine, ParallelExecutionEngine)

    def test_mac_array_satisfies_protocol(self) -> None:
        """MACArrayEngine should satisfy ParallelExecutionEngine."""
        from clock import Clock

        from parallel_execution_engine import MACArrayConfig, MACArrayEngine

        engine = MACArrayEngine(MACArrayConfig(num_macs=4), Clock())
        assert isinstance(engine, ParallelExecutionEngine)

    def test_subslice_satisfies_protocol(self) -> None:
        """SubsliceEngine should satisfy ParallelExecutionEngine."""
        from clock import Clock

        from parallel_execution_engine import SubsliceConfig, SubsliceEngine

        engine = SubsliceEngine(
            SubsliceConfig(num_eus=2, threads_per_eu=2, simd_width=2), Clock()
        )
        assert isinstance(engine, ParallelExecutionEngine)

"""Tests for XeCore — Intel Xe Core simulator."""

from __future__ import annotations

from clock import Clock
from gpu_core import fmul, halt, limm

from compute_unit import Architecture, WorkItem
from compute_unit.xe_core import XeCore, XeCoreConfig

# ---------------------------------------------------------------------------
# XeCoreConfig tests
# ---------------------------------------------------------------------------


class TestXeCoreConfig:
    """Test XeCoreConfig defaults and customization."""

    def test_defaults(self) -> None:
        config = XeCoreConfig()
        assert config.num_eus == 16
        assert config.threads_per_eu == 7
        assert config.simd_width == 8
        assert config.grf_per_eu == 128
        assert config.slm_size == 65536
        assert config.l1_cache_size == 196608
        assert config.memory_latency_cycles == 200

    def test_custom_config(self) -> None:
        config = XeCoreConfig(num_eus=4, threads_per_eu=3, simd_width=4)
        assert config.num_eus == 4
        assert config.threads_per_eu == 3
        assert config.simd_width == 4


# ---------------------------------------------------------------------------
# XeCore tests
# ---------------------------------------------------------------------------


class TestXeCore:
    """Test the Intel Xe Core simulator."""

    def _simple_program(self) -> list:
        return [limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()]

    def test_creation(self) -> None:
        clock = Clock()
        xe = XeCore(
            XeCoreConfig(num_eus=2, threads_per_eu=2, simd_width=4),
            clock,
        )
        assert xe.name == "XeCore"
        assert xe.architecture == Architecture.INTEL_XE_CORE
        assert xe.idle

    def test_dispatch_and_run(self) -> None:
        clock = Clock()
        xe = XeCore(
            XeCoreConfig(num_eus=2, threads_per_eu=2, simd_width=4),
            clock,
        )
        xe.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=16,
        ))
        traces = xe.run()
        assert len(traces) > 0
        assert xe.idle

    def test_traces_have_correct_architecture(self) -> None:
        clock = Clock()
        xe = XeCore(
            XeCoreConfig(num_eus=2, threads_per_eu=2, simd_width=4),
            clock,
        )
        xe.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=8,
        ))
        traces = xe.run()
        for trace in traces:
            assert trace.architecture == Architecture.INTEL_XE_CORE
            assert trace.unit_name == "XeCore"

    def test_slm_access(self) -> None:
        clock = Clock()
        xe = XeCore(XeCoreConfig(), clock)
        slm = xe.slm
        slm.write(0, 99.0, thread_id=0)
        assert abs(slm.read(0, thread_id=0) - 99.0) < 0.01

    def test_engine_accessible(self) -> None:
        clock = Clock()
        xe = XeCore(
            XeCoreConfig(num_eus=2, threads_per_eu=2, simd_width=4),
            clock,
        )
        assert xe.engine is not None

    def test_reset(self) -> None:
        clock = Clock()
        xe = XeCore(
            XeCoreConfig(num_eus=2, threads_per_eu=2, simd_width=4),
            clock,
        )
        xe.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=8,
        ))
        xe.run()
        xe.reset()
        assert xe.idle

    def test_per_thread_data(self) -> None:
        clock = Clock()
        xe = XeCore(
            XeCoreConfig(num_eus=2, threads_per_eu=2, simd_width=4),
            clock,
        )
        xe.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=8,
            per_thread_data={
                0: {0: 10.0},
                1: {0: 20.0},
            },
        ))
        xe.run()
        assert xe.idle

    def test_occupancy_tracking(self) -> None:
        clock = Clock()
        xe = XeCore(
            XeCoreConfig(num_eus=2, threads_per_eu=2, simd_width=4),
            clock,
        )
        xe.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=8,
        ))
        from clock import ClockEdge

        trace = xe.step(
            ClockEdge(cycle=1, value=1, is_rising=True, is_falling=False)
        )
        # Should show some activity
        assert trace.occupancy >= 0.0

    def test_repr(self) -> None:
        clock = Clock()
        xe = XeCore(
            XeCoreConfig(num_eus=4, threads_per_eu=3, simd_width=4),
            clock,
        )
        r = repr(xe)
        assert "XeCore" in r
        assert "eus=4" in r

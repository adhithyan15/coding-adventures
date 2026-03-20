"""Tests for subslice_engine.py — Intel Xe hybrid SIMD execution engine."""

from __future__ import annotations

from clock import Clock, ClockEdge
from gpu_core import halt, limm

from parallel_execution_engine import (
    ExecutionModel,
    ExecutionUnit,
    SubsliceConfig,
    SubsliceEngine,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_edge(cycle: int = 1) -> ClockEdge:
    return ClockEdge(cycle=cycle, value=1, is_rising=True, is_falling=False)


# ---------------------------------------------------------------------------
# SubsliceConfig
# ---------------------------------------------------------------------------


class TestSubsliceConfig:
    def test_defaults(self) -> None:
        config = SubsliceConfig()
        assert config.num_eus == 8
        assert config.threads_per_eu == 7
        assert config.simd_width == 8
        assert config.grf_size == 128
        assert config.slm_size == 65536

    def test_custom(self) -> None:
        config = SubsliceConfig(num_eus=4, threads_per_eu=2, simd_width=4)
        assert config.num_eus == 4
        assert config.threads_per_eu == 2
        assert config.simd_width == 4


# ---------------------------------------------------------------------------
# ExecutionUnit
# ---------------------------------------------------------------------------


class TestExecutionUnit:
    def test_creation(self) -> None:
        config = SubsliceConfig(num_eus=1, threads_per_eu=2, simd_width=4)
        eu = ExecutionUnit(eu_id=0, config=config)
        assert eu.eu_id == 0
        assert len(eu.threads) == 2
        assert len(eu.threads[0]) == 4

    def test_load_program(self) -> None:
        config = SubsliceConfig(num_eus=1, threads_per_eu=2, simd_width=2)
        eu = ExecutionUnit(eu_id=0, config=config)
        eu.load_program([limm(0, 1.0), halt()])
        # All threads should be active
        assert not eu.all_halted

    def test_step(self) -> None:
        config = SubsliceConfig(num_eus=1, threads_per_eu=2, simd_width=2)
        eu = ExecutionUnit(eu_id=0, config=config)
        eu.load_program([limm(0, 1.0), halt()])
        traces = eu.step()
        assert len(traces) > 0

    def test_all_halted(self) -> None:
        config = SubsliceConfig(num_eus=1, threads_per_eu=1, simd_width=2)
        eu = ExecutionUnit(eu_id=0, config=config)
        eu.load_program([halt()])
        eu.step()
        assert eu.all_halted is True

    def test_set_thread_lane_register(self) -> None:
        config = SubsliceConfig(num_eus=1, threads_per_eu=2, simd_width=2)
        eu = ExecutionUnit(eu_id=0, config=config)
        eu.load_program([limm(0, 1.0), halt()])
        eu.set_thread_lane_register(0, 1, 5, 42.0)
        assert eu.threads[0][1].registers.read_float(5) == 42.0

    def test_reset(self) -> None:
        config = SubsliceConfig(num_eus=1, threads_per_eu=1, simd_width=2)
        eu = ExecutionUnit(eu_id=0, config=config)
        eu.load_program([halt()])
        eu.step()
        assert eu.all_halted is True

        eu.reset()
        assert eu.all_halted is False


# ---------------------------------------------------------------------------
# SubsliceEngine — basic properties
# ---------------------------------------------------------------------------


class TestSubsliceEngineProperties:
    def test_name(self) -> None:
        engine = SubsliceEngine(
            SubsliceConfig(num_eus=2, threads_per_eu=2, simd_width=4), Clock()
        )
        assert engine.name == "SubsliceEngine"

    def test_width(self) -> None:
        engine = SubsliceEngine(
            SubsliceConfig(num_eus=2, threads_per_eu=3, simd_width=4), Clock()
        )
        assert engine.width == 2 * 3 * 4  # 24

    def test_execution_model(self) -> None:
        engine = SubsliceEngine(
            SubsliceConfig(num_eus=2, threads_per_eu=2, simd_width=4), Clock()
        )
        assert engine.execution_model == ExecutionModel.SIMD

    def test_initial_halted(self) -> None:
        engine = SubsliceEngine(
            SubsliceConfig(num_eus=2, threads_per_eu=2, simd_width=4), Clock()
        )
        assert engine.halted is False

    def test_config_access(self) -> None:
        config = SubsliceConfig(num_eus=2, threads_per_eu=2, simd_width=4)
        engine = SubsliceEngine(config, Clock())
        assert engine.config is config

    def test_eus_access(self) -> None:
        engine = SubsliceEngine(
            SubsliceConfig(num_eus=3, threads_per_eu=2, simd_width=4), Clock()
        )
        assert len(engine.eus) == 3

    def test_repr(self) -> None:
        engine = SubsliceEngine(
            SubsliceConfig(num_eus=2, threads_per_eu=2, simd_width=4), Clock()
        )
        r = repr(engine)
        assert "SubsliceEngine" in r


# ---------------------------------------------------------------------------
# SubsliceEngine — execution
# ---------------------------------------------------------------------------


class TestSubsliceEngineExecution:
    def test_simple_program(self) -> None:
        """All EU threads execute a simple program."""
        engine = SubsliceEngine(
            SubsliceConfig(num_eus=2, threads_per_eu=2, simd_width=2), Clock()
        )
        engine.load_program([limm(0, 42.0), halt()])
        traces = engine.run()
        assert len(traces) > 0
        assert engine.halted is True

    def test_per_eu_thread_lane_register(self) -> None:
        """Set and verify per-lane registers on specific EU/thread."""
        engine = SubsliceEngine(
            SubsliceConfig(num_eus=2, threads_per_eu=2, simd_width=2), Clock()
        )
        engine.load_program([limm(0, 1.0), halt()])
        engine.set_eu_thread_lane_register(0, 0, 1, 5, 99.0)

        result = engine.eus[0].threads[0][1].registers.read_float(5)
        assert result == 99.0

    def test_step_produces_trace(self) -> None:
        engine = SubsliceEngine(
            SubsliceConfig(num_eus=2, threads_per_eu=2, simd_width=2), Clock()
        )
        engine.load_program([limm(0, 1.0), halt()])

        trace = engine.step(make_edge())
        assert trace.cycle == 1
        assert trace.engine_name == "SubsliceEngine"
        assert trace.total_count == 2 * 2 * 2  # 8

    def test_halted_step(self) -> None:
        engine = SubsliceEngine(
            SubsliceConfig(num_eus=1, threads_per_eu=1, simd_width=2), Clock()
        )
        engine.load_program([halt()])
        engine.run()
        assert engine.halted is True

        trace = engine.step(make_edge())
        assert trace.active_count == 0
        assert "halted" in trace.description.lower()


# ---------------------------------------------------------------------------
# SubsliceEngine — reset
# ---------------------------------------------------------------------------


class TestSubsliceEngineReset:
    def test_reset(self) -> None:
        engine = SubsliceEngine(
            SubsliceConfig(num_eus=2, threads_per_eu=2, simd_width=2), Clock()
        )
        engine.load_program([limm(0, 42.0), halt()])
        engine.run()
        assert engine.halted is True

        engine.reset()
        assert engine.halted is False

        engine.run()
        assert engine.halted is True

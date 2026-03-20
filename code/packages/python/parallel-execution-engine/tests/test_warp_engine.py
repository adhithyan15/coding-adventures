"""Tests for warp_engine.py — SIMT parallel execution (NVIDIA/ARM Mali style)."""

from __future__ import annotations

import pytest
from clock import Clock, ClockEdge
from gpu_core import blt, fmul, halt, limm, nop

from parallel_execution_engine import (
    DivergenceStackEntry,
    ExecutionModel,
    ThreadContext,
    WarpConfig,
    WarpEngine,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_edge(cycle: int = 1) -> ClockEdge:
    """Create a rising clock edge for testing."""
    return ClockEdge(cycle=cycle, value=1, is_rising=True, is_falling=False)


# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------


class TestWarpConfig:
    """Test WarpConfig defaults and customization."""

    def test_defaults(self) -> None:
        config = WarpConfig()
        assert config.warp_width == 32
        assert config.num_registers == 32
        assert config.memory_per_thread == 1024
        assert config.max_divergence_depth == 32
        assert config.independent_thread_scheduling is False

    def test_custom(self) -> None:
        config = WarpConfig(warp_width=16, num_registers=64)
        assert config.warp_width == 16
        assert config.num_registers == 64


# ---------------------------------------------------------------------------
# ThreadContext
# ---------------------------------------------------------------------------


class TestThreadContext:
    """Test ThreadContext dataclass."""

    def test_defaults(self) -> None:
        from gpu_core import GPUCore

        ctx = ThreadContext(thread_id=0, core=GPUCore())
        assert ctx.thread_id == 0
        assert ctx.active is True
        assert ctx.pc == 0


# ---------------------------------------------------------------------------
# DivergenceStackEntry
# ---------------------------------------------------------------------------


class TestDivergenceStackEntry:
    """Test DivergenceStackEntry dataclass."""

    def test_creation(self) -> None:
        entry = DivergenceStackEntry(
            reconvergence_pc=10,
            saved_mask=[True, False, True, False],
        )
        assert entry.reconvergence_pc == 10
        assert entry.saved_mask == [True, False, True, False]


# ---------------------------------------------------------------------------
# WarpEngine — basic properties
# ---------------------------------------------------------------------------


class TestWarpEngineProperties:
    """Test basic engine properties."""

    def test_name(self) -> None:
        engine = WarpEngine(WarpConfig(warp_width=4), Clock())
        assert engine.name == "WarpEngine"

    def test_width(self) -> None:
        engine = WarpEngine(WarpConfig(warp_width=16), Clock())
        assert engine.width == 16

    def test_execution_model(self) -> None:
        engine = WarpEngine(WarpConfig(warp_width=4), Clock())
        assert engine.execution_model == ExecutionModel.SIMT

    def test_initial_halted(self) -> None:
        engine = WarpEngine(WarpConfig(warp_width=4), Clock())
        assert engine.halted is False

    def test_active_mask_all_true(self) -> None:
        engine = WarpEngine(WarpConfig(warp_width=4), Clock())
        assert engine.active_mask == [True, True, True, True]

    def test_config_access(self) -> None:
        config = WarpConfig(warp_width=8)
        engine = WarpEngine(config, Clock())
        assert engine.config is config

    def test_repr(self) -> None:
        engine = WarpEngine(WarpConfig(warp_width=4), Clock())
        r = repr(engine)
        assert "WarpEngine" in r
        assert "width=4" in r


# ---------------------------------------------------------------------------
# WarpEngine — program execution
# ---------------------------------------------------------------------------


class TestWarpEngineExecution:
    """Test basic program execution across threads."""

    def test_simple_program(self) -> None:
        """All threads execute LIMM + HALT."""
        engine = WarpEngine(WarpConfig(warp_width=4), Clock())
        engine.load_program([limm(0, 42.0), halt()])

        traces = engine.run()
        assert len(traces) >= 2  # at least LIMM + HALT

        # All threads should have R0 = 42.0
        for t in engine.threads:
            assert t.core.registers.read_float(0) == 42.0

    def test_per_thread_data(self) -> None:
        """Each thread gets different input, computes independently."""
        engine = WarpEngine(WarpConfig(warp_width=4), Clock())
        engine.load_program([
            limm(1, 2.0),       # R1 = 2.0 (same for all)
            fmul(2, 0, 1),      # R2 = R0 * R1
            halt(),
        ])

        # Give each thread a different R0
        for t in range(4):
            engine.set_thread_register(t, 0, float(t + 1))

        engine.run()

        # Thread t should have R2 = (t+1) * 2.0
        for t in range(4):
            result = engine.threads[t].core.registers.read_float(2)
            assert result == (t + 1) * 2.0

    def test_halts_when_all_done(self) -> None:
        """Engine halts when all threads execute HALT."""
        engine = WarpEngine(WarpConfig(warp_width=4), Clock())
        engine.load_program([halt()])
        engine.run()
        assert engine.halted is True

    def test_thread_register_out_of_range(self) -> None:
        """Setting register for invalid thread raises IndexError."""
        engine = WarpEngine(WarpConfig(warp_width=4), Clock())
        with pytest.raises(IndexError):
            engine.set_thread_register(4, 0, 1.0)
        with pytest.raises(IndexError):
            engine.set_thread_register(-1, 0, 1.0)

    def test_step_produces_traces(self) -> None:
        """Each step produces an EngineTrace with correct fields."""
        engine = WarpEngine(WarpConfig(warp_width=4), Clock())
        engine.load_program([limm(0, 1.0), halt()])

        trace = engine.step(make_edge())
        assert trace.cycle == 1
        assert trace.engine_name == "WarpEngine"
        assert trace.execution_model == ExecutionModel.SIMT
        assert trace.total_count == 4
        assert trace.active_count > 0
        assert 0.0 <= trace.utilization <= 1.0

    def test_utilization_in_trace(self) -> None:
        """Utilization should be active_count / total_count."""
        engine = WarpEngine(WarpConfig(warp_width=4), Clock())
        engine.load_program([limm(0, 1.0), halt()])

        trace = engine.step(make_edge())
        expected = trace.active_count / trace.total_count
        assert abs(trace.utilization - expected) < 0.001


# ---------------------------------------------------------------------------
# WarpEngine — divergence
# ---------------------------------------------------------------------------


class TestWarpEngineDivergence:
    """Test branch divergence handling."""

    def test_no_divergence_on_uniform_branch(self) -> None:
        """When all threads agree on a branch, no divergence."""
        engine = WarpEngine(WarpConfig(warp_width=4), Clock())
        # All threads have R0 = 0, R1 = 10, so R0 < R1 → all take branch
        engine.load_program([
            limm(0, 0.0),     # R0 = 0
            limm(1, 10.0),    # R1 = 10
            blt(0, 1, 2),     # if R0 < R1, skip 2 instructions
            nop(),            # skipped
            nop(),            # skipped
            halt(),
        ])

        engine.run()
        assert engine.halted is True

    def test_divergent_branch(self) -> None:
        """When threads disagree on a branch, divergence occurs."""
        engine = WarpEngine(WarpConfig(warp_width=4), Clock())

        # Thread 0,1: R0=0 < R1=2 → take branch (skip to halt)
        # Thread 2,3: R0=5 >= R1=2 → fall through
        engine.load_program([
            limm(1, 2.0),     # R1 = 2.0 (all threads)
            blt(0, 1, 2),     # if R0 < R1, skip 2
            limm(2, 99.0),    # R2 = 99 (only non-branching threads)
            halt(),           # halt (fall-through path)
            limm(2, 42.0),    # R2 = 42 (only branching threads)
            halt(),           # halt (branch path)
        ])

        # Threads 0,1 have R0=0 (< 2), threads 2,3 have R0=5 (>= 2)
        engine.set_thread_register(0, 0, 0.0)
        engine.set_thread_register(1, 0, 0.0)
        engine.set_thread_register(2, 0, 5.0)
        engine.set_thread_register(3, 0, 5.0)

        engine.run()
        assert engine.halted is True


# ---------------------------------------------------------------------------
# WarpEngine — reset
# ---------------------------------------------------------------------------


class TestWarpEngineReset:
    """Test engine reset functionality."""

    def test_reset_restores_initial_state(self) -> None:
        """After reset, engine should be in initial state."""
        engine = WarpEngine(WarpConfig(warp_width=4), Clock())
        engine.load_program([limm(0, 42.0), halt()])
        engine.run()
        assert engine.halted is True

        engine.reset()
        assert engine.halted is False
        assert all(t.active for t in engine.threads)

        # Can run again after reset
        engine.run()
        assert engine.halted is True

    def test_reset_clears_registers(self) -> None:
        """Reset should clear per-thread registers."""
        engine = WarpEngine(WarpConfig(warp_width=4), Clock())
        engine.load_program([limm(0, 42.0), halt()])
        engine.run()

        engine.reset()
        for t in engine.threads:
            assert t.core.registers.read_float(0) == 0.0


# ---------------------------------------------------------------------------
# WarpEngine — clock integration
# ---------------------------------------------------------------------------


class TestWarpEngineClockIntegration:
    """Test clock-driven execution."""

    def test_step_with_clock_edge(self) -> None:
        """step() should work with clock edges."""
        clock = Clock()
        engine = WarpEngine(WarpConfig(warp_width=4), clock)
        engine.load_program([limm(0, 1.0), halt()])

        edge = clock.tick()  # rising edge
        trace = engine.step(edge)
        assert trace.cycle == 1

    def test_halted_step_returns_trace(self) -> None:
        """Stepping a halted engine returns a halted trace."""
        engine = WarpEngine(WarpConfig(warp_width=4), Clock())
        engine.load_program([halt()])
        engine.run()
        assert engine.halted is True

        trace = engine.step(make_edge())
        assert trace.active_count == 0
        assert "halted" in trace.description.lower() or trace.utilization == 0.0

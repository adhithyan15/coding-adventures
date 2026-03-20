"""Tests for wavefront_engine.py — SIMD parallel execution (AMD GCN/RDNA style)."""

from __future__ import annotations

import pytest
from clock import Clock, ClockEdge
from gpu_core import fmul, halt, limm

from parallel_execution_engine import (
    ExecutionModel,
    ScalarRegisterFile,
    VectorRegisterFile,
    WavefrontConfig,
    WavefrontEngine,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_edge(cycle: int = 1) -> ClockEdge:
    return ClockEdge(cycle=cycle, value=1, is_rising=True, is_falling=False)


# ---------------------------------------------------------------------------
# VectorRegisterFile
# ---------------------------------------------------------------------------


class TestVectorRegisterFile:
    """Test the AMD-style vector register file."""

    def test_creation(self) -> None:
        vrf = VectorRegisterFile(num_vgprs=8, wave_width=4)
        assert vrf.num_vgprs == 8
        assert vrf.wave_width == 4

    def test_read_write(self) -> None:
        vrf = VectorRegisterFile(num_vgprs=8, wave_width=4)
        vrf.write(0, 2, 3.14)
        assert abs(vrf.read(0, 2) - 3.14) < 0.01

    def test_lanes_independent(self) -> None:
        """Different lanes of the same register are independent."""
        vrf = VectorRegisterFile(num_vgprs=4, wave_width=4)
        vrf.write(0, 0, 1.0)
        vrf.write(0, 1, 2.0)
        vrf.write(0, 2, 3.0)
        vrf.write(0, 3, 4.0)
        assert vrf.read(0, 0) == 1.0
        assert vrf.read(0, 1) == 2.0
        assert vrf.read(0, 2) == 3.0
        assert vrf.read(0, 3) == 4.0

    def test_read_all_lanes(self) -> None:
        vrf = VectorRegisterFile(num_vgprs=4, wave_width=4)
        for lane in range(4):
            vrf.write(0, lane, float(lane + 1))
        assert vrf.read_all_lanes(0) == [1.0, 2.0, 3.0, 4.0]


# ---------------------------------------------------------------------------
# ScalarRegisterFile
# ---------------------------------------------------------------------------


class TestScalarRegisterFile:
    """Test the AMD-style scalar register file."""

    def test_creation(self) -> None:
        srf = ScalarRegisterFile(num_sgprs=8)
        assert srf.num_sgprs == 8

    def test_read_write(self) -> None:
        srf = ScalarRegisterFile(num_sgprs=8)
        srf.write(3, 42.0)
        assert srf.read(3) == 42.0

    def test_initial_zero(self) -> None:
        srf = ScalarRegisterFile(num_sgprs=8)
        assert srf.read(0) == 0.0


# ---------------------------------------------------------------------------
# WavefrontConfig
# ---------------------------------------------------------------------------


class TestWavefrontConfig:
    """Test WavefrontConfig defaults."""

    def test_defaults(self) -> None:
        config = WavefrontConfig()
        assert config.wave_width == 32
        assert config.num_vgprs == 256
        assert config.num_sgprs == 104
        assert config.lds_size == 65536


# ---------------------------------------------------------------------------
# WavefrontEngine — basic properties
# ---------------------------------------------------------------------------


class TestWavefrontEngineProperties:
    def test_name(self) -> None:
        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        assert engine.name == "WavefrontEngine"

    def test_width(self) -> None:
        engine = WavefrontEngine(WavefrontConfig(wave_width=8), Clock())
        assert engine.width == 8

    def test_execution_model(self) -> None:
        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        assert engine.execution_model == ExecutionModel.SIMD

    def test_initial_halted(self) -> None:
        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        assert engine.halted is False

    def test_exec_mask_all_true(self) -> None:
        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        assert engine.exec_mask == [True, True, True, True]

    def test_config_access(self) -> None:
        config = WavefrontConfig(wave_width=4)
        engine = WavefrontEngine(config, Clock())
        assert engine.config is config

    def test_vrf_and_srf_access(self) -> None:
        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        assert engine.vrf is not None
        assert engine.srf is not None

    def test_repr(self) -> None:
        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        r = repr(engine)
        assert "WavefrontEngine" in r


# ---------------------------------------------------------------------------
# WavefrontEngine — execution
# ---------------------------------------------------------------------------


class TestWavefrontEngineExecution:
    def test_simple_program(self) -> None:
        """All lanes execute LIMM + HALT."""
        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        engine.load_program([limm(0, 42.0), halt()])
        traces = engine.run()
        assert len(traces) >= 2
        assert engine.halted is True

    def test_per_lane_data(self) -> None:
        """Each lane gets different input via vector registers."""
        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        engine.load_program([
            limm(1, 2.0),      # R1 = 2.0 (same for all)
            fmul(2, 0, 1),     # R2 = R0 * R1
            halt(),
        ])

        for lane in range(4):
            engine.set_lane_register(lane, 0, float(lane + 1))

        engine.run()

        # Check VRF for results
        for lane in range(4):
            result = engine.vrf.read(2, lane)
            assert result == (lane + 1) * 2.0

    def test_lane_register_out_of_range(self) -> None:
        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        with pytest.raises(IndexError):
            engine.set_lane_register(4, 0, 1.0)
        with pytest.raises(IndexError):
            engine.set_lane_register(-1, 0, 1.0)

    def test_scalar_register_out_of_range(self) -> None:
        engine = WavefrontEngine(WavefrontConfig(wave_width=4, num_sgprs=8), Clock())
        with pytest.raises(IndexError):
            engine.set_scalar_register(8, 1.0)
        with pytest.raises(IndexError):
            engine.set_scalar_register(-1, 1.0)


# ---------------------------------------------------------------------------
# WavefrontEngine — EXEC mask
# ---------------------------------------------------------------------------


class TestWavefrontEngineExecMask:
    def test_set_exec_mask(self) -> None:
        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        engine.set_exec_mask([True, False, True, False])
        assert engine.exec_mask == [True, False, True, False]

    def test_exec_mask_wrong_length(self) -> None:
        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        with pytest.raises(ValueError):
            engine.set_exec_mask([True, False])

    def test_masked_lanes_dont_update(self) -> None:
        """Masked lanes should not update their results."""
        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        engine.load_program([limm(0, 99.0), halt()])

        # Mask off lanes 2 and 3
        engine.set_exec_mask([True, True, False, False])

        engine.run()

        # Lanes 0,1 should have R0=99, lanes 2,3 should NOT have R0=99
        # (they execute but results go to the masked lane's core, not VRF)
        assert engine.vrf.read(0, 0) == 99.0
        assert engine.vrf.read(0, 1) == 99.0
        # Masked lanes: VRF should still be 0.0 since exec mask was off
        # (VRF sync only happens for active lanes)
        assert engine.vrf.read(0, 2) == 0.0
        assert engine.vrf.read(0, 3) == 0.0

    def test_utilization_with_mask(self) -> None:
        """Utilization should reflect the EXEC mask."""
        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        engine.load_program([limm(0, 1.0), halt()])
        engine.set_exec_mask([True, True, False, False])

        trace = engine.step(make_edge())
        # 2 out of 4 lanes active
        assert trace.active_count == 2
        assert abs(trace.utilization - 0.5) < 0.01


# ---------------------------------------------------------------------------
# WavefrontEngine — reset
# ---------------------------------------------------------------------------


class TestWavefrontEngineReset:
    def test_reset(self) -> None:
        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        engine.load_program([limm(0, 42.0), halt()])
        engine.run()
        assert engine.halted is True

        engine.reset()
        assert engine.halted is False
        assert engine.exec_mask == [True, True, True, True]

        engine.run()
        assert engine.halted is True


# ---------------------------------------------------------------------------
# WavefrontEngine — traces
# ---------------------------------------------------------------------------


class TestWavefrontEngineTraces:
    def test_trace_has_divergence_info(self) -> None:
        """SIMD traces should include divergence info (the EXEC mask)."""
        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        engine.load_program([limm(0, 1.0), halt()])

        trace = engine.step(make_edge())
        assert trace.divergence_info is not None

    def test_halted_step(self) -> None:
        engine = WavefrontEngine(WavefrontConfig(wave_width=4), Clock())
        engine.load_program([halt()])
        engine.run()

        trace = engine.step(make_edge())
        assert trace.active_count == 0

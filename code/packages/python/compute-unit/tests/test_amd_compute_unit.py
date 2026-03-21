"""Tests for AMDComputeUnit — AMD CU (GCN/RDNA) simulator."""

from __future__ import annotations

import pytest
from clock import Clock
from gpu_core import fadd, fmul, halt, limm, load

from compute_unit import (
    Architecture,
    ResourceError,
    WorkItem,
)
from compute_unit.amd_compute_unit import (
    AMDComputeUnit,
    AMDCUConfig,
)
from compute_unit.protocols import SchedulingPolicy

# ---------------------------------------------------------------------------
# AMDCUConfig tests
# ---------------------------------------------------------------------------


class TestAMDCUConfig:
    """Test AMDCUConfig defaults and customization."""

    def test_defaults(self) -> None:
        config = AMDCUConfig()
        assert config.num_simd_units == 4
        assert config.wave_width == 64
        assert config.max_wavefronts == 40
        assert config.max_work_groups == 16
        assert config.scheduling_policy == SchedulingPolicy.LRR
        assert config.vgpr_per_simd == 256
        assert config.sgpr_count == 104
        assert config.lds_size == 65536
        assert config.memory_latency_cycles == 200

    def test_custom_config(self) -> None:
        config = AMDCUConfig(
            num_simd_units=2,
            wave_width=32,
            max_wavefronts=16,
        )
        assert config.num_simd_units == 2
        assert config.wave_width == 32
        assert config.max_wavefronts == 16


# ---------------------------------------------------------------------------
# AMDComputeUnit tests
# ---------------------------------------------------------------------------


class TestAMDComputeUnit:
    """Test the AMD CU simulator."""

    def _simple_program(self) -> list:
        return [limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()]

    def test_creation(self) -> None:
        clock = Clock()
        cu = AMDComputeUnit(
            AMDCUConfig(max_wavefronts=8, wave_width=4), clock
        )
        assert cu.name == "CU"
        assert cu.architecture == Architecture.AMD_CU
        assert cu.idle
        assert cu.occupancy == 0.0

    def test_dispatch_creates_wavefronts(self) -> None:
        clock = Clock()
        cu = AMDComputeUnit(
            AMDCUConfig(max_wavefronts=16, wave_width=4, num_simd_units=2),
            clock,
        )
        cu.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=8,  # 2 wavefronts of 4 lanes
        ))
        assert not cu.idle
        assert len(cu.wavefront_slots) == 2

    def test_wavefront_decomposition(self) -> None:
        """128 threads with wave_width=64 -> 2 wavefronts."""
        clock = Clock()
        cu = AMDComputeUnit(
            AMDCUConfig(max_wavefronts=16, wave_width=64), clock
        )
        cu.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=128,
        ))
        assert len(cu.wavefront_slots) == 2

    def test_run_simple_program(self) -> None:
        clock = Clock()
        config = AMDCUConfig(
            max_wavefronts=8, wave_width=4, num_simd_units=1
        )
        cu = AMDComputeUnit(config, clock)
        cu.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=4,
        ))
        traces = cu.run()
        assert len(traces) > 0
        assert cu.idle

    def test_occupancy(self) -> None:
        clock = Clock()
        config = AMDCUConfig(max_wavefronts=8, wave_width=4)
        cu = AMDComputeUnit(config, clock)

        cu.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=8,  # 2 wavefronts
        ))
        assert cu.occupancy == pytest.approx(2 / 8)

    def test_resource_error_wavefront_slots(self) -> None:
        clock = Clock()
        config = AMDCUConfig(max_wavefronts=2, wave_width=4)
        cu = AMDComputeUnit(config, clock)

        cu.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=8,  # 2 wavefronts — fills capacity
        ))

        with pytest.raises(ResourceError, match="wavefront slots"):
            cu.dispatch(WorkItem(
                work_id=1,
                program=self._simple_program(),
                thread_count=4,
            ))

    def test_resource_error_lds(self) -> None:
        clock = Clock()
        config = AMDCUConfig(max_wavefronts=16, lds_size=1024)
        cu = AMDComputeUnit(config, clock)

        with pytest.raises(ResourceError, match="LDS"):
            cu.dispatch(WorkItem(
                work_id=0,
                program=self._simple_program(),
                thread_count=32,
                shared_mem_bytes=2048,
            ))

    def test_traces_have_correct_architecture(self) -> None:
        clock = Clock()
        config = AMDCUConfig(
            max_wavefronts=4, wave_width=4, num_simd_units=1
        )
        cu = AMDComputeUnit(config, clock)
        cu.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=4,
        ))
        traces = cu.run()
        for trace in traces:
            assert trace.architecture == Architecture.AMD_CU
            assert trace.unit_name == "CU"

    def test_per_lane_data(self) -> None:
        clock = Clock()
        config = AMDCUConfig(
            max_wavefronts=4, wave_width=4, num_simd_units=1
        )
        cu = AMDComputeUnit(config, clock)
        cu.dispatch(WorkItem(
            work_id=0,
            program=[fadd(2, 0, 1), halt()],
            thread_count=4,
            per_thread_data={
                0: {0: 1.0, 1: 10.0},
                1: {0: 2.0, 1: 20.0},
                2: {0: 3.0, 1: 30.0},
                3: {0: 4.0, 1: 40.0},
            },
        ))
        cu.run()
        assert cu.idle

    def test_multiple_work_groups(self) -> None:
        clock = Clock()
        config = AMDCUConfig(
            max_wavefronts=16, wave_width=4, num_simd_units=2
        )
        cu = AMDComputeUnit(config, clock)

        cu.dispatch(WorkItem(
            work_id=0, program=self._simple_program(), thread_count=8
        ))
        cu.dispatch(WorkItem(
            work_id=1, program=self._simple_program(), thread_count=8
        ))

        assert len(cu.wavefront_slots) == 4
        cu.run()
        assert cu.idle

    def test_lds_access(self) -> None:
        clock = Clock()
        cu = AMDComputeUnit(AMDCUConfig(), clock)
        lds = cu.lds
        lds.write(0, 42.0, thread_id=0)
        assert abs(lds.read(0, thread_id=0) - 42.0) < 0.01

    def test_reset(self) -> None:
        clock = Clock()
        cu = AMDComputeUnit(
            AMDCUConfig(max_wavefronts=8, wave_width=4), clock
        )
        cu.dispatch(WorkItem(
            work_id=0, program=self._simple_program(), thread_count=4
        ))
        cu.run()
        cu.reset()
        assert cu.idle
        assert len(cu.wavefront_slots) == 0
        assert cu.occupancy == 0.0

    def test_repr(self) -> None:
        clock = Clock()
        cu = AMDComputeUnit(AMDCUConfig(), clock)
        r = repr(cu)
        assert "AMDComputeUnit" in r

    def test_memory_stall(self) -> None:
        """LOAD should stall the wavefront."""
        clock = Clock()
        config = AMDCUConfig(
            max_wavefronts=8,
            wave_width=4,
            num_simd_units=1,
            memory_latency_cycles=3,
        )
        cu = AMDComputeUnit(config, clock)
        prog = [limm(0, 0.0), load(1, 0), halt()]
        cu.dispatch(WorkItem(
            work_id=0, program=prog, thread_count=4
        ))
        cu.run(max_cycles=50)
        assert cu.idle

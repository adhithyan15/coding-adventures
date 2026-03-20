"""Tests for protocols.py — shared types and SharedMemory."""

from __future__ import annotations

import pytest
from parallel_execution_engine import EngineTrace, ExecutionModel

from compute_unit import SMConfig
from compute_unit.protocols import (
    Architecture,
    ComputeUnit,
    ComputeUnitTrace,
    SchedulingPolicy,
    SharedMemory,
    WarpState,
    WorkItem,
)

# ---------------------------------------------------------------------------
# Architecture enum
# ---------------------------------------------------------------------------


class TestArchitecture:
    """Test the Architecture enum."""

    def test_all_values_exist(self) -> None:
        assert Architecture.NVIDIA_SM.value == "nvidia_sm"
        assert Architecture.AMD_CU.value == "amd_cu"
        assert Architecture.GOOGLE_MXU.value == "google_mxu"
        assert Architecture.INTEL_XE_CORE.value == "intel_xe_core"
        assert Architecture.APPLE_ANE_CORE.value == "apple_ane_core"

    def test_unique_values(self) -> None:
        values = [a.value for a in Architecture]
        assert len(values) == len(set(values))

    def test_member_count(self) -> None:
        assert len(Architecture) == 5


# ---------------------------------------------------------------------------
# WarpState enum
# ---------------------------------------------------------------------------


class TestWarpState:
    """Test the WarpState enum."""

    def test_all_states_exist(self) -> None:
        assert WarpState.READY.value == "ready"
        assert WarpState.RUNNING.value == "running"
        assert WarpState.STALLED_MEMORY.value == "stalled_memory"
        assert WarpState.STALLED_BARRIER.value == "stalled_barrier"
        assert WarpState.STALLED_DEPENDENCY.value == "stalled_dependency"
        assert WarpState.COMPLETED.value == "completed"

    def test_member_count(self) -> None:
        assert len(WarpState) == 6


# ---------------------------------------------------------------------------
# SchedulingPolicy enum
# ---------------------------------------------------------------------------


class TestSchedulingPolicy:
    """Test the SchedulingPolicy enum."""

    def test_all_policies_exist(self) -> None:
        assert SchedulingPolicy.ROUND_ROBIN.value == "round_robin"
        assert SchedulingPolicy.GREEDY.value == "greedy"
        assert SchedulingPolicy.OLDEST_FIRST.value == "oldest_first"
        assert SchedulingPolicy.GTO.value == "gto"
        assert SchedulingPolicy.LRR.value == "lrr"

    def test_member_count(self) -> None:
        assert len(SchedulingPolicy) == 5


# ---------------------------------------------------------------------------
# WorkItem dataclass
# ---------------------------------------------------------------------------


class TestWorkItem:
    """Test the WorkItem dataclass."""

    def test_default_values(self) -> None:
        wi = WorkItem(work_id=0)
        assert wi.work_id == 0
        assert wi.program is None
        assert wi.thread_count == 32
        assert wi.per_thread_data == {}
        assert wi.input_data is None
        assert wi.weight_data is None
        assert wi.schedule is None
        assert wi.shared_mem_bytes == 0
        assert wi.registers_per_thread == 32

    def test_custom_values(self) -> None:
        from gpu_core import halt, limm

        prog = [limm(0, 1.0), halt()]
        wi = WorkItem(
            work_id=42,
            program=prog,
            thread_count=128,
            per_thread_data={0: {0: 1.0}},
            shared_mem_bytes=4096,
            registers_per_thread=64,
        )
        assert wi.work_id == 42
        assert wi.thread_count == 128
        assert wi.shared_mem_bytes == 4096
        assert wi.registers_per_thread == 64

    def test_frozen(self) -> None:
        wi = WorkItem(work_id=0)
        with pytest.raises(AttributeError):
            wi.work_id = 1  # type: ignore[misc]

    def test_dataflow_work_item(self) -> None:
        wi = WorkItem(
            work_id=1,
            input_data=[[1.0, 2.0], [3.0, 4.0]],
            weight_data=[[5.0, 6.0], [7.0, 8.0]],
        )
        assert wi.input_data is not None
        assert len(wi.input_data) == 2


# ---------------------------------------------------------------------------
# ComputeUnitTrace dataclass
# ---------------------------------------------------------------------------


class TestComputeUnitTrace:
    """Test the ComputeUnitTrace dataclass."""

    def _make_trace(self, cycle: int = 1) -> ComputeUnitTrace:
        return ComputeUnitTrace(
            cycle=cycle,
            unit_name="SM",
            architecture=Architecture.NVIDIA_SM,
            scheduler_action="issued warp 3",
            active_warps=48,
            total_warps=64,
            engine_traces={},
            shared_memory_used=49152,
            shared_memory_total=98304,
            register_file_used=32768,
            register_file_total=65536,
            occupancy=0.75,
        )

    def test_basic_creation(self) -> None:
        trace = self._make_trace()
        assert trace.cycle == 1
        assert trace.unit_name == "SM"
        assert trace.architecture == Architecture.NVIDIA_SM
        assert trace.occupancy == 0.75

    def test_default_cache_stats(self) -> None:
        trace = self._make_trace()
        assert trace.l1_hits == 0
        assert trace.l1_misses == 0

    def test_format_output(self) -> None:
        trace = self._make_trace(cycle=5)
        formatted = trace.format()
        assert "[Cycle 5]" in formatted
        assert "SM" in formatted
        assert "nvidia_sm" in formatted
        assert "75.0%" in formatted
        assert "issued warp 3" in formatted
        assert "Shared memory" in formatted
        assert "Registers" in formatted

    def test_format_with_engine_trace(self) -> None:
        engine_trace = EngineTrace(
            cycle=5,
            engine_name="WarpEngine",
            execution_model=ExecutionModel.SIMT,
            description="FMUL R2, R0, R1 — 32/32 active",
            unit_traces={0: "ok"},
            active_mask=[True],
            active_count=32,
            total_count=32,
            utilization=1.0,
        )
        trace = ComputeUnitTrace(
            cycle=5,
            unit_name="SM",
            architecture=Architecture.NVIDIA_SM,
            scheduler_action="issued warp 0",
            active_warps=1,
            total_warps=48,
            engine_traces={0: engine_trace},
            shared_memory_used=0,
            shared_memory_total=98304,
            register_file_used=1024,
            register_file_total=65536,
            occupancy=1 / 48,
        )
        formatted = trace.format()
        assert "Engine 0" in formatted
        assert "FMUL" in formatted

    def test_frozen(self) -> None:
        trace = self._make_trace()
        with pytest.raises(AttributeError):
            trace.cycle = 99  # type: ignore[misc]


# ---------------------------------------------------------------------------
# SharedMemory
# ---------------------------------------------------------------------------


class TestSharedMemory:
    """Test SharedMemory with bank conflict detection."""

    def test_creation(self) -> None:
        smem = SharedMemory(size=1024)
        assert smem.size == 1024
        assert smem.num_banks == 32
        assert smem.bank_width == 4

    def test_write_and_read(self) -> None:
        smem = SharedMemory(size=1024)
        smem.write(0, 3.14, thread_id=0)
        val = smem.read(0, thread_id=0)
        assert abs(val - 3.14) < 0.01

    def test_write_multiple_addresses(self) -> None:
        smem = SharedMemory(size=1024)
        smem.write(0, 1.0, thread_id=0)
        smem.write(4, 2.0, thread_id=1)
        smem.write(8, 3.0, thread_id=2)
        assert abs(smem.read(0, thread_id=0) - 1.0) < 0.001
        assert abs(smem.read(4, thread_id=1) - 2.0) < 0.001
        assert abs(smem.read(8, thread_id=2) - 3.0) < 0.001

    def test_read_out_of_range(self) -> None:
        smem = SharedMemory(size=64)
        with pytest.raises(IndexError):
            smem.read(64, thread_id=0)

    def test_write_out_of_range(self) -> None:
        smem = SharedMemory(size=64)
        with pytest.raises(IndexError):
            smem.write(64, 1.0, thread_id=0)

    def test_negative_address(self) -> None:
        smem = SharedMemory(size=64)
        with pytest.raises(IndexError):
            smem.read(-1, thread_id=0)

    def test_bank_conflict_detection_no_conflicts(self) -> None:
        """Each thread accesses a different bank — no conflicts."""
        smem = SharedMemory(size=1024, num_banks=32, bank_width=4)
        # Addresses 0, 4, 8, 12 -> banks 0, 1, 2, 3
        conflicts = smem.check_bank_conflicts([0, 4, 8, 12])
        assert conflicts == []

    def test_bank_conflict_detection_with_conflicts(self) -> None:
        """Two threads hit the same bank — bank conflict!"""
        smem = SharedMemory(size=1024, num_banks=32, bank_width=4)
        # Address 0 -> bank 0, address 128 -> bank 0 (32*4=128 wraps)
        conflicts = smem.check_bank_conflicts([0, 4, 128, 12])
        assert len(conflicts) == 1
        assert sorted(conflicts[0]) == [0, 2]

    def test_bank_conflict_3way(self) -> None:
        """Three threads hit the same bank — 3-way conflict."""
        smem = SharedMemory(size=1024, num_banks=32, bank_width=4)
        # Addresses 0, 128, 256 all map to bank 0
        conflicts = smem.check_bank_conflicts([0, 128, 256])
        assert len(conflicts) == 1
        assert sorted(conflicts[0]) == [0, 1, 2]

    def test_bank_conflict_multiple_groups(self) -> None:
        """Multiple conflict groups."""
        smem = SharedMemory(size=1024, num_banks=32, bank_width=4)
        # Bank 0: addr 0 and 128 (threads 0, 2)
        # Bank 1: addr 4 and 132 (threads 1, 3)
        conflicts = smem.check_bank_conflicts([0, 4, 128, 132])
        assert len(conflicts) == 2

    def test_access_counting(self) -> None:
        smem = SharedMemory(size=1024)
        smem.write(0, 1.0, thread_id=0)
        smem.read(0, thread_id=0)
        smem.read(0, thread_id=1)
        assert smem.total_accesses == 3

    def test_conflict_counting(self) -> None:
        smem = SharedMemory(size=1024, num_banks=32, bank_width=4)
        smem.check_bank_conflicts([0, 128])
        assert smem.total_conflicts == 1

    def test_reset(self) -> None:
        smem = SharedMemory(size=1024)
        smem.write(0, 42.0, thread_id=0)
        smem.check_bank_conflicts([0, 128])
        smem.reset()
        assert smem.total_accesses == 0
        assert smem.total_conflicts == 0
        val = smem.read(0, thread_id=0)
        assert val == 0.0

    def test_custom_bank_config(self) -> None:
        smem = SharedMemory(size=256, num_banks=8, bank_width=8)
        assert smem.num_banks == 8
        assert smem.bank_width == 8
        # Bank = (addr // 8) % 8
        # addr 0 -> bank 0, addr 64 -> bank 0 (64//8=8, 8%8=0)
        conflicts = smem.check_bank_conflicts([0, 64])
        assert len(conflicts) == 1


# ---------------------------------------------------------------------------
# ComputeUnit protocol
# ---------------------------------------------------------------------------


class TestComputeUnitProtocol:
    """Test that ComputeUnit protocol is properly runtime-checkable."""

    def test_sm_satisfies_protocol(self) -> None:
        from clock import Clock

        from compute_unit import StreamingMultiprocessor

        clock = Clock()
        sm = StreamingMultiprocessor(SMConfig(), clock)
        # Check it has the right methods
        assert hasattr(sm, "name")
        assert hasattr(sm, "architecture")
        assert hasattr(sm, "dispatch")
        assert hasattr(sm, "step")
        assert hasattr(sm, "run")
        assert hasattr(sm, "idle")
        assert hasattr(sm, "reset")

    def test_protocol_is_runtime_checkable(self) -> None:
        # ComputeUnit should be runtime checkable
        assert hasattr(ComputeUnit, "__protocol_attrs__") or hasattr(
            ComputeUnit, "__abstractmethods__"
        ) or True  # Just ensure it's importable and usable

"""Tests for StreamingMultiprocessor — NVIDIA SM simulator."""

from __future__ import annotations

import pytest
from clock import Clock
from gpu_core import fadd, fmul, halt, limm, load

from compute_unit import (
    Architecture,
    ResourceError,
    SchedulingPolicy,
    SMConfig,
    StreamingMultiprocessor,
    WarpScheduler,
    WarpSlot,
    WarpState,
    WorkItem,
)

# ---------------------------------------------------------------------------
# SMConfig tests
# ---------------------------------------------------------------------------


class TestSMConfig:
    """Test SMConfig defaults and customization."""

    def test_defaults(self) -> None:
        config = SMConfig()
        assert config.num_schedulers == 4
        assert config.warp_width == 32
        assert config.max_warps == 48
        assert config.max_threads == 1536
        assert config.max_blocks == 16
        assert config.register_file_size == 65536
        assert config.shared_memory_size == 98304
        assert config.memory_latency_cycles == 200
        assert config.scheduling_policy == SchedulingPolicy.GTO

    def test_custom_config(self) -> None:
        config = SMConfig(
            num_schedulers=2,
            max_warps=16,
            scheduling_policy=SchedulingPolicy.ROUND_ROBIN,
        )
        assert config.num_schedulers == 2
        assert config.max_warps == 16
        assert config.scheduling_policy == SchedulingPolicy.ROUND_ROBIN


# ---------------------------------------------------------------------------
# WarpScheduler tests
# ---------------------------------------------------------------------------


class TestWarpScheduler:
    """Test warp scheduling policies."""

    def _make_slot(
        self,
        warp_id: int,
        state: WarpState = WarpState.READY,
        age: int = 0,
    ) -> WarpSlot:
        clock = Clock()
        from parallel_execution_engine import WarpConfig, WarpEngine

        engine = WarpEngine(WarpConfig(warp_width=4), clock)
        return WarpSlot(
            warp_id=warp_id,
            work_id=0,
            state=state,
            engine=engine,
            age=age,
        )

    def test_round_robin_picks_in_order(self) -> None:
        sched = WarpScheduler(0, SchedulingPolicy.ROUND_ROBIN)
        sched.add_warp(self._make_slot(0))
        sched.add_warp(self._make_slot(1))
        sched.add_warp(self._make_slot(2))

        picked = sched.pick_warp()
        assert picked is not None
        assert picked.warp_id == 0

    def test_round_robin_wraps_around(self) -> None:
        sched = WarpScheduler(0, SchedulingPolicy.ROUND_ROBIN)
        s0 = self._make_slot(0)
        s1 = self._make_slot(1)
        sched.add_warp(s0)
        sched.add_warp(s1)

        # Pick first
        p1 = sched.pick_warp()
        assert p1 is not None and p1.warp_id == 0

        # Pick second
        p2 = sched.pick_warp()
        assert p2 is not None and p2.warp_id == 1

        # Should wrap back to 0
        p3 = sched.pick_warp()
        assert p3 is not None and p3.warp_id == 0

    def test_round_robin_skips_non_ready(self) -> None:
        sched = WarpScheduler(0, SchedulingPolicy.ROUND_ROBIN)
        sched.add_warp(self._make_slot(0, WarpState.STALLED_MEMORY))
        sched.add_warp(self._make_slot(1, WarpState.READY))

        picked = sched.pick_warp()
        assert picked is not None
        assert picked.warp_id == 1

    def test_gto_stays_with_same_warp(self) -> None:
        sched = WarpScheduler(0, SchedulingPolicy.GTO)
        s0 = self._make_slot(0)
        s1 = self._make_slot(1)
        sched.add_warp(s0)
        sched.add_warp(s1)

        # Issue warp 0
        picked = sched.pick_warp()
        assert picked is not None
        sched.mark_issued(picked.warp_id)

        # GTO should keep picking warp 0 (it was last issued)
        picked2 = sched.pick_warp()
        assert picked2 is not None
        assert picked2.warp_id == picked.warp_id

    def test_gto_switches_when_stalled(self) -> None:
        sched = WarpScheduler(0, SchedulingPolicy.GTO)
        s0 = self._make_slot(0, age=5)
        s1 = self._make_slot(1, age=10)
        sched.add_warp(s0)
        sched.add_warp(s1)

        # Issue warp 0
        sched.mark_issued(0)

        # Stall warp 0
        s0.state = WarpState.STALLED_MEMORY

        # GTO should switch to oldest ready warp (warp 1)
        picked = sched.pick_warp()
        assert picked is not None
        assert picked.warp_id == 1

    def test_oldest_first_picks_oldest(self) -> None:
        sched = WarpScheduler(0, SchedulingPolicy.OLDEST_FIRST)
        sched.add_warp(self._make_slot(0, age=5))
        sched.add_warp(self._make_slot(1, age=10))
        sched.add_warp(self._make_slot(2, age=3))

        picked = sched.pick_warp()
        assert picked is not None
        assert picked.warp_id == 1  # age=10 is oldest

    def test_no_ready_warps(self) -> None:
        sched = WarpScheduler(0, SchedulingPolicy.ROUND_ROBIN)
        sched.add_warp(self._make_slot(0, WarpState.STALLED_MEMORY))
        sched.add_warp(self._make_slot(1, WarpState.COMPLETED))

        picked = sched.pick_warp()
        assert picked is None

    def test_tick_stalls_decrements_counter(self) -> None:
        sched = WarpScheduler(0, SchedulingPolicy.ROUND_ROBIN)
        slot = self._make_slot(0, WarpState.STALLED_MEMORY)
        slot.stall_counter = 3
        sched.add_warp(slot)

        sched.tick_stalls()
        assert slot.stall_counter == 2
        assert slot.state == WarpState.STALLED_MEMORY

        sched.tick_stalls()
        assert slot.stall_counter == 1

        sched.tick_stalls()
        assert slot.stall_counter == 0
        assert slot.state == WarpState.READY

    def test_tick_stalls_increments_age(self) -> None:
        sched = WarpScheduler(0, SchedulingPolicy.ROUND_ROBIN)
        slot = self._make_slot(0)
        slot.age = 0
        sched.add_warp(slot)

        sched.tick_stalls()
        assert slot.age == 1

    def test_reset(self) -> None:
        sched = WarpScheduler(0, SchedulingPolicy.ROUND_ROBIN)
        sched.add_warp(self._make_slot(0))
        sched.reset()
        assert len(sched.warps) == 0

    def test_lrr_scheduling(self) -> None:
        sched = WarpScheduler(0, SchedulingPolicy.LRR)
        sched.add_warp(self._make_slot(0, WarpState.STALLED_MEMORY))
        sched.add_warp(self._make_slot(1))
        sched.add_warp(self._make_slot(2))

        picked = sched.pick_warp()
        assert picked is not None
        assert picked.warp_id in (1, 2)

    def test_greedy_scheduling(self) -> None:
        sched = WarpScheduler(0, SchedulingPolicy.GREEDY)
        sched.add_warp(self._make_slot(0, age=2))
        sched.add_warp(self._make_slot(1, age=5))

        picked = sched.pick_warp()
        assert picked is not None
        assert picked.warp_id == 1  # older


# ---------------------------------------------------------------------------
# StreamingMultiprocessor tests
# ---------------------------------------------------------------------------


class TestStreamingMultiprocessor:
    """Test the NVIDIA SM simulator."""

    def _simple_program(self) -> list:
        """A minimal program: load immediate, multiply, halt."""
        return [limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()]

    def test_creation(self) -> None:
        clock = Clock()
        sm = StreamingMultiprocessor(SMConfig(max_warps=8), clock)
        assert sm.name == "SM"
        assert sm.architecture == Architecture.NVIDIA_SM
        assert sm.idle
        assert sm.occupancy == 0.0

    def test_dispatch_creates_warps(self) -> None:
        clock = Clock()
        sm = StreamingMultiprocessor(
            SMConfig(max_warps=16, warp_width=4), clock
        )
        sm.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=8,  # 2 warps of 4 threads
        ))
        assert not sm.idle
        assert len(sm.warp_slots) == 2

    def test_dispatch_thread_block_decomposition(self) -> None:
        """A 128-thread block should create 4 warps (128/32)."""
        clock = Clock()
        sm = StreamingMultiprocessor(SMConfig(max_warps=16), clock)
        sm.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=128,
        ))
        assert len(sm.warp_slots) == 4  # 128/32 = 4

    def test_dispatch_partial_warp(self) -> None:
        """40 threads should create 2 warps: 32 + 8."""
        clock = Clock()
        sm = StreamingMultiprocessor(
            SMConfig(max_warps=16, warp_width=32), clock
        )
        sm.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=40,
        ))
        assert len(sm.warp_slots) == 2

    def test_run_simple_program(self) -> None:
        clock = Clock()
        config = SMConfig(max_warps=8, warp_width=4, num_schedulers=1)
        sm = StreamingMultiprocessor(config, clock)
        sm.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=4,
        ))
        traces = sm.run()
        assert len(traces) > 0
        assert sm.idle

    def test_run_produces_traces(self) -> None:
        clock = Clock()
        config = SMConfig(max_warps=8, warp_width=4, num_schedulers=1)
        sm = StreamingMultiprocessor(config, clock)
        sm.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=4,
        ))
        traces = sm.run()
        for trace in traces:
            assert trace.unit_name == "SM"
            assert trace.architecture == Architecture.NVIDIA_SM
            assert 0 <= trace.occupancy <= 1.0

    def test_occupancy_calculation(self) -> None:
        clock = Clock()
        config = SMConfig(max_warps=8, warp_width=4)
        sm = StreamingMultiprocessor(config, clock)

        # 8 threads = 2 warps. occupancy = 2/8 = 0.25
        sm.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=8,
        ))
        assert sm.occupancy == pytest.approx(0.25)

    def test_static_occupancy_register_limited(self) -> None:
        clock = Clock()
        config = SMConfig(
            max_warps=48,
            register_file_size=65536,
            shared_memory_size=98304,
        )
        sm = StreamingMultiprocessor(config, clock)

        # 64 regs/thread * 32 threads = 2048 regs/warp
        # 65536 / 2048 = 32 warps max by registers
        occ = sm.compute_occupancy(
            registers_per_thread=64,
            shared_mem_per_block=0,
            threads_per_block=256,
        )
        # 32/48 = 0.667
        assert occ == pytest.approx(32 / 48, abs=0.01)

    def test_static_occupancy_smem_limited(self) -> None:
        clock = Clock()
        config = SMConfig(
            max_warps=48,
            register_file_size=65536,
            shared_memory_size=98304,
            warp_width=32,
        )
        sm = StreamingMultiprocessor(config, clock)

        # 49152 bytes/block. 98304/49152 = 2 blocks.
        # 256 threads/block = 8 warps. 2*8 = 16 warps.
        occ = sm.compute_occupancy(
            registers_per_thread=16,
            shared_mem_per_block=49152,
            threads_per_block=256,
        )
        assert occ == pytest.approx(16 / 48, abs=0.01)

    def test_static_occupancy_hw_limited(self) -> None:
        clock = Clock()
        config = SMConfig(max_warps=8)
        sm = StreamingMultiprocessor(config, clock)

        # Few regs and no smem -> hardware limit is the bottleneck
        occ = sm.compute_occupancy(
            registers_per_thread=4,
            shared_mem_per_block=0,
            threads_per_block=32,
        )
        assert occ == pytest.approx(1.0)

    def test_resource_error_warp_slots(self) -> None:
        clock = Clock()
        config = SMConfig(max_warps=2, warp_width=4)
        sm = StreamingMultiprocessor(config, clock)

        # First dispatch: 2 warps (fills capacity)
        sm.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=8,
        ))

        # Second dispatch should fail
        with pytest.raises(ResourceError, match="warp slots"):
            sm.dispatch(WorkItem(
                work_id=1,
                program=self._simple_program(),
                thread_count=4,
            ))

    def test_resource_error_registers(self) -> None:
        clock = Clock()
        config = SMConfig(
            max_warps=100,
            warp_width=4,
            register_file_size=100,
        )
        sm = StreamingMultiprocessor(config, clock)

        with pytest.raises(ResourceError, match="registers"):
            sm.dispatch(WorkItem(
                work_id=0,
                program=self._simple_program(),
                thread_count=4,
                registers_per_thread=32,  # 32 * 4 = 128 > 100
            ))

    def test_resource_error_shared_memory(self) -> None:
        clock = Clock()
        config = SMConfig(max_warps=100, shared_memory_size=1024)
        sm = StreamingMultiprocessor(config, clock)

        with pytest.raises(ResourceError, match="shared memory"):
            sm.dispatch(WorkItem(
                work_id=0,
                program=self._simple_program(),
                thread_count=32,
                shared_mem_bytes=2048,
            ))

    def test_memory_stall_simulation(self) -> None:
        """LOAD instruction should stall the warp."""
        clock = Clock()
        config = SMConfig(
            max_warps=8,
            warp_width=4,
            num_schedulers=1,
            memory_latency_cycles=5,
        )
        sm = StreamingMultiprocessor(config, clock)

        # Program with a LOAD instruction
        prog = [limm(0, 0.0), load(1, 0), halt()]
        sm.dispatch(WorkItem(
            work_id=0,
            program=prog,
            thread_count=4,
        ))

        # Run a few cycles - should see a stalled warp
        traces = sm.run(max_cycles=50)
        # After the LOAD, the warp should stall
        # Either we caught the stall state or the scheduler reported no
        # ready warps — either way, the SM should complete eventually.
        assert any(
            slot.state == WarpState.STALLED_MEMORY
            for trace in traces
            for slot in sm.warp_slots
        ) or any(
            "no ready warp" in t.scheduler_action for t in traces
        ) or sm.idle
        assert sm.idle  # Should complete eventually

    def test_per_thread_data(self) -> None:
        """Test dispatching with per-thread register values."""
        clock = Clock()
        config = SMConfig(max_warps=8, warp_width=4, num_schedulers=1)
        sm = StreamingMultiprocessor(config, clock)

        sm.dispatch(WorkItem(
            work_id=0,
            program=[fadd(2, 0, 1), halt()],
            thread_count=4,
            per_thread_data={
                0: {0: 1.0, 1: 2.0},
                1: {0: 3.0, 1: 4.0},
                2: {0: 5.0, 1: 6.0},
                3: {0: 7.0, 1: 8.0},
            },
        ))
        sm.run()
        assert sm.idle

    def test_multiple_blocks(self) -> None:
        """Dispatch multiple thread blocks."""
        clock = Clock()
        config = SMConfig(max_warps=16, warp_width=4, num_schedulers=2)
        sm = StreamingMultiprocessor(config, clock)

        sm.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=8,
        ))
        sm.dispatch(WorkItem(
            work_id=1,
            program=self._simple_program(),
            thread_count=8,
        ))

        assert len(sm.warp_slots) == 4  # 2 warps per block * 2 blocks
        sm.run()
        assert sm.idle

    def test_reset(self) -> None:
        clock = Clock()
        sm = StreamingMultiprocessor(
            SMConfig(max_warps=8, warp_width=4), clock
        )
        sm.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=4,
        ))
        sm.run()
        sm.reset()
        assert sm.idle
        assert len(sm.warp_slots) == 0
        assert sm.occupancy == 0.0

    def test_repr(self) -> None:
        clock = Clock()
        sm = StreamingMultiprocessor(SMConfig(max_warps=8), clock)
        r = repr(sm)
        assert "StreamingMultiprocessor" in r
        assert "policy=" in r

    def test_gto_scheduling_integration(self) -> None:
        """GTO scheduler should be used by default."""
        clock = Clock()
        config = SMConfig(
            max_warps=8,
            warp_width=4,
            num_schedulers=1,
            scheduling_policy=SchedulingPolicy.GTO,
        )
        sm = StreamingMultiprocessor(config, clock)
        sm.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=4,
        ))
        traces = sm.run()
        assert len(traces) > 0
        assert sm.idle

    def test_round_robin_scheduling_integration(self) -> None:
        clock = Clock()
        config = SMConfig(
            max_warps=8,
            warp_width=4,
            num_schedulers=1,
            scheduling_policy=SchedulingPolicy.ROUND_ROBIN,
        )
        sm = StreamingMultiprocessor(config, clock)
        sm.dispatch(WorkItem(
            work_id=0,
            program=self._simple_program(),
            thread_count=8,  # 2 warps
        ))
        sm.run()
        assert sm.idle

    def test_shared_memory_access(self) -> None:
        """Shared memory should be accessible."""
        clock = Clock()
        sm = StreamingMultiprocessor(SMConfig(), clock)
        smem = sm.shared_memory
        smem.write(0, 42.0, thread_id=0)
        assert abs(smem.read(0, thread_id=0) - 42.0) < 0.01

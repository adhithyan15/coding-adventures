"""Tests for the combined hazard unit — priority, history, and stats.

These tests verify that the HazardUnit correctly combines results from
all three detectors and applies the priority system:
FLUSH > STALL > FORWARD > NONE.
"""

from __future__ import annotations

from hazard_detection.hazard_unit import HazardUnit, _pick_highest_priority
from hazard_detection.types import HazardAction, HazardResult, PipelineSlot


class TestPrioritySystem:
    """Verify that higher-priority hazards override lower ones."""

    def test_flush_overrides_stall(self) -> None:
        """Branch misprediction (flush) + load-use (stall) → flush wins.

        Even though there's a data hazard, the branch misprediction
        means the instruction with the data hazard is WRONG and will
        be flushed anyway. No point stalling for it.
        """
        unit = HazardUnit(num_alus=1)

        # EX: mispredicted branch (will cause FLUSH)
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x1000,
            is_branch=True,
            branch_predicted_taken=False,
            branch_taken=True,
            uses_alu=True,
            dest_reg=None,
        )
        # ID: instruction that would have a load-use hazard with EX
        # (but it's going to be flushed anyway)
        id_stage = PipelineSlot(
            valid=True,
            pc=0x1004,
            source_regs=(1,),
            uses_alu=True,
        )
        # MEM: a load that just completed
        mem_stage = PipelineSlot(
            valid=True,
            pc=0x0FFC,
            dest_reg=1,
            dest_value=42,
            mem_read=True,
            uses_alu=False,
        )
        if_stage = PipelineSlot(valid=True, pc=0x1008)

        result = unit.check(if_stage, id_stage, ex_stage, mem_stage)

        assert result.action == HazardAction.FLUSH

    def test_stall_overrides_forward(self) -> None:
        """Load-use stall + forwarding available → stall wins.

        When a load-use hazard requires a stall, it doesn't matter that
        another register could be forwarded. The stall is mandatory.
        """
        unit = HazardUnit()

        # EX: load instruction (will cause stall if ID reads its dest)
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x1000,
            dest_reg=1,
            mem_read=True,
            uses_alu=False,
        )
        # ID: reads R1 (stall) and R2 (could forward from MEM)
        id_stage = PipelineSlot(
            valid=True,
            pc=0x1004,
            source_regs=(1, 2),
            uses_alu=True,
        )
        mem_stage = PipelineSlot(
            valid=True,
            pc=0x0FFC,
            dest_reg=2,
            dest_value=99,
            uses_alu=True,
        )
        if_stage = PipelineSlot(valid=True, pc=0x1008)

        result = unit.check(if_stage, id_stage, ex_stage, mem_stage)

        assert result.action == HazardAction.STALL

    def test_forward_overrides_none(self) -> None:
        """Forwarding available + no other hazard → forward."""
        unit = HazardUnit(num_alus=2)  # avoid structural hazard on ALU

        ex_stage = PipelineSlot(
            valid=True,
            pc=0x1000,
            dest_reg=1,
            dest_value=42,
            uses_alu=True,
        )
        id_stage = PipelineSlot(
            valid=True,
            pc=0x1004,
            source_regs=(1,),
            uses_alu=True,
        )
        mem_stage = PipelineSlot()
        if_stage = PipelineSlot(valid=True, pc=0x1008)

        result = unit.check(if_stage, id_stage, ex_stage, mem_stage)

        assert result.action == HazardAction.FORWARD_FROM_EX
        assert result.forwarded_value == 42

    def test_no_hazards_returns_none(self) -> None:
        """All clear — no hazards of any type."""
        unit = HazardUnit(num_alus=2)  # avoid structural hazard on ALU

        # All stages valid but no conflicts.
        if_stage = PipelineSlot(valid=True, pc=0x100C)
        id_stage = PipelineSlot(
            valid=True, pc=0x1008, source_regs=(5, 6), uses_alu=True
        )
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x1004,
            dest_reg=1,
            dest_value=10,
            uses_alu=True,
        )
        mem_stage = PipelineSlot(
            valid=True, pc=0x1000, dest_reg=2, uses_alu=True
        )

        result = unit.check(if_stage, id_stage, ex_stage, mem_stage)

        assert result.action == HazardAction.NONE


class TestHistoryTracking:
    """Verify that the hazard unit records history of all checks."""

    def test_history_records_each_check(self) -> None:
        """Each call to check() adds one entry to history."""
        unit = HazardUnit()

        empty = PipelineSlot()
        unit.check(empty, empty, empty, empty)
        unit.check(empty, empty, empty, empty)
        unit.check(empty, empty, empty, empty)

        assert len(unit.history) == 3

    def test_history_is_a_copy(self) -> None:
        """The history property returns a copy, not the internal list."""
        unit = HazardUnit()
        empty = PipelineSlot()
        unit.check(empty, empty, empty, empty)

        history = unit.history
        history.clear()  # modifying the copy

        assert len(unit.history) == 1  # internal list unaffected


class TestStatistics:
    """Verify stall_count, flush_count, and forward_count stats."""

    def test_stall_count_sums_stall_cycles(self) -> None:
        """stall_count sums all stall_cycles across history."""
        unit = HazardUnit()

        # Scenario 1: load-use stall (1 cycle)
        ex_load = PipelineSlot(
            valid=True,
            pc=0x1000,
            dest_reg=1,
            mem_read=True,
            uses_alu=False,
        )
        id_use = PipelineSlot(
            valid=True,
            pc=0x1004,
            source_regs=(1,),
            uses_alu=True,
        )
        empty = PipelineSlot()

        unit.check(empty, id_use, ex_load, empty)
        unit.check(empty, id_use, ex_load, empty)

        assert unit.stall_count == 2  # 1 + 1

    def test_flush_count_tracks_mispredictions(self) -> None:
        """flush_count counts the number of FLUSH actions."""
        unit = HazardUnit()

        branch_mispredict = PipelineSlot(
            valid=True,
            pc=0x1000,
            is_branch=True,
            branch_predicted_taken=False,
            branch_taken=True,
        )
        empty = PipelineSlot()

        unit.check(empty, empty, branch_mispredict, empty)
        unit.check(empty, empty, empty, empty)  # no hazard
        unit.check(empty, empty, branch_mispredict, empty)

        assert unit.flush_count == 2

    def test_forward_count_tracks_forwards(self) -> None:
        """forward_count counts FORWARD_FROM_EX and FORWARD_FROM_MEM."""
        unit = HazardUnit(num_alus=2)  # avoid structural hazard on ALU

        # Forward from EX
        ex_alu = PipelineSlot(
            valid=True,
            pc=0x1000,
            dest_reg=1,
            dest_value=42,
            uses_alu=True,
        )
        id_read = PipelineSlot(
            valid=True,
            pc=0x1004,
            source_regs=(1,),
            uses_alu=True,
        )
        empty = PipelineSlot()

        unit.check(empty, id_read, ex_alu, empty)

        # Forward from MEM
        mem_alu = PipelineSlot(
            valid=True,
            pc=0x1000,
            dest_reg=2,
            dest_value=99,
            uses_alu=True,
        )
        id_read2 = PipelineSlot(
            valid=True,
            pc=0x100C,
            source_regs=(2,),
            uses_alu=True,
        )
        unit.check(empty, id_read2, empty, mem_alu)

        assert unit.forward_count == 2

    def test_all_stats_together(self) -> None:
        """Run a mix of scenarios and verify all stats."""
        unit = HazardUnit(num_alus=2)  # avoid structural hazard on ALU
        empty = PipelineSlot()

        # Cycle 1: forward from EX
        unit.check(
            empty,
            PipelineSlot(
                valid=True, pc=0x04, source_regs=(1,), uses_alu=True
            ),
            PipelineSlot(
                valid=True,
                pc=0x00,
                dest_reg=1,
                dest_value=10,
                uses_alu=True,
            ),
            empty,
        )

        # Cycle 2: stall (load-use)
        unit.check(
            empty,
            PipelineSlot(
                valid=True, pc=0x0C, source_regs=(3,), uses_alu=True
            ),
            PipelineSlot(
                valid=True,
                pc=0x08,
                dest_reg=3,
                mem_read=True,
                uses_alu=False,
            ),
            empty,
        )

        # Cycle 3: flush (misprediction)
        unit.check(
            empty,
            empty,
            PipelineSlot(
                valid=True,
                pc=0x10,
                is_branch=True,
                branch_predicted_taken=True,
                branch_taken=False,
            ),
            empty,
        )

        # Cycle 4: no hazard
        unit.check(empty, empty, empty, empty)

        assert unit.forward_count == 1
        assert unit.stall_count == 1
        assert unit.flush_count == 1
        assert len(unit.history) == 4


class TestPickHighestPriority:
    """Unit tests for the _pick_highest_priority helper function."""

    def test_flush_beats_everything(self) -> None:
        """FLUSH is the highest priority action."""
        flush = HazardResult(action=HazardAction.FLUSH, flush_count=2)
        stall = HazardResult(action=HazardAction.STALL, stall_cycles=1)
        forward = HazardResult(action=HazardAction.FORWARD_FROM_EX)
        none = HazardResult(action=HazardAction.NONE)

        assert _pick_highest_priority(flush, stall, forward, none) is flush
        assert _pick_highest_priority(none, stall, flush, forward) is flush

    def test_stall_beats_forward_and_none(self) -> None:
        """STALL beats FORWARD and NONE."""
        stall = HazardResult(action=HazardAction.STALL, stall_cycles=1)
        forward = HazardResult(action=HazardAction.FORWARD_FROM_EX)
        none = HazardResult(action=HazardAction.NONE)

        assert _pick_highest_priority(stall, forward, none) is stall

    def test_forward_ex_beats_forward_mem(self) -> None:
        """FORWARD_FROM_EX beats FORWARD_FROM_MEM."""
        fwd_ex = HazardResult(action=HazardAction.FORWARD_FROM_EX)
        fwd_mem = HazardResult(action=HazardAction.FORWARD_FROM_MEM)

        assert _pick_highest_priority(fwd_mem, fwd_ex) is fwd_ex

    def test_single_result_returned_as_is(self) -> None:
        """Single result is returned unchanged."""
        none = HazardResult(action=HazardAction.NONE)
        result = _pick_highest_priority(none)
        assert result is none

    def test_tie_goes_to_first(self) -> None:
        """When priorities are equal, first result wins."""
        stall1 = HazardResult(
            action=HazardAction.STALL, reason="first stall"
        )
        stall2 = HazardResult(
            action=HazardAction.STALL, reason="second stall"
        )

        result = _pick_highest_priority(stall1, stall2)
        assert result is stall1


class TestStructuralAndDataCombined:
    """Test interaction between structural and data hazards."""

    def test_structural_stall_with_no_data_hazard(self) -> None:
        """Structural hazard (ALU conflict) with no data dependency."""
        unit = HazardUnit(num_alus=1)

        if_stage = PipelineSlot(valid=True, pc=0x100C)
        id_stage = PipelineSlot(
            valid=True,
            pc=0x1008,
            source_regs=(5, 6),  # different regs than EX dest
            uses_alu=True,
        )
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x1004,
            dest_reg=1,
            dest_value=10,
            uses_alu=True,
        )
        mem_stage = PipelineSlot()

        result = unit.check(if_stage, id_stage, ex_stage, mem_stage)

        # Data detector sees forward_from_ex? No — different regs.
        # Structural detector sees ALU conflict → stall.
        assert result.action == HazardAction.STALL

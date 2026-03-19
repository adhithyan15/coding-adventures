"""Tests for data hazard detection — RAW hazards, forwarding, and stalling.

These tests verify that the DataHazardDetector correctly identifies
data dependencies between pipeline stages and chooses the right
resolution strategy (forward vs. stall).
"""

from __future__ import annotations

from hazard_detection.data_hazard import DataHazardDetector
from hazard_detection.types import HazardAction, PipelineSlot


class TestRawForwardingFromEX:
    """RAW hazard where the value can be forwarded from the EX stage.

    Scenario:
        ADD R1, R2, R3    ← in EX stage (just computed R1 = 42)
        SUB R4, R1, R5    ← in ID stage (reads R1)

    The value of R1 is available in the EX stage (the ALU just produced it).
    We forward it directly to ID — zero stall cycles.
    """

    def setup_method(self) -> None:
        self.detector = DataHazardDetector()

    def test_single_source_reg_matches_ex_dest(self) -> None:
        """SUB reads R1, ADD in EX writes R1 → forward from EX."""
        id_stage = PipelineSlot(
            valid=True, pc=0x1004, source_regs=(1,), uses_alu=True
        )
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x1000,
            dest_reg=1,
            dest_value=42,
            uses_alu=True,
        )
        mem_stage = PipelineSlot()  # empty

        result = self.detector.detect(id_stage, ex_stage, mem_stage)

        assert result.action == HazardAction.FORWARD_FROM_EX
        assert result.forwarded_value == 42
        assert result.forwarded_from == "EX"
        assert result.stall_cycles == 0

    def test_second_source_reg_matches_ex_dest(self) -> None:
        """ADD R4, R5, R1 — R1 is the second source reg, still matches EX."""
        id_stage = PipelineSlot(
            valid=True, pc=0x1004, source_regs=(5, 1), uses_alu=True
        )
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x1000,
            dest_reg=1,
            dest_value=99,
            uses_alu=True,
        )
        mem_stage = PipelineSlot()

        result = self.detector.detect(id_stage, ex_stage, mem_stage)

        assert result.action == HazardAction.FORWARD_FROM_EX
        assert result.forwarded_value == 99


class TestRawForwardingFromMEM:
    """RAW hazard where the value is forwarded from the MEM stage.

    Scenario (2-instruction gap):
        ADD R1, R2, R3    ← in MEM stage (R1 computed 2 cycles ago)
        NOP               ← in EX stage  (no conflict)
        SUB R4, R1, R5    ← in ID stage  (reads R1)

    The value of R1 has passed through EX and is now in MEM.
    We forward from MEM.
    """

    def setup_method(self) -> None:
        self.detector = DataHazardDetector()

    def test_source_reg_matches_mem_dest(self) -> None:
        """R1 available in MEM stage → forward from MEM."""
        id_stage = PipelineSlot(
            valid=True, pc=0x100C, source_regs=(1,), uses_alu=True
        )
        ex_stage = PipelineSlot()  # NOP or different instruction
        mem_stage = PipelineSlot(
            valid=True,
            pc=0x1000,
            dest_reg=1,
            dest_value=77,
            uses_alu=True,
        )

        result = self.detector.detect(id_stage, ex_stage, mem_stage)

        assert result.action == HazardAction.FORWARD_FROM_MEM
        assert result.forwarded_value == 77
        assert result.forwarded_from == "MEM"

    def test_ex_takes_priority_over_mem_for_same_register(self) -> None:
        """If both EX and MEM write R1, EX is newer — use EX's value.

        This happens with back-to-back writes to the same register:
            ADD R1, R2, R3    ← in MEM (old value of R1)
            MUL R1, R4, R5    ← in EX  (new value of R1)
            SUB R6, R1, R7    ← in ID  (should get MUL's value)
        """
        id_stage = PipelineSlot(
            valid=True, pc=0x100C, source_regs=(1,), uses_alu=True
        )
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x1008,
            dest_reg=1,
            dest_value=200,  # newer value
            uses_alu=True,
        )
        mem_stage = PipelineSlot(
            valid=True,
            pc=0x1000,
            dest_reg=1,
            dest_value=100,  # older value
            uses_alu=True,
        )

        result = self.detector.detect(id_stage, ex_stage, mem_stage)

        assert result.action == HazardAction.FORWARD_FROM_EX
        assert result.forwarded_value == 200  # newer value from EX


class TestLoadUseHazard:
    """Load-use hazard: a load in EX followed by a use in ID — must stall.

    Scenario:
        LW R1, [addr]    ← in EX stage (value not available until after MEM)
        ADD R4, R1, R5   ← in ID stage (needs R1 in EX) — must stall!

    The load instruction's result won't be available until the MEM stage
    completes. But ADD needs it one cycle earlier, in EX. No amount of
    forwarding can bridge this 1-cycle gap. We must stall.
    """

    def setup_method(self) -> None:
        self.detector = DataHazardDetector()

    def test_load_followed_by_immediate_use_stalls(self) -> None:
        """LW R1 in EX, ADD using R1 in ID → stall 1 cycle."""
        id_stage = PipelineSlot(
            valid=True, pc=0x1004, source_regs=(1,), uses_alu=True
        )
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x1000,
            dest_reg=1,
            mem_read=True,  # this is a load instruction
            uses_alu=False,
        )
        mem_stage = PipelineSlot()

        result = self.detector.detect(id_stage, ex_stage, mem_stage)

        assert result.action == HazardAction.STALL
        assert result.stall_cycles == 1
        assert "load-use" in result.reason.lower()

    def test_load_with_gap_no_stall(self) -> None:
        """Load in MEM (not EX) + use in ID → forward from MEM, no stall.

        When there's a 1-instruction gap between load and use:
            LW R1, [addr]    ← now in MEM stage (load completing)
            NOP               ← in EX stage
            ADD R4, R1, R5   ← in ID stage

        The load value is available from MEM — forward it.
        """
        id_stage = PipelineSlot(
            valid=True, pc=0x100C, source_regs=(1,), uses_alu=True
        )
        ex_stage = PipelineSlot()  # NOP or unrelated
        mem_stage = PipelineSlot(
            valid=True,
            pc=0x1000,
            dest_reg=1,
            dest_value=55,
            mem_read=True,
            uses_alu=False,
        )

        result = self.detector.detect(id_stage, ex_stage, mem_stage)

        assert result.action == HazardAction.FORWARD_FROM_MEM
        assert result.forwarded_value == 55


class TestNoDataHazard:
    """Cases where no data hazard exists."""

    def setup_method(self) -> None:
        self.detector = DataHazardDetector()

    def test_different_registers_no_hazard(self) -> None:
        """Instructions use completely different registers — no conflict."""
        id_stage = PipelineSlot(
            valid=True, pc=0x1004, source_regs=(2, 3), uses_alu=True
        )
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x1000,
            dest_reg=1,
            dest_value=42,
            uses_alu=True,
        )
        mem_stage = PipelineSlot(
            valid=True,
            pc=0x0FFC,
            dest_reg=4,
            dest_value=10,
            uses_alu=True,
        )

        result = self.detector.detect(id_stage, ex_stage, mem_stage)

        assert result.action == HazardAction.NONE

    def test_empty_id_stage(self) -> None:
        """ID stage is a bubble (empty) — nothing to check."""
        id_stage = PipelineSlot(valid=False)
        ex_stage = PipelineSlot(
            valid=True, pc=0x1000, dest_reg=1, uses_alu=True
        )
        mem_stage = PipelineSlot()

        result = self.detector.detect(id_stage, ex_stage, mem_stage)

        assert result.action == HazardAction.NONE

    def test_id_has_no_source_regs(self) -> None:
        """Instruction reads no registers (e.g., NOP) — no dependency."""
        id_stage = PipelineSlot(
            valid=True, pc=0x1004, source_regs=(), uses_alu=True
        )
        ex_stage = PipelineSlot(
            valid=True, pc=0x1000, dest_reg=1, uses_alu=True
        )
        mem_stage = PipelineSlot()

        result = self.detector.detect(id_stage, ex_stage, mem_stage)

        assert result.action == HazardAction.NONE

    def test_ex_has_no_dest_reg(self) -> None:
        """EX instruction writes no register (e.g., store) — no conflict."""
        id_stage = PipelineSlot(
            valid=True, pc=0x1004, source_regs=(1,), uses_alu=True
        )
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x1000,
            dest_reg=None,  # store doesn't write a register
            mem_write=True,
            uses_alu=False,
        )
        mem_stage = PipelineSlot()

        result = self.detector.detect(id_stage, ex_stage, mem_stage)

        assert result.action == HazardAction.NONE

    def test_ex_is_empty_bubble(self) -> None:
        """EX stage is a bubble — no conflict possible."""
        id_stage = PipelineSlot(
            valid=True, pc=0x1004, source_regs=(1,), uses_alu=True
        )
        ex_stage = PipelineSlot(valid=False)
        mem_stage = PipelineSlot(valid=False)

        result = self.detector.detect(id_stage, ex_stage, mem_stage)

        assert result.action == HazardAction.NONE


class TestMultipleSourceRegisters:
    """Instructions with multiple source registers — mixed hazard cases."""

    def setup_method(self) -> None:
        self.detector = DataHazardDetector()

    def test_one_source_has_hazard_other_does_not(self) -> None:
        """ADD R4, R1, R5 — R1 has a hazard (EX writes R1), R5 is fine."""
        id_stage = PipelineSlot(
            valid=True, pc=0x1004, source_regs=(1, 5), uses_alu=True
        )
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x1000,
            dest_reg=1,
            dest_value=42,
            uses_alu=True,
        )
        mem_stage = PipelineSlot()

        result = self.detector.detect(id_stage, ex_stage, mem_stage)

        # Should forward because of R1 hazard
        assert result.action == HazardAction.FORWARD_FROM_EX
        assert result.forwarded_value == 42

    def test_stall_beats_forward(self) -> None:
        """If one source needs a stall and another needs forward, stall wins.

        LW R1, [addr]    ← in EX (load, must stall for R1)
        ADD R4, R1, R2   ← in ID (R1 needs stall, R2 needs forward from MEM)

        Even though R2 can be forwarded, R1 forces a stall. The stall is
        the higher-priority action.
        """
        id_stage = PipelineSlot(
            valid=True, pc=0x1008, source_regs=(1, 2), uses_alu=True
        )
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x1004,
            dest_reg=1,
            mem_read=True,  # load — must stall
            uses_alu=False,
        )
        mem_stage = PipelineSlot(
            valid=True,
            pc=0x1000,
            dest_reg=2,
            dest_value=88,
            uses_alu=True,
        )

        result = self.detector.detect(id_stage, ex_stage, mem_stage)

        # Stall takes priority over forward
        assert result.action == HazardAction.STALL
        assert result.stall_cycles == 1

    def test_both_sources_forward_from_different_stages(self) -> None:
        """R1 forwards from EX, R2 forwards from MEM.

        EX is higher priority than MEM, so the result is FORWARD_FROM_EX.
        """
        id_stage = PipelineSlot(
            valid=True, pc=0x100C, source_regs=(1, 2), uses_alu=True
        )
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x1008,
            dest_reg=1,
            dest_value=10,
            uses_alu=True,
        )
        mem_stage = PipelineSlot(
            valid=True,
            pc=0x1000,
            dest_reg=2,
            dest_value=20,
            uses_alu=True,
        )

        result = self.detector.detect(id_stage, ex_stage, mem_stage)

        # FORWARD_FROM_EX has higher priority than FORWARD_FROM_MEM
        assert result.action == HazardAction.FORWARD_FROM_EX
        assert result.forwarded_value == 10

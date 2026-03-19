"""Tests for structural hazard detection — resource conflict handling.

These tests verify that the StructuralHazardDetector correctly identifies
when two instructions compete for the same hardware resource.
"""

from __future__ import annotations

from hazard_detection.structural_hazard import StructuralHazardDetector
from hazard_detection.types import HazardAction, PipelineSlot


class TestALUConflict:
    """Two ALU instructions in adjacent stages with limited ALUs."""

    def test_two_alu_instructions_one_alu_stalls(self) -> None:
        """With 1 ALU, two ALU instructions at once → stall.

        ADD R1, R2, R3   ← in EX (using the ALU)
        SUB R4, R5, R6   ← in ID (about to enter EX, needs ALU)
        Only 1 ALU → SUB must wait.
        """
        detector = StructuralHazardDetector(num_alus=1)
        id_stage = PipelineSlot(
            valid=True, pc=0x1004, uses_alu=True, source_regs=(5, 6)
        )
        ex_stage = PipelineSlot(
            valid=True, pc=0x1000, uses_alu=True, dest_reg=1
        )

        result = detector.detect(id_stage, ex_stage)

        assert result.action == HazardAction.STALL
        assert result.stall_cycles == 1
        assert "ALU" in result.reason

    def test_two_alu_instructions_two_alus_no_stall(self) -> None:
        """With 2 ALUs, two ALU instructions can execute in parallel."""
        detector = StructuralHazardDetector(num_alus=2)
        id_stage = PipelineSlot(
            valid=True, pc=0x1004, uses_alu=True, source_regs=(5, 6)
        )
        ex_stage = PipelineSlot(
            valid=True, pc=0x1000, uses_alu=True, dest_reg=1
        )

        result = detector.detect(id_stage, ex_stage)

        assert result.action == HazardAction.NONE

    def test_one_alu_one_fp_no_conflict(self) -> None:
        """ALU + FP instruction at the same time — different units, no conflict."""
        detector = StructuralHazardDetector(num_alus=1, num_fp_units=1)
        id_stage = PipelineSlot(
            valid=True, pc=0x1004, uses_alu=True, uses_fp=False
        )
        ex_stage = PipelineSlot(
            valid=True, pc=0x1000, uses_alu=False, uses_fp=True
        )

        result = detector.detect(id_stage, ex_stage)

        assert result.action == HazardAction.NONE


class TestFPUnitConflict:
    """Two FP instructions with limited FP units."""

    def test_two_fp_instructions_one_unit_stalls(self) -> None:
        """With 1 FP unit, two FP instructions → stall."""
        detector = StructuralHazardDetector(num_fp_units=1)
        id_stage = PipelineSlot(
            valid=True, pc=0x1004, uses_fp=True, uses_alu=False
        )
        ex_stage = PipelineSlot(
            valid=True, pc=0x1000, uses_fp=True, uses_alu=False
        )

        result = detector.detect(id_stage, ex_stage)

        assert result.action == HazardAction.STALL
        assert "FP unit" in result.reason

    def test_two_fp_instructions_two_units_no_stall(self) -> None:
        """With 2 FP units, two FP instructions → no stall."""
        detector = StructuralHazardDetector(num_fp_units=2)
        id_stage = PipelineSlot(
            valid=True, pc=0x1004, uses_fp=True, uses_alu=False
        )
        ex_stage = PipelineSlot(
            valid=True, pc=0x1000, uses_fp=True, uses_alu=False
        )

        result = detector.detect(id_stage, ex_stage)

        assert result.action == HazardAction.NONE


class TestMemoryPortConflict:
    """Fetch and data access competing for a shared memory bus."""

    def test_split_caches_no_conflict(self) -> None:
        """With split L1I/L1D caches, IF and MEM never conflict."""
        detector = StructuralHazardDetector(split_caches=True)
        id_stage = PipelineSlot(valid=True, pc=0x1004, uses_alu=False)
        ex_stage = PipelineSlot(valid=True, pc=0x1000, uses_alu=False)
        if_stage = PipelineSlot(valid=True, pc=0x100C)
        mem_stage = PipelineSlot(
            valid=True, pc=0x0FF8, mem_read=True, uses_alu=False
        )

        result = detector.detect(
            id_stage, ex_stage, if_stage=if_stage, mem_stage=mem_stage
        )

        assert result.action == HazardAction.NONE

    def test_shared_cache_fetch_and_load_stalls(self) -> None:
        """With shared cache, IF and MEM (load) → stall."""
        detector = StructuralHazardDetector(split_caches=False)
        id_stage = PipelineSlot(valid=True, pc=0x1004, uses_alu=False)
        ex_stage = PipelineSlot(valid=True, pc=0x1000, uses_alu=False)
        if_stage = PipelineSlot(valid=True, pc=0x100C)
        mem_stage = PipelineSlot(
            valid=True, pc=0x0FF8, mem_read=True, uses_alu=False
        )

        result = detector.detect(
            id_stage, ex_stage, if_stage=if_stage, mem_stage=mem_stage
        )

        assert result.action == HazardAction.STALL
        assert "memory bus" in result.reason.lower()

    def test_shared_cache_fetch_and_store_stalls(self) -> None:
        """With shared cache, IF and MEM (store) → stall."""
        detector = StructuralHazardDetector(split_caches=False)
        id_stage = PipelineSlot(valid=True, pc=0x1004, uses_alu=False)
        ex_stage = PipelineSlot(valid=True, pc=0x1000, uses_alu=False)
        if_stage = PipelineSlot(valid=True, pc=0x100C)
        mem_stage = PipelineSlot(
            valid=True, pc=0x0FF8, mem_write=True, uses_alu=False
        )

        result = detector.detect(
            id_stage, ex_stage, if_stage=if_stage, mem_stage=mem_stage
        )

        assert result.action == HazardAction.STALL

    def test_shared_cache_no_mem_access_no_conflict(self) -> None:
        """Shared cache but MEM isn't doing a load/store — no conflict."""
        detector = StructuralHazardDetector(split_caches=False)
        id_stage = PipelineSlot(valid=True, pc=0x1004, uses_alu=False)
        ex_stage = PipelineSlot(valid=True, pc=0x1000, uses_alu=False)
        if_stage = PipelineSlot(valid=True, pc=0x100C)
        mem_stage = PipelineSlot(
            valid=True, pc=0x0FF8, mem_read=False, mem_write=False
        )

        result = detector.detect(
            id_stage, ex_stage, if_stage=if_stage, mem_stage=mem_stage
        )

        assert result.action == HazardAction.NONE


class TestEdgeCases:
    """Edge cases — empty stages, no if/mem provided."""

    def test_empty_id_stage_no_hazard(self) -> None:
        """ID stage is a bubble — can't have a structural hazard."""
        detector = StructuralHazardDetector(num_alus=1)
        id_stage = PipelineSlot(valid=False)
        ex_stage = PipelineSlot(valid=True, pc=0x1000, uses_alu=True)

        result = detector.detect(id_stage, ex_stage)

        assert result.action == HazardAction.NONE

    def test_empty_ex_stage_no_hazard(self) -> None:
        """EX stage is a bubble — resource is free."""
        detector = StructuralHazardDetector(num_alus=1)
        id_stage = PipelineSlot(valid=True, pc=0x1004, uses_alu=True)
        ex_stage = PipelineSlot(valid=False)

        result = detector.detect(id_stage, ex_stage)

        assert result.action == HazardAction.NONE

    def test_no_if_mem_stages_provided(self) -> None:
        """If if_stage and mem_stage are not provided, skip memory check."""
        detector = StructuralHazardDetector(
            num_alus=2, split_caches=False
        )
        id_stage = PipelineSlot(valid=True, pc=0x1004, uses_alu=True)
        ex_stage = PipelineSlot(valid=True, pc=0x1000, uses_alu=True)

        # Only checking execution unit conflict (2 ALUs → no stall)
        result = detector.detect(id_stage, ex_stage)

        assert result.action == HazardAction.NONE

    def test_shared_cache_mem_stage_empty(self) -> None:
        """Shared cache but MEM stage is a bubble — no conflict."""
        detector = StructuralHazardDetector(split_caches=False)
        id_stage = PipelineSlot(valid=True, pc=0x1004, uses_alu=False)
        ex_stage = PipelineSlot(valid=True, pc=0x1000, uses_alu=False)
        if_stage = PipelineSlot(valid=True, pc=0x100C)
        mem_stage = PipelineSlot(valid=False)

        result = detector.detect(
            id_stage, ex_stage, if_stage=if_stage, mem_stage=mem_stage
        )

        assert result.action == HazardAction.NONE

"""Tests for control hazard detection — branch misprediction handling.

These tests verify that the ControlHazardDetector correctly identifies
branch mispredictions and signals the pipeline to flush.
"""

from __future__ import annotations

from hazard_detection.control_hazard import ControlHazardDetector
from hazard_detection.types import HazardAction, PipelineSlot


class TestCorrectlyPredictedBranch:
    """When the branch predictor guessed right — no hazard."""

    def setup_method(self) -> None:
        self.detector = ControlHazardDetector()

    def test_predicted_taken_actually_taken(self) -> None:
        """Predictor said 'taken', branch IS taken — correct, no flush."""
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x1000,
            is_branch=True,
            branch_predicted_taken=True,
            branch_taken=True,
        )

        result = self.detector.detect(ex_stage)

        assert result.action == HazardAction.NONE
        assert result.flush_count == 0
        assert "correctly predicted" in result.reason

    def test_predicted_not_taken_actually_not_taken(self) -> None:
        """Predictor said 'not taken', branch is NOT taken — correct."""
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x2000,
            is_branch=True,
            branch_predicted_taken=False,
            branch_taken=False,
        )

        result = self.detector.detect(ex_stage)

        assert result.action == HazardAction.NONE
        assert result.flush_count == 0


class TestMispredictedBranch:
    """When the branch predictor guessed wrong — must flush."""

    def setup_method(self) -> None:
        self.detector = ControlHazardDetector()

    def test_predicted_not_taken_but_taken(self) -> None:
        """Predictor said 'not taken', but branch IS taken → FLUSH.

        The pipeline fetched fall-through instructions, but it should
        have jumped to the branch target. Flush IF and ID.
        """
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x1000,
            is_branch=True,
            branch_predicted_taken=False,
            branch_taken=True,
        )

        result = self.detector.detect(ex_stage)

        assert result.action == HazardAction.FLUSH
        assert result.flush_count == 2  # IF and ID
        assert "not-taken, actually taken" in result.reason

    def test_predicted_taken_but_not_taken(self) -> None:
        """Predictor said 'taken', but branch is NOT taken → FLUSH.

        The pipeline fetched from the branch target, but it should
        have continued with the fall-through path.
        """
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x3000,
            is_branch=True,
            branch_predicted_taken=True,
            branch_taken=False,
        )

        result = self.detector.detect(ex_stage)

        assert result.action == HazardAction.FLUSH
        assert result.flush_count == 2
        assert "taken, actually not-taken" in result.reason


class TestNonBranchInstruction:
    """Non-branch instructions can never cause a control hazard."""

    def setup_method(self) -> None:
        self.detector = ControlHazardDetector()

    def test_alu_instruction_no_control_hazard(self) -> None:
        """An ADD instruction in EX — no control hazard possible."""
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x1000,
            is_branch=False,
            source_regs=(2, 3),
            dest_reg=1,
            uses_alu=True,
        )

        result = self.detector.detect(ex_stage)

        assert result.action == HazardAction.NONE
        assert "not a branch" in result.reason

    def test_load_instruction_no_control_hazard(self) -> None:
        """A load instruction in EX — not a branch, no control hazard."""
        ex_stage = PipelineSlot(
            valid=True,
            pc=0x1000,
            is_branch=False,
            dest_reg=1,
            mem_read=True,
            uses_alu=False,
        )

        result = self.detector.detect(ex_stage)

        assert result.action == HazardAction.NONE


class TestEmptyEXStage:
    """Empty EX stage (bubble) — nothing to check."""

    def setup_method(self) -> None:
        self.detector = ControlHazardDetector()

    def test_empty_ex_no_hazard(self) -> None:
        """EX stage is a bubble — no instruction, no hazard."""
        ex_stage = PipelineSlot(valid=False)

        result = self.detector.detect(ex_stage)

        assert result.action == HazardAction.NONE
        assert "empty" in result.reason.lower()

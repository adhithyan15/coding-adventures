"""Tests for static branch predictors — AlwaysTaken, AlwaysNotTaken, BTFNT.

Static predictors are simple strategies that don't learn from history.
These tests verify their fixed prediction behavior and accuracy tracking.
"""

from __future__ import annotations

from branch_predictor.static import (
    AlwaysNotTakenPredictor,
    AlwaysTakenPredictor,
    BackwardTakenForwardNotTaken,
)


# ─── AlwaysTakenPredictor ─────────────────────────────────────────────────────


class TestAlwaysTaken:
    """Tests for AlwaysTakenPredictor."""

    def test_always_predicts_taken(self) -> None:
        p = AlwaysTakenPredictor()
        for pc in [0x0, 0x100, 0xDEAD, 0xFFFF_FFFF]:
            pred = p.predict(pc)
            assert pred.taken is True

    def test_confidence_is_zero(self) -> None:
        """Static predictors have no confidence — they're just guessing."""
        p = AlwaysTakenPredictor()
        assert p.predict(0x100).confidence == 0.0

    def test_target_is_none(self) -> None:
        """Static predictors don't know target addresses."""
        p = AlwaysTakenPredictor()
        assert p.predict(0x100).target is None

    def test_100_percent_on_all_taken(self) -> None:
        """Perfect accuracy when every branch is taken."""
        p = AlwaysTakenPredictor()
        for i in range(100):
            p.update(pc=i * 4, taken=True)
        assert p.stats.accuracy == 100.0

    def test_0_percent_on_all_not_taken(self) -> None:
        """Zero accuracy when no branch is taken."""
        p = AlwaysTakenPredictor()
        for i in range(100):
            p.update(pc=i * 4, taken=False)
        assert p.stats.accuracy == 0.0

    def test_mixed_sequence(self) -> None:
        """60% taken → 60% accuracy for always-taken."""
        p = AlwaysTakenPredictor()
        for i in range(60):
            p.update(pc=0x100, taken=True)
        for i in range(40):
            p.update(pc=0x100, taken=False)
        assert p.stats.accuracy == 60.0

    def test_reset_clears_stats(self) -> None:
        p = AlwaysTakenPredictor()
        p.update(pc=0x100, taken=True)
        p.reset()
        assert p.stats.predictions == 0


# ─── AlwaysNotTakenPredictor ──────────────────────────────────────────────────


class TestAlwaysNotTaken:
    """Tests for AlwaysNotTakenPredictor."""

    def test_always_predicts_not_taken(self) -> None:
        p = AlwaysNotTakenPredictor()
        for pc in [0x0, 0x100, 0xDEAD]:
            pred = p.predict(pc)
            assert pred.taken is False

    def test_100_percent_on_all_not_taken(self) -> None:
        p = AlwaysNotTakenPredictor()
        for i in range(100):
            p.update(pc=i * 4, taken=False)
        assert p.stats.accuracy == 100.0

    def test_0_percent_on_all_taken(self) -> None:
        p = AlwaysNotTakenPredictor()
        for i in range(100):
            p.update(pc=i * 4, taken=True)
        assert p.stats.accuracy == 0.0

    def test_inverse_of_always_taken(self) -> None:
        """AlwaysNotTaken's accuracy is 100% - AlwaysTaken's accuracy."""
        taken_pred = AlwaysTakenPredictor()
        not_taken_pred = AlwaysNotTakenPredictor()
        # 70% taken sequence
        outcomes = [True] * 70 + [False] * 30
        for i, taken in enumerate(outcomes):
            taken_pred.update(pc=i * 4, taken=taken)
            not_taken_pred.update(pc=i * 4, taken=taken)
        assert taken_pred.stats.accuracy == 70.0
        assert not_taken_pred.stats.accuracy == 30.0
        assert (
            abs(taken_pred.stats.accuracy + not_taken_pred.stats.accuracy - 100.0)
            < 1e-10
        )

    def test_reset_clears_stats(self) -> None:
        p = AlwaysNotTakenPredictor()
        p.update(pc=0x100, taken=False)
        p.reset()
        assert p.stats.predictions == 0


# ─── BackwardTakenForwardNotTaken ─────────────────────────────────────────────


class TestBTFNT:
    """Tests for BackwardTakenForwardNotTaken predictor."""

    def test_cold_start_predicts_not_taken(self) -> None:
        """Before seeing a target, default to not-taken."""
        p = BackwardTakenForwardNotTaken()
        pred = p.predict(pc=0x108)
        assert pred.taken is False

    def test_backward_branch_predicts_taken(self) -> None:
        """Backward branch (target < pc) → predict taken."""
        p = BackwardTakenForwardNotTaken()
        # First, teach the predictor the target
        p.update(pc=0x108, taken=True, target=0x100)
        # Now it knows target=0x100 < pc=0x108 → backward → taken
        pred = p.predict(pc=0x108)
        assert pred.taken is True

    def test_forward_branch_predicts_not_taken(self) -> None:
        """Forward branch (target > pc) → predict not-taken."""
        p = BackwardTakenForwardNotTaken()
        p.update(pc=0x200, taken=False, target=0x20C)
        pred = p.predict(pc=0x200)
        assert pred.taken is False

    def test_equal_target_predicts_taken(self) -> None:
        """target == pc (degenerate loop) → predict taken."""
        p = BackwardTakenForwardNotTaken()
        p.update(pc=0x100, taken=True, target=0x100)
        pred = p.predict(pc=0x100)
        assert pred.taken is True

    def test_backward_branch_accuracy_on_loop(self) -> None:
        """A loop (backward branch, taken 9 times, not-taken once).

        After the first update (which teaches the target), the predictor
        knows this is a backward branch → predicts taken.
        """
        p = BackwardTakenForwardNotTaken()
        pc = 0x108
        target = 0x100  # backward

        # Run the loop 10 times (9 taken + 1 not-taken)
        for i in range(10):
            taken = i < 9  # taken on iterations 0-8, not-taken on 9
            p.update(pc=pc, taken=taken, target=target)

        # BTFNT always predicts taken for backward branches (target < pc):
        # Updates 0-8: backward → predicted taken, actual taken → correct (9)
        # Update 9: backward → predicted taken, actual not-taken → WRONG (1)
        assert p.stats.correct == 9
        assert p.stats.incorrect == 1

    def test_forward_branch_accuracy(self) -> None:
        """Forward branch, not taken 8 out of 10 times."""
        p = BackwardTakenForwardNotTaken()
        pc = 0x200
        target = 0x20C  # forward

        outcomes = [False] * 8 + [True] * 2
        for taken in outcomes:
            p.update(pc=pc, taken=taken, target=target)

        # Update 0: no prior target → predicted not-taken, actual not-taken → correct
        # Updates 1-7: forward → predicted not-taken, actual not-taken → correct (7)
        # Updates 8-9: forward → predicted not-taken, actual taken → WRONG (2)
        assert p.stats.correct == 8
        assert p.stats.incorrect == 2

    def test_confidence_on_known_branch(self) -> None:
        """Known branches should have moderate confidence (0.5)."""
        p = BackwardTakenForwardNotTaken()
        p.update(pc=0x108, taken=True, target=0x100)
        pred = p.predict(pc=0x108)
        assert pred.confidence == 0.5

    def test_confidence_on_unknown_branch(self) -> None:
        """Unknown branches should have zero confidence."""
        p = BackwardTakenForwardNotTaken()
        pred = p.predict(pc=0x108)
        assert pred.confidence == 0.0

    def test_target_in_prediction(self) -> None:
        """Known branches include target in prediction."""
        p = BackwardTakenForwardNotTaken()
        p.update(pc=0x108, taken=True, target=0x100)
        pred = p.predict(pc=0x108)
        assert pred.target == 0x100

    def test_reset_clears_targets_and_stats(self) -> None:
        p = BackwardTakenForwardNotTaken()
        p.update(pc=0x108, taken=True, target=0x100)
        p.reset()
        assert p.stats.predictions == 0
        # After reset, should be cold start again
        pred = p.predict(pc=0x108)
        assert pred.taken is False  # no target known

    def test_multiple_branches(self) -> None:
        """Multiple branches with different directions."""
        p = BackwardTakenForwardNotTaken()
        # Branch A: backward (loop)
        p.update(pc=0x108, taken=True, target=0x100)
        # Branch B: forward (if-else)
        p.update(pc=0x200, taken=False, target=0x20C)

        assert p.predict(pc=0x108).taken is True  # backward → taken
        assert p.predict(pc=0x200).taken is False  # forward → not taken

    def test_update_without_target(self) -> None:
        """Update with target=None doesn't crash or overwrite existing target."""
        p = BackwardTakenForwardNotTaken()
        p.update(pc=0x108, taken=True, target=0x100)
        # Update without target — should preserve old target
        p.update(pc=0x108, taken=True, target=None)
        pred = p.predict(pc=0x108)
        assert pred.taken is True  # still knows it's backward

"""Tests for PredictionStats — the branch prediction scoreboard.

These tests verify the accuracy tracking, edge cases, and reset behavior
of the PredictionStats dataclass.
"""

from __future__ import annotations

from branch_predictor.stats import PredictionStats


class TestPredictionStatsCreation:
    """Test that PredictionStats initializes with zeroed counters."""

    def test_default_counters_are_zero(self) -> None:
        stats = PredictionStats()
        assert stats.predictions == 0
        assert stats.correct == 0
        assert stats.incorrect == 0

    def test_custom_initial_values(self) -> None:
        stats = PredictionStats(predictions=10, correct=7, incorrect=3)
        assert stats.predictions == 10
        assert stats.correct == 7
        assert stats.incorrect == 3


class TestAccuracyCalculation:
    """Test accuracy and misprediction rate calculations."""

    def test_accuracy_with_no_predictions(self) -> None:
        """0 predictions → 0.0% accuracy (not a division error)."""
        stats = PredictionStats()
        assert stats.accuracy == 0.0

    def test_misprediction_rate_with_no_predictions(self) -> None:
        """0 predictions → 0.0% misprediction rate."""
        stats = PredictionStats()
        assert stats.misprediction_rate == 0.0

    def test_perfect_accuracy(self) -> None:
        stats = PredictionStats(predictions=100, correct=100, incorrect=0)
        assert stats.accuracy == 100.0
        assert stats.misprediction_rate == 0.0

    def test_zero_accuracy(self) -> None:
        stats = PredictionStats(predictions=100, correct=0, incorrect=100)
        assert stats.accuracy == 0.0
        assert stats.misprediction_rate == 100.0

    def test_mixed_accuracy(self) -> None:
        stats = PredictionStats(predictions=200, correct=150, incorrect=50)
        assert stats.accuracy == 75.0
        assert stats.misprediction_rate == 25.0

    def test_accuracy_and_misprediction_sum_to_100(self) -> None:
        stats = PredictionStats(predictions=37, correct=23, incorrect=14)
        assert abs(stats.accuracy + stats.misprediction_rate - 100.0) < 1e-10


class TestRecord:
    """Test the record() method for logging prediction outcomes."""

    def test_record_correct(self) -> None:
        stats = PredictionStats()
        stats.record(correct=True)
        assert stats.predictions == 1
        assert stats.correct == 1
        assert stats.incorrect == 0

    def test_record_incorrect(self) -> None:
        stats = PredictionStats()
        stats.record(correct=False)
        assert stats.predictions == 1
        assert stats.correct == 0
        assert stats.incorrect == 1

    def test_record_sequence(self) -> None:
        """Record a mixed sequence and verify counts."""
        stats = PredictionStats()
        outcomes = [True, True, False, True, False, True, True, True, True, False]
        for outcome in outcomes:
            stats.record(correct=outcome)
        assert stats.predictions == 10
        assert stats.correct == 7
        assert stats.incorrect == 3
        assert stats.accuracy == 70.0


class TestReset:
    """Test the reset() method for clearing all counters."""

    def test_reset_clears_all_counters(self) -> None:
        stats = PredictionStats(predictions=50, correct=40, incorrect=10)
        stats.reset()
        assert stats.predictions == 0
        assert stats.correct == 0
        assert stats.incorrect == 0

    def test_reset_after_recording(self) -> None:
        stats = PredictionStats()
        for _ in range(20):
            stats.record(correct=True)
        stats.reset()
        assert stats.predictions == 0
        assert stats.accuracy == 0.0

    def test_record_after_reset(self) -> None:
        """Verify stats work correctly after a reset."""
        stats = PredictionStats()
        stats.record(correct=True)
        stats.record(correct=False)
        stats.reset()
        stats.record(correct=True)
        assert stats.predictions == 1
        assert stats.correct == 1
        assert stats.accuracy == 100.0

"""Tests for the one-bit branch predictor.

The one-bit predictor stores a single bit per branch: the last outcome.
These tests verify its learning behavior, the double-misprediction problem
on loops, and the aliasing problem with small tables.
"""

from __future__ import annotations

from branch_predictor.one_bit import OneBitPredictor


class TestOneBitBasics:
    """Basic predict/update behavior."""

    def test_cold_start_predicts_not_taken(self) -> None:
        """Uninitialized entries default to not-taken (bit = 0)."""
        p = OneBitPredictor()
        pred = p.predict(pc=0x100)
        assert pred.taken is False

    def test_predicts_last_outcome_taken(self) -> None:
        """After a taken branch, predicts taken next time."""
        p = OneBitPredictor()
        p.update(pc=0x100, taken=True)
        pred = p.predict(pc=0x100)
        assert pred.taken is True

    def test_predicts_last_outcome_not_taken(self) -> None:
        """After a not-taken branch, predicts not-taken."""
        p = OneBitPredictor()
        p.update(pc=0x100, taken=True)
        p.update(pc=0x100, taken=False)
        pred = p.predict(pc=0x100)
        assert pred.taken is False

    def test_confidence_is_half(self) -> None:
        """One bit of history → moderate confidence."""
        p = OneBitPredictor()
        assert p.predict(0x100).confidence == 0.5

    def test_different_branches_independent(self) -> None:
        """Two branches at different PCs have independent state."""
        p = OneBitPredictor(table_size=4096)  # large table to avoid aliasing
        p.update(pc=0x100, taken=True)
        p.update(pc=0x200, taken=False)
        assert p.predict(pc=0x100).taken is True
        assert p.predict(pc=0x200).taken is False


class TestOneBitLoopPattern:
    """The double-misprediction problem: 1-bit predictors mispredict twice per loop."""

    def test_loop_mispredicts_first_and_last(self) -> None:
        """A loop running 10 times: TTTTTTTTTN (9 taken, 1 not-taken).

        Expected mispredictions:
        - Iteration 1 (cold start): predict NT, actual T → WRONG
        - Iterations 2-9: predict T, actual T → correct (8x)
        - Iteration 10: predict T, actual NT → WRONG

        Total: 8 correct, 2 incorrect = 80% accuracy
        """
        p = OneBitPredictor()
        pc = 0x100

        for i in range(10):
            taken = i < 9
            p.update(pc=pc, taken=taken)

        assert p.stats.correct == 8
        assert p.stats.incorrect == 2
        assert p.stats.accuracy == 80.0

    def test_loop_repeated_invocations(self) -> None:
        """Running the same loop twice → mispredicts on re-entry too.

        First run: TTTTTTTTTN (cold start miss + exit miss = 2 wrong)
        Second run: TTTTTTTTTN (re-entry miss + exit miss = 2 wrong)

        After first run, bit = 0 (last outcome was NT).
        Second run iter 1: predict NT, actual T → WRONG.
        """
        p = OneBitPredictor()
        pc = 0x100

        # First invocation: 10 iterations
        for i in range(10):
            p.update(pc=pc, taken=(i < 9))

        # Second invocation: 10 more iterations
        for i in range(10):
            p.update(pc=pc, taken=(i < 9))

        # 2 mispredictions per invocation × 2 invocations = 4 wrong
        assert p.stats.incorrect == 4
        assert p.stats.correct == 16


class TestOneBitAliasing:
    """Table aliasing: two branches mapping to the same entry."""

    def test_aliasing_with_small_table(self) -> None:
        """With table_size=4, PCs differing by 4 alias to the same slot.

        Branch A at 0x100 → index 0 (0x100 % 4 = 0)
        Branch B at 0x104 → index 0 (0x104 % 4 = 0)

        They corrupt each other's predictions.
        """
        p = OneBitPredictor(table_size=4)

        # Branch A: taken
        p.update(pc=0x100, taken=True)
        assert p.predict(pc=0x100).taken is True

        # Branch B overwrites the same slot: not-taken
        p.update(pc=0x104, taken=False)

        # Branch A now sees B's state (not-taken) → WRONG
        assert p.predict(pc=0x100).taken is False

    def test_no_aliasing_with_large_table(self) -> None:
        """With a large enough table, nearby branches don't alias."""
        p = OneBitPredictor(table_size=4096)
        p.update(pc=0x100, taken=True)
        p.update(pc=0x104, taken=False)
        # Different indices → independent
        assert p.predict(pc=0x100).taken is True
        assert p.predict(pc=0x104).taken is False


class TestOneBitReset:
    """Reset clears the table and stats."""

    def test_reset_clears_table(self) -> None:
        p = OneBitPredictor()
        p.update(pc=0x100, taken=True)
        p.reset()
        # After reset, cold start again
        assert p.predict(pc=0x100).taken is False

    def test_reset_clears_stats(self) -> None:
        p = OneBitPredictor()
        p.update(pc=0x100, taken=True)
        p.reset()
        assert p.stats.predictions == 0


class TestOneBitAlternatingPattern:
    """Alternating branches (TNTNTN...) are worst-case for 1-bit."""

    def test_alternating_is_worst_case(self) -> None:
        """Alternating T/NT: every prediction is wrong after the first.

        Seq: T, N, T, N, T, N
        Pred: NT(cold), T, N, T, N, T  → all wrong except... none!
        Actually: pred NT vs T→wrong, pred T vs N→wrong, pred N vs T→wrong...
        Every prediction is wrong because it always predicts the opposite.
        """
        p = OneBitPredictor()
        pc = 0x100
        for i in range(100):
            taken = i % 2 == 0  # alternating T, N, T, N, ...
            p.update(pc=pc, taken=taken)

        # Every prediction is wrong: 0% accuracy
        assert p.stats.accuracy == 0.0

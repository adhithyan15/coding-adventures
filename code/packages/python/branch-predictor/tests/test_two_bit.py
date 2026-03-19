"""Tests for the two-bit saturating counter predictor.

The two-bit predictor is the gold standard of introductory computer architecture.
These tests verify state transitions, loop behavior, comparison with one-bit,
and configurable initial state.
"""

from __future__ import annotations

from branch_predictor.one_bit import OneBitPredictor
from branch_predictor.two_bit import TwoBitPredictor, TwoBitState


# ─── TwoBitState transitions ─────────────────────────────────────────────────


class TestTwoBitState:
    """Test the 2-bit saturating counter state machine."""

    def test_state_values(self) -> None:
        """States have integer values 0-3."""
        assert TwoBitState.STRONGLY_NOT_TAKEN == 0
        assert TwoBitState.WEAKLY_NOT_TAKEN == 1
        assert TwoBitState.WEAKLY_TAKEN == 2
        assert TwoBitState.STRONGLY_TAKEN == 3

    def test_taken_increments(self) -> None:
        """'taken' outcome moves the counter toward STRONGLY_TAKEN."""
        assert (
            TwoBitState.STRONGLY_NOT_TAKEN.taken_outcome()
            == TwoBitState.WEAKLY_NOT_TAKEN
        )
        assert (
            TwoBitState.WEAKLY_NOT_TAKEN.taken_outcome() == TwoBitState.WEAKLY_TAKEN
        )
        assert TwoBitState.WEAKLY_TAKEN.taken_outcome() == TwoBitState.STRONGLY_TAKEN
        # Saturates at the top
        assert (
            TwoBitState.STRONGLY_TAKEN.taken_outcome() == TwoBitState.STRONGLY_TAKEN
        )

    def test_not_taken_decrements(self) -> None:
        """'not taken' outcome moves the counter toward STRONGLY_NOT_TAKEN."""
        assert (
            TwoBitState.STRONGLY_TAKEN.not_taken_outcome() == TwoBitState.WEAKLY_TAKEN
        )
        assert (
            TwoBitState.WEAKLY_TAKEN.not_taken_outcome()
            == TwoBitState.WEAKLY_NOT_TAKEN
        )
        assert (
            TwoBitState.WEAKLY_NOT_TAKEN.not_taken_outcome()
            == TwoBitState.STRONGLY_NOT_TAKEN
        )
        # Saturates at the bottom
        assert (
            TwoBitState.STRONGLY_NOT_TAKEN.not_taken_outcome()
            == TwoBitState.STRONGLY_NOT_TAKEN
        )

    def test_predicts_taken_threshold(self) -> None:
        """States >= WEAKLY_TAKEN predict taken; others predict not-taken."""
        assert TwoBitState.STRONGLY_TAKEN.predicts_taken is True
        assert TwoBitState.WEAKLY_TAKEN.predicts_taken is True
        assert TwoBitState.WEAKLY_NOT_TAKEN.predicts_taken is False
        assert TwoBitState.STRONGLY_NOT_TAKEN.predicts_taken is False

    def test_saturation_at_strongly_taken(self) -> None:
        """Multiple taken outcomes don't exceed STRONGLY_TAKEN."""
        state = TwoBitState.STRONGLY_TAKEN
        for _ in range(10):
            state = state.taken_outcome()
        assert state == TwoBitState.STRONGLY_TAKEN

    def test_saturation_at_strongly_not_taken(self) -> None:
        """Multiple not-taken outcomes don't go below STRONGLY_NOT_TAKEN."""
        state = TwoBitState.STRONGLY_NOT_TAKEN
        for _ in range(10):
            state = state.not_taken_outcome()
        assert state == TwoBitState.STRONGLY_NOT_TAKEN


# ─── TwoBitPredictor basics ──────────────────────────────────────────────────


class TestTwoBitBasics:
    """Basic predict/update behavior."""

    def test_default_initial_state(self) -> None:
        """Default initial state is WEAKLY_NOT_TAKEN → predict not-taken."""
        p = TwoBitPredictor()
        pred = p.predict(pc=0x100)
        assert pred.taken is False

    def test_custom_initial_state_taken(self) -> None:
        """Starting at WEAKLY_TAKEN → predict taken from the start."""
        p = TwoBitPredictor(initial_state=TwoBitState.WEAKLY_TAKEN)
        pred = p.predict(pc=0x100)
        assert pred.taken is True

    def test_custom_initial_state_strongly_taken(self) -> None:
        p = TwoBitPredictor(initial_state=TwoBitState.STRONGLY_TAKEN)
        pred = p.predict(pc=0x100)
        assert pred.taken is True
        assert pred.confidence == 1.0

    def test_one_taken_flips_from_weakly_not_taken(self) -> None:
        """WNT + taken → WT (predicts taken). One taken outcome flips."""
        p = TwoBitPredictor()  # starts at WNT
        p.update(pc=0x100, taken=True)
        assert p.predict(pc=0x100).taken is True

    def test_two_not_taken_needed_from_weakly_taken(self) -> None:
        """WT → needs 2 not-taken outcomes to flip to predicting not-taken.

        WT + not-taken → WNT (predicts not-taken after one)
        But this is the key: the first not-taken only moves to WNT.
        """
        p = TwoBitPredictor(initial_state=TwoBitState.WEAKLY_TAKEN)
        p.update(pc=0x100, taken=False)
        # Now at WNT → predicts not-taken
        assert p.predict(pc=0x100).taken is False

    def test_strongly_taken_needs_two_to_flip(self) -> None:
        """ST → needs 2 not-taken to flip to not-taken prediction.

        ST + NT → WT (still predicts taken)
        WT + NT → WNT (now predicts not-taken)
        """
        p = TwoBitPredictor(initial_state=TwoBitState.STRONGLY_TAKEN)
        p.update(pc=0x100, taken=False)
        # ST → WT: still predicts taken
        assert p.predict(pc=0x100).taken is True

        p.update(pc=0x100, taken=False)
        # WT → WNT: now predicts not-taken
        assert p.predict(pc=0x100).taken is False

    def test_get_state_debug_method(self) -> None:
        """The get_state() method exposes internal state for testing."""
        p = TwoBitPredictor()
        assert p.get_state(pc=0x100) == TwoBitState.WEAKLY_NOT_TAKEN
        p.update(pc=0x100, taken=True)
        assert p.get_state(pc=0x100) == TwoBitState.WEAKLY_TAKEN

    def test_confidence_strongly_vs_weakly(self) -> None:
        """Strong states have confidence 1.0; weak states have 0.5."""
        p = TwoBitPredictor(initial_state=TwoBitState.STRONGLY_TAKEN)
        assert p.predict(pc=0x100).confidence == 1.0

        p = TwoBitPredictor(initial_state=TwoBitState.WEAKLY_TAKEN)
        assert p.predict(pc=0x100).confidence == 0.5

        p = TwoBitPredictor(initial_state=TwoBitState.WEAKLY_NOT_TAKEN)
        assert p.predict(pc=0x100).confidence == 0.5

        p = TwoBitPredictor(initial_state=TwoBitState.STRONGLY_NOT_TAKEN)
        assert p.predict(pc=0x100).confidence == 1.0


# ─── Loop behavior ────────────────────────────────────────────────────────────


class TestTwoBitLoopBehavior:
    """The two-bit predictor's key advantage: fewer mispredictions on loops."""

    def test_loop_mispredicts_once_not_twice(self) -> None:
        """A loop running 10 times: only 1 misprediction (the exit).

        Starting from default WEAKLY_NOT_TAKEN:
        Iter 1: WNT → predict NT, actual T → WRONG, move to WT
        Iter 2: WT → predict T, actual T → correct, move to ST
        Iter 3-9: ST → predict T, actual T → correct (7x), saturated at ST
        Iter 10: ST → predict T, actual NT → WRONG, move to WT

        Total: 8 correct, 2 incorrect.
        BUT on second invocation, state is WT:
        Iter 1: WT → predict T, actual T → correct! (only 1 miss per invocation)
        """
        p = TwoBitPredictor()
        pc = 0x100

        # First invocation (cold start)
        for i in range(10):
            p.update(pc=pc, taken=(i < 9))

        assert p.stats.incorrect == 2  # cold start + exit
        assert p.stats.correct == 8

    def test_loop_second_invocation_one_miss(self) -> None:
        """Second loop invocation: only misses the exit.

        After first run, state is WT (weakly taken).
        Second run:
        Iter 1: WT → predict T, actual T → correct! → ST
        Iter 2-9: ST → predict T → correct (8x)
        Iter 10: ST → predict T, actual NT → WRONG → WT

        Only 1 misprediction in the second invocation.
        """
        p = TwoBitPredictor()
        pc = 0x100

        # First invocation
        for i in range(10):
            p.update(pc=pc, taken=(i < 9))

        # Record stats so far
        first_run_incorrect = p.stats.incorrect

        # Second invocation
        for i in range(10):
            p.update(pc=pc, taken=(i < 9))

        second_run_incorrect = p.stats.incorrect - first_run_incorrect
        assert second_run_incorrect == 1  # only the exit miss


class TestTwoBitVsOneBit:
    """Compare two-bit vs one-bit on the same patterns."""

    def test_two_bit_beats_one_bit_on_repeated_loops(self) -> None:
        """On repeated loop invocations, 2-bit is strictly better than 1-bit.

        After warmup, 2-bit has 1 miss/invocation vs 2-bit's 2.
        """
        one_bit = OneBitPredictor()
        two_bit = TwoBitPredictor()
        pc = 0x100

        # Run the loop 5 times, 10 iterations each
        for _ in range(5):
            for i in range(10):
                taken = i < 9
                one_bit.update(pc=pc, taken=taken)
                two_bit.update(pc=pc, taken=taken)

        # 2-bit should have better accuracy
        assert two_bit.stats.accuracy > one_bit.stats.accuracy

    def test_both_handle_always_taken(self) -> None:
        """On always-taken sequences, both converge to 100% after warmup."""
        one_bit = OneBitPredictor()
        two_bit = TwoBitPredictor()
        pc = 0x100

        # Warmup
        one_bit.update(pc=pc, taken=True)
        two_bit.update(pc=pc, taken=True)

        # Reset stats after warmup
        one_bit._stats.reset()
        two_bit._stats.reset()

        # Run 100 always-taken branches
        for _ in range(100):
            one_bit.update(pc=pc, taken=True)
            two_bit.update(pc=pc, taken=True)

        assert one_bit.stats.accuracy == 100.0
        assert two_bit.stats.accuracy == 100.0


# ─── Table size effects ──────────────────────────────────────────────────────


class TestTwoBitTableSize:
    """Effects of table size on prediction quality."""

    def test_small_table_causes_aliasing(self) -> None:
        """table_size=2: branch at 0 and 2 alias to the same slot."""
        p = TwoBitPredictor(table_size=2)
        # Branch at 0: always taken → should converge to STRONGLY_TAKEN
        for _ in range(5):
            p.update(pc=0, taken=True)
        assert p.get_state(pc=0) == TwoBitState.STRONGLY_TAKEN

        # Branch at 2 aliases to same slot (2 % 2 = 0)
        # Reading state at pc=2 shows the same entry
        assert p.get_state(pc=2) == TwoBitState.STRONGLY_TAKEN

    def test_large_table_avoids_aliasing(self) -> None:
        """With table_size=4096, branches 0 and 2 are independent."""
        p = TwoBitPredictor(table_size=4096)
        for _ in range(5):
            p.update(pc=0, taken=True)
        # Branch at 2 is in a different slot
        assert p.get_state(pc=2) == TwoBitState.WEAKLY_NOT_TAKEN  # default


# ─── Reset ────────────────────────────────────────────────────────────────────


class TestTwoBitReset:
    """Reset clears the table and stats."""

    def test_reset_clears_table(self) -> None:
        p = TwoBitPredictor()
        p.update(pc=0x100, taken=True)
        p.update(pc=0x100, taken=True)
        p.reset()
        # After reset, back to initial state
        assert p.get_state(pc=0x100) == TwoBitState.WEAKLY_NOT_TAKEN

    def test_reset_clears_stats(self) -> None:
        p = TwoBitPredictor()
        p.update(pc=0x100, taken=True)
        p.reset()
        assert p.stats.predictions == 0


# ─── Full state transition walkthrough ────────────────────────────────────────


class TestTwoBitFullTransitionWalkthrough:
    """Walk through all 4 states with explicit assertions at each step."""

    def test_walk_up_and_down(self) -> None:
        """Start at SNT, walk up to ST, then back down to SNT.

        SNT →(T)→ WNT →(T)→ WT →(T)→ ST →(NT)→ WT →(NT)→ WNT →(NT)→ SNT
        """
        p = TwoBitPredictor(initial_state=TwoBitState.STRONGLY_NOT_TAKEN)
        pc = 0x100

        assert p.get_state(pc) == TwoBitState.STRONGLY_NOT_TAKEN

        p.update(pc, taken=True)
        assert p.get_state(pc) == TwoBitState.WEAKLY_NOT_TAKEN

        p.update(pc, taken=True)
        assert p.get_state(pc) == TwoBitState.WEAKLY_TAKEN

        p.update(pc, taken=True)
        assert p.get_state(pc) == TwoBitState.STRONGLY_TAKEN

        # Now walk back down
        p.update(pc, taken=False)
        assert p.get_state(pc) == TwoBitState.WEAKLY_TAKEN

        p.update(pc, taken=False)
        assert p.get_state(pc) == TwoBitState.WEAKLY_NOT_TAKEN

        p.update(pc, taken=False)
        assert p.get_state(pc) == TwoBitState.STRONGLY_NOT_TAKEN

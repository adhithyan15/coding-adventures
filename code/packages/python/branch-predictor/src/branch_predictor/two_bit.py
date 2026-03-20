"""Two-bit saturating counter predictor — the classic, used in most textbooks.

The two-bit predictor improves on the one-bit predictor by adding hysteresis.
Instead of flipping the prediction on every misprediction, it takes TWO
consecutive mispredictions to change the predicted direction. This is achieved
with a 2-bit saturating counter — a counter that counts up to 3 and down to 0,
but never wraps around (it "saturates" at the boundaries).

The four states and their meanings:

    ┌────────────────────────────────────────────────────────────────────────┐
    │  STRONGLY      WEAKLY        WEAKLY        STRONGLY                  │
    │  NOT TAKEN     NOT TAKEN     TAKEN         TAKEN                     │
    │    (00)          (01)         (10)          (11)                     │
    │                                                                      │
    │  Predict:      Predict:      Predict:      Predict:                  │
    │  NOT TAKEN     NOT TAKEN     TAKEN         TAKEN                     │
    │                                                                      │
    │  Confidence:   Confidence:   Confidence:   Confidence:               │
    │  HIGH          LOW           LOW           HIGH                      │
    └────────────────────────────────────────────────────────────────────────┘

State transition diagram:

    taken                taken               taken               taken
    ──────→              ──────→              ──────→              ──────→
    (sat)   SNT ◄──────── WNT ◄──────── WT ◄──────── ST   (sat)
            ──────→              ──────→              ──────→
          not taken          not taken           not taken

    SNT = Strongly Not Taken (0)
    WNT = Weakly Not Taken (1)
    WT  = Weakly Taken (2)
    ST  = Strongly Taken (3)

The prediction threshold is at the midpoint:
    states 0, 1 → predict NOT TAKEN
    states 2, 3 → predict TAKEN

Why this solves the double-misprediction problem:
    Consider the same loop as in one_bit.py (10 iterations):

    First invocation:
    Iter 1: state=WNT(1) → predict NOT TAKEN → actual TAKEN → WRONG, state→WT(2)
    Iter 2: state=WT(2)  → predict TAKEN     → actual TAKEN → correct, state→ST(3)
    ...
    Iter 9: state=ST(3)  → predict TAKEN     → actual TAKEN → correct (saturated)
    Iter 10: state=ST(3) → predict TAKEN     → actual NOT TAKEN → WRONG, state→WT(2)

    Second invocation:
    Iter 1: state=WT(2)  → predict TAKEN     → actual TAKEN → correct! state→ST(3)

    Only 1 misprediction on re-entry (vs 2 for the one-bit predictor).
    The "weakly taken" state acts as a buffer — one not-taken doesn't flip it.

Historical usage:
    - Alpha 21064: 2-bit counters with 2048 entries
    - Intel Pentium: 2-bit counters with 256 entries, indexed by branch history
    - Early ARM (ARM7): 2-bit counters with 64 entries
    - MIPS R10000: 2-bit counters as base predictor in a tournament scheme
"""

from __future__ import annotations

from enum import IntEnum

from state_machine import DFA

from branch_predictor.base import Prediction
from branch_predictor.stats import PredictionStats

# ─── Two-Bit DFA ─────────────────────────────────────────────────────────────
#
# This IS the formal state machine that the 2-bit saturating counter branch
# predictor implements. The DFA captures the complete transition logic in a
# declarative form — every (state, input) pair maps to exactly one next state.
#
# The four states correspond to the four values of a 2-bit counter:
#   SNT (00) = Strongly Not Taken
#   WNT (01) = Weakly Not Taken
#   WT  (10) = Weakly Taken
#   ST  (11) = Strongly Taken
#
# The accepting states {WT, ST} are the states that predict "taken". In the
# DFA formalism, "accepting" means the machine is in a state that satisfies
# the property we're testing — here, "does this branch predict taken?"
#
# By defining the predictor as a DFA, we gain:
#   1. Formal verification: the transition table is the single source of truth
#   2. Visualization: call TWO_BIT_DFA.to_dot() to generate a Graphviz diagram
#   3. Tracing: every transition is logged for debugging
#   4. Composition: the DFA can be minimized, intersected, or complemented

TWO_BIT_DFA = DFA(
    states={"SNT", "WNT", "WT", "ST"},
    alphabet={"taken", "not_taken"},
    transitions={
        ("SNT", "taken"): "WNT", ("SNT", "not_taken"): "SNT",
        ("WNT", "taken"): "WT",  ("WNT", "not_taken"): "SNT",
        ("WT", "taken"): "ST",   ("WT", "not_taken"): "WNT",
        ("ST", "taken"): "ST",   ("ST", "not_taken"): "WT",
    },
    initial="WNT",
    accepting={"WT", "ST"},  # states that predict "taken"
)

# ─── Mappings between TwoBitState enum values and DFA state names ────────────
#
# The TwoBitState enum uses integer values (0-3) for efficient comparison,
# while the DFA uses string names for readability. These mappings bridge
# the two representations so that the enum's transition methods can delegate
# to the DFA's transition table.

_STATE_TO_NAME: dict[TwoBitState, str] = {}  # populated after class definition
_NAME_TO_STATE: dict[str, TwoBitState] = {}  # populated after class definition


# ─── TwoBitState ──────────────────────────────────────────────────────────────
#
# We use IntEnum so the states have integer values (0-3) that correspond to
# the 2-bit counter value. This makes the increment/decrement logic natural:
#   taken → min(state + 1, 3)
#   not taken → max(state - 1, 0)
#
# The "saturating" part means we clamp at the boundaries rather than wrapping.
# In hardware, this is implemented with a simple 2-bit adder and saturation
# logic — about 4 gates per entry.


class TwoBitState(IntEnum):
    """The 4 states of a 2-bit saturating counter.

    State transitions::

        STRONGLY_NOT_TAKEN <-> WEAKLY_NOT_TAKEN <-> WEAKLY_TAKEN <-> STRONGLY_TAKEN
              (00)                  (01)               (10)             (11)

    On 'taken' outcome: increment (move right), saturate at STRONGLY_TAKEN.
    On 'not taken' outcome: decrement (move left), saturate at STRONGLY_NOT_TAKEN.

    Predict taken if state >= WEAKLY_TAKEN (bit 1 is set).

    Why this works: a loop that runs 10 times mispredicts only ONCE (the exit).
    After the first taken, the counter moves to STRONGLY_TAKEN. It takes TWO
    not-taken outcomes to flip the prediction. The single not-taken at loop exit
    only moves it to WEAKLY_TAKEN, which still predicts taken next time.
    """

    STRONGLY_NOT_TAKEN = 0
    WEAKLY_NOT_TAKEN = 1
    WEAKLY_TAKEN = 2
    STRONGLY_TAKEN = 3

    def taken_outcome(self) -> TwoBitState:
        """Transition on a 'taken' branch outcome (increment, saturate at 3).

        Delegates to the TWO_BIT_DFA transition table so that the formal DFA
        definition is the single source of truth for state transitions.

        Example::

            state = TwoBitState.WEAKLY_NOT_TAKEN  # 1
            state = state.taken_outcome()           # → WEAKLY_TAKEN (2)
            state = state.taken_outcome()           # → STRONGLY_TAKEN (3)
            state = state.taken_outcome()           # → STRONGLY_TAKEN (3) — saturated!
        """
        name = _STATE_TO_NAME[self]
        target = TWO_BIT_DFA.transitions[(name, "taken")]
        return _NAME_TO_STATE[target]

    def not_taken_outcome(self) -> TwoBitState:
        """Transition on a 'not taken' branch outcome (decrement, saturate at 0).

        Delegates to the TWO_BIT_DFA transition table so that the formal DFA
        definition is the single source of truth for state transitions.

        Example::

            state = TwoBitState.WEAKLY_TAKEN      # 2
            state = state.not_taken_outcome()       # → WEAKLY_NOT_TAKEN (1)
            state = state.not_taken_outcome()       # → STRONGLY_NOT_TAKEN (0)
            state = state.not_taken_outcome()       # → SNT (0) — saturated!
        """
        name = _STATE_TO_NAME[self]
        target = TWO_BIT_DFA.transitions[(name, "not_taken")]
        return _NAME_TO_STATE[target]

    @property
    def predicts_taken(self) -> bool:
        """Whether this state predicts 'taken'.

        The threshold is at WEAKLY_TAKEN (2). States 2 and 3 predict taken;
        states 0 and 1 predict not-taken. In hardware, this is just bit 1
        of the 2-bit counter — a single wire, zero logic.

        Example::

            assert TwoBitState.STRONGLY_TAKEN.predicts_taken is True
            assert TwoBitState.WEAKLY_TAKEN.predicts_taken is True
            assert TwoBitState.WEAKLY_NOT_TAKEN.predicts_taken is False
            assert TwoBitState.STRONGLY_NOT_TAKEN.predicts_taken is False
        """
        return self >= TwoBitState.WEAKLY_TAKEN


# ─── Populate the enum ↔ DFA name mappings ────────────────────────────────────
#
# We do this after TwoBitState is defined so both the enum and the DFA exist.
# The mapping is simple: each enum member maps to its abbreviated DFA name.

_STATE_TO_NAME.update({
    TwoBitState.STRONGLY_NOT_TAKEN: "SNT",
    TwoBitState.WEAKLY_NOT_TAKEN: "WNT",
    TwoBitState.WEAKLY_TAKEN: "WT",
    TwoBitState.STRONGLY_TAKEN: "ST",
})
_NAME_TO_STATE.update({name: state for state, name in _STATE_TO_NAME.items()})


# ─── TwoBitPredictor ─────────────────────────────────────────────────────────
#
# The predictor maintains a table of 2-bit saturating counters, one per entry.
# Each branch maps to an entry via (pc % table_size). On predict(), we read
# the counter; on update(), we increment or decrement it.
#
# The initial state is configurable. Common choices:
#   - WEAKLY_NOT_TAKEN (01): conservative start, requires 1 taken to flip
#   - WEAKLY_TAKEN (10): optimistic start (like "always taken" initially)
#
# Most real processors use WEAKLY_NOT_TAKEN as the initial state, because
# it only takes one taken branch to move to WEAKLY_TAKEN and start predicting
# correctly. Starting at STRONGLY_NOT_TAKEN would require TWO taken branches.


class TwoBitPredictor:
    """2-bit saturating counter predictor — the classic, used in most textbooks.

    This was used in real processors: Alpha 21064, early MIPS, early ARM.
    Modern CPUs use more sophisticated predictors (TAGE, perceptron) but
    the 2-bit counter is the foundation that all advanced predictors build on.

    Args:
        table_size: Number of entries in the prediction table. Default: 1024.
        initial_state: Starting state for all counter entries.
            Default: WEAKLY_NOT_TAKEN — a good balance between responsiveness
            and stability.

    Example::

        predictor = TwoBitPredictor(table_size=256)

        # First encounter — starts at WEAKLY_NOT_TAKEN → predicts NOT TAKEN
        pred = predictor.predict(pc=0x100)
        assert pred.taken is False

        # After one 'taken' outcome → moves to WEAKLY_TAKEN → predicts TAKEN
        predictor.update(pc=0x100, taken=True)
        pred = predictor.predict(pc=0x100)
        assert pred.taken is True
    """

    def __init__(
        self,
        table_size: int = 1024,
        initial_state: TwoBitState = TwoBitState.WEAKLY_NOT_TAKEN,
    ) -> None:
        self._table_size = table_size
        self._initial_state = initial_state

        # ── Prediction table ──────────────────────────────────────────────
        # Maps (index) → TwoBitState. Entries start at initial_state.
        # We use a dict and fill on first access (lazy initialization).
        self._table: dict[int, TwoBitState] = {}

        self._stats = PredictionStats()

    def _index(self, pc: int) -> int:
        """Compute the table index for a given PC.

        Same as OneBitPredictor — uses the lower bits of the PC.

        Args:
            pc: The program counter of the branch instruction.

        Returns:
            An integer in [0, table_size) used to index the prediction table.
        """
        return pc % self._table_size

    def _get_state(self, index: int) -> TwoBitState:
        """Get the state for a table entry, initializing if needed.

        Args:
            index: The table index.

        Returns:
            The current TwoBitState for this entry.
        """
        return self._table.get(index, self._initial_state)

    def predict(self, pc: int) -> Prediction:
        """Predict based on the 2-bit counter for this branch.

        Reads the counter state and returns taken/not-taken based on the
        threshold (states 2-3 → taken, states 0-1 → not-taken).

        Confidence mapping:
            STRONGLY states → 1.0 (high confidence)
            WEAKLY states   → 0.5 (low confidence)

        Args:
            pc: The program counter of the branch instruction.

        Returns:
            Prediction with taken and confidence based on counter state.
        """
        index = self._index(pc)
        state = self._get_state(index)

        # Confidence: strong states are more confident than weak states.
        # This is useful for tournament predictors that pick the most
        # confident sub-predictor.
        if state in (TwoBitState.STRONGLY_TAKEN, TwoBitState.STRONGLY_NOT_TAKEN):
            confidence = 1.0
        else:
            confidence = 0.5

        return Prediction(taken=state.predicts_taken, confidence=confidence)

    def update(self, pc: int, taken: bool, target: int | None = None) -> None:  # noqa: ARG002
        """Update the 2-bit counter based on the actual outcome.

        Increments on taken, decrements on not-taken, saturating at boundaries.

        Args:
            pc: The program counter of the branch instruction.
            taken: Whether the branch was actually taken.
            target: The actual target address (unused by this predictor).
        """
        index = self._index(pc)
        state = self._get_state(index)

        # Record accuracy BEFORE updating
        self._stats.record(correct=(state.predicts_taken == taken))

        # Transition the state
        if taken:
            self._table[index] = state.taken_outcome()
        else:
            self._table[index] = state.not_taken_outcome()

    @property
    def stats(self) -> PredictionStats:
        """Get prediction accuracy statistics."""
        return self._stats

    def reset(self) -> None:
        """Reset the prediction table and statistics."""
        self._table.clear()
        self._stats.reset()

    def get_state(self, pc: int) -> TwoBitState:
        """Inspect the current state for a branch address (for testing/debugging).

        Args:
            pc: The program counter of the branch instruction.

        Returns:
            The current TwoBitState for this branch's table entry.
        """
        return self._get_state(self._index(pc))

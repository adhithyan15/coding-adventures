"""Tests for the DFA (Deterministic Finite Automaton) implementation.

These tests cover:
1. Construction and validation
2. Processing single events and sequences
3. Acceptance checking
4. Introspection (reachability, completeness, validation)
5. Visualization (DOT and ASCII output)
6. Classic examples (turnstile, binary div-by-3, branch predictor)
7. Error cases
"""

import pytest

from state_machine.dfa import DFA
from state_machine.types import TransitionRecord

# ============================================================
# Fixtures — reusable DFA definitions
# ============================================================


@pytest.fixture()
def turnstile() -> DFA:
    """The classic turnstile: insert coin to unlock, push to lock."""
    return DFA(
        states={"locked", "unlocked"},
        alphabet={"coin", "push"},
        transitions={
            ("locked", "coin"): "unlocked",
            ("locked", "push"): "locked",
            ("unlocked", "coin"): "unlocked",
            ("unlocked", "push"): "locked",
        },
        initial="locked",
        accepting={"unlocked"},
    )


@pytest.fixture()
def div_by_3() -> DFA:
    """DFA that accepts binary strings representing numbers divisible by 3.

    States represent the remainder when divided by 3:
      r0 = remainder 0 (divisible by 3) — accepting
      r1 = remainder 1
      r2 = remainder 2

    Transition logic: new_remainder = (old_remainder * 2 + bit) mod 3
    """
    return DFA(
        states={"r0", "r1", "r2"},
        alphabet={"0", "1"},
        transitions={
            ("r0", "0"): "r0",  # (0*2+0) mod 3 = 0
            ("r0", "1"): "r1",  # (0*2+1) mod 3 = 1
            ("r1", "0"): "r2",  # (1*2+0) mod 3 = 2
            ("r1", "1"): "r0",  # (1*2+1) mod 3 = 0
            ("r2", "0"): "r1",  # (2*2+0) mod 3 = 1
            ("r2", "1"): "r2",  # (2*2+1) mod 3 = 2
        },
        initial="r0",
        accepting={"r0"},
    )


@pytest.fixture()
def branch_predictor() -> DFA:
    """2-bit saturating counter branch predictor as a DFA.

    States: SNT (strongly not-taken), WNT (weakly not-taken),
            WT (weakly taken), ST (strongly taken)

    This is equivalent to the TwoBitState in the branch-predictor package.
    """
    return DFA(
        states={"SNT", "WNT", "WT", "ST"},
        alphabet={"taken", "not_taken"},
        transitions={
            ("SNT", "taken"): "WNT",
            ("SNT", "not_taken"): "SNT",
            ("WNT", "taken"): "WT",
            ("WNT", "not_taken"): "SNT",
            ("WT", "taken"): "ST",
            ("WT", "not_taken"): "WNT",
            ("ST", "taken"): "ST",
            ("ST", "not_taken"): "WT",
        },
        initial="WNT",
        accepting={"WT", "ST"},  # states that predict "taken"
    )


# ============================================================
# Construction and Validation Tests
# ============================================================


class TestDFAConstruction:
    """Tests for DFA construction and input validation."""

    def test_valid_construction(self, turnstile: DFA) -> None:
        """A valid DFA should be created without errors."""
        assert turnstile.current_state == "locked"
        assert turnstile.initial == "locked"
        assert turnstile.states == frozenset({"locked", "unlocked"})
        assert turnstile.alphabet == frozenset({"coin", "push"})
        assert turnstile.accepting == frozenset({"unlocked"})

    def test_empty_states_rejected(self) -> None:
        """An empty states set is not a valid DFA."""
        with pytest.raises(ValueError, match="non-empty"):
            DFA(
                states=set(),
                alphabet={"a"},
                transitions={},
                initial="q0",
                accepting=set(),
            )

    def test_initial_not_in_states(self) -> None:
        """The initial state must be in the states set."""
        with pytest.raises(ValueError, match="Initial state"):
            DFA(
                states={"q0", "q1"},
                alphabet={"a"},
                transitions={("q0", "a"): "q1"},
                initial="q_missing",
                accepting=set(),
            )

    def test_accepting_not_subset_of_states(self) -> None:
        """Accepting states must be a subset of the states set."""
        with pytest.raises(ValueError, match="Accepting states"):
            DFA(
                states={"q0", "q1"},
                alphabet={"a"},
                transitions={("q0", "a"): "q1"},
                initial="q0",
                accepting={"q_missing"},
            )

    def test_transition_source_not_in_states(self) -> None:
        """Transition sources must be in the states set."""
        with pytest.raises(ValueError, match="source"):
            DFA(
                states={"q0"},
                alphabet={"a"},
                transitions={("q_bad", "a"): "q0"},
                initial="q0",
                accepting=set(),
            )

    def test_transition_event_not_in_alphabet(self) -> None:
        """Transition events must be in the alphabet."""
        with pytest.raises(ValueError, match="alphabet"):
            DFA(
                states={"q0"},
                alphabet={"a"},
                transitions={("q0", "b"): "q0"},
                initial="q0",
                accepting=set(),
            )

    def test_transition_target_not_in_states(self) -> None:
        """Transition targets must be in the states set."""
        with pytest.raises(ValueError, match="target"):
            DFA(
                states={"q0"},
                alphabet={"a"},
                transitions={("q0", "a"): "q_bad"},
                initial="q0",
                accepting=set(),
            )

    def test_action_without_transition(self) -> None:
        """An action defined for a non-existent transition should be rejected."""
        with pytest.raises(ValueError, match="no transition"):
            DFA(
                states={"q0"},
                alphabet={"a"},
                transitions={("q0", "a"): "q0"},
                initial="q0",
                accepting=set(),
                actions={("q0", "b"): lambda s, e, t: None},
            )

    def test_empty_accepting_set(self) -> None:
        """A DFA with no accepting states is valid — it just never accepts."""
        dfa = DFA(
            states={"q0"},
            alphabet={"a"},
            transitions={("q0", "a"): "q0"},
            initial="q0",
            accepting=set(),
        )
        assert dfa.accepting == frozenset()

    def test_transitions_property_returns_copy(self, turnstile: DFA) -> None:
        """The transitions property should return a copy, not the internal dict."""
        t1 = turnstile.transitions
        t2 = turnstile.transitions
        assert t1 == t2
        assert t1 is not t2


# ============================================================
# Processing Tests
# ============================================================


class TestDFAProcessing:
    """Tests for processing single events and sequences."""

    def test_process_single_event(self, turnstile: DFA) -> None:
        """Processing one event should move to the correct state."""
        result = turnstile.process("coin")
        assert result == "unlocked"
        assert turnstile.current_state == "unlocked"

    def test_process_multiple_events(self, turnstile: DFA) -> None:
        """Processing events sequentially should follow the transitions."""
        turnstile.process("coin")
        assert turnstile.current_state == "unlocked"
        turnstile.process("push")
        assert turnstile.current_state == "locked"
        turnstile.process("coin")
        assert turnstile.current_state == "unlocked"
        turnstile.process("coin")
        assert turnstile.current_state == "unlocked"

    def test_process_builds_trace(self, turnstile: DFA) -> None:
        """Each process() call should add a TransitionRecord to the trace."""
        turnstile.process("coin")
        turnstile.process("push")

        trace = turnstile.trace
        assert len(trace) == 2
        assert trace[0] == TransitionRecord("locked", "coin", "unlocked")
        assert trace[1] == TransitionRecord("unlocked", "push", "locked")

    def test_process_sequence(self, turnstile: DFA) -> None:
        """process_sequence should return trace for the given inputs."""
        trace = turnstile.process_sequence(["coin", "push", "coin"])
        assert len(trace) == 3
        assert trace[0].source == "locked"
        assert trace[0].target == "unlocked"
        assert trace[1].source == "unlocked"
        assert trace[1].target == "locked"
        assert trace[2].source == "locked"
        assert trace[2].target == "unlocked"

    def test_process_sequence_empty(self, turnstile: DFA) -> None:
        """An empty sequence should produce an empty trace."""
        trace = turnstile.process_sequence([])
        assert trace == []
        assert turnstile.current_state == "locked"

    def test_process_invalid_event(self, turnstile: DFA) -> None:
        """Processing an event not in the alphabet should raise ValueError."""
        with pytest.raises(ValueError, match="not in the alphabet"):
            turnstile.process("kick")

    def test_process_undefined_transition(self) -> None:
        """Processing an event with no transition defined should raise."""
        dfa = DFA(
            states={"q0", "q1"},
            alphabet={"a", "b"},
            transitions={("q0", "a"): "q1"},  # no transition for (q0, b)
            initial="q0",
            accepting=set(),
        )
        with pytest.raises(ValueError, match="No transition"):
            dfa.process("b")

    def test_self_loop(self) -> None:
        """A state that transitions to itself should work correctly."""
        dfa = DFA(
            states={"q0"},
            alphabet={"a"},
            transitions={("q0", "a"): "q0"},
            initial="q0",
            accepting={"q0"},
        )
        dfa.process("a")
        assert dfa.current_state == "q0"
        dfa.process("a")
        assert dfa.current_state == "q0"

    def test_actions_fire(self) -> None:
        """Actions should be called with (source, event, target) arguments."""
        log: list[tuple[str, str, str]] = []

        def logger(source: str, event: str, target: str) -> None:
            log.append((source, event, target))

        dfa = DFA(
            states={"a", "b"},
            alphabet={"x"},
            transitions={("a", "x"): "b", ("b", "x"): "a"},
            initial="a",
            accepting=set(),
            actions={("a", "x"): logger},
        )
        dfa.process("x")
        assert log == [("a", "x", "b")]
        dfa.process("x")
        assert len(log) == 1  # action only on (a, x), not (b, x)

    def test_action_name_in_trace(self) -> None:
        """The action name should appear in the TransitionRecord."""

        def my_action(source: str, event: str, target: str) -> None:
            pass

        dfa = DFA(
            states={"a", "b"},
            alphabet={"x"},
            transitions={("a", "x"): "b", ("b", "x"): "a"},
            initial="a",
            accepting=set(),
            actions={("a", "x"): my_action},
        )
        dfa.process("x")
        assert dfa.trace[0].action_name == "my_action"


# ============================================================
# Acceptance Tests
# ============================================================


class TestDFAAcceptance:
    """Tests for the accepts() method."""

    def test_accepts_basic(self, turnstile: DFA) -> None:
        """Turnstile accepts sequences ending in unlocked state."""
        assert turnstile.accepts(["coin"]) is True
        assert turnstile.accepts(["coin", "push"]) is False
        assert turnstile.accepts(["coin", "push", "coin"]) is True

    def test_accepts_empty_input(self, turnstile: DFA) -> None:
        """Empty input: accept if initial state is accepting."""
        assert turnstile.accepts([]) is False  # locked is not accepting

        # DFA where initial IS accepting
        dfa = DFA(
            states={"q0"},
            alphabet={"a"},
            transitions={("q0", "a"): "q0"},
            initial="q0",
            accepting={"q0"},
        )
        assert dfa.accepts([]) is True

    def test_accepts_does_not_modify_state(self, turnstile: DFA) -> None:
        """accepts() should not change the machine's current state."""
        turnstile.process("coin")
        assert turnstile.current_state == "unlocked"

        turnstile.accepts(["push", "push", "push"])
        assert turnstile.current_state == "unlocked"  # unchanged

    def test_accepts_does_not_modify_trace(self, turnstile: DFA) -> None:
        """accepts() should not add to the trace."""
        turnstile.process("coin")
        trace_len = len(turnstile.trace)

        turnstile.accepts(["push", "coin"])
        assert len(turnstile.trace) == trace_len  # unchanged

    def test_accepts_undefined_transition(self) -> None:
        """accepts() returns False if a transition is undefined (no crash)."""
        dfa = DFA(
            states={"q0", "q1"},
            alphabet={"a", "b"},
            transitions={("q0", "a"): "q1"},
            initial="q0",
            accepting={"q1"},
        )
        assert dfa.accepts(["a"]) is True
        assert dfa.accepts(["b"]) is False  # no transition, graceful reject

    def test_accepts_invalid_event(self, turnstile: DFA) -> None:
        """accepts() should raise on events not in alphabet."""
        with pytest.raises(ValueError, match="not in the alphabet"):
            turnstile.accepts(["kick"])

    def test_div_by_3(self, div_by_3: DFA) -> None:
        """Test binary divisibility by 3 with various inputs."""
        # 0 = 0 (div by 3) — empty string starts in r0 which is accepting
        assert div_by_3.accepts([]) is True

        # 1 = 1 (not div by 3)
        assert div_by_3.accepts(["1"]) is False

        # 10 = 2 (not div by 3)
        assert div_by_3.accepts(["1", "0"]) is False

        # 11 = 3 (div by 3)
        assert div_by_3.accepts(["1", "1"]) is True

        # 100 = 4 (not div by 3)
        assert div_by_3.accepts(["1", "0", "0"]) is False

        # 110 = 6 (div by 3)
        assert div_by_3.accepts(["1", "1", "0"]) is True

        # 1001 = 9 (div by 3)
        assert div_by_3.accepts(["1", "0", "0", "1"]) is True

        # 1100 = 12 (div by 3)
        assert div_by_3.accepts(["1", "1", "0", "0"]) is True

        # 1111 = 15 (div by 3)
        assert div_by_3.accepts(["1", "1", "1", "1"]) is True

        # 10000 = 16 (not div by 3)
        assert div_by_3.accepts(["1", "0", "0", "0", "0"]) is False


# ============================================================
# Branch Predictor as DFA Tests
# ============================================================


class TestBranchPredictorDFA:
    """Test the 2-bit saturating counter expressed as a DFA.

    This demonstrates that the branch-predictor package's TwoBitState
    can be modeled as a formal DFA.
    """

    def test_initial_state(self, branch_predictor: DFA) -> None:
        """The predictor starts in WNT (weakly not-taken)."""
        assert branch_predictor.current_state == "WNT"

    def test_warmup_to_strongly_taken(self, branch_predictor: DFA) -> None:
        """Two consecutive 'taken' outcomes should reach ST."""
        branch_predictor.process("taken")
        assert branch_predictor.current_state == "WT"
        branch_predictor.process("taken")
        assert branch_predictor.current_state == "ST"

    def test_saturation_at_st(self, branch_predictor: DFA) -> None:
        """ST is saturating — more 'taken' stays at ST."""
        branch_predictor.process_sequence(["taken", "taken", "taken", "taken"])
        assert branch_predictor.current_state == "ST"

    def test_saturation_at_snt(self, branch_predictor: DFA) -> None:
        """SNT is saturating — more 'not_taken' stays at SNT."""
        branch_predictor.process_sequence(
            ["not_taken", "not_taken", "not_taken"]
        )
        assert branch_predictor.current_state == "SNT"

    def test_hysteresis(self, branch_predictor: DFA) -> None:
        """One misprediction should not flip the prediction.

        ST → WT on 'not_taken', but WT still predicts 'taken' (accepting).
        """
        branch_predictor.process_sequence(["taken", "taken"])
        assert branch_predictor.current_state == "ST"

        branch_predictor.process("not_taken")
        assert branch_predictor.current_state == "WT"
        assert "WT" in branch_predictor.accepting  # still predicts taken

    def test_loop_pattern(self, branch_predictor: DFA) -> None:
        """Simulate a loop: 9 taken + 1 not-taken, repeated twice.

        With 2-bit predictor, should mispredict only the 'not_taken' exits.
        """
        pattern = ["taken"] * 9 + ["not_taken"]
        branch_predictor.process_sequence(pattern)
        # After 9 taken: ST. After not_taken: WT. Still predicts taken.
        assert branch_predictor.current_state == "WT"
        assert branch_predictor.current_state in branch_predictor.accepting

    def test_prediction_via_accepting(self, branch_predictor: DFA) -> None:
        """accepting states = predict taken, non-accepting = predict not-taken."""
        # WNT is not accepting (predicts not-taken)
        assert branch_predictor.current_state not in branch_predictor.accepting

        # After one 'taken': WT is accepting (predicts taken)
        branch_predictor.process("taken")
        assert branch_predictor.current_state in branch_predictor.accepting


# ============================================================
# Reset Tests
# ============================================================


class TestDFAReset:
    """Tests for the reset() method."""

    def test_reset_returns_to_initial(self, turnstile: DFA) -> None:
        """After reset, current state should be the initial state."""
        turnstile.process("coin")
        assert turnstile.current_state == "unlocked"

        turnstile.reset()
        assert turnstile.current_state == "locked"

    def test_reset_clears_trace(self, turnstile: DFA) -> None:
        """After reset, the trace should be empty."""
        turnstile.process_sequence(["coin", "push", "coin"])
        assert len(turnstile.trace) == 3

        turnstile.reset()
        assert turnstile.trace == []


# ============================================================
# Introspection Tests
# ============================================================


class TestDFAIntrospection:
    """Tests for reachable_states, is_complete, and validate."""

    def test_reachable_states_all(self, turnstile: DFA) -> None:
        """All states in the turnstile are reachable."""
        assert turnstile.reachable_states() == frozenset(
            {"locked", "unlocked"}
        )

    def test_reachable_states_with_unreachable(self) -> None:
        """Unreachable states should not be in the result."""
        dfa = DFA(
            states={"q0", "q1", "q_dead"},
            alphabet={"a"},
            transitions={("q0", "a"): "q1", ("q1", "a"): "q0"},
            initial="q0",
            accepting=set(),
        )
        assert dfa.reachable_states() == frozenset({"q0", "q1"})

    def test_is_complete_true(self, turnstile: DFA) -> None:
        """The turnstile has transitions for every (state, event) pair."""
        assert turnstile.is_complete() is True

    def test_is_complete_false(self) -> None:
        """An incomplete DFA is missing some transitions."""
        dfa = DFA(
            states={"q0", "q1"},
            alphabet={"a", "b"},
            transitions={("q0", "a"): "q1"},  # missing others
            initial="q0",
            accepting=set(),
        )
        assert dfa.is_complete() is False

    def test_validate_clean(self, turnstile: DFA) -> None:
        """A well-formed DFA should have no warnings."""
        assert turnstile.validate() == []

    def test_validate_unreachable(self) -> None:
        """validate() should report unreachable states."""
        dfa = DFA(
            states={"q0", "q1", "q_dead"},
            alphabet={"a"},
            transitions={
                ("q0", "a"): "q1",
                ("q1", "a"): "q0",
                ("q_dead", "a"): "q_dead",
            },
            initial="q0",
            accepting=set(),
        )
        warnings = dfa.validate()
        assert any("Unreachable" in w for w in warnings)
        assert any("q_dead" in w for w in warnings)

    def test_validate_unreachable_accepting(self) -> None:
        """validate() should specifically flag unreachable accepting states."""
        dfa = DFA(
            states={"q0", "q_dead"},
            alphabet={"a"},
            transitions={
                ("q0", "a"): "q0",
                ("q_dead", "a"): "q_dead",
            },
            initial="q0",
            accepting={"q_dead"},
        )
        warnings = dfa.validate()
        assert any("Unreachable accepting" in w for w in warnings)

    def test_validate_missing_transitions(self) -> None:
        """validate() should report missing transitions."""
        dfa = DFA(
            states={"q0", "q1"},
            alphabet={"a", "b"},
            transitions={("q0", "a"): "q1"},
            initial="q0",
            accepting=set(),
        )
        warnings = dfa.validate()
        assert any("Missing transitions" in w for w in warnings)


# ============================================================
# Visualization Tests
# ============================================================


class TestDFAVisualization:
    """Tests for DOT and ASCII output."""

    def test_to_dot_structure(self, turnstile: DFA) -> None:
        """DOT output should have the expected structure."""
        dot = turnstile.to_dot()
        assert "digraph DFA" in dot
        assert "__start" in dot
        assert "doublecircle" in dot  # accepting state
        assert "locked" in dot
        assert "unlocked" in dot
        assert "coin" in dot
        assert "push" in dot
        assert dot.endswith("}")

    def test_to_dot_initial_arrow(self, turnstile: DFA) -> None:
        """The initial state should have an arrow from a point node."""
        dot = turnstile.to_dot()
        assert '__start -> "locked"' in dot

    def test_to_dot_accepting_doublecircle(self, turnstile: DFA) -> None:
        """Accepting states should have doublecircle shape."""
        dot = turnstile.to_dot()
        assert '"unlocked" [shape=doublecircle]' in dot
        assert '"locked" [shape=circle]' in dot

    def test_to_ascii_contains_all_states(self, turnstile: DFA) -> None:
        """ASCII table should contain all states and events."""
        ascii_table = turnstile.to_ascii()
        assert "locked" in ascii_table
        assert "unlocked" in ascii_table
        assert "coin" in ascii_table
        assert "push" in ascii_table

    def test_to_ascii_marks_initial(self, turnstile: DFA) -> None:
        """The initial state should be marked with '>'."""
        ascii_table = turnstile.to_ascii()
        assert ">" in ascii_table

    def test_to_ascii_marks_accepting(self, turnstile: DFA) -> None:
        """Accepting states should be marked with '*'."""
        ascii_table = turnstile.to_ascii()
        assert "*" in ascii_table

    def test_to_table_header(self, turnstile: DFA) -> None:
        """The table's first row should be the header."""
        table = turnstile.to_table()
        assert table[0][0] == "State"
        assert "coin" in table[0]
        assert "push" in table[0]

    def test_to_table_data(self, turnstile: DFA) -> None:
        """The table should contain correct transition data."""
        table = turnstile.to_table()
        # Find the row for "locked"
        locked_row = [row for row in table if row[0] == "locked"][0]
        events = table[0][1:]
        coin_idx = events.index("coin") + 1
        push_idx = events.index("push") + 1
        assert locked_row[coin_idx] == "unlocked"
        assert locked_row[push_idx] == "locked"

    def test_to_table_missing_transitions(self) -> None:
        """Missing transitions should show '—' in the table."""
        dfa = DFA(
            states={"q0", "q1"},
            alphabet={"a", "b"},
            transitions={("q0", "a"): "q1"},
            initial="q0",
            accepting=set(),
        )
        table = dfa.to_table()
        q0_row = [row for row in table if row[0] == "q0"][0]
        assert "—" in q0_row


# ============================================================
# Repr Tests
# ============================================================


class TestDFARepr:
    """Tests for __repr__."""

    def test_repr_contains_key_info(self, turnstile: DFA) -> None:
        """repr should show states, alphabet, initial, accepting, current."""
        r = repr(turnstile)
        assert "DFA" in r
        assert "locked" in r
        assert "unlocked" in r
        assert "coin" in r
        assert "push" in r


# ============================================================
# Edge Cases
# ============================================================


class TestDFAEdgeCases:
    """Tests for edge cases and unusual configurations."""

    def test_single_state_self_loop(self) -> None:
        """A DFA with one state and a self-loop."""
        dfa = DFA(
            states={"q0"},
            alphabet={"a"},
            transitions={("q0", "a"): "q0"},
            initial="q0",
            accepting={"q0"},
        )
        assert dfa.accepts(["a", "a", "a"]) is True
        assert dfa.accepts([]) is True

    def test_large_alphabet(self) -> None:
        """A DFA with a large alphabet should work fine."""
        alphabet = {chr(i) for i in range(ord("a"), ord("z") + 1)}
        dfa = DFA(
            states={"q0", "q1"},
            alphabet=alphabet,
            transitions={
                **{("q0", c): "q1" for c in alphabet},
                **{("q1", c): "q0" for c in alphabet},
            },
            initial="q0",
            accepting={"q1"},
        )
        assert dfa.accepts(["a"]) is True
        assert dfa.accepts(["a", "b"]) is False
        assert dfa.accepts(["x", "y", "z"]) is True

    def test_trace_property_returns_copy(self, turnstile: DFA) -> None:
        """The trace property should return a copy."""
        turnstile.process("coin")
        t1 = turnstile.trace
        t2 = turnstile.trace
        assert t1 == t2
        assert t1 is not t2

    def test_div_by_3_comprehensive(self, div_by_3: DFA) -> None:
        """Test divisibility by 3 for all numbers 0-31."""
        for n in range(32):
            binary = bin(n)[2:]  # e.g., "11010"
            bits = list(binary)
            expected = n % 3 == 0
            # Special case: n=0 has no bits, empty input accepted (r0 is accepting)
            if n == 0:
                assert div_by_3.accepts([]) is True
            else:
                assert div_by_3.accepts(bits) is expected, (
                    f"Failed for n={n} (binary={binary}): "
                    f"expected {'accept' if expected else 'reject'}"
                )

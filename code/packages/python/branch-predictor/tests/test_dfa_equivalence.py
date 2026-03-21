"""Tests verifying that the DFA definitions match predictor behavior.

These tests ensure that the formal DFA state machines (TWO_BIT_DFA and
ONE_BIT_DFA) are equivalent to the predictor implementations they describe.
This is the "double-entry bookkeeping" of software: two independent
representations that must agree.
"""

from __future__ import annotations

from branch_predictor.one_bit import ONE_BIT_DFA, OneBitPredictor
from branch_predictor.two_bit import (
    _NAME_TO_STATE,
    _STATE_TO_NAME,
    TWO_BIT_DFA,
    TwoBitPredictor,
    TwoBitState,
)

# ─── TWO_BIT_DFA equivalence tests ──────────────────────────────────────────


class TestTwoBitDfaEquivalence:
    """Verify TWO_BIT_DFA transitions match TwoBitState methods."""

    def test_taken_transitions_match_for_all_states(self) -> None:
        """Every state's taken_outcome() must agree with the DFA."""
        for state in TwoBitState:
            name = _STATE_TO_NAME[state]
            dfa_target = TWO_BIT_DFA.transitions[(name, "taken")]
            method_target = state.taken_outcome()
            assert _STATE_TO_NAME[method_target] == dfa_target, (
                f"Mismatch for {state.name}.taken_outcome(): "
                f"method={method_target.name}, DFA={dfa_target}"
            )

    def test_not_taken_transitions_match_for_all_states(self) -> None:
        """Every state's not_taken_outcome() must agree with the DFA."""
        for state in TwoBitState:
            name = _STATE_TO_NAME[state]
            dfa_target = TWO_BIT_DFA.transitions[(name, "not_taken")]
            method_target = state.not_taken_outcome()
            assert _STATE_TO_NAME[method_target] == dfa_target, (
                f"Mismatch for {state.name}.not_taken_outcome(): "
                f"method={method_target.name}, DFA={dfa_target}"
            )

    def test_accepting_states_match_predicts_taken(self) -> None:
        """DFA accepting states must be exactly the states that predict taken."""
        for state in TwoBitState:
            name = _STATE_TO_NAME[state]
            assert state.predicts_taken == (name in TWO_BIT_DFA.accepting), (
                f"{state.name}: predicts_taken={state.predicts_taken}, "
                f"in accepting={name in TWO_BIT_DFA.accepting}"
            )

    def test_dfa_accepts_matches_predicts_taken_after_sequence(self) -> None:
        """DFA.accepts() on a sequence must match the predictor's state.

        After processing N "taken" events, the DFA should accept (predict
        taken) if and only if the TwoBitState would predict taken.
        """
        # Starting from WNT (initial), processing increasing numbers of "taken"
        # n=0: WNT → predicts not-taken → DFA should not accept
        # n=1: WNT → WT → predicts taken → DFA should accept
        # n=2: WNT → WT → ST → predicts taken → DFA should accept
        for n in range(5):
            sequence = ["taken"] * n
            dfa_accepts = TWO_BIT_DFA.accepts(sequence)
            # Manually walk the state
            state = TwoBitState.WEAKLY_NOT_TAKEN
            for _ in range(n):
                state = state.taken_outcome()
            assert dfa_accepts == state.predicts_taken, (
                f"After {n} 'taken': DFA accepts={dfa_accepts}, "
                f"state={state.name}, predicts_taken={state.predicts_taken}"
            )

    def test_dfa_accepts_mixed_sequence(self) -> None:
        """DFA acceptance matches predictor state on a mixed sequence."""
        events = ["taken", "taken", "not_taken", "taken", "not_taken", "not_taken"]
        dfa_accepts = TWO_BIT_DFA.accepts(events)
        # Walk manually
        state = TwoBitState.WEAKLY_NOT_TAKEN
        for event in events:
            if event == "taken":
                state = state.taken_outcome()
            else:
                state = state.not_taken_outcome()
        assert dfa_accepts == state.predicts_taken

    def test_state_name_mappings_are_bijective(self) -> None:
        """_STATE_TO_NAME and _NAME_TO_STATE must be exact inverses."""
        assert len(_STATE_TO_NAME) == len(TwoBitState)
        assert len(_NAME_TO_STATE) == len(TwoBitState)
        for state, name in _STATE_TO_NAME.items():
            assert _NAME_TO_STATE[name] is state

    def test_dfa_has_complete_transitions(self) -> None:
        """Every (state, event) pair must have a transition defined."""
        for state_name in TWO_BIT_DFA.states:
            for event in TWO_BIT_DFA.alphabet:
                assert (state_name, event) in TWO_BIT_DFA.transitions

    def test_dfa_initial_state_is_wnt(self) -> None:
        """The DFA initial state must be WNT (Weakly Not Taken)."""
        assert TWO_BIT_DFA.initial == "WNT"

    def test_predictor_uses_dfa_transitions(self) -> None:
        """TwoBitPredictor's update must produce same results via DFA."""
        predictor = TwoBitPredictor(table_size=4)
        pc = 0x100
        outcomes = [True, True, False, True, False, False, True]
        for taken in outcomes:
            predictor.update(pc=pc, taken=taken)

        # The predictor's final state should match walking the DFA
        final_state = predictor.get_state(pc)
        # Walk DFA manually from initial WNT
        dfa_state = "WNT"
        for taken in outcomes:
            event = "taken" if taken else "not_taken"
            dfa_state = TWO_BIT_DFA.transitions[(dfa_state, event)]
        assert _STATE_TO_NAME[final_state] == dfa_state


# ─── ONE_BIT_DFA equivalence tests ──────────────────────────────────────────


class TestOneBitDfaEquivalence:
    """Verify ONE_BIT_DFA transitions match OneBitPredictor behavior."""

    def test_transitions_match_predictor_update(self) -> None:
        """ONE_BIT_DFA transitions must agree with OneBitPredictor.update()."""
        predictor = OneBitPredictor(table_size=4)
        pc = 0x200
        outcomes = [True, False, True, True, False]
        for taken in outcomes:
            predictor.update(pc=pc, taken=taken)

        # Walk DFA from initial "not_taken"
        dfa_state = "not_taken"
        for taken in outcomes:
            event = "taken" if taken else "not_taken"
            dfa_state = ONE_BIT_DFA.transitions[(dfa_state, event)]

        # Compare: predictor stores True/False, DFA uses "taken"/"not_taken"
        predicted = predictor.predict(pc).taken
        dfa_predicts_taken = dfa_state == "taken"
        assert predicted == dfa_predicts_taken

    def test_dfa_accepts_single_taken(self) -> None:
        """After one 'taken' event, the DFA should accept."""
        assert ONE_BIT_DFA.accepts(["taken"]) is True

    def test_dfa_rejects_single_not_taken(self) -> None:
        """After one 'not_taken' event, the DFA should not accept."""
        assert ONE_BIT_DFA.accepts(["not_taken"]) is False

    def test_dfa_accepts_matches_predictor_on_sequence(self) -> None:
        """DFA acceptance after a sequence matches predictor's next prediction."""
        sequences = [
            ["taken"],
            ["not_taken"],
            ["taken", "not_taken"],
            ["taken", "taken", "not_taken"],
            ["not_taken", "taken", "taken"],
        ]
        for seq in sequences:
            # DFA acceptance
            dfa_accepts = ONE_BIT_DFA.accepts(seq)

            # Predictor: process each event, then check what it predicts
            predictor = OneBitPredictor(table_size=4)
            pc = 0x100
            for event_str in seq:
                predictor.update(pc=pc, taken=(event_str == "taken"))
            predictor_predicts_taken = predictor.predict(pc).taken

            assert dfa_accepts == predictor_predicts_taken, (
                f"Sequence {seq}: DFA accepts={dfa_accepts}, "
                f"predictor={predictor_predicts_taken}"
            )

    def test_dfa_has_complete_transitions(self) -> None:
        """Every (state, event) pair must have a transition defined."""
        for state_name in ONE_BIT_DFA.states:
            for event in ONE_BIT_DFA.alphabet:
                assert (state_name, event) in ONE_BIT_DFA.transitions

    def test_dfa_initial_state_is_not_taken(self) -> None:
        """The DFA initial state must be 'not_taken'."""
        assert ONE_BIT_DFA.initial == "not_taken"

    def test_dfa_accepting_is_taken(self) -> None:
        """The only accepting state should be 'taken'."""
        assert ONE_BIT_DFA.accepting == frozenset({"taken"})


# ─── DFA visualization tests ────────────────────────────────────────────────


class TestDfaVisualization:
    """Test that the DFA to_dot() produces valid Graphviz output."""

    def test_two_bit_dfa_to_dot_contains_all_states(self) -> None:
        """The Graphviz output must reference all 4 states."""
        dot = TWO_BIT_DFA.to_dot()
        for state_name in ("SNT", "WNT", "WT", "ST"):
            assert state_name in dot, f"State {state_name} missing from dot output"

    def test_two_bit_dfa_to_dot_contains_transitions(self) -> None:
        """The Graphviz output must contain transition arrows."""
        dot = TWO_BIT_DFA.to_dot()
        # Should contain '->' for edges in the digraph
        assert "->" in dot

    def test_one_bit_dfa_to_dot_contains_all_states(self) -> None:
        """The Graphviz output must reference both states."""
        dot = ONE_BIT_DFA.to_dot()
        assert "not_taken" in dot
        assert "taken" in dot

    def test_one_bit_dfa_to_dot_is_valid_digraph(self) -> None:
        """The output should be a valid digraph structure."""
        dot = ONE_BIT_DFA.to_dot()
        assert dot.strip().startswith("digraph")
        assert "{" in dot
        assert "}" in dot

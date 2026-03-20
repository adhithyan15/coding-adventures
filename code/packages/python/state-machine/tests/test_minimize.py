"""Tests for DFA minimization (Hopcroft's algorithm)."""

from state_machine.dfa import DFA
from state_machine.minimize import minimize
from state_machine.nfa import NFA


class TestMinimizeBasic:
    """Basic minimization tests."""

    def test_already_minimal(self) -> None:
        """A minimal DFA should not lose any states."""
        dfa = DFA(
            states={"q0", "q1"},
            alphabet={"a", "b"},
            transitions={
                ("q0", "a"): "q1",
                ("q0", "b"): "q0",
                ("q1", "a"): "q0",
                ("q1", "b"): "q1",
            },
            initial="q0",
            accepting={"q1"},
        )
        minimized = minimize(dfa)
        assert len(minimized.states) == 2

    def test_equivalent_states_merged(self) -> None:
        """States that behave identically should be merged.

        q1 and q2 are both accepting and both have the same transitions
        (self-loop on both 'a' and 'b'). They are equivalent.
        """
        dfa = DFA(
            states={"q0", "q1", "q2"},
            alphabet={"a", "b"},
            transitions={
                ("q0", "a"): "q1",
                ("q0", "b"): "q2",
                ("q1", "a"): "q1",
                ("q1", "b"): "q1",
                ("q2", "a"): "q2",
                ("q2", "b"): "q2",
            },
            initial="q0",
            accepting={"q1", "q2"},
        )
        minimized = minimize(dfa)
        # q1 and q2 should be merged into one state
        assert len(minimized.states) == 2

    def test_unreachable_states_removed(self) -> None:
        """Unreachable states should be removed during minimization."""
        dfa = DFA(
            states={"q0", "q1", "q_dead"},
            alphabet={"a"},
            transitions={
                ("q0", "a"): "q1",
                ("q1", "a"): "q0",
                ("q_dead", "a"): "q_dead",
            },
            initial="q0",
            accepting={"q1"},
        )
        minimized = minimize(dfa)
        assert len(minimized.states) == 2

    def test_language_preserved(self) -> None:
        """The minimized DFA must accept the same language."""
        dfa = DFA(
            states={"q0", "q1", "q2", "q3"},
            alphabet={"a", "b"},
            transitions={
                ("q0", "a"): "q1",
                ("q0", "b"): "q2",
                ("q1", "a"): "q3",
                ("q1", "b"): "q3",
                ("q2", "a"): "q3",
                ("q2", "b"): "q3",
                ("q3", "a"): "q3",
                ("q3", "b"): "q3",
            },
            initial="q0",
            accepting={"q1", "q2"},
        )
        minimized = minimize(dfa)

        test_inputs = [
            ["a"],
            ["b"],
            ["a", "a"],
            ["a", "b"],
            ["b", "a"],
            [],
        ]
        for events in test_inputs:
            assert dfa.accepts(events) == minimized.accepts(events), (
                f"Language mismatch on {events}"
            )

    def test_single_state(self) -> None:
        """A single-state DFA should remain single-state."""
        dfa = DFA(
            states={"q0"},
            alphabet={"a"},
            transitions={("q0", "a"): "q0"},
            initial="q0",
            accepting={"q0"},
        )
        minimized = minimize(dfa)
        assert len(minimized.states) == 1
        assert minimized.accepts(["a"]) is True
        assert minimized.accepts([]) is True


class TestMinimizeWithNFA:
    """Test minimization on DFAs produced by NFA→DFA conversion.

    Subset construction often produces bloated DFAs. Minimization should
    shrink them back down.
    """

    def test_nfa_to_dfa_to_minimized(self) -> None:
        """Full pipeline: NFA → DFA → minimized DFA."""
        # NFA for "ends with 'a'"
        nfa = NFA(
            states={"q0", "q1"},
            alphabet={"a", "b"},
            transitions={
                ("q0", "a"): {"q0", "q1"},
                ("q0", "b"): {"q0"},
            },
            initial="q0",
            accepting={"q1"},
        )
        dfa = nfa.to_dfa()
        minimized = minimize(dfa)

        # The minimal DFA for "ends with 'a'" has exactly 2 states
        assert len(minimized.states) == 2

        # Verify language
        assert minimized.accepts(["a"]) is True
        assert minimized.accepts(["b", "a"]) is True
        assert minimized.accepts(["a", "b", "a"]) is True
        assert minimized.accepts(["b"]) is False
        assert minimized.accepts(["a", "b"]) is False
        assert minimized.accepts([]) is False

    def test_minimized_preserves_language_exhaustive(self) -> None:
        """Exhaustively verify language equivalence for strings up to length 3."""
        # NFA for "contains 'aa'"
        nfa = NFA(
            states={"q0", "q1", "q2"},
            alphabet={"a", "b"},
            transitions={
                ("q0", "a"): {"q0", "q1"},
                ("q0", "b"): {"q0"},
                ("q1", "a"): {"q2"},
                ("q2", "a"): {"q2"},
                ("q2", "b"): {"q2"},
            },
            initial="q0",
            accepting={"q2"},
        )
        dfa = nfa.to_dfa()
        minimized = minimize(dfa)

        # Generate all strings up to length 3
        def gen(max_len: int) -> list[list[str]]:
            result: list[list[str]] = [[]]
            for _ in range(max_len):
                new: list[list[str]] = []
                for s in result:
                    for c in ["a", "b"]:
                        new.append([*s, c])
                result.extend(new)
            return result

        for s in gen(3):
            assert nfa.accepts(s) == minimized.accepts(s), (
                f"Mismatch on {''.join(s)!r}"
            )

"""Tests for the NFA (Non-deterministic Finite Automaton) implementation.

These tests cover:
1. Construction and validation
2. Epsilon closure computation
3. Processing events (non-deterministic branching)
4. Acceptance checking
5. Subset construction (NFA → DFA conversion)
6. Visualization
7. Classic examples
"""

import pytest

from state_machine.nfa import EPSILON, NFA

# ============================================================
# Fixtures
# ============================================================


@pytest.fixture()
def contains_ab() -> NFA:
    """NFA that accepts strings containing 'ab' as a substring.

    The NFA non-deterministically guesses where the substring starts:
    - In q0, on 'a', go to BOTH q0 (keep scanning) and q1 (start match)
    - In q1, on 'b', go to q2 (match complete)
    - In q2, on anything, stay in q2 (already matched)
    """
    return NFA(
        states={"q0", "q1", "q2"},
        alphabet={"a", "b"},
        transitions={
            ("q0", "a"): {"q0", "q1"},
            ("q0", "b"): {"q0"},
            ("q1", "b"): {"q2"},
            ("q2", "a"): {"q2"},
            ("q2", "b"): {"q2"},
        },
        initial="q0",
        accepting={"q2"},
    )


@pytest.fixture()
def epsilon_chain() -> NFA:
    """NFA with a chain of epsilon transitions: q0 --ε--> q1 --ε--> q2.

    Accepts any single 'a' (q2 has the only real transition).
    """
    return NFA(
        states={"q0", "q1", "q2", "q3"},
        alphabet={"a"},
        transitions={
            ("q0", EPSILON): {"q1"},
            ("q1", EPSILON): {"q2"},
            ("q2", "a"): {"q3"},
        },
        initial="q0",
        accepting={"q3"},
    )


@pytest.fixture()
def a_or_ab() -> NFA:
    """NFA that accepts "a" or "ab" using epsilon transitions.

    q0 --ε--> q1 (path for "a")
    q0 --ε--> q3 (path for "ab")
    q1 --a--> q2 (accept "a")
    q3 --a--> q4 --b--> q5 (accept "ab")
    """
    return NFA(
        states={"q0", "q1", "q2", "q3", "q4", "q5"},
        alphabet={"a", "b"},
        transitions={
            ("q0", EPSILON): {"q1", "q3"},
            ("q1", "a"): {"q2"},
            ("q3", "a"): {"q4"},
            ("q4", "b"): {"q5"},
        },
        initial="q0",
        accepting={"q2", "q5"},
    )


# ============================================================
# Construction Tests
# ============================================================


class TestNFAConstruction:
    """Tests for NFA construction and validation."""

    def test_valid_construction(self, contains_ab: NFA) -> None:
        """A valid NFA should be created without errors."""
        assert contains_ab.states == frozenset({"q0", "q1", "q2"})
        assert contains_ab.alphabet == frozenset({"a", "b"})
        assert contains_ab.initial == "q0"
        assert contains_ab.accepting == frozenset({"q2"})

    def test_empty_states_rejected(self) -> None:
        """Empty states set is invalid."""
        with pytest.raises(ValueError, match="non-empty"):
            NFA(states=set(), alphabet={"a"}, transitions={},
                initial="q0", accepting=set())

    def test_epsilon_in_alphabet_rejected(self) -> None:
        """The alphabet must not contain the empty string."""
        with pytest.raises(ValueError, match="epsilon"):
            NFA(states={"q0"}, alphabet={"a", ""}, transitions={},
                initial="q0", accepting=set())

    def test_initial_not_in_states(self) -> None:
        """Initial state must be in the states set."""
        with pytest.raises(ValueError, match="Initial"):
            NFA(states={"q0"}, alphabet={"a"}, transitions={},
                initial="q_bad", accepting=set())

    def test_accepting_not_subset(self) -> None:
        """Accepting states must be a subset of states."""
        with pytest.raises(ValueError, match="Accepting"):
            NFA(states={"q0"}, alphabet={"a"}, transitions={},
                initial="q0", accepting={"q_bad"})

    def test_transition_source_invalid(self) -> None:
        """Transition source must be in states."""
        with pytest.raises(ValueError, match="source"):
            NFA(states={"q0"}, alphabet={"a"},
                transitions={("q_bad", "a"): {"q0"}},
                initial="q0", accepting=set())

    def test_transition_event_invalid(self) -> None:
        """Transition event must be in alphabet or epsilon."""
        with pytest.raises(ValueError, match="alphabet"):
            NFA(states={"q0"}, alphabet={"a"},
                transitions={("q0", "z"): {"q0"}},
                initial="q0", accepting=set())

    def test_transition_target_invalid(self) -> None:
        """Transition targets must be in states."""
        with pytest.raises(ValueError, match="targets"):
            NFA(states={"q0"}, alphabet={"a"},
                transitions={("q0", "a"): {"q_bad"}},
                initial="q0", accepting=set())


# ============================================================
# Epsilon Closure Tests
# ============================================================


class TestEpsilonClosure:
    """Tests for epsilon closure computation."""

    def test_no_epsilon_transitions(self, contains_ab: NFA) -> None:
        """Without epsilon transitions, closure is just the input set."""
        assert contains_ab.epsilon_closure(frozenset({"q0"})) == frozenset(
            {"q0"}
        )

    def test_single_epsilon(self) -> None:
        """One epsilon transition: q0 --ε--> q1."""
        nfa = NFA(
            states={"q0", "q1"},
            alphabet={"a"},
            transitions={("q0", EPSILON): {"q1"}},
            initial="q0",
            accepting=set(),
        )
        assert nfa.epsilon_closure(frozenset({"q0"})) == frozenset(
            {"q0", "q1"}
        )

    def test_chained_epsilons(self, epsilon_chain: NFA) -> None:
        """Chain: q0 --ε--> q1 --ε--> q2."""
        assert epsilon_chain.epsilon_closure(frozenset({"q0"})) == frozenset(
            {"q0", "q1", "q2"}
        )

    def test_epsilon_cycle(self) -> None:
        """Epsilon cycle should terminate without infinite loop."""
        nfa = NFA(
            states={"q0", "q1"},
            alphabet={"a"},
            transitions={
                ("q0", EPSILON): {"q1"},
                ("q1", EPSILON): {"q0"},
            },
            initial="q0",
            accepting=set(),
        )
        assert nfa.epsilon_closure(frozenset({"q0"})) == frozenset(
            {"q0", "q1"}
        )

    def test_branching_epsilons(self, a_or_ab: NFA) -> None:
        """Branching: q0 --ε--> {q1, q3}."""
        assert a_or_ab.epsilon_closure(frozenset({"q0"})) == frozenset(
            {"q0", "q1", "q3"}
        )

    def test_closure_of_multiple_states(self, epsilon_chain: NFA) -> None:
        """Closure of {q0, q3} should include both chains."""
        result = epsilon_chain.epsilon_closure(frozenset({"q0", "q3"}))
        assert result == frozenset({"q0", "q1", "q2", "q3"})

    def test_empty_set_closure(self, epsilon_chain: NFA) -> None:
        """Closure of empty set is empty."""
        assert epsilon_chain.epsilon_closure(frozenset()) == frozenset()


# ============================================================
# Processing Tests
# ============================================================


class TestNFAProcessing:
    """Tests for NFA event processing."""

    def test_initial_states_include_epsilon_closure(
        self, epsilon_chain: NFA
    ) -> None:
        """The NFA should start in the epsilon closure of the initial state."""
        assert epsilon_chain.current_states == frozenset(
            {"q0", "q1", "q2"}
        )

    def test_process_deterministic_case(self, contains_ab: NFA) -> None:
        """When only one transition exists, processing is deterministic."""
        contains_ab.process("b")
        assert contains_ab.current_states == frozenset({"q0"})

    def test_process_non_deterministic(self, contains_ab: NFA) -> None:
        """On 'a', q0 goes to both q0 and q1 (non-deterministic split)."""
        contains_ab.process("a")
        assert contains_ab.current_states == frozenset({"q0", "q1"})

    def test_process_dead_paths_vanish(self, contains_ab: NFA) -> None:
        """Paths that have no transition vanish silently."""
        contains_ab.process("a")  # {q0, q1}
        contains_ab.process("a")  # q0→{q0,q1}, q1 has no 'a' transition → dies
        assert contains_ab.current_states == frozenset({"q0", "q1"})

    def test_process_reaches_accepting(self, contains_ab: NFA) -> None:
        """Processing 'a' then 'b' should reach the accepting state."""
        contains_ab.process("a")
        contains_ab.process("b")
        # q0→{q0}, q1→{q2} from 'b', plus q0 self-loop on 'b'
        assert "q2" in contains_ab.current_states

    def test_process_through_epsilon(self, epsilon_chain: NFA) -> None:
        """Process 'a' should work through epsilon chain to reach q3."""
        epsilon_chain.process("a")
        assert epsilon_chain.current_states == frozenset({"q3"})

    def test_process_invalid_event(self, contains_ab: NFA) -> None:
        """Invalid events should raise ValueError."""
        with pytest.raises(ValueError, match="not in the alphabet"):
            contains_ab.process("c")

    def test_process_sequence(self, contains_ab: NFA) -> None:
        """process_sequence should return trace with state sets."""
        trace = contains_ab.process_sequence(["a", "b"])
        assert len(trace) == 2
        # First step: from {q0} on 'a'
        before, event, after = trace[0]
        assert event == "a"
        assert "q0" in before
        assert "q1" in after  # non-deterministic branch
        # Second step: on 'b'
        _, event2, after2 = trace[1]
        assert event2 == "b"
        assert "q2" in after2  # accepting state reached


# ============================================================
# Acceptance Tests
# ============================================================


class TestNFAAcceptance:
    """Tests for the accepts() method."""

    def test_contains_ab_accepts(self, contains_ab: NFA) -> None:
        """NFA should accept strings containing 'ab'."""
        assert contains_ab.accepts(["a", "b"]) is True
        assert contains_ab.accepts(["b", "a", "b"]) is True
        assert contains_ab.accepts(["a", "a", "b"]) is True
        assert contains_ab.accepts(["a", "b", "a", "b"]) is True

    def test_contains_ab_rejects(self, contains_ab: NFA) -> None:
        """NFA should reject strings NOT containing 'ab'."""
        assert contains_ab.accepts(["a"]) is False
        assert contains_ab.accepts(["b"]) is False
        assert contains_ab.accepts(["b", "a"]) is False
        assert contains_ab.accepts(["b", "b", "b"]) is False
        assert contains_ab.accepts([]) is False

    def test_a_or_ab_accepts(self, a_or_ab: NFA) -> None:
        """NFA should accept 'a' and 'ab'."""
        assert a_or_ab.accepts(["a"]) is True
        assert a_or_ab.accepts(["a", "b"]) is True

    def test_a_or_ab_rejects(self, a_or_ab: NFA) -> None:
        """NFA should reject everything else."""
        assert a_or_ab.accepts([]) is False
        assert a_or_ab.accepts(["b"]) is False
        assert a_or_ab.accepts(["a", "a"]) is False
        assert a_or_ab.accepts(["a", "b", "a"]) is False

    def test_epsilon_chain_accepts(self, epsilon_chain: NFA) -> None:
        """Epsilon chain NFA should accept single 'a'."""
        assert epsilon_chain.accepts(["a"]) is True

    def test_epsilon_chain_rejects(self, epsilon_chain: NFA) -> None:
        """Epsilon chain NFA should reject empty and multi-character inputs."""
        assert epsilon_chain.accepts([]) is False
        assert epsilon_chain.accepts(["a", "a"]) is False

    def test_accepts_does_not_modify_state(self, contains_ab: NFA) -> None:
        """accepts() should not change the NFA's current state."""
        original = contains_ab.current_states
        contains_ab.accepts(["a", "b", "a"])
        assert contains_ab.current_states == original

    def test_accepts_invalid_event(self, contains_ab: NFA) -> None:
        """accepts() should raise on invalid events."""
        with pytest.raises(ValueError, match="not in the alphabet"):
            contains_ab.accepts(["c"])

    def test_early_rejection(self) -> None:
        """NFA that reaches empty state set should reject early."""
        nfa = NFA(
            states={"q0", "q1"},
            alphabet={"a", "b"},
            transitions={("q0", "a"): {"q1"}},
            initial="q0",
            accepting={"q1"},
        )
        # After 'b', no states are active — should reject
        assert nfa.accepts(["b"]) is False
        assert nfa.accepts(["b", "a"]) is False


# ============================================================
# Subset Construction Tests (NFA → DFA)
# ============================================================


class TestSubsetConstruction:
    """Tests for NFA-to-DFA conversion."""

    def test_deterministic_nfa_converts(self) -> None:
        """An NFA that is already deterministic should convert cleanly."""
        nfa = NFA(
            states={"q0", "q1"},
            alphabet={"a", "b"},
            transitions={
                ("q0", "a"): {"q1"},
                ("q0", "b"): {"q0"},
                ("q1", "a"): {"q0"},
                ("q1", "b"): {"q1"},
            },
            initial="q0",
            accepting={"q1"},
        )
        dfa = nfa.to_dfa()
        # Should have 2 states (each NFA state maps to one DFA state)
        assert len(dfa.states) == 2
        # Language should be equivalent
        assert dfa.accepts(["a"]) is True
        assert dfa.accepts(["a", "a"]) is False
        assert dfa.accepts(["a", "b"]) is True

    def test_contains_ab_converts(self, contains_ab: NFA) -> None:
        """The 'contains ab' NFA should convert to an equivalent DFA."""
        dfa = contains_ab.to_dfa()

        # Verify the DFA accepts the same language
        test_cases = [
            (["a", "b"], True),
            (["b", "a", "b"], True),
            (["a", "a", "b"], True),
            (["a"], False),
            (["b"], False),
            (["b", "a"], False),
            ([], False),
        ]
        for events, expected in test_cases:
            assert dfa.accepts(events) is expected, (
                f"DFA disagrees on {events}: expected {expected}"
            )

    def test_epsilon_nfa_converts(self, a_or_ab: NFA) -> None:
        """NFA with epsilon transitions should convert correctly."""
        dfa = a_or_ab.to_dfa()

        assert dfa.accepts(["a"]) is True
        assert dfa.accepts(["a", "b"]) is True
        assert dfa.accepts([]) is False
        assert dfa.accepts(["b"]) is False
        assert dfa.accepts(["a", "a"]) is False

    def test_epsilon_chain_converts(self, epsilon_chain: NFA) -> None:
        """Epsilon chain NFA should convert to DFA accepting single 'a'."""
        dfa = epsilon_chain.to_dfa()

        assert dfa.accepts(["a"]) is True
        assert dfa.accepts([]) is False
        assert dfa.accepts(["a", "a"]) is False

    def test_converted_dfa_is_valid(self, contains_ab: NFA) -> None:
        """The converted DFA should pass validation."""
        dfa = contains_ab.to_dfa()
        warnings = dfa.validate()
        # There may be valid warnings about incompleteness, but no errors
        for w in warnings:
            assert "Unreachable" not in w

    def test_comprehensive_language_equivalence(self) -> None:
        """Exhaustively test NFA/DFA equivalence for all strings up to length 4."""
        # NFA for "ends with 'ab'"
        nfa = NFA(
            states={"q0", "q1", "q2"},
            alphabet={"a", "b"},
            transitions={
                ("q0", "a"): {"q0", "q1"},
                ("q0", "b"): {"q0"},
                ("q1", "b"): {"q2"},
            },
            initial="q0",
            accepting={"q2"},
        )
        dfa = nfa.to_dfa()

        # Generate all strings of a,b up to length 4
        def gen_strings(alpha: list[str], max_len: int) -> list[list[str]]:
            result: list[list[str]] = [[]]
            for length in range(1, max_len + 1):
                for s in gen_strings_of_length(alpha, length):
                    result.append(s)
            return result

        def gen_strings_of_length(
            alpha: list[str], length: int
        ) -> list[list[str]]:
            if length == 0:
                return [[]]
            result: list[list[str]] = []
            for s in gen_strings_of_length(alpha, length - 1):
                for c in alpha:
                    result.append([*s, c])
            return result

        for s in gen_strings(["a", "b"], 4):
            nfa_result = nfa.accepts(s)
            dfa_result = dfa.accepts(s)
            assert nfa_result == dfa_result, (
                f"Disagreement on {''.join(s)!r}: "
                f"NFA={nfa_result}, DFA={dfa_result}"
            )


# ============================================================
# Reset Tests
# ============================================================


class TestNFAReset:
    """Tests for the reset() method."""

    def test_reset_returns_to_initial(self, contains_ab: NFA) -> None:
        """After reset, current states should be epsilon closure of initial."""
        contains_ab.process("a")
        assert "q1" in contains_ab.current_states

        contains_ab.reset()
        assert contains_ab.current_states == frozenset({"q0"})

    def test_reset_with_epsilon(self, epsilon_chain: NFA) -> None:
        """Reset should re-compute epsilon closure of initial state."""
        epsilon_chain.process("a")
        assert epsilon_chain.current_states == frozenset({"q3"})

        epsilon_chain.reset()
        assert epsilon_chain.current_states == frozenset(
            {"q0", "q1", "q2"}
        )


# ============================================================
# Visualization Tests
# ============================================================


class TestNFAVisualization:
    """Tests for DOT output."""

    def test_to_dot_structure(self, contains_ab: NFA) -> None:
        """DOT output should have expected structure."""
        dot = contains_ab.to_dot()
        assert "digraph NFA" in dot
        assert "__start" in dot
        assert "doublecircle" in dot
        assert "q0" in dot
        assert "q1" in dot
        assert "q2" in dot

    def test_to_dot_epsilon_label(self, epsilon_chain: NFA) -> None:
        """Epsilon transitions should be labeled 'ε' in DOT output."""
        dot = epsilon_chain.to_dot()
        assert "ε" in dot

    def test_repr(self, contains_ab: NFA) -> None:
        """repr should contain key information."""
        r = repr(contains_ab)
        assert "NFA" in r
        assert "q0" in r

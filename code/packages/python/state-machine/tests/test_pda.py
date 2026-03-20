"""Tests for the Pushdown Automaton (PDA) implementation."""

import pytest

from state_machine.pda import PDATransition, PushdownAutomaton

# ============================================================
# Fixtures
# ============================================================


@pytest.fixture()
def balanced_parens() -> PushdownAutomaton:
    """PDA that accepts balanced parentheses: (), (()), ((())), etc."""
    return PushdownAutomaton(
        states={"q0", "accept"},
        input_alphabet={"(", ")"},
        stack_alphabet={"(", "$"},
        transitions=[
            PDATransition("q0", "(", "$", "q0", ["$", "("]),
            PDATransition("q0", "(", "(", "q0", ["(", "("]),
            PDATransition("q0", ")", "(", "q0", []),
            PDATransition("q0", None, "$", "accept", []),
        ],
        initial="q0",
        initial_stack_symbol="$",
        accepting={"accept"},
    )


@pytest.fixture()
def anbn() -> PushdownAutomaton:
    """PDA that accepts a^n b^n: ab, aabb, aaabbb, etc.

    Strategy: push 'a' for each 'a', pop 'a' for each 'b'.
    Accept when stack is empty (only $ remains).
    """
    return PushdownAutomaton(
        states={"pushing", "popping", "accept"},
        input_alphabet={"a", "b"},
        stack_alphabet={"a", "$"},
        transitions=[
            # Push phase: reading a's
            PDATransition("pushing", "a", "$", "pushing", ["$", "a"]),
            PDATransition("pushing", "a", "a", "pushing", ["a", "a"]),
            # Switch to popping on first 'b'
            PDATransition("pushing", "b", "a", "popping", []),
            # Pop phase: reading b's
            PDATransition("popping", "b", "a", "popping", []),
            # Accept when stack is empty
            PDATransition("popping", None, "$", "accept", []),
        ],
        initial="pushing",
        initial_stack_symbol="$",
        accepting={"accept"},
    )


# ============================================================
# Construction Tests
# ============================================================


class TestPDAConstruction:
    """Tests for PDA construction and validation."""

    def test_valid_construction(self, balanced_parens: PushdownAutomaton) -> None:
        """A valid PDA should construct without errors."""
        assert balanced_parens.current_state == "q0"
        assert balanced_parens.stack == ("$",)

    def test_empty_states_rejected(self) -> None:
        """Empty states set is invalid."""
        with pytest.raises(ValueError, match="non-empty"):
            PushdownAutomaton(
                states=set(), input_alphabet=set(), stack_alphabet={"$"},
                transitions=[], initial="q0", initial_stack_symbol="$",
                accepting=set(),
            )

    def test_initial_not_in_states(self) -> None:
        """Initial state must be in states."""
        with pytest.raises(ValueError, match="Initial"):
            PushdownAutomaton(
                states={"q0"}, input_alphabet=set(), stack_alphabet={"$"},
                transitions=[], initial="q_bad", initial_stack_symbol="$",
                accepting=set(),
            )

    def test_initial_stack_not_in_alphabet(self) -> None:
        """Initial stack symbol must be in stack alphabet."""
        with pytest.raises(ValueError, match="stack symbol"):
            PushdownAutomaton(
                states={"q0"}, input_alphabet=set(), stack_alphabet={"$"},
                transitions=[], initial="q0", initial_stack_symbol="X",
                accepting=set(),
            )

    def test_duplicate_transitions_rejected(self) -> None:
        """Duplicate transitions make the PDA non-deterministic."""
        with pytest.raises(ValueError, match="Duplicate"):
            PushdownAutomaton(
                states={"q0", "q1"},
                input_alphabet={"a"},
                stack_alphabet={"$"},
                transitions=[
                    PDATransition("q0", "a", "$", "q0", ["$"]),
                    PDATransition("q0", "a", "$", "q1", ["$"]),
                ],
                initial="q0",
                initial_stack_symbol="$",
                accepting=set(),
            )


# ============================================================
# Balanced Parentheses Tests
# ============================================================


class TestBalancedParens:
    """Tests for the balanced parentheses PDA."""

    def test_simple_pair(self, balanced_parens: PushdownAutomaton) -> None:
        """() should be accepted."""
        assert balanced_parens.accepts(["(", ")"]) is True

    def test_nested(self, balanced_parens: PushdownAutomaton) -> None:
        """(()) should be accepted."""
        assert balanced_parens.accepts(["(", "(", ")", ")"]) is True

    def test_triple_nested(self, balanced_parens: PushdownAutomaton) -> None:
        """((())) should be accepted."""
        assert balanced_parens.accepts(
            ["(", "(", "(", ")", ")", ")"]
        ) is True

    def test_sequential(self, balanced_parens: PushdownAutomaton) -> None:
        """()() should be accepted."""
        assert balanced_parens.accepts(["(", ")", "(", ")"]) is True

    def test_empty_accepted(self, balanced_parens: PushdownAutomaton) -> None:
        """Empty string is balanced (zero pairs)."""
        assert balanced_parens.accepts([]) is True

    def test_unmatched_open(self, balanced_parens: PushdownAutomaton) -> None:
        """((( should be rejected — unmatched opens."""
        assert balanced_parens.accepts(["(", "(", "("]) is False

    def test_unmatched_close(self, balanced_parens: PushdownAutomaton) -> None:
        """) should be rejected — close without open."""
        assert balanced_parens.accepts([")"]) is False

    def test_wrong_order(self, balanced_parens: PushdownAutomaton) -> None:
        """)( should be rejected — wrong order."""
        assert balanced_parens.accepts([")", "("]) is False

    def test_partial_match(self, balanced_parens: PushdownAutomaton) -> None:
        """(() should be rejected — one unmatched open."""
        assert balanced_parens.accepts(["(", "(", ")"]) is False

    def test_extra_close(self, balanced_parens: PushdownAutomaton) -> None:
        """()) should be rejected — extra close."""
        assert balanced_parens.accepts(["(", ")", ")"]) is False


# ============================================================
# a^n b^n Tests
# ============================================================


class TestAnBn:
    """Tests for the a^n b^n PDA."""

    def test_ab(self, anbn: PushdownAutomaton) -> None:
        """ab (n=1) should be accepted."""
        assert anbn.accepts(["a", "b"]) is True

    def test_aabb(self, anbn: PushdownAutomaton) -> None:
        """aabb (n=2) should be accepted."""
        assert anbn.accepts(["a", "a", "b", "b"]) is True

    def test_aaabbb(self, anbn: PushdownAutomaton) -> None:
        """aaabbb (n=3) should be accepted."""
        assert anbn.accepts(["a", "a", "a", "b", "b", "b"]) is True

    def test_empty_rejected(self, anbn: PushdownAutomaton) -> None:
        """Empty string should be rejected (n=0 is not in the language)."""
        assert anbn.accepts([]) is False

    def test_a_only(self, anbn: PushdownAutomaton) -> None:
        """aaa should be rejected — no b's."""
        assert anbn.accepts(["a", "a", "a"]) is False

    def test_b_only(self, anbn: PushdownAutomaton) -> None:
        """bbb should be rejected — no a's."""
        assert anbn.accepts(["b", "b", "b"]) is False

    def test_more_as(self, anbn: PushdownAutomaton) -> None:
        """aab should be rejected — more a's than b's."""
        assert anbn.accepts(["a", "a", "b"]) is False

    def test_more_bs(self, anbn: PushdownAutomaton) -> None:
        """abb should be rejected — more b's than a's."""
        assert anbn.accepts(["a", "b", "b"]) is False

    def test_interleaved(self, anbn: PushdownAutomaton) -> None:
        """abab should be rejected — a's and b's must be grouped."""
        assert anbn.accepts(["a", "b", "a", "b"]) is False

    def test_ba(self, anbn: PushdownAutomaton) -> None:
        """ba should be rejected — wrong order."""
        assert anbn.accepts(["b", "a"]) is False


# ============================================================
# Processing and Trace Tests
# ============================================================


class TestPDAProcessing:
    """Tests for process() and trace."""

    def test_process_single(self, balanced_parens: PushdownAutomaton) -> None:
        """Processing '(' should push onto the stack."""
        balanced_parens.process("(")
        assert balanced_parens.current_state == "q0"
        # Stack should be: $ ( (top)
        assert balanced_parens.stack_top == "("

    def test_process_sequence_trace(
        self, balanced_parens: PushdownAutomaton
    ) -> None:
        """process_sequence should return trace entries."""
        trace = balanced_parens.process_sequence(["(", ")"])
        # Should have at least 2 entries (push, pop) + epsilon for accept
        assert len(trace) >= 2
        # First entry: push
        assert trace[0].event == "("
        assert trace[0].source == "q0"
        # Second entry: pop
        assert trace[1].event == ")"

    def test_process_no_transition(self) -> None:
        """Processing with no matching transition should raise."""
        pda = PushdownAutomaton(
            states={"q0"},
            input_alphabet={"a"},
            stack_alphabet={"$"},
            transitions=[],
            initial="q0",
            initial_stack_symbol="$",
            accepting=set(),
        )
        with pytest.raises(ValueError, match="No transition"):
            pda.process("a")

    def test_stack_inspection(self, balanced_parens: PushdownAutomaton) -> None:
        """Stack contents should be inspectable after each step.

        Stack is stored bottom-to-top: ("$", "(") means "$" at bottom, "(" on top.
        """
        balanced_parens.process("(")
        assert balanced_parens.stack == ("$", "(")
        assert balanced_parens.stack_top == "("

        balanced_parens.process("(")
        assert balanced_parens.stack == ("$", "(", "(")
        assert balanced_parens.stack_top == "("

        balanced_parens.process(")")
        assert balanced_parens.stack == ("$", "(")

        balanced_parens.process(")")
        assert balanced_parens.stack == ("$",)


# ============================================================
# Reset Tests
# ============================================================


class TestPDAReset:
    """Tests for reset()."""

    def test_reset(self, balanced_parens: PushdownAutomaton) -> None:
        """Reset should restore initial state and stack."""
        balanced_parens.process("(")
        balanced_parens.process("(")
        assert balanced_parens.stack_top == "("

        balanced_parens.reset()
        assert balanced_parens.current_state == "q0"
        assert balanced_parens.stack == ("$",)
        assert balanced_parens.trace == []


# ============================================================
# Accepts Non-Mutating Tests
# ============================================================


class TestPDAAcceptsNonMutating:
    """accepts() should not modify the PDA's state."""

    def test_accepts_does_not_modify(
        self, balanced_parens: PushdownAutomaton
    ) -> None:
        """accepts() should not change state or stack."""
        balanced_parens.process("(")
        original_state = balanced_parens.current_state
        original_stack = balanced_parens.stack

        balanced_parens.accepts([")", "(", ")"])

        assert balanced_parens.current_state == original_state
        assert balanced_parens.stack == original_stack


# ============================================================
# Repr Tests
# ============================================================


class TestPDARepr:
    """Tests for __repr__."""

    def test_repr(self, balanced_parens: PushdownAutomaton) -> None:
        """repr should contain key info."""
        r = repr(balanced_parens)
        assert "PDA" in r
        assert "q0" in r

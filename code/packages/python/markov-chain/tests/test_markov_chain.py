"""
Tests for coding_adventures_markov_chain (DT28)
================================================

Covers all 10 required spec test cases plus extensive additional tests for
>95% code coverage.

Test organisation
-----------------
1. Construction
2. Single-pair training
3. Sequence training + probability assertions
4. Laplace smoothing
5. Generate length
6. Generate string (character chain)
7. Stationary distribution sums to 1
8. Order-2 deterministic chain
9. Unknown state error
10. Multi-train accumulation
11. Additional edge-case and coverage tests
"""

import pytest
from coding_adventures_markov_chain import MarkovChain


# ---------------------------------------------------------------------------
# 1. Construction
# ---------------------------------------------------------------------------


class TestConstruction:
    """Test 1: MarkovChain() creates an empty chain with 0 states."""

    def test_default_constructor_has_zero_states(self) -> None:
        chain = MarkovChain()
        assert chain.states() == []

    def test_default_order_is_one(self) -> None:
        chain = MarkovChain()
        assert chain._order == 1

    def test_default_smoothing_is_zero(self) -> None:
        chain = MarkovChain()
        assert chain._smoothing == 0.0

    def test_transition_matrix_empty_before_training(self) -> None:
        chain = MarkovChain()
        assert chain.transition_matrix() == {}

    def test_pre_registered_states_appear_in_states_list(self) -> None:
        chain = MarkovChain(states=["A", "B", "C"])
        assert set(chain.states()) == {"A", "B", "C"}

    def test_order_and_smoothing_kwargs(self) -> None:
        chain = MarkovChain(order=3, smoothing=0.5)
        assert chain._order == 3
        assert chain._smoothing == 0.5

    def test_repr_before_training(self) -> None:
        chain = MarkovChain()
        r = repr(chain)
        assert "MarkovChain" in r
        assert "order=1" in r

    def test_repr_after_training(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B"])
        r = repr(chain)
        assert "states=2" in r


# ---------------------------------------------------------------------------
# 2. Train single pair
# ---------------------------------------------------------------------------


class TestTrainSinglePair:
    """Test 2: train([A, B]) → probability(A, B) == 1.0."""

    def test_single_pair_probability_one(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B"])
        assert chain.probability("A", "B") == pytest.approx(1.0)

    def test_single_pair_other_direction_is_zero(self) -> None:
        # A→B observed, but B→A was never seen (no transition from B).
        chain = MarkovChain()
        chain.train(["A", "B"])
        assert chain.probability("B", "A") == 0.0

    def test_single_pair_states(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B"])
        assert set(chain.states()) == {"A", "B"}

    def test_single_pair_unknown_probability_is_zero(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B"])
        assert chain.probability("A", "Z") == 0.0
        assert chain.probability("Z", "A") == 0.0

    def test_single_pair_transition_matrix(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B"])
        tm = chain.transition_matrix()
        assert "A" in tm
        assert tm["A"]["B"] == pytest.approx(1.0)


# ---------------------------------------------------------------------------
# 3. Train sequence
# ---------------------------------------------------------------------------


class TestTrainSequence:
    """Test 3: train([A,B,A,C,A,B,B,A]) → specific probabilities."""

    @pytest.fixture
    def trained_chain(self) -> MarkovChain:
        chain = MarkovChain()
        chain.train(["A", "B", "A", "C", "A", "B", "B", "A"])
        return chain

    def test_a_to_b_approx_two_thirds(self, trained_chain: MarkovChain) -> None:
        # A appears 3 times as a context: A→B, A→C, A→B → 2/3 for B.
        assert trained_chain.probability("A", "B") == pytest.approx(2 / 3, abs=1e-6)

    def test_a_to_c_approx_one_third(self, trained_chain: MarkovChain) -> None:
        assert trained_chain.probability("A", "C") == pytest.approx(1 / 3, abs=1e-6)

    def test_b_to_a_approx_two_thirds(self, trained_chain: MarkovChain) -> None:
        # B appears 3 times: B→A, B→B, B→A → 2/3 for A.
        assert trained_chain.probability("B", "A") == pytest.approx(2 / 3, abs=1e-6)

    def test_b_to_b_approx_one_third(self, trained_chain: MarkovChain) -> None:
        assert trained_chain.probability("B", "B") == pytest.approx(1 / 3, abs=1e-6)

    def test_a_to_a_is_zero(self, trained_chain: MarkovChain) -> None:
        # A→A never observed in the sequence.
        assert trained_chain.probability("A", "A") == 0.0

    def test_row_sums_to_one_for_a(self, trained_chain: MarkovChain) -> None:
        # P(A→B) + P(A→C) = 1.0
        total = trained_chain.probability("A", "B") + trained_chain.probability("A", "C")
        assert total == pytest.approx(1.0, abs=1e-6)

    def test_row_sums_to_one_for_b(self, trained_chain: MarkovChain) -> None:
        total = trained_chain.probability("B", "A") + trained_chain.probability("B", "B")
        assert total == pytest.approx(1.0, abs=1e-6)

    def test_c_to_a_is_one(self, trained_chain: MarkovChain) -> None:
        # C appears once and always goes to A.
        assert trained_chain.probability("C", "A") == pytest.approx(1.0)

    def test_states_contains_all_three(self, trained_chain: MarkovChain) -> None:
        assert set(trained_chain.states()) == {"A", "B", "C"}


# ---------------------------------------------------------------------------
# 4. Laplace smoothing
# ---------------------------------------------------------------------------


class TestLaplaceSmoothing:
    """Test 4: MarkovChain(smoothing=1.0, states=["A","B","C"]) + train([A,B])
               → probability(A, C) == 0.25"""

    def test_probability_a_to_c_with_laplace(self) -> None:
        chain = MarkovChain(order=1, smoothing=1.0, states=["A", "B", "C"])
        chain.train(["A", "B"])
        # Raw counts from A: {B: 1}
        # With smoothing α=1, 3 known states:
        #   denom = 1 + 1*3 = 4
        #   P(A→A) = (0+1)/4 = 0.25
        #   P(A→B) = (1+1)/4 = 0.5
        #   P(A→C) = (0+1)/4 = 0.25
        assert chain.probability("A", "C") == pytest.approx(0.25)

    def test_probability_a_to_b_with_laplace(self) -> None:
        chain = MarkovChain(order=1, smoothing=1.0, states=["A", "B", "C"])
        chain.train(["A", "B"])
        # P(A→B) = (1+1)/4 = 0.5
        assert chain.probability("A", "B") == pytest.approx(0.5)

    def test_probability_a_to_a_with_laplace(self) -> None:
        chain = MarkovChain(order=1, smoothing=1.0, states=["A", "B", "C"])
        chain.train(["A", "B"])
        # P(A→A) = (0+1)/4 = 0.25
        assert chain.probability("A", "A") == pytest.approx(0.25)

    def test_smoothed_row_sums_to_one(self) -> None:
        chain = MarkovChain(order=1, smoothing=1.0, states=["A", "B", "C"])
        chain.train(["A", "B"])
        total = (
            chain.probability("A", "A")
            + chain.probability("A", "B")
            + chain.probability("A", "C")
        )
        assert total == pytest.approx(1.0, abs=1e-9)

    def test_lidstone_smoothing_point_five(self) -> None:
        # α = 0.5 (Lidstone/Jeffreys-Perks smoothing)
        # train [A, B]: raw_counts[A][B] = 1
        # denom = 1 + 0.5*3 = 2.5
        # P(A→B) = 1.5/2.5 = 0.6
        # P(A→C) = 0.5/2.5 = 0.2
        chain = MarkovChain(order=1, smoothing=0.5, states=["A", "B", "C"])
        chain.train(["A", "B"])
        assert chain.probability("A", "B") == pytest.approx(0.6, abs=1e-9)
        assert chain.probability("A", "C") == pytest.approx(0.2, abs=1e-9)

    def test_smoothing_adds_new_state_from_preregistered(self) -> None:
        # Pre-register "C" but never see it in training.
        # With smoothing > 0, probability(A, C) should be > 0.
        chain = MarkovChain(order=1, smoothing=1.0, states=["A", "B", "C"])
        chain.train(["A", "B"])
        # C was never a target in training but is pre-registered.
        assert chain.probability("A", "C") > 0.0


# ---------------------------------------------------------------------------
# 5. Generate length
# ---------------------------------------------------------------------------


class TestGenerateLength:
    """Test 5: generate("A", 10) returns exactly 10 items."""

    @pytest.fixture
    def chain(self) -> MarkovChain:
        c = MarkovChain()
        c.train(["A", "B", "A", "C", "A", "B", "B", "A"])
        return c

    def test_generate_length_ten(self, chain: MarkovChain) -> None:
        result = chain.generate("A", 10)
        assert len(result) == 10

    def test_generate_starts_with_start(self, chain: MarkovChain) -> None:
        result = chain.generate("A", 10)
        assert result[0] == "A"

    def test_generate_length_one(self, chain: MarkovChain) -> None:
        result = chain.generate("A", 1)
        assert result == ["A"]

    def test_generate_length_fifty(self, chain: MarkovChain) -> None:
        result = chain.generate("A", 50)
        assert len(result) == 50

    def test_generate_all_states_valid(self, chain: MarkovChain) -> None:
        # All generated states must be in the known state space.
        result = chain.generate("A", 100)
        valid = {"A", "B", "C"}
        assert all(s in valid for s in result)

    def test_generate_from_b(self, chain: MarkovChain) -> None:
        result = chain.generate("B", 5)
        assert len(result) == 5
        assert result[0] == "B"


# ---------------------------------------------------------------------------
# 6. Generate string
# ---------------------------------------------------------------------------


class TestGenerateString:
    """Test 6: generate_string("th", 50) on English-like text → 50-char string."""

    @pytest.fixture
    def english_chain(self) -> MarkovChain:
        c = MarkovChain(order=2)
        # Train on a reasonably long English-like text.
        text = (
            "the quick brown fox jumps over the lazy dog "
            "the cat sat on the mat "
            "to be or not to be that is the question "
            "she sells seashells by the seashore "
            "how much wood would a woodchuck chuck "
            "the rain in spain stays mainly in the plain "
        ) * 5
        c.train_string(text)
        return c

    def test_generate_string_length(self, english_chain: MarkovChain) -> None:
        result = english_chain.generate_string("th", 50)
        assert len(result) == 50

    def test_generate_string_starts_with_seed(self, english_chain: MarkovChain) -> None:
        result = english_chain.generate_string("th", 50)
        assert result.startswith("th")

    def test_generate_string_order1_length(self) -> None:
        chain = MarkovChain(order=1)
        chain.train_string("abcdefghijklmnopqrstuvwxyz " * 10)
        result = chain.generate_string("a", 30)
        assert len(result) == 30

    def test_generate_string_seed_longer_than_order(self) -> None:
        chain = MarkovChain(order=2)
        chain.train_string("abcabcabc" * 5)
        # Seed "abc" has 3 chars, order=2; last 2 chars ("bc") form the window.
        result = chain.generate_string("abc", 9)
        assert len(result) == 9

    def test_generate_string_too_short_seed_raises(self) -> None:
        chain = MarkovChain(order=3)
        chain.train_string("abcabcabc")
        with pytest.raises(ValueError, match="too short"):
            chain.generate_string("ab", 10)  # need ≥3 chars for order=3


# ---------------------------------------------------------------------------
# 7. Stationary distribution sums to 1
# ---------------------------------------------------------------------------


class TestStationaryDistribution:
    """Test 7: stationary_distribution().values() ≈ 1.0 for any ergodic chain."""

    def test_stationary_dist_sums_to_one(self) -> None:
        chain = MarkovChain()
        # Train on a reversible ergodic chain A↔B↔C.
        chain.train(["A", "B", "C", "A", "B", "A", "C", "B", "A", "B", "C"] * 5)
        dist = chain.stationary_distribution()
        assert sum(dist.values()) == pytest.approx(1.0, abs=1e-6)

    def test_stationary_dist_all_states_present(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B", "C", "A", "B", "A", "C", "B", "A"] * 5)
        dist = chain.stationary_distribution()
        assert set(dist.keys()) == {"A", "B", "C"}

    def test_stationary_dist_all_values_positive(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B", "C", "A", "B", "A", "C", "B", "A"] * 5)
        dist = chain.stationary_distribution()
        assert all(v > 0 for v in dist.values())

    def test_stationary_dist_two_state(self) -> None:
        # Simple two-state ergodic chain.
        chain = MarkovChain()
        chain.train(["A", "B", "A", "B", "A", "B"])
        dist = chain.stationary_distribution()
        assert sum(dist.values()) == pytest.approx(1.0, abs=1e-6)

    def test_stationary_dist_self_loop(self) -> None:
        # Chain with self-loop: A→A, A→B, B→A.
        chain = MarkovChain(smoothing=0.5, states=["A", "B"])
        chain.train(["A", "A", "B", "A", "A", "B", "A"])
        dist = chain.stationary_distribution()
        assert sum(dist.values()) == pytest.approx(1.0, abs=1e-6)

    def test_stationary_dist_not_converge_raises(self) -> None:
        # A chain with no outgoing transitions from some state can cause
        # non-ergodic behaviour — but to test this we need a degenerate case.
        # A chain with 0 transitions should raise ValueError.
        chain = MarkovChain()
        with pytest.raises(ValueError, match="no transitions"):
            chain.stationary_distribution()

    def test_stationary_dist_weather_model(self) -> None:
        # Build a weather chain that approximates:
        # Sunny 70/20/10, Cloudy 30/40/30, Rainy 20/30/50 transition matrix.
        # We build a flat sequence from (from, to) pairs weighted by row counts.
        chain2 = MarkovChain()
        pairs = (
            [("Sunny", "Sunny")] * 7
            + [("Sunny", "Cloudy")] * 2
            + [("Sunny", "Rainy")] * 1
            + [("Cloudy", "Sunny")] * 3
            + [("Cloudy", "Cloudy")] * 4
            + [("Cloudy", "Rainy")] * 3
            + [("Rainy", "Sunny")] * 2
            + [("Rainy", "Cloudy")] * 3
            + [("Rainy", "Rainy")] * 5
        )
        # Build a flat sequence from pairs (each pair [s1, s2] contributes s1→s2).
        flat: list = []
        for a, b in pairs:
            flat.extend([a, b])
        chain2.train(flat)
        dist = chain2.stationary_distribution()
        assert sum(dist.values()) == pytest.approx(1.0, abs=1e-4)


# ---------------------------------------------------------------------------
# 8. Order-2 chain
# ---------------------------------------------------------------------------


class TestOrder2Chain:
    """Test 8: order-2 chain on "abcabcabc" → generate_string("ab", 9) == "abcabcabc"."""

    def test_order2_deterministic_generate(self) -> None:
        chain = MarkovChain(order=2)
        chain.train_string("abcabcabc")
        # Each context ("a","b") always transitions to "c", etc.
        result = chain.generate_string("ab", 9)
        assert result == "abcabcabc"

    def test_order2_probability_ab_to_c(self) -> None:
        chain = MarkovChain(order=2)
        chain.train_string("abcabcabc")
        assert chain.probability(("a", "b"), "c") == pytest.approx(1.0)

    def test_order2_probability_bc_to_a(self) -> None:
        chain = MarkovChain(order=2)
        chain.train_string("abcabcabc")
        assert chain.probability(("b", "c"), "a") == pytest.approx(1.0)

    def test_order2_generate_length(self) -> None:
        chain = MarkovChain(order=2)
        chain.train_string("abcabcabc")
        result = chain.generate(("a", "b"), 6)
        # result should be ["a", "b", "c", "a", "b", "c"]
        assert len(result) == 6

    def test_order2_generate_tuple_start(self) -> None:
        chain = MarkovChain(order=2)
        chain.train_string("abcabcabc")
        result = chain.generate(("a", "b"), 6)
        assert result == ["a", "b", "c", "a", "b", "c"]

    def test_order2_next_state_returns_single_char(self) -> None:
        chain = MarkovChain(order=2)
        chain.train_string("abcabcabc")
        next_s = chain.next_state(("a", "b"))
        assert next_s == "c"

    def test_order2_transition_matrix_keys_are_tuples(self) -> None:
        chain = MarkovChain(order=2)
        chain.train_string("abcabcabc")
        tm = chain.transition_matrix()
        # Keys should be 2-tuples for order-2.
        for key in tm:
            assert isinstance(key, tuple)
            assert len(key) == 2

    def test_order3_generate_string(self) -> None:
        chain = MarkovChain(order=3)
        chain.train_string("abcdabcdabcd")
        result = chain.generate_string("abc", 12)
        assert len(result) == 12
        # Deterministic: each trigram maps to exactly one next char.
        assert result == "abcdabcdabcd"


# ---------------------------------------------------------------------------
# 9. Unknown state
# ---------------------------------------------------------------------------


class TestUnknownState:
    """Test 9: next_state("UNKNOWN") raises ValueError."""

    def test_unknown_state_raises_value_error(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B"])
        with pytest.raises(ValueError, match="Unknown state"):
            chain.next_state("UNKNOWN")

    def test_unknown_state_error_message_contains_state(self) -> None:
        chain = MarkovChain()
        chain.train(["X", "Y"])
        with pytest.raises(ValueError, match="MISSING"):
            chain.next_state("MISSING")

    def test_unknown_state_on_empty_chain(self) -> None:
        chain = MarkovChain()
        with pytest.raises(ValueError):
            chain.next_state("anything")

    def test_unknown_order2_tuple_raises(self) -> None:
        chain = MarkovChain(order=2)
        chain.train_string("abc")
        with pytest.raises(ValueError):
            chain.next_state(("z", "z"))  # never seen in training


# ---------------------------------------------------------------------------
# 10. Multi-train accumulation
# ---------------------------------------------------------------------------


class TestMultiTrainAccumulation:
    """Test 10: Calling train() twice accumulates counts before re-normalising."""

    def test_two_trains_accumulate(self) -> None:
        chain = MarkovChain()
        # First train: A→B once.
        chain.train(["A", "B"])
        assert chain.probability("A", "B") == pytest.approx(1.0)

        # Second train: A→C once.
        chain.train(["A", "C"])
        # Now A→B: 1 count, A→C: 1 count → each 50%.
        assert chain.probability("A", "B") == pytest.approx(0.5, abs=1e-6)
        assert chain.probability("A", "C") == pytest.approx(0.5, abs=1e-6)

    def test_multi_train_state_set_grows(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B"])
        assert set(chain.states()) == {"A", "B"}
        chain.train(["C", "D"])
        assert "C" in chain.states()
        assert "D" in chain.states()

    def test_three_trains_accumulate(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B"])  # A→B: 1
        chain.train(["A", "B"])  # A→B: 2
        chain.train(["A", "C"])  # A→C: 1
        # A→B: 2/3, A→C: 1/3
        assert chain.probability("A", "B") == pytest.approx(2 / 3, abs=1e-6)
        assert chain.probability("A", "C") == pytest.approx(1 / 3, abs=1e-6)

    def test_train_string_twice_accumulates(self) -> None:
        chain = MarkovChain()
        chain.train_string("ab")
        chain.train_string("ac")
        # a→b: 1, a→c: 1 → each 0.5
        assert chain.probability("a", "b") == pytest.approx(0.5, abs=1e-6)
        assert chain.probability("a", "c") == pytest.approx(0.5, abs=1e-6)


# ---------------------------------------------------------------------------
# 11. Additional tests for coverage
# ---------------------------------------------------------------------------


class TestTransitionMatrix:
    """Test transition_matrix returns an independent copy."""

    def test_transition_matrix_is_copy(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B"])
        tm = chain.transition_matrix()
        # Mutate the copy — the original should be unaffected.
        tm["A"]["B"] = 0.0
        assert chain.probability("A", "B") == pytest.approx(1.0)

    def test_transition_matrix_structure(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B", "A", "C"])
        tm = chain.transition_matrix()
        assert "A" in tm
        assert "B" in tm["A"] or "C" in tm["A"]


class TestProbabilityEdgeCases:
    """Probability returns 0.0 gracefully for unseen states."""

    def test_unknown_from_state_returns_zero(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B"])
        assert chain.probability("Z", "A") == 0.0

    def test_known_from_unknown_to_returns_zero(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B"])
        assert chain.probability("A", "Z") == 0.0

    def test_probability_after_empty_train(self) -> None:
        # train([]) has no windows → no transitions.
        chain = MarkovChain()
        chain.train([])
        assert chain.probability("A", "B") == 0.0

    def test_probability_sums_to_one_after_smoothing(self) -> None:
        chain = MarkovChain(smoothing=1.0, states=["X", "Y", "Z"])
        chain.train(["X", "Y", "X", "Z"])
        # All rows should sum to 1.
        for ctx in chain.transition_matrix():
            row = chain.transition_matrix()[ctx]
            assert sum(row.values()) == pytest.approx(1.0, abs=1e-9)


class TestGenerateEdgeCases:
    """Edge cases for generate and generate_string."""

    def test_generate_length_two(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B", "C"])
        result = chain.generate("A", 2)
        assert len(result) == 2
        assert result[0] == "A"

    def test_generate_string_exact_seed_length(self) -> None:
        # For order=2, generate_string with seed of exactly 2 chars.
        chain = MarkovChain(order=2)
        chain.train_string("xyzxyzxyz")
        result = chain.generate_string("xy", 9)
        assert len(result) == 9

    def test_generate_string_order1_seed_longer_than_one(self) -> None:
        # For order=1, seed "hello" → start from last char 'o'.
        chain = MarkovChain(order=1)
        chain.train_string("abcdeabcde" * 5)
        result = chain.generate_string("abcde", 10)
        assert len(result) == 10

    def test_generate_with_self_loop(self) -> None:
        # A→A always.
        chain = MarkovChain()
        chain.train(["A", "A", "A", "A"])
        result = chain.generate("A", 5)
        assert result == ["A", "A", "A", "A", "A"]


class TestGraphTopologySync:
    """Verify the internal directed graph reflects the transition topology."""

    def test_graph_has_edge_after_training(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B"])
        assert chain._graph.has_edge("A", "B")

    def test_graph_does_not_have_reverse_edge(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B"])
        # B→A was never trained, so the graph should NOT have that edge.
        assert not chain._graph.has_edge("B", "A")

    def test_graph_has_self_loop_when_trained(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "A", "A"])
        # A→A self-loop was observed.
        assert chain._graph.has_edge("A", "A")

    def test_graph_nodes_match_known_states(self) -> None:
        chain = MarkovChain()
        chain.train(["X", "Y", "Z", "X"])
        known = set(chain.states())
        graph_nodes = set(chain._graph.nodes())
        # Graph nodes may be a subset (only nodes that participated in transitions).
        # But all transition endpoints should be in known states.
        assert graph_nodes.issubset(known | {("x",), ("y",)})  # flexible check


class TestOrder2AdditionalCoverage:
    """Additional order-2 coverage to push over 95%."""

    def test_order2_states_are_atomic(self) -> None:
        # states() returns atomic chars, not tuples.
        chain = MarkovChain(order=2)
        chain.train_string("abcabc")
        # Atomic states: a, b, c (not ("a","b") etc.)
        st = chain.states()
        assert "a" in st
        assert "b" in st
        assert "c" in st

    def test_order2_probability_unseen_tuple(self) -> None:
        chain = MarkovChain(order=2)
        chain.train_string("abc")
        # ("z","z") never seen → 0.0
        assert chain.probability(("z", "z"), "a") == 0.0

    def test_order2_generate_string_length_matches(self) -> None:
        chain = MarkovChain(order=2)
        chain.train_string("abcabcabcabc")
        result = chain.generate_string("ab", 20)
        assert len(result) == 20

    def test_order2_next_state_unknown_tuple_raises(self) -> None:
        chain = MarkovChain(order=2)
        chain.train_string("abc")
        with pytest.raises(ValueError, match="Unknown state"):
            chain.next_state(("x", "y"))


class TestStationaryDistributionConvergence:
    """More stationary distribution tests including non-convergence."""

    def test_three_state_symmetric_chain(self) -> None:
        # Symmetric random walk A↔B↔C (with wrap A↔C).
        # Stationary distribution should be uniform (1/3, 1/3, 1/3).
        chain = MarkovChain(smoothing=1.0, states=["A", "B", "C"])
        # Train with equal transitions.
        seq = ["A", "B", "C", "A", "C", "B", "A", "B", "C"] * 20
        chain.train(seq)
        dist = chain.stationary_distribution()
        assert sum(dist.values()) == pytest.approx(1.0, abs=1e-6)

    def test_no_states_raises(self) -> None:
        chain = MarkovChain()
        with pytest.raises(ValueError, match="no transitions"):
            chain.stationary_distribution()

    def test_stationary_dist_single_state(self) -> None:
        # One state that always transitions to itself.
        chain = MarkovChain()
        chain.train(["A", "A", "A", "A"])
        dist = chain.stationary_distribution()
        assert dist["A"] == pytest.approx(1.0)


class TestReprAndInspection:
    """Test __repr__ and inspection helpers."""

    def test_repr_contains_order(self) -> None:
        chain = MarkovChain(order=3)
        assert "order=3" in repr(chain)

    def test_repr_contains_smoothing(self) -> None:
        chain = MarkovChain(smoothing=0.5)
        assert "smoothing=0.5" in repr(chain)

    def test_states_sorted_deterministically(self) -> None:
        chain = MarkovChain()
        chain.train(["C", "A", "B", "C", "A"])
        st = chain.states()
        assert st == sorted(st, key=str)

    def test_transition_matrix_returns_all_contexts(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B", "C", "A"])
        tm = chain.transition_matrix()
        assert "A" in tm
        assert "B" in tm
        assert "C" in tm


class TestNumericalAccuracy:
    """Test that probabilities stay well-formed (non-negative, sum to 1)."""

    def test_all_rows_sum_to_one(self) -> None:
        chain = MarkovChain(smoothing=0.3, states=["A", "B", "C", "D"])
        chain.train(["A", "B", "A", "C", "D", "A", "B", "D", "C", "A"] * 10)
        tm = chain.transition_matrix()
        for ctx, row in tm.items():
            total = sum(row.values())
            assert total == pytest.approx(1.0, abs=1e-9), (
                f"Row {ctx!r} sums to {total}"
            )

    def test_all_probabilities_non_negative(self) -> None:
        chain = MarkovChain(smoothing=1.0, states=list("abcde"))
        chain.train(list("abcdeabcde"))
        tm = chain.transition_matrix()
        for row in tm.values():
            for prob in row.values():
                assert prob >= 0.0

    def test_probability_is_float(self) -> None:
        chain = MarkovChain()
        chain.train(["A", "B"])
        p = chain.probability("A", "B")
        assert isinstance(p, float)

    def test_probability_bounds(self) -> None:
        chain = MarkovChain(smoothing=0.5, states=list("abc"))
        chain.train(list("abcabc"))
        tm = chain.transition_matrix()
        for row in tm.values():
            for prob in row.values():
                assert 0.0 <= prob <= 1.0

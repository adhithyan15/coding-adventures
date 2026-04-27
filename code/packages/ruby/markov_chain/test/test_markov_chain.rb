# frozen_string_literal: true

require "test_helper"

# --------------------------------------------------------------------------
# test_markov_chain.rb — Tests for CodingAdventures::MarkovChain (DT28)
# --------------------------------------------------------------------------
#
# Test plan (10 mandatory spec cases + additional coverage):
#
#  1. Construction — empty chain, 0 states
#  2. Train single pair — probability(A, B) == 1.0
#  3. Train sequence — multi-step transitions from [A,B,A,C,A,B,B,A]
#  4. Laplace smoothing — probability(A, C) == 0.25 with 3 states
#  5. Generate length — exactly 10 elements
#  6. Generate string — correct length and prefix
#  7. Stationary distribution sums to 1
#  8. Order-2 chain — deterministic reproduce "abcabcabc"
#  9. Unknown state — KeyError raised
# 10. Multi-train accumulation — combined probabilities
#
# Additional coverage:
#  - train_string convenience method
#  - generate_string length guarantee
#  - probability returns 0.0 for unknown transition
#  - transition_matrix structure
#  - states returns sorted alphabet
#  - stationary distribution on 2-state chain
# --------------------------------------------------------------------------

module CodingAdventures
  class TestMarkovChainConstruction < Minitest::Test
    # -----------------------------------------------------------------------
    # Test 1: Construction — empty chain has 0 states
    # -----------------------------------------------------------------------
    #
    # A freshly constructed chain should have no states and no transitions.
    # The spec mandates this as the baseline before any training.

    def test_empty_chain_has_zero_states
      chain = MarkovChain.new
      assert_equal [], chain.states
    end

    def test_empty_chain_has_empty_transition_matrix
      chain = MarkovChain.new
      assert_equal({}, chain.transition_matrix)
    end

    def test_constructor_accepts_order_parameter
      chain = MarkovChain.new(order: 2)
      assert_equal [], chain.states
    end

    def test_constructor_accepts_smoothing_parameter
      chain = MarkovChain.new(smoothing: 1.0)
      assert_equal [], chain.states
    end

    def test_constructor_preregisters_states
      # When `states:` is supplied, those states should appear in the alphabet
      # even before any training happens.
      chain = MarkovChain.new(states: %w[A B C])
      assert_equal %w[A B C], chain.states
    end
  end

  class TestMarkovChainTraining < Minitest::Test
    # -----------------------------------------------------------------------
    # Test 2: Train single pair — probability(A, B) == 1.0
    # -----------------------------------------------------------------------
    #
    # If we train on [A, B] (one transition A→B), then with no other data,
    # the probability of going from A to B must be exactly 1.0.

    def test_train_single_pair_probability_one
      chain = MarkovChain.new
      chain.train(["A", "B"])
      # The public `probability` API accepts plain state values for order-1 chains.
      assert_in_delta 1.0, chain.probability("A", "B"), 1e-10
    end

    def test_train_single_pair_registers_states
      chain = MarkovChain.new
      chain.train(["A", "B"])
      assert_includes chain.states, "A"
      assert_includes chain.states, "B"
    end

    # -----------------------------------------------------------------------
    # Test 3: Train sequence [A, B, A, C, A, B, B, A]
    # -----------------------------------------------------------------------
    #
    # Transitions in this sequence:
    #   A→B (indices 0→1), B→A (1→2), A→C (2→3), C→A (3→4),
    #   A→B (4→5), B→B (5→6), B→A (6→7)
    #
    # Counts: A→B:2, A→C:1, B→A:2, B→B:1, C→A:1
    # Normalised:
    #   P(A→B) = 2/3 ≈ 0.667
    #   P(A→C) = 1/3 ≈ 0.333
    #   P(B→A) = 2/3 ≈ 0.667
    #   P(B→B) = 1/3 ≈ 0.333
    #   P(C→A) = 1.0

    def test_train_sequence_probability_a_to_b
      chain = MarkovChain.new
      chain.train(%w[A B A C A B B A])
      assert_in_delta 2.0 / 3.0, chain.probability("A", "B"), 1e-10
    end

    def test_train_sequence_probability_a_to_c
      chain = MarkovChain.new
      chain.train(%w[A B A C A B B A])
      assert_in_delta 1.0 / 3.0, chain.probability("A", "C"), 1e-10
    end

    def test_train_sequence_probability_b_to_a
      chain = MarkovChain.new
      chain.train(%w[A B A C A B B A])
      assert_in_delta 2.0 / 3.0, chain.probability("B", "A"), 1e-10
    end

    def test_train_sequence_probability_b_to_b
      chain = MarkovChain.new
      chain.train(%w[A B A C A B B A])
      assert_in_delta 1.0 / 3.0, chain.probability("B", "B"), 1e-10
    end

    def test_train_sequence_probability_c_to_a
      chain = MarkovChain.new
      chain.train(%w[A B A C A B B A])
      assert_in_delta 1.0, chain.probability("C", "A"), 1e-10
    end

    # -----------------------------------------------------------------------
    # Test 4: Laplace smoothing with explicit alphabet
    # -----------------------------------------------------------------------
    #
    # Setup: order=1, smoothing=1.0, states=['A','B','C']
    # Train: ['A', 'B']
    #
    # From A there is 1 observed transition A→B.
    # Counts with smoothing:  A→A: 0+1=1, A→B: 1+1=2, A→C: 0+1=1
    # Total = 4
    # P(A→C) = 1/4 = 0.25

    def test_laplace_smoothing_probability_a_to_c
      chain = MarkovChain.new(order: 1, smoothing: 1.0, states: %w[A B C])
      chain.train(["A", "B"])
      assert_in_delta 0.25, chain.probability("A", "C"), 1e-10
    end

    def test_laplace_smoothing_probability_a_to_b
      chain = MarkovChain.new(order: 1, smoothing: 1.0, states: %w[A B C])
      chain.train(["A", "B"])
      # P(A→B) = (1+1)/4 = 0.5
      assert_in_delta 0.5, chain.probability("A", "B"), 1e-10
    end

    def test_laplace_smoothing_row_sums_to_one
      chain = MarkovChain.new(order: 1, smoothing: 1.0, states: %w[A B C])
      chain.train(["A", "B"])
      row_sum = chain.probability("A", "A") +
                chain.probability("A", "B") +
                chain.probability("A", "C")
      assert_in_delta 1.0, row_sum, 1e-10
    end

    # -----------------------------------------------------------------------
    # Test 10: Multi-train accumulation
    # -----------------------------------------------------------------------
    #
    # Calling `train` twice should accumulate counts before re-normalising,
    # not reset them.  After two identical sequences the probabilities must
    # remain the same (same relative frequencies), because all counts double.

    def test_multi_train_accumulation
      chain = MarkovChain.new

      # First training pass
      chain.train(%w[A B A B])
      p_first = chain.probability("A", "B")

      # Second training pass with same data — counts double, ratios unchanged
      chain.train(%w[A B A B])
      p_second = chain.probability("A", "B")

      assert_in_delta p_first, p_second, 1e-10
    end

    def test_multi_train_combining_different_sequences
      chain = MarkovChain.new

      # First pass: only A→B
      chain.train(["A", "B"])
      assert_in_delta 1.0, chain.probability("A", "B"), 1e-10

      # Second pass: only A→C
      chain.train(["A", "C"])
      # Now counts: A→B:1, A→C:1 → P(A→B) = 0.5
      assert_in_delta 0.5, chain.probability("A", "B"), 1e-10
      assert_in_delta 0.5, chain.probability("A", "C"), 1e-10
    end
  end

  class TestMarkovChainGeneration < Minitest::Test
    # -----------------------------------------------------------------------
    # Test 5: generate returns exactly `length` states
    # -----------------------------------------------------------------------
    #
    # The contract is strict: regardless of the random walk, the output array
    # must contain exactly the requested number of states.

    def test_generate_returns_exact_length
      chain = MarkovChain.new(smoothing: 1.0)
      chain.train(%w[A B A C A B B A])

      # For order-1, `generate` accepts a plain state value as the start.
      result = chain.generate("A", 10)
      assert_equal 10, result.length
    end

    def test_generate_starts_with_given_state
      chain = MarkovChain.new(smoothing: 1.0)
      chain.train(%w[A B A C A B B A])

      result = chain.generate("A", 5)
      # The first element should always be the starting state.
      assert_equal "A", result.first
    end

    def test_generate_returns_only_known_states
      chain = MarkovChain.new(smoothing: 1.0)
      chain.train(%w[A B A C A B B A])
      known = chain.states.to_set

      result = chain.generate("A", 20)
      result.each do |s|
        assert_includes known, s, "Generated state #{s.inspect} not in alphabet"
      end
    end

    # -----------------------------------------------------------------------
    # Test 6: generate_string returns correct length and starts with seed
    # -----------------------------------------------------------------------

    def test_generate_string_returns_exact_length
      chain = MarkovChain.new(smoothing: 1.0)
      chain.train_string("the quick brown fox jumps over the lazy dog " * 10)

      result = chain.generate_string("t", 50)
      assert_equal 50, result.length
    end

    def test_generate_string_starts_with_seed
      chain = MarkovChain.new(smoothing: 1.0)
      chain.train_string("the quick brown fox jumps over the lazy dog " * 10)

      result = chain.generate_string("t", 50)
      assert_equal "t", result[0]
    end

    def test_generate_string_returns_string_type
      chain = MarkovChain.new(smoothing: 1.0)
      chain.train_string("abcabc")

      result = chain.generate_string("a", 5)
      assert_instance_of String, result
    end

    # -----------------------------------------------------------------------
    # Test 8: Order-2 chain reproduces "abcabcabc"
    # -----------------------------------------------------------------------
    #
    # When we train an order-2 chain on "abcabcabc" (repeated pattern), the
    # transitions become deterministic:
    #
    #   context ["a","b"] → always "c"
    #   context ["b","c"] → always "a"
    #   context ["c","a"] → always "b"
    #
    # Starting with seed "ab" (which becomes context ["a","b"]), the chain
    # must reproduce "abcabcabc" exactly (9 chars).

    def test_order_2_deterministic_generation
      chain = MarkovChain.new(order: 2)
      chain.train_string("abcabcabc")

      result = chain.generate_string("ab", 9)
      assert_equal "abcabcabc", result
    end

    def test_order_2_probability_ab_to_c_is_one
      chain = MarkovChain.new(order: 2)
      chain.train_string("abcabcabc")

      context = %w[a b].freeze
      assert_in_delta 1.0, chain.probability(context, "c"), 1e-10
    end

    def test_order_2_probability_bc_to_a_is_one
      chain = MarkovChain.new(order: 2)
      chain.train_string("abcabcabc")

      context = %w[b c].freeze
      assert_in_delta 1.0, chain.probability(context, "a"), 1e-10
    end
  end

  class TestMarkovChainProbability < Minitest::Test
    def test_probability_returns_zero_for_unknown_from
      chain = MarkovChain.new
      chain.train(["A", "B"])
      # "X" was never seen — plain state value accepted by public API
      assert_in_delta 0.0, chain.probability("X", "A"), 1e-10
    end

    def test_probability_returns_zero_for_unknown_to
      chain = MarkovChain.new
      chain.train(["A", "B"])
      assert_in_delta 0.0, chain.probability("A", "Z"), 1e-10
    end

    def test_probability_row_sums_to_one
      chain = MarkovChain.new
      chain.train(%w[A B A C A B B A])
      row = chain.transition_matrix

      row.each do |_context, targets|
        total = targets.values.sum
        assert_in_delta 1.0, total, 1e-9
      end
    end
  end

  class TestMarkovChainStationaryDistribution < Minitest::Test
    # -----------------------------------------------------------------------
    # Test 7: Stationary distribution sums to 1
    # -----------------------------------------------------------------------
    #
    # For any ergodic chain, the stationary distribution must be a valid
    # probability distribution: all values >= 0 and summing to 1.0.

    def test_stationary_distribution_sums_to_one
      chain = MarkovChain.new(smoothing: 1.0)
      chain.train(%w[A B A C A B B A])

      dist = chain.stationary_distribution
      total = dist.values.sum
      assert_in_delta 1.0, total, 1e-6
    end

    def test_stationary_distribution_all_values_nonnegative
      chain = MarkovChain.new(smoothing: 1.0)
      chain.train(%w[A B A C A B B A])

      dist = chain.stationary_distribution
      dist.each do |state, prob|
        assert prob >= 0.0, "Negative probability #{prob} for state #{state}"
      end
    end

    def test_stationary_distribution_two_state_chain
      # Simple 2-state ergodic chain:
      # A→B with probability 1, B→A with probability 1.
      # Stationary: π[A] = 0.5, π[B] = 0.5.
      chain = MarkovChain.new
      chain.train(["A", "B", "A", "B", "A", "B"])

      dist = chain.stationary_distribution
      assert_in_delta 0.5, dist["A"], 1e-6
      assert_in_delta 0.5, dist["B"], 1e-6
    end

    def test_stationary_distribution_includes_all_states
      chain = MarkovChain.new(smoothing: 1.0)
      chain.train(%w[A B A C A B B A])

      dist = chain.stationary_distribution
      chain.states.each do |s|
        assert dist.key?(s), "Missing state #{s} in stationary distribution"
      end
    end
  end

  class TestMarkovChainErrors < Minitest::Test
    # -----------------------------------------------------------------------
    # Test 9: Unknown state raises KeyError
    # -----------------------------------------------------------------------
    #
    # Calling `next_state` on a context that was never seen in training must
    # raise a KeyError, not silently return nil or crash with a less
    # informative error.

    def test_next_state_raises_key_error_for_unknown_state
      chain = MarkovChain.new
      chain.train(["A", "B"])

      assert_raises(KeyError) { chain.next_state("UNKNOWN") }
    end

    def test_next_state_raises_key_error_with_descriptive_message
      chain = MarkovChain.new
      chain.train(["A", "B"])

      err = assert_raises(KeyError) { chain.next_state("MISSING") }
      assert_includes err.message, "MISSING"
    end
  end

  class TestMarkovChainTrainString < Minitest::Test
    def test_train_string_equivalent_to_train_chars
      chain1 = MarkovChain.new
      chain2 = MarkovChain.new

      chain1.train("abcabc".chars)
      chain2.train_string("abcabc")

      # Both chains should produce the same transition matrix.
      assert_equal chain1.transition_matrix, chain2.transition_matrix
    end

    def test_train_string_registers_char_states
      chain = MarkovChain.new
      chain.train_string("abcabc")
      assert_includes chain.states, "a"
      assert_includes chain.states, "b"
      assert_includes chain.states, "c"
    end
  end

  class TestMarkovChainTransitionMatrix < Minitest::Test
    def test_transition_matrix_returns_hash
      chain = MarkovChain.new
      chain.train(["A", "B"])
      assert_instance_of Hash, chain.transition_matrix
    end

    def test_transition_matrix_is_a_copy
      chain = MarkovChain.new
      chain.train(["A", "B"])
      m1 = chain.transition_matrix
      m1[:poison] = "should not affect chain"
      m2 = chain.transition_matrix
      refute_equal m1, m2
    end
  end
end

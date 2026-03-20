# frozen_string_literal: true

require "test_helper"
require "set"

# ---------------------------------------------------------------------------
# Tests for DFA minimization (Hopcroft's algorithm).
# ---------------------------------------------------------------------------
#
# These tests verify that:
# 1. Already-minimal DFAs are not over-reduced
# 2. Equivalent states are correctly merged
# 3. Unreachable states are removed
# 4. The minimized DFA recognizes the same language
# 5. The NFA -> DFA -> minimize pipeline works end-to-end
# ---------------------------------------------------------------------------

module CodingAdventures
  module StateMachine
    class TestMinimizeBasic < Minitest::Test
      def test_already_minimal
        dfa = DFA.new(
          states: Set["q0", "q1"],
          alphabet: Set["a", "b"],
          transitions: {
            ["q0", "a"] => "q1",
            ["q0", "b"] => "q0",
            ["q1", "a"] => "q0",
            ["q1", "b"] => "q1"
          },
          initial: "q0",
          accepting: Set["q1"]
        )
        minimized = StateMachine.minimize(dfa)
        assert_equal 2, minimized.states.size
      end

      def test_equivalent_states_merged
        # q1 and q2 are both accepting and both have the same transitions
        # (self-loop on both 'a' and 'b'). They are equivalent and should
        # be merged into one state.
        dfa = DFA.new(
          states: Set["q0", "q1", "q2"],
          alphabet: Set["a", "b"],
          transitions: {
            ["q0", "a"] => "q1",
            ["q0", "b"] => "q2",
            ["q1", "a"] => "q1",
            ["q1", "b"] => "q1",
            ["q2", "a"] => "q2",
            ["q2", "b"] => "q2"
          },
          initial: "q0",
          accepting: Set["q1", "q2"]
        )
        minimized = StateMachine.minimize(dfa)
        assert_equal 2, minimized.states.size
      end

      def test_unreachable_states_removed
        dfa = DFA.new(
          states: Set["q0", "q1", "q_dead"],
          alphabet: Set["a"],
          transitions: {
            ["q0", "a"] => "q1",
            ["q1", "a"] => "q0",
            ["q_dead", "a"] => "q_dead"
          },
          initial: "q0",
          accepting: Set["q1"]
        )
        minimized = StateMachine.minimize(dfa)
        assert_equal 2, minimized.states.size
      end

      def test_language_preserved
        dfa = DFA.new(
          states: Set["q0", "q1", "q2", "q3"],
          alphabet: Set["a", "b"],
          transitions: {
            ["q0", "a"] => "q1",
            ["q0", "b"] => "q2",
            ["q1", "a"] => "q3",
            ["q1", "b"] => "q3",
            ["q2", "a"] => "q3",
            ["q2", "b"] => "q3",
            ["q3", "a"] => "q3",
            ["q3", "b"] => "q3"
          },
          initial: "q0",
          accepting: Set["q1", "q2"]
        )
        minimized = StateMachine.minimize(dfa)

        test_inputs = [%w[a], %w[b], %w[a a], %w[a b], %w[b a], []]
        test_inputs.each do |events|
          assert_equal dfa.accepts(events), minimized.accepts(events),
            "Language mismatch on #{events}"
        end
      end

      def test_single_state
        dfa = DFA.new(
          states: Set["q0"],
          alphabet: Set["a"],
          transitions: {["q0", "a"] => "q0"},
          initial: "q0",
          accepting: Set["q0"]
        )
        minimized = StateMachine.minimize(dfa)
        assert_equal 1, minimized.states.size
        assert minimized.accepts(%w[a])
        assert minimized.accepts([])
      end
    end

    class TestMinimizeWithNFA < Minitest::Test
      # Test minimization on DFAs produced by NFA->DFA conversion.
      # Subset construction often produces bloated DFAs. Minimization should
      # shrink them back down.

      def test_nfa_to_dfa_to_minimized
        # NFA for "ends with 'a'"
        nfa = NFA.new(
          states: Set["q0", "q1"],
          alphabet: Set["a", "b"],
          transitions: {
            ["q0", "a"] => Set["q0", "q1"],
            ["q0", "b"] => Set["q0"]
          },
          initial: "q0",
          accepting: Set["q1"]
        )
        dfa = nfa.to_dfa
        minimized = StateMachine.minimize(dfa)

        # The minimal DFA for "ends with 'a'" has exactly 2 states
        assert_equal 2, minimized.states.size

        # Verify language
        assert minimized.accepts(%w[a])
        assert minimized.accepts(%w[b a])
        assert minimized.accepts(%w[a b a])
        refute minimized.accepts(%w[b])
        refute minimized.accepts(%w[a b])
        refute minimized.accepts([])
      end

      def test_minimized_preserves_language_exhaustive
        # NFA for "contains 'aa'"
        nfa = NFA.new(
          states: Set["q0", "q1", "q2"],
          alphabet: Set["a", "b"],
          transitions: {
            ["q0", "a"] => Set["q0", "q1"],
            ["q0", "b"] => Set["q0"],
            ["q1", "a"] => Set["q2"],
            ["q2", "a"] => Set["q2"],
            ["q2", "b"] => Set["q2"]
          },
          initial: "q0",
          accepting: Set["q2"]
        )
        dfa = nfa.to_dfa
        minimized = StateMachine.minimize(dfa)

        # Generate all strings up to length 3
        gen(3).each do |s|
          assert_equal nfa.accepts(s), minimized.accepts(s),
            "Mismatch on '#{s.join}'"
        end
      end

      private

      def gen(max_len)
        result = [[]]
        (1..max_len).each do
          new_strings = []
          result.each do |s|
            %w[a b].each { |c| new_strings << [*s, c] }
          end
          result.concat(new_strings)
        end
        result
      end
    end
  end
end

# frozen_string_literal: true

require "test_helper"
require "set"

# ---------------------------------------------------------------------------
# Tests for the NFA (Non-deterministic Finite Automaton) implementation.
# ---------------------------------------------------------------------------
#
# These tests cover:
# 1. Construction and validation
# 2. Epsilon closure computation
# 3. Processing events (non-deterministic branching)
# 4. Acceptance checking
# 5. Subset construction (NFA -> DFA conversion)
# 6. Visualization
# 7. Classic examples
# ---------------------------------------------------------------------------

module CodingAdventures
  module StateMachine
    # === NFA Factory Methods ===

    # NFA that accepts strings containing 'ab' as a substring.
    #
    # The NFA non-deterministically guesses where the substring starts:
    # - In q0, on 'a', go to BOTH q0 (keep scanning) and q1 (start match)
    # - In q1, on 'b', go to q2 (match complete)
    # - In q2, on anything, stay in q2 (already matched)
    def self.make_contains_ab
      NFA.new(
        states: Set["q0", "q1", "q2"],
        alphabet: Set["a", "b"],
        transitions: {
          ["q0", "a"] => Set["q0", "q1"],
          ["q0", "b"] => Set["q0"],
          ["q1", "b"] => Set["q2"],
          ["q2", "a"] => Set["q2"],
          ["q2", "b"] => Set["q2"]
        },
        initial: "q0",
        accepting: Set["q2"]
      )
    end

    # NFA with a chain of epsilon transitions: q0 --e--> q1 --e--> q2.
    # Accepts any single 'a' (q2 has the only real transition).
    def self.make_epsilon_chain
      NFA.new(
        states: Set["q0", "q1", "q2", "q3"],
        alphabet: Set["a"],
        transitions: {
          ["q0", EPSILON] => Set["q1"],
          ["q1", EPSILON] => Set["q2"],
          ["q2", "a"] => Set["q3"]
        },
        initial: "q0",
        accepting: Set["q3"]
      )
    end

    # NFA that accepts "a" or "ab" using epsilon transitions.
    #
    #   q0 --e--> q1 (path for "a")
    #   q0 --e--> q3 (path for "ab")
    #   q1 --a--> q2 (accept "a")
    #   q3 --a--> q4 --b--> q5 (accept "ab")
    def self.make_a_or_ab
      NFA.new(
        states: Set["q0", "q1", "q2", "q3", "q4", "q5"],
        alphabet: Set["a", "b"],
        transitions: {
          ["q0", EPSILON] => Set["q1", "q3"],
          ["q1", "a"] => Set["q2"],
          ["q3", "a"] => Set["q4"],
          ["q4", "b"] => Set["q5"]
        },
        initial: "q0",
        accepting: Set["q2", "q5"]
      )
    end

    # ================================================================
    # Construction Tests
    # ================================================================

    class TestNFAConstruction < Minitest::Test
      def test_valid_construction
        nfa = StateMachine.make_contains_ab
        assert_equal Set["q0", "q1", "q2"], nfa.states
        assert_equal Set["a", "b"], nfa.alphabet
        assert_equal "q0", nfa.initial
        assert_equal Set["q2"], nfa.accepting
      end

      def test_empty_states_rejected
        error = assert_raises(ArgumentError) do
          NFA.new(states: Set.new, alphabet: Set["a"], transitions: {},
            initial: "q0", accepting: Set.new)
        end
        assert_match(/non-empty/, error.message)
      end

      def test_epsilon_in_alphabet_rejected
        error = assert_raises(ArgumentError) do
          NFA.new(states: Set["q0"], alphabet: Set["a", ""], transitions: {},
            initial: "q0", accepting: Set.new)
        end
        assert_match(/epsilon/, error.message)
      end

      def test_initial_not_in_states
        error = assert_raises(ArgumentError) do
          NFA.new(states: Set["q0"], alphabet: Set["a"], transitions: {},
            initial: "q_bad", accepting: Set.new)
        end
        assert_match(/Initial/, error.message)
      end

      def test_accepting_not_subset
        error = assert_raises(ArgumentError) do
          NFA.new(states: Set["q0"], alphabet: Set["a"], transitions: {},
            initial: "q0", accepting: Set["q_bad"])
        end
        assert_match(/Accepting/, error.message)
      end

      def test_transition_source_invalid
        error = assert_raises(ArgumentError) do
          NFA.new(states: Set["q0"], alphabet: Set["a"],
            transitions: {["q_bad", "a"] => Set["q0"]},
            initial: "q0", accepting: Set.new)
        end
        assert_match(/source/, error.message)
      end

      def test_transition_event_invalid
        error = assert_raises(ArgumentError) do
          NFA.new(states: Set["q0"], alphabet: Set["a"],
            transitions: {["q0", "z"] => Set["q0"]},
            initial: "q0", accepting: Set.new)
        end
        assert_match(/alphabet/, error.message)
      end

      def test_transition_target_invalid
        error = assert_raises(ArgumentError) do
          NFA.new(states: Set["q0"], alphabet: Set["a"],
            transitions: {["q0", "a"] => Set["q_bad"]},
            initial: "q0", accepting: Set.new)
        end
        assert_match(/targets/, error.message)
      end
    end

    # ================================================================
    # Epsilon Closure Tests
    # ================================================================

    class TestEpsilonClosure < Minitest::Test
      def test_no_epsilon_transitions
        nfa = StateMachine.make_contains_ab
        assert_equal Set["q0"], nfa.epsilon_closure(Set["q0"])
      end

      def test_single_epsilon
        nfa = NFA.new(
          states: Set["q0", "q1"],
          alphabet: Set["a"],
          transitions: {["q0", EPSILON] => Set["q1"]},
          initial: "q0",
          accepting: Set.new
        )
        assert_equal Set["q0", "q1"], nfa.epsilon_closure(Set["q0"])
      end

      def test_chained_epsilons
        nfa = StateMachine.make_epsilon_chain
        assert_equal Set["q0", "q1", "q2"], nfa.epsilon_closure(Set["q0"])
      end

      def test_epsilon_cycle
        nfa = NFA.new(
          states: Set["q0", "q1"],
          alphabet: Set["a"],
          transitions: {
            ["q0", EPSILON] => Set["q1"],
            ["q1", EPSILON] => Set["q0"]
          },
          initial: "q0",
          accepting: Set.new
        )
        assert_equal Set["q0", "q1"], nfa.epsilon_closure(Set["q0"])
      end

      def test_branching_epsilons
        nfa = StateMachine.make_a_or_ab
        assert_equal Set["q0", "q1", "q3"], nfa.epsilon_closure(Set["q0"])
      end

      def test_closure_of_multiple_states
        nfa = StateMachine.make_epsilon_chain
        result = nfa.epsilon_closure(Set["q0", "q3"])
        assert_equal Set["q0", "q1", "q2", "q3"], result
      end

      def test_empty_set_closure
        nfa = StateMachine.make_epsilon_chain
        assert_equal Set.new, nfa.epsilon_closure(Set.new)
      end
    end

    # ================================================================
    # Processing Tests
    # ================================================================

    class TestNFAProcessing < Minitest::Test
      def test_initial_states_include_epsilon_closure
        nfa = StateMachine.make_epsilon_chain
        assert_equal Set["q0", "q1", "q2"], nfa.current_states
      end

      def test_process_deterministic_case
        nfa = StateMachine.make_contains_ab
        nfa.process("b")
        assert_equal Set["q0"], nfa.current_states
      end

      def test_process_non_deterministic
        nfa = StateMachine.make_contains_ab
        nfa.process("a")
        assert_equal Set["q0", "q1"], nfa.current_states
      end

      def test_process_dead_paths_vanish
        nfa = StateMachine.make_contains_ab
        nfa.process("a")  # {q0, q1}
        nfa.process("a")  # q0->{q0,q1}, q1 has no 'a' -> dies
        assert_equal Set["q0", "q1"], nfa.current_states
      end

      def test_process_reaches_accepting
        nfa = StateMachine.make_contains_ab
        nfa.process("a")
        nfa.process("b")
        assert_includes nfa.current_states, "q2"
      end

      def test_process_through_epsilon
        nfa = StateMachine.make_epsilon_chain
        nfa.process("a")
        assert_equal Set["q3"], nfa.current_states
      end

      def test_process_invalid_event
        nfa = StateMachine.make_contains_ab
        assert_raises(ArgumentError) { nfa.process("c") }
      end

      def test_process_sequence
        nfa = StateMachine.make_contains_ab
        trace = nfa.process_sequence(%w[a b])
        assert_equal 2, trace.length
        before, event, after = trace[0]
        assert_equal "a", event
        assert_includes before, "q0"
        assert_includes after, "q1"
        _, event2, after2 = trace[1]
        assert_equal "b", event2
        assert_includes after2, "q2"
      end
    end

    # ================================================================
    # Acceptance Tests
    # ================================================================

    class TestNFAAcceptance < Minitest::Test
      def test_contains_ab_accepts
        nfa = StateMachine.make_contains_ab
        assert nfa.accepts(%w[a b])
        assert nfa.accepts(%w[b a b])
        assert nfa.accepts(%w[a a b])
        assert nfa.accepts(%w[a b a b])
      end

      def test_contains_ab_rejects
        nfa = StateMachine.make_contains_ab
        refute nfa.accepts(%w[a])
        refute nfa.accepts(%w[b])
        refute nfa.accepts(%w[b a])
        refute nfa.accepts(%w[b b b])
        refute nfa.accepts([])
      end

      def test_a_or_ab_accepts
        nfa = StateMachine.make_a_or_ab
        assert nfa.accepts(%w[a])
        assert nfa.accepts(%w[a b])
      end

      def test_a_or_ab_rejects
        nfa = StateMachine.make_a_or_ab
        refute nfa.accepts([])
        refute nfa.accepts(%w[b])
        refute nfa.accepts(%w[a a])
        refute nfa.accepts(%w[a b a])
      end

      def test_epsilon_chain_accepts
        nfa = StateMachine.make_epsilon_chain
        assert nfa.accepts(%w[a])
      end

      def test_epsilon_chain_rejects
        nfa = StateMachine.make_epsilon_chain
        refute nfa.accepts([])
        refute nfa.accepts(%w[a a])
      end

      def test_accepts_does_not_modify_state
        nfa = StateMachine.make_contains_ab
        original = nfa.current_states
        nfa.accepts(%w[a b a])
        assert_equal original, nfa.current_states
      end

      def test_accepts_invalid_event
        nfa = StateMachine.make_contains_ab
        assert_raises(ArgumentError) { nfa.accepts(%w[c]) }
      end

      def test_early_rejection
        nfa = NFA.new(
          states: Set["q0", "q1"],
          alphabet: Set["a", "b"],
          transitions: {["q0", "a"] => Set["q1"]},
          initial: "q0",
          accepting: Set["q1"]
        )
        refute nfa.accepts(%w[b])
        refute nfa.accepts(%w[b a])
      end
    end

    # ================================================================
    # Subset Construction Tests (NFA -> DFA)
    # ================================================================

    class TestSubsetConstruction < Minitest::Test
      def test_deterministic_nfa_converts
        nfa = NFA.new(
          states: Set["q0", "q1"],
          alphabet: Set["a", "b"],
          transitions: {
            ["q0", "a"] => Set["q1"],
            ["q0", "b"] => Set["q0"],
            ["q1", "a"] => Set["q0"],
            ["q1", "b"] => Set["q1"]
          },
          initial: "q0",
          accepting: Set["q1"]
        )
        dfa = nfa.to_dfa
        assert_equal 2, dfa.states.size
        assert dfa.accepts(%w[a])
        refute dfa.accepts(%w[a a])
        assert dfa.accepts(%w[a b])
      end

      def test_contains_ab_converts
        nfa = StateMachine.make_contains_ab
        dfa = nfa.to_dfa

        test_cases = [
          [%w[a b], true],
          [%w[b a b], true],
          [%w[a a b], true],
          [%w[a], false],
          [%w[b], false],
          [%w[b a], false],
          [[], false]
        ]
        test_cases.each do |events, expected|
          assert_equal expected, dfa.accepts(events),
            "DFA disagrees on #{events}: expected #{expected}"
        end
      end

      def test_epsilon_nfa_converts
        nfa = StateMachine.make_a_or_ab
        dfa = nfa.to_dfa

        assert dfa.accepts(%w[a])
        assert dfa.accepts(%w[a b])
        refute dfa.accepts([])
        refute dfa.accepts(%w[b])
        refute dfa.accepts(%w[a a])
      end

      def test_epsilon_chain_converts
        nfa = StateMachine.make_epsilon_chain
        dfa = nfa.to_dfa

        assert dfa.accepts(%w[a])
        refute dfa.accepts([])
        refute dfa.accepts(%w[a a])
      end

      def test_converted_dfa_is_valid
        nfa = StateMachine.make_contains_ab
        dfa = nfa.to_dfa
        warnings = dfa.validate
        warnings.each { |w| refute_includes w, "Unreachable" }
      end

      def test_comprehensive_language_equivalence
        # NFA for "ends with 'ab'"
        nfa = NFA.new(
          states: Set["q0", "q1", "q2"],
          alphabet: Set["a", "b"],
          transitions: {
            ["q0", "a"] => Set["q0", "q1"],
            ["q0", "b"] => Set["q0"],
            ["q1", "b"] => Set["q2"]
          },
          initial: "q0",
          accepting: Set["q2"]
        )
        dfa = nfa.to_dfa

        # Generate all strings of {a,b} up to length 4
        gen_strings(%w[a b], 4).each do |s|
          nfa_result = nfa.accepts(s)
          dfa_result = dfa.accepts(s)
          assert_equal nfa_result, dfa_result,
            "Disagreement on '#{s.join}': NFA=#{nfa_result}, DFA=#{dfa_result}"
        end
      end

      private

      def gen_strings(alpha, max_len)
        result = [[]]
        (1..max_len).each do |length|
          gen_strings_of_length(alpha, length).each { |s| result << s }
        end
        result
      end

      def gen_strings_of_length(alpha, length)
        return [[]] if length == 0
        result = []
        gen_strings_of_length(alpha, length - 1).each do |s|
          alpha.each { |c| result << [*s, c] }
        end
        result
      end
    end

    # ================================================================
    # Reset Tests
    # ================================================================

    class TestNFAReset < Minitest::Test
      def test_reset_returns_to_initial
        nfa = StateMachine.make_contains_ab
        nfa.process("a")
        assert_includes nfa.current_states, "q1"

        nfa.reset
        assert_equal Set["q0"], nfa.current_states
      end

      def test_reset_with_epsilon
        nfa = StateMachine.make_epsilon_chain
        nfa.process("a")
        assert_equal Set["q3"], nfa.current_states

        nfa.reset
        assert_equal Set["q0", "q1", "q2"], nfa.current_states
      end
    end

    # ================================================================
    # Visualization Tests
    # ================================================================

    class TestNFAVisualization < Minitest::Test
      def test_to_dot_structure
        nfa = StateMachine.make_contains_ab
        dot = nfa.to_dot
        assert_includes dot, "digraph NFA"
        assert_includes dot, "__start"
        assert_includes dot, "doublecircle"
        assert_includes dot, "q0"
        assert_includes dot, "q1"
        assert_includes dot, "q2"
      end

      def test_to_dot_epsilon_label
        nfa = StateMachine.make_epsilon_chain
        dot = nfa.to_dot
        assert_includes dot, "\u03B5"
      end

      def test_repr
        nfa = StateMachine.make_contains_ab
        r = nfa.inspect
        assert_includes r, "NFA"
        assert_includes r, "q0"
      end
    end
  end
end

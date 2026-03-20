# frozen_string_literal: true

require "test_helper"
require "set"

# ---------------------------------------------------------------------------
# Tests for the Pushdown Automaton (PDA) implementation.
# ---------------------------------------------------------------------------
#
# These tests cover:
# 1. Construction and validation
# 2. Balanced parentheses recognition (10 cases)
# 3. a^n b^n language recognition (10 cases)
# 4. Processing and trace inspection
# 5. Stack inspection
# 6. Reset behavior
# 7. Non-mutating accepts()
# ---------------------------------------------------------------------------

module CodingAdventures
  module StateMachine
    # === PDA Factory Methods ===

    # PDA that accepts balanced parentheses: (), (()), ((())), ()(), etc.
    #
    # Strategy:
    # - On '(': push '(' onto the stack above whatever is there
    # - On ')': pop '(' from the stack (matching an open paren)
    # - At end of input: epsilon-transition to accept if only '$' remains
    #
    # Stack diagram for "(())":
    #   Start:  [$]
    #   Read (: [$, (]
    #   Read (: [$, (, (]
    #   Read ): [$, (]
    #   Read ): [$]
    #   Epsilon: [] -> accept
    def self.make_balanced_parens
      PushdownAutomaton.new(
        states: Set["q0", "accept"],
        input_alphabet: Set["(", ")"],
        stack_alphabet: Set["(", "$"],
        transitions: [
          PDATransition.new("q0", "(", "$", "q0", ["$", "("]),
          PDATransition.new("q0", "(", "(", "q0", ["(", "("]),
          PDATransition.new("q0", ")", "(", "q0", []),
          PDATransition.new("q0", nil, "$", "accept", [])
        ],
        initial: "q0",
        initial_stack_symbol: "$",
        accepting: Set["accept"]
      )
    end

    # PDA that accepts a^n b^n: ab, aabb, aaabbb, etc.
    #
    # Strategy:
    # - Push 'a' for each 'a' read (pushing phase)
    # - Pop 'a' for each 'b' read (popping phase)
    # - Accept when stack has only '$' after all b's
    #
    # Stack diagram for "aabb":
    #   Start:   [$]
    #   Read a:  [$, a]
    #   Read a:  [$, a, a]
    #   Read b:  [$, a]      (popped one 'a')
    #   Read b:  [$]         (popped one 'a')
    #   Epsilon: [] -> accept
    def self.make_anbn
      PushdownAutomaton.new(
        states: Set["pushing", "popping", "accept"],
        input_alphabet: Set["a", "b"],
        stack_alphabet: Set["a", "$"],
        transitions: [
          PDATransition.new("pushing", "a", "$", "pushing", ["$", "a"]),
          PDATransition.new("pushing", "a", "a", "pushing", ["a", "a"]),
          PDATransition.new("pushing", "b", "a", "popping", []),
          PDATransition.new("popping", "b", "a", "popping", []),
          PDATransition.new("popping", nil, "$", "accept", [])
        ],
        initial: "pushing",
        initial_stack_symbol: "$",
        accepting: Set["accept"]
      )
    end

    # ================================================================
    # Construction Tests
    # ================================================================

    class TestPDAConstruction < Minitest::Test
      def test_valid_construction
        pda = StateMachine.make_balanced_parens
        assert_equal "q0", pda.current_state
        assert_equal ["$"], pda.stack
      end

      def test_empty_states_rejected
        error = assert_raises(ArgumentError) do
          PushdownAutomaton.new(
            states: Set.new, input_alphabet: Set.new, stack_alphabet: Set["$"],
            transitions: [], initial: "q0", initial_stack_symbol: "$",
            accepting: Set.new
          )
        end
        assert_match(/non-empty/, error.message)
      end

      def test_initial_not_in_states
        error = assert_raises(ArgumentError) do
          PushdownAutomaton.new(
            states: Set["q0"], input_alphabet: Set.new, stack_alphabet: Set["$"],
            transitions: [], initial: "q_bad", initial_stack_symbol: "$",
            accepting: Set.new
          )
        end
        assert_match(/Initial/, error.message)
      end

      def test_initial_stack_not_in_alphabet
        error = assert_raises(ArgumentError) do
          PushdownAutomaton.new(
            states: Set["q0"], input_alphabet: Set.new, stack_alphabet: Set["$"],
            transitions: [], initial: "q0", initial_stack_symbol: "X",
            accepting: Set.new
          )
        end
        assert_match(/stack symbol/, error.message)
      end

      def test_duplicate_transitions_rejected
        error = assert_raises(ArgumentError) do
          PushdownAutomaton.new(
            states: Set["q0", "q1"],
            input_alphabet: Set["a"],
            stack_alphabet: Set["$"],
            transitions: [
              PDATransition.new("q0", "a", "$", "q0", ["$"]),
              PDATransition.new("q0", "a", "$", "q1", ["$"])
            ],
            initial: "q0",
            initial_stack_symbol: "$",
            accepting: Set.new
          )
        end
        assert_match(/Duplicate/, error.message)
      end

      def test_accepting_not_subset
        error = assert_raises(ArgumentError) do
          PushdownAutomaton.new(
            states: Set["q0"],
            input_alphabet: Set.new,
            stack_alphabet: Set["$"],
            transitions: [],
            initial: "q0",
            initial_stack_symbol: "$",
            accepting: Set["q_bad"]
          )
        end
        assert_match(/Accepting/, error.message)
      end
    end

    # ================================================================
    # Balanced Parentheses Tests
    # ================================================================

    class TestBalancedParens < Minitest::Test
      def test_simple_pair
        pda = StateMachine.make_balanced_parens
        assert pda.accepts(["(", ")"])
      end

      def test_nested
        pda = StateMachine.make_balanced_parens
        assert pda.accepts(["(", "(", ")", ")"])
      end

      def test_triple_nested
        pda = StateMachine.make_balanced_parens
        assert pda.accepts(["(", "(", "(", ")", ")", ")"])
      end

      def test_sequential
        pda = StateMachine.make_balanced_parens
        assert pda.accepts(["(", ")", "(", ")"])
      end

      def test_empty_accepted
        pda = StateMachine.make_balanced_parens
        assert pda.accepts([])
      end

      def test_unmatched_open
        pda = StateMachine.make_balanced_parens
        refute pda.accepts(["(", "(", "("])
      end

      def test_unmatched_close
        pda = StateMachine.make_balanced_parens
        refute pda.accepts([")"])
      end

      def test_wrong_order
        pda = StateMachine.make_balanced_parens
        refute pda.accepts([")", "("])
      end

      def test_partial_match
        pda = StateMachine.make_balanced_parens
        refute pda.accepts(["(", "(", ")"])
      end

      def test_extra_close
        pda = StateMachine.make_balanced_parens
        refute pda.accepts(["(", ")", ")"])
      end
    end

    # ================================================================
    # a^n b^n Tests
    # ================================================================

    class TestAnBn < Minitest::Test
      def test_ab
        pda = StateMachine.make_anbn
        assert pda.accepts(%w[a b])
      end

      def test_aabb
        pda = StateMachine.make_anbn
        assert pda.accepts(%w[a a b b])
      end

      def test_aaabbb
        pda = StateMachine.make_anbn
        assert pda.accepts(%w[a a a b b b])
      end

      def test_empty_rejected
        pda = StateMachine.make_anbn
        refute pda.accepts([])
      end

      def test_a_only
        pda = StateMachine.make_anbn
        refute pda.accepts(%w[a a a])
      end

      def test_b_only
        pda = StateMachine.make_anbn
        refute pda.accepts(%w[b b b])
      end

      def test_more_as
        pda = StateMachine.make_anbn
        refute pda.accepts(%w[a a b])
      end

      def test_more_bs
        pda = StateMachine.make_anbn
        refute pda.accepts(%w[a b b])
      end

      def test_interleaved
        pda = StateMachine.make_anbn
        refute pda.accepts(%w[a b a b])
      end

      def test_ba
        pda = StateMachine.make_anbn
        refute pda.accepts(%w[b a])
      end
    end

    # ================================================================
    # Processing and Trace Tests
    # ================================================================

    class TestPDAProcessing < Minitest::Test
      def test_process_single
        pda = StateMachine.make_balanced_parens
        pda.process("(")
        assert_equal "q0", pda.current_state
        assert_equal "(", pda.stack_top
      end

      def test_process_sequence_trace
        pda = StateMachine.make_balanced_parens
        trace = pda.process_sequence(["(", ")"])
        assert trace.length >= 2
        assert_equal "(", trace[0].event
        assert_equal "q0", trace[0].source
        assert_equal ")", trace[1].event
      end

      def test_process_no_transition
        pda = PushdownAutomaton.new(
          states: Set["q0"],
          input_alphabet: Set["a"],
          stack_alphabet: Set["$"],
          transitions: [],
          initial: "q0",
          initial_stack_symbol: "$",
          accepting: Set.new
        )
        assert_raises(ArgumentError) { pda.process("a") }
      end

      def test_stack_inspection
        pda = StateMachine.make_balanced_parens
        pda.process("(")
        assert_equal ["$", "("], pda.stack
        assert_equal "(", pda.stack_top

        pda.process("(")
        assert_equal ["$", "(", "("], pda.stack
        assert_equal "(", pda.stack_top

        pda.process(")")
        assert_equal ["$", "("], pda.stack

        pda.process(")")
        assert_equal ["$"], pda.stack
      end
    end

    # ================================================================
    # Reset Tests
    # ================================================================

    class TestPDAReset < Minitest::Test
      def test_reset
        pda = StateMachine.make_balanced_parens
        pda.process("(")
        pda.process("(")
        assert_equal "(", pda.stack_top

        pda.reset
        assert_equal "q0", pda.current_state
        assert_equal ["$"], pda.stack
        assert_equal [], pda.trace
      end
    end

    # ================================================================
    # Accepts Non-Mutating Tests
    # ================================================================

    class TestPDAAcceptsNonMutating < Minitest::Test
      def test_accepts_does_not_modify
        pda = StateMachine.make_balanced_parens
        pda.process("(")
        original_state = pda.current_state
        original_stack = pda.stack

        pda.accepts([")", "(", ")"])

        assert_equal original_state, pda.current_state
        assert_equal original_stack, pda.stack
      end
    end

    # ================================================================
    # Repr Tests
    # ================================================================

    class TestPDARepr < Minitest::Test
      def test_repr
        pda = StateMachine.make_balanced_parens
        r = pda.inspect
        assert_includes r, "PDA"
        assert_includes r, "q0"
      end
    end
  end
end

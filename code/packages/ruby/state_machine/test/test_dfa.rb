# frozen_string_literal: true

require "test_helper"
require "set"

# ---------------------------------------------------------------------------
# Tests for the DFA (Deterministic Finite Automaton) implementation.
# ---------------------------------------------------------------------------
#
# These tests cover:
# 1. Construction and validation
# 2. Processing single events and sequences
# 3. Acceptance checking
# 4. Introspection (reachability, completeness, validation)
# 5. Visualization (DOT and ASCII output)
# 6. Classic examples (turnstile, binary div-by-3, branch predictor)
# 7. Error cases and edge cases
# ---------------------------------------------------------------------------

module CodingAdventures
  module StateMachine
    # === Helper Methods ===
    #
    # These factory methods create reusable DFA instances for testing.
    # They serve the same role as pytest fixtures.

    # The classic turnstile: insert coin to unlock, push to lock.
    #
    # This is the simplest non-trivial DFA -- two states, two inputs,
    # four transitions. It's the "hello world" of state machines.
    def self.make_turnstile
      DFA.new(
        states: Set["locked", "unlocked"],
        alphabet: Set["coin", "push"],
        transitions: {
          ["locked", "coin"] => "unlocked",
          ["locked", "push"] => "locked",
          ["unlocked", "coin"] => "unlocked",
          ["unlocked", "push"] => "locked"
        },
        initial: "locked",
        accepting: Set["unlocked"]
      )
    end

    # DFA that accepts binary strings representing numbers divisible by 3.
    #
    # This is a classic automata theory example. States represent the
    # current remainder when divided by 3:
    #
    #     r0 = remainder 0 (divisible by 3) -- accepting state
    #     r1 = remainder 1
    #     r2 = remainder 2
    #
    # Transition logic:
    #     When we read the next bit, the number so far doubles (shift left)
    #     and adds the new bit:
    #         new_value = old_value * 2 + bit
    #         new_remainder = (old_remainder * 2 + bit) mod 3
    #
    #     Truth table for transitions:
    #     State | Input | Calculation           | New State
    #     ------+-------+-----------------------+----------
    #     r0    | 0     | (0*2 + 0) mod 3 = 0   | r0
    #     r0    | 1     | (0*2 + 1) mod 3 = 1   | r1
    #     r1    | 0     | (1*2 + 0) mod 3 = 2   | r2
    #     r1    | 1     | (1*2 + 1) mod 3 = 0   | r0
    #     r2    | 0     | (2*2 + 0) mod 3 = 1   | r1
    #     r2    | 1     | (2*2 + 1) mod 3 = 2   | r2
    def self.make_div_by_3
      DFA.new(
        states: Set["r0", "r1", "r2"],
        alphabet: Set["0", "1"],
        transitions: {
          ["r0", "0"] => "r0",
          ["r0", "1"] => "r1",
          ["r1", "0"] => "r2",
          ["r1", "1"] => "r0",
          ["r2", "0"] => "r1",
          ["r2", "1"] => "r2"
        },
        initial: "r0",
        accepting: Set["r0"]
      )
    end

    # 2-bit saturating counter branch predictor as a DFA.
    #
    # States: SNT (strongly not-taken), WNT (weakly not-taken),
    #         WT (weakly taken), ST (strongly taken)
    #
    # This is equivalent to the TwoBitState in the branch-predictor package.
    # The branch predictor predicts "taken" when in WT or ST (the accepting
    # states), and "not taken" when in SNT or WNT.
    #
    #     State diagram:
    #
    #       SNT <--not_taken-- WNT <--not_taken-- WT <--not_taken-- ST
    #        |                  |                  |                  |
    #        +---taken--> WNT  +---taken--> WT    +---taken--> ST   +---taken--> ST
    #        |                                                       ^
    #        +-- not_taken --> SNT (self-loop)         taken --+-----+
    def self.make_branch_predictor
      DFA.new(
        states: Set["SNT", "WNT", "WT", "ST"],
        alphabet: Set["taken", "not_taken"],
        transitions: {
          ["SNT", "taken"] => "WNT",
          ["SNT", "not_taken"] => "SNT",
          ["WNT", "taken"] => "WT",
          ["WNT", "not_taken"] => "SNT",
          ["WT", "taken"] => "ST",
          ["WT", "not_taken"] => "WNT",
          ["ST", "taken"] => "ST",
          ["ST", "not_taken"] => "WT"
        },
        initial: "WNT",
        accepting: Set["WT", "ST"]
      )
    end

    # ================================================================
    # Construction and Validation Tests
    # ================================================================

    class TestDFAConstruction < Minitest::Test
      def test_valid_construction
        dfa = StateMachine.make_turnstile
        assert_equal "locked", dfa.current_state
        assert_equal "locked", dfa.initial
        assert_equal Set["locked", "unlocked"], dfa.states
        assert_equal Set["coin", "push"], dfa.alphabet
        assert_equal Set["unlocked"], dfa.accepting
      end

      def test_empty_states_rejected
        error = assert_raises(ArgumentError) do
          DFA.new(
            states: Set.new,
            alphabet: Set["a"],
            transitions: {},
            initial: "q0",
            accepting: Set.new
          )
        end
        assert_match(/non-empty/, error.message)
      end

      def test_initial_not_in_states
        error = assert_raises(ArgumentError) do
          DFA.new(
            states: Set["q0", "q1"],
            alphabet: Set["a"],
            transitions: {["q0", "a"] => "q1"},
            initial: "q_missing",
            accepting: Set.new
          )
        end
        assert_match(/Initial state/, error.message)
      end

      def test_accepting_not_subset_of_states
        error = assert_raises(ArgumentError) do
          DFA.new(
            states: Set["q0", "q1"],
            alphabet: Set["a"],
            transitions: {["q0", "a"] => "q1"},
            initial: "q0",
            accepting: Set["q_missing"]
          )
        end
        assert_match(/Accepting states/, error.message)
      end

      def test_transition_source_not_in_states
        error = assert_raises(ArgumentError) do
          DFA.new(
            states: Set["q0"],
            alphabet: Set["a"],
            transitions: {["q_bad", "a"] => "q0"},
            initial: "q0",
            accepting: Set.new
          )
        end
        assert_match(/source/, error.message)
      end

      def test_transition_event_not_in_alphabet
        error = assert_raises(ArgumentError) do
          DFA.new(
            states: Set["q0"],
            alphabet: Set["a"],
            transitions: {["q0", "b"] => "q0"},
            initial: "q0",
            accepting: Set.new
          )
        end
        assert_match(/alphabet/, error.message)
      end

      def test_transition_target_not_in_states
        error = assert_raises(ArgumentError) do
          DFA.new(
            states: Set["q0"],
            alphabet: Set["a"],
            transitions: {["q0", "a"] => "q_bad"},
            initial: "q0",
            accepting: Set.new
          )
        end
        assert_match(/target/, error.message)
      end

      def test_action_without_transition
        error = assert_raises(ArgumentError) do
          DFA.new(
            states: Set["q0"],
            alphabet: Set["a"],
            transitions: {["q0", "a"] => "q0"},
            initial: "q0",
            accepting: Set.new,
            actions: {["q0", "b"] => ->(s, e, t) {}}
          )
        end
        assert_match(/no transition/, error.message)
      end

      def test_empty_accepting_set
        dfa = DFA.new(
          states: Set["q0"],
          alphabet: Set["a"],
          transitions: {["q0", "a"] => "q0"},
          initial: "q0",
          accepting: Set.new
        )
        assert_equal Set.new, dfa.accepting
      end

      def test_transitions_property_returns_copy
        dfa = StateMachine.make_turnstile
        t1 = dfa.transitions
        t2 = dfa.transitions
        assert_equal t1, t2
        refute_same t1, t2
      end
    end

    # ================================================================
    # Processing Tests
    # ================================================================

    class TestDFAProcessing < Minitest::Test
      def test_process_single_event
        dfa = StateMachine.make_turnstile
        result = dfa.process("coin")
        assert_equal "unlocked", result
        assert_equal "unlocked", dfa.current_state
      end

      def test_process_multiple_events
        dfa = StateMachine.make_turnstile
        dfa.process("coin")
        assert_equal "unlocked", dfa.current_state
        dfa.process("push")
        assert_equal "locked", dfa.current_state
        dfa.process("coin")
        assert_equal "unlocked", dfa.current_state
        dfa.process("coin")
        assert_equal "unlocked", dfa.current_state
      end

      def test_process_builds_trace
        dfa = StateMachine.make_turnstile
        dfa.process("coin")
        dfa.process("push")

        trace = dfa.trace
        assert_equal 2, trace.length
        assert_equal TransitionRecord.new("locked", "coin", "unlocked", nil), trace[0]
        assert_equal TransitionRecord.new("unlocked", "push", "locked", nil), trace[1]
      end

      def test_process_sequence
        dfa = StateMachine.make_turnstile
        trace = dfa.process_sequence(%w[coin push coin])
        assert_equal 3, trace.length
        assert_equal "locked", trace[0].source
        assert_equal "unlocked", trace[0].target
        assert_equal "unlocked", trace[1].source
        assert_equal "locked", trace[1].target
        assert_equal "locked", trace[2].source
        assert_equal "unlocked", trace[2].target
      end

      def test_process_sequence_empty
        dfa = StateMachine.make_turnstile
        trace = dfa.process_sequence([])
        assert_equal [], trace
        assert_equal "locked", dfa.current_state
      end

      def test_process_invalid_event
        dfa = StateMachine.make_turnstile
        error = assert_raises(ArgumentError) { dfa.process("kick") }
        assert_match(/not in the alphabet/, error.message)
      end

      def test_process_undefined_transition
        dfa = DFA.new(
          states: Set["q0", "q1"],
          alphabet: Set["a", "b"],
          transitions: {["q0", "a"] => "q1"},
          initial: "q0",
          accepting: Set.new
        )
        error = assert_raises(ArgumentError) { dfa.process("b") }
        assert_match(/No transition/, error.message)
      end

      def test_self_loop
        dfa = DFA.new(
          states: Set["q0"],
          alphabet: Set["a"],
          transitions: {["q0", "a"] => "q0"},
          initial: "q0",
          accepting: Set["q0"]
        )
        dfa.process("a")
        assert_equal "q0", dfa.current_state
        dfa.process("a")
        assert_equal "q0", dfa.current_state
      end

      def test_actions_fire
        log = []
        logger = ->(source, event, target) { log << [source, event, target] }

        dfa = DFA.new(
          states: Set["a", "b"],
          alphabet: Set["x"],
          transitions: {["a", "x"] => "b", ["b", "x"] => "a"},
          initial: "a",
          accepting: Set.new,
          actions: {["a", "x"] => logger}
        )
        dfa.process("x")
        assert_equal [["a", "x", "b"]], log
        dfa.process("x")
        assert_equal 1, log.length # action only on (a, x), not (b, x)
      end

      def test_action_name_in_trace
        my_action = ->(source, event, target) {}
        # Define a named method instead for name extraction
        dfa = DFA.new(
          states: Set["a", "b"],
          alphabet: Set["x"],
          transitions: {["a", "x"] => "b", ["b", "x"] => "a"},
          initial: "a",
          accepting: Set.new,
          actions: {["a", "x"] => my_action}
        )
        dfa.process("x")
        refute_nil dfa.trace[0].action_name
      end
    end

    # ================================================================
    # Acceptance Tests
    # ================================================================

    class TestDFAAcceptance < Minitest::Test
      def test_accepts_basic
        dfa = StateMachine.make_turnstile
        assert dfa.accepts(%w[coin])
        refute dfa.accepts(%w[coin push])
        assert dfa.accepts(%w[coin push coin])
      end

      def test_accepts_empty_input
        turnstile = StateMachine.make_turnstile
        refute turnstile.accepts([]) # locked is not accepting

        # DFA where initial IS accepting
        dfa = DFA.new(
          states: Set["q0"],
          alphabet: Set["a"],
          transitions: {["q0", "a"] => "q0"},
          initial: "q0",
          accepting: Set["q0"]
        )
        assert dfa.accepts([])
      end

      def test_accepts_does_not_modify_state
        dfa = StateMachine.make_turnstile
        dfa.process("coin")
        assert_equal "unlocked", dfa.current_state

        dfa.accepts(%w[push push push])
        assert_equal "unlocked", dfa.current_state # unchanged
      end

      def test_accepts_does_not_modify_trace
        dfa = StateMachine.make_turnstile
        dfa.process("coin")
        trace_len = dfa.trace.length

        dfa.accepts(%w[push coin])
        assert_equal trace_len, dfa.trace.length # unchanged
      end

      def test_accepts_undefined_transition
        dfa = DFA.new(
          states: Set["q0", "q1"],
          alphabet: Set["a", "b"],
          transitions: {["q0", "a"] => "q1"},
          initial: "q0",
          accepting: Set["q1"]
        )
        assert dfa.accepts(%w[a])
        refute dfa.accepts(%w[b]) # no transition, graceful reject
      end

      def test_accepts_invalid_event
        dfa = StateMachine.make_turnstile
        assert_raises(ArgumentError) { dfa.accepts(%w[kick]) }
      end

      def test_div_by_3
        dfa = StateMachine.make_div_by_3

        # 0 = 0 (div by 3) -- empty string starts in r0 which is accepting
        assert dfa.accepts([])

        # 1 = 1 (not div by 3)
        refute dfa.accepts(%w[1])

        # 10 = 2 (not div by 3)
        refute dfa.accepts(%w[1 0])

        # 11 = 3 (div by 3)
        assert dfa.accepts(%w[1 1])

        # 100 = 4 (not div by 3)
        refute dfa.accepts(%w[1 0 0])

        # 110 = 6 (div by 3)
        assert dfa.accepts(%w[1 1 0])

        # 1001 = 9 (div by 3)
        assert dfa.accepts(%w[1 0 0 1])

        # 1100 = 12 (div by 3)
        assert dfa.accepts(%w[1 1 0 0])

        # 1111 = 15 (div by 3)
        assert dfa.accepts(%w[1 1 1 1])

        # 10000 = 16 (not div by 3)
        refute dfa.accepts(%w[1 0 0 0 0])
      end
    end

    # ================================================================
    # Branch Predictor as DFA Tests
    # ================================================================

    class TestBranchPredictorDFA < Minitest::Test
      def test_initial_state
        bp = StateMachine.make_branch_predictor
        assert_equal "WNT", bp.current_state
      end

      def test_warmup_to_strongly_taken
        bp = StateMachine.make_branch_predictor
        bp.process("taken")
        assert_equal "WT", bp.current_state
        bp.process("taken")
        assert_equal "ST", bp.current_state
      end

      def test_saturation_at_st
        bp = StateMachine.make_branch_predictor
        bp.process_sequence(%w[taken taken taken taken])
        assert_equal "ST", bp.current_state
      end

      def test_saturation_at_snt
        bp = StateMachine.make_branch_predictor
        bp.process_sequence(%w[not_taken not_taken not_taken])
        assert_equal "SNT", bp.current_state
      end

      def test_hysteresis
        bp = StateMachine.make_branch_predictor
        bp.process_sequence(%w[taken taken])
        assert_equal "ST", bp.current_state

        bp.process("not_taken")
        assert_equal "WT", bp.current_state
        assert_includes bp.accepting, "WT" # still predicts taken
      end

      def test_loop_pattern
        bp = StateMachine.make_branch_predictor
        pattern = Array.new(9, "taken") + ["not_taken"]
        bp.process_sequence(pattern)
        assert_equal "WT", bp.current_state
        assert_includes bp.accepting, bp.current_state
      end

      def test_prediction_via_accepting
        bp = StateMachine.make_branch_predictor
        refute_includes bp.accepting, bp.current_state # WNT not accepting

        bp.process("taken")
        assert_includes bp.accepting, bp.current_state # WT is accepting
      end
    end

    # ================================================================
    # Reset Tests
    # ================================================================

    class TestDFAReset < Minitest::Test
      def test_reset_returns_to_initial
        dfa = StateMachine.make_turnstile
        dfa.process("coin")
        assert_equal "unlocked", dfa.current_state

        dfa.reset
        assert_equal "locked", dfa.current_state
      end

      def test_reset_clears_trace
        dfa = StateMachine.make_turnstile
        dfa.process_sequence(%w[coin push coin])
        assert_equal 3, dfa.trace.length

        dfa.reset
        assert_equal [], dfa.trace
      end
    end

    # ================================================================
    # Introspection Tests
    # ================================================================

    class TestDFAIntrospection < Minitest::Test
      def test_reachable_states_all
        dfa = StateMachine.make_turnstile
        assert_equal Set["locked", "unlocked"], dfa.reachable_states
      end

      def test_reachable_states_with_unreachable
        dfa = DFA.new(
          states: Set["q0", "q1", "q_dead"],
          alphabet: Set["a"],
          transitions: {["q0", "a"] => "q1", ["q1", "a"] => "q0"},
          initial: "q0",
          accepting: Set.new
        )
        assert_equal Set["q0", "q1"], dfa.reachable_states
      end

      def test_is_complete_true
        dfa = StateMachine.make_turnstile
        assert dfa.complete?
      end

      def test_is_complete_false
        dfa = DFA.new(
          states: Set["q0", "q1"],
          alphabet: Set["a", "b"],
          transitions: {["q0", "a"] => "q1"},
          initial: "q0",
          accepting: Set.new
        )
        refute dfa.complete?
      end

      def test_validate_clean
        dfa = StateMachine.make_turnstile
        assert_equal [], dfa.validate
      end

      def test_validate_unreachable
        dfa = DFA.new(
          states: Set["q0", "q1", "q_dead"],
          alphabet: Set["a"],
          transitions: {
            ["q0", "a"] => "q1",
            ["q1", "a"] => "q0",
            ["q_dead", "a"] => "q_dead"
          },
          initial: "q0",
          accepting: Set.new
        )
        warnings = dfa.validate
        assert warnings.any? { |w| w.include?("Unreachable") }
        assert warnings.any? { |w| w.include?("q_dead") }
      end

      def test_validate_unreachable_accepting
        dfa = DFA.new(
          states: Set["q0", "q_dead"],
          alphabet: Set["a"],
          transitions: {
            ["q0", "a"] => "q0",
            ["q_dead", "a"] => "q_dead"
          },
          initial: "q0",
          accepting: Set["q_dead"]
        )
        warnings = dfa.validate
        assert warnings.any? { |w| w.include?("Unreachable accepting") }
      end

      def test_validate_missing_transitions
        dfa = DFA.new(
          states: Set["q0", "q1"],
          alphabet: Set["a", "b"],
          transitions: {["q0", "a"] => "q1"},
          initial: "q0",
          accepting: Set.new
        )
        warnings = dfa.validate
        assert warnings.any? { |w| w.include?("Missing transitions") }
      end
    end

    # ================================================================
    # Visualization Tests
    # ================================================================

    class TestDFAVisualization < Minitest::Test
      def test_to_dot_structure
        dfa = StateMachine.make_turnstile
        dot = dfa.to_dot
        assert_includes dot, "digraph DFA"
        assert_includes dot, "__start"
        assert_includes dot, "doublecircle"
        assert_includes dot, "locked"
        assert_includes dot, "unlocked"
        assert_includes dot, "coin"
        assert_includes dot, "push"
        assert dot.end_with?("}")
      end

      def test_to_dot_initial_arrow
        dfa = StateMachine.make_turnstile
        dot = dfa.to_dot
        assert_includes dot, '__start -> "locked"'
      end

      def test_to_dot_accepting_doublecircle
        dfa = StateMachine.make_turnstile
        dot = dfa.to_dot
        assert_includes dot, '"unlocked" [shape=doublecircle]'
        assert_includes dot, '"locked" [shape=circle]'
      end

      def test_to_ascii_contains_all_states
        dfa = StateMachine.make_turnstile
        ascii_table = dfa.to_ascii
        assert_includes ascii_table, "locked"
        assert_includes ascii_table, "unlocked"
        assert_includes ascii_table, "coin"
        assert_includes ascii_table, "push"
      end

      def test_to_ascii_marks_initial
        dfa = StateMachine.make_turnstile
        ascii_table = dfa.to_ascii
        assert_includes ascii_table, ">"
      end

      def test_to_ascii_marks_accepting
        dfa = StateMachine.make_turnstile
        ascii_table = dfa.to_ascii
        assert_includes ascii_table, "*"
      end

      def test_to_table_header
        dfa = StateMachine.make_turnstile
        table = dfa.to_table
        assert_equal "State", table[0][0]
        assert_includes table[0], "coin"
        assert_includes table[0], "push"
      end

      def test_to_table_data
        dfa = StateMachine.make_turnstile
        table = dfa.to_table
        locked_row = table.find { |row| row[0] == "locked" }
        events = table[0][1..]
        coin_idx = events.index("coin") + 1
        push_idx = events.index("push") + 1
        assert_equal "unlocked", locked_row[coin_idx]
        assert_equal "locked", locked_row[push_idx]
      end

      def test_to_table_missing_transitions
        dfa = DFA.new(
          states: Set["q0", "q1"],
          alphabet: Set["a", "b"],
          transitions: {["q0", "a"] => "q1"},
          initial: "q0",
          accepting: Set.new
        )
        table = dfa.to_table
        q0_row = table.find { |row| row[0] == "q0" }
        assert_includes q0_row, "\u2014"
      end
    end

    # ================================================================
    # Repr Tests
    # ================================================================

    class TestDFARepr < Minitest::Test
      def test_inspect_contains_key_info
        dfa = StateMachine.make_turnstile
        r = dfa.inspect
        assert_includes r, "DFA"
        assert_includes r, "locked"
        assert_includes r, "unlocked"
        assert_includes r, "coin"
        assert_includes r, "push"
      end
    end

    # ================================================================
    # Edge Cases
    # ================================================================

    class TestDFAEdgeCases < Minitest::Test
      def test_single_state_self_loop
        dfa = DFA.new(
          states: Set["q0"],
          alphabet: Set["a"],
          transitions: {["q0", "a"] => "q0"},
          initial: "q0",
          accepting: Set["q0"]
        )
        assert dfa.accepts(%w[a a a])
        assert dfa.accepts([])
      end

      def test_large_alphabet
        alphabet = ("a".."z").to_set
        transitions = {}
        alphabet.each do |c|
          transitions[["q0", c]] = "q1"
          transitions[["q1", c]] = "q0"
        end
        dfa = DFA.new(
          states: Set["q0", "q1"],
          alphabet: alphabet,
          transitions: transitions,
          initial: "q0",
          accepting: Set["q1"]
        )
        assert dfa.accepts(%w[a])
        refute dfa.accepts(%w[a b])
        assert dfa.accepts(%w[x y z])
      end

      def test_trace_property_returns_copy
        dfa = StateMachine.make_turnstile
        dfa.process("coin")
        t1 = dfa.trace
        t2 = dfa.trace
        assert_equal t1, t2
        refute_same t1, t2
      end

      def test_div_by_3_comprehensive
        dfa = StateMachine.make_div_by_3
        (0..31).each do |n|
          expected = (n % 3) == 0
          if n == 0
            assert dfa.accepts([]), "Failed for n=0"
          else
            binary = n.to_s(2)
            bits = binary.chars
            assert_equal expected, dfa.accepts(bits),
              "Failed for n=#{n} (binary=#{binary}): " \
              "expected #{expected ? "accept" : "reject"}"
          end
        end
      end
    end
  end
end

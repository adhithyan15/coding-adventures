# frozen_string_literal: true

require_relative "test_helper"

# ─── DFA Equivalence Tests ─────────────────────────────────────────────────────
#
# These tests verify that the DFA definitions (ONE_BIT_DFA and TWO_BIT_DFA)
# are structurally correct and produce the same behavior as the imperative
# predictor implementations. This is important because the DFAs serve as
# formal specifications of the branch predictor state machines.
#
# The strategy:
#   1. Verify the DFA structure (states, alphabet, completeness).
#   2. For every (state, event) pair, verify the DFA transition matches
#      the imperative transition function.
#   3. Run identical event sequences through both representations and
#      confirm they agree at every step.

module CodingAdventures
  module BranchPredictor
    # ─── Two-Bit DFA Equivalence ───────────────────────────────────────────────
    class TestTwoBitDFAEquivalence < Minitest::Test
      # -- Structural checks --

      def test_dfa_has_four_states
        assert_equal 4, TWO_BIT_DFA.states.size
        assert_includes TWO_BIT_DFA.states, "SNT"
        assert_includes TWO_BIT_DFA.states, "WNT"
        assert_includes TWO_BIT_DFA.states, "WT"
        assert_includes TWO_BIT_DFA.states, "ST"
      end

      def test_dfa_has_two_events
        assert_equal Set["taken", "not_taken"], TWO_BIT_DFA.alphabet
      end

      def test_dfa_initial_state_is_wnt
        assert_equal "WNT", TWO_BIT_DFA.initial
      end

      def test_dfa_accepting_states_are_taken_predictors
        assert_equal Set["WT", "ST"], TWO_BIT_DFA.accepting
      end

      def test_dfa_is_complete
        assert TWO_BIT_DFA.complete?,
          "TWO_BIT_DFA should have a transition for every (state, event) pair"
      end

      def test_dfa_has_no_validation_warnings
        warnings = TWO_BIT_DFA.validate
        assert_empty warnings, "TWO_BIT_DFA should have no warnings: #{warnings}"
      end

      # -- Transition equivalence: DFA vs TwoBitState --
      #
      # For every integer state (0-3) and every event (taken, not_taken),
      # the DFA transition must match TwoBitState's imperative transition.

      def test_all_taken_transitions_match
        TwoBitState::STATE_TO_NAME.each do |int_state, name|
          expected_int = TwoBitState.taken_outcome(int_state)
          expected_name = TwoBitState::STATE_TO_NAME[expected_int]

          dfa_target = TWO_BIT_DFA.transitions[[name, "taken"]]
          assert_equal expected_name, dfa_target,
            "Mismatch for (#{name}, taken): " \
            "TwoBitState says #{expected_name}, DFA says #{dfa_target}"
        end
      end

      def test_all_not_taken_transitions_match
        TwoBitState::STATE_TO_NAME.each do |int_state, name|
          expected_int = TwoBitState.not_taken_outcome(int_state)
          expected_name = TwoBitState::STATE_TO_NAME[expected_int]

          dfa_target = TWO_BIT_DFA.transitions[[name, "not_taken"]]
          assert_equal expected_name, dfa_target,
            "Mismatch for (#{name}, not_taken): " \
            "TwoBitState says #{expected_name}, DFA says #{dfa_target}"
        end
      end

      # -- Accepting-state equivalence: DFA acceptance == predicts_taken? --

      def test_accepting_matches_predicts_taken
        TwoBitState::STATE_TO_NAME.each do |int_state, name|
          imperative = TwoBitState.predicts_taken?(int_state)
          dfa_accepts = TWO_BIT_DFA.accepting.include?(name)
          assert_equal imperative, dfa_accepts,
            "State #{name} (#{int_state}): predicts_taken?=#{imperative}, " \
            "DFA accepting=#{dfa_accepts}"
        end
      end

      # -- Sequence equivalence --
      #
      # Run the same event sequence through both the DFA and TwoBitState,
      # checking that they agree after every step.

      def test_loop_sequence_equivalence
        # Simulate a 10-iteration loop: 9 taken + 1 not-taken
        events = (["taken"] * 9) + ["not_taken"]

        # DFA traversal (fresh copy via accepts/process)
        dfa = CodingAdventures::StateMachine::DFA.new(
          states: TWO_BIT_DFA.states,
          alphabet: TWO_BIT_DFA.alphabet,
          transitions: TWO_BIT_DFA.transitions,
          initial: TWO_BIT_DFA.initial,
          accepting: TWO_BIT_DFA.accepting
        )

        # Imperative traversal
        int_state = TwoBitState::WEAKLY_NOT_TAKEN

        events.each do |event|
          # Advance both
          dfa.process(event)
          int_state = if event == "taken"
            TwoBitState.taken_outcome(int_state)
          else
            TwoBitState.not_taken_outcome(int_state)
          end

          # They must agree
          assert_equal TwoBitState::STATE_TO_NAME[int_state], dfa.current_state,
            "Divergence after event '#{event}': " \
            "imperative=#{TwoBitState::STATE_TO_NAME[int_state]}, " \
            "DFA=#{dfa.current_state}"
        end
      end

      def test_alternating_sequence_equivalence
        # Alternating taken/not_taken -- stresses the weak states
        events = ["taken", "not_taken"] * 5

        dfa = CodingAdventures::StateMachine::DFA.new(
          states: TWO_BIT_DFA.states,
          alphabet: TWO_BIT_DFA.alphabet,
          transitions: TWO_BIT_DFA.transitions,
          initial: TWO_BIT_DFA.initial,
          accepting: TWO_BIT_DFA.accepting
        )

        int_state = TwoBitState::WEAKLY_NOT_TAKEN

        events.each do |event|
          dfa.process(event)
          int_state = if event == "taken"
            TwoBitState.taken_outcome(int_state)
          else
            TwoBitState.not_taken_outcome(int_state)
          end

          assert_equal TwoBitState::STATE_TO_NAME[int_state], dfa.current_state
        end
      end

      # -- DFA accepts() for prediction --
      #
      # The DFA's accepts() method answers: "after this event sequence, does
      # the machine predict taken?" This should match predicts_taken? on the
      # imperative state.

      def test_accepts_matches_predicts_taken_after_sequence
        sequences = [
          [],
          ["taken"],
          ["taken", "taken"],
          ["taken", "taken", "taken"],
          ["not_taken"],
          ["not_taken", "not_taken"],
          ["taken", "not_taken"],
          ["taken", "not_taken", "taken"]
        ]

        sequences.each do |seq|
          # Imperative: walk the state
          int_state = TwoBitState::WEAKLY_NOT_TAKEN
          seq.each do |event|
            int_state = if event == "taken"
              TwoBitState.taken_outcome(int_state)
            else
              TwoBitState.not_taken_outcome(int_state)
            end
          end

          imperative_predicts = TwoBitState.predicts_taken?(int_state)
          dfa_accepts = TWO_BIT_DFA.accepts(seq)

          assert_equal imperative_predicts, dfa_accepts,
            "Sequence #{seq}: predicts_taken?=#{imperative_predicts}, " \
            "DFA accepts=#{dfa_accepts}"
        end
      end
    end

    # ─── One-Bit DFA Equivalence ───────────────────────────────────────────────
    class TestOneBitDFAEquivalence < Minitest::Test
      # -- Structural checks --

      def test_dfa_has_two_states
        assert_equal 2, ONE_BIT_DFA.states.size
        assert_includes ONE_BIT_DFA.states, "NT"
        assert_includes ONE_BIT_DFA.states, "T"
      end

      def test_dfa_has_two_events
        assert_equal Set["taken", "not_taken"], ONE_BIT_DFA.alphabet
      end

      def test_dfa_initial_state_is_nt
        assert_equal "NT", ONE_BIT_DFA.initial
      end

      def test_dfa_accepting_state_is_t
        assert_equal Set["T"], ONE_BIT_DFA.accepting
      end

      def test_dfa_is_complete
        assert ONE_BIT_DFA.complete?,
          "ONE_BIT_DFA should have a transition for every (state, event) pair"
      end

      def test_dfa_has_no_validation_warnings
        warnings = ONE_BIT_DFA.validate
        assert_empty warnings, "ONE_BIT_DFA should have no warnings: #{warnings}"
      end

      # -- Transition equivalence: DFA vs OneBitPredictor behavior --
      #
      # The 1-bit predictor's transition is trivial: the new state IS the event.
      #   taken     -> T  (predict taken)
      #   not_taken -> NT (predict not taken)

      def test_nt_taken_goes_to_t
        assert_equal "T", ONE_BIT_DFA.transitions[["NT", "taken"]]
      end

      def test_nt_not_taken_stays_nt
        assert_equal "NT", ONE_BIT_DFA.transitions[["NT", "not_taken"]]
      end

      def test_t_taken_stays_t
        assert_equal "T", ONE_BIT_DFA.transitions[["T", "taken"]]
      end

      def test_t_not_taken_goes_to_nt
        assert_equal "NT", ONE_BIT_DFA.transitions[["T", "not_taken"]]
      end

      # -- Sequence equivalence with OneBitPredictor --

      def test_loop_sequence_equivalence
        events = (["taken"] * 9) + ["not_taken"]

        dfa = CodingAdventures::StateMachine::DFA.new(
          states: ONE_BIT_DFA.states,
          alphabet: ONE_BIT_DFA.alphabet,
          transitions: ONE_BIT_DFA.transitions,
          initial: ONE_BIT_DFA.initial,
          accepting: ONE_BIT_DFA.accepting
        )

        predictor = OneBitPredictor.new(table_size: 1024)
        pc = 0x100

        events.each do |event|
          taken = (event == "taken")
          dfa.process(event)
          predictor.update(pc: pc, taken: taken)

          # DFA accepting == predictor predicts taken
          dfa_predicts_taken = ONE_BIT_DFA.accepting.include?(dfa.current_state)
          predictor_predicts_taken = predictor.predict(pc: pc).taken?

          assert_equal dfa_predicts_taken, predictor_predicts_taken,
            "Divergence after '#{event}': " \
            "DFA=#{dfa.current_state} (taken=#{dfa_predicts_taken}), " \
            "predictor=#{predictor_predicts_taken}"
        end
      end

      def test_alternating_sequence_equivalence
        events = ["taken", "not_taken"] * 5

        dfa = CodingAdventures::StateMachine::DFA.new(
          states: ONE_BIT_DFA.states,
          alphabet: ONE_BIT_DFA.alphabet,
          transitions: ONE_BIT_DFA.transitions,
          initial: ONE_BIT_DFA.initial,
          accepting: ONE_BIT_DFA.accepting
        )

        predictor = OneBitPredictor.new(table_size: 1024)
        pc = 0x200

        events.each do |event|
          taken = (event == "taken")
          dfa.process(event)
          predictor.update(pc: pc, taken: taken)

          dfa_predicts_taken = ONE_BIT_DFA.accepting.include?(dfa.current_state)
          predictor_predicts_taken = predictor.predict(pc: pc).taken?

          assert_equal dfa_predicts_taken, predictor_predicts_taken
        end
      end

      # -- DFA accepts() --

      def test_accepts_matches_predictor_after_sequence
        sequences = [
          [],
          ["taken"],
          ["not_taken"],
          ["taken", "taken"],
          ["taken", "not_taken"],
          ["not_taken", "taken"],
          ["taken", "not_taken", "taken"]
        ]

        sequences.each do |seq|
          # Imperative: the 1-bit predictor's state after the sequence
          # is simply the last event (or false/NT if empty)
          expected_taken = if seq.empty?
            false
          else
            seq.last == "taken"
          end

          dfa_accepts = ONE_BIT_DFA.accepts(seq)

          assert_equal expected_taken, dfa_accepts,
            "Sequence #{seq}: expected taken=#{expected_taken}, " \
            "DFA accepts=#{dfa_accepts}"
        end
      end
    end
  end
end

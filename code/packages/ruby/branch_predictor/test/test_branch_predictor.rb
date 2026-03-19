# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module BranchPredictor
    # ─── PredictionStats Tests ──────────────────────────────────────────────────
    class TestPredictionStats < Minitest::Test
      def test_initial_state
        stats = PredictionStats.new
        assert_equal 0, stats.predictions
        assert_equal 0, stats.correct
        assert_equal 0, stats.incorrect
        assert_in_delta 0.0, stats.accuracy
        assert_in_delta 0.0, stats.misprediction_rate
      end

      def test_record_correct
        stats = PredictionStats.new
        stats.record(correct: true)
        assert_equal 1, stats.predictions
        assert_equal 1, stats.correct
        assert_equal 0, stats.incorrect
        assert_in_delta 100.0, stats.accuracy
        assert_in_delta 0.0, stats.misprediction_rate
      end

      def test_record_incorrect
        stats = PredictionStats.new
        stats.record(correct: false)
        assert_equal 1, stats.predictions
        assert_equal 0, stats.correct
        assert_equal 1, stats.incorrect
        assert_in_delta 0.0, stats.accuracy
        assert_in_delta 100.0, stats.misprediction_rate
      end

      def test_mixed_predictions
        stats = PredictionStats.new
        stats.record(correct: true)
        stats.record(correct: true)
        stats.record(correct: false)
        assert_equal 3, stats.predictions
        assert_equal 2, stats.correct
        assert_equal 1, stats.incorrect
        assert_in_delta 66.67, stats.accuracy, 0.01
        assert_in_delta 33.33, stats.misprediction_rate, 0.01
      end

      def test_reset
        stats = PredictionStats.new
        stats.record(correct: true)
        stats.record(correct: false)
        stats.reset
        assert_equal 0, stats.predictions
        assert_equal 0, stats.correct
        assert_equal 0, stats.incorrect
        assert_in_delta 0.0, stats.accuracy
      end
    end

    # ─── Prediction Tests ───────────────────────────────────────────────────────
    class TestPrediction < Minitest::Test
      def test_default_values
        pred = Prediction.new(taken: true)
        assert pred.taken?
        assert_in_delta 0.0, pred.confidence
        assert_nil pred.target
      end

      def test_full_constructor
        pred = Prediction.new(taken: true, confidence: 0.9, target: 0x400)
        assert pred.taken?
        assert_in_delta 0.9, pred.confidence
        assert_equal 0x400, pred.target
      end

      def test_not_taken
        pred = Prediction.new(taken: false)
        refute pred.taken?
      end

      def test_frozen
        pred = Prediction.new(taken: true)
        assert pred.frozen?
      end

      def test_equality
        a = Prediction.new(taken: true, confidence: 0.5, target: 0x100)
        b = Prediction.new(taken: true, confidence: 0.5, target: 0x100)
        assert_equal a, b
      end

      def test_inequality
        a = Prediction.new(taken: true)
        b = Prediction.new(taken: false)
        refute_equal a, b
      end
    end

    # ─── AlwaysTakenPredictor Tests ─────────────────────────────────────────────
    class TestAlwaysTakenPredictor < Minitest::Test
      def setup
        @pred = AlwaysTakenPredictor.new
      end

      def test_always_predicts_taken
        pred = @pred.predict(pc: 0x100)
        assert pred.taken?
        assert_in_delta 0.0, pred.confidence
      end

      def test_correct_on_taken_branch
        @pred.update(pc: 0x100, taken: true)
        assert_equal 1, @pred.stats.correct
        assert_equal 0, @pred.stats.incorrect
      end

      def test_incorrect_on_not_taken_branch
        @pred.update(pc: 0x100, taken: false)
        assert_equal 0, @pred.stats.correct
        assert_equal 1, @pred.stats.incorrect
      end

      def test_accuracy_on_loop
        # Simulates a loop that runs 10 times: 9 taken + 1 not-taken
        9.times { @pred.update(pc: 0x100, taken: true) }
        @pred.update(pc: 0x100, taken: false)
        assert_in_delta 90.0, @pred.stats.accuracy
      end

      def test_reset
        @pred.update(pc: 0x100, taken: true)
        @pred.reset
        assert_equal 0, @pred.stats.predictions
      end

      def test_different_pcs
        pred1 = @pred.predict(pc: 0x100)
        pred2 = @pred.predict(pc: 0x200)
        assert pred1.taken?
        assert pred2.taken?
      end

      def test_update_with_target
        @pred.update(pc: 0x100, taken: true, target: 0x200)
        assert_equal 1, @pred.stats.correct
      end
    end

    # ─── AlwaysNotTakenPredictor Tests ──────────────────────────────────────────
    class TestAlwaysNotTakenPredictor < Minitest::Test
      def setup
        @pred = AlwaysNotTakenPredictor.new
      end

      def test_always_predicts_not_taken
        pred = @pred.predict(pc: 0x100)
        refute pred.taken?
        assert_in_delta 0.0, pred.confidence
      end

      def test_correct_on_not_taken_branch
        @pred.update(pc: 0x100, taken: false)
        assert_equal 1, @pred.stats.correct
      end

      def test_incorrect_on_taken_branch
        @pred.update(pc: 0x100, taken: true)
        assert_equal 1, @pred.stats.incorrect
      end

      def test_accuracy_on_if_else
        # 50-50 taken/not-taken pattern
        5.times { @pred.update(pc: 0x100, taken: true) }
        5.times { @pred.update(pc: 0x100, taken: false) }
        assert_in_delta 50.0, @pred.stats.accuracy
      end

      def test_reset
        @pred.update(pc: 0x100, taken: false)
        @pred.reset
        assert_equal 0, @pred.stats.predictions
      end

      def test_update_with_target
        @pred.update(pc: 0x100, taken: false, target: 0x200)
        assert_equal 1, @pred.stats.correct
      end
    end

    # ─── BackwardTakenForwardNotTaken Tests ─────────────────────────────────────
    class TestBTFNT < Minitest::Test
      def setup
        @pred = BackwardTakenForwardNotTaken.new
      end

      def test_cold_start_predicts_not_taken
        pred = @pred.predict(pc: 0x108)
        refute pred.taken?
        assert_in_delta 0.0, pred.confidence
      end

      def test_backward_branch_predicts_taken
        # Teach the predictor a backward branch (loop back-edge)
        @pred.update(pc: 0x108, taken: true, target: 0x100)
        pred = @pred.predict(pc: 0x108)
        assert pred.taken?
        assert_in_delta 0.5, pred.confidence
        assert_equal 0x100, pred.target
      end

      def test_forward_branch_predicts_not_taken
        # Teach the predictor a forward branch (if-else skip)
        @pred.update(pc: 0x200, taken: false, target: 0x20C)
        pred = @pred.predict(pc: 0x200)
        refute pred.taken?
      end

      def test_equal_target_predicts_taken
        # Degenerate case: target == pc (infinite loop)
        @pred.update(pc: 0x100, taken: true, target: 0x100)
        pred = @pred.predict(pc: 0x100)
        assert pred.taken?
      end

      def test_loop_accuracy
        # First encounter -- target is stored before checking prediction,
        # so even the first update knows it's a backward branch -> predicts taken
        @pred.update(pc: 0x108, taken: true, target: 0x100)
        # Subsequent iterations -- backward branch predicts taken, correct
        8.times { @pred.update(pc: 0x108, taken: true, target: 0x100) }
        # Loop exit -- backward still predicts taken, wrong
        @pred.update(pc: 0x108, taken: false, target: 0x100)
        # 9 correct + 1 exit mispredict = 90%
        assert_in_delta 90.0, @pred.stats.accuracy
      end

      def test_reset
        @pred.update(pc: 0x108, taken: true, target: 0x100)
        @pred.reset
        assert_equal 0, @pred.stats.predictions
        # After reset, should be cold again
        pred = @pred.predict(pc: 0x108)
        refute pred.taken?
      end

      def test_nil_target_does_not_overwrite
        # First update with a target
        @pred.update(pc: 0x108, taken: true, target: 0x100)
        # Second update with nil target -- should not overwrite
        @pred.update(pc: 0x108, taken: true, target: nil)
        pred = @pred.predict(pc: 0x108)
        assert pred.taken?
        assert_equal 0x100, pred.target
      end
    end

    # ─── OneBitPredictor Tests ──────────────────────────────────────────────────
    class TestOneBitPredictor < Minitest::Test
      def setup
        @pred = OneBitPredictor.new(table_size: 1024)
      end

      def test_cold_start_predicts_not_taken
        pred = @pred.predict(pc: 0x100)
        refute pred.taken?
        assert_in_delta 0.5, pred.confidence
      end

      def test_learns_taken
        @pred.update(pc: 0x100, taken: true)
        pred = @pred.predict(pc: 0x100)
        assert pred.taken?
      end

      def test_learns_not_taken
        @pred.update(pc: 0x100, taken: true)
        @pred.update(pc: 0x100, taken: false)
        pred = @pred.predict(pc: 0x100)
        refute pred.taken?
      end

      def test_double_misprediction_on_loop
        # Simulating 10-iteration loop across 2 invocations:
        # First invocation:
        #   Iter 1:  bit=0, predict NT, actual T -> WRONG, set bit=1
        #   Iter 2-9: bit=1, predict T, actual T -> correct
        #   Iter 10: bit=1, predict T, actual NT -> WRONG, set bit=0
        # Second invocation:
        #   Iter 1: bit=0, predict NT, actual T -> WRONG
        @pred.update(pc: 0x100, taken: true)   # wrong (cold)
        8.times { @pred.update(pc: 0x100, taken: true) }   # 8 correct
        @pred.update(pc: 0x100, taken: false)  # wrong (exit)
        # 8 correct out of 10 = 80%
        assert_in_delta 80.0, @pred.stats.accuracy
      end

      def test_aliasing
        # Two branches that map to the same table entry (both map to index 0)
        small_pred = OneBitPredictor.new(table_size: 4)
        # Branch at 0x04 -> index 0, set to taken
        small_pred.update(pc: 0x04, taken: true)
        # Branch at 0x00 -> also index 0, overwrites with not-taken
        small_pred.update(pc: 0x00, taken: false)
        # Now predicting for 0x04 reads the corrupted entry
        pred = small_pred.predict(pc: 0x04)
        refute pred.taken?  # corrupted by aliasing!
      end

      def test_reset
        @pred.update(pc: 0x100, taken: true)
        @pred.reset
        assert_equal 0, @pred.stats.predictions
        pred = @pred.predict(pc: 0x100)
        refute pred.taken?
      end

      def test_different_branches_independent
        @pred.update(pc: 0x01, taken: true)
        @pred.update(pc: 0x02, taken: false)
        assert @pred.predict(pc: 0x01).taken?
        refute @pred.predict(pc: 0x02).taken?
      end

      def test_update_with_target_ignored
        @pred.update(pc: 0x100, taken: true, target: 0x200)
        pred = @pred.predict(pc: 0x100)
        assert pred.taken?
        assert_nil pred.target
      end
    end

    # ─── TwoBitState Tests ──────────────────────────────────────────────────────
    class TestTwoBitState < Minitest::Test
      def test_taken_outcome_increments
        assert_equal TwoBitState::WEAKLY_NOT_TAKEN,
          TwoBitState.taken_outcome(TwoBitState::STRONGLY_NOT_TAKEN)
        assert_equal TwoBitState::WEAKLY_TAKEN,
          TwoBitState.taken_outcome(TwoBitState::WEAKLY_NOT_TAKEN)
        assert_equal TwoBitState::STRONGLY_TAKEN,
          TwoBitState.taken_outcome(TwoBitState::WEAKLY_TAKEN)
      end

      def test_taken_outcome_saturates
        assert_equal TwoBitState::STRONGLY_TAKEN,
          TwoBitState.taken_outcome(TwoBitState::STRONGLY_TAKEN)
      end

      def test_not_taken_outcome_decrements
        assert_equal TwoBitState::WEAKLY_TAKEN,
          TwoBitState.not_taken_outcome(TwoBitState::STRONGLY_TAKEN)
        assert_equal TwoBitState::WEAKLY_NOT_TAKEN,
          TwoBitState.not_taken_outcome(TwoBitState::WEAKLY_TAKEN)
        assert_equal TwoBitState::STRONGLY_NOT_TAKEN,
          TwoBitState.not_taken_outcome(TwoBitState::WEAKLY_NOT_TAKEN)
      end

      def test_not_taken_outcome_saturates
        assert_equal TwoBitState::STRONGLY_NOT_TAKEN,
          TwoBitState.not_taken_outcome(TwoBitState::STRONGLY_NOT_TAKEN)
      end

      def test_predicts_taken
        refute TwoBitState.predicts_taken?(TwoBitState::STRONGLY_NOT_TAKEN)
        refute TwoBitState.predicts_taken?(TwoBitState::WEAKLY_NOT_TAKEN)
        assert TwoBitState.predicts_taken?(TwoBitState::WEAKLY_TAKEN)
        assert TwoBitState.predicts_taken?(TwoBitState::STRONGLY_TAKEN)
      end
    end

    # ─── TwoBitPredictor Tests ──────────────────────────────────────────────────
    class TestTwoBitPredictor < Minitest::Test
      def setup
        @pred = TwoBitPredictor.new(table_size: 1024)
      end

      def test_initial_predicts_not_taken
        pred = @pred.predict(pc: 0x100)
        refute pred.taken?
        assert_in_delta 0.5, pred.confidence  # WEAKLY_NOT_TAKEN -> 0.5
      end

      def test_one_taken_flips_to_taken
        @pred.update(pc: 0x100, taken: true)
        pred = @pred.predict(pc: 0x100)
        assert pred.taken?
        assert_in_delta 0.5, pred.confidence  # WEAKLY_TAKEN
      end

      def test_two_taken_becomes_strongly_taken
        @pred.update(pc: 0x100, taken: true)
        @pred.update(pc: 0x100, taken: true)
        pred = @pred.predict(pc: 0x100)
        assert pred.taken?
        assert_in_delta 1.0, pred.confidence  # STRONGLY_TAKEN
      end

      def test_hysteresis_prevents_double_mispredict
        # Get to STRONGLY_TAKEN
        3.times { @pred.update(pc: 0x100, taken: true) }
        assert_equal TwoBitState::STRONGLY_TAKEN, @pred.get_state(pc: 0x100)

        # One not-taken only goes to WEAKLY_TAKEN (still predicts taken)
        @pred.update(pc: 0x100, taken: false)
        assert_equal TwoBitState::WEAKLY_TAKEN, @pred.get_state(pc: 0x100)
        pred = @pred.predict(pc: 0x100)
        assert pred.taken?
      end

      def test_two_not_taken_flips_prediction
        # Start at WEAKLY_NOT_TAKEN (1), taken -> WEAKLY_TAKEN (2)
        @pred.update(pc: 0x100, taken: true)
        # not-taken -> WEAKLY_NOT_TAKEN (1)
        @pred.update(pc: 0x100, taken: false)
        # not-taken -> STRONGLY_NOT_TAKEN (0)
        @pred.update(pc: 0x100, taken: false)
        pred = @pred.predict(pc: 0x100)
        refute pred.taken?
        assert_in_delta 1.0, pred.confidence  # STRONGLY_NOT_TAKEN
      end

      def test_custom_initial_state
        pred = TwoBitPredictor.new(
          table_size: 256,
          initial_state: TwoBitState::STRONGLY_TAKEN
        )
        result = pred.predict(pc: 0x100)
        assert result.taken?
        assert_in_delta 1.0, result.confidence
      end

      def test_loop_solves_double_mispredict
        # Simulate a 10-iteration loop over 2 invocations
        # First invocation:
        #   Iter 1:  WNT(1) -> predict NT -> actual T -> WRONG -> WT(2)
        #   Iter 2:  WT(2)  -> predict T  -> actual T -> correct -> ST(3)
        #   Iter 3-9: ST(3) -> predict T  -> actual T -> correct (saturated)
        #   Iter 10: ST(3) -> predict T  -> actual NT -> WRONG -> WT(2)
        # Second invocation:
        #   Iter 1: WT(2) -> predict T -> actual T -> correct! -> ST(3)
        9.times { @pred.update(pc: 0x100, taken: true) }
        @pred.update(pc: 0x100, taken: false)
        @pred.update(pc: 0x100, taken: true)
        # 1 wrong (first) + 8 correct + 1 wrong (exit) + 1 correct (re-entry) = 9/11
        assert_in_delta 81.82, @pred.stats.accuracy, 0.01
      end

      def test_reset
        @pred.update(pc: 0x100, taken: true)
        @pred.reset
        assert_equal 0, @pred.stats.predictions
        pred = @pred.predict(pc: 0x100)
        refute pred.taken?
      end

      def test_get_state
        assert_equal TwoBitState::WEAKLY_NOT_TAKEN, @pred.get_state(pc: 0x100)
        @pred.update(pc: 0x100, taken: true)
        assert_equal TwoBitState::WEAKLY_TAKEN, @pred.get_state(pc: 0x100)
      end

      def test_update_with_target_ignored
        @pred.update(pc: 0x100, taken: true, target: 0x200)
        assert_equal TwoBitState::WEAKLY_TAKEN, @pred.get_state(pc: 0x100)
      end
    end

    # ─── BranchTargetBuffer Tests ───────────────────────────────────────────────
    class TestBranchTargetBuffer < Minitest::Test
      def setup
        @btb = BranchTargetBuffer.new(size: 256)
      end

      def test_cold_lookup_misses
        result = @btb.lookup(pc: 0x01)
        assert_nil result
        assert_equal 1, @btb.lookups
        assert_equal 0, @btb.hits
        assert_equal 1, @btb.misses
      end

      def test_update_then_lookup_hits
        @btb.update(pc: 0x01, target: 0x02, branch_type: "conditional")
        result = @btb.lookup(pc: 0x01)
        assert_equal 0x02, result
        assert_equal 1, @btb.hits
      end

      def test_aliasing_causes_miss
        # Two different PCs that map to the same BTB index
        # Use addresses that definitely alias: 0x01 and 0x01 + 256 = 0x101
        @btb.update(pc: 0x01, target: 0x50, branch_type: "conditional")
        @btb.update(pc: 0x01 + 256, target: 0x60, branch_type: "conditional")
        # The second update evicts the first (direct-mapped)
        result = @btb.lookup(pc: 0x01)
        assert_nil result  # evicted by aliasing
      end

      def test_get_entry
        @btb.update(pc: 0x01, target: 0x02, branch_type: "call")
        entry = @btb.get_entry(pc: 0x01)
        refute_nil entry
        assert entry.valid
        assert_equal 0x01, entry.tag
        assert_equal 0x02, entry.target
        assert_equal "call", entry.branch_type
      end

      def test_get_entry_miss
        entry = @btb.get_entry(pc: 0x01)
        assert_nil entry
      end

      def test_hit_rate_zero_lookups
        assert_in_delta 0.0, @btb.hit_rate
      end

      def test_hit_rate_all_hits
        @btb.update(pc: 0x01, target: 0x02)
        @btb.lookup(pc: 0x01)
        @btb.lookup(pc: 0x01)
        assert_in_delta 100.0, @btb.hit_rate
      end

      def test_hit_rate_mixed
        @btb.update(pc: 0x01, target: 0x02)
        @btb.lookup(pc: 0x01) # hit
        @btb.lookup(pc: 0x03) # miss
        assert_in_delta 50.0, @btb.hit_rate
      end

      def test_multiple_branches
        @btb.update(pc: 0x01, target: 0x10, branch_type: "conditional")
        @btb.update(pc: 0x02, target: 0x20, branch_type: "unconditional")
        @btb.update(pc: 0x03, target: 0x30, branch_type: "call")
        assert_equal 0x10, @btb.lookup(pc: 0x01)
        assert_equal 0x20, @btb.lookup(pc: 0x02)
        assert_equal 0x30, @btb.lookup(pc: 0x03)
      end

      def test_update_overwrites_target
        @btb.update(pc: 0x01, target: 0x10)
        @btb.update(pc: 0x01, target: 0x20)
        assert_equal 0x20, @btb.lookup(pc: 0x01)
      end

      def test_reset
        @btb.update(pc: 0x01, target: 0x02)
        @btb.lookup(pc: 0x01)
        @btb.reset
        assert_equal 0, @btb.lookups
        assert_equal 0, @btb.hits
        assert_equal 0, @btb.misses
        assert_nil @btb.lookup(pc: 0x01)
      end

      def test_default_branch_type
        @btb.update(pc: 0x01, target: 0x02)
        entry = @btb.get_entry(pc: 0x01)
        assert_equal "conditional", entry.branch_type
      end

      def test_return_branch_type
        @btb.update(pc: 0x01, target: 0x02, branch_type: "return")
        entry = @btb.get_entry(pc: 0x01)
        assert_equal "return", entry.branch_type
      end
    end
  end
end

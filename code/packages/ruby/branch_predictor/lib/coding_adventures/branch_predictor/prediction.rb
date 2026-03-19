# frozen_string_literal: true

# ─── Prediction ───────────────────────────────────────────────────────────────
#
# A Prediction is the output of a branch predictor's `predict` method. It
# bundles three pieces of information:
#
# 1. taken      -- will the branch jump to its target? (the core question)
# 2. confidence -- how sure is the predictor? (useful for hybrid/tournament
#                  predictors that choose between sub-predictors)
# 3. target     -- where does the branch go? (from the BTB, if available)
#
# Predictions are frozen (immutable) values. Once the predictor makes a guess,
# that guess should not change -- it represents a snapshot of the predictor's
# state at prediction time.

module CodingAdventures
  module BranchPredictor
    # A branch prediction -- the predictor's guess before the branch executes.
    #
    # In a real CPU, the prediction is produced in the fetch stage (cycle 1).
    # The branch is not resolved until the execute stage (cycle 10+ on a deep
    # pipeline). The CPU speculatively fetches instructions from the predicted
    # path. If the prediction is wrong, all that speculative work is flushed.
    #
    # @example A confident prediction that the branch is taken
    #   pred = Prediction.new(taken: true, confidence: 0.9, target: 0x400)
    #
    # @example A cold-start prediction with no confidence
    #   pred = Prediction.new(taken: false, confidence: 0.0)
    class Prediction
      # @return [Boolean] the predictor's guess: will the branch be taken?
      attr_reader :taken

      # @return [Float] confidence level from 0.0 (no confidence) to 1.0 (certain)
      attr_reader :confidence

      # @return [Integer, nil] predicted target address (from BTB, if available)
      attr_reader :target

      # @param taken [Boolean] will the branch be taken?
      # @param confidence [Float] how confident the predictor is (0.0 to 1.0)
      # @param target [Integer, nil] predicted target address
      def initialize(taken:, confidence: 0.0, target: nil)
        @taken = taken
        @confidence = confidence
        @target = target
        freeze
      end

      # @return [Boolean] alias for +taken+ for readability
      def taken?
        @taken
      end

      def ==(other)
        other.is_a?(Prediction) &&
          taken == other.taken &&
          confidence == other.confidence &&
          target == other.target
      end
    end
  end
end

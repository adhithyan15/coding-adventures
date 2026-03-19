# frozen_string_literal: true

# ─── PredictionStats ──────────────────────────────────────────────────────────
#
# Every branch predictor needs a scorecard. When a CPU designer evaluates a
# predictor, the first question is always: "What's the accuracy?" A predictor
# that's 95% accurate causes a pipeline flush on only 5% of branches, while
# a 70% accurate predictor flushes on 30% -- potentially halving throughput
# on a deeply pipelined machine.
#
# We track three counters:
#   predictions -- total number of branches seen
#   correct     -- how many the predictor got right
#   incorrect   -- how many it got wrong
#
# From these, we derive:
#   accuracy            -- correct / predictions * 100 (as a percentage)
#   misprediction_rate  -- incorrect / predictions * 100 (the complement)
#
# Edge case: if no predictions have been made yet, both rates return 0.0
# rather than raising a ZeroDivisionError. A predictor that has not seen any
# branches has no accuracy, not infinite accuracy.

module CodingAdventures
  module BranchPredictor
    # Tracks prediction accuracy for a branch predictor.
    #
    # The stats object is usually owned by a predictor and exposed via its
    # +stats+ method. The CPU core never creates PredictionStats directly --
    # it just reads the predictor's stats after running a benchmark.
    #
    # @example
    #   stats = PredictionStats.new
    #   stats.record(correct: true)   # predictor got it right
    #   stats.record(correct: true)   # right again
    #   stats.record(correct: false)  # wrong this time
    #   stats.accuracy  # => 66.67 (2 out of 3)
    class PredictionStats
      # @return [Integer] total number of predictions made
      attr_reader :predictions

      # @return [Integer] number of correct predictions
      attr_reader :correct

      # @return [Integer] number of incorrect predictions (mispredictions)
      attr_reader :incorrect

      def initialize
        @predictions = 0
        @correct = 0
        @incorrect = 0
      end

      # Record the outcome of a single prediction.
      #
      # This is the primary API that the CPU core calls after every branch.
      #
      # @param correct [Boolean] true if the predictor guessed correctly
      def record(correct:)
        @predictions += 1
        if correct
          @correct += 1
        else
          @incorrect += 1
        end
      end

      # Prediction accuracy as a percentage (0.0 to 100.0).
      #
      # Returns 0.0 if no predictions have been made, because "no data" is
      # semantically closer to "0% accurate" than "100% accurate" in a
      # benchmarking context.
      #
      # @return [Float] accuracy percentage
      def accuracy
        return 0.0 if @predictions == 0
        (@correct.to_f / @predictions) * 100.0
      end

      # Misprediction rate as a percentage (0.0 to 100.0).
      #
      # This is the complement of accuracy: misprediction_rate = 100 - accuracy.
      # CPU architects think in terms of misprediction rate because each
      # misprediction causes a pipeline flush -- a concrete, measurable cost.
      #
      # @return [Float] misprediction rate percentage
      def misprediction_rate
        return 0.0 if @predictions == 0
        (@incorrect.to_f / @predictions) * 100.0
      end

      # Reset all counters to zero.
      #
      # Called when starting a new benchmark. Without this, stats from a
      # previous run would contaminate the new measurement.
      def reset
        @predictions = 0
        @correct = 0
        @incorrect = 0
      end
    end
  end
end

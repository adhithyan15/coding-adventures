# frozen_string_literal: true

# ─── Static Branch Predictors ─────────────────────────────────────────────────
#
# Static predictors make the same prediction every time, regardless of history.
# They require zero hardware (no tables, no counters, no state) and serve as
# baselines against which dynamic predictors are measured.
#
# Three strategies are implemented here:
#
# 1. AlwaysTakenPredictor -- always predicts "taken"
#    Accuracy: ~60-70% on typical code. Why? Most branches are loop back-edges,
#    which are taken on every iteration except the last. A loop that runs 100
#    times has 100 branches: 99 taken + 1 not-taken = 99% accuracy on that loop.
#
# 2. AlwaysNotTakenPredictor -- always predicts "not taken"
#    Accuracy: ~30-40%. The worst reasonable strategy, but has a hardware
#    advantage: the "not taken" path is just the next sequential instruction,
#    so the CPU doesn't need to compute a target address. The Intel 8086 (1978)
#    effectively used this -- it had no branch prediction unit.
#
# 3. BackwardTakenForwardNotTaken (BTFNT) -- direction-based heuristic
#    Accuracy: ~65-75%. Backward branches (target < pc) are usually loop
#    back-edges, so predict taken. Forward branches (target > pc) are usually
#    if-else, so predict not-taken. Used in MIPS R4000 and SPARC.

module CodingAdventures
  module BranchPredictor
    # Always predicts "taken". Simple but surprisingly effective (~60% accurate).
    #
    # Most branches in real programs are loop back-edges, which are taken
    # on every iteration except the last. So "always taken" gets the loop body
    # right every time, only missing the final exit.
    #
    # Hardware cost: zero. The prediction logic is just a wire tied to 1.
    #
    # @example
    #   predictor = AlwaysTakenPredictor.new
    #   pred = predictor.predict(pc: 0x100)
    #   pred.taken? # => true
    class AlwaysTakenPredictor
      def initialize
        @stats = PredictionStats.new
      end

      # Always predict taken, with zero confidence (it's just a guess).
      #
      # @param pc [Integer] the program counter of the branch instruction (unused)
      # @return [Prediction]
      def predict(pc:) # rubocop:disable Lint/UnusedMethodArgument
        Prediction.new(taken: true, confidence: 0.0)
      end

      # Record whether the always-taken guess was correct.
      #
      # @param pc [Integer] the program counter (unused -- no per-branch state)
      # @param taken [Boolean] whether the branch was actually taken
      # @param target [Integer, nil] the actual target address (unused)
      def update(pc:, taken:, target: nil) # rubocop:disable Lint/UnusedMethodArgument
        @stats.record(correct: taken)
      end

      # @return [PredictionStats] prediction accuracy statistics
      def stats
        @stats
      end

      # Reset statistics (no predictor state to clear).
      def reset
        @stats.reset
      end
    end

    # Always predicts "not taken". The simplest possible predictor.
    #
    # This is the baseline against which all other predictors are measured.
    # If your fancy predictor can't beat "always not taken", something is wrong.
    #
    # Hardware advantage: the "not taken" path is the next sequential instruction,
    # which the fetch unit is already computing. Zero target-address overhead.
    #
    # @example
    #   predictor = AlwaysNotTakenPredictor.new
    #   pred = predictor.predict(pc: 0x100)
    #   pred.taken? # => false
    class AlwaysNotTakenPredictor
      def initialize
        @stats = PredictionStats.new
      end

      # Always predict not taken, with zero confidence.
      #
      # @param pc [Integer] the program counter (unused)
      # @return [Prediction]
      def predict(pc:) # rubocop:disable Lint/UnusedMethodArgument
        Prediction.new(taken: false, confidence: 0.0)
      end

      # Record whether the always-not-taken guess was correct.
      #
      # We predicted NOT taken, so we're correct when the branch is NOT taken.
      #
      # @param pc [Integer] the program counter (unused)
      # @param taken [Boolean] whether the branch was actually taken
      # @param target [Integer, nil] the actual target address (unused)
      def update(pc:, taken:, target: nil) # rubocop:disable Lint/UnusedMethodArgument
        @stats.record(correct: !taken)
      end

      # @return [PredictionStats]
      def stats
        @stats
      end

      # Reset statistics.
      def reset
        @stats.reset
      end
    end

    # BTFNT -- predicts taken for backward branches, not-taken for forward.
    #
    # Backward branches (target < pc) are usually loop back-edges -> predict taken.
    # Forward branches (target > pc) are usually if-else -> predict not-taken.
    #
    # This is what early MIPS and SPARC processors used. The predictor requires
    # knowing the branch target. On the first encounter (cold start), it defaults
    # to predicting NOT taken, since we don't yet know the target direction.
    #
    # Direction-based heuristic:
    #   - Backward branch (target < pc):  predict TAKEN  (loop back-edge)
    #   - Forward branch (target > pc):   predict NOT TAKEN (if-else)
    #   - Equal (target == pc):           predict TAKEN  (degenerate infinite loop)
    #
    # @example
    #   predictor = BackwardTakenForwardNotTaken.new
    #   predictor.update(pc: 0x108, taken: true, target: 0x100)
    #   pred = predictor.predict(pc: 0x108)
    #   pred.taken? # => true  (backward branch -> taken)
    class BackwardTakenForwardNotTaken
      def initialize
        @stats = PredictionStats.new
        # Maps PC -> last known target address. We need this because predict()
        # is called before decode, so we rely on previous updates to know the
        # branch direction.
        @targets = {}
      end

      # Predict based on branch direction: backward=taken, forward=not-taken.
      #
      # If we haven't seen this branch before (no known target), we default
      # to NOT taken -- the safe choice that doesn't require a target address.
      #
      # @param pc [Integer] the program counter of the branch instruction
      # @return [Prediction]
      def predict(pc:)
        target = @targets[pc]
        if target.nil?
          # Cold start -- we don't know the target direction yet
          return Prediction.new(taken: false, confidence: 0.0)
        end

        # Backward branch (target <= pc) -> taken (loop back-edge)
        # Forward branch (target > pc)   -> not taken (if-else)
        taken = target <= pc
        Prediction.new(taken: taken, confidence: 0.5, target: target)
      end

      # Record the branch outcome and learn the target address.
      #
      # The key learning here is remembering the target address for future
      # predictions. The BTFNT predictor doesn't adapt its strategy -- it always
      # uses the direction heuristic -- but it needs to know the target to
      # determine the direction.
      #
      # @param pc [Integer] the program counter
      # @param taken [Boolean] whether the branch was actually taken
      # @param target [Integer, nil] the actual target address
      def update(pc:, taken:, target: nil)
        # Store the target so we can use it for future direction-based predictions
        @targets[pc] = target unless target.nil?

        # Determine what we would have predicted, accounting for cold starts
        known_target = @targets[pc]
        predicted_taken = if known_target.nil?
          false
        else
          known_target <= pc
        end

        @stats.record(correct: predicted_taken == taken)
      end

      # @return [PredictionStats]
      def stats
        @stats
      end

      # Reset all state -- target cache and statistics.
      def reset
        @targets.clear
        @stats.reset
      end
    end
  end
end

# frozen_string_literal: true

# ─── Two-Bit Saturating Counter Predictor ─────────────────────────────────────
#
# The two-bit predictor improves on the one-bit predictor by adding hysteresis.
# Instead of flipping the prediction on every misprediction, it takes TWO
# consecutive mispredictions to change the predicted direction. This is achieved
# with a 2-bit saturating counter -- a counter that counts up to 3 and down to 0,
# but never wraps around (it "saturates" at the boundaries).
#
# The four states and their meanings:
#
#     State 0: STRONGLY_NOT_TAKEN  -> predict NOT TAKEN (high confidence)
#     State 1: WEAKLY_NOT_TAKEN    -> predict NOT TAKEN (low confidence)
#     State 2: WEAKLY_TAKEN        -> predict TAKEN     (low confidence)
#     State 3: STRONGLY_TAKEN      -> predict TAKEN     (high confidence)
#
# State transition diagram:
#
#     taken                taken               taken               taken
#     ------>              ------>              ------>              ------>
#     (sat)   SNT <------- WNT <------- WT <------- ST   (sat)
#             ------>              ------>              ------>
#           not taken          not taken           not taken
#
# The prediction threshold is at the midpoint:
#     states 0, 1 -> predict NOT TAKEN
#     states 2, 3 -> predict TAKEN
#
# Why this solves the double-misprediction problem:
#     A loop running 10 times: after the first taken, the counter moves to
#     STRONGLY_TAKEN. The single not-taken at loop exit only moves it to
#     WEAKLY_TAKEN, which still predicts taken next time. Only 1 misprediction
#     on re-entry (vs 2 for the one-bit predictor).
#
# Historical usage:
#     Alpha 21064: 2-bit counters with 2048 entries
#     Intel Pentium: 2-bit counters with 256 entries
#     Early ARM (ARM7): 2-bit counters with 64 entries

module CodingAdventures
  module BranchPredictor
    # The 4 states of a 2-bit saturating counter.
    #
    # In hardware, this is just a 2-bit register. The prediction is determined
    # by bit 1: if set, predict taken; if clear, predict not-taken. That's a
    # single wire -- zero logic gates.
    module TwoBitState
      STRONGLY_NOT_TAKEN = 0
      WEAKLY_NOT_TAKEN   = 1
      WEAKLY_TAKEN       = 2
      STRONGLY_TAKEN     = 3

      # Transition on a 'taken' outcome: increment, saturate at 3.
      #
      # @param state [Integer] current state (0-3)
      # @return [Integer] next state
      def self.taken_outcome(state)
        [state + 1, STRONGLY_TAKEN].min
      end

      # Transition on a 'not taken' outcome: decrement, saturate at 0.
      #
      # @param state [Integer] current state (0-3)
      # @return [Integer] next state
      def self.not_taken_outcome(state)
        [state - 1, STRONGLY_NOT_TAKEN].max
      end

      # Whether this state predicts 'taken'.
      #
      # The threshold is at WEAKLY_TAKEN (2). States 2 and 3 predict taken;
      # states 0 and 1 predict not-taken. In hardware, this is just bit 1
      # of the 2-bit counter.
      #
      # @param state [Integer] current state (0-3)
      # @return [Boolean]
      def self.predicts_taken?(state)
        state >= WEAKLY_TAKEN
      end
    end

    # 2-bit saturating counter predictor -- the classic, used in most textbooks.
    #
    # Used in real processors: Alpha 21064, early MIPS, early ARM. Modern CPUs
    # use more sophisticated predictors (TAGE, perceptron) but the 2-bit counter
    # is the foundation that all advanced predictors build on.
    #
    # @example
    #   predictor = TwoBitPredictor.new(table_size: 256)
    #   pred = predictor.predict(pc: 0x100)
    #   pred.taken? # => false (starts at WEAKLY_NOT_TAKEN)
    #   predictor.update(pc: 0x100, taken: true)
    #   pred = predictor.predict(pc: 0x100)
    #   pred.taken? # => true (moved to WEAKLY_TAKEN)
    class TwoBitPredictor
      # @param table_size [Integer] number of entries in the prediction table
      # @param initial_state [Integer] starting state for all counter entries
      #   (default: WEAKLY_NOT_TAKEN -- good balance of responsiveness and stability)
      def initialize(table_size: 1024, initial_state: TwoBitState::WEAKLY_NOT_TAKEN)
        @table_size = table_size
        @initial_state = initial_state
        @table = {}
        @stats = PredictionStats.new
      end

      # Predict based on the 2-bit counter for this branch.
      #
      # Reads the counter state. States 2-3 predict taken, states 0-1 not-taken.
      # Confidence is 1.0 for strong states, 0.5 for weak states.
      #
      # @param pc [Integer] the program counter
      # @return [Prediction]
      def predict(pc:)
        index = pc % @table_size
        state = @table.fetch(index, @initial_state)

        confidence = if state == TwoBitState::STRONGLY_TAKEN || state == TwoBitState::STRONGLY_NOT_TAKEN
          1.0
        else
          0.5
        end

        Prediction.new(taken: TwoBitState.predicts_taken?(state), confidence: confidence)
      end

      # Update the 2-bit counter based on the actual outcome.
      #
      # Increments on taken, decrements on not-taken, saturating at boundaries.
      #
      # @param pc [Integer] the program counter
      # @param taken [Boolean] whether the branch was actually taken
      # @param target [Integer, nil] the actual target address (unused)
      def update(pc:, taken:, target: nil) # rubocop:disable Lint/UnusedMethodArgument
        index = pc % @table_size
        state = @table.fetch(index, @initial_state)

        # Record accuracy BEFORE updating
        @stats.record(correct: TwoBitState.predicts_taken?(state) == taken)

        # Transition the state
        @table[index] = if taken
          TwoBitState.taken_outcome(state)
        else
          TwoBitState.not_taken_outcome(state)
        end
      end

      # @return [PredictionStats]
      def stats
        @stats
      end

      # Reset the prediction table and statistics.
      def reset
        @table.clear
        @stats.reset
      end

      # Inspect the current state for a branch address (for testing/debugging).
      #
      # @param pc [Integer] the program counter
      # @return [Integer] the current TwoBitState for this branch's table entry
      def get_state(pc:)
        index = pc % @table_size
        @table.fetch(index, @initial_state)
      end
    end
  end
end

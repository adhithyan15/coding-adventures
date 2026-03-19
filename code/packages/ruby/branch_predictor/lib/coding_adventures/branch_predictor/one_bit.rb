# frozen_string_literal: true

# ─── One-Bit Branch Predictor ─────────────────────────────────────────────────
#
# The one-bit predictor is the simplest dynamic predictor. Unlike static
# predictors (AlwaysTaken, BTFNT), it actually learns from the branch's
# history. Each branch address maps to a single bit of state that records
# the last outcome:
#
#     bit = 0  ->  predict NOT TAKEN
#     bit = 1  ->  predict TAKEN
#
# After each branch resolves, the bit is updated to match the actual outcome.
# This means the predictor always predicts "whatever happened last time."
#
# Hardware implementation:
#     A small SRAM table indexed by the lower bits of the PC.
#     Each entry is a single flip-flop (1 bit of storage).
#     Total storage: table_size * 1 bit.
#     For a 1024-entry table: 1024 bits = 128 bytes.
#
# The aliasing problem:
#     Since the table is indexed by (pc % table_size), two different branches
#     can map to the same entry. When branches alias, they corrupt each other's
#     predictions. With larger tables (1024+), aliasing is rare.
#
# The double-misprediction problem:
#     Consider a loop that runs 10 times then exits. The one-bit predictor
#     will mispredict TWICE per loop invocation: once at the start (cold/stale
#     state) and once at the exit. The two-bit predictor solves this by
#     requiring two consecutive mispredictions to flip the prediction.
#
# State diagram:
#
#     +-----------------+     taken      +-----------------+
#     | Predict NOT TAKEN| ------------> |  Predict TAKEN   |
#     |    (bit = 0)     | <------------ |    (bit = 1)     |
#     +-----------------+   not taken    +-----------------+

module CodingAdventures
  module BranchPredictor
    # 1-bit predictor -- one flip-flop per branch address.
    #
    # Maintains a table of 1-bit entries indexed by (pc % table_size).
    # Each entry remembers the LAST outcome of that branch. Every misprediction
    # flips the bit. This is too aggressive -- a single anomalous outcome
    # changes the prediction. The 2-bit predictor adds hysteresis to fix this.
    #
    # @param table_size [Integer] number of entries in the prediction table.
    #   Must be a power of 2 for efficient hardware. Default: 1024 = 128 bytes.
    #
    # @example
    #   predictor = OneBitPredictor.new(table_size: 1024)
    #   pred = predictor.predict(pc: 0x100)
    #   pred.taken? # => false (cold start)
    #   predictor.update(pc: 0x100, taken: true)
    #   pred = predictor.predict(pc: 0x100)
    #   pred.taken? # => true (remembers last outcome)
    class OneBitPredictor
      # @param table_size [Integer] number of entries in the prediction table
      def initialize(table_size: 1024)
        # In hardware, this is the number of rows in a small SRAM.
        @table_size = table_size

        # Maps (index) -> last_outcome. We use a hash rather than an array
        # to avoid pre-allocating entries that are never accessed.
        # In hardware, all entries exist physically but start at 0 (not-taken).
        @table = {}

        @stats = PredictionStats.new
      end

      # Predict based on the last outcome of this branch.
      #
      # On a cold start (branch not yet seen), defaults to NOT TAKEN.
      # This is a common design choice -- the bit starts at 0.
      #
      # @param pc [Integer] the program counter of the branch instruction
      # @return [Prediction]
      def predict(pc:)
        index = pc % @table_size
        taken = @table.fetch(index, false)
        # Confidence: 0.5 because we only have 1 bit of history.
        Prediction.new(taken: taken, confidence: 0.5)
      end

      # Update the prediction table with the actual outcome.
      #
      # Simply sets the bit to match the actual outcome. This is the "flip"
      # that gives the 1-bit predictor its characteristic behavior.
      #
      # @param pc [Integer] the program counter
      # @param taken [Boolean] whether the branch was actually taken
      # @param target [Integer, nil] the actual target address (unused)
      def update(pc:, taken:, target: nil) # rubocop:disable Lint/UnusedMethodArgument
        index = pc % @table_size
        # Record accuracy BEFORE updating the table
        predicted = @table.fetch(index, false)
        @stats.record(correct: predicted == taken)
        # Now update the table to remember this outcome for next time
        @table[index] = taken
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
    end
  end
end

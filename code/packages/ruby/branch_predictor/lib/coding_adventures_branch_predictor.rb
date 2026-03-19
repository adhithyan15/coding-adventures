# frozen_string_literal: true

# ─── Branch Predictor ─────────────────────────────────────────────────────────
#
# This is the top-level entry point for the branch predictor gem. It loads all
# the predictor implementations in dependency order:
#
#   1. Prediction  -- the value object returned by every predictor
#   2. Stats       -- the accuracy tracker used by every predictor
#   3. Static      -- AlwaysTaken, AlwaysNotTaken, BTFNT (no learning)
#   4. OneBit      -- simplest dynamic predictor (1-bit per branch)
#   5. TwoBit      -- classic 2-bit saturating counter (solves double-mispredict)
#   6. BTB         -- Branch Target Buffer (caches WHERE branches go)
#
# Usage:
#   require "coding_adventures_branch_predictor"
#   predictor = CodingAdventures::BranchPredictor::TwoBitPredictor.new
#   pred = predictor.predict(pc: 0x100)

require_relative "coding_adventures/branch_predictor/version"
require_relative "coding_adventures/branch_predictor/prediction"
require_relative "coding_adventures/branch_predictor/stats"
require_relative "coding_adventures/branch_predictor/static_predictor"
require_relative "coding_adventures/branch_predictor/one_bit"
require_relative "coding_adventures/branch_predictor/two_bit"
require_relative "coding_adventures/branch_predictor/btb"

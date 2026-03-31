-- coding_adventures/branch_predictor/init.lua — Branch Prediction Algorithms
--
-- # Branch Prediction
--
-- A branch predictor guesses the outcome of branch instructions (if/else,
-- loops, function returns) BEFORE the branch condition is evaluated. This
-- sounds absurd — why guess when you can just wait? — but it is one of the
-- most critical performance features in modern CPUs.
--
-- ## The Pipeline Problem
--
-- In a 5-stage pipeline, by the time the CPU knows whether a branch is
-- taken (end of Execute stage), it has already fetched 2 more instructions.
-- If the branch IS taken, those 2 instructions are WRONG and must be
-- discarded — a pipeline flush that wastes 2 cycles.
--
-- In a 13-stage pipeline (like ARM Cortex-A78), a misprediction wastes
-- ~11 cycles. A good branch predictor is 95-99% correct, turning that
-- potential 11-cycle penalty into a ~0.1-cycle average cost.
--
-- ## Package Structure
--
--   branch_predictor/
--     init.lua       — this file: re-exports public API
--     stats.lua      — accuracy statistics (predictions, correct, accuracy%)
--     prediction.lua — Prediction result type (taken, confidence, address)
--     static.lua     — Static predictors: AlwaysTaken, AlwaysNotTaken, BTFNT
--     one_bit.lua    — 1-bit predictor (single flip-flop per branch)
--     two_bit.lua    — 2-bit saturating counter (the classic)
--     btb.lua        — Branch Target Buffer (caches where branches go)
--
-- ## Quick Start
--
--   local bp = require("coding_adventures.branch_predictor")
--
--   -- Create a 2-bit predictor
--   local pred = bp.TwoBit.new()
--   local prediction, pred = pred:predict(0x100)
--   print(prediction.predicted_taken)  -- false (initial state WNT)
--   pred = pred:update(0x100, true)    -- branch was taken
--   prediction, pred = pred:predict(0x100)
--   print(prediction.predicted_taken)  -- true (now WT after 1 taken)
--
--   -- Use a BTB alongside the predictor
--   local btb = bp.BTB.new(64)
--   btb = btb:update(0x100, 0x200)    -- branch at 0x100 goes to 0x200
--   local target, btb = btb:lookup(0x100)
--   print(target)  -- 0x200

local stats_mod  = require("coding_adventures.branch_predictor.stats")
local pred_mod   = require("coding_adventures.branch_predictor.prediction")
local static_mod = require("coding_adventures.branch_predictor.static")
local one_bit    = require("coding_adventures.branch_predictor.one_bit")
local two_bit    = require("coding_adventures.branch_predictor.two_bit")
local btb_mod    = require("coding_adventures.branch_predictor.btb")

return {
    VERSION = "0.1.0",

    -- Types
    Stats            = stats_mod,
    Prediction       = pred_mod,

    -- Static predictors (no learning, no state)
    AlwaysTaken      = static_mod.AlwaysTaken,
    AlwaysNotTaken   = static_mod.AlwaysNotTaken,
    BTFNT            = static_mod.BTFNT,

    -- Dynamic predictors (learn from branch history)
    OneBit           = one_bit,
    TwoBit           = two_bit.TwoBit,

    -- Target predictor
    BTB              = btb_mod,

    -- State constants for 2-bit predictor (useful for tests)
    SNT              = two_bit.SNT,
    WNT              = two_bit.WNT,
    WT               = two_bit.WT,
    ST               = two_bit.ST,
}

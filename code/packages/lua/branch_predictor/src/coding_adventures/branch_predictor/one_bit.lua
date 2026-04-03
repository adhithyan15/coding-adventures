-- branch_predictor/one_bit.lua — 1-bit branch predictor
--
-- The one-bit predictor is the simplest dynamic predictor. Unlike static
-- predictors, it LEARNS from branch history. Each branch address maps to
-- a single bit of state that records the last outcome:
--
--     bit = false  →  predict NOT TAKEN
--     bit = true   →  predict TAKEN
--
-- After each branch resolves, the bit is updated to match the actual
-- outcome. "Predict whatever happened last time."
--
-- ## State Machine
--
-- This predictor IS a 2-state Deterministic Finite Automaton:
--
--     +-------------+   taken    +-------------+
--     |  not_taken  | ---------> |    taken    |
--     |  (predict   | <--------- |  (predict   |
--     |  not taken) |  not_taken |    taken)   |
--     +-------------+            +-------------+
--           |  ^                       |  ^
--           |  | not_taken             |  | taken
--           +--+                       +--+
--
-- ## The double-misprediction problem
--
-- For a loop of N iterations: T T T ... T N  T T T ... T N
-- The 1-bit predictor mispredicts TWICE per invocation:
--   1. On entry (if last exit set bit to "not taken")
--   2. On exit (bit says "taken", but the loop ends)
--
-- The 2-bit predictor solves this by requiring TWO consecutive
-- mispredictions before changing its mind.
--
-- ## Hardware
--
-- A small SRAM table indexed by the lower bits of the PC.
-- Each entry is a single flip-flop (1 bit of storage).
-- For a 1024-entry table: 1024 bits = 128 bytes of hardware.

local Stats      = require("coding_adventures.branch_predictor.stats")
local Prediction = require("coding_adventures.branch_predictor.prediction")

local OneBit = {}
OneBit.__index = OneBit

-- Create a new 1-bit predictor.
--
-- Parameters:
--   table_size  (number) — number of entries in the prediction table
--                         (default: 1024). In hardware, a power of 2.
function OneBit.new(table_size)
    table_size = table_size or 1024
    return setmetatable({
        table_size = table_size,
        _table     = {},        -- map: index -> boolean (taken prediction)
        stats      = Stats.new(),
    }, OneBit)
end

-- Compute the table index for a PC address.
-- Uses modulo to fold the full 32/64-bit PC into the table size.
-- Two different PCs that share the same low bits will ALIAS to the
-- same entry — this is a known limitation of direct-indexed predictors.
local function index_of(predictor, pc)
    return pc % predictor.table_size
end

-- Predict based on the last outcome of this branch.
--
-- Cold start: first prediction for any PC defaults to NOT TAKEN
-- (the bit starts at 0 = false).
--
-- Returns: prediction, predictor (predictor unchanged during predict)
function OneBit:predict(pc)
    local idx   = index_of(self, pc)
    local taken = self._table[idx] or false
    return Prediction.new(taken, 0.5), self
end

-- Update the prediction table with the actual outcome.
--
-- The new state IS the outcome: "remember what just happened."
-- Records accuracy BEFORE updating so we measure what was actually
-- predicted (not what we'll predict next time).
--
-- Returns: new OneBit predictor with updated table and stats.
function OneBit:update(pc, taken, _target)
    local idx       = index_of(self, pc)
    local predicted = self._table[idx] or false

    -- Record accuracy before flipping the bit
    local new_stats = self.stats:record(predicted == taken)

    -- New state: the actual outcome (1-bit = always predict last outcome)
    local new_table = {}
    for k, v in pairs(self._table) do new_table[k] = v end
    new_table[idx] = taken

    return setmetatable({
        table_size = self.table_size,
        _table     = new_table,
        stats      = new_stats,
    }, OneBit)
end

function OneBit:get_stats() return self.stats end

-- Reset the prediction table and statistics.
function OneBit:reset()
    return OneBit.new(self.table_size)
end

return OneBit

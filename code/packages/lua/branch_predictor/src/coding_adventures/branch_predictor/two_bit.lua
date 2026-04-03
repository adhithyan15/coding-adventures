-- branch_predictor/two_bit.lua — 2-bit saturating counter predictor
--
-- The two-bit predictor improves on the one-bit predictor by adding
-- HYSTERESIS. Instead of flipping the prediction on every misprediction,
-- it takes TWO consecutive mispredictions to change the predicted direction.
--
-- ## The Four States
--
--     SNT (00) — Strongly Not Taken    → predict NOT TAKEN
--     WNT (01) — Weakly Not Taken      → predict NOT TAKEN
--     WT  (10) — Weakly Taken          → predict TAKEN
--     ST  (11) — Strongly Taken        → predict TAKEN
--
-- State diagram (saturating counter):
--
--   "taken" outcome moves RIGHT (increment, saturate at ST):
--     SNT -> WNT -> WT -> ST -> ST (capped)
--
--   "not_taken" outcome moves LEFT (decrement, saturate at SNT):
--     ST -> WT -> WNT -> SNT -> SNT (capped)
--
--   Prediction threshold: WT and ST → TAKEN; SNT and WNT → NOT TAKEN
--
-- ## Why hysteresis helps
--
-- Consider a loop of 10 iterations running twice:
--
--   First invocation:
--     Iter  1: WNT → predict NOT TAKEN → actual TAKEN  → WRONG → WT
--     Iter  2: WT  → predict TAKEN     → actual TAKEN  → correct → ST
--     ...
--     Iter  9: ST  → predict TAKEN     → actual TAKEN  → correct
--     Iter 10: ST  → predict TAKEN     → actual NOT TAKEN → WRONG → WT
--
--   Second invocation:
--     Iter  1: WT  → predict TAKEN     → actual TAKEN  → correct!
--
--   Only 1 misprediction on loop re-entry (vs 2 for 1-bit predictor).
--   The "weakly taken" state acts as a buffer — a single "not taken"
--   doesn't immediately flip the prediction.
--
-- ## Historical usage
--
--   Alpha 21064: 2-bit counters with 2048 entries
--   Intel Pentium: 2-bit counters with 256 entries
--   MIPS R10000: 2-bit counters as base predictor in tournament scheme
--
-- ## Confidence
--
-- Strong states (SNT, ST) → confidence 1.0
-- Weak states  (WNT, WT)  → confidence 0.5
-- Tournament predictors use confidence to pick the best sub-predictor.

local Stats      = require("coding_adventures.branch_predictor.stats")
local Prediction = require("coding_adventures.branch_predictor.prediction")

-- State constants
local SNT = "SNT"  -- Strongly Not Taken (00)
local WNT = "WNT"  -- Weakly Not Taken   (01)
local WT  = "WT"   -- Weakly Taken       (10)
local ST  = "ST"   -- Strongly Taken     (11)

-- Transition table: state x outcome -> next state
-- Encodes the saturating counter logic.
local TAKEN_TRANSITION = {
    [SNT] = WNT,
    [WNT] = WT,
    [WT]  = ST,
    [ST]  = ST,   -- saturate
}
local NOT_TAKEN_TRANSITION = {
    [ST]  = WT,
    [WT]  = WNT,
    [WNT] = SNT,
    [SNT] = SNT,  -- saturate
}

-- States that predict TAKEN (the "accepting" states in DFA terms)
local PREDICTS_TAKEN = { [WT] = true, [ST] = true }

local TwoBit = {}
TwoBit.__index = TwoBit

-- Create a new 2-bit predictor.
--
-- Parameters:
--   table_size     (number) — number of entries (default: 1024)
--   initial_state  (string) — starting state for all entries (default: "WNT")
--     "WNT": conservative, requires 1 taken outcome to start predicting taken
--     "WT":  optimistic, starts predicting taken immediately
function TwoBit.new(table_size, initial_state)
    table_size    = table_size    or 1024
    initial_state = initial_state or WNT
    return setmetatable({
        table_size    = table_size,
        initial_state = initial_state,
        _table        = {},        -- map: index -> state string
        stats         = Stats.new(),
    }, TwoBit)
end

local function index_of(predictor, pc)
    return pc % predictor.table_size
end

local function get_state(predictor, idx)
    return predictor._table[idx] or predictor.initial_state
end

-- Predict based on the 2-bit counter for this branch.
-- Returns: prediction, predictor (predictor unchanged during predict)
function TwoBit:predict(pc)
    local idx   = index_of(self, pc)
    local state = get_state(self, idx)
    local taken = PREDICTS_TAKEN[state] == true

    local confidence
    if state == ST or state == SNT then
        confidence = 1.0   -- strong states = high confidence
    else
        confidence = 0.5   -- weak states = low confidence
    end

    return Prediction.new(taken, confidence), self
end

-- Update the 2-bit counter based on the actual outcome.
-- Increments on taken, decrements on not-taken, saturating at boundaries.
function TwoBit:update(pc, taken, _target)
    local idx   = index_of(self, pc)
    local state = get_state(self, idx)

    -- Record accuracy BEFORE updating
    local predicted = PREDICTS_TAKEN[state] == true
    local new_stats = self.stats:record(predicted == taken)

    -- Transition the saturating counter
    local next_state
    if taken then
        next_state = TAKEN_TRANSITION[state]
    else
        next_state = NOT_TAKEN_TRANSITION[state]
    end

    local new_table = {}
    for k, v in pairs(self._table) do new_table[k] = v end
    new_table[idx] = next_state

    return setmetatable({
        table_size    = self.table_size,
        initial_state = self.initial_state,
        _table        = new_table,
        stats         = new_stats,
    }, TwoBit)
end

function TwoBit:get_stats() return self.stats end

-- Inspect the current state for a branch address (for testing/debugging).
-- Returns the state string: "SNT", "WNT", "WT", or "ST".
function TwoBit:get_state_for_pc(pc)
    return get_state(self, index_of(self, pc))
end

function TwoBit:reset()
    return TwoBit.new(self.table_size, self.initial_state)
end

-- Export state constants for tests and other modules
return {
    TwoBit = TwoBit,
    SNT = SNT,
    WNT = WNT,
    WT  = WT,
    ST  = ST,
}

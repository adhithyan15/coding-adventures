-- branch_predictor/static.lua — Static predictors (no learning)
--
-- Static predictors make the same prediction every time, regardless of
-- branch history. They require zero hardware (no tables, no counters) and
-- serve as baselines against which dynamic predictors are measured.
--
-- Three strategies:
--
--   AlwaysTaken      — always predicts taken  (~60-70% accurate)
--   AlwaysNotTaken   — always predicts not-taken (~30-40% accurate)
--   BTFNT            — Backward Taken, Forward Not Taken (~65-75% accurate)
--
-- ## Why static predictors matter
--
-- The Intel 8086 (1978) had no branch predictor — it effectively used
-- "always not taken" by continuing to fetch sequentially. The MIPS R4000
-- (1991) used BTFNT as its primary strategy. Even today, static predictors
-- are used in embedded processors where die area is at a premium.

local Stats      = require("coding_adventures.branch_predictor.stats")
local Prediction = require("coding_adventures.branch_predictor.prediction")

-- =========================================================================
-- AlwaysTaken — the optimistic predictor
-- =========================================================================
--
-- Always predicts "taken" regardless of the branch address.
--
-- Why it works:
--   Most branches in real programs are loop back-edges, taken on every
--   iteration except the last. A 100-iteration loop has 99 taken + 1
--   not-taken = 99% accuracy on that loop. The overall ~60% comes from
--   mixing loops with if-else branches.
--
-- Hardware cost: ZERO. The prediction logic is just a wire tied to 1.

local AlwaysTaken = {}
AlwaysTaken.__index = AlwaysTaken

function AlwaysTaken.new()
    return setmetatable({ stats = Stats.new() }, AlwaysTaken)
end

-- Predict: always taken (pc is ignored).
-- Returns {prediction, predictor} — predictor is unchanged during prediction.
function AlwaysTaken:predict(_pc)
    return Prediction.new(true, 0.0), self
end

-- Update: record whether our "always taken" guess was right.
-- We were correct when the branch was actually taken.
function AlwaysTaken:update(_pc, taken, _target)
    return setmetatable({ stats = self.stats:record(taken == true) }, AlwaysTaken)
end

function AlwaysTaken:get_stats() return self.stats end

function AlwaysTaken:reset()
    return AlwaysTaken.new()
end

-- =========================================================================
-- AlwaysNotTaken — the pessimistic predictor (the baseline)
-- =========================================================================
--
-- Always predicts "not taken". This is the baseline that every other
-- predictor must beat to justify its hardware cost.
--
-- Hardware advantage: the "not taken" path is just PC+instruction_size,
-- which the fetch unit is ALREADY computing. No target address calculation
-- needed. This is why early processors implicitly used this strategy.

local AlwaysNotTaken = {}
AlwaysNotTaken.__index = AlwaysNotTaken

function AlwaysNotTaken.new()
    return setmetatable({ stats = Stats.new() }, AlwaysNotTaken)
end

function AlwaysNotTaken:predict(_pc)
    return Prediction.new(false, 0.0), self
end

function AlwaysNotTaken:update(_pc, taken, _target)
    -- Correct when the branch was NOT taken (predicted not-taken, actual not-taken)
    return setmetatable({ stats = self.stats:record(taken == false) }, AlwaysNotTaken)
end

function AlwaysNotTaken:get_stats() return self.stats end

function AlwaysNotTaken:reset()
    return AlwaysNotTaken.new()
end

-- =========================================================================
-- BTFNT — Backward Taken, Forward Not Taken
-- =========================================================================
--
-- Uses the branch direction as a heuristic:
--
--   Backward branch (target <= pc): predict TAKEN
--     → Usually a loop back-edge. Loops are taken N-1 out of N times.
--
--   Forward branch (target > pc): predict NOT TAKEN
--     → Usually an if-then-else. The "else" path is often skipped.
--
-- Historical usage:
--   MIPS R4000 (1991), SPARC V8 (1992), early ARM processors
--
-- Cold start: on the first encounter of a branch, we don't know the
-- target yet, so we default to "not taken" and remember the target
-- from the update call.

local BTFNT = {}
BTFNT.__index = BTFNT

function BTFNT.new()
    return setmetatable({
        targets = {},   -- map: pc -> last known target
        stats   = Stats.new(),
    }, BTFNT)
end

-- Predict based on branch direction.
-- If no target is known yet (cold start), default to not-taken.
function BTFNT:predict(pc)
    local target = self.targets[pc]
    if target == nil then
        -- Cold start — default to not-taken (safe fallback)
        return Prediction.new(false, 0.0), self
    end
    -- Backward (target <= pc) -> taken (loop back-edge)
    -- Forward  (target > pc)  -> not taken (if-else)
    local taken = target <= pc
    return Prediction.new(taken, 0.5, target), self
end

-- Update: record the actual outcome and learn the target address.
function BTFNT:update(pc, taken, target)
    -- Remember the target for future direction-based predictions
    local new_targets = {}
    for k, v in pairs(self.targets) do new_targets[k] = v end
    if target ~= nil then
        new_targets[pc] = target
    end

    -- What would we have predicted with the updated target knowledge?
    local known_target = new_targets[pc]
    local predicted_taken
    if known_target == nil then
        predicted_taken = false
    else
        predicted_taken = known_target <= pc
    end

    local new_stats = self.stats:record(predicted_taken == taken)
    return setmetatable({ targets = new_targets, stats = new_stats }, BTFNT)
end

function BTFNT:get_stats() return self.stats end

function BTFNT:reset()
    return BTFNT.new()
end

return {
    AlwaysTaken    = AlwaysTaken,
    AlwaysNotTaken = AlwaysNotTaken,
    BTFNT          = BTFNT,
}

-- branch_predictor/stats.lua — Prediction accuracy tracking
--
-- Every branch predictor needs a scorecard. When a CPU designer evaluates
-- a predictor, the first question is always: "What's the accuracy?"
-- A predictor that is 95% accurate causes a pipeline flush on only 5%
-- of branches, while a 70% accurate predictor flushes on 30% — potentially
-- halving throughput on a deeply pipelined machine.
--
-- ## Counters
--
-- We track three simple counters:
--   predictions  — total number of branches seen
--   correct      — how many the predictor got right
--   incorrect    — how many it got wrong
--
-- From these we derive:
--   accuracy            — correct / predictions × 100  (as a percentage)
--   misprediction_rate  — incorrect / predictions × 100 (the complement)
--
-- ## Real-world context
--
--   Intel Pentium Pro  : ~90% accuracy with two-level adaptive predictor
--   Modern CPUs (2015+): 95-99% accuracy using TAGE or perceptron predictors
--   A 1% improvement   : measurable speedup on branch-heavy workloads

local Stats = {}
Stats.__index = Stats

-- Create a new Stats object with all counters at zero.
--
-- Usage:
--   local s = Stats.new()
--   s = s:record(true)   -- correct prediction
--   s = s:record(false)  -- wrong prediction
--   print(s:accuracy())  -- 50.0
function Stats.new()
    return setmetatable({
        predictions = 0,
        correct     = 0,
        incorrect   = 0,
    }, Stats)
end

-- Record the outcome of a single prediction.
--
-- Parameters:
--   correct_guess  (boolean) — true if the predictor guessed right
--
-- Returns a NEW Stats object with updated counters (immutable style).
function Stats:record(correct_guess)
    local s = Stats.new()
    s.predictions = self.predictions + 1
    if correct_guess then
        s.correct   = self.correct + 1
        s.incorrect = self.incorrect
    else
        s.correct   = self.correct
        s.incorrect = self.incorrect + 1
    end
    return s
end

-- Prediction accuracy as a percentage (0.0 to 100.0).
-- Returns 0.0 if no predictions have been made yet.
function Stats:accuracy()
    if self.predictions == 0 then return 0.0 end
    return self.correct / self.predictions * 100.0
end

-- Misprediction rate as a percentage (0.0 to 100.0).
-- This is 100 - accuracy. CPU architects think in misprediction rate
-- because each misprediction causes a concrete pipeline flush.
function Stats:misprediction_rate()
    if self.predictions == 0 then return 0.0 end
    return self.incorrect / self.predictions * 100.0
end

-- Reset all counters to zero.
function Stats:reset()
    return Stats.new()
end

return Stats

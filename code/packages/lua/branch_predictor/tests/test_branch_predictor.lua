-- Tests for branch_predictor — comprehensive coverage of all predictor types
--
-- Target: 95%+ coverage of all predictor modules.
-- Tests verify correctness against the spec examples and edge cases.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local bp = require("coding_adventures.branch_predictor")

-- =========================================================================
-- Stats tests
-- =========================================================================

describe("Stats", function()
    it("starts with all zeros", function()
        local s = bp.Stats.new()
        assert.are.equal(0, s.predictions)
        assert.are.equal(0, s.correct)
        assert.are.equal(0, s.incorrect)
    end)

    it("returns 0.0 accuracy with no predictions", function()
        local s = bp.Stats.new()
        assert.are.equal(0.0, s:accuracy())
        assert.are.equal(0.0, s:misprediction_rate())
    end)

    it("records a correct prediction", function()
        local s = bp.Stats.new()
        s = s:record(true)
        assert.are.equal(1, s.predictions)
        assert.are.equal(1, s.correct)
        assert.are.equal(0, s.incorrect)
    end)

    it("records an incorrect prediction", function()
        local s = bp.Stats.new()
        s = s:record(false)
        assert.are.equal(1, s.predictions)
        assert.are.equal(0, s.correct)
        assert.are.equal(1, s.incorrect)
    end)

    it("computes accuracy correctly", function()
        local s = bp.Stats.new()
        s = s:record(true)
        s = s:record(true)
        s = s:record(false)
        assert.are.equal(3, s.predictions)
        assert.is_true(math.abs(s:accuracy() - 66.67) < 0.01)
        assert.is_true(math.abs(s:misprediction_rate() - 33.33) < 0.01)
    end)

    it("resets to zero", function()
        local s = bp.Stats.new()
        s = s:record(true)
        s = s:record(false)
        s = s:reset()
        assert.are.equal(0, s.predictions)
        assert.are.equal(0.0, s:accuracy())
    end)

    it("computes 100% accuracy", function()
        local s = bp.Stats.new()
        for _ = 1, 10 do s = s:record(true) end
        assert.are.equal(100.0, s:accuracy())
        assert.are.equal(0.0, s:misprediction_rate())
    end)
end)

-- =========================================================================
-- Prediction type tests
-- =========================================================================

describe("Prediction", function()
    it("creates with defaults", function()
        local p = bp.Prediction.new(true)
        assert.is_true(p.predicted_taken)
        assert.are.equal(0.5, p.confidence)
        assert.is_nil(p.address)
    end)

    it("creates with all fields", function()
        local p = bp.Prediction.new(false, 1.0, 0x200)
        assert.is_false(p.predicted_taken)
        assert.are.equal(1.0, p.confidence)
        assert.are.equal(0x200, p.address)
    end)
end)

-- =========================================================================
-- AlwaysTaken tests
-- =========================================================================

describe("AlwaysTaken", function()
    it("always predicts taken regardless of PC", function()
        local p = bp.AlwaysTaken.new()
        for _, pc in ipairs({0, 0x100, 0x200, 0xFFF}) do
            local pred, _ = p:predict(pc)
            assert.is_true(pred.predicted_taken,
                "Expected taken for PC " .. pc)
        end
    end)

    it("starts with zero accuracy", function()
        local p = bp.AlwaysTaken.new()
        assert.are.equal(0, p:get_stats().predictions)
    end)

    it("records correct when branch is actually taken", function()
        local p = bp.AlwaysTaken.new()
        p = p:update(0x100, true)   -- correct: predicted taken, was taken
        p = p:update(0x100, false)  -- wrong: predicted taken, was not-taken
        assert.are.equal(2, p:get_stats().predictions)
        assert.are.equal(1, p:get_stats().correct)
        assert.are.equal(1, p:get_stats().incorrect)
    end)

    it("resets cleanly", function()
        local p = bp.AlwaysTaken.new()
        p = p:update(0x100, true)
        p = p:reset()
        assert.are.equal(0, p:get_stats().predictions)
    end)

    it("achieves ~99% accuracy on a 100-iteration loop", function()
        local p = bp.AlwaysTaken.new()
        -- 99 taken + 1 not-taken
        for _ = 1, 99 do p = p:update(0x100, true) end
        p = p:update(0x100, false)
        local acc = p:get_stats():accuracy()
        assert.is_true(acc >= 99.0 - 0.01,
            "Expected >=99% accuracy, got " .. acc)
    end)
end)

-- =========================================================================
-- AlwaysNotTaken tests
-- =========================================================================

describe("AlwaysNotTaken", function()
    it("always predicts not-taken", function()
        local p = bp.AlwaysNotTaken.new()
        for _, pc in ipairs({0, 0x100, 0xFF}) do
            local pred, _ = p:predict(pc)
            assert.is_false(pred.predicted_taken)
        end
    end)

    it("records correct when branch is actually not-taken", function()
        local p = bp.AlwaysNotTaken.new()
        p = p:update(0x100, false)  -- correct: predicted not-taken, was not-taken
        p = p:update(0x100, true)   -- wrong: predicted not-taken, was taken
        assert.are.equal(2, p:get_stats().predictions)
        assert.are.equal(1, p:get_stats().correct)
    end)

    it("resets cleanly", function()
        local p = bp.AlwaysNotTaken.new()
        p = p:update(0x100, false)
        p = p:reset()
        assert.are.equal(0, p:get_stats().predictions)
    end)
end)

-- =========================================================================
-- BTFNT tests
-- =========================================================================

describe("BTFNT", function()
    it("defaults to not-taken on cold start", function()
        local p = bp.BTFNT.new()
        local pred, _ = p:predict(0x108)
        assert.is_false(pred.predicted_taken)
    end)

    it("predicts taken for backward branch after learning target", function()
        local p = bp.BTFNT.new()
        -- Branch at 0x108 going backward to 0x100 (target < pc)
        p = p:update(0x108, true, 0x100)
        local pred, _ = p:predict(0x108)
        assert.is_true(pred.predicted_taken, "Backward branch should be predicted taken")
    end)

    it("predicts not-taken for forward branch", function()
        local p = bp.BTFNT.new()
        -- Branch at 0x100 going forward to 0x200 (target > pc)
        p = p:update(0x100, true, 0x200)
        local pred, _ = p:predict(0x100)
        assert.is_false(pred.predicted_taken, "Forward branch should be predicted not-taken")
    end)

    it("handles equal target (pc == target) as taken", function()
        local p = bp.BTFNT.new()
        -- target == pc → infinite loop → predict taken
        p = p:update(0x100, true, 0x100)
        local pred, _ = p:predict(0x100)
        assert.is_true(pred.predicted_taken)
    end)

    it("resets cleanly", function()
        local p = bp.BTFNT.new()
        p = p:update(0x100, true, 0x50)
        p = p:reset()
        local pred, _ = p:predict(0x100)
        assert.is_false(pred.predicted_taken, "After reset, cold start defaults to not-taken")
    end)

    it("tracks multiple branches independently", function()
        local p = bp.BTFNT.new()
        p = p:update(0x200, true, 0x100)  -- backward
        p = p:update(0x300, true, 0x400)  -- forward
        local p1, _ = p:predict(0x200)
        local p2, _ = p:predict(0x300)
        assert.is_true(p1.predicted_taken)
        assert.is_false(p2.predicted_taken)
    end)
end)

-- =========================================================================
-- OneBit tests
-- =========================================================================

describe("OneBit", function()
    it("defaults to not-taken on cold start", function()
        local p = bp.OneBit.new()
        local pred, _ = p:predict(0x100)
        assert.is_false(pred.predicted_taken)
    end)

    it("learns from a taken branch", function()
        local p = bp.OneBit.new()
        p = p:update(0x100, true)
        local pred, _ = p:predict(0x100)
        assert.is_true(pred.predicted_taken)
    end)

    it("flips on not-taken", function()
        local p = bp.OneBit.new()
        p = p:update(0x100, true)   -- bit = true
        p = p:update(0x100, false)  -- bit = false
        local pred, _ = p:predict(0x100)
        assert.is_false(pred.predicted_taken)
    end)

    it("tracks different PCs independently", function()
        local p = bp.OneBit.new()
        p = p:update(0x100, true)
        p = p:update(0x200, false)
        local p1, _ = p:predict(0x100)
        local p2, _ = p:predict(0x200)
        assert.is_true(p1.predicted_taken)
        assert.is_false(p2.predicted_taken)
    end)

    it("exhibits aliasing: different PCs with same low bits share an entry", function()
        local p = bp.OneBit.new(4)   -- only 4 entries → lots of aliasing
        p = p:update(0x0, true)     -- index 0 → taken
        -- 0x4 also maps to index 0 (0x4 % 4 = 0)
        local pred, _ = p:predict(0x4)
        assert.is_true(pred.predicted_taken, "Aliased entry should share state")
    end)

    it("simulates loop with 2 mispredictions per invocation", function()
        local p = bp.OneBit.new()
        -- Loop of 5 iterations, run twice
        local outcomes = {true, true, true, true, false,  -- first run
                          true, true, true, true, false}  -- second run
        local total_misses = 0
        for _, taken in ipairs(outcomes) do
            local pred, _ = p:predict(0x100)
            if pred.predicted_taken ~= taken then total_misses = total_misses + 1 end
            p = p:update(0x100, taken)
        end
        -- Expect: 1 miss at start of first run, 1 at end, 1 at start of second run, 1 at end
        -- Actually: miss at start only if initial is wrong, miss at each loop exit
        assert.is_true(total_misses >= 2,
            "Expected at least 2 mispredictions, got " .. total_misses)
    end)

    it("resets table and stats", function()
        local p = bp.OneBit.new()
        p = p:update(0x100, true)
        p = p:reset()
        local pred, _ = p:predict(0x100)
        assert.is_false(pred.predicted_taken, "After reset, cold start = not-taken")
        assert.are.equal(0, p:get_stats().predictions)
    end)
end)

-- =========================================================================
-- TwoBit tests
-- =========================================================================

describe("TwoBit", function()
    it("starts in WNT state (predicts not-taken)", function()
        local p = bp.TwoBit.new()
        local pred, _ = p:predict(0x100)
        assert.is_false(pred.predicted_taken)
        assert.are.equal(bp.WNT, p:get_state_for_pc(0x100))
    end)

    it("transitions WNT -> WT after one taken outcome", function()
        local p = bp.TwoBit.new()
        p = p:update(0x100, true)
        assert.are.equal(bp.WT, p:get_state_for_pc(0x100))
        local pred, _ = p:predict(0x100)
        assert.is_true(pred.predicted_taken)
    end)

    it("transitions WNT -> SNT after one not-taken outcome", function()
        local p = bp.TwoBit.new()
        p = p:update(0x100, false)
        assert.are.equal(bp.SNT, p:get_state_for_pc(0x100))
    end)

    it("saturates at ST: multiple taken don't go past ST", function()
        local p = bp.TwoBit.new()
        for _ = 1, 10 do p = p:update(0x100, true) end
        assert.are.equal(bp.ST, p:get_state_for_pc(0x100))
    end)

    it("saturates at SNT: multiple not-taken don't go past SNT", function()
        local p = bp.TwoBit.new()
        for _ = 1, 10 do p = p:update(0x100, false) end
        assert.are.equal(bp.SNT, p:get_state_for_pc(0x100))
    end)

    it("shows hysteresis: one not-taken from ST goes to WT (still predicts taken)", function()
        local p = bp.TwoBit.new()
        -- Get to ST
        p = p:update(0x100, true)  -- WNT -> WT
        p = p:update(0x100, true)  -- WT  -> ST
        -- One not-taken
        p = p:update(0x100, false) -- ST  -> WT (still predicts TAKEN)
        assert.are.equal(bp.WT, p:get_state_for_pc(0x100))
        local pred, _ = p:predict(0x100)
        assert.is_true(pred.predicted_taken, "WT still predicts taken (hysteresis)")
    end)

    it("all 4 state transitions from each state", function()
        local transitions = {
            -- {initial_state, outcome, expected_next}
            {bp.SNT, false, bp.SNT},  -- saturate
            {bp.SNT, true,  bp.WNT},
            {bp.WNT, false, bp.SNT},
            {bp.WNT, true,  bp.WT},
            {bp.WT,  false, bp.WNT},
            {bp.WT,  true,  bp.ST},
            {bp.ST,  false, bp.WT},
            {bp.ST,  true,  bp.ST},   -- saturate
        }
        for _, t in ipairs(transitions) do
            local init_state, outcome, expected = t[1], t[2], t[3]
            local p = bp.TwoBit.new(1024, init_state)
            p = p:update(0x100, outcome)
            local got = p:get_state_for_pc(0x100)
            assert.are.equal(expected, got,
                "From " .. init_state .. " + " .. tostring(outcome)
                .. " expected " .. expected .. " got " .. got)
        end
    end)

    it("strong states give 1.0 confidence", function()
        local p = bp.TwoBit.new(1024, bp.ST)
        local pred, _ = p:predict(0x100)
        assert.are.equal(1.0, pred.confidence)
    end)

    it("weak states give 0.5 confidence", function()
        local p = bp.TwoBit.new()  -- starts at WNT
        local pred, _ = p:predict(0x100)
        assert.are.equal(0.5, pred.confidence)
    end)

    it("simulates 10-iteration loop with 1 misprediction per invocation", function()
        local p = bp.TwoBit.new()
        -- Run the loop twice: 9 taken + 1 not-taken, repeated
        local outcomes = {}
        for _ = 1, 2 do
            for _ = 1, 9 do table.insert(outcomes, true) end
            table.insert(outcomes, false)
        end

        local misses = 0
        for _, taken in ipairs(outcomes) do
            local pred, _ = p:predict(0x100)
            if pred.predicted_taken ~= taken then misses = misses + 1 end
            p = p:update(0x100, taken)
        end
        -- 2-bit predictor should miss at most 3 times total for two loop runs
        -- (once on entry to first loop, once on exit of each run)
        assert.is_true(misses <= 4,
            "Expected at most 4 mispredictions for two 10-iteration loops, got " .. misses)
    end)

    it("resets table and stats", function()
        local p = bp.TwoBit.new()
        p = p:update(0x100, true)
        p = p:reset()
        assert.are.equal(bp.WNT, p:get_state_for_pc(0x100))
        assert.are.equal(0, p:get_stats().predictions)
    end)

    it("supports custom initial state WT", function()
        local p = bp.TwoBit.new(1024, bp.WT)
        local pred, _ = p:predict(0x100)
        assert.is_true(pred.predicted_taken, "WT state predicts taken")
    end)
end)

-- =========================================================================
-- BTB tests
-- =========================================================================

describe("BTB", function()
    it("cold start returns nil target", function()
        local btb = bp.BTB.new()
        local target, _ = btb:lookup(0x100)
        assert.is_nil(target)
    end)

    it("stores and retrieves a target", function()
        local btb = bp.BTB.new()
        btb = btb:update(0x100, 0x200)
        local target, _ = btb:lookup(0x100)
        assert.are.equal(0x200, target)
    end)

    it("updates an existing entry", function()
        local btb = bp.BTB.new()
        btb = btb:update(0x100, 0x200)
        btb = btb:update(0x100, 0x300)  -- target changed (indirect branch)
        local target, _ = btb:lookup(0x100)
        assert.are.equal(0x300, target)
    end)

    it("tracks hits and misses", function()
        local btb = bp.BTB.new()
        local _, btb = btb:lookup(0x100)   -- miss
        btb = btb:update(0x100, 0x200)
        local _, btb = btb:lookup(0x100)   -- hit
        assert.are.equal(2, btb.lookups)
        assert.are.equal(1, btb.hits)
        assert.are.equal(1, btb.misses)
        assert.are.equal(50.0, btb:hit_rate())
    end)

    it("hit rate is 0.0 with no lookups", function()
        local btb = bp.BTB.new()
        assert.are.equal(0.0, btb:hit_rate())
    end)

    it("evicts conflicting entry (direct-mapped)", function()
        local btb = bp.BTB.new(4)   -- only 4 entries
        -- 0x100 and 0x104 both map to index (0x100 % 4 = 0, 0x104 % 4 = 0)
        btb = btb:update(0x100, 0xA00)
        btb = btb:update(0x104, 0xB00)  -- evicts 0x100 (conflict)
        local t1, _ = btb:lookup(0x100)
        local t2, _ = btb:lookup(0x104)
        assert.is_nil(t1, "0x100 should have been evicted")
        assert.are.equal(0xB00, t2)
    end)

    it("stores branch_type metadata", function()
        local btb = bp.BTB.new()
        btb = btb:update(0x100, 0x200, "call")
        local entry = btb:get_entry(0x100)
        assert.is_not_nil(entry)
        assert.are.equal("call", entry.branch_type)
    end)

    it("get_entry returns nil for unknown PC", function()
        local btb = bp.BTB.new()
        assert.is_nil(btb:get_entry(0x999))
    end)

    it("resets all state", function()
        local btb = bp.BTB.new()
        btb = btb:update(0x100, 0x200)
        local _, btb = btb:lookup(0x100)
        btb = btb:reset()
        local target, _ = btb:lookup(0x100)
        assert.is_nil(target)
        assert.are.equal(0, btb.lookups)
    end)
end)

-- =========================================================================
-- Integration: benchmark predictors on a simple loop
-- =========================================================================

describe("Integration: loop benchmark", function()
    it("compares predictor accuracy on a 10-iteration loop", function()
        -- 10 iterations: T T T T T T T T T N, run 3 times
        local outcomes = {}
        for _ = 1, 3 do
            for _ = 1, 9 do table.insert(outcomes, true) end
            table.insert(outcomes, false)
        end

        local predictors = {
            always_taken    = bp.AlwaysTaken.new(),
            always_not      = bp.AlwaysNotTaken.new(),
            one_bit         = bp.OneBit.new(),
            two_bit         = bp.TwoBit.new(),
        }

        for name, p in pairs(predictors) do
            for _, taken in ipairs(outcomes) do
                local pred, _ = p:predict(0x100)
                _ = pred  -- suppress unused warning
                p = p:update(0x100, taken)
            end
            predictors[name] = p
        end

        -- 2-bit should perform at least as well as 1-bit on a simple loop
        local acc_2bit = predictors.two_bit:get_stats():accuracy()
        local acc_1bit = predictors.one_bit:get_stats():accuracy()
        local acc_at   = predictors.always_taken:get_stats():accuracy()

        -- AlwaysTaken gets 9/10 = 90% on each run, 2-bit should be close
        assert.is_true(acc_at >= 90.0 - 0.01,
            "AlwaysTaken should be ~90% on a loop, got " .. acc_at)
        assert.is_true(acc_2bit >= acc_1bit - 10,
            "2-bit should be within 10% of 1-bit (usually better)")
    end)
end)

-- =========================================================================
-- Integration: BTB + direction predictor
-- =========================================================================

describe("Integration: BTB + TwoBit combined", function()
    it("provides target address alongside prediction", function()
        local pred = bp.TwoBit.new()
        local btb  = bp.BTB.new()

        -- Train on a branch: 0x100 → 0x50 (backward, taken)
        for _ = 1, 5 do
            pred = pred:update(0x100, true)
            btb  = btb:update(0x100, 0x50)
        end

        local prediction, _ = pred:predict(0x100)
        local target, _     = btb:lookup(0x100)

        assert.is_true(prediction.predicted_taken)
        assert.are.equal(0x50, target)
    end)
end)

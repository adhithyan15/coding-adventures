-- Tests for markov_chain — comprehensive busted test suite (DT28)
-- ================================================================
--
-- These tests cover all 10 spec-required test cases plus additional
-- edge cases for full coverage. Tests use the busted framework with
-- describe/it/assert style (mirrors the Go and TypeScript test suites).
--
-- Test organisation:
--   1. Construction — empty chain, version
--   2. Training — single pair, sequence, smoothing
--   3. Generation — length, string output, order-2
--   4. Stationary distribution — ergodic chain
--   5. Error conditions — unknown state
--   6. Multi-train accumulation

-- Add src/ to the module search path so we can require the package
-- without installing it via luarocks.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

-- Also make directed_graph available via the same src path trick.
-- The directed_graph package lives two levels up relative to the tests dir.
local directed_graph_src = "../../directed_graph/src/?.lua;" ..
                            "../../directed_graph/src/?/init.lua;"
package.path = directed_graph_src .. package.path

local mc_mod = require("coding_adventures.markov_chain")
local MarkovChain = mc_mod.MarkovChain

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Sum all values in a table.
-- Used to verify that probability rows and distributions sum to 1.
local function sum_values(t)
    local total = 0.0
    for _, v in pairs(t) do
        total = total + v
    end
    return total
end

--- Count elements in an array table.
local function count(t)
    local n = 0
    for _ in ipairs(t) do
        n = n + 1
    end
    return n
end

-- =========================================================================
-- Version
-- =========================================================================

describe("markov_chain module", function()
    it("has a version string", function()
        assert.are.equal("0.1.0", mc_mod.VERSION)
    end)
end)

-- =========================================================================
-- Test 1 — Construction
-- Spec: MarkovChain.new() creates an empty chain with 0 states.
-- =========================================================================

describe("MarkovChain construction", function()
    it("creates an empty chain with no states", function()
        -- An freshly constructed chain should have an empty state list.
        local chain = MarkovChain.new()
        assert.are.equal(0, count(chain:states()))
    end)

    it("defaults to order 1", function()
        local chain = MarkovChain.new()
        assert.are.equal(1, chain._order)
    end)

    it("defaults to smoothing 0.0", function()
        local chain = MarkovChain.new()
        assert.are.equal(0.0, chain._smoothing)
    end)

    it("accepts explicit order and smoothing", function()
        local chain = MarkovChain.new(2, 0.5)
        assert.are.equal(2, chain._order)
        assert.are.equal(0.5, chain._smoothing)
    end)

    it("pre-registers states from the states list", function()
        -- Pre-registering states is important for smoothing over an alphabet
        -- that may not fully appear in the training sequence.
        local chain = MarkovChain.new(1, 1.0, {"A", "B", "C"})
        local states = chain:states()
        assert.are.equal(3, count(states))
    end)

    it("has an empty transition matrix after construction", function()
        local chain = MarkovChain.new()
        local tm = chain:transition_matrix()
        local n = 0
        for _ in pairs(tm) do
            n = n + 1
        end
        assert.are.equal(0, n)
    end)
end)

-- =========================================================================
-- Test 2 — Train single pair
-- Spec: train on [A, B] (order=1). probability(A, B) == 1.0
-- =========================================================================

describe("MarkovChain training — single pair", function()
    it("assigns probability 1.0 to the only observed transition", function()
        -- When only one transition is observed from A (namely A→B),
        -- P(A→B) must be 1.0 because it's the only way out of A.
        local chain = MarkovChain.new()
        chain:train({"A", "B"})
        assert.are.equal(1.0, chain:probability("A", "B"))
    end)

    it("registers both states after training", function()
        local chain = MarkovChain.new()
        chain:train({"A", "B"})
        assert.are.equal(2, count(chain:states()))
    end)

    it("has probability 0.0 for unseen transition", function()
        local chain = MarkovChain.new()
        chain:train({"A", "B"})
        -- B has no outgoing transitions in this training set.
        assert.are.equal(0.0, chain:probability("B", "A"))
    end)
end)

-- =========================================================================
-- Test 3 — Train sequence
-- Spec: train on [A, B, A, C, A, B, B, A].
--   probability(A, B) ≈ 0.667, probability(A, C) ≈ 0.333
--   probability(B, A) ≈ 0.667, probability(B, B) ≈ 0.333
-- =========================================================================

describe("MarkovChain training — sequence", function()
    local chain

    -- Create and train a fresh chain before each test in this block.
    -- Sequence: A B A C A B B A
    -- Transitions observed:
    --   A→B: 2  (positions 1→2, 5→6)
    --   B→A: 2  (positions 2→3, 6→7 counting from 1)
    --   A→C: 1  (positions 3→4)
    --   C→A: 1  (positions 4→5)
    --   A→B: +1  wait, let me recount:
    --   Seq indices: 1=A 2=B 3=A 4=C 5=A 6=B 7=B 8=A
    --   Windows (i, i+1): (1,2)=A→B, (2,3)=B→A, (3,4)=A→C, (4,5)=C→A,
    --                     (5,6)=A→B, (6,7)=B→B, (7,8)=B→A
    --   Counts: A→B:2, B→A:2, A→C:1, C→A:1, B→B:1
    --   Rows: A→{B:2/3, C:1/3}, B→{A:2/3, B:1/3}, C→{A:1/1}
    before_each(function()
        chain = MarkovChain.new()
        chain:train({"A", "B", "A", "C", "A", "B", "B", "A"})
    end)

    it("P(A→B) ≈ 0.667", function()
        local p = chain:probability("A", "B")
        assert.is_true(math.abs(p - 2.0/3.0) < 1e-9,
            "Expected ~0.667, got " .. tostring(p))
    end)

    it("P(A→C) ≈ 0.333", function()
        local p = chain:probability("A", "C")
        assert.is_true(math.abs(p - 1.0/3.0) < 1e-9,
            "Expected ~0.333, got " .. tostring(p))
    end)

    it("P(B→A) ≈ 0.667", function()
        local p = chain:probability("B", "A")
        assert.is_true(math.abs(p - 2.0/3.0) < 1e-9,
            "Expected ~0.667, got " .. tostring(p))
    end)

    it("P(B→B) ≈ 0.333", function()
        local p = chain:probability("B", "B")
        assert.is_true(math.abs(p - 1.0/3.0) < 1e-9,
            "Expected ~0.333, got " .. tostring(p))
    end)

    it("each row sums to 1.0", function()
        -- A valid probability distribution must sum to 1.
        local tm = chain:transition_matrix()
        for ctx, row in pairs(tm) do
            local s = sum_values(row)
            assert.is_true(math.abs(s - 1.0) < 1e-9,
                "Row for context " .. ctx .. " sums to " .. tostring(s))
        end
    end)
end)

-- =========================================================================
-- Test 4 — Laplace smoothing
-- Spec: MarkovChain.new(1, 1.0, {"A","B","C"}), train({"A","B"}),
--       probability("A","C") == 1/4 (1 smoothed count out of 4 total).
--
-- Breakdown:
--   Pre-registered states: {A, B, C} — alphabet size = 3
--   Training: A→B observed once.
--   Raw counts for context "A": {B: 1}
--   Smoothed counts: A→A = 0+1=1, A→B = 1+1=2, A→C = 0+1=1
--   Total denominator = 1 + 1*3 = 4
--   P(A→A) = 1/4, P(A→B) = 2/4 = 0.5, P(A→C) = 1/4 = 0.25
-- =========================================================================

describe("MarkovChain training — Laplace smoothing", function()
    local chain

    before_each(function()
        -- Pre-register {A, B, C} so smoothing covers all three states,
        -- including C which never appears in the training sequence.
        chain = MarkovChain.new(1, 1.0, {"A", "B", "C"})
        chain:train({"A", "B"})
    end)

    it("P(A→C) == 0.25 with smoothing=1.0 and 3 states", function()
        -- Laplace smoothing: (0 + 1) / (1 + 1*3) = 1/4 = 0.25
        local p = chain:probability("A", "C")
        assert.is_true(math.abs(p - 0.25) < 1e-9,
            "Expected 0.25, got " .. tostring(p))
    end)

    it("P(A→B) == 0.5 with smoothing=1.0", function()
        -- (1 + 1) / (1 + 1*3) = 2/4 = 0.5
        local p = chain:probability("A", "B")
        assert.is_true(math.abs(p - 0.5) < 1e-9,
            "Expected 0.5, got " .. tostring(p))
    end)

    it("A-row sums to 1.0 with smoothing", function()
        local pAA = chain:probability("A", "A")
        local pAB = chain:probability("A", "B")
        local pAC = chain:probability("A", "C")
        local total = pAA + pAB + pAC
        assert.is_true(math.abs(total - 1.0) < 1e-9,
            "Row sum = " .. tostring(total))
    end)
end)

-- =========================================================================
-- Test 5 — Generate length
-- Spec: generate(A, 10) returns a list of exactly 10 states.
-- =========================================================================

describe("MarkovChain generation — length", function()
    it("generate returns exactly the requested number of states", function()
        -- Set a fixed random seed for reproducibility.
        math.randomseed(42)
        local chain = MarkovChain.new()
        chain:train({"A", "B", "A", "B", "A", "B"})
        local result = chain:generate("A", 10)
        -- Must return exactly 10 elements.
        assert.are.equal(10, count(result))
    end)

    it("generate returns a table (array)", function()
        local chain = MarkovChain.new()
        chain:train({"A", "B", "A"})
        local result = chain:generate("A", 5)
        assert.is_table(result)
    end)

    it("generate with length 1 returns just the start state", function()
        local chain = MarkovChain.new()
        chain:train({"A", "B"})
        local result = chain:generate("A", 1)
        assert.are.equal(1, count(result))
        assert.are.equal("A", result[1])
    end)

    it("all generated states are from the known alphabet", function()
        math.randomseed(42)
        local chain = MarkovChain.new(1, 0.0, {"X", "Y", "Z"})
        chain:train({"X", "Y", "Z", "X", "Y"})
        local result = chain:generate("X", 20)
        for _, s in ipairs(result) do
            assert.is_true(s == "X" or s == "Y" or s == "Z",
                "Unexpected state: " .. tostring(s))
        end
    end)
end)

-- =========================================================================
-- Test 6 — Generate string
-- Spec: generate_string("th", 50) on a character chain trained on
--       English text returns a 50-char string starting with "th".
-- =========================================================================

describe("MarkovChain generation — string", function()
    it("generate_string returns exactly the requested number of characters", function()
        math.randomseed(42)
        local chain = MarkovChain.new()
        -- Train on a short English phrase repeated enough to build statistics.
        local text = "the quick brown fox jumps over the lazy dog "
        for _ = 1, 5 do
            chain:train_string(text)
        end
        local result = chain:generate_string("th", 50)
        -- Must be exactly 50 characters.
        assert.are.equal(50, #result)
    end)

    it("generate_string output starts with the seed character", function()
        math.randomseed(42)
        local chain = MarkovChain.new()
        local text = "abcdefghijklmnopqrstuvwxyz "
        for _ = 1, 3 do
            chain:train_string(text)
        end
        local result = chain:generate_string("a", 10)
        assert.are.equal("a", result:sub(1,1))
    end)
end)

-- =========================================================================
-- Test 7 — Stationary distribution sums to 1
-- Spec: for any ergodic chain, sum(stationary_distribution().values) ≈ 1.0
-- =========================================================================

describe("MarkovChain stationary distribution", function()
    it("sums to 1.0 for an ergodic chain", function()
        -- Build a simple ergodic chain: A↔B↔C↔A (a 3-cycle).
        -- This is ergodic: every state is reachable from every other.
        local chain = MarkovChain.new(1, 0.1, {"A", "B", "C"})
        -- Train a cycle so all transitions are observed.
        for _ = 1, 10 do
            chain:train({"A", "B", "C", "A", "B", "C"})
        end
        local dist = chain:stationary_distribution()
        local total = sum_values(dist)
        assert.is_true(math.abs(total - 1.0) < 1e-6,
            "Stationary distribution sum = " .. tostring(total))
    end)

    it("all probabilities are non-negative", function()
        local chain = MarkovChain.new(1, 0.1, {"A", "B", "C"})
        chain:train({"A", "B", "C", "A"})
        local dist = chain:stationary_distribution()
        for _, p in pairs(dist) do
            assert.is_true(p >= 0.0, "Negative probability: " .. tostring(p))
        end
    end)

    it("A↔B 2-cycle has equal stationary distribution", function()
        -- For a perfectly symmetric A↔B chain, π(A) = π(B) = 0.5.
        local chain = MarkovChain.new(1, 0.0)
        -- Observe A→B and B→A equally.
        chain:train({"A", "B", "A", "B", "A", "B", "A", "B"})
        local dist = chain:stationary_distribution()
        assert.is_true(math.abs(dist["A"] - 0.5) < 1e-6,
            "π(A) = " .. tostring(dist["A"]))
        assert.is_true(math.abs(dist["B"] - 0.5) < 1e-6,
            "π(B) = " .. tostring(dist["B"]))
    end)
end)

-- =========================================================================
-- Test 8 — Order-2 chain
-- Spec: train on "abcabcabc" with order=2.
--       generate_string("ab", 9) == "abcabcabc"
--
-- Why this is deterministic:
--   Digrams in "abcabcabc": ab, bc, ca, ab, bc, ca, ab, bc
--   Transitions:
--     "a\0b" → c (count=3, p=1.0)
--     "b\0c" → a (count=2, p=1.0)
--     "c\0a" → b (count=2, p=1.0)
--   Since each context has only one possible successor, generation is
--   completely deterministic regardless of random seed.
-- =========================================================================

describe("MarkovChain order-2 chain", function()
    it("generate_string(ab, 9) == 'abcabcabc' for order-2 trained on 'abcabcabc'", function()
        local chain = MarkovChain.new(2)
        chain:train_string("abcabcabc")
        -- No random seed needed — deterministic because each context has p=1.0
        local result = chain:generate_string("ab", 9)
        assert.are.equal("abcabcabc", result)
    end)

    it("order-2 context 'ab' transitions to 'c' with probability 1.0", function()
        local chain = MarkovChain.new(2)
        chain:train_string("abcabcabc")
        -- The context key for ("a","b") is "a\0b"
        local p = chain:probability("a\0b", "c")
        assert.are.equal(1.0, p)
    end)

    it("order-2 context 'bc' transitions to 'a' with probability 1.0", function()
        local chain = MarkovChain.new(2)
        chain:train_string("abcabcabc")
        local p = chain:probability("b\0c", "a")
        assert.are.equal(1.0, p)
    end)

    it("generates exactly the requested length for order-2", function()
        local chain = MarkovChain.new(2)
        chain:train_string("abcabcabc")
        local result = chain:generate_string("ab", 15)
        assert.are.equal(15, #result)
    end)
end)

-- =========================================================================
-- Test 9 — Unknown state error
-- Spec: assert.has_error(function() chain:next_state("UNKNOWN") end)
-- =========================================================================

describe("MarkovChain error conditions", function()
    it("next_state raises an error for an unknown state", function()
        local chain = MarkovChain.new()
        chain:train({"A", "B"})
        -- next_state should call error() for any state not in _transitions.
        assert.has_error(function()
            chain:next_state("UNKNOWN")
        end)
    end)

    it("next_state error message contains 'Unknown state'", function()
        local chain = MarkovChain.new()
        chain:train({"A", "B"})
        local ok, err = pcall(function()
            chain:next_state("GHOST")
        end)
        assert.is_false(ok)
        -- err is the error message string from error("Unknown state: ...")
        assert.is_true(string.find(err, "Unknown state", 1, true) ~= nil,
            "Error should mention 'Unknown state', got: " .. tostring(err))
    end)

    it("probability returns 0.0 for completely unknown context (no error)", function()
        -- Unlike next_state, probability() is lenient: unknown context → 0.0.
        local chain = MarkovChain.new()
        chain:train({"A", "B"})
        assert.are.equal(0.0, chain:probability("UNKNOWN", "A"))
    end)
end)

-- =========================================================================
-- Test 10 — Multi-train accumulation
-- Spec: calling train() twice accumulates counts before renormalising,
--       so probabilities reflect the combined training data.
-- =========================================================================

describe("MarkovChain multi-train accumulation", function()
    it("accumulated counts shift probabilities toward more-observed transitions", function()
        -- First training: only A→B observed. P(A→B) = 1.0.
        local chain = MarkovChain.new()
        chain:train({"A", "B"})
        assert.are.equal(1.0, chain:probability("A", "B"))

        -- Second training: add A→C once. Now A→B: 1, A→C: 1.
        -- P(A→B) should be 0.5, P(A→C) should be 0.5.
        chain:train({"A", "C"})
        local pAB = chain:probability("A", "B")
        local pAC = chain:probability("A", "C")
        assert.is_true(math.abs(pAB - 0.5) < 1e-9,
            "Expected P(A→B)=0.5 after two trains, got " .. tostring(pAB))
        assert.is_true(math.abs(pAC - 0.5) < 1e-9,
            "Expected P(A→C)=0.5 after two trains, got " .. tostring(pAC))
    end)

    it("training on the same data twice doubles all counts (ratios unchanged)", function()
        local chain = MarkovChain.new()
        chain:train({"A", "B", "A", "C"})
        local pAB_once = chain:probability("A", "B")

        chain:train({"A", "B", "A", "C"})
        local pAB_twice = chain:probability("A", "B")

        -- Doubling all counts doesn't change the ratios.
        assert.is_true(math.abs(pAB_once - pAB_twice) < 1e-9,
            "P(A→B) should be identical after training same data twice")
    end)

    it("state count grows as new states are introduced across train calls", function()
        local chain = MarkovChain.new()
        chain:train({"A", "B"})
        assert.are.equal(2, count(chain:states()))

        chain:train({"B", "C"})
        assert.are.equal(3, count(chain:states()))
    end)
end)

-- =========================================================================
-- Additional edge-case tests for higher coverage
-- =========================================================================

describe("MarkovChain edge cases", function()
    it("train_string works identically to train with char array", function()
        local chain_str = MarkovChain.new()
        chain_str:train_string("ABAB")

        local chain_arr = MarkovChain.new()
        chain_arr:train({"A", "B", "A", "B"})

        assert.are.equal(chain_str:probability("A", "B"),
                         chain_arr:probability("A", "B"))
    end)

    it("states() returns a copy (mutations do not affect internal state)", function()
        local chain = MarkovChain.new()
        chain:train({"X", "Y"})
        local s = chain:states()
        s[1] = "MUTATED"
        -- Internal state should be unaffected.
        local s2 = chain:states()
        assert.is_true(s2[1] ~= "MUTATED")
    end)

    it("transition_matrix() returns a copy", function()
        local chain = MarkovChain.new()
        chain:train({"A", "B"})
        local tm = chain:transition_matrix()
        tm["A"]["B"] = 0.0  -- mutate the copy
        -- Internal transitions should be unaffected.
        assert.are.equal(1.0, chain:probability("A", "B"))
    end)

    it("order-1 generate starts with the given start state", function()
        math.randomseed(42)
        local chain = MarkovChain.new()
        chain:train({"A", "B", "C", "A", "B"})
        local result = chain:generate("A", 5)
        assert.are.equal("A", result[1])
    end)

    it("zero smoothing leaves unobserved transitions at 0.0", function()
        local chain = MarkovChain.new(1, 0.0, {"A", "B", "C"})
        chain:train({"A", "B"})
        -- C was pre-registered but never appeared in training.
        -- With zero smoothing A→C and A→A should be 0.
        assert.are.equal(0.0, chain:probability("A", "C"))
        assert.are.equal(0.0, chain:probability("A", "A"))
    end)
end)

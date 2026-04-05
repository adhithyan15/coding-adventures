-- ============================================================================
-- Tests for Statistics Package
-- ============================================================================
-- These tests use the Busted testing framework (https://lunarmodules.github.io/busted/).
-- Run with: cd tests && busted . --verbose --pattern=test_
--
-- The test suite covers:
--   - Descriptive statistics (mean, median, mode, variance, std_dev, min, max, range)
--   - Frequency analysis (frequency_count, frequency_distribution, chi_squared,
--     chi_squared_text)
--   - Cryptanalysis helpers (index_of_coincidence, entropy, ENGLISH_FREQUENCIES)
--   - Cross-language parity test vectors
-- ============================================================================

-- CRITICAL: Set package.path before any require so Busted can find our module.
-- This is a Lua monorepo lesson: test files MUST set up the path to ../src/.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local Stats = require("coding_adventures.stats")

-- ============================================================================
-- Helper: compare floats within a tolerance (epsilon)
-- ============================================================================
-- Floating-point arithmetic is not exact. 0.1 + 0.2 ~= 0.30000000000000004.
-- We use a small epsilon (1e-6) for most comparisons.
local function approx(a, b, eps)
    eps = eps or 1e-6
    return math.abs(a - b) < eps
end

-- ============================================================================
-- Descriptive Statistics Tests
-- ============================================================================

describe("Descriptive Statistics", function()

    -- ── Mean ──────────────────────────────────────────────────────────────
    describe("mean", function()
        it("computes the mean of a simple list", function()
            -- Parity test vector: mean({1,2,3,4,5}) = 3.0
            assert.is_true(approx(Stats.mean({1, 2, 3, 4, 5}), 3.0))
        end)

        it("computes the mean of the worked example", function()
            assert.is_true(approx(Stats.mean({2, 4, 4, 4, 5, 5, 7, 9}), 5.0))
        end)

        it("handles a single value", function()
            assert.is_true(approx(Stats.mean({42}), 42.0))
        end)

        it("handles negative values", function()
            assert.is_true(approx(Stats.mean({-1, -2, -3}), -2.0))
        end)

        it("errors on empty input", function()
            assert.has_error(function() Stats.mean({}) end)
        end)
    end)

    -- ── Median ────────────────────────────────────────────────────────────
    describe("median", function()
        it("computes median of odd-length list", function()
            assert.is_true(approx(Stats.median({1, 2, 3, 4, 5}), 3.0))
        end)

        it("computes median of even-length list", function()
            assert.is_true(approx(Stats.median({2, 4, 4, 4, 5, 5, 7, 9}), 4.5))
        end)

        it("handles a single value", function()
            assert.is_true(approx(Stats.median({7}), 7.0))
        end)

        it("sorts unsorted input", function()
            assert.is_true(approx(Stats.median({5, 1, 3}), 3.0))
        end)

        it("errors on empty input", function()
            assert.has_error(function() Stats.median({}) end)
        end)
    end)

    -- ── Mode ──────────────────────────────────────────────────────────────
    describe("mode", function()
        it("finds the most frequent value", function()
            assert.is_true(approx(Stats.mode({2, 4, 4, 4, 5, 5, 7, 9}), 4.0))
        end)

        it("returns first occurrence on tie", function()
            -- Both 1 and 2 appear twice; 1 comes first
            assert.is_true(approx(Stats.mode({1, 2, 1, 2, 3}), 1.0))
        end)

        it("handles single value", function()
            assert.is_true(approx(Stats.mode({99}), 99.0))
        end)

        it("errors on empty input", function()
            assert.has_error(function() Stats.mode({}) end)
        end)
    end)

    -- ── Variance ──────────────────────────────────────────────────────────
    describe("variance", function()
        it("computes sample variance (default)", function()
            -- Parity test vector: 4.571428571428571
            local result = Stats.variance({2, 4, 4, 4, 5, 5, 7, 9})
            assert.is_true(approx(result, 4.571428571428571))
        end)

        it("computes population variance", function()
            -- Parity test vector: 4.0
            local result = Stats.variance({2, 4, 4, 4, 5, 5, 7, 9}, true)
            assert.is_true(approx(result, 4.0))
        end)

        it("handles two values (sample)", function()
            local result = Stats.variance({1, 3})
            assert.is_true(approx(result, 2.0))
        end)

        it("errors on single value for sample variance", function()
            assert.has_error(function() Stats.variance({42}) end)
        end)

        it("allows single value for population variance", function()
            assert.is_true(approx(Stats.variance({42}, true), 0.0))
        end)
    end)

    -- ── Standard Deviation ────────────────────────────────────────────────
    describe("standard_deviation", function()
        it("computes sample standard deviation", function()
            local result = Stats.standard_deviation({2, 4, 4, 4, 5, 5, 7, 9})
            assert.is_true(approx(result, 2.13809, 1e-4))
        end)

        it("computes population standard deviation", function()
            local result = Stats.standard_deviation({2, 4, 4, 4, 5, 5, 7, 9}, true)
            assert.is_true(approx(result, 2.0))
        end)
    end)

    -- ── Min / Max / Range ─────────────────────────────────────────────────
    describe("min", function()
        it("finds the minimum value", function()
            assert.is_true(approx(Stats.min({2, 4, 4, 4, 5, 5, 7, 9}), 2.0))
        end)

        it("handles negative values", function()
            assert.is_true(approx(Stats.min({3, -1, 7}), -1.0))
        end)

        it("errors on empty input", function()
            assert.has_error(function() Stats.min({}) end)
        end)
    end)

    describe("max", function()
        it("finds the maximum value", function()
            assert.is_true(approx(Stats.max({2, 4, 4, 4, 5, 5, 7, 9}), 9.0))
        end)

        it("errors on empty input", function()
            assert.has_error(function() Stats.max({}) end)
        end)
    end)

    describe("range", function()
        it("computes the range", function()
            assert.is_true(approx(Stats.range({2, 4, 4, 4, 5, 5, 7, 9}), 7.0))
        end)

        it("returns 0 for identical values", function()
            assert.is_true(approx(Stats.range({5, 5, 5}), 0.0))
        end)
    end)
end)

-- ============================================================================
-- Frequency Analysis Tests
-- ============================================================================

describe("Frequency Analysis", function()

    -- ── frequency_count ───────────────────────────────────────────────────
    describe("frequency_count", function()
        it("counts letters case-insensitively", function()
            local counts = Stats.frequency_count("Hello")
            assert.are.equal(1, counts["H"])
            assert.are.equal(1, counts["E"])
            assert.are.equal(2, counts["L"])
            assert.are.equal(1, counts["O"])
        end)

        it("ignores non-alphabetic characters", function()
            local counts = Stats.frequency_count("A1B2C3!!!")
            assert.are.equal(1, counts["A"])
            assert.are.equal(1, counts["B"])
            assert.are.equal(1, counts["C"])
            assert.are.equal(0, counts["D"])
        end)

        it("handles empty string", function()
            local counts = Stats.frequency_count("")
            assert.are.equal(0, counts["A"])
        end)
    end)

    -- ── frequency_distribution ────────────────────────────────────────────
    describe("frequency_distribution", function()
        it("computes proportions", function()
            local dist = Stats.frequency_distribution("AABB")
            assert.is_true(approx(dist["A"], 0.5))
            assert.is_true(approx(dist["B"], 0.5))
            assert.is_true(approx(dist["C"], 0.0))
        end)

        it("handles empty string", function()
            local dist = Stats.frequency_distribution("")
            assert.is_true(approx(dist["A"], 0.0))
        end)
    end)

    -- ── chi_squared ───────────────────────────────────────────────────────
    describe("chi_squared", function()
        it("computes chi-squared for parallel arrays", function()
            -- Parity test vector: 10.0
            local result = Stats.chi_squared({10, 20, 30}, {20, 20, 20})
            assert.is_true(approx(result, 10.0))
        end)

        it("returns 0 for identical distributions", function()
            local result = Stats.chi_squared({10, 10, 10}, {10, 10, 10})
            assert.is_true(approx(result, 0.0))
        end)

        it("errors on mismatched lengths", function()
            assert.has_error(function()
                Stats.chi_squared({1, 2}, {1, 2, 3})
            end)
        end)
    end)

    -- ── chi_squared_text ──────────────────────────────────────────────────
    describe("chi_squared_text", function()
        it("computes chi-squared of text against expected frequencies", function()
            local result = Stats.chi_squared_text("AABB", Stats.ENGLISH_FREQUENCIES)
            assert.is_true(result > 0)
        end)

        it("returns 0 for empty text", function()
            local result = Stats.chi_squared_text("", Stats.ENGLISH_FREQUENCIES)
            assert.is_true(approx(result, 0.0))
        end)

        it("returns 0 for non-alphabetic text", function()
            local result = Stats.chi_squared_text("12345!!!", Stats.ENGLISH_FREQUENCIES)
            assert.is_true(approx(result, 0.0))
        end)
    end)
end)

-- ============================================================================
-- Cryptanalysis Helpers Tests
-- ============================================================================

describe("Cryptanalysis Helpers", function()

    -- ── index_of_coincidence ──────────────────────────────────────────────
    describe("index_of_coincidence", function()
        it("computes IC for AABB", function()
            -- Parity test vector: (2*1 + 2*1) / (4*3) = 4/12 = 0.333...
            local result = Stats.index_of_coincidence("AABB")
            assert.is_true(approx(result, 1.0 / 3.0, 1e-6))
        end)

        it("returns 0 for text with fewer than 2 letters", function()
            assert.is_true(approx(Stats.index_of_coincidence("A"), 0.0))
            assert.is_true(approx(Stats.index_of_coincidence(""), 0.0))
        end)

        it("computes IC for English-like text", function()
            -- A pangram has near-uniform distribution, so IC will be low.
            -- Longer, more natural English text would give IC ~ 0.0667.
            local result = Stats.index_of_coincidence("THEQUICKBROWNFOXJUMPSOVERTHELAZYDOG")
            assert.is_true(result > 0) -- positive value
        end)

        it("returns 1.0 for text with all same letter", function()
            local result = Stats.index_of_coincidence("AAAA")
            assert.is_true(approx(result, 1.0))
        end)
    end)

    -- ── entropy ───────────────────────────────────────────────────────────
    describe("entropy", function()
        it("computes entropy of uniform 26-letter text", function()
            -- Parity test vector: log2(26) ~ 4.700
            local text = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
            local result = Stats.entropy(text)
            assert.is_true(approx(result, math.log(26) / math.log(2), 0.01))
        end)

        it("returns 0 for single-letter text", function()
            local result = Stats.entropy("AAAA")
            assert.is_true(approx(result, 0.0))
        end)

        it("computes entropy of two-letter uniform distribution", function()
            -- H("AABB") = -2*(0.5 * log2(0.5)) = 1.0
            local result = Stats.entropy("AABB")
            assert.is_true(approx(result, 1.0, 0.01))
        end)

        it("returns 0 for empty text", function()
            local result = Stats.entropy("")
            assert.is_true(approx(result, 0.0))
        end)
    end)

    -- ── ENGLISH_FREQUENCIES ───────────────────────────────────────────────
    describe("ENGLISH_FREQUENCIES", function()
        it("has 26 entries", function()
            local count = 0
            for _ in pairs(Stats.ENGLISH_FREQUENCIES) do
                count = count + 1
            end
            assert.are.equal(26, count)
        end)

        it("sums to approximately 1.0", function()
            local total = 0
            for _, v in pairs(Stats.ENGLISH_FREQUENCIES) do
                total = total + v
            end
            assert.is_true(approx(total, 1.0, 0.01))
        end)

        it("has E as the most common letter", function()
            local max_letter = "A"
            local max_freq = 0
            for letter, freq in pairs(Stats.ENGLISH_FREQUENCIES) do
                if freq > max_freq then
                    max_freq = freq
                    max_letter = letter
                end
            end
            assert.are.equal("E", max_letter)
        end)
    end)
end)

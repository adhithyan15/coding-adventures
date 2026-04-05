-- Tests for coding_adventures.reed_solomon (MA02)
--
-- Covers:
--   1. build_generator      — generator polynomial construction and verification
--   2. encode               — systematic encoding, output length, check bytes
--   3. syndromes            — all-zero on valid codewords, non-zero on corrupted
--   4. decode               — no errors, single/double/multi-error correction
--   5. error_locator        — Berlekamp-Massey output shape and degree
--   6. round-trip           — decode(encode(msg, n)) == msg for many cases
--   7. error cases          — InvalidInput and TooManyErrors propagation

-- Add this package's src directory (reed-solomon) and the gf256 src directory
-- to the require search path. The tests run from the `tests/` subdirectory,
-- so paths are relative to that location.
package.path = "../src/?.lua;../src/?/init.lua;../../gf256/src/?.lua;../../gf256/src/?/init.lua;" .. package.path

local rs = require("coding_adventures.reed_solomon")
local gf = require("coding_adventures.gf256")

-- ============================================================================
-- Helper: make a fresh copy of a table (shallow)
-- ============================================================================
local function copy(t)
    local c = {}
    for i = 1, #t do c[i] = t[i] end
    return c
end

-- Helper: check that two tables have equal content
local function tables_equal(a, b)
    if #a ~= #b then return false end
    for i = 1, #a do
        if a[i] ~= b[i] then return false end
    end
    return true
end

-- ============================================================================
-- build_generator
-- ============================================================================
describe("build_generator", function()

    it("n_check=2 returns the canonical {8, 6, 1} cross-language vector", function()
        -- g(x) = (x + α¹)(x + α²) = (x+2)(x+4) = x² + 6x + 8
        -- In LE: {8, 6, 1}  (constant term first)
        local g = rs.build_generator(2)
        assert.are.equal(3, #g)
        assert.are.equal(8, g[1])
        assert.are.equal(6, g[2])
        assert.are.equal(1, g[3])
    end)

    it("n_check=2: both roots α¹=2 and α²=4 evaluate to zero", function()
        -- If g = {8, 6, 1} (LE), then g(2) and g(4) should be 0.
        local g = rs.build_generator(2)
        -- Evaluate LE poly at x: iterate from high degree to low
        local function eval_le(p, x)
            local acc = 0
            for i = #p, 1, -1 do
                acc = gf.add(gf.multiply(acc, x), p[i])
            end
            return acc
        end
        assert.are.equal(0, eval_le(g, 2),  "g(α¹) should be 0")
        assert.are.equal(0, eval_le(g, 4),  "g(α²) should be 0")
    end)

    it("n_check=4 returns degree-4 polynomial (length 5)", function()
        local g = rs.build_generator(4)
        assert.are.equal(5, #g)
        assert.are.equal(1, g[5])   -- monic: leading coefficient is 1
    end)

    it("n_check=4: roots α¹..α⁴ all evaluate to zero", function()
        local g = rs.build_generator(4)
        local function eval_le(p, x)
            local acc = 0
            for i = #p, 1, -1 do
                acc = gf.add(gf.multiply(acc, x), p[i])
            end
            return acc
        end
        for i = 1, 4 do
            local alpha_i = gf.power(2, i)
            assert.are.equal(0, eval_le(g, alpha_i),
                "g(α^" .. i .. ") should be 0")
        end
    end)

    it("n_check=8 returns degree-8 polynomial (length 9, monic)", function()
        local g = rs.build_generator(8)
        assert.are.equal(9, #g)
        assert.are.equal(1, g[9])
    end)

    it("leading coefficient is always 1 (monic) for n_check=2,4,6,8", function()
        for _, nc in ipairs({2, 4, 6, 8}) do
            local g = rs.build_generator(nc)
            assert.are.equal(1, g[#g],
                "build_generator(" .. nc .. ") leading coefficient should be 1")
        end
    end)

    it("raises InvalidInput for n_check=0", function()
        assert.has_error(function() rs.build_generator(0) end)
    end)

    it("raises InvalidInput for odd n_check=3", function()
        assert.has_error(function() rs.build_generator(3) end)
    end)

    it("raises InvalidInput for odd n_check=1", function()
        assert.has_error(function() rs.build_generator(1) end)
    end)

end)

-- ============================================================================
-- encode
-- ============================================================================
describe("encode", function()

    it("output length = #message + n_check", function()
        local msg = {4, 3, 2, 1}
        local cw  = rs.encode(msg, 2)
        assert.are.equal(6, #cw)
    end)

    it("systematic: first #message bytes are unchanged", function()
        local msg = {10, 20, 30, 40, 50}
        local cw  = rs.encode(msg, 4)
        for i = 1, #msg do
            assert.are.equal(msg[i], cw[i],
                "codeword byte " .. i .. " should match message byte " .. i)
        end
    end)

    it("syndromes of a freshly-encoded codeword are all zero", function()
        local msg = {1, 2, 3, 4, 5}
        local cw  = rs.encode(msg, 4)
        local s   = rs.syndromes(cw, 4)
        for i = 1, 4 do
            assert.are.equal(0, s[i], "syndrome " .. i .. " should be 0")
        end
    end)

    it("syndromes all-zero for n_check=2 and various messages", function()
        for _, msg in ipairs({
            {0},
            {255},
            {1, 2, 3},
            {100, 200, 150, 50},
        }) do
            local cw = rs.encode(msg, 2)
            local s  = rs.syndromes(cw, 2)
            assert.are.equal(0, s[1])
            assert.are.equal(0, s[2])
        end
    end)

    it("check bytes are not all zero for a non-trivial message", function()
        -- For message {4, 3, 2, 1} with n_check=2, the check bytes should be
        -- non-zero (they encode the remainder of polynomial division).
        local msg = {4, 3, 2, 1}
        local cw  = rs.encode(msg, 2)
        -- At least one check byte should be non-zero.
        local any_nonzero = (cw[5] ~= 0) or (cw[6] ~= 0)
        assert.is_true(any_nonzero)
    end)

    it("raises InvalidInput for n_check=0", function()
        assert.has_error(function() rs.encode({1, 2, 3}, 0) end)
    end)

    it("raises InvalidInput for odd n_check", function()
        assert.has_error(function() rs.encode({1, 2, 3}, 3) end)
    end)

    it("raises InvalidInput when total > 255", function()
        -- 250-byte message + 8 check = 258 > 255
        local long_msg = {}
        for i = 1, 250 do long_msg[i] = i % 256 end
        assert.has_error(function() rs.encode(long_msg, 8) end)
    end)

    it("single-byte message encodes without error", function()
        local cw = rs.encode({42}, 2)
        assert.are.equal(3, #cw)
        assert.are.equal(42, cw[1])
    end)

    it("all-zero message produces all-zero codeword", function()
        -- 0 · anything = 0 in GF(256), so all-zero message → all-zero codeword.
        local cw = rs.encode({0, 0, 0, 0}, 4)
        for i = 1, 8 do
            assert.are.equal(0, cw[i], "byte " .. i .. " of all-zero codeword should be 0")
        end
    end)

end)

-- ============================================================================
-- syndromes
-- ============================================================================
describe("syndromes", function()

    it("all-zero syndromes for a valid codeword", function()
        local cw = rs.encode({7, 8, 9}, 4)
        local s  = rs.syndromes(cw, 4)
        for i = 1, 4 do
            assert.are.equal(0, s[i])
        end
    end)

    it("non-zero syndrome when one byte is corrupted", function()
        local cw = rs.encode({1, 2, 3, 4}, 4)
        local corrupted = copy(cw)
        corrupted[1] = corrupted[1] ~ 0xFF   -- flip all bits of byte 1
        local s = rs.syndromes(corrupted, 4)
        local any_nonzero = false
        for i = 1, 4 do
            if s[i] ~= 0 then any_nonzero = true end
        end
        assert.is_true(any_nonzero, "corrupted codeword should have at least one non-zero syndrome")
    end)

    it("syndrome table length equals n_check", function()
        local cw = rs.encode({1}, 6)
        local s  = rs.syndromes(cw, 6)
        assert.are.equal(6, #s)
    end)

    it("syndromes of all-zero codeword are all zero", function()
        -- The all-zero polynomial evaluates to 0 everywhere.
        local cw = {0, 0, 0, 0, 0, 0}
        local s  = rs.syndromes(cw, 4)
        for i = 1, 4 do
            assert.are.equal(0, s[i])
        end
    end)

end)

-- ============================================================================
-- error_locator (Berlekamp-Massey)
-- ============================================================================
describe("error_locator", function()

    it("all-zero syndromes → {1} (no errors)", function()
        -- When all syndromes are zero, BM finds no errors: Λ(x) = 1.
        local synds = {0, 0, 0, 0}
        local lam = rs.error_locator(synds)
        assert.are.equal(1, #lam)
        assert.are.equal(1, lam[1])
    end)

    it("Λ[1] is always 1 (constant term = Λ₀ = 1)", function()
        -- Berlekamp-Massey guarantees Λ(0) = 1 for any syndrome input.
        local cw = rs.encode({5, 10, 15, 20}, 4)
        local corrupted = copy(cw)
        corrupted[2] = corrupted[2] ~ 1
        local s = rs.syndromes(corrupted, 4)
        local lam = rs.error_locator(s)
        assert.are.equal(1, lam[1])
    end)

    it("1 error → Λ has degree 1 (length 2)", function()
        local cw = rs.encode({10, 20, 30}, 4)
        local corrupted = copy(cw)
        corrupted[3] = 0xFF
        local s = rs.syndromes(corrupted, 4)
        local lam = rs.error_locator(s)
        assert.are.equal(2, #lam)
    end)

    it("2 errors → Λ has degree 2 (length 3)", function()
        local cw = rs.encode({10, 20, 30, 40, 50}, 8)
        local corrupted = copy(cw)
        corrupted[1] = corrupted[1] ~ 1
        corrupted[5] = corrupted[5] ~ 2
        local s = rs.syndromes(corrupted, 8)
        local lam = rs.error_locator(s)
        assert.are.equal(3, #lam)
    end)

    it("degree of Λ matches number of errors injected", function()
        local cw = rs.encode({1, 2, 3, 4, 5, 6, 7, 8}, 8)
        -- Inject 4 errors (capacity t=4 for n_check=8)
        local corrupted = copy(cw)
        corrupted[1] = corrupted[1] ~ 7
        corrupted[3] = corrupted[3] ~ 11
        corrupted[5] = corrupted[5] ~ 13
        corrupted[7] = corrupted[7] ~ 17
        local s = rs.syndromes(corrupted, 8)
        local lam = rs.error_locator(s)
        -- Degree should be 4 → length 5
        assert.are.equal(5, #lam)
    end)

end)

-- ============================================================================
-- decode — no errors
-- ============================================================================
describe("decode (no errors)", function()

    it("decode of a clean codeword returns original message", function()
        local msg = {4, 3, 2, 1}
        local cw  = rs.encode(msg, 2)
        local dec = rs.decode(cw, 2)
        assert.is_true(tables_equal(msg, dec))
    end)

    it("decode with n_check=4 and no errors", function()
        local msg = {10, 20, 30, 40, 50}
        local cw  = rs.encode(msg, 4)
        local dec = rs.decode(cw, 4)
        assert.is_true(tables_equal(msg, dec))
    end)

    it("decode with n_check=8 and no errors", function()
        local msg = {1, 2, 3, 4, 5, 6}
        local cw  = rs.encode(msg, 8)
        local dec = rs.decode(cw, 8)
        assert.is_true(tables_equal(msg, dec))
    end)

    it("decode all-zero message with no errors", function()
        local msg = {0, 0, 0, 0}
        local cw  = rs.encode(msg, 4)
        local dec = rs.decode(cw, 4)
        assert.is_true(tables_equal(msg, dec))
    end)

end)

-- ============================================================================
-- decode — single error correction
-- ============================================================================
describe("decode (single error)", function()

    it("corrects error at first byte (n_check=2)", function()
        local msg = {4, 3, 2, 1}
        local cw  = rs.encode(msg, 2)
        local corrupted = copy(cw)
        corrupted[1] = corrupted[1] ~ 0x55
        local dec = rs.decode(corrupted, 2)
        assert.is_true(tables_equal(msg, dec))
    end)

    it("corrects error at last message byte", function()
        local msg = {4, 3, 2, 1}
        local cw  = rs.encode(msg, 2)
        local corrupted = copy(cw)
        corrupted[4] = 0x00
        local dec = rs.decode(corrupted, 2)
        assert.is_true(tables_equal(msg, dec))
    end)

    it("corrects error in a check byte", function()
        local msg = {10, 20, 30}
        local cw  = rs.encode(msg, 2)
        local corrupted = copy(cw)
        -- Corrupt the first check byte
        corrupted[4] = corrupted[4] ~ 0xAA
        local dec = rs.decode(corrupted, 2)
        assert.is_true(tables_equal(msg, dec))
    end)

    it("corrects single error at various positions (n_check=4)", function()
        local msg = {1, 2, 3, 4, 5}
        local cw  = rs.encode(msg, 4)
        for pos = 1, #cw do
            local corrupted = copy(cw)
            corrupted[pos] = corrupted[pos] ~ 0x33
            local ok, dec = pcall(rs.decode, corrupted, 4)
            assert.is_true(ok, "decode failed at position " .. pos)
            assert.is_true(tables_equal(msg, dec),
                "decode produced wrong result for error at position " .. pos)
        end
    end)

end)

-- ============================================================================
-- decode — two error correction (n_check=4)
-- ============================================================================
describe("decode (two errors, n_check=4)", function()

    it("corrects 2 errors at positions 1 and 3", function()
        local msg = {10, 20, 30, 40}
        local cw  = rs.encode(msg, 4)
        local corrupted = copy(cw)
        corrupted[1] = corrupted[1] ~ 0x11
        corrupted[3] = corrupted[3] ~ 0x22
        local dec = rs.decode(corrupted, 4)
        assert.is_true(tables_equal(msg, dec))
    end)

    it("corrects 2 errors at positions 2 and 5", function()
        local msg = {5, 10, 15, 20, 25}
        local cw  = rs.encode(msg, 4)
        local corrupted = copy(cw)
        corrupted[2] = corrupted[2] ~ 0x77
        corrupted[5] = corrupted[5] ~ 0x88
        local dec = rs.decode(corrupted, 4)
        assert.is_true(tables_equal(msg, dec))
    end)

    it("corrects 2 errors when both are in check bytes", function()
        local msg = {1, 2, 3}
        local cw  = rs.encode(msg, 4)
        local corrupted = copy(cw)
        corrupted[4] = corrupted[4] ~ 1
        corrupted[7] = corrupted[7] ~ 2
        local dec = rs.decode(corrupted, 4)
        assert.is_true(tables_equal(msg, dec))
    end)

end)

-- ============================================================================
-- decode — four error correction (n_check=8)
-- ============================================================================
describe("decode (four errors, n_check=8)", function()

    it("corrects 4 errors scattered in codeword", function()
        local msg = {1, 2, 3, 4, 5}
        local cw  = rs.encode(msg, 8)
        local corrupted = copy(cw)
        corrupted[1]  = corrupted[1]  ~ 0x11
        corrupted[4]  = corrupted[4]  ~ 0x22
        corrupted[7]  = corrupted[7]  ~ 0x33
        corrupted[10] = corrupted[10] ~ 0x44
        local dec = rs.decode(corrupted, 8)
        assert.is_true(tables_equal(msg, dec))
    end)

    it("corrects 4 errors at first 4 positions", function()
        local msg = {11, 22, 33, 44, 55, 66}
        local cw  = rs.encode(msg, 8)
        local corrupted = copy(cw)
        for i = 1, 4 do
            corrupted[i] = corrupted[i] ~ (i * 0x10)
        end
        local dec = rs.decode(corrupted, 8)
        assert.is_true(tables_equal(msg, dec))
    end)

end)

-- ============================================================================
-- decode — TooManyErrors
-- ============================================================================
describe("decode (TooManyErrors)", function()

    it("raises TooManyErrors when t+1 errors exceed capacity (n_check=2, t=1)", function()
        local msg = {4, 3, 2, 1}
        local cw  = rs.encode(msg, 2)
        local corrupted = copy(cw)
        -- Inject 2 errors (exceeds t=1)
        corrupted[1] = corrupted[1] ~ 0x55
        corrupted[2] = corrupted[2] ~ 0xAA
        assert.has_error(function() rs.decode(corrupted, 2) end)
    end)

    it("raises TooManyErrors for 3 errors with n_check=4 (t=2)", function()
        local msg = {1, 2, 3, 4, 5}
        local cw  = rs.encode(msg, 4)
        local corrupted = copy(cw)
        corrupted[1] = corrupted[1] ~ 1
        corrupted[3] = corrupted[3] ~ 2
        corrupted[5] = corrupted[5] ~ 3
        assert.has_error(function() rs.decode(corrupted, 4) end)
    end)

    it("raises InvalidInput for n_check=0", function()
        assert.has_error(function() rs.decode({1, 2, 3}, 0) end)
    end)

    it("raises InvalidInput for odd n_check", function()
        assert.has_error(function() rs.decode({1, 2, 3}, 3) end)
    end)

    it("raises InvalidInput when received is shorter than n_check", function()
        assert.has_error(function() rs.decode({1}, 4) end)
    end)

end)

-- ============================================================================
-- round-trip property
-- ============================================================================
describe("round-trip: decode(encode(msg, n)) == msg", function()

    it("round-trip for varied messages, n_check=2", function()
        local messages = {
            {1},
            {0},
            {255},
            {1, 2, 3},
            {10, 20, 30, 40, 50},
            {0, 128, 255, 1},
        }
        for _, msg in ipairs(messages) do
            local cw  = rs.encode(msg, 2)
            local dec = rs.decode(cw, 2)
            assert.is_true(tables_equal(msg, dec),
                "round-trip failed for message length " .. #msg)
        end
    end)

    it("round-trip for varied messages, n_check=4", function()
        local messages = {
            {100},
            {1, 2},
            {5, 10, 15, 20, 25},
            {200, 150, 100, 50},
        }
        for _, msg in ipairs(messages) do
            local cw  = rs.encode(msg, 4)
            local dec = rs.decode(cw, 4)
            assert.is_true(tables_equal(msg, dec))
        end
    end)

    it("round-trip for a longer message, n_check=8", function()
        local msg = {}
        for i = 1, 20 do msg[i] = (i * 13) % 256 end
        local cw  = rs.encode(msg, 8)
        local dec = rs.decode(cw, 8)
        assert.is_true(tables_equal(msg, dec))
    end)

    it("round-trip with correction: encode, corrupt 1, decode → original", function()
        local msg = {77, 88, 99, 111}
        local cw  = rs.encode(msg, 4)
        local corrupted = copy(cw)
        corrupted[2] = 0x00
        local dec = rs.decode(corrupted, 4)
        assert.is_true(tables_equal(msg, dec))
    end)

    it("round-trip: max-length single message byte, n_check=2", function()
        -- 1 + 2 = 3 < 255, well within bounds
        local msg = {127}
        local dec = rs.decode(rs.encode(msg, 2), 2)
        assert.is_true(tables_equal(msg, dec))
    end)

end)

-- ============================================================================
-- module API
-- ============================================================================
describe("module API", function()

    it("exports VERSION string '0.1.0'", function()
        assert.are.equal("0.1.0", rs.VERSION)
    end)

    it("exports encode as a function", function()
        assert.is_function(rs.encode)
    end)

    it("exports decode as a function", function()
        assert.is_function(rs.decode)
    end)

    it("exports syndromes as a function", function()
        assert.is_function(rs.syndromes)
    end)

    it("exports build_generator as a function", function()
        assert.is_function(rs.build_generator)
    end)

    it("exports error_locator as a function", function()
        assert.is_function(rs.error_locator)
    end)

end)

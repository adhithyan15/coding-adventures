-- test_conformance.lua — the shared rng corpus (spec rng.md, "Conformance
-- corpus"), read from its flat form code/specs/fixtures/rng-v1/cases.txt.
--
-- The corpus pins exact outputs every language's rng package must reproduce;
-- its results come from the independent reference
-- code/scripts/rng_conformance.py. Each line is whitespace-separated:
--
--     case <id> <generator> <seed>
--     next_u32 <result>
--     next_u64 <result>
--     next_float <16 hex digits of the IEEE-754 bits>
--     next_int_in_range <min> <max> <result | error>
--     end
--
-- busted runs from the tests directory. Lua integers are signed 64-bit and
-- wrap, and tonumber turns decimals beyond 2^63 into floats, so numbers are
-- parsed digit by digit with wrapping arithmetic: an unsigned 64-bit value
-- becomes the integer with the same bit pattern.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local rng = require("coding_adventures.rng")

local CORPUS = "../../../../specs/fixtures/rng-v1/cases.txt"

-- Decimal (optionally negative) or hex text to a wrapping 64-bit integer.
local function parse(text, base)
    base = base or 10
    local negative = text:sub(1, 1) == "-"
    if negative then text = text:sub(2) end
    local n = 0
    for i = 1, #text do
        n = n * base + tonumber(text:sub(i, i), base)
    end
    if negative then n = -n end
    return n
end

local function float_bits(x)
    return (string.unpack(">i8", string.pack(">d", x)))
end

local GENERATORS = { lcg = rng.LCG, xorshift64 = rng.Xorshift64, pcg32 = rng.PCG32 }

describe("rng-v1 corpus", function()
    it("reproduces every case", function()
        local file = assert(io.open(CORPUS, "r"))
        local cases, calls, id, g = 0, 0, "", nil
        for line in file:lines() do
            if line ~= "" and line:sub(1, 1) ~= "#" then
                local t = {}
                for word in line:gmatch("%S+") do t[#t + 1] = word end
                local op = t[1]
                if op == "case" then
                    id = t[2]
                    g = GENERATORS[t[3]].new(parse(t[4]))
                    cases = cases + 1
                elseif op == "end" then
                    g = nil
                elseif op == "next_u32" then
                    calls = calls + 1
                    assert.are.equal(parse(t[2]), g:next_u32(), id)
                elseif op == "next_u64" then
                    calls = calls + 1
                    assert.are.equal(parse(t[2]), g:next_u64(), id)
                elseif op == "next_float" then
                    calls = calls + 1
                    assert.are.equal(parse(t[2], 16), float_bits(g:next_float()), id)
                elseif op == "next_int_in_range" then
                    calls = calls + 1
                    local lo, hi = parse(t[2]), parse(t[3])
                    if t[4] == "error" then
                        -- Raised before any draw, so the sequence carries on.
                        assert.has_error(function() g:next_int_in_range(lo, hi) end)
                    else
                        assert.are.equal(parse(t[4]), g:next_int_in_range(lo, hi), id)
                    end
                else
                    error("malformed corpus line: " .. line)
                end
            end
        end
        file:close()
        -- Guard against a truncated or empty file passing vacuously.
        assert.is_true(cases >= 27)
        assert.is_true(calls >= 150)
    end)
end)

describe("regressions for the v1 range bug", function()
    it("reaches beyond 2^32 for wide ranges", function()
        local g = rng.PCG32.new(3)
        local beyond = false
        for _ = 1, 64 do
            if g:next_int_in_range(0, 1 << 40) >= 1 << 32 then beyond = true end
        end
        assert.is_true(beyond)
    end)

    it("covers the whole integer domain without dividing by zero", function()
        local g = rng.LCG.new(3)
        local negative, positive = false, false
        for _ = 1, 64 do
            local v = g:next_int_in_range(math.mininteger, math.maxinteger)
            negative = negative or v < 0
            positive = positive or v > 0
        end
        assert.is_true(negative and positive)
    end)

    it("handles spans that are multiples of 2^32", function()
        local g = rng.Xorshift64.new(3)
        for _ = 1, 100 do
            local v = g:next_int_in_range(0, (1 << 33) - 1)
            assert.is_true(v >= 0 and v < 1 << 33)
        end
    end)

    it("terminates for span 2^63 + 1", function()
        local g = rng.Xorshift64.new(3)
        for _ = 1, 100 do
            assert.is_true(g:next_int_in_range(math.mininteger, 0) <= 0)
        end
    end)

    it("rejects non-integer or reversed bounds without drawing", function()
        local g, fresh = rng.LCG.new(9), rng.LCG.new(9)
        assert.has_error(function() g:next_int_in_range(1, 0) end)
        assert.has_error(function() g:next_int_in_range(0.5, 2) end)
        assert.has_error(function() g:next_int_in_range(0, 2 ^ 70) end)
        assert.are.equal(fresh:next_u32(), g:next_u32())
    end)
end)

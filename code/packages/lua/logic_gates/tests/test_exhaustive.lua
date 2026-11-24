-- Lane-local exhaustive tests for the 11 basic gates (LGE-ID-lua).
--
-- Covers: literal truth tables, NAND-built == direct, every rejected value in
-- every argument position (structured INVALID_BIT error naming the first bad
-- parameter), the type-confusion matrix, arity, and result type.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local lg = require("coding_adventures.logic_gates")

-- name -> { arity, truth table keyed "ab" (or "a") }
local TRUTH = {
    AND      = { 2, { ["00"] = 0, ["01"] = 0, ["10"] = 0, ["11"] = 1 } },
    OR       = { 2, { ["00"] = 0, ["01"] = 1, ["10"] = 1, ["11"] = 1 } },
    XOR      = { 2, { ["00"] = 0, ["01"] = 1, ["10"] = 1, ["11"] = 0 } },
    NAND     = { 2, { ["00"] = 1, ["01"] = 1, ["10"] = 1, ["11"] = 0 } },
    NOR      = { 2, { ["00"] = 1, ["01"] = 0, ["10"] = 0, ["11"] = 0 } },
    XNOR     = { 2, { ["00"] = 1, ["01"] = 0, ["10"] = 0, ["11"] = 1 } },
    NAND_AND = { 2, { ["00"] = 0, ["01"] = 0, ["10"] = 0, ["11"] = 1 } },
    NAND_OR  = { 2, { ["00"] = 0, ["01"] = 1, ["10"] = 1, ["11"] = 1 } },
    NAND_XOR = { 2, { ["00"] = 0, ["01"] = 1, ["10"] = 1, ["11"] = 0 } },
    NOT      = { 1, { ["0"] = 1, ["1"] = 0 } },
    NAND_NOT = { 1, { ["0"] = 1, ["1"] = 0 } },
}
local NAMES = { "AND", "OR", "NOT", "XOR", "NAND", "NOR", "XNOR",
                "NAND_NOT", "NAND_AND", "NAND_OR", "NAND_XOR" }
local PARAMS = { "a", "b" }

-- Hostile values: metamethods that must never be consulted or crash us.
local hostile = setmetatable({}, {
    __eq = function() return true end,
    __index = function() error("__index called") end,
    __tostring = function() error("__tostring called") end,
    __len = function() error("__len called") end,
})
local user = io.stdout

local function bad_values()
    local list = {
        { "1.0", 1.0 }, { "0.0", 0.0 }, { "-0.0", -0.0 }, { "nan", 0 / 0 },
        { "inf", math.huge }, { "-inf", -math.huge },
        { "'1'", "1" }, { "'0'", "0" }, { "''", "" }, { "true", true }, { "false", false },
        { "nil", nil }, { "table", {} }, { "hostile", hostile }, { "function", print },
        { "userdata", user }, { "coroutine", coroutine.create(print) },
        { "mininteger", math.mininteger }, { "maxinteger", math.maxinteger },
        { "2^31", 1 << 31 }, { "-2^31", -(1 << 31) },
        { "2^53", 1 << 53 }, { "-2^53", -(1 << 53) },
        { "longstring", string.rep("x", 10000) },
    }
    for i = -1024, 1024 do
        if i ~= 0 and i ~= 1 then list[#list + 1] = { tostring(i), i } end
    end
    return list
end

local function call(name, ...)
    return pcall(lg[name], ...)
end

describe("exhaustive: literal truth tables", function()
    for _, name in ipairs(NAMES) do
        local arity, table_ = TRUTH[name][1], TRUTH[name][2]
        it(name .. " matches its truth table", function()
            for key, want in pairs(table_) do
                local args = {}
                for i = 1, arity do args[i] = tonumber(key:sub(i, i)) end
                local got = lg[name](table.unpack(args, 1, arity))
                assert.are.equal(want, got)
                assert.are.equal("integer", math.type(got))
            end
        end)
    end

    it("NAND-built gates equal the direct gates", function()
        for a = 0, 1 do
            assert.are.equal(lg.NOT(a), lg.NAND_NOT(a))
            for b = 0, 1 do
                assert.are.equal(lg.AND(a, b), lg.NAND_AND(a, b))
                assert.are.equal(lg.OR(a, b), lg.NAND_OR(a, b))
                assert.are.equal(lg.XOR(a, b), lg.NAND_XOR(a, b))
            end
        end
    end)

    it("valid inputs never error and results are integer 0 or 1", function()
        for _, name in ipairs(NAMES) do
            local arity = TRUTH[name][1]
            for a = 0, 1 do
                for b = 0, 1 do
                    local ok, r
                    if arity == 1 then ok, r = call(name, a) else ok, r = call(name, a, b) end
                    assert.is_true(ok)
                    assert.are.equal("integer", math.type(r))
                    assert.is_true(r == 0 or r == 1)
                end
            end
        end
    end)
end)

describe("exhaustive: invalid bits are rejected with a structured error", function()
    local values = bad_values()

    for _, name in ipairs(NAMES) do
        local arity = TRUTH[name][1]
        for pos = 1, arity do
            it(name .. " rejects every bad value in position " .. pos, function()
                for _, case in ipairs(values) do
                    for other = 0, 1 do
                        local args = { n = arity }
                        for i = 1, arity do args[i] = other end
                        args[pos] = case[2]
                        local ok, err = pcall(lg[name], table.unpack(args, 1, arity))
                        assert.is_false(ok, name .. " accepted " .. case[1])
                        assert.is_true(lg.is_invalid_bit(err), "not INVALID_BIT for " .. case[1])
                        assert.are.equal("INVALID_BIT", err.code)
                        assert.are.equal(PARAMS[pos], err.param)
                        assert.is_string(err.value)
                        assert.is_true(#err.value < 200)
                        assert.is_string(tostring(err))
                    end
                end
            end)
        end

        if arity == 2 then
            it(name .. " names the FIRST invalid parameter when both are bad", function()
                local ok, err = call(name, "x", 7)
                assert.is_false(ok)
                assert.are.equal("a", err.param)
                ok, err = call(name, 1, 7)
                assert.are.equal("b", err.param)
            end)
        end

        it(name .. " rejects missing arguments (nil)", function()
            local ok, err = call(name)
            assert.is_false(ok)
            assert.is_true(lg.is_invalid_bit(err))
            assert.are.equal("a", err.param)
            if arity == 2 then
                ok, err = call(name, 1)
                assert.is_false(ok)
                assert.is_true(lg.is_invalid_bit(err))
                assert.are.equal("b", err.param)
            end
        end)

        it(name .. " rejects extra arguments", function()
            local args = { 1, 1, 1 }
            if arity == 1 then args = { 1, 1 } end
            local ok, err = pcall(lg[name], table.unpack(args))
            assert.is_false(ok)
            assert.is_true(lg.is_invalid_arity(err))
            assert.is_false(lg.is_invalid_bit(err))
            -- an explicit trailing nil is still an extra argument
            ok, err = pcall(lg[name], 1, arity == 2 and 1 or nil, nil, nil)
            assert.is_false(ok)
        end)
    end

    it("the error renders a readable message and is not a string", function()
        local ok, err = call("AND", 1.0, 1)
        assert.is_false(ok)
        assert.are.equal("table", type(err))
        assert.truthy(tostring(err):find("a must be 0 or 1", 1, true))
        assert.truthy(err.value:find("float", 1, true))
    end)

    it("is_invalid_bit is false for foreign values", function()
        assert.is_false(lg.is_invalid_bit("logic_gates: a must be 0 or 1"))
        assert.is_false(lg.is_invalid_bit(nil))
        assert.is_false(lg.is_invalid_bit({ code = "INVALID_BIT" }))
        assert.is_false(lg.is_invalid_arity({ code = "INVALID_ARITY" }))
    end)

    it("never invokes metamethods of a hostile argument", function()
        for _, name in ipairs(NAMES) do
            local ok, err
            if TRUTH[name][1] == 1 then ok, err = call(name, hostile) else ok, err = call(name, hostile, 1) end
            assert.is_false(ok)
            assert.is_true(lg.is_invalid_bit(err))
            assert.are.equal("table", err.value)
        end
    end)

    it("ANDn / ORn also reject floats", function()
        assert.is_false((pcall(lg.ANDn, 1, 1.0)))
        assert.is_false((pcall(lg.ORn, 0.0, 0)))
        assert.are.equal(1, lg.ANDn(1, 1, 1))
        assert.are.equal(0, lg.ORn(0, 0, 0))
    end)
end)

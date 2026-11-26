-- LGE-LUA-HARDEN: replacing globals after load must not make validation
-- fail open.  Kept in its own file because the coverage hook (luacov) itself
-- depends on globals, so BUILD runs this file WITHOUT coverage.

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local lg = require("coding_adventures.logic_gates")

describe("replacing globals after load cannot make validation fail open", function()
    local noop = function() end
    local function battery()
        local results = {}
        local function try(name, f, ...)
            local ok, err = pcall(f, ...)
            results[name] = { ok = ok, err = err }
        end
        try("and_str", lg.AND, "x", 1)
        try("not_tbl", lg.NOT, {})
        try("andn", lg.ANDn, 1, 2)
        try("andn_few", lg.ANDn, 1)
        try("reg_hole", lg.Register, { 1, nil, 1 }, 1)
        try("reg_str", lg.Register, "x", 1)
        try("counter", lg.Counter, 1, 0, { bits = { 0 }, width = 2 })
        try("width", lg.new_counter_state, 99)
        try("good", lg.AND, 1, 1)
        return results
    end
    local targets = {
        { _G, "error" }, { _G, "type" }, { _G, "select" }, { _G, "setmetatable" },
        { _G, "getmetatable" }, { _G, "rawget" }, { _G, "rawset" }, { _G, "rawequal" },
        { _G, "rawlen" }, { _G, "next" }, { _G, "ipairs" }, { _G, "pairs" },
        { _G, "math" }, { _G, "tostring" }, { _G, "tonumber" }, { _G, "require" },
        { string, "format" }, { string, "sub" }, { string, "rep" },
        { math, "type" }, { math, "tointeger" },
        { table, "pack" }, { table, "unpack" }, { table, "concat" },
    }
    for _, target in ipairs(targets) do
        local lib, name = target[1], target[2]
        it("still rejects bad input when " .. name .. " is replaced", function()
            local original = lib[name]
            local pcall_ = pcall
            local results
            -- pcall is the one global the harness itself needs; capture first.
            lib[name] = noop
            local ok, r = pcall_(battery)
            lib[name] = original
            assert.is_true(ok)
            results = r
            assert.is_false(results.and_str.ok)
            assert.is_true(lg.is_invalid_bit(results.and_str.err))
            assert.is_true(lg.is_invalid_bit(results.not_tbl.err))
            assert.is_true(lg.is_invalid_bit(results.andn.err))
            assert.is_true(lg.is_invalid_arity(results.andn_few.err))
            assert.is_true(lg.is_invalid_shape(results.reg_hole.err))
            assert.is_true(lg.is_invalid_shape(results.reg_str.err))
            assert.is_true(lg.is_invalid_shape(results.counter.err))
            assert.is_true(lg.is_invalid_width(results.width.err))
            -- A valid call may fail CLOSED (the transistors dependency uses
            -- globals of its own) but must never return a wrong value.
            if results.good.ok then
                assert.are.equal(1, results.good.err)
            else
                assert.is_true(lg.is_internal(results.good.err))
            end
        end)
    end
end)


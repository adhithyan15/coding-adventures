-- LGE-LUA-HARDEN: strict table snapshots, capped n-ary gates, structured
-- errors, captured internals, unforgeable error identity.

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local lg = require("coding_adventures.logic_gates")
local gates = require("coding_adventures.logic_gates.gates")
local sequential = require("coding_adventures.logic_gates.sequential")

local function perr(f, ...)
    local ok, err = pcall(f, ...)
    assert.is_false(ok)
    return err
end

-- A metatable whose every hook counts the call (and, where legal, throws).
local function spy_meta(calls)
    local function hit(name)
        return function() calls[#calls + 1] = name; error("spy:" .. name) end
    end
    return {
        __index = hit("__index"), __len = hit("__len"), __eq = hit("__eq"),
        __tostring = hit("__tostring"), __name = "Spy", __pairs = hit("__pairs"),
        __newindex = hit("__newindex"), __call = hit("__call"),
    }
end

describe("sparse and holey tables are rejected, not truncated", function()
    it("Register rejects {1,nil,1} and {[1]=1,[3]=1}", function()
        assert.is_true(lg.is_invalid_shape(perr(lg.Register, { 1, nil, 1 }, 1)))
        assert.is_true(lg.is_invalid_shape(perr(lg.Register, { [1] = 1, [3] = 1 }, 1)))
    end)
    it("rejects extra non-sequence keys", function()
        assert.is_true(lg.is_invalid_shape(perr(lg.Register, { 1, 0, x = 1 }, 1)))
        assert.is_true(lg.is_invalid_shape(perr(lg.Register, { [0] = 1, 1 }, 1)))
    end)
    it("state lists and counter bits reject holes too", function()
        local ff = lg.new_flip_flop_state
        assert.is_true(lg.is_invalid_shape(
            perr(lg.ShiftRegister, 0, 1, { [1] = ff(), [3] = ff() }, "left")))
        assert.is_true(lg.is_invalid_shape(
            perr(lg.Counter, 1, 0, { bits = { 0, nil, 0 }, width = 3 })))
        assert.is_true(lg.is_invalid_shape(
            perr(lg.Counter, 1, 0, { bits = { [1] = 0, [3] = 0 }, width = 3 })))
    end)
    it("still accepts dense tables and returns fresh arrays", function()
        local data = { 1, 0, 1 }
        local out, st = lg.Register(data, 1)
        assert.are.equal(3, #out)
        assert.are_not.equal(data, out)
        assert.are.equal(3, #st)
    end)
    it("snapshot enforces the cap before iterating further", function()
        local big = {}
        for i = 1, 65537 do big[i] = 0 end
        assert.is_true(lg.is_invalid_width(perr(lg.Register, big, 1)))
        -- Exactly at the cap the snapshot succeeds (checked directly: a full
        -- 65536-flip-flop simulation is too slow under coverage hooks).
        big[65537] = nil
        -- The 65536-bit data list passes the snapshot; the cheap state-length
        -- mismatch then proves we got past it.
        local err = perr(lg.Register, big, 1, { lg.new_flip_flop_state() })
        assert.is_true(lg.is_invalid_shape(err))
        assert.truthy(tostring(err):find("length mismatch", 1, true))
    end)
end)

describe("tables with metatables never run caller code", function()
    it("data / state / counter state with spy metatables", function()
        local calls = {}
        local function spied(t) return setmetatable(t, spy_meta(calls)) end
        local ff = lg.new_flip_flop_state
        assert.is_true(lg.is_invalid_shape(perr(lg.Register, spied({ 1, 0 }), 1)))
        assert.is_true(lg.is_invalid_shape(perr(lg.Register, { 1 }, 1, spied({ ff() }))))
        assert.is_true(lg.is_invalid_shape(perr(lg.Register, { 1 }, 1, { spied(ff()) })))
        assert.is_true(lg.is_invalid_shape(perr(lg.ShiftRegister, 0, 1, spied({ ff() }), "left")))
        assert.is_true(lg.is_invalid_shape(
            perr(lg.Counter, 1, 0, spied({ bits = { 0 }, width = 1 }))))
        assert.is_true(lg.is_invalid_shape(
            perr(lg.Counter, 1, 0, { bits = spied({ 0 }), width = 1 })))
        assert.is_true(lg.is_invalid_shape(perr(lg.DFlipFlop, 1, 1, spied(ff()))))
        assert.are.equal(0, #calls)
    end)
    it("a faked __metatable field does not smuggle a table through", function()
        local t = setmetatable({ 1 }, { __metatable = nil, __len = function() error("len") end })
        assert.is_true(lg.is_invalid_shape(perr(lg.Register, t, 1)))
        local t2 = setmetatable({ 1 }, { __metatable = false })
        assert.is_true(lg.is_invalid_shape(perr(lg.Register, t2, 1)))
    end)
    it("hostile values in error descriptions are never stringified", function()
        local calls = {}
        local v = setmetatable({}, spy_meta(calls))
        local err = perr(lg.AND, v, 1)
        assert.is_true(lg.is_invalid_bit(err))
        assert.are.equal("table", err.value)
        assert.is_string(tostring(err))
        assert.are.equal(0, #calls)
    end)
end)

describe("ANDn / ORn caps and structured arity errors", function()
    local function many(n, v)
        local t = {}
        for i = 1, n do t[i] = v end
        return t
    end
    it("accepts exactly MAX_INPUTS", function()
        assert.are.equal(65536, lg.MAX_INPUTS)
        assert.are.equal(1, lg.ANDn(table.unpack(many(65536, 1))))
    end)
    it("rejects MAX_INPUTS + 1 with a structured error", function()
        local err = perr(lg.ANDn, table.unpack(many(65537, 1)))
        assert.is_true(lg.is_invalid_arity(err))
        assert.are.equal("INVALID_ARITY", lg.error_code(err))
        assert.is_true(lg.is_invalid_arity(perr(lg.ORn, table.unpack(many(65537, 0)))))
    end)
    it("a very large argument list gets the structured cap error first", function()
        -- Effective limit: the Lua VM itself rejects roughly 500k+ varargs
        -- (stack overflow in the call prologue) before any package code can
        -- run; below that the cap error below fires before any copying.
        local err = perr(lg.ORn, table.unpack(many(100000, 0)))
        assert.is_true(lg.is_invalid_arity(err))
    end)
    it("too few inputs is a structured error, never a bare string", function()
        for _, f in ipairs({ lg.ANDn, lg.ORn }) do
            for _, args in ipairs({ { n = 0 }, { 1, n = 1 } }) do
                local err = perr(f, table.unpack(args, 1, args.n))
                assert.is_table(err)
                assert.is_true(lg.is_invalid_arity(err))
                assert.is_string(tostring(err))
            end
        end
    end)
    it("trailing nil counts as an input and fails validation", function()
        assert.is_true(lg.is_invalid_bit(perr(lg.ANDn, 1, 1, nil)))
    end)
    it("CMOS failures raise a structured INTERNAL error", function()
        local cmos = require("coding_adventures.transistors.cmos_gates")
        assert.is_function(cmos.CMOSNand)
        -- Force a failure by rebuilding the module with a failing CMOS gate.
        local saved = package.loaded["coding_adventures.logic_gates.gates"]
        local saved_internal = package.loaded["coding_adventures.logic_gates.internal"]
        package.loaded["coding_adventures.logic_gates.internal"] = nil
        local orig = cmos.CMOSAnd
        cmos.CMOSAnd = function()
            return { evaluate_digital = function() return nil, "boom" end }
        end
        package.loaded["coding_adventures.logic_gates.gates"] = nil
        local fresh = require("coding_adventures.logic_gates.gates")
        cmos.CMOSAnd = orig
        package.loaded["coding_adventures.logic_gates.gates"] = saved
        package.loaded["coding_adventures.logic_gates.internal"] = saved_internal
        local err = perr(fresh.AND, 1, 1)
        assert.is_true(fresh.is_internal(err))
        assert.are.equal("INTERNAL", fresh.error_code(err))
        assert.is_nil(tostring(err):find("boom", 1, true))
    end)
end)

describe("composites bind to captured internals", function()
    it("overwriting an export through the public table raises", function()
        assert.has_error(function() gates.NAND = function() return 0 end end)
        assert.has_error(function() lg.NAND = function() return 0 end end)
        assert.has_error(function() sequential.Counter = function() end end)
        assert.has_error(function() lg.NEW_EXPORT = 1 end)
        assert.are.equal(0, lg.NAND(1, 1))
    end)
    it("even rawset rebinding cannot change composites or circuits", function()
        local names = { "NAND", "NOR", "AND", "OR", "NOT", "XOR" }
        local fake = function() return 0 end
        for _, n in ipairs(names) do
            rawset(gates, n, fake)
            rawset(lg, n, fake)
        end
        local ok = pcall(function()
            assert.are.equal(1, lg.NAND_AND(1, 1))
            assert.are.equal(1, lg.NAND_OR(0, 1))
            assert.are.equal(1, lg.NAND_XOR(1, 0))
            assert.are.equal(1, lg.NAND_NOT(0))
            assert.are.equal(1, lg.ANDn(1, 1, 1))
            local q = sequential.SRLatch(1, 0, 0, 1)
            assert.are.equal(1, q)
            local out = lg.Counter(1, 0, lg.new_counter_state(2))
            assert.are.same({ 1, 0 }, out)
        end)
        for _, n in ipairs(names) do rawset(gates, n, nil); rawset(lg, n, nil) end
        assert.is_true(ok)
        assert.are.equal(0, lg.NAND(1, 1))
    end)
    it("minting helpers are not public", function()
        assert.is_nil(gates.raise_invalid_width)
        assert.is_nil(gates.raise_invalid_shape)
        assert.is_nil(gates.validate_bit)
        assert.is_nil(gates.snapshot_list)
        assert.is_nil(gates.sequential.raise)
    end)
    it("no public path yields the kit; a self-built kit is unrelated", function()
        local build = require("coding_adventures.logic_gates.internal")
        assert.is_function(build)
        local mine = build()
        -- A caller-built kit mints errors the package does not recognise ...
        local ok, forged = pcall(mine.validate_bit, 2, "a")
        assert.is_false(ok)
        assert.is_false(lg.is_invalid_bit(forged))
        -- ... and mutating it changes nothing about the real circuits.
        mine.raise = function() end
        mine.snapshot_list = function() return {}, 0 end
        mine.validate_bit = function(v) return v end
        assert.is_true(lg.is_invalid_width(perr(lg.new_counter_state, -5)))
        assert.is_true(lg.is_invalid_bit(perr(lg.Register, { 2 }, 1)))
        assert.is_true(lg.is_invalid_bit(perr(lg.AND, 2, 1)))
    end)
    it("patching the transistors class cannot redirect or corrupt gates", function()
        local cmos = require("coding_adventures.transistors.cmos_gates")
        local idx = getmetatable(cmos.CMOSNand()).__index
        local orig = idx.evaluate_digital
        idx.evaluate_digital = function() return 1 end
        local r = lg.NAND(1, 1)
        idx.evaluate_digital = orig
        assert.are.equal(0, r)
    end)
end)

describe("error identity is unforgeable and read-only", function()
    local function genuine() return perr(lg.AND, 2, 1) end

    it("a forged table is not an error of ours", function()
        local real = genuine()
        local forged = { code = "INVALID_BIT", param = "a", value = "integer 2" }
        assert.is_false(lg.is_invalid_bit(forged))
        assert.is_nil(lg.error_code(forged))
        assert.is_false(lg.is_invalid_bit(setmetatable({}, { __index = real })))
    end)
    it("fields are read-only through the public interface", function()
        local err = genuine()
        assert.has_error(function() err.code = "X" end)
        assert.has_error(function() err.param = "X" end)
        assert.has_error(function() err.extra = 1 end)
        assert.are.equal("INVALID_BIT", err.code)
        assert.is_true(lg.is_invalid_bit(err))
    end)
    it("metatable is locked", function()
        local err = genuine()
        assert.are.equal("logic_gates error", getmetatable(err))
        assert.has_error(function() setmetatable(err, {}) end)
    end)
    it("rawset tampering cannot change what the library believes", function()
        local err = genuine()
        rawset(err, "code", "INVALID_SHAPE")
        assert.is_true(lg.is_invalid_bit(err))
        assert.is_false(lg.is_invalid_shape(err))
        assert.are.equal("INVALID_BIT", lg.error_code(err))
        assert.truthy(tostring(err):find("must be 0 or 1", 1, true))
    end)
    it("all kinds are recognised only by their own predicate", function()
        local e = {
            bit = genuine(),
            arity = perr(lg.AND, 1, 1, 1),
            width = perr(lg.new_counter_state, 65),
            shape = perr(lg.Register, "x", 1),
        }
        assert.are.equal("INVALID_ARITY", lg.error_code(e.arity))
        assert.is_true(lg.is_invalid_width(e.width))
        assert.is_true(lg.is_invalid_shape(e.shape))
        assert.is_false(lg.is_invalid_bit(e.shape))
        assert.is_false(lg.is_invalid_arity(e.bit))
        assert.is_false(lg.is_internal(e.bit))
    end)
end)

describe("fixed-text value descriptions", function()
    it("describes by type only", function()
        assert.are.equal("float", perr(lg.AND, 1.5, 1).value)
        assert.are.equal("float", perr(lg.AND, 0 / 0, 1).value)
        assert.are.equal("string", perr(lg.AND, ("x"):rep(100000), 1).value)
        assert.are.equal("boolean", perr(lg.AND, true, 1).value)
        assert.are.equal("function", perr(lg.AND, print, 1).value)
        assert.are.equal("nil", perr(lg.AND, nil, 1).value)
        assert.are.equal("integer 2", perr(lg.AND, 2, 1).value)
    end)
    it("huge integers are described by size, not value", function()
        assert.are.equal("integer (large magnitude)", perr(lg.AND, math.maxinteger, 1).value)
        assert.are.equal("integer (large magnitude)", perr(lg.AND, math.mininteger, 1).value)
    end)
    it("string coercion metamethods on strings do not matter", function()
        local err = perr(lg.NOT, "1")
        assert.are.equal("string", err.value)
    end)
end)

describe("error text is captured at creation", function()
    it("rawset of a hostile value does not run caller code in tostring", function()
        local calls = 0
        local err = perr(lg.AND, 2, 1)
        rawset(err, "value", setmetatable({}, { __tostring = function() calls = calls + 1 return "x" end }))
        rawset(err, "param", setmetatable({}, { __tostring = function() calls = calls + 1 return "x" end }))
        local text = tostring(err)
        assert.are.equal(0, calls)
        assert.truthy(text:find("integer 2", 1, true))
    end)
end)

describe("public modules are read-only", function()
    it("module tables and error metatables are locked", function()
        assert.are.equal("logic_gates module (read-only)", getmetatable(lg))
        assert.has_error(function() setmetatable(lg, {}) end)
        local n = 0
        for _ in pairs(lg) do n = n + 1 end
        assert.is_true(n > 20)
    end)
end)

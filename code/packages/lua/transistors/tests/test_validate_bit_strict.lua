-- Strict bit validation for transistors.types.validate_bit (LGE-ID-lua).
--
-- In Lua 5.3+ `1.0 == 1`, so an `==`-based validator silently accepts floats.
-- The cross-lane contract (LG02) rejects them; validate_bit must too.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local T = require("coding_adventures.transistors.types")
local cmos = require("coding_adventures.transistors.cmos_gates")

local hostile = setmetatable({}, {
    __eq = function() return true end,
    __tostring = function() error("__tostring must not be called") end,
    __index = function() error("__index must not be called") end,
})

local bad = {
    { "float 1.0", 1.0 }, { "float 0.0", 0.0 }, { "float -0.0", -0.0 },
    { "nan", 0 / 0 }, { "inf", math.huge }, { "-inf", -math.huge },
    { "string 1", "1" }, { "string 0", "0" }, { "true", true }, { "false", false },
    { "table", {} }, { "function", print }, { "coroutine", coroutine.create(print) },
    { "hostile table", hostile }, { "2", 2 }, { "-1", -1 },
    { "mininteger", math.mininteger }, { "maxinteger", math.maxinteger },
}

describe("validate_bit strictness", function()
    it("accepts integer 0 and 1", function()
        assert.is_nil(T.validate_bit(0, "x"))
        assert.is_nil(T.validate_bit(1, "x"))
    end)

    for _, case in ipairs(bad) do
        it("rejects " .. case[1], function()
            local err = T.validate_bit(case[2], "x")
            assert.is_string(err)
            assert.truthy(err:find("x must be 0 or 1", 1, true))
        end)
    end

    it("rejects nil", function()
        assert.is_string(T.validate_bit(nil, "x"))
    end)

    it("cmos gates reject 1.0 through the error return", function()
        local r, err = cmos.CMOSNand():evaluate_digital(1.0, 1)
        assert.is_nil(r)
        assert.is_string(err)
        r, err = cmos.CMOSInverter():evaluate_digital(0.0)
        assert.is_nil(r)
        assert.is_string(err)
    end)
end)

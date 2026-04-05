-- Tests for wasm-validator

local m = require("coding_adventures.wasm_validator")

describe("wasm-validator", function()
    it("has a VERSION", function()
        assert.is_not_nil(m.VERSION)
        assert.equals("0.1.0", m.VERSION)
    end)

    -- TODO: Add real tests
end)

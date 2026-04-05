-- Tests for wasm-runtime

local m = require("coding_adventures.wasm_runtime")

describe("wasm-runtime", function()
    it("has a VERSION", function()
        assert.is_not_nil(m.VERSION)
        assert.equals("0.1.0", m.VERSION)
    end)

    -- TODO: Add real tests
end)

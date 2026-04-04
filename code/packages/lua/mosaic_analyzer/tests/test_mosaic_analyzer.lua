-- Tests for mosaic-analyzer

local m = require("coding_adventures.mosaic_analyzer")

describe("mosaic-analyzer", function()
    it("has a VERSION", function()
        assert.is_not_nil(m.VERSION)
        assert.equals("0.1.0", m.VERSION)
    end)

    -- TODO: Add real tests
end)

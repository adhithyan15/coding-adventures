-- Tests for mosaic-emit-webcomponent

local m = require("coding_adventures.mosaic_emit_webcomponent")

describe("mosaic-emit-webcomponent", function()
    it("has a VERSION", function()
        assert.is_not_nil(m.VERSION)
        assert.equals("0.1.0", m.VERSION)
    end)

    -- TODO: Add real tests
end)

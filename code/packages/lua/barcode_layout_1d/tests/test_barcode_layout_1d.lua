package.path = (
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    "../../paint_instructions/src/?.lua;" ..
    "../../paint_instructions/src/?/init.lua;" ..
    package.path
)

local layout = require("coding_adventures.barcode_layout_1d")

describe("barcode_layout_1d", function()
    it("builds runs from a binary pattern", function()
        local runs = layout.runs_from_binary_pattern("111001")
        assert.equal("bar", runs[1].color)
        assert.equal(3, runs[1].modules)
        assert.equal("space", runs[2].color)
    end)

    it("lays out a paint scene", function()
        local runs = layout.runs_from_width_pattern(
            "WNW",
            {"bar", "space", "bar"},
            { source_char = "A", source_index = 0 }
        )
        local scene = layout.layout_barcode_1d(runs)
        assert.equal(27 * 4, scene.width)
        assert.equal(120, scene.height)
        assert.equal(2, #scene.instructions)
    end)
end)

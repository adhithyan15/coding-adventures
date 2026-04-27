package.path = (
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    "../../barcode_layout_1d/src/?.lua;" ..
    "../../barcode_layout_1d/src/?/init.lua;" ..
    "../../paint_instructions/src/?.lua;" ..
    "../../paint_instructions/src/?/init.lua;" ..
    package.path
)

local code39 = require("coding_adventures.code39")

describe("code39 module", function()
    it("encodes a known pattern", function()
        local encoded = code39.encode_code39_char("A")
        assert.equal("WNNNNWNNW", encoded.pattern)
    end)

    it("expands module-based runs", function()
        local runs = code39.expand_code39_runs("A")
        assert.equal(29, #runs)
        assert.equal("inter-character-gap", runs[10].role)
        assert.equal(3, runs[11].modules)
    end)

    it("emits a paint scene", function()
        local scene = code39.draw_code39("A")
        assert.equal("code39", scene.metadata.symbology)
        assert.equal(120, scene.height)
        assert.is_true(scene.width > 0)
    end)
end)

package.path = (
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    "../../barcode_layout_1d/src/?.lua;" ..
    "../../barcode_layout_1d/src/?/init.lua;" ..
    "../../paint_instructions/src/?.lua;" ..
    "../../paint_instructions/src/?/init.lua;" ..
    package.path
)

local codabar = require("coding_adventures.codabar")

describe("codabar module", function()
    it("normalizes with default guards", function()
        assert.equal("A40156A", codabar.normalize_codabar("40156"))
    end)

    it("expands inter-character gaps", function()
        local runs = codabar.expand_codabar_runs("40156")
        local found_gap = false
        for _, run in ipairs(runs) do
            if run.role == "inter-character-gap" then
                found_gap = true
                break
            end
        end
        assert.is_true(found_gap)
    end)

    it("emits a paint scene", function()
        local scene = codabar.draw_codabar("40156")
        assert.equal("codabar", scene.metadata.symbology)
        assert.equal("A", scene.metadata.start)
        assert.equal("A", scene.metadata.stop)
        assert.is_true(scene.width > 0)
    end)
end)

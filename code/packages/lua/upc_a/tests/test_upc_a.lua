package.path = (
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    "../../barcode_layout_1d/src/?.lua;" ..
    "../../barcode_layout_1d/src/?/init.lua;" ..
    "../../paint_instructions/src/?.lua;" ..
    "../../paint_instructions/src/?/init.lua;" ..
    package.path
)

local upc_a = require("coding_adventures.upc_a")

describe("upc_a module", function()
    it("computes a known check digit", function()
        assert.equal("2", upc_a.compute_upc_a_check_digit("03600029145"))
    end)

    it("emits 95 modules", function()
        local total_modules = 0
        for _, run in ipairs(upc_a.expand_upc_a_runs("036000291452")) do
            total_modules = total_modules + run.modules
        end
        assert.equal(95, total_modules)
    end)

    it("emits a paint scene", function()
        local scene = upc_a.draw_upc_a("03600029145")
        assert.equal("upc-a", scene.metadata.symbology)
        assert.is_true(scene.width > 0)
    end)
end)

package.path = (
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    "../../barcode_layout_1d/src/?.lua;" ..
    "../../barcode_layout_1d/src/?/init.lua;" ..
    "../../paint_instructions/src/?.lua;" ..
    "../../paint_instructions/src/?/init.lua;" ..
    package.path
)

local ean_13 = require("coding_adventures.ean_13")

describe("ean_13 module", function()
    it("computes a known check digit", function()
        assert.equal("1", ean_13.compute_ean_13_check_digit("400638133393"))
    end)

    it("tracks the left parity pattern", function()
        assert.equal("LGLLGG", ean_13.left_parity_pattern("4006381333931"))
    end)

    it("emits 95 modules", function()
        local total_modules = 0
        for _, run in ipairs(ean_13.expand_ean_13_runs("4006381333931")) do
            total_modules = total_modules + run.modules
        end
        assert.equal(95, total_modules)
    end)
end)

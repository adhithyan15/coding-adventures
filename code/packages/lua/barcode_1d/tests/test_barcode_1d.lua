package.path = (
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    "../../code39/src/?.lua;" ..
    "../../code39/src/?/init.lua;" ..
    "../../barcode_layout_1d/src/?.lua;" ..
    "../../barcode_layout_1d/src/?/init.lua;" ..
    "../../paint_instructions/src/?.lua;" ..
    "../../paint_instructions/src/?/init.lua;" ..
    "../../pixel-container/src/?.lua;" ..
    "../../pixel-container/src/?/init.lua;" ..
    "../../paint_vm_metal_native/src/?.lua;" ..
    "../../paint_vm_metal_native/src/?/init.lua;" ..
    "../../paint_codec_png_native/src/?.lua;" ..
    "../../paint_codec_png_native/src/?/init.lua;" ..
    package.path
)

local barcode_1d = require("coding_adventures.barcode_1d")

describe("barcode_1d", function()
    it("builds a code39 scene", function()
        local scene = barcode_1d.build_scene("HELLO-123", "code39")
        assert.equal("code39", scene.metadata.symbology)
        assert.equal(120, scene.height)
        assert.is_true(scene.width > 0)
    end)

    it("reports backend availability", function()
        assert.is_true(barcode_1d.current_backend() == nil or barcode_1d.current_backend() == "metal")
    end)

    it("renders PNG bytes when Metal is available", function()
        if barcode_1d.current_backend() ~= "metal" then
            return
        end

        local png = barcode_1d.render_png("HELLO-123", "code39")
        assert.equal("\137PNG\r\n\26\n", png:sub(1, 8))
        assert.is_true(#png > 100)
    end)
end)

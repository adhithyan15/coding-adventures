package.path = (
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    "../../pixel-container/src/?.lua;" ..
    "../../pixel-container/src/?/init.lua;" ..
    package.path
)

local paint_codec_png_native = require("coding_adventures.paint_codec_png_native")
local pixel_container = require("coding_adventures.pixel_container")

describe("paint_codec_png_native", function()
    it("encodes a one-pixel PNG", function()
        local pixels = pixel_container.new(1, 1)
        pixel_container.set_pixel(pixels, 0, 0, 0, 0, 0, 255)
        local png = paint_codec_png_native.encode(pixels)
        assert.equal("\137PNG\r\n\26\n", png:sub(1, 8))
    end)
end)

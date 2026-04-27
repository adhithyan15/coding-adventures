package.path = (
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    package.path
)

local paint = require("coding_adventures.paint_instructions")

describe("paint_instructions", function()
    it("builds a rect", function()
        local rect = paint.paint_rect(1, 2, 3, 4)
        assert.equal("rect", rect.kind)
        assert.equal(3, rect.width)
    end)

    it("builds a scene", function()
        local scene = paint.paint_scene(10, 20, {})
        assert.equal(10, scene.width)
        assert.equal(20, scene.height)
    end)
end)

package.path = table.concat({
    "src/?.lua",
    "src/?/init.lua",
    "../paint_instructions/src/?.lua",
    "../paint_instructions/src/?/init.lua",
    package.path,
}, ";")

local paint_instructions = require("coding_adventures.paint_instructions")
local paint_vm_ascii = require("coding_adventures.paint_vm_ascii")

describe("paint_vm_ascii", function()
    it("exposes a version", function()
        assert.are.equal("0.1.0", paint_vm_ascii.VERSION)
    end)

    it("renders filled rects as block characters", function()
        local scene = paint_instructions.paint_scene(3, 2, {
            paint_instructions.paint_rect(0, 0, 2, 1, "#000000"),
        })

        local result = paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
        assert.is_true(result:find("█", 1, true) ~= nil)
    end)

    it("ignores transparent rects", function()
        local scene = paint_instructions.paint_scene(3, 2, {
            paint_instructions.paint_rect(0, 0, 2, 1, "transparent"),
        })

        local result = paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
        assert.are.equal("", result)
    end)
end)

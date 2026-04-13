package.path = (
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    "../../pixel-container/src/?.lua;" ..
    "../../pixel-container/src/?/init.lua;" ..
    package.path
)

local paint_vm_metal_native = require("coding_adventures.paint_vm_metal_native")

describe("paint_vm_metal_native", function()
    it("reports availability on supported runtimes", function()
        assert.is_true(paint_vm_metal_native.available())
    end)
end)

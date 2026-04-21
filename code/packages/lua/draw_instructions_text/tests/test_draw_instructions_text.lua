-- Tests for coding_adventures.draw_instructions_text

package.path = "../src/?.lua;../src/?/init.lua;"
  .. "../../draw_instructions/src/?.lua;../../draw_instructions/src/?/init.lua;"
  .. package.path

local Draw = require("coding_adventures.draw_instructions")
local text = require("coding_adventures.draw_instructions_text")

describe("draw_instructions_text", function()
    it("has a VERSION", function()
        assert.is_not_nil(text.VERSION)
        assert.equals("0.1.0", text.VERSION)
    end)

    it("renders filled rectangles as block characters", function()
        local scene = Draw.create_scene(2, 1, {
            Draw.draw_rect(0, 0, 1, 0, "#000000"),
        }, "#fff")
        local result = text.render_text(scene, { scale_x = 1, scale_y = 1 })
        assert.is_true(result:find("\226\150\136", 1, true) ~= nil)
    end)

    it("renders transparent rectangles as stroked boxes", function()
        local scene = Draw.create_scene(5, 3, {
            Draw.draw_rect(0, 0, 4, 2, "transparent"),
        }, "#fff")
        local result = text.render_text(scene, { scale_x = 1, scale_y = 1 })
        local lines = {}
        for line in result:gmatch("[^\n]+") do lines[#lines + 1] = line end
        assert.equals("\226\148\140\226\148\128\226\148\128\226\148\128\226\148\144", lines[1])
        assert.equals("\226\148\130   \226\148\130", lines[2])
        assert.equals("\226\148\148\226\148\128\226\148\128\226\148\128\226\148\152", lines[3])
    end)

    it("renders text labels with start alignment", function()
        local scene = Draw.create_scene(10, 1, {
            Draw.draw_text(0, 0, "Hello", "#000", "monospace", 16, "start"),
        }, "#fff")
        assert.equals("Hello", text.render_text(scene, { scale_x = 1, scale_y = 1 }))
    end)

    it("renders horizontal and vertical lines", function()
        local scene = Draw.create_scene(5, 3, {
            Draw.draw_line(0, 1, 4, 1, "#000"),
            Draw.draw_line(2, 0, 2, 2, "#000"),
        }, "#fff")
        local result = text.render_text(scene, { scale_x = 1, scale_y = 1 })
        assert.is_true(result:find("\226\148\128", 1, true) ~= nil)
        assert.is_true(result:find("\226\148\130", 1, true) ~= nil)
    end)

    it("clips manually-authored clip children", function()
        local scene = Draw.create_scene(10, 1, {
            {
                kind = "clip",
                x = 0,
                y = 0,
                width = 2,
                height = 0,
                children = {
                    Draw.draw_text(0, 0, "Hello", "#000", "monospace", 16, "start"),
                },
            },
        }, "#fff")
        assert.equals("Hel", text.render_text(scene, { scale_x = 1, scale_y = 1 }))
    end)
end)

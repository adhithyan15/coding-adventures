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

    describe("paint_rect", function()
        it("defaults fill to opaque black", function()
            local rect = paint.paint_rect(0, 0, 1, 1)
            assert.equal("#000000", rect.fill)
        end)

        it("defaults stroke to empty (no stroke) for backward compatibility", function()
            local rect = paint.paint_rect(0, 0, 1, 1, "#ff0000")
            assert.equal("", rect.stroke)
            assert.equal(0, rect.stroke_width)
        end)

        it("accepts an explicit stroke and stroke_width", function()
            local rect = paint.paint_rect(0, 0, 1, 1, "#ff0000", nil, "#00ff00", 2)
            assert.equal("#00ff00", rect.stroke)
            assert.equal(2, rect.stroke_width)
        end)

        it("copies metadata rather than aliasing the caller's table", function()
            local meta = { role = "data" }
            local rect = paint.paint_rect(0, 0, 1, 1, "#000000", meta)
            meta.role = "mutated"
            assert.equal("data", rect.metadata.role)
        end)
    end)

    describe("paint_line", function()
        it("builds a line instruction with the given endpoints and stroke", function()
            local line = paint.paint_line(0, 0, 10, 10, "#000000", 1)
            assert.equal("line", line.kind)
            assert.equal(0, line.x1)
            assert.equal(10, line.x2)
            assert.equal("#000000", line.stroke)
        end)
    end)

    describe("paint_glyph_placement / paint_glyph_run", function()
        it("builds a glyph placement", function()
            local placement = paint.paint_glyph_placement(65, 0, 0)
            assert.equal(65, placement.glyph_id)
        end)

        it("builds a glyph_run from a list of placements", function()
            local run = paint.paint_glyph_run({
                paint.paint_glyph_placement(72, 0, 0),
                paint.paint_glyph_placement(105, 8, 0),
            }, "terminal-mono", 16, "#000000")
            assert.equal("glyph_run", run.kind)
            assert.equal(2, #run.glyphs)
            assert.equal("terminal-mono", run.font_ref)
        end)

        it("copies the glyph list rather than aliasing the caller's table", function()
            local glyphs = { paint.paint_glyph_placement(65, 0, 0) }
            local run = paint.paint_glyph_run(glyphs, "f", 1, "#000000")
            glyphs[2] = paint.paint_glyph_placement(66, 1, 0)
            assert.equal(1, #run.glyphs)
        end)
    end)

    describe("transform2d / identity_transform", function()
        it("builds an identity transform", function()
            local t = paint.identity_transform()
            assert.equal(1, t.a)
            assert.equal(0, t.b)
            assert.equal(0, t.c)
            assert.equal(1, t.d)
            assert.equal(0, t.e)
            assert.equal(0, t.f)
        end)

        it("is_identity_transform treats nil as identity", function()
            assert.is_true(paint.is_identity_transform(nil))
        end)

        it("is_identity_transform recognizes a non-identity transform", function()
            local t = paint.transform2d(2, 0, 0, 1, 0, 0)
            assert.is_false(paint.is_identity_transform(t))
        end)
    end)

    describe("paint_group", function()
        it("builds a plain group with no options", function()
            local group = paint.paint_group({ paint.paint_rect(0, 0, 1, 1) })
            assert.equal("group", group.kind)
            assert.equal(1, #group.children)
            assert.is_nil(group.transform)
            assert.is_nil(group.opacity)
        end)

        it("accepts transform/opacity/metadata via opts", function()
            local t = paint.transform2d(2, 0, 0, 1, 0, 0)
            local group = paint.paint_group({}, { transform = t, opacity = 0.5, metadata = { k = "v" } })
            assert.equal(t, group.transform)
            assert.equal(0.5, group.opacity)
            assert.equal("v", group.metadata.k)
        end)
    end)

    describe("paint_clip", function()
        it("builds a clip with rectangle bounds and children", function()
            local clip = paint.paint_clip(0, 0, 10, 10, { paint.paint_rect(0, 0, 1, 1) })
            assert.equal("clip", clip.kind)
            assert.equal(10, clip.width)
            assert.equal(1, #clip.children)
        end)
    end)

    describe("paint_layer", function()
        it("builds a plain layer with no options", function()
            local layer = paint.paint_layer({ paint.paint_rect(0, 0, 1, 1) })
            assert.equal("layer", layer.kind)
            assert.is_false(layer.has_filters)
            assert.is_nil(layer.blend_mode)
        end)

        it("accepts has_filters/blend_mode/opacity/transform via opts", function()
            local layer = paint.paint_layer({}, {
                has_filters = true,
                blend_mode = "multiply",
                opacity = 0.75,
            })
            assert.is_true(layer.has_filters)
            assert.equal("multiply", layer.blend_mode)
            assert.equal(0.75, layer.opacity)
        end)
    end)

    describe("paint_path (pre-existing)", function()
        it("still builds a path instruction", function()
            local path = paint.paint_path({ { kind = "move_to", x = 0, y = 0 } })
            assert.equal("path", path.kind)
        end)
    end)
end)

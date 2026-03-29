-- Tests for coding_adventures.draw_instructions
--
-- Covers: draw_rect, draw_text, draw_line, draw_circle, draw_group,
--         create_scene, defaults, and metadata passing.
--
-- Lua 5.4 busted test suite.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local Draw = require("coding_adventures.draw_instructions")

describe("draw_instructions", function()

    -- -----------------------------------------------------------------------
    -- Version
    -- -----------------------------------------------------------------------

    it("has VERSION", function()
        assert.is_not_nil(Draw.VERSION)
        assert.equals("0.1.0", Draw.VERSION)
    end)

    -- -----------------------------------------------------------------------
    -- draw_rect
    -- -----------------------------------------------------------------------

    it("draw_rect returns a table with kind='rect'", function()
        local r = Draw.draw_rect(10, 20, 100, 50, "#ff0000")
        assert.equals("rect", r.kind)
    end)

    it("draw_rect stores x, y, width, height correctly", function()
        local r = Draw.draw_rect(10, 20, 100, 50, "#ff0000")
        assert.equals(10,  r.x)
        assert.equals(20,  r.y)
        assert.equals(100, r.width)
        assert.equals(50,  r.height)
    end)

    it("draw_rect stores the fill color", function()
        local r = Draw.draw_rect(0, 0, 10, 10, "#aabbcc")
        assert.equals("#aabbcc", r.fill)
    end)

    it("draw_rect defaults fill to '#000000' when not provided", function()
        local r = Draw.draw_rect(0, 0, 10, 10)
        assert.equals("#000000", r.fill)
    end)

    it("draw_rect has an empty metadata table by default", function()
        local r = Draw.draw_rect(0, 0, 10, 10)
        assert.is_not_nil(r.metadata)
        assert.equals(0, #r.metadata)
    end)

    it("draw_rect accepts and stores metadata", function()
        local r = Draw.draw_rect(0, 0, 10, 10, "#fff", {id="box1"})
        assert.equals("box1", r.metadata.id)
    end)

    it("draw_rect metadata is a copy, not the same table", function()
        local meta = {id = "original"}
        local r = Draw.draw_rect(0, 0, 10, 10, "#fff", meta)
        meta.id = "changed"
        assert.equals("original", r.metadata.id)
    end)

    -- -----------------------------------------------------------------------
    -- draw_text
    -- -----------------------------------------------------------------------

    it("draw_text returns a table with kind='text'", function()
        local t = Draw.draw_text(5, 15, "Hello")
        assert.equals("text", t.kind)
    end)

    it("draw_text stores x, y, and value", function()
        local t = Draw.draw_text(5, 15, "Hello")
        assert.equals(5,       t.x)
        assert.equals(15,      t.y)
        assert.equals("Hello", t.value)
    end)

    it("draw_text defaults fill to '#000000'", function()
        local t = Draw.draw_text(0, 0, "Hi")
        assert.equals("#000000", t.fill)
    end)

    it("draw_text defaults font_family to 'monospace'", function()
        local t = Draw.draw_text(0, 0, "Hi")
        assert.equals("monospace", t.font_family)
    end)

    it("draw_text defaults font_size to 16", function()
        local t = Draw.draw_text(0, 0, "Hi")
        assert.equals(16, t.font_size)
    end)

    it("draw_text defaults align to 'middle'", function()
        local t = Draw.draw_text(0, 0, "Hi")
        assert.equals("middle", t.align)
    end)

    it("draw_text stores custom typography parameters", function()
        local t = Draw.draw_text(1, 2, "X", "#ff0000", "serif", 24, "start")
        assert.equals("#ff0000", t.fill)
        assert.equals("serif",   t.font_family)
        assert.equals(24,        t.font_size)
        assert.equals("start",   t.align)
    end)

    it("draw_text has an empty metadata table by default", function()
        local t = Draw.draw_text(0, 0, "X")
        assert.is_not_nil(t.metadata)
    end)

    -- -----------------------------------------------------------------------
    -- draw_line
    -- -----------------------------------------------------------------------

    it("draw_line returns a table with kind='line'", function()
        local l = Draw.draw_line(0, 0, 100, 100)
        assert.equals("line", l.kind)
    end)

    it("draw_line stores x1, y1, x2, y2", function()
        local l = Draw.draw_line(1, 2, 3, 4, "#123456")
        assert.equals(1, l.x1)
        assert.equals(2, l.y1)
        assert.equals(3, l.x2)
        assert.equals(4, l.y2)
    end)

    it("draw_line stores the stroke color", function()
        local l = Draw.draw_line(0, 0, 10, 10, "#aabbcc")
        assert.equals("#aabbcc", l.stroke)
    end)

    it("draw_line defaults stroke to '#000000'", function()
        local l = Draw.draw_line(0, 0, 10, 10)
        assert.equals("#000000", l.stroke)
    end)

    it("draw_line has an empty metadata table by default", function()
        local l = Draw.draw_line(0, 0, 10, 10)
        assert.is_not_nil(l.metadata)
    end)

    -- -----------------------------------------------------------------------
    -- draw_circle
    -- -----------------------------------------------------------------------

    it("draw_circle returns a table with kind='circle'", function()
        local c = Draw.draw_circle(50, 50, 25)
        assert.equals("circle", c.kind)
    end)

    it("draw_circle stores cx, cy, r", function()
        local c = Draw.draw_circle(10, 20, 30, "#0000ff")
        assert.equals(10, c.cx)
        assert.equals(20, c.cy)
        assert.equals(30, c.r)
    end)

    it("draw_circle stores the fill color", function()
        local c = Draw.draw_circle(0, 0, 5, "#ff00ff")
        assert.equals("#ff00ff", c.fill)
    end)

    it("draw_circle defaults fill to '#000000'", function()
        local c = Draw.draw_circle(0, 0, 5)
        assert.equals("#000000", c.fill)
    end)

    it("draw_circle has an empty metadata table by default", function()
        local c = Draw.draw_circle(0, 0, 5)
        assert.is_not_nil(c.metadata)
    end)

    -- -----------------------------------------------------------------------
    -- draw_group
    -- -----------------------------------------------------------------------

    it("draw_group returns a table with kind='group'", function()
        local g = Draw.draw_group({})
        assert.equals("group", g.kind)
    end)

    it("draw_group stores the children array", function()
        local r = Draw.draw_rect(0, 0, 10, 10)
        local t = Draw.draw_text(5, 5, "Hi")
        local g = Draw.draw_group({r, t})
        assert.equals(2, #g.children)
        assert.equals("rect", g.children[1].kind)
        assert.equals("text", g.children[2].kind)
    end)

    it("draw_group with no children defaults to empty table", function()
        local g = Draw.draw_group()
        assert.equals(0, #g.children)
    end)

    it("draw_group has an empty metadata table by default", function()
        local g = Draw.draw_group({})
        assert.is_not_nil(g.metadata)
    end)

    it("draw_group accepts metadata", function()
        local g = Draw.draw_group({}, {label="container"})
        assert.equals("container", g.metadata.label)
    end)

    -- -----------------------------------------------------------------------
    -- create_scene
    -- -----------------------------------------------------------------------

    it("create_scene stores width and height", function()
        local s = Draw.create_scene(800, 600, {})
        assert.equals(800, s.width)
        assert.equals(600, s.height)
    end)

    it("create_scene defaults background to '#ffffff'", function()
        local s = Draw.create_scene(800, 600, {})
        assert.equals("#ffffff", s.background)
    end)

    it("create_scene stores custom background color", function()
        local s = Draw.create_scene(800, 600, {}, "#112233")
        assert.equals("#112233", s.background)
    end)

    it("create_scene stores instructions", function()
        local r = Draw.draw_rect(0, 0, 10, 10)
        local s = Draw.create_scene(100, 100, {r})
        assert.equals(1, #s.instructions)
        assert.equals("rect", s.instructions[1].kind)
    end)

    it("create_scene with no instructions defaults to empty table", function()
        local s = Draw.create_scene(100, 100)
        assert.equals(0, #s.instructions)
    end)

    it("create_scene has metadata", function()
        local s = Draw.create_scene(100, 100, {}, "#fff", {title="My Scene"})
        assert.equals("My Scene", s.metadata.title)
    end)

end)

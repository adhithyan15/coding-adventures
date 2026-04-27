-- Tests for coding_adventures.draw_instructions_svg

package.path = "../src/?.lua;../src/?/init.lua;"
  .. "../../draw_instructions/src/?.lua;../../draw_instructions/src/?/init.lua;"
  .. package.path

local Draw = require("coding_adventures.draw_instructions")
local svg = require("coding_adventures.draw_instructions_svg")

describe("draw_instructions_svg", function()
    it("has a VERSION", function()
        assert.is_not_nil(svg.VERSION)
        assert.equals("0.1.0", svg.VERSION)
    end)

    it("renders a complete SVG document", function()
        local scene = Draw.create_scene(120, 80, {
            Draw.draw_rect(10, 20, 30, 40, "#ff0000"),
        }, "#ffffff")
        local result = svg.render_svg(scene)
        assert.is_true(result:match('^<svg xmlns="http://www%.w3%.org/2000/svg"') ~= nil)
        assert.is_true(result:find('width="120"', 1, true) ~= nil)
        assert.is_true(result:find('height="80"', 1, true) ~= nil)
        assert.is_true(result:find('viewBox="0 0 120 80"', 1, true) ~= nil)
        assert.is_true(result:find('<rect x="0" y="0" width="120" height="80" fill="#ffffff" />', 1, true) ~= nil)
        assert.is_true(result:find('<rect x="10" y="20" width="30" height="40" fill="#ff0000" />', 1, true) ~= nil)
    end)

    it("escapes text and metadata", function()
        local text = Draw.draw_text(10, 20, 'A & B <C>')
        text.metadata = { id = 'x"y' }
        local scene = Draw.create_scene(100, 50, { text }, "#fff", { label = 'Chart & "Demo"' })
        local result = svg.render_svg(scene)
        assert.is_true(result:find('aria-label="Chart &amp; &quot;Demo&quot;"', 1, true) ~= nil)
        assert.is_true(result:find('data-id="x&quot;y"', 1, true) ~= nil)
        assert.is_true(result:find('>A &amp; B &lt;C&gt;</text>', 1, true) ~= nil)
    end)

    it("renders groups, lines, and circles", function()
        local group = Draw.draw_group({
            Draw.draw_line(0, 0, 5, 5, "#333333"),
            Draw.draw_circle(10, 10, 3, "#00ff00"),
        })
        local result = svg.render_svg(Draw.create_scene(20, 20, { group }, "#fff"))
        assert.is_true(result:find("<g>", 1, true) ~= nil)
        assert.is_true(result:find('<line x1="0" y1="0" x2="5" y2="5" stroke="#333333" stroke-width="1" />', 1, true) ~= nil)
        assert.is_true(result:find('<circle cx="10" cy="10" r="3" fill="#00ff00" />', 1, true) ~= nil)
    end)

    it("renders manually-authored clip instructions deterministically", function()
        local clip = {
            kind = "clip",
            x = 0,
            y = 0,
            width = 10,
            height = 10,
            children = { Draw.draw_rect(0, 0, 20, 20, "red") },
        }
        local scene = Draw.create_scene(20, 20, { clip }, "#fff")
        local first = svg.render_svg(scene)
        local second = svg.render_svg(scene)
        assert.equals(first, second)
        assert.is_true(first:find('<clipPath id="clip-1">', 1, true) ~= nil)
        assert.is_true(first:find('<g clip-path="url(#clip-1)">', 1, true) ~= nil)
    end)
end)

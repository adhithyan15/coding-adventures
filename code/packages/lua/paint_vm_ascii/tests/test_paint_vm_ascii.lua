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

    describe("rect stroke", function()
        it("renders a box-drawing border for a stroked rect with no fill", function()
            local scene = paint_instructions.paint_scene(4, 4, {
                paint_instructions.paint_rect(0, 0, 3, 3, "", nil, "#000000", 1),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            assert.is_true(result:find("┌", 1, true) ~= nil)
            assert.is_true(result:find("┐", 1, true) ~= nil)
            assert.is_true(result:find("└", 1, true) ~= nil)
            assert.is_true(result:find("┘", 1, true) ~= nil)
        end)

        it("renders fill interior with stroke border when both are set", function()
            local scene = paint_instructions.paint_scene(4, 4, {
                paint_instructions.paint_rect(0, 0, 3, 3, "#000000", nil, "#000000", 1),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            assert.is_true(result:find("█", 1, true) ~= nil)
            assert.is_true(result:find("┌", 1, true) ~= nil)
        end)

        it("rejects negative rect dimensions", function()
            local scene = paint_instructions.paint_scene(4, 4, {
                paint_instructions.paint_rect(0, 0, -1, 3, "#000000"),
            })
            assert.has_error(function()
                paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            end)
        end)
    end)

    describe("line", function()
        it("renders a horizontal line", function()
            local scene = paint_instructions.paint_scene(5, 1, {
                paint_instructions.paint_line(0, 0, 4, 0, "#000000", 1),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            assert.is_true(result:find("─", 1, true) ~= nil)
        end)

        it("renders a vertical line", function()
            local scene = paint_instructions.paint_scene(1, 5, {
                paint_instructions.paint_line(0, 0, 0, 4, "#000000", 1),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            assert.is_true(result:find("│", 1, true) ~= nil)
        end)

        it("renders a shallow diagonal (dx > dy) without hanging", function()
            local scene = paint_instructions.paint_scene(10, 3, {
                paint_instructions.paint_line(0, 0, 9, 2, "#000000", 1),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            assert.is_true(#result > 0)
        end)

        it("renders a steep diagonal (dy > dx) without hanging", function()
            local scene = paint_instructions.paint_scene(3, 10, {
                paint_instructions.paint_line(0, 0, 2, 9, "#000000", 1),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            assert.is_true(#result > 0)
        end)

        it("renders a 45-degree diagonal", function()
            local scene = paint_instructions.paint_scene(5, 5, {
                paint_instructions.paint_line(0, 0, 4, 4, "#000000", 1),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            assert.is_true(#result > 0)
        end)

        it("renders a reversed-direction diagonal", function()
            local scene = paint_instructions.paint_scene(5, 5, {
                paint_instructions.paint_line(4, 4, 0, 0, "#000000", 1),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            assert.is_true(#result > 0)
        end)

        -- Regression test for the Bresenham-seeded-at-zero hang documented
        -- in GitHub issue #12093 and code/specs (dart/swift already fixed
        -- this; this pins it down for the Lua port too). Without the
        -- deltaCol - deltaRow seed, this exact (deltaRow=1, deltaCol=3)
        -- slope never converges and the test would time out instead of
        -- failing cleanly.
        it("terminates for the known deltaRow=1, deltaCol=3 hang-triggering slope", function()
            local scene = paint_instructions.paint_scene(4, 2, {
                paint_instructions.paint_line(0, 0, 3, 1, "#000000", 1),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            assert.is_true(#result > 0)
        end)

        it("rejects non-finite line coordinates", function()
            local scene = paint_instructions.paint_scene(4, 4, {
                paint_instructions.paint_line(0, 0, 0 / 0, 1, "#000000", 1),
            })
            assert.has_error(function()
                paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            end)
        end)
    end)

    describe("glyph_run", function()
        it("places literal glyph characters at their scaled positions", function()
            local scene = paint_instructions.paint_scene(16, 16, {
                paint_instructions.paint_glyph_run({
                    paint_instructions.paint_glyph_placement(string.byte("H"), 0, 0),
                    paint_instructions.paint_glyph_placement(string.byte("i"), 8, 0),
                }, "terminal-mono", 16, "#000000"),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 8, scale_y = 16 })
            assert.are.equal("Hi", result)
        end)

        it("skips space characters implicitly (never emitted by callers)", function()
            local scene = paint_instructions.paint_scene(24, 16, {
                paint_instructions.paint_glyph_run({
                    paint_instructions.paint_glyph_placement(string.byte("A"), 0, 0),
                    paint_instructions.paint_glyph_placement(string.byte("B"), 16, 0),
                }, "terminal-mono", 16, "#000000"),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 8, scale_y = 16 })
            assert.are.equal("A B", result)
        end)

        it("replaces unsafe code points (control chars, surrogates) with '?'", function()
            local scene = paint_instructions.paint_scene(8, 16, {
                paint_instructions.paint_glyph_run({
                    paint_instructions.paint_glyph_placement(0x1B, 0, 0), -- ESC
                }, "terminal-mono", 16, "#000000"),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 8, scale_y = 16 })
            assert.are.equal("?", result)

            local scene2 = paint_instructions.paint_scene(8, 16, {
                paint_instructions.paint_glyph_run({
                    paint_instructions.paint_glyph_placement(0xD800, 0, 0), -- lone surrogate
                }, "terminal-mono", 16, "#000000"),
            })
            assert.are.equal("?", paint_vm_ascii.render(scene2, { scale_x = 8, scale_y = 16 }))
        end)

        it("replaces out-of-range code points with '?' instead of erroring", function()
            local scene = paint_instructions.paint_scene(8, 16, {
                paint_instructions.paint_glyph_run({
                    paint_instructions.paint_glyph_placement(0x110000, 0, 0), -- one past max
                }, "terminal-mono", 16, "#000000"),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 8, scale_y = 16 })
            assert.are.equal("?", result)
        end)

        it("skips glyphs with a non-finite position rather than failing the render", function()
            local scene = paint_instructions.paint_scene(8, 16, {
                paint_instructions.paint_glyph_run({
                    paint_instructions.paint_glyph_placement(string.byte("A"), 0 / 0, 0),
                }, "terminal-mono", 16, "#000000"),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 8, scale_y = 16 })
            assert.are.equal("", result)
        end)

        it("text overwrites box-drawing/fill characters underneath it (text priority)", function()
            local scene = paint_instructions.paint_scene(3, 1, {
                paint_instructions.paint_rect(0, 0, 2, 0, "#000000"),
                paint_instructions.paint_glyph_run({
                    paint_instructions.paint_glyph_placement(string.byte("X"), 0, 0),
                }, "terminal-mono", 1, "#000000"),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            assert.are.equal("X", result:sub(1, 1))
        end)
    end)

    describe("group / clip / layer", function()
        it("renders a plain group's children", function()
            local scene = paint_instructions.paint_scene(2, 2, {
                paint_instructions.paint_group({
                    paint_instructions.paint_rect(0, 0, 1, 1, "#000000"),
                }),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            assert.is_true(result:find("█", 1, true) ~= nil)
        end)

        it("rejects a group with a non-identity transform", function()
            local scene = paint_instructions.paint_scene(2, 2, {
                paint_instructions.paint_group(
                    { paint_instructions.paint_rect(0, 0, 1, 1, "#000000") },
                    { transform = paint_instructions.transform2d(2, 0, 0, 1, 0, 0) }
                ),
            })
            assert.has_error(function()
                paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            end)
        end)

        it("rejects a group with non-default opacity", function()
            local scene = paint_instructions.paint_scene(2, 2, {
                paint_instructions.paint_group(
                    { paint_instructions.paint_rect(0, 0, 1, 1, "#000000") },
                    { opacity = 0.5 }
                ),
            })
            assert.has_error(function()
                paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            end)
        end)

        it("clips children to the intersection of nested clip rectangles", function()
            local scene = paint_instructions.paint_scene(4, 4, {
                paint_instructions.paint_clip(0, 0, 2, 2, {
                    paint_instructions.paint_rect(0, 0, 4, 4, "#000000"),
                }),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            -- Only the top-left 2x2 region should be filled.
            local lines = {}
            for line in (result .. "\n"):gmatch("(.-)\n") do
                lines[#lines + 1] = line
            end
            assert.are.equal("██", lines[1])
            assert.are.equal("██", lines[2])
        end)

        it("renders a plain layer's children", function()
            local scene = paint_instructions.paint_scene(2, 2, {
                paint_instructions.paint_layer({
                    paint_instructions.paint_rect(0, 0, 1, 1, "#000000"),
                }),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            assert.is_true(result:find("█", 1, true) ~= nil)
        end)

        it("rejects a layer with filters", function()
            local scene = paint_instructions.paint_scene(2, 2, {
                paint_instructions.paint_layer(
                    { paint_instructions.paint_rect(0, 0, 1, 1, "#000000") },
                    { has_filters = true }
                ),
            })
            assert.has_error(function()
                paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            end)
        end)

        it("rejects a layer with a non-normal blend mode", function()
            local scene = paint_instructions.paint_scene(2, 2, {
                paint_instructions.paint_layer(
                    { paint_instructions.paint_rect(0, 0, 1, 1, "#000000") },
                    { blend_mode = "multiply" }
                ),
            })
            assert.has_error(function()
                paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            end)
        end)

        it("rejects nesting deeper than 64 levels", function()
            local inner = paint_instructions.paint_rect(0, 0, 1, 1, "#000000")
            for _ = 1, 70 do
                inner = paint_instructions.paint_group({ inner })
            end
            local scene = paint_instructions.paint_scene(1, 1, { inner })
            assert.has_error(function()
                paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            end)
        end)

        it("accepts exactly 64 levels of nesting", function()
            local inner = paint_instructions.paint_rect(0, 0, 1, 1, "#000000")
            for _ = 1, 64 do
                inner = paint_instructions.paint_group({ inner })
            end
            local scene = paint_instructions.paint_scene(1, 1, { inner })
            assert.has_no.errors(function()
                paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            end)
        end)
    end)

    describe("unsupported instructions", function()
        it("rejects path instructions", function()
            local scene = paint_instructions.paint_scene(1, 1, {
                paint_instructions.paint_path({}),
            })
            assert.has_error(function()
                paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            end)
        end)

        it("rejects an unrecognized instruction kind", function()
            local scene = paint_instructions.paint_scene(1, 1, { { kind = "gradient" } })
            assert.has_error(function()
                paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            end)
        end)
    end)

    describe("scene bounds and sizing", function()
        it("renders an empty (zero-instruction) scene as an empty string", function()
            local scene = paint_instructions.paint_scene(10, 10, {})
            assert.are.equal("", paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 }))
        end)

        it("renders a zero-width scene as an empty string", function()
            local scene = paint_instructions.paint_scene(0, 10, {})
            assert.are.equal("", paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 }))
        end)

        it("renders a zero-height scene as an empty string", function()
            local scene = paint_instructions.paint_scene(10, 0, {})
            assert.are.equal("", paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 }))
        end)

        it("rejects negative scene dimensions", function()
            local scene = paint_instructions.paint_scene(-1, 10, {})
            assert.has_error(function()
                paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            end)
        end)

        it("rejects a scene whose per-axis cell count exceeds the cap", function()
            local scene = paint_instructions.paint_scene(1000000, 1, {})
            assert.has_error(function()
                paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            end)
        end)

        it("rejects a zero-width, huge-height scene (defeats a product-only cap check)", function()
            local scene = paint_instructions.paint_scene(0, 1e18, {})
            assert.has_error(function()
                paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            end)
        end)

        it("rejects a non-positive scale_x", function()
            local scene = paint_instructions.paint_scene(1, 1, {})
            assert.has_error(function()
                paint_vm_ascii.render(scene, { scale_x = 0, scale_y = 1 })
            end)
        end)

        it("rejects a non-positive scale_y", function()
            local scene = paint_instructions.paint_scene(1, 1, {})
            assert.has_error(function()
                paint_vm_ascii.render(scene, { scale_x = 1, scale_y = -1 })
            end)
        end)

        -- Regression tests: `scene.width < 0` and `sx <= 0` are both false
        -- for NaN (every comparison against NaN is false), so a NaN input
        -- could previously slip past these checks and reach ceil_div()/the
        -- scene-size cap with a NaN cols/rows value instead of being
        -- rejected loudly.
        it("rejects a NaN scene width", function()
            local nan = 0 / 0
            local scene = paint_instructions.paint_scene(nan, 10, {})
            assert.has_error(function()
                paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            end)
        end)

        it("rejects a NaN scene height", function()
            local nan = 0 / 0
            local scene = paint_instructions.paint_scene(10, nan, {})
            assert.has_error(function()
                paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            end)
        end)

        it("rejects a NaN scale_x", function()
            local nan = 0 / 0
            local scene = paint_instructions.paint_scene(1, 1, {})
            assert.has_error(function()
                paint_vm_ascii.render(scene, { scale_x = nan, scale_y = 1 })
            end)
        end)

        it("rejects a NaN scale_y", function()
            local nan = 0 / 0
            local scene = paint_instructions.paint_scene(1, 1, {})
            assert.has_error(function()
                paint_vm_ascii.render(scene, { scale_x = 1, scale_y = nan })
            end)
        end)

        it("defaults to scale_x=8, scale_y=16 when options is omitted", function()
            local scene = paint_instructions.paint_scene(8, 16, {
                paint_instructions.paint_glyph_run({
                    paint_instructions.paint_glyph_placement(string.byte("Z"), 0, 0),
                }, "f", 1, "#000000"),
            })
            assert.are.equal("Z", paint_vm_ascii.render(scene))
        end)
    end)

    describe("trimming", function()
        it("trims trailing spaces on each line", function()
            local scene = paint_instructions.paint_scene(4, 1, {
                paint_instructions.paint_glyph_run({
                    paint_instructions.paint_glyph_placement(string.byte("A"), 0, 0),
                }, "f", 1, "#000000"),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            assert.are.equal("A", result)
        end)

        it("trims trailing blank lines at the end of the document", function()
            local scene = paint_instructions.paint_scene(1, 4, {
                paint_instructions.paint_glyph_run({
                    paint_instructions.paint_glyph_placement(string.byte("A"), 0, 0),
                }, "f", 1, "#000000"),
            })
            local result = paint_vm_ascii.render(scene, { scale_x = 1, scale_y = 1 })
            assert.are.equal("A", result)
        end)
    end)
end)

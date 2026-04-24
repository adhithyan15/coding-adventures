-- Tests for coding-adventures-barcode-2d
--
-- Run from the tests/ directory with:
--   busted . --verbose --pattern=test_
--
-- The package.path lines below let the test runner find the source without
-- needing the package installed via luarocks (useful for local dev).

package.path = (
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    "../../paint_instructions/src/?.lua;" ..
    "../../paint_instructions/src/?/init.lua;" ..
    package.path
)

local b2d = require("coding_adventures.barcode_2d")

-- ============================================================================
-- VERSION
-- ============================================================================

describe("barcode_2d.VERSION", function()
    it("is a string", function()
        assert.is_string(b2d.VERSION)
    end)

    it("is 0.1.0", function()
        assert.equal("0.1.0", b2d.VERSION)
    end)
end)

-- ============================================================================
-- make_module_grid
-- ============================================================================

describe("barcode_2d.make_module_grid", function()
    it("creates a grid with the correct dimensions", function()
        local g = b2d.make_module_grid(5, 7)
        assert.equal(5, g.rows)
        assert.equal(7, g.cols)
    end)

    it("initialises every module to false (light)", function()
        local g = b2d.make_module_grid(3, 4)
        for r = 1, 3 do
            for c = 1, 4 do
                assert.is_false(g.modules[r][c])
            end
        end
    end)

    it("defaults to square module shape", function()
        local g = b2d.make_module_grid(2, 2)
        assert.equal("square", g.module_shape)
    end)

    it("accepts hex module shape", function()
        local g = b2d.make_module_grid(33, 30, "hex")
        assert.equal("hex", g.module_shape)
        assert.equal(33, g.rows)
        assert.equal(30, g.cols)
    end)

    it("creates independent row tables", function()
        -- Mutating one row should not affect others (sanity check that rows
        -- are separate tables, not aliases of the same table).
        local g = b2d.make_module_grid(3, 3)
        g.modules[1][1] = true
        assert.is_false(g.modules[2][1])
        assert.is_false(g.modules[3][1])
    end)
end)

-- ============================================================================
-- set_module
-- ============================================================================

describe("barcode_2d.set_module", function()
    it("returns a new grid with the specified module set to true", function()
        local g  = b2d.make_module_grid(3, 3)
        local g2 = b2d.set_module(g, 2, 2, true)
        assert.is_true(g2.modules[2][2])
    end)

    it("does not mutate the original grid", function()
        local g  = b2d.make_module_grid(3, 3)
        local _  = b2d.set_module(g, 2, 2, true)
        assert.is_false(g.modules[2][2])
    end)

    it("returns a different table than the input", function()
        local g  = b2d.make_module_grid(3, 3)
        local g2 = b2d.set_module(g, 1, 1, true)
        assert.are_not.equal(g, g2)
    end)

    it("preserves unmodified rows by reference", function()
        -- Row 1 should be a different table in g2 (it was replaced), but
        -- rows 2 and 3 should be the same table objects (shared).
        local g  = b2d.make_module_grid(3, 3)
        local g2 = b2d.set_module(g, 1, 1, true)
        -- The replaced row is a new table.
        assert.are_not.equal(g.modules[1], g2.modules[1])
        -- The untouched rows are the same table (shared reference).
        assert.equal(g.modules[2], g2.modules[2])
        assert.equal(g.modules[3], g2.modules[3])
    end)

    it("can set a module to false", function()
        local g  = b2d.make_module_grid(3, 3)
        local g2 = b2d.set_module(g, 1, 1, true)
        local g3 = b2d.set_module(g2, 1, 1, false)
        assert.is_false(g3.modules[1][1])
    end)

    it("preserves all grid metadata", function()
        local g  = b2d.make_module_grid(5, 6, "hex")
        local g2 = b2d.set_module(g, 3, 4, true)
        assert.equal(5,     g2.rows)
        assert.equal(6,     g2.cols)
        assert.equal("hex", g2.module_shape)
    end)

    it("raises an error for row out of range (too small)", function()
        local g = b2d.make_module_grid(3, 3)
        assert.has_error(function()
            b2d.set_module(g, 0, 1, true)
        end)
    end)

    it("raises an error for row out of range (too large)", function()
        local g = b2d.make_module_grid(3, 3)
        assert.has_error(function()
            b2d.set_module(g, 4, 1, true)
        end)
    end)

    it("raises an error for col out of range (too small)", function()
        local g = b2d.make_module_grid(3, 3)
        assert.has_error(function()
            b2d.set_module(g, 1, 0, true)
        end)
    end)

    it("raises an error for col out of range (too large)", function()
        local g = b2d.make_module_grid(3, 3)
        assert.has_error(function()
            b2d.set_module(g, 1, 4, true)
        end)
    end)

    it("allows setting the corner modules (boundary check)", function()
        local g = b2d.make_module_grid(5, 5)
        -- Four corners, all valid.
        local g2 = b2d.set_module(g, 1, 1, true)
        local g3 = b2d.set_module(g2, 1, 5, true)
        local g4 = b2d.set_module(g3, 5, 1, true)
        local g5 = b2d.set_module(g4, 5, 5, true)
        assert.is_true(g5.modules[1][1])
        assert.is_true(g5.modules[1][5])
        assert.is_true(g5.modules[5][1])
        assert.is_true(g5.modules[5][5])
    end)
end)

-- ============================================================================
-- layout -- square modules
-- ============================================================================

describe("barcode_2d.layout (square)", function()
    it("returns a PaintScene with correct dimensions for an all-light grid", function()
        -- 5x5 grid, module_size_px=10, quiet_zone=4
        -- total = (5 + 2*4) * 10 = 130
        local g     = b2d.make_module_grid(5, 5)
        local scene = b2d.layout(g)
        assert.equal(130, scene.width)
        assert.equal(130, scene.height)
    end)

    it("emits exactly one background instruction for an all-light grid", function()
        -- An all-light grid has no dark modules, so only the background rect.
        local g     = b2d.make_module_grid(3, 3)
        local scene = b2d.layout(g)
        assert.equal(1, #scene.instructions)
        assert.equal("rect", scene.instructions[1].kind)
    end)

    it("emits background + one rect per dark module", function()
        -- 2x2 grid with two dark modules -> 1 background + 2 module rects
        local g = b2d.make_module_grid(2, 2)
        g = b2d.set_module(g, 1, 1, true)
        g = b2d.set_module(g, 2, 2, true)
        local scene = b2d.layout(g, { quiet_zone_modules = 0 })
        assert.equal(3, #scene.instructions)
    end)

    it("places the first dark module at the correct pixel origin", function()
        -- module at (1,1), module_size=10, quiet_zone=2
        -- quiet_px = 2*10 = 20
        -- x = 20 + (1-1)*10 = 20
        -- y = 20 + (1-1)*10 = 20
        local g = b2d.make_module_grid(3, 3)
        g = b2d.set_module(g, 1, 1, true)
        local scene = b2d.layout(g, { module_size_px = 10, quiet_zone_modules = 2 })
        -- instructions[1] is the background, instructions[2] is the first dark module
        local rect = scene.instructions[2]
        assert.equal("rect", rect.kind)
        assert.equal(20, rect.x)
        assert.equal(20, rect.y)
        assert.equal(10, rect.width)
        assert.equal(10, rect.height)
    end)

    it("places module at (row=2, col=3) at the correct pixel origin", function()
        -- row=2, col=3, module_size=5, quiet_zone=1
        -- quiet_px = 1*5 = 5
        -- x = 5 + (3-1)*5 = 5 + 10 = 15
        -- y = 5 + (2-1)*5 = 5 + 5  = 10
        local g = b2d.make_module_grid(4, 4)
        g = b2d.set_module(g, 2, 3, true)
        local scene = b2d.layout(g, { module_size_px = 5, quiet_zone_modules = 1 })
        local rect = scene.instructions[2]
        assert.equal(15, rect.x)
        assert.equal(10, rect.y)
    end)

    it("uses the foreground colour for dark module rects", function()
        local g = b2d.make_module_grid(2, 2)
        g = b2d.set_module(g, 1, 1, true)
        local scene = b2d.layout(g, { foreground = "#ff0000", quiet_zone_modules = 0 })
        -- instructions[1] is background, instructions[2] is the dark module
        assert.equal("#ff0000", scene.instructions[2].fill)
    end)

    it("uses the background colour for the background rect", function()
        local g     = b2d.make_module_grid(2, 2)
        local scene = b2d.layout(g, { background = "#eeeeee" })
        assert.equal("#eeeeee", scene.instructions[1].fill)
    end)

    it("uses zero quiet zone when quiet_zone_modules=0", function()
        -- 4x4 grid, no quiet zone, module_size=8 -> 32 x 32
        local g     = b2d.make_module_grid(4, 4)
        local scene = b2d.layout(g, { quiet_zone_modules = 0, module_size_px = 8 })
        assert.equal(32, scene.width)
        assert.equal(32, scene.height)
    end)

    it("raises an error when module_size_px is zero", function()
        local g = b2d.make_module_grid(3, 3)
        assert.has_error(function()
            b2d.layout(g, { module_size_px = 0 })
        end)
    end)

    it("raises an error when module_size_px is negative", function()
        local g = b2d.make_module_grid(3, 3)
        assert.has_error(function()
            b2d.layout(g, { module_size_px = -5 })
        end)
    end)

    it("raises an error when quiet_zone_modules is negative", function()
        local g = b2d.make_module_grid(3, 3)
        assert.has_error(function()
            b2d.layout(g, { quiet_zone_modules = -1 })
        end)
    end)

    it("raises an error when config.module_shape does not match grid.module_shape", function()
        -- Grid is "square" but config says "hex".
        local g = b2d.make_module_grid(3, 3, "square")
        assert.has_error(function()
            b2d.layout(g, { module_shape = "hex" })
        end)
    end)

    it("produces a 21x21 QR v1 sized scene with default config", function()
        -- QR Code v1: 21x21, quiet_zone=4, module_size=10
        -- total = (21 + 8) * 10 = 290
        local g     = b2d.make_module_grid(21, 21)
        local scene = b2d.layout(g)
        assert.equal(290, scene.width)
        assert.equal(290, scene.height)
    end)
end)

-- ============================================================================
-- layout -- hex modules
-- ============================================================================

describe("barcode_2d.layout (hex)", function()
    it("raises an error when grid is square but config says hex", function()
        local g = b2d.make_module_grid(3, 3, "square")
        assert.has_error(function()
            b2d.layout(g, { module_shape = "hex" })
        end)
    end)

    it("raises an error when grid is hex but config is default (square)", function()
        local g = b2d.make_module_grid(5, 5, "hex")
        assert.has_error(function()
            b2d.layout(g)
        end)
    end)

    it("returns a PaintScene for a hex grid with default config", function()
        local g     = b2d.make_module_grid(5, 5, "hex")
        local scene = b2d.layout(g, { module_shape = "hex" })
        assert.is_not_nil(scene)
        assert.is_number(scene.width)
        assert.is_number(scene.height)
    end)

    it("emits one background instruction for an all-light hex grid", function()
        local g     = b2d.make_module_grid(3, 3, "hex")
        local scene = b2d.layout(g, { module_shape = "hex" })
        assert.equal(1, #scene.instructions)
        assert.equal("rect", scene.instructions[1].kind)
    end)

    it("emits background + one path per dark module", function()
        -- 2x2 hex grid with one dark module -> 1 background + 1 path
        local g = b2d.make_module_grid(2, 2, "hex")
        g = b2d.set_module(g, 1, 1, true)
        local scene = b2d.layout(g, { module_shape = "hex" })
        assert.equal(2, #scene.instructions)
        assert.equal("path", scene.instructions[2].kind)
    end)

    it("each hex path has exactly 7 commands (move_to + 5 line_to + close)", function()
        local g = b2d.make_module_grid(2, 2, "hex")
        g = b2d.set_module(g, 1, 2, true)
        local scene = b2d.layout(g, { module_shape = "hex" })
        local path  = scene.instructions[2]
        assert.equal(7, #path.commands)
        assert.equal("move_to", path.commands[1].kind)
        assert.equal("line_to", path.commands[2].kind)
        assert.equal("line_to", path.commands[3].kind)
        assert.equal("line_to", path.commands[4].kind)
        assert.equal("line_to", path.commands[5].kind)
        assert.equal("line_to", path.commands[6].kind)
        assert.equal("close",   path.commands[7].kind)
    end)

    it("uses the foreground colour for hex paths", function()
        local g = b2d.make_module_grid(2, 2, "hex")
        g = b2d.set_module(g, 1, 1, true)
        local scene = b2d.layout(g, { module_shape = "hex", foreground = "#0000ff" })
        assert.equal("#0000ff", scene.instructions[2].fill)
    end)

    it("MaxiCode sized grid: 33 rows x 30 cols", function()
        -- Just verify it runs without error and produces a scene.
        local g     = b2d.make_module_grid(33, 30, "hex")
        local scene = b2d.layout(g, { module_shape = "hex", quiet_zone_modules = 1 })
        assert.is_number(scene.width)
        assert.is_number(scene.height)
        assert.is_true(scene.width > 0)
        assert.is_true(scene.height > 0)
    end)
end)

-- ============================================================================
-- _build_flat_top_hex_path (internal, but exposed for testing)
-- ============================================================================

describe("barcode_2d._build_flat_top_hex_path", function()
    local deg_to_rad = math.pi / 180

    it("returns exactly 7 commands", function()
        local cmds = b2d._build_flat_top_hex_path(0, 0, 5, deg_to_rad)
        assert.equal(7, #cmds)
    end)

    it("first command is move_to", function()
        local cmds = b2d._build_flat_top_hex_path(0, 0, 5, deg_to_rad)
        assert.equal("move_to", cmds[1].kind)
    end)

    it("commands 2-6 are line_to", function()
        local cmds = b2d._build_flat_top_hex_path(0, 0, 5, deg_to_rad)
        for i = 2, 6 do
            assert.equal("line_to", cmds[i].kind)
        end
    end)

    it("last command is close", function()
        local cmds = b2d._build_flat_top_hex_path(0, 0, 5, deg_to_rad)
        assert.equal("close", cmds[7].kind)
    end)

    it("vertex 0 is at (cx + R, cy) for 0-degree start angle", function()
        -- cos(0) = 1, sin(0) = 0
        local R    = 6.0
        local cmds = b2d._build_flat_top_hex_path(10, 20, R, deg_to_rad)
        -- Allow tiny floating-point tolerance.
        assert.is_true(math.abs(cmds[1].x - (10 + R)) < 1e-10)
        assert.is_true(math.abs(cmds[1].y - 20)        < 1e-10)
    end)
end)

-- ============================================================================
-- DEFAULT_CONFIG
-- ============================================================================

describe("barcode_2d.DEFAULT_CONFIG", function()
    it("has module_size_px = 10", function()
        assert.equal(10.0, b2d.DEFAULT_CONFIG.module_size_px)
    end)

    it("has quiet_zone_modules = 4", function()
        assert.equal(4, b2d.DEFAULT_CONFIG.quiet_zone_modules)
    end)

    it("has foreground = #000000", function()
        assert.equal("#000000", b2d.DEFAULT_CONFIG.foreground)
    end)

    it("has background = #ffffff", function()
        assert.equal("#ffffff", b2d.DEFAULT_CONFIG.background)
    end)

    it("has show_annotations = false", function()
        assert.is_false(b2d.DEFAULT_CONFIG.show_annotations)
    end)

    it("has module_shape = square", function()
        assert.equal("square", b2d.DEFAULT_CONFIG.module_shape)
    end)
end)

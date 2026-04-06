-- Tests for coding_adventures.pixel_container (IC00)
--
-- Covers:
--   1. new            — dimensions stored, data length, initial values
--   2. pixel_at       — in-bounds reads, out-of-bounds returns 0,0,0,0
--   3. set_pixel      — write and read back, out-of-bounds no-op
--   4. fill_pixels    — all pixels overwritten
--   5. clone          — deep copy, no aliasing
--   6. equals         — pixel-exact comparison, dimension mismatch
--   7. error handling — invalid constructor arguments
--   8. edge cases     — 1×1 images, corner pixels, boundary pixels

package.path = package.path .. ";../src/?.lua;../src/?/init.lua"

local pc = require("coding_adventures.pixel_container")

-- ============================================================================
-- new
-- ============================================================================
describe("new", function()

    it("stores width and height", function()
        local c = pc.new(10, 20)
        assert.are.equal(10, c.width)
        assert.are.equal(20, c.height)
    end)

    it("data table has width * height * 4 entries", function()
        local c = pc.new(5, 3)
        assert.are.equal(5 * 3 * 4, #c.data)
    end)

    it("all bytes are initialised to 0", function()
        local c = pc.new(4, 4)
        for i = 1, #c.data do
            assert.are.equal(0, c.data[i],
                "data[" .. i .. "] should be 0 after new()")
        end
    end)

    it("1×1 container has exactly 4 bytes", function()
        local c = pc.new(1, 1)
        assert.are.equal(4, #c.data)
    end)

    it("raises error for width = 0", function()
        assert.has_error(function() pc.new(0, 1) end)
    end)

    it("raises error for height = 0", function()
        assert.has_error(function() pc.new(1, 0) end)
    end)

    it("raises error for negative width", function()
        assert.has_error(function() pc.new(-1, 1) end)
    end)

    it("raises error for non-integer width", function()
        assert.has_error(function() pc.new(2.5, 1) end)
    end)

    it("exports VERSION string", function()
        assert.is_string(pc.VERSION)
    end)

end)

-- ============================================================================
-- pixel_at
-- ============================================================================
describe("pixel_at", function()

    it("freshly created container returns 0,0,0,0 for any in-bounds pixel", function()
        local c = pc.new(8, 8)
        local r, g, b, a = pc.pixel_at(c, 3, 5)
        assert.are.equal(0, r)
        assert.are.equal(0, g)
        assert.are.equal(0, b)
        assert.are.equal(0, a)
    end)

    it("returns 0,0,0,0 for x out of bounds (right)", function()
        local c = pc.new(4, 4)
        local r, g, b, a = pc.pixel_at(c, 4, 0)
        assert.are.equal(0, r)
        assert.are.equal(0, g)
        assert.are.equal(0, b)
        assert.are.equal(0, a)
    end)

    it("returns 0,0,0,0 for y out of bounds (bottom)", function()
        local c = pc.new(4, 4)
        local r, g, b, a = pc.pixel_at(c, 0, 4)
        assert.are.equal(0, r)
        assert.are.equal(0, g)
        assert.are.equal(0, b)
        assert.are.equal(0, a)
    end)

    it("returns 0,0,0,0 for negative x", function()
        local c = pc.new(4, 4)
        local r, g, b, a = pc.pixel_at(c, -1, 0)
        assert.are.equal(0, r)
        assert.are.equal(0, g)
        assert.are.equal(0, b)
        assert.are.equal(0, a)
    end)

    it("returns 0,0,0,0 for negative y", function()
        local c = pc.new(4, 4)
        local r, g, b, a = pc.pixel_at(c, 0, -1)
        assert.are.equal(0, r)
        assert.are.equal(0, g)
        assert.are.equal(0, b)
        assert.are.equal(0, a)
    end)

end)

-- ============================================================================
-- set_pixel / pixel_at round-trip
-- ============================================================================
describe("set_pixel", function()

    it("set and read back a pixel at (0,0)", function()
        local c = pc.new(10, 10)
        pc.set_pixel(c, 0, 0, 255, 128, 64, 200)
        local r, g, b, a = pc.pixel_at(c, 0, 0)
        assert.are.equal(255, r)
        assert.are.equal(128, g)
        assert.are.equal(64,  b)
        assert.are.equal(200, a)
    end)

    it("set and read back a pixel at the last position (width-1, height-1)", function()
        local c = pc.new(5, 7)
        pc.set_pixel(c, 4, 6, 10, 20, 30, 40)
        local r, g, b, a = pc.pixel_at(c, 4, 6)
        assert.are.equal(10, r)
        assert.are.equal(20, g)
        assert.are.equal(30, b)
        assert.are.equal(40, a)
    end)

    it("setting one pixel does not disturb neighbours", function()
        local c = pc.new(3, 3)
        pc.set_pixel(c, 1, 1, 100, 101, 102, 103)
        -- Adjacent pixels should still be 0
        local r0, g0, b0, a0 = pc.pixel_at(c, 0, 1)
        assert.are.equal(0, r0)
        local r2, g2, b2, a2 = pc.pixel_at(c, 2, 1)
        assert.are.equal(0, r2)
        -- Centre pixel should have the set value
        local r, g, b, a = pc.pixel_at(c, 1, 1)
        assert.are.equal(100, r)
        assert.are.equal(103, a)
    end)

    it("set_pixel is a no-op for x = width (out of bounds)", function()
        local c = pc.new(4, 4)
        -- Should not error and should not corrupt any data
        pc.set_pixel(c, 4, 0, 255, 255, 255, 255)
        local r, g, b, a = pc.pixel_at(c, 3, 0)
        assert.are.equal(0, r)
    end)

    it("set_pixel is a no-op for negative coordinates", function()
        local c = pc.new(4, 4)
        pc.set_pixel(c, -1, -1, 255, 255, 255, 255)
        local r, _, _, _ = pc.pixel_at(c, 0, 0)
        assert.are.equal(0, r)
    end)

    it("alpha channel stores 0 (transparent) and 255 (opaque) correctly", function()
        local c = pc.new(2, 1)
        pc.set_pixel(c, 0, 0, 0, 0, 0, 0)
        pc.set_pixel(c, 1, 0, 0, 0, 0, 255)
        local _, _, _, a0 = pc.pixel_at(c, 0, 0)
        local _, _, _, a1 = pc.pixel_at(c, 1, 0)
        assert.are.equal(0,   a0)
        assert.are.equal(255, a1)
    end)

    it("overwriting a pixel replaces the previous value", function()
        local c = pc.new(3, 3)
        pc.set_pixel(c, 2, 2, 10, 20, 30, 40)
        pc.set_pixel(c, 2, 2, 50, 60, 70, 80)
        local r, g, b, a = pc.pixel_at(c, 2, 2)
        assert.are.equal(50, r)
        assert.are.equal(60, g)
        assert.are.equal(70, b)
        assert.are.equal(80, a)
    end)

end)

-- ============================================================================
-- fill_pixels
-- ============================================================================
describe("fill_pixels", function()

    it("sets every pixel to the given RGBA value", function()
        local c = pc.new(4, 4)
        pc.fill_pixels(c, 255, 0, 128, 255)
        for y = 0, 3 do
            for x = 0, 3 do
                local r, g, b, a = pc.pixel_at(c, x, y)
                assert.are.equal(255, r)
                assert.are.equal(0,   g)
                assert.are.equal(128, b)
                assert.are.equal(255, a)
            end
        end
    end)

    it("fill with all-zero clears the image", function()
        local c = pc.new(3, 3)
        pc.set_pixel(c, 1, 1, 200, 200, 200, 200)
        pc.fill_pixels(c, 0, 0, 0, 0)
        for i = 1, #c.data do
            assert.are.equal(0, c.data[i])
        end
    end)

    it("fill works on a 1×1 container", function()
        local c = pc.new(1, 1)
        pc.fill_pixels(c, 77, 88, 99, 111)
        local r, g, b, a = pc.pixel_at(c, 0, 0)
        assert.are.equal(77,  r)
        assert.are.equal(88,  g)
        assert.are.equal(99,  b)
        assert.are.equal(111, a)
    end)

end)

-- ============================================================================
-- clone
-- ============================================================================
describe("clone", function()

    it("clone has the same dimensions as the original", function()
        local c = pc.new(6, 8)
        local d = pc.clone(c)
        assert.are.equal(c.width,  d.width)
        assert.are.equal(c.height, d.height)
    end)

    it("clone contains the same pixel values", function()
        local c = pc.new(3, 3)
        pc.set_pixel(c, 1, 1, 12, 34, 56, 78)
        local d = pc.clone(c)
        local r, g, b, a = pc.pixel_at(d, 1, 1)
        assert.are.equal(12, r)
        assert.are.equal(34, g)
        assert.are.equal(56, b)
        assert.are.equal(78, a)
    end)

    it("modifying the clone does not affect the original", function()
        local c = pc.new(3, 3)
        pc.set_pixel(c, 0, 0, 1, 2, 3, 4)
        local d = pc.clone(c)
        pc.set_pixel(d, 0, 0, 99, 99, 99, 99)
        local r, _, _, _ = pc.pixel_at(c, 0, 0)
        assert.are.equal(1, r)  -- original unchanged
    end)

end)

-- ============================================================================
-- equals
-- ============================================================================
describe("equals", function()

    it("two fresh containers with the same dimensions are equal", function()
        local a = pc.new(4, 4)
        local b = pc.new(4, 4)
        assert.is_true(pc.equals(a, b))
    end)

    it("a container equals its clone", function()
        local c = pc.new(5, 5)
        pc.fill_pixels(c, 100, 101, 102, 103)
        assert.is_true(pc.equals(c, pc.clone(c)))
    end)

    it("containers with different widths are not equal", function()
        local a = pc.new(4, 4)
        local b = pc.new(5, 4)
        assert.is_false(pc.equals(a, b))
    end)

    it("containers with different heights are not equal", function()
        local a = pc.new(4, 4)
        local b = pc.new(4, 5)
        assert.is_false(pc.equals(a, b))
    end)

    it("containers with one differing pixel are not equal", function()
        local a = pc.new(4, 4)
        local b = pc.clone(a)
        pc.set_pixel(b, 2, 2, 1, 0, 0, 0)
        assert.is_false(pc.equals(a, b))
    end)

end)

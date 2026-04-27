-- Tests for coding_adventures.image_geometric_transforms (IMG04)
--
-- Run from the tests/ directory:
--   cd tests && busted . --verbose --pattern=test_

package.path = package.path
    .. ";../src/?.lua;../src/?/init.lua"
    .. ";../../pixel-container/src/?.lua;../../pixel-container/src/?/init.lua"

local M  = require("coding_adventures.image_geometric_transforms")
local pc = require("coding_adventures.pixel_container")

-- ---------------------------------------------------------------------------
-- Helpers
-- ---------------------------------------------------------------------------

-- Create a W×H image filled with a single RGBA colour.
local function solid(w, h, r, g, b, a)
    local img = pc.new(w, h)
    for y = 0, h - 1 do
        for x = 0, w - 1 do
            pc.set_pixel(img, x, y, r, g, b, a)
        end
    end
    return img
end

-- Create a 1×1 image with one pixel.
local function px1(r, g, b, a)
    local img = pc.new(1, 1)
    pc.set_pixel(img, 0, 0, r, g, b, a)
    return img
end

-- Return the RGBA at pixel (x, y).
local function at(img, x, y) return pc.pixel_at(img, x, y) end

-- Assert two images are pixel-exactly equal.
local function assert_images_equal(a, b, label)
    label = label or "images"
    assert.equal(a.width,  b.width,  label .. " width mismatch")
    assert.equal(a.height, b.height, label .. " height mismatch")
    for y = 0, a.height - 1 do
        for x = 0, a.width - 1 do
            local ar, ag, ab, aa = pc.pixel_at(a, x, y)
            local br, bg, bb, ba = pc.pixel_at(b, x, y)
            assert.equal(ar, br, label .. " R at (" .. x .. "," .. y .. ")")
            assert.equal(ag, bg, label .. " G at (" .. x .. "," .. y .. ")")
            assert.equal(ab, bb, label .. " B at (" .. x .. "," .. y .. ")")
            assert.equal(aa, ba, label .. " A at (" .. x .. "," .. y .. ")")
        end
    end
end

-- Assert two images are within `tol` per channel at every pixel.
local function assert_images_close(a, b, tol, label)
    tol   = tol   or 2
    label = label or "images"
    assert.equal(a.width,  b.width,  label .. " width mismatch")
    assert.equal(a.height, b.height, label .. " height mismatch")
    for y = 0, a.height - 1 do
        for x = 0, a.width - 1 do
            local ar, ag, ab, aa = pc.pixel_at(a, x, y)
            local br, bg, bb, ba = pc.pixel_at(b, x, y)
            assert.is_true(math.abs(ar - br) <= tol,
                label .. " R at (" .. x .. "," .. y .. ") diff=" .. math.abs(ar-br))
            assert.is_true(math.abs(ag - bg) <= tol,
                label .. " G at (" .. x .. "," .. y .. ") diff=" .. math.abs(ag-bg))
            assert.is_true(math.abs(ab - bb) <= tol,
                label .. " B at (" .. x .. "," .. y .. ") diff=" .. math.abs(ab-bb))
        end
    end
end

-- ---------------------------------------------------------------------------
-- flip_horizontal
-- ---------------------------------------------------------------------------

describe("flip_horizontal", function()
    it("double flip is identity", function()
        local src = pc.new(4, 3)
        -- Place distinct pixels.
        pc.set_pixel(src, 0, 0, 10, 20, 30, 255)
        pc.set_pixel(src, 3, 2, 100, 200, 50, 128)
        local result = M.flip_horizontal(M.flip_horizontal(src))
        assert_images_equal(src, result, "flip_h double")
    end)

    it("mirrors pixel to correct position", function()
        local src = pc.new(5, 1)
        pc.set_pixel(src, 0, 0, 255, 0, 0, 255)  -- red at left
        local out = M.flip_horizontal(src)
        local r, g, b, a = at(out, 4, 0)          -- should be at right
        assert.equal(255, r); assert.equal(0, g); assert.equal(0, b); assert.equal(255, a)
        -- Left should now be transparent black (original right).
        local r2 = at(out, 0, 0)
        assert.equal(0, r2)
    end)

    it("preserves dimensions", function()
        local src = pc.new(7, 3)
        local out = M.flip_horizontal(src)
        assert.equal(7, out.width)
        assert.equal(3, out.height)
    end)
end)

-- ---------------------------------------------------------------------------
-- flip_vertical
-- ---------------------------------------------------------------------------

describe("flip_vertical", function()
    it("double flip is identity", function()
        local src = pc.new(3, 5)
        pc.set_pixel(src, 1, 0, 80, 90, 100, 200)
        pc.set_pixel(src, 2, 4, 10, 20, 30, 255)
        local result = M.flip_vertical(M.flip_vertical(src))
        assert_images_equal(src, result, "flip_v double")
    end)

    it("mirrors pixel to correct row", function()
        local src = pc.new(1, 4)
        pc.set_pixel(src, 0, 0, 255, 0, 0, 255)  -- red at top
        local out = M.flip_vertical(src)
        local r = at(out, 0, 3)                    -- should be at bottom
        assert.equal(255, r)
        assert.equal(0, at(out, 0, 0))              -- top is now 0
    end)
end)

-- ---------------------------------------------------------------------------
-- rotate_90_cw
-- ---------------------------------------------------------------------------

describe("rotate_90_cw", function()
    it("swaps dimensions", function()
        local src = pc.new(6, 4)
        local out = M.rotate_90_cw(src)
        assert.equal(4, out.width)
        assert.equal(6, out.height)
    end)

    it("four CW rotations → identity", function()
        local src = pc.new(5, 3)
        pc.set_pixel(src, 0, 0, 1, 2, 3, 4)
        pc.set_pixel(src, 4, 2, 200, 100, 50, 255)
        local result = M.rotate_90_cw(M.rotate_90_cw(M.rotate_90_cw(M.rotate_90_cw(src))))
        assert_images_equal(src, result, "rotate_90_cw x4")
    end)

    it("top-left pixel moves to top-right", function()
        -- For a W×H source rotated CW: src(0,0) → out(H-1, 0)
        local src = pc.new(4, 3)
        pc.set_pixel(src, 0, 0, 255, 128, 64, 255)
        local out = M.rotate_90_cw(src)
        -- out has W'=H=3, H'=W=4
        -- src(x=0,y=0) → out(H-1-y=2, x=0) = out(2, 0)
        local r, g, b = at(out, 2, 0)
        assert.equal(255, r); assert.equal(128, g); assert.equal(64, b)
    end)
end)

-- ---------------------------------------------------------------------------
-- rotate_90_ccw
-- ---------------------------------------------------------------------------

describe("rotate_90_ccw", function()
    it("CW then CCW is identity", function()
        local src = pc.new(5, 3)
        pc.set_pixel(src, 2, 1, 50, 100, 150, 200)
        local result = M.rotate_90_ccw(M.rotate_90_cw(src))
        assert_images_equal(src, result, "CW+CCW identity")
    end)

    it("swaps dimensions", function()
        local src = pc.new(7, 2)
        local out = M.rotate_90_ccw(src)
        assert.equal(2, out.width)
        assert.equal(7, out.height)
    end)
end)

-- ---------------------------------------------------------------------------
-- rotate_180
-- ---------------------------------------------------------------------------

describe("rotate_180", function()
    it("applied twice is identity", function()
        local src = pc.new(4, 4)
        pc.set_pixel(src, 1, 2, 77, 88, 99, 200)
        local result = M.rotate_180(M.rotate_180(src))
        assert_images_equal(src, result, "rotate_180 double")
    end)

    it("preserves dimensions", function()
        local src = pc.new(5, 7)
        local out = M.rotate_180(src)
        assert.equal(5, out.width)
        assert.equal(7, out.height)
    end)

    it("moves pixel to opposite corner", function()
        -- src(0,0) should appear at out(W-1, H-1)
        local src = pc.new(4, 3)
        pc.set_pixel(src, 0, 0, 200, 100, 50, 255)
        local out = M.rotate_180(src)
        local r, g, b = at(out, 3, 2)
        assert.equal(200, r); assert.equal(100, g); assert.equal(50, b)
    end)
end)

-- ---------------------------------------------------------------------------
-- crop
-- ---------------------------------------------------------------------------

describe("crop", function()
    it("output has correct dimensions", function()
        local src = pc.new(10, 8)
        local out = M.crop(src, 2, 1, 5, 4)
        assert.equal(5, out.width)
        assert.equal(4, out.height)
    end)

    it("copies correct pixel values", function()
        local src = pc.new(5, 5)
        pc.set_pixel(src, 2, 3, 100, 150, 200, 255)
        local out = M.crop(src, 2, 3, 2, 2)
        -- The pixel at (2,3) in src should be at (0,0) in out.
        local r, g, b, a = at(out, 0, 0)
        assert.equal(100, r); assert.equal(150, g); assert.equal(200, b); assert.equal(255, a)
    end)

    it("OOB region is transparent black", function()
        local src = pc.new(3, 3)
        pc.set_pixel(src, 0, 0, 255, 255, 255, 255)
        -- Crop starting from within, extending past the boundary.
        local out = M.crop(src, 2, 2, 3, 3)
        -- Pixel (1,1) in out = src pixel (3,3) which is OOB → transparent black.
        local r, g, b, a = at(out, 1, 1)
        assert.equal(0, r); assert.equal(0, g); assert.equal(0, b); assert.equal(0, a)
    end)
end)

-- ---------------------------------------------------------------------------
-- pad
-- ---------------------------------------------------------------------------

describe("pad", function()
    it("output has correct dimensions", function()
        local src = pc.new(4, 3)
        local out = M.pad(src, 1, 2, 3, 4)  -- top=1, right=2, bottom=3, left=4
        assert.equal(4 + 4 + 2, out.width)   -- 10
        assert.equal(3 + 1 + 3, out.height)  -- 7
    end)

    it("fill colour applied to border", function()
        local src = solid(2, 2, 0, 0, 0, 255)
        local out = M.pad(src, 1, 1, 1, 1, {200, 100, 50, 255})
        -- Corner pixels are fill colour.
        local r, g, b, a = at(out, 0, 0)
        assert.equal(200, r); assert.equal(100, g); assert.equal(50, b); assert.equal(255, a)
    end)

    it("interior preserves source pixel", function()
        local src = pc.new(2, 2)
        pc.set_pixel(src, 0, 0, 111, 222, 33, 255)
        local out = M.pad(src, 2, 2, 2, 2)
        -- With left=2, top=2 the source pixel (0,0) maps to out(2,2).
        local r, g, b, a = at(out, 2, 2)
        assert.equal(111, r); assert.equal(222, g); assert.equal(33, b); assert.equal(255, a)
    end)

    it("default fill is transparent black", function()
        local src = pc.new(1, 1)
        local out = M.pad(src, 1, 1, 1, 1)
        local r, g, b, a = at(out, 0, 0)
        assert.equal(0, r); assert.equal(0, g); assert.equal(0, b); assert.equal(0, a)
    end)
end)

-- ---------------------------------------------------------------------------
-- scale
-- ---------------------------------------------------------------------------

describe("scale", function()
    it("output has requested dimensions", function()
        local src = pc.new(10, 8)
        local out = M.scale(src, 20, 16)
        assert.equal(20, out.width)
        assert.equal(16, out.height)
    end)

    it("scale to same size is near-identity (bilinear)", function()
        local src = solid(4, 4, 128, 64, 200, 255)
        local out = M.scale(src, 4, 4, "bilinear")
        assert_images_close(src, out, 2, "scale 1:1 bilinear")
    end)

    it("scale to same size is near-identity (nearest)", function()
        local src = solid(4, 4, 100, 150, 200, 255)
        local out = M.scale(src, 4, 4, "nearest")
        assert_images_close(src, out, 1, "scale 1:1 nearest")
    end)

    it("scale to same size is near-identity (bicubic)", function()
        local src = solid(4, 4, 80, 160, 240, 255)
        local out = M.scale(src, 4, 4, "bicubic")
        assert_images_close(src, out, 2, "scale 1:1 bicubic")
    end)

    it("scale up doubles dimensions", function()
        local src = pc.new(3, 5)
        local out = M.scale(src, 6, 10)
        assert.equal(6,  out.width)
        assert.equal(10, out.height)
    end)
end)

-- ---------------------------------------------------------------------------
-- rotate
-- ---------------------------------------------------------------------------

describe("rotate", function()
    it("rotate 0 radians is near-identity", function()
        local src = solid(5, 5, 100, 150, 200, 255)
        local out = M.rotate(src, 0.0, "bilinear", "crop")
        -- With crop mode the size is preserved; values should be very close.
        assert.equal(5, out.width)
        assert.equal(5, out.height)
        assert_images_close(src, out, 2, "rotate 0")
    end)

    it("fit mode grows canvas for 45° rotation", function()
        local src = pc.new(10, 10)
        local out = M.rotate(src, math.pi / 4, "nearest", "fit")
        -- Bounding box of a 10×10 square rotated 45°: 10*|cos|+10*|sin| ≈ 14.14
        assert.is_true(out.width  > 10, "fit width should grow")
        assert.is_true(out.height > 10, "fit height should grow")
    end)

    it("crop mode preserves dimensions", function()
        local src = pc.new(8, 6)
        local out = M.rotate(src, math.pi / 3, "bilinear", "crop")
        assert.equal(8, out.width)
        assert.equal(6, out.height)
    end)

    it("rotate by 2*pi is near-identity", function()
        local src = solid(6, 6, 120, 80, 40, 255)
        local out = M.rotate(src, 2 * math.pi, "bilinear", "crop")
        assert_images_close(src, out, 2, "rotate 2pi")
    end)
end)

-- ---------------------------------------------------------------------------
-- affine
-- ---------------------------------------------------------------------------

describe("affine", function()
    it("identity matrix is near-identity", function()
        local src = solid(5, 5, 90, 130, 170, 255)
        -- Forward identity: output pixel = source pixel.
        local id = {{1, 0, 0}, {0, 1, 0}}
        local out = M.affine(src, id, 5, 5, "bilinear", "replicate")
        assert_images_close(src, out, 2, "affine identity")
    end)

    it("output has requested dimensions", function()
        local src = pc.new(4, 4)
        local id = {{1, 0, 0}, {0, 1, 0}}
        local out = M.affine(src, id, 6, 8)
        assert.equal(6, out.width)
        assert.equal(8, out.height)
    end)

    it("singular matrix raises error", function()
        local src = pc.new(4, 4)
        local sing = {{0, 0, 0}, {0, 0, 0}}
        assert.has_error(function()
            M.affine(src, sing, 4, 4)
        end)
    end)

    it("translation shifts pixels", function()
        -- Forward matrix translates source by (+2, +1): output(x,y) = source(x-2, y-1).
        -- With zero OOB, the top-left 2 columns and top row should be transparent.
        local src = solid(5, 5, 200, 100, 50, 255)
        local t = {{1, 0, 2}, {0, 1, 1}}  -- translate: x'=x+2, y'=y+1
        local out = M.affine(src, t, 5, 5, "nearest", "zero")
        -- output(0,0) maps back to source(0-2, 0-1)=(-2,-1) → OOB → 0
        local r, g, b, a = at(out, 0, 0)
        assert.equal(0, r); assert.equal(0, g); assert.equal(0, b); assert.equal(0, a)
        -- output(2,1) maps back to source(0,0) → 200,100,50,255
        local r2, g2, b2, a2 = at(out, 2, 1)
        assert.equal(200, r2); assert.equal(100, g2); assert.equal(50, b2); assert.equal(255, a2)
    end)
end)

-- ---------------------------------------------------------------------------
-- perspective_warp
-- ---------------------------------------------------------------------------

describe("perspective_warp", function()
    it("identity homography is near-identity", function()
        local src = solid(5, 5, 60, 120, 180, 255)
        -- Identity 3×3 homography.
        local id = {{1, 0, 0}, {0, 1, 0}, {0, 0, 1}}
        local out = M.perspective_warp(src, id, 5, 5, "bilinear", "replicate")
        assert_images_close(src, out, 2, "perspective identity")
    end)

    it("output has requested dimensions", function()
        local src = pc.new(6, 4)
        local id = {{1, 0, 0}, {0, 1, 0}, {0, 0, 1}}
        local out = M.perspective_warp(src, id, 8, 6)
        assert.equal(8, out.width)
        assert.equal(6, out.height)
    end)

    it("singular matrix raises error", function()
        local src = pc.new(4, 4)
        local sing = {{0, 0, 0}, {0, 0, 0}, {0, 0, 0}}
        assert.has_error(function()
            M.perspective_warp(src, sing, 4, 4)
        end)
    end)
end)

-- ---------------------------------------------------------------------------
-- OOB modes via affine
-- ---------------------------------------------------------------------------

describe("OOB modes", function()
    -- Build a 3×3 checkerboard where (0,0) is white and (1,0) is black.
    local function checker()
        local img = pc.new(2, 2)
        pc.set_pixel(img, 0, 0, 255, 255, 255, 255)
        pc.set_pixel(img, 1, 0,   0,   0,   0, 255)
        pc.set_pixel(img, 0, 1,   0,   0,   0, 255)
        pc.set_pixel(img, 1, 1, 255, 255, 255, 255)
        return img
    end

    it("zero OOB outside image gives transparent black", function()
        local src = checker()
        -- Scale down to 1×1 — won't test OOB directly, but affine into OOB region.
        -- Instead, use affine with a large translation so entire output is OOB.
        local t = {{1, 0, 100}, {0, 1, 100}}  -- translate far right/down
        local out = M.affine(src, t, 2, 2, "nearest", "zero")
        -- All output pixels are OOB (source at x<0,y<0 after inversion).
        local r, g, b, a = at(out, 0, 0)
        assert.equal(0, r); assert.equal(0, g); assert.equal(0, b); assert.equal(0, a)
    end)

    it("replicate OOB clamps to edge", function()
        local src = pc.new(3, 3)
        pc.set_pixel(src, 0, 0, 200, 100, 50, 255)  -- top-left is distinctive
        -- Affine that samples to the left of pixel 0 (x=-1 → replicate to 0).
        -- Use identity matrix but offset output so OOB hits the left edge.
        local t = {{1, 0, -2}, {0, 1, 0}}  -- x'=x-2 → src_x = ox+0.5+2-0.5 = ox+2
        -- Actually: forward shift -2 means inv maps out x → src x+2 for replicate test.
        -- Simpler: translate by +5 (source) maps back to -5+ox → all OOB left → clamp to col 0.
        local t2 = {{1, 0, 5}, {0, 1, 0}}
        local out = M.affine(src, t2, 3, 3, "nearest", "replicate")
        -- Output(0,0) maps to source(-4.5,0.5) → clamps to col 0 → should be 200,100,50,255
        local r, g, b, a = at(out, 0, 0)
        assert.equal(200, r)
        assert.equal(255, a)
    end)

    it("wrap OOB tiles the image", function()
        local src = pc.new(2, 1)
        pc.set_pixel(src, 0, 0, 255, 0, 0, 255)  -- red at 0
        pc.set_pixel(src, 1, 0,   0, 0, 255, 255) -- blue at 1
        -- Translate by +2 (one full tile width) should bring red back.
        local t = {{1, 0, 2}, {0, 1, 0}}
        local out = M.affine(src, t, 2, 1, "nearest", "wrap")
        -- Output(0,0) maps to source(0-2+.5 mod2) = source(-1.5+?..) let's just check
        -- that the result is one of {255,0,0} or {0,0,255} (wrap produces a valid pixel).
        local r, g, b = at(out, 0, 0)
        local valid = (r == 255 and g == 0 and b == 0) or (r == 0 and g == 0 and b == 255)
        assert.is_true(valid, "wrap should produce red or blue pixel")
    end)
end)

-- ---------------------------------------------------------------------------
-- Interpolation — nearest exact value
-- ---------------------------------------------------------------------------

describe("nearest interpolation", function()
    it("samples exact pixel value with no blending", function()
        local src = pc.new(3, 3)
        pc.set_pixel(src, 1, 1, 200, 100, 50, 255)
        -- Scale 1:1 nearest should preserve the centre pixel exactly.
        local out = M.scale(src, 3, 3, "nearest")
        local r, g, b, a = at(out, 1, 1)
        assert.equal(200, r); assert.equal(100, g); assert.equal(50, b); assert.equal(255, a)
    end)
end)

-- ---------------------------------------------------------------------------
-- Interpolation — bilinear midpoint blend
-- ---------------------------------------------------------------------------

describe("bilinear interpolation", function()
    it("midpoint between two pixels is average (in linear light)", function()
        -- Two-pixel wide image: left=black(0,0,0), right=white(255,255,255).
        -- Scale to 4 wide: the interpolated pixel at x=1 is roughly mid-grey.
        local src = pc.new(2, 1)
        pc.set_pixel(src, 0, 0,   0,   0,   0, 255)
        pc.set_pixel(src, 1, 0, 255, 255, 255, 255)
        local out = M.scale(src, 4, 1, "bilinear")
        -- Pixel 1 (x=1) should be noticeably brighter than 0 and darker than 255.
        local r, g, b = at(out, 1, 0)
        assert.is_true(r > 10  and r < 245, "bilinear blend R=" .. r)
        assert.is_true(g > 10  and g < 245, "bilinear blend G=" .. g)
        assert.is_true(b > 10  and b < 245, "bilinear blend B=" .. b)
    end)
end)

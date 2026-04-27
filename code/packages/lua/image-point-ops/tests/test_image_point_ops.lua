-- Tests for coding_adventures.image_point_ops (IMG03)

package.path = package.path
    .. ";../src/?.lua;../src/?/init.lua"
    .. ";../../pixel-container/src/?.lua;../../pixel-container/src/?/init.lua"

local ipo = require("coding_adventures.image_point_ops")
local pc  = require("coding_adventures.pixel_container")

-- Helper: 1×1 image with a single pixel.
local function solid(r, g, b, a)
    local img = pc.new(1, 1)
    pc.set_pixel(img, 0, 0, r, g, b, a)
    return img
end

local function px(img)
    return pc.pixel_at(img, 0, 0)
end

-- ── describe("dimensions") ──────────────────────────────────────────────

describe("dimensions", function()
    it("preserves size", function()
        local img = pc.new(3, 5)
        local out = ipo.invert(img)
        assert.equal(3, out.width)
        assert.equal(5, out.height)
    end)
end)

-- ── describe("invert") ──────────────────────────────────────────────────

describe("invert", function()
    it("flips RGB", function()
        local out = ipo.invert(solid(10, 100, 200, 255))
        local r, g, b, a = px(out)
        assert.equal(245, r)
        assert.equal(155, g)
        assert.equal(55, b)
        assert.equal(255, a)
    end)

    it("preserves alpha", function()
        local out = ipo.invert(solid(10, 100, 200, 128))
        local _, _, _, a = px(out)
        assert.equal(128, a)
    end)

    it("double invert is identity", function()
        local img = solid(30, 80, 180, 255)
        local r, g, b, a = px(ipo.invert(ipo.invert(img)))
        local ir, ig, ib, ia = px(img)
        assert.equal(ir, r); assert.equal(ig, g); assert.equal(ib, b); assert.equal(ia, a)
    end)
end)

-- ── describe("threshold") ───────────────────────────────────────────────

describe("threshold", function()
    it("above gives white", function()
        local out = ipo.threshold(solid(200, 200, 200, 255), 128)
        local r, g, b = px(out)
        assert.equal(255, r); assert.equal(255, g); assert.equal(255, b)
    end)

    it("below gives black", function()
        local out = ipo.threshold(solid(50, 50, 50, 255), 128)
        local r = px(out)
        assert.equal(0, r)
    end)

    it("threshold_luminance white stays white", function()
        local out = ipo.threshold_luminance(solid(255, 255, 255, 255), 128)
        local r = px(out)
        assert.equal(255, r)
    end)
end)

-- ── describe("posterize") ───────────────────────────────────────────────

describe("posterize", function()
    it("2 levels binarises", function()
        local out = ipo.posterize(solid(50, 50, 50, 255), 2)
        local r = px(out)
        assert.is_true(r == 0 or r == 255, "expected 0 or 255, got " .. tostring(r))
    end)
end)

-- ── describe("swap_rgb_bgr") ─────────────────────────────────────────────

describe("swap_rgb_bgr", function()
    it("swaps R and B", function()
        local out = ipo.swap_rgb_bgr(solid(255, 0, 0, 255))
        local r, g, b = px(out)
        assert.equal(0, r); assert.equal(0, g); assert.equal(255, b)
    end)
end)

-- ── describe("extract_channel") ──────────────────────────────────────────

describe("extract_channel", function()
    it("extract R zeroes G and B", function()
        local out = ipo.extract_channel(solid(100, 150, 200, 255), 0)
        local r, g, b = px(out)
        assert.equal(100, r); assert.equal(0, g); assert.equal(0, b)
    end)

    it("extract G zeroes R and B", function()
        local out = ipo.extract_channel(solid(100, 150, 200, 255), 1)
        local r, g, b = px(out)
        assert.equal(0, r); assert.equal(150, g); assert.equal(0, b)
    end)
end)

-- ── describe("brightness") ───────────────────────────────────────────────

describe("brightness", function()
    it("clamps high", function()
        local out = ipo.brightness(solid(250, 10, 10, 255), 20)
        local r, g = px(out)
        assert.equal(255, r)
        assert.equal(30, g)
    end)

    it("clamps low", function()
        local out = ipo.brightness(solid(5, 10, 10, 255), -20)
        local r = px(out)
        assert.equal(0, r)
    end)
end)

-- ── describe("contrast") ─────────────────────────────────────────────────

describe("contrast", function()
    it("factor=1 is identity", function()
        local img = solid(100, 150, 200, 255)
        local out = ipo.contrast(img, 1.0)
        local r, g, b = px(out)
        local ir, ig, ib = px(img)
        assert.is_true(math.abs(r - ir) <= 1)
        assert.is_true(math.abs(g - ig) <= 1)
        assert.is_true(math.abs(b - ib) <= 1)
    end)
end)

-- ── describe("gamma") ────────────────────────────────────────────────────

describe("gamma", function()
    it("gamma=1 is identity", function()
        local img = solid(100, 150, 200, 255)
        local out = ipo.gamma(img, 1.0)
        local r = px(out)
        local ir = px(img)
        assert.is_true(math.abs(r - ir) <= 1)
    end)

    it("gamma<1 brightens midtones", function()
        local out = ipo.gamma(solid(128, 128, 128, 255), 0.5)
        local r = px(out)
        assert.is_true(r > 128)
    end)
end)

-- ── describe("exposure") ─────────────────────────────────────────────────

describe("exposure", function()
    it("+1 stop brightens", function()
        local img = solid(100, 100, 100, 255)
        local out = ipo.exposure(img, 1.0)
        local r = px(out)
        local ir = px(img)
        assert.is_true(r > ir)
    end)
end)

-- ── describe("greyscale") ────────────────────────────────────────────────

describe("greyscale", function()
    it("white stays white", function()
        for _, method in ipairs({"rec709", "bt601", "average"}) do
            local out = ipo.greyscale(solid(255, 255, 255, 255), method)
            local r, g, b = px(out)
            assert.equal(255, r); assert.equal(255, g); assert.equal(255, b)
        end
    end)

    it("black stays black", function()
        local out = ipo.greyscale(solid(0, 0, 0, 255))
        local r, g, b = px(out)
        assert.equal(0, r); assert.equal(0, g); assert.equal(0, b)
    end)

    it("gives equal channels", function()
        local out = ipo.greyscale(solid(100, 100, 100, 255))
        local r, g, b = px(out)
        assert.equal(r, g); assert.equal(g, b)
    end)
end)

-- ── describe("sepia") ────────────────────────────────────────────────────

describe("sepia", function()
    it("preserves alpha", function()
        local out = ipo.sepia(solid(128, 128, 128, 200))
        local _, _, _, a = px(out)
        assert.equal(200, a)
    end)
end)

-- ── describe("colour_matrix") ─────────────────────────────────────────────

describe("colour_matrix", function()
    it("identity matrix is identity", function()
        local img = solid(80, 120, 200, 255)
        local id = {{1,0,0},{0,1,0},{0,0,1}}
        local out = ipo.colour_matrix(img, id)
        local r, g, b = px(out)
        local ir, ig, ib = px(img)
        assert.is_true(math.abs(r - ir) <= 1)
        assert.is_true(math.abs(g - ig) <= 1)
        assert.is_true(math.abs(b - ib) <= 1)
    end)
end)

-- ── describe("saturate") ──────────────────────────────────────────────────

describe("saturate", function()
    it("factor=0 gives grey", function()
        local out = ipo.saturate(solid(200, 100, 50, 255), 0.0)
        local r, g, b = px(out)
        assert.equal(r, g); assert.equal(g, b)
    end)
end)

-- ── describe("hue_rotate") ────────────────────────────────────────────────

describe("hue_rotate", function()
    it("360° is identity", function()
        local img = solid(200, 80, 40, 255)
        local out = ipo.hue_rotate(img, 360.0)
        local r, g, b = px(out)
        local ir, ig, ib = px(img)
        assert.is_true(math.abs(r - ir) <= 2)
        assert.is_true(math.abs(g - ig) <= 2)
        assert.is_true(math.abs(b - ib) <= 2)
    end)
end)

-- ── describe("colorspace") ────────────────────────────────────────────────

describe("colorspace", function()
    it("sRGB linear roundtrip", function()
        local img = solid(100, 150, 200, 255)
        local out = ipo.linear_to_srgb_image(ipo.srgb_to_linear_image(img))
        local r, g, b = px(out)
        local ir, ig, ib = px(img)
        assert.is_true(math.abs(r - ir) <= 2)
        assert.is_true(math.abs(g - ig) <= 2)
        assert.is_true(math.abs(b - ib) <= 2)
    end)
end)

-- ── describe("luts") ──────────────────────────────────────────────────────

describe("luts", function()
    it("apply_lut1d invert LUT", function()
        local lut = {}
        for i = 0, 255 do lut[i] = 255 - i end
        local out = ipo.apply_lut1d_u8(solid(100, 0, 200, 255), lut, lut, lut)
        local r, g, b = px(out)
        assert.equal(155, r); assert.equal(255, g); assert.equal(55, b)
    end)

    it("build_lut1d_u8 identity", function()
        local lut = ipo.build_lut1d_u8(function(v) return v end)
        for i = 0, 255 do
            assert.is_true(math.abs(lut[i] - i) <= 1, "index " .. i)
        end
    end)

    it("build_gamma_lut gamma=1 identity", function()
        local lut = ipo.build_gamma_lut(1.0)
        for i = 0, 255 do
            assert.is_true(math.abs(lut[i] - i) <= 1, "index " .. i)
        end
    end)
end)

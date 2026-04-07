-- Tests for coding_adventures.image_codec_qoi (IC03)
--
-- Covers:
--   1. module API       — mime_type, encode/decode functions, codec table
--   2. header structure — magic, big-endian dimensions, channels, colorspace
--   3. end marker       — 8-byte end-of-stream sentinel
--   4. QOI_OP_RUN       — repeated pixel compresses; run limit of 62
--   5. QOI_OP_INDEX     — seen-pixel table lookups
--   6. QOI_OP_DIFF      — small deltas in [-2, 1]
--   7. QOI_OP_LUMA      — medium deltas
--   8. QOI_OP_RGB/RGBA  — raw fallback ops
--   9. round-trips      — encode then decode returns identical container
--  10. error handling   — bad magic, truncated data

package.path = package.path
    .. ";../src/?.lua;../src/?/init.lua"
    .. ";../../pixel-container/src/?.lua;../../pixel-container/src/?/init.lua"

local qoi = require("coding_adventures.image_codec_qoi")
local pc  = require("coding_adventures.pixel_container")

-- ============================================================================
-- Helpers
-- ============================================================================

--- Read a big-endian uint32 from a string at 1-based position p.
local function read_u32_be(s, p)
    return string.unpack(">I4", s, p)
end

--- Build a container filled with a single colour.
local function solid(w, h, r, g, b, a)
    local c = pc.new(w, h)
    pc.fill_pixels(c, r, g, b, a)
    return c
end

-- ============================================================================
-- Module API
-- ============================================================================
describe("module API", function()

    it("exports VERSION string", function()
        assert.is_string(qoi.VERSION)
    end)

    it("exports mime_type = 'image/qoi'", function()
        assert.are.equal("image/qoi", qoi.mime_type)
    end)

    it("exports encode_qoi as a function", function()
        assert.is_function(qoi.encode_qoi)
    end)

    it("exports decode_qoi as a function", function()
        assert.is_function(qoi.decode_qoi)
    end)

    it("codec.mime_type = 'image/qoi'", function()
        assert.are.equal("image/qoi", qoi.codec.mime_type)
    end)

    it("codec.encode is encode_qoi", function()
        assert.are.equal(qoi.encode_qoi, qoi.codec.encode)
    end)

    it("codec.decode is decode_qoi", function()
        assert.are.equal(qoi.decode_qoi, qoi.codec.decode)
    end)

end)

-- ============================================================================
-- encode_qoi — header structure
-- ============================================================================
describe("encode_qoi: header", function()

    it("starts with 'qoif' magic", function()
        local c = pc.new(1, 1)
        local data = qoi.encode_qoi(c)
        assert.are.equal("qoif", string.sub(data, 1, 4))
    end)

    it("bytes 5–8 are width in big-endian", function()
        local c = pc.new(7, 3)
        local data = qoi.encode_qoi(c)
        assert.are.equal(7, read_u32_be(data, 5))
    end)

    it("bytes 9–12 are height in big-endian", function()
        local c = pc.new(7, 3)
        local data = qoi.encode_qoi(c)
        assert.are.equal(3, read_u32_be(data, 9))
    end)

    it("byte 13 is channels = 4 (RGBA)", function()
        local c = pc.new(1, 1)
        local data = qoi.encode_qoi(c)
        assert.are.equal(4, string.byte(data, 13))
    end)

    it("byte 14 is colorspace = 0 (sRGB)", function()
        local c = pc.new(1, 1)
        local data = qoi.encode_qoi(c)
        assert.are.equal(0, string.byte(data, 14))
    end)

end)

-- ============================================================================
-- encode_qoi — end marker
-- ============================================================================
describe("encode_qoi: end marker", function()

    it("last 8 bytes are 00 00 00 00 00 00 00 01", function()
        local c = pc.new(2, 2)
        local data = qoi.encode_qoi(c)
        local tail = string.sub(data, -8)
        local expected = string.char(0,0,0,0,0,0,0,1)
        assert.are.equal(expected, tail)
    end)

end)

-- ============================================================================
-- QOI_OP_RUN compression
-- ============================================================================
describe("QOI_OP_RUN", function()

    it("solid colour image is smaller than 14 + n_pixels * 5 bytes", function()
        -- Without run encoding, 100 pixels would be at least 100 bytes of data.
        -- With run encoding, they collapse to just a few RUN ops.
        local c = solid(10, 10, 255, 0, 0, 255)
        local data = qoi.encode_qoi(c)
        -- Header(14) + a few ops + end(8) should be far less than 14 + 500 + 8
        assert.is_true(#data < 50, "solid image should compress well, got " .. #data .. " bytes")
    end)

    it("1×1 image encodes without error", function()
        local c = pc.new(1, 1)
        assert.has_no_error(function() qoi.encode_qoi(c) end)
    end)

    it("run limit: 62 identical pixels followed by a different pixel", function()
        -- Build a 63-pixel image: 62 red pixels then 1 blue pixel (1 row, 63 wide)
        local c = pc.new(63, 1)
        for x = 0, 61 do pc.set_pixel(c, x, 0, 255, 0, 0, 255) end
        pc.set_pixel(c, 62, 0, 0, 0, 255, 255)
        -- Should encode and decode correctly
        local c2 = qoi.decode_qoi(qoi.encode_qoi(c))
        assert.is_true(pc.equals(c, c2))
    end)

end)

-- ============================================================================
-- QOI_OP_DIFF
-- ============================================================================
describe("QOI_OP_DIFF", function()

    it("pixels differing by 1 in each channel encode and decode correctly", function()
        local c = pc.new(2, 1)
        pc.set_pixel(c, 0, 0, 100, 100, 100, 255)
        pc.set_pixel(c, 1, 0, 101, 101, 101, 255)  -- dr=dg=db=1, da=0 → DIFF
        local c2 = qoi.decode_qoi(qoi.encode_qoi(c))
        assert.is_true(pc.equals(c, c2))
    end)

    it("pixels differing by -2 in one channel (wrap boundary)", function()
        local c = pc.new(2, 1)
        pc.set_pixel(c, 0, 0, 1, 128, 128, 255)
        -- dr = -2 (border of DIFF range), dg = db = 0
        pc.set_pixel(c, 1, 0, 255, 128, 128, 255)
        local c2 = qoi.decode_qoi(qoi.encode_qoi(c))
        assert.is_true(pc.equals(c, c2))
    end)

end)

-- ============================================================================
-- QOI_OP_LUMA
-- ============================================================================
describe("QOI_OP_LUMA", function()

    it("medium green delta encodes and round-trips correctly", function()
        local c = pc.new(2, 1)
        pc.set_pixel(c, 0, 0, 100, 100, 100, 255)
        -- dg = 10 (outside DIFF range [-2,1] but within LUMA range [-32,31])
        -- dr = 10, db = 10 → dr - dg = 0, db - dg = 0 (both within [-8,7])
        pc.set_pixel(c, 1, 0, 110, 110, 110, 255)
        local c2 = qoi.decode_qoi(qoi.encode_qoi(c))
        assert.is_true(pc.equals(c, c2))
    end)

    it("asymmetric LUMA delta round-trips correctly", function()
        local c = pc.new(2, 1)
        pc.set_pixel(c, 0, 0, 128, 128, 128, 255)
        -- dg = 5, dr = 7 (dr-dg=2), db = 3 (db-dg=-2) — fits LUMA
        pc.set_pixel(c, 1, 0, 135, 133, 131, 255)
        local c2 = qoi.decode_qoi(qoi.encode_qoi(c))
        assert.is_true(pc.equals(c, c2))
    end)

end)

-- ============================================================================
-- QOI_OP_RGB and QOI_OP_RGBA
-- ============================================================================
describe("QOI_OP_RGB and QOI_OP_RGBA", function()

    it("large RGB delta uses QOI_OP_RGB and round-trips correctly", function()
        local c = pc.new(2, 1)
        pc.set_pixel(c, 0, 0, 0,   0,   0,   255)
        pc.set_pixel(c, 1, 0, 200, 100, 50,  255)  -- too large for DIFF/LUMA
        local c2 = qoi.decode_qoi(qoi.encode_qoi(c))
        assert.is_true(pc.equals(c, c2))
    end)

    it("alpha change uses QOI_OP_RGBA and round-trips correctly", function()
        local c = pc.new(2, 1)
        pc.set_pixel(c, 0, 0, 100, 100, 100, 255)
        pc.set_pixel(c, 1, 0, 100, 100, 100, 128)  -- only alpha changed
        local c2 = qoi.decode_qoi(qoi.encode_qoi(c))
        assert.is_true(pc.equals(c, c2))
    end)

end)

-- ============================================================================
-- QOI_OP_INDEX
-- ============================================================================
describe("QOI_OP_INDEX", function()

    it("pixel returning to a previously seen colour uses index op", function()
        -- Pattern: A, B, A — second A should be an INDEX op
        local c = pc.new(3, 1)
        pc.set_pixel(c, 0, 0, 200, 50, 100, 255)
        pc.set_pixel(c, 1, 0, 10,  20, 30,  255)
        pc.set_pixel(c, 2, 0, 200, 50, 100, 255)  -- same as pixel 0 → INDEX
        local c2 = qoi.decode_qoi(qoi.encode_qoi(c))
        assert.is_true(pc.equals(c, c2))
    end)

end)

-- ============================================================================
-- Round-trips
-- ============================================================================
describe("round-trip: encode then decode", function()

    it("1×1 all-zero image round-trips", function()
        local c = pc.new(1, 1)
        local c2 = qoi.decode_qoi(qoi.encode_qoi(c))
        assert.is_true(pc.equals(c, c2))
    end)

    it("1×1 opaque white pixel round-trips", function()
        local c = solid(1, 1, 255, 255, 255, 255)
        local c2 = qoi.decode_qoi(qoi.encode_qoi(c))
        assert.is_true(pc.equals(c, c2))
    end)

    it("4×4 checkerboard pattern round-trips", function()
        local c = pc.new(4, 4)
        for y = 0, 3 do
            for x = 0, 3 do
                if (x + y) % 2 == 0 then
                    pc.set_pixel(c, x, y, 255, 255, 255, 255)
                else
                    pc.set_pixel(c, x, y, 0, 0, 0, 255)
                end
            end
        end
        local c2 = qoi.decode_qoi(qoi.encode_qoi(c))
        assert.is_true(pc.equals(c, c2))
    end)

    it("gradient image round-trips", function()
        local c = pc.new(16, 16)
        for y = 0, 15 do
            for x = 0, 15 do
                pc.set_pixel(c, x, y, x * 16, y * 16, (x + y) * 8, 255)
            end
        end
        local c2 = qoi.decode_qoi(qoi.encode_qoi(c))
        assert.is_true(pc.equals(c, c2))
    end)

    it("image with varying alpha round-trips", function()
        local c = pc.new(4, 4)
        for y = 0, 3 do
            for x = 0, 3 do
                pc.set_pixel(c, x, y, 128, 64, 32, (x + y) * 32)
            end
        end
        local c2 = qoi.decode_qoi(qoi.encode_qoi(c))
        assert.is_true(pc.equals(c, c2))
    end)

    it("decode produces correct dimensions", function()
        local c = pc.new(19, 7)
        local c2 = qoi.decode_qoi(qoi.encode_qoi(c))
        assert.are.equal(19, c2.width)
        assert.are.equal(7,  c2.height)
    end)

    it("noisy image (all distinct pixels) round-trips", function()
        -- Force use of RGB/RGBA ops for every pixel
        local c = pc.new(8, 8)
        for y = 0, 7 do
            for x = 0, 7 do
                -- Large jumps to avoid DIFF/LUMA matching
                pc.set_pixel(c, x, y,
                    (x * 37 + y * 13) % 256,
                    (x * 73 + y * 31) % 256,
                    (x * 53 + y * 61) % 256,
                    255)
            end
        end
        local c2 = qoi.decode_qoi(qoi.encode_qoi(c))
        assert.is_true(pc.equals(c, c2))
    end)

end)

-- ============================================================================
-- decode_qoi — error handling
-- ============================================================================
describe("decode_qoi: error handling", function()

    it("raises error for non-string input", function()
        assert.has_error(function() qoi.decode_qoi(nil) end)
    end)

    it("raises error for data too short", function()
        assert.has_error(function() qoi.decode_qoi("qoif") end)
    end)

    it("raises error for wrong magic", function()
        local c = pc.new(1, 1)
        local data = qoi.encode_qoi(c)
        local bad = "xxxx" .. string.sub(data, 5)
        assert.has_error(function() qoi.decode_qoi(bad) end)
    end)

end)

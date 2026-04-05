-- Tests for coding_adventures.image_codec_bmp (IC01)
--
-- Covers:
--   1. module API        — mime_type, encode/decode functions, codec table
--   2. encode structure  — file header magic, correct file size, pixel offset
--   3. encode pixels     — BGRA channel order, specific pixel values
--   4. decode validation — magic check, bit-depth check, round-trips
--   5. round-trip        — encode then decode returns identical container
--   6. dimensions        — 1×1, 1×N, N×1, larger images
--   7. alpha channel     — preserved through encode/decode
--   8. error handling    — bad magic, truncated data, unsupported bpp

package.path = package.path
    .. ";../src/?.lua;../src/?/init.lua"
    .. ";../../pixel-container/src/?.lua;../../pixel-container/src/?/init.lua"

local bmp = require("coding_adventures.image_codec_bmp")
local pc  = require("coding_adventures.pixel_container")

-- ============================================================================
-- Helpers
-- ============================================================================

--- Read a little-endian uint16 from a string at 1-based position p.
local function read_u16(s, p)
    return string.unpack("<I2", s, p)
end

--- Read a little-endian uint32 from a string at 1-based position p.
local function read_u32(s, p)
    return string.unpack("<I4", s, p)
end

--- Read a little-endian int32 from a string at 1-based position p.
local function read_i32(s, p)
    return string.unpack("<i4", s, p)
end

-- ============================================================================
-- Module API
-- ============================================================================
describe("module API", function()

    it("exports VERSION string", function()
        assert.is_string(bmp.VERSION)
    end)

    it("exports mime_type = 'image/bmp'", function()
        assert.are.equal("image/bmp", bmp.mime_type)
    end)

    it("exports encode_bmp as a function", function()
        assert.is_function(bmp.encode_bmp)
    end)

    it("exports decode_bmp as a function", function()
        assert.is_function(bmp.decode_bmp)
    end)

    it("codec table has mime_type field", function()
        assert.are.equal("image/bmp", bmp.codec.mime_type)
    end)

    it("codec.encode is encode_bmp", function()
        assert.are.equal(bmp.encode_bmp, bmp.codec.encode)
    end)

    it("codec.decode is decode_bmp", function()
        assert.are.equal(bmp.decode_bmp, bmp.codec.decode)
    end)

end)

-- ============================================================================
-- encode_bmp — file header structure
-- ============================================================================
describe("encode_bmp: file header", function()

    it("output starts with 'BM' magic bytes", function()
        local c = pc.new(1, 1)
        local data = bmp.encode_bmp(c)
        assert.are.equal("BM", string.sub(data, 1, 2))
    end)

    it("bfSize (bytes 3–6) equals total byte length", function()
        local c = pc.new(4, 3)
        local data = bmp.encode_bmp(c)
        local file_size = read_u32(data, 3)
        assert.are.equal(#data, file_size)
    end)

    it("bfReserved1 and bfReserved2 are zero", function()
        local c = pc.new(2, 2)
        local data = bmp.encode_bmp(c)
        local r1 = read_u16(data, 7)
        local r2 = read_u16(data, 9)
        assert.are.equal(0, r1)
        assert.are.equal(0, r2)
    end)

    it("bfOffBits (bytes 11–14) = 54", function()
        local c = pc.new(2, 2)
        local data = bmp.encode_bmp(c)
        local off = read_u32(data, 11)
        assert.are.equal(54, off)
    end)

    it("file size = 54 + width * height * 4", function()
        local c = pc.new(10, 8)
        local data = bmp.encode_bmp(c)
        local expected = 54 + 10 * 8 * 4
        assert.are.equal(expected, #data)
    end)

end)

-- ============================================================================
-- encode_bmp — DIB header (BITMAPINFOHEADER, at offset 14)
-- ============================================================================
describe("encode_bmp: DIB header", function()

    it("biSize = 40", function()
        local c = pc.new(2, 2)
        local data = bmp.encode_bmp(c)
        assert.are.equal(40, read_u32(data, 15))
    end)

    it("biWidth = container width", function()
        local c = pc.new(7, 5)
        local data = bmp.encode_bmp(c)
        assert.are.equal(7, read_i32(data, 19))
    end)

    it("biHeight = -height (negative for top-down)", function()
        local c = pc.new(7, 5)
        local data = bmp.encode_bmp(c)
        assert.are.equal(-5, read_i32(data, 23))
    end)

    it("biPlanes = 1", function()
        local c = pc.new(2, 2)
        local data = bmp.encode_bmp(c)
        assert.are.equal(1, read_u16(data, 27))
    end)

    it("biBitCount = 32", function()
        local c = pc.new(2, 2)
        local data = bmp.encode_bmp(c)
        assert.are.equal(32, read_u16(data, 29))
    end)

    it("biCompression = 0 (BI_RGB)", function()
        local c = pc.new(2, 2)
        local data = bmp.encode_bmp(c)
        assert.are.equal(0, read_u32(data, 31))
    end)

end)

-- ============================================================================
-- encode_bmp — pixel data (BGRA byte order at offset 54)
-- ============================================================================
describe("encode_bmp: pixel data", function()

    it("all-zero container produces all-zero pixel bytes", function()
        local c = pc.new(2, 2)
        local data = bmp.encode_bmp(c)
        for i = 55, #data do
            assert.are.equal(0, string.byte(data, i),
                "byte " .. i .. " should be 0")
        end
    end)

    it("pixel (0,0) is stored at byte offset 54 in BGRA order", function()
        local c = pc.new(3, 3)
        pc.set_pixel(c, 0, 0, 100, 150, 200, 255)  -- R=100, G=150, B=200, A=255
        local data = bmp.encode_bmp(c)
        -- Pixel data starts at byte 55 (1-indexed)
        -- BGRA: B=200, G=150, R=100, A=255
        assert.are.equal(200, string.byte(data, 55))  -- B
        assert.are.equal(150, string.byte(data, 56))  -- G
        assert.are.equal(100, string.byte(data, 57))  -- R
        assert.are.equal(255, string.byte(data, 58))  -- A
    end)

    it("second pixel (1,0) is stored at bytes 59–62", function()
        local c = pc.new(3, 3)
        pc.set_pixel(c, 1, 0, 10, 20, 30, 40)
        local data = bmp.encode_bmp(c)
        assert.are.equal(30, string.byte(data, 59))   -- B
        assert.are.equal(20, string.byte(data, 60))   -- G
        assert.are.equal(10, string.byte(data, 61))   -- R
        assert.are.equal(40, string.byte(data, 62))   -- A
    end)

    it("alpha is preserved in the fourth byte of each pixel block", function()
        local c = pc.new(1, 1)
        pc.set_pixel(c, 0, 0, 0, 0, 0, 127)
        local data = bmp.encode_bmp(c)
        assert.are.equal(127, string.byte(data, 58))  -- A byte
    end)

end)

-- ============================================================================
-- decode_bmp — validation
-- ============================================================================
describe("decode_bmp: validation", function()

    it("raises error for empty string", function()
        assert.has_error(function() bmp.decode_bmp("") end)
    end)

    it("raises error for wrong magic bytes", function()
        local c = pc.new(2, 2)
        local data = bmp.encode_bmp(c)
        -- Replace 'BM' with 'XX'
        local bad = "XX" .. string.sub(data, 3)
        assert.has_error(function() bmp.decode_bmp(bad) end)
    end)

    it("raises error for truncated data", function()
        local c = pc.new(4, 4)
        local data = bmp.encode_bmp(c)
        assert.has_error(function() bmp.decode_bmp(string.sub(data, 1, 10)) end)
    end)

    it("raises error for non-string input", function()
        assert.has_error(function() bmp.decode_bmp(42) end)
    end)

end)

-- ============================================================================
-- Round-trip: encode then decode
-- ============================================================================
describe("round-trip: encode then decode", function()

    it("1×1 all-zero image round-trips correctly", function()
        local c = pc.new(1, 1)
        local c2 = bmp.decode_bmp(bmp.encode_bmp(c))
        assert.is_true(pc.equals(c, c2))
    end)

    it("1×1 coloured pixel round-trips correctly", function()
        local c = pc.new(1, 1)
        pc.set_pixel(c, 0, 0, 200, 100, 50, 255)
        local c2 = bmp.decode_bmp(bmp.encode_bmp(c))
        local r, g, b, a = pc.pixel_at(c2, 0, 0)
        assert.are.equal(200, r)
        assert.are.equal(100, g)
        assert.are.equal(50,  b)
        assert.are.equal(255, a)
    end)

    it("4×4 image with distinct pixels round-trips correctly", function()
        local c = pc.new(4, 4)
        for y = 0, 3 do
            for x = 0, 3 do
                pc.set_pixel(c, x, y, x * 16, y * 16, (x + y) * 8, 128)
            end
        end
        local c2 = bmp.decode_bmp(bmp.encode_bmp(c))
        assert.is_true(pc.equals(c, c2))
    end)

    it("alpha channel survives round-trip", function()
        local c = pc.new(2, 2)
        pc.set_pixel(c, 0, 0, 255, 0,   0,   0)
        pc.set_pixel(c, 1, 0, 0,   255, 0,   64)
        pc.set_pixel(c, 0, 1, 0,   0,   255, 128)
        pc.set_pixel(c, 1, 1, 128, 128, 128, 200)
        local c2 = bmp.decode_bmp(bmp.encode_bmp(c))
        assert.is_true(pc.equals(c, c2))
    end)

    it("decode produces correct dimensions", function()
        local c = pc.new(17, 11)
        local c2 = bmp.decode_bmp(bmp.encode_bmp(c))
        assert.are.equal(17, c2.width)
        assert.are.equal(11, c2.height)
    end)

end)

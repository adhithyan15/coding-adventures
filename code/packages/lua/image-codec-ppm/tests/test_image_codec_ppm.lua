-- Tests for coding_adventures.image_codec_ppm (IC02)
--
-- Covers:
--   1. module API         — mime_type, encode/decode functions, codec table
--   2. encode header      — P6 magic, width/height/maxval in header
--   3. encode pixel data  — RGB byte order (no alpha), correct offsets
--   4. alpha handling     — encode drops alpha; decode sets alpha = 255
--   5. decode validation  — magic, format, maxval, truncated data
--   6. round-trip         — encode then decode returns matching RGB (A=255)
--   7. comment handling   — decode tolerates '#' comment lines in header
--   8. dimensions         — 1×1, 1×N, N×1, larger images

package.path = package.path
    .. ";../src/?.lua;../src/?/init.lua"
    .. ";../../pixel-container/src/?.lua;../../pixel-container/src/?/init.lua"

local ppm = require("coding_adventures.image_codec_ppm")
local pc  = require("coding_adventures.pixel_container")

-- ============================================================================
-- Module API
-- ============================================================================
describe("module API", function()

    it("exports VERSION string", function()
        assert.is_string(ppm.VERSION)
    end)

    it("exports mime_type = 'image/x-portable-pixmap'", function()
        assert.are.equal("image/x-portable-pixmap", ppm.mime_type)
    end)

    it("exports encode_ppm as a function", function()
        assert.is_function(ppm.encode_ppm)
    end)

    it("exports decode_ppm as a function", function()
        assert.is_function(ppm.decode_ppm)
    end)

    it("codec.mime_type = 'image/x-portable-pixmap'", function()
        assert.are.equal("image/x-portable-pixmap", ppm.codec.mime_type)
    end)

    it("codec.encode is encode_ppm", function()
        assert.are.equal(ppm.encode_ppm, ppm.codec.encode)
    end)

    it("codec.decode is decode_ppm", function()
        assert.are.equal(ppm.decode_ppm, ppm.codec.decode)
    end)

end)

-- ============================================================================
-- encode_ppm — header structure
-- ============================================================================
describe("encode_ppm: header", function()

    it("header starts with 'P6\\n'", function()
        local c = pc.new(1, 1)
        local data = ppm.encode_ppm(c)
        assert.are.equal("P6\n", string.sub(data, 1, 3))
    end)

    it("header contains correct width and height", function()
        local c = pc.new(7, 5)
        local data = ppm.encode_ppm(c)
        -- Header is "P6\n7 5\n255\n"
        assert.is_truthy(string.find(data, "7 5", 1, true))
    end)

    it("header contains maxval 255", function()
        local c = pc.new(2, 2)
        local data = ppm.encode_ppm(c)
        assert.is_truthy(string.find(data, "255", 1, true))
    end)

    it("header is plain ASCII (only checks bytes up to first newline after maxval)", function()
        -- Use a large enough image so header = "P6\n100 100\n255\n" = 15 bytes,
        -- which is entirely ASCII before pixel data begins.
        local c = pc.new(100, 100)
        local data = ppm.encode_ppm(c)
        local header = "P6\n100 100\n255\n"
        local header_len = #header
        local header_part = string.sub(data, 1, header_len)
        for i = 1, #header_part do
            local b = string.byte(header_part, i)
            -- Allow printable ASCII (32–126) and common whitespace (9, 10, 13)
            local ok = (b >= 32 and b <= 126) or b == 9 or b == 10 or b == 13
            if not ok then
                assert.is_true(false, "non-ASCII byte " .. b .. " at position " .. i)
            end
        end
    end)

    it("file size = header_len + width * height * 3", function()
        local c = pc.new(4, 3)
        local data = ppm.encode_ppm(c)
        -- Header is "P6\n4 3\n255\n" = 12 bytes
        local header = "P6\n4 3\n255\n"
        local expected = #header + 4 * 3 * 3
        assert.are.equal(expected, #data)
    end)

end)

-- ============================================================================
-- encode_ppm — pixel data
-- ============================================================================
describe("encode_ppm: pixel data", function()

    it("all-zero container produces all-zero pixel bytes after header", function()
        local c = pc.new(2, 2)
        local data = ppm.encode_ppm(c)
        -- Header is "P6\n2 2\n255\n" = 12 bytes; pixel data starts at byte 13
        local header = "P6\n2 2\n255\n"
        for i = #header + 1, #data do
            assert.are.equal(0, string.byte(data, i))
        end
    end)

    it("first pixel (0,0) is at correct position with RGB order", function()
        local c = pc.new(3, 3)
        pc.set_pixel(c, 0, 0, 100, 150, 200, 255)
        local data = ppm.encode_ppm(c)
        local header = "P6\n3 3\n255\n"
        local p = #header + 1
        assert.are.equal(100, string.byte(data, p))     -- R
        assert.are.equal(150, string.byte(data, p + 1)) -- G
        assert.are.equal(200, string.byte(data, p + 2)) -- B
    end)

    it("second pixel (1,0) follows immediately after first pixel", function()
        local c = pc.new(3, 3)
        pc.set_pixel(c, 1, 0, 10, 20, 30, 99)
        local data = ppm.encode_ppm(c)
        local header = "P6\n3 3\n255\n"
        local p = #header + 1 + 3  -- skip first pixel (3 bytes)
        assert.are.equal(10, string.byte(data, p))
        assert.are.equal(20, string.byte(data, p + 1))
        assert.are.equal(30, string.byte(data, p + 2))
    end)

    it("alpha is NOT stored — pixel bytes are 3 per pixel, not 4", function()
        local c = pc.new(1, 1)
        pc.set_pixel(c, 0, 0, 50, 100, 150, 200)
        local data = ppm.encode_ppm(c)
        local header = "P6\n1 1\n255\n"
        -- Pixel section should be exactly 3 bytes (R, G, B), not 4
        assert.are.equal(#header + 3, #data)
    end)

end)

-- ============================================================================
-- decode_ppm — validation
-- ============================================================================
describe("decode_ppm: validation", function()

    it("raises error for empty string", function()
        assert.has_error(function() ppm.decode_ppm("") end)
    end)

    it("raises error for wrong magic (P3 not supported)", function()
        assert.has_error(function()
            ppm.decode_ppm("P3\n1 1\n255\n255 0 0\n")
        end)
    end)

    it("raises error for maxval != 255", function()
        assert.has_error(function()
            -- 2-byte maxval
            ppm.decode_ppm("P6\n1 1\n65535\n" .. string.char(0, 0))
        end)
    end)

    it("raises error for truncated pixel data", function()
        -- Header says 2×2 but only 3 bytes of pixel data provided
        local header = "P6\n2 2\n255\n"
        assert.has_error(function()
            ppm.decode_ppm(header .. "\x00\x00\x00")
        end)
    end)

    it("raises error for non-string input", function()
        assert.has_error(function() ppm.decode_ppm(nil) end)
    end)

end)

-- ============================================================================
-- decode_ppm — comment handling
-- ============================================================================
describe("decode_ppm: comment handling", function()

    it("skips '#' comment line between magic and dimensions", function()
        local header = "P6\n# created by test\n1 1\n255\n"
        local data   = header .. string.char(10, 20, 30)
        local c = ppm.decode_ppm(data)
        local r, g, b, a = pc.pixel_at(c, 0, 0)
        assert.are.equal(10,  r)
        assert.are.equal(20,  g)
        assert.are.equal(30,  b)
        assert.are.equal(255, a)
    end)

    it("skips '#' comment line between dimensions and maxval", function()
        local header = "P6\n2 2\n# comment\n255\n"
        local pixels = string.char(1,2,3, 4,5,6, 7,8,9, 10,11,12)
        local c = ppm.decode_ppm(header .. pixels)
        assert.are.equal(2, c.width)
        assert.are.equal(2, c.height)
    end)

end)

-- ============================================================================
-- Round-trip
-- ============================================================================
describe("round-trip: encode then decode", function()

    it("1×1 pixel (RGB only) round-trips correctly", function()
        local c = pc.new(1, 1)
        pc.set_pixel(c, 0, 0, 200, 100, 50, 255)
        local c2 = ppm.decode_ppm(ppm.encode_ppm(c))
        local r, g, b, a = pc.pixel_at(c2, 0, 0)
        assert.are.equal(200, r)
        assert.are.equal(100, g)
        assert.are.equal(50,  b)
        assert.are.equal(255, a)  -- alpha is always 255 after decode
    end)

    it("decode always produces alpha = 255 regardless of source alpha", function()
        local c = pc.new(2, 1)
        pc.set_pixel(c, 0, 0, 10, 20, 30, 0)    -- fully transparent
        pc.set_pixel(c, 1, 0, 40, 50, 60, 128)  -- half transparent
        local c2 = ppm.decode_ppm(ppm.encode_ppm(c))
        local _, _, _, a0 = pc.pixel_at(c2, 0, 0)
        local _, _, _, a1 = pc.pixel_at(c2, 1, 0)
        assert.are.equal(255, a0)
        assert.are.equal(255, a1)
    end)

    it("4×3 image RGB channels survive round-trip", function()
        local c = pc.new(4, 3)
        for y = 0, 2 do
            for x = 0, 3 do
                pc.set_pixel(c, x, y, x * 50, y * 70, (x + y) * 20, 255)
            end
        end
        local c2 = ppm.decode_ppm(ppm.encode_ppm(c))
        for y = 0, 2 do
            for x = 0, 3 do
                local r1, g1, b1 = pc.pixel_at(c, x, y)
                local r2, g2, b2 = pc.pixel_at(c2, x, y)
                assert.are.equal(r1, r2)
                assert.are.equal(g1, g2)
                assert.are.equal(b1, b2)
            end
        end
    end)

    it("decode produces correct dimensions", function()
        local c = pc.new(13, 9)
        local c2 = ppm.decode_ppm(ppm.encode_ppm(c))
        assert.are.equal(13, c2.width)
        assert.are.equal(9,  c2.height)
    end)

    it("all channels 255 survive round-trip", function()
        local c = pc.new(2, 2)
        pc.fill_pixels(c, 255, 255, 255, 255)
        local c2 = ppm.decode_ppm(ppm.encode_ppm(c))
        for y = 0, 1 do
            for x = 0, 1 do
                local r, g, b, a = pc.pixel_at(c2, x, y)
                assert.are.equal(255, r)
                assert.are.equal(255, g)
                assert.are.equal(255, b)
                assert.are.equal(255, a)
            end
        end
    end)

end)

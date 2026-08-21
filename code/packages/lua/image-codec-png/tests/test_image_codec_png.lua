package.path = package.path
    .. ";../src/?.lua;../src/?/init.lua"
    .. ";../../pixel-container/src/?.lua;../../pixel-container/src/?/init.lua"
    .. ";../../zip/src/?.lua;../../zip/src/?/init.lua"
    .. ";../../lzss/src/?.lua;../../lzss/src/?/init.lua"

local pc = require("coding_adventures.pixel_container")
local png = require("coding_adventures.image_codec_png")
local zip = require("coding_adventures.zip")

local function expect_error(code, action)
    local ok, failure = pcall(action)
    assert.is_false(ok)
    assert.is_table(failure)
    assert.are.equal(code, failure.code)
    assert.are.equal(code, failure.message)
    assert.are.equal(code, tostring(failure))
end

local function u32(value)
    return string.pack(">I4", value)
end

local function chunk(kind, payload)
    local checksum = zip.crc32(kind)
    checksum = zip.crc32(payload, checksum)
    return u32(#payload) .. kind .. payload .. u32(checksum)
end

local function insert_at(value, offset, addition)
    return value:sub(1, offset) .. addition .. value:sub(offset + 1)
end

describe("portable PNG public API", function()
    it("implements the PixelContainer codec convention", function()
        local codec = png.PngCodec.new()
        assert.are.equal("image/png", codec.mime_type)
        local pixels = pc.from_bytes(1, 1, string.char(1, 2, 3, 4))
        local decoded = codec:decode(codec:encode(pixels))
        assert.is_true(pc.equals(pixels, decoded))
        assert.is_true(pc.equals(pixels, png.PngCodec.new({max_pixels = 1}):decode(codec:encode(pixels))))
    end)

    it("validates caller pixel limits before parsing", function()
        local invalid = {0, -1, 1.5, png.PNG_MAX_PIXELS + 1, 0 / 0, math.huge, -math.huge, true}
        for _, value in ipairs(invalid) do
            expect_error("invalid-max-pixels", function()
                png.PngCodec.new({max_pixels = value})
            end)
            expect_error("invalid-max-pixels", function()
                png.decode_png("", {max_pixels = value})
            end)
        end
    end)

    it("validates open PixelContainer state before allocation", function()
        expect_error("invalid-image-dimensions", function()
            png.encode_png({width = 0, height = 1, data = {}})
        end)
        expect_error("invalid-image-dimensions", function()
            png.encode_png({width = png.PNG_MAX_DIMENSION + 1, height = 1, data = {}})
        end)
        expect_error("invalid-pixel-data-length", function()
            png.encode_png({width = 1, height = 1, data = {1, 2, 3}})
        end)
        expect_error("invalid-pixel-data-length", function()
            png.encode_png({width = 1, height = 1, data = {[1] = 1, [2] = 2, [4] = 4}})
        end)
        expect_error("invalid-pixel-data-length", function()
            png.encode_png({width = 1, height = 1, data = {1, 2, 3, 4, 5}})
        end)
        expect_error("invalid-pixel-data-length", function()
            png.encode_png({width = 1, height = 1, data = {1, 2, 3, 256}})
        end)
    end)

    it("returns a compact mutable PixelContainer byte table", function()
        local decoded = png.decode_png(png.encode_png(pc.from_bytes(1, 1, string.char(1, 2, 3, 4))))
        assert.are.equal(4, #decoded.data)
        assert.is_nil(next(decoded.data))
        decoded.data[4] = 99
        assert.are.equal(99, select(4, pc.pixel_at(decoded, 0, 0)))
    end)

    it("publishes an immutable closed payload-blind taxonomy", function()
        local codes = png.png_error_codes()
        assert.are.equal(29, #codes)
        expect_error("invalid-filter", function()
            error(png.PngError.new("invalid-filter"), 0)
        end)
        assert.has_error(function()
            png.PNG_ERROR_CODES[1] = "changed"
        end)
    end)

    it("preserves CRC and first-IHDR precedence for APNG", function()
        local encoded = png.encode_png(pc.new(1, 1))
        local valid = chunk("acTL", string.rep("\0", 8))
        expect_error("unsupported-feature", function()
            png.decode_png(insert_at(encoded, 33, valid))
        end)
        local corrupt = valid:sub(1, -2) .. string.char(valid:byte(-1) ~ 1)
        expect_error("chunk-crc-mismatch", function()
            png.decode_png(insert_at(encoded, 33, corrupt))
        end)
        expect_error("chunk-before-ihdr", function()
            png.decode_png(insert_at(encoded, 8, valid))
        end)
    end)

    it("rejects a maximal unsigned chunk length before slicing", function()
        expect_error("truncated-chunk", function()
            png.decode_png("\137PNG\r\n\26\n" .. u32(0xffffffff) .. "IHDR")
        end)
    end)

    it("matches Adler-32 vectors across the reduction boundary", function()
        assert.are.equal(0x11e60398, png.adler32("Wikipedia"))
        local bytes = {}
        for index = 0, 5552 do
            bytes[#bytes + 1] = string.char(index & 0xff)
        end
        assert.are.equal(0x2ccab2ef, png.adler32(table.concat(bytes)))
    end)
end)

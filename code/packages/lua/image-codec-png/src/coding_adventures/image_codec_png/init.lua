-- IC18 portable PNG codec for Lua.
--
-- Production owns PNG framing, validation, filters, and the zlib wrapper. Raw
-- RFC 1951 and CRC-32 come only from the repository ZIP package.

local pc = require("coding_adventures.pixel_container")
local zip = require("coding_adventures.zip")

local M = {}

M.VERSION = "0.1.0"
M.mime_type = "image/png"
M.PNG_MAX_DIMENSION = 16384
M.PNG_MAX_PIXELS = 32 * 1024 * 1024

local ERROR_CODES = {
    "invalid-max-pixels",
    "invalid-image-dimensions",
    "invalid-pixel-data-length",
    "file-too-short",
    "invalid-signature",
    "truncated-chunk",
    "invalid-chunk-type",
    "chunk-crc-mismatch",
    "chunk-before-ihdr",
    "duplicate-ihdr",
    "invalid-ihdr-length",
    "invalid-dimensions",
    "dimension-limit",
    "pixel-limit",
    "unsupported-feature",
    "invalid-plte",
    "invalid-trns",
    "nonconsecutive-idat",
    "invalid-iend",
    "trailing-data",
    "unknown-critical-chunk",
    "missing-required-chunk",
    "invalid-zlib-header",
    "preset-dictionary",
    "inflate-failed",
    "inflated-length-mismatch",
    "idat-cavity",
    "adler-mismatch",
    "invalid-filter",
}

local error_proxy = {}
setmetatable(error_proxy, {
    __len = function() return #ERROR_CODES end,
    __index = ERROR_CODES,
    __newindex = function() error("PNG_ERROR_CODES is immutable", 2) end,
    __pairs = function() return ipairs(ERROR_CODES) end,
    __metatable = "immutable-error-taxonomy",
})
M.PNG_ERROR_CODES = error_proxy

function M.png_error_codes()
    local copy = {}
    for index, code in ipairs(ERROR_CODES) do copy[index] = code end
    return copy
end

local PngError = {}
PngError.__index = PngError

function PngError.new(code)
    return setmetatable({code = code, message = code}, PngError)
end

function PngError:__tostring()
    return self.code
end

M.PngError = PngError

local function fail(code)
    error(PngError.new(code), 0)
end

local function validate_max_pixels(value)
    if value == nil then return M.PNG_MAX_PIXELS end
    if type(value) ~= "number" or value ~= value or value == math.huge
        or value == -math.huge or value % 1 ~= 0 or value <= 0
        or value > M.PNG_MAX_PIXELS
    then
        fail("invalid-max-pixels")
    end
    return value
end

local PngCodec = {}
PngCodec.__index = PngCodec

function PngCodec.new(options)
    options = options or {}
    local requested = options.max_pixels
    return setmetatable({
        mime_type = M.mime_type,
        max_pixels = validate_max_pixels(requested),
    }, PngCodec)
end

function PngCodec.encode(_, pixels)
    return M.encode_png(pixels)
end

function PngCodec:decode(data)
    return M._decode_with_limit(data, self.max_pixels)
end

M.PngCodec = PngCodec

local SIGNATURE = "\137PNG\r\n\26\n"
local ADLER_MOD = 65521
local BYTE_CHUNK = 4096

function M.adler32(data)
    local a = 1
    local b = 0
    for first = 1, #data, 5552 do
        local last = math.min(first + 5551, #data)
        for index = first, last do
            a = a + data:byte(index)
            b = b + a
        end
        a = a % ADLER_MOD
        b = b % ADLER_MOD
    end
    return ((b << 16) | a) & 0xffffffff
end

local function paeth(a, b, c)
    local prediction = a + b - c
    local distance_a = math.abs(prediction - a)
    local distance_b = math.abs(prediction - b)
    local distance_c = math.abs(prediction - c)
    if distance_a <= distance_b and distance_a <= distance_c then return a end
    if distance_b <= distance_c then return b end
    return c
end

local function byte_string_from_reader(length, reader)
    local chunks = {}
    local values = {}
    for index = 1, length do
        values[#values + 1] = reader(index)
        if #values == BYTE_CHUNK then
            chunks[#chunks + 1] = string.char(table.unpack(values))
            values = {}
        end
    end
    if #values > 0 then chunks[#chunks + 1] = string.char(table.unpack(values)) end
    return table.concat(chunks)
end

local function filter_row(filter, row, prior, bytes_per_pixel)
    local score = 0
    local filtered = byte_string_from_reader(#row, function(index)
        local current = row:byte(index)
        local left = index > bytes_per_pixel and row:byte(index - bytes_per_pixel) or 0
        local above = prior:byte(index) or 0
        local above_left = index > bytes_per_pixel and prior:byte(index - bytes_per_pixel) or 0
        local predicted = 0
        if filter == 1 then
            predicted = left
        elseif filter == 2 then
            predicted = above
        elseif filter == 3 then
            predicted = (left + above) // 2
        elseif filter == 4 then
            predicted = paeth(left, above, above_left)
        end
        local residual = (current - predicted) & 0xff
        score = score + (residual < 128 and residual or 256 - residual)
        return residual
    end)
    return filtered, score
end

local function choose_filter(row, prior, bytes_per_pixel)
    local best_filter = 0
    local best_row
    local best_score
    for filter = 0, 4 do
        local candidate, score = filter_row(filter, row, prior, bytes_per_pixel)
        if best_score == nil or score < best_score then
            best_filter = filter
            best_row = candidate
            best_score = score
        end
    end
    return best_filter, best_row
end

local function undo_filter(filter, filtered, prior, bytes_per_pixel)
    local row = {}
    for index = 1, #filtered do
        local left = index > bytes_per_pixel and row[index - bytes_per_pixel] or 0
        local above = prior:byte(index) or 0
        local above_left = index > bytes_per_pixel and prior:byte(index - bytes_per_pixel) or 0
        local predicted = 0
        if filter == 1 then
            predicted = left
        elseif filter == 2 then
            predicted = above
        elseif filter == 3 then
            predicted = (left + above) // 2
        elseif filter == 4 then
            predicted = paeth(left, above, above_left)
        end
        row[index] = (filtered:byte(index) + predicted) & 0xff
    end
    return byte_string_from_reader(#row, function(index) return row[index] end)
end

local function u32(value)
    return string.pack(">I4", value & 0xffffffff)
end

local function make_chunk(kind, payload)
    local checksum = zip.crc32(kind)
    checksum = zip.crc32(payload, checksum)
    return u32(#payload) .. kind .. payload .. u32(checksum)
end

local function valid_dimension(value)
    return type(value) == "number" and value == value and value ~= math.huge
        and value ~= -math.huge and value % 1 == 0 and value > 0
end

local function data_is_compact(data)
    return type(data) == "table" and getmetatable(data) == "pixel-byte-buffer"
end

function M.encode_png(pixels)
    if type(pixels) ~= "table" or not valid_dimension(pixels.width)
        or not valid_dimension(pixels.height)
        or pixels.width > M.PNG_MAX_DIMENSION or pixels.height > M.PNG_MAX_DIMENSION
    then
        fail("invalid-image-dimensions")
    end
    local width = pixels.width
    local height = pixels.height
    local pixel_count = width * height
    if pixel_count > M.PNG_MAX_PIXELS then fail("invalid-image-dimensions") end
    local expected_length = pixel_count * 4
    local data = pixels.data
    if type(data) ~= "table" or #data ~= expected_length then
        fail("invalid-pixel-data-length")
    end
    if not data_is_compact(data) then
        for key in pairs(data) do
            if type(key) == "number" and key % 1 == 0
                and (key < 1 or key > expected_length)
            then
                fail("invalid-pixel-data-length")
            end
        end
    end

    local stride = width * 4
    local filtered_parts = {}
    local prior = string.rep("\0", stride)
    for row_index = 0, height - 1 do
        local first = row_index * stride + 1
        local row = byte_string_from_reader(stride, function(offset)
            local value = data[first + offset - 1]
            if type(value) ~= "number" or value ~= value or value % 1 ~= 0
                or value < 0 or value > 255
            then
                fail("invalid-pixel-data-length")
            end
            return value
        end)
        local filter, encoded_row = choose_filter(row, prior, 4)
        filtered_parts[#filtered_parts + 1] = string.char(filter) .. encoded_row
        prior = row
    end
    local filtered = table.concat(filtered_parts)

    local ihdr = string.pack(">I4I4BBBBB", width, height, 8, 6, 0, 0, 0)
    local deflated = zip.raw_deflate(filtered)
    local zlib = "\120\156" .. deflated .. u32(M.adler32(filtered))
    return SIGNATURE
        .. make_chunk("IHDR", ihdr)
        .. make_chunk("IDAT", zlib)
        .. make_chunk("IEND", "")
end

local function valid_chunk_type(kind)
    if #kind ~= 4 or (kind:byte(3) & 0x20) ~= 0 then return false end
    for index = 1, 4 do
        local value = kind:byte(index)
        if not ((value >= 0x41 and value <= 0x5a)
            or (value >= 0x61 and value <= 0x7a))
        then
            return false
        end
    end
    return true
end

local function decode_with_limit(data, limit)
    if type(data) ~= "string" or #data < #SIGNATURE then fail("file-too-short") end
    if data:sub(1, #SIGNATURE) ~= SIGNATURE then fail("invalid-signature") end

    local width = 0
    local height = 0
    local colour_type = 0
    local saw_ihdr = false
    local saw_iend = false
    local saw_plte = false
    local saw_trns = false
    local in_idat = false
    local idat_ended = false
    local transparent_grey
    local transparent_rgb
    local idat_parts = {}
    local idat_length = 0

    local position = #SIGNATURE + 1
    while position <= #data do
        if #data - position + 1 < 8 then fail("truncated-chunk") end
        local length = string.unpack(">I4", data, position)
        if length > #data - position - 11 then fail("truncated-chunk") end
        local kind = data:sub(position + 4, position + 7)
        local data_start = position + 8
        local data_end = data_start + length - 1
        local payload = length == 0 and "" or data:sub(data_start, data_end)
        if not valid_chunk_type(kind) then fail("invalid-chunk-type") end
        local declared_crc = string.unpack(">I4", data, data_end + 1)
        local actual_crc = zip.crc32(kind)
        actual_crc = zip.crc32(payload, actual_crc)
        if actual_crc ~= declared_crc then fail("chunk-crc-mismatch") end
        if not saw_ihdr and kind ~= "IHDR" then fail("chunk-before-ihdr") end

        if kind == "IHDR" then
            if saw_ihdr then fail("duplicate-ihdr") end
            if length ~= 13 then fail("invalid-ihdr-length") end
            width, height = string.unpack(">I4I4", payload)
            local bit_depth, compression, filter_method, interlace
            bit_depth, colour_type, compression, filter_method, interlace =
                string.unpack("BBBBB", payload, 9)
            if width == 0 or height == 0 then fail("invalid-dimensions") end
            if width > M.PNG_MAX_DIMENSION or height > M.PNG_MAX_DIMENSION then
                fail("dimension-limit")
            end
            if width * height > limit then fail("pixel-limit") end
            if compression ~= 0 or filter_method ~= 0 or interlace ~= 0 then
                fail("unsupported-feature")
            end
            if bit_depth ~= 8 or colour_type == 3
                or (colour_type ~= 0 and colour_type ~= 2
                    and colour_type ~= 4 and colour_type ~= 6)
            then
                fail("unsupported-feature")
            end
            saw_ihdr = true
        elseif kind == "PLTE" then
            if saw_plte or #idat_parts > 0 or saw_trns
                or (colour_type ~= 2 and colour_type ~= 6)
                or length < 3 or length > 768 or length % 3 ~= 0
            then
                fail("invalid-plte")
            end
            saw_plte = true
        elseif kind == "tRNS" then
            if saw_trns or #idat_parts > 0 then fail("invalid-trns") end
            if colour_type == 0 then
                if length ~= 2 then fail("invalid-trns") end
                transparent_grey = string.unpack(">I2", payload)
                if transparent_grey > 255 then fail("invalid-trns") end
            elseif colour_type == 2 then
                if length ~= 6 then fail("invalid-trns") end
                local red, green, blue = string.unpack(">I2I2I2", payload)
                if red > 255 or green > 255 or blue > 255 then fail("invalid-trns") end
                transparent_rgb = {red, green, blue}
            else
                fail("invalid-trns")
            end
            saw_trns = true
        elseif kind == "IDAT" then
            if idat_ended then fail("nonconsecutive-idat") end
            idat_length = idat_length + length
            if idat_length > #data then fail("truncated-chunk") end
            idat_parts[#idat_parts + 1] = payload
            in_idat = true
        elseif kind == "IEND" then
            if length ~= 0 then fail("invalid-iend") end
            if position + 12 ~= #data + 1 then fail("trailing-data") end
            saw_iend = true
        elseif kind == "acTL" or kind == "fcTL" or kind == "fdAT" then
            fail("unsupported-feature")
        elseif (kind:byte(1) & 0x20) == 0 then
            fail("unknown-critical-chunk")
        end

        if kind ~= "IDAT" and in_idat then
            in_idat = false
            idat_ended = true
        end
        position = position + 12 + length
    end

    if not saw_ihdr or not saw_iend or #idat_parts == 0 then
        fail("missing-required-chunk")
    end
    local zlib = table.concat(idat_parts)
    if #zlib < 6 then fail("invalid-zlib-header") end
    local cmf = zlib:byte(1)
    local flg = zlib:byte(2)
    if (cmf & 0x0f) ~= 8 or (cmf >> 4) > 7
        or ((cmf << 8) | flg) % 31 ~= 0
    then
        fail("invalid-zlib-header")
    end
    if (flg & 0x20) ~= 0 then fail("preset-dictionary") end

    local channels = ({[0] = 1, [2] = 3, [4] = 2, [6] = 4})[colour_type]
    local stride = width * channels
    local expected = height * (stride + 1)
    local deflate = zlib:sub(3, -5)
    local inflated, inflate_error = zip.raw_inflate_counted(deflate, expected)
    if inflated == nil then
        if inflate_error == "output-limit-exceeded" then
            fail("inflated-length-mismatch")
        end
        fail("inflate-failed")
    end
    if #inflated.output ~= expected then fail("inflated-length-mismatch") end
    if inflated.bytes_consumed ~= #deflate then fail("idat-cavity") end
    local declared_adler = string.unpack(">I4", zlib, #zlib - 3)
    if M.adler32(inflated.output) ~= declared_adler then fail("adler-mismatch") end

    local row_size = stride + 1
    for row_index = 0, height - 1 do
        if inflated.output:byte(row_index * row_size + 1) > 4 then
            fail("invalid-filter")
        end
    end

    local rgba_rows = {}
    local prior = string.rep("\0", stride)
    for row_index = 0, height - 1 do
        local source = row_index * row_size + 1
        local filter = inflated.output:byte(source)
        local filtered = inflated.output:sub(source + 1, source + stride)
        local row = undo_filter(filter, filtered, prior, channels)
        if channels == 4 then
            rgba_rows[#rgba_rows + 1] = row
        else
            rgba_rows[#rgba_rows + 1] = byte_string_from_reader(width * 4, function(index)
                local pixel = (index - 1) // 4
                local channel = (index - 1) % 4
                local source_index = pixel * channels + 1
                if channels == 1 then
                    local grey = row:byte(source_index)
                    if channel < 3 then return grey end
                    return transparent_grey == grey and 0 or 255
                elseif channels == 2 then
                    if channel < 3 then return row:byte(source_index) end
                    return row:byte(source_index + 1)
                end
                local red = row:byte(source_index)
                local green = row:byte(source_index + 1)
                local blue = row:byte(source_index + 2)
                if channel == 0 then return red end
                if channel == 1 then return green end
                if channel == 2 then return blue end
                local transparent = transparent_rgb ~= nil
                    and transparent_rgb[1] == red
                    and transparent_rgb[2] == green
                    and transparent_rgb[3] == blue
                return transparent and 0 or 255
            end)
        end
        prior = row
    end
    return pc.from_byte_chunks(width, height, rgba_rows)
end

M._decode_with_limit = decode_with_limit

function M.decode_png(data, options)
    options = options or {}
    return decode_with_limit(data, validate_max_pixels(options.max_pixels))
end

M.codec = PngCodec.new()

return M

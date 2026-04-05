-- image_codec_ppm — IC02: PPM (Portable Pixmap) image encoder and decoder
--
-- # What Is PPM?
--
-- PPM is the "Portable Pixmap" format from the Netpbm family of tools.
-- It is arguably the simplest possible image format that can store full colour:
-- a plain-text header followed immediately by raw RGB bytes.
--
-- There are two variants:
--
--   P3  — ASCII text values (human-readable but large)
--   P6  — Raw binary values (compact; this module uses P6)
--
-- # File Structure (P6 / binary PPM)
--
--   ┌─────────────────────────────────────────────────────────────┐
--   │  Header (ASCII text, newline-separated)                     │
--   │    Line 1: "P6"                                             │
--   │    Line 2: "<width> <height>"                               │
--   │    Line 3: "<max_value>"  (always 255 in our implementation)│
--   │    (a single whitespace byte here ends the header)         │
--   │  Pixel data (raw binary, 3 bytes per pixel: R, G, B)       │
--   └─────────────────────────────────────────────────────────────┘
--
-- # Key Differences From BMP
--
-- 1. Alpha is NOT stored.  PPM is an RGB-only format.
--    When encoding, alpha is silently dropped.
--    When decoding, alpha is set to 255 (fully opaque) for every pixel.
--
-- 2. The header is plain ASCII text.
--    This makes PPM easy to inspect with a text editor — you can see the
--    dimensions of an image without a hex dump.
--
-- 3. No padding.  Rows are exactly width * 3 bytes, no alignment required.
--
-- 4. Byte order is always R, G, B (no BGR reversal like BMP).
--
-- 5. Comments.  PPM files may contain lines starting with '#' between header
--    fields.  Our decoder skips comment lines when parsing the header.
--
-- # Header Parsing Strategy
--
-- We scan the binary string for the first three whitespace-delimited tokens
-- (magic, width, height) and one more token (max_value), skipping '#' comment
-- lines along the way.  The pixel data begins immediately after the single
-- whitespace character that terminates the last header token.
--
-- This handles files with:
--   - Unix (LF), Windows (CRLF), or old Mac (CR) line endings
--   - Multiple spaces or tabs between tokens
--   - Comment lines starting with '#'
--
-- # Coordinate System
--
-- Same as pixel-container: (0, 0) = top-left, rows top-to-bottom.
-- The first row of pixel data corresponds to y = 0.

local pc = require("coding_adventures.pixel_container")

local M = {}

M.VERSION   = "0.1.0"
M.mime_type = "image/x-portable-pixmap"

-- ---------------------------------------------------------------------------
-- encode_ppm: PixelContainer → binary string
-- ---------------------------------------------------------------------------

--- Encode a PixelContainer into a P6 PPM binary string.
--
-- Alpha is dropped: only R, G, B bytes are written per pixel.
-- The max channel value is always 255.
--
-- @param c  table  pixel container
-- @return string  raw PPM file bytes (ASCII header + raw RGB pixels)
function M.encode_ppm(c)
    local w = c.width
    local h = c.height

    -- -------------------------------------------------------------------------
    -- Build the ASCII header.
    -- Format: "P6\n<width> <height>\n255\n"
    -- -------------------------------------------------------------------------
    -- A single newline after "255" ends the header.  The very next byte is
    -- the first Red channel byte of pixel (0, 0).
    local header = string.format("P6\n%d %d\n255\n", w, h)

    -- -------------------------------------------------------------------------
    -- Build pixel data: R, G, B (no alpha) in row-major order.
    -- -------------------------------------------------------------------------
    local rows = {}
    for y = 0, h - 1 do
        local row_parts = {}
        for x = 0, w - 1 do
            local r, g, b, _a = pc.pixel_at(c, x, y)
            -- string.char produces a 3-byte string: R byte, G byte, B byte.
            row_parts[#row_parts + 1] = string.char(r, g, b)
        end
        rows[#rows + 1] = table.concat(row_parts)
    end

    return header .. table.concat(rows)
end

-- ---------------------------------------------------------------------------
-- Internal: header token scanner
-- ---------------------------------------------------------------------------

--- Scan the string `s` from position `pos` (1-based) and return the next
-- non-whitespace, non-comment token as a string, plus the position after it.
--
-- Skips:
--   - whitespace (space, tab, CR, LF)
--   - comment lines: any '#' character causes the rest of that line to be
--     skipped (up to and including the following LF or CR)
--
-- @param s    string  the PPM file data
-- @param pos  number  1-based starting position
-- @return string, number  token text, position after the token
-- @error  string if end-of-string is reached before a token is found
local function next_token(s, pos)
    local n = #s
    -- Skip whitespace and comment lines
    while pos <= n do
        local c = string.byte(s, pos)
        if c == 0x23 then
            -- '#' — skip until end of line (LF = 0x0A)
            while pos <= n and string.byte(s, pos) ~= 0x0A do
                pos = pos + 1
            end
            -- Skip the LF itself
            pos = pos + 1
        elseif c == 0x20 or c == 0x09 or c == 0x0A or c == 0x0D then
            -- Space, Tab, LF, CR — skip whitespace
            pos = pos + 1
        else
            -- Found start of a token
            break
        end
    end

    if pos > n then
        error("decode_ppm: unexpected end of header")
    end

    -- Collect token characters until whitespace or end-of-string
    local start = pos
    while pos <= n do
        local c = string.byte(s, pos)
        if c == 0x20 or c == 0x09 or c == 0x0A or c == 0x0D then
            break
        end
        pos = pos + 1
    end

    return string.sub(s, start, pos - 1), pos
end

-- ---------------------------------------------------------------------------
-- decode_ppm: binary string → PixelContainer
-- ---------------------------------------------------------------------------

--- Decode a P6 PPM binary string into a PixelContainer.
--
-- Sets alpha to 255 for every pixel (PPM has no alpha channel).
-- Handles comment lines (#) in the header.
-- Supports LF, CRLF, and CR line endings.
--
-- @param data  string  raw PPM file bytes
-- @return table  pixel container
-- @error  string if the data is not a valid P6 PPM file
function M.decode_ppm(data)
    if type(data) ~= "string" then
        error("decode_ppm: expected string, got " .. type(data))
    end
    if #data < 7 then
        error("decode_ppm: data too short")
    end

    -- -------------------------------------------------------------------------
    -- Parse header: magic, width, height, max_value
    -- -------------------------------------------------------------------------
    local pos = 1

    -- Token 1: magic ("P6")
    local magic
    magic, pos = next_token(data, pos)
    if magic ~= "P6" then
        error("decode_ppm: unsupported format '" .. magic .. "' (expected P6)")
    end

    -- Token 2: width
    local width_str
    width_str, pos = next_token(data, pos)
    local width = tonumber(width_str)
    if not width or width < 1 then
        error("decode_ppm: invalid width '" .. width_str .. "'")
    end

    -- Token 3: height
    local height_str
    height_str, pos = next_token(data, pos)
    local height = tonumber(height_str)
    if not height or height < 1 then
        error("decode_ppm: invalid height '" .. height_str .. "'")
    end

    local MAX_DIMENSION = 16384
    if width > MAX_DIMENSION or height > MAX_DIMENSION then
        error("decode_ppm: image dimensions too large")
    end

    -- Token 4: max value
    local maxval_str
    maxval_str, pos = next_token(data, pos)
    local maxval = tonumber(maxval_str)
    if not maxval or maxval ~= 255 then
        error("decode_ppm: only maxval=255 is supported (got '" .. maxval_str .. "')")
    end

    -- After the last header token there must be exactly ONE whitespace byte
    -- (the spec says "a single whitespace character").  We already advanced
    -- `pos` to the first character AFTER the maxval token — which is that
    -- single whitespace.  Advance past it.
    pos = pos + 1  -- skip the one mandatory whitespace after maxval

    -- -------------------------------------------------------------------------
    -- Validate pixel data length
    -- -------------------------------------------------------------------------
    local expected_bytes = width * height * 3
    local available = #data - pos + 1
    if available < expected_bytes then
        error(string.format(
            "decode_ppm: pixel data too short (need %d bytes, got %d)",
            expected_bytes, available))
    end

    -- -------------------------------------------------------------------------
    -- Read pixels: R, G, B per pixel; set A = 255
    -- -------------------------------------------------------------------------
    local c = pc.new(width, height)
    for y = 0, height - 1 do
        for x = 0, width - 1 do
            local r = string.byte(data, pos)
            local g = string.byte(data, pos + 1)
            local b = string.byte(data, pos + 2)
            pc.set_pixel(c, x, y, r, g, b, 255)
            pos = pos + 3
        end
    end

    return c
end

-- ---------------------------------------------------------------------------
-- Codec table (conforms to the ImageCodec interface)
-- ---------------------------------------------------------------------------

M.codec = {
    mime_type = M.mime_type,
    encode    = M.encode_ppm,
    decode    = M.decode_ppm,
}

return M

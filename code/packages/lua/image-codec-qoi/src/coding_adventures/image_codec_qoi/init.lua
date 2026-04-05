-- image_codec_qoi — IC03: QOI (Quite OK Image) encoder and decoder
--
-- # What Is QOI?
--
-- QOI (Quite OK Image format) was designed by Dominic Szablewski in 2021.
-- It is a lossless, streaming image codec that achieves near-PNG compression
-- ratios with much simpler implementation logic — the reference C implementation
-- is only ~300 lines.
--
-- QOI works by describing each pixel as one of six possible operations:
--
--   1. QOI_OP_RGB    — raw 24-bit RGB values (no alpha change)
--   2. QOI_OP_RGBA   — raw 32-bit RGBA values
--   3. QOI_OP_INDEX  — reference to a previously-seen pixel from a 64-entry hash table
--   4. QOI_OP_DIFF   — small delta from the previous pixel (r/g/b each in ±1..2)
--   5. QOI_OP_LUMA   — medium delta: green ±32, r/b relative to green ±8
--   6. QOI_OP_RUN    — repeat the previous pixel N times (N in 1..62)
--
-- # File Structure
--
--   ┌──────────────────────────────────────────────────────────────┐
--   │  Header (14 bytes, big-endian)                               │
--   │  Chunk stream (variable length)                              │
--   │  End marker (8 bytes: 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x01)│
--   └──────────────────────────────────────────────────────────────┘
--
-- # Header (14 bytes, big-endian)
--
--   Offset  Size  Field      Value
--   ------  ----  ---------  ------------------------------------------------
--   0       4     magic      0x716F6966  = "qoif"
--   4       4     width      image width in pixels (uint32 BE)
--   8       4     height     image height in pixels (uint32 BE)
--   12      1     channels   3 = RGB, 4 = RGBA
--   13      1     colorspace 0 = sRGB with linear alpha, 1 = all linear
--                            (informational only — we always write 0)
--
-- # The 64-Entry Hash Table
--
-- QOI maintains a "seen pixels" array of 64 RGBA slots.  Every pixel processed
-- is stored at index:
--
--   hash = (r * 3 + g * 5 + b * 7 + a * 11) % 64
--
-- If the current pixel matches the pixel at that hash slot (all four channels),
-- we emit QOI_OP_INDEX instead of raw data.
--
-- # Chunk Encoding
--
-- Each chunk starts with a 1-byte or 2-byte tag:
--
--   QOI_OP_RGB   = 0b11111110  (0xFE)  tag byte + 3 raw bytes R, G, B
--   QOI_OP_RGBA  = 0b11111111  (0xFF)  tag byte + 4 raw bytes R, G, B, A
--   QOI_OP_INDEX = 0b00xxxxxx  (top 2 bits = 00)  6-bit index into hash table
--   QOI_OP_DIFF  = 0b01xxxxxx  (top 2 bits = 01)  dr+2, dg+2, db+2 packed in 6 bits
--   QOI_OP_LUMA  = 0b10xxxxxx  (top 2 bits = 10)  dg+32 in 6 bits; second byte = dr-dg+8, db-dg+8
--   QOI_OP_RUN   = 0b11xxxxxx  (top 2 bits = 11, value != 0xFE/0xFF)  run of (xxxxxx+1) pixels
--
-- # Delta Wrapping (DIFF and LUMA)
--
-- Deltas must wrap around the GF(256) byte field — i.e., if the previous red
-- was 2 and the current red is 254, the delta is 254 - 2 = 252, but with
-- wrapping we interpret 252 as -4 (because 252 - 256 = -4).
--
-- The correct wrapping formula for a channel delta dr = (cur - prev) is:
--
--   -- In Lua 5.4, `&` is bitwise AND, `//` is integer division
--   dr_wrapped = ((cur - prev) & 0xFF)
--   -- Then map the unsigned byte [0..255] into a signed value [-128..127]:
--   if dr_wrapped >= 128 then dr_wrapped = dr_wrapped - 256 end
--
-- This is equivalent to the C `int8_t` cast.
--
-- # Signed Wrap Helper
--
-- For repeated use we define wrap_delta(cur, prev) → signed integer in [-128, 127].
--
-- # QOI_OP_DIFF fits when: dr, dg, db all in [-2, 1]
-- # QOI_OP_LUMA fits when: dg in [-32, 31] AND (dr - dg) in [-8, 7] AND (db - dg) in [-8, 7]
--
-- # End Marker
--
-- The stream ends with the 8-byte sequence: 00 00 00 00 00 00 00 01
-- These are all zeros followed by a single 1 bit.  This can never appear as
-- a valid sequence of QOI ops, so it unambiguously marks the file end.
--
-- # string.pack / string.unpack
--
-- Header uses:
--   ">I4I4I4BB"  = big-endian uint32 x3, then two uint8 bytes
-- Actually the magic is a 4-byte raw string, so we use:
--   ">c4I4I4BB"  = 4-byte raw string, uint32 BE, uint32 BE, two bytes

local pc = require("coding_adventures.pixel_container")

local M = {}

M.VERSION   = "0.1.0"
M.mime_type = "image/qoi"

-- QOI magic bytes: the ASCII string "qoif"
local QOI_MAGIC = "qoif"

-- The 8-byte end-of-stream marker
local QOI_END_MARKER = string.char(0, 0, 0, 0, 0, 0, 0, 1)

-- Op tag values (1-byte prefix)
local QOI_OP_RGB   = 0xFE  -- 11111110
local QOI_OP_RGBA  = 0xFF  -- 11111111
-- Tag masks for the 2-bit prefix
local TAG_INDEX = 0x00  -- 00xxxxxx
local TAG_DIFF  = 0x40  -- 01xxxxxx
local TAG_LUMA  = 0x80  -- 10xxxxxx
local TAG_RUN   = 0xC0  -- 11xxxxxx
local TAG_MASK  = 0xC0  -- mask to extract top 2 bits

-- ---------------------------------------------------------------------------
-- Hash function for the 64-entry seen-pixels table
-- ---------------------------------------------------------------------------

--- Compute the QOI hash index for an RGBA pixel.
--
-- The hash maps each pixel to one of 64 slots.  Pixels with different RGBA
-- values can collide (hash table, not a set), so we always verify on read.
--
-- Formula: (r*3 + g*5 + b*7 + a*11) % 64
--
-- The coefficients (3, 5, 7, 11) are small odd primes chosen to spread the
-- distribution across the 64 slots.
--
-- @param r  number  red channel (0–255)
-- @param g  number  green channel (0–255)
-- @param b  number  blue channel (0–255)
-- @param a  number  alpha channel (0–255)
-- @return number  index in [0, 63]
local function hash_pixel(r, g, b, a)
    return (r * 3 + g * 5 + b * 7 + a * 11) % 64
end

-- ---------------------------------------------------------------------------
-- Signed-byte delta helper
-- ---------------------------------------------------------------------------

--- Compute the signed delta between two channel values, with 8-bit wrapping.
--
-- In GF(256) all arithmetic wraps mod 256.  A raw subtraction (cur - prev)
-- gives a value in [-(255), 255].  We want the wrapped signed value in
-- [-128, 127].
--
-- Step 1: (cur - prev) & 0xFF  →  maps to [0, 255] (unsigned 8-bit)
-- Step 2: subtract 256 if >= 128  →  maps to [-128, 127] (signed 8-bit)
--
-- Example: cur=2, prev=254  →  (2-254) & 0xFF = (-252) & 0xFF = 4  →  4
--          cur=254, prev=2  →  (254-2) & 0xFF = 252  →  252-256 = -4
--
-- @param cur   number  current channel value (0–255)
-- @param prev  number  previous channel value (0–255)
-- @return number  signed delta in [-128, 127]
local function wrap_delta(cur, prev)
    -- Lua 5.4: `&` is bitwise AND on integers.
    -- We force integer context with `//1` just to be safe.
    local d = (cur - prev) & 0xFF
    if d >= 128 then d = d - 256 end
    return d
end

-- ---------------------------------------------------------------------------
-- encode_qoi: PixelContainer → binary string
-- ---------------------------------------------------------------------------

--- Encode a PixelContainer into a QOI binary string.
--
-- The encoder tries each op in priority order:
--   1. RUN     — if pixel equals previous pixel (repeat)
--   2. INDEX   — if pixel is in the seen-pixels table at its hash slot
--   3. DIFF    — if all three RGB deltas fit in [-2, 1] (alpha unchanged)
--   4. LUMA    — if green delta in [-32,31] and dr-dg, db-dg in [-8,7] (alpha unchanged)
--   5. QOI_OP_RGB  — if only RGB changed (alpha unchanged)
--   6. QOI_OP_RGBA — fallback (alpha also changed)
--
-- @param c  table  pixel container
-- @return string  raw QOI file bytes
function M.encode_qoi(c)
    local w = c.width
    local h = c.height

    -- -------------------------------------------------------------------------
    -- Build the 14-byte big-endian header.
    -- ">c4I4I4BB" = 4-byte raw string, two uint32 BE, two uint8
    -- channels = 4 (RGBA), colorspace = 0 (sRGB)
    -- -------------------------------------------------------------------------
    local header = string.pack(">c4I4I4BB",
        QOI_MAGIC,
        w,
        h,
        4,   -- channels = RGBA
        0    -- colorspace = sRGB
    )

    -- -------------------------------------------------------------------------
    -- Encoder state
    -- -------------------------------------------------------------------------

    -- The 64-slot seen-pixels table.  Each entry is a table {r, g, b, a}.
    -- Initialised to all zeros (RGBA = 0, 0, 0, 0).
    local seen = {}
    for i = 0, 63 do
        seen[i] = {0, 0, 0, 0}
    end

    -- Previous pixel starts as RGBA = (0, 0, 0, 255) per the spec.
    local prev_r, prev_g, prev_b, prev_a = 0, 0, 0, 255

    -- Output chunk accumulator
    local chunks = {}

    -- Current run length (consecutive identical pixels)
    local run = 0

    -- -------------------------------------------------------------------------
    -- Helper: flush a pending run (if any)
    -- -------------------------------------------------------------------------
    local function flush_run()
        if run > 0 then
            -- QOI_OP_RUN: tag = 0b11xxxxxx, where xxxxxx = run - 1
            -- Run length is stored as (run - 1) to allow encoding 1..62
            -- (values 63 and 64 are reserved for QOI_OP_RGB and QOI_OP_RGBA).
            chunks[#chunks + 1] = string.char(TAG_RUN | (run - 1))
            run = 0
        end
    end

    -- -------------------------------------------------------------------------
    -- Main encoding loop
    -- -------------------------------------------------------------------------
    for y = 0, h - 1 do
        for x = 0, w - 1 do
            local r, g, b, a = pc.pixel_at(c, x, y)

            -- -----------------------------------------------------------------
            -- Op 1: QOI_OP_RUN
            -- If the current pixel is identical to the previous, extend the run.
            -- The maximum run length is 62 (not 63 or 64, which are reserved).
            -- -----------------------------------------------------------------
            if r == prev_r and g == prev_g and b == prev_b and a == prev_a then
                run = run + 1
                if run == 62 then
                    flush_run()
                end
            else
                -- We have a new pixel.  Flush any pending run first.
                flush_run()

                -- -------------------------------------------------------------
                -- Op 2: QOI_OP_INDEX
                -- Check if the current pixel is in the seen-pixels table.
                -- The seen table is updated for the current pixel by every
                -- non-INDEX op (same update strategy as the decoder).
                -- For INDEX the value at seen[idx] is already correct.
                -- -------------------------------------------------------------
                local idx = hash_pixel(r, g, b, a)
                local s = seen[idx]
                if s[1] == r and s[2] == g and s[3] == b and s[4] == a then
                    -- Match!  Emit a 1-byte index reference.
                    -- Tag: 0b00xxxxxx where xxxxxx = idx
                    chunks[#chunks + 1] = string.char(TAG_INDEX | idx)
                else
                    -- No index match.  Update seen for the current pixel, then
                    -- try delta operations (alpha must be unchanged for DIFF/LUMA).
                    seen[idx] = {r, g, b, a}

                    local dr = wrap_delta(r, prev_r)
                    local dg = wrap_delta(g, prev_g)
                    local db = wrap_delta(b, prev_b)
                    local da = wrap_delta(a, prev_a)

                    -- ---------------------------------------------------------
                    -- Op 3: QOI_OP_DIFF
                    -- Condition: alpha unchanged AND dr, dg, db each in [-2, 1]
                    -- Encoding:  tag = 0b01drr dgg dbb
                    --   dr stored as (dr + 2), giving range [0, 3] in 2 bits
                    --   dg stored as (dg + 2), giving range [0, 3] in 2 bits
                    --   db stored as (db + 2), giving range [0, 3] in 2 bits
                    --   Packed into 6 bits: (dr+2)<<4 | (dg+2)<<2 | (db+2)
                    -- ---------------------------------------------------------
                    if da == 0
                        and dr >= -2 and dr <= 1
                        and dg >= -2 and dg <= 1
                        and db >= -2 and db <= 1
                    then
                        local byte = TAG_DIFF
                            | ((dr + 2) << 4)
                            | ((dg + 2) << 2)
                            | ((db + 2))
                        chunks[#chunks + 1] = string.char(byte)

                    -- ---------------------------------------------------------
                    -- Op 4: QOI_OP_LUMA
                    -- Condition: alpha unchanged AND
                    --   dg       in [-32, 31]
                    --   dr - dg  in [-8, 7]
                    --   db - dg  in [-8, 7]
                    --
                    -- Encoding (2 bytes):
                    --   Byte 1: 0b10gggggg  (tag + dg + 32 in 6 bits)
                    --   Byte 2: 0b rrrr bbbb
                    --     where rrrr = (dr - dg + 8) and bbbb = (db - dg + 8)
                    --
                    -- The "relative to green" trick exploits the fact that in
                    -- most images, colour channels change together (e.g. a grey
                    -- gradient has dr = dg = db).  By storing dr and db relative
                    -- to dg, we get smaller residuals and can use fewer bits.
                    -- ---------------------------------------------------------
                    elseif da == 0
                        and dg >= -32 and dg <= 31
                        and (dr - dg) >= -8 and (dr - dg) <= 7
                        and (db - dg) >= -8 and (db - dg) <= 7
                    then
                        local byte1 = TAG_LUMA | (dg + 32)
                        local byte2 = ((dr - dg + 8) << 4) | (db - dg + 8)
                        chunks[#chunks + 1] = string.char(byte1, byte2)

                    -- ---------------------------------------------------------
                    -- Op 5: QOI_OP_RGB
                    -- Alpha unchanged, but delta too large for DIFF/LUMA.
                    -- Emit 4 bytes: 0xFE, R, G, B
                    -- ---------------------------------------------------------
                    elseif da == 0 then
                        chunks[#chunks + 1] = string.char(QOI_OP_RGB, r, g, b)

                    -- ---------------------------------------------------------
                    -- Op 6: QOI_OP_RGBA
                    -- Alpha changed.  Emit 5 bytes: 0xFF, R, G, B, A
                    -- ---------------------------------------------------------
                    else
                        chunks[#chunks + 1] = string.char(QOI_OP_RGBA, r, g, b, a)
                    end
                end
            end

            prev_r, prev_g, prev_b, prev_a = r, g, b, a
        end
    end

    -- Flush the final run, if any.
    flush_run()

    return header .. table.concat(chunks) .. QOI_END_MARKER
end

-- ---------------------------------------------------------------------------
-- decode_qoi: binary string → PixelContainer
-- ---------------------------------------------------------------------------

--- Decode a QOI binary string into a PixelContainer.
--
-- Processes the chunk stream sequentially, maintaining the same state as the
-- encoder (previous pixel, seen-pixels table).  Stops when either:
--   - All width * height pixels have been decoded, OR
--   - The end marker (8 bytes of zeros + one byte of 0x01) is encountered.
--
-- @param data  string  raw QOI file bytes
-- @return table  pixel container
-- @error  string if the data is not a valid QOI file
function M.decode_qoi(data)
    if type(data) ~= "string" then
        error("decode_qoi: expected string, got " .. type(data))
    end
    if #data < 22 then
        -- Minimum valid QOI: 14-byte header + 1-byte chunk + 8-byte end marker
        error("decode_qoi: data too short (" .. #data .. " bytes)")
    end

    -- -------------------------------------------------------------------------
    -- Parse the 14-byte header
    -- -------------------------------------------------------------------------
    local magic, width, height, channels, colorspace =
        string.unpack(">c4I4I4BB", data, 1)

    if magic ~= QOI_MAGIC then
        error("decode_qoi: invalid magic '" .. magic .. "' (expected 'qoif')")
    end
    if width < 1 or height < 1 then
        error("decode_qoi: invalid dimensions " .. width .. "x" .. height)
    end

    local MAX_DIMENSION = 16384
    if width > MAX_DIMENSION or height > MAX_DIMENSION then
        error("decode_qoi: image dimensions too large")
    end

    if channels ~= 3 and channels ~= 4 then
        error("decode_qoi: unsupported channels " .. channels)
    end

    -- -------------------------------------------------------------------------
    -- Decoder state (mirrors the encoder state exactly)
    -- -------------------------------------------------------------------------

    -- 64-slot seen-pixels table, all zeros initially
    local seen = {}
    for i = 0, 63 do
        seen[i] = {0, 0, 0, 0}
    end

    -- Previous pixel starts at (0, 0, 0, 255)
    local prev_r, prev_g, prev_b, prev_a = 0, 0, 0, 255

    local c = pc.new(width, height)
    local total_pixels = width * height
    local pixels_written = 0

    -- Current position in the byte string (1-based, after the 14-byte header)
    local pos = 15  -- 14 + 1

    -- -------------------------------------------------------------------------
    -- Main decoding loop
    -- -------------------------------------------------------------------------
    while pixels_written < total_pixels do
        if pos > #data then
            error("decode_qoi: unexpected end of stream")
        end

        local b1 = string.byte(data, pos)
        pos = pos + 1

        local r, g, b, a

        if b1 == QOI_OP_RGBA then
            -- -----------------------------------------------------------------
            -- QOI_OP_RGBA: read 4 raw bytes
            -- -----------------------------------------------------------------
            if pos + 3 > #data then error("decode_qoi: truncated RGBA chunk") end
            r = string.byte(data, pos)
            g = string.byte(data, pos + 1)
            b = string.byte(data, pos + 2)
            a = string.byte(data, pos + 3)
            pos = pos + 4

        elseif b1 == QOI_OP_RGB then
            -- -----------------------------------------------------------------
            -- QOI_OP_RGB: read 3 raw bytes, keep previous alpha
            -- -----------------------------------------------------------------
            if pos + 2 > #data then error("decode_qoi: truncated RGB chunk") end
            r = string.byte(data, pos)
            g = string.byte(data, pos + 1)
            b = string.byte(data, pos + 2)
            a = prev_a
            pos = pos + 3

        else
            local tag = b1 & TAG_MASK

            if tag == TAG_INDEX then
                -- -------------------------------------------------------------
                -- QOI_OP_INDEX: look up the 6-bit index in the seen table
                -- -------------------------------------------------------------
                local idx = b1 & 0x3F
                local s = seen[idx]
                r, g, b, a = s[1], s[2], s[3], s[4]

            elseif tag == TAG_DIFF then
                -- -------------------------------------------------------------
                -- QOI_OP_DIFF: 2-bit deltas for R, G, B; alpha unchanged
                --
                -- Bit layout of the 6 payload bits (after the 2-bit tag):
                --   bits 5-4: dr+2   (2 bits)
                --   bits 3-2: dg+2   (2 bits)
                --   bits 1-0: db+2   (2 bits)
                --
                -- To decode: extract each 2-bit field and subtract 2 to get
                -- the signed delta.  Apply delta with byte wrapping.
                -- -------------------------------------------------------------
                local dr = ((b1 >> 4) & 0x03) - 2   -- bits 5-4, offset -2
                local dg = ((b1 >> 2) & 0x03) - 2   -- bits 3-2, offset -2
                local db = ( b1       & 0x03) - 2   -- bits 1-0, offset -2
                -- Apply deltas with modular arithmetic to wrap around 0–255
                r = (prev_r + dr) & 0xFF
                g = (prev_g + dg) & 0xFF
                b = (prev_b + db) & 0xFF
                a = prev_a

            elseif tag == TAG_LUMA then
                -- -------------------------------------------------------------
                -- QOI_OP_LUMA: larger deltas; second byte has dr-dg and db-dg
                --
                -- Byte 1 (b1): 0b10gggggg — dg + 32 in bits 5-0
                -- Byte 2 (b2): 0b rrrr bbbb
                --   upper nibble: dr - dg + 8
                --   lower nibble: db - dg + 8
                --
                -- To decode:
                --   dg = (b1 & 0x3F) - 32
                --   dr = ((b2 >> 4) & 0x0F) - 8 + dg
                --   db = ( b2       & 0x0F) - 8 + dg
                -- -------------------------------------------------------------
                if pos > #data then error("decode_qoi: truncated LUMA chunk") end
                local b2 = string.byte(data, pos)
                pos = pos + 1
                local dg = (b1 & 0x3F) - 32
                local dr = ((b2 >> 4) & 0x0F) - 8 + dg
                local db = ( b2       & 0x0F) - 8 + dg
                r = (prev_r + dr) & 0xFF
                g = (prev_g + dg) & 0xFF
                b = (prev_b + db) & 0xFF
                a = prev_a

            else  -- tag == TAG_RUN (0xC0)
                -- -------------------------------------------------------------
                -- QOI_OP_RUN: repeat previous pixel (run_length+1) times
                --
                -- The 6-bit payload encodes (run_length - 1), so:
                --   actual run = (b1 & 0x3F) + 1
                --
                -- The values 0x3E (62 in 6 bits, meaning run=63) and
                -- 0x3F (meaning run=64) are never used because those 1-byte
                -- values are reserved for QOI_OP_RGB (0xFE) and
                -- QOI_OP_RGBA (0xFF).  The maximum run is therefore 62.
                -- -------------------------------------------------------------
                local run_len = (b1 & 0x3F) + 1
                r, g, b, a = prev_r, prev_g, prev_b, prev_a
                -- Write all pixels in the run
                for _ = 1, run_len do
                    if pixels_written >= total_pixels then break end
                    local px_y = pixels_written // width
                    local px_x = pixels_written  % width
                    pc.set_pixel(c, px_x, px_y, r, g, b, a)
                    pixels_written = pixels_written + 1
                end
                -- Update seen table for the run pixel
                seen[hash_pixel(r, g, b, a)] = {r, g, b, a}
                prev_r, prev_g, prev_b, prev_a = r, g, b, a
                -- Skip the normal write at the bottom of the loop (already done)
                goto continue
            end
        end

        -- Update the seen-pixels table with the new pixel
        seen[hash_pixel(r, g, b, a)] = {r, g, b, a}

        -- Write the pixel to the container
        local px_y = pixels_written // width
        local px_x = pixels_written  % width
        pc.set_pixel(c, px_x, px_y, r, g, b, a)
        pixels_written = pixels_written + 1

        prev_r, prev_g, prev_b, prev_a = r, g, b, a

        ::continue::
    end

    return c
end

-- ---------------------------------------------------------------------------
-- Codec table (conforms to the ImageCodec interface)
-- ---------------------------------------------------------------------------

M.codec = {
    mime_type = M.mime_type,
    encode    = M.encode_qoi,
    decode    = M.decode_qoi,
}

return M

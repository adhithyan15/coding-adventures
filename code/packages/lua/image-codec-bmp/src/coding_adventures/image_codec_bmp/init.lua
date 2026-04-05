-- image_codec_bmp — IC01: BMP (bitmap) image encoder and decoder
--
-- # What Is BMP?
--
-- BMP (Windows Bitmap) is one of the simplest raster image file formats.
-- It was designed for Microsoft Windows in the 1980s and remains completely
-- uncompressed in its most common variant.  There are no compression tables,
-- no colour transforms, no entropy coding — just raw bytes with a short header.
--
-- That simplicity makes BMP the ideal first codec to implement: you can verify
-- correctness by opening the output in any image viewer.
--
-- # File Structure
--
-- A 32-bit RGBA BMP file has three sections:
--
--   ┌──────────────────────────────────────────────────────────────┐
--   │  BITMAPFILEHEADER  (14 bytes)                                │
--   │  BITMAPINFOHEADER  (40 bytes)  ← DIB header                 │
--   │  Pixel data        (width * |height| * 4 bytes)             │
--   └──────────────────────────────────────────────────────────────┘
--
-- # BITMAPFILEHEADER (14 bytes, little-endian)
--
--   Offset  Size  Field         Value
--   ------  ----  -----------   -------------------------------------------
--   0       2     bfType        0x4D42  = 'B','M'  (magic number)
--   2       4     bfSize        total file size in bytes
--   6       2     bfReserved1   0
--   8       2     bfReserved2   0
--   10      4     bfOffBits     byte offset to pixel data = 14 + 40 = 54
--
-- # BITMAPINFOHEADER (40 bytes, little-endian)
--
--   Offset  Size  Field             Meaning
--   ------  ----  ---------------   ----------------------------------------
--   0       4     biSize            header size = 40
--   4       4     biWidth           image width in pixels (signed, positive)
--   8       4     biHeight          image height — NEGATIVE for top-down rows
--   12      2     biPlanes          1 (always)
--   14      2     biBitCount        32 (bits per pixel: 8R + 8G + 8B + 8A)
--   16      4     biCompression     BI_BITFIELDS = 3 (needed for alpha channel)
--   20      4     biSizeImage       pixel data size in bytes (can be 0 for BI_RGB)
--   24      4     biXPelsPerMeter   0 (unspecified)
--   28      4     biYPelsPerMeter   0 (unspecified)
--   32      4     biClrUsed         0 (all colours used)
--   36      4     biClrImportant    0 (all colours important)
--
-- # Why Negative Height?
--
-- Positive biHeight means the first row in the file is the BOTTOM row
-- (legacy Windows convention — images were originally stored bottom-up).
-- Negative biHeight means the first row in the file is the TOP row
-- (top-down / normal screen order).
--
-- We always write negative heights so our row order matches the pixel
-- container's (0,0) = top-left convention.
--
-- # BI_BITFIELDS and Alpha
--
-- When biCompression = BI_BITFIELDS (3) and biBitCount = 32, the file may
-- include extra colour-mask DWORDs after the header.  For our purposes we use
-- the BITMAPV4HEADER-style interpretation: bytes in each pixel are BGRA order.
--
-- Actually, the simplest compatible approach for 32-bit is:
--   biCompression = BI_RGB (0)
--   biBitCount = 32
-- and store BGRA bytes in the pixel data.  Most viewers will honour the alpha.
-- We use this simpler BI_RGB approach.
--
-- # Pixel Data (BI_RGB, 32 bpp, top-down)
--
-- Each pixel is 4 bytes: Blue, Green, Red, Alpha  (BGRA — NOT RGBA!).
-- BMP stores channels in BGR order by historical convention (little-endian
-- stores the "blue" byte at the lowest address, reading left-to-right B-G-R).
--
-- # Row Padding
--
-- For 32 bpp, each row is already a multiple of 4 bytes (width * 4), so no
-- padding is needed.  For other bit depths, rows are padded to 4-byte
-- boundaries — but we only support 32 bpp, so padding is not an issue here.
--
-- # string.pack / string.unpack
--
-- Lua 5.3+ provides string.pack and string.unpack for binary I/O.
-- Format strings used here:
--
--   "<"    = little-endian
--   "I2"   = unsigned 16-bit integer  (2 bytes)
--   "I4"   = unsigned 32-bit integer  (4 bytes)
--   "i4"   = signed   32-bit integer  (4 bytes)
--   "c2"   = raw 2-byte string (used for the 'BM' magic)
--   "B"    = unsigned byte (8 bits)

local pc = require("coding_adventures.pixel_container")

local M = {}

M.VERSION   = "0.1.0"
M.mime_type = "image/bmp"

-- Header sizes (bytes)
local FILE_HEADER_SIZE = 14
local DIB_HEADER_SIZE  = 40
local PIXEL_DATA_OFFSET = FILE_HEADER_SIZE + DIB_HEADER_SIZE  -- 54

-- ---------------------------------------------------------------------------
-- encode_bmp: PixelContainer → binary string
-- ---------------------------------------------------------------------------

--- Encode a PixelContainer into a BMP binary string.
--
-- The output is a valid 32-bit RGBA BMP file (BI_RGB, top-down, no padding).
-- The alpha channel is preserved in the fourth byte of each pixel.
--
-- @param c  table  pixel container (must have width, height, data fields)
-- @return string  raw BMP file bytes
function M.encode_bmp(c)
    local w = c.width
    local h = c.height

    -- Total file size = headers + pixel data
    local pixel_data_size = w * h * 4
    local file_size = PIXEL_DATA_OFFSET + pixel_data_size

    -- -------------------------------------------------------------------------
    -- BITMAPFILEHEADER (14 bytes)
    -- -------------------------------------------------------------------------
    -- bfType: 'B','M' as raw bytes (0x42, 0x4D).
    -- We use string.char to produce the two magic bytes rather than pack "I2"
    -- because "I2" would be 0x4D42 little-endian = 0x42,0x4D — but it's
    -- cleaner to be explicit here.
    local file_header = string.pack("<c2I4I2I2I4",
        "BM",                -- bfType: magic bytes
        file_size,           -- bfSize
        0,                   -- bfReserved1
        0,                   -- bfReserved2
        PIXEL_DATA_OFFSET    -- bfOffBits
    )

    -- -------------------------------------------------------------------------
    -- BITMAPINFOHEADER (40 bytes)
    -- -------------------------------------------------------------------------
    -- biHeight is negative to signal top-down row order.
    -- biCompression = 0 (BI_RGB) — simplest, widely supported.
    local dib_header = string.pack("<I4i4i4I2I2I4I4i4i4I4I4",
        DIB_HEADER_SIZE,   -- biSize = 40
        w,                 -- biWidth  (signed positive)
        -h,                -- biHeight (negative = top-down)
        1,                 -- biPlanes = 1
        32,                -- biBitCount = 32 (BGRA)
        0,                 -- biCompression = BI_RGB
        pixel_data_size,   -- biSizeImage
        0,                 -- biXPelsPerMeter
        0,                 -- biYPelsPerMeter
        0,                 -- biClrUsed
        0                  -- biClrImportant
    )

    -- -------------------------------------------------------------------------
    -- Pixel data: BGRA order, top-down, row-major
    -- -------------------------------------------------------------------------
    -- BMP stores channels as Blue, Green, Red, Alpha.
    -- Our container stores R, G, B, A.
    -- We must swap R and B when writing each pixel.
    local rows = {}
    for y = 0, h - 1 do
        local row_bytes = {}
        for x = 0, w - 1 do
            local r, g, b, a = pc.pixel_at(c, x, y)
            -- Pack as BGRA (note: Blue first, then Green, Red, Alpha)
            row_bytes[#row_bytes + 1] = string.char(b, g, r, a)
        end
        rows[#rows + 1] = table.concat(row_bytes)
    end

    return file_header .. dib_header .. table.concat(rows)
end

-- ---------------------------------------------------------------------------
-- decode_bmp: binary string → PixelContainer
-- ---------------------------------------------------------------------------

--- Decode a BMP binary string into a PixelContainer.
--
-- Supports 32-bit RGBA/BGRA BMP files with BITMAPINFOHEADER.
-- Both top-down (negative biHeight) and bottom-up (positive biHeight) are
-- handled: we normalise to top-down while populating the container.
--
-- @param data  string  raw BMP file bytes
-- @return table  pixel container
-- @error  string if the data is not a valid BMP
function M.decode_bmp(data)
    if type(data) ~= "string" then
        error("decode_bmp: expected string, got " .. type(data))
    end

    -- Need at least 54 bytes for headers
    if #data < PIXEL_DATA_OFFSET then
        error("decode_bmp: data too short (" .. #data .. " bytes)")
    end

    -- -------------------------------------------------------------------------
    -- Parse BITMAPFILEHEADER
    -- -------------------------------------------------------------------------
    local magic = string.sub(data, 1, 2)
    if magic ~= "BM" then
        error("decode_bmp: invalid magic bytes '" .. magic .. "' (expected 'BM')")
    end

    -- bfOffBits is at offset 10, 4 bytes (uint32 LE)
    local off_bits = string.unpack("<I4", data, 11)  -- position 11 = offset 10 + 1

    -- -------------------------------------------------------------------------
    -- Parse BITMAPINFOHEADER (starts at byte offset 14, position 15)
    -- -------------------------------------------------------------------------
    local dib_size, biWidth, biHeight, biPlanes, biBitCount, biCompression =
        string.unpack("<I4i4i4I2I2I4", data, 15)

    if dib_size < 40 then
        error("decode_bmp: unsupported DIB header size " .. dib_size)
    end
    if biBitCount ~= 32 then
        error("decode_bmp: only 32-bit BMP is supported (got " .. biBitCount .. " bpp)")
    end
    if biCompression ~= 0 and biCompression ~= 3 then
        error("decode_bmp: unsupported compression " .. biCompression)
    end

    -- biHeight is negative for top-down, positive for bottom-up
    local top_down = (biHeight < 0)
    local height = math.abs(biHeight)
    local width  = biWidth

    if width < 1 or height < 1 then
        error("decode_bmp: invalid dimensions " .. width .. "x" .. height)
    end

    local expected_pixel_bytes = width * height * 4
    if #data < off_bits + expected_pixel_bytes then
        error("decode_bmp: pixel data truncated")
    end

    -- -------------------------------------------------------------------------
    -- Read pixel data (BGRA order, 4 bytes per pixel)
    -- -------------------------------------------------------------------------
    local c = pc.new(width, height)
    local pos = off_bits + 1  -- 1-based string position

    for row_index = 0, height - 1 do
        -- Map file row index to container y coordinate.
        -- Top-down: row_index 0 → y=0, row_index 1 → y=1, ...
        -- Bottom-up: row_index 0 → y=height-1, row_index 1 → y=height-2, ...
        local y = top_down and row_index or (height - 1 - row_index)

        for x = 0, width - 1 do
            -- Read 4 bytes: B, G, R, A
            local b_byte, g_byte, r_byte, a_byte = string.byte(data, pos, pos + 3)
            pc.set_pixel(c, x, y, r_byte, g_byte, b_byte, a_byte)
            pos = pos + 4
        end
    end

    return c
end

-- ---------------------------------------------------------------------------
-- Codec table (conforms to the ImageCodec interface)
-- ---------------------------------------------------------------------------

M.codec = {
    mime_type = M.mime_type,
    encode    = M.encode_bmp,
    decode    = M.decode_bmp,
}

return M

-- pixel_container — IC00: Fixed RGBA8 pixel buffer
--
-- # What Is a Pixel Container?
--
-- A pixel container is the simplest possible in-memory image representation:
-- a flat array of bytes, one byte per channel, four channels per pixel,
-- pixels arranged left-to-right then top-to-bottom (row-major order).
--
-- # Memory Layout
--
-- Each pixel occupies exactly 4 consecutive bytes:
--
--   byte 0: Red   (0–255)
--   byte 1: Green (0–255)
--   byte 2: Blue  (0–255)
--   byte 3: Alpha (0–255, 255 = fully opaque, 0 = fully transparent)
--
-- For a pixel at column x, row y (both 0-indexed), the byte offset of its
-- Red channel in the flat array is:
--
--   offset = (y * width + x) * 4
--
-- In Lua, arrays are 1-indexed, so the table index of that Red byte is:
--
--   index = (y * width + x) * 4 + 1
--
-- This means:
--   R is at index + 0
--   G is at index + 1
--   B is at index + 2
--   A is at index + 3
--
-- # Why Row-Major?
--
-- Row-major (C order) means we store an entire row before starting the next.
-- This is the most common layout for raster images and matches the BMP, PPM,
-- and QOI file formats we build on top of this container.
--
-- # Coordinate System
--
--   (0,0) is top-left.
--   x increases rightward; y increases downward.
--   (width-1, height-1) is bottom-right.
--
-- # Thread Safety
--
-- This module is purely functional: `new`, `pixel_at` are read-only;
-- `set_pixel` and `fill_pixels` mutate in place. No global state is used.
-- Lua does not have threads, so this is fine for all practical purposes.

local M = {}

M.VERSION = "0.1.0"

-- ---------------------------------------------------------------------------
-- Constructor
-- ---------------------------------------------------------------------------

--- Create a new PixelContainer initialised to all-black transparent pixels.
--
-- All bytes in `data` start at 0 — that is, RGBA = (0, 0, 0, 0), which is
-- "fully-transparent black" in premultiplied alpha or "black, invisible" in
-- straight alpha conventions.
--
-- @param width  number  image width in pixels (must be > 0)
-- @param height number  image height in pixels (must be > 0)
-- @return table  { width=number, height=number, data=table }
-- @error  string if width or height is not a positive integer
function M.new(width, height)
    if type(width)  ~= "number" or width  < 1 or math.floor(width)  ~= width then
        error("pixel_container.new: width must be a positive integer, got " .. tostring(width))
    end
    if type(height) ~= "number" or height < 1 or math.floor(height) ~= height then
        error("pixel_container.new: height must be a positive integer, got " .. tostring(height))
    end

    -- Allocate width * height * 4 bytes, all zero.
    -- In Lua, tables are 1-indexed, so data[1] is the Red channel of pixel (0,0).
    local n = width * height * 4
    local data = {}
    for i = 1, n do
        data[i] = 0
    end

    return { width = width, height = height, data = data }
end

-- ---------------------------------------------------------------------------
-- Internal helper: compute the 1-based index of channel 0 (Red) for pixel (x, y)
-- ---------------------------------------------------------------------------

--- Return the 1-based index into `data` for the Red channel of pixel (x, y).
-- Coordinates are 0-indexed.  Returns nil if out of bounds.
--
-- Layout: index = (y * width + x) * 4 + 1
--
-- @param c  table   pixel container
-- @param x  number  0-indexed column
-- @param y  number  0-indexed row
-- @return number|nil  1-based index, or nil if (x, y) is out of bounds
local function base_index(c, x, y)
    -- Reject negative coordinates or coordinates beyond the image boundary.
    if x < 0 or y < 0 or x >= c.width or y >= c.height then
        return nil
    end
    -- Row-major: row y starts at offset y * width pixels from the origin.
    -- Multiply by 4 because each pixel is 4 bytes.
    -- Add 1 to convert from 0-based C-style offset to 1-based Lua index.
    return (y * c.width + x) * 4 + 1
end

-- ---------------------------------------------------------------------------
-- Read / Write API
-- ---------------------------------------------------------------------------

--- Return the RGBA values at pixel (x, y). Coordinates are 0-indexed.
--
-- Returns four values: r, g, b, a (each in the range 0–255).
-- If (x, y) is outside the image bounds, returns 0, 0, 0, 0.
--
-- Example:
--   local r, g, b, a = pc.pixel_at(container, 3, 7)
--
-- @param c  table   pixel container
-- @param x  number  0-indexed column
-- @param y  number  0-indexed row
-- @return number, number, number, number  r, g, b, a
function M.pixel_at(c, x, y)
    local i = base_index(c, x, y)
    if i == nil then
        -- Out-of-bounds: silently return transparent black.
        return 0, 0, 0, 0
    end
    return c.data[i], c.data[i+1], c.data[i+2], c.data[i+3]
end

--- Set the RGBA values at pixel (x, y). Coordinates are 0-indexed.
--
-- No-op if (x, y) is outside the image bounds, so callers can draw freely
-- without explicit bounds checking.
--
-- Values should be integers in the range 0–255.  Values outside this range
-- are stored as-is; the container does not clamp or error on overflow.
--
-- Example:
--   pc.set_pixel(container, 10, 20, 255, 128, 0, 255) -- orange pixel
--
-- @param c  table   pixel container
-- @param x  number  0-indexed column
-- @param y  number  0-indexed row
-- @param r  number  Red   channel (0–255)
-- @param g  number  Green channel (0–255)
-- @param b  number  Blue  channel (0–255)
-- @param a  number  Alpha channel (0–255, 255 = fully opaque)
function M.set_pixel(c, x, y, r, g, b, a)
    local i = base_index(c, x, y)
    if i == nil then
        return  -- out-of-bounds: silently ignore
    end
    c.data[i]   = r
    c.data[i+1] = g
    c.data[i+2] = b
    c.data[i+3] = a
end

--- Fill every pixel in the container with the given RGBA values.
--
-- Iterates in row-major order and overwrites every 4-byte pixel block.
-- This is O(width * height) but avoids per-pixel bounds checks.
--
-- Example:
--   pc.fill_pixels(container, 255, 255, 255, 255) -- solid white
--
-- @param c  table   pixel container
-- @param r  number  Red   channel (0–255)
-- @param g  number  Green channel (0–255)
-- @param b  number  Blue  channel (0–255)
-- @param a  number  Alpha channel (0–255)
function M.fill_pixels(c, r, g, b, a)
    local n = c.width * c.height
    for px = 0, n - 1 do
        local i = px * 4 + 1
        c.data[i]   = r
        c.data[i+1] = g
        c.data[i+2] = b
        c.data[i+3] = a
    end
end

-- ---------------------------------------------------------------------------
-- Utility: clone
-- ---------------------------------------------------------------------------

--- Return a deep copy of a pixel container (new data table, same dimensions).
--
-- Useful when you want to compare before/after states, or when a codec needs
-- to produce a new container without aliasing the original.
--
-- @param c  table  source pixel container
-- @return table  new pixel container with identical pixels
function M.clone(c)
    local n = c.width * c.height * 4
    local data = {}
    for i = 1, n do
        data[i] = c.data[i]
    end
    return { width = c.width, height = c.height, data = data }
end

-- ---------------------------------------------------------------------------
-- Utility: equals (pixel-exact comparison)
-- ---------------------------------------------------------------------------

--- Return true if two containers have the same dimensions and identical pixels.
--
-- Useful in unit tests.
--
-- @param a  table  first container
-- @param b  table  second container
-- @return boolean
function M.equals(a, b)
    if a.width ~= b.width or a.height ~= b.height then
        return false
    end
    local n = a.width * a.height * 4
    for i = 1, n do
        if a.data[i] ~= b.data[i] then
            return false
        end
    end
    return true
end

-- ---------------------------------------------------------------------------
-- ImageCodec "interface" convention (documentation only)
-- ---------------------------------------------------------------------------
--
-- A codec table conforms to the following shape:
--
--   codec.mime_type  string                  e.g. "image/bmp"
--   codec.encode(c)  function → string       serialise container to bytes
--   codec.decode(s)  function → container    parse bytes into container
--
-- This module is NOT a codec itself — it is the data model.  Codecs (BMP,
-- PPM, QOI) import `pixel_container` and return / accept containers.

return M

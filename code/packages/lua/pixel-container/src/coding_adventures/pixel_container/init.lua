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

-- Lua numbers stored one-per-table-slot are far larger than bytes.  A
-- 32-megapixel RGBA image would therefore consume multiple gigabytes if data
-- were a conventional dense table.  Keep the public table indexing contract,
-- but back it with immutable 4 KiB byte strings and a tiny proxy metatable.
local BYTE_CHUNK_SIZE = 4096
local ZERO_CHUNK = string.rep("\0", BYTE_CHUNK_SIZE)
local BUFFER_STATES = setmetatable({}, {__mode = "k"})
local NIL_VALUE = {}

local function validate_dimensions(width, height, caller)
    if type(width) ~= "number" or width < 1 or math.floor(width) ~= width then
        error(caller .. ": width must be a positive integer, got " .. tostring(width))
    end
    if type(height) ~= "number" or height < 1 or math.floor(height) ~= height then
        error(caller .. ": height must be a positive integer, got " .. tostring(height))
    end
    if width > math.maxinteger // 4 // height then
        error(caller .. ": dimensions exceed the addressable byte length")
    end
    return width * height * 4
end

local function read_buffer_byte(state, index)
    local override = state.overrides[index]
    if override ~= nil then
        if override == NIL_VALUE then return nil end
        return override
    end
    local chunk_index = ((index - 1) // BYTE_CHUNK_SIZE) + 1
    local chunk_offset = ((index - 1) % BYTE_CHUNK_SIZE) + 1
    return state.chunks[chunk_index]:byte(chunk_offset)
end

local function write_buffer_byte(state, index, value)
    if type(index) ~= "number" or math.floor(index) ~= index
        or index < 1 or index > state.length
    then
        error("pixel byte index out of bounds: " .. tostring(index))
    end
    if type(value) ~= "number" or math.floor(value) ~= value
        or value < 0 or value > 255
    then
        state.overrides[index] = value == nil and NIL_VALUE or value
        return
    end
    state.overrides[index] = nil
    local chunk_index = ((index - 1) // BYTE_CHUNK_SIZE) + 1
    local chunk_offset = ((index - 1) % BYTE_CHUNK_SIZE) + 1
    local chunk = state.chunks[chunk_index]
    state.chunks[chunk_index] = chunk:sub(1, chunk_offset - 1)
        .. string.char(value) .. chunk:sub(chunk_offset + 1)
end

local function proxy_for_state(state)
    local proxy = {}
    local function iterator(_, previous)
        local index = previous + 1
        if index > state.length then return nil end
        return index, read_buffer_byte(state, index)
    end
    setmetatable(proxy, {
        __len = function() return state.length end,
        __index = function(_, index)
            if type(index) ~= "number" or math.floor(index) ~= index
                or index < 1 or index > state.length
            then
                return nil
            end
            return read_buffer_byte(state, index)
        end,
        __newindex = function(_, index, value)
            write_buffer_byte(state, index, value)
        end,
        __pairs = function() return iterator, proxy, 0 end,
        __metatable = "pixel-byte-buffer",
    })
    BUFFER_STATES[proxy] = state
    return proxy
end

local function make_buffer(length, bytes)
    local state = {length = length, chunks = {}, overrides = {}}
    local chunk_count = (length + BYTE_CHUNK_SIZE - 1) // BYTE_CHUNK_SIZE
    for chunk_index = 1, chunk_count do
        local first = (chunk_index - 1) * BYTE_CHUNK_SIZE + 1
        local size = math.min(BYTE_CHUNK_SIZE, length - first + 1)
        if bytes == nil then
            state.chunks[chunk_index] = size == BYTE_CHUNK_SIZE
                and ZERO_CHUNK or string.rep("\0", size)
        else
            state.chunks[chunk_index] = bytes:sub(first, first + size - 1)
        end
    end
    return proxy_for_state(state)
end

local function make_buffer_from_parts(length, parts)
    local state = {length = length, chunks = {}, overrides = {}}
    local pending = ""
    local total = 0
    for _, part in ipairs(parts) do
        if type(part) ~= "string" then
            error("pixel_container.from_byte_chunks: every part must be a string")
        end
        total = total + #part
        if total > length then
            error("pixel_container.from_byte_chunks: byte length must equal width * height * 4")
        end
        local cursor = 1
        while cursor <= #part do
            local needed = BYTE_CHUNK_SIZE - #pending
            local count = math.min(needed, #part - cursor + 1)
            pending = pending .. part:sub(cursor, cursor + count - 1)
            cursor = cursor + count
            if #pending == BYTE_CHUNK_SIZE then
                state.chunks[#state.chunks + 1] = pending
                pending = ""
            end
        end
    end
    if total ~= length then
        error("pixel_container.from_byte_chunks: byte length must equal width * height * 4")
    end
    if #pending > 0 then state.chunks[#state.chunks + 1] = pending end

    return proxy_for_state(state)
end

local function write_pixel_bytes(data, index, r, g, b, a)
    local state = BUFFER_STATES[data]
    local function is_byte(value)
        return type(value) == "number" and math.floor(value) == value
            and value >= 0 and value <= 255
    end
    if state ~= nil then
        local all_bytes = is_byte(r) and is_byte(g) and is_byte(b) and is_byte(a)
        if all_bytes then
            local remaining = string.char(r, g, b, a)
            local cursor = index
            while #remaining > 0 do
                local chunk_index = ((cursor - 1) // BYTE_CHUNK_SIZE) + 1
                local chunk_offset = ((cursor - 1) % BYTE_CHUNK_SIZE) + 1
                local chunk = state.chunks[chunk_index]
                local count = math.min(#remaining, #chunk - chunk_offset + 1)
                state.chunks[chunk_index] = chunk:sub(1, chunk_offset - 1)
                    .. remaining:sub(1, count)
                    .. chunk:sub(chunk_offset + count)
                for offset = 0, count - 1 do state.overrides[cursor + offset] = nil end
                cursor = cursor + count
                remaining = remaining:sub(count + 1)
            end
            return
        end
    end
    data[index] = r
    data[index + 1] = g
    data[index + 2] = b
    data[index + 3] = a
end

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
    local length = validate_dimensions(width, height, "pixel_container.new")
    return {width = width, height = height, data = make_buffer(length)}
end

--- Create a PixelContainer from an exact binary RGBA8 byte string.
-- The returned `data` value is still a mutable, 1-indexed Lua table interface,
-- but it keeps a compact byte-string backing instead of boxed numeric entries.
function M.from_bytes(width, height, bytes)
    local length = validate_dimensions(width, height, "pixel_container.from_bytes")
    if type(bytes) ~= "string" or #bytes ~= length then
        error("pixel_container.from_bytes: byte length must equal width * height * 4")
    end
    return {width = width, height = height, data = make_buffer(length, bytes)}
end

--- Create a compact container from binary chunks without one full-size join.
function M.from_byte_chunks(width, height, parts)
    local length = validate_dimensions(width, height, "pixel_container.from_byte_chunks")
    if type(parts) ~= "table" then
        error("pixel_container.from_byte_chunks: parts must be a table")
    end
    return {width = width, height = height, data = make_buffer_from_parts(length, parts)}
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
    write_pixel_bytes(c.data, i, r, g, b, a)
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
    local state = BUFFER_STATES[c.data]
    local function is_byte(value)
        return type(value) == "number" and math.floor(value) == value
            and value >= 0 and value <= 255
    end
    local all_bytes = state ~= nil and is_byte(r) and is_byte(g)
        and is_byte(b) and is_byte(a)
    if all_bytes then
        local pixel = string.char(r, g, b, a)
        local full_chunk = pixel:rep(BYTE_CHUNK_SIZE // 4)
        for chunk_index = 1, #state.chunks do
            local size = #state.chunks[chunk_index]
            state.chunks[chunk_index] = size == BYTE_CHUNK_SIZE
                and full_chunk or pixel:rep(size // 4)
        end
        state.overrides = {}
        return
    end
    local n = c.width * c.height
    for px = 0, n - 1 do
        local i = px * 4 + 1
        write_pixel_bytes(c.data, i, r, g, b, a)
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
    local source_state = BUFFER_STATES[c.data]
    if source_state ~= nil then
        local data = make_buffer(n)
        local target_state = BUFFER_STATES[data]
        for index, chunk in ipairs(source_state.chunks) do
            target_state.chunks[index] = chunk
        end
        for index, value in pairs(source_state.overrides) do
            target_state.overrides[index] = value
        end
        return {width = c.width, height = c.height, data = data}
    end
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

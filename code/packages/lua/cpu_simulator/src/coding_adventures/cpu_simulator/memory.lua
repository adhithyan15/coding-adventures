-- =============================================================================
-- Memory — byte-addressable RAM simulation
-- =============================================================================
--
-- TWO IMPLEMENTATIONS
-- ===================
--
-- Memory       — dense array-backed storage; best for small, contiguous programs
-- SparseMemory — hash-backed storage; best for sparse address spaces (e.g., when
--               a program touches only a few pages of a 32-bit address space)
--
-- BYTE ORDER
-- ==========
-- All multi-byte word reads/writes use LITTLE-ENDIAN order — the same as
-- ARM (default), x86, and RISC-V.  Byte 0 = least significant byte.
--
--   address 0x100:  b0 (LSB)
--   address 0x101:  b1
--   address 0x102:  b2
--   address 0x103:  b3 (MSB)
--
--   read_word(0x100) = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)

local bit = bit or {}
-- Lua 5.4 has native bitwise operators; provide fallback for older versions
local function band(a, b) return a & b end
local function bor(a, b)  return a | b end
local function lshift(a, n) return a << n end
local function rshift(a, n) return a >> n end

-- ---------------------------------------------------------------------------
-- Memory — dense array-backed
-- ---------------------------------------------------------------------------

local Memory = {}
Memory.__index = Memory

-- Memory.new(size) — create a zero-filled memory of `size` bytes
-- Internally uses a 1-indexed Lua table; address N maps to index N+1.
function Memory.new(size)
    assert(type(size) == "number" and size >= 1, "size must be >= 1")
    local self = setmetatable({}, Memory)
    self.size = size
    self.data = {}
    for i = 1, size do
        self.data[i] = 0
    end
    return self
end

-- read_byte(address) → 0-255
function Memory:read_byte(address)
    assert(address >= 0 and address < self.size,
        string.format("Memory.read_byte: address 0x%X out of range [0, %d)", address, self.size))
    return self.data[address + 1]
end

-- write_byte(address, value) — stores value & 0xFF
function Memory:write_byte(address, value)
    assert(address >= 0 and address < self.size,
        string.format("Memory.write_byte: address 0x%X out of range [0, %d)", address, self.size))
    self.data[address + 1] = band(value, 0xFF)
end

-- read_word(address) — reads 4 bytes little-endian → 32-bit integer
function Memory:read_word(address)
    local b0 = self:read_byte(address)
    local b1 = self:read_byte(address + 1)
    local b2 = self:read_byte(address + 2)
    local b3 = self:read_byte(address + 3)
    return band(b0 | lshift(b1, 8) | lshift(b2, 16) | lshift(b3, 24), 0xFFFFFFFF)
end

-- write_word(address, value) — writes 32-bit integer as 4 bytes little-endian
function Memory:write_word(address, value)
    local v = band(value, 0xFFFFFFFF)
    self:write_byte(address,     band(v, 0xFF))
    self:write_byte(address + 1, band(rshift(v, 8),  0xFF))
    self:write_byte(address + 2, band(rshift(v, 16), 0xFF))
    self:write_byte(address + 3, band(rshift(v, 24), 0xFF))
end

-- load_bytes(address, bytes) — bulk-write a list of byte values
function Memory:load_bytes(address, bytes)
    for i, b in ipairs(bytes) do
        self:write_byte(address + i - 1, b)
    end
end

-- dump(start, length) — returns a list of length byte values starting at start
function Memory:dump(start, length)
    local result = {}
    for i = 0, length - 1 do
        result[i + 1] = self:read_byte(start + i)
    end
    return result
end

-- ---------------------------------------------------------------------------
-- SparseMemory — hash-backed (efficient for sparse address spaces)
-- ---------------------------------------------------------------------------
-- Only stores bytes that have been explicitly written.  Reading an unwritten
-- address returns 0 (just like real DRAM after a cold reset).
--
-- This is useful for simulating large 32-bit or 64-bit address spaces where
-- a program only touches a few KB scattered across many pages.

local SparseMemory = {}
SparseMemory.__index = SparseMemory

function SparseMemory.new()
    local self = setmetatable({}, SparseMemory)
    self.data = {}   -- {[address] = byte_value}
    return self
end

function SparseMemory:read_byte(address)
    return self.data[address] or 0
end

function SparseMemory:write_byte(address, value)
    local v = band(value, 0xFF)
    if v == 0 then
        -- Keep memory sparse: delete zero entries
        self.data[address] = nil
    else
        self.data[address] = v
    end
end

function SparseMemory:read_word(address)
    local b0 = self:read_byte(address)
    local b1 = self:read_byte(address + 1)
    local b2 = self:read_byte(address + 2)
    local b3 = self:read_byte(address + 3)
    return band(b0 | lshift(b1, 8) | lshift(b2, 16) | lshift(b3, 24), 0xFFFFFFFF)
end

function SparseMemory:write_word(address, value)
    local v = band(value, 0xFFFFFFFF)
    self:write_byte(address,     band(v, 0xFF))
    self:write_byte(address + 1, band(rshift(v, 8),  0xFF))
    self:write_byte(address + 2, band(rshift(v, 16), 0xFF))
    self:write_byte(address + 3, band(rshift(v, 24), 0xFF))
end

function SparseMemory:load_bytes(address, bytes)
    for i, b in ipairs(bytes) do
        self:write_byte(address + i - 1, b)
    end
end

-- dump returns a dense list for the range [start, start+length)
function SparseMemory:dump(start, length)
    local result = {}
    for i = 0, length - 1 do
        result[i + 1] = self:read_byte(start + i)
    end
    return result
end

return {
    Memory       = Memory,
    SparseMemory = SparseMemory,
}

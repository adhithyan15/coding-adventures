-- memory.lua — Byte-addressable RAM simulation
--
-- Memory is the large, slower storage in a computer. The CPU loads data
-- from memory into registers (fast storage), operates on it there, and
-- stores results back to memory.
--
-- This module implements two memory types:
--
--   Memory        — fixed-size, byte-addressed memory (backed by a Lua table)
--   SparseMemory  — sparse memory for large address spaces (only stores
--                   non-zero locations)
--
-- BYTE ORDERING: Both implementations use little-endian byte order for
-- multi-byte reads/writes. This means the LEAST significant byte is stored
-- at the lower address:
--
--   Address:  0x00  0x01  0x02  0x03
--   Value:    0xAB  0xCD  0xEF  0x12   (stores 32-bit 0x12EFCDAB)
--
-- Little-endian is used by x86, ARM (in LE mode), and RISC-V. Big-endian
-- (most significant byte first) is used by SPARC and older MIPS designs.
-- We choose little-endian because it matches the most common modern CPUs.
--
-- WORD SIZE: We work with 32-bit words (4 bytes). Each memory read_word()
-- reads 4 consecutive bytes starting at `address`.

local Memory = {}
Memory.__index = Memory

--- Creates a new fixed-size Memory.
--
-- All bytes are initialized to 0. Valid addresses are [0, size-1].
--
-- @param size  number  Number of bytes (must be >= 1)
-- @return Memory
function Memory.new(size)
    assert(size >= 1, "memory size must be >= 1")
    local m = setmetatable({
        size = size,
        data = {},  -- 1-indexed internally; address 0 → data[1]
    }, Memory)
    -- Initialize all bytes to 0
    for i = 1, size do
        m.data[i] = 0
    end
    return m
end

--- Reads a single byte from the given address.
-- @param address  number  Byte address [0, size-1]
-- @return number  Byte value [0, 255]
function Memory:read_byte(address)
    assert(address >= 0 and address < self.size,
        string.format("memory read out of bounds: address %d, size %d", address, self.size))
    return self.data[address + 1]
end

--- Writes a single byte to the given address.
-- @param address  number  Byte address [0, size-1]
-- @param value    number  Byte value (masked to 8 bits)
function Memory:write_byte(address, value)
    assert(address >= 0 and address < self.size,
        string.format("memory write out of bounds: address %d, size %d", address, self.size))
    self.data[address + 1] = value & 0xFF
end

--- Reads a 32-bit word (4 bytes, little-endian) from the given address.
--
-- Little-endian layout:
--   data[address+0] = bits  7..0
--   data[address+1] = bits 15..8
--   data[address+2] = bits 23..16
--   data[address+3] = bits 31..24
--
-- @param address  number  Byte address (should be 4-byte aligned)
-- @return number  32-bit unsigned integer
function Memory:read_word(address)
    local b0 = self:read_byte(address)
    local b1 = self:read_byte(address + 1)
    local b2 = self:read_byte(address + 2)
    local b3 = self:read_byte(address + 3)
    return (b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)) & 0xFFFFFFFF
end

--- Writes a 32-bit word (4 bytes, little-endian) to the given address.
-- @param address  number  Byte address (should be 4-byte aligned)
-- @param value    number  32-bit value
function Memory:write_word(address, value)
    local v = value & 0xFFFFFFFF
    self:write_byte(address,     v & 0xFF)
    self:write_byte(address + 1, (v >> 8)  & 0xFF)
    self:write_byte(address + 2, (v >> 16) & 0xFF)
    self:write_byte(address + 3, (v >> 24) & 0xFF)
end

--- Loads a list of byte values into memory starting at the given address.
-- @param address  number  Start address
-- @param bytes    table   List of byte values
function Memory:load_bytes(address, bytes)
    for i, b in ipairs(bytes) do
        self:write_byte(address + i - 1, b)
    end
end

--- Returns a list of bytes from [start, start+length-1].
-- @param start   number  Start address
-- @param length  number  Number of bytes to dump
-- @return table  List of byte values
function Memory:dump(start, length)
    local result = {}
    for i = 0, length - 1 do
        result[i + 1] = self:read_byte(start + i)
    end
    return result
end

-- ========================================================================
-- SparseMemory
-- ========================================================================
--
-- SparseMemory stores only non-zero locations. This is useful for large
-- address spaces (e.g., a 4GB address space) where most locations are zero.
--
-- Under the hood it uses a Lua table as a dictionary:
--   sparse[address] = byte_value  (only for non-zero locations)

local SparseMemory = {}
SparseMemory.__index = SparseMemory

--- Creates a new SparseMemory with the given size limit.
-- @param size  number  Maximum address space size
-- @return SparseMemory
function SparseMemory.new(size)
    assert(size >= 1, "sparse memory size must be >= 1")
    return setmetatable({
        size = size,
        data = {},  -- address → byte value (only non-zero)
    }, SparseMemory)
end

function SparseMemory:read_byte(address)
    assert(address >= 0 and address < self.size,
        string.format("sparse memory read out of bounds: %d", address))
    return self.data[address] or 0
end

function SparseMemory:write_byte(address, value)
    assert(address >= 0 and address < self.size,
        string.format("sparse memory write out of bounds: %d", address))
    local v = value & 0xFF
    if v == 0 then
        self.data[address] = nil  -- keep it sparse
    else
        self.data[address] = v
    end
end

function SparseMemory:read_word(address)
    local b0 = self:read_byte(address)
    local b1 = self:read_byte(address + 1)
    local b2 = self:read_byte(address + 2)
    local b3 = self:read_byte(address + 3)
    return (b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)) & 0xFFFFFFFF
end

function SparseMemory:write_word(address, value)
    local v = value & 0xFFFFFFFF
    self:write_byte(address,     v & 0xFF)
    self:write_byte(address + 1, (v >> 8)  & 0xFF)
    self:write_byte(address + 2, (v >> 16) & 0xFF)
    self:write_byte(address + 3, (v >> 24) & 0xFF)
end

function SparseMemory:load_bytes(address, bytes)
    for i, b in ipairs(bytes) do
        self:write_byte(address + i - 1, b)
    end
end

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

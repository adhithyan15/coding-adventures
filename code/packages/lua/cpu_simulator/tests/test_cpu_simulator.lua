-- test_cpu_simulator.lua — Tests for the cpu_simulator package
--
-- Tests cover:
--   1. Memory — read/write byte and word, bounds checking, load/dump
--   2. SparseMemory — same API, sparse behavior
--   3. RegisterFile — read/write, bit masking, dump

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local cpu_sim      = require("coding_adventures.cpu_simulator")
local Memory       = cpu_sim.Memory
local SparseMemory = cpu_sim.SparseMemory
local RegisterFile = cpu_sim.RegisterFile

-- ========================================================================
-- Memory tests
-- ========================================================================

describe("Memory", function()

    it("initializes all bytes to 0", function()
        local m = Memory.new(64)
        for i = 0, 63 do
            assert.are.equal(0, m:read_byte(i))
        end
    end)

    it("read_byte and write_byte round-trip", function()
        local m = Memory.new(16)
        m:write_byte(0, 0xFF)
        m:write_byte(1, 0xAB)
        m:write_byte(15, 0x42)
        assert.are.equal(0xFF, m:read_byte(0))
        assert.are.equal(0xAB, m:read_byte(1))
        assert.are.equal(0x42, m:read_byte(15))
    end)

    it("write_byte masks value to 8 bits", function()
        local m = Memory.new(8)
        m:write_byte(0, 0x1FF)  -- should store 0xFF
        assert.are.equal(0xFF, m:read_byte(0))
    end)

    it("read_word reads little-endian 32-bit word", function()
        local m = Memory.new(8)
        -- Store 0x12345678 in little-endian at address 0
        m:write_byte(0, 0x78)
        m:write_byte(1, 0x56)
        m:write_byte(2, 0x34)
        m:write_byte(3, 0x12)
        assert.are.equal(0x12345678, m:read_word(0))
    end)

    it("write_word stores little-endian bytes", function()
        local m = Memory.new(8)
        m:write_word(0, 0xDEADBEEF)
        assert.are.equal(0xEF, m:read_byte(0))  -- LSB first
        assert.are.equal(0xBE, m:read_byte(1))
        assert.are.equal(0xAD, m:read_byte(2))
        assert.are.equal(0xDE, m:read_byte(3))  -- MSB last
    end)

    it("write_word/read_word round-trip", function()
        local m = Memory.new(16)
        m:write_word(0, 0xCAFEBABE)
        m:write_word(4, 0x00000001)
        m:write_word(8, 0xFFFFFFFF)
        assert.are.equal(0xCAFEBABE, m:read_word(0))
        assert.are.equal(0x00000001, m:read_word(4))
        assert.are.equal(0xFFFFFFFF, m:read_word(8))
    end)

    it("load_bytes stores bytes sequentially", function()
        local m = Memory.new(16)
        m:load_bytes(4, { 0x01, 0x02, 0x03, 0x04 })
        assert.are.equal(0x01, m:read_byte(4))
        assert.are.equal(0x02, m:read_byte(5))
        assert.are.equal(0x03, m:read_byte(6))
        assert.are.equal(0x04, m:read_byte(7))
    end)

    it("dump returns the correct slice of bytes", function()
        local m = Memory.new(16)
        m:load_bytes(2, { 0xAA, 0xBB, 0xCC })
        local bytes = m:dump(2, 3)
        assert.are.same({ 0xAA, 0xBB, 0xCC }, bytes)
    end)

    it("out-of-bounds read raises an error", function()
        local m = Memory.new(8)
        assert.has_error(function() m:read_byte(8) end)
    end)

    it("out-of-bounds write raises an error", function()
        local m = Memory.new(8)
        assert.has_error(function() m:write_byte(-1, 0) end)
    end)

    it("new() rejects size < 1", function()
        assert.has_error(function() Memory.new(0) end)
    end)

end)

-- ========================================================================
-- SparseMemory tests
-- ========================================================================

describe("SparseMemory", function()

    it("reads 0 from unwritten addresses", function()
        local m = SparseMemory.new(1024 * 1024)
        assert.are.equal(0, m:read_byte(0))
        assert.are.equal(0, m:read_byte(65535))
        assert.are.equal(0, m:read_word(0))
    end)

    it("read_byte/write_byte round-trip", function()
        local m = SparseMemory.new(1024)
        m:write_byte(500, 0x7F)
        assert.are.equal(0x7F, m:read_byte(500))
    end)

    it("writing 0 removes the entry (stays sparse)", function()
        local m = SparseMemory.new(1024)
        m:write_byte(100, 0x42)
        m:write_byte(100, 0x00)  -- should remove the entry
        assert.are.equal(0, m:read_byte(100))
        assert.is_nil(m.data[100])  -- verify sparse behavior
    end)

    it("read_word/write_word round-trip", function()
        local m = SparseMemory.new(65536)
        m:write_word(1000, 0xABCDEF01)
        assert.are.equal(0xABCDEF01, m:read_word(1000))
    end)

    it("load_bytes and dump work correctly", function()
        local m = SparseMemory.new(65536)
        m:load_bytes(200, { 0x11, 0x22, 0x33 })
        local bytes = m:dump(200, 3)
        assert.are.same({ 0x11, 0x22, 0x33 }, bytes)
    end)

end)

-- ========================================================================
-- RegisterFile tests
-- ========================================================================

describe("RegisterFile", function()

    it("initializes all registers to 0", function()
        local rf = RegisterFile.new(16, 32)
        for i = 0, 15 do
            assert.are.equal(0, rf:read(i))
        end
    end)

    it("read/write round-trip", function()
        local rf = RegisterFile.new(16, 32)
        rf:write(0,  0xDEADBEEF)
        rf:write(7,  12345)
        rf:write(15, 0)
        assert.are.equal(0xDEADBEEF, rf:read(0))
        assert.are.equal(12345,      rf:read(7))
        assert.are.equal(0,          rf:read(15))
    end)

    it("masks writes to bit_width (32-bit)", function()
        local rf = RegisterFile.new(4, 32)
        -- Write a value larger than 32 bits
        rf:write(0, 0x1FFFFFFFF)  -- only lower 32 bits should be stored
        assert.are.equal(0xFFFFFFFF, rf:read(0))
    end)

    it("masks writes for 8-bit registers", function()
        local rf = RegisterFile.new(4, 8)
        rf:write(0, 0x1FF)  -- 511 decimal; only 0xFF should be kept
        assert.are.equal(0xFF, rf:read(0))
    end)

    it("num_regs() returns the correct count", function()
        local rf = RegisterFile.new(32, 64)
        assert.are.equal(32, rf:num_regs())
    end)

    it("dump() returns all register values keyed by name", function()
        local rf = RegisterFile.new(4, 32)
        rf:write(0, 10)
        rf:write(1, 20)
        rf:write(2, 30)
        rf:write(3, 40)
        local d = rf:dump()
        assert.are.equal(10, d["R0"])
        assert.are.equal(20, d["R1"])
        assert.are.equal(30, d["R2"])
        assert.are.equal(40, d["R3"])
    end)

    it("out-of-bounds read raises an error", function()
        local rf = RegisterFile.new(4, 32)
        assert.has_error(function() rf:read(4) end)
    end)

    it("out-of-bounds write raises an error", function()
        local rf = RegisterFile.new(4, 32)
        assert.has_error(function() rf:write(-1, 0) end)
    end)

    it("default constructor creates 16 32-bit registers", function()
        local rf = RegisterFile.new()
        assert.are.equal(16, rf:num_regs())
        assert.are.equal(32, rf.bit_width)
    end)

end)

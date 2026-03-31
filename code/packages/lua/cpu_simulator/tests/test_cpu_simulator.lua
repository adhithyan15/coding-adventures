-- Tests for coding_adventures.cpu_simulator

local cpu_sim      = require("coding_adventures.cpu_simulator")
local Memory       = cpu_sim.Memory
local SparseMemory = cpu_sim.SparseMemory
local RegisterFile = cpu_sim.RegisterFile

-- ---------------------------------------------------------------------------
-- Memory tests
-- ---------------------------------------------------------------------------

describe("Memory", function()
    it("initialises all bytes to 0", function()
        local m = Memory.new(64)
        for i = 0, 63 do
            assert.equals(0, m:read_byte(i))
        end
    end)

    it("write_byte / read_byte roundtrip", function()
        local m = Memory.new(64)
        m:write_byte(0, 0xAB)
        assert.equals(0xAB, m:read_byte(0))
    end)

    it("write_byte masks to 8 bits", function()
        local m = Memory.new(64)
        m:write_byte(0, 0x1FF)  -- 511 & 0xFF = 0xFF
        assert.equals(0xFF, m:read_byte(0))
    end)

    it("write_word / read_word little-endian roundtrip", function()
        local m = Memory.new(64)
        m:write_word(0, 0xDEADBEEF)
        assert.equals(0xEF, m:read_byte(0))
        assert.equals(0xBE, m:read_byte(1))
        assert.equals(0xAD, m:read_byte(2))
        assert.equals(0xDE, m:read_byte(3))
        assert.equals(0xDEADBEEF, m:read_word(0))
    end)

    it("write_word at non-zero offset", function()
        local m = Memory.new(64)
        m:write_word(8, 0x12345678)
        assert.equals(0x12345678, m:read_word(8))
    end)

    it("write_word truncates to 32 bits", function()
        local m = Memory.new(64)
        -- 0x1_00000001 & 0xFFFFFFFF = 0x00000001
        m:write_word(0, 0x100000001)
        assert.equals(0x00000001, m:read_word(0))
    end)

    it("load_bytes fills memory correctly", function()
        local m = Memory.new(64)
        m:load_bytes(4, {0x01, 0x02, 0x03, 0x04})
        assert.equals(0x01, m:read_byte(4))
        assert.equals(0x04, m:read_byte(7))
    end)

    it("dump returns correct byte list", function()
        local m = Memory.new(64)
        m:write_byte(0, 0xAA)
        m:write_byte(1, 0xBB)
        m:write_byte(2, 0xCC)
        local bytes = m:dump(0, 3)
        assert.equals(3, #bytes)
        assert.equals(0xAA, bytes[1])
        assert.equals(0xBB, bytes[2])
        assert.equals(0xCC, bytes[3])
    end)

    it("out-of-range read raises error", function()
        local m = Memory.new(4)
        assert.has_error(function() m:read_byte(4) end)
    end)

    it("out-of-range write raises error", function()
        local m = Memory.new(4)
        assert.has_error(function() m:write_byte(4, 0) end)
    end)
end)

-- ---------------------------------------------------------------------------
-- SparseMemory tests
-- ---------------------------------------------------------------------------

describe("SparseMemory", function()
    it("unwritten addresses return 0", function()
        local m = SparseMemory.new()
        assert.equals(0, m:read_byte(0))
        assert.equals(0, m:read_byte(0xFFFF))
        assert.equals(0, m:read_word(0x1000))
    end)

    it("write_byte / read_byte roundtrip", function()
        local m = SparseMemory.new()
        m:write_byte(0x100, 0x42)
        assert.equals(0x42, m:read_byte(0x100))
    end)

    it("write_word / read_word little-endian roundtrip", function()
        local m = SparseMemory.new()
        m:write_word(0x200, 0xCAFEBABE)
        assert.equals(0xCAFEBABE, m:read_word(0x200))
    end)

    it("load_bytes works on sparse memory", function()
        local m = SparseMemory.new()
        m:load_bytes(0, {0x11, 0x22, 0x33})
        assert.equals(0x11, m:read_byte(0))
        assert.equals(0x22, m:read_byte(1))
        assert.equals(0x33, m:read_byte(2))
        assert.equals(0, m:read_byte(3))
    end)

    it("dump on sparse memory returns zeros for unwritten", function()
        local m = SparseMemory.new()
        m:write_byte(1, 0xFF)
        local bytes = m:dump(0, 3)
        assert.equals(0,    bytes[1])
        assert.equals(0xFF, bytes[2])
        assert.equals(0,    bytes[3])
    end)

    it("writing zero removes entry (keeps sparse)", function()
        local m = SparseMemory.new()
        m:write_byte(5, 0x55)
        assert.equals(0x55, m:read_byte(5))
        m:write_byte(5, 0)
        assert.equals(0, m:read_byte(5))
        assert.is_nil(m.data[5])  -- should be absent from the table
    end)
end)

-- ---------------------------------------------------------------------------
-- RegisterFile tests
-- ---------------------------------------------------------------------------

describe("RegisterFile", function()
    it("initialises to all zeros", function()
        local rf = RegisterFile.new(16, 32)
        for i = 0, 15 do
            assert.equals(0, rf:read(i))
        end
    end)

    it("write / read roundtrip", function()
        local rf = RegisterFile.new(16, 32)
        rf:write(3, 0xDEAD)
        assert.equals(0xDEAD, rf:read(3))
    end)

    it("masks 32-bit values on write", function()
        local rf = RegisterFile.new(16, 32)
        rf:write(0, 0x1FFFFFFFF)  -- larger than 32 bits
        assert.equals(0xFFFFFFFF, rf:read(0))
    end)

    it("supports 8-bit register file", function()
        local rf = RegisterFile.new(4, 8)
        rf:write(0, 0x1FF)  -- 511, truncated to 0xFF
        assert.equals(0xFF, rf:read(0))
    end)

    it("dump returns {R0=v, R1=v, ...}", function()
        local rf = RegisterFile.new(4, 32)
        rf:write(1, 100)
        local d = rf:dump()
        assert.equals(0,   d["R0"])
        assert.equals(100, d["R1"])
        assert.not_nil(d["R3"])
    end)

    it("out-of-range read raises error", function()
        local rf = RegisterFile.new(8, 32)
        assert.has_error(function() rf:read(8) end)
    end)

    it("out-of-range write raises error", function()
        local rf = RegisterFile.new(8, 32)
        assert.has_error(function() rf:write(-1, 0) end)
    end)

    it("independent registers don't alias", function()
        local rf = RegisterFile.new(16, 32)
        rf:write(0, 10)
        rf:write(1, 20)
        assert.equals(10, rf:read(0))
        assert.equals(20, rf:read(1))
    end)
end)

-- Tests for block_ram — SRAM cells, arrays, and RAM modules.
--
-- These tests verify every component in the memory hierarchy:
--   1. SRAMCell   — single-bit storage
--   2. SRAMArray  — 2D grid of cells
--   3. SinglePortRAM — synchronous RAM with one port and three read modes
--   4. DualPortRAM   — dual-port RAM with collision detection
--   5. ConfigurableBRAM — FPGA-style reconfigurable Block RAM

-- Add src/ and logic_gates to the module search path.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" ..
               "../../logic_gates/src/?.lua;" ..
               "../../logic_gates/src/?/init.lua;" .. package.path

local block_ram = require("coding_adventures.block_ram")

-- Shorthand constructors
local SRAMCell        = block_ram.SRAMCell
local SRAMArray       = block_ram.SRAMArray
local SinglePortRAM   = block_ram.SinglePortRAM
local DualPortRAM     = block_ram.DualPortRAM
local ConfigurableBRAM = block_ram.ConfigurableBRAM

local READ_FIRST  = block_ram.READ_FIRST
local WRITE_FIRST = block_ram.WRITE_FIRST
local NO_CHANGE   = block_ram.NO_CHANGE

-- =========================================================================
-- Helper: assert two arrays are element-wise equal
-- =========================================================================
local function assert_array_equal(got, want)
    assert.are.equal(#want, #got, "array length mismatch")
    for i = 1, #want do
        assert.are.equal(want[i], got[i],
            string.format("index %d: got %d, want %d", i, got[i], want[i]))
    end
end

-- =========================================================================
-- Module-level tests
-- =========================================================================

describe("block_ram module", function()
    it("has a version", function()
        assert.are.equal("0.1.0", block_ram.VERSION)
    end)

    it("exports read mode constants", function()
        assert.are.equal(1, READ_FIRST)
        assert.are.equal(2, WRITE_FIRST)
        assert.are.equal(3, NO_CHANGE)
    end)

    it("exports all constructors", function()
        assert.is_not_nil(SRAMCell)
        assert.is_not_nil(SRAMArray)
        assert.is_not_nil(SinglePortRAM)
        assert.is_not_nil(DualPortRAM)
        assert.is_not_nil(ConfigurableBRAM)
    end)
end)

-- =========================================================================
-- SRAMCell Tests
-- =========================================================================

describe("SRAMCell", function()
    it("initializes to 0", function()
        local cell = SRAMCell.new()
        assert.are.equal(0, cell:get_value())
    end)

    it("writes and reads when word line is active", function()
        local cell = SRAMCell.new()
        cell:write(1, 1)
        assert.are.equal(1, cell:get_value())

        local val = cell:read(1)
        assert.are.equal(1, val)
    end)

    it("returns nil when reading with word line inactive", function()
        local cell = SRAMCell.new()
        cell:write(1, 1)
        local val = cell:read(0)
        assert.is_nil(val)
    end)

    it("does not write when word line is inactive", function()
        local cell = SRAMCell.new()
        cell:write(1, 1)  -- store 1
        cell:write(0, 0)  -- word line inactive, should not change
        assert.are.equal(1, cell:get_value())
    end)

    it("overwrites correctly", function()
        local cell = SRAMCell.new()
        cell:write(1, 1)
        cell:write(1, 0)
        assert.are.equal(0, cell:get_value())
    end)

    it("can store 0", function()
        local cell = SRAMCell.new()
        cell:write(1, 0)
        assert.are.equal(0, cell:get_value())
        assert.are.equal(0, cell:read(1))
    end)

    it("errors on invalid word line for read", function()
        local cell = SRAMCell.new()
        assert.has_error(function() cell:read(2) end)
        assert.has_error(function() cell:read(-1) end)
    end)

    it("errors on invalid word line for write", function()
        local cell = SRAMCell.new()
        assert.has_error(function() cell:write(2, 0) end)
        assert.has_error(function() cell:write(-1, 0) end)
    end)

    it("errors on invalid bit line for write", function()
        local cell = SRAMCell.new()
        assert.has_error(function() cell:write(1, 2) end)
        assert.has_error(function() cell:write(1, -1) end)
    end)
end)

-- =========================================================================
-- SRAMArray Tests
-- =========================================================================

describe("SRAMArray", function()
    it("initializes all cells to 0", function()
        local arr = SRAMArray.new(4, 8)
        local rows, cols = arr:shape()
        assert.are.equal(4, rows)
        assert.are.equal(8, cols)

        for r = 0, 3 do
            local data = arr:read(r)
            for c = 1, 8 do
                assert.are.equal(0, data[c],
                    string.format("row %d, col %d should be 0", r, c))
            end
        end
    end)

    it("writes and reads a row", function()
        local arr = SRAMArray.new(4, 8)
        local data = {1, 0, 1, 0, 0, 1, 0, 1}
        arr:write(0, data)
        assert_array_equal(arr:read(0), data)

        -- Other rows should still be all zeros
        assert_array_equal(arr:read(1), {0, 0, 0, 0, 0, 0, 0, 0})
    end)

    it("overwrites a row", function()
        local arr = SRAMArray.new(2, 4)
        arr:write(0, {1, 1, 1, 1})
        arr:write(0, {0, 0, 0, 0})
        assert_array_equal(arr:read(0), {0, 0, 0, 0})
    end)

    it("handles multiple rows independently", function()
        local arr = SRAMArray.new(3, 2)
        arr:write(0, {1, 0})
        arr:write(1, {0, 1})
        arr:write(2, {1, 1})

        assert_array_equal(arr:read(0), {1, 0})
        assert_array_equal(arr:read(1), {0, 1})
        assert_array_equal(arr:read(2), {1, 1})
    end)

    it("returns shape correctly", function()
        local arr = SRAMArray.new(7, 3)
        local rows, cols = arr:shape()
        assert.are.equal(7, rows)
        assert.are.equal(3, cols)
    end)

    it("handles 1x1 array", function()
        local arr = SRAMArray.new(1, 1)
        assert_array_equal(arr:read(0), {0})
        arr:write(0, {1})
        assert_array_equal(arr:read(0), {1})
    end)

    it("errors on invalid construction", function()
        assert.has_error(function() SRAMArray.new(0, 1) end)
        assert.has_error(function() SRAMArray.new(1, 0) end)
        assert.has_error(function() SRAMArray.new(-1, 4) end)
        assert.has_error(function() SRAMArray.new(4, -1) end)
    end)

    it("errors on out-of-range row for read", function()
        local arr = SRAMArray.new(2, 4)
        assert.has_error(function() arr:read(-1) end)
        assert.has_error(function() arr:read(2) end)
    end)

    it("errors on out-of-range row for write", function()
        local arr = SRAMArray.new(2, 4)
        assert.has_error(function() arr:write(-1, {0, 0, 0, 0}) end)
        assert.has_error(function() arr:write(2, {0, 0, 0, 0}) end)
    end)

    it("errors on wrong data length for write", function()
        local arr = SRAMArray.new(2, 4)
        assert.has_error(function() arr:write(0, {0, 0}) end)
    end)

    it("errors on invalid bit in data for write", function()
        local arr = SRAMArray.new(2, 4)
        assert.has_error(function() arr:write(0, {0, 0, 2, 0}) end)
    end)
end)

-- =========================================================================
-- SinglePortRAM Tests
-- =========================================================================

describe("SinglePortRAM", function()
    it("writes and reads data", function()
        local ram = SinglePortRAM.new(256, 8, READ_FIRST)
        local data = {1, 1, 0, 0, 1, 0, 1, 0}
        local zeros = {0, 0, 0, 0, 0, 0, 0, 0}

        -- Write: clock LOW then HIGH (rising edge triggers write)
        ram:tick(0, 0, data, 1)
        ram:tick(1, 0, data, 1)

        -- Read: clock LOW then HIGH
        ram:tick(0, 0, zeros, 0)
        local out = ram:tick(1, 0, zeros, 0)
        assert_array_equal(out, data)
    end)

    describe("ReadFirst mode", function()
        it("returns old value during write", function()
            local ram = SinglePortRAM.new(4, 4, READ_FIRST)

            -- Write initial data
            ram:tick(0, 0, {1, 0, 1, 0}, 1)
            ram:tick(1, 0, {1, 0, 1, 0}, 1)

            -- Overwrite: ReadFirst returns OLD value
            ram:tick(0, 0, {0, 1, 0, 1}, 1)
            local out = ram:tick(1, 0, {0, 1, 0, 1}, 1)
            assert_array_equal(out, {1, 0, 1, 0})
        end)

        it("data is actually written despite returning old value", function()
            local ram = SinglePortRAM.new(4, 4, READ_FIRST)

            ram:tick(0, 0, {1, 0, 1, 0}, 1)
            ram:tick(1, 0, {1, 0, 1, 0}, 1)

            -- Overwrite
            ram:tick(0, 0, {0, 1, 0, 1}, 1)
            ram:tick(1, 0, {0, 1, 0, 1}, 1)

            -- Read back to verify new data is stored
            ram:tick(0, 0, {0, 0, 0, 0}, 0)
            local out = ram:tick(1, 0, {0, 0, 0, 0}, 0)
            assert_array_equal(out, {0, 1, 0, 1})
        end)
    end)

    describe("WriteFirst mode", function()
        it("returns new value during write", function()
            local ram = SinglePortRAM.new(4, 4, WRITE_FIRST)

            -- Write initial data
            ram:tick(0, 0, {1, 0, 1, 0}, 1)
            ram:tick(1, 0, {1, 0, 1, 0}, 1)

            -- Overwrite: WriteFirst returns NEW value
            ram:tick(0, 0, {0, 1, 0, 1}, 1)
            local out = ram:tick(1, 0, {0, 1, 0, 1}, 1)
            assert_array_equal(out, {0, 1, 0, 1})
        end)
    end)

    describe("NoChange mode", function()
        it("returns previous read value during write", function()
            local ram = SinglePortRAM.new(4, 4, NO_CHANGE)

            -- Read address 0 first (all zeros)
            ram:tick(0, 0, {0, 0, 0, 0}, 0)
            local out = ram:tick(1, 0, {0, 0, 0, 0}, 0)
            assert_array_equal(out, {0, 0, 0, 0})

            -- Write: NoChange returns previous read value (zeros)
            ram:tick(0, 0, {1, 1, 1, 1}, 1)
            out = ram:tick(1, 0, {1, 1, 1, 1}, 1)
            assert_array_equal(out, {0, 0, 0, 0})
        end)

        it("data is actually written", function()
            local ram = SinglePortRAM.new(4, 4, NO_CHANGE)

            ram:tick(0, 0, {0, 0, 0, 0}, 0)
            ram:tick(1, 0, {0, 0, 0, 0}, 0)

            ram:tick(0, 0, {1, 1, 1, 1}, 1)
            ram:tick(1, 0, {1, 1, 1, 1}, 1)

            -- Read back the written data
            ram:tick(0, 0, {0, 0, 0, 0}, 0)
            local out = ram:tick(1, 0, {0, 0, 0, 0}, 0)
            assert_array_equal(out, {1, 1, 1, 1})
        end)
    end)

    it("defaults to READ_FIRST when read_mode is nil", function()
        local ram = SinglePortRAM.new(4, 4)  -- no read_mode argument

        -- Write initial
        ram:tick(0, 0, {1, 0, 1, 0}, 1)
        ram:tick(1, 0, {1, 0, 1, 0}, 1)

        -- Overwrite: should behave as ReadFirst and return old value
        ram:tick(0, 0, {0, 1, 0, 1}, 1)
        local out = ram:tick(1, 0, {0, 1, 0, 1}, 1)
        assert_array_equal(out, {1, 0, 1, 0})
    end)

    it("handles multiple addresses", function()
        local ram = SinglePortRAM.new(4, 4, READ_FIRST)

        -- Write to different addresses
        for addr = 0, 3 do
            local data = {0, 0, 0, 0}
            data[addr + 1] = 1  -- Lua 1-indexed
            ram:tick(0, addr, data, 1)
            ram:tick(1, addr, data, 1)
        end

        -- Read back each address
        for addr = 0, 3 do
            local zeros = {0, 0, 0, 0}
            ram:tick(0, addr, zeros, 0)
            local out = ram:tick(1, addr, zeros, 0)
            local expected = {0, 0, 0, 0}
            expected[addr + 1] = 1
            assert_array_equal(out, expected)
        end
    end)

    it("returns last read when no rising edge", function()
        local ram = SinglePortRAM.new(4, 4, READ_FIRST)
        -- Without rising edge, output should be last read (all zeros)
        local out = ram:tick(0, 0, {1, 1, 1, 1}, 1)
        assert_array_equal(out, {0, 0, 0, 0})
    end)

    it("ignores falling edge", function()
        local ram = SinglePortRAM.new(4, 4, READ_FIRST)

        -- Write on rising edge (addr 0 was zeros, READ_FIRST returns old=zeros)
        ram:tick(0, 0, {1, 0, 1, 0}, 1)
        ram:tick(1, 0, {1, 0, 1, 0}, 1)

        -- Falling edge (1->0) should not trigger a new operation.
        -- last_read is the old value at addr 0 before the write, which was zeros.
        local out = ram:tick(0, 0, {0, 0, 0, 0}, 0)
        assert_array_equal(out, {0, 0, 0, 0})
    end)

    it("ignores high-to-high (no edge)", function()
        local ram = SinglePortRAM.new(4, 4, READ_FIRST)

        -- First rising edge writes {1,1,1,1}. READ_FIRST returns old (zeros).
        ram:tick(0, 0, {1, 1, 1, 1}, 1)
        ram:tick(1, 0, {1, 1, 1, 1}, 1)

        -- High-to-high: should not trigger another operation.
        -- last_read is the old value (zeros from READ_FIRST).
        local out = ram:tick(1, 0, {0, 0, 0, 0}, 1)
        assert_array_equal(out, {0, 0, 0, 0})
    end)

    it("exposes dump for inspection", function()
        local ram = SinglePortRAM.new(2, 4, READ_FIRST)

        ram:tick(0, 0, {1, 0, 1, 0}, 1)
        ram:tick(1, 0, {1, 0, 1, 0}, 1)

        local dump = ram:dump()
        assert.are.equal(2, #dump)
        assert_array_equal(dump[1], {1, 0, 1, 0})
        assert_array_equal(dump[2], {0, 0, 0, 0})
    end)

    it("reports depth and width", function()
        local ram = SinglePortRAM.new(256, 8, READ_FIRST)
        assert.are.equal(256, ram:depth())
        assert.are.equal(8, ram:width())
    end)

    describe("validation", function()
        it("errors on depth=0", function()
            assert.has_error(function() SinglePortRAM.new(0, 8, READ_FIRST) end)
        end)

        it("errors on width=0", function()
            assert.has_error(function() SinglePortRAM.new(4, 0, READ_FIRST) end)
        end)

        it("errors on address out of range", function()
            local ram = SinglePortRAM.new(4, 4, READ_FIRST)
            assert.has_error(function()
                ram:tick(1, 4, {0, 0, 0, 0}, 0)
            end)
        end)

        it("errors on negative address", function()
            local ram = SinglePortRAM.new(4, 4, READ_FIRST)
            assert.has_error(function()
                ram:tick(1, -1, {0, 0, 0, 0}, 0)
            end)
        end)

        it("errors on wrong data length", function()
            local ram = SinglePortRAM.new(4, 4, READ_FIRST)
            assert.has_error(function()
                ram:tick(1, 0, {0, 0}, 0)
            end)
        end)

        it("errors on bad clock signal", function()
            local ram = SinglePortRAM.new(4, 4, READ_FIRST)
            assert.has_error(function()
                ram:tick(2, 0, {0, 0, 0, 0}, 0)
            end)
        end)

        it("errors on bad write enable", function()
            local ram = SinglePortRAM.new(4, 4, READ_FIRST)
            assert.has_error(function()
                ram:tick(1, 0, {0, 0, 0, 0}, 2)
            end)
        end)

        it("errors on bad data bit", function()
            local ram = SinglePortRAM.new(4, 4, READ_FIRST)
            assert.has_error(function()
                ram:tick(1, 0, {0, 2, 0, 0}, 0)
            end)
        end)
    end)
end)

-- =========================================================================
-- DualPortRAM Tests
-- =========================================================================

describe("DualPortRAM", function()
    it("supports independent port operations", function()
        local ram = DualPortRAM.new(8, 4, READ_FIRST, READ_FIRST)
        local zeros = {0, 0, 0, 0}
        local data_a = {1, 0, 1, 0}
        local data_b = {0, 1, 0, 1}

        -- Write to addr 0 via port A, addr 1 via port B (simultaneously)
        ram:tick(0, 0, data_a, 1, 1, data_b, 1)
        ram:tick(1, 0, data_a, 1, 1, data_b, 1)

        -- Read back via opposite ports
        ram:tick(0, 1, zeros, 0, 0, zeros, 0)
        local out_a, out_b, err = ram:tick(1, 1, zeros, 0, 0, zeros, 0)
        assert.is_nil(err)
        assert_array_equal(out_a, data_b)  -- Port A reads addr 1
        assert_array_equal(out_b, data_a)  -- Port B reads addr 0
    end)

    it("detects write collision", function()
        local ram = DualPortRAM.new(4, 4, READ_FIRST, READ_FIRST)
        local data = {1, 1, 1, 1}

        -- Both ports write to same address
        ram:tick(0, 0, data, 1, 0, data, 1)
        local out_a, out_b, err = ram:tick(1, 0, data, 1, 0, data, 1)

        assert.is_nil(out_a)
        assert.is_nil(out_b)
        assert.is_not_nil(err)
        assert.truthy(string.find(err, "write collision"))
        assert.truthy(string.find(err, "address 0"))
    end)

    it("allows both ports to read same address", function()
        local ram = DualPortRAM.new(4, 4, READ_FIRST, READ_FIRST)
        local data = {1, 0, 1, 0}
        local zeros = {0, 0, 0, 0}

        -- Write data to addr 0
        ram:tick(0, 0, data, 1, 0, zeros, 0)
        ram:tick(1, 0, data, 1, 0, zeros, 0)

        -- Both ports read same address
        ram:tick(0, 0, zeros, 0, 0, zeros, 0)
        local out_a, out_b, err = ram:tick(1, 0, zeros, 0, 0, zeros, 0)
        assert.is_nil(err)
        assert_array_equal(out_a, data)
        assert_array_equal(out_b, data)
    end)

    it("allows write on one port and read on another at different addresses", function()
        local ram = DualPortRAM.new(4, 4, READ_FIRST, READ_FIRST)
        local data_a = {1, 1, 0, 0}
        local zeros = {0, 0, 0, 0}

        -- Port A writes to addr 0, Port B reads addr 1 (zeros)
        ram:tick(0, 0, data_a, 1, 1, zeros, 0)
        local out_a, out_b, err = ram:tick(1, 0, data_a, 1, 1, zeros, 0)

        assert.is_nil(err)
        -- Port A: ReadFirst returns old value at addr 0 (zeros)
        assert_array_equal(out_a, zeros)
        -- Port B: reads addr 1 (zeros)
        assert_array_equal(out_b, zeros)
    end)

    it("returns last read when no rising edge", function()
        local ram = DualPortRAM.new(4, 4, READ_FIRST, READ_FIRST)
        local zeros = {0, 0, 0, 0}

        local out_a, out_b, err = ram:tick(0, 0, zeros, 0, 0, zeros, 0)
        assert.is_nil(err)
        assert_array_equal(out_a, zeros)
        assert_array_equal(out_b, zeros)
    end)

    describe("WriteFirst mode", function()
        it("returns new value during write", function()
            local ram = DualPortRAM.new(4, 4, WRITE_FIRST, WRITE_FIRST)
            local zeros = {0, 0, 0, 0}
            local data = {1, 0, 1, 0}

            ram:tick(0, 0, data, 1, 0, zeros, 0)
            local out_a, _, err = ram:tick(1, 0, data, 1, 0, zeros, 0)
            assert.is_nil(err)
            assert_array_equal(out_a, data)
        end)
    end)

    describe("NoChange mode", function()
        it("returns previous read value during write", function()
            local ram = DualPortRAM.new(4, 4, NO_CHANGE, NO_CHANGE)
            local zeros = {0, 0, 0, 0}
            local data = {1, 1, 1, 1}

            -- Write via port A: NoChange returns previous read (zeros)
            ram:tick(0, 0, data, 1, 0, zeros, 0)
            local out_a, _, err = ram:tick(1, 0, data, 1, 0, zeros, 0)
            assert.is_nil(err)
            assert_array_equal(out_a, zeros)
        end)
    end)

    describe("mixed read modes", function()
        it("supports different read modes per port", function()
            local ram = DualPortRAM.new(4, 4, READ_FIRST, WRITE_FIRST)
            local zeros = {0, 0, 0, 0}
            local data_a = {1, 0, 1, 0}
            local data_b = {0, 1, 0, 1}

            -- Write via both ports to different addresses
            ram:tick(0, 0, data_a, 1, 1, data_b, 1)
            local out_a, out_b, err = ram:tick(1, 0, data_a, 1, 1, data_b, 1)
            assert.is_nil(err)
            -- Port A (READ_FIRST): returns old value (zeros)
            assert_array_equal(out_a, zeros)
            -- Port B (WRITE_FIRST): returns new value
            assert_array_equal(out_b, data_b)
        end)
    end)

    it("reports depth and width", function()
        local ram = DualPortRAM.new(16, 8, READ_FIRST, READ_FIRST)
        assert.are.equal(16, ram:depth())
        assert.are.equal(8, ram:width())
    end)

    describe("validation", function()
        it("errors on depth=0", function()
            assert.has_error(function()
                DualPortRAM.new(0, 4, READ_FIRST, READ_FIRST)
            end)
        end)

        it("errors on width=0", function()
            assert.has_error(function()
                DualPortRAM.new(4, 0, READ_FIRST, READ_FIRST)
            end)
        end)

        it("errors on bad clock", function()
            local ram = DualPortRAM.new(4, 4, READ_FIRST, READ_FIRST)
            local zeros = {0, 0, 0, 0}
            assert.has_error(function()
                ram:tick(2, 0, zeros, 0, 0, zeros, 0)
            end)
        end)

        it("errors on bad write enable A", function()
            local ram = DualPortRAM.new(4, 4, READ_FIRST, READ_FIRST)
            local zeros = {0, 0, 0, 0}
            assert.has_error(function()
                ram:tick(1, 0, zeros, 2, 0, zeros, 0)
            end)
        end)

        it("errors on bad write enable B", function()
            local ram = DualPortRAM.new(4, 4, READ_FIRST, READ_FIRST)
            local zeros = {0, 0, 0, 0}
            assert.has_error(function()
                ram:tick(1, 0, zeros, 0, 0, zeros, 2)
            end)
        end)

        it("errors on address A out of range", function()
            local ram = DualPortRAM.new(4, 4, READ_FIRST, READ_FIRST)
            local zeros = {0, 0, 0, 0}
            assert.has_error(function()
                ram:tick(1, 4, zeros, 0, 0, zeros, 0)
            end)
        end)

        it("errors on address B out of range", function()
            local ram = DualPortRAM.new(4, 4, READ_FIRST, READ_FIRST)
            local zeros = {0, 0, 0, 0}
            assert.has_error(function()
                ram:tick(1, 0, zeros, 0, 4, zeros, 0)
            end)
        end)

        it("errors on data A wrong length", function()
            local ram = DualPortRAM.new(4, 4, READ_FIRST, READ_FIRST)
            local zeros = {0, 0, 0, 0}
            assert.has_error(function()
                ram:tick(1, 0, {0}, 0, 0, zeros, 0)
            end)
        end)

        it("errors on data B wrong length", function()
            local ram = DualPortRAM.new(4, 4, READ_FIRST, READ_FIRST)
            local zeros = {0, 0, 0, 0}
            assert.has_error(function()
                ram:tick(1, 0, zeros, 0, 0, {0}, 0)
            end)
        end)

        it("errors on bad bit in data A", function()
            local ram = DualPortRAM.new(4, 4, READ_FIRST, READ_FIRST)
            local zeros = {0, 0, 0, 0}
            assert.has_error(function()
                ram:tick(1, 0, {0, 2, 0, 0}, 0, 0, zeros, 0)
            end)
        end)

        it("errors on bad bit in data B", function()
            local ram = DualPortRAM.new(4, 4, READ_FIRST, READ_FIRST)
            local zeros = {0, 0, 0, 0}
            assert.has_error(function()
                ram:tick(1, 0, zeros, 0, 0, {0, 0, 2, 0}, 0)
            end)
        end)

        it("errors on negative address A", function()
            local ram = DualPortRAM.new(4, 4, READ_FIRST, READ_FIRST)
            local zeros = {0, 0, 0, 0}
            assert.has_error(function()
                ram:tick(1, -1, zeros, 0, 0, zeros, 0)
            end)
        end)

        it("errors on negative address B", function()
            local ram = DualPortRAM.new(4, 4, READ_FIRST, READ_FIRST)
            local zeros = {0, 0, 0, 0}
            assert.has_error(function()
                ram:tick(1, 0, zeros, 0, -1, zeros, 0)
            end)
        end)
    end)
end)

-- =========================================================================
-- ConfigurableBRAM Tests
-- =========================================================================

describe("ConfigurableBRAM", function()
    it("reports correct initial properties", function()
        local bram = ConfigurableBRAM.new(1024, 8)
        assert.are.equal(128, bram:depth())
        assert.are.equal(8, bram:width())
        assert.are.equal(1024, bram:total_bits())
    end)

    it("writes and reads via port A", function()
        local bram = ConfigurableBRAM.new(256, 8)
        local data = {1, 0, 1, 0, 0, 1, 0, 1}
        local zeros = {0, 0, 0, 0, 0, 0, 0, 0}

        -- Write via port A
        bram:tick_a(0, 0, data, 1)
        bram:tick_a(1, 0, data, 1)

        -- Read via port A
        bram:tick_a(0, 0, zeros, 0)
        local out = bram:tick_a(1, 0, zeros, 0)
        assert_array_equal(out, data)
    end)

    it("writes and reads via port B", function()
        local bram = ConfigurableBRAM.new(256, 8)
        local data = {0, 1, 0, 1, 1, 0, 1, 0}
        local zeros = {0, 0, 0, 0, 0, 0, 0, 0}

        -- Write via port B
        bram:tick_b(0, 0, data, 1)
        bram:tick_b(1, 0, data, 1)

        -- Read via port B
        bram:tick_b(0, 0, zeros, 0)
        local out = bram:tick_b(1, 0, zeros, 0)
        assert_array_equal(out, data)
    end)

    describe("reconfigure", function()
        it("changes depth and width", function()
            local bram = ConfigurableBRAM.new(1024, 8)
            assert.are.equal(128, bram:depth())
            assert.are.equal(8, bram:width())

            bram:reconfigure(16)
            assert.are.equal(64, bram:depth())
            assert.are.equal(16, bram:width())
        end)

        it("clears data after reconfigure", function()
            local bram = ConfigurableBRAM.new(1024, 8)

            -- Write some data
            local data = {1, 1, 1, 1, 1, 1, 1, 1}
            bram:tick_a(0, 0, data, 1)
            bram:tick_a(1, 0, data, 1)

            -- Reconfigure to 16-bit width
            bram:reconfigure(16)

            -- Old data should be cleared
            local zeros = {}
            for i = 1, 16 do zeros[i] = 0 end
            bram:tick_a(0, 0, zeros, 0)
            local out = bram:tick_a(1, 0, zeros, 0)
            assert_array_equal(out, zeros)
        end)

        it("supports 1-bit width", function()
            local bram = ConfigurableBRAM.new(1024, 8)
            bram:reconfigure(1)
            assert.are.equal(1024, bram:depth())
            assert.are.equal(1, bram:width())
        end)

        it("supports 32-bit width", function()
            local bram = ConfigurableBRAM.new(1024, 8)
            bram:reconfigure(32)
            assert.are.equal(32, bram:depth())
            assert.are.equal(32, bram:width())
        end)

        it("total_bits stays constant after reconfigure", function()
            local bram = ConfigurableBRAM.new(1024, 8)
            bram:reconfigure(16)
            assert.are.equal(1024, bram:total_bits())
            bram:reconfigure(4)
            assert.are.equal(1024, bram:total_bits())
        end)
    end)

    it("handles multiple addresses", function()
        local bram = ConfigurableBRAM.new(64, 4)
        -- depth = 16

        -- Write to several addresses via port A
        for addr = 0, 3 do
            local data = {0, 0, 0, 0}
            data[addr + 1] = 1
            bram:tick_a(0, addr, data, 1)
            bram:tick_a(1, addr, data, 1)
        end

        -- Read back
        for addr = 0, 3 do
            local zeros = {0, 0, 0, 0}
            bram:tick_a(0, addr, zeros, 0)
            local out = bram:tick_a(1, addr, zeros, 0)
            local expected = {0, 0, 0, 0}
            expected[addr + 1] = 1
            assert_array_equal(out, expected)
        end
    end)

    it("errors on bad clock for tick_a", function()
        local bram = ConfigurableBRAM.new(64, 4)
        assert.has_error(function()
            bram:tick_a(2, 0, {0, 0, 0, 0}, 0)
        end)
    end)

    it("errors on bad clock for tick_b", function()
        local bram = ConfigurableBRAM.new(64, 4)
        assert.has_error(function()
            bram:tick_b(2, 0, {0, 0, 0, 0}, 0)
        end)
    end)

    describe("validation", function()
        it("errors on total_bits=0", function()
            assert.has_error(function() ConfigurableBRAM.new(0, 8) end)
        end)

        it("errors on width=0", function()
            assert.has_error(function() ConfigurableBRAM.new(1024, 0) end)
        end)

        it("errors when width does not divide total_bits", function()
            assert.has_error(function() ConfigurableBRAM.new(1024, 3) end)
        end)

        it("errors on reconfigure with width=0", function()
            local bram = ConfigurableBRAM.new(1024, 8)
            assert.has_error(function() bram:reconfigure(0) end)
        end)

        it("errors on reconfigure when width does not divide total_bits", function()
            local bram = ConfigurableBRAM.new(1024, 8)
            assert.has_error(function() bram:reconfigure(3) end)
        end)
    end)
end)

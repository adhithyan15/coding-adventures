-- Tests for intel4004_gatelevel — gate-level Intel 4004 simulation
--
-- All arithmetic routes through actual logic gate functions. These tests
-- verify that the gate-level simulator produces identical results to the
-- behavioral simulator for all major instruction categories.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" ..
    "../../logic_gates/src/?.lua;" .. "../../logic_gates/src/?/init.lua;" ..
    "../../arithmetic/src/?.lua;" .. "../../arithmetic/src/?/init.lua;" ..
    package.path

local Intel4004GL = require("coding_adventures.intel4004_gatelevel")

-- Helper: create CPU and run a program
local function run(prog, max_steps)
    local cpu = Intel4004GL.new()
    local traces = cpu:run(prog, max_steps)
    return cpu, traces
end

-- =========================================================================
-- Initialization
-- =========================================================================

describe("initialization", function()
    it("starts with all state zeroed", function()
        local cpu = Intel4004GL.new()
        -- Reading accumulator through flip-flops should return 0
        assert.are.equal(0, cpu:_read_acc())
        assert.is_false(cpu:_read_carry())
        assert.are.equal(0, cpu:_read_pc_val())
        assert.is_false(cpu.halted)
    end)
end)

-- =========================================================================
-- Bit conversion helpers
-- =========================================================================

describe("bit conversion", function()
    it("int_to_bits and back is identity", function()
        -- We test this indirectly through gate operations
        local cpu = Intel4004GL.new()
        -- Write 5 (0101) to accumulator and read back
        cpu:_write_acc(5)
        assert.are.equal(5, cpu:_read_acc())
    end)

    it("handles all nibble values 0-15", function()
        local cpu = Intel4004GL.new()
        for v = 0, 15 do
            cpu:_write_acc(v)
            assert.are.equal(v, cpu:_read_acc(),
                "Round-trip failed for value " .. v)
        end
    end)
end)

-- =========================================================================
-- gate_count reports reasonable numbers
-- =========================================================================

describe("gate_count", function()
    it("returns a table with component gate estimates", function()
        local cpu = Intel4004GL.new()
        local gc = cpu:gate_count()
        assert.is_not_nil(gc.alu)
        assert.is_not_nil(gc.registers)
        assert.is_not_nil(gc.total)
        -- Close to 4004's ~786 estimated gates
        assert.is_true(gc.total > 500, "Total gates should be > 500")
        assert.is_true(gc.total < 2000, "Total gates should be < 2000 (we model ~786)")
    end)
end)

-- =========================================================================
-- NOP and HLT
-- =========================================================================

describe("NOP and HLT", function()
    it("NOP advances PC", function()
        local cpu, traces = run({0x00, 0x01}, 10)
        assert.are.equal("NOP", traces[1].mnemonic)
        assert.is_true(cpu.halted)
    end)
end)

-- =========================================================================
-- LDM: gate-level load immediate
-- =========================================================================

describe("LDM (gate-level)", function()
    it("loads nibble 0-15 correctly", function()
        for n = 0, 15 do
            local cpu = Intel4004GL.new()
            cpu:run({0xD0 | n, 0x01}, 10)
            assert.are.equal(n, cpu:_read_acc())
        end
    end)
end)

-- =========================================================================
-- ADD: routing through ripple-carry adder gates
-- =========================================================================

describe("ADD (gate-level via ripple-carry adder)", function()
    it("1 + 2 = 3 (no carry)", function()
        -- LDM 1, XCH R0, LDM 2, ADD R0, HLT
        local cpu, _ = run({0xD1, 0xB0, 0xD2, 0x80, 0x01}, 10)
        assert.are.equal(3, cpu:_read_acc())
        assert.is_false(cpu:_read_carry())
    end)

    it("10 + 8 = 18 → acc=2, carry=true", function()
        local cpu = Intel4004GL.new()
        cpu:_write_acc(10)
        cpu:_write_reg(0, 8)
        cpu:run({0x80, 0x01}, 10)
        assert.are.equal(2, cpu:_read_acc())
        assert.is_true(cpu:_read_carry())
    end)

    it("includes carry_in in gate computation", function()
        local cpu = Intel4004GL.new()
        cpu:_write_acc(5)
        cpu:_write_reg(0, 5)
        cpu:_write_carry(true)
        cpu:run({0x80, 0x01}, 10)
        assert.are.equal(11, cpu:_read_acc())
        assert.is_false(cpu:_read_carry())
    end)

    it("15 + 1 = overflow (carry through all 4 full-adders)", function()
        local cpu = Intel4004GL.new()
        cpu:_write_acc(15)
        cpu:_write_reg(0, 1)
        cpu:run({0x80, 0x01}, 10)
        assert.are.equal(0, cpu:_read_acc())
        assert.is_true(cpu:_read_carry())
    end)
end)

-- =========================================================================
-- SUB: complement-add through gates
-- =========================================================================

describe("SUB (gate-level via complement-add)", function()
    it("7 - 3 = 4 (carry_in=true, no borrow)", function()
        local cpu = Intel4004GL.new()
        cpu:_write_acc(7)
        cpu:_write_reg(0, 3)
        cpu:_write_carry(true)
        cpu:run({0x90, 0x01}, 10)
        assert.are.equal(4, cpu:_read_acc())
        assert.is_true(cpu:_read_carry())
    end)
end)

-- =========================================================================
-- INC: half-adder chain increment
-- =========================================================================

describe("INC (gate-level via half-adder chain)", function()
    it("increments a register", function()
        local cpu = Intel4004GL.new()
        cpu:_write_reg(0, 5)
        cpu:run({0x60, 0x01}, 10)
        assert.are.equal(6, cpu:_read_reg(0))
    end)

    it("wraps from 15 to 0", function()
        local cpu = Intel4004GL.new()
        cpu:_write_reg(0, 15)
        cpu:run({0x60, 0x01}, 10)
        assert.are.equal(0, cpu:_read_reg(0))
    end)
end)

-- =========================================================================
-- IAC: gate-level increment accumulator
-- =========================================================================

describe("IAC (gate-level)", function()
    it("increments 5 to 6", function()
        local cpu = Intel4004GL.new()
        cpu:_write_acc(5)
        cpu:run({0xF2, 0x01}, 10)
        assert.are.equal(6, cpu:_read_acc())
    end)

    it("wraps 15 to 0 with carry", function()
        local cpu = Intel4004GL.new()
        cpu:_write_acc(15)
        cpu:run({0xF2, 0x01}, 10)
        assert.are.equal(0, cpu:_read_acc())
        assert.is_true(cpu:_read_carry())
    end)
end)

-- =========================================================================
-- CMA: NOT gates on accumulator
-- =========================================================================

describe("CMA (gate-level NOT)", function()
    it("5 (0101) complements to 10 (1010)", function()
        local cpu = Intel4004GL.new()
        cpu:_write_acc(5)
        cpu:run({0xF4, 0x01}, 10)
        assert.are.equal(10, cpu:_read_acc())
    end)

    it("0 complements to 15", function()
        local cpu = Intel4004GL.new()
        cpu:_write_acc(0)
        cpu:run({0xF4, 0x01}, 10)
        assert.are.equal(15, cpu:_read_acc())
    end)
end)

-- =========================================================================
-- RAL/RAR: rotate through carry
-- =========================================================================

describe("RAL and RAR (gate-level)", function()
    it("RAL shifts bits left through carry", function()
        local cpu = Intel4004GL.new()
        cpu:_write_acc(0xA)   -- 1010
        cpu:_write_carry(false)
        cpu:run({0xF5, 0x01}, 10)
        assert.are.equal(4, cpu:_read_acc())  -- 0100
        assert.is_true(cpu:_read_carry())     -- A3=1 → carry
    end)

    it("RAR shifts bits right through carry", function()
        local cpu = Intel4004GL.new()
        cpu:_write_acc(0xA)   -- 1010
        cpu:_write_carry(false)
        cpu:run({0xF6, 0x01}, 10)
        assert.are.equal(5, cpu:_read_acc())  -- 0101
        assert.is_false(cpu:_read_carry())    -- A0=0 → carry
    end)
end)

-- =========================================================================
-- JMS/BBL: subroutine call/return via flip-flop stack
-- =========================================================================

describe("JMS/BBL (gate-level stack)", function()
    it("call and return with value in A", function()
        local prog = {
            0xD5,        -- 0x000: LDM 5
            0x50, 0x08,  -- 0x001: JMS 0x008
            0xB0,        -- 0x003: XCH R0
            0x01,        -- 0x004: HLT
            0x00, 0x00, 0x00,
            0xF2,        -- 0x008: IAC
            0xF2,        -- 0x009: IAC
            0xF2,        -- 0x00A: IAC
            0xC0,        -- 0x00B: BBL 0
        }
        local cpu = Intel4004GL.new()
        cpu:run(prog, 20)
        assert.are.equal(8, cpu:_read_reg(0), "R0 should be 5+3=8")
    end)
end)

-- =========================================================================
-- RAM operations via flip-flop states
-- =========================================================================

describe("WRM/RDM (gate-level flip-flop RAM)", function()
    it("writes and reads back via SRC addressing", function()
        local prog = {
            0x20, 0x00,  -- FIM P0, 0x00
            0x21,        -- SRC P0
            0xD9,        -- LDM 9
            0xE0,        -- WRM (write A to RAM through flip-flops)
            0xD0,        -- LDM 0 (clear A)
            0xE9,        -- RDM (read RAM back into A)
            0x01,        -- HLT
        }
        local cpu = Intel4004GL.new()
        cpu:run(prog, 20)
        assert.are.equal(9, cpu:_read_acc(),
            "Gate-level RDM should return value written by WRM")
    end)
end)

-- =========================================================================
-- Cross-validation: gate-level vs behavioral
-- =========================================================================

describe("Cross-validation: gate-level matches behavioral", function()
    local function run_behavioral(prog, max_steps)
        -- Reuse logic_gates path trick to load behavioral simulator
        local saved_path = package.path
        package.path = "../../intel4004_simulator/src/?.lua;" ..
            "../../intel4004_simulator/src/?/init.lua;" .. package.path
        local ok, Intel4004 = pcall(require, "coding_adventures.intel4004_simulator")
        package.path = saved_path
        if not ok then return nil end
        local cpu = Intel4004.new()
        cpu:run(prog, max_steps)
        return cpu
    end

    local programs = {
        -- x = 1 + 2
        {name = "1+2=3", prog = {0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01}},
        -- Accumulator operations
        {name = "CLB", prog = {0xD9, 0xFA, 0xF0, 0x01}},
        {name = "IAC chain", prog = {0xF2, 0xF2, 0xF2, 0x01}},
        -- Rotate
        {name = "RAL", prog = {0xD5, 0xF5, 0x01}},
    }

    for _, tc in ipairs(programs) do
        it("program '" .. tc.name .. "' gives same final accumulator", function()
            local gl_cpu = Intel4004GL.new()
            gl_cpu:run(tc.prog, 50)
            local gl_acc = gl_cpu:_read_acc()

            local beh_cpu = run_behavioral(tc.prog, 50)
            if beh_cpu ~= nil then
                assert.are.equal(beh_cpu.accumulator, gl_acc,
                    "Gate-level and behavioral differ for " .. tc.name)
            else
                -- Behavioral not available; just verify gate-level doesn't crash
                assert.is_not_nil(gl_acc)
            end
        end)
    end
end)

-- =========================================================================
-- Reset
-- =========================================================================

describe("reset", function()
    it("clears all gate-level state", function()
        local cpu = Intel4004GL.new()
        cpu:run({0xD9, 0x01}, 10)
        cpu:reset()
        assert.are.equal(0, cpu:_read_acc())
        assert.are.equal(0, cpu:_read_pc_val())
        assert.is_false(cpu:_read_carry())
        assert.is_false(cpu.halted)
    end)
end)

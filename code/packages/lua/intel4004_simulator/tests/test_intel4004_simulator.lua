-- Tests for intel4004_simulator — comprehensive coverage of the Intel 4004 ISA
--
-- We test every instruction category plus end-to-end programs from the spec.
-- Target: 95%+ coverage.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local Intel4004 = require("coding_adventures.intel4004_simulator")

-- =========================================================================
-- Helper: create a fresh CPU and run a byte-string program
-- =========================================================================
local function run(bytes_table, max_steps)
    local cpu = Intel4004.new()
    local traces = cpu:run(bytes_table, max_steps)
    return cpu, traces
end

-- =========================================================================
-- Basic state initialization
-- =========================================================================

describe("initialization", function()
    it("starts with all registers zeroed", function()
        local cpu = Intel4004.new()
        assert.are.equal(0, cpu.accumulator)
        assert.is_false(cpu.carry)
        assert.are.equal(0, cpu.pc)
        assert.is_false(cpu.halted)
        for i = 1, 16 do
            assert.are.equal(0, cpu.registers[i])
        end
    end)
end)

-- =========================================================================
-- NOP and HLT
-- =========================================================================

describe("NOP and HLT", function()
    it("NOP advances PC and does nothing else", function()
        local cpu, traces = run({0x00, 0x01}, 10)
        assert.are.equal(2, #traces)
        assert.are.equal("NOP", traces[1].mnemonic)
        assert.is_true(cpu.halted)
    end)

    it("HLT stops execution", function()
        local cpu = Intel4004.new()
        cpu:run({0x01}, 100)
        assert.is_true(cpu.halted)
    end)

    it("step after halt raises error", function()
        local cpu = Intel4004.new()
        cpu:run({0x01}, 10)
        assert.has_error(function() cpu:step() end)
    end)
end)

-- =========================================================================
-- LDM: Load immediate into accumulator
-- =========================================================================

describe("LDM", function()
    it("loads each nibble value 0-15 correctly", function()
        for n = 0, 15 do
            local cpu = Intel4004.new()
            cpu:run({0xD0 | n, 0x01}, 10)
            assert.are.equal(n, cpu.accumulator,
                "LDM " .. n .. " should set A = " .. n)
        end
    end)

    it("trace shows correct mnemonic", function()
        local cpu, traces = run({0xD5, 0x01}, 10)
        assert.are.equal("LDM 5", traces[1].mnemonic)
        assert.are.equal(0, traces[1].accumulator_before)
        assert.are.equal(5, traces[1].accumulator_after)
    end)
end)

-- =========================================================================
-- LD: Load register into accumulator
-- =========================================================================

describe("LD", function()
    it("loads each register into accumulator", function()
        for reg = 0, 15 do
            local cpu = Intel4004.new()
            cpu.registers[reg + 1] = 7
            cpu:run({0xA0 | reg, 0x01}, 10)
            assert.are.equal(7, cpu.accumulator)
        end
    end)
end)

-- =========================================================================
-- XCH: Exchange accumulator and register
-- =========================================================================

describe("XCH", function()
    it("swaps accumulator and register", function()
        local cpu = Intel4004.new()
        cpu:run({0xD5, 0xB0, 0x01}, 10)  -- LDM 5, XCH R0, HLT
        assert.are.equal(0, cpu.accumulator)   -- A had 5, swapped with R0=0
        assert.are.equal(5, cpu.registers[1])  -- R0 now has 5
    end)

    it("XCH trace shows before/after", function()
        local cpu, traces = run({0xD5, 0xB0, 0x01}, 10)
        assert.are.equal("XCH R0", traces[2].mnemonic)
        assert.are.equal(5, traces[2].accumulator_before)
        assert.are.equal(0, traces[2].accumulator_after)
    end)
end)

-- =========================================================================
-- INC: Increment register
-- =========================================================================

describe("INC", function()
    it("increments a register", function()
        local cpu = Intel4004.new()
        cpu.registers[3] = 5    -- R2 = 5
        cpu:run({0x62, 0x01}, 10)   -- INC R2
        assert.are.equal(6, cpu.registers[3])
    end)

    it("wraps from 15 to 0", function()
        local cpu = Intel4004.new()
        cpu.registers[1] = 15
        cpu:run({0x60, 0x01}, 10)
        assert.are.equal(0, cpu.registers[1])
    end)

    it("does NOT affect carry", function()
        local cpu = Intel4004.new()
        cpu.carry = false
        cpu.registers[1] = 15
        cpu:run({0x60, 0x01}, 10)
        assert.is_false(cpu.carry)
    end)
end)

-- =========================================================================
-- ADD: Add register to accumulator with carry
-- =========================================================================

describe("ADD", function()
    it("adds register to accumulator without carry", function()
        -- 1 + 2 = 3
        local cpu, _ = run({0xD1, 0xB0, 0xD2, 0x80, 0x01}, 10)
        -- LDM 1, XCH R0, LDM 2, ADD R0, HLT
        assert.are.equal(3, cpu.accumulator)
    end)

    it("sets carry on overflow", function()
        -- A=10, R0=8: 10+8=18 > 15, carry set
        local cpu = Intel4004.new()
        cpu.accumulator    = 10
        cpu.registers[1]   = 8
        cpu:run({0x80, 0x01}, 10)
        assert.are.equal(2, cpu.accumulator)  -- 18 & 0xF = 2
        assert.is_true(cpu.carry)
    end)

    it("includes carry_in in the addition", function()
        -- A=5, R0=5, carry=true: 5+5+1=11
        local cpu = Intel4004.new()
        cpu.accumulator  = 5
        cpu.registers[1] = 5
        cpu.carry        = true
        cpu:run({0x80, 0x01}, 10)
        assert.are.equal(11, cpu.accumulator)
        assert.is_false(cpu.carry)
    end)
end)

-- =========================================================================
-- SUB: Subtract register from accumulator
-- =========================================================================

describe("SUB", function()
    it("subtracts register from accumulator (no borrow)", function()
        -- A=7, R0=3, carry=true (no borrow): 7 + ~3 + 0 = 7 + 12 = 19 → 3, carry=1
        local cpu = Intel4004.new()
        cpu.accumulator  = 7
        cpu.registers[1] = 3
        cpu.carry        = true   -- no borrow
        cpu:run({0x90, 0x01}, 10)
        assert.are.equal(4, cpu.accumulator)
        assert.is_true(cpu.carry)
    end)

    it("borrows when A < Rn (carry=false means borrow)", function()
        -- A=3, R0=7, carry=true: 3 + ~7 + 0 = 3 + 8 = 11 → carry=0 (borrow occurred)
        local cpu = Intel4004.new()
        cpu.accumulator  = 3
        cpu.registers[1] = 7
        cpu.carry        = true
        cpu:run({0x90, 0x01}, 10)
        assert.is_false(cpu.carry, "Borrow should clear carry")
    end)
end)

-- =========================================================================
-- CLB, CLC, STC, CMC
-- =========================================================================

describe("CLB/CLC/STC/CMC", function()
    it("CLB clears accumulator and carry", function()
        local cpu = Intel4004.new()
        cpu.accumulator = 0xF
        cpu.carry = true
        cpu:run({0xF0, 0x01}, 10)
        assert.are.equal(0, cpu.accumulator)
        assert.is_false(cpu.carry)
    end)

    it("CLC clears carry only", function()
        local cpu = Intel4004.new()
        cpu.accumulator = 7
        cpu.carry = true
        cpu:run({0xF1, 0x01}, 10)
        assert.are.equal(7, cpu.accumulator)
        assert.is_false(cpu.carry)
    end)

    it("STC sets carry", function()
        local cpu = Intel4004.new()
        cpu.carry = false
        cpu:run({0xFA, 0x01}, 10)
        assert.is_true(cpu.carry)
    end)

    it("CMC complements carry (false -> true)", function()
        local cpu = Intel4004.new()
        cpu.carry = false
        cpu:run({0xF3, 0x01}, 10)
        assert.is_true(cpu.carry)
    end)

    it("CMC complements carry (true -> false)", function()
        local cpu = Intel4004.new()
        cpu.carry = true
        cpu:run({0xF3, 0x01}, 10)
        assert.is_false(cpu.carry)
    end)
end)

-- =========================================================================
-- IAC, DAC
-- =========================================================================

describe("IAC and DAC", function()
    it("IAC increments accumulator", function()
        local cpu = Intel4004.new()
        cpu.accumulator = 5
        cpu:run({0xF2, 0x01}, 10)
        assert.are.equal(6, cpu.accumulator)
        assert.is_false(cpu.carry)
    end)

    it("IAC wraps from 15 and sets carry", function()
        local cpu = Intel4004.new()
        cpu.accumulator = 15
        cpu:run({0xF2, 0x01}, 10)
        assert.are.equal(0, cpu.accumulator)
        assert.is_true(cpu.carry)
    end)

    it("DAC decrements accumulator", function()
        local cpu = Intel4004.new()
        cpu.accumulator = 5
        cpu:run({0xF8, 0x01}, 10)
        assert.are.equal(4, cpu.accumulator)
        assert.is_true(cpu.carry)  -- no borrow
    end)

    it("DAC wraps from 0 to 15 and clears carry", function()
        local cpu = Intel4004.new()
        cpu.accumulator = 0
        cpu:run({0xF8, 0x01}, 10)
        assert.are.equal(15, cpu.accumulator)
        assert.is_false(cpu.carry)  -- borrow occurred
    end)
end)

-- =========================================================================
-- CMA: Complement accumulator
-- =========================================================================

describe("CMA", function()
    it("inverts all 4 bits", function()
        local cpu = Intel4004.new()
        cpu.accumulator = 5   -- 0101 → ~0101 = 1010 = 10
        cpu:run({0xF4, 0x01}, 10)
        assert.are.equal(10, cpu.accumulator)
    end)

    it("complement of 0 is 15", function()
        local cpu = Intel4004.new()
        cpu.accumulator = 0
        cpu:run({0xF4, 0x01}, 10)
        assert.are.equal(15, cpu.accumulator)
    end)
end)

-- =========================================================================
-- RAL, RAR: Rotate through carry
-- =========================================================================

describe("RAL and RAR", function()
    it("RAL rotates left: A3 goes to carry, old carry goes to A0", function()
        local cpu = Intel4004.new()
        cpu.accumulator = 0xA  -- 1010
        cpu.carry       = false
        cpu:run({0xF5, 0x01}, 10)
        -- Shift left: 1010 << 1 = 0100 with A3=1 going to carry, old_carry=0 to A0
        -- Result: 0100 | 0 = 0100 = 4, carry = 1
        assert.are.equal(4, cpu.accumulator)
        assert.is_true(cpu.carry)
    end)

    it("RAR rotates right: A0 goes to carry, old carry goes to A3", function()
        local cpu = Intel4004.new()
        cpu.accumulator = 0xA  -- 1010
        cpu.carry       = false
        cpu:run({0xF6, 0x01}, 10)
        -- Shift right: 1010 >> 1 = 0101 with A0=0 going to carry, old_carry=0 to A3
        -- Result: 0101 | (0 << 3) = 0101 = 5, carry = 0
        assert.are.equal(5, cpu.accumulator)
        assert.is_false(cpu.carry)
    end)
end)

-- =========================================================================
-- TCC, TCS
-- =========================================================================

describe("TCC and TCS", function()
    it("TCC transfers carry=true to A=1, clears carry", function()
        local cpu = Intel4004.new()
        cpu.carry = true
        cpu:run({0xF7, 0x01}, 10)
        assert.are.equal(1, cpu.accumulator)
        assert.is_false(cpu.carry)
    end)

    it("TCC transfers carry=false to A=0, clears carry", function()
        local cpu = Intel4004.new()
        cpu.carry = false
        cpu:run({0xF7, 0x01}, 10)
        assert.are.equal(0, cpu.accumulator)
    end)

    it("TCS sets A=10 when carry=true", function()
        local cpu = Intel4004.new()
        cpu.carry = true
        cpu:run({0xF9, 0x01}, 10)
        assert.are.equal(10, cpu.accumulator)
        assert.is_false(cpu.carry)
    end)

    it("TCS sets A=9 when carry=false", function()
        local cpu = Intel4004.new()
        cpu.carry = false
        cpu:run({0xF9, 0x01}, 10)
        assert.are.equal(9, cpu.accumulator)
    end)
end)

-- =========================================================================
-- DAA: Decimal adjust accumulator (BCD correction)
-- =========================================================================

describe("DAA", function()
    it("no adjustment when A <= 9 and no carry", function()
        local cpu = Intel4004.new()
        cpu.accumulator = 7
        cpu.carry = false
        cpu:run({0xFB, 0x01}, 10)
        assert.are.equal(7, cpu.accumulator)
    end)

    it("adds 6 when A > 9", function()
        -- A = 12 (BCD overflow): 12 + 6 = 18, A=2, carry remains (was false)
        local cpu = Intel4004.new()
        cpu.accumulator = 12
        cpu.carry = false
        cpu:run({0xFB, 0x01}, 10)
        assert.are.equal(2, cpu.accumulator)
    end)

    it("BCD addition: 7 + 8 = 15 → DAA → 5 with carry", function()
        -- From spec: LDM 7, XCH R0, LDM 8, ADD R0, DAA, XCH R0, HLT
        local cpu, _ = run({0xD7, 0xB0, 0xD8, 0x80, 0xFB, 0xB0, 0x01}, 10)
        assert.are.equal(5, cpu.registers[1])  -- R0 = 5 (low BCD digit)
        assert.is_true(cpu.carry, "Carry = high BCD digit")
    end)
end)

-- =========================================================================
-- KBP: Keyboard process (1-hot to binary)
-- =========================================================================

describe("KBP", function()
    local cases = {
        {0, 0}, {1, 1}, {2, 2}, {4, 3}, {8, 4},
        {3, 15}, {5, 15}, {6, 15}, {7, 15},   -- invalid inputs → 15
    }
    for _, c in ipairs(cases) do
        local input, expected = c[1], c[2]
        it(string.format("KBP(%d) = %d", input, expected), function()
            local cpu = Intel4004.new()
            cpu.accumulator = input
            cpu:run({0xFC, 0x01}, 10)
            assert.are.equal(expected, cpu.accumulator)
        end)
    end
end)

-- =========================================================================
-- JUN: Unconditional jump
-- =========================================================================

describe("JUN", function()
    it("jumps to 12-bit address", function()
        -- JUN to address 0x006, NOP, NOP, NOP, NOP, NOP, LDM 7, HLT
        local prog = {
            0x40, 0x06,  -- JUN 0x006
            0x00,        -- NOP (skipped)
            0x00, 0x00, 0x00,
            0xD7,        -- LDM 7  (at 0x006)
            0x01,        -- HLT
        }
        local cpu = Intel4004.new()
        cpu:run(prog, 20)
        assert.are.equal(7, cpu.accumulator, "JUN should skip to LDM 7")
    end)
end)

-- =========================================================================
-- JCN: Conditional jump
-- =========================================================================

describe("JCN", function()
    it("jumps when accumulator is zero (condition 0x4)", function()
        -- JCN 4, target  (test_zero=true)
        -- Target is page-relative: addr_after_jcn & 0xF00 | raw2
        -- Instruction at 0x000, 2 bytes, raw2 = 0x05 → jump to 0x005
        local prog = {
            0x14, 0x05,  -- JCN 4, 0x05 (jump if A==0)
            0xD9,        -- LDM 9  (skipped if A==0)
            0x00, 0x00,
            0xD7,        -- LDM 7  (at 0x005)
            0x01,        -- HLT
        }
        local cpu = Intel4004.new()
        cpu.accumulator = 0    -- A=0 → condition met
        cpu:run(prog, 20)
        assert.are.equal(7, cpu.accumulator, "JCN should branch when A=0")
    end)

    it("does NOT jump when condition is false", function()
        local prog = {
            0x14, 0x05,  -- JCN 4, 0x05 (jump if A==0)
            0xD9,        -- LDM 9  (executed when A!=0)
            0x01,        -- HLT
            0x00, 0x00,
            0xD7,        -- LDM 7  (not reached)
            0x01,
        }
        local cpu = Intel4004.new()
        cpu.accumulator = 5    -- A=5 → condition NOT met
        cpu:run(prog, 20)
        assert.are.equal(9, cpu.accumulator)
    end)

    it("inversion bit (0x8) inverts test result", function()
        -- JCN 0xC = 0x8 | 0x4: invert(test_zero)  → jump if A != 0
        local prog = {
            0x1C, 0x05,  -- JCN 12, 0x05 (jump if A != 0)
            0xD3,        -- LDM 3 (skipped)
            0x01, 0x00,
            0xD7,        -- LDM 7 (at 0x005)
            0x01,
        }
        local cpu = Intel4004.new()
        cpu.accumulator = 5   -- A != 0 → inverted test = jump
        cpu:run(prog, 20)
        assert.are.equal(7, cpu.accumulator)
    end)
end)

-- =========================================================================
-- JMS and BBL: Subroutine call/return
-- =========================================================================

describe("JMS and BBL", function()
    it("calls subroutine and returns with value in A", function()
        -- From spec: LDM 5, JMS ADD_THREE, XCH R0, HLT, (padding), ADD_THREE: IAC, IAC, IAC, BBL 0
        local prog = {
            0xD5,        -- 0x000: LDM 5
            0x50, 0x08,  -- 0x001: JMS 0x008 (ADD_THREE)
            0xB0,        -- 0x003: XCH R0
            0x01,        -- 0x004: HLT
            0x00, 0x00, 0x00, -- 0x005-0x007: padding
            0xF2,        -- 0x008: IAC
            0xF2,        -- 0x009: IAC
            0xF2,        -- 0x00A: IAC
            0xC0,        -- 0x00B: BBL 0
        }
        local cpu = Intel4004.new()
        cpu:run(prog, 20)
        assert.are.equal(8, cpu.registers[1], "R0 should be 5+3=8")
    end)

    it("BBL loads immediate into A", function()
        -- JMS sub, HLT ... sub: BBL 7
        local prog = {
            0x50, 0x04,  -- 0x000: JMS 0x004
            0x01,        -- 0x002: HLT
            0x00,        -- padding
            0xC7,        -- 0x004: BBL 7
        }
        local cpu = Intel4004.new()
        cpu:run(prog, 20)
        assert.are.equal(7, cpu.accumulator, "BBL 7 sets A=7")
    end)

    it("supports 2 levels of nesting", function()
        local prog = {
            0x50, 0x06,  -- 0x000: JMS 0x006
            0x01,        -- 0x002: HLT
            0x00, 0x00, 0x00,
            0x50, 0x0A,  -- 0x006: JMS 0x00A
            0xC5,        -- 0x008: BBL 5
            0x00,
            0xC3,        -- 0x00A: BBL 3
        }
        local cpu = Intel4004.new()
        cpu:run(prog, 20)
        assert.are.equal(5, cpu.accumulator, "Outer BBL 5 should win")
    end)
end)

-- =========================================================================
-- ISZ: Increment and skip if zero
-- =========================================================================

describe("ISZ", function()
    it("increments register and jumps when not zero", function()
        -- ISZ R0, 0x06 (page-relative)
        -- R0 starts at 0, increments to 1 → jump (non-zero)
        -- Skip the NOP at 0x002, land on LDM 7 at 0x006
        -- But ISZ: if val != 0 JUMP. If 0, continue.
        -- So R0=0 → R0=1 (non-zero) → jump to 0x006
        local prog = {
            0x70, 0x06,  -- 0x000: ISZ R0, 0x006
            0xD3,        -- 0x002: LDM 3 (skipped)
            0x01, 0x00, 0x00,
            0xD7,        -- 0x006: LDM 7
            0x01,        -- 0x007: HLT
        }
        local cpu = Intel4004.new()
        cpu:run(prog, 20)
        assert.are.equal(7, cpu.accumulator)
    end)

    it("does NOT jump when register wraps to zero", function()
        -- R0=15 → ISZ R0 → R0=0 → no jump → execute next instruction
        local prog = {
            0x70, 0x06,  -- 0x000: ISZ R0, 0x006
            0xD3,        -- 0x002: LDM 3 (executed because no jump)
            0x01,        -- 0x003: HLT
            0x00, 0x00,
            0xD7,        -- 0x006: LDM 7 (not reached)
            0x01,
        }
        local cpu = Intel4004.new()
        cpu.registers[1] = 15  -- R0 = 15
        cpu:run(prog, 20)
        assert.are.equal(3, cpu.accumulator)
    end)
end)

-- =========================================================================
-- FIM and SRC + RAM operations
-- =========================================================================

describe("FIM, SRC, WRM, RDM", function()
    it("FIM loads 8-bit immediate into register pair", function()
        -- FIM P0, 0x37 → R0=3, R1=7
        local cpu, _ = run({0x20, 0x37, 0x01}, 10)
        assert.are.equal(3, cpu.registers[1])  -- R0
        assert.are.equal(7, cpu.registers[2])  -- R1
    end)

    it("WRM and RDM round-trip through RAM", function()
        -- FIM P0, 0x00 (R0=0, R1=0 → address register 0, char 0)
        -- SRC P0 (set RAM address)
        -- LDM 9, WRM (write 9 to RAM[0][0])
        -- LDM 0, RDM (clear A, then read back)
        local prog = {
            0x20, 0x00,  -- FIM P0, 0x00
            0x21,        -- SRC P0
            0xD9,        -- LDM 9
            0xE0,        -- WRM (write A=9 to RAM main char)
            0xD0,        -- LDM 0 (clear A)
            0xE9,        -- RDM (read RAM main char into A)
            0x01,        -- HLT
        }
        local cpu = Intel4004.new()
        cpu:run(prog, 20)
        assert.are.equal(9, cpu.accumulator, "RDM should read back the value written by WRM")
    end)
end)

-- =========================================================================
-- DCL: RAM bank selection
-- =========================================================================

describe("DCL", function()
    it("DCL selects RAM bank 1 (A=0)", function()
        local cpu = Intel4004.new()
        cpu.accumulator = 0
        cpu:run({0xFD, 0x01}, 10)
        assert.are.equal(1, cpu.ram_bank)
    end)

    it("DCL selects RAM bank 2 (A=1)", function()
        local cpu = Intel4004.new()
        cpu.accumulator = 1
        cpu:run({0xFD, 0x01}, 10)
        assert.are.equal(2, cpu.ram_bank)
    end)

    it("DCL selects RAM bank 4 (A=3)", function()
        local cpu = Intel4004.new()
        cpu.accumulator = 3
        cpu:run({0xFD, 0x01}, 10)
        assert.are.equal(4, cpu.ram_bank)
    end)
end)

-- =========================================================================
-- WRR and RDR: ROM I/O port
-- =========================================================================

describe("WRR and RDR", function()
    it("WRR writes accumulator to ROM port, RDR reads it back", function()
        local cpu = Intel4004.new()
        cpu.accumulator = 11
        cpu:run({0xE2, 0xD0, 0xEA, 0x01}, 10)
        -- WRR: port = 11; LDM 0: A=0; RDR: A = port = 11
        assert.are.equal(11, cpu.accumulator)
    end)
end)

-- =========================================================================
-- Status characters: WR0-WR3, RD0-RD3
-- =========================================================================

describe("WR/RD status characters", function()
    it("writes and reads all 4 status characters", function()
        -- FIM P0, 0x00, SRC P0, LDM N, WRN, LDM 0, RDN, HLT
        for idx = 0, 3 do
            local cpu = Intel4004.new()
            cpu:run({
                0x20, 0x00,            -- FIM P0, 0x00
                0x21,                  -- SRC P0
                0xD0 | (idx + 5),      -- LDM (idx+5)
                0xE4 + idx,            -- WRn
                0xD0,                  -- LDM 0
                0xEC + idx,            -- RDn
                0x01,                  -- HLT
            }, 20)
            assert.are.equal(idx + 5, cpu.accumulator,
                "Status char " .. idx .. " should return " .. (idx+5))
        end
    end)
end)

-- =========================================================================
-- End-to-end: x = 1 + 2
-- =========================================================================

describe("E2E: x = 1 + 2", function()
    it("computes 1 + 2 = 3 stored in R1", function()
        -- LDM 1, XCH R0, LDM 2, ADD R0, XCH R1, HLT
        local prog = {0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01}
        local cpu = Intel4004.new()
        cpu:run(prog, 20)
        assert.are.equal(3, cpu.registers[2], "R1 should be 3 = 1+2")
    end)
end)

-- =========================================================================
-- End-to-end: 3 × 4 using ISZ loop
-- =========================================================================

describe("E2E: multiply 3 x 4 via loop", function()
    it("multiplies 3 x 4 = 12", function()
        -- Multiply 3 × 4 using repeated addition:
        -- FIM P0, 0x30 (R0=3, R1=0)
        -- FIM P1, 0x0C (R2=0, R3=12 which is -4 mod 16)
        -- LOOP: LD R0, ADD R2, XCH R2, ISZ R3, LOOP
        -- HLT
        -- ISZ R3 increments R3 from 12,13,14,15,0 — jumps while !=0, falls through at 0
        local prog = {
            0x20, 0x30,  -- 0x000: FIM P0, 0x30 (R0=3, R1=0)
            0x22, 0x0C,  -- 0x002: FIM P1, 0x0C (R2=0, R3=12)
            0xA0,        -- 0x004: LD R0  (LOOP)
            0x82,        -- 0x005: ADD R2
            0xB2,        -- 0x006: XCH R2
            0x73, 0x04,  -- 0x007: ISZ R3, 0x004 (same page: 0x004)
            0x01,        -- 0x009: HLT
        }
        local cpu = Intel4004.new()
        cpu:run(prog, 100)
        assert.are.equal(12, cpu.registers[3], "R2 should be 3*4=12")
    end)
end)

-- =========================================================================
-- Reset
-- =========================================================================

describe("reset", function()
    it("clears all state", function()
        local cpu = Intel4004.new()
        cpu:run({0xD9, 0x01}, 10)
        cpu:reset()
        assert.are.equal(0, cpu.accumulator)
        assert.are.equal(0, cpu.pc)
        assert.is_false(cpu.carry)
        assert.is_false(cpu.halted)
    end)
end)

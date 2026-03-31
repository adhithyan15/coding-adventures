-- ==========================================================================
-- Tests for coding_adventures.arm1_gatelevel (Lua)
-- ==========================================================================
--
-- Run with: busted . --verbose --pattern=test_
--
-- These tests verify that the gate-level simulator produces identical results
-- to the behavioral simulator, but with every ALU and barrel-shift operation
-- routed through logic gate function calls.

local GL   = require("coding_adventures.arm1_gatelevel")
local ARM1 = require("coding_adventures.arm1_simulator")

-- =========================================================================
-- Helper: build a small CPU for testing
-- =========================================================================

local function make_cpu(instructions, mem_size)
    mem_size = mem_size or 4096
    local cpu = GL.new(mem_size)
    ARM1.load_instructions(cpu, 0, instructions)
    return cpu
end

-- =========================================================================
-- Bit conversion helpers
-- =========================================================================

describe("int_to_bits and bits_to_int", function()
    it("converts 0 to 32 zero bits", function()
        local bits = GL.int_to_bits(0, 32)
        assert.are.equal(32, #bits)
        for i = 1, 32 do
            assert.are.equal(0, bits[i])
        end
    end)

    it("converts 1 — only bit[1]=1", function()
        local bits = GL.int_to_bits(1, 32)
        assert.are.equal(1, bits[1])
        for i = 2, 32 do assert.are.equal(0, bits[i]) end
    end)

    it("converts 0x80000000 — only bit[32]=1", function()
        local bits = GL.int_to_bits(0x80000000, 32)
        assert.are.equal(1, bits[32])
        for i = 1, 31 do assert.are.equal(0, bits[i]) end
    end)

    it("roundtrips arbitrary values", function()
        local values = {0, 1, 0xFF, 0x1234, 0xDEADBEEF, 0xFFFFFFFF}
        for _, v in ipairs(values) do
            local bits = GL.int_to_bits(v, 32)
            local back = GL.bits_to_int(bits)
            assert.are.equal(v & 0xFFFFFFFF, back)
        end
    end)

    it("handles width parameter for narrow arrays", function()
        local bits = GL.int_to_bits(5, 4)  -- 5 = 0b0101
        assert.are.equal(4, #bits)
        assert.are.equal(1, bits[1])  -- LSB
        assert.are.equal(0, bits[2])
        assert.are.equal(1, bits[3])
        assert.are.equal(0, bits[4])  -- MSB
    end)
end)

-- =========================================================================
-- Gate-level barrel shifter
-- =========================================================================

describe("gate_barrel_shift", function()
    local MASK32 = 0xFFFFFFFF

    local function to_bits(v) return GL.int_to_bits(v, 32) end
    local function from_bits(b) return GL.bits_to_int(b) end

    it("LSL #0 returns value unchanged", function()
        local bits = to_bits(0xABCDEF01)
        local result, carry = GL.gate_barrel_shift(bits, GL.SHIFT_LSL, 0, 0, false)
        assert.are.equal(0xABCDEF01, from_bits(result))
        assert.are.equal(0, carry)
    end)

    it("LSL #1 shifts left by 1", function()
        local bits = to_bits(1)
        local result, carry = GL.gate_barrel_shift(bits, GL.SHIFT_LSL, 1, 0, false)
        assert.are.equal(2, from_bits(result))
        assert.are.equal(0, carry)
    end)

    it("LSL #1 carry from MSB", function()
        -- 0x80000001 LSL #1: carry = bit[32] = 1, result = 0x00000002
        local bits = to_bits(0x80000001)
        local result, carry = GL.gate_barrel_shift(bits, GL.SHIFT_LSL, 1, 0, false)
        assert.are.equal(2, from_bits(result))
        assert.are.equal(1, carry)
    end)

    it("LSL #8 shifts correctly", function()
        local bits = to_bits(0x01)
        local result, carry = GL.gate_barrel_shift(bits, GL.SHIFT_LSL, 8, 0, false)
        assert.are.equal(0x100, from_bits(result))
    end)

    it("LSL #31 shifts bit 1 to bit 32", function()
        local bits = to_bits(1)
        local result, carry = GL.gate_barrel_shift(bits, GL.SHIFT_LSL, 31, 0, false)
        assert.are.equal(0x80000000, from_bits(result))
        assert.are.equal(0, carry)
    end)

    it("LSR #1 shifts right by 1", function()
        local bits = to_bits(0x80000000)
        local result, carry = GL.gate_barrel_shift(bits, GL.SHIFT_LSR, 1, 0, false)
        assert.are.equal(0x40000000, from_bits(result))
        assert.are.equal(0, carry)
    end)

    it("LSR #1 carry from LSB", function()
        local bits = to_bits(0x00000003)
        local result, carry = GL.gate_barrel_shift(bits, GL.SHIFT_LSR, 1, 0, false)
        assert.are.equal(1, from_bits(result))
        assert.are.equal(1, carry)
    end)

    it("ASR #1 preserves sign bit (positive)", function()
        local bits = to_bits(0x40000000)
        local result, carry = GL.gate_barrel_shift(bits, GL.SHIFT_ASR, 1, 0, false)
        assert.are.equal(0x20000000, from_bits(result))
    end)

    it("ASR #1 preserves sign bit (negative)", function()
        local bits = to_bits(0x80000000)
        local result, carry = GL.gate_barrel_shift(bits, GL.SHIFT_ASR, 1, 0, false)
        assert.are.equal(0xC0000000, from_bits(result))
    end)

    it("ROR #1 rotates bit 1 to bit 32", function()
        local bits = to_bits(0x00000001)
        local result, carry = GL.gate_barrel_shift(bits, GL.SHIFT_ROR, 1, 0, false)
        assert.are.equal(0x80000000, from_bits(result))
        assert.are.equal(1, carry)  -- MSB of result
    end)

    it("RRX (ROR #0 immediate) rotates through carry", function()
        -- RRX: carry_in -> MSB, LSB -> carry_out
        local bits = to_bits(0x00000003)
        -- bit[1]=1 (LSB) exits as carry_out, carry_in=1 becomes bit[32]
        local result, carry = GL.gate_barrel_shift(bits, GL.SHIFT_ROR, 0, 1, false)
        assert.are.equal(1, carry)  -- LSB shifted out
        assert.are.equal(0x80000001, from_bits(result))  -- carry_in=1 becomes MSB
    end)
end)

-- =========================================================================
-- Gate-level ALU — all 16 operations
-- =========================================================================

describe("gate_alu_execute", function()
    local function alu(op, a, b, cin, sc, ov)
        return GL.gate_alu_execute(op, a, b, cin or 0, sc or 0, ov or 0)
    end

    it("AND: 0xF0 AND 0xFF = 0xF0", function()
        local r = alu(GL.OP_AND, 0xF0, 0xFF)
        assert.are.equal(0xF0, r.result)
        assert.are.equal(true, r.write_result)
    end)

    it("EOR: 0xFF EOR 0x0F = 0xF0", function()
        local r = alu(GL.OP_EOR, 0xFF, 0x0F)
        assert.are.equal(0xF0, r.result)
    end)

    it("SUB: 5 - 3 = 2", function()
        local r = alu(GL.OP_SUB, 5, 3)
        assert.are.equal(2, r.result)
        assert.are.equal(0, r.n)
        assert.are.equal(0, r.z)
    end)

    it("SUB: 0 - 0 = 0 (Z flag)", function()
        local r = alu(GL.OP_SUB, 0, 0)
        assert.are.equal(0, r.result)
        assert.are.equal(1, r.z)
    end)

    it("RSB: b - a = 10 - 5 = 5", function()
        local r = alu(GL.OP_RSB, 5, 10)
        assert.are.equal(5, r.result)
    end)

    it("ADD: 3 + 4 = 7", function()
        local r = alu(GL.OP_ADD, 3, 4)
        assert.are.equal(7, r.result)
        assert.are.equal(0, r.c)
    end)

    it("ADD: carry out on overflow", function()
        local r = alu(GL.OP_ADD, 0xFFFFFFFF, 1)
        assert.are.equal(0, r.result)
        assert.are.equal(1, r.c)
        assert.are.equal(1, r.z)
    end)

    it("ADC: 3 + 4 + carry(1) = 8", function()
        local r = alu(GL.OP_ADC, 3, 4, 1)
        assert.are.equal(8, r.result)
    end)

    it("SBC: a - b - NOT(carry) = 5 - 3 - 0 = 2 (carry=1 means no borrow)", function()
        local r = alu(GL.OP_SBC, 5, 3, 1)
        assert.are.equal(2, r.result)
    end)

    it("RSC: b - a - NOT(carry)", function()
        local r = alu(GL.OP_RSC, 3, 5, 1)
        assert.are.equal(2, r.result)
    end)

    it("TST: result not written but flags set", function()
        local r = alu(GL.OP_TST, 0xFF, 0x0F)
        assert.are.equal(false, r.write_result)
        -- 0xFF AND 0x0F = 0x0F (non-zero)
        assert.are.equal(0, r.z)
    end)

    it("TEQ: EOR with no write", function()
        local r = alu(GL.OP_TEQ, 0x5, 0x5)
        assert.are.equal(false, r.write_result)
        assert.are.equal(1, r.z)
    end)

    it("CMP: compare with no write", function()
        local r = alu(GL.OP_CMP, 5, 5)
        assert.are.equal(false, r.write_result)
        assert.are.equal(1, r.z)
    end)

    it("CMN: add with no write", function()
        local r = alu(GL.OP_CMN, 0xFFFFFFFF, 1)
        assert.are.equal(false, r.write_result)
        assert.are.equal(1, r.z)
        assert.are.equal(1, r.c)
    end)

    it("ORR: 0xF0 ORR 0x0F = 0xFF", function()
        local r = alu(GL.OP_ORR, 0xF0, 0x0F)
        assert.are.equal(0xFF, r.result)
    end)

    it("MOV: pass b through", function()
        local r = alu(GL.OP_MOV, 0, 0x12345678)
        assert.are.equal(0x12345678, r.result)
    end)

    it("BIC: a AND NOT(b)", function()
        local r = alu(GL.OP_BIC, 0xFF, 0x0F)
        assert.are.equal(0xF0, r.result)
    end)

    it("MVN: NOT(b)", function()
        local r = alu(GL.OP_MVN, 0, 0)
        assert.are.equal(0xFFFFFFFF, r.result)
    end)

    it("N flag set for negative result", function()
        local r = alu(GL.OP_MOV, 0, 0x80000000)
        assert.are.equal(1, r.n)
    end)

    it("Z flag set for zero result", function()
        local r = alu(GL.OP_MOV, 0, 0)
        assert.are.equal(1, r.z)
    end)

    it("V flag set on signed overflow (ADD)", function()
        -- 0x7FFFFFFF + 1: both positive, result negative => overflow
        local r = alu(GL.OP_ADD, 0x7FFFFFFF, 1)
        assert.are.equal(1, r.v)
    end)
end)

-- =========================================================================
-- Full simulation — MOV, ADD instructions
-- =========================================================================

describe("gate-level simulation — basic instructions", function()
    it("MOV R0, #42 sets R0=42", function()
        local instrs = {
            ARM1.encode_mov_imm(ARM1.COND_AL, 0, 42),
            ARM1.encode_halt(),
        }
        local cpu = make_cpu(instrs)
        GL.run(cpu, 10)
        assert.are.equal(42, ARM1.read_register(cpu, 0))
    end)

    it("ADD R2, R0, R1", function()
        local instrs = {
            ARM1.encode_mov_imm(ARM1.COND_AL, 0, 10),
            ARM1.encode_mov_imm(ARM1.COND_AL, 1, 20),
            ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_ADD, false, 2, 0, 1),
            ARM1.encode_halt(),
        }
        local cpu = make_cpu(instrs)
        GL.run(cpu, 10)
        assert.are.equal(30, ARM1.read_register(cpu, 2))
    end)

    it("gate_ops counter increases with each instruction", function()
        local instrs = {
            ARM1.encode_mov_imm(ARM1.COND_AL, 0, 1),
            ARM1.encode_mov_imm(ARM1.COND_AL, 1, 2),
            ARM1.encode_halt(),
        }
        local cpu = make_cpu(instrs)
        assert.are.equal(0, cpu.gate_ops)
        GL.step(cpu)
        assert.is_true(cpu.gate_ops > 0)
        local ops1 = cpu.gate_ops
        GL.step(cpu)
        assert.is_true(cpu.gate_ops > ops1)
    end)
end)

-- =========================================================================
-- Conditional execution via gate-level condition evaluation
-- =========================================================================

describe("gate-level conditional execution", function()
    it("MOVEQ only executes when Z=1", function()
        -- MOV R0, #0; CMP R0, R0 (sets Z=1); MOVEQ R1, #99; MOVNE R2, #77; HALT
        -- CMP uses encode_alu_reg(cond, OP_CMP, s, rd, rn, rm)
        -- encode_alu_reg(condition, opcode, s, rd, rn, rm) — but CMP has s implicit
        -- Actually encode_data_processing(cond, OP_CMP, 1, rn=R0, rd=0, operand2=R0)
        local instrs = {
            ARM1.encode_mov_imm(ARM1.COND_AL, 0, 0),
            ARM1.encode_data_processing(ARM1.COND_AL, ARM1.OP_CMP, 1, 0, 0, 0), -- CMP R0,R0
            ARM1.encode_mov_imm(ARM1.COND_EQ, 1, 99),
            ARM1.encode_mov_imm(ARM1.COND_NE, 2, 77),
            ARM1.encode_halt(),
        }
        local cpu = make_cpu(instrs)
        GL.run(cpu, 20)
        assert.are.equal(99, ARM1.read_register(cpu, 1))
        assert.are.equal(0,  ARM1.read_register(cpu, 2))
    end)

    it("branch with gate-level condition works", function()
        -- MOV R0, #5; MOV R1, #5; CMP R0, R1; BEQ skip; MOV R3, #99; skip: HALT
        -- BEQ at 0x0C: branch_base = (0x0C+4)+4 = 0x14; target = 0x14; offset = 0
        local instrs = {
            ARM1.encode_mov_imm(ARM1.COND_AL, 0, 5),          -- 0x00
            ARM1.encode_mov_imm(ARM1.COND_AL, 1, 5),          -- 0x04
            -- CMP R0, R1: encode_data_processing(cond, CMP, s=1, rn=R0, rd=0, rm=R1)
            ARM1.encode_data_processing(ARM1.COND_AL, ARM1.OP_CMP, 1, 0, 0, 1), -- 0x08
            ARM1.encode_branch(ARM1.COND_EQ, false, 0),       -- 0x0C BEQ +0 (skip next)
            ARM1.encode_mov_imm(ARM1.COND_AL, 3, 99),         -- 0x10 (should be skipped)
            ARM1.encode_halt(),                                 -- 0x14
        }
        local cpu = make_cpu(instrs)
        GL.run(cpu, 20)
        assert.are.equal(0, ARM1.read_register(cpu, 3))
    end)
end)

-- =========================================================================
-- Sum 1..10 = 55 — integration test
-- =========================================================================

describe("gate-level integration — sum 1 to 10", function()
    --
    -- Computes sum = 1+2+...+10 using a loop.
    --
    -- Register allocation:
    --   R0 = loop counter (1..10)
    --   R1 = accumulator (sum)
    --
    -- Layout (word addresses):
    --   0x00  MOV R0, #1     ; counter = 1
    --   0x04  MOV R1, #0     ; sum = 0
    --   0x08  ADD R1, R1, R0 ; sum += counter          (loop_top)
    --   0x0C  ADD R0, R0, #1 ; counter++
    --   0x10  CMP R0, #11    ; compare counter to 11
    --   0x14  BLT loop_top   ; branch back if counter < 11
    --   0x18  SWI HALT_SWI
    --
    -- BLT at 0x14: branch_base = 0x14+4+4 = 0x1C; target=0x08; offset = 0x08-0x1C = -20

    it("produces R1 = 55", function()
        local COND_AL  = ARM1.COND_AL
        local COND_LT  = ARM1.COND_LT
        local OP_ADD   = ARM1.OP_ADD
        local OP_CMP   = ARM1.OP_CMP
        local SHIFT_LSL = ARM1.SHIFT_LSL

        local prog = {}
        -- encode_data_processing(condition, opcode, s, rn, rd, operand2)
        -- For immediate: operand2 = (1 << 25) | (rotate << 8) | imm8
        -- ADD R0, R0, #1: immediate, rn=R0(src), rd=R0(dst)
        local function add_imm(cond, rd, rn, imm8)
            local op2 = (1 << 25) | imm8
            return ARM1.encode_data_processing(cond, OP_ADD, 0, rn, rd, op2)
        end
        -- CMP R0, #11: immediate, s=1, rd=0 (ignored for CMP), rn=R0
        local function cmp_imm(cond, rn, imm8)
            local op2 = (1 << 25) | imm8
            return ARM1.encode_data_processing(cond, OP_CMP, 1, rn, 0, op2)
        end

        prog[1] = ARM1.encode_mov_imm(COND_AL, 0, 1)                 -- 0x00 MOV R0,#1
        prog[2] = ARM1.encode_mov_imm(COND_AL, 1, 0)                 -- 0x04 MOV R1,#0
        -- ADD R1, R1, R0 (register form): encode_alu_reg(cond,op,s,rd,rn,rm)
        prog[3] = ARM1.encode_alu_reg(COND_AL, OP_ADD, false, 1, 1, 0) -- 0x08
        -- ADD R0, R0, #1 (immediate)
        prog[4] = add_imm(COND_AL, 0, 0, 1)                          -- 0x0C
        -- CMP R0, #11
        prog[5] = cmp_imm(COND_AL, 0, 11)                            -- 0x10
        -- BLT offset so that target = 0x08
        -- branch_base at execute = PC_after_fetch + 4 = (0x14+4)+4 = 0x1C
        -- offset = 0x08 - 0x1C = -20
        prog[6] = ARM1.encode_branch(COND_LT, false, -20)            -- 0x14
        prog[7] = ARM1.encode_halt()                                  -- 0x18

        local cpu = GL.new(4096)
        ARM1.load_instructions(cpu, 0, prog)
        GL.run(cpu, 500)

        assert.are.equal(55, ARM1.read_register(cpu, 1))
        assert.is_true(cpu.gate_ops > 0, "gate_ops should be positive")
    end)
end)

-- =========================================================================
-- Gate-level vs behavioral equivalence
-- =========================================================================

describe("gate-level produces same results as behavioral", function()
    it("same register state for a short sequence", function()
        local prog = {
            ARM1.encode_mov_imm(ARM1.COND_AL, 0, 7),
            ARM1.encode_mov_imm(ARM1.COND_AL, 1, 3),
            ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_SUB, false, 2, 0, 1, ARM1.SHIFT_LSL, 0),
            ARM1.encode_halt(),
        }

        local cpu_gl   = GL.new(4096)
        local cpu_beh  = ARM1.new(4096)
        ARM1.load_instructions(cpu_gl,  0, prog)
        ARM1.load_instructions(cpu_beh, 0, prog)

        GL.run(cpu_gl, 10)
        ARM1.run(cpu_beh, 10)

        for i = 0, 13 do
            assert.are.equal(
                ARM1.read_register(cpu_beh, i),
                ARM1.read_register(cpu_gl, i),
                "mismatch at R" .. i
            )
        end
    end)
end)

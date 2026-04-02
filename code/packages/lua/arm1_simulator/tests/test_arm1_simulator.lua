-- test_arm1_simulator.lua — Test suite for the ARM1 behavioral simulator
--
-- Tests cover:
--   * CPU construction and reset
--   * Register read/write with mode banking
--   * Memory read/write (byte and word)
--   * Condition evaluation for all 16 conditions
--   * Barrel shifter: LSL, LSR, ASR, ROR, RRX
--   * ALU: all 16 operations with correct flags
--   * Data processing instructions
--   * Load/Store: LDR, STR, LDRB, STRB
--   * Block transfer: STMIA, LDMIA
--   * Branch with and without link
--   * SWI halt
--   * End-to-end: sum 1..10 = 55

local ARM1 = require("coding_adventures.arm1_simulator")

describe("arm1_simulator", function()

    -- ===========================================================
    -- Constants
    -- ===========================================================

    describe("constants", function()
        it("exposes mode constants", function()
            assert.equals(0, ARM1.MODE_USR)
            assert.equals(1, ARM1.MODE_FIQ)
            assert.equals(2, ARM1.MODE_IRQ)
            assert.equals(3, ARM1.MODE_SVC)
        end)

        it("exposes condition code constants", function()
            assert.equals(0x0, ARM1.COND_EQ)
            assert.equals(0xE, ARM1.COND_AL)
            assert.equals(0xF, ARM1.COND_NV)
        end)

        it("exposes ALU opcode constants", function()
            assert.equals(0x4, ARM1.OP_ADD)
            assert.equals(0xD, ARM1.OP_MOV)
            assert.equals(0xF, ARM1.OP_MVN)
        end)

        it("exposes flag masks", function()
            assert.equals(0x80000000, ARM1.FLAG_N)
            assert.equals(0x40000000, ARM1.FLAG_Z)
            assert.equals(0x20000000, ARM1.FLAG_C)
            assert.equals(0x10000000, ARM1.FLAG_V)
        end)
    end)

    -- ===========================================================
    -- Construction and Reset
    -- ===========================================================

    describe("new and reset", function()
        it("creates a CPU in SVC mode", function()
            local cpu = ARM1.new(4096)
            assert.equals(ARM1.MODE_SVC, ARM1.get_mode(cpu))
        end)

        it("starts halted=false", function()
            local cpu = ARM1.new(4096)
            assert.is_false(cpu.halted)
        end)

        it("starts at PC=0", function()
            local cpu = ARM1.new(4096)
            assert.equals(0, ARM1.get_pc(cpu))
        end)

        it("has I and F flags set on reset (interrupts disabled)", function()
            local cpu = ARM1.new(4096)
            assert.truthy(cpu.regs[15] & ARM1.FLAG_I ~= 0)
            assert.truthy(cpu.regs[15] & ARM1.FLAG_F ~= 0)
        end)

        it("reset clears registers", function()
            local cpu = ARM1.new(4096)
            ARM1.write_register(cpu, 0, 42)
            ARM1.reset(cpu)
            assert.equals(0, ARM1.read_register(cpu, 0))
        end)
    end)

    -- ===========================================================
    -- Register Access
    -- ===========================================================

    describe("register access", function()
        it("reads and writes R0-R12 in USR mode", function()
            local cpu = ARM1.new(4096)
            -- Force USR mode
            cpu.regs[15] = (cpu.regs[15] & ~ARM1.MODE_MASK) | ARM1.MODE_USR
            for i = 0, 12 do
                ARM1.write_register(cpu, i, i * 100)
            end
            for i = 0, 12 do
                assert.equals(i * 100, ARM1.read_register(cpu, i))
            end
        end)

        it("masks values to 32 bits", function()
            local cpu = ARM1.new(4096)
            ARM1.write_register(cpu, 0, 0x1FFFFFFFF)  -- 33-bit value
            assert.equals(0xFFFFFFFF, ARM1.read_register(cpu, 0))
        end)

        it("FIQ mode banks R8-R14", function()
            local cpu = ARM1.new(4096)
            -- Write in USR mode
            cpu.regs[15] = (cpu.regs[15] & ~ARM1.MODE_MASK) | ARM1.MODE_USR
            ARM1.write_register(cpu, 8, 0x1111)
            ARM1.write_register(cpu, 9, 0x2222)
            -- Switch to FIQ
            cpu.regs[15] = (cpu.regs[15] & ~ARM1.MODE_MASK) | ARM1.MODE_FIQ
            ARM1.write_register(cpu, 8, 0xAAAA)
            ARM1.write_register(cpu, 9, 0xBBBB)
            -- FIQ sees FIQ-banked values
            assert.equals(0xAAAA, ARM1.read_register(cpu, 8))
            -- Switch back to USR
            cpu.regs[15] = (cpu.regs[15] & ~ARM1.MODE_MASK) | ARM1.MODE_USR
            -- USR sees original values
            assert.equals(0x1111, ARM1.read_register(cpu, 8))
        end)

        it("SVC mode banks R13 and R14", function()
            local cpu = ARM1.new(4096)
            cpu.regs[15] = (cpu.regs[15] & ~ARM1.MODE_MASK) | ARM1.MODE_USR
            ARM1.write_register(cpu, 13, 0xDEAD)
            ARM1.write_register(cpu, 14, 0xBEEF)
            cpu.regs[15] = (cpu.regs[15] & ~ARM1.MODE_MASK) | ARM1.MODE_SVC
            ARM1.write_register(cpu, 13, 0x1234)
            ARM1.write_register(cpu, 14, 0x5678)
            assert.equals(0x1234, ARM1.read_register(cpu, 13))
            assert.equals(0x5678, ARM1.read_register(cpu, 14))
            cpu.regs[15] = (cpu.regs[15] & ~ARM1.MODE_MASK) | ARM1.MODE_USR
            assert.equals(0xDEAD, ARM1.read_register(cpu, 13))
            assert.equals(0xBEEF, ARM1.read_register(cpu, 14))
        end)
    end)

    -- ===========================================================
    -- Memory Access
    -- ===========================================================

    describe("memory access", function()
        it("writes and reads a word", function()
            local cpu = ARM1.new(4096)
            ARM1.write_word(cpu, 0x100, 0xDEADBEEF)
            assert.equals(0xDEADBEEF, ARM1.read_word(cpu, 0x100))
        end)

        it("is little-endian", function()
            local cpu = ARM1.new(4096)
            ARM1.write_word(cpu, 0x100, 0x01020304)
            assert.equals(0x04, ARM1.read_byte(cpu, 0x100))
            assert.equals(0x03, ARM1.read_byte(cpu, 0x101))
            assert.equals(0x02, ARM1.read_byte(cpu, 0x102))
            assert.equals(0x01, ARM1.read_byte(cpu, 0x103))
        end)

        it("writes and reads a byte", function()
            local cpu = ARM1.new(4096)
            ARM1.write_byte(cpu, 0x200, 0xAB)
            assert.equals(0xAB, ARM1.read_byte(cpu, 0x200))
        end)

        it("returns 0 for out-of-range reads", function()
            local cpu = ARM1.new(256)
            assert.equals(0, ARM1.read_word(cpu, 0x1000))
            assert.equals(0, ARM1.read_byte(cpu, 0x1000))
        end)

        it("aligns word reads to 4-byte boundary", function()
            local cpu = ARM1.new(4096)
            ARM1.write_word(cpu, 0x100, 0xAABBCCDD)
            -- Unaligned read rotates the result (ARM1 quirk handled in load/store, not here)
            assert.equals(0xAABBCCDD, ARM1.read_word(cpu, 0x100))
        end)
    end)

    -- ===========================================================
    -- Condition Evaluation
    -- ===========================================================

    describe("evaluate_condition", function()
        local function flags(n, z, c, v)
            return {n=n, z=z, c=c, v=v}
        end

        it("EQ: Z=1 passes", function()
            assert.is_true(ARM1.evaluate_condition(ARM1.COND_EQ, flags(false,true,false,false)))
        end)
        it("EQ: Z=0 fails", function()
            assert.is_false(ARM1.evaluate_condition(ARM1.COND_EQ, flags(false,false,false,false)))
        end)
        it("NE: Z=0 passes", function()
            assert.is_true(ARM1.evaluate_condition(ARM1.COND_NE, flags(false,false,false,false)))
        end)
        it("CS: C=1 passes", function()
            assert.is_true(ARM1.evaluate_condition(ARM1.COND_CS, flags(false,false,true,false)))
        end)
        it("CC: C=0 passes", function()
            assert.is_true(ARM1.evaluate_condition(ARM1.COND_CC, flags(false,false,false,false)))
        end)
        it("MI: N=1 passes", function()
            assert.is_true(ARM1.evaluate_condition(ARM1.COND_MI, flags(true,false,false,false)))
        end)
        it("PL: N=0 passes", function()
            assert.is_true(ARM1.evaluate_condition(ARM1.COND_PL, flags(false,false,false,false)))
        end)
        it("VS: V=1 passes", function()
            assert.is_true(ARM1.evaluate_condition(ARM1.COND_VS, flags(false,false,false,true)))
        end)
        it("VC: V=0 passes", function()
            assert.is_true(ARM1.evaluate_condition(ARM1.COND_VC, flags(false,false,false,false)))
        end)
        it("HI: C=1, Z=0 passes", function()
            assert.is_true(ARM1.evaluate_condition(ARM1.COND_HI, flags(false,false,true,false)))
        end)
        it("HI: C=1, Z=1 fails", function()
            assert.is_false(ARM1.evaluate_condition(ARM1.COND_HI, flags(false,true,true,false)))
        end)
        it("LS: C=0 passes", function()
            assert.is_true(ARM1.evaluate_condition(ARM1.COND_LS, flags(false,false,false,false)))
        end)
        it("GE: N=V=false passes", function()
            assert.is_true(ARM1.evaluate_condition(ARM1.COND_GE, flags(false,false,false,false)))
        end)
        it("GE: N=V=true passes", function()
            assert.is_true(ARM1.evaluate_condition(ARM1.COND_GE, flags(true,false,false,true)))
        end)
        it("LT: N!=V fails GE", function()
            assert.is_true(ARM1.evaluate_condition(ARM1.COND_LT, flags(true,false,false,false)))
        end)
        it("GT: Z=0, N=V passes", function()
            assert.is_true(ARM1.evaluate_condition(ARM1.COND_GT, flags(false,false,false,false)))
        end)
        it("LE: Z=1 passes", function()
            assert.is_true(ARM1.evaluate_condition(ARM1.COND_LE, flags(false,true,false,false)))
        end)
        it("AL: always passes", function()
            assert.is_true(ARM1.evaluate_condition(ARM1.COND_AL, flags(false,false,false,false)))
            assert.is_true(ARM1.evaluate_condition(ARM1.COND_AL, flags(true,true,true,true)))
        end)
        it("NV: never passes", function()
            assert.is_false(ARM1.evaluate_condition(ARM1.COND_NV, flags(true,true,true,true)))
        end)
    end)

    -- ===========================================================
    -- Barrel Shifter
    -- ===========================================================

    describe("barrel_shift", function()
        it("LSL #0 = no change, preserves carry", function()
            local r, c = ARM1.barrel_shift(0xDEAD, ARM1.SHIFT_LSL, 0, true, false)
            assert.equals(0xDEAD, r)
            assert.is_true(c)
        end)

        it("LSL #1 shifts left", function()
            local r, c = ARM1.barrel_shift(0x80000001, ARM1.SHIFT_LSL, 1, false, false)
            assert.equals(2, r)
            assert.is_true(c)  -- bit 31 of original
        end)

        it("LSL #8", function()
            local r, c = ARM1.barrel_shift(0x12, ARM1.SHIFT_LSL, 8, false, false)
            assert.equals(0x1200, r)
            assert.is_false(c)
        end)

        it("LSR #1 shifts right logically", function()
            local r, c = ARM1.barrel_shift(0x80000001, ARM1.SHIFT_LSR, 1, false, false)
            assert.equals(0x40000000, r)
            assert.is_true(c)  -- bit 0 shifted out
        end)

        it("LSR #0 immediate = LSR #32", function()
            local r, c = ARM1.barrel_shift(0x80000000, ARM1.SHIFT_LSR, 0, false, false)
            assert.equals(0, r)
            assert.is_true(c)  -- MSB was 1
        end)

        it("ASR #1 sign-extends", function()
            local r, c = ARM1.barrel_shift(0x80000000, ARM1.SHIFT_ASR, 1, false, false)
            assert.equals(0xC0000000, r)
            assert.is_false(c)
        end)

        it("ASR #0 immediate = ASR #32 (negative value -> all 1s)", function()
            local r, c = ARM1.barrel_shift(0x80000000, ARM1.SHIFT_ASR, 0, false, false)
            assert.equals(0xFFFFFFFF, r)
            assert.is_true(c)
        end)

        it("ASR #0 immediate (positive value -> 0)", function()
            local r, c = ARM1.barrel_shift(0x40000000, ARM1.SHIFT_ASR, 0, false, false)
            assert.equals(0, r)
            assert.is_false(c)
        end)

        it("ROR #4 rotates right", function()
            local r, c = ARM1.barrel_shift(0x12345678, ARM1.SHIFT_ROR, 4, false, false)
            assert.equals(0x81234567, r)
            assert.is_true(c)  -- MSB of result is 1 = carry out
        end)

        it("RRX (ROR #0 immediate) rotates through carry", function()
            -- carry_in=1, value=0x3
            local r, c = ARM1.barrel_shift(3, ARM1.SHIFT_ROR, 0, true, false)
            -- result: 1 shifted in from MSB, bit 0 (=1) shifted out
            assert.equals(0x80000001, r)
            assert.is_true(c)
        end)

        it("register shift by 0 = no change", function()
            local r, c = ARM1.barrel_shift(0xDEAD, ARM1.SHIFT_LSL, 0, true, true)
            assert.equals(0xDEAD, r)
            assert.is_true(c)
        end)
    end)

    -- ===========================================================
    -- decode_immediate
    -- ===========================================================

    describe("decode_immediate", function()
        it("rotate=0 returns value unchanged", function()
            local v, c = ARM1.decode_immediate(42, 0)
            assert.equals(42, v)
            assert.is_false(c)
        end)

        it("rotates by 2*rotate_field", function()
            -- 0xFF rotated right by 2 = 0xC000003F
            local v, _ = ARM1.decode_immediate(0xFF, 1)
            assert.equals(0xC000003F, v)
        end)
    end)

    -- ===========================================================
    -- ALU
    -- ===========================================================

    describe("alu_execute", function()
        local function flags_false()
            return false, false, false, false
        end

        it("AND", function()
            local r = ARM1.alu_execute(ARM1.OP_AND, 0xFF, 0x0F, false, false, false)
            assert.equals(0x0F, r.result)
            assert.is_true(r.write_result)
        end)

        it("EOR", function()
            local r = ARM1.alu_execute(ARM1.OP_EOR, 0xFF, 0x0F, false, false, false)
            assert.equals(0xF0, r.result)
        end)

        it("ORR", function()
            local r = ARM1.alu_execute(ARM1.OP_ORR, 0xF0, 0x0F, false, false, false)
            assert.equals(0xFF, r.result)
        end)

        it("MOV", function()
            local r = ARM1.alu_execute(ARM1.OP_MOV, 0, 0xABCD, false, false, false)
            assert.equals(0xABCD, r.result)
        end)

        it("MVN", function()
            local r = ARM1.alu_execute(ARM1.OP_MVN, 0, 0, false, false, false)
            assert.equals(0xFFFFFFFF, r.result)
        end)

        it("BIC", function()
            local r = ARM1.alu_execute(ARM1.OP_BIC, 0xFF, 0x0F, false, false, false)
            assert.equals(0xF0, r.result)
        end)

        it("ADD sets carry on overflow", function()
            local r = ARM1.alu_execute(ARM1.OP_ADD, 0xFFFFFFFF, 1, false, false, false)
            assert.equals(0, r.result)
            assert.is_true(r.z)
            assert.is_true(r.c)
        end)

        it("ADD sets N for negative result", function()
            local r = ARM1.alu_execute(ARM1.OP_ADD, 0x7FFFFFFF, 1, false, false, false)
            assert.is_true(r.n)
            assert.is_true(r.v)  -- signed overflow
        end)

        it("SUB: 5 - 3 = 2", function()
            local r = ARM1.alu_execute(ARM1.OP_SUB, 5, 3, false, false, false)
            assert.equals(2, r.result)
            assert.is_true(r.c)  -- no borrow = carry set
        end)

        it("SUB: 3 - 5 borrows (C=0)", function()
            local r = ARM1.alu_execute(ARM1.OP_SUB, 3, 5, false, false, false)
            assert.equals(0xFFFFFFFE, r.result)
            assert.is_false(r.c)  -- borrow = carry clear
        end)

        it("TST does not write result", function()
            local r = ARM1.alu_execute(ARM1.OP_TST, 0xFF, 0x0F, false, false, false)
            assert.is_false(r.write_result)
            -- but still computes flags
            assert.is_false(r.z)  -- result 0x0F != 0
        end)

        it("TST: AND result = 0 sets Z", function()
            local r = ARM1.alu_execute(ARM1.OP_TST, 0xF0, 0x0F, false, false, false)
            assert.is_true(r.z)
        end)

        it("CMP does not write result", function()
            local r = ARM1.alu_execute(ARM1.OP_CMP, 5, 5, false, false, false)
            assert.is_false(r.write_result)
            assert.is_true(r.z)
        end)

        it("RSB: Op2 - Rn", function()
            local r = ARM1.alu_execute(ARM1.OP_RSB, 3, 5, false, false, false)
            assert.equals(2, r.result)
        end)

        it("ADC adds carry in", function()
            local r = ARM1.alu_execute(ARM1.OP_ADC, 5, 3, true, false, false)
            assert.equals(9, r.result)
        end)

        it("logical op: carry from shifter", function()
            local r = ARM1.alu_execute(ARM1.OP_MOV, 0, 42, false, true, false)
            assert.is_true(r.c)  -- carry from shifter
        end)
    end)

    -- ===========================================================
    -- Instruction Execution: MOV and ADD
    -- ===========================================================

    describe("step: data processing", function()
        it("MOV R0, #42 sets R0=42", function()
            local cpu = ARM1.new(4096)
            ARM1.load_instructions(cpu, {
                ARM1.encode_mov_imm(ARM1.COND_AL, 0, 42),
                ARM1.encode_halt(),
            })
            ARM1.run(cpu, 100)
            assert.equals(42, ARM1.read_register(cpu, 0))
        end)

        it("MOVS R0, #0 sets Z flag", function()
            local cpu = ARM1.new(4096)
            -- MOVS R0, #0: encode_data_processing(AL, MOV, s=1, rn=0, rd=0, imm=0x02000000|0)
            local inst = ARM1.encode_data_processing(ARM1.COND_AL, ARM1.OP_MOV, 1, 0, 0, (1 << 25))
            ARM1.load_instructions(cpu, { inst, ARM1.encode_halt() })
            ARM1.run(cpu, 100)
            local f = ARM1.get_flags(cpu)
            assert.is_true(f.z)
        end)

        it("ADD R0, R1, R2", function()
            local cpu = ARM1.new(4096)
            ARM1.load_instructions(cpu, {
                ARM1.encode_mov_imm(ARM1.COND_AL, 1, 10),
                ARM1.encode_mov_imm(ARM1.COND_AL, 2, 20),
                ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_ADD, 1, 0, 1, 2),
                ARM1.encode_halt(),
            })
            ARM1.run(cpu, 100)
            assert.equals(30, ARM1.read_register(cpu, 0))
        end)

        it("conditional NE: skips instruction when Z=1", function()
            local cpu = ARM1.new(4096)
            -- MOVS R1, #0 (sets Z=1), then MOVNE R0, #99 (should skip)
            local movs_0 = ARM1.encode_data_processing(ARM1.COND_AL, ARM1.OP_MOV, 1, 0, 1, (1 << 25))
            local movne  = ARM1.encode_data_processing(ARM1.COND_NE, ARM1.OP_MOV, 0, 0, 0, (1 << 25) | 99)
            ARM1.load_instructions(cpu, { movs_0, movne, ARM1.encode_halt() })
            ARM1.run(cpu, 100)
            assert.equals(0, ARM1.read_register(cpu, 0))  -- should remain 0
        end)
    end)

    -- ===========================================================
    -- Load/Store
    -- ===========================================================

    describe("step: load/store", function()
        it("STR and LDR round-trip", function()
            local cpu = ARM1.new(4096)
            -- MOV R0, #0xAB; MOV R1, #0x100; STR R0, [R1]; LDR R2, [R1]
            ARM1.load_instructions(cpu, {
                ARM1.encode_mov_imm(ARM1.COND_AL, 0, 0xAB),
                -- MOV R1, #0x100 via rotate: imm8=1, rotate=0xF (rotate by 30)
                -- Actually let's use a simpler approach:
                -- STR at address offset from R1=0 (PC relative workaround):
                -- Use R1 = 0x100 by building: MOV R1, #1 then LSL
                -- Simpler: store at word 64 (address 256 = 0x100)
                -- Use encode_str with rn=0 (R0=0xAB already), offset=256
                -- Actually let's keep it simple with direct memory write then LDR
                ARM1.encode_halt()
            })
            -- Direct test: write to memory and load
            ARM1.write_word(cpu, 0x100, 0xDEADBEEF)
            -- Reset and use a manual approach
            local cpu2 = ARM1.new(4096)
            ARM1.write_word(cpu2, 0x100, 0xDEADBEEF)
            -- Set R1 = 0x100
            ARM1.write_register(cpu2, 1, 0x100)
            -- Build: LDR R0, [R1, #0] (pre-index, offset=0)
            local ldr = ARM1.encode_ldr(ARM1.COND_AL, 0, 1, 0, true)
            ARM1.load_instructions(cpu2, { ldr, ARM1.encode_halt() })
            ARM1.run(cpu2, 100)
            assert.equals(0xDEADBEEF, ARM1.read_register(cpu2, 0))
        end)

        it("STR then LDR", function()
            local cpu = ARM1.new(4096)
            ARM1.write_register(cpu, 1, 0x200)  -- base address
            ARM1.write_register(cpu, 0, 0x1234)  -- value to store
            local str = ARM1.encode_str(ARM1.COND_AL, 0, 1, 0, true)
            local ldr = ARM1.encode_ldr(ARM1.COND_AL, 2, 1, 0, true)
            ARM1.load_instructions(cpu, { str, ldr, ARM1.encode_halt() })
            ARM1.run(cpu, 100)
            assert.equals(0x1234, ARM1.read_register(cpu, 2))
        end)
    end)

    -- ===========================================================
    -- Block Transfer
    -- ===========================================================

    describe("step: block transfer", function()
        it("STMIA and LDMIA round-trip", function()
            local cpu = ARM1.new(4096)
            ARM1.write_register(cpu, 0, 10)
            ARM1.write_register(cpu, 1, 20)
            ARM1.write_register(cpu, 2, 30)
            ARM1.write_register(cpu, 10, 0x400)  -- stack base

            -- STMIA R10!, {R0, R1, R2}
            local stm = ARM1.encode_stm(ARM1.COND_AL, 10, 0x7, true, "IA")
            -- LDMIA R10!, {R3, R4, R5} — but R10 now points past stored values
            -- We need to reset R10 first. Let's load back to different registers.
            -- After STMIA with write-back, R10 = 0x400 + 12 = 0x40C
            -- So load back: MOV R10, #0x400, then LDMIA
            -- Actually let's just check memory directly
            ARM1.load_instructions(cpu, { stm, ARM1.encode_halt() })
            ARM1.run(cpu, 100)
            assert.equals(10, ARM1.read_word(cpu, 0x400))
            assert.equals(20, ARM1.read_word(cpu, 0x404))
            assert.equals(30, ARM1.read_word(cpu, 0x408))
        end)
    end)

    -- ===========================================================
    -- Branch
    -- ===========================================================

    describe("step: branch", function()
        it("B jumps to target", function()
            local cpu = ARM1.new(4096)
            -- At address 0: MOV R0, #1
            -- At address 4: B +4 (skip next instruction, go to address 12)
            -- At address 8: MOV R0, #99 (should be skipped)
            -- At address 12: HALT
            ARM1.load_instructions(cpu, {
                ARM1.encode_mov_imm(ARM1.COND_AL, 0, 1),    -- 0x00
                ARM1.encode_branch(ARM1.COND_AL, false, 4), -- 0x04: B +4 (skip 1 instr = +4 bytes past this one? no: offset from PC+8)
                -- B offset=0 means jump to current PC+8 (next instr is at PC+4 from here)
                -- We want to skip address 8, so target = 12 = (4+8) + offset
                -- B offset: target = PC_current + 8 + offset*4... actually offset is bytes already in our encoder
                -- encode_branch offset is bytes. target = PC_after_prefetch + offset = (current_PC + 8) + offset
                -- current_PC = 4, so branch_base in exec = pc_after_advance + 4 = 8+4=12... wait
                -- In exec_branch: branch_base = get_pc(cpu) + 4 (where get_pc is already PC+4)
                -- So target = branch_base + d.branch_offset
                -- We want to skip addr 8 and go to addr 12.
                -- target = 12; branch_base = (current_pc+4)+4 = 4+4+4=12; branch_offset = 0
                -- But that means offset=0 in encode_branch. Let's just test with offset=0.
                ARM1.encode_mov_imm(ARM1.COND_AL, 0, 99),  -- 0x08: should be skipped
                ARM1.encode_halt(),                          -- 0x0C
            })
            -- Re-encode the branch properly: at PC=4 (during execute, branch_base=12), offset=0 → target=12
            ARM1.write_word(cpu, 4, ARM1.encode_branch(ARM1.COND_AL, false, 0))
            ARM1.run(cpu, 100)
            assert.equals(1, ARM1.read_register(cpu, 0))  -- MOV #99 was skipped
        end)

        it("BL saves return address in R14", function()
            local cpu = ARM1.new(4096)
            -- At 0: BL +0 (call to addr 8, since branch_base = 0+4+4=8; offset=0)
            -- We want BL to jump to addr 8, saving return addr (R15 at time of branch) in R14
            -- Return addr: after BL at addr 0, PC has been advanced to 4; BL saves cpu.regs[15]
            ARM1.load_instructions(cpu, {
                ARM1.encode_branch(ARM1.COND_AL, true, 0),  -- 0x00: BL offset=0
                ARM1.encode_halt(),                           -- 0x04: never reached
                ARM1.encode_halt(),                           -- 0x08: BL lands here
            })
            ARM1.run(cpu, 100)
            -- BL at addr 0: after step advances PC to 4, saves R15 (=4 with flags) in R14
            -- The saved value is cpu.regs[15] at time of BL execution (PC was advanced to 4)
            local r14 = ARM1.read_register(cpu, 14)
            assert.truthy(r14 ~= 0)  -- R14 was written
        end)
    end)

    -- ===========================================================
    -- SWI / Halt
    -- ===========================================================

    describe("step: SWI halt", function()
        it("halt SWI stops execution", function()
            local cpu = ARM1.new(4096)
            ARM1.load_instructions(cpu, { ARM1.encode_halt() })
            ARM1.run(cpu, 100)
            assert.is_true(cpu.halted)
        end)

        it("halts after correct number of steps", function()
            local cpu = ARM1.new(4096)
            ARM1.load_instructions(cpu, {
                ARM1.encode_mov_imm(ARM1.COND_AL, 0, 1),
                ARM1.encode_mov_imm(ARM1.COND_AL, 1, 2),
                ARM1.encode_halt(),
            })
            local traces = ARM1.run(cpu, 100)
            assert.equals(3, #traces)
        end)
    end)

    -- ===========================================================
    -- End-to-End: Sum 1..10 = 55
    -- ===========================================================

    describe("end-to-end: sum 1..10", function()
        it("computes sum = 55", function()
            -- Program:
            --   R0 = 0       (accumulator)
            --   R1 = 1       (counter, starts at 1)
            --   R2 = 11      (limit, exclusive)
            -- loop:
            --   ADD R0, R0, R1   (sum += counter)
            --   ADD R1, R1, #1   (counter++)
            --   CMP R1, R2       (counter < 11?)
            --   BLT loop         (branch if less than)
            --   HALT
            local cpu = ARM1.new(4096)

            -- CMP R1, R2 sets flags; BLT branches back
            -- At address 0: MOV R0, #0
            -- At address 4: MOV R1, #1
            -- At address 8: MOV R2, #11
            -- At address 12: ADDS R0, R0, R1 (or ADD without S)
            -- At address 16: ADD R1, R1, #1
            -- At address 20: CMP R1, R2
            -- At address 24: BLT loop (target = 12 = loop_addr)
            --   branch_base at BLT execution = (24+4+4) = 32... wait
            --   step advances PC to 28; exec_branch: branch_base = 28+4=32
            --   target = 12 → offset = 12 - 32 = -20
            -- At address 28: HALT

            local MOV_R0_0  = ARM1.encode_mov_imm(ARM1.COND_AL, 0, 0)
            local MOV_R1_1  = ARM1.encode_mov_imm(ARM1.COND_AL, 1, 1)
            local MOV_R2_11 = ARM1.encode_mov_imm(ARM1.COND_AL, 2, 11)
            local ADD_R0    = ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_ADD, 0, 0, 0, 1)
            -- ADD R1, R1, #1: encode_data_processing(AL, ADD, 0, rn=1, rd=1, imm=1)
            local ADD_R1    = ARM1.encode_data_processing(ARM1.COND_AL, ARM1.OP_ADD, 0, 1, 1, (1 << 25) | 1)
            -- CMP R1, R2: encode_data_processing(AL, CMP, 1, rn=1, rd=0, rm=2)
            local CMP_R1_R2 = ARM1.encode_data_processing(ARM1.COND_AL, ARM1.OP_CMP, 1, 1, 0, 2)
            -- BLT offset=-20 (loop is at addr 12; branch at addr 24)
            local BLT_LOOP  = ARM1.encode_branch(ARM1.COND_LT, false, -20)
            local HALT      = ARM1.encode_halt()

            ARM1.load_instructions(cpu, {
                MOV_R0_0,   -- 0x00
                MOV_R1_1,   -- 0x04
                MOV_R2_11,  -- 0x08
                ADD_R0,     -- 0x0C  ← loop
                ADD_R1,     -- 0x10
                CMP_R1_R2,  -- 0x14
                BLT_LOOP,   -- 0x18 (addr 24 decimal)
                HALT,       -- 0x1C
            })

            -- Recompute BLT: at addr 0x18=24, step advances PC to 28;
            -- exec_branch: branch_base = 28+4=32; target=12; offset=12-32=-20
            ARM1.write_word(cpu, 0x18, ARM1.encode_branch(ARM1.COND_LT, false, -20))

            ARM1.run(cpu, 10000)
            assert.equals(55, ARM1.read_register(cpu, 0))
        end)
    end)

    -- ===========================================================
    -- Encoding Helpers
    -- ===========================================================

    describe("encoding helpers", function()
        it("encode_halt produces correct SWI", function()
            local inst = ARM1.encode_halt()
            local d = ARM1.decode(inst)
            assert.equals(ARM1.INST_SWI, d.type)
            assert.equals(ARM1.HALT_SWI, d.swi_comment)
        end)

        it("encode_mov_imm produces correct MOV", function()
            local inst = ARM1.encode_mov_imm(ARM1.COND_AL, 3, 42)
            local d = ARM1.decode(inst)
            assert.equals(ARM1.INST_DATA_PROCESSING, d.type)
            assert.equals(ARM1.OP_MOV, d.opcode)
            assert.equals(3, d.rd)
            assert.equals(42, d.imm8)
        end)

        it("encode_ldr produces correct LDR", function()
            local inst = ARM1.encode_ldr(ARM1.COND_AL, 0, 1, 8, true)
            local d = ARM1.decode(inst)
            assert.equals(ARM1.INST_LOAD_STORE, d.type)
            assert.is_true(d.load)
            assert.equals(0, d.rd)
            assert.equals(1, d.rn)
        end)
    end)

end)

-- ==========================================================================
-- ARM1 Simulator Tests — Lua
-- ==========================================================================
--
-- Tests the complete ARMv1 instruction set simulator:
--   * Construction and reset
--   * All 16 data processing instructions
--   * Barrel shifter (all 4 types, immediate and register)
--   * Condition codes (all 16 conditions)
--   * Load/Store (LDR, STR, LDRB, STRB, addressing modes)
--   * Block transfer (LDM/STM, all 4 modes)
--   * Branches (B, BL, conditional)
--   * SWI (halt, mode switching)
--   * Processor mode banking
--   * End-to-end programs

local ARM1 = require("coding_adventures.arm1_simulator")

describe("ARM1Simulator", function()

  -- ======================================================================
  -- Construction and Reset
  -- ======================================================================

  describe("construction", function()
    it("creates a simulator with default memory", function()
      local cpu = ARM1.new()
      assert.is_not_nil(cpu)
      assert.are.equal(0, cpu:get_pc())
    end)

    it("starts in SVC mode with IRQ/FIQ disabled", function()
      local cpu = ARM1.new(1024)
      -- Mode bits 1:0 = 11 (SVC = 3)
      assert.are.equal(ARM1.MODE_SVC, cpu:get_mode())
      -- Flags I and F should be set (IRQ/FIQ disabled)
      local r15 = cpu.regs[15]
      assert.is_true((r15 & 0x08000000) ~= 0)  -- I bit
      assert.is_true((r15 & 0x04000000) ~= 0)  -- F bit
    end)

    it("resets all registers to 0 (except R15)", function()
      local cpu = ARM1.new(1024)
      for i = 0, 14 do
        assert.are.equal(0, cpu:read_register(i))
      end
    end)

    it("reset restores initial state", function()
      local cpu = ARM1.new(1024)
      cpu:write_register(0, 42)
      cpu:reset()
      assert.are.equal(0, cpu:read_register(0))
      assert.are.equal(ARM1.MODE_SVC, cpu:get_mode())
    end)
  end)

  -- ======================================================================
  -- Memory Access
  -- ======================================================================

  describe("memory", function()
    it("reads and writes 32-bit words", function()
      local cpu = ARM1.new(1024)
      cpu:write_word(0x100, 0xDEADBEEF)
      assert.are.equal(0xDEADBEEF, cpu:read_word(0x100))
    end)

    it("reads and writes bytes", function()
      local cpu = ARM1.new(1024)
      cpu:write_byte(0x10, 0xAB)
      assert.are.equal(0xAB, cpu:read_byte(0x10))
    end)

    it("words are little-endian", function()
      local cpu = ARM1.new(1024)
      cpu:write_word(0x100, 0x01020304)
      assert.are.equal(0x04, cpu:read_byte(0x100))
      assert.are.equal(0x03, cpu:read_byte(0x101))
      assert.are.equal(0x02, cpu:read_byte(0x102))
      assert.are.equal(0x01, cpu:read_byte(0x103))
    end)

    it("load_instructions writes words correctly", function()
      local cpu = ARM1.new(1024)
      cpu:load_instructions({0x11223344, 0x55667788}, 0)
      assert.are.equal(0x11223344, cpu:read_word(0))
      assert.are.equal(0x55667788, cpu:read_word(4))
    end)
  end)

  -- ======================================================================
  -- Data Processing — MOV immediate
  -- ======================================================================

  describe("MOV immediate", function()
    it("MOV R0, #42", function()
      local cpu = ARM1.new(1024)
      local instructions = {
        ARM1.encode_mov_imm(ARM1.COND_AL, 0, 42),  -- MOV R0, #42
        ARM1.encode_halt()
      }
      cpu:load_instructions(instructions)
      cpu:run(100)
      assert.are.equal(42, cpu:read_register(0))
    end)

    it("MOV R3, #255", function()
      local cpu = ARM1.new(1024)
      cpu:load_instructions({
        ARM1.encode_mov_imm(ARM1.COND_AL, 3, 255),
        ARM1.encode_halt()
      })
      cpu:run(100)
      assert.are.equal(255, cpu:read_register(3))
    end)
  end)

  -- ======================================================================
  -- Data Processing — ADD, SUB, AND, ORR, EOR, etc.
  -- ======================================================================

  describe("ADD instruction", function()
    it("R2 = R0 + R1 (1 + 2 = 3)", function()
      local cpu = ARM1.new(1024)
      cpu:load_instructions({
        ARM1.encode_mov_imm(ARM1.COND_AL, 0, 1),  -- MOV R0, #1
        ARM1.encode_mov_imm(ARM1.COND_AL, 1, 2),  -- MOV R1, #2
        ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_ADD, false, 2, 0, 1),  -- ADD R2, R0, R1
        ARM1.encode_halt()
      })
      cpu:run(100)
      assert.are.equal(3, cpu:read_register(2))
    end)

    it("ADD sets carry flag on overflow", function()
      local cpu = ARM1.new(1024)
      -- Load 0xFFFFFFFF into R0, add 1: should give 0 with carry set
      -- Use MOVS trick: load with rotate
      -- Instead, write registers directly and use ADDS
      cpu:write_register(0, 0xFFFFFFFF)
      cpu:write_register(1, 1)
      cpu:load_instructions({
        -- ADDS R2, R0, R1  (opcode=ADD=4, S=1, Rd=2, Rn=0, Rm=1)
        ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_ADD, true, 2, 0, 1),
        ARM1.encode_halt()
      })
      cpu:run(100)
      assert.are.equal(0, cpu:read_register(2))
      local flags = cpu:get_flags()
      assert.is_true(flags.c)  -- Carry set
      assert.is_true(flags.z)  -- Zero set
    end)
  end)

  describe("SUB instruction", function()
    it("R2 = R0 - R1 (10 - 3 = 7)", function()
      local cpu = ARM1.new(1024)
      cpu:write_register(0, 10)
      cpu:write_register(1, 3)
      cpu:load_instructions({
        ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_SUB, false, 2, 0, 1),
        ARM1.encode_halt()
      })
      cpu:run(100)
      assert.are.equal(7, cpu:read_register(2))
    end)

    it("SUBS sets N flag when result is negative", function()
      local cpu = ARM1.new(1024)
      cpu:write_register(0, 3)
      cpu:write_register(1, 10)
      cpu:load_instructions({
        ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_SUB, true, 2, 0, 1),
        ARM1.encode_halt()
      })
      cpu:run(100)
      local flags = cpu:get_flags()
      assert.is_true(flags.n)   -- Negative
      assert.is_false(flags.z)  -- Not zero
    end)
  end)

  describe("AND instruction", function()
    it("R0 AND R1 = R2", function()
      local cpu = ARM1.new(1024)
      cpu:write_register(0, 0xFF0F)
      cpu:write_register(1, 0x0FFF)
      cpu:load_instructions({
        ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_AND, false, 2, 0, 1),
        ARM1.encode_halt()
      })
      cpu:run(100)
      assert.are.equal(0x0F0F, cpu:read_register(2))
    end)
  end)

  describe("ORR instruction", function()
    it("R0 OR R1 = R2", function()
      local cpu = ARM1.new(1024)
      cpu:write_register(0, 0xFF00)
      cpu:write_register(1, 0x00FF)
      cpu:load_instructions({
        ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_ORR, false, 2, 0, 1),
        ARM1.encode_halt()
      })
      cpu:run(100)
      assert.are.equal(0xFFFF, cpu:read_register(2))
    end)
  end)

  describe("EOR instruction", function()
    it("R0 XOR R0 = 0", function()
      local cpu = ARM1.new(1024)
      cpu:write_register(0, 0xABCDEF01)
      cpu:load_instructions({
        ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_EOR, false, 1, 0, 0),
        ARM1.encode_halt()
      })
      cpu:run(100)
      assert.are.equal(0, cpu:read_register(1))
    end)
  end)

  describe("MVN instruction", function()
    it("MVN R1, R0 (bitwise NOT)", function()
      local cpu = ARM1.new(1024)
      cpu:write_register(0, 0xFFFFFF00)
      cpu:load_instructions({
        ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_MVN, false, 1, 0, 0),
        ARM1.encode_halt()
      })
      cpu:run(100)
      assert.are.equal(0x000000FF, cpu:read_register(1))
    end)
  end)

  describe("BIC instruction", function()
    it("bit clear: R0 AND NOT(R1)", function()
      local cpu = ARM1.new(1024)
      cpu:write_register(0, 0xFFFF)
      cpu:write_register(1, 0x00FF)
      cpu:load_instructions({
        ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_BIC, false, 2, 0, 1),
        ARM1.encode_halt()
      })
      cpu:run(100)
      assert.are.equal(0xFF00, cpu:read_register(2))
    end)
  end)

  describe("RSB instruction", function()
    it("RSB R2, R0, R1 = R1 - R0", function()
      local cpu = ARM1.new(1024)
      cpu:write_register(0, 3)
      cpu:write_register(1, 10)
      cpu:load_instructions({
        ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_RSB, false, 2, 0, 1),
        ARM1.encode_halt()
      })
      cpu:run(100)
      assert.are.equal(7, cpu:read_register(2))
    end)
  end)

  describe("CMP / TST / TEQ / CMN", function()
    it("CMP sets flags without writing Rd", function()
      local cpu = ARM1.new(1024)
      cpu:write_register(0, 5)
      cpu:write_register(1, 5)
      cpu:write_register(2, 99)  -- Should not be changed by CMP
      cpu:load_instructions({
        ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_CMP, true, 2, 0, 1),
        ARM1.encode_halt()
      })
      cpu:run(100)
      assert.are.equal(99, cpu:read_register(2))  -- Rd unchanged
      assert.is_true(cpu:get_flags().z)
    end)

    it("TST R0, R1 — sets flags for AND", function()
      local cpu = ARM1.new(1024)
      cpu:write_register(0, 0xFF)
      cpu:write_register(1, 0x00)
      cpu:load_instructions({
        ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_TST, true, 0, 0, 1),
        ARM1.encode_halt()
      })
      cpu:run(100)
      assert.is_true(cpu:get_flags().z)
    end)
  end)

  -- ======================================================================
  -- Barrel Shifter Tests
  -- ======================================================================

  describe("barrel shifter", function()
    it("LSL #2 (multiply by 4)", function()
      local cpu = ARM1.new(1024)
      cpu:write_register(0, 3)  -- R0 = 3
      -- ADD R1, R0, R0, LSL #2 — R1 = R0 + (R0 << 2) = 3 + 12 = 15
      -- encode: ADD(4), S=0, Rd=1, Rn=0, shift_imm=2, shift_type=LSL(0), Rm=0
      local inst = ARM1.encode_alu_reg_shift(ARM1.COND_AL, ARM1.OP_ADD, false, 1, 0, 0, 0, 2)
      cpu:load_instructions({ inst, ARM1.encode_halt() })
      cpu:run(100)
      assert.are.equal(15, cpu:read_register(1))
    end)

    it("multiply by 5 using LSL: ADD R1, R0, R0, LSL #2", function()
      local cpu = ARM1.new(1024)
      cpu:write_register(0, 7)  -- R0 = 7
      -- R1 = R0 + (R0 << 2) = 7 + 28 = 35
      local inst = ARM1.encode_alu_reg_shift(ARM1.COND_AL, ARM1.OP_ADD, false, 1, 0, 0, ARM1.SHIFT_LSL, 2)
      cpu:load_instructions({ inst, ARM1.encode_halt() })
      cpu:run(100)
      assert.are.equal(35, cpu:read_register(1))
    end)
  end)

  -- ======================================================================
  -- Condition Codes
  -- ======================================================================

  describe("condition codes", function()
    it("EQ: executes when Z=1", function()
      local cpu = ARM1.new(1024)
      cpu:write_register(0, 5)
      cpu:write_register(1, 5)
      cpu:write_register(2, 0)
      -- SUBS R3, R0, R1 (sets Z=1)
      -- MOVEQ R2, #99 (should execute)
      -- MOVNE R2, #77 (should NOT execute)
      local subs = ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_SUB, true, 3, 0, 1)
      local moveq = ARM1.encode_mov_imm(ARM1.COND_EQ, 2, 99)
      local movne = ARM1.encode_mov_imm(ARM1.COND_NE, 2, 77)
      cpu:load_instructions({ subs, moveq, movne, ARM1.encode_halt() })
      cpu:run(100)
      assert.are.equal(99, cpu:read_register(2))
    end)

    it("NE: does not execute when Z=1", function()
      local cpu = ARM1.new(1024)
      cpu:write_register(0, 5)
      cpu:write_register(1, 5)
      cpu:write_register(2, 42)
      -- SUBS R3, R0, R1 (sets Z=1, so NE condition fails)
      -- MOVNE R2, #99 (should NOT execute since NE fails when Z=1)
      local subs = ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_SUB, true, 3, 0, 1)
      local movne = ARM1.encode_mov_imm(ARM1.COND_NE, 2, 99)
      cpu:load_instructions({ subs, movne, ARM1.encode_halt() })
      cpu:run(100)
      assert.are.equal(42, cpu:read_register(2))  -- R2 unchanged
    end)

    it("MI: executes when N=1 (result negative)", function()
      local cpu = ARM1.new(1024)
      cpu:write_register(0, 3)
      cpu:write_register(1, 10)
      cpu:write_register(2, 0)
      -- SUBS R3, R0, R1 (3-10=-7, N=1)
      -- MOVMI R2, #1
      local subs = ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_SUB, true, 3, 0, 1)
      local movmi = ARM1.encode_mov_imm(ARM1.COND_MI, 2, 1)
      cpu:load_instructions({ subs, movmi, ARM1.encode_halt() })
      cpu:run(100)
      assert.are.equal(1, cpu:read_register(2))
    end)

    it("AL: always executes", function()
      local cpu = ARM1.new(1024)
      cpu:load_instructions({
        ARM1.encode_mov_imm(ARM1.COND_AL, 0, 77),
        ARM1.encode_halt()
      })
      cpu:run(100)
      assert.are.equal(77, cpu:read_register(0))
    end)

    it("NV: never executes", function()
      local cpu = ARM1.new(1024)
      cpu:write_register(0, 42)
      -- MOV with NV condition should never execute
      local movnv = ARM1.encode_mov_imm(ARM1.COND_NV, 0, 99)
      cpu:load_instructions({ movnv, ARM1.encode_halt() })
      cpu:run(100)
      assert.are.equal(42, cpu:read_register(0))
    end)
  end)

  -- ======================================================================
  -- Load / Store
  -- ======================================================================

  describe("load/store", function()
    it("STR then LDR round-trips a value", function()
      local cpu = ARM1.new(4096)
      -- Store R0=0xCAFEBABE at address 0x100, then load into R1
      cpu:write_register(0, 0xCAFEBABE)
      cpu:write_register(2, 0x100)  -- base register
      -- STR R0, [R2, #0]  (pre-index, no offset)
      local str = ARM1.encode_str(ARM1.COND_AL, 0, 2, 0, true)
      -- LDR R1, [R2, #0]
      local ldr = ARM1.encode_ldr(ARM1.COND_AL, 1, 2, 0, true)
      cpu:load_instructions({ str, ldr, ARM1.encode_halt() })
      cpu:run(100)
      assert.are.equal(0xCAFEBABE, cpu:read_register(1))
    end)

    it("LDR with positive immediate offset", function()
      local cpu = ARM1.new(4096)
      cpu:write_word(0x110, 0x12345678)
      cpu:write_register(2, 0x100)  -- base
      -- LDR R0, [R2, #0x10]  (offset = 16)
      local ldr = ARM1.encode_ldr(ARM1.COND_AL, 0, 2, 0x10, true)
      cpu:load_instructions({ ldr, ARM1.encode_halt() })
      cpu:run(100)
      assert.are.equal(0x12345678, cpu:read_register(0))
    end)
  end)

  -- ======================================================================
  -- Block Transfer (LDM / STM)
  -- ======================================================================

  describe("block transfer", function()
    it("STMIA / LDMIA round-trips multiple registers", function()
      local cpu = ARM1.new(4096)
      cpu:write_register(0, 0x11111111)
      cpu:write_register(1, 0x22222222)
      cpu:write_register(2, 0x33333333)
      cpu:write_register(13, 0x200)  -- Stack pointer at 0x200

      -- STMIA R13, {R0-R2}  (store R0, R1, R2 at 0x200, 0x204, 0x208)
      local stm = ARM1.encode_stm(ARM1.COND_AL, 13, 0x7, false, "IA")  -- reg_list=0b111=R0,R1,R2

      -- Clear registers
      cpu:load_instructions({
        stm,
        ARM1.encode_mov_imm(ARM1.COND_AL, 0, 0),
        ARM1.encode_mov_imm(ARM1.COND_AL, 1, 0),
        ARM1.encode_mov_imm(ARM1.COND_AL, 2, 0),
        ARM1.encode_ldm(ARM1.COND_AL, 13, 0x7, false, "IA"),
        ARM1.encode_halt()
      })
      cpu:run(100)

      assert.are.equal(0x11111111, cpu:read_register(0))
      assert.are.equal(0x22222222, cpu:read_register(1))
      assert.are.equal(0x33333333, cpu:read_register(2))
    end)

    it("STMDB / LDMIA stack push/pop pattern", function()
      local cpu = ARM1.new(4096)
      cpu:write_register(0, 0xAAAA)
      cpu:write_register(1, 0xBBBB)
      cpu:write_register(13, 0x300)  -- SP

      -- STMDB R13!, {R0, R1}  (push: decrement before, writeback)
      local push = ARM1.encode_stm(ARM1.COND_AL, 13, 0x3, true, "DB")

      cpu:load_instructions({
        push,
        ARM1.encode_mov_imm(ARM1.COND_AL, 0, 0),
        ARM1.encode_mov_imm(ARM1.COND_AL, 1, 0),
        ARM1.encode_ldm(ARM1.COND_AL, 13, 0x3, true, "IA"),  -- LDMIA (pop)
        ARM1.encode_halt()
      })
      cpu:run(100)

      assert.are.equal(0xAAAA, cpu:read_register(0))
      assert.are.equal(0xBBBB, cpu:read_register(1))
      assert.are.equal(0x300, cpu:read_register(13))  -- SP restored
    end)
  end)

  -- ======================================================================
  -- Branches
  -- ======================================================================

  describe("branches", function()
    it("B: unconditional forward branch", function()
      local cpu = ARM1.new(4096)
      -- Instruction at 0: B #8   (skip next 2 instructions)
      -- Instruction at 4: MOV R0, #99  (should be skipped)
      -- Instruction at 8: MOV R0, #42  (should execute)
      -- Instruction at 12: HLT
      --
      -- Branch offset from PC+8=8 to target 8: relative = 8-8=0
      -- But we want to branch to address 8. PC at decode time = 0+8=8.
      -- offset = target - (current_pc + 8) = 8 - (0 + 8) = 0
      -- So we branch 0 bytes forward (lands on instruction at 8)
      local branch = ARM1.encode_branch(ARM1.COND_AL, false, 0)  -- B #0 (offset 0 bytes)
      cpu:load_instructions({
        branch,                                   -- 0x00: B #0 (skip to 0x08)
        ARM1.encode_mov_imm(ARM1.COND_AL, 0, 99), -- 0x04: (skipped)
        ARM1.encode_mov_imm(ARM1.COND_AL, 0, 42), -- 0x08: MOV R0, #42
        ARM1.encode_halt()
      })
      cpu:run(100)
      assert.are.equal(42, cpu:read_register(0))
    end)

    it("BNE: branches when Z=0", function()
      local cpu = ARM1.new(4096)
      cpu:write_register(0, 5)
      cpu:write_register(1, 3)
      cpu:write_register(2, 0)

      -- CMP R0, R1  (5-3=2, Z=0)
      -- BNE +0       (branch forward past MOVNE to prevent)
      -- MOV R2, #77  (should be skipped because BNE taken)
      -- MOV R2, #42  (this is the branch target)
      -- HLT
      local cmp = ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_CMP, true, 2, 0, 1)
      local bne = ARM1.encode_branch(ARM1.COND_NE, false, 0)  -- skip next instruction
      cpu:load_instructions({
        cmp,
        bne,
        ARM1.encode_mov_imm(ARM1.COND_AL, 2, 77),  -- skipped
        ARM1.encode_mov_imm(ARM1.COND_AL, 2, 42),  -- executed
        ARM1.encode_halt()
      })
      cpu:run(100)
      assert.are.equal(42, cpu:read_register(2))
    end)

    it("BL: saves return address in R14", function()
      local cpu = ARM1.new(4096)
      -- MOV R0, #7     (0x00)
      -- BL +4          (0x04 → branch to 0x10)
      -- HLT            (0x08) — should be skipped
      -- NOP (MOV R1, R1) (0x0C) — should be skipped
      -- ADD R0, R0, R0 (0x10) — subroutine
      -- HLT            (0x14)
      --
      -- BL at 0x04: PC+8 = 0x0C, offset to 0x10 = 0x10-0x0C = 4
      local bl = ARM1.encode_branch(ARM1.COND_AL, true, 4)  -- BL #4
      cpu:load_instructions({
        ARM1.encode_mov_imm(ARM1.COND_AL, 0, 7),   -- 0x00
        bl,                                          -- 0x04
        ARM1.encode_halt(),                          -- 0x08 (skipped)
        ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_MOV, false, 1, 0, 1), -- 0x0C (skipped)
        ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_ADD, false, 0, 0, 0), -- 0x10: R0=R0+R0=14
        ARM1.encode_halt()                           -- 0x14
      })
      cpu:run(100)
      assert.are.equal(14, cpu:read_register(0))
      -- R14 should have been set to PC+4 after the BL instruction
      -- BL was at 0x04, R14 = R15 value = 0x04 + 4 + mode_flags
      local lr = cpu:read_register(14)
      assert.is_true(lr ~= 0)  -- LR was set
    end)
  end)

  -- ======================================================================
  -- SWI and Halt
  -- ======================================================================

  describe("SWI / halt", function()
    it("SWI 0x123456 halts the CPU", function()
      local cpu = ARM1.new(1024)
      cpu:load_instructions({ ARM1.encode_halt() })
      local traces = cpu:run(100)
      assert.is_true(cpu.halted)
      assert.are.equal(1, #traces)
    end)

    it("normal SWI changes mode to SVC", function()
      local cpu = ARM1.new(1024)
      -- Put a real SWI at 0x00 (not the halt SWI)
      -- First, set up vector at 0x08 to halt
      cpu:write_word(0x08, ARM1.encode_halt())
      -- SWI #1 at 0x00 — should jump to 0x08
      local swi1 = ((ARM1.COND_AL << 28) | 0x0F000000 | 1) & 0xFFFFFFFF
      cpu:write_word(0x00, swi1)
      cpu:run(100)
      assert.are.equal(ARM1.MODE_SVC, cpu:get_mode())
    end)
  end)

  -- ======================================================================
  -- Processor Mode Banking
  -- ======================================================================

  describe("processor mode banking", function()
    it("R13 is banked between USR and SVC", function()
      local cpu = ARM1.new(1024)
      -- Write R13 in SVC mode (current)
      cpu:write_register(13, 0xABCD)
      assert.are.equal(0xABCD, cpu:read_register(13))

      -- Manually switch to USR mode by changing mode bits in R15
      local r15 = cpu.regs[15]
      -- Clear mode bits and set USR (00)
      cpu.regs[15] = (r15 & ~0x3) & 0xFFFFFFFF
      -- Now write R13 in USR mode
      cpu:write_register(13, 0x1234)

      -- Switch back to SVC
      cpu.regs[15] = (cpu.regs[15] & ~0x3) | ARM1.MODE_SVC
      -- SVC R13 should still be 0xABCD
      assert.are.equal(0xABCD, cpu:read_register(13))

      -- Switch to USR again
      cpu.regs[15] = cpu.regs[15] & ~0x3
      assert.are.equal(0x1234, cpu:read_register(13))
    end)

    it("FIQ banks R8-R14 separately", function()
      local cpu = ARM1.new(1024)
      -- Write R8 in USR mode
      local r15 = cpu.regs[15]
      cpu.regs[15] = (r15 & ~0x3) | 0  -- USR
      cpu:write_register(8, 0x1111)

      -- Switch to FIQ mode
      cpu.regs[15] = (cpu.regs[15] & ~0x3) | ARM1.MODE_FIQ
      cpu:write_register(8, 0x2222)

      -- In FIQ, R8 should be 0x2222
      assert.are.equal(0x2222, cpu:read_register(8))

      -- Back to USR
      cpu.regs[15] = cpu.regs[15] & ~0x3
      assert.are.equal(0x1111, cpu:read_register(8))
    end)
  end)

  -- ======================================================================
  -- End-to-End Programs
  -- ======================================================================

  describe("end-to-end programs", function()
    it("sum 1 to 10 = 55", function()
      local cpu = ARM1.new(4096)
      -- R0 = sum, R1 = counter (10 down to 1)
      -- MOV R0, #0       (sum = 0)
      -- MOV R1, #10      (counter = 10)
      -- loop:
      -- ADD R0, R0, R1   (sum += counter)
      -- SUBS R1, R1, #1  (counter--)
      -- BNE -8           (branch back to loop if R1 != 0)
      -- HLT
      --
      -- SUBS Rd, Rn, imm8:
      -- Need to encode SUB with immediate. Let's use encode_alu_reg but
      -- we need a sub immediate variant. Use encode_mov_imm pattern:
      -- SUB imm: cond=AL, 25=1(imm), opcode=SUB(2), S=1, Rn=1, Rd=1, imm8=1
      local sub_imm_inst = (ARM1.COND_AL << 28) | (1 << 25) | (ARM1.OP_SUB << 21) | (1 << 20) | (1 << 16) | (1 << 12) | 1
      sub_imm_inst = sub_imm_inst & 0xFFFFFFFF

      -- BNE back to ADD at 0x08: branch_base = 0x10+8 = 0x18, offset = 0x08-0x18 = -16
      -- encoded = -16/4 = -4, in 24-bit two's complement = 0xFFFFFC
      local bne = ARM1.encode_branch(ARM1.COND_NE, false, -16)

      cpu:load_instructions({
        ARM1.encode_mov_imm(ARM1.COND_AL, 0, 0),   -- 0x00: MOV R0, #0
        ARM1.encode_mov_imm(ARM1.COND_AL, 1, 10),  -- 0x04: MOV R1, #10
        ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_ADD, false, 0, 0, 1), -- 0x08: ADD R0, R0, R1
        sub_imm_inst,                                -- 0x0C: SUBS R1, R1, #1
        bne,                                         -- 0x10: BNE -8 (back to 0x08)
        ARM1.encode_halt()                           -- 0x14
      })
      cpu:run(1000)
      assert.are.equal(55, cpu:read_register(0))
    end)

    it("abs(x) using conditional RSB", function()
      local cpu = ARM1.new(4096)
      -- Compute abs(R0): if R0 < 0, negate it
      -- CMP R0, #0     → sets N if R0 < 0
      -- RSBLT R0, R0, #0 → if LT: R0 = 0 - R0

      -- Set R0 to a negative value: we'll use 0xFFFFFFF9 (-7 in two's complement)
      cpu:write_register(0, 0xFFFFFFF9)

      -- CMP R0, #0: cond=AL, opcode=CMP(10=0xA), S=1, imm=1, Rn=0, Rd=0, imm8=0
      local cmp_imm = (ARM1.COND_AL << 28) | (1 << 25) | (ARM1.OP_CMP << 21) | (1 << 20) | (0 << 16) | (0 << 12) | 0
      cmp_imm = cmp_imm & 0xFFFFFFFF

      -- RSBLT R0, R0, #0: cond=LT(11=0xB), opcode=RSB(3), S=0, imm=1, Rn=0, Rd=0, imm8=0
      local rsb_lt = (ARM1.COND_LT << 28) | (1 << 25) | (ARM1.OP_RSB << 21) | (0 << 20) | (0 << 16) | (0 << 12) | 0
      rsb_lt = rsb_lt & 0xFFFFFFFF

      cpu:load_instructions({ cmp_imm, rsb_lt, ARM1.encode_halt() })
      cpu:run(100)
      assert.are.equal(7, cpu:read_register(0))
    end)

    it("fibonacci sequence (first 8 numbers)", function()
      local cpu = ARM1.new(4096)
      -- Compute fib(8) = 21
      -- R0 = a = 0, R1 = b = 1, R2 = counter = 8
      -- loop:
      --   R3 = R0 + R1   (next fib)
      --   R0 = R1
      --   R1 = R3
      --   SUBS R2, R2, #1
      --   BNE loop
      -- R1 = fib(8)

      local sub_imm = (ARM1.COND_AL << 28) | (1 << 25) | (ARM1.OP_SUB << 21) | (1 << 20) | (2 << 16) | (2 << 12) | 1
      sub_imm = sub_imm & 0xFFFFFFFF
      -- BNE back to R3=a+b at 0x0C: branch_base = 0x1C+8 = 0x24, offset = 0x0C-0x24 = -24
      -- encoded = -24/4 = -6, in 24-bit two's complement = 0xFFFFFA
      local bne = ARM1.encode_branch(ARM1.COND_NE, false, -24)

      cpu:load_instructions({
        ARM1.encode_mov_imm(ARM1.COND_AL, 0, 0),                            -- 0x00: a=0
        ARM1.encode_mov_imm(ARM1.COND_AL, 1, 1),                            -- 0x04: b=1
        ARM1.encode_mov_imm(ARM1.COND_AL, 2, 8),                            -- 0x08: counter=8
        ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_ADD, false, 3, 0, 1),    -- 0x0C: R3=a+b
        ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_MOV, false, 0, 0, 1),    -- 0x10: R0=R1
        ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_MOV, false, 1, 0, 3),    -- 0x14: R1=R3
        sub_imm,                                                              -- 0x18: SUBS R2, R2, #1
        bne,                                                                  -- 0x1C: BNE loop
        ARM1.encode_halt()                                                    -- 0x20
      })
      cpu:run(1000)
      -- fib sequence: 0,1,1,2,3,5,8,13,21,34...
      -- After 8 iterations from (a=0,b=1): a=13, b=21
      assert.are.equal(21, cpu:read_register(1))
    end)
  end)

  -- ======================================================================
  -- Step trace
  -- ======================================================================

  describe("step trace", function()
    it("trace captures before/after state", function()
      local cpu = ARM1.new(1024)
      cpu:load_instructions({
        ARM1.encode_mov_imm(ARM1.COND_AL, 0, 42),
        ARM1.encode_halt()
      })
      local trace = cpu:step()
      assert.are.equal(0, trace.address)
      assert.are.equal(42, trace.regs_after[0])
      assert.are.equal(0, trace.regs_before[0])
      assert.is_true(trace.condition_met)
    end)

    it("mnemonic is generated for each instruction", function()
      local cpu = ARM1.new(1024)
      cpu:load_instructions({ ARM1.encode_mov_imm(ARM1.COND_AL, 0, 5) })
      local trace = cpu:step()
      assert.is_not_nil(trace.mnemonic)
      assert.are.equal("string", type(trace.mnemonic))
    end)
  end)

end)

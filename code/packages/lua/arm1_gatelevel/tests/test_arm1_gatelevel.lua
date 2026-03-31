-- Tests for coding_adventures.arm1_gatelevel
-- Verifies that the gate-level simulator produces identical results to
-- the behavioral simulator while routing through actual gate function calls.

local GL   = require("coding_adventures.arm1_gatelevel")
local ARM1 = require("coding_adventures.arm1_simulator")

-- ===========================================================================
-- Bit Conversion Helpers
-- ===========================================================================

describe("bit conversion", function()
  it("int_to_bits: converts 0", function()
    local bits = GL.int_to_bits(0, 8)
    for i = 1, 8 do assert.equals(0, bits[i]) end
  end)

  it("int_to_bits: converts 5 (binary 101)", function()
    local bits = GL.int_to_bits(5, 8)
    assert.equals(1, bits[1])  -- bit 0
    assert.equals(0, bits[2])  -- bit 1
    assert.equals(1, bits[3])  -- bit 2
  end)

  it("bits_to_int: roundtrip", function()
    for _, v in ipairs({0, 1, 42, 255, 0xDEADBEEF}) do
      local bits = GL.int_to_bits(v & 0xFFFFFFFF, 32)
      assert.equals(v & 0xFFFFFFFF, GL.bits_to_int(bits))
    end
  end)
end)

-- ===========================================================================
-- Gate-Level ALU
-- ===========================================================================

describe("gate-level ALU", function()
  local function make_bits(v)
    return GL.int_to_bits(v & 0xFFFFFFFF, 32)
  end

  it("ADD: 1 + 2 = 3", function()
    local r = GL.gate_alu_execute(ARM1.OP_ADD, make_bits(1), make_bits(2), 0, 0, 0)
    assert.equals(3, GL.bits_to_int(r.result_bits))
    assert.equals(0, r.n)
    assert.equals(0, r.z)
  end)

  it("ADD: overflow sets carry and zero", function()
    local r = GL.gate_alu_execute(ARM1.OP_ADD, make_bits(0xFFFFFFFF), make_bits(1), 0, 0, 0)
    assert.equals(0, GL.bits_to_int(r.result_bits))
    assert.equals(1, r.c)
    assert.equals(1, r.z)
  end)

  it("SUB: 10 - 3 = 7", function()
    local r = GL.gate_alu_execute(ARM1.OP_SUB, make_bits(10), make_bits(3), 0, 0, 0)
    assert.equals(7, GL.bits_to_int(r.result_bits))
  end)

  it("AND", function()
    local r = GL.gate_alu_execute(ARM1.OP_AND, make_bits(0xFF0F), make_bits(0x0FFF), 0, 0, 0)
    assert.equals(0x0F0F, GL.bits_to_int(r.result_bits))
  end)

  it("ORR", function()
    local r = GL.gate_alu_execute(ARM1.OP_ORR, make_bits(0xFF00), make_bits(0x00FF), 0, 0, 0)
    assert.equals(0xFFFF, GL.bits_to_int(r.result_bits))
  end)

  it("EOR (XOR with self = 0)", function()
    local r = GL.gate_alu_execute(ARM1.OP_EOR, make_bits(0xABCDEF01), make_bits(0xABCDEF01), 0, 0, 0)
    assert.equals(0, GL.bits_to_int(r.result_bits))
    assert.equals(1, r.z)
  end)

  it("MVN (bitwise NOT)", function()
    local r = GL.gate_alu_execute(ARM1.OP_MVN, make_bits(0), make_bits(0xFFFFFF00), 0, 0, 0)
    assert.equals(0x000000FF, GL.bits_to_int(r.result_bits))
  end)

  it("BIC (bit clear)", function()
    local r = GL.gate_alu_execute(ARM1.OP_BIC, make_bits(0xFFFF), make_bits(0x00FF), 0, 0, 0)
    assert.equals(0xFF00, GL.bits_to_int(r.result_bits))
  end)

  it("MOV", function()
    local r = GL.gate_alu_execute(ARM1.OP_MOV, make_bits(0), make_bits(42), 0, 0, 0)
    assert.equals(42, GL.bits_to_int(r.result_bits))
  end)

  it("N flag set on negative result", function()
    -- SUB 3 - 10 = -7 (in two's complement)
    local r = GL.gate_alu_execute(ARM1.OP_SUB, make_bits(3), make_bits(10), 0, 0, 0)
    assert.equals(1, r.n)
  end)
end)

-- ===========================================================================
-- Gate-Level Barrel Shifter
-- ===========================================================================

describe("gate-level barrel shifter", function()
  local function bits(v) return GL.int_to_bits(v & 0xFFFFFFFF, 32) end

  it("LSL #0 = pass-through", function()
    local result, carry = GL.gate_barrel_shift(bits(42), ARM1.SHIFT_LSL, 0, 0, false)
    assert.equals(42, GL.bits_to_int(result))
    assert.equals(0, carry)
  end)

  it("LSL #1 = multiply by 2", function()
    local result, _ = GL.gate_barrel_shift(bits(5), ARM1.SHIFT_LSL, 1, 0, false)
    assert.equals(10, GL.bits_to_int(result))
  end)

  it("LSL #2 = multiply by 4", function()
    local result, _ = GL.gate_barrel_shift(bits(3), ARM1.SHIFT_LSL, 2, 0, false)
    assert.equals(12, GL.bits_to_int(result))
  end)

  it("LSR #1 = divide by 2", function()
    local result, _ = GL.gate_barrel_shift(bits(10), ARM1.SHIFT_LSR, 1, 0, false)
    assert.equals(5, GL.bits_to_int(result))
  end)

  it("ASR preserves sign bit", function()
    -- 0xFFFFFFFC >> 2 (arithmetic) = 0xFFFFFFFF (-1 for signed)
    local result, _ = GL.gate_barrel_shift(bits(0xFFFFFFFC), ARM1.SHIFT_ASR, 2, 0, false)
    assert.equals(0xFFFFFFFF, GL.bits_to_int(result))
  end)

  it("ROR #8 rotates correctly", function()
    -- 0x12345678 ROR 8 = 0x78123456
    local result, _ = GL.gate_barrel_shift(bits(0x12345678), ARM1.SHIFT_ROR, 8, 0, false)
    assert.equals(0x78123456, GL.bits_to_int(result))
  end)

  it("RRX: rotate through carry (amount=0, not by_reg)", function()
    -- 0x10 RRX with carry=1: MSB gets carry=1, LSB shifts to carry
    -- 0x10 = ...000010000; after RRX with C=1: 0x80000008, carry=0
    local result, carry = GL.gate_barrel_shift(bits(0x10), ARM1.SHIFT_ROR, 0, 1, false)
    assert.equals(1, carry == 0 and 1 or 0, "carry should be 0")  -- bit 0 of 0x10 = 0
    assert.equals(1, result[32])  -- MSB = old carry = 1
  end)
end)

-- ===========================================================================
-- Gate-Level Immediate Decode
-- ===========================================================================

describe("gate_decode_immediate", function()
  it("rotate=0 returns value unchanged", function()
    local bits, carry = GL.gate_decode_immediate(42, 0)
    assert.equals(42, GL.bits_to_int(bits))
    assert.equals(0, carry)
  end)

  it("rotate=1 → ROR #2", function()
    -- imm8=1, rotate=1 → value = 1 ROR 2 = 0x40000000
    local bits, _ = GL.gate_decode_immediate(1, 1)
    assert.equals(0x40000000, GL.bits_to_int(bits))
  end)
end)

-- ===========================================================================
-- Full Simulation Tests (same as behavioral simulator)
-- ===========================================================================

describe("gate-level ARM1 simulation", function()
  it("construction: PC=0, mode=SVC, I/F set", function()
    local cpu = GL.new(1024)
    assert.equals(0, GL.get_pc(cpu))
    assert.equals(ARM1.MODE_SVC, GL.get_mode(cpu))
    local r15 = GL.bits_to_int(cpu.regs[15])
    assert.is_true((r15 & ARM1.FLAG_I) ~= 0)
    assert.is_true((r15 & ARM1.FLAG_F) ~= 0)
  end)

  it("reset clears registers", function()
    local cpu = GL.new(1024)
    GL.write_register(cpu, 0, 42)
    GL.reset(cpu)
    assert.equals(0, GL.read_register(cpu, 0))
    assert.equals(ARM1.MODE_SVC, GL.get_mode(cpu))
  end)

  it("memory: word round-trip", function()
    local cpu = GL.new(4096)
    GL.write_word(cpu, 0x100, 0xDEADBEEF)
    assert.equals(0xDEADBEEF, GL.read_word(cpu, 0x100))
  end)

  it("memory: byte round-trip", function()
    local cpu = GL.new(4096)
    GL.write_byte(cpu, 0x10, 0xAB)
    assert.equals(0xAB, GL.read_byte(cpu, 0x10))
  end)

  it("memory: little-endian", function()
    local cpu = GL.new(4096)
    GL.write_word(cpu, 0x200, 0x01020304)
    assert.equals(0x04, GL.read_byte(cpu, 0x200))
    assert.equals(0x03, GL.read_byte(cpu, 0x201))
    assert.equals(0x02, GL.read_byte(cpu, 0x202))
    assert.equals(0x01, GL.read_byte(cpu, 0x203))
  end)

  it("MOV immediate", function()
    local cpu = GL.new(1024)
    GL.load_instructions(cpu, {
      ARM1.encode_mov_imm(ARM1.COND_AL, 0, 42),
      ARM1.encode_halt(),
    })
    GL.run(cpu, 100)
    assert.equals(42, GL.read_register(cpu, 0))
  end)

  it("ADD R2 = R0 + R1", function()
    local cpu = GL.new(1024)
    GL.load_instructions(cpu, {
      ARM1.encode_mov_imm(ARM1.COND_AL, 0, 1),
      ARM1.encode_mov_imm(ARM1.COND_AL, 1, 2),
      ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_ADD, 0, 2, 0, 1),
      ARM1.encode_halt(),
    })
    GL.run(cpu, 100)
    assert.equals(3, GL.read_register(cpu, 2))
  end)

  it("ADD carry flag", function()
    local cpu = GL.new(1024)
    GL.write_register(cpu, 0, 0xFFFFFFFF)
    GL.write_register(cpu, 1, 1)
    GL.load_instructions(cpu, {
      ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_ADD, 1, 2, 0, 1),
      ARM1.encode_halt(),
    })
    GL.run(cpu, 100)
    assert.equals(0, GL.read_register(cpu, 2))
    local flags = GL.get_flags(cpu)
    assert.is_true(flags.c)
    assert.is_true(flags.z)
  end)

  it("SUB", function()
    local cpu = GL.new(1024)
    GL.write_register(cpu, 0, 10)
    GL.write_register(cpu, 1, 3)
    GL.load_instructions(cpu, {
      ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_SUB, 0, 2, 0, 1),
      ARM1.encode_halt(),
    })
    GL.run(cpu, 100)
    assert.equals(7, GL.read_register(cpu, 2))
  end)

  it("AND", function()
    local cpu = GL.new(1024)
    GL.write_register(cpu, 0, 0xFF0F)
    GL.write_register(cpu, 1, 0x0FFF)
    GL.load_instructions(cpu, {
      ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_AND, 0, 2, 0, 1),
      ARM1.encode_halt(),
    })
    GL.run(cpu, 100)
    assert.equals(0x0F0F, GL.read_register(cpu, 2))
  end)

  it("barrel shifter via ADD with LSL", function()
    local cpu = GL.new(1024)
    GL.write_register(cpu, 0, 3)
    -- ADD R1, R0, R0, LSL #2 → R1 = 3 + 12 = 15
    local inst = ARM1.encode_alu_reg_shift(ARM1.COND_AL, ARM1.OP_ADD, 0, 1, 0, 0, ARM1.SHIFT_LSL, 2)
    GL.load_instructions(cpu, { inst, ARM1.encode_halt() })
    GL.run(cpu, 100)
    assert.equals(15, GL.read_register(cpu, 1))
  end)

  it("STR/LDR round-trip", function()
    local cpu = GL.new(4096)
    GL.write_register(cpu, 0, 0xCAFEBABE)
    GL.write_register(cpu, 2, 0x100)
    GL.load_instructions(cpu, {
      ARM1.encode_str(ARM1.COND_AL, 0, 2, 0, 1),
      ARM1.encode_ldr(ARM1.COND_AL, 1, 2, 0, 1),
      ARM1.encode_halt(),
    })
    GL.run(cpu, 100)
    assert.equals(0xCAFEBABE, GL.read_register(cpu, 1))
  end)

  it("STMIA/LDMIA round-trip", function()
    local cpu = GL.new(4096)
    GL.write_register(cpu, 0, 0x11111111)
    GL.write_register(cpu, 1, 0x22222222)
    GL.write_register(cpu, 2, 0x33333333)
    GL.write_register(cpu, 13, 0x200)
    GL.load_instructions(cpu, {
      ARM1.encode_stm(ARM1.COND_AL, 13, 0x7, 0, 'IA'),
      ARM1.encode_mov_imm(ARM1.COND_AL, 0, 0),
      ARM1.encode_mov_imm(ARM1.COND_AL, 1, 0),
      ARM1.encode_mov_imm(ARM1.COND_AL, 2, 0),
      ARM1.encode_ldm(ARM1.COND_AL, 13, 0x7, 0, 'IA'),
      ARM1.encode_halt(),
    })
    GL.run(cpu, 100)
    assert.equals(0x11111111, GL.read_register(cpu, 0))
    assert.equals(0x22222222, GL.read_register(cpu, 1))
    assert.equals(0x33333333, GL.read_register(cpu, 2))
  end)

  it("B forward branch", function()
    local cpu = GL.new(4096)
    local branch = ARM1.encode_branch(ARM1.COND_AL, 0, 0)
    GL.load_instructions(cpu, {
      branch,
      ARM1.encode_mov_imm(ARM1.COND_AL, 0, 99),
      ARM1.encode_mov_imm(ARM1.COND_AL, 0, 42),
      ARM1.encode_halt(),
    })
    GL.run(cpu, 100)
    assert.equals(42, GL.read_register(cpu, 0))
  end)

  it("halt", function()
    local cpu = GL.new(1024)
    GL.load_instructions(cpu, { ARM1.encode_halt() })
    local traces = GL.run(cpu, 100)
    assert.is_true(cpu.halted)
    assert.equals(1, #traces)
  end)

  it("NV condition never executes", function()
    local cpu = GL.new(1024)
    GL.write_register(cpu, 0, 42)
    local movnv = ARM1.encode_mov_imm(ARM1.COND_NV, 0, 99)
    GL.load_instructions(cpu, { movnv, ARM1.encode_halt() })
    GL.run(cpu, 100)
    assert.equals(42, GL.read_register(cpu, 0))
  end)

  it("gate_ops counter increments", function()
    local cpu = GL.new(1024)
    GL.load_instructions(cpu, {
      ARM1.encode_mov_imm(ARM1.COND_AL, 0, 1),
      ARM1.encode_mov_imm(ARM1.COND_AL, 1, 2),
      ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_ADD, 0, 2, 0, 1),
      ARM1.encode_halt(),
    })
    GL.run(cpu, 100)
    assert.is_true(cpu.gate_ops > 0)
  end)

  it("end-to-end: sum 1 to 10 = 55", function()
    local cpu = GL.new(4096)

    -- SUBS R1, R1, #1
    local sub_imm = (ARM1.COND_AL << 28) | (1 << 25) | (ARM1.OP_SUB << 21) |
                    (1 << 20) | (1 << 16) | (1 << 12) | 1
    sub_imm = sub_imm & 0xFFFFFFFF

    local bne = ARM1.encode_branch(ARM1.COND_NE, 0, -8)

    GL.load_instructions(cpu, {
      ARM1.encode_mov_imm(ARM1.COND_AL, 0, 0),
      ARM1.encode_mov_imm(ARM1.COND_AL, 1, 10),
      ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_ADD, 0, 0, 0, 1),
      sub_imm,
      bne,
      ARM1.encode_halt(),
    })
    GL.run(cpu, 1000)
    assert.equals(55, GL.read_register(cpu, 0))
  end)
end)

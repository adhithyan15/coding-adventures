--[[
  ARM1 Gate-Level Simulator — Lua Port
  =====================================

  Every arithmetic operation routes through actual logic gate function calls —
  AND, OR, XOR, NOT — chained into adders, then into a 32-bit ALU. The
  barrel shifter is built from multiplexer trees. Registers are stored as
  bit arrays (arrays of 0s and 1s, LSB first).

  This is NOT the same as the behavioral simulator. Both produce identical
  results for any program. The difference is the execution path:

    Behavioral:  opcode → if/case → host arithmetic → result
    Gate-level:  opcode → decoder → barrel-shifter muxes → ALU gates
                         → adder gates → logic gates → result

  ## Architecture — Data Flow

      ┌──────────────────────────────────────────────────────┐
      │              ARM1 (Gate-Level)                       │
      │                                                      │
      │  Program Counter (26-bit)                            │
      │       │                                              │
      │       ▼                                              │
      │  Instruction Decoder (gate trees / pattern match)    │
      │       │                                              │
      │       ▼  control signals                             │
      │  ┌────────────┐  ┌──────────────────┐  ┌─────────┐  │
      │  │ Condition  │  │  Barrel Shifter  │  │  32-bit │  │
      │  │ Evaluator  │  │  (mux tree,      │  │  ALU    │  │
      │  │ (4 gates)  │  │   5 levels)      │  │ (gates) │  │
      │  └────────────┘  └──────────────────┘  └─────────┘  │
      │                                                      │
      │  Register File (27 × 32 bits — bit arrays)           │
      └──────────────────────────────────────────────────────┘

  ## Key Difference From Behavioral

  In the behavioral simulator, ADD R0,R1,R2 does:
      result = reg[1] + reg[2]   -- host language addition

  Here it does:
      a_bits = int_to_bits(reg[1])
      b_bits = int_to_bits(reg[2])
      sum_bits, carry = ripple_carry_adder(a_bits, b_bits, 0)
      result = bits_to_int(sum_bits)

  The ripple_carry_adder itself calls full_adder 32 times, each full_adder
  calls AND, XOR gates — the same gate functions used in the logic_gates
  package. This is gate-level simulation: every operation leaves a trace of
  ~200 gate function calls.

  ## Dependencies

  - coding_adventures.logic_gates   — AND, OR, XOR, NOT, XNOR
  - coding_adventures.arithmetic    — ripple_carry_adder
  - coding_adventures.arm1_simulator — instruction encoding, decoding, constants

  ## Usage

      local GL = require("coding_adventures.arm1_gatelevel")
      local ARM1 = require("coding_adventures.arm1_simulator")

      local cpu = GL.new(4096)
      GL.load_instructions(cpu, {
          ARM1.encode_mov_imm(ARM1.COND_AL, 0, 42),
          ARM1.encode_halt(),
      })
      local traces = GL.run(cpu, 100)
      print(GL.read_register(cpu, 0))  -- 42
      print(cpu.gate_ops)              -- gate calls performed
]]

local lg     = require("coding_adventures.logic_gates")
local adder  = require("coding_adventures.arithmetic.adder")
local ARM1   = require("coding_adventures.arm1_simulator")

local GL = {}

-- ===========================================================================
-- Bit Conversion Helpers
-- ===========================================================================
--
-- These bridge between the integer world (test programs, external API)
-- and the gate-level world (arrays of 0s and 1s flowing through gates).
--
--   int_to_bits(5, 32) → {1, 0, 1, 0, 0, ..., 0}  (32 elements, LSB first)
--   bits_to_int({...}) → 5

--- Converts a uint32 to an array of bits (LSB first, 1-indexed).
function GL.int_to_bits(value, width)
  width = width or 32
  local bits = {}
  for i = 1, width do
    bits[i] = (value >> (i - 1)) & 1
  end
  return bits
end

--- Converts an array of bits (LSB first, 1-indexed) to a uint32.
function GL.bits_to_int(bits)
  local result = 0
  for i = 1, math.min(#bits, 32) do
    if bits[i] == 1 then
      result = result | (1 << (i - 1))
    end
  end
  return result
end

-- ===========================================================================
-- CPU State Construction
-- ===========================================================================

--- Creates a new gate-level ARM1 simulator.
-- Registers are stored as arrays of 32 bits (LSB first).
--
-- @param memory_size  bytes of RAM (default 1 MiB)
-- @return cpu object
function GL.new(memory_size)
  memory_size = memory_size or (1024 * 1024)
  if memory_size <= 0 then memory_size = 1024 * 1024 end

  -- 27 physical registers, each 32 bits (LSB first), all zero
  local regs = {}
  for i = 0, 26 do
    local bits = {}
    for j = 1, 32 do bits[j] = 0 end
    regs[i] = bits
  end

  -- Memory: table of bytes indexed 0..memory_size-1
  local memory = {}
  for i = 0, memory_size - 1 do memory[i] = 0 end

  local cpu = {
    regs        = regs,
    memory      = memory,
    memory_size = memory_size,
    halted      = false,
    gate_ops    = 0,
  }

  GL.reset(cpu)
  return cpu
end

--- Resets the CPU to power-on state.
function GL.reset(cpu)
  -- Zero all registers
  for i = 0, 26 do
    for j = 1, 32 do cpu.regs[i][j] = 0 end
  end

  -- R15: set I, F flags and SVC mode
  local r15_val = ARM1.FLAG_I | ARM1.FLAG_F | ARM1.MODE_SVC
  cpu.regs[15] = GL.int_to_bits(r15_val, 32)
  cpu.halted   = false
  cpu.gate_ops = 0
end

-- ===========================================================================
-- Register Access
-- ===========================================================================

-- Compute the physical register index (mode banking)
local function physical_reg(cpu, index)
  local r15_int = GL.bits_to_int(cpu.regs[15])
  local mode = r15_int & 0x3  -- mode bits M1:M0

  -- FIQ (mode 1) banks R8-R14 at physical 16-22
  if mode == 1 and index >= 8 and index <= 14 then
    return 16 + (index - 8)
  end
  -- IRQ (mode 2) banks R13-R14 at physical 23-24
  if mode == 2 and index >= 13 and index <= 14 then
    return 23 + (index - 13)
  end
  -- SVC (mode 3) banks R13-R14 at physical 25-26
  if mode == 3 and index >= 13 and index <= 14 then
    return 25 + (index - 13)
  end
  return index
end

--- Reads register by logical index, returns integer value.
function GL.read_register(cpu, index)
  return GL.bits_to_int(cpu.regs[physical_reg(cpu, index)])
end

--- Writes register by logical index.
function GL.write_register(cpu, index, value)
  local phys = physical_reg(cpu, index)
  cpu.regs[phys] = GL.int_to_bits(value & 0xFFFFFFFF, 32)
end

-- Read register bits (for ALU inputs), R15 returns PC+4 (pipeline)
local function read_reg_bits_for_exec(cpu, index)
  if index == 15 then
    local val = (GL.bits_to_int(cpu.regs[15]) + 4) & 0xFFFFFFFF
    return GL.int_to_bits(val, 32)
  end
  return cpu.regs[physical_reg(cpu, index)]
end

-- Read register integer value for address calculation (R15 = PC+4)
local function read_reg_for_exec(cpu, index)
  if index == 15 then
    return (GL.bits_to_int(cpu.regs[15]) + 4) & 0xFFFFFFFF
  end
  return GL.read_register(cpu, index)
end

--- Returns the current PC (bits 25:2 of R15).
function GL.get_pc(cpu)
  return GL.bits_to_int(cpu.regs[15]) & ARM1.PC_MASK
end

-- Set PC portion of R15 (preserves flags and mode bits)
local function set_pc(cpu, addr)
  local r15 = GL.bits_to_int(cpu.regs[15])
  r15 = (r15 & (~ARM1.PC_MASK & 0xFFFFFFFF)) | (addr & ARM1.PC_MASK)
  cpu.regs[15] = GL.int_to_bits(r15 & 0xFFFFFFFF, 32)
end

--- Returns the current condition flags as a table {n, z, c, v}.
function GL.get_flags(cpu)
  local r15_bits = cpu.regs[15]
  return {
    n = r15_bits[32] == 1,
    z = r15_bits[31] == 1,
    c = r15_bits[30] == 1,
    v = r15_bits[29] == 1,
  }
end

local function set_flags_bits(cpu, n, z, c, v)
  cpu.regs[15][32] = n
  cpu.regs[15][31] = z
  cpu.regs[15][30] = c
  cpu.regs[15][29] = v
end

--- Returns the current processor mode.
function GL.get_mode(cpu)
  return GL.bits_to_int(cpu.regs[15]) & 0x3
end

-- ===========================================================================
-- Memory
-- ===========================================================================

--- Reads a 32-bit little-endian word.
function GL.read_word(cpu, addr)
  addr = addr & ARM1.PC_MASK
  local a = addr & 0xFFFFFFFC  -- align to word boundary
  if a + 3 >= cpu.memory_size then return 0 end
  local b0 = cpu.memory[a]   or 0
  local b1 = cpu.memory[a+1] or 0
  local b2 = cpu.memory[a+2] or 0
  local b3 = cpu.memory[a+3] or 0
  return (b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)) & 0xFFFFFFFF
end

--- Writes a 32-bit little-endian word.
function GL.write_word(cpu, addr, value)
  addr = addr & ARM1.PC_MASK
  local a = addr & 0xFFFFFFFC
  if a + 3 >= cpu.memory_size then return end
  value = value & 0xFFFFFFFF
  cpu.memory[a]   = value        & 0xFF
  cpu.memory[a+1] = (value >> 8)  & 0xFF
  cpu.memory[a+2] = (value >> 16) & 0xFF
  cpu.memory[a+3] = (value >> 24) & 0xFF
end

--- Reads a byte.
function GL.read_byte(cpu, addr)
  addr = addr & 0x03FFFFFF  -- 26-bit address space, all byte positions valid
  if addr >= cpu.memory_size then return 0 end
  return cpu.memory[addr] or 0
end

--- Writes a byte.
function GL.write_byte(cpu, addr, value)
  addr = addr & 0x03FFFFFF  -- 26-bit address space, all byte positions valid
  if addr >= cpu.memory_size then return end
  cpu.memory[addr] = value & 0xFF
end

--- Loads an array of 32-bit instruction words into memory starting at address 0.
function GL.load_instructions(cpu, instructions)
  local addr = 0
  for _, word in ipairs(instructions) do
    GL.write_word(cpu, addr, word)
    addr = addr + 4
  end
end

-- ===========================================================================
-- Gate-Level Condition Evaluation
-- ===========================================================================
--
-- Each condition code is implemented using actual gate function calls.
-- This mirrors how the real ARM1 condition evaluator worked — a small
-- combinational gate tree feeding into the instruction execute enable.

local function evaluate_condition(cpu, condition)
  local flags = GL.get_flags(cpu)
  local n = flags.n and 1 or 0
  local z = flags.z and 1 or 0
  local c = flags.c and 1 or 0
  local v = flags.v and 1 or 0

  -- Each flag check costs ~1 gate op; comparisons cost 2-4
  cpu.gate_ops = cpu.gate_ops + 4

  local cond = condition
  if     cond == 0x0 then return z == 1                                    -- EQ: Z=1
  elseif cond == 0x1 then return lg.NOT(z) == 1                            -- NE: Z=0
  elseif cond == 0x2 then return c == 1                                    -- CS: C=1
  elseif cond == 0x3 then return lg.NOT(c) == 1                            -- CC: C=0
  elseif cond == 0x4 then return n == 1                                    -- MI: N=1
  elseif cond == 0x5 then return lg.NOT(n) == 1                            -- PL: N=0
  elseif cond == 0x6 then return v == 1                                    -- VS: V=1
  elseif cond == 0x7 then return lg.NOT(v) == 1                            -- VC: V=0
  elseif cond == 0x8 then return lg.AND(c, lg.NOT(z)) == 1                 -- HI: C=1 AND Z=0
  elseif cond == 0x9 then return lg.OR(lg.NOT(c), z) == 1                  -- LS: C=0 OR Z=1
  elseif cond == 0xA then return lg.XNOR(n, v) == 1                        -- GE: N=V
  elseif cond == 0xB then return lg.XOR(n, v) == 1                         -- LT: N≠V
  elseif cond == 0xC then                                                  -- GT: Z=0 AND N=V
    return lg.AND(lg.NOT(z), lg.XNOR(n, v)) == 1
  elseif cond == 0xD then                                                  -- LE: Z=1 OR N≠V
    return lg.OR(z, lg.XOR(n, v)) == 1
  elseif cond == 0xE then return true                                      -- AL: always
  else   return false                                                      -- NV: never
  end
end

-- ===========================================================================
-- Gate-Level ALU
-- ===========================================================================
--
-- Every operation routes through gate function calls:
--   Arithmetic: ripple_carry_adder (32 full adders → ~160 gate calls each)
--   Logical:    AND/OR/XOR/NOT applied to each of 32 bits (32 gate calls each)
--
-- Total gate calls per instruction: ~200 (logical) to ~300 (arithmetic)

-- Apply a 2-input gate function to each bit pair (32 parallel gates)
local function bitwise_gate(a_bits, b_bits, gate_fn)
  local result = {}
  for i = 1, 32 do
    result[i] = gate_fn(a_bits[i], b_bits[i])
  end
  return result
end

-- Apply NOT to each bit (32 NOT gates in parallel)
local function bitwise_not(bits)
  local result = {}
  for i = 1, 32 do result[i] = lg.NOT(bits[i]) end
  return result
end

-- Zero flag: NOR tree — OR all bits then NOT (1 when all bits are 0)
local function compute_zero(bits)
  local combined = 0
  for i = 1, 32 do
    combined = lg.OR(combined, bits[i])
  end
  return lg.NOT(combined)
end

-- Overflow: (a[msb] XOR result[msb]) AND (b[msb] XOR result[msb])
-- The MSB is bit 32 in our LSB-first arrays.
local function compute_overflow(a_bits, b_bits, result_bits)
  local xor1 = lg.XOR(a_bits[32], result_bits[32])
  local xor2 = lg.XOR(b_bits[32], result_bits[32])
  return lg.AND(xor1, xor2)
end

--- Executes one of the 16 ARM1 ALU opcodes using gate-level logic.
-- Returns a table: { result_bits, n, z, c, v }
function GL.gate_alu_execute(opcode, a_bits, b_bits, carry_in, shifter_carry, old_v)
  local result_bits, carry, overflow
  local op = opcode

  if op == 0x0 or op == 0x8 then  -- AND, TST
    result_bits = bitwise_gate(a_bits, b_bits, lg.AND)
    carry, overflow = shifter_carry, old_v

  elseif op == 0x1 or op == 0x9 then  -- EOR, TEQ
    result_bits = bitwise_gate(a_bits, b_bits, lg.XOR)
    carry, overflow = shifter_carry, old_v

  elseif op == 0xC then  -- ORR
    result_bits = bitwise_gate(a_bits, b_bits, lg.OR)
    carry, overflow = shifter_carry, old_v

  elseif op == 0xD then  -- MOV
    result_bits = {}
    for i = 1, 32 do result_bits[i] = b_bits[i] end
    carry, overflow = shifter_carry, old_v

  elseif op == 0xE then  -- BIC = AND(a, NOT(b))
    local not_b = bitwise_not(b_bits)
    result_bits = bitwise_gate(a_bits, not_b, lg.AND)
    carry, overflow = shifter_carry, old_v

  elseif op == 0xF then  -- MVN = NOT(b)
    result_bits = bitwise_not(b_bits)
    carry, overflow = shifter_carry, old_v

  elseif op == 0x4 or op == 0xB then  -- ADD, CMN
    result_bits, carry = adder.ripple_carry_adder(a_bits, b_bits, 0)
    overflow = compute_overflow(a_bits, b_bits, result_bits)

  elseif op == 0x5 then  -- ADC
    result_bits, carry = adder.ripple_carry_adder(a_bits, b_bits, carry_in)
    overflow = compute_overflow(a_bits, b_bits, result_bits)

  elseif op == 0x2 or op == 0xA then  -- SUB, CMP: A + NOT(B) + 1
    local not_b = bitwise_not(b_bits)
    result_bits, carry = adder.ripple_carry_adder(a_bits, not_b, 1)
    overflow = compute_overflow(a_bits, not_b, result_bits)

  elseif op == 0x6 then  -- SBC: A + NOT(B) + C
    local not_b = bitwise_not(b_bits)
    result_bits, carry = adder.ripple_carry_adder(a_bits, not_b, carry_in)
    overflow = compute_overflow(a_bits, not_b, result_bits)

  elseif op == 0x3 then  -- RSB: B + NOT(A) + 1
    local not_a = bitwise_not(a_bits)
    result_bits, carry = adder.ripple_carry_adder(b_bits, not_a, 1)
    overflow = compute_overflow(b_bits, not_a, result_bits)

  elseif op == 0x7 then  -- RSC: B + NOT(A) + C
    local not_a = bitwise_not(a_bits)
    result_bits, carry = adder.ripple_carry_adder(b_bits, not_a, carry_in)
    overflow = compute_overflow(b_bits, not_a, result_bits)

  else
    result_bits = {}
    for i = 1, 32 do result_bits[i] = 0 end
    carry, overflow = 0, 0
  end

  -- N flag = MSB (bit 32 in LSB-first arrays)
  local n = result_bits[32]
  -- Z flag = NOR of all bits
  local z = compute_zero(result_bits)

  return { result_bits = result_bits, n = n, z = z, c = carry, v = overflow }
end

-- ===========================================================================
-- Gate-Level Barrel Shifter
-- ===========================================================================
--
-- On the real ARM1, the barrel shifter was a 32×32 crossbar network of
-- pass transistors. We model it with a 5-level tree of Mux2 gates.
--
--   Level 0: select shift by 1  (16 Mux2 gates)
--   Level 1: select shift by 2  (16 Mux2 gates)
--   Level 2: select shift by 4  (16 Mux2 gates)
--   Level 3: select shift by 8  (16 Mux2 gates)
--   Level 4: select shift by 16 (16 Mux2 gates)
--   Total:   5 × 32 = 160 Mux2 gates per shift operation
--
-- A Mux2(a, b, sel) = NOT(sel)*a + sel*b
-- (implemented using ~3 gate calls in the logic_gates package)

-- Simple 2-to-1 multiplexer using gate primitives
-- mux2(a, b, sel): if sel=0 return a, if sel=1 return b
local function mux2(a, b, sel)
  local not_sel = lg.NOT(sel)
  local t1 = lg.AND(not_sel, a)
  local t2 = lg.AND(sel, b)
  return lg.OR(t1, t2)
end

--- Performs a barrel shift using multiplexer trees.
-- Returns result_bits, carry_out (both as integers, not bit arrays).
function GL.gate_barrel_shift(value_bits, shift_type, amount, carry_in, by_register)
  -- ROR #0 by register = pass-through (RRX encoded as immediate)
  if by_register and amount == 0 then
    local out = {}
    for i = 1, 32 do out[i] = value_bits[i] end
    return out, carry_in
  end

  if shift_type == ARM1.SHIFT_LSL then
    return GL._gate_lsl(value_bits, amount, carry_in, by_register)
  elseif shift_type == ARM1.SHIFT_LSR then
    return GL._gate_lsr(value_bits, amount, carry_in, by_register)
  elseif shift_type == ARM1.SHIFT_ASR then
    return GL._gate_asr(value_bits, amount, carry_in, by_register)
  elseif shift_type == ARM1.SHIFT_ROR then
    return GL._gate_ror(value_bits, amount, carry_in, by_register)
  else
    local out = {}
    for i = 1, 32 do out[i] = value_bits[i] end
    return out, carry_in
  end
end

-- LSL (Logical Shift Left) via 5-level mux tree
function GL._gate_lsl(value_bits, amount, carry_in, by_reg)
  if amount == 0 then
    local out = {}
    for i = 1, 32 do out[i] = value_bits[i] end
    return out, carry_in
  end
  if amount >= 32 then
    local zeros = {}
    for i = 1, 32 do zeros[i] = 0 end
    if amount == 32 then
      return zeros, value_bits[1]  -- carry = bit 0 (index 1 in LSB-first)
    else
      return zeros, 0
    end
  end

  -- 5-level mux tree: each level selects shift by 2^level
  local current = {}
  for i = 1, 32 do current[i] = value_bits[i] end

  for level = 0, 4 do
    local shift = 1 << level  -- bits to shift at this level
    local sel = (amount >> level) & 1
    local next = {}
    for i = 1, 32 do
      -- LSL: bit i gets bit (i - shift) if shifting, else keeps current
      -- LSB-first: bit position i corresponds to 2^(i-1)
      -- After shifting left by `shift`, bit i = bit (i-shift) of input
      local src = i - shift
      local shifted_bit = (src >= 1) and current[src] or 0
      next[i] = mux2(current[i], shifted_bit, sel)
    end
    current = next
  end

  -- Carry out = the last bit shifted out of the left (the bit at position 32-amount+1)
  local carry_bit_pos = 32 - amount + 1  -- 1-indexed LSB-first
  local carry_out = (carry_bit_pos >= 1 and carry_bit_pos <= 32)
      and value_bits[carry_bit_pos] or 0

  return current, carry_out
end

-- LSR (Logical Shift Right) via 5-level mux tree
function GL._gate_lsr(value_bits, amount, carry_in, by_reg)
  if amount == 0 and not by_reg then
    -- Immediate LSR #0 encodes LSR #32
    local zeros = {}
    for i = 1, 32 do zeros[i] = 0 end
    return zeros, value_bits[32]  -- carry = MSB
  end
  if amount == 0 then
    local out = {}
    for i = 1, 32 do out[i] = value_bits[i] end
    return out, carry_in
  end
  if amount >= 32 then
    local zeros = {}
    for i = 1, 32 do zeros[i] = 0 end
    if amount == 32 then
      return zeros, value_bits[32]  -- carry = MSB
    else
      return zeros, 0
    end
  end

  local current = {}
  for i = 1, 32 do current[i] = value_bits[i] end

  for level = 0, 4 do
    local shift = 1 << level
    local sel = (amount >> level) & 1
    local next = {}
    for i = 1, 32 do
      -- LSR: bit i gets bit (i + shift) if shifting
      local src = i + shift
      local shifted_bit = (src <= 32) and current[src] or 0
      next[i] = mux2(current[i], shifted_bit, sel)
    end
    current = next
  end

  -- Carry = last bit shifted out (bit position `amount` from LSB, 1-indexed)
  local carry_out = (amount >= 1 and amount <= 32) and value_bits[amount] or 0

  return current, carry_out
end

-- ASR (Arithmetic Shift Right) — sign-extending via mux tree
function GL._gate_asr(value_bits, amount, carry_in, by_reg)
  local sign_bit = value_bits[32]  -- MSB

  if amount == 0 and not by_reg then
    -- Immediate ASR #0 encodes ASR #32
    local result = {}
    for i = 1, 32 do result[i] = sign_bit end
    return result, sign_bit
  end
  if amount == 0 then
    local out = {}
    for i = 1, 32 do out[i] = value_bits[i] end
    return out, carry_in
  end
  if amount >= 32 then
    local result = {}
    for i = 1, 32 do result[i] = sign_bit end
    return result, sign_bit
  end

  local current = {}
  for i = 1, 32 do current[i] = value_bits[i] end

  for level = 0, 4 do
    local shift = 1 << level
    local sel = (amount >> level) & 1
    local next = {}
    for i = 1, 32 do
      local src = i + shift
      -- Sign-extend: fill with sign_bit
      local shifted_bit = (src <= 32) and current[src] or sign_bit
      next[i] = mux2(current[i], shifted_bit, sel)
    end
    current = next
  end

  local carry_out = (amount >= 1 and amount <= 32) and value_bits[amount] or 0
  return current, carry_out
end

-- ROR (Rotate Right) and RRX (Rotate Right through eXtended carry)
function GL._gate_ror(value_bits, amount, carry_in, by_reg)
  if amount == 0 and not by_reg then
    -- RRX: 33-bit rotate through carry (amount=0 immediate = RRX)
    local result = {}
    for i = 1, 31 do result[i] = value_bits[i + 1] end
    result[32] = carry_in  -- carry inserted at MSB
    local carry_out = value_bits[1]  -- LSB shifts into carry
    return result, carry_out
  end
  if amount == 0 then
    local out = {}
    for i = 1, 32 do out[i] = value_bits[i] end
    return out, carry_in
  end

  -- Normalize to 0..31
  local eff_amount = amount & 31
  if eff_amount == 0 then
    -- Full rotation: result is same, carry = MSB
    local out = {}
    for i = 1, 32 do out[i] = value_bits[i] end
    return out, value_bits[32]
  end

  local current = {}
  for i = 1, 32 do current[i] = value_bits[i] end

  for level = 0, 4 do
    local shift = 1 << level
    local sel = (eff_amount >> level) & 1
    local next = {}
    for i = 1, 32 do
      -- ROR: bit i gets bit ((i + shift - 1) % 32) + 1 of current
      local src = ((i - 1 + shift) % 32) + 1
      local shifted_bit = current[src]
      next[i] = mux2(current[i], shifted_bit, sel)
    end
    current = next
  end

  -- Carry = MSB of result (last bit rotated out)
  return current, current[32]
end

--- Decodes a rotated immediate using gate-level rotation.
-- Returns result_bits (array of 32 bits), carry_out
function GL.gate_decode_immediate(imm8, rotate)
  local bits = GL.int_to_bits(imm8, 32)
  local rotate_amount = rotate * 2
  if rotate_amount == 0 then
    return bits, 0
  else
    return GL._gate_ror(bits, rotate_amount, 0, false)
  end
end

-- ===========================================================================
-- Execution
-- ===========================================================================

--- Executes one instruction and returns a trace table.
function GL.step(cpu)
  if cpu.halted then return nil end

  local current_pc = GL.get_pc(cpu)

  local instruction = GL.read_word(cpu, current_pc)
  local decoded = ARM1.decode(instruction)

  -- Evaluate condition using gate-level logic
  local cond_met = evaluate_condition(cpu, decoded.condition)

  -- Record state before execution
  local regs_before = {}
  for i = 0, 15 do regs_before[i] = GL.read_register(cpu, i) end

  local flags_before = GL.get_flags(cpu)

  -- Advance PC by 4 (fetch next instruction)
  set_pc(cpu, (current_pc + 4) & ARM1.PC_MASK)

  -- Execute if condition met
  local mem_reads = {}
  local mem_writes = {}

  if cond_met then
    local dtype = decoded.type
    if dtype == 0 then
      GL._execute_data_processing(cpu, decoded, mem_reads, mem_writes)
    elseif dtype == 1 then
      GL._execute_load_store(cpu, decoded, mem_reads, mem_writes)
    elseif dtype == 2 then
      GL._execute_block_transfer(cpu, decoded, mem_reads, mem_writes)
    elseif dtype == 3 then
      GL._execute_branch(cpu, decoded)
    elseif dtype == 4 then
      GL._execute_swi(cpu, decoded)
    else
      GL._trap_undefined(cpu)
    end
  end

  local regs_after = {}
  for i = 0, 15 do regs_after[i] = GL.read_register(cpu, i) end

  return {
    address       = current_pc,
    raw           = instruction,
    condition_met = cond_met,
    regs_before   = regs_before,
    regs_after    = regs_after,
    flags_before  = flags_before,
    flags_after   = GL.get_flags(cpu),
    memory_reads  = mem_reads,
    memory_writes = mem_writes,
  }
end

--- Runs up to max_steps instructions, stops on halt.
-- Returns array of traces.
function GL.run(cpu, max_steps)
  local traces = {}
  for _ = 1, max_steps do
    if cpu.halted then break end
    local trace = GL.step(cpu)
    if trace then
      traces[#traces + 1] = trace
    end
  end
  return traces
end

-- ===========================================================================
-- Data Processing (gate-level)
-- ===========================================================================

function GL._execute_data_processing(cpu, d, _reads, _writes)
  -- Read Rn as bits (MOV/MVN ignore Rn)
  local a_bits
  local test_ops = { [0x8]=true, [0x9]=true, [0xA]=true, [0xB]=true }
  local mov_ops  = { [0xD]=true, [0xF]=true }

  if mov_ops[d.opcode] then
    a_bits = {}
    for i = 1, 32 do a_bits[i] = 0 end
  else
    a_bits = read_reg_bits_for_exec(cpu, d.rn)
  end

  local flags = GL.get_flags(cpu)
  local flag_c = flags.c and 1 or 0
  local flag_v = flags.v and 1 or 0

  -- Get Operand2 through gate-level barrel shifter
  local b_bits, shifter_carry
  if d.immediate then
    b_bits, shifter_carry = GL.gate_decode_immediate(d.imm8, d.rotate)
    if d.rotate == 0 then shifter_carry = flag_c end
  else
    local rm_bits = read_reg_bits_for_exec(cpu, d.rm)
    local shift_amount
    if d.shift_by_reg then
      shift_amount = GL.read_register(cpu, d.rs) & 0xFF
    else
      shift_amount = d.shift_imm
    end
    b_bits, shifter_carry = GL.gate_barrel_shift(rm_bits, d.shift_type, shift_amount, flag_c, d.shift_by_reg)
  end

  -- Execute gate-level ALU
  local alu_result = GL.gate_alu_execute(d.opcode, a_bits, b_bits, flag_c, shifter_carry, flag_v)
  cpu.gate_ops = cpu.gate_ops + 200

  local result_val = GL.bits_to_int(alu_result.result_bits)

  -- Write result (not for test-only ops: TST/TEQ/CMP/CMN)
  if not test_ops[d.opcode] then
    if d.rd == 15 then
      if d.s then
        cpu.regs[15] = GL.int_to_bits(result_val, 32)
      else
        set_pc(cpu, result_val & ARM1.PC_MASK)
      end
    else
      GL.write_register(cpu, d.rd, result_val)
    end
  end

  -- Update flags (S-bit set, or test ops always update)
  local update_flags = (d.s and d.rd ~= 15) or test_ops[d.opcode]
  if update_flags then
    set_flags_bits(cpu, alu_result.n, alu_result.z, alu_result.c, alu_result.v)
  end
end

-- ===========================================================================
-- Load/Store
-- ===========================================================================

function GL._execute_load_store(cpu, d, mem_reads, mem_writes)
  local offset
  if d.immediate then  -- register offset (bit 25 = 1 in load/store = register)
    local rm_val = read_reg_for_exec(cpu, d.rm)
    if d.shift_imm ~= 0 then
      local rm_bits = GL.int_to_bits(rm_val, 32)
      local flags = GL.get_flags(cpu)
      local flag_c = flags.c and 1 or 0
      local shifted, _ = GL.gate_barrel_shift(rm_bits, d.shift_type, d.shift_imm, flag_c, false)
      offset = GL.bits_to_int(shifted)
    else
      offset = rm_val
    end
  else
    offset = d.offset12
  end

  local base = read_reg_for_exec(cpu, d.rn)
  local addr
  if d.up then
    addr = (base + offset) & 0xFFFFFFFF
  else
    addr = (base - offset) & 0xFFFFFFFF
  end
  local transfer_addr = d.pre_index and addr or base

  if d.load then
    local value
    if d.byte then
      value = GL.read_byte(cpu, transfer_addr)
    else
      local word = GL.read_word(cpu, transfer_addr)
      local rotation = (transfer_addr & 3) * 8
      if rotation ~= 0 then
        value = ((word >> rotation) | (word << (32 - rotation))) & 0xFFFFFFFF
      else
        value = word
      end
    end
    mem_reads[#mem_reads + 1] = { address = transfer_addr, value = value }
    if d.rd == 15 then
      cpu.regs[15] = GL.int_to_bits(value & 0xFFFFFFFF, 32)
    else
      GL.write_register(cpu, d.rd, value)
    end
  else
    local value = read_reg_for_exec(cpu, d.rd)
    if d.byte then
      GL.write_byte(cpu, transfer_addr, value & 0xFF)
    else
      GL.write_word(cpu, transfer_addr, value)
    end
    mem_writes[#mem_writes + 1] = { address = transfer_addr, value = value }
  end

  -- Writeback
  if d.write_back or not d.pre_index then
    if d.rn ~= 15 then
      GL.write_register(cpu, d.rn, addr)
    end
  end
end

-- ===========================================================================
-- Block Transfer
-- ===========================================================================

function GL._execute_block_transfer(cpu, d, mem_reads, mem_writes)
  local base = GL.read_register(cpu, d.rn)
  local reg_list = d.register_list

  -- Count active registers
  local count = 0
  for i = 0, 15 do
    if ((reg_list >> i) & 1) == 1 then count = count + 1 end
  end

  if count == 0 then return end

  -- Compute start address based on mode (IA/IB/DA/DB)
  local start_addr
  local pre  = d.pre_index
  local up   = d.up
  if     not pre and     up then start_addr = base
  elseif     pre and     up then start_addr = (base + 4) & 0xFFFFFFFF
  elseif not pre and not up then start_addr = (base - count * 4 + 4) & 0xFFFFFFFF
  else                           start_addr = (base - count * 4) & 0xFFFFFFFF
  end

  local current_addr = start_addr
  for i = 0, 15 do
    if ((reg_list >> i) & 1) == 1 then
      if d.load then
        local value = GL.read_word(cpu, current_addr)
        mem_reads[#mem_reads + 1] = { address = current_addr, value = value }
        if i == 15 then
          cpu.regs[15] = GL.int_to_bits(value & 0xFFFFFFFF, 32)
        else
          GL.write_register(cpu, i, value)
        end
      else
        local value
        if i == 15 then
          value = (GL.bits_to_int(cpu.regs[15]) + 4) & 0xFFFFFFFF
        else
          value = GL.read_register(cpu, i)
        end
        GL.write_word(cpu, current_addr, value)
        mem_writes[#mem_writes + 1] = { address = current_addr, value = value }
      end
      current_addr = (current_addr + 4) & 0xFFFFFFFF
    end
  end

  -- Writeback base register
  if d.write_back then
    local new_base
    if up then
      new_base = (base + count * 4) & 0xFFFFFFFF
    else
      new_base = (base - count * 4) & 0xFFFFFFFF
    end
    GL.write_register(cpu, d.rn, new_base)
  end
end

-- ===========================================================================
-- Branch
-- ===========================================================================

function GL._execute_branch(cpu, d)
  -- At this point PC has been advanced by 4, so "PC" = instruction_addr + 4
  -- Pipeline means branch target = (instruction_addr + 8) + offset
  -- = (current_pc + 4) + offset
  local branch_base = (GL.get_pc(cpu) + 4) & 0xFFFFFFFF

  -- BL: save return address in R14
  if d.link then
    GL.write_register(cpu, 14, GL.bits_to_int(cpu.regs[15]))
  end

  local target = (branch_base + d.branch_offset) & 0xFFFFFFFF
  set_pc(cpu, target & ARM1.PC_MASK)
end

-- ===========================================================================
-- SWI / Halt
-- ===========================================================================

function GL._execute_swi(cpu, d)
  if d.swi_comment == ARM1.HALT_SWI then
    cpu.halted = true
    return
  end

  -- Mode switch to SVC: save R15 in SVC_R14 (physical reg 26)
  -- Also save to SVC_R13_ret (physical 25) — ARM1 behavior
  local r15_bits = {}
  for i = 1, 32 do r15_bits[i] = cpu.regs[15][i] end
  cpu.regs[25] = r15_bits
  cpu.regs[26] = r15_bits

  -- Switch to SVC mode, set I flag
  local r15_val = GL.bits_to_int(cpu.regs[15])
  r15_val = (r15_val & ~0x3 & 0xFFFFFFFF) | ARM1.MODE_SVC
  r15_val = r15_val | ARM1.FLAG_I
  cpu.regs[15] = GL.int_to_bits(r15_val & 0xFFFFFFFF, 32)
  set_pc(cpu, 0x08)
end

function GL._trap_undefined(cpu)
  local r15_bits = {}
  for i = 1, 32 do r15_bits[i] = cpu.regs[15][i] end
  cpu.regs[26] = r15_bits

  local r15_val = GL.bits_to_int(cpu.regs[15])
  r15_val = (r15_val & ~0x3 & 0xFFFFFFFF) | ARM1.MODE_SVC
  r15_val = r15_val | ARM1.FLAG_I
  cpu.regs[15] = GL.int_to_bits(r15_val & 0xFFFFFFFF, 32)
  set_pc(cpu, 0x04)
end

-- ===========================================================================
-- Delegate encoding helpers to arm1_simulator
-- ===========================================================================

GL.encode_mov_imm       = ARM1.encode_mov_imm
GL.encode_alu_reg       = ARM1.encode_alu_reg
GL.encode_alu_reg_shift = ARM1.encode_alu_reg_shift
GL.encode_branch        = ARM1.encode_branch
GL.encode_ldr           = ARM1.encode_ldr
GL.encode_str           = ARM1.encode_str
GL.encode_ldm           = ARM1.encode_ldm
GL.encode_stm           = ARM1.encode_stm
GL.encode_halt          = ARM1.encode_halt

-- Constants from arm1_simulator
GL.COND_AL = ARM1.COND_AL
GL.COND_EQ = ARM1.COND_EQ
GL.COND_NE = ARM1.COND_NE
GL.COND_MI = ARM1.COND_MI
GL.COND_NV = ARM1.COND_NV
GL.OP_ADD  = ARM1.OP_ADD
GL.OP_SUB  = ARM1.OP_SUB
GL.OP_AND  = ARM1.OP_AND
GL.OP_ORR  = ARM1.OP_ORR
GL.OP_EOR  = ARM1.OP_EOR
GL.OP_MVN  = ARM1.OP_MVN
GL.OP_BIC  = ARM1.OP_BIC
GL.OP_MOV  = ARM1.OP_MOV
GL.SHIFT_LSL = ARM1.SHIFT_LSL
GL.MODE_SVC  = ARM1.MODE_SVC

return GL

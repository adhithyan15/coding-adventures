-- ==========================================================================
-- ARM1 Behavioral Simulator — Lua Port
-- ==========================================================================
--
-- The ARM1 was designed by Sophie Wilson and Steve Furber at Acorn Computers
-- in Cambridge, UK. It first powered on April 26, 1985 — and worked correctly
-- on its very first attempt. This module is a behavioral simulator of the
-- complete ARMv1 instruction set.
--
-- # Why the ARM1?
--
-- The ARM1 had just 25,000 transistors — one order of magnitude fewer than
-- the Intel 386 (275,000) released the same year. Its accidental low power
-- consumption (~0.1W vs 2W for the 386) later made the ARM architecture
-- dominant in mobile computing. Today, ARM ships over 20 billion chips per
-- year. Every iPhone, iPad, and Android phone uses an ARM descendant.
--
-- # ARMv1 Key Features
--
--   * 32-bit RISC processor with fixed 32-bit instructions
--   * 16 visible registers (R0-R15), 25 physical (banked modes)
--   * R15 = PC + flags + mode — combined into one register!
--   * Conditional execution on EVERY instruction (not just branches)
--   * Barrel shifter on every data processing instruction (shifts are free)
--   * 26-bit address space (64 MiB)
--   * 3-stage pipeline: Fetch → Decode → Execute
--   * NO multiply instruction (added in ARM2/ARMv2)
--
-- # Register Map
--
--   R0-R12   General purpose
--   R13      Stack Pointer (by convention)
--   R14      Link Register — BL stores return address here
--   R15      PC + Status Register (unique to ARMv1):
--
--     Bit 31: N (Negative)    Bit 27: I (IRQ disable)
--     Bit 30: Z (Zero)        Bit 26: F (FIQ disable)
--     Bit 29: C (Carry)       Bits 25:2: Program Counter (24 bits)
--     Bit 28: V (Overflow)    Bits 1:0: Processor Mode
--
-- # Processor Modes
--
--   M1:M0  Mode   Banked Registers
--   -----  ----   ----------------
--   00     USR    (base set — none banked)
--   01     FIQ    R8_fiq..R14_fiq (7 banked — fast interrupt)
--   10     IRQ    R13_irq, R14_irq (2 banked)
--   11     SVC    R13_svc, R14_svc (2 banked)
--
-- # Lua 32-bit Integer Notes
--
-- Lua 5.4 integers are 64-bit signed. We mask to 32 bits with & 0xFFFFFFFF
-- to simulate unsigned 32-bit registers. The >> operator is logical (unsigned)
-- shift right, and << is logical shift left.
--
-- ==========================================================================

local ARM1Simulator = {}
ARM1Simulator.__index = ARM1Simulator

-- =========================================================================
-- Constants — Processor Modes
-- =========================================================================

ARM1Simulator.MODE_USR = 0  -- User mode (normal operation)
ARM1Simulator.MODE_FIQ = 1  -- Fast Interrupt (banks R8-R14)
ARM1Simulator.MODE_IRQ = 2  -- Normal Interrupt (banks R13, R14)
ARM1Simulator.MODE_SVC = 3  -- Supervisor (OS mode, banks R13, R14)

-- =========================================================================
-- Constants — Status Register Flags (exported for gate-level simulator)
-- =========================================================================
--
-- These flags live in R15 (which doubles as PC + status register in ARMv1).
-- They are exported so that the gate-level simulator can reference them
-- without duplicating magic constants.

ARM1Simulator.FLAG_I = 0x08000000  -- bit 27: IRQ disable
ARM1Simulator.FLAG_F = 0x04000000  -- bit 26: FIQ disable

-- Mode name strings
local MODE_NAMES = { [0]="USR", [1]="FIQ", [2]="IRQ", [3]="SVC" }

-- =========================================================================
-- Constants — Condition Codes
-- =========================================================================
--
-- Every ARM instruction has a 4-bit condition code in bits 31:28. This is
-- ARM's signature feature — any instruction can be made conditional.
--
--   Code  Suffix  Test            Flags Tested
--   ----  ------  ----            ------------
--   0000  EQ      Equal           Z=1
--   0001  NE      Not equal       Z=0
--   0010  CS/HS   Carry set       C=1
--   0011  CC/LO   Carry clear     C=0
--   0100  MI      Minus/negative  N=1
--   0101  PL      Plus/positive   N=0
--   0110  VS      Overflow        V=1
--   0111  VC      No overflow     V=0
--   1000  HI      Unsigned higher C=1 AND Z=0
--   1001  LS      Unsigned ≤      C=0 OR Z=1
--   1010  GE      Signed ≥        N=V
--   1011  LT      Signed <        N≠V
--   1100  GT      Signed >        Z=0 AND N=V
--   1101  LE      Signed ≤        Z=1 OR N≠V
--   1110  AL      Always          (unconditional)
--   1111  NV      Never           (reserved — never executes)

local COND_EQ = 0x0
local COND_NE = 0x1
local COND_CS = 0x2
local COND_CC = 0x3
local COND_MI = 0x4
local COND_PL = 0x5
local COND_VS = 0x6
local COND_VC = 0x7
local COND_HI = 0x8
local COND_LS = 0x9
local COND_GE = 0xA
local COND_LT = 0xB
local COND_GT = 0xC
local COND_LE = 0xD
local COND_AL = 0xE
local COND_NV = 0xF

local COND_NAMES = {
  [0]="EQ", [1]="NE", [2]="CS", [3]="CC",
  [4]="MI", [5]="PL", [6]="VS", [7]="VC",
  [8]="HI", [9]="LS", [10]="GE", [11]="LT",
  [12]="GT", [13]="LE", [14]="", [15]="NV"
}

-- =========================================================================
-- Constants — ALU Opcodes
-- =========================================================================
--
-- 16 ALU operations selected by bits 24:21 of data processing instructions.
--
--   Opcode  Mnemonic  Operation
--   ------  --------  ---------
--   0000    AND       Rd = Rn AND Op2
--   0001    EOR       Rd = Rn XOR Op2
--   0010    SUB       Rd = Rn - Op2
--   0011    RSB       Rd = Op2 - Rn  (Reverse Subtract)
--   0100    ADD       Rd = Rn + Op2
--   0101    ADC       Rd = Rn + Op2 + C
--   0110    SBC       Rd = Rn - Op2 - NOT(C)
--   0111    RSC       Rd = Op2 - Rn - NOT(C)
--   1000    TST       flags = Rn AND Op2  (test only, no Rd write)
--   1001    TEQ       flags = Rn XOR Op2  (test only)
--   1010    CMP       flags = Rn - Op2    (test only)
--   1011    CMN       flags = Rn + Op2    (test only)
--   1100    ORR       Rd = Rn OR Op2
--   1101    MOV       Rd = Op2            (Rn ignored)
--   1110    BIC       Rd = Rn AND NOT(Op2)  (Bit Clear)
--   1111    MVN       Rd = NOT(Op2)         (Move Negated)

local OP_AND = 0x0
local OP_EOR = 0x1
local OP_SUB = 0x2
local OP_RSB = 0x3
local OP_ADD = 0x4
local OP_ADC = 0x5
local OP_SBC = 0x6
local OP_RSC = 0x7
local OP_TST = 0x8
local OP_TEQ = 0x9
local OP_CMP = 0xA
local OP_CMN = 0xB
local OP_ORR = 0xC
local OP_MOV = 0xD
local OP_BIC = 0xE
local OP_MVN = 0xF

local OP_NAMES = {
  [0]="AND", [1]="EOR", [2]="SUB", [3]="RSB",
  [4]="ADD", [5]="ADC", [6]="SBC", [7]="RSC",
  [8]="TST", [9]="TEQ", [10]="CMP", [11]="CMN",
  [12]="ORR", [13]="MOV", [14]="BIC", [15]="MVN"
}

-- =========================================================================
-- Constants — Shift Types (for the barrel shifter)
-- =========================================================================
--
-- The barrel shifter is ARM's most distinctive feature. Every data
-- processing instruction can shift its second operand for free.
--
--   Type  Mnemonic  Operation
--   ----  --------  ---------
--   00    LSL       Logical Shift Left   (fill vacated bits with 0)
--   01    LSR       Logical Shift Right  (fill vacated bits with 0)
--   10    ASR       Arithmetic Shift Right (fill with sign bit)
--   11    ROR       Rotate Right         (circular rotation)
--         (RRX)     ROR with amount=0 = Rotate Right Extended through Carry

local SHIFT_LSL = 0
local SHIFT_LSR = 1
local SHIFT_ASR = 2
local SHIFT_ROR = 3

local SHIFT_NAMES = { [0]="LSL", [1]="LSR", [2]="ASR", [3]="ROR" }

-- =========================================================================
-- Constants — R15 Bit Fields
-- =========================================================================
--
-- In ARMv1, R15 combines the Program Counter and Processor Status Register.
-- This is a defining characteristic distinguishing ARMv1 from all later ARM
-- versions (ARMv2+ split them into separate CPSR and PC registers).
--
--  31  30  29  28  27  26  25                    2   1   0
-- ┌───┬───┬───┬───┬───┬───┬──────────────────────┬───┬───┐
-- │ N │ Z │ C │ V │ I │ F │   24-bit PC           │M1 │M0 │
-- └───┴───┴───┴───┴───┴───┴──────────────────────┴───┴───┘

local FLAG_N   = 0x80000000  -- bit 31: Negative
local FLAG_Z   = 0x40000000  -- bit 30: Zero
local FLAG_C   = 0x20000000  -- bit 29: Carry
local FLAG_V   = 0x10000000  -- bit 28: Overflow
local FLAG_I   = 0x08000000  -- bit 27: IRQ disable
local FLAG_F   = 0x04000000  -- bit 26: FIQ disable
local PC_MASK  = 0x03FFFFFC  -- bits 25:2: Program Counter
local MODE_MASK = 0x3         -- bits 1:0: Processor Mode
local MASK32   = 0xFFFFFFFF  -- 32-bit mask
local HALT_SWI = 0x123456    -- our pseudo-halt SWI number

-- =========================================================================
-- Constants — Instruction Types
-- =========================================================================

local INST_DATA_PROCESSING = 0
local INST_LOAD_STORE      = 1
local INST_BLOCK_TRANSFER  = 2
local INST_BRANCH          = 3
local INST_SWI             = 4
local INST_COPROCESSOR     = 5
local INST_UNDEFINED       = 6

-- =========================================================================
-- Helper: 32-bit arithmetic
-- =========================================================================

-- Mask a value to 32 unsigned bits
local function mask32(v)
  return v & MASK32
end

-- Bitwise NOT constrained to 32 bits
local function bnot32(v)
  return (~v) & MASK32
end

-- Arithmetic right shift — fill upper bits with sign bit
local function asr(v, amount)
  if amount >= 32 then
    if (v & 0x80000000) ~= 0 then return MASK32 else return 0 end
  end
  if amount == 0 then return v end
  local sign_bit = (v & 0x80000000) ~= 0
  local result = v >> amount
  if sign_bit then
    -- Fill upper 'amount' bits with 1s
    local fill = ((1 << amount) - 1) << (32 - amount)
    result = result | fill
  end
  return mask32(result)
end

-- Rotate right by 'amount' bits (circular, 32-bit)
local function ror32(v, amount)
  amount = amount & 31
  if amount == 0 then return mask32(v) end
  return mask32((v >> amount) | (v << (32 - amount)))
end

-- =========================================================================
-- Helper: test if opcode is test-only (TST, TEQ, CMP, CMN)
-- =========================================================================

local function is_test_op(opcode)
  return opcode >= OP_TST and opcode <= OP_CMN
end

local function is_logical_op(opcode)
  return opcode == OP_AND or opcode == OP_EOR or opcode == OP_TST
      or opcode == OP_TEQ or opcode == OP_ORR or opcode == OP_MOV
      or opcode == OP_BIC or opcode == OP_MVN
end

-- =========================================================================
-- Constructor
-- =========================================================================

--- Creates a new ARM1 simulator.
-- @param memory_size number of bytes of memory (default: 1 MiB)
-- @return ARM1Simulator object
function ARM1Simulator.new(memory_size)
  memory_size = memory_size or (1024 * 1024)
  if memory_size <= 0 then memory_size = 1024 * 1024 end

  local self = setmetatable({}, ARM1Simulator)

  -- 27 physical registers:
  --   0-15:  base registers R0-R15
  --   16-22: FIQ banked R8_fiq..R14_fiq  (indices 16+0..16+6)
  --   23-24: IRQ banked R13_irq, R14_irq
  --   25-26: SVC banked R13_svc, R14_svc
  self.regs = {}
  for i = 0, 26 do self.regs[i] = 0 end

  -- Memory as a Lua table of bytes (index 0..memory_size-1)
  self.memory = {}
  for i = 0, memory_size - 1 do self.memory[i] = 0 end
  self.memory_size = memory_size

  self.halted = false

  -- Reset to SVC mode, IRQ/FIQ disabled, PC=0
  self:reset()

  return self
end

-- =========================================================================
-- Reset
-- =========================================================================

--- Resets the CPU to power-on state: SVC mode, IRQ/FIQ disabled, PC=0.
function ARM1Simulator:reset()
  for i = 0, 26 do self.regs[i] = 0 end
  -- R15: SVC mode (bits 1:0 = 11), I and F disabled (bits 27,26 = 1,1)
  self.regs[15] = FLAG_I | FLAG_F | ARM1Simulator.MODE_SVC
  self.halted = false
end

-- =========================================================================
-- Register Access (with mode banking)
-- =========================================================================
--
-- Physical register mapping:
--   FIQ mode: logical R8-R14 → physical regs[16..22]
--   IRQ mode: logical R13-R14 → physical regs[23..24]
--   SVC mode: logical R13-R14 → physical regs[25..26]
--   All others: logical index = physical index

local function physical_reg_index(self, index)
  local mode = self.regs[15] & MODE_MASK
  if mode == ARM1Simulator.MODE_FIQ and index >= 8 and index <= 14 then
    return 16 + (index - 8)
  elseif mode == ARM1Simulator.MODE_IRQ and index >= 13 and index <= 14 then
    return 23 + (index - 13)
  elseif mode == ARM1Simulator.MODE_SVC and index >= 13 and index <= 14 then
    return 25 + (index - 13)
  else
    return index
  end
end

--- Reads register R0-R15, respecting mode banking.
function ARM1Simulator:read_register(index)
  local phys = physical_reg_index(self, index)
  return self.regs[phys]
end

--- Writes register R0-R15, respecting mode banking.
function ARM1Simulator:write_register(index, value)
  local phys = physical_reg_index(self, index)
  self.regs[phys] = mask32(value)
end

--- Returns the current program counter (26-bit address, word-aligned).
function ARM1Simulator:get_pc()
  return self.regs[15] & PC_MASK
end

--- Sets the PC portion of R15 without disturbing flags/mode.
function ARM1Simulator:set_pc(addr)
  local r15 = self.regs[15]
  self.regs[15] = mask32((r15 & bnot32(PC_MASK)) | (addr & PC_MASK))
end

--- Returns a table with current condition flags: {n, z, c, v}.
function ARM1Simulator:get_flags()
  local r15 = self.regs[15]
  return {
    n = (r15 & FLAG_N) ~= 0,
    z = (r15 & FLAG_Z) ~= 0,
    c = (r15 & FLAG_C) ~= 0,
    v = (r15 & FLAG_V) ~= 0
  }
end

--- Updates condition flags in R15 from a flags table {n, z, c, v}.
function ARM1Simulator:set_flags(f)
  local r15 = self.regs[15] & bnot32(FLAG_N | FLAG_Z | FLAG_C | FLAG_V)
  if f.n then r15 = r15 | FLAG_N end
  if f.z then r15 = r15 | FLAG_Z end
  if f.c then r15 = r15 | FLAG_C end
  if f.v then r15 = r15 | FLAG_V end
  self.regs[15] = mask32(r15)
end

--- Returns the current processor mode (0=USR, 1=FIQ, 2=IRQ, 3=SVC).
function ARM1Simulator:get_mode()
  return self.regs[15] & MODE_MASK
end

--- Returns the mode name string.
function ARM1Simulator:get_mode_name()
  return MODE_NAMES[self:get_mode()] or "???"
end

-- =========================================================================
-- Memory Access
-- =========================================================================
--
-- The ARM1 has a 26-bit address space (64 MiB). Memory is byte-addressable
-- and little-endian (lowest byte at the lowest address).

--- Reads a 32-bit word from memory (little-endian, word-aligned).
function ARM1Simulator:read_word(addr)
  addr = (addr & PC_MASK) & bnot32(3)  -- mask and align
  if addr + 3 >= self.memory_size then return 0 end
  local b0 = self.memory[addr]     or 0
  local b1 = self.memory[addr + 1] or 0
  local b2 = self.memory[addr + 2] or 0
  local b3 = self.memory[addr + 3] or 0
  return mask32(b0 | (b1 << 8) | (b2 << 16) | (b3 << 24))
end

--- Writes a 32-bit word to memory (little-endian, word-aligned).
function ARM1Simulator:write_word(addr, value)
  addr = (addr & PC_MASK) & bnot32(3)
  if addr + 3 >= self.memory_size then return end
  value = mask32(value)
  self.memory[addr]     = value & 0xFF
  self.memory[addr + 1] = (value >> 8)  & 0xFF
  self.memory[addr + 2] = (value >> 16) & 0xFF
  self.memory[addr + 3] = (value >> 24) & 0xFF
end

--- Reads a single byte from memory.
function ARM1Simulator:read_byte(addr)
  addr = addr & 0x03FFFFFF  -- 26-bit address space, all byte positions valid
  if addr >= self.memory_size then return 0 end
  return self.memory[addr] or 0
end

--- Writes a single byte to memory.
function ARM1Simulator:write_byte(addr, value)
  addr = addr & 0x03FFFFFF  -- 26-bit address space, all byte positions valid
  if addr >= self.memory_size then return end
  self.memory[addr] = value & 0xFF
end

--- Loads machine code (bytes table or string) into memory at start_address.
function ARM1Simulator:load_program(code, start_address)
  start_address = start_address or 0
  if type(code) == "string" then
    for i = 1, #code do
      local byte_val = string.byte(code, i)
      local addr = start_address + (i - 1)
      if addr < self.memory_size then
        self.memory[addr] = byte_val
      end
    end
  elseif type(code) == "table" then
    for i, b in ipairs(code) do
      local addr = start_address + (i - 1)
      if addr < self.memory_size then
        self.memory[addr] = b & 0xFF
      end
    end
  end
end

--- Loads a list of 32-bit instruction words into memory.
function ARM1Simulator:load_instructions(instructions, start_address)
  start_address = start_address or 0
  for i, inst in ipairs(instructions) do
    self:write_word(start_address + (i - 1) * 4, inst)
  end
end

-- =========================================================================
-- Condition Evaluation
-- =========================================================================
--
-- Tests whether the given condition code is satisfied by the current flags.
-- This is the behavioral equivalent of the ARM1's condition evaluation PLA.

local function evaluate_condition(cond_code, flags)
  local n, z, c, v = flags.n, flags.z, flags.c, flags.v
  if     cond_code == COND_EQ then return z
  elseif cond_code == COND_NE then return not z
  elseif cond_code == COND_CS then return c
  elseif cond_code == COND_CC then return not c
  elseif cond_code == COND_MI then return n
  elseif cond_code == COND_PL then return not n
  elseif cond_code == COND_VS then return v
  elseif cond_code == COND_VC then return not v
  elseif cond_code == COND_HI then return c and not z
  elseif cond_code == COND_LS then return (not c) or z
  elseif cond_code == COND_GE then return n == v
  elseif cond_code == COND_LT then return n ~= v
  elseif cond_code == COND_GT then return (not z) and (n == v)
  elseif cond_code == COND_LE then return z or (n ~= v)
  elseif cond_code == COND_AL then return true
  elseif cond_code == COND_NV then return false
  else return false
  end
end

-- =========================================================================
-- Barrel Shifter
-- =========================================================================
--
-- The barrel shifter processes the second operand before the ALU. On the
-- real ARM1 chip, it was a 32x32 crossbar network of pass transistors —
-- any of the 32 output bits could be connected to any of the 32 input bits.
--
-- This gives ARM its "shift for free" property: every data processing
-- instruction can shift or rotate its second operand at no clock penalty.
--
-- Returns: result, carry_out

--- Applies a barrel shift to a 32-bit value.
-- @param value  32-bit input
-- @param shift_type  0=LSL, 1=LSR, 2=ASR, 3=ROR
-- @param amount  shift amount
-- @param carry_in  current carry flag (boolean)
-- @param by_register  true if amount came from a register (vs immediate)
-- @return result, carry_out
local function barrel_shift(value, shift_type, amount, carry_in, by_register)
  -- Register shift with amount=0: pass through unchanged
  if by_register and amount == 0 then
    return value, carry_in
  end

  if shift_type == SHIFT_LSL then
    -- LSL: Logical Shift Left — fill with 0s
    if amount == 0 then
      return value, carry_in
    elseif amount == 32 then
      return 0, (value & 1) ~= 0
    elseif amount > 32 then
      return 0, false
    else
      local carry = ((value >> (32 - amount)) & 1) ~= 0
      return mask32(value << amount), carry
    end

  elseif shift_type == SHIFT_LSR then
    -- LSR: Logical Shift Right — fill with 0s
    -- Special: immediate LSR #0 encodes LSR #32
    if not by_register and amount == 0 then
      return 0, (value >> 31) ~= 0
    elseif amount == 0 then
      return value, carry_in
    elseif amount == 32 then
      return 0, (value >> 31) ~= 0
    elseif amount > 32 then
      return 0, false
    else
      local carry = ((value >> (amount - 1)) & 1) ~= 0
      return value >> amount, carry
    end

  elseif shift_type == SHIFT_ASR then
    -- ASR: Arithmetic Shift Right — fill with sign bit
    -- Special: immediate ASR #0 encodes ASR #32
    if not by_register and amount == 0 then
      if (value & 0x80000000) ~= 0 then
        return MASK32, true
      else
        return 0, false
      end
    elseif amount == 0 then
      return value, carry_in
    elseif amount >= 32 then
      if (value & 0x80000000) ~= 0 then
        return MASK32, true
      else
        return 0, false
      end
    else
      local carry = ((value >> (amount - 1)) & 1) ~= 0
      return asr(value, amount), carry
    end

  elseif shift_type == SHIFT_ROR then
    -- ROR: Rotate Right — circular, 33-bit with carry for RRX
    -- Special: immediate ROR #0 encodes RRX (rotate right through carry)
    if not by_register and amount == 0 then
      -- RRX: bit 31 = carry_in, carry_out = bit 0
      local carry = (value & 1) ~= 0
      local result = value >> 1
      if carry_in then result = result | 0x80000000 end
      return mask32(result), carry
    elseif amount == 0 then
      return value, carry_in
    else
      local result = ror32(value, amount)
      local carry = ((result >> 31) & 1) ~= 0
      return result, carry
    end
  end

  return value, carry_in
end

--- Decodes a rotated immediate value (for I=1 data processing instructions).
-- The 8-bit immediate is rotated right by 2*rotate positions.
-- Returns: value, carry_out
local function decode_immediate(imm8, rotate_field)
  if rotate_field == 0 then
    return imm8, false
  end
  local rotate_amount = rotate_field * 2
  local value = ror32(imm8, rotate_amount)
  local carry = (value >> 31) ~= 0
  return value, carry
end

-- =========================================================================
-- ALU — 32-bit Arithmetic Logic Unit
-- =========================================================================
--
-- Performs one of the 16 ARM ALU operations. Flag computation differs:
--   Arithmetic ops (ADD, SUB, etc.): C from adder carry, V from overflow
--   Logical ops (AND, EOR, etc.): C from barrel shifter, V unchanged

--- Performs 32-bit addition with carry, computing carry and overflow.
-- Uses 64-bit arithmetic in Lua (which is fine since Lua integers are 64-bit).
local function add32(a, b, carry_in)
  local cin = carry_in and 1 or 0
  local sum = a + b + cin  -- 64-bit result
  local result = mask32(sum)
  local carry = sum > MASK32
  -- Overflow: both operands have same sign but result differs
  local overflow = (((a ~ result) & (b ~ result)) >> 31) ~= 0
  return result, carry, overflow
end

--- Executes one of the 16 ALU operations.
-- Returns: {result, n, z, c, v, write_result}
local function alu_execute(opcode, a, b, carry_in, shifter_carry, old_v)
  local write_result = not is_test_op(opcode)
  local result, carry, overflow

  -- Logical operations: carry from barrel shifter, V unchanged
  if opcode == OP_AND or opcode == OP_TST then
    result = (a & b) & MASK32
    carry, overflow = shifter_carry, old_v
  elseif opcode == OP_EOR or opcode == OP_TEQ then
    result = (a ~ b) & MASK32
    carry, overflow = shifter_carry, old_v
  elseif opcode == OP_ORR then
    result = (a | b) & MASK32
    carry, overflow = shifter_carry, old_v
  elseif opcode == OP_MOV then
    result = b & MASK32
    carry, overflow = shifter_carry, old_v
  elseif opcode == OP_BIC then
    result = (a & bnot32(b)) & MASK32
    carry, overflow = shifter_carry, old_v
  elseif opcode == OP_MVN then
    result = bnot32(b)
    carry, overflow = shifter_carry, old_v
  -- Arithmetic operations: carry from adder, V from overflow detection
  elseif opcode == OP_ADD or opcode == OP_CMN then
    result, carry, overflow = add32(a, b, false)
  elseif opcode == OP_ADC then
    result, carry, overflow = add32(a, b, carry_in)
  elseif opcode == OP_SUB or opcode == OP_CMP then
    result, carry, overflow = add32(a, bnot32(b), true)
  elseif opcode == OP_SBC then
    result, carry, overflow = add32(a, bnot32(b), carry_in)
  elseif opcode == OP_RSB then
    result, carry, overflow = add32(b, bnot32(a), true)
  elseif opcode == OP_RSC then
    result, carry, overflow = add32(b, bnot32(a), carry_in)
  else
    result = 0; carry = false; overflow = false
  end

  result = mask32(result)
  return {
    result = result,
    n = (result >> 31) ~= 0,
    z = result == 0,
    c = carry,
    v = overflow,
    write_result = write_result
  }
end

-- =========================================================================
-- Decoder
-- =========================================================================
--
-- Extracts all fields from a 32-bit ARM instruction. The real ARM1 decoder
-- was a PLA (Programmable Logic Array) with 42 rows of 36-bit control words.
-- We implement the same decoding purely in Lua bit operations.

--- Decodes a 32-bit ARM instruction into a table of fields.
local function decode(instruction)
  local d = {
    raw       = instruction,
    condition = (instruction >> 28) & 0xF,
    type      = INST_UNDEFINED,
    -- Data processing
    opcode = 0, s = false, rn = 0, rd = 0,
    immediate = false, imm8 = 0, rotate = 0,
    rm = 0, shift_type = 0, shift_by_reg = false,
    shift_imm = 0, rs = 0,
    -- Load/Store
    load = false, byte_access = false, pre_index = false,
    up = false, write_back = false, offset12 = 0,
    -- Block Transfer
    register_list = 0, force_user = false,
    -- Branch
    link = false, branch_offset = 0,
    -- SWI
    swi_comment = 0
  }

  local bits2726 = (instruction >> 26) & 0x3
  local bit25    = (instruction >> 25) & 0x1

  if bits2726 == 0 then
    -- Data Processing / PSR Transfer
    d.type = INST_DATA_PROCESSING
    local is_imm = ((instruction >> 25) & 1) == 1
    d.immediate = is_imm
    d.opcode    = (instruction >> 21) & 0xF
    d.s         = ((instruction >> 20) & 1) == 1
    d.rn        = (instruction >> 16) & 0xF
    d.rd        = (instruction >> 12) & 0xF

    if is_imm then
      d.imm8   = instruction & 0xFF
      d.rotate = (instruction >> 8) & 0xF
    else
      local shift_by_reg = ((instruction >> 4) & 1) == 1
      d.rm            = instruction & 0xF
      d.shift_type    = (instruction >> 5) & 0x3
      d.shift_by_reg  = shift_by_reg
      if shift_by_reg then
        d.rs = (instruction >> 8) & 0xF
      else
        d.shift_imm = (instruction >> 7) & 0x1F
      end
    end

  elseif bits2726 == 1 then
    -- Single Data Transfer (LDR/STR)
    d.type        = INST_LOAD_STORE
    d.immediate   = ((instruction >> 25) & 1) == 1  -- I bit (1=reg offset)
    d.pre_index   = ((instruction >> 24) & 1) == 1
    d.up          = ((instruction >> 23) & 1) == 1
    d.byte_access = ((instruction >> 22) & 1) == 1
    d.write_back  = ((instruction >> 21) & 1) == 1
    d.load        = ((instruction >> 20) & 1) == 1
    d.rn          = (instruction >> 16) & 0xF
    d.rd          = (instruction >> 12) & 0xF
    d.rm          = instruction & 0xF
    d.shift_type  = (instruction >> 5) & 0x3
    d.shift_imm   = (instruction >> 7) & 0x1F
    d.offset12    = instruction & 0xFFF

  elseif bits2726 == 2 and bit25 == 0 then
    -- Block Data Transfer (LDM/STM)
    d.type          = INST_BLOCK_TRANSFER
    d.pre_index     = ((instruction >> 24) & 1) == 1
    d.up            = ((instruction >> 23) & 1) == 1
    d.force_user    = ((instruction >> 22) & 1) == 1
    d.write_back    = ((instruction >> 21) & 1) == 1
    d.load          = ((instruction >> 20) & 1) == 1
    d.rn            = (instruction >> 16) & 0xF
    d.register_list = instruction & 0xFFFF

  elseif bits2726 == 2 and bit25 == 1 then
    -- Branch / Branch with Link
    d.type  = INST_BRANCH
    d.link  = ((instruction >> 24) & 1) == 1
    -- 24-bit signed offset, sign-extended and shifted left by 2
    local raw_offset = instruction & 0x00FFFFFF
    -- Sign-extend from 24 bits to 64 bits
    if (raw_offset >> 23) ~= 0 then
      raw_offset = raw_offset | 0xFFFFFFFFFF000000  -- sign extend
    end
    -- Convert to signed Lua integer and multiply by 4
    d.branch_offset = raw_offset * 4

  elseif bits2726 == 3 then
    -- Coprocessor / SWI
    if ((instruction >> 24) & 0xF) == 0xF then
      d.type = INST_SWI
      d.swi_comment = instruction & 0x00FFFFFF
    else
      d.type = INST_COPROCESSOR
    end

  end

  return d
end

-- =========================================================================
-- Disassembly
-- =========================================================================

local function disasm_reg_list(reg_list)
  local regs = {}
  for i = 0, 15 do
    if (reg_list >> i) & 1 == 1 then
      if i == 15 then table.insert(regs, "PC")
      elseif i == 14 then table.insert(regs, "LR")
      elseif i == 13 then table.insert(regs, "SP")
      else table.insert(regs, "R" .. i)
      end
    end
  end
  return table.concat(regs, ", ")
end

local function disasm_operand2(d)
  if d.immediate then
    local val = decode_immediate(d.imm8, d.rotate)
    return "#" .. val
  else
    if not d.shift_by_reg and d.shift_imm == 0 and d.shift_type == SHIFT_LSL then
      return "R" .. d.rm
    end
    if d.shift_by_reg then
      return string.format("R%d, %s R%d", d.rm, SHIFT_NAMES[d.shift_type], d.rs)
    else
      local amount = d.shift_imm
      -- Encode special cases for disassembly
      if (d.shift_type == SHIFT_LSR or d.shift_type == SHIFT_ASR) and amount == 0 then
        amount = 32
      elseif d.shift_type == SHIFT_ROR and amount == 0 then
        return "R" .. d.rm .. ", RRX"
      end
      return string.format("R%d, %s #%d", d.rm, SHIFT_NAMES[d.shift_type], amount)
    end
  end
end

--- Returns a human-readable assembly string for a decoded instruction.
function ARM1Simulator.disassemble(d)
  local cond = COND_NAMES[d.condition] or "??"

  if d.type == INST_DATA_PROCESSING then
    local op = OP_NAMES[d.opcode] or "???"
    local suf = ""
    if d.s and not is_test_op(d.opcode) then suf = "S" end
    local op2 = disasm_operand2(d)
    if d.opcode == OP_MOV or d.opcode == OP_MVN then
      return string.format("%s%s%s R%d, %s", op, cond, suf, d.rd, op2)
    elseif is_test_op(d.opcode) then
      return string.format("%s%s R%d, %s", op, cond, d.rn, op2)
    else
      return string.format("%s%s%s R%d, R%d, %s", op, cond, suf, d.rd, d.rn, op2)
    end

  elseif d.type == INST_LOAD_STORE then
    local op = d.load and "LDR" or "STR"
    local bsuf = d.byte_access and "B" or ""
    local offset
    if d.immediate then
      if d.shift_imm ~= 0 then
        offset = string.format("R%d, %s #%d", d.rm, SHIFT_NAMES[d.shift_type], d.shift_imm)
      else
        offset = "R" .. d.rm
      end
    else
      offset = "#" .. d.offset12
    end
    local sign = d.up and "" or "-"
    if d.pre_index then
      local wb = d.write_back and "!" or ""
      return string.format("%s%s%s R%d, [R%d, %s%s]%s", op, cond, bsuf, d.rd, d.rn, sign, offset, wb)
    else
      return string.format("%s%s%s R%d, [R%d], %s%s", op, cond, bsuf, d.rd, d.rn, sign, offset)
    end

  elseif d.type == INST_BLOCK_TRANSFER then
    local op = d.load and "LDM" or "STM"
    local mode
    if not d.pre_index and d.up then mode = "IA"
    elseif d.pre_index and d.up then mode = "IB"
    elseif not d.pre_index and not d.up then mode = "DA"
    else mode = "DB"
    end
    local wb = d.write_back and "!" or ""
    return string.format("%s%s%s R%d%s, {%s}", op, cond, mode, d.rn, wb, disasm_reg_list(d.register_list))

  elseif d.type == INST_BRANCH then
    local op = d.link and "BL" or "B"
    return string.format("%s%s #%d", op, cond, d.branch_offset)

  elseif d.type == INST_SWI then
    if d.swi_comment == HALT_SWI then
      return "HLT" .. cond
    else
      return string.format("SWI%s #0x%06X", cond, d.swi_comment)
    end

  elseif d.type == INST_COPROCESSOR then
    return "CDP" .. cond .. " (coprocessor)"

  else
    return string.format("UND%s #0x%08X", cond, d.raw)
  end
end

-- =========================================================================
-- Execution — Step
-- =========================================================================
--
-- The fetch-decode-execute cycle:
--   1. FETCH:   read 32-bit instruction at PC
--   2. DECODE:  extract all fields
--   3. CHECK:   evaluate condition code (4 bits in bits 31:28)
--   4. EXECUTE: if condition met, perform operation
--   5. ADVANCE: PC += 4 (unless branch or PC write)
--
-- ARM pipeline note: the PC is always 8 bytes ahead of the currently
-- executing instruction because of the 3-stage pipeline (Fetch, Decode,
-- Execute). When step() is called, we fetch from current_pc, advance
-- PC by 4, then execute. During execution, reads of R15 return PC+4
-- (the actual PC after advancing, giving the full +8 effect).

local function capture_regs(self)
  local regs = {}
  for i = 0, 15 do
    regs[i] = self:read_register(i)
  end
  return regs
end

-- Reads a register as seen during execution (R15 = PC+8 due to pipeline)
local function read_reg_for_exec(self, index)
  if index == 15 then
    -- PC was already advanced +4 in step(); add 4 more for full pipeline effect
    return mask32(self.regs[15] + 4)
  end
  return self:read_register(index)
end

-- Trap: enter SVC mode, save return address, jump to handler vector
local function trap_undefined(self)
  local r15_val = self.regs[15]
  self.regs[26] = r15_val  -- R14_svc = current PC+flags
  local r15 = self.regs[15]
  r15 = (r15 & bnot32(MODE_MASK)) | ARM1Simulator.MODE_SVC
  r15 = r15 | FLAG_I
  self.regs[15] = mask32(r15)
  self:set_pc(0x04)  -- Undefined Instruction vector
end

-- Execute a data processing instruction
local function execute_data_processing(self, d)
  -- Get first operand (Rn). MOV/MVN ignore Rn.
  local a = 0
  if d.opcode ~= OP_MOV and d.opcode ~= OP_MVN then
    a = read_reg_for_exec(self, d.rn)
  end

  -- Get second operand through barrel shifter
  local current_flags = self:get_flags()
  local b, shifter_carry

  if d.immediate then
    b, shifter_carry = decode_immediate(d.imm8, d.rotate)
    if d.rotate == 0 then shifter_carry = current_flags.c end
  else
    local rm_val = read_reg_for_exec(self, d.rm)
    local shift_amount
    if d.shift_by_reg then
      shift_amount = read_reg_for_exec(self, d.rs) & 0xFF
    else
      shift_amount = d.shift_imm
    end
    b, shifter_carry = barrel_shift(rm_val, d.shift_type, shift_amount, current_flags.c, d.shift_by_reg)
  end

  -- Execute ALU
  local alu = alu_execute(d.opcode, a, b, current_flags.c, shifter_carry, current_flags.v)

  -- Write result to Rd (unless test-only op)
  if alu.write_result then
    if d.rd == 15 then
      if d.s then
        -- MOVS PC, LR: restore entire R15 (PC + flags)
        self.regs[15] = mask32(alu.result)
      else
        self:set_pc(alu.result & PC_MASK)
      end
    else
      self:write_register(d.rd, alu.result)
    end
  end

  -- Update flags if S bit set (and Rd is not R15)
  if (d.s and d.rd ~= 15) or is_test_op(d.opcode) then
    self:set_flags({ n = alu.n, z = alu.z, c = alu.c, v = alu.v })
  end
end

-- Execute a load/store instruction
local function execute_load_store(self, d, mem_reads, mem_writes)
  -- Compute offset
  local offset
  if d.immediate then
    -- I=1: offset is a shifted register
    local rm_val = read_reg_for_exec(self, d.rm)
    if d.shift_imm ~= 0 then
      offset = barrel_shift(rm_val, d.shift_type, d.shift_imm, self:get_flags().c, false)
    else
      offset = rm_val
    end
  else
    -- I=0: offset is a 12-bit immediate
    offset = d.offset12
  end

  local base = read_reg_for_exec(self, d.rn)
  local addr
  if d.up then
    addr = mask32(base + offset)
  else
    addr = mask32(base - offset)
  end

  local transfer_addr = d.pre_index and addr or base

  if d.load then
    -- LDR / LDRB
    local value
    if d.byte_access then
      value = self:read_byte(transfer_addr)
    else
      local word = self:read_word(transfer_addr)
      -- ARM1 quirk: unaligned word load rotates the data
      local rotation = (transfer_addr & 3) * 8
      if rotation ~= 0 then
        word = ror32(word, rotation)
      end
      value = word
    end
    table.insert(mem_reads, { address = transfer_addr, value = value })

    if d.rd == 15 then
      self.regs[15] = mask32(value)
    else
      self:write_register(d.rd, value)
    end
  else
    -- STR / STRB
    local value = read_reg_for_exec(self, d.rd)
    if d.byte_access then
      self:write_byte(transfer_addr, value & 0xFF)
    else
      self:write_word(transfer_addr, value)
    end
    table.insert(mem_writes, { address = transfer_addr, value = value })
  end

  -- Write-back
  if d.write_back or not d.pre_index then
    if d.rn ~= 15 then
      self:write_register(d.rn, addr)
    end
  end
end

-- Execute a block data transfer (LDM/STM)
local function execute_block_transfer(self, d, mem_reads, mem_writes)
  local base = self:read_register(d.rn)
  local reg_list = d.register_list

  -- Count registers in the list
  local count = 0
  for i = 0, 15 do
    if (reg_list >> i) & 1 == 1 then count = count + 1 end
  end

  if count == 0 then return end

  -- Calculate start address based on pre/post and up/down
  local start_addr
  if not d.pre_index and d.up then
    start_addr = base                          -- IA (Increment After)
  elseif d.pre_index and d.up then
    start_addr = mask32(base + 4)             -- IB (Increment Before)
  elseif not d.pre_index and not d.up then
    start_addr = mask32(base - count * 4 + 4) -- DA (Decrement After)
  else
    start_addr = mask32(base - count * 4)     -- DB (Decrement Before)
  end

  -- Transfer registers in order (lowest to highest number always)
  local addr = start_addr
  for i = 0, 15 do
    if (reg_list >> i) & 1 == 1 then
      if d.load then
        local value = self:read_word(addr)
        table.insert(mem_reads, { address = addr, value = value })
        if i == 15 then
          self.regs[15] = mask32(value)
        else
          self:write_register(i, value)
        end
      else
        local value
        if i == 15 then
          value = mask32(self.regs[15] + 4)  -- STM stores PC+12 (pipeline)
        else
          value = self:read_register(i)
        end
        self:write_word(addr, value)
        table.insert(mem_writes, { address = addr, value = value })
      end
      addr = mask32(addr + 4)
    end
  end

  -- Write-back
  if d.write_back then
    local new_base
    if d.up then
      new_base = mask32(base + count * 4)
    else
      new_base = mask32(base - count * 4)
    end
    self:write_register(d.rn, new_base)
  end
end

-- Execute a branch instruction
local function execute_branch(self, d)
  -- PC was advanced +4 in step(); branch is relative to PC+8 (pipeline)
  local branch_base = mask32(self:get_pc() + 4)

  if d.link then
    -- BL: save return address (full R15 with flags/mode) in R14
    local return_addr = self.regs[15]
    self:write_register(14, return_addr)
  end

  -- Compute target: branch_base + signed_offset
  local target = (branch_base + d.branch_offset) & 0x3FFFFFFF  -- keep 30 bits
  self:set_pc(target & PC_MASK)
end

-- Execute a SWI instruction
local function execute_swi(self, d)
  if d.swi_comment == HALT_SWI then
    self.halted = true
  else
    -- Real SWI: enter Supervisor mode
    -- Save return address (current R15 with flags) into R14_svc
    local r15_val = self.regs[15]
    self.regs[25] = r15_val  -- R13_svc (unused but set for completeness)
    self.regs[26] = r15_val  -- R14_svc = return address

    -- Set SVC mode, disable IRQs
    local r15 = self.regs[15]
    r15 = (r15 & bnot32(MODE_MASK)) | ARM1Simulator.MODE_SVC
    r15 = r15 | FLAG_I
    self.regs[15] = mask32(r15)
    self:set_pc(0x08)  -- SWI vector
  end
end

--- Executes one instruction and returns a trace table.
function ARM1Simulator:step()
  local current_pc = self:get_pc()
  local regs_before = capture_regs(self)
  local flags_before = self:get_flags()

  -- Fetch
  local instruction = self:read_word(current_pc)

  -- Decode
  local decoded = decode(instruction)

  -- Evaluate condition
  local cond_met = evaluate_condition(decoded.condition, flags_before)

  local mem_reads = {}
  local mem_writes = {}

  -- Advance PC before execute (pipeline: PC = PC + 4)
  self:set_pc(mask32(current_pc + 4))

  -- Execute if condition met
  if cond_met then
    if decoded.type == INST_DATA_PROCESSING then
      execute_data_processing(self, decoded)
    elseif decoded.type == INST_LOAD_STORE then
      execute_load_store(self, decoded, mem_reads, mem_writes)
    elseif decoded.type == INST_BLOCK_TRANSFER then
      execute_block_transfer(self, decoded, mem_reads, mem_writes)
    elseif decoded.type == INST_BRANCH then
      execute_branch(self, decoded)
    elseif decoded.type == INST_SWI then
      execute_swi(self, decoded)
    elseif decoded.type == INST_COPROCESSOR then
      trap_undefined(self)
    elseif decoded.type == INST_UNDEFINED then
      trap_undefined(self)
    end
  end

  -- Build trace
  local trace = {
    address        = current_pc,
    raw            = instruction,
    mnemonic       = ARM1Simulator.disassemble(decoded),
    condition      = COND_NAMES[decoded.condition] or "??",
    condition_met  = cond_met,
    regs_before    = regs_before,
    regs_after     = capture_regs(self),
    flags_before   = flags_before,
    flags_after    = self:get_flags(),
    memory_reads   = mem_reads,
    memory_writes  = mem_writes,
  }

  return trace
end

--- Executes instructions until halted or max_steps reached.
-- @param max_steps  maximum number of instructions to execute
-- @return traces, final_halted_state
function ARM1Simulator:run(max_steps)
  max_steps = max_steps or 100000
  local traces = {}

  while not self.halted and #traces < max_steps do
    local trace = self:step()
    table.insert(traces, trace)
  end

  return traces
end

-- =========================================================================
-- Encoding Helpers
-- =========================================================================
--
-- These helpers create instruction words for test programs, eliminating
-- the need for a full assembler. The encoding follows the ARM Architecture
-- Reference Manual bit field specifications.

--- Creates a MOV immediate instruction: MOV Rd, #imm8
function ARM1Simulator.encode_mov_imm(condition, rd, imm8)
  -- bits 31:28=cond, 27:26=00, 25=1(imm), 24:21=1101(MOV), 20=0(no flags)
  -- bits 19:16=0000(Rn ignored), 15:12=Rd, 11:8=0000(rotate=0), 7:0=imm8
  local inst = (condition << 28) | 0x03A00000  -- 0b00001110_10100000...
  inst = inst | (rd << 12) | imm8
  return mask32(inst)
end

--- Creates a data processing instruction with register operand.
-- Returns: ALU_REG cond, opcode, s, rd, rn, rm
function ARM1Simulator.encode_alu_reg(condition, opcode, s, rd, rn, rm)
  local s_bit = s and 1 or 0
  local inst = (condition << 28) | (opcode << 21) | (s_bit << 20)
  inst = inst | (rn << 16) | (rd << 12) | rm
  return mask32(inst)
end

--- Creates a data processing instruction with register + shift.
function ARM1Simulator.encode_alu_reg_shift(condition, opcode, s, rd, rn, rm, shift_type, shift_imm)
  local s_bit = s and 1 or 0
  local inst = (condition << 28) | (opcode << 21) | (s_bit << 20)
  inst = inst | (rn << 16) | (rd << 12) | (shift_imm << 7) | (shift_type << 5) | rm
  return mask32(inst)
end

--- Creates a Branch or Branch-with-Link instruction.
function ARM1Simulator.encode_branch(condition, link, offset)
  -- offset is in bytes, relative to the current instruction address.
  -- offset is relative to PC+8 (the pipeline prefetch base).
  -- execute_branch computes branch_base = (PC+4)+4 = PC+8 after step() advances PC.
  -- target = branch_base + branch_offset = (PC+8) + offset.
  -- So encoded = offset / 4.
  local inst = (condition << 28) | 0x0A000000
  if link then inst = inst | 0x01000000 end
  local encoded = math.floor(offset / 4) & 0x00FFFFFF
  return mask32(inst | encoded)
end

--- Creates the halt pseudo-instruction (SWI 0x123456).
function ARM1Simulator.encode_halt()
  return mask32((COND_AL << 28) | 0x0F000000 | HALT_SWI)
end

--- Creates a Load Register with immediate offset.
function ARM1Simulator.encode_ldr(condition, rd, rn, offset, pre_index)
  local inst = (condition << 28) | 0x04100000  -- L=1, I=0
  inst = inst | (rd << 12) | (rn << 16)
  if pre_index then inst = inst | (1 << 24) end
  if offset >= 0 then
    inst = inst | (1 << 23) | (offset & 0xFFF)  -- U=1, add offset
  else
    inst = inst | ((-offset) & 0xFFF)             -- U=0, subtract offset
  end
  return mask32(inst)
end

--- Creates a Store Register with immediate offset.
function ARM1Simulator.encode_str(condition, rd, rn, offset, pre_index)
  local inst = (condition << 28) | 0x04000000  -- L=0, I=0
  inst = inst | (rd << 12) | (rn << 16)
  if pre_index then inst = inst | (1 << 24) end
  if offset >= 0 then
    inst = inst | (1 << 23) | (offset & 0xFFF)
  else
    inst = inst | ((-offset) & 0xFFF)
  end
  return mask32(inst)
end

--- Creates a Load Multiple instruction.
function ARM1Simulator.encode_ldm(condition, rn, reg_list, write_back, bt_mode)
  local inst = (condition << 28) | 0x08100000  -- bits 27:25=100, L=1
  inst = inst | (rn << 16) | reg_list
  if write_back then inst = inst | (1 << 21) end
  if bt_mode == "IA" then
    inst = inst | (1 << 23)
  elseif bt_mode == "IB" then
    inst = inst | (1 << 24) | (1 << 23)
  elseif bt_mode == "DA" then
    -- no P or U bits
  elseif bt_mode == "DB" then
    inst = inst | (1 << 24)
  end
  return mask32(inst)
end

--- Creates a Store Multiple instruction.
function ARM1Simulator.encode_stm(condition, rn, reg_list, write_back, bt_mode)
  local inst = ARM1Simulator.encode_ldm(condition, rn, reg_list, write_back, bt_mode)
  return mask32(inst & bnot32(1 << 20))  -- Clear L bit
end

-- =========================================================================
-- Condition code constants (exported for callers)
-- =========================================================================

ARM1Simulator.COND_EQ = COND_EQ
ARM1Simulator.COND_NE = COND_NE
ARM1Simulator.COND_CS = COND_CS
ARM1Simulator.COND_CC = COND_CC
ARM1Simulator.COND_MI = COND_MI
ARM1Simulator.COND_PL = COND_PL
ARM1Simulator.COND_VS = COND_VS
ARM1Simulator.COND_VC = COND_VC
ARM1Simulator.COND_HI = COND_HI
ARM1Simulator.COND_LS = COND_LS
ARM1Simulator.COND_GE = COND_GE
ARM1Simulator.COND_LT = COND_LT
ARM1Simulator.COND_GT = COND_GT
ARM1Simulator.COND_LE = COND_LE
ARM1Simulator.COND_AL = COND_AL
ARM1Simulator.COND_NV = COND_NV

-- =========================================================================
-- ALU opcode constants (exported for callers)
-- =========================================================================

ARM1Simulator.OP_AND = OP_AND
ARM1Simulator.OP_EOR = OP_EOR
ARM1Simulator.OP_SUB = OP_SUB
ARM1Simulator.OP_RSB = OP_RSB
ARM1Simulator.OP_ADD = OP_ADD
ARM1Simulator.OP_ADC = OP_ADC
ARM1Simulator.OP_SBC = OP_SBC
ARM1Simulator.OP_RSC = OP_RSC
ARM1Simulator.OP_TST = OP_TST
ARM1Simulator.OP_TEQ = OP_TEQ
ARM1Simulator.OP_CMP = OP_CMP
ARM1Simulator.OP_CMN = OP_CMN
ARM1Simulator.OP_ORR = OP_ORR
ARM1Simulator.OP_MOV = OP_MOV
ARM1Simulator.OP_BIC = OP_BIC
ARM1Simulator.OP_MVN = OP_MVN

-- Shift type constants
ARM1Simulator.SHIFT_LSL = SHIFT_LSL
ARM1Simulator.SHIFT_LSR = SHIFT_LSR
ARM1Simulator.SHIFT_ASR = SHIFT_ASR
ARM1Simulator.SHIFT_ROR = SHIFT_ROR

-- Instruction type constants
ARM1Simulator.INST_DATA_PROCESSING = INST_DATA_PROCESSING
ARM1Simulator.INST_LOAD_STORE      = INST_LOAD_STORE
ARM1Simulator.INST_BLOCK_TRANSFER  = INST_BLOCK_TRANSFER
ARM1Simulator.INST_BRANCH          = INST_BRANCH
ARM1Simulator.INST_SWI             = INST_SWI
ARM1Simulator.INST_COPROCESSOR     = INST_COPROCESSOR
ARM1Simulator.INST_UNDEFINED       = INST_UNDEFINED

-- Program counter and special SWI constants (exported for gate-level simulator)
ARM1Simulator.PC_MASK  = 0x03FFFFFC  -- bits 25:2: Program Counter field in R15
ARM1Simulator.HALT_SWI = 0x123456    -- pseudo-halt SWI number used by encode_halt()

return ARM1Simulator

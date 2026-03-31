-- ==========================================================================
-- ARM1 Behavioral Simulator — Lua Port
-- ==========================================================================
--
-- The ARM1 was designed by Sophie Wilson and Steve Furber at Acorn Computers
-- in Cambridge, UK. First silicon powered on April 26, 1985 — and worked
-- correctly on the very first attempt. The ARM1 had just 25,000 transistors
-- and a 26-bit address space (64 MiB). Its accidentally low power consumption
-- (~0.1W) later made the ARM architecture dominant in mobile computing.
--
-- # Instruction Format
--
-- Every ARM instruction is 32 bits wide and follows this layout:
--
--   31:28  27:26  25  24:21     20  19:16  15:12  11:0
--   Cond   Type   I   Opcode/   S   Rn     Rd     Operand2
--                     Function
--
-- # Register File (26-bit architecture)
--
--   Registers 0-15 are visible to the programmer.
--   R15 = PC (bits 25:2) + NZCVIF flags (bits 31:26) + mode (bits 1:0)
--
--   Physical register layout (27 total):
--     0-15:  base registers (R0-R15)
--    16-22:  FIQ banked registers (R8_fiq..R14_fiq)
--    23-24:  IRQ banked registers (R13_irq, R14_irq)
--    25-26:  SVC banked registers (R13_svc, R14_svc)
--
-- # R15 Layout
--
--   Bit 31: N (Negative)       Bit 27: I (IRQ disable)
--   Bit 30: Z (Zero)           Bit 26: F (FIQ disable)
--   Bit 29: C (Carry)          Bits 25:2: PC (24-bit word address)
--   Bit 28: V (Overflow)       Bits 1:0: Processor Mode

local M = {}

-- =========================================================================
-- Constants
-- =========================================================================

-- Processor modes
M.MODE_USR = 0   -- User mode (normal execution)
M.MODE_FIQ = 1   -- Fast IRQ (banks R8-R14)
M.MODE_IRQ = 2   -- IRQ mode (banks R13-R14)
M.MODE_SVC = 3   -- Supervisor mode (banks R13-R14)

-- Condition codes (bits 31:28 of every instruction)
M.COND_EQ  = 0x0  -- Equal (Z=1)
M.COND_NE  = 0x1  -- Not Equal (Z=0)
M.COND_CS  = 0x2  -- Carry Set / Unsigned >= (C=1)
M.COND_CC  = 0x3  -- Carry Clear / Unsigned < (C=0)
M.COND_MI  = 0x4  -- Minus / Negative (N=1)
M.COND_PL  = 0x5  -- Plus / Positive (N=0)
M.COND_VS  = 0x6  -- Overflow Set (V=1)
M.COND_VC  = 0x7  -- Overflow Clear (V=0)
M.COND_HI  = 0x8  -- Unsigned Higher (C=1 AND Z=0)
M.COND_LS  = 0x9  -- Unsigned Lower or Same (C=0 OR Z=1)
M.COND_GE  = 0xA  -- Signed >= (N=V)
M.COND_LT  = 0xB  -- Signed < (N!=V)
M.COND_GT  = 0xC  -- Signed > (Z=0 AND N=V)
M.COND_LE  = 0xD  -- Signed <= (Z=1 OR N!=V)
M.COND_AL  = 0xE  -- Always (unconditional)
M.COND_NV  = 0xF  -- Never (do not use)

-- ALU opcodes (bits 24:21 of data processing instructions)
M.OP_AND = 0x0  -- AND: Rd = Rn AND Op2
M.OP_EOR = 0x1  -- EOR: Rd = Rn XOR Op2
M.OP_SUB = 0x2  -- SUB: Rd = Rn - Op2
M.OP_RSB = 0x3  -- RSB: Rd = Op2 - Rn (Reverse SUBtract)
M.OP_ADD = 0x4  -- ADD: Rd = Rn + Op2
M.OP_ADC = 0x5  -- ADC: Rd = Rn + Op2 + C
M.OP_SBC = 0x6  -- SBC: Rd = Rn - Op2 - (1 - C)
M.OP_RSC = 0x7  -- RSC: Rd = Op2 - Rn - (1 - C)
M.OP_TST = 0x8  -- TST: flags from Rn AND Op2 (no write)
M.OP_TEQ = 0x9  -- TEQ: flags from Rn XOR Op2 (no write)
M.OP_CMP = 0xA  -- CMP: flags from Rn - Op2 (no write)
M.OP_CMN = 0xB  -- CMN: flags from Rn + Op2 (no write)
M.OP_ORR = 0xC  -- ORR: Rd = Rn OR Op2
M.OP_MOV = 0xD  -- MOV: Rd = Op2
M.OP_BIC = 0xE  -- BIC: Rd = Rn AND NOT Op2 (Bit Clear)
M.OP_MVN = 0xF  -- MVN: Rd = NOT Op2

local OP_NAMES = {
    [0]="AND","EOR","SUB","RSB","ADD","ADC","SBC","RSC",
    "TST","TEQ","CMP","CMN","ORR","MOV","BIC","MVN"
}

-- Shift types
M.SHIFT_LSL = 0  -- Logical Shift Left
M.SHIFT_LSR = 1  -- Logical Shift Right
M.SHIFT_ASR = 2  -- Arithmetic Shift Right (sign-extending)
M.SHIFT_ROR = 3  -- Rotate Right (or RRX when amount=0, immediate)

local SHIFT_NAMES = {[0]="LSL","LSR","ASR","ROR"}

-- R15 bit masks
M.FLAG_N    = 0x80000000  -- Negative flag
M.FLAG_Z    = 0x40000000  -- Zero flag
M.FLAG_C    = 0x20000000  -- Carry flag
M.FLAG_V    = 0x10000000  -- Overflow flag
M.FLAG_I    = 0x08000000  -- IRQ disable
M.FLAG_F    = 0x04000000  -- FIQ disable
M.PC_MASK   = 0x03FFFFFC  -- PC bits 25:2 (26-bit byte address, word-aligned)
M.MODE_MASK = 0x3          -- Mode bits 1:0
M.MASK32    = 0xFFFFFFFF   -- 32-bit mask
M.HALT_SWI  = 0x123456    -- Our pseudo-halt SWI number

-- Instruction type constants
M.INST_DATA_PROCESSING = 0
M.INST_LOAD_STORE      = 1
M.INST_BLOCK_TRANSFER  = 2
M.INST_BRANCH          = 3
M.INST_SWI             = 4
M.INST_COPROCESSOR     = 5
M.INST_UNDEFINED       = 6

-- =========================================================================
-- Helpers
-- =========================================================================

local function mode_string(m)
    if m == M.MODE_USR then return "USR"
    elseif m == M.MODE_FIQ then return "FIQ"
    elseif m == M.MODE_IRQ then return "IRQ"
    elseif m == M.MODE_SVC then return "SVC"
    else return "???" end
end
M.mode_string = mode_string

local function cond_string(c)
    local t = {[0]="EQ","NE","CS","CC","MI","PL","VS","VC","HI","LS","GE","LT","GT","LE","","NV"}
    return t[c] or "??"
end
M.cond_string = cond_string

local function op_string(op)
    return OP_NAMES[op] or "???"
end
M.op_string = op_string

local function shift_string(st)
    return SHIFT_NAMES[st] or "???"
end
M.shift_string = shift_string

local function test_op(opcode)
    return opcode >= M.OP_TST and opcode <= M.OP_CMN
end
M.test_op = test_op

local function logical_op(opcode)
    return opcode == M.OP_AND or opcode == M.OP_EOR
        or opcode == M.OP_TST or opcode == M.OP_TEQ
        or opcode == M.OP_ORR or opcode == M.OP_MOV
        or opcode == M.OP_BIC or opcode == M.OP_MVN
end
M.logical_op = logical_op

-- =========================================================================
-- CPU Construction
-- =========================================================================

-- Maps a logical register index (0-15) to a physical register index
-- based on the current mode stored in R15.
local function physical_reg(regs, index)
    local r15 = regs[15]
    local mode = r15 & M.MODE_MASK

    if mode == M.MODE_FIQ and index >= 8 and index <= 14 then
        return 16 + (index - 8)
    elseif mode == M.MODE_IRQ and index >= 13 and index <= 14 then
        return 23 + (index - 13)
    elseif mode == M.MODE_SVC and index >= 13 and index <= 14 then
        return 25 + (index - 13)
    else
        return index
    end
end

-- Creates a new ARM1 CPU state table.
--
--   cpu.regs    — array [0..26] of 32-bit integers (physical registers)
--   cpu.memory  — array [0..size-1] of bytes
--   cpu.halted  — boolean
--
-- On power-on, the ARM1 enters Supervisor mode with IRQs and FIQs disabled
-- and begins executing from address 0x00000000.
function M.new(memory_size)
    memory_size = memory_size or (1024 * 1024)
    if memory_size <= 0 then memory_size = 1024 * 1024 end

    local mem = {}
    for i = 0, memory_size - 1 do mem[i] = 0 end

    local cpu = {
        regs = {},
        memory = mem,
        mem_size = memory_size,
        halted = false,
    }
    for i = 0, 26 do cpu.regs[i] = 0 end

    return M.reset(cpu)
end

-- Resets the CPU to power-on state: SVC mode, IRQ/FIQ disabled, PC = 0.
function M.reset(cpu)
    for i = 0, 26 do cpu.regs[i] = 0 end
    -- Set R15: SVC mode (bits 1:0 = 11), IRQ disabled (bit 27), FIQ disabled (bit 26)
    cpu.regs[15] = M.FLAG_I | M.FLAG_F | M.MODE_SVC
    cpu.halted = false
    return cpu
end

-- =========================================================================
-- Register Access
-- =========================================================================

-- Reads logical register n (0-15) respecting mode banking.
function M.read_register(cpu, n)
    local phys = physical_reg(cpu.regs, n)
    return cpu.regs[phys]
end

-- Writes logical register n (0-15) respecting mode banking.
function M.write_register(cpu, n, value)
    local phys = physical_reg(cpu.regs, n)
    cpu.regs[phys] = value & M.MASK32
end

-- Returns the current program counter (26-bit byte address).
function M.get_pc(cpu)
    return cpu.regs[15] & M.PC_MASK
end

-- Sets the PC portion of R15 without disturbing flags or mode.
function M.set_pc(cpu, addr)
    local r15 = cpu.regs[15]
    cpu.regs[15] = ((r15 & (~M.PC_MASK & M.MASK32)) | (addr & M.PC_MASK)) & M.MASK32
end

-- Returns the current flags as a table {n, z, c, v} (booleans).
function M.get_flags(cpu)
    local r15 = cpu.regs[15]
    return {
        n = (r15 & M.FLAG_N) ~= 0,
        z = (r15 & M.FLAG_Z) ~= 0,
        c = (r15 & M.FLAG_C) ~= 0,
        v = (r15 & M.FLAG_V) ~= 0,
    }
end

-- Sets the condition flags in R15.
function M.set_flags(cpu, n, z, c, v)
    local r15 = cpu.regs[15]
    local mask = ~(M.FLAG_N | M.FLAG_Z | M.FLAG_C | M.FLAG_V) & M.MASK32
    r15 = r15 & mask
    if n then r15 = r15 | M.FLAG_N end
    if z then r15 = r15 | M.FLAG_Z end
    if c then r15 = r15 | M.FLAG_C end
    if v then r15 = r15 | M.FLAG_V end
    cpu.regs[15] = r15 & M.MASK32
end

-- Returns the current processor mode.
function M.get_mode(cpu)
    return cpu.regs[15] & M.MODE_MASK
end

-- =========================================================================
-- Memory Access
-- =========================================================================

-- Reads a 32-bit word from memory (little-endian, word-aligned).
function M.read_word(cpu, addr)
    addr = addr & M.PC_MASK
    local a = addr & (~3 & M.MASK32)
    if a + 3 >= cpu.mem_size then return 0 end
    local b0 = cpu.memory[a]   or 0
    local b1 = cpu.memory[a+1] or 0
    local b2 = cpu.memory[a+2] or 0
    local b3 = cpu.memory[a+3] or 0
    return (b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)) & M.MASK32
end

-- Writes a 32-bit word to memory (little-endian, word-aligned).
function M.write_word(cpu, addr, value)
    addr = addr & M.PC_MASK
    local a = addr & (~3 & M.MASK32)
    if a + 3 >= cpu.mem_size then return end
    value = value & M.MASK32
    cpu.memory[a]   = value & 0xFF
    cpu.memory[a+1] = (value >> 8)  & 0xFF
    cpu.memory[a+2] = (value >> 16) & 0xFF
    cpu.memory[a+3] = (value >> 24) & 0xFF
end

-- Reads a single byte from memory.
function M.read_byte(cpu, addr)
    addr = addr & M.PC_MASK
    if addr >= cpu.mem_size then return 0 end
    return cpu.memory[addr] or 0
end

-- Writes a single byte to memory.
function M.write_byte(cpu, addr, value)
    addr = addr & M.PC_MASK
    if addr >= cpu.mem_size then return end
    cpu.memory[addr] = value & 0xFF
end

-- Loads a list of 32-bit instruction words into memory starting at address 0.
function M.load_instructions(cpu, instructions)
    local addr = 0
    for _, inst in ipairs(instructions) do
        M.write_word(cpu, addr, inst)
        addr = addr + 4
    end
end

-- =========================================================================
-- Condition Evaluation
-- =========================================================================
--
-- Every ARM1 instruction has a 4-bit condition code. The instruction only
-- executes if the current flags satisfy the condition.
--
--   Code  Suffix  Test
--   ----  ------  ----
--   0000  EQ      Z = 1
--   0001  NE      Z = 0
--   0010  CS      C = 1
--   0011  CC      C = 0
--   0100  MI      N = 1
--   0101  PL      N = 0
--   0110  VS      V = 1
--   0111  VC      V = 0
--   1000  HI      C = 1 AND Z = 0
--   1001  LS      C = 0 OR  Z = 1
--   1010  GE      N = V
--   1011  LT      N ≠ V
--   1100  GT      Z = 0 AND N = V
--   1101  LE      Z = 1 OR  N ≠ V
--   1110  AL      always
--   1111  NV      never

function M.evaluate_condition(cond, flags)
    local n, z, c, v = flags.n, flags.z, flags.c, flags.v
    if     cond == M.COND_EQ then return z
    elseif cond == M.COND_NE then return not z
    elseif cond == M.COND_CS then return c
    elseif cond == M.COND_CC then return not c
    elseif cond == M.COND_MI then return n
    elseif cond == M.COND_PL then return not n
    elseif cond == M.COND_VS then return v
    elseif cond == M.COND_VC then return not v
    elseif cond == M.COND_HI then return c and not z
    elseif cond == M.COND_LS then return (not c) or z
    elseif cond == M.COND_GE then return n == v
    elseif cond == M.COND_LT then return n ~= v
    elseif cond == M.COND_GT then return (not z) and (n == v)
    elseif cond == M.COND_LE then return z or (n ~= v)
    elseif cond == M.COND_AL then return true
    elseif cond == M.COND_NV then return false
    else   return false
    end
end

-- =========================================================================
-- Barrel Shifter
-- =========================================================================
--
-- The barrel shifter is the ARM1's most distinctive hardware feature. On
-- the real chip, it was a 32x32 crossbar network of pass transistors. Every
-- data processing instruction's second operand passes through the barrel
-- shifter before reaching the ALU — shifts are free in ARM.
--
-- Returns: result (32-bit int), carry_out (boolean)

-- LSL: Logical Shift Left
local function shift_lsl(value, amount, carry_in)
    if amount == 0 then return value, carry_in end
    if amount >= 32 then
        if amount == 32 then
            return 0, (value & 1) ~= 0
        else
            return 0, false
        end
    end
    local carry = ((value >> (32 - amount)) & 1) ~= 0
    local result = (value << amount) & M.MASK32
    return result, carry
end

-- LSR: Logical Shift Right
-- Special case: immediate LSR #0 encodes LSR #32
local function shift_lsr(value, amount, carry_in, by_register)
    if amount == 0 and not by_register then
        -- Immediate LSR #0 = LSR #32
        return 0, (value >> 31) ~= 0
    elseif amount == 0 and by_register then
        return value, carry_in
    elseif amount >= 32 then
        if amount == 32 then return 0, (value >> 31) ~= 0
        else return 0, false end
    end
    local carry = ((value >> (amount - 1)) & 1) ~= 0
    return value >> amount, carry
end

-- ASR: Arithmetic Shift Right (sign-extending)
-- Special case: immediate ASR #0 encodes ASR #32
local function shift_asr(value, amount, carry_in, by_register)
    if amount == 0 and not by_register then
        -- Immediate ASR #0 = ASR #32
        if (value >> 31) ~= 0 then return M.MASK32, true
        else return 0, false end
    elseif amount == 0 and by_register then
        return value, carry_in
    elseif amount >= 32 then
        if (value >> 31) ~= 0 then return M.MASK32, true
        else return 0, false end
    end
    local carry = ((value >> (amount - 1)) & 1) ~= 0
    -- Sign-extend: fill upper bits with bit 31
    if (value >> 31) ~= 0 then
        local fill = (M.MASK32 << (32 - amount)) & M.MASK32
        return ((value >> amount) | fill) & M.MASK32, carry
    else
        return value >> amount, carry
    end
end

-- ROR: Rotate Right
-- Special case: immediate ROR #0 = RRX (rotate right through carry)
local function shift_ror(value, amount, carry_in, by_register)
    if amount == 0 and not by_register then
        -- RRX: rotate right through carry
        local carry = (value & 1) ~= 0
        local result = value >> 1
        if carry_in then result = result | 0x80000000 end
        return result & M.MASK32, carry
    elseif amount == 0 and by_register then
        return value, carry_in
    end
    amount = amount & 31
    if amount == 0 then
        return value, (value >> 31) ~= 0
    end
    local result = ((value >> amount) | (value << (32 - amount))) & M.MASK32
    local carry = ((result >> 31) & 1) ~= 0
    return result, carry
end

-- Applies a barrel shift to a 32-bit value.
--   value:       the input integer
--   shift_type:  0=LSL, 1=LSR, 2=ASR, 3=ROR
--   amount:      shift amount (0-31 for register, special for immediate)
--   carry_in:    current C flag (boolean)
--   by_register: true if amount comes from a register
--
-- Returns: result (int), carry_out (boolean)
function M.barrel_shift(value, shift_type, amount, carry_in, by_register)
    if amount == 0 and by_register then
        -- Register shift by 0 = no change, no carry change
        return value, carry_in
    end
    if shift_type == M.SHIFT_LSL then
        return shift_lsl(value, amount, carry_in)
    elseif shift_type == M.SHIFT_LSR then
        return shift_lsr(value, amount, carry_in, by_register)
    elseif shift_type == M.SHIFT_ASR then
        return shift_asr(value, amount, carry_in, by_register)
    elseif shift_type == M.SHIFT_ROR then
        return shift_ror(value, amount, carry_in, by_register)
    end
    return value, carry_in
end

-- Decodes a rotated 8-bit immediate from the Operand2 field.
-- Returns: value (int), carry_out (boolean)
function M.decode_immediate(imm8, rotate_field)
    if rotate_field == 0 then return imm8, false end
    local rotate_amount = rotate_field * 2
    local value = ((imm8 >> rotate_amount) | (imm8 << (32 - rotate_amount))) & M.MASK32
    local carry = (value >> 31) ~= 0
    return value, carry
end

-- =========================================================================
-- ALU
-- =========================================================================
--
-- The ARM1's ALU supports 16 operations. Flag computation differs:
--   Arithmetic ops: C = adder carry out, V = signed overflow
--   Logical ops:    C = barrel shifter carry out, V = unchanged

local function add32(a, b, carry_in)
    -- Use Lua integers (64-bit on 64-bit platforms) for arithmetic
    local cin = carry_in and 1 or 0
    local sum = a + b + cin
    local result = sum & M.MASK32
    local carry = (sum >> 32) ~= 0
    -- Overflow: inputs same sign but result differs
    local overflow = (((a ~ result) & (b ~ result)) >> 31) ~= 0
    return result, carry, overflow
end

-- Executes one of the 16 ALU operations.
-- Returns a table: {result, n, z, c, v, write_result}
function M.alu_execute(opcode, a, b, carry_in, shifter_carry, old_v)
    local result, carry, overflow
    local write_result = not test_op(opcode)

    if opcode == M.OP_AND or opcode == M.OP_TST then
        result = a & b
        carry, overflow = shifter_carry, old_v
    elseif opcode == M.OP_EOR or opcode == M.OP_TEQ then
        result = a ~ b
        carry, overflow = shifter_carry, old_v
    elseif opcode == M.OP_ORR then
        result = a | b
        carry, overflow = shifter_carry, old_v
    elseif opcode == M.OP_MOV then
        result = b
        carry, overflow = shifter_carry, old_v
    elseif opcode == M.OP_BIC then
        result = a & (~b & M.MASK32)
        carry, overflow = shifter_carry, old_v
    elseif opcode == M.OP_MVN then
        result = ~b & M.MASK32
        carry, overflow = shifter_carry, old_v
    elseif opcode == M.OP_ADD or opcode == M.OP_CMN then
        result, carry, overflow = add32(a, b, false)
    elseif opcode == M.OP_ADC then
        result, carry, overflow = add32(a, b, carry_in)
    elseif opcode == M.OP_SUB or opcode == M.OP_CMP then
        result, carry, overflow = add32(a, ~b & M.MASK32, true)
    elseif opcode == M.OP_SBC then
        result, carry, overflow = add32(a, ~b & M.MASK32, carry_in)
    elseif opcode == M.OP_RSB then
        result, carry, overflow = add32(b, ~a & M.MASK32, true)
    elseif opcode == M.OP_RSC then
        result, carry, overflow = add32(b, ~a & M.MASK32, carry_in)
    else
        result = 0
        carry, overflow = shifter_carry, old_v
    end

    result = result & M.MASK32
    return {
        result = result,
        n = (result >> 31) ~= 0,
        z = result == 0,
        c = carry,
        v = overflow,
        write_result = write_result,
    }
end

-- =========================================================================
-- Decoder
-- =========================================================================
--
-- Extracts all fields from a 32-bit ARM instruction word.
-- Returns a decoded instruction table.

local function decode_data_processing(d, inst)
    local is_imm = ((inst >> 25) & 1) == 1
    d.immediate  = is_imm
    d.opcode     = (inst >> 21) & 0xF
    d.s          = ((inst >> 20) & 1) == 1
    d.rn         = (inst >> 16) & 0xF
    d.rd         = (inst >> 12) & 0xF

    if is_imm then
        d.imm8   = inst & 0xFF
        d.rotate = (inst >> 8) & 0xF
    else
        d.shift_by_reg = ((inst >> 4) & 1) == 1
        d.rm           = inst & 0xF
        d.shift_type   = (inst >> 5) & 0x3
        if d.shift_by_reg then
            d.rs = (inst >> 8) & 0xF
        else
            d.shift_imm = (inst >> 7) & 0x1F
        end
    end
    return d
end

local function decode_load_store(d, inst)
    d.immediate   = ((inst >> 25) & 1) == 1
    d.pre_index   = ((inst >> 24) & 1) == 1
    d.up          = ((inst >> 23) & 1) == 1
    d.byte        = ((inst >> 22) & 1) == 1
    d.write_back  = ((inst >> 21) & 1) == 1
    d.load        = ((inst >> 20) & 1) == 1
    d.rn          = (inst >> 16) & 0xF
    d.rd          = (inst >> 12) & 0xF
    d.rm          = inst & 0xF
    d.shift_type  = (inst >> 5) & 0x3
    d.shift_imm   = (inst >> 7) & 0x1F
    d.offset12    = inst & 0xFFF
    return d
end

local function decode_block_transfer(d, inst)
    d.pre_index     = ((inst >> 24) & 1) == 1
    d.up            = ((inst >> 23) & 1) == 1
    d.force_user    = ((inst >> 22) & 1) == 1
    d.write_back    = ((inst >> 21) & 1) == 1
    d.load          = ((inst >> 20) & 1) == 1
    d.rn            = (inst >> 16) & 0xF
    d.register_list = inst & 0xFFFF
    return d
end

local function decode_branch(d, inst)
    local is_link = ((inst >> 24) & 1) == 1
    local offset  = inst & 0x00FFFFFF
    -- Sign-extend from 24 bits
    if (offset >> 23) ~= 0 then
        offset = offset | 0xFF000000
    end
    -- Treat as signed (Lua integers are 64-bit signed)
    if offset >= 0x80000000 then
        offset = offset - 0x100000000
    end
    d.link          = is_link
    d.branch_offset = offset * 4
    return d
end

function M.decode(instruction)
    local d = {
        raw         = instruction,
        condition   = (instruction >> 28) & 0xF,
        type        = M.INST_UNDEFINED,
        -- data processing defaults
        opcode      = 0, s = false, rn = 0, rd = 0,
        immediate   = false, imm8 = 0, rotate = 0,
        rm = 0, shift_type = 0, shift_by_reg = false, shift_imm = 0, rs = 0,
        -- load/store defaults
        load = false, byte = false, pre_index = false, up = false,
        write_back = false, offset12 = 0,
        -- block transfer defaults
        register_list = 0, force_user = false,
        -- branch defaults
        link = false, branch_offset = 0,
        -- SWI
        swi_comment = 0,
    }

    local bits2726 = (instruction >> 26) & 0x3
    local bit25    = (instruction >> 25) & 0x1

    if bits2726 == 0 then
        d.type = M.INST_DATA_PROCESSING
        return decode_data_processing(d, instruction)
    elseif bits2726 == 1 then
        d.type = M.INST_LOAD_STORE
        return decode_load_store(d, instruction)
    elseif bits2726 == 2 and bit25 == 0 then
        d.type = M.INST_BLOCK_TRANSFER
        return decode_block_transfer(d, instruction)
    elseif bits2726 == 2 and bit25 == 1 then
        d.type = M.INST_BRANCH
        return decode_branch(d, instruction)
    elseif bits2726 == 3 then
        if ((instruction >> 24) & 0xF) == 0xF then
            d.type        = M.INST_SWI
            d.swi_comment = instruction & 0x00FFFFFF
        else
            d.type = M.INST_COPROCESSOR
        end
    end
    return d
end

-- =========================================================================
-- Disassembly
-- =========================================================================

local function disasm_reg_list(list)
    local regs = {}
    for i = 0, 15 do
        if ((list >> i) & 1) == 1 then
            if i == 15 then table.insert(regs, "PC")
            elseif i == 14 then table.insert(regs, "LR")
            elseif i == 13 then table.insert(regs, "SP")
            else table.insert(regs, "R" .. i) end
        end
    end
    return table.concat(regs, ", ")
end

local function disasm_operand2(d)
    if d.immediate then
        local val, _ = M.decode_immediate(d.imm8, d.rotate)
        return "#" .. val
    else
        if not d.shift_by_reg and d.shift_imm == 0 and d.shift_type == M.SHIFT_LSL then
            return "R" .. d.rm
        elseif d.shift_by_reg then
            return "R" .. d.rm .. ", " .. shift_string(d.shift_type) .. " R" .. d.rs
        else
            local amount = d.shift_imm
            local is_rrx = false
            if (d.shift_type == M.SHIFT_LSR or d.shift_type == M.SHIFT_ASR) and amount == 0 then
                amount = 32
            elseif d.shift_type == M.SHIFT_ROR and amount == 0 then
                is_rrx = true
            end
            if is_rrx then
                return "R" .. d.rm .. ", RRX"
            else
                return "R" .. d.rm .. ", " .. shift_string(d.shift_type) .. " #" .. amount
            end
        end
    end
end

function M.disassemble(d)
    local cond = cond_string(d.condition)
    if d.type == M.INST_DATA_PROCESSING then
        local op  = op_string(d.opcode)
        local suf = (d.s and not test_op(d.opcode)) and "S" or ""
        local op2 = disasm_operand2(d)
        if d.opcode == M.OP_MOV or d.opcode == M.OP_MVN then
            return op .. cond .. suf .. " R" .. d.rd .. ", " .. op2
        elseif test_op(d.opcode) then
            return op .. cond .. " R" .. d.rn .. ", " .. op2
        else
            return op .. cond .. suf .. " R" .. d.rd .. ", R" .. d.rn .. ", " .. op2
        end
    elseif d.type == M.INST_LOAD_STORE then
        local op    = d.load and "LDR" or "STR"
        local bsuf  = d.byte and "B" or ""
        local sign  = d.up and "" or "-"
        local offset
        if d.immediate then
            if d.shift_imm ~= 0 then
                offset = "R" .. d.rm .. ", " .. shift_string(d.shift_type) .. " #" .. d.shift_imm
            else
                offset = "R" .. d.rm
            end
        else
            offset = "#" .. d.offset12
        end
        if d.pre_index then
            local wb = d.write_back and "!" or ""
            return op .. cond .. bsuf .. " R" .. d.rd .. ", [R" .. d.rn .. ", " .. sign .. offset .. "]" .. wb
        else
            return op .. cond .. bsuf .. " R" .. d.rd .. ", [R" .. d.rn .. "], " .. sign .. offset
        end
    elseif d.type == M.INST_BLOCK_TRANSFER then
        local op = d.load and "LDM" or "STM"
        local bt_mode
        if not d.pre_index and d.up then bt_mode = "IA"
        elseif d.pre_index and d.up then bt_mode = "IB"
        elseif not d.pre_index and not d.up then bt_mode = "DA"
        else bt_mode = "DB" end
        local wb = d.write_back and "!" or ""
        return op .. cond .. bt_mode .. " R" .. d.rn .. wb .. ", {" .. disasm_reg_list(d.register_list) .. "}"
    elseif d.type == M.INST_BRANCH then
        local op = d.link and "BL" or "B"
        return op .. cond .. " #" .. d.branch_offset
    elseif d.type == M.INST_SWI then
        if d.swi_comment == M.HALT_SWI then
            return "HLT" .. cond
        else
            return string.format("SWI%s #0x%X", cond, d.swi_comment)
        end
    elseif d.type == M.INST_COPROCESSOR then
        return "CDP" .. cond .. " (coprocessor)"
    else
        return string.format("UND%s #0x%08X", cond, d.raw)
    end
end

-- =========================================================================
-- Execution — Helper
-- =========================================================================

-- Reads a register for use during execution.
-- R15 appears as PC+8 during execution (3-stage ARM1 pipeline):
-- step() has already advanced PC by 4, so we add 4 more.
local function read_reg_exec(cpu, n)
    if n == 15 then
        return (cpu.regs[15] + 4) & M.MASK32
    else
        return M.read_register(cpu, n)
    end
end

-- =========================================================================
-- Data Processing Execution
-- =========================================================================

local function exec_data_processing(cpu, d)
    local a = (d.opcode ~= M.OP_MOV and d.opcode ~= M.OP_MVN)
               and read_reg_exec(cpu, d.rn) or 0
    local flags = M.get_flags(cpu)
    local b, shifter_carry

    if d.immediate then
        b, shifter_carry = M.decode_immediate(d.imm8, d.rotate)
        -- If no rotation, carry is unchanged
        if d.rotate == 0 then shifter_carry = flags.c end
    else
        local rm_val = read_reg_exec(cpu, d.rm)
        local shift_amount
        if d.shift_by_reg then
            shift_amount = read_reg_exec(cpu, d.rs) & 0xFF
        else
            shift_amount = d.shift_imm
        end
        b, shifter_carry = M.barrel_shift(rm_val, d.shift_type, shift_amount, flags.c, d.shift_by_reg)
    end

    local alu = M.alu_execute(d.opcode, a, b, flags.c, shifter_carry, flags.v)

    -- Write result to Rd (unless test-only)
    if alu.write_result then
        if d.rd == 15 then
            if d.s then
                -- MOVS PC, x — restore entire R15 (mode switch)
                cpu.regs[15] = alu.result & M.MASK32
            else
                M.set_pc(cpu, alu.result & M.PC_MASK)
            end
        else
            M.write_register(cpu, d.rd, alu.result)
        end
    end

    -- Update flags if S bit set and Rd != R15
    if d.s and d.rd ~= 15 then
        M.set_flags(cpu, alu.n, alu.z, alu.c, alu.v)
    end
    -- Test ops always update flags
    if test_op(d.opcode) then
        M.set_flags(cpu, alu.n, alu.z, alu.c, alu.v)
    end
end

-- =========================================================================
-- Load/Store Execution
-- =========================================================================

local function exec_load_store(cpu, d)
    local base = read_reg_exec(cpu, d.rn)
    local offset

    if d.immediate then
        -- In load/store, "immediate" bit means register offset (confusing naming)
        local rm_val = read_reg_exec(cpu, d.rm)
        if d.shift_imm ~= 0 then
            offset, _ = M.barrel_shift(rm_val, d.shift_type, d.shift_imm, M.get_flags(cpu).c, false)
        else
            offset = rm_val
        end
    else
        offset = d.offset12
    end

    local addr
    if d.up then
        addr = (base + offset) & M.MASK32
    else
        addr = (base - offset) & M.MASK32
    end

    local transfer_addr = d.pre_index and addr or base

    local reads, writes = {}, {}

    if d.load then
        local value
        if d.byte then
            value = M.read_byte(cpu, transfer_addr)
        else
            value = M.read_word(cpu, transfer_addr)
            -- ARM1 quirk: unaligned word loads rotate the data
            local rotation = (transfer_addr & 3) * 8
            if rotation ~= 0 then
                value = ((value >> rotation) | (value << (32 - rotation))) & M.MASK32
            end
        end
        table.insert(reads, {address = transfer_addr, value = value})
        if d.rd == 15 then
            cpu.regs[15] = value & M.MASK32
        else
            M.write_register(cpu, d.rd, value)
        end
    else
        local value = read_reg_exec(cpu, d.rd)
        if d.byte then
            M.write_byte(cpu, transfer_addr, value & 0xFF)
        else
            M.write_word(cpu, transfer_addr, value)
        end
        table.insert(writes, {address = transfer_addr, value = value})
    end

    -- Write-back
    if d.write_back or not d.pre_index then
        if d.rn ~= 15 then
            M.write_register(cpu, d.rn, addr)
        end
    end

    return reads, writes
end

-- =========================================================================
-- Block Transfer Execution
-- =========================================================================

local function exec_block_transfer(cpu, d)
    local base = M.read_register(cpu, d.rn)
    local list = d.register_list
    local reads, writes = {}, {}

    -- Count registers in list
    local count = 0
    for i = 0, 15 do
        if ((list >> i) & 1) == 1 then count = count + 1 end
    end
    if count == 0 then return reads, writes end

    -- Calculate start address
    local start_addr
    if not d.pre_index and d.up then
        start_addr = base                         -- IA
    elseif d.pre_index and d.up then
        start_addr = (base + 4) & M.MASK32        -- IB
    elseif not d.pre_index and not d.up then
        start_addr = (base - count * 4 + 4) & M.MASK32  -- DA
    else
        start_addr = (base - count * 4) & M.MASK32      -- DB
    end

    local addr = start_addr
    for i = 0, 15 do
        if ((list >> i) & 1) == 1 then
            if d.load then
                local value = M.read_word(cpu, addr)
                table.insert(reads, {address = addr, value = value})
                if i == 15 then
                    cpu.regs[15] = value & M.MASK32
                else
                    M.write_register(cpu, i, value)
                end
            else
                local value
                if i == 15 then
                    value = (cpu.regs[15] + 4) & M.MASK32
                else
                    value = M.read_register(cpu, i)
                end
                M.write_word(cpu, addr, value)
                table.insert(writes, {address = addr, value = value})
            end
            addr = (addr + 4) & M.MASK32
        end
    end

    -- Write-back
    if d.write_back then
        local new_base
        if d.up then
            new_base = (base + count * 4) & M.MASK32
        else
            new_base = (base - count * 4) & M.MASK32
        end
        M.write_register(cpu, d.rn, new_base)
    end

    return reads, writes
end

-- =========================================================================
-- Branch Execution
-- =========================================================================

local function exec_branch(cpu, d)
    -- PC has already been advanced by 4 in step(); branch is relative to PC+8
    local branch_base = (M.get_pc(cpu) + 4) & M.MASK32

    if d.link then
        -- BL: save return address (full R15 with flags) in R14
        M.write_register(cpu, 14, cpu.regs[15])
    end

    local target = (branch_base + d.branch_offset) & M.MASK32
    M.set_pc(cpu, target & M.PC_MASK)
end

-- =========================================================================
-- SWI Execution
-- =========================================================================

local function exec_swi(cpu, d)
    if d.swi_comment == M.HALT_SWI then
        cpu.halted = true
        return
    end
    -- Enter Supervisor mode
    local r15_val = cpu.regs[15]
    cpu.regs[25] = r15_val  -- R13_svc
    cpu.regs[26] = r15_val  -- R14_svc
    -- Set SVC mode, disable IRQs
    local r15 = cpu.regs[15]
    r15 = (r15 & (~M.MODE_MASK & M.MASK32)) | M.MODE_SVC
    r15 = r15 | M.FLAG_I
    cpu.regs[15] = r15 & M.MASK32
    M.set_pc(cpu, 0x08)
end

-- =========================================================================
-- Trap Undefined
-- =========================================================================

local function trap_undefined(cpu)
    local r15_val = cpu.regs[15]
    cpu.regs[26] = r15_val  -- R14_svc
    local r15 = cpu.regs[15]
    r15 = (r15 & (~M.MODE_MASK & M.MASK32)) | M.MODE_SVC
    r15 = r15 | M.FLAG_I
    cpu.regs[15] = r15 & M.MASK32
    M.set_pc(cpu, 0x04)
end

-- =========================================================================
-- Step
-- =========================================================================
--
-- Executes one instruction. Returns a trace table describing what happened.

function M.step(cpu)
    if cpu.halted then
        return {halted = true}
    end

    local current_pc  = M.get_pc(cpu)
    local flags_before = M.get_flags(cpu)

    -- FETCH
    local instruction = M.read_word(cpu, current_pc)

    -- DECODE
    local d = M.decode(instruction)

    -- EVALUATE CONDITION
    local cond_met = M.evaluate_condition(d.condition, flags_before)

    -- ADVANCE PC
    M.set_pc(cpu, (current_pc + 4) & M.PC_MASK)

    local reads, writes = {}, {}

    -- EXECUTE (if condition met)
    if cond_met then
        if d.type == M.INST_DATA_PROCESSING then
            exec_data_processing(cpu, d)
        elseif d.type == M.INST_LOAD_STORE then
            reads, writes = exec_load_store(cpu, d)
        elseif d.type == M.INST_BLOCK_TRANSFER then
            reads, writes = exec_block_transfer(cpu, d)
        elseif d.type == M.INST_BRANCH then
            exec_branch(cpu, d)
        elseif d.type == M.INST_SWI then
            exec_swi(cpu, d)
        else
            trap_undefined(cpu)
        end
    end

    return {
        address       = current_pc,
        raw           = instruction,
        mnemonic      = M.disassemble(d),
        condition     = cond_string(d.condition),
        condition_met = cond_met,
        memory_reads  = reads,
        memory_writes = writes,
    }
end

-- =========================================================================
-- Run
-- =========================================================================

-- Executes instructions until halted or max_steps reached.
-- Returns a list of trace tables.
function M.run(cpu, max_steps)
    local traces = {}
    for _ = 1, max_steps do
        if cpu.halted then break end
        local trace = M.step(cpu)
        table.insert(traces, trace)
        if cpu.halted then break end
    end
    return traces
end

-- =========================================================================
-- Encoding Helpers
-- =========================================================================
--
-- These functions build ARM instruction words for use in test programs.

function M.encode_data_processing(condition, opcode, s, rn, rd, operand2)
    return ((condition << 28) | operand2 | (opcode << 21) | (s << 20)
            | (rn << 16) | (rd << 12)) & M.MASK32
end

-- MOV Rd, #imm8 — Rd = imm8
function M.encode_mov_imm(condition, rd, imm8)
    return M.encode_data_processing(condition, M.OP_MOV, 0, 0, rd, (1 << 25) | imm8)
end

-- Data processing with a register operand: OP Rd, Rn, Rm
function M.encode_alu_reg(condition, opcode, s, rd, rn, rm)
    return M.encode_data_processing(condition, opcode, s, rn, rd, rm)
end

-- Branch or Branch-with-Link. offset is a byte count (signed).
function M.encode_branch(condition, link, offset)
    local inst = (condition << 28) | 0x0A000000
    if link then inst = inst | 0x01000000 end
    local encoded = (offset // 4) & 0x00FFFFFF
    return (inst | encoded) & M.MASK32
end

-- Our pseudo-halt (SWI 0x123456)
function M.encode_halt()
    return ((M.COND_AL << 28) | 0x0F000000 | M.HALT_SWI) & M.MASK32
end

-- LDR Rd, [Rn, #offset]
function M.encode_ldr(condition, rd, rn, offset, pre_index)
    local inst = (condition << 28) | 0x04100000
    inst = inst | (rd << 12) | (rn << 16)
    if pre_index then inst = inst | (1 << 24) end
    if offset >= 0 then
        inst = inst | (1 << 23) | (offset & 0xFFF)
    else
        inst = inst | ((-offset) & 0xFFF)
    end
    return inst & M.MASK32
end

-- STR Rd, [Rn, #offset]
function M.encode_str(condition, rd, rn, offset, pre_index)
    local inst = (condition << 28) | 0x04000000
    inst = inst | (rd << 12) | (rn << 16)
    if pre_index then inst = inst | (1 << 24) end
    if offset >= 0 then
        inst = inst | (1 << 23) | (offset & 0xFFF)
    else
        inst = inst | ((-offset) & 0xFFF)
    end
    return inst & M.MASK32
end

-- LDMIA/LDMIB/LDMDA/LDMDB Rn{!}, {register_list}
function M.encode_ldm(condition, rn, reg_list, write_back, bt_mode)
    local inst = (condition << 28) | 0x08100000
    inst = inst | (rn << 16) | reg_list
    if write_back then inst = inst | (1 << 21) end
    if bt_mode == "IA" then
        inst = inst | (1 << 23)
    elseif bt_mode == "IB" then
        inst = inst | (1 << 24) | (1 << 23)
    elseif bt_mode == "DA" then
        -- no extra bits
    elseif bt_mode == "DB" then
        inst = inst | (1 << 24)
    end
    return inst & M.MASK32
end

-- STMIA/STMIB/STMDA/STMDB Rn{!}, {register_list}
function M.encode_stm(condition, rn, reg_list, write_back, bt_mode)
    local inst = M.encode_ldm(condition, rn, reg_list, write_back, bt_mode)
    return (inst & ~(1 << 20)) & M.MASK32
end

return M

-- ==========================================================================
-- ARM1 Gate-Level Simulator — Lua Port
-- ==========================================================================
--
-- This module implements the ARM1 processor at the gate level: every
-- arithmetic and logic operation routes through actual logic gate function
-- calls from the logic_gates and arithmetic packages.
--
-- # What Makes This Different From the Behavioral Simulator?
--
-- The behavioral simulator executes instructions directly:
--   ADD R0, R1, R2  →  result = reg[1] + reg[2]
--
-- This gate-level simulator routes everything through gates:
--   ADD R0, R1, R2  →  a_bits = int_to_bits(reg[1])
--                    →  b_bits = int_to_bits(reg[2])
--                    →  sum_bits, carry = ripple_carry_adder(a_bits, b_bits, 0)
--                    →    each full_adder: XOR(XOR(a,b),cin), AND(a,b), AND(xor,cin), OR(...)
--                    →  result = bits_to_int(sum_bits)
--
-- Every ADD leaves ~200 gate calls. Every SUB does NOT on 32 bits (32 calls),
-- then ripple_carry_adder (~160 calls).
--
-- # Bit Arrays
--
-- Bits are stored LSB-first in 1-indexed Lua arrays:
--   bit array[1] = LSB (bit 0, weight 2^0)
--   bit array[32] = MSB (bit 31, weight 2^31)
--
-- # Gate-Level vs Behavioral
--
-- This simulator inherits all instruction decoding, memory, and control
-- flow from arm1_simulator. It overrides the barrel shifter and ALU
-- to use gate-level primitives.

local lg    = require("coding_adventures.logic_gates")
local adder = require("coding_adventures.arithmetic.adder")
local ARM1  = require("coding_adventures.arm1_simulator")

local M = {}

-- Delegate all constants from the behavioral simulator
M.MODE_USR = ARM1.MODE_USR
M.MODE_FIQ = ARM1.MODE_FIQ
M.MODE_IRQ = ARM1.MODE_IRQ
M.MODE_SVC = ARM1.MODE_SVC
M.COND_EQ  = ARM1.COND_EQ
M.COND_NE  = ARM1.COND_NE
M.COND_CS  = ARM1.COND_CS
M.COND_CC  = ARM1.COND_CC
M.COND_MI  = ARM1.COND_MI
M.COND_PL  = ARM1.COND_PL
M.COND_VS  = ARM1.COND_VS
M.COND_VC  = ARM1.COND_VC
M.COND_HI  = ARM1.COND_HI
M.COND_LS  = ARM1.COND_LS
M.COND_GE  = ARM1.COND_GE
M.COND_LT  = ARM1.COND_LT
M.COND_GT  = ARM1.COND_GT
M.COND_LE  = ARM1.COND_LE
M.COND_AL  = ARM1.COND_AL
M.COND_NV  = ARM1.COND_NV
M.OP_AND   = ARM1.OP_AND
M.OP_EOR   = ARM1.OP_EOR
M.OP_SUB   = ARM1.OP_SUB
M.OP_RSB   = ARM1.OP_RSB
M.OP_ADD   = ARM1.OP_ADD
M.OP_ADC   = ARM1.OP_ADC
M.OP_SBC   = ARM1.OP_SBC
M.OP_RSC   = ARM1.OP_RSC
M.OP_TST   = ARM1.OP_TST
M.OP_TEQ   = ARM1.OP_TEQ
M.OP_CMP   = ARM1.OP_CMP
M.OP_CMN   = ARM1.OP_CMN
M.OP_ORR   = ARM1.OP_ORR
M.OP_MOV   = ARM1.OP_MOV
M.OP_BIC   = ARM1.OP_BIC
M.OP_MVN   = ARM1.OP_MVN
M.SHIFT_LSL = ARM1.SHIFT_LSL
M.SHIFT_LSR = ARM1.SHIFT_LSR
M.SHIFT_ASR = ARM1.SHIFT_ASR
M.SHIFT_ROR = ARM1.SHIFT_ROR
M.FLAG_N    = ARM1.FLAG_N
M.FLAG_Z    = ARM1.FLAG_Z
M.FLAG_C    = ARM1.FLAG_C
M.FLAG_V    = ARM1.FLAG_V
M.FLAG_I    = ARM1.FLAG_I
M.FLAG_F    = ARM1.FLAG_F
M.PC_MASK   = ARM1.PC_MASK
M.MODE_MASK = ARM1.MODE_MASK
M.MASK32    = ARM1.MASK32
M.HALT_SWI  = ARM1.HALT_SWI

-- Delegate encoding helpers
M.encode_data_processing = function(...) return ARM1.encode_data_processing(...) end
M.encode_mov_imm         = function(...) return ARM1.encode_mov_imm(...) end
M.encode_alu_reg         = function(...) return ARM1.encode_alu_reg(...) end
M.encode_branch          = function(...) return ARM1.encode_branch(...) end
M.encode_halt            = function(...) return ARM1.encode_halt(...) end
M.encode_ldr             = function(...) return ARM1.encode_ldr(...) end
M.encode_str             = function(...) return ARM1.encode_str(...) end
M.encode_ldm             = function(...) return ARM1.encode_ldm(...) end
M.encode_stm             = function(...) return ARM1.encode_stm(...) end

-- =========================================================================
-- Bit Conversion Helpers
-- =========================================================================
--
-- Bridge between the integer register API and gate-level bit arrays.
--
-- int_to_bits(v, w): convert integer v to w-element LSB-first array
--   bit[1] = LSB (bit 0), bit[w] = MSB (bit w-1)
--
-- bits_to_int(bits): convert LSB-first array back to integer

function M.int_to_bits(v, w)
    w = w or 32
    local bits = {}
    for i = 1, w do
        bits[i] = (v >> (i - 1)) & 1
    end
    return bits
end

function M.bits_to_int(bits)
    local v = 0
    for i = 1, #bits do
        if bits[i] ~= 0 then
            v = v | (1 << (i - 1))
        end
    end
    return v & ARM1.MASK32
end

-- =========================================================================
-- Mux2 (2-to-1 Multiplexer from gates)
-- =========================================================================
--
-- mux2(a, b, sel) = OR(AND(NOT(sel), a), AND(sel, b))
--
-- sel=0 → output = a
-- sel=1 → output = b
--
-- This is used in the gate-level barrel shifter to implement the 5-level
-- shift tree.

local function mux2(a, b, sel)
    return lg.OR(lg.AND(lg.NOT(sel), a), lg.AND(sel, b))
end

-- =========================================================================
-- Gate-Level Barrel Shifter
-- =========================================================================
--
-- The ARM1's barrel shifter is a 32×32 crossbar of pass transistors.
-- We model it as a 5-level Mux2 tree: each level shifts by 2^level bits.
--
-- Each level i (0..4) controls a shift of 2^i positions:
--   Level 0: shift by 1
--   Level 1: shift by 2
--   Level 2: shift by 4
--   Level 3: shift by 8
--   Level 4: shift by 16
--
-- bit arrays are 1-indexed, LSB-first (bit[1]=LSB, bit[32]=MSB)
--
-- Returns: result_bits (array), carry_out (0 or 1)

local function gate_lsl_by_n(bits, n)
    -- Shift left by exactly n positions (0 < n <= 32)
    -- Result[i] = bits[i-n] if i>n else 0
    local result = {}
    for i = 1, 32 do
        if i > n then
            result[i] = bits[i - n]
        else
            result[i] = 0
        end
    end
    return result
end

local function gate_lsr_by_n(bits, n)
    -- Shift right by exactly n positions (0 < n <= 32)
    -- Result[i] = bits[i+n] if i+n <= 32 else 0
    local result = {}
    for i = 1, 32 do
        if i + n <= 32 then
            result[i] = bits[i + n]
        else
            result[i] = 0
        end
    end
    return result
end

local function gate_asr_by_n(bits, n)
    -- Arithmetic shift right: fill with MSB (bit[32])
    local msb = bits[32]
    local result = {}
    for i = 1, 32 do
        if i + n <= 32 then
            result[i] = bits[i + n]
        else
            result[i] = msb
        end
    end
    return result
end

local function gate_ror_by_n(bits, n)
    -- Rotate right by n positions (n mod 32)
    --
    -- In LSB-first 1-indexed arrays, rotating the 32-bit integer right by n
    -- means each logical bit[k] moves to bit[k-n] (wrapping).
    -- In our array: result[i] = bits[src] where
    --   src = ((i + n - 1) mod 32) + 1
    --
    -- Verification for ROR by 1: bit[1] of result should come from bit[2]
    -- of input (LSB shifts out, MSB absorbs top bit).
    --   i=1, n=1: src = ((1+1-1) % 32)+1 = (1 % 32)+1 = 2  ✓
    --   i=32, n=1: src = ((32+1-1) % 32)+1 = (32 % 32)+1 = 0+1 = 1  ✓
    n = n & 31
    if n == 0 then return {table.unpack(bits)} end
    local result = {}
    for i = 1, 32 do
        local src = ((i + n - 1) % 32) + 1
        result[i] = bits[src]
    end
    return result
end

-- Gate-level LSL using 5-level Mux2 tree
-- Each level controls a shift by 2^level positions using 32 mux2 calls
local function gate_lsl_tree(bits, amount_bits)
    -- amount_bits[1..5] = shift amount bits (LSB-first, bit 1=shift by 1)
    local current = {table.unpack(bits)}

    for level = 0, 4 do
        local shift_amt = 1 << level
        local shifted = gate_lsl_by_n(current, shift_amt)
        local sel = amount_bits[level + 1]  -- 1-indexed
        local next_bits = {}
        for i = 1, 32 do
            next_bits[i] = mux2(current[i], shifted[i], sel)
        end
        current = next_bits
    end

    return current
end

local function gate_lsr_tree(bits, amount_bits)
    local current = {table.unpack(bits)}
    for level = 0, 4 do
        local shift_amt = 1 << level
        local shifted = gate_lsr_by_n(current, shift_amt)
        local sel = amount_bits[level + 1]
        local next_bits = {}
        for i = 1, 32 do
            next_bits[i] = mux2(current[i], shifted[i], sel)
        end
        current = next_bits
    end
    return current
end

local function gate_asr_tree(bits, amount_bits)
    local current = {table.unpack(bits)}
    for level = 0, 4 do
        local shift_amt = 1 << level
        local shifted = gate_asr_by_n(current, shift_amt)
        local sel = amount_bits[level + 1]
        local next_bits = {}
        for i = 1, 32 do
            next_bits[i] = mux2(current[i], shifted[i], sel)
        end
        current = next_bits
    end
    return current
end

local function gate_ror_tree(bits, amount_bits)
    local current = {table.unpack(bits)}
    for level = 0, 4 do
        local shift_amt = 1 << level
        local shifted = gate_ror_by_n(current, shift_amt)
        local sel = amount_bits[level + 1]
        local next_bits = {}
        for i = 1, 32 do
            next_bits[i] = mux2(current[i], shifted[i], sel)
        end
        current = next_bits
    end
    return current
end

-- Gate-level barrel shift.
-- value_bits: 32-element LSB-first array
-- shift_type: 0=LSL, 1=LSR, 2=ASR, 3=ROR
-- amount:     shift amount (integer 0-31, or 32 for special cases)
-- carry_in:   0 or 1 (used for RRX and as default carry)
-- by_register: true if amount from register
--
-- Returns: result_bits (array), carry_out (0 or 1)
function M.gate_barrel_shift(value_bits, shift_type, amount, carry_in, by_register)
    -- In Lua, 0 is truthy, so use explicit boolean test to convert to 0/1.
    carry_in = (carry_in == true or carry_in == 1) and 1 or 0

    -- Register shift by 0: no change
    if amount == 0 and by_register then
        return {table.unpack(value_bits)}, carry_in
    end

    local carry_out = carry_in

    if shift_type == ARM1.SHIFT_LSL then
        if amount == 0 then
            return {table.unpack(value_bits)}, carry_in
        elseif amount >= 32 then
            if amount == 32 then
                carry_out = value_bits[1]  -- LSB of original
            else
                carry_out = 0
            end
            local result = {}
            for i = 1, 32 do result[i] = 0 end
            return result, carry_out
        end
        -- carry = bit (32 - amount + 1) in 1-indexed LSB-first = bit at position 32-amount
        carry_out = value_bits[32 - amount + 1]
        -- Use 5-bit Mux2 tree
        local amt_bits = M.int_to_bits(amount, 5)
        local result = gate_lsl_tree(value_bits, amt_bits)
        return result, carry_out

    elseif shift_type == ARM1.SHIFT_LSR then
        if amount == 0 and not by_register then
            -- Immediate LSR #0 = LSR #32
            carry_out = value_bits[32]  -- MSB
            local result = {}
            for i = 1, 32 do result[i] = 0 end
            return result, carry_out
        elseif amount == 0 then
            return {table.unpack(value_bits)}, carry_in
        elseif amount >= 32 then
            carry_out = (amount == 32) and value_bits[32] or 0
            local result = {}
            for i = 1, 32 do result[i] = 0 end
            return result, carry_out
        end
        -- carry = bit (amount) in 1-indexed LSB-first
        carry_out = value_bits[amount]
        local amt_bits = M.int_to_bits(amount, 5)
        local result = gate_lsr_tree(value_bits, amt_bits)
        return result, carry_out

    elseif shift_type == ARM1.SHIFT_ASR then
        if amount == 0 and not by_register then
            -- Immediate ASR #0 = ASR #32
            local msb = value_bits[32]
            carry_out = msb
            local result = {}
            for i = 1, 32 do result[i] = msb end
            return result, carry_out
        elseif amount == 0 then
            return {table.unpack(value_bits)}, carry_in
        elseif amount >= 32 then
            local msb = value_bits[32]
            carry_out = msb
            local result = {}
            for i = 1, 32 do result[i] = msb end
            return result, carry_out
        end
        -- carry = bit (amount) in 1-indexed LSB-first
        carry_out = value_bits[amount]
        local amt_bits = M.int_to_bits(amount, 5)
        local result = gate_asr_tree(value_bits, amt_bits)
        return result, carry_out

    elseif shift_type == ARM1.SHIFT_ROR then
        if amount == 0 and not by_register then
            -- RRX: rotate right through carry
            carry_out = value_bits[1]  -- LSB shifted out
            local result = {}
            result[32] = carry_in      -- carry_in becomes MSB
            for i = 1, 31 do
                result[i] = value_bits[i + 1]
            end
            return result, carry_out
        elseif amount == 0 then
            return {table.unpack(value_bits)}, carry_in
        end
        local eff = amount & 31
        if eff == 0 then
            -- Multiple of 32: value unchanged, carry = MSB
            carry_out = value_bits[32]
            return {table.unpack(value_bits)}, carry_out
        end
        local amt_bits = M.int_to_bits(eff, 5)
        local result = gate_ror_tree(value_bits, amt_bits)
        carry_out = result[32]  -- MSB after rotation
        return result, carry_out
    end

    return {table.unpack(value_bits)}, carry_in
end

-- Gate-level rotated immediate decode.
-- Returns: result_bits, carry_out
function M.gate_decode_immediate(imm8, rotate)
    local value, carry = ARM1.decode_immediate(imm8, rotate)
    local c = carry and 1 or 0
    return M.int_to_bits(value, 32), c
end

-- =========================================================================
-- Gate-Level Condition Evaluation
-- =========================================================================
--
-- Each condition is implemented using AND/OR/XOR/NOT gate calls.
-- This mirrors the ARM1's condition evaluation hardware.
--
-- N, Z, C, V are 0/1 integers extracted from the flags.

local function eval_cond_gates(cond, flags, cpu)
    local n = flags.n and 1 or 0
    local z = flags.z and 1 or 0
    local c = flags.c and 1 or 0
    local v = flags.v and 1 or 0
    cpu.gate_ops = cpu.gate_ops + 1  -- minimal counting

    if cond == ARM1.COND_EQ then return z == 1
    elseif cond == ARM1.COND_NE then return lg.NOT(z) == 1
    elseif cond == ARM1.COND_CS then return c == 1
    elseif cond == ARM1.COND_CC then return lg.NOT(c) == 1
    elseif cond == ARM1.COND_MI then return n == 1
    elseif cond == ARM1.COND_PL then return lg.NOT(n) == 1
    elseif cond == ARM1.COND_VS then return v == 1
    elseif cond == ARM1.COND_VC then return lg.NOT(v) == 1
    elseif cond == ARM1.COND_HI then return lg.AND(c, lg.NOT(z)) == 1
    elseif cond == ARM1.COND_LS then return lg.OR(lg.NOT(c), z) == 1
    elseif cond == ARM1.COND_GE then return lg.XNOR(n, v) == 1
    elseif cond == ARM1.COND_LT then return lg.XOR(n, v) == 1
    elseif cond == ARM1.COND_GT then return lg.AND(lg.NOT(z), lg.XNOR(n, v)) == 1
    elseif cond == ARM1.COND_LE then return lg.OR(z, lg.XOR(n, v)) == 1
    elseif cond == ARM1.COND_AL then return true
    elseif cond == ARM1.COND_NV then return false
    else return false
    end
end

-- =========================================================================
-- Gate-Level ALU
-- =========================================================================
--
-- All 16 ARM1 ALU operations implemented through gate calls.
--
-- Logical ops: AND/OR/XOR/NOT applied bit-by-bit (32 gate calls each)
-- Arithmetic: ripple_carry_adder (~160 gate calls for 32-bit add)
--
-- Parameters (all as integers, not bit arrays):
--   opcode, a, b: integer operands
--   carry_in: boolean
--   shifter_carry: 0 or 1
--   old_v: 0 or 1
--
-- Returns a table: {result, n, z, c, v, write_result}

function M.gate_alu_execute(opcode, a, b, carry_in, shifter_carry, old_v)
    local a_bits = M.int_to_bits(a, 32)
    local b_bits = M.int_to_bits(b, 32)
    local c_in   = (carry_in == 1 or carry_in == true) and 1 or 0
    local sc     = (type(shifter_carry) == "boolean") and (shifter_carry and 1 or 0) or shifter_carry
    local ov     = (type(old_v) == "boolean") and (old_v and 1 or 0) or old_v

    local result_bits = {}
    local carry, overflow = sc, ov
    local write_result = not (opcode >= ARM1.OP_TST and opcode <= ARM1.OP_CMN)

    if opcode == ARM1.OP_AND or opcode == ARM1.OP_TST then
        for i = 1, 32 do result_bits[i] = lg.AND(a_bits[i], b_bits[i]) end
        carry = sc

    elseif opcode == ARM1.OP_EOR or opcode == ARM1.OP_TEQ then
        for i = 1, 32 do result_bits[i] = lg.XOR(a_bits[i], b_bits[i]) end
        carry = sc

    elseif opcode == ARM1.OP_ORR then
        for i = 1, 32 do result_bits[i] = lg.OR(a_bits[i], b_bits[i]) end
        carry = sc

    elseif opcode == ARM1.OP_MOV then
        result_bits = {table.unpack(b_bits)}
        carry = sc

    elseif opcode == ARM1.OP_BIC then
        for i = 1, 32 do result_bits[i] = lg.AND(a_bits[i], lg.NOT(b_bits[i])) end
        carry = sc

    elseif opcode == ARM1.OP_MVN then
        for i = 1, 32 do result_bits[i] = lg.NOT(b_bits[i]) end
        carry = sc

    elseif opcode == ARM1.OP_ADD or opcode == ARM1.OP_CMN then
        local sum_bits, cout = adder.ripple_carry_adder(a_bits, b_bits, 0)
        result_bits = sum_bits
        carry = (cout == 1 or cout == true) and 1 or 0
        -- Overflow: inputs same sign but result differs
        local sa = a_bits[32]; local sb = b_bits[32]; local sr = sum_bits[32]
        overflow = lg.AND(lg.XNOR(sa, sb), lg.XOR(sa, sr))

    elseif opcode == ARM1.OP_ADC then
        local sum_bits, cout = adder.ripple_carry_adder(a_bits, b_bits, c_in)
        result_bits = sum_bits
        carry = (cout == 1 or cout == true) and 1 or 0
        local sa = a_bits[32]; local sb = b_bits[32]; local sr = sum_bits[32]
        overflow = lg.AND(lg.XNOR(sa, sb), lg.XOR(sa, sr))

    elseif opcode == ARM1.OP_SUB or opcode == ARM1.OP_CMP then
        -- SUB a, b = a + NOT(b) + 1
        local nb = {}
        for i = 1, 32 do nb[i] = lg.NOT(b_bits[i]) end
        local sum_bits, cout = adder.ripple_carry_adder(a_bits, nb, 1)
        result_bits = sum_bits
        carry = (cout == 1 or cout == true) and 1 or 0
        local sa = a_bits[32]; local sb = nb[32]; local sr = sum_bits[32]
        overflow = lg.AND(lg.XNOR(sa, sb), lg.XOR(sa, sr))

    elseif opcode == ARM1.OP_SBC then
        local nb = {}
        for i = 1, 32 do nb[i] = lg.NOT(b_bits[i]) end
        local sum_bits, cout = adder.ripple_carry_adder(a_bits, nb, c_in)
        result_bits = sum_bits
        carry = (cout == 1 or cout == true) and 1 or 0
        local sa = a_bits[32]; local sb = nb[32]; local sr = sum_bits[32]
        overflow = lg.AND(lg.XNOR(sa, sb), lg.XOR(sa, sr))

    elseif opcode == ARM1.OP_RSB then
        -- RSB b, a = b + NOT(a) + 1
        local na = {}
        for i = 1, 32 do na[i] = lg.NOT(a_bits[i]) end
        local sum_bits, cout = adder.ripple_carry_adder(b_bits, na, 1)
        result_bits = sum_bits
        carry = (cout == 1 or cout == true) and 1 or 0
        local sa = b_bits[32]; local sb = na[32]; local sr = sum_bits[32]
        overflow = lg.AND(lg.XNOR(sa, sb), lg.XOR(sa, sr))

    elseif opcode == ARM1.OP_RSC then
        local na = {}
        for i = 1, 32 do na[i] = lg.NOT(a_bits[i]) end
        local sum_bits, cout = adder.ripple_carry_adder(b_bits, na, c_in)
        result_bits = sum_bits
        carry = (cout == 1 or cout == true) and 1 or 0
        local sa = b_bits[32]; local sb = na[32]; local sr = sum_bits[32]
        overflow = lg.AND(lg.XNOR(sa, sb), lg.XOR(sa, sr))

    else
        result_bits = {table.unpack(a_bits)}
    end

    local result = M.bits_to_int(result_bits)
    return {
        result_bits  = result_bits,
        result       = result,
        n            = result_bits[32],
        z            = (result == 0) and 1 or 0,
        c            = carry,
        v            = overflow,
        write_result = write_result,
    }
end

-- =========================================================================
-- CPU Construction — Gate-Level Wrapper
-- =========================================================================
--
-- The gate-level CPU reuses all of arm1_simulator's infrastructure.
-- We add:
--   cpu.gate_ops — cumulative gate function call counter

function M.new(memory_size)
    local cpu = ARM1.new(memory_size)
    cpu.gate_ops = 0
    return cpu
end

function M.reset(cpu)
    ARM1.reset(cpu)
    cpu.gate_ops = 0
    return cpu
end

-- Delegate memory and register access to ARM1
M.read_register  = ARM1.read_register
M.write_register = ARM1.write_register
M.get_pc         = ARM1.get_pc
M.set_pc         = ARM1.set_pc
M.get_flags      = ARM1.get_flags
M.set_flags      = ARM1.set_flags
M.get_mode       = ARM1.get_mode
M.read_word      = ARM1.read_word
M.write_word     = ARM1.write_word
M.read_byte      = ARM1.read_byte
M.write_byte     = ARM1.write_byte
M.load_instructions = ARM1.load_instructions

-- =========================================================================
-- Gate-Level Step
-- =========================================================================
--
-- Overrides ARM1.step() to use gate-level condition evaluation, barrel
-- shifter, and ALU.

-- Reads a register for execution (R15 = PC+8 due to pipeline)
local function read_reg_exec(cpu, n)
    if n == 15 then
        return (cpu.regs[15] + 4) & ARM1.MASK32
    else
        return ARM1.read_register(cpu, n)
    end
end

local function exec_data_processing_gl(cpu, d)
    local a = (d.opcode ~= ARM1.OP_MOV and d.opcode ~= ARM1.OP_MVN)
               and read_reg_exec(cpu, d.rn) or 0
    local flags = ARM1.get_flags(cpu)
    local b_bits, shifter_carry

    if d.immediate then
        b_bits, shifter_carry = M.gate_decode_immediate(d.imm8, d.rotate)
        if d.rotate == 0 then shifter_carry = flags.c and 1 or 0 end
    else
        local rm_val = read_reg_exec(cpu, d.rm)
        local rm_bits = M.int_to_bits(rm_val, 32)
        local shift_amount
        if d.shift_by_reg then
            shift_amount = read_reg_exec(cpu, d.rs) & 0xFF
        else
            shift_amount = d.shift_imm
        end
        local c_bool = flags.c
        b_bits, shifter_carry = M.gate_barrel_shift(
            rm_bits, d.shift_type, shift_amount,
            c_bool, d.shift_by_reg
        )
    end

    local c_int = flags.c and 1 or 0
    local v_int = flags.v and 1 or 0
    local alu = M.gate_alu_execute(d.opcode, a, M.bits_to_int(b_bits), c_int, shifter_carry, v_int)

    -- Accumulate gate ops (approximate: 200 per data processing instruction)
    cpu.gate_ops = cpu.gate_ops + 200

    if alu.write_result then
        if d.rd == 15 then
            if d.s then
                cpu.regs[15] = alu.result & ARM1.MASK32
            else
                ARM1.set_pc(cpu, alu.result & ARM1.PC_MASK)
            end
        else
            ARM1.write_register(cpu, d.rd, alu.result)
        end
    end

    local n = alu.n == 1
    local z = alu.z == 1
    local c = alu.c == 1
    local v = alu.v == 1

    if d.s and d.rd ~= 15 then
        ARM1.set_flags(cpu, n, z, c, v)
    end
    if d.opcode >= ARM1.OP_TST and d.opcode <= ARM1.OP_CMN then
        ARM1.set_flags(cpu, n, z, c, v)
    end
end

function M.step(cpu)
    if cpu.halted then return {halted = true} end

    local current_pc    = ARM1.get_pc(cpu)
    local flags_before  = ARM1.get_flags(cpu)
    local instruction   = ARM1.read_word(cpu, current_pc)
    local d             = ARM1.decode(instruction)

    -- Gate-level condition evaluation
    local cond_met = eval_cond_gates(d.condition, flags_before, cpu)

    -- Advance PC
    ARM1.set_pc(cpu, (current_pc + 4) & ARM1.PC_MASK)

    local reads, writes = {}, {}

    if cond_met then
        if d.type == ARM1.INST_DATA_PROCESSING then
            exec_data_processing_gl(cpu, d)
        elseif d.type == ARM1.INST_LOAD_STORE then
            -- Use behavioral for memory ops (no gate level needed for address computation)
            reads, writes = ARM1.step and {} or {}
            -- Re-invoke behavioral load/store by temporarily delegating
            -- We'll call the behavioral step for these instruction types
            -- by resetting PC and running behavioral step (then restoring gate_ops)
            -- Actually: re-implement inline using ARM1's helpers
            local cpu_save_go = cpu.gate_ops
            ARM1.set_pc(cpu, current_pc)  -- reset PC so behavioral step refetches
            local btrace = ARM1.step(cpu)
            cpu.gate_ops = cpu_save_go + 50  -- approximate gate count for load/store
            reads  = btrace.memory_reads  or {}
            writes = btrace.memory_writes or {}
            -- Note: behavioral step already advanced PC again, but that's correct
            -- since it fetches the same instruction
            return {
                address       = current_pc,
                raw           = instruction,
                mnemonic      = ARM1.disassemble(d),
                condition_met = true,
                memory_reads  = reads,
                memory_writes = writes,
                gate_ops      = cpu.gate_ops,
            }
        elseif d.type == ARM1.INST_BLOCK_TRANSFER then
            local cpu_save_go = cpu.gate_ops
            ARM1.set_pc(cpu, current_pc)
            local btrace = ARM1.step(cpu)
            cpu.gate_ops = cpu_save_go + 100
            reads  = btrace.memory_reads  or {}
            writes = btrace.memory_writes or {}
            return {
                address       = current_pc,
                raw           = instruction,
                mnemonic      = ARM1.disassemble(d),
                condition_met = true,
                memory_reads  = reads,
                memory_writes = writes,
                gate_ops      = cpu.gate_ops,
            }
        elseif d.type == ARM1.INST_BRANCH then
            -- Branch: use behavioral (just PC arithmetic, no gates)
            local branch_base = (ARM1.get_pc(cpu) + 4) & ARM1.MASK32
            if d.link then
                ARM1.write_register(cpu, 14, cpu.regs[15])
            end
            local target = (branch_base + d.branch_offset) & ARM1.MASK32
            ARM1.set_pc(cpu, target & ARM1.PC_MASK)
            cpu.gate_ops = cpu.gate_ops + 4
        elseif d.type == ARM1.INST_SWI then
            if d.swi_comment == ARM1.HALT_SWI then
                cpu.halted = true
            else
                local r15_val = cpu.regs[15]
                cpu.regs[25] = r15_val
                cpu.regs[26] = r15_val
                local r15 = cpu.regs[15]
                r15 = (r15 & (~ARM1.MODE_MASK & ARM1.MASK32)) | ARM1.MODE_SVC
                r15 = r15 | ARM1.FLAG_I
                cpu.regs[15] = r15 & ARM1.MASK32
                ARM1.set_pc(cpu, 0x08)
            end
        else
            -- Undefined/coprocessor: trap
            cpu.regs[26] = cpu.regs[15]
            local r15 = cpu.regs[15]
            r15 = (r15 & (~ARM1.MODE_MASK & ARM1.MASK32)) | ARM1.MODE_SVC
            r15 = r15 | ARM1.FLAG_I
            cpu.regs[15] = r15 & ARM1.MASK32
            ARM1.set_pc(cpu, 0x04)
        end
    end

    return {
        address       = current_pc,
        raw           = instruction,
        mnemonic      = ARM1.disassemble(d),
        condition_met = cond_met,
        memory_reads  = reads,
        memory_writes = writes,
        gate_ops      = cpu.gate_ops,
    }
end

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

return M

-- alu — The Arithmetic Logic Unit: a CPU's calculator
--
-- # What Is an ALU?
--
-- An ALU is the part of a CPU that actually executes arithmetic and logic
-- commands. You give it two numbers (A and B) and a control signal (the
-- operation code). It routes those numbers through various circuits — like
-- our ripple carry adder for addition — and outputs the result alongside
-- helpful "status flags" that let the CPU make decisions.
--
-- For example, after a subtraction, the CPU might check the Zero flag to
-- see if two values were equal (if A - B == 0, then A == B). The branch
-- instructions in assembly language ("jump if zero", "jump if negative")
-- rely entirely on these flags.
--
-- # Status Flags
--
-- Real CPUs set four standard condition flags after every ALU operation:
--
--   * Zero (Z)     — Is every bit of the result 0?
--                    Used for equality checks: "if x == 0" or "if a == b".
--
--   * Carry (C)    — Did the unsigned addition overflow past the top bit?
--                    Used for multi-precision arithmetic and unsigned
--                    comparisons.
--
--   * Negative (N) — Is the most significant bit (MSB) of the result 1?
--                    In two's complement, MSB=1 means the number is negative.
--
--   * Overflow (V) — Did signed arithmetic produce an impossible result?
--                    Example: adding two large positive numbers and getting
--                    a negative result means we "ran out of bits."
--
-- # Supported Operations
--
--   "add" — Binary addition using the ripple carry adder
--   "sub" — Subtraction via two's complement: A - B = A + (~B + 1)
--   "and" — Bitwise AND across all bit positions
--   "or"  — Bitwise OR across all bit positions
--   "xor" — Bitwise XOR across all bit positions
--   "not" — Bitwise NOT of the A bus (B is ignored)

local lg = require("coding_adventures.logic_gates")
local adder = require("coding_adventures.arithmetic.adder")

local alu = {}

-- ========================================================================
-- Operation Constants
-- ========================================================================
--
-- These string constants serve as the "instruction set" for the ALU.
-- In a real CPU, these would be encoded as binary control signals on
-- physical wires. Here, we use descriptive strings for clarity.

alu.ADD = "add"
alu.SUB = "sub"
alu.AND = "and"
alu.OR  = "or"
alu.XOR = "xor"
alu.NOT = "not"

-- ========================================================================
-- ALUResult — The output of every ALU operation
-- ========================================================================
--
-- Every ALU operation returns not just the computed value, but also a set
-- of condition flags that describe properties of that value. This is how
-- CPUs implement conditional logic: execute an operation, then check the
-- flags to decide what to do next.
--
-- We use a metatable to give ALUResult a proper type identity and a
-- human-readable string representation for debugging.

local ALUResult = {}
ALUResult.__index = ALUResult

-- Create a new ALUResult.
--
-- @param value    Table of result bits (LSB at index 1)
-- @param zero     boolean — is every bit 0?
-- @param carry    boolean — did unsigned overflow occur?
-- @param negative boolean — is the MSB 1?
-- @param overflow boolean — did signed overflow occur?
-- @return ALUResult instance
function ALUResult.new(value, zero, carry, negative, overflow)
    local self = setmetatable({}, ALUResult)
    self.value = value
    self.zero = zero
    self.carry = carry
    self.negative = negative
    self.overflow = overflow
    return self
end

-- Human-readable display for debugging.
--
-- Shows the bit pattern (MSB first, the natural reading order) and
-- which flags are set. Example output:
--   ALUResult{1000 Z=false C=false N=true V=true}
function ALUResult:__tostring()
    -- Display bits MSB first (reverse of internal LSB-first order)
    local bits = {}
    for i = #self.value, 1, -1 do
        bits[#bits + 1] = tostring(self.value[i])
    end
    return string.format(
        "ALUResult{%s Z=%s C=%s N=%s V=%s}",
        table.concat(bits),
        tostring(self.zero),
        tostring(self.carry),
        tostring(self.negative),
        tostring(self.overflow)
    )
end

alu.ALUResult = ALUResult

-- ========================================================================
-- ALU — The main arithmetic logic unit
-- ========================================================================
--
-- An ALU instance is configured with a fixed bit width, just like a real
-- CPU has a fixed data bus width (8-bit, 16-bit, 32-bit, 64-bit). All
-- inputs must match this width, and all outputs will have this width.

local ALU = {}
ALU.__index = ALU

-- Create a new ALU with the specified bit width.
--
-- @param bit_width  Number of bits for the data buses (must be >= 1)
-- @return ALU instance
function ALU.new(bit_width)
    assert(bit_width >= 1, "bit_width must be at least 1")
    local self = setmetatable({}, ALU)
    self.bit_width = bit_width
    return self
end

alu.ALU = ALU

-- ========================================================================
-- Internal Helpers
-- ========================================================================

-- bitwise_op — Apply a two-input gate across parallel bit arrays.
--
-- This is how CPUs implement bitwise operations: the same gate is
-- physically replicated once per bit position, and all copies operate
-- simultaneously (in parallel). There is no carry propagation, so
-- bitwise operations are much faster than addition in real hardware.
--
-- @param a   Table of bits
-- @param b   Table of bits (same length as a)
-- @param op  A function(int, int) -> int (a logic gate)
-- @return    Table of result bits
local function bitwise_op(a, b, op)
    local result = {}
    for i = 1, #a do
        result[i] = op(a[i], b[i])
    end
    return result
end

-- twos_complement_negate — Convert a binary number to its negation.
--
-- # Two's Complement: The Clever Trick Behind Negative Numbers
--
-- How do computers represent negative numbers using only 0s and 1s?
-- They use a system called two's complement. To negate a number x:
--
--   Step 1: Flip every bit (NOT operation).
--   Step 2: Add 1.
--
-- Why does this work? Consider any number x and its bitwise complement
-- NOT(x). Together they form a number with all 1s:
--
--   x + NOT(x) = 1111...1
--
-- If we add 1 more, the all-1s value rolls over to all-0s (ignoring the
-- carry out of the top bit):
--
--   x + NOT(x) + 1 = 0000...0
--
-- Rearranging:
--
--   NOT(x) + 1 = -x
--
-- The beauty of two's complement is that the ALU can use the EXACT SAME
-- adder circuit for both addition and subtraction. To compute A - B, we
-- simply compute A + (-B) = A + NOT(B) + 1. No special subtraction
-- hardware needed!
--
-- @param bits  Table of bits representing the number to negate
-- @return negated  Table of bits representing -x
-- @return carry    Carry out from the addition
local function twos_complement_negate(bits)
    -- Step 1: Flip every bit
    local inverted = {}
    for i = 1, #bits do
        inverted[i] = lg.NOT(bits[i])
    end

    -- Step 2: Add 1 (the number 1 in binary is 0001, i.e., LSB = 1)
    local one = {}
    for i = 1, #bits do
        one[i] = 0
    end
    one[1] = 1

    return adder.ripple_carry_adder(inverted, one, 0)
end

-- ========================================================================
-- Execute — The main entry point for ALU operations
-- ========================================================================
--
-- This function is the "control unit" of the ALU. It reads the operation
-- code and routes the input buses A and B into the appropriate circuit.
-- After computing the result, it calculates the four condition flags.
--
-- In real hardware, this routing is done with multiplexers — circuits
-- that select one of several inputs based on a control signal. Here,
-- we use a Lua if/elseif chain for the same purpose.
--
-- @param op  Operation code (one of alu.ADD, alu.SUB, alu.AND, alu.OR,
--            alu.XOR, alu.NOT)
-- @param a   Table of bits for the A bus (length must equal bit_width)
-- @param b   Table of bits for the B bus (length must equal bit_width,
--            except for NOT where b is ignored)
-- @return    ALUResult with value and condition flags
function ALU:execute(op, a, b)
    assert(#a == self.bit_width, "a length must match bit_width")
    -- NOT only uses the A bus; all other operations require B to match width
    if op ~= alu.NOT then
        assert(#b == self.bit_width, "b length must match bit_width")
    end

    local value
    local carry_bit = 0

    -- ----------------------------------------------------------------
    -- Step 1: Compute the result based on the operation code
    -- ----------------------------------------------------------------

    if op == alu.ADD then
        -- Straight addition through the ripple carry adder.
        value, carry_bit = adder.ripple_carry_adder(a, b, 0)

    elseif op == alu.SUB then
        -- Subtraction is addition in disguise: A - B = A + (-B).
        -- We negate B using two's complement, then add.
        local neg_b, _ = twos_complement_negate(b)
        value, carry_bit = adder.ripple_carry_adder(a, neg_b, 0)

    elseif op == alu.AND then
        value = bitwise_op(a, b, lg.AND)

    elseif op == alu.OR then
        value = bitwise_op(a, b, lg.OR)

    elseif op == alu.XOR then
        value = bitwise_op(a, b, lg.XOR)

    elseif op == alu.NOT then
        -- NOT is unary: it only operates on the A bus.
        value = {}
        for i = 1, #a do
            value[i] = lg.NOT(a[i])
        end

    else
        error("unknown operation: " .. tostring(op))
    end

    -- ----------------------------------------------------------------
    -- Step 2: Compute the condition flags
    -- ----------------------------------------------------------------

    -- Zero flag: true if every bit in the result is 0.
    -- This is the basis for equality checks in assembly: compare two
    -- values by subtracting them, then check if the result is zero.
    local zero = true
    for _, bit in ipairs(value) do
        if bit ~= 0 then
            zero = false
            break
        end
    end

    -- Negative flag: simply the most significant bit (MSB).
    -- In two's complement representation, MSB=1 means the number is
    -- negative. MSB=0 means positive (or zero).
    local negative = #value > 0 and value[#value] == 1

    -- Carry flag: did the addition produce a carry out of the top bit?
    -- This indicates unsigned overflow — the result is too large to fit
    -- in the available bits.
    local carry = carry_bit == 1

    -- Overflow flag: did signed arithmetic corrupt the sign?
    --
    -- Overflow occurs when adding two numbers with the same sign produces
    -- a result with a different sign. This is mathematically impossible
    -- and indicates that we "ran out of bits" to represent the magnitude.
    --
    -- Examples (4-bit two's complement, range -8 to +7):
    --   * 5 + 5 = 10, but 10 doesn't fit in 4 signed bits → overflow
    --   * (-5) + (-5) = -10, doesn't fit → overflow
    --   * 3 + (-2) = 1, different signs → NEVER overflows
    --
    -- For subtraction, we check the sign of -B (the negated operand)
    -- rather than B, since the actual addition uses the negated value.
    local overflow = false
    if op == alu.ADD or op == alu.SUB then
        local a_sign = a[#a]
        local b_sign
        if op == alu.ADD then
            b_sign = b[#b]
        else
            -- For subtraction A - B, we're really doing A + NOT(B) + 1.
            -- The effective sign of the second operand is the inverse of B's MSB.
            b_sign = lg.NOT(b[#b])
        end
        local result_sign = value[#value]

        -- If both operands had the same sign but the result has a
        -- different sign, a signed overflow occurred.
        if a_sign == b_sign and result_sign ~= a_sign then
            overflow = true
        end
    end

    return ALUResult.new(value, zero, carry, negative, overflow)
end

return alu

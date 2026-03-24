-- arithmetic — Integer arithmetic circuits built from logic gates
--
-- # Moving from Logic to Math
--
-- In the logic-gates package, we saw how transistors combine to form gates
-- that perform basic Boolean operations (AND, OR, XOR). But how do we get
-- a computer to do actual math?
--
-- This package answers that question. By creatively wiring together those
-- fundamental logic gates, we can build circuits that add, subtract, and
-- manipulate binary numbers. From a simple "Half Adder" that adds two
-- individual bits, we build up to an entire "Arithmetic Logic Unit" (ALU)
-- — the mathematical heart of every CPU.
--
-- This is Layer 9 of the coding-adventures computing stack.
-- It depends on Layer 10: logic-gates.
--
-- # Package Structure
--
--   arithmetic/
--     adder.lua  — Half adder, full adder, ripple carry adder
--     alu.lua    — Arithmetic Logic Unit with status flags
--     init.lua   — This file: re-exports the public API
--
-- # Quick Start
--
--   local arith = require("coding_adventures.arithmetic")
--
--   -- Add two bits
--   local sum, carry = arith.half_adder(1, 1)  -- sum=0, carry=1
--
--   -- Add two 4-bit numbers (5 + 3 = 8)
--   local result, cout = arith.ripple_carry_adder(
--       {1,0,1,0}, {1,1,0,0}, 0
--   )
--
--   -- Use the ALU
--   local alu = arith.ALU.new(4)
--   local res = alu:execute(arith.ADD, {1,0,1,0}, {1,1,0,0})
--   print(res.value)     -- {0,0,0,1} (8 in LSB-first binary)
--   print(res.zero)      -- false
--   print(res.negative)  -- true (MSB is 1 in 4-bit two's complement)

local adder_mod = require("coding_adventures.arithmetic.adder")
local alu_mod = require("coding_adventures.arithmetic.alu")

return {
    VERSION = "0.1.0",

    -- Adder circuits
    half_adder = adder_mod.half_adder,
    full_adder = adder_mod.full_adder,
    ripple_carry_adder = adder_mod.ripple_carry_adder,

    -- ALU
    ALU = alu_mod.ALU,
    ALUResult = alu_mod.ALUResult,

    -- Operation constants
    ADD = alu_mod.ADD,
    SUB = alu_mod.SUB,
    AND = alu_mod.AND,
    OR  = alu_mod.OR,
    XOR = alu_mod.XOR,
    NOT = alu_mod.NOT,
}

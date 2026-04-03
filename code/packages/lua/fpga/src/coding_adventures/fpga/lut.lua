--[[
  LUT — Lookup Table, the fundamental logic element of an FPGA.

  ## What is a LUT?

  A Lookup Table (LUT) is a small SRAM-based truth table that can implement
  ANY Boolean function of N inputs. In modern FPGAs, the most common size
  is a 4-input LUT (LUT4), which has 2^4 = 16 bits of configuration memory.

  ## How a LUT Works

  The key insight is that any Boolean function can be represented by its
  truth table. A 4-input function has 16 possible input combinations, so
  the truth table has 16 entries. We store these 16 output values in 16
  bits of SRAM. When inputs arrive, they form a 4-bit address that selects
  one of the 16 stored values — no gates needed!

  Example: implementing AND(a, b) with a 2-input LUT:

      Address (inputs) │ Stored Value (output)
      ─────────────────┼──────────────────────
         00 (a=0,b=0)  │         0
         01 (a=0,b=1)  │         0
         10 (a=1,b=0)  │         0
         11 (a=1,b=1)  │         1

  The LUT stores {0, 0, 0, 1} — the truth table for AND.

  ## Why LUTs?

  LUTs are the heart of FPGA flexibility. Any N-input Boolean function —
  AND, OR, XOR, majority vote, parity check, or any custom function —
  can be implemented by loading the right 2^N-bit pattern into the LUT.
  The FPGA bitstream (configuration file) specifies what pattern to load
  into each LUT, effectively "programming" the chip to implement any
  desired circuit.

  ## This Implementation

  We model a LUT as a table with:
    - `num_inputs` — the number of input pins (typically 4 or 6)
    - `truth_table` — an array of 2^num_inputs output bits (1-indexed in Lua)
]]

local LUT = {}
LUT.__index = LUT

--- Creates a new unconfigured LUT with the given number of inputs.
-- The truth table is initialized to all zeros (implements constant-0).
-- Use configure() to load a truth table.
--
-- @param num_inputs  number of input pins (positive integer)
-- @return new LUT object
function LUT.new(num_inputs)
  assert(type(num_inputs) == "number" and num_inputs > 0 and math.floor(num_inputs) == num_inputs,
    "num_inputs must be a positive integer")

  local table_size = 1 << num_inputs
  local truth_table = {}
  for i = 1, table_size do
    truth_table[i] = 0
  end

  return setmetatable({
    num_inputs  = num_inputs,
    truth_table = truth_table,
  }, LUT)
end

--- Configures the LUT with a truth table.
-- The truth table must be an array of exactly 2^num_inputs bits (0 or 1).
-- Returns the same LUT object (mutates in place).
--
-- @param truth_table  array of 0/1 values, length = 2^num_inputs
-- @return self (for chaining)
function LUT:configure(truth_table)
  local expected = 1 << self.num_inputs
  assert(#truth_table == expected,
    string.format("truth table must have %d entries for %d-input LUT, got %d",
      expected, self.num_inputs, #truth_table))

  for i, bit in ipairs(truth_table) do
    assert(bit == 0 or bit == 1,
      string.format("truth table entries must be 0 or 1, got %s at index %d", tostring(bit), i))
  end

  self.truth_table = truth_table
  return self
end

--- Evaluates the LUT for the given inputs.
-- Inputs form a binary address into the truth table (MSB first).
--
-- @param inputs  array of num_inputs bits {0,1,...}
-- @return  0 or 1
function LUT:evaluate(inputs)
  assert(#inputs == self.num_inputs,
    string.format("expected %d inputs, got %d", self.num_inputs, #inputs))

  -- Convert input bits to an index (MSB first)
  local index = 0
  for i = 1, #inputs do
    local bit = inputs[i]
    assert(bit == 0 or bit == 1,
      string.format("inputs must be 0 or 1, got %s at index %d", tostring(bit), i))
    index = (index << 1) | bit
  end

  -- truth_table is 1-indexed in Lua
  return self.truth_table[index + 1]
end

return LUT

-- lut.lua — Lookup Table: the fundamental logic element of an FPGA
--
-- # What is a LUT?
--
-- A Lookup Table (LUT) is a small SRAM-based truth table that can implement
-- ANY Boolean function of N inputs. In modern FPGAs, a 4-input LUT (LUT4)
-- has 2^4 = 16 bits of configuration memory.
--
-- How it works: inputs form a binary address into the truth table.
-- The stored value at that address is the output.
--
-- Example: implementing AND(a, b) with a 2-input LUT:
--
--   Address (inputs) | Stored Value
--   ─────────────────┼─────────────
--     00  (a=0,b=0)  |     0
--     01  (a=0,b=1)  |     0
--     10  (a=1,b=0)  |     0
--     11  (a=1,b=1)  |     1
--
-- The truth table is [0, 0, 0, 1]. Inputs [1,1] → address 3 → value 1.
-- Inputs are MSB-first: first input is the most significant bit.

local LUT = {}
LUT.__index = LUT

-- Creates a new unconfigured LUT with the given number of inputs.
-- The truth table is initialized to all zeros.
function LUT.new(num_inputs)
    num_inputs = num_inputs or 4
    assert(num_inputs > 0, "num_inputs must be > 0")
    local table_size = 1 << num_inputs
    local tt = {}
    for i = 1, table_size do tt[i] = 0 end
    return setmetatable({num_inputs = num_inputs, truth_table = tt}, LUT)
end

-- Configures the LUT with a truth table.
-- truth_table must be an array of exactly 2^num_inputs bits (0 or 1).
function LUT:configure(truth_table)
    local expected = 1 << self.num_inputs
    assert(#truth_table == expected,
        string.format("truth table must have %d entries for %d-input LUT, got %d",
            expected, self.num_inputs, #truth_table))
    self.truth_table = truth_table
    return self
end

-- Evaluates the LUT for the given inputs (array of bits, MSB-first).
-- Returns 0 or 1.
function LUT:evaluate(inputs)
    assert(#inputs == self.num_inputs,
        string.format("expected %d inputs, got %d", self.num_inputs, #inputs))
    -- Build index: MSB-first means first input is the highest bit
    local index = 0
    for i = 1, #inputs do
        index = (index << 1) | inputs[i]
    end
    -- 1-indexed: index 0 maps to truth_table[1]
    return self.truth_table[index + 1]
end

return LUT

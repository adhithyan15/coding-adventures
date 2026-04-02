-- clb.lua — Configurable Logic Block (CLB)
--
-- A CLB contains 2 slices arranged in a carry chain:
--
--   Slice 0 carry_out → Slice 1 carry_in
--
-- The CLB has 4 LUT inputs (2 per slice):
--   inputs.s0_a, inputs.s0_b — LUT inputs for Slice 0
--   inputs.s1_a, inputs.s1_b — LUT inputs for Slice 1
--
-- evaluate() returns: outputs (array of 4 bits), carry_out

local Slice = require("coding_adventures.fpga.slice")

local CLB = {}
CLB.__index = CLB

-- Creates a new CLB at grid position (row, col).
-- opts: same as Slice.new opts, applied to both slices
function CLB.new(row, col, opts)
    opts = opts or {}
    return setmetatable({
        slice_0 = Slice.new(opts),
        slice_1 = Slice.new(opts),
        row = row,
        col = col,
    }, CLB)
end

-- Configures both slices.
-- config may have keys slice_0 and slice_1, each a config for Slice.configure
function CLB:configure(config)
    if config.slice_0 then self.slice_0:configure(config.slice_0) end
    if config.slice_1 then self.slice_1:configure(config.slice_1) end
    return self
end

-- Evaluates the CLB.
-- inputs: table with keys s0_a, s0_b, s1_a, s1_b (each an array of bits)
-- clock: 0 or 1
-- carry_in: 0 or 1
--
-- Returns: outputs (array of 4 bits), carry_out
function CLB:evaluate(inputs, clock, carry_in)
    local s0_a, s0_b, carry_mid = self.slice_0:evaluate(inputs.s0_a, inputs.s0_b, clock, carry_in)
    local s1_a, s1_b, carry_out = self.slice_1:evaluate(inputs.s1_a, inputs.s1_b, clock, carry_mid)
    return {s0_a, s0_b, s1_a, s1_b}, carry_out
end

return CLB

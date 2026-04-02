--[[
  CLB — Configurable Logic Block, the primary logic resource in an FPGA.

  ## What is a CLB?

  A Configurable Logic Block (CLB) is the main repeating logic tile in an
  FPGA. Each CLB contains two Slices, giving it a total of:

    - 4 LUTs  (2 per slice)
    - 4 Flip-Flops (2 per slice)
    - 2 Carry Chains (1 per slice)

  CLBs are arranged in a grid across the FPGA, connected by the
  programmable routing network (switch matrices). The FPGA bitstream
  configures each CLB's LUT contents, flip-flop usage, and carry chain
  settings.

  ## CLB Layout

      ┌─────────────────────────────────────┐
      │           CLB (row, col)            │
      │  ┌──────────┐    ┌──────────┐      │
      │  │ Slice 0  │    │ Slice 1  │      │
      │  │ LUT_A    │    │ LUT_A    │      │
      │  │ LUT_B    │    │ LUT_B    │      │
      │  │ FF_A     │    │ FF_A     │      │
      │  │ FF_B     │    │ FF_B     │      │
      │  │ Carry ───┼────┤→ Carry   │      │
      │  └──────────┘    └──────────┘      │
      └─────────────────────────────────────┘

  The carry chain propagates from Slice 0 to Slice 1, enabling efficient
  multi-bit arithmetic across the full CLB.
]]

local Slice = require("coding_adventures.fpga.slice")

local CLB = {}
CLB.__index = CLB

--- Creates a new CLB at the given grid position.
--
-- @param row   row index (0-based)
-- @param col   column index (0-based)
-- @param opts  optional table:
--                lut_inputs (default 4) — inputs per LUT
--
-- @return new CLB object
function CLB.new(row, col, opts)
  opts = opts or {}
  local lut_inputs = opts.lut_inputs or 4

  return setmetatable({
    slice_0 = Slice.new({ lut_inputs = lut_inputs }),
    slice_1 = Slice.new({ lut_inputs = lut_inputs }),
    row     = row,
    col     = col,
  }, CLB)
end

--- Configures both slices in the CLB.
-- config is a table with optional keys "slice_0" and "slice_1",
-- each being a configuration table passed to Slice:configure().
--
-- @param config  table with optional slice_0 / slice_1 keys
-- @return self (for chaining)
function CLB:configure(config)
  if config.slice_0 then
    self.slice_0:configure(config.slice_0)
  end
  if config.slice_1 then
    self.slice_1:configure(config.slice_1)
  end
  return self
end

--- Evaluates the CLB.
--
-- inputs is a table with keys:
--   s0_a  — array of bits for Slice 0 LUT A
--   s0_b  — array of bits for Slice 0 LUT B
--   s1_a  — array of bits for Slice 1 LUT A
--   s1_b  — array of bits for Slice 1 LUT B
--
-- The carry chain propagates from Slice 0 to Slice 1 when carry_enable
-- is set on both slices.
--
-- @param inputs    table of signal arrays
-- @param clock     clock signal (0 or 1)
-- @param carry_in  carry input to Slice 0 (0 or 1)
--
-- @return outputs (array: {s0_a, s0_b, s1_a, s1_b}), carry_out
function CLB:evaluate(inputs, clock, carry_in)
  -- Evaluate Slice 0 — carry_in feeds in from external
  local s0_a, s0_b, carry_mid = self.slice_0:evaluate(
    inputs.s0_a, inputs.s0_b, clock, carry_in)

  -- Evaluate Slice 1 — carry_mid from Slice 0 feeds in
  local s1_a, s1_b, carry_out = self.slice_1:evaluate(
    inputs.s1_a, inputs.s1_b, clock, carry_mid)

  local outputs = { s0_a, s0_b, s1_a, s1_b }
  return outputs, carry_out
end

return CLB

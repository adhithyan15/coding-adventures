--[[
  Slice — the basic compute unit within a CLB.

  ## What is a Slice?

  A slice is a grouping of related logic resources within a Configurable
  Logic Block (CLB). Each slice contains:

    - 2 LUTs (for combinational logic)
    - 2 Flip-Flops (for sequential logic / state storage)
    - 1 Carry Chain (for fast arithmetic)

  ## How a Slice Works

  Each LUT computes a combinational function. The LUT output can either:
    1. Pass directly to the slice output (combinational path)
    2. Pass through a flip-flop first (registered path)

  The flip-flop captures the LUT output on the clock edge, creating a
  pipeline register. The `use_ff_a` and `use_ff_b` flags control whether
  each LUT output is registered.

  ## Carry Chain

  The carry chain allows efficient implementation of adders and counters.
  When carry_enable is true:

      sum_a     = lut_a_result XOR carry_in
      carry_mid = lut_a_result AND carry_in   (carry generate)
      sum_b     = lut_b_result XOR carry_mid
      carry_out = lut_b_result AND carry_mid

  This matches the standard full-adder carry propagation scheme.

  ## Flip-Flop Model

  A D flip-flop captures its D input on the rising edge of the clock (0→1).
  We model it as: if clock is 1, Q = D; otherwise Q stays unchanged.
  (Since we call evaluate on each cycle, this is equivalent to a clock-edge
  triggered D flip-flop.)
]]

local LUT = require("coding_adventures.fpga.lut")

local Slice = {}
Slice.__index = Slice

--- Creates a new slice with two N-input LUTs.
--
-- Options (table, optional):
--   lut_inputs   (default 4) — number of inputs per LUT
--   use_ff_a     (default false) — register LUT A output through FF
--   use_ff_b     (default false) — register LUT B output through FF
--   carry_enable (default false) — enable carry chain arithmetic
--
-- @param opts  optional configuration table
-- @return new Slice object
function Slice.new(opts)
  opts = opts or {}
  local lut_inputs   = opts.lut_inputs   or 4
  local use_ff_a     = opts.use_ff_a     or false
  local use_ff_b     = opts.use_ff_b     or false
  local carry_enable = opts.carry_enable or false

  return setmetatable({
    lut_a        = LUT.new(lut_inputs),
    lut_b        = LUT.new(lut_inputs),
    -- Flip-flop state: Q starts at 0
    ff_a         = 0,
    ff_b         = 0,
    use_ff_a     = use_ff_a,
    use_ff_b     = use_ff_b,
    carry_enable = carry_enable,
  }, Slice)
end

--- Configures the LUTs in this slice.
-- config is a table with optional keys "lut_a" and "lut_b",
-- each being a truth table (array of bits).
--
-- @param config  table with optional lut_a / lut_b keys
-- @return self (for chaining)
function Slice:configure(config)
  if config.lut_a then
    self.lut_a:configure(config.lut_a)
  end
  if config.lut_b then
    self.lut_b:configure(config.lut_b)
  end
  return self
end

--- Evaluates the slice with given inputs and clock signal.
--
-- @param inputs_a  array of bits for LUT A (length = lut_inputs)
-- @param inputs_b  array of bits for LUT B (length = lut_inputs)
-- @param clock     clock signal (0 or 1)
-- @param carry_in  carry input (0 or 1), used when carry_enable is true
--
-- @return output_a, output_b, carry_out
--   (Slice is mutated in-place for FF state)
function Slice:evaluate(inputs_a, inputs_b, clock, carry_in)
  -- Evaluate both LUTs combinationally
  local lut_a_result = self.lut_a:evaluate(inputs_a)
  local lut_b_result = self.lut_b:evaluate(inputs_b)

  -- Apply carry chain if enabled
  local out_a_comb, carry_mid, out_b_comb, carry_out

  if self.carry_enable then
    -- Full-adder style carry:
    --   sum   = data XOR carry_in
    --   carry = data AND carry_in  (generate) OR (propagate AND carry_in)
    -- For a LUT-based carry chain, the LUT typically computes the
    -- generate/propagate signals; we model it simply as:
    out_a_comb = lut_a_result ~ carry_in   -- XOR: sum bit
    carry_mid  = lut_a_result & carry_in   -- AND: carry out of A

    out_b_comb = lut_b_result ~ carry_mid
    carry_out  = lut_b_result & carry_mid
  else
    out_a_comb = lut_a_result
    out_b_comb = lut_b_result
    carry_mid  = 0
    carry_out  = 0
  end

  -- Apply flip-flops if enabled (rising-edge triggered: capture when clock=1)
  local output_a, output_b
  if self.use_ff_a then
    if clock == 1 then
      self.ff_a = out_a_comb
    end
    output_a = self.ff_a
  else
    output_a = out_a_comb
  end

  if self.use_ff_b then
    if clock == 1 then
      self.ff_b = out_b_comb
    end
    output_b = self.ff_b
  else
    output_b = out_b_comb
  end

  return output_a, output_b, carry_out
end

return Slice

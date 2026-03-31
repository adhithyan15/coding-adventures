-- slice.lua — The basic compute unit within a CLB
--
-- A slice contains:
--   - 2 LUTs (LUT A and LUT B) for combinational logic
--   - 2 D flip-flops (FF A and FF B) for registered outputs
--   - 1 carry chain for fast arithmetic
--
-- The carry chain: when carry_enable is true:
--   out_a_comb = LUT_A_result XOR carry_in   (sum bit)
--   carry_mid  = LUT_A_result AND carry_in   (carry to B)
--   out_b_comb = LUT_B_result XOR carry_mid
--   carry_out  = LUT_B_result AND carry_mid
--
-- Each LUT output can pass through a flip-flop (registered) or bypass it
-- (combinational), controlled by use_ff_a and use_ff_b.
--
-- The flip-flop is a positive-edge-triggered D flip-flop:
-- when clock=1 (rising edge), the output captures the input.

local LUT = require("coding_adventures.fpga.lut")

local Slice = {}
Slice.__index = Slice

-- Creates a new slice.
-- opts:
--   lut_inputs   (default 4) — inputs per LUT
--   use_ff_a     (default false) — register LUT A output
--   use_ff_b     (default false) — register LUT B output
--   carry_enable (default false) — enable carry chain
function Slice.new(opts)
    opts = opts or {}
    local n = opts.lut_inputs or 4
    return setmetatable({
        lut_a        = LUT.new(n),
        lut_b        = LUT.new(n),
        ff_a         = 0,  -- flip-flop state for output A
        ff_b         = 0,  -- flip-flop state for output B
        use_ff_a     = opts.use_ff_a     or false,
        use_ff_b     = opts.use_ff_b     or false,
        carry_enable = opts.carry_enable or false,
    }, Slice)
end

-- Configures the LUTs. config table may have keys lut_a and lut_b
-- (each a truth table array). Mutates slice in place.
function Slice:configure(config)
    if config.lut_a then self.lut_a:configure(config.lut_a) end
    if config.lut_b then self.lut_b:configure(config.lut_b) end
    return self
end

-- Evaluates the slice.
-- inputs_a, inputs_b: arrays of bits for LUT A and LUT B
-- clock: 0 or 1 (flip-flops capture when clock=1)
-- carry_in: 0 or 1 (used when carry_enable=true)
--
-- Returns: output_a, output_b, carry_out
-- Also mutates ff_a and ff_b state.
function Slice:evaluate(inputs_a, inputs_b, clock, carry_in)
    local lut_a_result = self.lut_a:evaluate(inputs_a)
    local lut_b_result = self.lut_b:evaluate(inputs_b)

    local out_a_comb, out_b_comb, carry_out

    if self.carry_enable then
        out_a_comb = lut_a_result ~ carry_in     -- XOR in Lua 5.4
        local carry_mid = lut_a_result & carry_in -- AND
        out_b_comb  = lut_b_result ~ carry_mid
        carry_out   = lut_b_result & carry_mid
    else
        out_a_comb = lut_a_result
        out_b_comb = lut_b_result
        carry_out  = 0
    end

    -- Apply flip-flops (capture on rising edge = clock=1)
    local output_a, output_b

    if self.use_ff_a then
        if clock == 1 then self.ff_a = out_a_comb end
        output_a = self.ff_a
    else
        output_a = out_a_comb
    end

    if self.use_ff_b then
        if clock == 1 then self.ff_b = out_b_comb end
        output_b = self.ff_b
    else
        output_b = out_b_comb
    end

    return output_a, output_b, carry_out
end

return Slice

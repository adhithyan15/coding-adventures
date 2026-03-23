-- pipeline.lua -- Pipelined floating-point arithmetic
--
-- === Why Pipelining? ===
--
-- Imagine a car factory with a single worker who does everything: welds the
-- frame, installs the engine, paints the body, mounts the wheels, inspects
-- the result. One car takes 5 hours. Want 100 cars? That's 500 hours.
--
-- Now imagine a factory with 5 stations, each doing one step. The first car
-- still takes 5 hours to pass through all 5 stations. But while it moves to
-- station 2, a NEW car enters station 1. After the initial 5-hour fill-up
-- time, a finished car rolls off the line every HOUR -- 5x throughput!
--
-- This is pipelining, and it's exactly how GPUs achieve massive throughput.
--
-- === Latency vs Throughput ===
--
--   Latency:     Time for ONE operation to complete start-to-finish.
--   Throughput:  How many operations complete per unit time.
--
-- For a 5-stage pipeline:
--
--   Latency = 5 clock cycles (one operation still takes 5 cycles)
--   Throughput = 1 result per clock cycle (after pipeline fills up)
--
-- === Pipeline Timing Diagram ===
--
--   Clock:  1    2    3    4    5    6    7    8
--   --------------------------------------------
--   Stage1: [A1] [B1] [C1] [D1]  -    -    -    -
--   Stage2:  -   [A2] [B2] [C2] [D2]  -    -    -
--   Stage3:  -    -   [A3] [B3] [C3] [D3]  -    -
--   Stage4:  -    -    -   [A4] [B4] [C4] [D4]  -
--   Stage5:  -    -    -    -   [A5] [B5] [C5] [D5]
--
-- === How This Connects to GPUs ===
--
-- A modern GPU has thousands of CUDA cores, each containing pipelined FP units.
-- With 5000 cores each running pipelined FP at 1.5 GHz:
--
--   5000 cores x 1 result/cycle x 1.5 GHz = 7.5 TFLOPS

local formats = require("coding_adventures.fp_arithmetic.formats")
local ieee754 = require("coding_adventures.fp_arithmetic.ieee754")
local logic_gates = require("coding_adventures.logic_gates")

local FloatBits = formats.FloatBits
local int_to_bits_msb = formats.int_to_bits_msb
local bits_msb_to_int = formats.bits_msb_to_int
local bit_length = formats.bit_length
local make_nan = formats.make_nan
local make_inf = formats.make_inf
local make_zero = formats.make_zero

local is_nan = ieee754.is_nan
local is_inf = ieee754.is_inf
local is_zero = ieee754.is_zero

-- =========================================================================
-- Stage data -- intermediate pipeline data passed between stages
-- =========================================================================

--- Creates a new stage_data table with the given fields.
--- In hardware, this data lives in pipeline registers -- banks of D flip-flops
--- that capture values on each clock edge.
---
--- The "special" field handles bypass cases (NaN, Inf, Zero) that skip
--- the normal computation stages.
local function new_stage_data(fields)
    return fields or {}
end

-- =========================================================================
-- PipelinedFPAdder -- 5-stage pipelined floating-point adder
-- =========================================================================

--- PipelinedFPAdder is a 5-stage pipelined floating-point adder driven by a clock.
---
--- In real GPU hardware, the FP adder is pipelined so that while one
--- addition is being normalized (stage 4), a newer addition is being
--- aligned (stage 2), and an even newer one is being unpacked (stage 1).
---
--- === Pipeline Stages ===
---
---   Stage 1: UNPACK    -- Extract sign, exponent, mantissa. Handle specials.
---   Stage 2: ALIGN     -- Compare exponents, shift smaller mantissa right.
---   Stage 3: ADD/SUB   -- Add or subtract aligned mantissas.
---   Stage 4: NORMALIZE -- Shift result to get leading 1 in correct position.
---   Stage 5: ROUND     -- Apply round-to-nearest-even, pack into FloatBits.
local PipelinedFPAdder = {}
PipelinedFPAdder.__index = PipelinedFPAdder

--- Creates a new 5-stage pipelined adder and registers it as a listener
--- on the given clock.
---
--- @param clk table A Clock instance
--- @param fmt table A FloatFormat instance
--- @return table A PipelinedFPAdder instance
function PipelinedFPAdder.new(clk, fmt)
    local self = setmetatable({}, PipelinedFPAdder)
    self.clk = clk
    self.fmt = fmt
    self.results = {}
    self.cycle_count = 0
    self._stages = {nil, nil, nil, nil, nil}  -- 5 stages
    self._inputs_pending = {}

    clk:register_listener(function(edge) self:_on_clock_edge(edge) end)
    return self
end

--- Queues a new addition (a + b) to enter the pipeline on the next
--- rising clock edge.
function PipelinedFPAdder:submit(a, b)
    self._inputs_pending[#self._inputs_pending + 1] = {a = a, b = b}
end

--- Advances the pipeline on rising clock edges.
function PipelinedFPAdder:_on_clock_edge(edge)
    if not edge.is_rising then return end

    self.cycle_count = self.cycle_count + 1

    -- Shift pipeline forward (from end to avoid overwriting)
    for i = 5, 2, -1 do
        self._stages[i] = self:_adder_process_stage(i, self._stages[i - 1])
    end

    -- Load new input
    if #self._inputs_pending > 0 then
        local inp = table.remove(self._inputs_pending, 1)
        self._stages[1] = self:_adder_stage_unpack(inp.a, inp.b)
    else
        self._stages[1] = nil
    end

    -- Collect output from last stage
    if self._stages[5] ~= nil then
        if self._stages[5].special ~= nil then
            self.results[#self.results + 1] = self._stages[5].special
        end
        self._stages[5] = nil
    end
end

function PipelinedFPAdder:_adder_process_stage(stage_num, input)
    if input == nil then return nil end
    if stage_num == 2 then return self:_adder_stage_align(input)
    elseif stage_num == 3 then return self:_adder_stage_add(input)
    elseif stage_num == 4 then return self:_adder_stage_normalize(input)
    elseif stage_num == 5 then return self:_adder_stage_round_pack(input)
    end
    return nil
end

-- Stage 0: UNPACK
function PipelinedFPAdder:_adder_stage_unpack(a, b)
    local f = self.fmt

    if is_nan(a) or is_nan(b) then
        return {special = make_nan(f)}
    end
    local a_inf, b_inf = is_inf(a), is_inf(b)
    if a_inf and b_inf then
        if a.sign == b.sign then
            return {special = make_inf(a.sign, f)}
        end
        return {special = make_nan(f)}
    end
    if a_inf then return {special = a} end
    if b_inf then return {special = b} end

    local a_zero, b_zero = is_zero(a), is_zero(b)
    if a_zero and b_zero then
        return {special = make_zero(logic_gates.AND(a.sign, b.sign), f)}
    end
    if a_zero then return {special = b} end
    if b_zero then return {special = a} end

    local exp_a = bits_msb_to_int(a.exponent)
    local exp_b = bits_msb_to_int(b.exponent)
    local mant_a = bits_msb_to_int(a.mantissa)
    local mant_b = bits_msb_to_int(b.mantissa)

    if exp_a ~= 0 then mant_a = (1 << f.mantissa_bits) | mant_a
    else exp_a = 1 end
    if exp_b ~= 0 then mant_b = (1 << f.mantissa_bits) | mant_b
    else exp_b = 1 end

    local guard_bits = 3
    mant_a = mant_a << guard_bits
    mant_b = mant_b << guard_bits

    return {
        sign_a = a.sign, sign_b = b.sign,
        exp_a = exp_a, exp_b = exp_b,
        mant_a = mant_a, mant_b = mant_b,
        guard_bits = guard_bits,
    }
end

-- Stage 1: ALIGN
function PipelinedFPAdder:_adder_stage_align(data)
    if data.special then return data end

    local f = self.fmt
    local exp_a, exp_b = data.exp_a, data.exp_b
    local mant_a, mant_b = data.mant_a, data.mant_b
    local guard_bits = data.guard_bits

    local result_exp
    if exp_a >= exp_b then
        local exp_diff = exp_a - exp_b
        if exp_diff > 0 then
            if exp_diff < (f.mantissa_bits + 1 + guard_bits) then
                local shifted_out = mant_b & ((1 << exp_diff) - 1)
                mant_b = mant_b >> exp_diff
                if shifted_out ~= 0 then mant_b = mant_b | 1 end
            else
                local sticky = (mant_b ~= 0) and 1 or 0
                mant_b = mant_b >> exp_diff
                if sticky ~= 0 then mant_b = mant_b | 1 end
            end
        end
        result_exp = exp_a
    else
        local exp_diff = exp_b - exp_a
        if exp_diff > 0 then
            if exp_diff < (f.mantissa_bits + 1 + guard_bits) then
                local shifted_out = mant_a & ((1 << exp_diff) - 1)
                mant_a = mant_a >> exp_diff
                if shifted_out ~= 0 then mant_a = mant_a | 1 end
            else
                local sticky = (mant_a ~= 0) and 1 or 0
                mant_a = mant_a >> exp_diff
                if sticky ~= 0 then mant_a = mant_a | 1 end
            end
        end
        result_exp = exp_b
    end

    return {
        sign_a = data.sign_a, sign_b = data.sign_b,
        mant_a = mant_a, mant_b = mant_b,
        result_exp = result_exp, guard_bits = guard_bits,
    }
end

-- Stage 2: ADD/SUB
function PipelinedFPAdder:_adder_stage_add(data)
    if data.special then return data end

    local result_mant, result_sign
    if data.sign_a == data.sign_b then
        result_mant = data.mant_a + data.mant_b
        result_sign = data.sign_a
    else
        if data.mant_a >= data.mant_b then
            result_mant = data.mant_a - data.mant_b
            result_sign = data.sign_a
        else
            result_mant = data.mant_b - data.mant_a
            result_sign = data.sign_b
        end
    end

    if result_mant == 0 then
        return {special = make_zero(0, self.fmt)}
    end

    return {
        result_sign = result_sign, result_mant = result_mant,
        result_exp = data.result_exp, guard_bits = data.guard_bits,
    }
end

-- Stage 3: NORMALIZE
function PipelinedFPAdder:_adder_stage_normalize(data)
    if data.special then return data end

    local f = self.fmt
    local result_mant = data.result_mant
    local result_exp = data.result_exp
    local guard_bits = data.guard_bits
    local normal_pos = f.mantissa_bits + guard_bits
    local leading_pos = bit_length(result_mant) - 1

    if leading_pos > normal_pos then
        local shift_amount = leading_pos - normal_pos
        local lost_bits = result_mant & ((1 << shift_amount) - 1)
        result_mant = result_mant >> shift_amount
        if lost_bits ~= 0 then result_mant = result_mant | 1 end
        result_exp = result_exp + shift_amount
    elseif leading_pos < normal_pos then
        local shift_amount = normal_pos - leading_pos
        if result_exp - shift_amount >= 1 then
            result_mant = result_mant << shift_amount
            result_exp = result_exp - shift_amount
        else
            local actual_shift = result_exp - 1
            if actual_shift > 0 then
                result_mant = result_mant << actual_shift
            end
            result_exp = 0
        end
    end

    return {
        result_sign = data.result_sign, result_mant = result_mant,
        result_exp = result_exp, guard_bits = guard_bits,
    }
end

-- Stage 4: ROUND & PACK
function PipelinedFPAdder:_adder_stage_round_pack(data)
    if data.special then return data end

    local f = self.fmt
    local result_mant = data.result_mant
    local result_exp = data.result_exp
    local result_sign = data.result_sign
    local guard_bits = data.guard_bits

    local guard = (result_mant >> (guard_bits - 1)) & 1
    local round_bit = (result_mant >> (guard_bits - 2)) & 1
    local sticky_bit = result_mant & ((1 << (guard_bits - 2)) - 1)
    if sticky_bit ~= 0 then sticky_bit = 1 end

    result_mant = result_mant >> guard_bits

    if guard == 1 then
        if round_bit == 1 or sticky_bit == 1 then
            result_mant = result_mant + 1
        elseif (result_mant & 1) == 1 then
            result_mant = result_mant + 1
        end
    end

    if result_mant >= (1 << (f.mantissa_bits + 1)) then
        result_mant = result_mant >> 1
        result_exp = result_exp + 1
    end

    local max_exp = (1 << f.exponent_bits) - 1
    if result_exp >= max_exp then
        return {special = make_inf(result_sign, f)}
    end
    if result_exp <= 0 then
        if result_exp < -(f.mantissa_bits) then
            return {special = make_zero(result_sign, f)}
        end
        local shift = 1 - result_exp
        result_mant = result_mant >> shift
        result_exp = 0
    end

    if result_exp > 0 then
        result_mant = result_mant & ((1 << f.mantissa_bits) - 1)
    end

    return {
        special = FloatBits.new(
            result_sign,
            int_to_bits_msb(result_exp, f.exponent_bits),
            int_to_bits_msb(result_mant, f.mantissa_bits),
            f
        )
    }
end

-- =========================================================================
-- PipelinedFPMultiplier -- 4-stage pipelined floating-point multiplier
-- =========================================================================

--- PipelinedFPMultiplier is a 4-stage pipelined floating-point multiplier.
---
--- Multiplication is simpler than addition because there's no alignment step.
---
---   Stage 1: UNPACK + SIGN + EXPONENT
---   Stage 2: MULTIPLY MANTISSAS
---   Stage 3: NORMALIZE
---   Stage 4: ROUND & PACK
local PipelinedFPMultiplier = {}
PipelinedFPMultiplier.__index = PipelinedFPMultiplier

function PipelinedFPMultiplier.new(clk, fmt)
    local self = setmetatable({}, PipelinedFPMultiplier)
    self.clk = clk
    self.fmt = fmt
    self.results = {}
    self.cycle_count = 0
    self._stages = {nil, nil, nil, nil}
    self._inputs_pending = {}

    clk:register_listener(function(edge) self:_on_clock_edge(edge) end)
    return self
end

function PipelinedFPMultiplier:submit(a, b)
    self._inputs_pending[#self._inputs_pending + 1] = {a = a, b = b}
end

function PipelinedFPMultiplier:_on_clock_edge(edge)
    if not edge.is_rising then return end

    self.cycle_count = self.cycle_count + 1

    for i = 4, 2, -1 do
        self._stages[i] = self:_mul_process_stage(i, self._stages[i - 1])
    end

    if #self._inputs_pending > 0 then
        local inp = table.remove(self._inputs_pending, 1)
        self._stages[1] = self:_mul_stage_unpack_exp(inp.a, inp.b)
    else
        self._stages[1] = nil
    end

    if self._stages[4] ~= nil then
        if self._stages[4].special ~= nil then
            self.results[#self.results + 1] = self._stages[4].special
        end
        self._stages[4] = nil
    end
end

function PipelinedFPMultiplier:_mul_process_stage(stage_num, input)
    if input == nil then return nil end
    if stage_num == 2 then return self:_mul_stage_multiply(input)
    elseif stage_num == 3 then return self:_mul_stage_normalize(input)
    elseif stage_num == 4 then return self:_mul_stage_round_pack(input)
    end
    return nil
end

-- Stage 0: UNPACK + SIGN + EXPONENT
function PipelinedFPMultiplier:_mul_stage_unpack_exp(a, b)
    local f = self.fmt
    local result_sign = logic_gates.XOR(a.sign, b.sign)

    if is_nan(a) or is_nan(b) then return {special = make_nan(f)} end

    local a_inf, b_inf = is_inf(a), is_inf(b)
    local a_zero, b_zero = is_zero(a), is_zero(b)

    if (a_inf and b_zero) or (b_inf and a_zero) then return {special = make_nan(f)} end
    if a_inf or b_inf then return {special = make_inf(result_sign, f)} end
    if a_zero or b_zero then return {special = make_zero(result_sign, f)} end

    local exp_a = bits_msb_to_int(a.exponent)
    local exp_b = bits_msb_to_int(b.exponent)
    local mant_a = bits_msb_to_int(a.mantissa)
    local mant_b = bits_msb_to_int(b.mantissa)

    if exp_a ~= 0 then mant_a = (1 << f.mantissa_bits) | mant_a
    else exp_a = 1 end
    if exp_b ~= 0 then mant_b = (1 << f.mantissa_bits) | mant_b
    else exp_b = 1 end

    return {
        result_sign = result_sign,
        result_exp = exp_a + exp_b - f.bias,
        mant_a = mant_a, mant_b = mant_b,
    }
end

-- Stage 1: MULTIPLY MANTISSAS
function PipelinedFPMultiplier:_mul_stage_multiply(data)
    if data.special then return data end
    return {
        result_sign = data.result_sign,
        result_exp = data.result_exp,
        product = data.mant_a * data.mant_b,
    }
end

-- Stage 2: NORMALIZE
function PipelinedFPMultiplier:_mul_stage_normalize(data)
    if data.special then return data end

    local f = self.fmt
    local product = data.product
    local result_exp = data.result_exp

    local product_leading = bit_length(product) - 1
    local normal_pos = 2 * f.mantissa_bits

    if product_leading > normal_pos then
        result_exp = result_exp + (product_leading - normal_pos)
    elseif product_leading < normal_pos then
        result_exp = result_exp - (normal_pos - product_leading)
    end

    return {
        result_sign = data.result_sign,
        result_exp = result_exp,
        product = product,
        product_leading = product_leading,
    }
end

-- Stage 3: ROUND & PACK
function PipelinedFPMultiplier:_mul_stage_round_pack(data)
    if data.special then return data end

    local f = self.fmt
    local result_sign = data.result_sign
    local result_exp = data.result_exp
    local product = data.product
    local product_leading = data.product_leading

    local round_pos = product_leading - f.mantissa_bits
    local result_mant

    if round_pos > 0 then
        local guard = (product >> (round_pos - 1)) & 1
        local round_bit, sticky = 0, 0
        if round_pos >= 2 then
            round_bit = (product >> (round_pos - 2)) & 1
            if (product & ((1 << (round_pos - 2)) - 1)) ~= 0 then sticky = 1 end
        end
        result_mant = product >> round_pos
        if guard == 1 then
            if round_bit == 1 or sticky == 1 then
                result_mant = result_mant + 1
            elseif (result_mant & 1) == 1 then
                result_mant = result_mant + 1
            end
        end
        if result_mant >= (1 << (f.mantissa_bits + 1)) then
            result_mant = result_mant >> 1
            result_exp = result_exp + 1
        end
    elseif round_pos == 0 then
        result_mant = product
    else
        result_mant = product << (-round_pos)
    end

    local max_exp = (1 << f.exponent_bits) - 1
    if result_exp >= max_exp then return {special = make_inf(result_sign, f)} end
    if result_exp <= 0 then
        if result_exp < -(f.mantissa_bits) then return {special = make_zero(result_sign, f)} end
        local shift = 1 - result_exp
        result_mant = result_mant >> shift
        result_exp = 0
    end
    if result_exp > 0 then
        result_mant = result_mant & ((1 << f.mantissa_bits) - 1)
    end

    return {
        special = FloatBits.new(
            result_sign,
            int_to_bits_msb(result_exp, f.exponent_bits),
            int_to_bits_msb(result_mant, f.mantissa_bits),
            f
        )
    }
end

-- =========================================================================
-- PipelinedFMA -- 6-stage pipelined fused multiply-add
-- =========================================================================

--- PipelinedFMA is a 6-stage pipelined fused multiply-add unit.
---
--- FMA computes a * b + c with a single rounding step. It's the most
--- important operation in machine learning because the dot product is
--- just a chain of FMAs.
---
---   Stage 1: UNPACK all three operands
---   Stage 2: MULTIPLY a * b mantissas (full precision!)
---   Stage 3: ALIGN product with c
---   Stage 4: ADD product + c
---   Stage 5: NORMALIZE
---   Stage 6: ROUND & PACK (single rounding step!)
local PipelinedFMA = {}
PipelinedFMA.__index = PipelinedFMA

function PipelinedFMA.new(clk, fmt)
    local self = setmetatable({}, PipelinedFMA)
    self.clk = clk
    self.fmt = fmt
    self.results = {}
    self.cycle_count = 0
    self._stages = {nil, nil, nil, nil, nil, nil}
    self._inputs_pending = {}

    clk:register_listener(function(edge) self:_on_clock_edge(edge) end)
    return self
end

function PipelinedFMA:submit(a, b, c)
    self._inputs_pending[#self._inputs_pending + 1] = {a = a, b = b, c = c}
end

function PipelinedFMA:_on_clock_edge(edge)
    if not edge.is_rising then return end

    self.cycle_count = self.cycle_count + 1

    for i = 6, 2, -1 do
        self._stages[i] = self:_fma_process_stage(i, self._stages[i - 1])
    end

    if #self._inputs_pending > 0 then
        local inp = table.remove(self._inputs_pending, 1)
        self._stages[1] = self:_fma_stage_unpack(inp.a, inp.b, inp.c)
    else
        self._stages[1] = nil
    end

    if self._stages[6] ~= nil then
        if self._stages[6].special ~= nil then
            self.results[#self.results + 1] = self._stages[6].special
        end
        self._stages[6] = nil
    end
end

function PipelinedFMA:_fma_process_stage(stage_num, input)
    if input == nil then return nil end
    if stage_num == 2 then return self:_fma_stage_multiply(input)
    elseif stage_num == 3 then return self:_fma_stage_align(input)
    elseif stage_num == 4 then return self:_fma_stage_add(input)
    elseif stage_num == 5 then return self:_fma_stage_normalize(input)
    elseif stage_num == 6 then return self:_fma_stage_round_pack(input)
    end
    return nil
end

-- Stage 0: UNPACK all three operands
function PipelinedFMA:_fma_stage_unpack(a, b, c)
    local f = self.fmt

    if is_nan(a) or is_nan(b) or is_nan(c) then return {special = make_nan(f)} end

    local a_inf, b_inf, c_inf = is_inf(a), is_inf(b), is_inf(c)
    local a_zero, b_zero = is_zero(a), is_zero(b)
    local product_sign = logic_gates.XOR(a.sign, b.sign)

    if (a_inf and b_zero) or (b_inf and a_zero) then return {special = make_nan(f)} end
    if a_inf or b_inf then
        if c_inf and product_sign ~= c.sign then return {special = make_nan(f)} end
        return {special = make_inf(product_sign, f)}
    end
    if a_zero or b_zero then
        if is_zero(c) then
            return {special = make_zero(logic_gates.AND(product_sign, c.sign), f)}
        end
        return {special = c}
    end
    if c_inf then return {special = c} end

    local exp_a = bits_msb_to_int(a.exponent)
    local exp_b = bits_msb_to_int(b.exponent)
    local mant_a = bits_msb_to_int(a.mantissa)
    local mant_b = bits_msb_to_int(b.mantissa)
    local exp_c = bits_msb_to_int(c.exponent)
    local mant_c = bits_msb_to_int(c.mantissa)

    if exp_a ~= 0 then mant_a = (1 << f.mantissa_bits) | mant_a else exp_a = 1 end
    if exp_b ~= 0 then mant_b = (1 << f.mantissa_bits) | mant_b else exp_b = 1 end
    if exp_c ~= 0 then mant_c = (1 << f.mantissa_bits) | mant_c else exp_c = 1 end

    return {
        product_sign = product_sign, c_sign = c.sign,
        exp_a = exp_a, exp_b = exp_b,
        mant_a = mant_a, mant_b = mant_b,
        exp_c = exp_c, mant_c = mant_c,
    }
end

-- Stage 1: MULTIPLY
function PipelinedFMA:_fma_stage_multiply(data)
    if data.special then return data end

    local f = self.fmt
    local product = data.mant_a * data.mant_b
    local product_exp = data.exp_a + data.exp_b - f.bias

    local product_leading = bit_length(product) - 1
    local normal_pos = 2 * f.mantissa_bits

    if product_leading > normal_pos then
        product_exp = product_exp + (product_leading - normal_pos)
    elseif product_leading < normal_pos then
        product_exp = product_exp - (normal_pos - product_leading)
    end

    return {
        product_sign = data.product_sign, c_sign = data.c_sign,
        product = product, product_exp = product_exp,
        product_leading = product_leading,
        exp_c = data.exp_c, mant_c = data.mant_c,
    }
end

-- Stage 2: ALIGN
function PipelinedFMA:_fma_stage_align(data)
    if data.special then return data end

    local f = self.fmt
    local product = data.product
    local product_exp = data.product_exp
    local product_leading = data.product_leading
    local mant_c = data.mant_c

    local exp_diff = product_exp - data.exp_c
    local c_scale_shift = product_leading - f.mantissa_bits
    local c_aligned
    if c_scale_shift >= 0 then
        c_aligned = mant_c << c_scale_shift
    else
        c_aligned = mant_c >> (-c_scale_shift)
    end

    local result_exp
    if exp_diff >= 0 then
        c_aligned = c_aligned >> exp_diff
        result_exp = product_exp
    else
        product = product >> (-exp_diff)
        result_exp = data.exp_c
    end

    return {
        product_sign = data.product_sign, c_sign = data.c_sign,
        product = product, c_aligned = c_aligned,
        result_exp = result_exp, product_leading = product_leading,
    }
end

-- Stage 3: ADD
function PipelinedFMA:_fma_stage_add(data)
    if data.special then return data end

    local result_mant, result_sign
    if data.product_sign == data.c_sign then
        result_mant = data.product + data.c_aligned
        result_sign = data.product_sign
    else
        if data.product >= data.c_aligned then
            result_mant = data.product - data.c_aligned
            result_sign = data.product_sign
        else
            result_mant = data.c_aligned - data.product
            result_sign = data.c_sign
        end
    end

    if result_mant == 0 then
        return {special = make_zero(0, self.fmt)}
    end

    return {
        result_sign = result_sign, result_mant = result_mant,
        result_exp = data.result_exp, product_leading = data.product_leading,
    }
end

-- Stage 4: NORMALIZE
function PipelinedFMA:_fma_stage_normalize(data)
    if data.special then return data end

    local f = self.fmt
    local result_mant = data.result_mant
    local result_exp = data.result_exp
    local product_leading = data.product_leading
    local target_pos = product_leading
    if target_pos < f.mantissa_bits then target_pos = f.mantissa_bits end

    local result_leading = bit_length(result_mant) - 1
    if result_leading > target_pos then
        result_exp = result_exp + (result_leading - target_pos)
    elseif result_leading < target_pos then
        result_exp = result_exp - (target_pos - result_leading)
    end

    return {
        result_sign = data.result_sign, result_mant = result_mant,
        result_exp = result_exp,
    }
end

-- Stage 5: ROUND & PACK
function PipelinedFMA:_fma_stage_round_pack(data)
    if data.special then return data end

    local f = self.fmt
    local result_sign = data.result_sign
    local result_exp = data.result_exp
    local result_mant = data.result_mant

    local result_leading = bit_length(result_mant) - 1
    local round_pos = result_leading - f.mantissa_bits

    if round_pos > 0 then
        local guard = (result_mant >> (round_pos - 1)) & 1
        local round_bit, sticky = 0, 0
        if round_pos >= 2 then
            round_bit = (result_mant >> (round_pos - 2)) & 1
            if (result_mant & ((1 << (round_pos - 2)) - 1)) ~= 0 then sticky = 1 end
        end
        result_mant = result_mant >> round_pos
        if guard == 1 then
            if round_bit == 1 or sticky == 1 then
                result_mant = result_mant + 1
            elseif (result_mant & 1) == 1 then
                result_mant = result_mant + 1
            end
        end
        if result_mant >= (1 << (f.mantissa_bits + 1)) then
            result_mant = result_mant >> 1
            result_exp = result_exp + 1
        end
    elseif round_pos < 0 then
        result_mant = result_mant << (-round_pos)
    end

    local max_exp = (1 << f.exponent_bits) - 1
    if result_exp >= max_exp then return {special = make_inf(result_sign, f)} end
    if result_exp <= 0 then
        if result_exp < -(f.mantissa_bits) then return {special = make_zero(result_sign, f)} end
        local shift = 1 - result_exp
        result_mant = result_mant >> shift
        result_exp = 0
    end
    if result_exp > 0 then
        result_mant = result_mant & ((1 << f.mantissa_bits) - 1)
    end

    return {
        special = FloatBits.new(
            result_sign,
            int_to_bits_msb(result_exp, f.exponent_bits),
            int_to_bits_msb(result_mant, f.mantissa_bits),
            f
        )
    }
end

-- =========================================================================
-- FPUnit -- a complete floating-point unit with all three pipelines
-- =========================================================================

--- FPUnit is a complete floating-point unit with pipelined adder, multiplier,
--- and FMA. This is what sits inside every GPU core.
---
---   +--------------------------------------------------+
---   |                    FP Unit                        |
---   |                                                  |
---   |   +-----------------------------+                |
---   |   |  Pipelined FP Adder (5)     |                |
---   |   +-----------------------------+                |
---   |                                                  |
---   |   +-----------------------------+                |
---   |   |  Pipelined FP Multiplier (4)|                |
---   |   +-----------------------------+                |
---   |                                                  |
---   |   +-----------------------------+                |
---   |   |  Pipelined FMA Unit (6)     |                |
---   |   +-----------------------------+                |
---   |                                                  |
---   |   All three share the same clock signal          |
---   +--------------------------------------------------+
local FPUnit = {}
FPUnit.__index = FPUnit

--- Creates a complete floating-point unit with all three pipelines
--- sharing the same clock.
---
--- @param clk table A Clock instance
--- @param fmt table A FloatFormat instance
--- @return table An FPUnit instance
function FPUnit.new(clk, fmt)
    local self = setmetatable({}, FPUnit)
    self.clk = clk
    self.fmt = fmt
    self.adder = PipelinedFPAdder.new(clk, fmt)
    self.multiplier = PipelinedFPMultiplier.new(clk, fmt)
    self.fma = PipelinedFMA.new(clk, fmt)
    return self
end

--- Runs the clock for n complete cycles.
---
--- @param n number Number of complete cycles to run
function FPUnit:tick(n)
    for _ = 1, n do
        self.clk:full_cycle()
    end
end

-- =========================================================================
-- Module exports
-- =========================================================================

return {
    PipelinedFPAdder = PipelinedFPAdder,
    PipelinedFPMultiplier = PipelinedFPMultiplier,
    PipelinedFMA = PipelinedFMA,
    FPUnit = FPUnit,
}

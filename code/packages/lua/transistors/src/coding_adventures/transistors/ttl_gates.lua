-- ttl_gates.lua -- Historical BJT-based digital logic (TTL and RTL)
--
-- === What is TTL? ===
--
-- TTL stands for Transistor-Transistor Logic.  It was the dominant digital
-- logic family from the mid-1960s through the 1980s.  The "7400 series" --
-- a family of TTL chips -- defined the standard logic gates.
--
-- === Why TTL Lost to CMOS ===
--
-- TTL's fatal flaw: STATIC POWER CONSUMPTION.
--
-- In a TTL gate, current flows through resistors and transistors even when
-- the gate is doing nothing.  A single TTL NAND gate dissipates ~1-10 mW:
--
--   1 million gates x 10 mW/gate = 10,000 watts (a space heater!)
--
-- CMOS gates consume near-zero power at rest, allowing chips to scale
-- to billions of gates.
--
-- === RTL: The Predecessor to TTL ===
--
-- Before TTL came RTL (Resistor-Transistor Logic).  An RTL inverter is
-- just one transistor with two resistors.  It was used in the Apollo
-- Guidance Computer that landed humans on the moon in 1969.

local types = require("coding_adventures.transistors.types")
local bjt   = require("coding_adventures.transistors.bjt")

local ttl = {}

-- ==========================================================================
-- TTL NAND -- 7400-series style
-- ==========================================================================
--
-- Simplified circuit:
--
--       Vcc (+5 V)
--        |
--        R1 (4 k ohm)
--        |
--   +----+----+
--   |  Q1     |  Multi-emitter input transistor
--   |  (NPN)  |
--   +-- E1 ---+-- Input A
--   +-- E2 ---+-- Input B
--   +----+----+
--        |
--   +----+----+
--   |  Q2     |  Phase splitter
--   +----+----+
--        |
--   +----+----+
--   |  Q3     |  Output transistor
--   +----+----+
--        |
--       GND
--
-- Any input LOW -> output HIGH.  ALL inputs HIGH -> output LOW (NAND).

local TTLNand = {}
TTLNand.__index = TTLNand

--- Create a TTL NAND gate.  Default Vcc is 5 V.
function ttl.TTLNand(vcc, params)
    local self = setmetatable({}, TTLNand)
    self.vcc      = vcc or 5.0
    self.params   = params or types.BJTParams()
    self.r_pullup = 4000.0  -- 4 k ohm pull-up resistor
    self.q1       = bjt.NPN(self.params)
    self.q2       = bjt.NPN(self.params)
    self.q3       = bjt.NPN(self.params)
    return self
end

function TTLNand:evaluate(va, vb)
    local vcc    = self.vcc
    local vbe_on = self.params.vbe_on

    -- TTL input thresholds: LOW < 0.8 V, HIGH > 2.0 V
    local a_high = va > 2.0
    local b_high = vb > 2.0

    local output_v, logic_value, current

    if a_high and b_high then
        -- ALL inputs HIGH -> output LOW
        output_v    = self.params.vce_sat  -- ~0.2 V
        logic_value = 0
        -- Static current: Vcc through resistor chain
        current = (vcc - 2 * vbe_on - self.params.vce_sat) / self.r_pullup
        if current < 0 then current = 0 end
    else
        -- At least one input LOW -> output HIGH
        output_v    = vcc - vbe_on  -- ~4.3 V
        logic_value = 1
        -- Small bias current through pull-up
        current = (vcc - output_v) / self.r_pullup
        if current < 0 then current = 0 end
    end

    local power = current * vcc
    local delay = 10e-9  -- 10 ns typical for TTL

    return types.GateOutput({
        logic_value       = logic_value,
        voltage           = output_v,
        current_draw      = current,
        power_dissipation = power,
        propagation_delay = delay,
        transistor_count  = 3,
    })
end

function TTLNand:evaluate_digital(a, b)
    local err = types.validate_bit(a, "a")
    if err then return nil, err end
    err = types.validate_bit(b, "b")
    if err then return nil, err end
    local va = a == 1 and self.vcc or 0.0
    local vb = b == 1 and self.vcc or 0.0
    return self:evaluate(va, vb).logic_value, nil
end

--- Static power dissipation -- significantly higher than CMOS.
-- TTL gates consume power continuously due to resistor-based biasing.
-- Worst case is when output is LOW.
function TTLNand:static_power()
    local current = (self.vcc - 2 * self.params.vbe_on - self.params.vce_sat) / self.r_pullup
    if current < 0 then current = 0 end
    return current * self.vcc
end

-- ==========================================================================
-- RTL INVERTER -- the earliest IC logic family
-- ==========================================================================
--
-- Circuit:
--
--       Vcc
--        |
--       Rc (collector resistor, ~1 k ohm)
--        |
--   +----+----+
--   |  Q1     |  Single NPN transistor
--   +----+----+
--        |
--       GND
--
--   Input -- Rb (base resistor, ~10 k ohm) -- Base of Q1
--
-- Input HIGH: Q1 saturates -> output LOW.
-- Input LOW:  Q1 cutoff -> output pulled HIGH through Rc.
--
-- RTL was used in the Apollo Guidance Computer (AGC) that navigated
-- Apollo 11 to the moon in 1969.

local RTLInverter = {}
RTLInverter.__index = RTLInverter

function ttl.RTLInverter(vcc, r_base, r_collector, params)
    local self = setmetatable({}, RTLInverter)
    self.vcc         = vcc or 3.3
    self.r_base      = r_base or 10000.0
    self.r_collector = r_collector or 1000.0
    self.params      = params or types.BJTParams()
    self.q1          = bjt.NPN(self.params)
    return self
end

function RTLInverter:evaluate(v_input)
    local vcc    = self.vcc
    local vbe_on = self.params.vbe_on

    local output_v, logic_value, current

    if v_input > vbe_on then
        -- Q1 is ON -- calculate base current and check saturation
        local ib = (v_input - vbe_on) / self.r_base

        -- Collector current: min of beta*Ib and circuit-limited current
        local ic_max = (vcc - self.params.vce_sat) / self.r_collector
        local ic     = ib * self.params.beta
        if ic > ic_max then ic = ic_max end

        output_v = vcc - ic * self.r_collector
        if output_v < self.params.vce_sat then
            output_v = self.params.vce_sat
        end

        if output_v < vcc / 2.0 then
            logic_value = 0
        else
            logic_value = 1
        end
        current = ic + ib
    else
        -- Q1 is OFF -- output pulled to Vcc through Rc
        output_v    = vcc
        logic_value = 1
        current     = 0.0
    end

    local power = current * vcc
    local delay = 50e-9  -- RTL is slow: ~50 ns typical

    return types.GateOutput({
        logic_value       = logic_value,
        voltage           = output_v,
        current_draw      = current,
        power_dissipation = power,
        propagation_delay = delay,
        transistor_count  = 1,
    })
end

function RTLInverter:evaluate_digital(a)
    local err = types.validate_bit(a, "a")
    if err then return nil, err end
    local v_input = a == 1 and self.vcc or 0.0
    return self:evaluate(v_input).logic_value, nil
end

return ttl

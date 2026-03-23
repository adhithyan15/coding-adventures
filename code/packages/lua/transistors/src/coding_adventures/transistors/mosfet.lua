-- mosfet.lua -- NMOS and PMOS transistor simulation
--
-- === What is a MOSFET? ===
--
-- MOSFET stands for Metal-Oxide-Semiconductor Field-Effect Transistor.
-- It is the most common type of transistor in the world -- every CPU,
-- GPU, and phone chip is built from billions of MOSFETs.
--
-- A MOSFET has three terminals:
--
--   Gate (G):   The control terminal.  Voltage here controls the switch.
--   Drain (D):  Current flows IN here (for NMOS) or OUT here (for PMOS).
--   Source (S): Current flows OUT here (for NMOS) or IN here (for PMOS).
--
-- The key insight: a MOSFET is VOLTAGE-controlled.  Applying a voltage to
-- the gate creates an electric field that either allows or blocks current
-- flow between drain and source.  No current flows into the gate itself
-- (it is insulated by a thin oxide layer), which means:
--   - Near-zero input power consumption
--   - Very high input impedance (good for amplifiers)
--   - Can be packed extremely densely on a chip
--
-- === NMOS vs PMOS ===
--
--   NMOS: Gate HIGH -> ON  (conducts drain to source)
--   PMOS: Gate LOW  -> ON  (conducts source to drain)
--
-- This complementary behavior is the foundation of CMOS (Complementary
-- MOS) logic.  By pairing NMOS and PMOS transistors we can build gates
-- that consume near-zero power in steady state.

local types = require("coding_adventures.transistors.types")

local mosfet = {}

-- ==========================================================================
-- NMOS -- N-channel MOSFET
-- ==========================================================================
--
-- An NMOS transistor conducts current from drain to source when the gate
-- voltage exceeds the threshold voltage (Vgs > Vth).  Think of it as a
-- normally-OPEN switch that CLOSES when you apply voltage to the gate.
--
-- In a digital circuit, NMOS connects the output to GROUND:
--
--   Output --|
--            | NMOS (gate = input signal)
--            |
--           GND
--
--   Input HIGH -> NMOS ON  -> output pulled to GND (LOW)
--   Input LOW  -> NMOS OFF -> output disconnected from GND

local NMOS = {}
NMOS.__index = NMOS

--- Create a new NMOS transistor.
-- @param params  optional MOSFETParams override table
-- @return NMOS instance
function mosfet.NMOS(params)
    local self = setmetatable({}, NMOS)
    self.params = params or types.MOSFETParams()
    return self
end

--- Determine the operating region given terminal voltages.
--
-- The operating region determines which equations govern current flow:
--
--   Cutoff:     Vgs < Vth            (gate voltage below threshold)
--   Linear:     Vgs >= Vth AND Vds < Vgs - Vth
--   Saturation: Vgs >= Vth AND Vds >= Vgs - Vth
--
-- @param vgs  Gate-to-source voltage (volts)
-- @param vds  Drain-to-source voltage (volts)
-- @return string  One of types.MOSFET_CUTOFF, MOSFET_LINEAR, MOSFET_SATURATION
function NMOS:region(vgs, vds)
    local vth = self.params.vth
    if vgs < vth then
        return types.MOSFET_CUTOFF
    end
    -- Overdrive voltage: how far above threshold the gate is driven.
    local vov = vgs - vth
    if vds < vov then
        return types.MOSFET_LINEAR
    end
    return types.MOSFET_SATURATION
end

--- Calculate drain-to-source current (Ids) in amperes.
--
-- Uses the simplified MOSFET current equations (Shockley model):
--
--   Cutoff:     Ids = 0  (no channel, no current)
--   Linear:     Ids = K * ((Vgs - Vth) * Vds - 0.5 * Vds^2)
--   Saturation: Ids = 0.5 * K * (Vgs - Vth)^2
--
-- @param vgs  Gate-to-source voltage
-- @param vds  Drain-to-source voltage
-- @return number  Current in amperes
function NMOS:drain_current(vgs, vds)
    local reg = self:region(vgs, vds)
    local k   = self.params.k
    local vth = self.params.vth

    if reg == types.MOSFET_CUTOFF then
        return 0.0
    end

    local vov = vgs - vth  -- overdrive voltage

    if reg == types.MOSFET_LINEAR then
        -- Linear / ohmic region: transistor acts like a voltage-controlled
        -- resistor.  Current increases with both Vgs and Vds.
        return k * (vov * vds - 0.5 * vds * vds)
    end

    -- Saturation region: channel is "pinched off" at the drain end.
    -- Current depends only on Vgs, not Vds.  This is why saturation is
    -- used for amplifiers -- output current is controlled solely by input
    -- voltage.
    return 0.5 * k * vov * vov
end

--- Returns true when the gate voltage exceeds the threshold.
-- This is the simplified digital view: ON or OFF, no in-between.
function NMOS:is_conducting(vgs)
    return vgs >= self.params.vth
end

--- Output voltage when used as a pull-down switch in a CMOS circuit.
--
--   ON:  output ~ 0 V   (pulled to ground through low-resistance channel)
--   OFF: output ~ Vdd   (pulled up by the PMOS network)
function NMOS:output_voltage(vgs, vdd)
    if self:is_conducting(vgs) then
        return 0.0
    end
    return vdd
end

--- Small-signal transconductance gm = dIds / dVgs.
--
-- This is the key parameter for amplifier design.  In saturation:
--   gm = K * (Vgs - Vth)
--
-- Higher gm = more gain, but also more power consumption.
function NMOS:transconductance(vgs, vds)
    local reg = self:region(vgs, vds)
    if reg == types.MOSFET_CUTOFF then
        return 0.0
    end
    local vov = vgs - self.params.vth
    return self.params.k * vov
end

-- ==========================================================================
-- PMOS -- P-channel MOSFET
-- ==========================================================================
--
-- A PMOS transistor is the complement of NMOS.  It conducts current from
-- source to drain when the gate voltage is LOW (below the source voltage
-- by more than |Vth|).  Think of it as a normally-CLOSED switch that OPENS
-- when you apply voltage.
--
-- PMOS transistors form the pull-UP network in CMOS gates:
--
--   Vdd
--    |
--    | PMOS (gate = input signal)
--    |
--   Output
--
--   Input LOW  -> PMOS ON  -> output pulled to Vdd (HIGH)
--   Input HIGH -> PMOS OFF -> output disconnected from Vdd
--
-- PMOS uses the same equations as NMOS, but with reversed voltage
-- polarities.  For PMOS, Vgs and Vds are typically negative.

local PMOS = {}
PMOS.__index = PMOS

--- Create a new PMOS transistor.
function mosfet.PMOS(params)
    local self = setmetatable({}, PMOS)
    self.params = params or types.MOSFETParams()
    return self
end

--- Operating region for PMOS using absolute values.
--
--   Cutoff:     |Vgs| < Vth
--   Linear:     |Vgs| >= Vth AND |Vds| < |Vgs| - Vth
--   Saturation: |Vgs| >= Vth AND |Vds| >= |Vgs| - Vth
function PMOS:region(vgs, vds)
    local vth     = self.params.vth
    local abs_vgs = math.abs(vgs)
    local abs_vds = math.abs(vds)

    if abs_vgs < vth then
        return types.MOSFET_CUTOFF
    end
    local vov = abs_vgs - vth
    if abs_vds < vov then
        return types.MOSFET_LINEAR
    end
    return types.MOSFET_SATURATION
end

--- Drain current for PMOS (source-to-drain).
-- Same equations as NMOS but using absolute values.
-- Current magnitude is returned (always >= 0).
function PMOS:drain_current(vgs, vds)
    local reg = self:region(vgs, vds)
    local k   = self.params.k
    local vth = self.params.vth

    if reg == types.MOSFET_CUTOFF then
        return 0.0
    end

    local abs_vgs = math.abs(vgs)
    local abs_vds = math.abs(vds)
    local vov     = abs_vgs - vth

    if reg == types.MOSFET_LINEAR then
        return k * (vov * abs_vds - 0.5 * abs_vds * abs_vds)
    end

    return 0.5 * k * vov * vov
end

--- Returns true when |Vgs| >= Vth.
-- PMOS turns ON when the gate is pulled below the source.
function PMOS:is_conducting(vgs)
    return math.abs(vgs) >= self.params.vth
end

--- Output voltage when used as a pull-up switch.
--
--   ON:  output ~ Vdd (pulled to supply through low-resistance channel)
--   OFF: output ~ 0 V (pulled down by NMOS network)
function PMOS:output_voltage(vgs, vdd)
    if self:is_conducting(vgs) then
        return vdd
    end
    return 0.0
end

--- Small-signal transconductance gm for PMOS.
-- Same formula as NMOS but using absolute values.
function PMOS:transconductance(vgs, vds)
    local reg = self:region(vgs, vds)
    if reg == types.MOSFET_CUTOFF then
        return 0.0
    end
    local vov = math.abs(vgs) - self.params.vth
    return self.params.k * vov
end

return mosfet

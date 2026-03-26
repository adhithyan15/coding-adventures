-- bjt.lua -- NPN and PNP bipolar junction transistor simulation
--
-- === What is a BJT? ===
--
-- BJT stands for Bipolar Junction Transistor.  Invented in 1947 at Bell
-- Labs by John Bardeen, Walter Brattain, and William Shockley, the BJT
-- replaced vacuum tubes and launched the electronics revolution.
--
-- A BJT has three terminals:
--
--   Base (B):      The control terminal.  Current here controls the switch.
--   Collector (C): Current flows IN here (for NPN) or OUT here (for PNP).
--   Emitter (E):   Current flows OUT here (for NPN) or IN here (for PNP).
--
-- The key difference from MOSFETs: a BJT is CURRENT-controlled.  You must
-- supply a continuous current to the base to keep it on.  This means:
--   - Base current = wasted power (even in steady state)
--   - Lower input impedance than MOSFETs
--   - But historically faster switching (before CMOS caught up)
--
-- === The Current Gain (beta) ===
--
--   Ic = beta * Ib
--
-- A tiny base current (microamps) controls a much larger collector current
-- (milliamps).  This amplification property made radios, televisions, and
-- early computers possible.

local types = require("coding_adventures.transistors.types")

local bjt = {}

-- ==========================================================================
-- NPN -- NPN bipolar junction transistor
-- ==========================================================================
--
-- An NPN transistor turns ON when current flows into the base terminal
-- (Vbe > ~0.7 V).  A small base current controls a much larger collector
-- current through the current gain: Ic = beta * Ib.
--
-- Operating regions:
--
--   CUTOFF:     Vbe < 0.7 V -> no current -> switch OFF.
--   ACTIVE:     Vbe >= 0.7 V, Vce > 0.2 V -> linear amplifier.
--   SATURATION: Vbe >= 0.7 V, Vce <= 0.2 V -> fully ON (switch).

local NPN = {}
NPN.__index = NPN

--- Create a new NPN transistor.
-- @param params  optional BJTParams override table
function bjt.NPN(params)
    local self = setmetatable({}, NPN)
    self.params = params or types.BJTParams()
    return self
end

--- Determine the operating region from terminal voltages.
--
--   Cutoff:     Vbe < VbeOn
--   Saturation: Vbe >= VbeOn AND Vce <= VceSat
--   Active:     Vbe >= VbeOn AND Vce > VceSat
function NPN:region(vbe, vce)
    if vbe < self.params.vbe_on then
        return types.BJT_CUTOFF
    end
    if vce <= self.params.vce_sat then
        return types.BJT_SATURATION
    end
    return types.BJT_ACTIVE
end

--- Calculate collector current (Ic) in amperes.
--
-- Uses the simplified Ebers-Moll model:
--
--   Cutoff:             Ic = 0
--   Active/Saturation:  Ic = Is * (exp(Vbe / Vt) - 1)
--
-- where Vt = kT/q ~ 26 mV at room temperature.
-- The exponent is clamped to 40 to prevent floating-point overflow.
function NPN:collector_current(vbe, vce)
    local reg = self:region(vbe, vce)
    if reg == types.BJT_CUTOFF then
        return 0.0
    end

    -- Thermal voltage: Vt = kT/q ~ 26 mV at room temperature.
    local vt = 0.026

    -- Ebers-Moll: the exponential relationship is why BJTs are such
    -- good amplifiers -- a small change in Vbe causes a large change in Ic.
    local exponent = math.min(vbe / vt, 40.0)  -- clamp to prevent overflow
    return self.params.is * (math.exp(exponent) - 1.0)
end

--- Calculate base current (Ib) in amperes.
--
-- Ib = Ic / beta in the active region.
--
-- This is the "wasted" current that makes BJTs less efficient than
-- MOSFETs for digital logic.  Every TTL gate has base current flowing
-- continuously, consuming significant power.
function NPN:base_current(vbe, vce)
    local ic = self:collector_current(vbe, vce)
    if ic == 0.0 then
        return 0.0
    end
    return ic / self.params.beta
end

--- Returns true when Vbe >= VbeOn (~0.7 V for silicon).
function NPN:is_conducting(vbe)
    return vbe >= self.params.vbe_on
end

--- Small-signal transconductance gm = Ic / Vt.
--
-- BJTs typically have higher gm than MOSFETs for the same current,
-- which is why they are still preferred for some analog applications.
function NPN:transconductance(vbe, vce)
    local ic = self:collector_current(vbe, vce)
    if ic == 0.0 then
        return 0.0
    end
    local vt = 0.026
    return ic / vt
end

-- ==========================================================================
-- PNP -- PNP bipolar junction transistor
-- ==========================================================================
--
-- The complement of NPN.  A PNP transistor turns ON when the base is
-- pulled LOW relative to the emitter (|Vbe| >= VbeOn).  Current flows
-- from emitter to collector.
--
-- For PNP, the "natural" voltages are reversed from NPN:
--   - Vbe is typically NEGATIVE (base below emitter)
--   - Vce is typically NEGATIVE (collector below emitter)
--
-- We use absolute values internally, same as PMOS.

local PNP = {}
PNP.__index = PNP

--- Create a new PNP transistor.
function bjt.PNP(params)
    local self = setmetatable({}, PNP)
    self.params = params or types.BJTParams()
    return self
end

--- Operating region for PNP using absolute values.
function PNP:region(vbe, vce)
    local abs_vbe = math.abs(vbe)
    local abs_vce = math.abs(vce)

    if abs_vbe < self.params.vbe_on then
        return types.BJT_CUTOFF
    end
    if abs_vce <= self.params.vce_sat then
        return types.BJT_SATURATION
    end
    return types.BJT_ACTIVE
end

--- Collector current magnitude for PNP.
-- Same equations as NPN but using absolute values.  Returns >= 0.
function PNP:collector_current(vbe, vce)
    local reg = self:region(vbe, vce)
    if reg == types.BJT_CUTOFF then
        return 0.0
    end

    local abs_vbe = math.abs(vbe)
    local vt = 0.026

    local exponent = math.min(abs_vbe / vt, 40.0)
    return self.params.is * (math.exp(exponent) - 1.0)
end

--- Base current magnitude for PNP.
function PNP:base_current(vbe, vce)
    local ic = self:collector_current(vbe, vce)
    if ic == 0.0 then
        return 0.0
    end
    return ic / self.params.beta
end

--- Returns true when |Vbe| >= VbeOn (base pulled below emitter).
function PNP:is_conducting(vbe)
    return math.abs(vbe) >= self.params.vbe_on
end

--- Small-signal transconductance gm for PNP.
function PNP:transconductance(vbe, vce)
    local ic = self:collector_current(vbe, vce)
    if ic == 0.0 then
        return 0.0
    end
    local vt = 0.026
    return ic / vt
end

return bjt

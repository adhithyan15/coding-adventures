-- types.lua -- Constants, parameter structs, and result types for transistor simulation
--
-- This module defines the vocabulary shared by all other modules in the
-- transistors package.  Every struct in the Go implementation becomes a Lua
-- table constructor here, and every set of constants becomes a flat string
-- constant.  Lua is duck-typed, so "structs" are simply tables with
-- documented keys -- metatables give us nice constructors and defaults.
--
-- ==========================================================================
-- OPERATING REGION CONSTANTS
-- ==========================================================================
--
-- A transistor is an analog device that operates differently depending on
-- the voltages applied to its terminals.  The three "regions" describe
-- these different operating modes.

local types = {}

-- ---------------------------------------------------------------------------
-- MOSFET operating regions
-- ---------------------------------------------------------------------------
--
-- Think of a MOSFET like a water faucet with three positions:
--
--   CUTOFF:     Faucet fully closed.  No water flows.
--               (Vgs < Vth -- gate voltage too low to turn on)
--
--   LINEAR:     Faucet open; water flow increases as you turn the handle
--               more.  Flow is proportional to both handle position AND
--               water pressure.
--               (Vgs > Vth, Vds < Vgs - Vth -- acts like a resistor)
--
--   SATURATION: Faucet wide open, but the pipe is the bottleneck.
--               Adding more pressure doesn't increase flow much.
--               (Vgs > Vth, Vds >= Vgs - Vth -- current roughly constant)
--
-- For digital circuits we only use CUTOFF (OFF) and deep LINEAR (ON).
-- For analog amplifiers we operate in SATURATION.

types.MOSFET_CUTOFF     = "cutoff"
types.MOSFET_LINEAR     = "linear"
types.MOSFET_SATURATION = "saturation"

-- ---------------------------------------------------------------------------
-- BJT operating regions
-- ---------------------------------------------------------------------------
--
-- Similar to MOSFET regions but with different names and physics:
--
--   CUTOFF:     No base current -> no collector current.  Switch OFF.
--               (Vbe < ~0.7 V)
--
--   ACTIVE:     Small base current, large collector current.
--               Ic = beta * Ib.  This is the AMPLIFIER region.
--               (Vbe >= ~0.7 V, Vce > ~0.2 V)
--
--   SATURATION: Both junctions forward-biased.  Collector current is
--               maximum -- transistor is fully ON as a switch.
--               (Vbe >= ~0.7 V, Vce <= ~0.2 V)
--
-- Confusing naming alert:
--   MOSFET "saturation" = constant current (amplifier).
--   BJT    "saturation" = fully ON (switch).
-- These are DIFFERENT behaviors despite sharing a name.

types.BJT_CUTOFF     = "cutoff"
types.BJT_ACTIVE     = "active"
types.BJT_SATURATION = "saturation"

-- ==========================================================================
-- ELECTRICAL PARAMETERS
-- ==========================================================================

-- ---------------------------------------------------------------------------
-- MOSFETParams
-- ---------------------------------------------------------------------------
-- Holds the physical characteristics of a MOSFET transistor.
--
-- Default values represent a typical 180 nm CMOS process -- the last
-- "large" process node still widely used in education and analog /
-- mixed-signal chips.
--
-- Key parameters:
--   Vth    -- Threshold voltage.  Minimum Vgs to turn the transistor ON.
--   K      -- Transconductance parameter.  K = mu * Cox * (W/L).
--   W, L   -- Channel width and length.  W/L ratio tunes transistor strength.
--   c_gate -- Gate capacitance.  Determines switching speed.
--   c_drain-- Drain junction capacitance.  Contributes to output load.

local MOSFETParams = {}
MOSFETParams.__index = MOSFETParams

function types.MOSFETParams(overrides)
    local self = setmetatable({}, MOSFETParams)
    -- 180 nm defaults
    self.vth     = 0.4
    self.k       = 0.001
    self.w       = 1e-6
    self.l       = 180e-9
    self.c_gate  = 1e-15
    self.c_drain = 0.5e-15
    if overrides then
        for key, val in pairs(overrides) do
            self[key] = val
        end
    end
    return self
end

-- ---------------------------------------------------------------------------
-- BJTParams
-- ---------------------------------------------------------------------------
-- Holds the physical characteristics of a BJT transistor.
--
-- Default values represent a typical 2N2222-style NPN transistor -- one of
-- the most common transistors ever made, used in everything from hobby
-- projects to early spacecraft.
--
-- Key parameters:
--   beta    -- Current gain (hfe).  Ic / Ib.  beta=100 means 1 mA of base
--              current controls 100 mA of collector current.
--   vbe_on  -- Base-emitter voltage when conducting (~0.6-0.7 V for silicon).
--   vce_sat -- Collector-emitter voltage when fully saturated (~0.1-0.3 V).
--   is      -- Reverse saturation current (leakage when OFF).
--   c_base  -- Base capacitance.  Limits switching speed.

local BJTParams = {}
BJTParams.__index = BJTParams

function types.BJTParams(overrides)
    local self = setmetatable({}, BJTParams)
    -- 2N2222 defaults
    self.beta    = 100.0
    self.vbe_on  = 0.7
    self.vce_sat = 0.2
    self.is      = 1e-14
    self.c_base  = 5e-12
    if overrides then
        for key, val in pairs(overrides) do
            self[key] = val
        end
    end
    return self
end

-- ---------------------------------------------------------------------------
-- CircuitParams
-- ---------------------------------------------------------------------------
-- Holds parameters for a complete logic gate circuit.
--
--   vdd         -- Supply voltage.  Modern CMOS: 0.7-1.2 V.
--                  Older CMOS: 3.3 V or 5 V.  TTL always 5 V.
--   temperature -- Junction temperature in Kelvin.  Room temp ~ 300 K.

local CircuitParams = {}
CircuitParams.__index = CircuitParams

function types.CircuitParams(overrides)
    local self = setmetatable({}, CircuitParams)
    self.vdd         = 3.3
    self.temperature = 300.0
    if overrides then
        for key, val in pairs(overrides) do
            self[key] = val
        end
    end
    return self
end

-- ==========================================================================
-- RESULT TYPES
-- ==========================================================================

-- ---------------------------------------------------------------------------
-- GateOutput
-- ---------------------------------------------------------------------------
-- Result of evaluating a logic gate with voltage-level detail.
--
-- Unlike a pure Boolean logic gate that returns 0 or 1, this gives the
-- full electrical picture: output voltage, current draw, power, delay.

function types.GateOutput(fields)
    return {
        logic_value        = fields.logic_value        or 0,
        voltage            = fields.voltage            or 0.0,
        current_draw       = fields.current_draw       or 0.0,
        power_dissipation  = fields.power_dissipation  or 0.0,
        propagation_delay  = fields.propagation_delay  or 0.0,
        transistor_count   = fields.transistor_count   or 0,
    }
end

-- ---------------------------------------------------------------------------
-- AmplifierAnalysis
-- ---------------------------------------------------------------------------
-- Result of analyzing a transistor as an amplifier.
--
--   voltage_gain      -- How much the output voltage changes per unit input.
--   transconductance  -- gm: output current change / input voltage change.
--   input_impedance   -- How much the amplifier "loads" the signal source.
--   output_impedance  -- How "stiff" the output is.
--   bandwidth         -- Frequency at which gain drops to 70.7 % (-3 dB).
--   operating_point   -- DC bias conditions (table of name -> value).

function types.AmplifierAnalysis(fields)
    return {
        voltage_gain      = fields.voltage_gain      or 0.0,
        transconductance  = fields.transconductance  or 0.0,
        input_impedance   = fields.input_impedance   or 0.0,
        output_impedance  = fields.output_impedance  or 0.0,
        bandwidth         = fields.bandwidth         or 0.0,
        operating_point   = fields.operating_point   or {},
    }
end

-- ---------------------------------------------------------------------------
-- NoiseMargins
-- ---------------------------------------------------------------------------
-- How much electrical noise a digital signal can tolerate before being
-- misinterpreted.
--
--   vol -- Output LOW voltage
--   voh -- Output HIGH voltage
--   vil -- Input LOW threshold (max voltage accepted as 0)
--   vih -- Input HIGH threshold (min voltage accepted as 1)
--   nml -- Noise Margin LOW  = vil - vol
--   nmh -- Noise Margin HIGH = voh - vih

function types.NoiseMargins(fields)
    return {
        vol = fields.vol or 0.0,
        voh = fields.voh or 0.0,
        vil = fields.vil or 0.0,
        vih = fields.vih or 0.0,
        nml = fields.nml or 0.0,
        nmh = fields.nmh or 0.0,
    }
end

-- ---------------------------------------------------------------------------
-- PowerAnalysis
-- ---------------------------------------------------------------------------
--   static_power      -- Power consumed when not switching (leakage).
--   dynamic_power     -- Power consumed during switching (C * Vdd^2 * f * alpha).
--   total_power       -- Static + Dynamic.
--   energy_per_switch -- Energy for one 0->1->0 transition (C * Vdd^2).

function types.PowerAnalysis(fields)
    return {
        static_power      = fields.static_power      or 0.0,
        dynamic_power     = fields.dynamic_power     or 0.0,
        total_power       = fields.total_power       or 0.0,
        energy_per_switch = fields.energy_per_switch or 0.0,
    }
end

-- ---------------------------------------------------------------------------
-- TimingAnalysis
-- ---------------------------------------------------------------------------
--   tphl          -- Propagation delay HIGH to LOW output.
--   tplh          -- Propagation delay LOW to HIGH output.
--   tpd           -- Average propagation delay = (tphl + tplh) / 2.
--   rise_time     -- Time for output to go from 10 % to 90 % of Vdd.
--   fall_time     -- Time for output to go from 90 % to 10 % of Vdd.
--   max_frequency -- Maximum clock frequency = 1 / (2 * tpd).

function types.TimingAnalysis(fields)
    return {
        tphl          = fields.tphl          or 0.0,
        tplh          = fields.tplh          or 0.0,
        tpd           = fields.tpd           or 0.0,
        rise_time     = fields.rise_time     or 0.0,
        fall_time     = fields.fall_time     or 0.0,
        max_frequency = fields.max_frequency or 0.0,
    }
end

-- ---------------------------------------------------------------------------
-- validate_bit -- helper shared across gate modules
-- ---------------------------------------------------------------------------
-- Returns nil on success, or an error string if value is not 0 or 1.

function types.validate_bit(value, name)
    if value ~= 0 and value ~= 1 then
        return string.format("%s must be 0 or 1, got %s", name, tostring(value))
    end
    return nil
end

return types

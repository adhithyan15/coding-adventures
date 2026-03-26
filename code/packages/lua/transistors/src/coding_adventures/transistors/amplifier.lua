-- amplifier.lua -- Transistors as analog signal amplifiers
--
-- === Beyond Digital: Transistors as Amplifiers ===
--
-- A transistor used as a digital switch operates in only two states: ON/OFF.
-- But transistors are fundamentally ANALOG devices.  When biased in the
-- right operating region (saturation for MOSFET, active for BJT), they can
-- amplify small signals into larger ones.
--
-- === Common-Source Amplifier (MOSFET) ===
--
--       Vdd
--        |
--       [Rd]  <- voltage drop = Ids x Rd
--        |
--   -----| Drain (output)
--   Gate -||
--   -----| Source
--        |
--       GND
--
--   Voltage gain: Av = -gm x Rd  (inverting amplifier)
--
-- === Common-Emitter Amplifier (BJT) ===
--
--       Vcc
--        |
--       [Rc]
--        |
--   -----| Collector (output)
--   Base -|-->
--   -----| Emitter
--        |
--       GND
--
--   Voltage gain: Av = -gm x Rc = -(Ic / Vt) x Rc

local types = require("coding_adventures.transistors.types")

local amplifier = {}

--- Analyze an NMOS common-source amplifier.
--
-- The input signal is applied to the gate and the output is taken from
-- the drain.  A drain resistor (r_drain) converts drain current variation
-- into a voltage swing.
--
-- For the amplifier to work, the MOSFET must be biased in SATURATION:
-- Vgs > Vth AND Vds >= Vgs - Vth.
--
-- @param t        NMOS transistor instance
-- @param vgs      Gate-to-source bias voltage
-- @param vdd      Supply voltage
-- @param r_drain  Drain resistor value (ohms)
-- @param c_load   Load capacitance (farads)
-- @return AmplifierAnalysis
function amplifier.analyze_common_source(t, vgs, vdd, r_drain, c_load)
    -- Calculate DC operating point
    local ids = t:drain_current(vgs, vdd)       -- approximate: Vds ~ Vdd initially
    local vds = vdd - ids * r_drain              -- actual drain voltage

    -- Recalculate with correct Vds
    local vds_actual = vds
    if vds_actual < 0 then vds_actual = 0 end
    ids = t:drain_current(vgs, vds_actual)
    vds = vdd - ids * r_drain

    -- Transconductance
    local gm = t:transconductance(vgs, vds_actual)

    -- Voltage gain: Av = -gm x Rd (negative = inverting)
    local voltage_gain = -gm * r_drain

    -- Input impedance: essentially infinite for MOSFET (gate is insulated)
    local input_impedance = 1e12  -- 1 T-ohm

    -- Output impedance: approximately Rd
    local output_impedance = r_drain

    -- Bandwidth: f_3dB = 1 / (2 * pi * Rd * C_load)
    local bandwidth = 1.0 / (2.0 * math.pi * r_drain * c_load)

    local operating_point = {
        vgs = vgs,
        vds = vds,
        ids = ids,
        gm  = gm,
    }

    return types.AmplifierAnalysis({
        voltage_gain     = voltage_gain,
        transconductance = gm,
        input_impedance  = input_impedance,
        output_impedance = output_impedance,
        bandwidth        = bandwidth,
        operating_point  = operating_point,
    })
end

--- Analyze an NPN common-emitter amplifier.
--
-- BJT amplifiers typically have higher voltage gain than MOSFET amplifiers
-- at the same current, because BJT transconductance (gm = Ic / Vt) is
-- higher than MOSFET transconductance for the same bias current.
--
-- However, BJT amplifiers have lower input impedance because base current
-- flows continuously.
--
-- @param t            NPN transistor instance
-- @param vbe          Base-emitter bias voltage
-- @param vcc          Supply voltage
-- @param r_collector  Collector resistor value (ohms)
-- @param c_load       Load capacitance (farads)
-- @return AmplifierAnalysis
function amplifier.analyze_common_emitter(t, vbe, vcc, r_collector, c_load)
    -- Calculate DC operating point
    local vce = vcc  -- initial approximation
    local ic  = t:collector_current(vbe, vce)
    vce = vcc - ic * r_collector
    if vce < 0 then vce = 0 end

    -- Recalculate with correct Vce
    ic = t:collector_current(vbe, vce)

    -- Transconductance
    local gm = t:transconductance(vbe, vce)

    -- Voltage gain: Av = -gm x Rc
    local voltage_gain = -gm * r_collector

    -- Input impedance: r_pi = beta * Vt / Ic
    local beta = t.params.beta
    local vt   = 0.026
    local r_pi
    if ic > 0 then
        r_pi = beta * vt / ic
    else
        r_pi = 1e12  -- very high when no current flows
    end

    local input_impedance  = r_pi
    local output_impedance = r_collector

    -- Bandwidth: f_3dB = 1 / (2 * pi * Rc * C_load)
    local bandwidth = 1.0 / (2.0 * math.pi * r_collector * c_load)

    local operating_point = {
        vbe = vbe,
        vce = vce,
        ic  = ic,
        ib  = t:base_current(vbe, vce),
        gm  = gm,
    }

    return types.AmplifierAnalysis({
        voltage_gain     = voltage_gain,
        transconductance = gm,
        input_impedance  = input_impedance,
        output_impedance = output_impedance,
        bandwidth        = bandwidth,
        operating_point  = operating_point,
    })
end

return amplifier

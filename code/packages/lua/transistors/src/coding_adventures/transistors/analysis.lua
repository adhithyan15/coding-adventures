-- analysis.lua -- Noise margins, power, timing, and technology comparison
--
-- Digital logic designers don't just care about truth tables -- they care
-- about:
--
-- 1. NOISE MARGINS: Can the circuit tolerate voltage fluctuations?
--    A chip has billions of wires running millimeters apart, each creating
--    electromagnetic interference on its neighbors.
--
-- 2. POWER: How much energy does the chip consume?  Power is the #1
--    constraint in modern chip design.
--
-- 3. TIMING: How fast can the circuit switch?  The propagation delay
--    through a gate determines the maximum clock frequency.
--
-- 4. SCALING: How do these properties change as we shrink transistors?
--    Moore's Law predicts transistor count doubles every ~2 years.

local types      = require("coding_adventures.transistors.types")
local mosfet_mod = require("coding_adventures.transistors.mosfet")

local analysis = {}

-- ---------------------------------------------------------------------------
-- Internal helper: extract circuit parameters from a gate table.
-- We use duck-typing: if it has a .circuit field it is CMOS; if it has
-- a .vcc field it is TTL.
-- ---------------------------------------------------------------------------

local function extract_gate_info(gate)
    if gate.circuit then
        -- CMOS gate (CMOSInverter, CMOSNand, CMOSNor, ...)
        local nmos = gate.nmos or gate.nmos1
        local pmos = gate.pmos or gate.pmos1
        return {
            is_cmos    = true,
            vdd        = gate.circuit.vdd,
            nmos       = nmos,
            pmos       = pmos,
            static_pow = 0.0,
        }
    elseif gate.vcc then
        -- TTL gate
        return {
            is_cmos    = false,
            vdd        = gate.vcc,
            static_pow = gate:static_power(),
        }
    else
        return nil, "unsupported gate type"
    end
end

-- ---------------------------------------------------------------------------
-- Noise Margins
-- ---------------------------------------------------------------------------

--- Compute noise margins for a gate.
--
-- Noise margins tell you how much electrical noise a digital signal can
-- tolerate before being misinterpreted by the next gate.
--
-- For CMOS:
--   VOL ~ 0 V, VOH ~ Vdd  -> large noise margins
--   NML ~ NMH ~ 0.4 * Vdd  (symmetric)
--
-- For TTL:
--   VOL ~ 0.2 V, VOH ~ 3.5 V -> smaller margins
--   VIL = 0.8 V, VIH = 2.0 V  (defined by spec)
--
-- Supported: CMOSInverter, TTLNand.
-- @return NoiseMargins, err_string
function analysis.compute_noise_margins(gate)
    local vol, voh, vil, vih

    -- Duck-type: CMOSInverter has .nmos and .pmos (not .nmos1)
    if gate.circuit and gate.nmos and gate.pmos and not gate.nmos1 then
        local vdd = gate.circuit.vdd
        vol = 0.0
        voh = vdd
        vil = 0.4 * vdd
        vih = 0.6 * vdd
    elseif gate.vcc and gate.q1 then
        -- TTLNand
        vol = 0.2
        voh = gate.vcc - 0.7
        vil = 0.8
        vih = 2.0
    else
        return nil, "unsupported gate type"
    end

    local nml = vil - vol
    local nmh = voh - vih

    return types.NoiseMargins({
        vol = vol, voh = voh,
        vil = vil, vih = vih,
        nml = nml, nmh = nmh,
    }), nil
end

-- ---------------------------------------------------------------------------
-- Power Analysis
-- ---------------------------------------------------------------------------

--- Analyze power consumption for a gate at a given frequency.
--
-- === Power in CMOS ===
--   P_total   = P_static + P_dynamic
--   P_static  ~ negligible (nanowatts)
--   P_dynamic = C_load * Vdd^2 * f * activity_factor
--
-- === Power in TTL ===
--   P_static  ~ milliwatts (DOMINATES!)
--   P_dynamic = similar formula but static power is so large it barely matters
--
-- @return PowerAnalysis, err_string
function analysis.analyze_power(gate, frequency, c_load, activity_factor)
    local info, err = extract_gate_info(gate)
    if not info then return nil, err end

    local static_power = info.static_pow
    local vdd          = info.vdd

    -- Dynamic power: P = C * V^2 * f * alpha
    local dynamic = c_load * vdd * vdd * frequency * activity_factor
    local total   = static_power + dynamic

    -- Energy per switching event: E = C * V^2
    local energy_per_switch = c_load * vdd * vdd

    return types.PowerAnalysis({
        static_power      = static_power,
        dynamic_power     = dynamic,
        total_power       = total,
        energy_per_switch = energy_per_switch,
    }), nil
end

-- ---------------------------------------------------------------------------
-- Timing Analysis
-- ---------------------------------------------------------------------------

-- Internal: calculate CMOS timing parameters from transistor characteristics.
local function cmos_timing(vdd, nmos, pmos, c_load)
    local k   = nmos.params.k
    local vth = nmos.params.vth

    -- Saturation current for NMOS pull-down
    local ids_sat_n = 1e-12
    if vdd > vth then
        ids_sat_n = 0.5 * k * (vdd - vth) * (vdd - vth)
    end

    -- Saturation current for PMOS pull-up
    local ids_sat_p = 1e-12
    if vdd > pmos.params.vth then
        ids_sat_p = 0.5 * pmos.params.k * (vdd - pmos.params.vth)
                        * (vdd - pmos.params.vth)
    end

    -- Propagation delays
    local tphl = c_load * vdd / (2.0 * ids_sat_n)  -- Pull-down (NMOS)
    local tplh = c_load * vdd / (2.0 * ids_sat_p)  -- Pull-up (PMOS)

    -- Rise and fall times (2.2 RC time constants)
    local r_on_n    = vdd / (2.0 * ids_sat_n)
    local r_on_p    = vdd / (2.0 * ids_sat_p)
    local rise_time = 2.2 * r_on_p * c_load
    local fall_time = 2.2 * r_on_n * c_load

    return tphl, tplh, rise_time, fall_time
end

--- Analyze timing characteristics for a gate.
--
-- For CMOS:
--   t_pd ~ (C_load * Vdd) / (2 * I_sat)
--
-- For TTL:
--   t_pd ~ 5-15 ns (fixed by transistor switching speed)
--
-- @return TimingAnalysis, err_string
function analysis.analyze_timing(gate, c_load)
    local tphl, tplh, rise_time, fall_time

    -- Duck-type dispatch
    if gate.vcc and gate.q1 then
        -- TTL
        tphl      = 7e-9
        tplh      = 11e-9
        rise_time = 15e-9
        fall_time = 10e-9
    elseif gate.circuit then
        local nmos = gate.nmos or gate.nmos1
        local pmos = gate.pmos or gate.pmos1
        if not nmos or not pmos then
            return nil, "unsupported gate type"
        end
        tphl, tplh, rise_time, fall_time = cmos_timing(
            gate.circuit.vdd, nmos, pmos, c_load)
    else
        return nil, "unsupported gate type"
    end

    local tpd = (tphl + tplh) / 2.0

    local max_frequency = math.huge
    if tpd > 0 then
        max_frequency = 1.0 / (2.0 * tpd)
    end

    return types.TimingAnalysis({
        tphl          = tphl,
        tplh          = tplh,
        tpd           = tpd,
        rise_time     = rise_time,
        fall_time     = fall_time,
        max_frequency = max_frequency,
    }), nil
end

-- ---------------------------------------------------------------------------
-- Technology Comparison
-- ---------------------------------------------------------------------------

--- Compare CMOS and TTL NAND gates across all metrics.
--
-- This demonstrates WHY CMOS replaced TTL:
--   - CMOS has ~1000x less static power
--   - CMOS has better noise margins (relative to Vdd)
--   - CMOS can operate at lower voltages
function analysis.compare_cmos_vs_ttl(frequency, c_load)
    -- We require these here to avoid circular dependency at module load time
    local cmos_gates = require("coding_adventures.transistors.cmos_gates")
    local ttl_gates  = require("coding_adventures.transistors.ttl_gates")

    local cmos_nand = cmos_gates.CMOSNand()
    local ttl_nand  = ttl_gates.TTLNand(5.0)

    local cmos_pow, _ = analysis.analyze_power(cmos_nand, frequency, c_load, 0.5)
    local ttl_pow, _  = analysis.analyze_power(ttl_nand,  frequency, c_load, 0.5)

    local cmos_tim, _ = analysis.analyze_timing(cmos_nand, c_load)
    local ttl_tim, _  = analysis.analyze_timing(ttl_nand,  c_load)

    local cmos_inv = cmos_gates.CMOSInverter()
    local cmos_nm, _ = analysis.compute_noise_margins(cmos_inv)
    local ttl_nm, _  = analysis.compute_noise_margins(ttl_nand)

    return {
        cmos = {
            transistor_count    = 4,
            supply_voltage      = cmos_nand.circuit.vdd,
            static_power_w      = cmos_pow.static_power,
            dynamic_power_w     = cmos_pow.dynamic_power,
            total_power_w       = cmos_pow.total_power,
            propagation_delay_s = cmos_tim.tpd,
            max_frequency_hz    = cmos_tim.max_frequency,
            noise_margin_low_v  = cmos_nm.nml,
            noise_margin_high_v = cmos_nm.nmh,
        },
        ttl = {
            transistor_count    = 3,
            supply_voltage      = ttl_nand.vcc,
            static_power_w      = ttl_pow.static_power,
            dynamic_power_w     = ttl_pow.dynamic_power,
            total_power_w       = ttl_pow.total_power,
            propagation_delay_s = ttl_tim.tpd,
            max_frequency_hz    = ttl_tim.max_frequency,
            noise_margin_low_v  = ttl_nm.nml,
            noise_margin_high_v = ttl_nm.nmh,
        },
    }
end

-- ---------------------------------------------------------------------------
-- CMOS Scaling
-- ---------------------------------------------------------------------------

--- Demonstrate how CMOS performance changes with technology scaling.
--
-- As transistors shrink (Moore's Law):
--   - Gate length decreases -> faster switching
--   - Supply voltage decreases -> less power per switch
--   - Gate capacitance decreases -> less energy per transition
--   - BUT leakage current INCREASES -> more static power (the "leakage wall")
--
-- @param nodes  optional list of process nodes in meters; defaults to
--               {180 nm, 90 nm, 45 nm, 22 nm, 7 nm, 3 nm}
function analysis.demonstrate_cmos_scaling(nodes)
    local cmos_gates = require("coding_adventures.transistors.cmos_gates")

    if not nodes then
        nodes = { 180e-9, 90e-9, 45e-9, 22e-9, 7e-9, 3e-9 }
    end

    local results = {}

    for _, node in ipairs(nodes) do
        -- Empirical scaling relationships (simplified)
        local scale = node / 180e-9

        local vdd = 3.3 * math.sqrt(scale)
        if vdd < 0.7 then vdd = 0.7 end

        local vth = 0.4 * (scale ^ 0.3)
        if vth < 0.15 then vth = 0.15 end

        local c_gate = 1e-15 * scale
        local k      = 0.001 / math.sqrt(scale)

        -- Create transistor and circuit with scaled parameters
        local params  = types.MOSFETParams({
            vth = vth, k = k, l = node, c_gate = c_gate,
            w = 1e-6, c_drain = 0.5e-15,
        })
        local circuit = types.CircuitParams({ vdd = vdd, temperature = 300.0 })
        local inv     = cmos_gates.CMOSInverter(circuit, params, params)

        local load_cap = c_gate * 10
        local timing, _ = analysis.analyze_timing(inv, load_cap)
        local power, _  = analysis.analyze_power(inv, 1e9, load_cap, 0.5)

        -- Leakage current increases exponentially as Vth decreases
        local leakage = 1e-12 * math.exp((0.4 - vth) / 0.052)

        results[#results + 1] = {
            node_nm              = node * 1e9,
            vdd_v                = vdd,
            vth_v                = vth,
            c_gate_f             = c_gate,
            propagation_delay_s  = timing.tpd,
            dynamic_power_w      = power.dynamic_power,
            leakage_current_a    = leakage,
            max_frequency_hz     = timing.max_frequency,
        }
    end

    return results
end

return analysis

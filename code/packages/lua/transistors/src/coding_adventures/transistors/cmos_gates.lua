-- cmos_gates.lua -- Logic gates built from MOSFET pairs (CMOS technology)
--
-- === What is CMOS? ===
--
-- CMOS stands for Complementary Metal-Oxide-Semiconductor.  It is the
-- technology used in virtually every digital chip made since the 1980s.
--
-- The "complementary" refers to pairing NMOS and PMOS transistors:
--   - PMOS transistors form the PULL-UP network  (connects output to Vdd)
--   - NMOS transistors form the PULL-DOWN network (connects output to GND)
--
-- For any valid input combination, exactly ONE network is active:
--   - If pull-up is ON   -> output = Vdd (logic HIGH)
--   - If pull-down is ON -> output = GND (logic LOW)
--   - Never both ON simultaneously -> near-zero static power
--
-- === Transistor Counts ===
--
--   Gate    | NMOS | PMOS | Total
--   --------|------|------|------
--   NOT     |  1   |  1   |   2
--   NAND    |  2   |  2   |   4
--   NOR     |  2   |  2   |   4
--   AND     |  3   |  3   |   6
--   OR      |  3   |  3   |   6
--   XOR     |  3   |  3   |   6

local types  = require("coding_adventures.transistors.types")
local mosfet = require("coding_adventures.transistors.mosfet")

local cmos = {}

-- ==========================================================================
-- CMOS INVERTER (NOT gate) -- 2 transistors
-- ==========================================================================
--
-- The simplest and most important CMOS circuit.  Every other CMOS gate is
-- a variation of this fundamental pattern.
--
--       Vdd
--        |
--   +----+----+
--   |  PMOS   |--- Gate --- Input (A)
--   +----+----+
--        |
--        +------------- Output (Y = NOT A)
--        |
--   +----+----+
--   |  NMOS   |--- Gate --- Input (A)
--   +----+----+
--        |
--       GND
--
-- Input HIGH: NMOS ON, PMOS OFF -> output LOW.
-- Input LOW:  NMOS OFF, PMOS ON -> output HIGH.
-- Static power: ZERO (one transistor always OFF, breaking current path).

local CMOSInverter = {}
CMOSInverter.__index = CMOSInverter

--- Create a CMOS inverter.
-- @param circuit_params  optional CircuitParams
-- @param nmos_params     optional MOSFETParams for the NMOS
-- @param pmos_params     optional MOSFETParams for the PMOS
function cmos.CMOSInverter(circuit_params, nmos_params, pmos_params)
    local self = setmetatable({}, CMOSInverter)
    self.circuit = circuit_params or types.CircuitParams()
    self.nmos    = mosfet.NMOS(nmos_params)
    self.pmos    = mosfet.PMOS(pmos_params)
    return self
end

--- Evaluate the inverter with an analog input voltage.
-- Returns a full GateOutput with electrical detail.
function CMOSInverter:evaluate(input_voltage)
    local vdd = self.circuit.vdd

    -- NMOS: gate = input, source = GND -> Vgs_n = Vin
    local vgs_n = input_voltage
    -- PMOS: gate = input, source = Vdd -> Vgs_p = Vin - Vdd (negative when LOW)
    local vgs_p = input_voltage - vdd

    local nmos_on = self.nmos:is_conducting(vgs_n)
    local pmos_on = self.pmos:is_conducting(vgs_p)

    -- Determine output voltage
    local output_v
    if pmos_on and not nmos_on then
        output_v = vdd      -- PMOS pulls to Vdd
    elseif nmos_on and not pmos_on then
        output_v = 0.0      -- NMOS pulls to GND
    else
        -- Both on (transition region) or both off -- approximate as Vdd/2
        output_v = vdd / 2.0
    end

    -- Digital interpretation: above Vdd/2 is logic 1
    local logic_value = 0
    if output_v > vdd / 2.0 then
        logic_value = 1
    end

    -- Current draw: only significant during transition (both on)
    local current = 0.0
    if nmos_on and pmos_on then
        local vds_n = vdd / 2.0
        current = self.nmos:drain_current(vgs_n, vds_n)
    end

    local power = current * vdd

    -- Propagation delay estimate
    local c_load = self.nmos.params.c_drain + self.pmos.params.c_drain
    local delay
    if current > 0 then
        delay = c_load * vdd / (2.0 * current)
    else
        local ids_sat = self.nmos:drain_current(vdd, vdd)
        if ids_sat > 0 then
            delay = c_load * vdd / (2.0 * ids_sat)
        else
            delay = 1e-9
        end
    end

    return types.GateOutput({
        logic_value       = logic_value,
        voltage           = output_v,
        current_draw      = current,
        power_dissipation = power,
        propagation_delay = delay,
        transistor_count  = 2,
    })
end

--- Evaluate with a digital input (0 or 1), returning 0 or 1.
-- Returns result, err_string.
function CMOSInverter:evaluate_digital(a)
    local err = types.validate_bit(a, "a")
    if err then return nil, err end
    local vin = 0.0
    if a == 1 then vin = self.circuit.vdd end
    return self:evaluate(vin).logic_value, nil
end

--- Static power dissipation (ideally ~0 for CMOS).
function CMOSInverter:static_power()
    return 0.0
end

--- Dynamic power: P = C_load * Vdd^2 * frequency.
--
-- This is the dominant power consumption mechanism in CMOS.  Every time
-- the output switches, the load capacitance must be charged or
-- discharged.  The energy per transition is C * Vdd^2.
function CMOSInverter:dynamic_power(frequency, c_load)
    local vdd = self.circuit.vdd
    return c_load * vdd * vdd * frequency
end

--- Voltage Transfer Characteristic curve.
-- Returns a list of {vin, vout} pairs showing the sharp switching
-- threshold of CMOS.
function CMOSInverter:voltage_transfer_characteristic(steps)
    local vdd    = self.circuit.vdd
    local points = {}
    for i = 0, steps do
        local vin    = vdd * i / steps
        local result = self:evaluate(vin)
        points[#points + 1] = { vin, result.voltage }
    end
    return points
end

-- ==========================================================================
-- CMOS NAND -- 4 transistors
-- ==========================================================================
--
-- Pull-down: NMOS in SERIES   -> BOTH must be ON to pull output LOW.
-- Pull-up:   PMOS in PARALLEL -> EITHER can pull output HIGH.
--
-- This is why NAND is the "natural" CMOS gate -- it requires only 4
-- transistors.  AND needs 6 (NAND + inverter).

local CMOSNand = {}
CMOSNand.__index = CMOSNand

function cmos.CMOSNand(circuit_params, nmos_params, pmos_params)
    local self = setmetatable({}, CMOSNand)
    self.circuit = circuit_params or types.CircuitParams()
    self.nmos1   = mosfet.NMOS(nmos_params)
    self.nmos2   = mosfet.NMOS(nmos_params)
    self.pmos1   = mosfet.PMOS(pmos_params)
    self.pmos2   = mosfet.PMOS(pmos_params)
    return self
end

function CMOSNand:evaluate(va, vb)
    local vdd = self.circuit.vdd

    local vgs_n1 = va
    local vgs_n2 = vb
    local vgs_p1 = va - vdd
    local vgs_p2 = vb - vdd

    local nmos1_on = self.nmos1:is_conducting(vgs_n1)
    local nmos2_on = self.nmos2:is_conducting(vgs_n2)
    local pmos1_on = self.pmos1:is_conducting(vgs_p1)
    local pmos2_on = self.pmos2:is_conducting(vgs_p2)

    -- Pull-down: NMOS in SERIES -- BOTH must be ON
    local pulldown_on = nmos1_on and nmos2_on
    -- Pull-up: PMOS in PARALLEL -- EITHER can pull up
    local pullup_on   = pmos1_on or pmos2_on

    local output_v
    if pullup_on and not pulldown_on then
        output_v = vdd
    elseif pulldown_on and not pullup_on then
        output_v = 0.0
    else
        output_v = vdd / 2.0
    end

    local logic_value = 0
    if output_v > vdd / 2.0 then logic_value = 1 end

    local current = 0.0
    if pulldown_on and pullup_on then current = 0.001 end

    local c_load  = self.nmos1.params.c_drain + self.pmos1.params.c_drain
    local ids_sat = self.nmos1:drain_current(vdd, vdd)
    local delay   = 1e-9
    if ids_sat > 0 then
        delay = c_load * vdd / (2.0 * ids_sat)
    end

    return types.GateOutput({
        logic_value       = logic_value,
        voltage           = output_v,
        current_draw      = current,
        power_dissipation = current * vdd,
        propagation_delay = delay,
        transistor_count  = 4,
    })
end

function CMOSNand:evaluate_digital(a, b)
    local err = types.validate_bit(a, "a")
    if err then return nil, err end
    err = types.validate_bit(b, "b")
    if err then return nil, err end
    local vdd = self.circuit.vdd
    local va  = a == 1 and vdd or 0.0
    local vb  = b == 1 and vdd or 0.0
    return self:evaluate(va, vb).logic_value, nil
end

function CMOSNand:transistor_count()
    return 4
end

-- ==========================================================================
-- CMOS NOR -- 4 transistors
-- ==========================================================================
--
-- Pull-down: NMOS in PARALLEL -> EITHER ON pulls output LOW.
-- Pull-up:   PMOS in SERIES   -> BOTH must be ON to pull output HIGH.

local CMOSNor = {}
CMOSNor.__index = CMOSNor

function cmos.CMOSNor(circuit_params, nmos_params, pmos_params)
    local self = setmetatable({}, CMOSNor)
    self.circuit = circuit_params or types.CircuitParams()
    self.nmos1   = mosfet.NMOS(nmos_params)
    self.nmos2   = mosfet.NMOS(nmos_params)
    self.pmos1   = mosfet.PMOS(pmos_params)
    self.pmos2   = mosfet.PMOS(pmos_params)
    return self
end

function CMOSNor:evaluate(va, vb)
    local vdd = self.circuit.vdd

    local vgs_n1 = va
    local vgs_n2 = vb
    local vgs_p1 = va - vdd
    local vgs_p2 = vb - vdd

    local nmos1_on = self.nmos1:is_conducting(vgs_n1)
    local nmos2_on = self.nmos2:is_conducting(vgs_n2)
    local pmos1_on = self.pmos1:is_conducting(vgs_p1)
    local pmos2_on = self.pmos2:is_conducting(vgs_p2)

    -- Pull-down: NMOS in PARALLEL -- EITHER ON pulls low
    local pulldown_on = nmos1_on or nmos2_on
    -- Pull-up: PMOS in SERIES -- BOTH must be ON
    local pullup_on   = pmos1_on and pmos2_on

    local output_v
    if pullup_on and not pulldown_on then
        output_v = vdd
    elseif pulldown_on and not pullup_on then
        output_v = 0.0
    else
        output_v = vdd / 2.0
    end

    local logic_value = 0
    if output_v > vdd / 2.0 then logic_value = 1 end

    local current = 0.0
    if pulldown_on and pullup_on then current = 0.001 end

    local c_load  = self.nmos1.params.c_drain + self.pmos1.params.c_drain
    local ids_sat = self.nmos1:drain_current(vdd, vdd)
    local delay   = 1e-9
    if ids_sat > 0 then
        delay = c_load * vdd / (2.0 * ids_sat)
    end

    return types.GateOutput({
        logic_value       = logic_value,
        voltage           = output_v,
        current_draw      = current,
        power_dissipation = current * vdd,
        propagation_delay = delay,
        transistor_count  = 4,
    })
end

function CMOSNor:evaluate_digital(a, b)
    local err = types.validate_bit(a, "a")
    if err then return nil, err end
    err = types.validate_bit(b, "b")
    if err then return nil, err end
    local vdd = self.circuit.vdd
    local va  = a == 1 and vdd or 0.0
    local vb  = b == 1 and vdd or 0.0
    return self:evaluate(va, vb).logic_value, nil
end

-- ==========================================================================
-- CMOS AND -- 6 transistors (NAND + Inverter)
-- ==========================================================================
--
-- There is no "direct" CMOS AND gate.  The CMOS topology naturally produces
-- inverted outputs, so to get AND we must add an inverter.

local CMOSAnd = {}
CMOSAnd.__index = CMOSAnd

function cmos.CMOSAnd(circuit_params)
    local self = setmetatable({}, CMOSAnd)
    self.circuit = circuit_params or types.CircuitParams()
    self.nand    = cmos.CMOSNand(self.circuit)
    self.inv     = cmos.CMOSInverter(self.circuit)
    return self
end

--- AND = NOT(NAND(A, B)).
function CMOSAnd:evaluate(va, vb)
    local nand_out = self.nand:evaluate(va, vb)
    local inv_out  = self.inv:evaluate(nand_out.voltage)
    return types.GateOutput({
        logic_value       = inv_out.logic_value,
        voltage           = inv_out.voltage,
        current_draw      = nand_out.current_draw + inv_out.current_draw,
        power_dissipation = nand_out.power_dissipation + inv_out.power_dissipation,
        propagation_delay = nand_out.propagation_delay + inv_out.propagation_delay,
        transistor_count  = 6,
    })
end

function CMOSAnd:evaluate_digital(a, b)
    local err = types.validate_bit(a, "a")
    if err then return nil, err end
    err = types.validate_bit(b, "b")
    if err then return nil, err end
    local vdd = self.circuit.vdd
    local va  = a == 1 and vdd or 0.0
    local vb  = b == 1 and vdd or 0.0
    return self:evaluate(va, vb).logic_value, nil
end

-- ==========================================================================
-- CMOS OR -- 6 transistors (NOR + Inverter)
-- ==========================================================================

local CMOSOr = {}
CMOSOr.__index = CMOSOr

function cmos.CMOSOr(circuit_params)
    local self = setmetatable({}, CMOSOr)
    self.circuit = circuit_params or types.CircuitParams()
    self.nor     = cmos.CMOSNor(self.circuit)
    self.inv     = cmos.CMOSInverter(self.circuit)
    return self
end

--- OR = NOT(NOR(A, B)).
function CMOSOr:evaluate(va, vb)
    local nor_out = self.nor:evaluate(va, vb)
    local inv_out = self.inv:evaluate(nor_out.voltage)
    return types.GateOutput({
        logic_value       = inv_out.logic_value,
        voltage           = inv_out.voltage,
        current_draw      = nor_out.current_draw + inv_out.current_draw,
        power_dissipation = nor_out.power_dissipation + inv_out.power_dissipation,
        propagation_delay = nor_out.propagation_delay + inv_out.propagation_delay,
        transistor_count  = 6,
    })
end

function CMOSOr:evaluate_digital(a, b)
    local err = types.validate_bit(a, "a")
    if err then return nil, err end
    err = types.validate_bit(b, "b")
    if err then return nil, err end
    local vdd = self.circuit.vdd
    local va  = a == 1 and vdd or 0.0
    local vb  = b == 1 and vdd or 0.0
    return self:evaluate(va, vb).logic_value, nil
end

-- ==========================================================================
-- CMOS XOR -- 6 transistors (4 NANDs conceptually)
-- ==========================================================================
--
-- XOR(A, B) = NAND(NAND(A, NAND(A,B)), NAND(B, NAND(A,B)))
--
-- This construction proves that XOR can be built from the universal
-- NAND gate alone.

local CMOSXor = {}
CMOSXor.__index = CMOSXor

function cmos.CMOSXor(circuit_params)
    local self = setmetatable({}, CMOSXor)
    self.circuit = circuit_params or types.CircuitParams()
    self.nand1   = cmos.CMOSNand(self.circuit)
    self.nand2   = cmos.CMOSNand(self.circuit)
    self.nand3   = cmos.CMOSNand(self.circuit)
    self.nand4   = cmos.CMOSNand(self.circuit)
    return self
end

function CMOSXor:evaluate(va, vb)
    local vdd = self.circuit.vdd

    -- Step 1: NAND(A, B)
    local nand_ab   = self.nand1:evaluate(va, vb)
    -- Step 2: NAND(A, NAND(A,B))
    local nand_a_nab = self.nand2:evaluate(va, nand_ab.voltage)
    -- Step 3: NAND(B, NAND(A,B))
    local nand_b_nab = self.nand3:evaluate(vb, nand_ab.voltage)
    -- Step 4: NAND(step2, step3)
    local result     = self.nand4:evaluate(nand_a_nab.voltage, nand_b_nab.voltage)

    local total_current = nand_ab.current_draw + nand_a_nab.current_draw
                        + nand_b_nab.current_draw + result.current_draw

    -- Critical path: nand1 -> max(nand2, nand3) -> nand4
    local max_middle = math.max(nand_a_nab.propagation_delay,
                                nand_b_nab.propagation_delay)
    local total_delay = nand_ab.propagation_delay + max_middle
                      + result.propagation_delay

    return types.GateOutput({
        logic_value       = result.logic_value,
        voltage           = result.voltage,
        current_draw      = total_current,
        power_dissipation = total_current * vdd,
        propagation_delay = total_delay,
        transistor_count  = 6,
    })
end

function CMOSXor:evaluate_digital(a, b)
    local err = types.validate_bit(a, "a")
    if err then return nil, err end
    err = types.validate_bit(b, "b")
    if err then return nil, err end
    local vdd = self.circuit.vdd
    local va  = a == 1 and vdd or 0.0
    local vb  = b == 1 and vdd or 0.0
    return self:evaluate(va, vb).logic_value, nil
end

--- Alias: build XOR from 4 NANDs to demonstrate universality.
function CMOSXor:evaluate_from_nands(a, b)
    return self:evaluate_digital(a, b)
end

-- ==========================================================================
-- CMOS XNOR -- XOR + Inverter = 8 transistors
-- ==========================================================================
--
-- XNOR(A, B) = NOT(XOR(A, B))
--
-- XNOR is called the "equivalence gate": it outputs 1 when both inputs
-- have the SAME value (both LOW or both HIGH).  This makes it the
-- natural hardware primitive for bit-equality tests.
--
-- Construction: chain the 6-transistor CMOSXor with the 2-transistor
-- CMOSInverter, exactly mirroring how CMOSAnd = NAND + Inverter and
-- CMOSOr = NOR + Inverter are built.
--
--   Truth table:
--     A | B | XOR | XNOR
--     --|---|-----|-----
--     0 | 0 |  0  |  1   (same -- equal)
--     0 | 1 |  1  |  0   (different)
--     1 | 0 |  1  |  0   (different)
--     1 | 1 |  0  |  1   (same -- equal)
--
-- Transistor count: CMOSXor (6) + CMOSInverter (2) = 8.

local CMOSXnor = {}
CMOSXnor.__index = CMOSXnor

function cmos.CMOSXnor(circuit_params)
    local self = setmetatable({}, CMOSXnor)
    self.circuit  = circuit_params or types.CircuitParams()
    self.xor_gate = cmos.CMOSXor(self.circuit)
    self.inv      = cmos.CMOSInverter(self.circuit)
    return self
end

--- XNOR = NOT(XOR(A, B)).
function CMOSXnor:evaluate(va, vb)
    local xor_out = self.xor_gate:evaluate(va, vb)
    local inv_out = self.inv:evaluate(xor_out.voltage)
    return types.GateOutput({
        logic_value       = inv_out.logic_value,
        voltage           = inv_out.voltage,
        current_draw      = xor_out.current_draw + inv_out.current_draw,
        power_dissipation = xor_out.power_dissipation + inv_out.power_dissipation,
        propagation_delay = xor_out.propagation_delay + inv_out.propagation_delay,
        transistor_count  = 8,
    })
end

function CMOSXnor:evaluate_digital(a, b)
    local err = types.validate_bit(a, "a")
    if err then return nil, err end
    err = types.validate_bit(b, "b")
    if err then return nil, err end
    local vdd = self.circuit.vdd
    local va  = a == 1 and vdd or 0.0
    local vb  = b == 1 and vdd or 0.0
    return self:evaluate(va, vb).logic_value, nil
end

return cmos

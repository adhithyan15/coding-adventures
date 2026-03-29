-- Comprehensive tests for the transistors package.
--
-- Covers: types, NMOS, PMOS, NPN, PNP, CMOS gates (inverter, NAND, NOR,
-- AND, OR, XOR), TTL NAND, RTL inverter, amplifier analysis, noise margins,
-- power analysis, timing analysis, CMOS-vs-TTL comparison, and CMOS scaling.

-- Add src/ to the module search path so we can require the package.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local T = require("coding_adventures.transistors")

-- =========================================================================
-- Helper: approximate equality for floating-point comparisons
-- =========================================================================

local function approx(a, b, tol)
    tol = tol or 1e-9
    return math.abs(a - b) <= tol * (1 + math.abs(a) + math.abs(b))
end

-- =========================================================================
-- types
-- =========================================================================

describe("types", function()
    it("exports MOSFET operating region constants", function()
        assert.are.equal("cutoff",     T.MOSFET_CUTOFF)
        assert.are.equal("linear",     T.MOSFET_LINEAR)
        assert.are.equal("saturation", T.MOSFET_SATURATION)
    end)

    it("exports BJT operating region constants", function()
        assert.are.equal("cutoff",     T.BJT_CUTOFF)
        assert.are.equal("active",     T.BJT_ACTIVE)
        assert.are.equal("saturation", T.BJT_SATURATION)
    end)

    it("has a version", function()
        assert.are.equal("0.1.0", T.VERSION)
    end)

    describe("MOSFETParams", function()
        it("returns 180 nm defaults with no arguments", function()
            local p = T.MOSFETParams()
            assert.are.equal(0.4,      p.vth)
            assert.are.equal(0.001,    p.k)
            assert.are.equal(1e-6,     p.w)
            assert.are.equal(180e-9,   p.l)
            assert.are.equal(1e-15,    p.c_gate)
            assert.are.equal(0.5e-15,  p.c_drain)
        end)

        it("allows overrides", function()
            local p = T.MOSFETParams({ vth = 0.3, k = 0.002 })
            assert.are.equal(0.3,   p.vth)
            assert.are.equal(0.002, p.k)
            -- defaults still set for non-overridden fields
            assert.are.equal(1e-6,  p.w)
        end)
    end)

    describe("BJTParams", function()
        it("returns 2N2222 defaults", function()
            local p = T.BJTParams()
            assert.are.equal(100.0, p.beta)
            assert.are.equal(0.7,   p.vbe_on)
            assert.are.equal(0.2,   p.vce_sat)
            assert.are.equal(1e-14, p.is)
            assert.are.equal(5e-12, p.c_base)
        end)

        it("allows overrides", function()
            local p = T.BJTParams({ beta = 200 })
            assert.are.equal(200, p.beta)
            assert.are.equal(0.7, p.vbe_on)
        end)
    end)

    describe("CircuitParams", function()
        it("returns 3.3 V / 300 K defaults", function()
            local p = T.CircuitParams()
            assert.are.equal(3.3,   p.vdd)
            assert.are.equal(300.0, p.temperature)
        end)

        it("allows overrides", function()
            local p = T.CircuitParams({ vdd = 5.0 })
            assert.are.equal(5.0, p.vdd)
        end)
    end)

    describe("GateOutput", function()
        it("creates output with defaults", function()
            local g = T.GateOutput({})
            assert.are.equal(0,   g.logic_value)
            assert.are.equal(0.0, g.voltage)
            assert.are.equal(0,   g.transistor_count)
        end)

        it("accepts explicit fields", function()
            local g = T.GateOutput({ logic_value = 1, voltage = 3.3, transistor_count = 4 })
            assert.are.equal(1,   g.logic_value)
            assert.are.equal(3.3, g.voltage)
            assert.are.equal(4,   g.transistor_count)
        end)
    end)

    describe("validate_bit", function()
        it("returns nil for valid bits", function()
            assert.is_nil(T.validate_bit(0, "x"))
            assert.is_nil(T.validate_bit(1, "x"))
        end)

        it("returns error string for invalid values", function()
            local err = T.validate_bit(2, "x")
            assert.is_not_nil(err)
            assert.truthy(err:find("x must be 0 or 1"))
        end)

        it("returns error for negative values", function()
            assert.is_not_nil(T.validate_bit(-1, "a"))
        end)
    end)
end)

-- =========================================================================
-- NMOS
-- =========================================================================

describe("NMOS", function()
    local nmos

    before_each(function()
        nmos = T.NMOS()
    end)

    describe("region", function()
        it("is cutoff when Vgs < Vth", function()
            assert.are.equal("cutoff", nmos:region(0.0, 0.0))
            assert.are.equal("cutoff", nmos:region(0.3, 1.0))
        end)

        it("is linear when Vgs >= Vth and Vds < Vov", function()
            -- Vgs=1.0, Vth=0.4, Vov=0.6, Vds=0.3 < 0.6
            assert.are.equal("linear", nmos:region(1.0, 0.3))
        end)

        it("is saturation when Vgs >= Vth and Vds >= Vov", function()
            -- Vgs=1.0, Vth=0.4, Vov=0.6, Vds=1.0 >= 0.6
            assert.are.equal("saturation", nmos:region(1.0, 1.0))
        end)

        it("boundary: Vgs exactly at Vth is not cutoff", function()
            -- Vgs=0.4=Vth, Vov=0, Vds=0 -> linear (vds < vov is false since 0 < 0 is false)
            assert.are.equal("saturation", nmos:region(0.4, 0.0))
        end)
    end)

    describe("drain_current", function()
        it("is zero in cutoff", function()
            assert.are.equal(0.0, nmos:drain_current(0.0, 0.0))
            assert.are.equal(0.0, nmos:drain_current(0.3, 1.0))
        end)

        it("is positive in linear region", function()
            local ids = nmos:drain_current(1.0, 0.3)
            assert.is_true(ids > 0)
        end)

        it("is positive in saturation", function()
            local ids = nmos:drain_current(1.0, 1.0)
            assert.is_true(ids > 0)
        end)

        it("matches Shockley model in linear region", function()
            local vgs, vds = 1.0, 0.3
            local k, vth = 0.001, 0.4
            local vov = vgs - vth
            local expected = k * (vov * vds - 0.5 * vds * vds)
            assert.is_true(approx(expected, nmos:drain_current(vgs, vds)))
        end)

        it("matches Shockley model in saturation", function()
            local vgs, vds = 1.0, 1.0
            local k, vth = 0.001, 0.4
            local vov = vgs - vth
            local expected = 0.5 * k * vov * vov
            assert.is_true(approx(expected, nmos:drain_current(vgs, vds)))
        end)
    end)

    describe("is_conducting", function()
        it("false below threshold", function()
            assert.is_false(nmos:is_conducting(0.0))
            assert.is_false(nmos:is_conducting(0.39))
        end)

        it("true at and above threshold", function()
            assert.is_true(nmos:is_conducting(0.4))
            assert.is_true(nmos:is_conducting(3.3))
        end)
    end)

    describe("output_voltage", function()
        it("is 0 when conducting", function()
            assert.are.equal(0.0, nmos:output_voltage(3.3, 3.3))
        end)

        it("is Vdd when not conducting", function()
            assert.are.equal(3.3, nmos:output_voltage(0.0, 3.3))
        end)
    end)

    describe("transconductance", function()
        it("is zero in cutoff", function()
            assert.are.equal(0.0, nmos:transconductance(0.0, 0.0))
        end)

        it("equals K * Vov in saturation/linear", function()
            local vgs = 1.0
            local gm = nmos:transconductance(vgs, 1.0)
            local expected = 0.001 * (vgs - 0.4)
            assert.is_true(approx(expected, gm))
        end)
    end)

    it("accepts custom params", function()
        local custom = T.MOSFETParams({ vth = 0.2 })
        local n = T.NMOS(custom)
        assert.is_true(n:is_conducting(0.2))
        assert.is_false(n:is_conducting(0.1))
    end)
end)

-- =========================================================================
-- PMOS
-- =========================================================================

describe("PMOS", function()
    local pmos

    before_each(function()
        pmos = T.PMOS()
    end)

    describe("region", function()
        it("is cutoff when |Vgs| < Vth", function()
            assert.are.equal("cutoff", pmos:region(0.0, 0.0))
            assert.are.equal("cutoff", pmos:region(-0.3, -0.5))
        end)

        it("is linear when |Vgs| >= Vth and |Vds| < |Vgs| - Vth", function()
            -- |Vgs|=1.0, Vov=0.6, |Vds|=0.3 < 0.6
            assert.are.equal("linear", pmos:region(-1.0, -0.3))
        end)

        it("is saturation when |Vgs| >= Vth and |Vds| >= |Vgs| - Vth", function()
            assert.are.equal("saturation", pmos:region(-1.0, -1.0))
        end)
    end)

    describe("drain_current", function()
        it("is zero in cutoff", function()
            assert.are.equal(0.0, pmos:drain_current(0.0, 0.0))
        end)

        it("is positive in linear region", function()
            assert.is_true(pmos:drain_current(-1.0, -0.3) > 0)
        end)

        it("is positive in saturation", function()
            assert.is_true(pmos:drain_current(-1.0, -1.0) > 0)
        end)

        it("matches Shockley model in saturation", function()
            local vgs, vds = -1.0, -1.0
            local k, vth = 0.001, 0.4
            local vov = math.abs(vgs) - vth
            local expected = 0.5 * k * vov * vov
            assert.is_true(approx(expected, pmos:drain_current(vgs, vds)))
        end)
    end)

    describe("is_conducting", function()
        it("false when |Vgs| below threshold", function()
            assert.is_false(pmos:is_conducting(0.0))
            assert.is_false(pmos:is_conducting(-0.3))
        end)

        it("true when |Vgs| at/above threshold", function()
            assert.is_true(pmos:is_conducting(-0.4))
            assert.is_true(pmos:is_conducting(-3.3))
        end)
    end)

    describe("output_voltage", function()
        it("is Vdd when conducting", function()
            assert.are.equal(3.3, pmos:output_voltage(-3.3, 3.3))
        end)

        it("is 0 when not conducting", function()
            assert.are.equal(0.0, pmos:output_voltage(0.0, 3.3))
        end)
    end)

    describe("transconductance", function()
        it("is zero in cutoff", function()
            assert.are.equal(0.0, pmos:transconductance(0.0, 0.0))
        end)

        it("is positive when conducting", function()
            assert.is_true(pmos:transconductance(-1.0, -1.0) > 0)
        end)
    end)
end)

-- =========================================================================
-- NPN
-- =========================================================================

describe("NPN", function()
    local npn

    before_each(function()
        npn = T.NPN()
    end)

    describe("region", function()
        it("is cutoff when Vbe < VbeOn", function()
            assert.are.equal("cutoff", npn:region(0.0, 5.0))
            assert.are.equal("cutoff", npn:region(0.6, 3.0))
        end)

        it("is active when Vbe >= VbeOn and Vce > VceSat", function()
            assert.are.equal("active", npn:region(0.7, 1.0))
            assert.are.equal("active", npn:region(0.7, 5.0))
        end)

        it("is saturation when Vbe >= VbeOn and Vce <= VceSat", function()
            assert.are.equal("saturation", npn:region(0.7, 0.2))
            assert.are.equal("saturation", npn:region(0.7, 0.1))
        end)
    end)

    describe("collector_current", function()
        it("is zero in cutoff", function()
            assert.are.equal(0.0, npn:collector_current(0.0, 5.0))
        end)

        it("is positive in active region", function()
            assert.is_true(npn:collector_current(0.7, 5.0) > 0)
        end)

        it("follows Ebers-Moll exponential model", function()
            local vbe = 0.7
            local vt  = 0.026
            local expected = 1e-14 * (math.exp(math.min(vbe / vt, 40.0)) - 1.0)
            assert.is_true(approx(expected, npn:collector_current(vbe, 5.0)))
        end)

        it("clamps exponent to prevent overflow", function()
            -- Vbe = 2.0 -> exponent = 2.0/0.026 ~ 77, clamped to 40
            local ic = npn:collector_current(2.0, 5.0)
            assert.is_true(ic > 0)
            assert.is_true(ic < 1e10)  -- finite, not inf
        end)
    end)

    describe("base_current", function()
        it("is zero in cutoff", function()
            assert.are.equal(0.0, npn:base_current(0.0, 5.0))
        end)

        it("equals Ic / beta", function()
            local ic = npn:collector_current(0.7, 5.0)
            local ib = npn:base_current(0.7, 5.0)
            assert.is_true(approx(ic / 100.0, ib))
        end)
    end)

    describe("is_conducting", function()
        it("false below VbeOn", function()
            assert.is_false(npn:is_conducting(0.0))
            assert.is_false(npn:is_conducting(0.69))
        end)

        it("true at/above VbeOn", function()
            assert.is_true(npn:is_conducting(0.7))
            assert.is_true(npn:is_conducting(1.0))
        end)
    end)

    describe("transconductance", function()
        it("is zero in cutoff", function()
            assert.are.equal(0.0, npn:transconductance(0.0, 5.0))
        end)

        it("equals Ic / Vt in active region", function()
            local ic = npn:collector_current(0.7, 5.0)
            local gm = npn:transconductance(0.7, 5.0)
            assert.is_true(approx(ic / 0.026, gm))
        end)
    end)

    it("accepts custom params", function()
        local custom = T.BJTParams({ beta = 200, vbe_on = 0.6 })
        local n = T.NPN(custom)
        assert.is_true(n:is_conducting(0.6))
        assert.is_false(n:is_conducting(0.5))
    end)
end)

-- =========================================================================
-- PNP
-- =========================================================================

describe("PNP", function()
    local pnp

    before_each(function()
        pnp = T.PNP()
    end)

    describe("region", function()
        it("is cutoff when |Vbe| < VbeOn", function()
            assert.are.equal("cutoff", pnp:region(0.0, 0.0))
            assert.are.equal("cutoff", pnp:region(-0.5, -3.0))
        end)

        it("is active when |Vbe| >= VbeOn and |Vce| > VceSat", function()
            assert.are.equal("active", pnp:region(-0.7, -1.0))
        end)

        it("is saturation when |Vbe| >= VbeOn and |Vce| <= VceSat", function()
            assert.are.equal("saturation", pnp:region(-0.7, -0.1))
        end)
    end)

    describe("collector_current", function()
        it("is zero in cutoff", function()
            assert.are.equal(0.0, pnp:collector_current(0.0, 0.0))
        end)

        it("is positive when conducting", function()
            assert.is_true(pnp:collector_current(-0.7, -1.0) > 0)
        end)
    end)

    describe("base_current", function()
        it("is zero in cutoff", function()
            assert.are.equal(0.0, pnp:base_current(0.0, 0.0))
        end)

        it("equals Ic / beta", function()
            local ic = pnp:collector_current(-0.7, -1.0)
            local ib = pnp:base_current(-0.7, -1.0)
            assert.is_true(approx(ic / 100.0, ib))
        end)
    end)

    describe("is_conducting", function()
        it("false when |Vbe| below VbeOn", function()
            assert.is_false(pnp:is_conducting(0.0))
            assert.is_false(pnp:is_conducting(-0.5))
        end)

        it("true when |Vbe| at/above VbeOn", function()
            assert.is_true(pnp:is_conducting(-0.7))
            assert.is_true(pnp:is_conducting(-1.0))
        end)
    end)

    describe("transconductance", function()
        it("is zero in cutoff", function()
            assert.are.equal(0.0, pnp:transconductance(0.0, 0.0))
        end)

        it("is positive when conducting", function()
            assert.is_true(pnp:transconductance(-0.7, -1.0) > 0)
        end)
    end)
end)

-- =========================================================================
-- CMOS Inverter
-- =========================================================================

describe("CMOSInverter", function()
    local inv

    before_each(function()
        inv = T.CMOSInverter()
    end)

    it("has 2 transistors", function()
        local out = inv:evaluate(0.0)
        assert.are.equal(2, out.transistor_count)
    end)

    describe("truth table", function()
        it("NOT 0 = 1", function()
            local val, err = inv:evaluate_digital(0)
            assert.is_nil(err)
            assert.are.equal(1, val)
        end)

        it("NOT 1 = 0", function()
            local val, err = inv:evaluate_digital(1)
            assert.is_nil(err)
            assert.are.equal(0, val)
        end)
    end)

    describe("analog evaluation", function()
        it("input 0 V -> output Vdd", function()
            local out = inv:evaluate(0.0)
            assert.are.equal(3.3, out.voltage)
            assert.are.equal(1, out.logic_value)
        end)

        it("input Vdd -> output 0 V", function()
            local out = inv:evaluate(3.3)
            assert.are.equal(0.0, out.voltage)
            assert.are.equal(0, out.logic_value)
        end)

        it("input at midpoint -> output Vdd/2 (transition)", function()
            local out = inv:evaluate(1.65)
            assert.are.equal(1.65, out.voltage)
        end)
    end)

    it("static power is zero for CMOS", function()
        assert.are.equal(0.0, inv:static_power())
    end)

    it("dynamic power equals C * Vdd^2 * f", function()
        local p = inv:dynamic_power(1e9, 1e-15)
        local expected = 1e-15 * 3.3 * 3.3 * 1e9
        assert.is_true(approx(expected, p))
    end)

    it("voltage transfer characteristic returns correct point count", function()
        local points = inv:voltage_transfer_characteristic(10)
        assert.are.equal(11, #points)  -- 0..10 inclusive
        -- First point: Vin=0 -> Vout=Vdd
        assert.are.equal(0.0, points[1][1])
        assert.are.equal(3.3, points[1][2])
        -- Last point: Vin=Vdd -> Vout=0
        assert.is_true(approx(3.3, points[11][1]))
        assert.are.equal(0.0, points[11][2])
    end)

    it("rejects invalid digital inputs", function()
        local val, err = inv:evaluate_digital(2)
        assert.is_nil(val)
        assert.is_not_nil(err)
    end)

    it("accepts custom circuit params", function()
        local circuit = T.CircuitParams({ vdd = 5.0 })
        local inv5 = T.CMOSInverter(circuit)
        local out = inv5:evaluate(0.0)
        assert.are.equal(5.0, out.voltage)
    end)
end)

-- =========================================================================
-- CMOS NAND
-- =========================================================================

describe("CMOSNand", function()
    local gate

    before_each(function()
        gate = T.CMOSNand()
    end)

    it("has 4 transistors", function()
        assert.are.equal(4, gate:transistor_count())
    end)

    describe("truth table (NAND)", function()
        -- NAND truth table:
        --   0 NAND 0 = 1
        --   0 NAND 1 = 1
        --   1 NAND 0 = 1
        --   1 NAND 1 = 0
        local cases = {
            { 0, 0, 1 },
            { 0, 1, 1 },
            { 1, 0, 1 },
            { 1, 1, 0 },
        }
        for _, c in ipairs(cases) do
            it(string.format("%d NAND %d = %d", c[1], c[2], c[3]), function()
                local val, err = gate:evaluate_digital(c[1], c[2])
                assert.is_nil(err)
                assert.are.equal(c[3], val)
            end)
        end
    end)

    it("rejects invalid inputs", function()
        local val, err = gate:evaluate_digital(0, 3)
        assert.is_nil(val)
        assert.is_not_nil(err)
    end)

    describe("analog evaluation", function()
        it("both inputs HIGH -> output LOW", function()
            local out = gate:evaluate(3.3, 3.3)
            assert.are.equal(0, out.logic_value)
            assert.are.equal(0.0, out.voltage)
        end)

        it("both inputs LOW -> output HIGH", function()
            local out = gate:evaluate(0.0, 0.0)
            assert.are.equal(1, out.logic_value)
            assert.are.equal(3.3, out.voltage)
        end)
    end)
end)

-- =========================================================================
-- CMOS NOR
-- =========================================================================

describe("CMOSNor", function()
    local gate

    before_each(function()
        gate = T.CMOSNor()
    end)

    describe("truth table (NOR)", function()
        local cases = {
            { 0, 0, 1 },
            { 0, 1, 0 },
            { 1, 0, 0 },
            { 1, 1, 0 },
        }
        for _, c in ipairs(cases) do
            it(string.format("%d NOR %d = %d", c[1], c[2], c[3]), function()
                local val, err = gate:evaluate_digital(c[1], c[2])
                assert.is_nil(err)
                assert.are.equal(c[3], val)
            end)
        end
    end)

    it("rejects invalid inputs", function()
        local val, err = gate:evaluate_digital(5, 0)
        assert.is_nil(val)
        assert.is_not_nil(err)
    end)
end)

-- =========================================================================
-- CMOS AND
-- =========================================================================

describe("CMOSAnd", function()
    local gate

    before_each(function()
        gate = T.CMOSAnd()
    end)

    describe("truth table (AND)", function()
        local cases = {
            { 0, 0, 0 },
            { 0, 1, 0 },
            { 1, 0, 0 },
            { 1, 1, 1 },
        }
        for _, c in ipairs(cases) do
            it(string.format("%d AND %d = %d", c[1], c[2], c[3]), function()
                local val, err = gate:evaluate_digital(c[1], c[2])
                assert.is_nil(err)
                assert.are.equal(c[3], val)
            end)
        end
    end)

    it("analog: reports 6 transistors", function()
        local out = gate:evaluate(3.3, 3.3)
        assert.are.equal(6, out.transistor_count)
    end)

    it("rejects invalid inputs", function()
        local val, err = gate:evaluate_digital(-1, 0)
        assert.is_nil(val)
        assert.is_not_nil(err)
    end)
end)

-- =========================================================================
-- CMOS OR
-- =========================================================================

describe("CMOSOr", function()
    local gate

    before_each(function()
        gate = T.CMOSOr()
    end)

    describe("truth table (OR)", function()
        local cases = {
            { 0, 0, 0 },
            { 0, 1, 1 },
            { 1, 0, 1 },
            { 1, 1, 1 },
        }
        for _, c in ipairs(cases) do
            it(string.format("%d OR %d = %d", c[1], c[2], c[3]), function()
                local val, err = gate:evaluate_digital(c[1], c[2])
                assert.is_nil(err)
                assert.are.equal(c[3], val)
            end)
        end
    end)

    it("analog: reports 6 transistors", function()
        local out = gate:evaluate(0.0, 3.3)
        assert.are.equal(6, out.transistor_count)
    end)

    it("rejects invalid inputs", function()
        local val, err = gate:evaluate_digital(0, 2)
        assert.is_nil(val)
        assert.is_not_nil(err)
    end)
end)

-- =========================================================================
-- CMOS XOR
-- =========================================================================

describe("CMOSXor", function()
    local gate

    before_each(function()
        gate = T.CMOSXor()
    end)

    describe("truth table (XOR)", function()
        local cases = {
            { 0, 0, 0 },
            { 0, 1, 1 },
            { 1, 0, 1 },
            { 1, 1, 0 },
        }
        for _, c in ipairs(cases) do
            it(string.format("%d XOR %d = %d", c[1], c[2], c[3]), function()
                local val, err = gate:evaluate_digital(c[1], c[2])
                assert.is_nil(err)
                assert.are.equal(c[3], val)
            end)
        end
    end)

    it("evaluate_from_nands matches evaluate_digital", function()
        for _, c in ipairs({{0,0},{0,1},{1,0},{1,1}}) do
            local v1, _ = gate:evaluate_digital(c[1], c[2])
            local v2, _ = gate:evaluate_from_nands(c[1], c[2])
            assert.are.equal(v1, v2)
        end
    end)

    it("analog: reports 6 transistors", function()
        local out = gate:evaluate(0.0, 3.3)
        assert.are.equal(6, out.transistor_count)
    end)

    it("rejects invalid inputs", function()
        local val, err = gate:evaluate_digital(0, 3)
        assert.is_nil(val)
        assert.is_not_nil(err)
    end)
end)

-- =========================================================================
-- CMOS XNOR
-- =========================================================================

describe("CMOSXnor", function()
    local gate

    before_each(function()
        gate = T.CMOSXnor()
    end)

    describe("truth table (XNOR)", function()
        local cases = {
            { 0, 0, 1 },
            { 0, 1, 0 },
            { 1, 0, 0 },
            { 1, 1, 1 },
        }
        for _, c in ipairs(cases) do
            it(string.format("%d XNOR %d = %d", c[1], c[2], c[3]), function()
                local val, err = gate:evaluate_digital(c[1], c[2])
                assert.is_nil(err)
                assert.are.equal(c[3], val)
            end)
        end
    end)

    it("XNOR is inverse of XOR", function()
        -- XNOR(a, b) = NOT(XOR(a, b)) for all input combinations.
        local xor_gate = T.CMOSXor()
        local inv      = T.CMOSInverter()
        for _, c in ipairs({{0,0},{0,1},{1,0},{1,1}}) do
            local xnor_val, _ = gate:evaluate_digital(c[1], c[2])
            local xor_val, _  = xor_gate:evaluate_digital(c[1], c[2])
            local not_xor, _  = inv:evaluate_digital(xor_val)
            assert.are.equal(not_xor, xnor_val)
        end
    end)

    it("analog: reports 8 transistors", function()
        local out = gate:evaluate(0.0, 3.3)
        assert.are.equal(8, out.transistor_count)
    end)

    it("rejects invalid inputs", function()
        local val, err = gate:evaluate_digital(0, 3)
        assert.is_nil(val)
        assert.is_not_nil(err)
    end)
end)

-- =========================================================================
-- TTL NAND
-- =========================================================================

describe("TTLNand", function()
    local gate

    before_each(function()
        gate = T.TTLNand(5.0)
    end)

    describe("truth table (NAND)", function()
        local cases = {
            { 0, 0, 1 },
            { 0, 1, 1 },
            { 1, 0, 1 },
            { 1, 1, 0 },
        }
        for _, c in ipairs(cases) do
            it(string.format("%d NAND %d = %d", c[1], c[2], c[3]), function()
                local val, err = gate:evaluate_digital(c[1], c[2])
                assert.is_nil(err)
                assert.are.equal(c[3], val)
            end)
        end
    end)

    it("has 3 transistors", function()
        local out = gate:evaluate(5.0, 5.0)
        assert.are.equal(3, out.transistor_count)
    end)

    describe("analog evaluation", function()
        it("both HIGH -> output ~VceSat (0.2 V)", function()
            local out = gate:evaluate(5.0, 5.0)
            assert.are.equal(0.2, out.voltage)
            assert.are.equal(0, out.logic_value)
        end)

        it("at least one LOW -> output ~Vcc - VbeOn", function()
            local out = gate:evaluate(0.0, 5.0)
            assert.is_true(approx(4.3, out.voltage))
            assert.are.equal(1, out.logic_value)
        end)
    end)

    it("static power is significant (milliwatts)", function()
        local p = gate:static_power()
        assert.is_true(p > 0.001)  -- > 1 mW
    end)

    it("rejects invalid inputs", function()
        local val, err = gate:evaluate_digital(2, 0)
        assert.is_nil(val)
        assert.is_not_nil(err)
    end)

    it("propagation delay is 10 ns", function()
        local out = gate:evaluate(5.0, 5.0)
        assert.is_true(approx(10e-9, out.propagation_delay))
    end)

    it("default Vcc is used when argument provided", function()
        local g2 = T.TTLNand(3.3)
        assert.are.equal(3.3, g2.vcc)
    end)
end)

-- =========================================================================
-- RTL Inverter
-- =========================================================================

describe("RTLInverter", function()
    local gate

    before_each(function()
        gate = T.RTLInverter(5.0, 10000, 1000)
    end)

    describe("truth table", function()
        it("NOT 0 = 1", function()
            local val, err = gate:evaluate_digital(0)
            assert.is_nil(err)
            assert.are.equal(1, val)
        end)

        it("NOT 1 = 0", function()
            local val, err = gate:evaluate_digital(1)
            assert.is_nil(err)
            assert.are.equal(0, val)
        end)
    end)

    it("has 1 transistor", function()
        local out = gate:evaluate(0.0)
        assert.are.equal(1, out.transistor_count)
    end)

    it("output is Vcc when input is LOW", function()
        local out = gate:evaluate(0.0)
        assert.are.equal(5.0, out.voltage)
    end)

    it("output is near VceSat when input is HIGH", function()
        local out = gate:evaluate(5.0)
        assert.is_true(out.voltage <= 0.3)
    end)

    it("propagation delay is 50 ns", function()
        local out = gate:evaluate(5.0)
        assert.is_true(approx(50e-9, out.propagation_delay))
    end)

    it("rejects invalid digital inputs", function()
        local val, err = gate:evaluate_digital(5)
        assert.is_nil(val)
        assert.is_not_nil(err)
    end)
end)

-- =========================================================================
-- Amplifier Analysis
-- =========================================================================

describe("amplifier analysis", function()
    describe("common-source (MOSFET)", function()
        it("has negative (inverting) voltage gain", function()
            local nmos = T.NMOS()
            local result = T.analyze_common_source(nmos, 1.0, 3.3, 10000, 1e-12)
            assert.is_true(result.voltage_gain < 0)
        end)

        it("has very high input impedance", function()
            local nmos = T.NMOS()
            local result = T.analyze_common_source(nmos, 1.0, 3.3, 10000, 1e-12)
            assert.are.equal(1e12, result.input_impedance)
        end)

        it("output impedance equals drain resistor", function()
            local nmos = T.NMOS()
            local result = T.analyze_common_source(nmos, 1.0, 3.3, 10000, 1e-12)
            assert.are.equal(10000, result.output_impedance)
        end)

        it("has positive bandwidth", function()
            local nmos = T.NMOS()
            local result = T.analyze_common_source(nmos, 1.0, 3.3, 10000, 1e-12)
            assert.is_true(result.bandwidth > 0)
        end)

        it("operating point contains vgs, vds, ids, gm", function()
            local nmos = T.NMOS()
            local result = T.analyze_common_source(nmos, 1.0, 3.3, 10000, 1e-12)
            assert.is_not_nil(result.operating_point.vgs)
            assert.is_not_nil(result.operating_point.vds)
            assert.is_not_nil(result.operating_point.ids)
            assert.is_not_nil(result.operating_point.gm)
        end)
    end)

    describe("common-emitter (BJT)", function()
        it("has negative (inverting) voltage gain", function()
            local npn = T.NPN()
            local result = T.analyze_common_emitter(npn, 0.7, 5.0, 1000, 1e-12)
            assert.is_true(result.voltage_gain < 0)
        end)

        it("has finite input impedance (r_pi)", function()
            local npn = T.NPN()
            local result = T.analyze_common_emitter(npn, 0.7, 5.0, 1000, 1e-12)
            assert.is_true(result.input_impedance > 0)
            assert.is_true(result.input_impedance < 1e12)
        end)

        it("output impedance equals collector resistor", function()
            local npn = T.NPN()
            local result = T.analyze_common_emitter(npn, 0.7, 5.0, 1000, 1e-12)
            assert.are.equal(1000, result.output_impedance)
        end)

        it("operating point contains vbe, vce, ic, ib, gm", function()
            local npn = T.NPN()
            local result = T.analyze_common_emitter(npn, 0.7, 5.0, 1000, 1e-12)
            local op = result.operating_point
            assert.is_not_nil(op.vbe)
            assert.is_not_nil(op.vce)
            assert.is_not_nil(op.ic)
            assert.is_not_nil(op.ib)
            assert.is_not_nil(op.gm)
        end)

        it("has very high impedance in cutoff", function()
            local npn = T.NPN()
            local result = T.analyze_common_emitter(npn, 0.0, 5.0, 1000, 1e-12)
            assert.are.equal(1e12, result.input_impedance)
        end)
    end)
end)

-- =========================================================================
-- Noise Margins
-- =========================================================================

describe("noise margins", function()
    it("CMOS inverter has symmetric margins", function()
        local inv = T.CMOSInverter()
        local nm, err = T.compute_noise_margins(inv)
        assert.is_nil(err)
        assert.are.equal(0.0, nm.vol)
        assert.are.equal(3.3, nm.voh)
        assert.is_true(approx(0.4 * 3.3, nm.vil))
        assert.is_true(approx(0.6 * 3.3, nm.vih))
        assert.is_true(nm.nml > 0)
        assert.is_true(nm.nmh > 0)
    end)

    it("TTL NAND has defined margins", function()
        local gate = T.TTLNand(5.0)
        local nm, err = T.compute_noise_margins(gate)
        assert.is_nil(err)
        assert.are.equal(0.2, nm.vol)
        assert.is_true(approx(4.3, nm.voh))
        assert.are.equal(0.8, nm.vil)
        assert.are.equal(2.0, nm.vih)
        assert.is_true(approx(0.6, nm.nml))
        assert.is_true(approx(2.3, nm.nmh))
    end)

    it("returns error for unsupported gate type", function()
        local nm, err = T.compute_noise_margins({ foo = "bar" })
        assert.is_nil(nm)
        assert.is_not_nil(err)
    end)
end)

-- =========================================================================
-- Power Analysis
-- =========================================================================

describe("power analysis", function()
    it("CMOS has zero static power", function()
        local inv = T.CMOSInverter()
        local pa, err = T.analyze_power(inv, 1e9, 1e-15, 0.5)
        assert.is_nil(err)
        assert.are.equal(0.0, pa.static_power)
        assert.is_true(pa.dynamic_power > 0)
        assert.are.equal(pa.static_power + pa.dynamic_power, pa.total_power)
    end)

    it("TTL has significant static power", function()
        local gate = T.TTLNand(5.0)
        local pa, err = T.analyze_power(gate, 1e6, 1e-12, 0.5)
        assert.is_nil(err)
        assert.is_true(pa.static_power > 0)
    end)

    it("energy per switch = C * Vdd^2", function()
        local inv = T.CMOSInverter()
        local pa, _ = T.analyze_power(inv, 1e9, 1e-15, 0.5)
        local expected = 1e-15 * 3.3 * 3.3
        assert.is_true(approx(expected, pa.energy_per_switch))
    end)

    it("CMOS NAND power analysis works", function()
        local gate = T.CMOSNand()
        local pa, err = T.analyze_power(gate, 1e9, 1e-15, 0.5)
        assert.is_nil(err)
        assert.are.equal(0.0, pa.static_power)
    end)

    it("CMOS NOR power analysis works", function()
        local gate = T.CMOSNor()
        local pa, err = T.analyze_power(gate, 1e9, 1e-15, 0.5)
        assert.is_nil(err)
        assert.are.equal(0.0, pa.static_power)
    end)

    it("returns error for unsupported gate type", function()
        local pa, err = T.analyze_power({}, 1e9, 1e-15, 0.5)
        assert.is_nil(pa)
        assert.is_not_nil(err)
    end)
end)

-- =========================================================================
-- Timing Analysis
-- =========================================================================

describe("timing analysis", function()
    it("CMOS inverter has positive delays", function()
        local inv = T.CMOSInverter()
        local ta, err = T.analyze_timing(inv, 1e-15)
        assert.is_nil(err)
        assert.is_true(ta.tphl > 0)
        assert.is_true(ta.tplh > 0)
        assert.is_true(ta.tpd > 0)
        assert.is_true(ta.rise_time > 0)
        assert.is_true(ta.fall_time > 0)
        assert.is_true(ta.max_frequency > 0)
    end)

    it("tpd is average of tphl and tplh", function()
        local inv = T.CMOSInverter()
        local ta, _ = T.analyze_timing(inv, 1e-15)
        assert.is_true(approx((ta.tphl + ta.tplh) / 2.0, ta.tpd))
    end)

    it("max_frequency = 1 / (2 * tpd)", function()
        local inv = T.CMOSInverter()
        local ta, _ = T.analyze_timing(inv, 1e-15)
        assert.is_true(approx(1.0 / (2.0 * ta.tpd), ta.max_frequency))
    end)

    it("TTL has fixed timing characteristics", function()
        local gate = T.TTLNand(5.0)
        local ta, err = T.analyze_timing(gate, 1e-12)
        assert.is_nil(err)
        assert.is_true(approx(7e-9, ta.tphl))
        assert.is_true(approx(11e-9, ta.tplh))
        assert.is_true(approx(15e-9, ta.rise_time))
        assert.is_true(approx(10e-9, ta.fall_time))
    end)

    it("CMOS NAND timing works", function()
        local gate = T.CMOSNand()
        local ta, err = T.analyze_timing(gate, 1e-15)
        assert.is_nil(err)
        assert.is_true(ta.tpd > 0)
    end)

    it("CMOS NOR timing works", function()
        local gate = T.CMOSNor()
        local ta, err = T.analyze_timing(gate, 1e-15)
        assert.is_nil(err)
        assert.is_true(ta.tpd > 0)
    end)

    it("returns error for unsupported gate type", function()
        local ta, err = T.analyze_timing({}, 1e-15)
        assert.is_nil(ta)
        assert.is_not_nil(err)
    end)
end)

-- =========================================================================
-- CMOS vs TTL Comparison
-- =========================================================================

describe("compare_cmos_vs_ttl", function()
    it("returns cmos and ttl sections", function()
        local result = T.compare_cmos_vs_ttl(1e6, 1e-12)
        assert.is_not_nil(result.cmos)
        assert.is_not_nil(result.ttl)
    end)

    it("CMOS has lower static power than TTL", function()
        local result = T.compare_cmos_vs_ttl(1e6, 1e-12)
        assert.is_true(result.cmos.static_power_w < result.ttl.static_power_w)
    end)

    it("CMOS uses 4 transistors, TTL uses 3", function()
        local result = T.compare_cmos_vs_ttl(1e6, 1e-12)
        assert.are.equal(4, result.cmos.transistor_count)
        assert.are.equal(3, result.ttl.transistor_count)
    end)

    it("both have positive propagation delay", function()
        local result = T.compare_cmos_vs_ttl(1e6, 1e-12)
        assert.is_true(result.cmos.propagation_delay_s > 0)
        assert.is_true(result.ttl.propagation_delay_s > 0)
    end)
end)

-- =========================================================================
-- CMOS Scaling
-- =========================================================================

describe("demonstrate_cmos_scaling", function()
    it("returns data for default 6 process nodes", function()
        local results = T.demonstrate_cmos_scaling()
        assert.are.equal(6, #results)
    end)

    it("first node is 180 nm", function()
        local results = T.demonstrate_cmos_scaling()
        assert.is_true(approx(180, results[1].node_nm))
    end)

    it("last node is 3 nm", function()
        local results = T.demonstrate_cmos_scaling()
        assert.is_true(approx(3, results[6].node_nm))
    end)

    it("Vdd decreases with scaling", function()
        local results = T.demonstrate_cmos_scaling()
        assert.is_true(results[1].vdd_v > results[6].vdd_v)
    end)

    it("leakage current increases with scaling", function()
        local results = T.demonstrate_cmos_scaling()
        assert.is_true(results[6].leakage_current_a > results[1].leakage_current_a)
    end)

    it("accepts custom node list", function()
        local results = T.demonstrate_cmos_scaling({ 180e-9, 45e-9 })
        assert.are.equal(2, #results)
        assert.is_true(approx(180, results[1].node_nm))
        assert.is_true(approx(45, results[2].node_nm))
    end)

    it("all results have positive max_frequency", function()
        local results = T.demonstrate_cmos_scaling()
        for _, r in ipairs(results) do
            assert.is_true(r.max_frequency_hz > 0)
        end
    end)
end)

-- =========================================================================
-- Result type constructors
-- =========================================================================

describe("result type constructors", function()
    it("AmplifierAnalysis defaults", function()
        local a = T.AmplifierAnalysis({})
        assert.are.equal(0.0, a.voltage_gain)
        assert.are.equal(0.0, a.transconductance)
    end)

    it("NoiseMargins defaults", function()
        local nm = T.NoiseMargins({})
        assert.are.equal(0.0, nm.vol)
        assert.are.equal(0.0, nm.nml)
    end)

    it("PowerAnalysis defaults", function()
        local pa = T.PowerAnalysis({})
        assert.are.equal(0.0, pa.static_power)
        assert.are.equal(0.0, pa.total_power)
    end)

    it("TimingAnalysis defaults", function()
        local ta = T.TimingAnalysis({})
        assert.are.equal(0.0, ta.tpd)
        assert.are.equal(0.0, ta.max_frequency)
    end)
end)

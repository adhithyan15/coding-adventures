-- transistors -- MOSFET, BJT, CMOS, and TTL transistor-level circuit simulation
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- Layer 10 in the computing stack.
--
-- === Why transistors matter ===
--
-- Logic gates (AND, OR, NOT) are abstractions.  In real hardware, each gate
-- is built from transistors -- tiny electrically-controlled switches.  This
-- package simulates those transistors and shows how gates emerge from them.
--
-- There are two main transistor families:
--
--   MOSFET (Metal-Oxide-Semiconductor Field-Effect Transistor):
--     Voltage-controlled.  Used in all modern chips (CMOS technology).
--     Near-zero static power consumption.
--
--   BJT (Bipolar Junction Transistor):
--     Current-controlled.  Used in historical TTL logic (7400 series).
--     Higher static power, but historically faster.
--
-- === Package organization ===
--
--   types.lua       -- Constants, parameter structs, result types
--   mosfet.lua      -- NMOS and PMOS transistor simulation
--   bjt.lua         -- NPN and PNP transistor simulation
--   cmos_gates.lua  -- Logic gates built from MOSFET pairs (CMOS)
--   ttl_gates.lua   -- Logic gates built from BJT transistors (TTL/RTL)
--   amplifier.lua   -- Transistors as analog signal amplifiers
--   analysis.lua    -- Noise margins, power, timing, technology comparison

local types      = require("coding_adventures.transistors.types")
local mosfet     = require("coding_adventures.transistors.mosfet")
local bjt        = require("coding_adventures.transistors.bjt")
local cmos_gates = require("coding_adventures.transistors.cmos_gates")
local ttl_gates  = require("coding_adventures.transistors.ttl_gates")
local amplifier  = require("coding_adventures.transistors.amplifier")
local analysis   = require("coding_adventures.transistors.analysis")

return {
    VERSION = "0.1.0",

    -- ======================================================================
    -- Constants
    -- ======================================================================

    -- MOSFET operating regions
    MOSFET_CUTOFF     = types.MOSFET_CUTOFF,
    MOSFET_LINEAR     = types.MOSFET_LINEAR,
    MOSFET_SATURATION = types.MOSFET_SATURATION,

    -- BJT operating regions
    BJT_CUTOFF     = types.BJT_CUTOFF,
    BJT_ACTIVE     = types.BJT_ACTIVE,
    BJT_SATURATION = types.BJT_SATURATION,

    -- ======================================================================
    -- Parameter constructors
    -- ======================================================================

    MOSFETParams  = types.MOSFETParams,
    BJTParams     = types.BJTParams,
    CircuitParams = types.CircuitParams,

    -- ======================================================================
    -- Result constructors
    -- ======================================================================

    GateOutput        = types.GateOutput,
    AmplifierAnalysis = types.AmplifierAnalysis,
    NoiseMargins      = types.NoiseMargins,
    PowerAnalysis     = types.PowerAnalysis,
    TimingAnalysis    = types.TimingAnalysis,

    -- ======================================================================
    -- Utility
    -- ======================================================================

    validate_bit = types.validate_bit,

    -- ======================================================================
    -- Transistors
    -- ======================================================================

    NMOS = mosfet.NMOS,
    PMOS = mosfet.PMOS,
    NPN  = bjt.NPN,
    PNP  = bjt.PNP,

    -- ======================================================================
    -- CMOS gates
    -- ======================================================================

    CMOSInverter = cmos_gates.CMOSInverter,
    CMOSNand     = cmos_gates.CMOSNand,
    CMOSNor      = cmos_gates.CMOSNor,
    CMOSAnd      = cmos_gates.CMOSAnd,
    CMOSOr       = cmos_gates.CMOSOr,
    CMOSXor      = cmos_gates.CMOSXor,

    -- ======================================================================
    -- TTL / RTL gates
    -- ======================================================================

    TTLNand      = ttl_gates.TTLNand,
    RTLInverter  = ttl_gates.RTLInverter,

    -- ======================================================================
    -- Amplifier analysis
    -- ======================================================================

    analyze_common_source  = amplifier.analyze_common_source,
    analyze_common_emitter = amplifier.analyze_common_emitter,

    -- ======================================================================
    -- Electrical analysis
    -- ======================================================================

    compute_noise_margins     = analysis.compute_noise_margins,
    analyze_power             = analysis.analyze_power,
    analyze_timing            = analysis.analyze_timing,
    compare_cmos_vs_ttl       = analysis.compare_cmos_vs_ttl,
    demonstrate_cmos_scaling  = analysis.demonstrate_cmos_scaling,
}

"""Transistors — the electronic switches beneath logic gates.

This package models transistors at the electrical level, showing how
logic gates are physically constructed from MOSFET and BJT transistors.

=== Package Organization ===

    transistors.mosfet     — NMOS and PMOS transistors
    transistors.bjt        — NPN and PNP transistors
    transistors.cmos_gates — CMOS logic gates built from MOSFET pairs
    transistors.ttl_gates  — TTL logic gates built from BJTs (historical)
    transistors.amplifier  — Analog amplifier analysis
    transistors.analysis   — Noise margins, power, timing, technology comparison
    transistors.types      — Enums, parameters, and result dataclasses
"""

# Types and enums
from transistors.types import (
    AmplifierAnalysis,
    BJTParams,
    BJTRegion,
    CircuitParams,
    GateOutput,
    MOSFETParams,
    MOSFETRegion,
    NoiseMargins,
    PowerAnalysis,
    TimingAnalysis,
    TransistorType,
)

# Transistor models
from transistors.mosfet import NMOS, PMOS
from transistors.bjt import NPN, PNP

# CMOS logic gates
from transistors.cmos_gates import (
    CMOSAnd,
    CMOSInverter,
    CMOSNand,
    CMOSNor,
    CMOSOr,
    CMOSXor,
)

# TTL logic gates
from transistors.ttl_gates import RTLInverter, TTLNand

# Amplifier analysis
from transistors.amplifier import (
    analyze_common_emitter_amp,
    analyze_common_source_amp,
)

# Electrical analysis
from transistors.analysis import (
    analyze_power,
    analyze_timing,
    compare_cmos_vs_ttl,
    compute_noise_margins,
    demonstrate_cmos_scaling,
)

__all__ = [
    # Types
    "AmplifierAnalysis",
    "BJTParams",
    "BJTRegion",
    "CircuitParams",
    "GateOutput",
    "MOSFETParams",
    "MOSFETRegion",
    "NoiseMargins",
    "PowerAnalysis",
    "TimingAnalysis",
    "TransistorType",
    # Transistors
    "NMOS",
    "PMOS",
    "NPN",
    "PNP",
    # CMOS gates
    "CMOSAnd",
    "CMOSInverter",
    "CMOSNand",
    "CMOSNor",
    "CMOSOr",
    "CMOSXor",
    # TTL gates
    "RTLInverter",
    "TTLNand",
    # Analysis
    "analyze_common_emitter_amp",
    "analyze_common_source_amp",
    "analyze_power",
    "analyze_timing",
    "compare_cmos_vs_ttl",
    "compute_noise_margins",
    "demonstrate_cmos_scaling",
]

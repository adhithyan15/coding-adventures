"""Analog Amplifier Analysis — transistors as signal amplifiers.

=== Beyond Digital: Transistors as Amplifiers ===

A transistor used as a digital switch operates in only two states: ON and OFF.
But transistors are fundamentally ANALOG devices. When biased in the right
operating region (saturation for MOSFET, active for BJT), they can amplify
small signals into larger ones.

This is how every audio amplifier, radio receiver, and analog sensor circuit
works. Even in a "digital" CPU, analog amplifier properties matter: the
clock distribution network uses amplifiers (buffers) to drive signals across
the chip.

=== Common-Source Amplifier (MOSFET) ===

The most basic MOSFET amplifier. The input signal modulates Vgs, which
modulates Ids (via transconductance gm), which creates a voltage drop
across the drain resistor Rd:

        Vdd
         │
        [Rd]  ← voltage drop = Ids × Rd
         │
    ─────┤ Drain (output)
Gate ──┤│
    ─────┤ Source
         │
        GND

    Voltage gain: Av = -gm × Rd
    The negative sign means it's an INVERTING amplifier.

=== Common-Emitter Amplifier (BJT) ===

The BJT equivalent. Input signal modulates Vbe, which modulates Ic
(via transconductance gm = Ic/Vt), which creates a voltage drop
across the collector resistor Rc:

        Vcc
         │
        [Rc]
         │
    ─────┤ Collector (output)
Base ──┤──>
    ─────┤ Emitter
         │
        GND

    Voltage gain: Av = -gm × Rc = -(Ic/Vt) × Rc
"""

from __future__ import annotations

import math

from transistors.bjt import NPN
from transistors.mosfet import NMOS
from transistors.types import AmplifierAnalysis, MOSFETRegion


def analyze_common_source_amp(
    transistor: NMOS,
    vgs: float,
    vdd: float,
    r_drain: float,
    c_load: float = 1e-12,
) -> AmplifierAnalysis:
    """Analyze an NMOS common-source amplifier configuration.

    The common-source amplifier is the most basic MOSFET amplifier topology.
    The input signal is applied to the gate, and the output is taken from
    the drain. A drain resistor (Rd) converts the drain current variation
    into a voltage swing.

    For the amplifier to work, the MOSFET must be biased in SATURATION:
        Vgs > Vth AND Vds >= Vgs - Vth

    Args:
        transistor: NMOS transistor instance with desired parameters.
        vgs: DC gate-to-source bias voltage (V). Must be > Vth for amplification.
        vdd: Supply voltage (V).
        r_drain: Drain resistor value (ohms). Higher Rd = more gain but less bandwidth.
        c_load: Output load capacitance (F). Default 1 pF.

    Returns:
        AmplifierAnalysis with gain, impedance, and bandwidth.
    """
    # Calculate DC operating point
    ids = transistor.drain_current(vgs, vdd)  # Approximate: Vds ≈ Vdd initially
    vds = vdd - ids * r_drain  # Actual drain voltage

    # Recalculate with correct Vds
    ids = transistor.drain_current(vgs, max(vds, 0.0))
    vds = vdd - ids * r_drain

    # Transconductance
    gm = transistor.transconductance(vgs, max(vds, 0.0))

    # Voltage gain: Av = -gm × Rd
    # Negative because it's an inverting amplifier
    voltage_gain = -gm * r_drain

    # Input impedance: essentially infinite for MOSFET (gate is insulated)
    # In practice limited by gate leakage, but we model it as very high
    input_impedance = 1e12  # 1 TΩ (typical MOSFET gate impedance)

    # Output impedance: approximately Rd in parallel with transistor output resistance
    # For simplicity, Zout ≈ Rd
    output_impedance = r_drain

    # Bandwidth: f_3dB = 1 / (2π × Rd × C_load)
    # This is the frequency at which gain drops to 70.7% (-3dB)
    bandwidth = 1.0 / (2.0 * math.pi * r_drain * c_load)

    operating_point = {
        "vgs": vgs,
        "vds": vds,
        "ids": ids,
        "gm": gm,
    }

    return AmplifierAnalysis(
        voltage_gain=voltage_gain,
        transconductance=gm,
        input_impedance=input_impedance,
        output_impedance=output_impedance,
        bandwidth=bandwidth,
        operating_point=operating_point,
    )


def analyze_common_emitter_amp(
    transistor: NPN,
    vbe: float,
    vcc: float,
    r_collector: float,
    c_load: float = 1e-12,
) -> AmplifierAnalysis:
    """Analyze an NPN common-emitter amplifier configuration.

    The BJT equivalent of the common-source amplifier. Input is applied
    to the base, output taken from the collector.

    BJT amplifiers typically have higher voltage gain than MOSFET amplifiers
    at the same current, because BJT transconductance (gm = Ic/Vt) is
    higher than MOSFET transconductance (gm = 2*Ids/(Vgs-Vth)) for the
    same bias current.

    However, BJT amplifiers have lower input impedance because base current
    flows continuously.

    Args:
        transistor: NPN transistor instance.
        vbe: DC base-emitter bias voltage (V). Should be ~0.7V for active region.
        vcc: Supply voltage (V).
        r_collector: Collector resistor value (ohms).
        c_load: Output load capacitance (F).

    Returns:
        AmplifierAnalysis with gain, impedance, and bandwidth.
    """
    # Calculate DC operating point
    vce = vcc  # Initial approximation
    ic = transistor.collector_current(vbe, vce)
    vce = vcc - ic * r_collector
    vce = max(vce, 0.0)

    # Recalculate with correct Vce
    ic = transistor.collector_current(vbe, vce)

    # Transconductance
    gm = transistor.transconductance(vbe, vce)

    # Voltage gain: Av = -gm × Rc
    voltage_gain = -gm * r_collector

    # Input impedance: r_pi = beta / gm = beta * Vt / Ic
    beta = transistor.params.beta
    vt = 0.026
    if ic > 0:
        r_pi = beta * vt / ic
    else:
        r_pi = 1e12  # Very high when no current flows

    input_impedance = r_pi
    output_impedance = r_collector

    # Bandwidth
    bandwidth = 1.0 / (2.0 * math.pi * r_collector * c_load)

    operating_point = {
        "vbe": vbe,
        "vce": vce,
        "ic": ic,
        "ib": transistor.base_current(vbe, vce),
        "gm": gm,
    }

    return AmplifierAnalysis(
        voltage_gain=voltage_gain,
        transconductance=gm,
        input_impedance=input_impedance,
        output_impedance=output_impedance,
        bandwidth=bandwidth,
        operating_point=operating_point,
    )

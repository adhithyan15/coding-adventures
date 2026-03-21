"""Electrical Analysis — noise margins, power, timing, and technology comparison.

=== Why Electrical Analysis Matters ===

Digital logic designers don't just care about truth tables — they care about:

1. NOISE MARGINS: Can the circuit tolerate voltage fluctuations on the wires?
   A chip has billions of wires running millimeters apart, each creating
   electromagnetic interference on its neighbors. If noise margins are too
   small, a logic 1 could be misread as a logic 0.

2. POWER: How much energy does the chip consume? A modern CPU runs at
   ~100 watts. This power must be dissipated as heat, requiring fans,
   heat sinks, and thermal management. Power = the #1 constraint in
   modern chip design.

3. TIMING: How fast can the circuit switch? The propagation delay through
   a gate determines the maximum clock frequency. A gate with 100ps delay
   can run at ~5 GHz. Faster gates = faster chips, but also more power.

4. SCALING: How do these properties change as we shrink transistors?
   Moore's Law predicts transistor count doubles every ~2 years, but the
   "power wall" (dynamic power) and the "leakage wall" (static power)
   constrain how far we can push frequency and voltage.
"""

from __future__ import annotations

from transistors.cmos_gates import CMOSInverter, CMOSNand, CMOSNor
from transistors.ttl_gates import TTLNand
from transistors.types import (
    CircuitParams,
    MOSFETParams,
    NoiseMargins,
    PowerAnalysis,
    TimingAnalysis,
)


def compute_noise_margins(
    gate: CMOSInverter | TTLNand,
) -> NoiseMargins:
    """Analyze noise margins for a gate.

    Noise margins tell you how much electrical noise a digital signal
    can tolerate before being misinterpreted by the next gate in the chain.

    We compute the noise margins by evaluating the gate's transfer
    characteristic:
        VOL: output voltage when driving LOW
        VOH: output voltage when driving HIGH
        VIL: maximum input voltage still recognized as LOW
        VIH: minimum input voltage still recognized as HIGH

    For CMOS:
        VOL ≈ 0V, VOH ≈ Vdd → large noise margins
        NML ≈ NMH ≈ 0.4 × Vdd (symmetric)

    For TTL:
        VOL ≈ 0.2V, VOH ≈ 3.5V → smaller margins
        VIL = 0.8V, VIH = 2.0V (defined by spec)
    """
    if isinstance(gate, CMOSInverter):
        vdd = gate.circuit.vdd
        # CMOS has nearly ideal rail-to-rail output
        vol = 0.0
        voh = vdd
        # Input thresholds at ~40% and ~60% of Vdd (symmetric CMOS)
        vil = 0.4 * vdd
        vih = 0.6 * vdd
    elif isinstance(gate, TTLNand):
        # TTL specifications (standard 74xx series)
        vol = 0.2  # Vce_sat of output transistor
        voh = gate.vcc - 0.7  # Vcc minus one diode drop
        vil = 0.8  # Standard TTL input LOW threshold
        vih = 2.0  # Standard TTL input HIGH threshold
    else:
        msg = f"Unsupported gate type: {type(gate)}"
        raise TypeError(msg)

    nml = vil - vol
    nmh = voh - vih

    return NoiseMargins(
        vol=vol,
        voh=voh,
        vil=vil,
        vih=vih,
        nml=nml,
        nmh=nmh,
    )


def analyze_power(
    gate: CMOSInverter | CMOSNand | CMOSNor | TTLNand,
    frequency: float = 1e9,
    c_load: float = 1e-12,
    activity_factor: float = 0.5,
) -> PowerAnalysis:
    """Compute power consumption for a gate at a given operating frequency.

    === Power in CMOS ===

    P_total = P_static + P_dynamic

    P_static = V_dd × I_leakage ≈ negligible (nanowatts)

    P_dynamic = C_load × Vdd^2 × f × alpha
        where alpha = activity factor (fraction of clock cycles
        in which the output actually switches)

    === Power in TTL ===

    P_total = P_static + P_dynamic

    P_static = V_cc × I_cc ≈ milliwatts (DOMINATES!)

    P_dynamic = similar formula but static power is so large
    that it barely matters.

    Args:
        gate: The gate to analyze.
        frequency: Operating frequency in Hz (default 1 GHz).
        c_load: Load capacitance in Farads (default 1 pF).
        activity_factor: Fraction of cycles with output transition (0-1).
    """
    if isinstance(gate, TTLNand):
        # TTL: static power dominates
        static = gate.static_power
        vdd = gate.vcc
    elif isinstance(gate, (CMOSInverter, CMOSNand, CMOSNor)):
        # CMOS: static power is negligible
        static = 0.0  # Ideal CMOS has zero static power
        vdd = gate.circuit.vdd
    else:
        msg = f"Unsupported gate type: {type(gate)}"
        raise TypeError(msg)

    # Dynamic power: P = C × V^2 × f × alpha
    dynamic = c_load * vdd * vdd * frequency * activity_factor
    total = static + dynamic

    # Energy per switching event: E = C × V^2
    energy_per_switch = c_load * vdd * vdd

    return PowerAnalysis(
        static_power=static,
        dynamic_power=dynamic,
        total_power=total,
        energy_per_switch=energy_per_switch,
    )


def analyze_timing(
    gate: CMOSInverter | CMOSNand | CMOSNor | TTLNand,
    c_load: float = 1e-12,
) -> TimingAnalysis:
    """Compute timing characteristics for a gate.

    === Propagation Delay ===

    The time it takes for a change at the input to appear at the output.

    For CMOS:
        t_pd ≈ (C_load × Vdd) / (2 × I_sat)

        I_sat = 0.5 × k × (Vdd - Vth)^2 (saturation current of MOSFET)

        Lower Vdd → less current → SLOWER (speed-power tradeoff)

    For TTL:
        t_pd ≈ 5-15 ns (fixed by transistor switching speed)

    === Rise and Fall Times ===

        Rise time: time for output to go from 10% to 90% of Vdd
        Fall time: time for output to go from 90% to 10% of Vdd

        t_rise ≈ 2.2 × R_p × C_load (PMOS pull-up)
        t_fall ≈ 2.2 × R_n × C_load (NMOS pull-down)

    Args:
        gate: The gate to analyze.
        c_load: Load capacitance in Farads (default 1 pF).
    """
    if isinstance(gate, TTLNand):
        # TTL has relatively fixed timing characteristics
        tphl = 7e-9    # HIGH to LOW: ~7 ns
        tplh = 11e-9   # LOW to HIGH: ~11 ns (slower pull-up)
        tpd = (tphl + tplh) / 2.0
        rise_time = 15e-9
        fall_time = 10e-9
    elif isinstance(gate, (CMOSInverter, CMOSNand, CMOSNor)):
        vdd = gate.circuit.vdd

        # Get NMOS parameters for pull-down timing
        if isinstance(gate, CMOSInverter):
            nmos = gate.nmos
            pmos = gate.pmos
        else:
            nmos = gate.nmos1
            pmos = gate.pmos1

        # Saturation current (approximation for timing)
        k = nmos.params.k
        vth = nmos.params.vth
        ids_sat_n = 0.5 * k * (vdd - vth) ** 2 if vdd > vth else 1e-12
        ids_sat_p = 0.5 * pmos.params.k * (vdd - pmos.params.vth) ** 2 if vdd > pmos.params.vth else 1e-12

        # Propagation delays
        tphl = c_load * vdd / (2.0 * ids_sat_n)  # Pull-down (NMOS)
        tplh = c_load * vdd / (2.0 * ids_sat_p)  # Pull-up (PMOS)
        tpd = (tphl + tplh) / 2.0

        # Rise and fall times (2.2 RC time constants)
        # R_on ≈ Vdd / (2 × Ids_sat)
        r_on_n = vdd / (2.0 * ids_sat_n) if ids_sat_n > 0 else 1e6
        r_on_p = vdd / (2.0 * ids_sat_p) if ids_sat_p > 0 else 1e6
        rise_time = 2.2 * r_on_p * c_load
        fall_time = 2.2 * r_on_n * c_load
    else:
        msg = f"Unsupported gate type: {type(gate)}"
        raise TypeError(msg)

    max_frequency = 1.0 / (2.0 * tpd) if tpd > 0 else float("inf")

    return TimingAnalysis(
        tphl=tphl,
        tplh=tplh,
        tpd=tpd,
        rise_time=rise_time,
        fall_time=fall_time,
        max_frequency=max_frequency,
    )


def compare_cmos_vs_ttl(
    frequency: float = 1e6,
    c_load: float = 1e-12,
) -> dict[str, dict[str, float | int]]:
    """Compare CMOS and TTL NAND gates across all metrics.

    This function demonstrates WHY CMOS replaced TTL:
    - CMOS has ~1000x less static power
    - CMOS has better noise margins (relative to Vdd)
    - CMOS can operate at lower voltages
    - CMOS gates use fewer transistors

    The only metric where TTL historically won was raw speed, but CMOS
    caught up and surpassed TTL by the 1990s.
    """
    cmos_nand = CMOSNand()
    ttl_nand = TTLNand()

    cmos_power = analyze_power(cmos_nand, frequency, c_load)
    ttl_power = analyze_power(ttl_nand, frequency, c_load)

    cmos_timing = analyze_timing(cmos_nand, c_load)
    ttl_timing = analyze_timing(ttl_nand, c_load)

    cmos_nm = compute_noise_margins(CMOSInverter())
    ttl_nm = compute_noise_margins(ttl_nand)

    return {
        "cmos": {
            "transistor_count": 4,
            "supply_voltage": cmos_nand.circuit.vdd,
            "static_power_w": cmos_power.static_power,
            "dynamic_power_w": cmos_power.dynamic_power,
            "total_power_w": cmos_power.total_power,
            "propagation_delay_s": cmos_timing.tpd,
            "max_frequency_hz": cmos_timing.max_frequency,
            "noise_margin_low_v": cmos_nm.nml,
            "noise_margin_high_v": cmos_nm.nmh,
        },
        "ttl": {
            "transistor_count": 3,
            "supply_voltage": ttl_nand.vcc,
            "static_power_w": ttl_power.static_power,
            "dynamic_power_w": ttl_power.dynamic_power,
            "total_power_w": ttl_power.total_power,
            "propagation_delay_s": ttl_timing.tpd,
            "max_frequency_hz": ttl_timing.max_frequency,
            "noise_margin_low_v": ttl_nm.nml,
            "noise_margin_high_v": ttl_nm.nmh,
        },
    }


def demonstrate_cmos_scaling(
    technology_nodes: list[float] | None = None,
) -> list[dict[str, float]]:
    """Show how CMOS performance changes with technology scaling.

    As transistors shrink (Moore's Law), several properties change:
    - Gate length decreases → faster switching
    - Supply voltage decreases → less power per switch
    - Gate capacitance decreases → less energy per transition
    - BUT leakage current INCREASES → more static power (the "leakage wall")

    This function models these trends for common technology nodes.
    """
    if technology_nodes is None:
        technology_nodes = [180e-9, 90e-9, 45e-9, 22e-9, 7e-9, 3e-9]

    results: list[dict[str, float]] = []

    for node in technology_nodes:
        # Empirical scaling relationships (simplified)
        # These approximate real industry trends
        scale = node / 180e-9  # Relative to 180nm baseline

        vdd = max(0.7, 3.3 * scale**0.5)  # Vdd scales slower than geometry
        vth = max(0.15, 0.4 * scale**0.3)  # Vth can't scale as fast (leakage)
        c_gate = 1e-15 * scale  # Capacitance scales linearly with area
        k = 0.001 / scale**0.5  # Transconductance improves with scaling

        # Create transistor and circuit with scaled parameters
        params = MOSFETParams(vth=vth, k=k, l=node, c_gate=c_gate)
        circuit = CircuitParams(vdd=vdd)
        inv = CMOSInverter(circuit, params, params)

        timing = analyze_timing(inv, c_load=c_gate * 10)
        power = analyze_power(inv, frequency=1e9, c_load=c_gate * 10)

        # Leakage current increases exponentially as Vth decreases
        # I_leak ∝ exp(-Vth / (n * Vt)) where Vt ≈ 26mV
        import math

        leakage = 1e-12 * math.exp((0.4 - vth) / 0.052)  # Relative to 180nm

        results.append(
            {
                "node_nm": node * 1e9,
                "vdd_v": vdd,
                "vth_v": vth,
                "c_gate_f": c_gate,
                "propagation_delay_s": timing.tpd,
                "dynamic_power_w": power.dynamic_power,
                "leakage_current_a": leakage,
                "max_frequency_hz": timing.max_frequency,
            }
        )

    return results

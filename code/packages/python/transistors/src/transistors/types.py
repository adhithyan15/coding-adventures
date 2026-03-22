"""Shared types for the transistors package.

=== Enums and Parameter Dataclasses ===

These types define the vocabulary of transistor simulation. Every transistor
has an operating region (cutoff, linear, saturation), and every circuit has
electrical parameters (voltage, capacitance, etc.).

We use frozen dataclasses for parameters because transistor characteristics
are fixed once manufactured — you can't change a transistor's threshold
voltage after fabrication. Freezing enforces this immutability in code.
"""

from dataclasses import dataclass
from enum import Enum


# ===========================================================================
# OPERATING REGION ENUMS
# ===========================================================================
# A transistor is an analog device that operates differently depending on
# the voltages applied to its terminals. The three "regions" describe these
# different operating modes.


class MOSFETRegion(Enum):
    """Operating region of a MOSFET transistor.

    Think of it like a water faucet with three positions:

        CUTOFF:     Faucet is fully closed. No water flows.
                    (Vgs < Vth — gate voltage too low to turn on)

        LINEAR:     Faucet is open, and water flow increases as you
                    turn the handle more. Flow is proportional to
                    both handle position AND water pressure.
                    (Vgs > Vth, Vds < Vgs - Vth — acts like a resistor)

        SATURATION: Faucet is wide open, but the pipe is the bottleneck.
                    Adding more pressure doesn't increase flow much.
                    (Vgs > Vth, Vds >= Vgs - Vth — current is roughly constant)

    For digital circuits, we only use CUTOFF (OFF) and deep LINEAR (ON).
    For analog amplifiers, we operate in SATURATION.
    """

    CUTOFF = "cutoff"
    LINEAR = "linear"
    SATURATION = "saturation"


class BJTRegion(Enum):
    """Operating region of a BJT transistor.

    Similar to MOSFET regions but with different names and physics:

        CUTOFF:      No base current → no collector current. Switch OFF.
                     (Vbe < ~0.7V)

        ACTIVE:      Small base current, large collector current.
                     Ic = beta * Ib. This is the AMPLIFIER region.
                     (Vbe >= ~0.7V, Vce > ~0.2V)

        SATURATION:  Both junctions forward-biased. Collector current
                     is maximum — transistor is fully ON as a switch.
                     (Vbe >= ~0.7V, Vce <= ~0.2V)

    Confusing naming alert: MOSFET "saturation" = constant current (amplifier).
    BJT "saturation" = fully ON (switch). These are DIFFERENT behaviors despite
    sharing a name. Hardware engineers have been confusing students with this
    for decades.
    """

    CUTOFF = "cutoff"
    ACTIVE = "active"
    SATURATION = "saturation"


class TransistorType(Enum):
    """Transistor polarity/type."""

    NMOS = "nmos"
    PMOS = "pmos"
    NPN = "npn"
    PNP = "pnp"


# ===========================================================================
# ELECTRICAL PARAMETERS
# ===========================================================================
# These dataclasses hold the physical characteristics of transistors.
# Default values represent common, well-documented transistor types
# so that users can start experimenting immediately without needing
# to look up datasheets.


@dataclass(frozen=True)
class MOSFETParams:
    """Electrical parameters for a MOSFET transistor.

    Default values represent a typical 180nm CMOS process — the last
    "large" process node that is still widely used in education and
    analog/mixed-signal chips.

    Key parameters:
        vth:     Threshold voltage — the minimum Vgs to turn the transistor ON.
                 Lower Vth = faster switching but more leakage current.
                 Modern CPUs use Vth around 0.2-0.4V.

        k:       Transconductance parameter — controls how much current flows
                 for a given Vgs. Higher k = more current = faster but more power.
                 k = mu * Cox * (W/L) where mu is carrier mobility and Cox is
                 oxide capacitance per unit area.

        w, l:    Channel width and length. The W/L ratio is the main knob
                 chip designers use to tune transistor strength. Wider = more
                 current. Shorter = faster but harder to manufacture.

        c_gate:  Gate capacitance — determines switching speed. The gate
                 capacitor must charge/discharge to switch the transistor,
                 so smaller C = faster switching.

        c_drain: Drain junction capacitance — contributes to output load.
    """

    vth: float = 0.4
    k: float = 0.001
    w: float = 1e-6
    l: float = 180e-9
    c_gate: float = 1e-15
    c_drain: float = 0.5e-15


@dataclass(frozen=True)
class BJTParams:
    """Electrical parameters for a BJT transistor.

    Default values represent a typical small-signal NPN transistor
    like the 2N2222 — one of the most common transistors ever made,
    used in everything from hobby projects to early spacecraft.

    Key parameters:
        beta:    Current gain (hfe) — the ratio Ic/Ib. A beta of 100
                 means 1mA of base current controls 100mA of collector
                 current. This amplification is what made transistors
                 revolutionary. Vacuum tubes could amplify too, but
                 they were huge, hot, and unreliable.

        vbe_on:  Base-emitter voltage when conducting. For silicon BJTs,
                 this is always around 0.6-0.7V — it's a fundamental
                 property of the silicon PN junction.

        vce_sat: Collector-emitter voltage when fully saturated (switch ON).
                 Ideally 0V, practically about 0.1-0.3V.

        is_:     Reverse saturation current — the tiny leakage current
                 that flows even when the transistor is OFF. Named is_
                 with trailing underscore because 'is' is a Python keyword.

        c_base:  Base capacitance — limits switching speed.
    """

    beta: float = 100.0
    vbe_on: float = 0.7
    vce_sat: float = 0.2
    is_: float = 1e-14
    c_base: float = 5e-12


@dataclass(frozen=True)
class CircuitParams:
    """Parameters for a complete logic gate circuit.

    vdd:         Supply voltage. Modern CMOS uses 0.7-1.2V, older CMOS
                 used 3.3V or 5V, TTL always uses 5V. Lower voltage
                 means less power (P scales with V^2) but also less
                 noise margin and slower switching.

    temperature: Junction temperature in Kelvin. Room temperature is
                 ~300K (27C). Higher temperature increases leakage
                 current and reduces carrier mobility. A CPU under
                 load might reach 370K (97C).
    """

    vdd: float = 3.3
    temperature: float = 300.0


# ===========================================================================
# RESULT TYPES
# ===========================================================================
# These dataclasses hold the results of transistor and circuit analysis.
# Each one bundles together related measurements so callers don't need
# to track multiple return values.


@dataclass(frozen=True)
class GateOutput:
    """Result of evaluating a logic gate with voltage-level detail.

    Unlike the logic_gates package which only returns 0 or 1, this gives
    you the full electrical picture: what voltage does the output actually
    sit at? How much power is being consumed? How long did the signal
    take to propagate?

    This is the difference between a logic simulator and a circuit simulator.
    """

    logic_value: int
    voltage: float
    current_draw: float
    power_dissipation: float
    propagation_delay: float
    transistor_count: int


@dataclass(frozen=True)
class AmplifierAnalysis:
    """Results of analyzing a transistor as an amplifier.

    When a transistor operates in its linear/active region (not as a
    digital switch), it can amplify signals. These parameters describe
    the quality of that amplification.

    voltage_gain:      How much the output voltage changes per unit change
                       in input voltage. Negative for inverting amplifiers
                       (common-source MOSFET, common-emitter BJT).

    transconductance:  gm — the ratio of output current change to input
                       voltage change. The fundamental amplification parameter.
                       Units: Siemens (A/V).

    input_impedance:   How much the amplifier "loads" the signal source.
                       MOSFET: very high (>1 GΩ) because the gate is insulated.
                       BJT: moderate (~1-10 kΩ) because base current flows.

    output_impedance:  How "stiff" the output is. Lower = can drive heavier loads.

    bandwidth:         Frequency at which gain drops to 70.7% (-3dB) of its
                       low-frequency value. Limited by parasitic capacitances.
    """

    voltage_gain: float
    transconductance: float
    input_impedance: float
    output_impedance: float
    bandwidth: float
    operating_point: dict[str, float]


@dataclass(frozen=True)
class NoiseMargins:
    """Noise margin analysis for a logic family.

    Noise margins tell you how much electrical noise (voltage fluctuation)
    a digital signal can tolerate before being misinterpreted. Think of it
    as the "safety zone" between what counts as HIGH and what counts as LOW.

    In a noisy environment (like a CPU with billions of switching transistors),
    adequate noise margins are essential for reliable operation.

        vol: Output LOW voltage — what the gate actually outputs for logic 0
        voh: Output HIGH voltage — what the gate actually outputs for logic 1
        vil: Input LOW threshold — maximum voltage the next gate accepts as 0
        vih: Input HIGH threshold — minimum voltage the next gate accepts as 1

        nml: Noise Margin LOW  = vil - vol (how much noise a LOW can tolerate)
        nmh: Noise Margin HIGH = voh - vih (how much noise a HIGH can tolerate)
    """

    vol: float
    voh: float
    vil: float
    vih: float
    nml: float
    nmh: float


@dataclass(frozen=True)
class PowerAnalysis:
    """Power consumption breakdown for a gate or circuit.

    static_power:      Power consumed even when the gate is not switching.
                       For CMOS: dominated by transistor leakage (~nW).
                       For TTL: dominated by resistor bias current (~mW).

    dynamic_power:     Power consumed during switching transitions.
                       P_dyn = C_load * Vdd^2 * f * alpha
                       This dominates in modern CMOS circuits.

    total_power:       static + dynamic.

    energy_per_switch: Energy for one complete 0→1→0 transition.
                       E = C_load * Vdd^2. Useful for comparing efficiency.
    """

    static_power: float
    dynamic_power: float
    total_power: float
    energy_per_switch: float


@dataclass(frozen=True)
class TimingAnalysis:
    """Timing characteristics for a gate.

    tphl:          Propagation delay from HIGH to LOW output.
    tplh:          Propagation delay from LOW to HIGH output.
    tpd:           Average propagation delay = (tphl + tplh) / 2.
    rise_time:     Time for output to go from 10% to 90% of Vdd.
    fall_time:     Time for output to go from 90% to 10% of Vdd.
    max_frequency: Maximum clock frequency = 1 / (2 * tpd).
    """

    tphl: float
    tplh: float
    tpd: float
    rise_time: float
    fall_time: float
    max_frequency: float

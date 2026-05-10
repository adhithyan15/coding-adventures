"""Circuit element data classes."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class Resistor:
    """R<name> n+ n- value"""

    name: str
    n_plus: str  # node identifier
    n_minus: str
    resistance: float  # ohms


@dataclass(frozen=True, slots=True)
class Capacitor:
    """C<name> n+ n- value"""

    name: str
    n_plus: str
    n_minus: str
    capacitance: float  # farads
    initial_voltage: float = 0.0


@dataclass(frozen=True, slots=True)
class Inductor:
    """L<name> n+ n- value"""

    name: str
    n_plus: str
    n_minus: str
    inductance: float  # henries


@dataclass(frozen=True, slots=True)
class VoltageSource:
    """V<name> n+ n- value"""

    name: str
    n_plus: str
    n_minus: str
    voltage: float  # volts


@dataclass(frozen=True, slots=True)
class CurrentSource:
    """I<name> n+ n- value (current flows from n+ to n-)"""

    name: str
    n_plus: str
    n_minus: str
    current: float  # amperes


@dataclass(frozen=True, slots=True)
class Diode:
    """Simple diode using Shockley equation."""

    name: str
    anode: str
    cathode: str
    Is: float = 1e-15  # saturation current
    Vt: float = 0.02585  # thermal voltage


@dataclass(frozen=True, slots=True)
class Mosfet:
    """A MOSFET instance backed by a mosfet_models.MOSFET model."""

    name: str
    drain: str
    gate: str
    source: str
    body: str
    model: object  # mosfet_models.MOSFET; using `object` to avoid Protocol overhead


@dataclass(frozen=True, slots=True)
class BJT:
    """Bipolar Junction Transistor — simplified Ebers-Moll (forward active).

    The BJT is a three-terminal current-controlled device. In the forward-
    active region it behaves like a current-amplifying element:

    - **NPN**: base–emitter junction forward biased (Vbe = Vb − Ve > 0).
      Collector current flows *into* the collector.
      Ic = Is * (exp(Vbe / Vt) − 1)

    - **PNP**: emitter–base junction forward biased (Veb = Ve − Vb > 0).
      Collector current flows *out of* the collector.
      Ic = Is * (exp(Veb / Vt) − 1)

    MNA linearisation (Newton step)
    --------------------------------
    Around operating point voltage Vjunc (Vbe or Veb, clamped to 0.7 V):

        exp_term = exp(Vjunc / Vt)
        Ic0      = Is * (exp_term − 1)          # DC collector current
        gm       = (Is / Vt) * exp_term          # transconductance (A/V)
        gπ       = gm / beta_f                   # base–emitter conductance
        Ib0      = Ic0 / beta_f                  # base current at OP

    Two stamps are applied:

    1. **Junction stamp** (gπ between B-E for NPN, E-B for PNP):
       Models the base–emitter diode resistance.  Stamped via `_stamp_g`.

    2. **Transconductance VCCS** (gm × Vjunc controls Ic):
       A voltage-controlled current source whose controlling voltage is
       Vjunc.  For NPN the source-node pair is (E, B) and the drain-node
       pair is (C, E).  The G-matrix rows for C and E each get ±gm
       contributions from the B and E columns.

    Companion current sources (Norton equivalents) are added to the RHS
    vector b so that the linear system G·x = b is consistent at Vjunc:

        Ieq_junction = Ib0 − gπ * Vjunc   (junction Norton offset)
        Ieq_collector = Ic0 − gm * Vjunc  (VCCS Norton offset)

    Parameters
    ----------
    name:
        Instance identifier (e.g. "Q1").
    collector, base, emitter:
        Node names.  Any may be ground ("0", "gnd", "GND").
    polarity:
        "NPN" (default) or "PNP".
    Is:
        Saturation current in Amperes (default 10 fA).
    beta_f:
        Forward current gain hFE (default 100).
    Vt:
        Thermal voltage in Volts.  At 300 K, Vt = kT/q ≈ 25.85 mV.
    """

    name: str
    collector: str
    base: str
    emitter: str
    polarity: str = "NPN"   # "NPN" or "PNP"
    Is: float = 1e-14       # saturation current (A)
    beta_f: float = 100.0   # forward current gain hFE
    Vt: float = 0.02585     # thermal voltage (V) at ~300 K


Element = (
    Resistor
    | Capacitor
    | Inductor
    | VoltageSource
    | CurrentSource
    | Diode
    | Mosfet
    | BJT
)

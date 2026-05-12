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


# ---------------------------------------------------------------------------
# Controlled (dependent) sources — SPICE E / G / F / H elements
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class VCVS:
    """Voltage-Controlled Voltage Source (SPICE ``E`` element).

    The output voltage is proportional to the voltage difference between two
    controlling nodes:

        V(n_plus, n_minus) = gain × [V(ctrl_plus) − V(ctrl_minus)]

    MNA stamp
    ---------
    Like an independent VoltageSource, a VCVS introduces a **branch unknown**
    for its output current (call it ``I_k``).  The KVL constraint equation
    and the output-port KCL rows look like this in the MNA matrix::

        row n_plus  : … + I_k = 0
        row n_minus : … − I_k = 0
        row k (KVL) : V_n_plus − V_n_minus
                      − gain × V_ctrl_plus + gain × V_ctrl_minus = 0

    In matrix notation (with column indices for ctrl_plus / ctrl_minus
    written as ``cp`` / ``cm``)::

        G[n_plus][k]   +=  1       (KCL: I_k exits n_plus)
        G[n_minus][k]  -=  1       (KCL: I_k enters n_minus)
        G[k][n_plus]   +=  1       (KVL: +V_n_plus)
        G[k][n_minus]  -=  1       (KVL: −V_n_minus)
        G[k][cp]       -=  gain    (KVL: −gain × V_ctrl_plus)
        G[k][cm]       +=  gain    (KVL: +gain × V_ctrl_minus)
        b[k]            =  0       (KVL RHS: ideal V-source → always 0)

    The branch unknown index ``k`` is ``n_nodes + position`` where
    ``position`` is the element's index in the ``_branch_sources`` list.

    Parameters
    ----------
    name:
        Instance name, e.g. ``"E1"``.
    n_plus, n_minus:
        Output port nodes (positive and negative terminals).
    ctrl_plus, ctrl_minus:
        Controlling (sensing) port nodes.
    gain:
        Dimensionless voltage gain (can be negative for inverting behaviour).

    Examples
    --------
    Unity-gain buffer (voltage follower):

        E1 out 0 in 0 1.0

    Inverting amplifier with gain −10:

        E_amp out 0 in 0 -10.0
    """

    name: str
    n_plus: str    # output positive
    n_minus: str   # output negative
    ctrl_plus: str   # controlling node +
    ctrl_minus: str  # controlling node −
    gain: float    # dimensionless voltage gain


@dataclass(frozen=True, slots=True)
class VCCS:
    """Voltage-Controlled Current Source (SPICE ``G`` element).

    The output current is proportional to the voltage difference between two
    controlling nodes:

        I(n_plus → n_minus) = gm × [V(ctrl_plus) − V(ctrl_minus)]

    MNA stamp
    ---------
    A VCCS does **not** introduce a branch unknown — it contributes only
    conductance (off-diagonal) entries into the MNA G matrix.  These entries
    implement the current injection in KCL::

        G[n_plus][ctrl_plus]   +=  gm
        G[n_plus][ctrl_minus]  -=  gm
        G[n_minus][ctrl_plus]  -=  gm
        G[n_minus][ctrl_minus] +=  gm

    This is identical to the MOSFET/BJT transconductance stamp used internally
    by ``_stamp_bjt`` and ``_stamp_mosfet``.  A standalone ``VCCS`` element
    exposes the same primitive to circuit users (op-amp macromodels, etc.).

    Parameters
    ----------
    name:
        Instance name, e.g. ``"G1"``.
    n_plus, n_minus:
        Output port nodes.  Positive current flows from n_plus through the
        external circuit to n_minus.
    ctrl_plus, ctrl_minus:
        Controlling (sensing) port nodes.
    gm:
        Transconductance in Siemens (A/V).

    Examples
    --------
    VCCS with gm = 0.1 A/V, controlled by the differential voltage "in"−"0":

        G1 out 0 in 0 0.1

    Two-port macromodel amplifier (control port + output port):

        Rin  in   0  1e6   # high-impedance input
        G1   out  0  in 0  0.02   # gm = 20 mA/V
        Rout out  0  5000  # 5 kΩ output resistance
    """

    name: str
    n_plus: str    # output positive
    n_minus: str   # output negative
    ctrl_plus: str   # controlling node +
    ctrl_minus: str  # controlling node −
    gm: float      # transconductance (A/V)


@dataclass(frozen=True, slots=True)
class CCCS:
    """Current-Controlled Current Source (SPICE ``F`` element).

    The output current is proportional to the current flowing through a
    designated controlling element (a ``VoltageSource`` used as an ammeter):

        I(n_plus → n_minus) = beta × I(ctrl_source)

    where ``I(ctrl_source)`` is the branch current of the controlling
    ``VoltageSource`` (positive = current flowing into its ``n_plus`` terminal
    in the MNA convention).

    MNA stamp
    ---------
    Because the controlling current is already a branch unknown ``x[k_ctrl]``
    (the column corresponding to the controlling ``VoltageSource``), a CCCS
    adds off-diagonal entries in the MNA rows for its output nodes::

        G[n_plus][k_ctrl]   +=  beta
        G[n_minus][k_ctrl]  -=  beta

    No new branch unknown is needed.  The controlling ``VoltageSource`` is
    typically a 0 V source inserted purely as an ideal ammeter::

        V_sense  mid  mid_2  0.0   # 0 V, just measures current
        F1       out  0      V_sense  beta

    Parameters
    ----------
    name:
        Instance name, e.g. ``"F1"``.
    n_plus, n_minus:
        Output port nodes.
    ctrl_source:
        **Name** of the controlling ``VoltageSource`` (e.g. ``"V_sense"``).
        A ``ValueError`` is raised at simulation time if no ``VoltageSource``
        with this name exists in the circuit.
    beta:
        Dimensionless current gain.

    Examples
    --------
    Current mirror (simplified):

        V_sense  a  b  0.0       # 0 V ammeter in controlling branch
        F1       c  0  V_sense   2.0   # mirror with gain 2
    """

    name: str
    n_plus: str    # output positive
    n_minus: str   # output negative
    ctrl_source: str  # name of controlling VoltageSource
    beta: float    # dimensionless current gain


@dataclass(frozen=True, slots=True)
class CCVS:
    """Current-Controlled Voltage Source (SPICE ``H`` element).

    The output voltage is proportional to the current flowing through a
    designated controlling element (a ``VoltageSource`` used as an ammeter):

        V(n_plus, n_minus) = transresistance × I(ctrl_source)

    MNA stamp
    ---------
    Like a VCVS, a CCVS introduces a **branch unknown** for its own output
    current (call it ``I_k``).  The KVL equation references the controlling
    branch current ``x[k_ctrl]``::

        G[n_plus][k]       +=  1
        G[n_minus][k]      -=  1
        G[k][n_plus]       +=  1
        G[k][n_minus]      -=  1
        G[k][k_ctrl]       -=  transresistance   (KVL: −rm × I_ctrl)
        b[k]                =  0

    Reading the KVL row: ``V_n_plus − V_n_minus − rm × x[k_ctrl] = 0``.

    Parameters
    ----------
    name:
        Instance name, e.g. ``"H1"``.
    n_plus, n_minus:
        Output port nodes.
    ctrl_source:
        **Name** of the controlling ``VoltageSource`` (e.g. ``"V_sense"``).
    transresistance:
        Transresistance (transimpedance) in Ohms.

    Examples
    --------
    Transresistance amplifier with rm = 1000 Ω:

        V_sense  in  0  0.0         # 0 V ammeter, measures input current
        H1       out 0  V_sense  1000.0
    """

    name: str
    n_plus: str    # output positive
    n_minus: str   # output negative
    ctrl_source: str  # name of controlling VoltageSource
    transresistance: float  # Ohms


Element = (
    Resistor
    | Capacitor
    | Inductor
    | VoltageSource
    | CurrentSource
    | Diode
    | Mosfet
    | BJT
    | VCVS
    | VCCS
    | CCCS
    | CCVS
)

"""Circuit element data classes."""

from __future__ import annotations

import bisect
import math
from dataclasses import dataclass

# ---------------------------------------------------------------------------
# Source waveforms — SPICE3 transient source forms
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class PwlWaveform:
    """Piecewise-linear (PWL) voltage or current waveform.

    Linearly interpolates between a sequence of ``(time, value)`` breakpoints.
    Before the first breakpoint the value is clamped to the first value; after
    the last breakpoint it is clamped to the last value.

    SPICE3 syntax::

        V1 a 0 PWL(0 0  1n 0  1.001n 1.8  10n 1.8)

    Example — a 0 → 1.8 V step at t = 1 ns with 1 ps rise time::

        PwlWaveform(points=((0, 0), (1e-9, 0), (1.001e-9, 1.8), (10e-9, 1.8)))

    The ``points`` tuple must contain at least two breakpoints; breakpoints
    must be in strictly increasing time order.  All values may be negative.
    """

    # ((t0, v0), (t1, v1), …) — at least two breakpoints, monotone in time.
    points: tuple[tuple[float, float], ...]

    def __call__(self, t: float) -> float:
        """Return the linearly-interpolated value at time *t*."""
        times = [p[0] for p in self.points]
        values = [p[1] for p in self.points]

        if t <= times[0]:
            return values[0]
        if t >= times[-1]:
            return values[-1]

        # Find the segment [i-1, i] that contains t.
        i = bisect.bisect_right(times, t)
        t0, v0 = times[i - 1], values[i - 1]
        t1, v1 = times[i], values[i]
        slope = (v1 - v0) / (t1 - t0)
        return v0 + slope * (t - t0)


@dataclass(frozen=True, slots=True)
class SinWaveform:
    """Sinusoidal (SIN) voltage or current waveform.

    SPICE3 formula (after the optional delay ``td``)::

        v(t) = offset + amplitude × sin(2π × freq × (t − td))
                      × exp(−damping × (t − td))

    Before the delay time the source holds at ``offset + amplitude × sin(0) =
    offset``.

    SPICE3 syntax::

        V1 a 0 SIN(V_offset V_amplitude FREQ TD THETA)

    The ``damping`` parameter corresponds to SPICE's ``THETA`` (exponential
    decay rate, 1/s).  ``damping = 0`` gives a pure undamped sinusoid.

    Parameters
    ----------
    offset:
        DC offset voltage/current (V or A).
    amplitude:
        Peak amplitude (V or A).
    frequency:
        Frequency in Hz.
    delay:
        Start delay in seconds (default 0).
    damping:
        Exponential decay rate (1/s, default 0 = no damping).
    """

    offset: float = 0.0
    amplitude: float = 1.0
    frequency: float = 1.0
    delay: float = 0.0
    damping: float = 0.0

    def __call__(self, t: float) -> float:
        """Evaluate the sinusoidal waveform at time *t*."""
        if t < self.delay:
            return self.offset
        dt = t - self.delay
        envelope = math.exp(-self.damping * dt) if self.damping != 0.0 else 1.0
        return self.offset + self.amplitude * math.sin(2.0 * math.pi * self.frequency * dt) * envelope


@dataclass(frozen=True, slots=True)
class PulseWaveform:
    """Rectangular pulse (PULSE) voltage or current waveform.

    Generates a periodic trapezoidal pulse train:

    - From ``t = 0`` to ``t = td``: holds at *v_initial*.
    - Rise over *tr* seconds from *v_initial* to *v_pulsed*.
    - Holds at *v_pulsed* for *pw* seconds.
    - Falls over *tf* seconds back to *v_initial*.
    - Holds at *v_initial* until the next period starts at ``td + period``.
    - Repeats with period *period*.

    SPICE3 syntax::

        V1 a 0 PULSE(V1 V2 TD TR TF PW PER)

    Parameters
    ----------
    v_initial:
        Value before and between pulses (V or A).
    v_pulsed:
        Peak pulse value (V or A).
    delay:
        Time before first pulse edge (s, default 0).
    rise_time:
        Rise time (s, default 0).
    fall_time:
        Fall time (s, default 0).
    pulse_width:
        Width of the high phase (s, default half-period).
    period:
        Full cycle period (s, default 1).
    """

    v_initial: float = 0.0
    v_pulsed: float = 1.0
    delay: float = 0.0
    rise_time: float = 0.0
    fall_time: float = 0.0
    pulse_width: float = 0.5
    period: float = 1.0

    def __call__(self, t: float) -> float:
        """Evaluate the pulse waveform at time *t*."""
        if t < self.delay:
            return self.v_initial

        # Fold into the current period.
        t_rel = (t - self.delay) % self.period

        tr = max(self.rise_time, 0.0)
        tf = max(self.fall_time, 0.0)
        pw = self.pulse_width

        if tr > 0 and t_rel < tr:
            # Rising edge
            return self.v_initial + (self.v_pulsed - self.v_initial) * (t_rel / tr)
        elif t_rel < tr + pw:
            # High phase (flat top)
            return self.v_pulsed
        elif tf > 0 and t_rel < tr + pw + tf:
            # Falling edge
            phase = (t_rel - tr - pw) / tf
            return self.v_pulsed + (self.v_initial - self.v_pulsed) * phase
        else:
            # Low phase (between pulses)
            return self.v_initial


@dataclass(frozen=True, slots=True)
class ExpWaveform:
    """Double-exponential (EXP) voltage or current waveform.

    Models a rising exponential followed by a falling exponential:

    - For ``t < rise_delay``:           ``v = v_initial``
    - For ``rise_delay ≤ t < fall_delay``:
      ``v = v_initial + (v_pulsed − v_initial) × (1 − exp(−(t − td1)/tc1))``
    - For ``t ≥ fall_delay``:
      adds the falling component back towards *v_initial*.

    SPICE3 syntax::

        V1 a 0 EXP(V1 V2 TD1 TC1 TD2 TC2)

    Parameters
    ----------
    v_initial:
        Value before the rising edge (V or A).
    v_pulsed:
        Peak (asymptote) value reached by the rising exponential (V or A).
    rise_delay:
        Time constant start for the rising edge (s, default 0).
    rise_tc:
        Rising time constant (s, default 1).
    fall_delay:
        Time constant start for the falling edge (s, default 1).
    fall_tc:
        Falling time constant (s, default 1).
    """

    v_initial: float = 0.0
    v_pulsed: float = 1.0
    rise_delay: float = 0.0
    rise_tc: float = 1.0
    fall_delay: float = 1.0
    fall_tc: float = 1.0

    def __call__(self, t: float) -> float:
        """Evaluate the double-exponential waveform at time *t*."""
        if t <= self.rise_delay:
            return self.v_initial

        # Rising component
        value = self.v_initial + (self.v_pulsed - self.v_initial) * (
            1.0 - math.exp(-(t - self.rise_delay) / self.rise_tc)
        )

        # Falling component (kicks in at fall_delay)
        if t >= self.fall_delay:
            value += (self.v_initial - self.v_pulsed) * (
                1.0 - math.exp(-(t - self.fall_delay) / self.fall_tc)
            )

        return value


# Waveform union type — accepted by VoltageSource.waveform and
# CurrentSource.waveform.  A plain callable ``(float) -> float`` (e.g. a
# lambda) is also accepted at runtime; the type alias covers the four named
# forms only.
Waveform = PwlWaveform | SinWaveform | PulseWaveform | ExpWaveform


@dataclass(frozen=True, slots=True)
class AcSource:
    """Small-signal AC source specification.

    ``magnitude`` is the source phasor magnitude in volts or amperes.
    ``phase_degrees`` is the optional phase angle in degrees.
    """

    magnitude: float
    phase_degrees: float = 0.0


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
    initial_current: float = 0.0


@dataclass(frozen=True, slots=True)
class VoltageSource:
    """V<name> n+ n- value [waveform] [ac]

    An independent voltage source.  In DC analysis the ``voltage`` field is
    used directly.  In AC analysis, ``ac`` supplies the small-signal phasor
    when present.  In *transient* analysis, if ``waveform`` is not ``None``
    the engine calls ``waveform(t)`` at each timestep and
    temporarily substitutes the returned value for ``voltage``; the stored
    ``voltage`` field then serves only as the t = 0 bias.

    Example — 1 V sinusoidal source at 1 kHz::

        VoltageSource("V1", "in", "0", voltage=0.0,
                      waveform=SinWaveform(amplitude=1.0, frequency=1e3))
    """

    name: str
    n_plus: str
    n_minus: str
    voltage: float  # volts (DC value / t=0 bias)
    waveform: Waveform | None = None  # time-varying override
    ac: AcSource | None = None  # small-signal AC phasor


@dataclass(frozen=True, slots=True)
class CurrentSource:
    """I<name> n+ n- value [waveform] [ac] (current flows from n+ to n-)

    An independent current source.  In DC analysis the ``current`` field is
    used directly.  In AC analysis, ``ac`` supplies the small-signal phasor
    when present.  In *transient* analysis, if ``waveform`` is not ``None``
    the engine calls ``waveform(t)`` at each timestep and
    temporarily substitutes the returned value for ``current``; the stored
    ``current`` field then serves only as the t = 0 bias.

    Example — pulse current source switching between 0 A and 10 mA::

        CurrentSource("I1", "out", "0", current=0.0,
                      waveform=PulseWaveform(v_initial=0.0, v_pulsed=10e-3,
                                             pulse_width=0.5e-6, period=1e-6))
    """

    name: str
    n_plus: str
    n_minus: str
    current: float  # amperes (DC value / t=0 bias)
    waveform: Waveform | None = None  # time-varying override
    ac: AcSource | None = None  # small-signal AC phasor


@dataclass(frozen=True, slots=True)
class BSource:
    """Behavioral source (SPICE ``B`` element).

    Exactly one expression must be supplied. ``current_expr`` models a current
    flowing from ``n_plus`` to ``n_minus``. ``voltage_expr`` models the voltage
    constraint ``V(n_plus, n_minus)`` and introduces a branch unknown, like an
    ideal voltage source.

    Expressions support numeric constants, ``+``, ``-``, ``*``, ``/``,
    parentheses, and node-voltage references ``V(node)`` or ``V(node1,node2)``.
    """

    name: str
    n_plus: str
    n_minus: str
    voltage_expr: str | None = None
    current_expr: str | None = None


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
    | BSource
    | Diode
    | Mosfet
    | BJT
    | VCVS
    | VCCS
    | CCCS
    | CCVS
)

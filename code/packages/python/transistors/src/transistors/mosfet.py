"""MOSFET Transistors — the building blocks of modern digital circuits.

=== What is a MOSFET? ===

MOSFET stands for Metal-Oxide-Semiconductor Field-Effect Transistor. It is
the most common type of transistor in the world — every CPU, GPU, and phone
chip is built from billions of MOSFETs.

A MOSFET has three terminals:
    Gate (G):   The control terminal. Voltage here controls the switch.
    Drain (D):  Current flows IN here (for NMOS) or OUT here (for PMOS).
    Source (S): Current flows OUT here (for NMOS) or IN here (for PMOS).

The key insight: a MOSFET is VOLTAGE-controlled. Applying a voltage to the
gate creates an electric field that either allows or blocks current flow
between drain and source. No current flows into the gate itself (it's
insulated by a thin oxide layer), which means:
    - Near-zero input power consumption
    - Very high input impedance (good for amplifiers)
    - Can be packed extremely densely on a chip

=== NMOS vs PMOS ===

The two MOSFET types are complementary — they turn on under opposite conditions:

    NMOS: Gate HIGH → ON  (conducts drain to source)
    PMOS: Gate LOW  → ON  (conducts source to drain)

This complementary behavior is the foundation of CMOS (Complementary MOS)
logic. By pairing NMOS and PMOS transistors, we can build gates that consume
near-zero power in steady state — only burning energy during transitions.

=== The Three Operating Regions ===

A MOSFET operates in one of three regions depending on the voltages at
its terminals:

    1. CUTOFF:     Vgs < Vth → transistor is OFF, no current flows.
                   Used as the OFF state in digital logic.

    2. LINEAR:     Vgs > Vth, Vds < (Vgs - Vth) → acts like a variable
                   resistor. Current is proportional to both Vgs and Vds.
                   Used as the ON state in digital logic (deep linear region).

    3. SATURATION: Vgs > Vth, Vds >= (Vgs - Vth) → current is roughly
                   constant regardless of Vds. Used for analog amplifiers.
"""

from __future__ import annotations

import math

from transistors.types import MOSFETParams, MOSFETRegion


class NMOS:
    """N-channel MOSFET transistor.

    An NMOS transistor conducts current from drain to source when the gate
    voltage exceeds the threshold voltage (Vgs > Vth). Think of it as a
    normally-OPEN switch that CLOSES when you apply voltage to the gate.

    === Water analogy ===

        Imagine a water pipe with an electrically-controlled valve:

            Water pressure (Vdd) ──→ [VALVE] ──→ Water out (Vss/ground)
                                       ↑
                                   Gate voltage

        - Gate voltage HIGH: valve opens, water flows (current flows D→S)
        - Gate voltage LOW:  valve closed, water blocked (no current)
        - Gate voltage MEDIUM: valve partially open (analog amplifier mode)

    === In a digital circuit ===

        When used as a digital switch, NMOS connects the output to GROUND:

            Output ──┤
                     │ NMOS (gate = input signal)
                     │
                    GND

        Input HIGH → NMOS ON → output pulled to GND (LOW)
        Input LOW  → NMOS OFF → output disconnected from GND

    === Parameters ===

        params: MOSFETParams defining the transistor's electrical characteristics.
                If None, defaults to a typical 180nm CMOS process.
    """

    def __init__(self, params: MOSFETParams | None = None) -> None:
        self.params = params or MOSFETParams()

    def region(self, vgs: float, vds: float) -> MOSFETRegion:
        """Determine the operating region given terminal voltages.

        The operating region determines which equations govern current flow.
        For NMOS:
            Cutoff:     Vgs < Vth            (gate voltage below threshold)
            Linear:     Vgs >= Vth AND Vds < Vgs - Vth
            Saturation: Vgs >= Vth AND Vds >= Vgs - Vth

        Args:
            vgs: Gate-to-Source voltage (V). Positive turns NMOS on.
            vds: Drain-to-Source voltage (V). Positive for normal operation.

        Returns:
            MOSFETRegion enum value.

        Example:
            >>> t = NMOS()
            >>> t.region(vgs=0.0, vds=1.0)  # gate below threshold
            MOSFETRegion.CUTOFF
            >>> t.region(vgs=1.5, vds=0.1)  # gate on, low drain voltage
            MOSFETRegion.LINEAR
            >>> t.region(vgs=1.0, vds=3.0)  # gate on, high drain voltage
            MOSFETRegion.SATURATION
        """
        vth = self.params.vth

        if vgs < vth:
            return MOSFETRegion.CUTOFF

        vov = vgs - vth  # Overdrive voltage
        if vds < vov:
            return MOSFETRegion.LINEAR
        return MOSFETRegion.SATURATION

    def drain_current(self, vgs: float, vds: float) -> float:
        """Calculate drain-to-source current (Ids) in amperes.

        Uses the simplified MOSFET current equations (Shockley model):

            Cutoff (Vgs < Vth):
                Ids = 0
                No channel exists, no current flows.

            Linear (Vgs >= Vth, Vds < Vgs - Vth):
                Ids = k * ((Vgs - Vth) * Vds - 0.5 * Vds^2)
                The transistor acts like a voltage-controlled resistor.
                Current increases with both Vgs and Vds.

            Saturation (Vgs >= Vth, Vds >= Vgs - Vth):
                Ids = 0.5 * k * (Vgs - Vth)^2
                The channel is "pinched off" at the drain end.
                Current depends only on Vgs, not Vds.
                This is why saturation is used for amplifiers — the
                output current is controlled solely by the input voltage.

        Args:
            vgs: Gate-to-Source voltage (V).
            vds: Drain-to-Source voltage (V). Must be >= 0 for NMOS.

        Returns:
            Drain current in amperes. Always >= 0 for NMOS.
        """
        region = self.region(vgs, vds)
        k = self.params.k
        vth = self.params.vth

        if region == MOSFETRegion.CUTOFF:
            return 0.0

        vov = vgs - vth  # Overdrive voltage

        if region == MOSFETRegion.LINEAR:
            # Linear/ohmic region: Ids = k * ((Vgs-Vth)*Vds - 0.5*Vds^2)
            return k * (vov * vds - 0.5 * vds * vds)

        # Saturation region: Ids = 0.5 * k * (Vgs-Vth)^2
        return 0.5 * k * vov * vov

    def is_conducting(self, vgs: float) -> bool:
        """Digital abstraction: is this transistor ON?

        Returns True when the gate voltage exceeds the threshold voltage.
        This is the simplified view used in digital circuit analysis —
        the transistor is either fully ON or fully OFF, with no in-between.

        In real circuits, the transition is gradual (the transistor passes
        through the linear region), but for digital logic we only care
        about the final steady state.

        Args:
            vgs: Gate-to-Source voltage (V).

        Returns:
            True if Vgs >= Vth (transistor is ON).
        """
        return vgs >= self.params.vth

    def output_voltage(self, vgs: float, vdd: float) -> float:
        """Output voltage when used as a pull-down switch.

        In a CMOS circuit, NMOS transistors form the pull-down network
        (connecting output to ground). When the NMOS is ON, it pulls
        the output to ~0V. When OFF, the output floats (determined by
        the pull-up network).

        For a simple NMOS switch with a resistive pull-up:
            ON:  output ≈ 0V (pulled to ground through low-resistance channel)
            OFF: output ≈ Vdd (pulled up by load resistor)

        Args:
            vgs: Gate-to-Source voltage (V).
            vdd: Supply voltage (V).

        Returns:
            Output voltage in volts.
        """
        if self.is_conducting(vgs):
            # ON: output pulled to ground. In reality there's a small
            # voltage drop across the channel (Vds_on), but for digital
            # logic we approximate it as 0V.
            return 0.0
        # OFF: output pulled to Vdd by the pull-up network.
        return vdd

    def transconductance(self, vgs: float, vds: float) -> float:
        """Calculate small-signal transconductance gm.

        Transconductance is the key parameter for amplifier design.
        It tells you how much the output current changes per unit
        change in input voltage:

            gm = dIds / dVgs

        In saturation:
            gm = k * (Vgs - Vth) = 2 * Ids / (Vgs - Vth)

        Higher gm = more gain, but also more power consumption.

        Args:
            vgs: Gate-to-Source voltage (V).
            vds: Drain-to-Source voltage (V).

        Returns:
            Transconductance in Siemens (A/V). Returns 0.0 in cutoff.
        """
        region = self.region(vgs, vds)
        if region == MOSFETRegion.CUTOFF:
            return 0.0

        vov = vgs - self.params.vth
        return self.params.k * vov


class PMOS:
    """P-channel MOSFET transistor.

    A PMOS transistor is the complement of NMOS. It conducts current from
    source to drain when the gate voltage is LOW (below the source voltage
    by more than |Vth|). Think of it as a normally-CLOSED switch that OPENS
    when you apply voltage.

    === Why PMOS matters ===

    PMOS transistors form the pull-UP network in CMOS gates. When we need
    to connect the output to Vdd (logic HIGH), PMOS transistors do the job:

        Vdd
         │
         │ PMOS (gate = input signal)
         ┤
         │
        Output

        Input LOW  → PMOS ON → output pulled to Vdd (HIGH)
        Input HIGH → PMOS OFF → output disconnected from Vdd

    === NMOS vs PMOS symmetry ===

    PMOS uses the same equations as NMOS, but with reversed voltage
    polarities. For PMOS, Vgs and Vds are typically negative (because
    the source is connected to Vdd, the highest voltage in the circuit).

    In this implementation, we handle the sign conventions internally.
    The `is_conducting` method takes Vgs as a negative value for PMOS:
    Vgs = Vgate - Vsource, where Vsource = Vdd, so Vgs is negative
    when the gate is pulled below Vdd.

    PMOS transistors are typically 2-3x wider than their NMOS counterparts
    to compensate for the lower mobility of holes vs electrons. This is
    why real chip layouts show wider PMOS transistors in the pull-up network.
    """

    def __init__(self, params: MOSFETParams | None = None) -> None:
        self.params = params or MOSFETParams()

    def region(self, vgs: float, vds: float) -> MOSFETRegion:
        """Determine operating region for PMOS.

        For PMOS, we use the magnitudes of Vgs and Vds (which are typically
        negative in a circuit). The regions are:

            Cutoff:     |Vgs| < Vth  (equivalently, Vgs > -Vth)
            Linear:     |Vgs| >= Vth AND |Vds| < |Vgs| - Vth
            Saturation: |Vgs| >= Vth AND |Vds| >= |Vgs| - Vth

        Args:
            vgs: Gate-to-Source voltage (V). Typically negative for PMOS.
            vds: Drain-to-Source voltage (V). Typically negative for PMOS.
        """
        vth = self.params.vth
        abs_vgs = abs(vgs)
        abs_vds = abs(vds)

        if abs_vgs < vth:
            return MOSFETRegion.CUTOFF

        vov = abs_vgs - vth
        if abs_vds < vov:
            return MOSFETRegion.LINEAR
        return MOSFETRegion.SATURATION

    def drain_current(self, vgs: float, vds: float) -> float:
        """Calculate source-to-drain current for PMOS.

        Same equations as NMOS but using absolute values of voltages.
        Current magnitude is returned (always >= 0).
        """
        region = self.region(vgs, vds)
        k = self.params.k
        vth = self.params.vth

        if region == MOSFETRegion.CUTOFF:
            return 0.0

        abs_vgs = abs(vgs)
        abs_vds = abs(vds)
        vov = abs_vgs - vth

        if region == MOSFETRegion.LINEAR:
            return k * (vov * abs_vds - 0.5 * abs_vds * abs_vds)

        return 0.5 * k * vov * vov

    def is_conducting(self, vgs: float) -> bool:
        """Digital abstraction: is this PMOS transistor ON?

        PMOS turns ON when Vgs is sufficiently negative (gate pulled
        below the source). Returns True when |Vgs| >= Vth.

        Remember: for PMOS in a CMOS circuit, Source is connected to Vdd.
        So Vgs = Vgate - Vdd. When the gate is at 0V and Vdd = 3.3V,
        Vgs = 0 - 3.3 = -3.3V, which is well below -Vth, so PMOS is ON.

        Args:
            vgs: Gate-to-Source voltage (V). Typically negative for PMOS.
        """
        return abs(vgs) >= self.params.vth

    def output_voltage(self, vgs: float, vdd: float) -> float:
        """Output voltage when used as a pull-up switch.

        PMOS forms the pull-up network in CMOS:
            ON:  output ≈ Vdd (pulled to supply through low-resistance channel)
            OFF: output ≈ 0V (pulled down by NMOS network)

        Args:
            vgs: Gate-to-Source voltage (V).
            vdd: Supply voltage (V).
        """
        if self.is_conducting(vgs):
            return vdd
        return 0.0

    def transconductance(self, vgs: float, vds: float) -> float:
        """Calculate small-signal transconductance gm for PMOS.

        Same formula as NMOS but using absolute values.
        """
        region = self.region(vgs, vds)
        if region == MOSFETRegion.CUTOFF:
            return 0.0

        vov = abs(vgs) - self.params.vth
        return self.params.k * vov

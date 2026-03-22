"""BJT Transistors — the original solid-state amplifier.

=== What is a BJT? ===

BJT stands for Bipolar Junction Transistor. Invented in 1947 at Bell Labs
by John Bardeen, Walter Brattain, and William Shockley, the BJT replaced
vacuum tubes and launched the electronics revolution. Before the BJT,
computers filled entire rooms with thousands of hot, unreliable vacuum tubes.
After the BJT, they could be shrunk to a desk — and eventually to a pocket.

A BJT has three terminals:
    Base (B):      The control terminal. Current here controls the switch.
    Collector (C): Current flows IN here (for NPN) or OUT here (for PNP).
    Emitter (E):   Current flows OUT here (for NPN) or IN here (for PNP).

The key difference from MOSFETs: a BJT is CURRENT-controlled. You must
supply a continuous current to the base to keep it on. This means:
    - Base current = wasted power (even in steady state)
    - Lower input impedance than MOSFETs
    - But historically faster switching (before CMOS caught up)

=== NPN vs PNP ===

    NPN: Base current flows B→E. Collector current flows C→E.
         "Current flows IN to the base to turn it ON."

    PNP: Base current flows E→B. Collector current flows E→C.
         "Current flows OUT of the base to turn it ON."

=== The Current Gain (beta) ===

The magic of the BJT is current amplification:

    Ic = beta * Ib

A tiny base current (microamps) controls a much larger collector current
(milliamps). Beta (β, also called hfe) is typically 50-300 for small-signal
transistors. This amplification property made radios, televisions, and
early computers possible.

=== Why CMOS Replaced BJT for Digital Logic ===

In TTL (Transistor-Transistor Logic), the dominant BJT logic family:
    - Static power: ~1-10 mW per gate (base current always flows)
    - A chip with 1 million gates would consume 1-10 kW just sitting idle!

In CMOS:
    - Static power: ~nanowatts per gate (no DC current path)
    - A chip with 1 billion gates consumes milliwatts in idle

This power advantage is why CMOS completely replaced BJT for digital logic
by the 1990s. BJTs are still used in analog circuits, power electronics,
and RF amplifiers where their higher transconductance matters.
"""

from __future__ import annotations

import math

from transistors.types import BJTParams, BJTRegion


class NPN:
    """NPN bipolar junction transistor.

    An NPN transistor turns ON when current flows into the base terminal
    (Vbe > ~0.7V). A small base current controls a much larger collector
    current through the current gain relationship: Ic = beta * Ib.

    === Water analogy ===

        A small stream (base current) controls the gate on a dam:

            Large reservoir ──→ [DAM/GATE] ──→ River downstream
            (collector)            ↑            (emitter)
                              Small stream
                              (base current)

        - Small stream flowing: gate opens, river flows (Ic = beta * Ib)
        - Small stream dry:    gate closed, no river flow (Ic = 0)
        - Larger stream:       gate opens wider (more Ic)

    === Operating regions ===

        CUTOFF:      Vbe < 0.7V → no base current → no collector current.
                     The transistor is OFF. Used as logic 1 in TTL
                     (output pulled HIGH through a resistor).

        ACTIVE:      Vbe ≈ 0.7V, Vce > 0.2V → Ic = beta * Ib.
                     The transistor is a LINEAR AMPLIFIER. This is where
                     audio amplifiers, radio receivers, and analog circuits
                     operate.

        SATURATION:  Vbe ≈ 0.7V, Vce ≈ 0.2V → transistor is fully ON.
                     Collector current is limited by the external circuit,
                     not by beta * Ib. Used as logic 0 in TTL
                     (output pulled LOW through the saturated transistor).
    """

    def __init__(self, params: BJTParams | None = None) -> None:
        self.params = params or BJTParams()

    def region(self, vbe: float, vce: float) -> BJTRegion:
        """Determine the operating region from terminal voltages.

        Args:
            vbe: Base-to-Emitter voltage (V). Must exceed ~0.7V to turn on.
            vce: Collector-to-Emitter voltage (V). Determines active vs saturated.

        Returns:
            BJTRegion enum value.

        Example:
            >>> t = NPN()
            >>> t.region(vbe=0.0, vce=5.0)   # base voltage too low
            BJTRegion.CUTOFF
            >>> t.region(vbe=0.7, vce=3.0)   # normal amplifier operation
            BJTRegion.ACTIVE
            >>> t.region(vbe=0.7, vce=0.1)   # fully saturated (switch ON)
            BJTRegion.SATURATION
        """
        if vbe < self.params.vbe_on:
            return BJTRegion.CUTOFF

        if vce <= self.params.vce_sat:
            return BJTRegion.SATURATION

        return BJTRegion.ACTIVE

    def collector_current(self, vbe: float, vce: float) -> float:
        """Calculate collector current (Ic) in amperes.

        The collector current depends on the operating region:

            Cutoff:
                Ic = 0. No base current, no collector current.

            Active:
                Ic = beta * Ib, where Ib is derived from the Ebers-Moll
                model: Ib = Is * (exp(Vbe / Vt) - 1) / beta.
                Simplified: Ic = Is * (exp(Vbe / Vt) - 1)
                where Vt = kT/q ≈ 26mV at room temperature.

            Saturation:
                Ic is limited by the external circuit. We return a high
                current value representing the transistor as a closed switch
                with Vce_sat across it.

        Args:
            vbe: Base-to-Emitter voltage (V).
            vce: Collector-to-Emitter voltage (V).

        Returns:
            Collector current in amperes.
        """
        region = self.region(vbe, vce)

        if region == BJTRegion.CUTOFF:
            return 0.0

        # Thermal voltage: Vt = kT/q ≈ 26mV at room temperature
        vt = 0.026

        if region == BJTRegion.ACTIVE:
            # Ebers-Moll model (simplified):
            # Ic = Is * (exp(Vbe/Vt) - 1)
            #
            # The exponential relationship is why BJTs are such good
            # amplifiers — a small change in Vbe causes a large change in Ic.
            exponent = min(vbe / vt, 40.0)  # Clamp to prevent overflow
            return self.params.is_ * (math.exp(exponent) - 1.0)

        # Saturation: transistor is fully ON.
        # In real circuits, Ic_sat = (Vcc - Vce_sat) / Rc, which depends
        # on the external circuit. We return the current that the transistor
        # CAN supply in its fully-on state using the active equation at
        # the edge of saturation.
        exponent = min(vbe / vt, 40.0)
        return self.params.is_ * (math.exp(exponent) - 1.0)

    def base_current(self, vbe: float, vce: float) -> float:
        """Calculate base current (Ib) in amperes.

        Ib = Ic / beta in the active region.

        This is the "wasted" current that makes BJTs less efficient than
        MOSFETs for digital logic. Every TTL gate has base current flowing
        continuously, which adds up to significant power consumption when
        you have millions of gates.

        Args:
            vbe: Base-to-Emitter voltage (V).
            vce: Collector-to-Emitter voltage (V).

        Returns:
            Base current in amperes.
        """
        ic = self.collector_current(vbe, vce)
        if ic == 0.0:
            return 0.0
        return ic / self.params.beta

    def is_conducting(self, vbe: float) -> bool:
        """Digital abstraction: is this transistor ON?

        Returns True when Vbe >= Vbe_on (typically 0.7V).
        """
        return vbe >= self.params.vbe_on

    def transconductance(self, vbe: float, vce: float) -> float:
        """Calculate small-signal transconductance gm.

        For a BJT in the active region:
            gm = Ic / Vt

        BJTs typically have higher gm than MOSFETs for the same current,
        which is why they're still preferred for some analog applications.

        Args:
            vbe: Base-to-Emitter voltage (V).
            vce: Collector-to-Emitter voltage (V).

        Returns:
            Transconductance in Siemens (A/V).
        """
        ic = self.collector_current(vbe, vce)
        if ic == 0.0:
            return 0.0
        vt = 0.026
        return ic / vt


class PNP:
    """PNP bipolar junction transistor.

    The complement of NPN. A PNP transistor turns ON when the base is
    pulled LOW relative to the emitter (Veb > 0.7V, equivalently
    Vbe < -0.7V in our convention). Current flows from emitter to collector.

    === When to use PNP ===

    In analog circuits, PNP transistors are used for:
    - Push-pull output stages (paired with NPN)
    - Current mirrors
    - Level shifting

    In TTL digital logic, PNP was used in the multi-emitter input transistor
    of the 7400-series NAND gate. The multi-emitter structure is unique to
    BJTs and has no MOSFET equivalent.

    === Voltage conventions ===

    For PNP, the "natural" voltages are reversed from NPN:
    - Vbe is typically NEGATIVE (base below emitter)
    - Vce is typically NEGATIVE (collector below emitter)

    We use absolute values internally, same as PMOS.
    """

    def __init__(self, params: BJTParams | None = None) -> None:
        self.params = params or BJTParams()

    def region(self, vbe: float, vce: float) -> BJTRegion:
        """Determine operating region for PNP.

        Uses absolute values of Vbe and Vce since PNP operates with
        reversed polarities.
        """
        abs_vbe = abs(vbe)
        abs_vce = abs(vce)

        if abs_vbe < self.params.vbe_on:
            return BJTRegion.CUTOFF

        if abs_vce <= self.params.vce_sat:
            return BJTRegion.SATURATION

        return BJTRegion.ACTIVE

    def collector_current(self, vbe: float, vce: float) -> float:
        """Calculate collector current magnitude for PNP.

        Same equations as NPN but using absolute values.
        Returns current magnitude (always >= 0).
        """
        region = self.region(vbe, vce)

        if region == BJTRegion.CUTOFF:
            return 0.0

        abs_vbe = abs(vbe)
        vt = 0.026

        exponent = min(abs_vbe / vt, 40.0)
        return self.params.is_ * (math.exp(exponent) - 1.0)

    def base_current(self, vbe: float, vce: float) -> float:
        """Calculate base current magnitude for PNP."""
        ic = self.collector_current(vbe, vce)
        if ic == 0.0:
            return 0.0
        return ic / self.params.beta

    def is_conducting(self, vbe: float) -> bool:
        """Digital abstraction: is this PNP transistor ON?

        PNP turns ON when |Vbe| >= Vbe_on (base pulled below emitter).
        """
        return abs(vbe) >= self.params.vbe_on

    def transconductance(self, vbe: float, vce: float) -> float:
        """Calculate small-signal transconductance gm for PNP."""
        ic = self.collector_current(vbe, vce)
        if ic == 0.0:
            return 0.0
        vt = 0.026
        return ic / vt

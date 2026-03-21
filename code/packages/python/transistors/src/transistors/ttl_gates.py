"""TTL Logic Gates — historical BJT-based digital logic.

=== What is TTL? ===

TTL stands for Transistor-Transistor Logic. It was the dominant digital
logic family from the mid-1960s through the 1980s, when CMOS replaced it.
The "7400 series" — a family of TTL chips — defined the standard logic
gates that every digital system used.

The 7400 itself was a quad 2-input NAND gate: four NAND gates in a single
14-pin chip. It was introduced by Texas Instruments in 1966 and became one
of the most successful integrated circuits ever made.

=== Why TTL Lost to CMOS ===

TTL's fatal flaw: STATIC POWER CONSUMPTION.

In a TTL gate, current flows through resistors and transistors even when
the gate is doing nothing. A single TTL NAND gate dissipates ~1-10 mW
at rest. That may sound small, but:

    1 million gates × 10 mW/gate = 10,000 watts

That's a space heater, not a computer chip! This is why TTL chips maxed
out at a few thousand gates per chip.

CMOS gates consume near-zero power at rest (only transistor leakage).
This allowed chips to scale to billions of gates — the modern CPU.

=== RTL: The Predecessor to TTL ===

Before TTL came RTL (Resistor-Transistor Logic), the simplest possible
transistor logic. An RTL inverter is just one transistor with two resistors.
It was slow and power-hungry, but it was used in the Apollo Guidance
Computer that landed humans on the moon in 1969.
"""

from __future__ import annotations

import math

from transistors.bjt import NPN
from transistors.types import BJTParams, GateOutput


def _validate_bit(value: int, name: str = "input") -> None:
    """Validate binary digit."""
    if not isinstance(value, int) or isinstance(value, bool):
        msg = f"{name} must be an int, got {type(value).__name__}"
        raise TypeError(msg)
    if value not in (0, 1):
        msg = f"{name} must be 0 or 1, got {value}"
        raise ValueError(msg)


class TTLNand:
    """TTL NAND gate using NPN transistors (7400-series style).

    === Simplified Circuit ===

            Vcc (+5V)
             │
             R1 (4kΩ)
             │
        ┌────┴────┐
        │  Q1     │     Multi-emitter input transistor
        │  (NPN)  │
        ├── E1 ───┤── Input A
        ├── E2 ───┤── Input B
        └────┬────┘
             │
        ┌────┴────┐
        │  Q2     │     Phase splitter
        │  (NPN)  │
        └────┬────┘
             │
        ┌────┴────┐
        │  Q3     │     Output transistor
        │  (NPN)  │
        └────┬────┘
             │
            GND

    === Operation ===

    Any input LOW:
        Q1's base-emitter junction forward-biases through the LOW input,
        stealing current from Q2's base → Q2 and Q3 turn OFF →
        output pulled HIGH through pull-up resistor.

    ALL inputs HIGH:
        Q1's base-collector junction forward-biases, driving current
        into Q2's base → Q2 saturates → Q3 saturates →
        output pulled LOW (Vce_sat ≈ 0.2V).

    Result: NAND function.

    === The Problem: Static Power ===

    When Q3 is ON: current flows Vcc → R1 → Q1 → Q2 → Q3 → GND.
    This current flows CONTINUOUSLY, consuming ~1-10 mW per gate.
    This is the fundamental reason CMOS replaced TTL.
    """

    def __init__(
        self,
        vcc: float = 5.0,
        bjt_params: BJTParams | None = None,
    ) -> None:
        self.vcc = vcc
        self.params = bjt_params or BJTParams()
        # We model the simplified TTL with resistor values
        self.r_pullup = 4000.0  # 4kΩ pull-up resistor
        self.q1 = NPN(self.params)
        self.q2 = NPN(self.params)
        self.q3 = NPN(self.params)

    def evaluate(self, va: float, vb: float) -> GateOutput:
        """Evaluate the TTL NAND gate with analog input voltages.

        Args:
            va: Input A voltage (V). LOW < 0.8V, HIGH > 2.0V.
            vb: Input B voltage (V).

        Returns:
            GateOutput with voltage and power details.
        """
        vcc = self.vcc
        vbe_on = self.params.vbe_on

        # TTL input thresholds
        a_high = va > 2.0
        b_high = vb > 2.0

        if a_high and b_high:
            # ALL inputs HIGH → output LOW
            # Q1 collector drives Q2 base, Q2 drives Q3, Q3 saturates
            output_v = self.params.vce_sat  # ~0.2V
            logic_value = 0

            # Static current: Vcc through resistor chain
            # I ≈ (Vcc - Vbe_Q2 - Vbe_Q3 - Vce_sat_Q3) / R_pullup
            current = (vcc - 2 * vbe_on - self.params.vce_sat) / self.r_pullup
            current = max(current, 0.0)
        else:
            # At least one input LOW → output HIGH
            output_v = vcc - vbe_on  # ~4.3V (Vcc minus one diode drop)
            logic_value = 1

            # Less current flows when output is HIGH
            # Small bias current through pull-up
            current = (vcc - output_v) / self.r_pullup
            current = max(current, 0.0)

        power = current * vcc

        # TTL propagation delay: typically 5-15 ns
        delay = 10e-9  # 10 ns typical

        return GateOutput(
            logic_value=logic_value,
            voltage=output_v,
            current_draw=current,
            power_dissipation=power,
            propagation_delay=delay,
            transistor_count=3,  # Simplified: Q1 + Q2 + Q3
        )

    def evaluate_digital(self, a: int, b: int) -> int:
        """Evaluate with digital inputs (0 or 1)."""
        _validate_bit(a, "a")
        _validate_bit(b, "b")
        va = self.vcc if a == 1 else 0.0
        vb = self.vcc if b == 1 else 0.0
        return self.evaluate(va, vb).logic_value

    @property
    def static_power(self) -> float:
        """Static power dissipation — significantly higher than CMOS.

        TTL gates consume power continuously due to the resistor-based
        biasing. The worst case is when the output is LOW (all inputs HIGH),
        because maximum current flows through the resistor chain.

        Returns:
            Static power in watts. Typically ~1-10 mW for a single gate.
        """
        # Worst case: output LOW, all inputs HIGH
        current = (self.vcc - 2 * self.params.vbe_on - self.params.vce_sat) / self.r_pullup
        return max(current, 0.0) * self.vcc


class RTLInverter:
    """Resistor-Transistor Logic inverter — the earliest IC logic family.

    === Circuit Diagram ===

            Vcc
             │
            Rc (collector resistor, ~1kΩ)
             │
        ┌────┴────┐
        │  Q1     │     Single NPN transistor
        │  (NPN)  │
        └────┬────┘
             │
            GND

        Input ──── Rb (base resistor, ~10kΩ) ──── Base of Q1

    === Operation ===

    Input HIGH (Vcc):
        Current flows through Rb into Q1's base → Q1 saturates →
        output pulled LOW through Q1 (Vce_sat ≈ 0.2V).

    Input LOW (0V):
        No base current → Q1 in cutoff → output pulled HIGH
        through Rc to Vcc.

    === Historical Note ===

    RTL was used in the Apollo Guidance Computer (AGC), which navigated
    Apollo 11 to the moon in 1969. The AGC contained about 5,600 NOR
    gates built from RTL circuits, and had a clock speed of 2 MHz.
    Your phone has about a million times more gates and runs a thousand
    times faster.
    """

    def __init__(
        self,
        vcc: float = 5.0,
        r_base: float = 10_000.0,
        r_collector: float = 1_000.0,
        bjt_params: BJTParams | None = None,
    ) -> None:
        self.vcc = vcc
        self.r_base = r_base
        self.r_collector = r_collector
        self.params = bjt_params or BJTParams()
        self.q1 = NPN(self.params)

    def evaluate(self, v_input: float) -> GateOutput:
        """Evaluate the RTL inverter with an analog input voltage."""
        vcc = self.vcc
        vbe_on = self.params.vbe_on

        # Calculate base current
        if v_input > vbe_on:
            ib = (v_input - vbe_on) / self.r_base
            # Q1 is ON — check if saturated
            ic = min(ib * self.params.beta, (vcc - self.params.vce_sat) / self.r_collector)
            output_v = vcc - ic * self.r_collector
            output_v = max(output_v, self.params.vce_sat)
            logic_value = 0 if output_v < vcc / 2.0 else 1
            current = ic + ib
        else:
            # Q1 is OFF — output pulled to Vcc through Rc
            output_v = vcc
            logic_value = 1
            current = 0.0  # No current flows when OFF (approximately)

        power = current * vcc
        delay = 50e-9  # RTL is slow: ~50 ns typical

        return GateOutput(
            logic_value=logic_value,
            voltage=output_v,
            current_draw=current,
            power_dissipation=power,
            propagation_delay=delay,
            transistor_count=1,
        )

    def evaluate_digital(self, a: int) -> int:
        """Evaluate with digital input (0 or 1)."""
        _validate_bit(a, "a")
        v_input = self.vcc if a == 1 else 0.0
        return self.evaluate(v_input).logic_value

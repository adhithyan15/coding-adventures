"""CMOS Logic Gates — building digital logic from transistor pairs.

=== What is CMOS? ===

CMOS stands for Complementary Metal-Oxide-Semiconductor. It is the
technology used in virtually every digital chip made since the 1980s:
CPUs, GPUs, memory, phone processors — all CMOS.

The "complementary" refers to pairing NMOS and PMOS transistors:
    - PMOS transistors form the PULL-UP network (connects output to Vdd)
    - NMOS transistors form the PULL-DOWN network (connects output to GND)

For any valid input combination, exactly ONE network is active:
    - If pull-up is ON → output = Vdd (logic HIGH)
    - If pull-down is ON → output = GND (logic LOW)
    - Never both ON simultaneously → no DC current path → near-zero static power

This last property is CMOS's killer feature. A chip with 10 billion
transistors consumes essentially zero power when not switching. Only
the gates that are actively changing state burn energy. This is why
your phone battery lasts hours instead of minutes.

=== The CMOS Design Rules ===

Rule 1: Pull-up network uses ONLY PMOS transistors.
Rule 2: Pull-down network uses ONLY NMOS transistors.
Rule 3: Pull-up and pull-down are COMPLEMENTARY:
    - Where pull-down has transistors in SERIES, pull-up has them in PARALLEL.
    - Where pull-down has transistors in PARALLEL, pull-up has them in SERIES.

These rules guarantee that for every input, exactly one network conducts.

=== Transistor Counts ===

    Gate    | NMOS | PMOS | Total | Notes
    --------|------|------|-------|------
    NOT     |  1   |  1   |   2   | The simplest CMOS circuit
    NAND    |  2   |  2   |   4   | Natural CMOS gate
    NOR     |  2   |  2   |   4   | Natural CMOS gate
    AND     |  3   |  3   |   6   | NAND + NOT
    OR      |  3   |  3   |   6   | NOR + NOT
    XOR     |  3   |  3   |   6   | Transmission gate design
"""

from __future__ import annotations

from transistors.mosfet import NMOS, PMOS
from transistors.types import CircuitParams, GateOutput, MOSFETParams


def _validate_bit(value: int, name: str = "input") -> None:
    """Validate that a value is a binary digit (0 or 1).

    We reuse the same strict validation as the logic_gates package:
    reject booleans, floats, and out-of-range integers.
    """
    if not isinstance(value, int) or isinstance(value, bool):
        msg = f"{name} must be an int, got {type(value).__name__}"
        raise TypeError(msg)
    if value not in (0, 1):
        msg = f"{name} must be 0 or 1, got {value}"
        raise ValueError(msg)


class CMOSInverter:
    """CMOS NOT gate: 1 PMOS + 1 NMOS = 2 transistors.

    The simplest and most important CMOS circuit. Every other CMOS gate
    is a variation of this fundamental pattern.

    === Circuit Diagram ===

             Vdd
              │
         ┌────┴────┐
         │  PMOS   │
         │         │─── Gate ──── Input (A)
         └────┬────┘
              │
              ├──────────────── Output (Y = NOT A)
              │
         ┌────┴────┐
         │  NMOS   │
         │         │─── Gate ──── Input (A)
         └────┬────┘
              │
             GND

    === How it works ===

    Input A = HIGH (Vdd):
        NMOS: Vgs = Vdd > Vth → ON → pulls output to GND
        PMOS: Vgs = 0 → OFF → disconnected from Vdd
        Output = LOW (GND) = NOT HIGH ✓

    Input A = LOW (0V):
        NMOS: Vgs = 0 < Vth → OFF → disconnected from GND
        PMOS: Vgs = -Vdd → ON → pulls output to Vdd
        Output = HIGH (Vdd) = NOT LOW ✓

    Static power: ZERO. In both states, one transistor is OFF, breaking
    the current path from Vdd to GND. The only power consumed is during
    the brief switching transition.
    """

    TRANSISTOR_COUNT = 2

    def __init__(
        self,
        circuit_params: CircuitParams | None = None,
        nmos_params: MOSFETParams | None = None,
        pmos_params: MOSFETParams | None = None,
    ) -> None:
        self.circuit = circuit_params or CircuitParams()
        self.nmos = NMOS(nmos_params)
        self.pmos = PMOS(pmos_params)

    def evaluate(self, input_voltage: float) -> GateOutput:
        """Evaluate the inverter with an analog input voltage.

        Maps the input voltage through the CMOS transfer characteristic
        to produce an output voltage. This is the "real" electrical
        simulation — not just 0/1 logic but actual voltage levels.

        The transfer characteristic (VTC) has a sharp transition:
        - Input near 0V → output near Vdd
        - Input near Vdd → output near 0V
        - Input near Vdd/2 → both transistors partially on (transition region)

        Args:
            input_voltage: Input voltage in volts (0 to Vdd).

        Returns:
            GateOutput with voltage, current, power, and timing details.
        """
        vdd = self.circuit.vdd

        # NMOS: gate = input, source = GND
        # Vgs_n = Vin - 0 = Vin
        vgs_n = input_voltage

        # PMOS: gate = input, source = Vdd
        # Vgs_p = Vin - Vdd (negative when input is LOW)
        vgs_p = input_voltage - vdd

        nmos_on = self.nmos.is_conducting(vgs_n)
        pmos_on = self.pmos.is_conducting(vgs_p)

        # Determine output voltage
        if pmos_on and not nmos_on:
            output_v = vdd  # PMOS pulls to Vdd
        elif nmos_on and not pmos_on:
            output_v = 0.0  # NMOS pulls to GND
        elif nmos_on and pmos_on:
            # Both on (transition region) — voltage divider
            # Approximate as Vdd/2
            output_v = vdd / 2.0
        else:
            # Both off (shouldn't happen in normal operation)
            output_v = vdd / 2.0

        # Digital interpretation
        logic_value = 1 if output_v > vdd / 2.0 else 0

        # Current draw: only significant during transition
        if nmos_on and pmos_on:
            # Short-circuit current during transition
            vds_n = vdd / 2.0
            current = self.nmos.drain_current(vgs_n, vds_n)
        else:
            current = 0.0  # Static: no current path

        power = current * vdd

        # Propagation delay estimate
        c_load = self.nmos.params.c_drain + self.pmos.params.c_drain
        if current > 0:
            delay = c_load * vdd / (2.0 * current)
        else:
            # Approximate delay using saturation current
            ids_sat = self.nmos.drain_current(vdd, vdd)
            delay = c_load * vdd / (2.0 * ids_sat) if ids_sat > 0 else 1e-9

        return GateOutput(
            logic_value=logic_value,
            voltage=output_v,
            current_draw=current,
            power_dissipation=power,
            propagation_delay=delay,
            transistor_count=self.TRANSISTOR_COUNT,
        )

    def evaluate_digital(self, a: int) -> int:
        """Evaluate with digital input (0 or 1), returns 0 or 1.

        Convenience method that maps 0 → 0V, 1 → Vdd.
        """
        _validate_bit(a, "a")
        vin = self.circuit.vdd if a == 1 else 0.0
        return self.evaluate(vin).logic_value

    def voltage_transfer_characteristic(
        self, steps: int = 100
    ) -> list[tuple[float, float]]:
        """Generate the VTC curve: list of (Vin, Vout) points.

        The VTC shows the sharp switching threshold of CMOS — the output
        snaps from HIGH to LOW over a very narrow input range. This sharp
        transition is what makes CMOS excellent for digital logic:
        small noise on the input doesn't change the output.
        """
        vdd = self.circuit.vdd
        points: list[tuple[float, float]] = []
        for i in range(steps + 1):
            vin = vdd * i / steps
            result = self.evaluate(vin)
            points.append((vin, result.voltage))
        return points

    @property
    def static_power(self) -> float:
        """Static power dissipation (ideally ~0 for CMOS).

        In an ideal CMOS inverter, one transistor is always OFF, so no
        DC current flows from Vdd to GND. Static power comes only from
        transistor leakage current, which is negligible for our model.
        """
        return 0.0

    def dynamic_power(self, frequency: float, c_load: float) -> float:
        """Dynamic power: P = C_load * Vdd^2 * f.

        This is the dominant power consumption mechanism in CMOS:
        every time the output switches, the load capacitance must be
        charged (0→1) or discharged (1→0). The energy per transition
        is C * Vdd^2, and if we switch f times per second, the power
        is C * Vdd^2 * f.

        This formula explains:
        - Why reducing Vdd is so effective (power ∝ V^2)
        - Why clock frequency matters (power ∝ f)
        - Why smaller transistors are better (smaller C)

        Args:
            frequency: Switching frequency in Hz.
            c_load: Load capacitance in Farads.

        Returns:
            Dynamic power in Watts.
        """
        vdd = self.circuit.vdd
        return c_load * vdd * vdd * frequency


class CMOSNand:
    """CMOS NAND gate: 2 PMOS parallel + 2 NMOS series = 4 transistors.

    === Circuit Diagram ===

              Vdd
          ┌────┴────┐
     ┌────┤  PMOS1  ├────┐     Pull-up: PMOS in PARALLEL
     │    └────┬────┘    │     Either PMOS ON → output HIGH
     │    Gate=A         │
     │         │    ┌────┴────┐
     │         │    │  PMOS2  │
     │         │    └────┬────┘
     │         │    Gate=B
     └─────────┴────┬────┘
                    │
                 Output
                    │
              ┌─────┴─────┐
              │   NMOS1   │    Pull-down: NMOS in SERIES
              └─────┬─────┘    BOTH NMOS ON → output LOW
              Gate=A
              ┌─────┴─────┐
              │   NMOS2   │
              └─────┬─────┘
              Gate=B
                    │
                   GND

    === Why NAND is the natural CMOS gate ===

    NAND requires only 4 transistors (2 PMOS + 2 NMOS). AND requires 6
    (NAND + inverter). This is because the CMOS structure naturally
    produces an inverted output. The pull-down network computes the
    function, and the pull-up network produces its complement.

    In professional chip design, circuits are built from NAND and NOR
    gates rather than AND and OR, precisely because of this efficiency.
    """

    TRANSISTOR_COUNT = 4

    def __init__(
        self,
        circuit_params: CircuitParams | None = None,
        nmos_params: MOSFETParams | None = None,
        pmos_params: MOSFETParams | None = None,
    ) -> None:
        self.circuit = circuit_params or CircuitParams()
        self.nmos1 = NMOS(nmos_params)
        self.nmos2 = NMOS(nmos_params)
        self.pmos1 = PMOS(pmos_params)
        self.pmos2 = PMOS(pmos_params)

    def evaluate(self, va: float, vb: float) -> GateOutput:
        """Evaluate the NAND gate with analog input voltages."""
        vdd = self.circuit.vdd

        # NMOS gates connect to inputs, sources to GND (through series chain)
        vgs_n1 = va
        vgs_n2 = vb

        # PMOS gates connect to inputs, sources to Vdd
        vgs_p1 = va - vdd
        vgs_p2 = vb - vdd

        nmos1_on = self.nmos1.is_conducting(vgs_n1)
        nmos2_on = self.nmos2.is_conducting(vgs_n2)
        pmos1_on = self.pmos1.is_conducting(vgs_p1)
        pmos2_on = self.pmos2.is_conducting(vgs_p2)

        # Pull-down: NMOS in SERIES — BOTH must be ON
        pulldown_on = nmos1_on and nmos2_on
        # Pull-up: PMOS in PARALLEL — EITHER can pull up
        pullup_on = pmos1_on or pmos2_on

        if pullup_on and not pulldown_on:
            output_v = vdd
        elif pulldown_on and not pullup_on:
            output_v = 0.0
        else:
            output_v = vdd / 2.0

        logic_value = 1 if output_v > vdd / 2.0 else 0
        current = 0.0 if not (pulldown_on and pullup_on) else 0.001

        c_load = self.nmos1.params.c_drain + self.pmos1.params.c_drain
        ids_sat = self.nmos1.drain_current(vdd, vdd)
        delay = c_load * vdd / (2.0 * ids_sat) if ids_sat > 0 else 1e-9

        return GateOutput(
            logic_value=logic_value,
            voltage=output_v,
            current_draw=current,
            power_dissipation=current * vdd,
            propagation_delay=delay,
            transistor_count=self.TRANSISTOR_COUNT,
        )

    def evaluate_digital(self, a: int, b: int) -> int:
        """Evaluate with digital inputs (0 or 1)."""
        _validate_bit(a, "a")
        _validate_bit(b, "b")
        vdd = self.circuit.vdd
        va = vdd if a == 1 else 0.0
        vb = vdd if b == 1 else 0.0
        return self.evaluate(va, vb).logic_value

    @property
    def transistor_count(self) -> int:
        """Returns 4."""
        return self.TRANSISTOR_COUNT


class CMOSNor:
    """CMOS NOR gate: 2 PMOS series + 2 NMOS parallel = 4 transistors.

    === Circuit Diagram ===

              Vdd
              │
         ┌────┴────┐
         │  PMOS1  │     Pull-up: PMOS in SERIES
         └────┬────┘     BOTH must be ON (both inputs LOW)
         Gate=A
         ┌────┴────┐
         │  PMOS2  │
         └────┬────┘
         Gate=B
              │
           Output
              │
     ┌────────┴────────┐
     │  NMOS1  │  NMOS2 │     Pull-down: NMOS in PARALLEL
     │ Gate=A  │ Gate=B  │     EITHER ON → output LOW
     └────────┬────────┘
              │
             GND
    """

    TRANSISTOR_COUNT = 4

    def __init__(
        self,
        circuit_params: CircuitParams | None = None,
        nmos_params: MOSFETParams | None = None,
        pmos_params: MOSFETParams | None = None,
    ) -> None:
        self.circuit = circuit_params or CircuitParams()
        self.nmos1 = NMOS(nmos_params)
        self.nmos2 = NMOS(nmos_params)
        self.pmos1 = PMOS(pmos_params)
        self.pmos2 = PMOS(pmos_params)

    def evaluate(self, va: float, vb: float) -> GateOutput:
        """Evaluate the NOR gate with analog input voltages."""
        vdd = self.circuit.vdd

        vgs_n1 = va
        vgs_n2 = vb
        vgs_p1 = va - vdd
        vgs_p2 = vb - vdd

        nmos1_on = self.nmos1.is_conducting(vgs_n1)
        nmos2_on = self.nmos2.is_conducting(vgs_n2)
        pmos1_on = self.pmos1.is_conducting(vgs_p1)
        pmos2_on = self.pmos2.is_conducting(vgs_p2)

        # Pull-down: NMOS in PARALLEL — EITHER ON pulls low
        pulldown_on = nmos1_on or nmos2_on
        # Pull-up: PMOS in SERIES — BOTH must be ON
        pullup_on = pmos1_on and pmos2_on

        if pullup_on and not pulldown_on:
            output_v = vdd
        elif pulldown_on and not pullup_on:
            output_v = 0.0
        else:
            output_v = vdd / 2.0

        logic_value = 1 if output_v > vdd / 2.0 else 0
        current = 0.0 if not (pulldown_on and pullup_on) else 0.001

        c_load = self.nmos1.params.c_drain + self.pmos1.params.c_drain
        ids_sat = self.nmos1.drain_current(vdd, vdd)
        delay = c_load * vdd / (2.0 * ids_sat) if ids_sat > 0 else 1e-9

        return GateOutput(
            logic_value=logic_value,
            voltage=output_v,
            current_draw=current,
            power_dissipation=current * vdd,
            propagation_delay=delay,
            transistor_count=self.TRANSISTOR_COUNT,
        )

    def evaluate_digital(self, a: int, b: int) -> int:
        """Evaluate with digital inputs (0 or 1)."""
        _validate_bit(a, "a")
        _validate_bit(b, "b")
        vdd = self.circuit.vdd
        va = vdd if a == 1 else 0.0
        vb = vdd if b == 1 else 0.0
        return self.evaluate(va, vb).logic_value


class CMOSAnd:
    """CMOS AND gate: NAND + Inverter = 6 transistors.

    There is no "direct" CMOS AND gate. The CMOS topology naturally
    produces inverted outputs (NAND, NOR), so to get AND we must add
    an inverter after the NAND. This adds 2 transistors and one gate
    delay, which is why NAND-based design is preferred in real chips.
    """

    TRANSISTOR_COUNT = 6

    def __init__(self, circuit_params: CircuitParams | None = None) -> None:
        self.circuit = circuit_params or CircuitParams()
        self._nand = CMOSNand(circuit_params)
        self._inv = CMOSInverter(circuit_params)

    def evaluate(self, va: float, vb: float) -> GateOutput:
        """AND = NOT(NAND(A, B))."""
        nand_out = self._nand.evaluate(va, vb)
        inv_out = self._inv.evaluate(nand_out.voltage)
        return GateOutput(
            logic_value=inv_out.logic_value,
            voltage=inv_out.voltage,
            current_draw=nand_out.current_draw + inv_out.current_draw,
            power_dissipation=nand_out.power_dissipation + inv_out.power_dissipation,
            propagation_delay=nand_out.propagation_delay + inv_out.propagation_delay,
            transistor_count=self.TRANSISTOR_COUNT,
        )

    def evaluate_digital(self, a: int, b: int) -> int:
        """Evaluate with digital inputs."""
        _validate_bit(a, "a")
        _validate_bit(b, "b")
        vdd = self.circuit.vdd
        va = vdd if a == 1 else 0.0
        vb = vdd if b == 1 else 0.0
        return self.evaluate(va, vb).logic_value


class CMOSOr:
    """CMOS OR gate: NOR + Inverter = 6 transistors."""

    TRANSISTOR_COUNT = 6

    def __init__(self, circuit_params: CircuitParams | None = None) -> None:
        self.circuit = circuit_params or CircuitParams()
        self._nor = CMOSNor(circuit_params)
        self._inv = CMOSInverter(circuit_params)

    def evaluate(self, va: float, vb: float) -> GateOutput:
        """OR = NOT(NOR(A, B))."""
        nor_out = self._nor.evaluate(va, vb)
        inv_out = self._inv.evaluate(nor_out.voltage)
        return GateOutput(
            logic_value=inv_out.logic_value,
            voltage=inv_out.voltage,
            current_draw=nor_out.current_draw + inv_out.current_draw,
            power_dissipation=nor_out.power_dissipation + inv_out.power_dissipation,
            propagation_delay=nor_out.propagation_delay + inv_out.propagation_delay,
            transistor_count=self.TRANSISTOR_COUNT,
        )

    def evaluate_digital(self, a: int, b: int) -> int:
        """Evaluate with digital inputs."""
        _validate_bit(a, "a")
        _validate_bit(b, "b")
        vdd = self.circuit.vdd
        va = vdd if a == 1 else 0.0
        vb = vdd if b == 1 else 0.0
        return self.evaluate(va, vb).logic_value


class CMOSXor:
    """CMOS XOR gate using transmission gate design = 6 transistors.

    XOR can be built from NAND gates (4 NANDs = 16 transistors) or using
    an optimized transmission gate topology (6 transistors). We implement
    the 4-NAND version for educational purposes (demonstrating NAND
    universality) and use it for the digital evaluation.

    === XOR from 4 NANDs ===

        XOR(A, B) = NAND(NAND(A, NAND(A,B)), NAND(B, NAND(A,B)))

    This construction proves that XOR can be built from the universal
    NAND gate alone, which in turn is built from just 4 transistors.
    """

    TRANSISTOR_COUNT = 6

    def __init__(self, circuit_params: CircuitParams | None = None) -> None:
        self.circuit = circuit_params or CircuitParams()
        self._nand1 = CMOSNand(circuit_params)
        self._nand2 = CMOSNand(circuit_params)
        self._nand3 = CMOSNand(circuit_params)
        self._nand4 = CMOSNand(circuit_params)

    def evaluate(self, va: float, vb: float) -> GateOutput:
        """XOR using 4 NAND gates."""
        vdd = self.circuit.vdd

        # Step 1: NAND(A, B)
        nand_ab = self._nand1.evaluate(va, vb)
        # Step 2: NAND(A, NAND(A,B))
        nand_a_nab = self._nand2.evaluate(va, nand_ab.voltage)
        # Step 3: NAND(B, NAND(A,B))
        nand_b_nab = self._nand3.evaluate(vb, nand_ab.voltage)
        # Step 4: NAND(step2, step3)
        result = self._nand4.evaluate(nand_a_nab.voltage, nand_b_nab.voltage)

        total_current = (
            nand_ab.current_draw
            + nand_a_nab.current_draw
            + nand_b_nab.current_draw
            + result.current_draw
        )
        total_delay = (
            nand_ab.propagation_delay
            + max(nand_a_nab.propagation_delay, nand_b_nab.propagation_delay)
            + result.propagation_delay
        )

        return GateOutput(
            logic_value=result.logic_value,
            voltage=result.voltage,
            current_draw=total_current,
            power_dissipation=total_current * vdd,
            propagation_delay=total_delay,
            transistor_count=self.TRANSISTOR_COUNT,
        )

    def evaluate_digital(self, a: int, b: int) -> int:
        """Evaluate with digital inputs."""
        _validate_bit(a, "a")
        _validate_bit(b, "b")
        vdd = self.circuit.vdd
        va = vdd if a == 1 else 0.0
        vb = vdd if b == 1 else 0.0
        return self.evaluate(va, vb).logic_value

    def evaluate_from_nands(self, a: int, b: int) -> int:
        """Build XOR from 4 NAND gates to demonstrate universality.

        This is the same as evaluate_digital but makes the NAND
        construction explicit for educational purposes.
        """
        return self.evaluate_digital(a, b)

# 11 — Transistors

## Overview

The transistors package models the electronic switches that logic gates are built from. Every AND, OR, NOT, and NAND gate in a real CPU is physically constructed from transistors — tiny electrically-controlled switches etched into silicon. This package shows exactly how that construction works, and the logic gates package uses it under the hood.

We model two families of transistors:

- **MOSFET (Metal-Oxide-Semiconductor Field-Effect Transistor)**: The technology used in all modern chips. We implement both NMOS and PMOS variants, and show how pairing them creates CMOS (Complementary MOS) logic — the technology behind every CPU, GPU, and phone chip made since the 1980s.

- **BJT (Bipolar Junction Transistor)**: The older technology used in TTL (Transistor-Transistor Logic) chips from the 1960s-1980s. We implement NPN and PNP variants for historical context and to show why CMOS replaced them.

Beyond digital switching, we also model the **analog behavior** of transistors: operating regions, gain, transconductance, and amplification. A transistor is fundamentally an analog device — digital logic is a special case where we deliberately keep it in only two states.

We include a **full electrical model**: voltage thresholds, capacitance, power dissipation, and switching speed. This lets us answer questions like "why does lowering Vdd save so much power?" and "why did CMOS replace TTL?"

This is Layer 0 of the computing stack. It has no dependencies.

## Layer Position

```
[YOU ARE HERE] → Logic Gates → Arithmetic → CPU → ARM → Assembler → ...
```

**Input from:** Nothing — this is the foundation beneath the foundation.
**Output to:** Logic gates package (spec 10). The logic gates are built from CMOS transistor circuits internally.

```
Conceptual hierarchy:

    Source Code
        │
        ▼
    ... (compiler, VM, CPU, arithmetic) ...
        │
        ▼
    Logic Gates (spec 10)     ← pure functions on 0/1 (public API unchanged)
        │
        ▼ (internally built from)
    Transistors (spec 11)     ← electrical simulation  ← YOU ARE HERE
        │
        ▼
    Physics (not modeled)     ← quantum mechanics, semiconductor band theory
```

## Concepts

### What is a transistor?

A transistor is an electrically-controlled switch. It has three terminals, and the voltage or current at one terminal controls whether current flows between the other two.

**Water analogy:**

```
Imagine a water pipe with a valve in the middle:

    Water In (Source/Emitter)
        │
        │
    ┌───┴───┐
    │ VALVE │ ← controlled by a handle (Gate/Base)
    └───┬───┘
        │
        │
    Water Out (Drain/Collector)

- Turn the handle one way: water flows freely (switch ON)
- Turn the handle the other way: water stops (switch OFF)
- Turn it partway: some water flows (AMPLIFIER mode)

A transistor does this with electricity instead of water.
The "handle" is controlled by voltage (MOSFET) or current (BJT).
```

### The two transistor families

```
MOSFET (modern, used in CMOS)          BJT (older, used in TTL)
┌────────────────────────┐             ┌────────────────────────┐
│ Voltage-controlled     │             │ Current-controlled     │
│ Gate voltage controls  │             │ Base current controls   │
│   drain-source current │             │   collector current     │
│                        │             │                        │
│ Variants:              │             │ Variants:              │
│   NMOS (N-channel)     │             │   NPN                  │
│   PMOS (P-channel)     │             │   PNP                  │
│                        │             │                        │
│ Advantages:            │             │ Advantages:            │
│   Near-zero static     │             │   Higher speed         │
│   power consumption    │             │   (historically)       │
│   Very high density    │             │   Better analog gain   │
│   Pairs into CMOS      │             │                        │
│                        │             │ Disadvantages:         │
│ Used in:               │             │   High static power    │
│   All modern CPUs,     │             │   Lower density        │
│   GPUs, phone chips    │             │   Needs constant base  │
│                        │             │   current              │
└────────────────────────┘             └────────────────────────┘
```

### MOSFET terminals and symbols

```
NMOS Transistor               PMOS Transistor
(N-channel MOSFET)            (P-channel MOSFET)

     Drain (D)                     Source (S)
       │                             │
       │                             │
  ─────┤                        ─────┤
Gate ──┤│                  Gate ──┤│──o   (circle = inverted)
  ─────┤                        ─────┤
       │                             │
       │                             │
     Source (S)                     Drain (D)

NMOS behavior:                PMOS behavior:
  Vgs > Vth → ON (conducts)    Vgs < -|Vth| → ON (conducts)
  Vgs < Vth → OFF (blocks)    Vgs > -|Vth| → OFF (blocks)
  "High gate = ON"             "Low gate = ON"

  Vgs = Gate-to-Source voltage
  Vth = Threshold voltage (typically 0.3V - 0.7V for modern CMOS)
```

**Key insight:** NMOS and PMOS are complementary — NMOS turns ON with high voltage, PMOS turns ON with low voltage. CMOS exploits this: for any input, exactly one is ON and the other is OFF, so no static current flows.

### MOSFET operating regions

A MOSFET has three operating regions, depending on the gate-to-source voltage (Vgs) and drain-to-source voltage (Vds):

```
                          │
    Drain Current (Ids)   │          Saturation Region
                          │         ╱  (Ids ≈ constant)
                          │        ╱
                          │       ╱━━━━━━━━━━━━━━━━━━  Vgs = high
                          │      ╱
                          │     ╱━━━━━━━━━━━━━━━━━━━━  Vgs = medium
                          │    ╱
                          │   ╱  Linear
                          │  ╱   (Ohmic)
                          │ ╱    Region
                          │╱
                ──────────┼──────────────────────────
                          │              Vds →

Cutoff:      Vgs < Vth         → Ids ≈ 0 (switch OFF)
Linear:      Vgs > Vth,        → Ids ∝ Vds (acts like a resistor)
             Vds < Vgs - Vth
Saturation:  Vgs > Vth,        → Ids ≈ constant (amplifier region)
             Vds > Vgs - Vth

Digital switching uses only Cutoff (OFF) and deep Linear (ON).
Analog amplifiers operate in Saturation.
```

**Current equations (simplified, for NMOS):**

```
Cutoff (Vgs < Vth):
    Ids = 0

Linear/Ohmic (Vgs >= Vth, Vds < Vgs - Vth):
    Ids = k * ((Vgs - Vth) * Vds - 0.5 * Vds^2)

Saturation (Vgs >= Vth, Vds >= Vgs - Vth):
    Ids = 0.5 * k * (Vgs - Vth)^2

Where:
    k = mu_n * Cox * (W / L)
    mu_n = electron mobility (~0.06 m^2/Vs for silicon)
    Cox  = oxide capacitance per unit area
    W    = channel width
    L    = channel length
    W/L  = aspect ratio (key design parameter)
```

### BJT terminals and operating regions

```
NPN Transistor                PNP Transistor

     Collector (C)                Emitter (E)
       │                             │
       │                             │
  ─────┤                        ─────┤
Base ──┤──>                Base ──┤──<   (arrow shows current direction)
  ─────┤                        ─────┤
       │                             │
       │                             │
     Emitter (E)                  Collector (C)

NPN: Current flows C→E           PNP: Current flows E→C
     when base current            when base current
     flows B→E.                   flows E→B.

Operating Regions:
  Cutoff:      Vbe < 0.7V → OFF (no collector current)
  Active:      Vbe ≈ 0.7V, Vce > 0.2V → Ic = beta * Ib (amplifier)
  Saturation:  Vbe ≈ 0.7V, Vce < 0.2V → fully ON (switch)

  beta (β or hfe) = current gain, typically 50-300
  Vbe = base-emitter voltage
  Vce = collector-emitter voltage
```

### CMOS gate construction

The fundamental insight of CMOS: pair a **pull-up network** (PMOS transistors connecting output to Vdd) with a **pull-down network** (NMOS transistors connecting output to Vss/ground). For every input combination, exactly one network is active.

#### CMOS Inverter (NOT gate) — the simplest CMOS circuit

```
         Vdd (power supply, e.g. 5V or 1.8V)
          │
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
          │
         Vss (ground, 0V)

When A = HIGH (Vdd):
  - NMOS gate is HIGH → NMOS turns ON → output pulled to Vss (LOW)
  - PMOS gate is HIGH → PMOS turns OFF → no connection to Vdd
  - Output = LOW  (NOT of HIGH)

When A = LOW (Vss):
  - NMOS gate is LOW → NMOS turns OFF → no connection to Vss
  - PMOS gate is LOW → PMOS turns ON → output pulled to Vdd (HIGH)
  - Output = HIGH  (NOT of LOW)

Static power: ZERO — one transistor is always OFF, breaking the
path from Vdd to Vss. This is why CMOS dominates: billions of gates
on a chip, and only the ones actively switching consume power.
```

#### CMOS NAND gate (2-input)

```
              Vdd
          ┌────┴────┐
     ┌────┤  PMOS1  ├────┐         Pull-up network:
     │    └────┬────┘    │         PMOS in PARALLEL
     │    Gate=A         │         (either one ON pulls output HIGH)
     │         │    ┌────┴────┐
     │         │    │  PMOS2  │
     │         │    └────┬────┘
     │         │    Gate=B
     │         │         │
     └─────────┴────┬────┘
                    │
                 Output (Y = NOT(A AND B))
                    │
              ┌─────┴─────┐
              │   NMOS1   │        Pull-down network:
              └─────┬─────┘        NMOS in SERIES
              Gate=A               (BOTH must be ON to pull output LOW)
              ┌─────┴─────┐
              │   NMOS2   │
              └─────┬─────┘
              Gate=B
                    │
                   Vss

A=0, B=0: Both PMOS ON (parallel), both NMOS OFF → Output=1
A=0, B=1: PMOS1 ON → Output=1. NMOS2 ON but NMOS1 OFF → no path to ground.
A=1, B=0: PMOS2 ON → Output=1. NMOS1 ON but NMOS2 OFF → no path to ground.
A=1, B=1: Both PMOS OFF, both NMOS ON → Output=0.

NAND truth table: Y = NOT(A AND B)
```

#### CMOS NOR gate (2-input)

```
              Vdd
              │
         ┌────┴────┐
         │  PMOS1  │              Pull-up network:
         └────┬────┘              PMOS in SERIES
         Gate=A                   (BOTH must be ON → both inputs LOW)
         ┌────┴────┐
         │  PMOS2  │
         └────┬────┘
         Gate=B
              │
           Output (Y = NOT(A OR B))
              │
     ┌────────┴────────┐
┌────┴────┐       ┌────┴────┐     Pull-down network:
│  NMOS1  │       │  NMOS2  │     NMOS in PARALLEL
└────┬────┘       └────┬────┘     (either one ON pulls output LOW)
Gate=A            Gate=B
     └────────┬────────┘
              │
             Vss

A=0, B=0: Both PMOS ON (series path to Vdd), both NMOS OFF → Output=1
A=0, B=1: PMOS1 ON but PMOS2 OFF → no Vdd path. NMOS2 ON → Output=0.
A=1, B=0: PMOS2 ON but PMOS1 OFF → no Vdd path. NMOS1 ON → Output=0.
A=1, B=1: Both PMOS OFF. Both NMOS ON → Output=0.

NOR truth table: Y = NOT(A OR B)
```

#### Deriving AND, OR, XOR from NAND and NOR

```
AND(A,B) = NOT(NAND(A,B))     → CMOS NAND + CMOS Inverter = 4+2 = 6 transistors
OR(A,B)  = NOT(NOR(A,B))      → CMOS NOR + CMOS Inverter  = 4+2 = 6 transistors
XOR(A,B) = (A AND NOT B) OR (NOT A AND B)
         → multiple CMOS stages, typically 8-12 transistors
         → optimized transmission-gate XOR uses 6 transistors
```

### TTL gate construction (historical, BJT-based)

```
TTL NAND Gate (7400-series)

            Vcc (+5V)
             │
             R1 (4kOhm)
             │
        ┌────┴────┐
        │  Q1     │     Multi-emitter input transistor
        │  (NPN)  │     (one emitter per input)
        ├── E1 ───┤── Input A
        ├── E2 ───┤── Input B
        └────┬────┘
             │ (collector)
             │
        ┌────┴────┐
        │  Q2     │     Phase splitter
        │  (NPN)  │
        └────┬────┘
             │
        ┌────┴────┐
        │  Q3     │     Output driver
        │  (NPN)  │
        └────┬────┘
             │
            GND

Simplified behavior:
- Any input LOW → Q1 saturates, steals base current from Q2
  → Q2/Q3 OFF → Output pulled HIGH through pull-up resistor
- ALL inputs HIGH → Q1 cutoff, current flows to Q2 base
  → Q2/Q3 ON → Output pulled LOW
- Result: NAND function

Problem: When Q3 is ON, current flows Vcc → R → Q3 → GND
constantly. This is STATIC POWER DISSIPATION — the reason TTL
was replaced by CMOS.

TTL power per gate: ~1-10 mW
CMOS static power:  ~nanowatts (only leakage)
```

### Electrical model parameters

```
┌──────────────────────────────────────────────────────────────┐
│ Parameter         │ Symbol  │ Typical CMOS  │ Typical TTL   │
├───────────────────┼─────────┼───────────────┼───────────────┤
│ Supply voltage    │ Vdd/Vcc │ 1.0-3.3V      │ 5.0V          │
│ Threshold voltage │ Vth     │ 0.3-0.7V      │ 0.7V (Vbe)    │
│ Noise margin high │ NMH     │ ~0.3V         │ ~0.4V         │
│ Noise margin low  │ NML     │ ~0.3V         │ ~0.4V         │
│ Propagation delay │ tpd     │ 0.01-1 ns     │ 5-15 ns       │
│ Static power      │ Pstat   │ ~nW (leakage) │ ~1-10 mW      │
│ Dynamic power     │ Pdyn    │ C*V^2*f       │ C*V^2*f       │
│ Gate capacitance  │ Cg      │ ~1 fF         │ ~5 pF         │
└──────────────────────────────────────────────────────────────┘
```

### Power dissipation

```
Dynamic Power Formula:
  P_dynamic = C_load * Vdd^2 * f_switching * alpha

  C_load     = output load capacitance
  Vdd        = supply voltage
  f_switching = clock frequency
  alpha      = activity factor (fraction of cycles the gate switches)

This is why lowering Vdd matters so much — power scales with V SQUARED.
Going from 5V (TTL) to 1V (modern CMOS) reduces dynamic power by 25x.

Propagation Delay:
  t_pd ≈ (C_load * Vdd) / (2 * Ids_sat)

  Lower Vdd → less current → SLOWER
  This is the fundamental speed-vs-power tradeoff in chip design.

Noise Margins:
  NMH = VOH_min - VIH_min   (how much noise a HIGH signal can tolerate)
  NML = VIL_max - VOL_max   (how much noise a LOW signal can tolerate)

  VOH = output HIGH voltage (ideally Vdd)
  VOL = output LOW voltage  (ideally 0V)
  VIH = minimum input voltage recognized as HIGH
  VIL = maximum input voltage recognized as LOW
```

## Public API

```python
from dataclasses import dataclass
from enum import Enum


# ===========================================================================
# ENUMS
# ===========================================================================

class MOSFETRegion(Enum):
    """Operating region of a MOSFET transistor."""
    CUTOFF = "cutoff"           # Vgs < Vth — no current, switch OFF
    LINEAR = "linear"           # Vgs > Vth, Vds < Vgs-Vth — resistor-like
    SATURATION = "saturation"   # Vgs > Vth, Vds > Vgs-Vth — constant current


class BJTRegion(Enum):
    """Operating region of a BJT transistor."""
    CUTOFF = "cutoff"           # Vbe < 0.7V — no current, switch OFF
    ACTIVE = "active"           # Vbe ~ 0.7V, Vce > 0.2V — amplifier
    SATURATION = "saturation"   # Vbe ~ 0.7V, Vce < 0.2V — fully ON


class TransistorType(Enum):
    """Transistor polarity/type."""
    NMOS = "nmos"
    PMOS = "pmos"
    NPN = "npn"
    PNP = "pnp"


# ===========================================================================
# ELECTRICAL PARAMETERS
# ===========================================================================

@dataclass(frozen=True)
class MOSFETParams:
    """Electrical parameters for a MOSFET transistor.

    These define the physical characteristics of the transistor.
    Default values represent a typical 180nm CMOS process.
    """
    vth: float = 0.4          # Threshold voltage (V)
    k: float = 0.001          # Transconductance parameter (A/V^2), k = mu * Cox * W/L
    w: float = 1e-6           # Channel width (m)
    l: float = 180e-9         # Channel length (m)
    c_gate: float = 1e-15     # Gate capacitance (F)
    c_drain: float = 0.5e-15  # Drain capacitance (F)


@dataclass(frozen=True)
class BJTParams:
    """Electrical parameters for a BJT transistor.

    Default values represent a typical small-signal NPN (e.g. 2N2222).
    """
    beta: float = 100.0       # Current gain (Ic/Ib), typically 50-300
    vbe_on: float = 0.7       # Base-emitter turn-on voltage (V)
    vce_sat: float = 0.2      # Collector-emitter saturation voltage (V)
    is_: float = 1e-14        # Reverse saturation current (A)
    c_base: float = 5e-12     # Base capacitance (F)


@dataclass(frozen=True)
class CircuitParams:
    """Parameters for a complete logic gate circuit."""
    vdd: float = 3.3          # Supply voltage (V)
    temperature: float = 300.0  # Temperature (K), room temp = 300K


# ===========================================================================
# TRANSISTOR CLASSES
# ===========================================================================

class NMOS:
    """N-channel MOSFET transistor.

    An NMOS transistor turns ON (conducts from drain to source) when
    the gate voltage is HIGH (above threshold). Think of it as a
    normally-open switch that closes when you apply voltage to the gate.

    Water analogy: a valve that opens when you push the handle UP.
    """

    def __init__(self, params: MOSFETParams | None = None) -> None: ...

    def region(self, vgs: float, vds: float) -> MOSFETRegion:
        """Determine the operating region given terminal voltages."""
        ...

    def drain_current(self, vgs: float, vds: float) -> float:
        """Calculate drain-to-source current (Ids) in amps.

        Uses the simplified MOSFET equations:
          Cutoff:     Ids = 0
          Linear:     Ids = k * ((Vgs-Vth)*Vds - 0.5*Vds^2)
          Saturation: Ids = 0.5 * k * (Vgs-Vth)^2
        """
        ...

    def is_conducting(self, vgs: float) -> bool:
        """Digital abstraction: is this transistor ON?

        Returns True if Vgs > Vth (gate voltage exceeds threshold).
        """
        ...

    def output_voltage(self, vgs: float, vdd: float) -> float:
        """Voltage at drain when used as a switch with a pull-up.

        When ON: output ~ 0V (pulled to ground through channel)
        When OFF: output ~ vdd (pulled up by load)
        """
        ...


class PMOS:
    """P-channel MOSFET transistor.

    A PMOS transistor turns ON (conducts from source to drain) when
    the gate voltage is LOW (below source voltage by more than |Vth|).
    It is the complement of NMOS.

    Water analogy: a valve that opens when you push the handle DOWN.
    """

    def __init__(self, params: MOSFETParams | None = None) -> None: ...
    def region(self, vgs: float, vds: float) -> MOSFETRegion: ...
    def drain_current(self, vgs: float, vds: float) -> float: ...
    def is_conducting(self, vgs: float) -> bool: ...
    def output_voltage(self, vgs: float, vdd: float) -> float: ...


class NPN:
    """NPN bipolar junction transistor.

    An NPN transistor turns ON when base current flows into the base
    (Vbe > 0.7V). A small base current controls a much larger collector
    current: Ic = beta * Ib. This current amplification is what made
    transistors revolutionary.

    Water analogy: a small stream (base current) controls the gate
    on a much larger river (collector current).
    """

    def __init__(self, params: BJTParams | None = None) -> None: ...

    def region(self, vbe: float, vce: float) -> BJTRegion:
        """Determine operating region from terminal voltages."""
        ...

    def collector_current(self, vbe: float, vce: float) -> float:
        """Calculate collector current (Ic) in amps.

          Cutoff:     Ic = 0
          Active:     Ic = beta * Ib (where Ib derived from Vbe)
          Saturation: Ic limited by external circuit
        """
        ...

    def is_conducting(self, vbe: float) -> bool:
        """Digital abstraction: is this transistor ON?"""
        ...


class PNP:
    """PNP bipolar junction transistor.

    Complement of NPN. Turns ON when base is pulled LOW relative to
    emitter (Veb > 0.7V). Current flows from emitter to collector.
    """

    def __init__(self, params: BJTParams | None = None) -> None: ...
    def region(self, vbe: float, vce: float) -> BJTRegion: ...
    def collector_current(self, vbe: float, vce: float) -> float: ...
    def is_conducting(self, vbe: float) -> bool: ...


# ===========================================================================
# ANALOG AMPLIFIER ANALYSIS
# ===========================================================================

@dataclass(frozen=True)
class AmplifierAnalysis:
    """Results of analyzing a transistor as an amplifier."""
    voltage_gain: float       # Av = -gm * Rd (for common-source MOSFET)
    transconductance: float   # gm = 2*Ids/(Vgs-Vth) in saturation
    input_impedance: float    # Zin (ohms) — very high for MOSFET
    output_impedance: float   # Zout (ohms)
    bandwidth: float          # f_3dB = gm / (2*pi*C_load)
    operating_point: dict     # DC bias point: Vgs, Vds, Ids


def analyze_common_source_amp(
    transistor: NMOS,
    vgs: float,
    vdd: float,
    r_drain: float,
    c_load: float = 1e-12,
) -> AmplifierAnalysis:
    """Analyze an NMOS common-source amplifier configuration.

    The common-source amplifier is the most basic MOSFET amplifier.
    The input signal modulates Vgs, which modulates Ids, which creates
    a voltage swing across the drain resistor.
    """
    ...


def analyze_common_emitter_amp(
    transistor: NPN,
    vbe: float,
    vcc: float,
    r_collector: float,
    c_load: float = 1e-12,
) -> AmplifierAnalysis:
    """Analyze an NPN common-emitter amplifier configuration."""
    ...


# ===========================================================================
# CMOS LOGIC GATE CIRCUITS
# ===========================================================================

@dataclass(frozen=True)
class GateOutput:
    """Result of evaluating a CMOS or TTL gate with voltage-level detail."""
    logic_value: int          # 0 or 1 (digital interpretation)
    voltage: float            # Actual output voltage (V)
    current_draw: float       # Total current from Vdd (A)
    power_dissipation: float  # Instantaneous power (W)
    propagation_delay: float  # Estimated delay (seconds)
    transistor_count: int     # Number of transistors in this gate


class CMOSInverter:
    """CMOS NOT gate: 1 PMOS + 1 NMOS = 2 transistors.

    The simplest and most important CMOS circuit. Every other CMOS
    gate is a variation of this pattern.
    """

    def __init__(
        self,
        circuit_params: CircuitParams | None = None,
        nmos_params: MOSFETParams | None = None,
        pmos_params: MOSFETParams | None = None,
    ) -> None: ...

    def evaluate(self, input_voltage: float) -> GateOutput:
        """Evaluate the inverter with an analog input voltage.

        Maps input voltage through the CMOS transfer characteristic
        to produce an output voltage, then interprets it digitally.
        """
        ...

    def evaluate_digital(self, a: int) -> int:
        """Evaluate with digital input (0 or 1), returns 0 or 1.

        Convenience method that maps 0 -> 0V, 1 -> Vdd, then
        evaluates and returns the digital interpretation.
        """
        ...

    def voltage_transfer_characteristic(
        self, steps: int = 100
    ) -> list[tuple[float, float]]:
        """Generate the VTC curve: list of (Vin, Vout) points.

        The VTC shows the sharp switching threshold of CMOS — the
        output snaps from HIGH to LOW over a very narrow input range.
        """
        ...

    @property
    def static_power(self) -> float:
        """Static power dissipation (ideally ~0 for CMOS)."""
        ...

    def dynamic_power(self, frequency: float, c_load: float) -> float:
        """Dynamic power: P = C_load * Vdd^2 * f."""
        ...


class CMOSNand:
    """CMOS NAND gate: 2 PMOS parallel + 2 NMOS series = 4 transistors."""

    def __init__(
        self,
        circuit_params: CircuitParams | None = None,
        nmos_params: MOSFETParams | None = None,
        pmos_params: MOSFETParams | None = None,
    ) -> None: ...

    def evaluate(self, va: float, vb: float) -> GateOutput: ...
    def evaluate_digital(self, a: int, b: int) -> int: ...

    @property
    def transistor_count(self) -> int:
        """Returns 4."""
        ...


class CMOSNor:
    """CMOS NOR gate: 2 PMOS series + 2 NMOS parallel = 4 transistors."""

    def __init__(
        self,
        circuit_params: CircuitParams | None = None,
        nmos_params: MOSFETParams | None = None,
        pmos_params: MOSFETParams | None = None,
    ) -> None: ...

    def evaluate(self, va: float, vb: float) -> GateOutput: ...
    def evaluate_digital(self, a: int, b: int) -> int: ...


class CMOSAnd:
    """CMOS AND gate: NAND + Inverter = 6 transistors.

    There is no "direct" CMOS AND gate — NAND is the natural gate.
    AND requires an extra inverter stage, which is why NAND-based
    design is preferred in real chips.
    """

    def __init__(self, circuit_params: CircuitParams | None = None) -> None: ...
    def evaluate(self, va: float, vb: float) -> GateOutput: ...
    def evaluate_digital(self, a: int, b: int) -> int: ...


class CMOSOr:
    """CMOS OR gate: NOR + Inverter = 6 transistors."""

    def __init__(self, circuit_params: CircuitParams | None = None) -> None: ...
    def evaluate(self, va: float, vb: float) -> GateOutput: ...
    def evaluate_digital(self, a: int, b: int) -> int: ...


class CMOSXor:
    """CMOS XOR gate using transmission gate design = 6 transistors.

    XOR can be built from NAND gates (requires 4 NANDs = 16 transistors)
    or using an optimized transmission gate topology (6 transistors).
    We implement both and let users compare.
    """

    def __init__(self, circuit_params: CircuitParams | None = None) -> None: ...
    def evaluate(self, va: float, vb: float) -> GateOutput: ...
    def evaluate_digital(self, a: int, b: int) -> int: ...

    def evaluate_from_nands(self, a: int, b: int) -> int:
        """Build XOR from 4 NAND gates to demonstrate universality."""
        ...


# ===========================================================================
# TTL LOGIC GATES (historical, BJT-based)
# ===========================================================================

class TTLNand:
    """TTL NAND gate using NPN transistors (7400-series style).

    Demonstrates the historical TTL circuit topology with multi-emitter
    input transistor, phase splitter, and totem-pole output.
    """

    def __init__(
        self,
        vcc: float = 5.0,
        bjt_params: BJTParams | None = None,
    ) -> None: ...

    def evaluate(self, va: float, vb: float) -> GateOutput: ...
    def evaluate_digital(self, a: int, b: int) -> int: ...

    @property
    def static_power(self) -> float:
        """Static power — significantly higher than CMOS."""
        ...


class RTLInverter:
    """Resistor-Transistor Logic inverter (earliest IC logic family).

    The simplest possible transistor circuit for logic:
    one NPN transistor with a base resistor and collector resistor.
    Predates TTL. Very slow, very power-hungry, but simple to understand.
    """

    def __init__(
        self,
        vcc: float = 5.0,
        r_base: float = 10_000.0,
        r_collector: float = 1_000.0,
        bjt_params: BJTParams | None = None,
    ) -> None: ...

    def evaluate(self, v_input: float) -> GateOutput: ...
    def evaluate_digital(self, a: int) -> int: ...


# ===========================================================================
# ELECTRICAL ANALYSIS FUNCTIONS
# ===========================================================================

@dataclass(frozen=True)
class NoiseMargins:
    """Noise margin analysis for a logic family."""
    vol: float    # Output LOW voltage
    voh: float    # Output HIGH voltage
    vil: float    # Input LOW threshold
    vih: float    # Input HIGH threshold
    nml: float    # Noise margin LOW  = VIL - VOL
    nmh: float    # Noise margin HIGH = VOH - VIH


def compute_noise_margins(gate: CMOSInverter | TTLNand) -> NoiseMargins:
    """Analyze noise margins for a gate.

    Noise margins tell you how much electrical noise a signal can
    tolerate before being misinterpreted. Larger margins = more robust.
    """
    ...


@dataclass(frozen=True)
class PowerAnalysis:
    """Power consumption breakdown for a gate or circuit."""
    static_power: float       # Leakage power when not switching (W)
    dynamic_power: float      # Switching power at given frequency (W)
    total_power: float        # static + dynamic
    energy_per_switch: float  # Energy for one 0->1->0 transition (J)


def analyze_power(
    gate: CMOSInverter | CMOSNand | CMOSNor | TTLNand,
    frequency: float = 1e9,
    c_load: float = 1e-12,
    activity_factor: float = 0.5,
) -> PowerAnalysis:
    """Compute power consumption for a gate at a given operating frequency.

    Args:
        gate: The gate to analyze
        frequency: Switching frequency (Hz), default 1 GHz
        c_load: Output load capacitance (F), default 1 pF
        activity_factor: Fraction of cycles where output switches (0-1)
    """
    ...


@dataclass(frozen=True)
class TimingAnalysis:
    """Timing characteristics for a gate."""
    tphl: float       # Propagation delay HIGH->LOW (seconds)
    tplh: float       # Propagation delay LOW->HIGH (seconds)
    tpd: float        # Average propagation delay (seconds)
    rise_time: float  # 10%-90% rise time (seconds)
    fall_time: float  # 90%-10% fall time (seconds)
    max_frequency: float  # Maximum operating frequency (Hz)


def analyze_timing(
    gate: CMOSInverter | CMOSNand | CMOSNor | TTLNand,
    c_load: float = 1e-12,
) -> TimingAnalysis:
    """Compute timing characteristics for a gate.

    Propagation delay limits how fast a circuit can operate.
    t_pd ~ (C_load * Vdd) / (2 * I_sat)
    """
    ...


# ===========================================================================
# COMPARISON AND EDUCATIONAL UTILITIES
# ===========================================================================

def compare_cmos_vs_ttl(
    frequency: float = 1e6,
    c_load: float = 1e-12,
) -> dict:
    """Compare CMOS and TTL NAND gates across all metrics.

    Returns a dictionary with side-by-side comparison of:
    - Transistor count
    - Power consumption
    - Speed (propagation delay)
    - Noise margins
    - Supply voltage

    This function demonstrates WHY CMOS replaced TTL.
    """
    ...


def demonstrate_cmos_scaling(
    technology_nodes: list[float] | None = None,
) -> list[dict]:
    """Show how CMOS performance changes with technology scaling.

    Default technology nodes: [180nm, 90nm, 45nm, 22nm, 7nm, 3nm]

    For each node, shows:
    - Threshold voltage (decreases)
    - Supply voltage (decreases)
    - Gate capacitance (decreases)
    - Switching speed (increases)
    - Dynamic power (decreases with V^2)
    - Leakage current (increases — the modern challenge)

    This demonstrates Moore's Law in action and the "power wall"
    that has driven the shift to multi-core processors.
    """
    ...
```

## Data Flow

```
Input:  Terminal voltages (Vgs, Vds for MOSFET; Vbe, Vce for BJT)
        OR digital values (0 or 1) for convenience methods

Processing:
  1. Determine operating region from terminal voltages
  2. Calculate current using region-appropriate equation
  3. For gate circuits: evaluate pull-up and pull-down networks
  4. Determine output voltage from network analysis
  5. Interpret output voltage as digital value (above/below threshold)
  6. Calculate power dissipation and propagation delay

Output: GateOutput with logic value, voltage, current, power, delay
        OR simple int (0 or 1) for digital-only evaluation
        OR AmplifierAnalysis for analog mode
```

## Test Strategy

### 1. Individual transistor tests

Verify all three operating regions for each transistor type:

```python
# NMOS: verify all three operating regions
def test_nmos_cutoff():
    t = NMOS()
    assert t.region(vgs=0.0, vds=1.0) == MOSFETRegion.CUTOFF
    assert t.drain_current(vgs=0.0, vds=1.0) == 0.0
    assert not t.is_conducting(vgs=0.0)

def test_nmos_linear():
    t = NMOS()
    assert t.region(vgs=1.5, vds=0.1) == MOSFETRegion.LINEAR
    assert t.drain_current(vgs=1.5, vds=0.1) > 0

def test_nmos_saturation():
    t = NMOS()
    assert t.region(vgs=1.0, vds=3.0) == MOSFETRegion.SATURATION

# PMOS: complementary behavior (ON when gate is LOW)
def test_pmos_complementary():
    t = PMOS()
    assert t.is_conducting(vgs=-1.5)    # gate pulled LOW
    assert not t.is_conducting(vgs=0.0)  # gate at source level

# BJT: verify cutoff, active, saturation
def test_npn_active():
    t = NPN()
    assert t.region(vbe=0.7, vce=3.0) == BJTRegion.ACTIVE
    assert t.collector_current(vbe=0.7, vce=3.0) > 0
```

### 2. CMOS gate truth table verification

Every CMOS gate must produce the same logical result as the logic gates package:

```python
def test_cmos_nand_truth_table():
    nand = CMOSNand()
    assert nand.evaluate_digital(0, 0) == 1
    assert nand.evaluate_digital(0, 1) == 1
    assert nand.evaluate_digital(1, 0) == 1
    assert nand.evaluate_digital(1, 1) == 0
```

### 3. Voltage-level tests

```python
def test_cmos_inverter_voltage_swing():
    inv = CMOSInverter(CircuitParams(vdd=3.3))
    out_low = inv.evaluate(3.3)
    out_high = inv.evaluate(0.0)
    assert out_low.voltage < 0.1     # near ground
    assert out_high.voltage > 3.2    # near Vdd
```

### 4. Electrical property tests

```python
def test_cmos_zero_static_power():
    inv = CMOSInverter()
    assert inv.static_power < 1e-9   # less than 1 nanowatt

def test_ttl_has_static_power():
    nand = TTLNand()
    assert nand.static_power > 1e-3  # more than 1 milliwatt

def test_dynamic_power_scales_with_v_squared():
    inv_high = CMOSInverter(CircuitParams(vdd=3.3))
    inv_low = CMOSInverter(CircuitParams(vdd=1.65))
    p_high = inv_high.dynamic_power(frequency=1e9, c_load=1e-12)
    p_low = inv_low.dynamic_power(frequency=1e9, c_load=1e-12)
    ratio = p_high / p_low
    assert 3.5 < ratio < 4.5  # should be approximately 4x
```

### 5. Cross-validation with logic gates package

```python
def test_cmos_matches_logic_gates():
    from logic_gates import AND, OR, NOT, NAND, NOR, XOR

    cmos = {
        'not': CMOSInverter(), 'nand': CMOSNand(), 'nor': CMOSNor(),
        'and': CMOSAnd(), 'or': CMOSOr(), 'xor': CMOSXor(),
    }
    for a in (0, 1):
        assert cmos['not'].evaluate_digital(a) == NOT(a)
        for b in (0, 1):
            assert cmos['nand'].evaluate_digital(a, b) == NAND(a, b)
            assert cmos['nor'].evaluate_digital(a, b) == NOR(a, b)
            assert cmos['and'].evaluate_digital(a, b) == AND(a, b)
            assert cmos['or'].evaluate_digital(a, b) == OR(a, b)
            assert cmos['xor'].evaluate_digital(a, b) == XOR(a, b)
```

### 6. Analog amplifier tests

```python
def test_common_source_gain():
    t = NMOS()
    result = analyze_common_source_amp(t, vgs=1.5, vdd=3.3, r_drain=10_000)
    assert result.voltage_gain < 0   # inverting amplifier

def test_mosfet_high_input_impedance():
    t = NMOS()
    result = analyze_common_source_amp(t, vgs=1.5, vdd=3.3, r_drain=10_000)
    assert result.input_impedance > 1e9  # > 1 gigaohm
```

### Coverage target: 95%+

Every class, method, operating region boundary, and edge case should be tested.

## Future Extensions

- **Transmission gates**: NMOS+PMOS pairs used as bidirectional analog switches, essential for multiplexers and SRAM cells.
- **SRAM cell**: 6-transistor SRAM cell showing how transistors store a single bit of memory.
- **Ring oscillator**: Chain of odd-numbered inverters that oscillates, used to characterize process speed.
- **SPICE-style simulation**: Time-domain simulation with capacitor charging/discharging for more accurate waveforms.
- **FinFET and GAA**: Modern 3D transistor structures that replaced planar MOSFET at 22nm and below.
- **Process variation**: Monte Carlo simulation of threshold voltage variation across millions of transistors on a chip.
- **Power gating**: Sleep transistors that disconnect circuits from Vdd to eliminate leakage.

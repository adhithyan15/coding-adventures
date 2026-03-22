# Transistors

**Layer 0 of the computing stack** — the physical electronic switches that logic gates are built from.

## What is a transistor?

A transistor is an electronic switch controlled by voltage. Think of it as a water valve: a small control signal (the gate/base voltage) controls whether a large current can flow between two other terminals.

In real hardware, every logic gate — NOT, AND, OR, NAND, NOR, XOR — is built from a handful of transistors wired together. A modern CPU contains billions of transistors, but each one is just a tiny voltage-controlled switch.

## Transistor families

### MOSFET (Metal-Oxide-Semiconductor Field-Effect Transistor)

The dominant transistor type in modern digital circuits. Uses an electric field to control current flow — essentially zero input current, which means near-zero static power dissipation.

- **NMOS**: Conducts when gate voltage is HIGH (Vgs > Vth). Used in pull-down networks.
- **PMOS**: Conducts when gate voltage is LOW (Vgs < -|Vth|). Used in pull-up networks.

Together, NMOS and PMOS form **CMOS** (Complementary MOS) — the basis of all modern digital logic.

### BJT (Bipolar Junction Transistor)

The older transistor type, still used in analog circuits and some specialty applications. Uses a small base current to control a larger collector current.

- **NPN**: Conducts when base-emitter voltage exceeds ~0.7V.
- **PNP**: Conducts when emitter-base voltage exceeds ~0.7V (base pulled low).

BJTs were used in **TTL** (Transistor-Transistor Logic) gates like the 7400-series NAND.

## How gates are built from transistors

### CMOS Inverter (NOT gate) — 2 transistors

```
        Vdd
         │
      ┌──┴──┐
 In ──┤ PMOS │──┬── Out
      └──┬──┘  │
         │     │
      ┌──┴──┐  │
 In ──┤ NMOS │──┘
      └──┬──┘
         │
        GND
```

When input is HIGH: NMOS conducts (pulls output to GND), PMOS is off → output LOW.
When input is LOW: PMOS conducts (pulls output to Vdd), NMOS is off → output HIGH.

### CMOS NAND — 4 transistors

Two PMOS in parallel (pull-up) + two NMOS in series (pull-down). Output is LOW only when both inputs are HIGH.

### CMOS NOR — 4 transistors

Two PMOS in series (pull-up) + two NMOS in parallel (pull-down). Output is HIGH only when both inputs are LOW.

## Package contents

This package provides:

1. **Transistor models**: `NMOS`, `PMOS`, `NPN`, `PNP` with operating region detection, drain/collector current calculation, and transconductance
2. **CMOS logic gates**: `CMOSInverter`, `CMOSNand`, `CMOSNor`, `CMOSAnd`, `CMOSOr`, `CMOSXor` — gates built from transistor instances
3. **TTL logic gates**: `TTLNand`, `RTLInverter` — historical BJT-based gates
4. **Amplifier analysis**: Common-source (MOSFET) and common-emitter (BJT) amplifier characterization
5. **Electrical analysis**: Noise margins, power consumption, timing, CMOS technology scaling

## Usage

```python
from transistors import NMOS, PMOS, CMOSInverter, CMOSNand

# Individual transistor
nmos = NMOS()
print(nmos.region(vgs=1.5, vds=3.0))  # MOSFETRegion.SATURATION
print(nmos.drain_current(vgs=1.5, vds=3.0))  # Current in amps

# CMOS gate (built from NMOS + PMOS transistors)
inv = CMOSInverter()
print(inv.evaluate_digital(0))  # 1
print(inv.evaluate_digital(1))  # 0

# Analog behavior
result = inv.evaluate(3.3)  # Input voltage → GateOutput with voltage, delay, transistor count
print(result.voltage)  # ~0.0V (output pulled to GND)

# Power analysis
from transistors import analyze_power
power = analyze_power(inv, frequency=1e9)
print(power.static_power)   # ~0W (CMOS advantage!)
print(power.dynamic_power)  # P = CV²f
```

## Where it fits in the stack

```
[Transistors] → Logic Gates → Arithmetic → CPU → ...
```

Transistors are the physical foundation. Logic gates compose transistors into boolean functions. Everything above builds on logic gates.

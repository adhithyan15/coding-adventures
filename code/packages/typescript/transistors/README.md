# @coding-adventures/transistors

Transistor-level circuit simulation — Layer 2 of the computing stack.

## What is this?

This package bridges the gap between abstract logic gates (Layer 1) and the physical transistors that implement them. It provides:

- **MOSFET transistors** (NMOS, PMOS) — the building blocks of modern CMOS digital circuits
- **BJT transistors** (NPN, PNP) — the original solid-state amplifiers used in TTL logic
- **CMOS logic gates** — NOT, NAND, NOR, AND, OR, XOR built from MOSFET pairs
- **TTL logic gates** — historical BJT-based NAND and RTL inverter
- **Amplifier analysis** — common-source (MOSFET) and common-emitter (BJT) amplifiers
- **Electrical analysis** — noise margins, power consumption, timing, technology scaling

## How it fits in the stack

```
Layer 3: Memory (flip-flops, registers, SRAM, DRAM)
Layer 2: Transistors  <-- YOU ARE HERE
Layer 1: Logic Gates (AND, OR, NOT, XOR, NAND, NOR)
```

Logic gates from Layer 1 give you truth tables — `AND(1, 1) = 1`. This package shows you *how* those gates work at the electrical level: what voltages appear, how much power is consumed, and how fast the gate can switch.

## Usage

```typescript
import {
  NMOS, PMOS, NPN, PNP,
  CMOSInverter, CMOSNand, TTLNand,
  analyzeCommonSourceAmp,
  computeNoiseMargins, analyzePower, analyzeTiming,
  compareCmosVsTtl, demonstrateCmosScaling,
} from "@coding-adventures/transistors";

// Create a CMOS inverter and evaluate it
const inv = new CMOSInverter();
console.log(inv.evaluateDigital(0)); // 1
console.log(inv.evaluateDigital(1)); // 0

// Get electrical details
const result = inv.evaluate(3.3); // Input = Vdd
console.log(result.voltage);           // ~0V (output LOW)
console.log(result.powerDissipation);   // ~0W (static)
console.log(result.propagationDelay);   // ~ps range

// Compare CMOS vs TTL
const comparison = compareCmosVsTtl();
console.log(comparison.cmos.static_power_w); // ~0 W
console.log(comparison.ttl.static_power_w);  // ~mW range

// See how technology scaling affects performance
const scaling = demonstrateCmosScaling();
// Shows trends from 180nm to 3nm
```

## Why CMOS replaced TTL

The key insight this package demonstrates:

| Metric | CMOS | TTL |
|--------|------|-----|
| Static power | ~0 W | ~1-10 mW per gate |
| Transistor count (NAND) | 4 | 3 |
| Supply voltage | 0.7-3.3V | 5V |
| Noise margins | Symmetric, ~40% Vdd | Asymmetric |

At 1 million gates: CMOS idle power ~ 0W, TTL idle power ~ 10,000W. That's a space heater, not a computer chip.

## Testing

```bash
npm test
```

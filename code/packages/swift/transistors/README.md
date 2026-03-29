# transistors

MOSFET and BJT transistor physics simulation — the semiconductor devices that implement every logic gate.

This is the foundation of the computing stack. Every logic gate, flip-flop, and CPU register ultimately reduces to the physics implemented here.

## Stack position

Layer 11 — below `logic-gates`, above nothing. Every other package in this stack depends (directly or transitively) on this one.

## What's inside

| File | Contents |
|------|----------|
| `Types.swift` | Parameter structs, region enums, output records |
| `MOSFET.swift` | NMOS and PMOS transistor physics (Shockley square-law) |
| `BJT.swift` | NPN and PNP bipolar transistor physics (Ebers-Moll) |
| `CMOSGates.swift` | CMOS inverter, NAND, NOR, AND, OR, XOR gates |
| `TTLGates.swift` | TTL NAND gate and RTL inverter |
| `Amplifier.swift` | Single-stage common-source and common-emitter amplifier analysis |
| `Analysis.swift` | Noise margins, power, timing, CMOS vs TTL comparison, Moore's Law scaling |

## Usage

```swift
import Transistors

// Evaluate a CMOS AND gate digitally
let and = CMOSAnd()
let result = and.evaluateDigital(1, 1)  // → 1

// Full physics evaluation with analog voltages
let nand = CMOSNand()
let out = nand.evaluate(va: 1.8, vb: 1.8)
print(out.logicValue)         // 0
print(out.powerDissipation)   // watts
print(out.propagationDelay)   // seconds

// Amplifier analysis
let nmos = NMOS()
let amp = analyzeCommonSource(transistor: nmos, vgs: 0.8, vdd: 1.8,
                               rDrain: 10_000, cLoad: 10e-15)
print(amp.voltageGain)        // negative (inverting)
print(amp.bandwidth)          // Hz

// CMOS vs TTL comparison
let comparison = compareCMOSvsTTL(frequency: 1e6, cLoad: 10e-15)
for row in comparison {
    print("\(row.property): CMOS=\(row.cmos), TTL=\(row.ttl)")
}
```

## Development

```bash
swift test --enable-code-coverage --verbose
```

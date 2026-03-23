# transistors

MOSFET, BJT, CMOS, and TTL transistor-level circuit simulation -- the physical foundation beneath logic gates.

## Layer 10

This package is part of Layer 10 of the coding-adventures computing stack. It sits between raw physics and the `logic_gates` package, showing how digital logic emerges from transistor-level circuits.

## What's inside

| Module | Description |
|--------|-------------|
| `types` | Constants (operating regions), parameter structs, result types |
| `mosfet` | NMOS and PMOS field-effect transistor simulation |
| `bjt` | NPN and PNP bipolar junction transistor simulation |
| `cmos_gates` | CMOS inverter, NAND, NOR, AND, OR, XOR gates |
| `ttl_gates` | TTL NAND (7400-series) and RTL inverter (Apollo-era) |
| `amplifier` | Common-source and common-emitter amplifier analysis |
| `analysis` | Noise margins, power, timing, CMOS scaling |

## Usage

```lua
local T = require("coding_adventures.transistors")

-- Create an NMOS transistor with default 180 nm parameters
local nmos = T.NMOS()
print(nmos:region(1.0, 1.0))          --> "saturation"
print(nmos:drain_current(1.0, 1.0))   --> 1.8e-04

-- Build a CMOS NAND gate and verify the truth table
local nand = T.CMOSNand()
print(nand:evaluate_digital(0, 0))    --> 1
print(nand:evaluate_digital(1, 1))    --> 0

-- Analog evaluation with full electrical detail
local out = nand:evaluate(3.3, 3.3)
print(out.voltage, out.logic_value, out.transistor_count)

-- Compare CMOS vs TTL power consumption
local cmp = T.compare_cmos_vs_ttl(1e6, 1e-12)
print(cmp.cmos.static_power_w)   --> 0  (near-zero for CMOS)
print(cmp.ttl.static_power_w)    --> ~milliwatts

-- Amplifier analysis
local amp = T.analyze_common_source(nmos, 1.0, 3.3, 10000, 1e-12)
print(amp.voltage_gain)           --> negative (inverting)
```

## How it fits in the stack

```
  Operating System
        |
      CPU / ALU
        |
    Adders, Muxes
        |
    Logic Gates        <-- logic_gates package
        |
    Transistors        <-- THIS PACKAGE
        |
    Physics (electrons, silicon, doping)
```

## Ported from

Go implementation at `code/packages/go/transistors/`. The Lua port uses metatable OOP and literate programming style with explanations of MOSFET and BJT physics inline.

## Development

```bash
# Run tests (from the package root)
cd tests && busted . --verbose --pattern=test_
```

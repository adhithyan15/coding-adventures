# CodingAdventures.Transistors

Transistor-level circuit simulation in Elixir. This package models MOSFET and BJT
transistors at the electrical level, showing how logic gates are physically
constructed from transistor pairs.

## Where It Fits

This is the layer **below** `logic_gates`. While `logic_gates` treats gates as
black boxes (input 0/1, output 0/1), this package shows what happens inside:
voltages, currents, power dissipation, and propagation delays.

```
logic_gates  (digital abstraction: 0 and 1)
    |
transistors  (electrical reality: volts, amps, watts)  <-- you are here
    |
silicon      (physics: electrons, holes, electric fields)
```

## Modules

| Module | Description |
|--------|-------------|
| `Transistors.MOSFET` | NMOS and PMOS transistor functions |
| `Transistors.BJT` | NPN and PNP transistor functions |
| `Transistors.CMOSGates` | CMOS logic gates (NOT, NAND, NOR, AND, OR, XOR) |
| `Transistors.TTLGates` | TTL NAND gate and RTL inverter |
| `Transistors.Amplifier` | Common-source and common-emitter amplifier analysis |
| `Transistors.Analysis` | Noise margins, power, timing, CMOS vs TTL comparison |
| `Transistors.Types` | Parameter structs and result structs |

## Usage

```elixir
alias CodingAdventures.Transistors.MOSFET
alias CodingAdventures.Transistors.CMOSGates
alias CodingAdventures.Transistors.Analysis

# Check NMOS operating region
MOSFET.nmos_region(1.5, 0.1)  # => :linear

# Evaluate a CMOS NAND gate
CMOSGates.nand_evaluate_digital(1, 1)  # => 0

# Compare CMOS vs TTL
Analysis.compare_cmos_vs_ttl()
```

## Running Tests

```bash
cd code/packages/elixir/transistors
mix deps.get
mix test --cover
```

## Design Notes

This is an Elixir port of the Python `transistors` package. Instead of classes
with state (Python), we use pure module functions with parameter structs passed
in. This is idiomatic Elixir — functions are stateless, data flows through them,
and structs carry configuration.

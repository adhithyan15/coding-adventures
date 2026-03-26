# coding_adventures_transistors

Transistor-level circuit simulation for the coding-adventures computing stack. This package sits one layer above logic gates, implementing the actual transistor physics that make digital logic work.

## What's Inside

- **MOSFET models** (NMOS, PMOS): Region detection (cutoff/linear/saturation), drain current calculation using the Shockley model, transconductance, and digital switch abstraction.
- **BJT models** (NPN, PNP): Region detection (cutoff/active/saturation), collector/base current via Ebers-Moll, transconductance, and digital switch abstraction.
- **CMOS logic gates**: Inverter, NAND, NOR, AND, OR, XOR — each with analog voltage simulation and digital evaluation. Demonstrates why CMOS uses near-zero static power.
- **TTL logic gates**: NAND (7400-style) and RTL inverter — historical BJT-based logic with realistic static power consumption.
- **Amplifier analysis**: Common-source (MOSFET) and common-emitter (BJT) amplifier configurations with gain, impedance, bandwidth analysis.
- **Electrical analysis**: Noise margins, power consumption breakdown (static vs dynamic), timing characteristics, CMOS vs TTL comparison, and technology scaling from 180nm to 3nm.

## How It Fits in the Stack

```
Layer 12: CPU Simulator (uses transistors for gate-level simulation)
Layer 11: Transistors        <-- THIS PACKAGE
Layer 10: Logic Gates        (pure boolean logic, no voltage/current)
```

Logic gates give you 0s and 1s. Transistors give you *why* those 0s and 1s work — actual voltages, currents, power consumption, and timing. This is the bridge between abstract boolean logic and real silicon.

## Usage

```ruby
require "coding_adventures_transistors"

include CodingAdventures::Transistors

# === MOSFET Transistors ===
nmos = NMOS.new
nmos.conducting?(vgs: 3.3)                    # => true
nmos.region(vgs: 1.5, vds: 3.0)              # => "saturation"
nmos.drain_current(vgs: 1.5, vds: 3.0)       # => 6.05e-4 A

# === CMOS Logic ===
inv = CMOSInverter.new
inv.evaluate_digital(0)                        # => 1
inv.evaluate_digital(1)                        # => 0
inv.static_power                               # => 0.0 (!)

nand = CMOSNand.new
nand.evaluate_digital(1, 1)                    # => 0

# === TTL Logic (for comparison) ===
ttl = TTLNand.new
ttl.evaluate_digital(1, 1)                     # => 0
ttl.static_power                               # => ~4.5 mW (ouch)

# === Amplifier Analysis ===
result = Amplifier.analyze_common_source_amp(
  NMOS.new, vgs: 1.5, vdd: 3.3, r_drain: 10_000
)
result.voltage_gain       # negative (inverting)
result.input_impedance    # ~1 Tohm (MOSFET advantage)

# === Electrical Analysis ===
nm = Analysis.compute_noise_margins(CMOSInverter.new)
nm.nml  # noise margin low
nm.nmh  # noise margin high

comparison = Analysis.compare_cmos_vs_ttl
# Shows why CMOS replaced TTL: ~1000x less static power
```

## Installation

```ruby
gem "coding_adventures_transistors"
```

## Running Tests

```bash
bundle install
bundle exec rake test
```

## License

MIT

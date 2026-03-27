# resistor

Ideal resistor modeling from first principles.

## Layer

**PHY02** - the first passive circuit primitive after `wave`.

This package models an ideal resistor with enough realism to support learning:

- Ohm's law
- conductance
- power and energy dissipation
- tolerance bounds
- first-order temperature coefficient adjustment
- series and parallel equivalents
- unloaded voltage divider analysis

## Why this matters

Resistors are the entry point into real circuits. They show up in:

- voltage dividers
- transistor biasing
- RC and RL networks
- line terminations
- resistor ladders used in DACs

## API

```python
from coding_adventures_resistor import (
    Resistor,
    parallel_equivalent,
    series_equivalent,
    voltage_divider,
)

r = Resistor(1_000.0, tolerance=0.01, power_rating_watts=0.25)

current = r.current_for_voltage(5.0)        # 0.005 A
power = r.power_for_voltage(5.0)            # 0.025 W
hot = r.resistance_at_temperature(75.0)     # with tempco if configured

r1 = Resistor(1_000.0)
r2 = Resistor(1_000.0)

series = series_equivalent([r1, r2])        # 2000.0 ohms
parallel = parallel_equivalent([r1, r2])    # 500.0 ohms
vout = voltage_divider(5.0, r1, r2)         # 2.5 V
```

## Core equations

- Ohm's law: `V = I * R`
- Conductance: `G = 1 / R`
- Power: `P = V * I = I^2 * R = V^2 / R`
- Energy over time: `E = P * t`

## DAC connection

Yes, resistor ladders are a classic DAC architecture. A great next step after
this package is either:

- a `voltage_divider` extension with load modeling, or
- an `r2r_ladder_dac` package built from this resistor primitive

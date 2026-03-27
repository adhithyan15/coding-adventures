# PHY02: Resistor

## 1. Overview

The `resistor` package models the simplest lumped electrical component: a device
that opposes current flow and dissipates electrical energy as heat.

This is the natural next step after `wave` if we want to grow toward real
electronics. Waves describe changing signals in time. Resistors are one of the
first things that shape those signals in actual circuits.

At the most basic level, an ideal resistor is defined by **Ohm's law**:

$$V = I R$$

where:
- $V$ is voltage across the resistor (volts)
- $I$ is current through the resistor (amps)
- $R$ is resistance (ohms)

If you know any two of those values, the third is determined.

## 2. Why Resistors Matter

Resistors are everywhere:

- limiting LED current
- biasing transistor gates and bases
- creating voltage dividers
- setting RC and RL time constants
- terminating transmission lines
- building resistor ladders for DACs and ADCs

An ideal resistor is memoryless. Unlike capacitors and inductors, it does not
store energy. At every instant, the voltage and current are related immediately
by $V = IR$.

That makes it the right first passive analog component to model.

## 3. Core Concepts

### 3.1 Resistance

Resistance is how strongly a component resists current flow.

- Unit: ohms ($\Omega$)
- Symbol: $R$

Higher resistance means less current for the same applied voltage.

### 3.2 Conductance

Conductance is the inverse of resistance:

$$G = \frac{1}{R}$$

- Unit: siemens (S)
- Symbol: $G$

Conductance is often more convenient when combining parallel paths.

### 3.3 Power Dissipation

A resistor converts electrical energy into heat.

Equivalent power formulas:

$$P = VI = I^2 R = \frac{V^2}{R}$$

- Unit: watts (W)
- Symbol: $P$

This matters because real resistors have power ratings. Exceeding the rating
can overheat and damage the part.

### 3.4 Tolerance

Real resistors are not exact. A "10 kΩ ±1%" resistor may actually be anywhere
from 9.9 kΩ to 10.1 kΩ.

Tolerance matters in:

- precision analog circuits
- filters
- bias networks
- resistor ladders for DACs

### 3.5 Temperature Coefficient

Resistance changes with temperature. A simple first-order model is:

$$R(T) = R_0 \left(1 + \alpha (T - T_0)\right)$$

where:
- $R_0$ is the nominal resistance at reference temperature $T_0$
- $\alpha$ is the temperature coefficient

For a first package, this can be optional metadata or a helper method.

## 4. Ideal vs Real Resistors

### 4.1 Ideal resistor

An ideal resistor:

- always obeys $V = IR$
- has no capacitance
- has no inductance
- has no noise
- has no temperature drift
- has no power limit

This is the right place to start.

### 4.2 Real resistor

A real resistor also has:

- tolerance
- power rating
- temperature coefficient
- Johnson thermal noise
- small parasitic inductance and capacitance

Those effects become important later, especially for audio precision,
high-frequency circuits, and transmission lines.

## 5. API Surface

The package exposes one main type and a few composition helpers.

### 5.1 `Resistor`

`Resistor(resistance_ohms, tolerance=0.0, tempco_ppm_per_c=0.0, power_rating_watts=None)`

Validation:

- `resistance_ohms` must be `> 0`
- `tolerance` must be `>= 0`
- `power_rating_watts`, if provided, must be `> 0`

Properties:

- `.resistance_ohms`
- `.tolerance`
- `.tempco_ppm_per_c`
- `.power_rating_watts`

Methods:

- `.conductance()` returns `1 / R`
- `.current_for_voltage(voltage)` returns `I = V / R`
- `.voltage_for_current(current)` returns `V = I * R`
- `.power_for_voltage(voltage)` returns `P = V^2 / R`
- `.power_for_current(current)` returns `P = I^2 R`
- `.energy_for_voltage(voltage, duration_seconds)` returns `E = P * t`
- `.energy_for_current(current, duration_seconds)` returns `E = P * t`
- `.min_resistance()` returns `R * (1 - tolerance)`
- `.max_resistance()` returns `R * (1 + tolerance)`
- `.resistance_at_temperature(celsius, reference_celsius=25.0)` uses first-order tempco

### 5.2 Network helpers

- `series_equivalent(resistors)` returns the equivalent resistance of resistors in series
- `parallel_equivalent(resistors)` returns the equivalent resistance of resistors in parallel
- `voltage_divider(vin, r_top, r_bottom)` returns the unloaded divider output

These helpers are the bridge toward resistor ladders.

## 6. Why This Connects to DACs

Yes: **many DACs are built from resistor networks**, and resistor ladders are one
of the classic architectures.

Two important examples:

- **Binary-weighted DAC**: uses resistors like $R$, $2R$, $4R$, $8R$, ...
- **R-2R ladder DAC**: uses only two resistor values, $R$ and $2R$

The R-2R ladder is especially important because matching two resistor ratios is
easier than fabricating many precise weighted values. That makes it practical
both in silicon and discrete designs.

So understanding resistors is absolutely the right place to start if you want to
build toward DACs from first principles.

## 7. Suggested Learning Order

1. Ideal resistor
2. Series and parallel combinations
3. Voltage divider
4. Current divider
5. RC circuit
6. RL circuit
7. RLC resonance
8. Transmission line as distributed R, L, C, G
9. Resistor ladders
10. DAC architectures

## 8. Test Strategy

Core tests:

- construction rejects non-positive resistance
- conductance is inverse resistance
- current and voltage helpers obey Ohm's law
- power formulas agree with each other
- series equivalent sums resistances
- parallel equivalent matches reciprocal rule
- voltage divider returns expected ratio
- tolerance bounds are computed correctly
- temperature adjustment behaves correctly at positive and negative offsets

## 9. Future Extensions

- current divider helper
- loaded voltage divider
- thermal noise density: $e_n = \sqrt{4 k T R}$
- parasitic inductance and capacitance
- frequency-domain impedance
- SPICE-style resistor element in a circuit solver

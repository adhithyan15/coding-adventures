# PHY01: Simple Harmonic Wave

## 1. Overview

The `wave` package models the simplest possible electromagnetic abstraction: a sinusoidal wave oscillating in time. This is the foundational building block for all radio and wireless communication — every signal, from AM radio to 5G, is ultimately a combination of sine waves.

A single sinusoidal wave is fully described by three numbers:

$$y(t) = A \sin(2\pi f t + \varphi)$$

where:
- $A$ is the **amplitude** (how tall the wave is — the peak displacement from zero)
- $f$ is the **frequency** (how many complete cycles per second, measured in Hertz)
- $\varphi$ is the **phase** (where in the cycle the wave starts, in radians)

## 2. The Physics

### 2.1 What is a Wave?

A wave is a disturbance that carries energy through space or time without transporting matter. Drop a stone in a pond — the ripples carry energy outward, but the water molecules just bob up and down in place.

For electromagnetic waves (radio, light, WiFi), the "disturbance" is oscillating electric and magnetic fields. But mathematically, the shape is the same sine curve whether it's a water wave, a sound wave, or a radio wave.

### 2.2 The Three Properties

**Amplitude ($A$):** The maximum displacement from the resting position. For a radio wave, this corresponds to the strength of the electric field. A louder sound has a larger amplitude. A brighter light has a larger amplitude. Units depend on what's waving — volts/meter for E-fields, pascals for sound, meters for water.

**Frequency ($f$):** The number of complete oscillations per second, measured in Hertz (Hz). A 440 Hz sound wave completes 440 full cycles every second — that's the note A above middle C. A WiFi signal at 2.4 GHz completes 2,400,000,000 cycles per second.

**Phase ($\varphi$):** The starting offset of the wave, in radians. A phase of 0 means the wave starts at zero and rises. A phase of $\pi/2$ means the wave starts at its peak. Phase matters when comparing or combining multiple waves — two waves at the same frequency but opposite phase ($\varphi = \pi$) cancel each other out completely (destructive interference).

### 2.3 Derived Properties

From the three core properties, we can compute:

- **Period** $T = 1/f$ — the time for one complete cycle (seconds)
- **Angular frequency** $\omega = 2\pi f$ — frequency in radians per second (convenient for the math)

## 3. API Surface

The package exposes a single `Wave` class/struct:

### 3.1 Constructor

`Wave(amplitude, frequency, phase=0.0)` creates a new wave.

**Validation:**
- `amplitude` must be $\geq 0$ (negative amplitude is meaningless; use phase to flip)
- `frequency` must be $> 0$ (zero frequency is a constant, not a wave)

### 3.2 Properties

| Property | Type | Description |
|----------|------|-------------|
| `.amplitude` | float | Peak amplitude $A$ |
| `.frequency` | float | Frequency in Hz |
| `.phase` | float | Phase offset in radians |

### 3.3 Derived Properties

| Method | Returns | Formula |
|--------|---------|---------|
| `.period()` | float | $T = 1/f$ |
| `.angular_frequency()` | float | $\omega = 2\pi f$ |

### 3.4 Core Method

| Method | Returns | Formula |
|--------|---------|---------|
| `.evaluate(t)` | float | $A \sin(2\pi f t + \varphi)$ |

Where $t$ is time in seconds. This is the heart of the package — given a moment in time, what is the wave's value?

## 4. Dependency

The `wave` package depends on the `trig` package (PHY00) for:
- `PI` constant (used in $2\pi f$)
- `sin()` function (the core of the wave equation)

No standard-library math functions are used. The entire computation chain is built from first principles.

## 5. Cross-Language Parity

Implemented identically across all 6 host languages (Python, Go, Ruby, TypeScript, Rust, Elixir). Each implementation passes the same test cases:

1. **Zero crossing:** A wave with phase 0 evaluates to 0 at $t = 0$
2. **Peak:** A 1 Hz wave with phase 0 reaches amplitude $A$ at $t = 0.25$ (quarter period)
3. **Period:** Value at $t$ equals value at $t + T$ for any $t$
4. **Phase shift:** A wave with phase $\pi/2$ starts at its peak
5. **Derived properties:** Period and angular frequency computed correctly
6. **Validation:** Negative amplitude and zero frequency are rejected

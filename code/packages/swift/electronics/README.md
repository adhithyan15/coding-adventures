# Electronics

First-layer hardware implementation containing foundational components.

## Overview

The `electronics` package is the workspace's first interaction point between raw abstract numbers (like waveforms and power supplies) and real-life mapped circuit components.

It holds simple, composable models that serve as building blocks for future generalized circuit/transistor modeling.

### Included Features
- `IdealResistor`: Strict OHMs law implementation linking current, voltage, and power arrays.
- `VoltageDivider`: Calculates output voltages natively given sequential resistors.
- Helper modules `DCAnalysis` and `SinusoidalResistorResponse` directly wiring abstract `power-supply` models against defined physics rules.

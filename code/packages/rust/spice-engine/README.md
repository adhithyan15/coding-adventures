# spice-engine

`spice-engine` provides SPICE-style circuit analysis primitives for Rust.

The initial slices implement:

- DC operating-point analysis for linear circuits using modified nodal analysis
  (MNA).
- DC source sweeps over independent voltage and current sources.
- AC small-signal frequency sweeps for linear RC/RL circuits.
- Backward-Euler transient analysis for linear RC/RL circuits.

The package supports resistors, capacitors, inductors, independent current sources,
independent voltage sources, ground aliases, node voltages, and voltage source
branch currents.
